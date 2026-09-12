import { ref } from 'vue';

export function useImport({ rawContent, currentFilename, saveCurrentNote, loadInPlaceBlocks, viewMode, markPhoneReachable, markPhoneUnreachable, fetchNotesList, loadNote }) {
  const aiTab = ref('chat');
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
    if (showUrlInput.value) showFileInput.value = false;
    urlFetchError.value = '';
  }

  function toggleFileInput() {
    showFileInput.value = !showFileInput.value;
    if (showFileInput.value) showUrlInput.value = false;
    fileFetchError.value = '';
  }

  async function fetchUrlContent() {
    if (!importUrl.value.trim() || isFetchingUrl.value) return;
    isFetchingUrl.value = true;
    urlFetchError.value = '';
    try {
      const res = await fetch('/api/ai/fetch_url', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ url: importUrl.value.trim() })
      });
      if (res.ok) {
        const data = await res.json();
        importSourceText.value = data.content || '';
        if (data.title) importTitle.value = data.title;
        showUrlInput.value = false;
        importUrl.value = '';
      } else {
        const data = await res.json().catch(() => ({}));
        urlFetchError.value = data.error || `HTTP ${res.status}`;
      }
    } catch (e) {
      urlFetchError.value = 'Failed to fetch URL: ' + e.message;
    } finally {
      isFetchingUrl.value = false;
    }
  }

  async function loadServerFile() {
    if (!importFilePath.value.trim() || isLoadingFile.value) return;
    isLoadingFile.value = true;
    fileFetchError.value = '';
    try {
      const res = await fetch('/api/ai/read_file', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ file_path: importFilePath.value.trim() })
      });
      if (res.ok) {
        const data = await res.json();
        importSourceText.value = data.content || '';
        showFileInput.value = false;
        importFilePath.value = '';
      } else {
        const data = await res.json().catch(() => ({}));
        fileFetchError.value = data.error || `HTTP ${res.status}`;
      }
    } catch (e) {
      fileFetchError.value = 'Failed to load file: ' + e.message;
    } finally {
      isLoadingFile.value = false;
    }
  }

  function onFileSelect(e) {
    const file = e.target.files[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => { importSourceText.value = reader.result; };
    reader.readAsText(file);
  }

  function onFileDrop(e) {
    isDraggingFile.value = false;
    const file = e.dataTransfer.files[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => { importSourceText.value = reader.result; };
    reader.readAsText(file);
  }

  function pasteClipboard() {
    navigator.clipboard.readText().then(text => {
      if (text) importSourceText.value = text;
    }).catch(() => {});
  }

  async function importContent() {
    if (!importSourceText.value.trim()) return;
    try {
      const res = await fetch('/api/ai/import', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          content: importSourceText.value,
          title: importTitle.value,
          mode: importMode.value,
          custom_instruction: importCustomInstruction.value,
          filename: currentFilename.value,
        })
      });
      if (res.ok) {
        const data = await res.json();
        if (data.content) {
          rawContent.value = data.content;
          saveCurrentNote();
          if (loadInPlaceBlocks && viewMode.value === 'inplace') {
            await loadInPlaceBlocks(data.content);
          }
        }
        if (data.filename && data.filename !== currentFilename.value) {
          await loadNote(data.filename);
        }
        importSourceText.value = '';
        importTitle.value = '';
        if (fetchNotesList) await fetchNotesList();
      }
    } catch (_) {}
  }

  return {
    aiTab, importSourceText, importTitle, importMode,
    importCustomInstruction, importUrl, showUrlInput,
    isFetchingUrl, urlFetchError, showFileInput,
    importFilePath, isLoadingFile, fileFetchError,
    fileInputRef, isDraggingFile,
    toggleUrlInput, toggleFileInput, fetchUrlContent,
    loadServerFile, onFileSelect, onFileDrop, pasteClipboard,
    importContent,
  };
}
