import { ref, computed } from 'vue';

export function useAiAssistant() {
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

  const aiConfig = ref({
    provider: 'ollama',
    model: 'llama3.2',
    system_prompt: ''
  });

  const isCurrentModelInList = computed(() => {
    if (!aiConfig.value.model) return false;
    return availableModels.value.some(m => m.id === aiConfig.value.model || m.name === aiConfig.value.model);
  });

  async function fetchAiConfig() {
    try {
      const res = await fetch('/api/ai/config');
      if (res.ok) {
        const data = await res.json();
        if (data.provider) aiConfig.value.provider = data.provider;
        if (data.model) aiConfig.value.model = data.model;
        if (data.system_prompt !== undefined) aiConfig.value.system_prompt = data.system_prompt;
      }
    } catch (_) {}
  }

  async function fetchAvailableModels() {
    isLoadingModels.value = true;
    modelsError.value = '';
    try {
      const res = await fetch('/api/ai/models');
      if (res.ok) {
        const data = await res.json();
        availableModels.value = data.models || [];
      } else {
        modelsError.value = 'Failed to load models';
      }
    } catch (e) {
      modelsError.value = 'Failed to load models: ' + e.message;
    } finally {
      isLoadingModels.value = false;
    }
  }

  function onProviderChange() {
    fetchAvailableModels();
  }

  function onModelSelect(modelId) {
    aiConfig.value.model = modelId;
  }

  async function sendAiPrompt(prompt, currentFilename, rawContent) {
    if (!prompt.trim() || isAiBusy.value) return;
    isAiBusy.value = true;
    aiError.value = '';
    messages.value.push({ role: 'user', content: prompt });
    aiPromptInput.value = '';

    try {
      const res = await fetch('/api/ai/chat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          message: prompt,
          filename: currentFilename,
          content: rawContent,
          messages: messages.value.slice(0, -1),
        })
      });

      if (!res.ok) {
        const data = await res.json().catch(() => ({}));
        throw new Error(data.error || `HTTP ${res.status}`);
      }

      const contentType = res.headers.get('content-type') || '';
      if (contentType.includes('text/event-stream')) {
        isAiStreaming.value = true;
        streamingText.value = '';
        const reader = res.body.getReader();
        const decoder = new TextDecoder();
        let buffer = '';

        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          buffer += decoder.decode(value, { stream: true });
          const lines = buffer.split('\n');
          buffer = lines.pop();
          for (const line of lines) {
            if (line.startsWith('data: ')) {
              const data = line.slice(6);
              if (data === '[DONE]') break;
              try {
                const parsed = JSON.parse(data);
                if (parsed.text) streamingText.value += parsed.text;
                if (parsed.action) pendingAction.value = parsed.action;
                if (parsed.can_undo !== undefined) canUndo.value = parsed.can_undo;
              } catch (_) {}
            }
          }
        }

        messages.value.push({ role: 'assistant', content: streamingText.value });
        isAiStreaming.value = false;
        streamingText.value = '';
      } else {
        const data = await res.json();
        messages.value.push({ role: 'assistant', content: data.response || data.message || '' });
        if (data.action) pendingAction.value = data.action;
        if (data.can_undo !== undefined) canUndo.value = data.can_undo;
      }
    } catch (e) {
      aiError.value = e.message;
    } finally {
      isAiBusy.value = false;
    }
  }

  async function confirmAction(currentFilename) {
    if (!pendingAction.value) return;
    try {
      const res = await fetch('/api/ai/confirm', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action: pendingAction.value, filename: currentFilename })
      });
      if (res.ok) {
        pendingAction.value = null;
        canUndo.value = true;
      }
    } catch (_) {}
  }

  async function undoAction(currentFilename) {
    try {
      const res = await fetch('/api/ai/undo', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ filename: currentFilename })
      });
      if (res.ok) {
        canUndo.value = false;
        return await res.json();
      }
    } catch (_) {}
    return null;
  }

  function clearChat() {
    messages.value = [];
    pendingAction.value = null;
    canUndo.value = false;
    aiError.value = '';
  }

  return {
    openAiDrawer, showAiSettings, isAiBusy, isAiStreaming,
    streamingText, aiPromptInput, aiError, messages,
    pendingAction, canUndo, availableModels, isLoadingModels,
    modelsError, aiConfig, isCurrentModelInList,
    fetchAiConfig, fetchAvailableModels, onProviderChange,
    onModelSelect, sendAiPrompt, confirmAction, undoAction, clearChat,
  };
}
