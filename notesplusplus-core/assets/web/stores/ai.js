import { defineStore } from 'pinia';
import { ref, computed, nextTick } from 'vue';
import { apiFetch, apiJson } from './api.js';
import { consumeSseStream } from '/composables/utils.js';

export const useAiStore = defineStore('ai', () => {
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

  const aiConfig = ref({ provider: 'ollama', model: 'llama3.2', system_prompt: '' });
  const aiTab = ref('chat');

  const isCurrentModelInList = computed(() => {
    if (!aiConfig.value.model) return false;
    return availableModels.value.some(m => m.id === aiConfig.value.model || m.name === aiConfig.value.model);
  });

  function scrollChatToBottom() {
    nextTick(() => {
      if (chatMessagesContainer.value) chatMessagesContainer.value.scrollTop = chatMessagesContainer.value.scrollHeight;
    });
  }

  async function fetchAiConfig() {
    try {
      const data = await apiJson('/api/ai/config');
      if (data.provider) aiConfig.value.provider = data.provider;
      if (data.model) aiConfig.value.model = data.model;
      aiConfig.value.system_prompt = data.system_prompt || '';
      canUndo.value = !!data.can_undo;
      await fetchAvailableModels();
    } catch (err) {
      console.warn('Could not load AI config:', err);
    }
  }

  async function fetchAvailableModels() {
    isLoadingModels.value = true;
    modelsError.value = '';
    try {
      const data = await apiJson('/api/ai/models');
      availableModels.value = (data && Array.isArray(data.models)) ? data.models : [];
    } catch (err) {
      console.warn('Failed to fetch models from server:', err);
    } finally {
      isLoadingModels.value = false;
    }
  }

  async function onProviderChange() { await saveAiSettings(); }
  async function onModelSelect() { await saveAiSettings(); }

  async function saveAiSettings() {
    try {
      await apiJson('/api/ai/config', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          provider: aiConfig.value.provider,
          model: aiConfig.value.model,
          system_prompt: aiConfig.value.system_prompt
        })
      });
      showAiSettings.value = false;
      await fetchAvailableModels();
    } catch (err) {
      alert('Failed to save AI config: ' + err.message);
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
    const { useNotesStore } = await import('./notes.js');
    const notesStore = useNotesStore();

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
      const response = await apiFetch('/api/ai/chat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          prompt,
          context_filename: notesStore.currentFilename,
          context_content: notesStore.rawContent
        })
      });

      if (!response.ok) throw new Error(`HTTP ${response.status}: ${await response.text()}`);

      let fullAnswer = '';
      await consumeSseStream(
        response,
        (token) => { streamingText.value += token; fullAnswer += token; scrollChatToBottom(); },
        async (payload) => {
          if (payload.type === 'pending_confirmation') {
            pendingAction.value = payload.action;
            openAiDrawer.value = true;
          } else if (payload.type === 'finished') {
            if (payload.content) fullAnswer = payload.content;
            if (payload.can_undo !== undefined) canUndo.value = payload.can_undo;
            if (payload.last_snapshot_id || payload.last_created_note) {
              await notesStore.fetchNotesList();
              await notesStore.loadNote(notesStore.currentFilename);
            }
          } else if (payload.type === 'error') {
            aiError.value = payload.error;
          }
        }
      );

      if (fullAnswer.trim()) messages.value.push({ role: 'assistant', content: fullAnswer });
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
    const prompts = {
      summarize: 'Please provide a clear, structured summary of this AsciiDoc document.',
      fix_grammar: 'Review this AsciiDoc document, fixing all grammar and spelling errors while preserving formatting, headings, and structure.',
      add_admonition: 'Analyze this document and add relevant AsciiDoc [NOTE], [TIP], or [WARNING] blocks to highlight key takeaways.',
      format_table: 'Convert the main data or list points in this document into a well-structured AsciiDoc table |=== ... |===',
      continue_writing: 'Continue writing the next logical section of this AsciiDoc document.'
    };
    await sendUserPrompt(prompts[templateId] || `Run template ${templateId} on this note.`);
  }

  async function confirmAction(approved) {
    const { useNotesStore } = await import('./notes.js');
    const notesStore = useNotesStore();

    if (isAiBusy.value) return;
    isAiBusy.value = true;
    isAiStreaming.value = true;
    streamingText.value = '';
    pendingAction.value = null;

    try {
      const response = await apiFetch('/api/ai/confirm', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ approved })
      });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);

      let fullAnswer = '';
      await consumeSseStream(
        response,
        (token) => { streamingText.value += token; fullAnswer += token; scrollChatToBottom(); },
        async (payload) => {
          if (payload.type === 'finished') {
            if (payload.content) fullAnswer = payload.content;
            if (payload.can_undo !== undefined) canUndo.value = payload.can_undo;
            await notesStore.fetchNotesList();
            await notesStore.loadNote(notesStore.currentFilename);
          } else if (payload.type === 'error') {
            aiError.value = payload.error;
          }
        }
      );

      if (fullAnswer.trim()) messages.value.push({ role: 'assistant', content: fullAnswer });
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
    const { useNotesStore } = await import('./notes.js');
    const notesStore = useNotesStore();

    try {
      const data = await apiJson('/api/ai/undo', { method: 'POST' });
      canUndo.value = !!data.can_undo;
      await notesStore.loadNote(notesStore.currentFilename);
      messages.value.push({ role: 'assistant', content: `↩ ${data.message}` });
    } catch (err) {
      alert('Undo error: ' + err.message);
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
});
