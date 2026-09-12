import { ref } from 'vue';

export function useImport({ rawContent, currentFilename, saveCurrentNote, viewMode, markPhoneReachable, markPhoneUnreachable, fetchNotesList, loadNote }) {
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
    if (showUrlInput.value) {
      showFileInput.value = false;
      urlFetchError.value = '';
    }
  }

  function toggleFileInput() {
    showFileInput.value = !showFileInput.value;
    if (showFileInput.value) {
      showUrlInput.value = false;
      fileFetchError.value = '';
    }
  }

  function triggerFilePicker() {
    if (fileInputRef.value) fileInputRef.value.click();
  }

  function clearImportSource() {
    importSourceText.value = '';
    importTitle.value = '';
    urlFetchError.value = '';
    fileFetchError.value = '';
  }

  async function fetchUrlContent() {
    const url = (importUrl.value || '').trim();
    if (!url) return;
    isFetchingUrl.value = true;
    urlFetchError.value = '';
    try {
      const res = await fetch('/api/ai/fetch_url', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ url })
      });
      const data = await res.json();
      if (res.ok && data.ok) {
        importSourceText.value = data.content || '';
        if (!importTitle.value.trim()) {
          try {
            const parsedUrl = new URL(url.startsWith('http') ? url : `https://${url}`);
            const pathParts = parsedUrl.pathname.split('/').filter(p => p.length > 0);
            if (pathParts.length > 0) {
              const lastPart = pathParts[pathParts.length - 1].replace(/\.[^/.]+$/, '').replace(/[-_]+/g, ' ');
              if (lastPart.length > 2) {
                importTitle.value = lastPart.charAt(0).toUpperCase() + lastPart.slice(1);
              }
            } else if (parsedUrl.hostname) {
              importTitle.value = parsedUrl.hostname;
            }
          } catch (_) {}
        }
        showUrlInput.value = false;
      } else {
        urlFetchError.value = data.error || 'Failed to fetch URL';
      }
    } catch (e) {
      urlFetchError.value = `Network error: ${e.message}`;
    } finally {
      isFetchingUrl.value = false;
    }
  }

  async function loadServerFile() {
    const path = (importFilePath.value || '').trim();
    if (!path) return;
    isLoadingFile.value = true;
    fileFetchError.value = '';
    try {
      const res = await fetch('/api/ai/read_file', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ file_path: path })
      });
      const data = await res.json();
      if (res.ok && data.ok) {
        importSourceText.value = data.content || '';
        if (!importTitle.value.trim()) {
          const parts = path.split(/[\/\\]/);
          const lastPart = parts[parts.length - 1].replace(/\.[^/.]+$/, '').replace(/[-_]+/g, ' ');
          if (lastPart.length > 0) {
            importTitle.value = lastPart.charAt(0).toUpperCase() + lastPart.slice(1);
          }
        }
        showFileInput.value = false;
      } else {
        fileFetchError.value = data.error || 'Failed to read server file';
      }
    } catch (e) {
      fileFetchError.value = `Network error: ${e.message}`;
    } finally {
      isLoadingFile.value = false;
    }
  }

  function readFileObject(file) {
    if (!file) return;
    const isHtml = file.name.endsWith('.html') || file.name.endsWith('.htm') || (file.type && file.type.includes('html'));
    const reader = new FileReader();
    reader.onload = async (event) => {
      let text = event.target.result || '';
      if (isHtml || (text.trim().startsWith('<') && (text.includes('<p') || text.includes('<div') || text.includes('<html') || text.includes('<!DOCTYPE') || text.includes('<!doctype')))) {
        try {
          const res = await fetch('/api/ai/preprocess_html', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ html: text })
          });
          const data = await res.json();
          if (res.ok && data.ok) text = data.content;
        } catch (_) {}
      }
      importSourceText.value = text;
      if (!importTitle.value.trim()) {
        const cleanName = file.name.replace(/\.[^/.]+$/, '').replace(/[-_]+/g, ' ');
        if (cleanName.length > 0) {
          importTitle.value = cleanName.charAt(0).toUpperCase() + cleanName.slice(1);
        }
      }
      showFileInput.value = false;
    };
    reader.readAsText(file);
  }

  function onFileSelect(e) {
    const file = e.target && e.target.files && e.target.files[0];
    if (file) readFileObject(file);
    if (e.target) e.target.value = '';
  }

  function onFileDrop(e) {
    isDraggingFile.value = false;
    const file = e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files[0];
    if (file) readFileObject(file);
  }

  async function pasteClipboard() {
    try {
      if (navigator.clipboard && navigator.clipboard.readText) {
        const text = await navigator.clipboard.readText();
        if (text) importSourceText.value = text;
      }
    } catch (e) {
      console.warn('Clipboard read error:', e);
    }
  }

  return {
    importSourceText, importTitle, importMode,
    importCustomInstruction, importUrl, showUrlInput,
    isFetchingUrl, urlFetchError, showFileInput,
    importFilePath, isLoadingFile, fileFetchError,
    fileInputRef, isDraggingFile,
    toggleUrlInput, toggleFileInput, triggerFilePicker,
    clearImportSource, fetchUrlContent,
    loadServerFile, onFileSelect, onFileDrop, pasteClipboard,
  };
}
