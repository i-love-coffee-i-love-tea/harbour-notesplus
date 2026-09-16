import { defineStore } from 'pinia';
import { ref } from 'vue';
import { apiFetch, apiJson } from './api.js';
import { consumeSseStream, slugify, titleFromPath } from '/composables/utils.js';
import { useNotesStore } from './notes.js';
import { useAiStore } from './ai.js';
import { useUiStore } from './ui.js';

export const useImportStore = defineStore('import', () => {
  const importSourceText = ref('');
  const importTitle = ref('');
  const importMode = ref('convert_full');
  const importCustomInstruction = ref('');
  const importUrl = ref('');
  const showUrlInput = ref(false);
  const isFetchingUrl = ref(false);
  const urlFetchError = ref('');
  const showFileInput = ref(false);
  const importFilePath = ref('');
  const isLoadingFile = ref(false);
  const fileFetchError = ref('');
  const fileInputRef = ref(null);
  const isDraggingFile = ref(false);

  function toggleUrlInput() {
    showUrlInput.value = !showUrlInput.value;
    if (showUrlInput.value) { showFileInput.value = false; urlFetchError.value = ''; }
  }
  function toggleFileInput() {
    showFileInput.value = !showFileInput.value;
    if (showFileInput.value) { showUrlInput.value = false; fileFetchError.value = ''; }
  }
  function triggerFilePicker() { fileInputRef.value?.click(); }
  function clearImportSource() {
    importSourceText.value = ''; importTitle.value = '';
    urlFetchError.value = ''; fileFetchError.value = '';
  }

  async function fetchUrlContent() {
    const url = (importUrl.value || '').trim();
    if (!url) return;
    isFetchingUrl.value = true; urlFetchError.value = '';
    try {
      const data = await apiJson('/api/ai/fetch_url', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ url })
      });
      if (data.ok) {
        importSourceText.value = data.content || '';
        if (!importTitle.value.trim()) {
          const t = titleFromPath(url.startsWith('http') ? url : `https://${url}`);
          if (t) importTitle.value = t;
        }
        showUrlInput.value = false;
      } else { urlFetchError.value = data.error || 'Failed to fetch URL'; }
    } catch (e) { urlFetchError.value = `Network error: ${e.message}`; }
    finally { isFetchingUrl.value = false; }
  }

  async function loadServerFile() {
    const path = (importFilePath.value || '').trim();
    if (!path) return;
    isLoadingFile.value = true; fileFetchError.value = '';
    try {
      const data = await apiJson('/api/ai/read_file', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ file_path: path })
      });
      if (data.ok) {
        importSourceText.value = data.content || '';
        if (!importTitle.value.trim()) {
          const t = titleFromPath(path);
          if (t) importTitle.value = t;
        }
        showFileInput.value = false;
      } else { fileFetchError.value = data.error || 'Failed to read server file'; }
    } catch (e) { fileFetchError.value = `Network error: ${e.message}`; }
    finally { isLoadingFile.value = false; }
  }

  function readFileObject(file) {
    if (!file) return;
    const isHtml = file.name.endsWith('.html') || file.name.endsWith('.htm') || (file.type && file.type.includes('html'));
    const reader = new FileReader();
    reader.onload = async (event) => {
      let text = event.target.result || '';
      if (isHtml || (text.trim().startsWith('<') && (text.includes('<p') || text.includes('<div') || text.includes('<html')))) {
        try {
          const data = await apiJson('/api/ai/preprocess_html', {
            method: 'POST', headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ html: text })
          });
          if (data.ok) text = data.content;
        } catch (_) {}
      }
      importSourceText.value = text;
      if (!importTitle.value.trim()) {
        const t = titleFromPath(file.name);
        if (t) importTitle.value = t;
      }
      showFileInput.value = false;
    };
    reader.readAsText(file);
  }

  function onFileSelect(e) {
    const file = e.target?.files?.[0];
    if (file) readFileObject(file);
    if (e.target) e.target.value = '';
  }
  function onFileDrop(e) {
    isDraggingFile.value = false;
    const file = e.dataTransfer?.files?.[0];
    if (file) readFileObject(file);
  }
  async function pasteClipboard() {
    try {
      if (navigator.clipboard?.readText) {
        const text = await navigator.clipboard.readText();
        if (text) importSourceText.value = text;
      }
    } catch (e) { console.warn('Clipboard read error:', e); }
  }

  // ─── Import with SSE Streaming ───────────────────────────
  async function importText() {
    const notesStore = useNotesStore();
    const aiStore = useAiStore();
    if (aiStore.isAiBusy || !importSourceText.value.trim()) return;
    aiStore.openAiDrawer = true;
    aiStore.isAiBusy = true;
    aiStore.isAiStreaming = true;
    aiStore.streamingText = '';
    aiStore.aiError = '';

    const userMsg = `Import: ${importSourceText.value.substring(0, 80)}${importSourceText.value.length > 80 ? '...' : ''}`;
    aiStore.messages.push({ role: 'user', content: userMsg });

    try {
      const response = await apiFetch('/api/ai/template', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          template_id: 'import_convert',
          content: importSourceText.value,
          context_filename: importTitle.value ? slugify(importTitle.value) + '.adoc' : 'imported-note.adoc',
          target_title: importTitle.value || undefined,
          mode: importMode.value,
          custom_instruction: importCustomInstruction.value || undefined
        })
      });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);

      let fullAnswer = '', createdNote = null;
      await consumeSseStream(response,
        (token) => { aiStore.streamingText += token; fullAnswer += token; aiStore.scrollChatToBottom(); },
        (payload) => {
          if (payload.type === 'finished') {
            if (payload.content) fullAnswer = payload.content;
            aiStore.canUndo = payload.can_undo || false;
            if (payload.last_created_note) createdNote = payload.last_created_note;
          } else if (payload.type === 'pending_confirmation') aiStore.pendingAction = payload.action;
          else if (payload.type === 'error') aiStore.aiError = payload.error;
        }
      );

      if (fullAnswer.trim()) aiStore.messages.push({ role: 'assistant', content: fullAnswer });
      await notesStore.fetchNotesList();
      if (createdNote) await notesStore.loadNote(createdNote);
      else if (notesStore.currentFilename) await notesStore.loadNote(notesStore.currentFilename);
      importSourceText.value = '';
    } catch (err) { aiStore.aiError = `Import Error: ${err.message}`; }
    finally {
      aiStore.isAiBusy = false; aiStore.isAiStreaming = false;
      aiStore.streamingText = ''; aiStore.scrollChatToBottom();
    }
  }

  async function openImportFromNewModal() {
    const uiStore = useUiStore();
    const aiStore = useAiStore();
    uiStore.openNewNoteModal = false;
    aiStore.openAiDrawer = true;
    aiStore.aiTab = 'import';
  }

  return {
    importSourceText, importTitle, importMode, importCustomInstruction,
    importUrl, showUrlInput, isFetchingUrl, urlFetchError,
    showFileInput, importFilePath, isLoadingFile, fileFetchError,
    fileInputRef, isDraggingFile,
    toggleUrlInput, toggleFileInput, triggerFilePicker,
    clearImportSource, fetchUrlContent, loadServerFile,
    onFileSelect, onFileDrop, pasteClipboard,
    importText, openImportFromNewModal,
  };
});
