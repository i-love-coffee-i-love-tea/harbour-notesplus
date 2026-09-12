import { ref, computed, nextTick } from 'vue';
import { consumeSseStream } from './utils.js';

export function useAiAssistant({ currentFilename, rawContent, notesList, markPhoneReachable, markPhoneUnreachable, fetchNotesList, loadNote }) {
  const openAiDrawer = ref(false);
  const showAiSettings = ref(false);
  const isAiBusy = ref(false);
  const isAiStreaming = ref(false);
  const streamingText = ref('');
  const aiPromptInput = ref('');
  const aiError = ref('');
  const messages = ref([]);
  const pendingAction = ref(null);
  const canUndo = ref(false);
  const availableModels = ref([]);
  const isLoadingModels = ref(false);
  const modelsError = ref('');
  const chatMessagesContainer = ref(null);

  const aiConfig = ref({
    provider: 'ollama',
    model: 'llama3.2',
    system_prompt: ''
  });

  const aiTab = ref('chat');

  const isCurrentModelInList = computed(() => {
    if (!aiConfig.value.model) return false;
    return availableModels.value.some(m => m.id === aiConfig.value.model || m.name === aiConfig.value.model);
  });

  function scrollChatToBottom() {
    nextTick(() => {
      if (chatMessagesContainer.value) {
        chatMessagesContainer.value.scrollTop = chatMessagesContainer.value.scrollHeight;
      }
    });
  }

  async function fetchAiConfig() {
    try {
      const res = await fetch('/api/ai/config');
      if (res.ok) {
        if (markPhoneReachable) markPhoneReachable();
        const data = await res.json();
        if (data.provider) aiConfig.value.provider = data.provider;
        if (data.model) aiConfig.value.model = data.model;
        aiConfig.value.system_prompt = data.system_prompt || '';
        canUndo.value = !!data.can_undo;
        await fetchAvailableModels();
      } else {
        if (markPhoneUnreachable) markPhoneUnreachable(new Error(`HTTP ${res.status}`));
      }
    } catch (err) {
      console.warn('Could not load AI config:', err);
      if (markPhoneUnreachable) markPhoneUnreachable(err);
    }
  }

  async function fetchAvailableModels() {
    isLoadingModels.value = true;
    modelsError.value = '';
    try {
      const res = await fetch('/api/ai/models');
      if (res.ok) {
        if (markPhoneReachable) markPhoneReachable();
        const data = await res.json();
        availableModels.value = (data && Array.isArray(data.models)) ? data.models : [];
      } else {
        console.warn('Could not fetch models from server:', res.status);
      }
    } catch (err) {
      console.warn('Failed to fetch models from server:', err);
    } finally {
      isLoadingModels.value = false;
    }
  }

  async function onProviderChange() {
    await saveAiSettings();
  }

  async function onModelSelect() {
    await saveAiSettings();
  }

  async function saveAiSettings() {
    try {
      const res = await fetch('/api/ai/config', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          provider: aiConfig.value.provider,
          model: aiConfig.value.model,
          system_prompt: aiConfig.value.system_prompt
        })
      });
      if (res.ok) {
        if (markPhoneReachable) markPhoneReachable();
        showAiSettings.value = false;
        await fetchAvailableModels();
      } else {
        if (markPhoneUnreachable) markPhoneUnreachable(new Error(`HTTP ${res.status}`));
      }
    } catch (err) {
      alert('Failed to save AI config: ' + err.message);
      if (markPhoneUnreachable) markPhoneUnreachable(err);
    }
  }

  function toggleAiDrawer() {
    openAiDrawer.value = !openAiDrawer.value;
    if (openAiDrawer.value) {
      aiTab.value = 'chat';
      scrollChatToBottom();
      if (availableModels.value.length === 0) fetchAvailableModels();
    }
  }

  function toggleAiDrawerImport() {
    openAiDrawer.value = !openAiDrawer.value;
    if (openAiDrawer.value) {
      aiTab.value = 'import';
      if (availableModels.value.length === 0) fetchAvailableModels();
    }
  }

  async function sendUserPrompt(customInstruction = null) {
    const instructionText = typeof customInstruction === 'string' ? customInstruction : null;
    const prompt = (instructionText || aiPromptInput.value || '').trim();
    if (!prompt || isAiBusy.value) return;

    messages.value.push({ role: 'user', content: prompt });
    if (!instructionText) aiPromptInput.value = '';
    aiError.value = '';
    isAiBusy.value = true;
    isAiStreaming.value = true;
    streamingText.value = '';
    scrollChatToBottom();

    try {
      const response = await fetch('/api/ai/chat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          prompt: prompt,
          context_filename: currentFilename.value,
          context_content: rawContent.value
        })
      });

      if (!response.ok) {
        if (markPhoneUnreachable) markPhoneUnreachable(new Error(`HTTP ${response.status}`));
        throw new Error(`HTTP ${response.status}: ${await response.text()}`);
      }
      if (markPhoneReachable) markPhoneReachable();

      let fullAnswer = '';

      await consumeSseStream(
        response,
        (token) => {
          streamingText.value += token;
          fullAnswer += token;
          scrollChatToBottom();
        },
        async (payload) => {
          if (payload.type === 'pending_confirmation') {
            pendingAction.value = payload.action;
            openAiDrawer.value = true;
          } else if (payload.type === 'finished') {
            if (payload.content) fullAnswer = payload.content;
            if (payload.can_undo !== undefined) canUndo.value = payload.can_undo;
            if (payload.last_snapshot_id || payload.last_created_note) {
              await fetchNotesList();
              await loadNote(currentFilename.value);
            }
          } else if (payload.type === 'error') {
            aiError.value = payload.error;
          }
        }
      );

      if (fullAnswer.trim()) {
        messages.value.push({ role: 'assistant', content: fullAnswer });
      }
    } catch (err) {
      aiError.value = `Server AI Error: ${err.message}`;
    } finally {
      isAiBusy.value = false;
      isAiStreaming.value = false;
      streamingText.value = '';
      scrollChatToBottom();
    }
  }

  async function triggerTemplate(templateId) {
    if (isAiBusy.value) return;
    openAiDrawer.value = true;

    const templatePrompts = {
      summarize: 'Please provide a clear, structured summary of this AsciiDoc document.',
      fix_grammar: 'Review this AsciiDoc document, fixing all grammar and spelling errors while preserving formatting, headings, and structure.',
      add_admonition: 'Analyze this document and add relevant AsciiDoc [NOTE], [TIP], or [WARNING] blocks to highlight key takeaways.',
      format_table: 'Convert the main data or list points in this document into a well-structured AsciiDoc table |=== ... |===',
      continue_writing: 'Continue writing the next logical section of this AsciiDoc document.'
    };

    const prompt = templatePrompts[templateId] || `Run template ${templateId} on this note.`;
    await sendUserPrompt(prompt);
  }

  async function confirmAction(approved) {
    if (isAiBusy.value) return;
    isAiBusy.value = true;
    isAiStreaming.value = true;
    streamingText.value = '';
    pendingAction.value = null;

    try {
      const response = await fetch('/api/ai/confirm', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ approved })
      });

      if (!response.ok) {
        if (markPhoneUnreachable) markPhoneUnreachable(new Error(`HTTP ${response.status}`));
        throw new Error(`HTTP ${response.status}`);
      }
      if (markPhoneReachable) markPhoneReachable();

      let fullAnswer = '';

      await consumeSseStream(
        response,
        (token) => {
          streamingText.value += token;
          fullAnswer += token;
          scrollChatToBottom();
        },
        async (payload) => {
          if (payload.type === 'finished') {
            if (payload.content) fullAnswer = payload.content;
            if (payload.can_undo !== undefined) canUndo.value = payload.can_undo;
            await fetchNotesList();
            await loadNote(currentFilename.value);
          } else if (payload.type === 'error') {
            aiError.value = payload.error;
          }
        }
      );

      if (fullAnswer.trim()) {
        messages.value.push({ role: 'assistant', content: fullAnswer });
      }
    } catch (err) {
      aiError.value = `Failed to confirm action: ${err.message}`;
    } finally {
      isAiBusy.value = false;
      isAiStreaming.value = false;
      streamingText.value = '';
      scrollChatToBottom();
    }
  }

  async function undoLastAiAction() {
    try {
      const res = await fetch('/api/ai/undo', { method: 'POST' });
      if (res.ok) {
        if (markPhoneReachable) markPhoneReachable();
        const data = await res.json();
        canUndo.value = !!data.can_undo;
        await loadNote(currentFilename.value);
        messages.value.push({ role: 'assistant', content: `\u21a9 ${data.message}` });
      } else {
        if (markPhoneUnreachable) markPhoneUnreachable(new Error(`HTTP ${res.status}`));
        alert('Undo failed: Server error');
      }
    } catch (err) {
      alert('Undo error: ' + err.message);
      if (markPhoneUnreachable) markPhoneUnreachable(err);
    }
  }

  return {
    openAiDrawer, showAiSettings, isAiBusy, isAiStreaming,
    streamingText, aiPromptInput, aiError, messages,
    pendingAction, canUndo, availableModels, isLoadingModels,
    modelsError, aiConfig, isCurrentModelInList, aiTab,
    chatMessagesContainer, scrollChatToBottom,
    fetchAiConfig, fetchAvailableModels, onProviderChange,
    onModelSelect, saveAiSettings, toggleAiDrawer, toggleAiDrawerImport,
    sendUserPrompt, triggerTemplate, confirmAction, undoLastAiAction,
  };
}
