import { defineStore } from 'pinia';
import { ref, computed, nextTick } from 'vue';
import { apiFetch, apiJson, apiText } from './api.js';
import { extractSlides, consumeSseStream, parseBlocksFromText, slugify, titleFromPath, isExternalUrlStr, computeFilenameFromQuery, buildLinkPreview } from '/composables/utils.js';

// Cached module references for cross-store access
let _notesMod = null, _aiMod = null;
async function getNotesStore() { if (!_notesMod) _notesMod = await import('./notes.js'); return _notesMod.useNotesStore(); }
async function getAiStore() { if (!_aiMod) _aiMod = await import('./ai.js'); return _aiMod.useAiStore(); }

export const useUiStore = defineStore('ui', () => {
  const viewMode = ref('split');
  const showExportMenu = ref(false);
  const showAccountMenu = ref(false);
  const openNewNoteModal = ref(false);
  const newNoteTitle = ref('');
  const newNoteTemplate = ref('blank');
  const inPlaceBlocks = ref([]);
  const editingBlockIndex = ref(-1);
  const activeBlockText = ref('');

  // ─── Presentation ────────────────────────────────────────
  const previousViewMode = ref('split');
  const slides = ref([]);
  const currentSlideIndex = ref(0);
  const showSlideOverview = ref(false);
  const isPresentationFullscreen = ref(false);
  const presentationStageRef = ref(null);
  const slideHtmlCache = new Map();

  const currentSlideHtml = computed(() => {
    if (!slides.value.length) return '';
    const slide = slides.value[currentSlideIndex.value];
    return slide ? (slide.html || '') : '';
  });

  async function renderSlideHtml(idx) {
    if (idx < 0 || idx >= slides.value.length) return;
    const slide = slides.value[idx];
    if (slideHtmlCache.has(slide.raw)) { slide.html = slideHtmlCache.get(slide.raw); return; }
    try {
      const html = await apiText('/api/render', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content: slide.raw, full: false })
      });
      slideHtmlCache.set(slide.raw, html);
      slide.html = html;
    } catch (e) { console.warn('Failed to render slide:', e); }
  }

  async function renderAllSlides() {
    for (let i = 0; i < slides.value.length; i++) {
      if (!slides.value[i].html) await renderSlideHtml(i);
    }
  }

  async function prepareSlides(text) {
    const notesStore = await getNotesStore();
    const content = text !== undefined ? text : notesStore.rawContent;
    const parsed = extractSlides(content);
    for (const s of parsed) {
      if (slideHtmlCache.has(s.raw)) s.html = slideHtmlCache.get(s.raw);
    }
    slides.value = parsed;
    if (currentSlideIndex.value >= parsed.length) currentSlideIndex.value = Math.max(0, parsed.length - 1);
    else if (currentSlideIndex.value < 0) currentSlideIndex.value = 0;
    renderSlideHtml(currentSlideIndex.value);
    if (currentSlideIndex.value + 1 < parsed.length) renderSlideHtml(currentSlideIndex.value + 1);
    if (currentSlideIndex.value - 1 >= 0) renderSlideHtml(currentSlideIndex.value - 1);
  }

  function nextSlide() {
    if (currentSlideIndex.value < slides.value.length - 1) {
      currentSlideIndex.value++;
      renderSlideHtml(currentSlideIndex.value);
      if (currentSlideIndex.value + 1 < slides.value.length) renderSlideHtml(currentSlideIndex.value + 1);
    }
  }

  function prevSlide() {
    if (currentSlideIndex.value > 0) {
      currentSlideIndex.value--;
      renderSlideHtml(currentSlideIndex.value);
      if (currentSlideIndex.value - 1 >= 0) renderSlideHtml(currentSlideIndex.value - 1);
    }
  }

  function goToSlide(idx) {
    if (idx >= 0 && idx < slides.value.length) {
      currentSlideIndex.value = idx;
      showSlideOverview.value = false;
      renderSlideHtml(idx);
    }
  }

  async function enterPresentationMode() {
    if (viewMode.value !== 'present') previousViewMode.value = viewMode.value;
    viewMode.value = 'present';
    await prepareSlides();
    renderAllSlides();
  }

  function exitPresentationMode() {
    if (isPresentationFullscreen.value) {
      if (document.exitFullscreen) document.exitFullscreen().catch(() => {});
      else if (document.webkitExitFullscreen) document.webkitExitFullscreen();
    }
    viewMode.value = previousViewMode.value || 'split';
    showSlideOverview.value = false;
  }

  function togglePresentationFullscreen() {
    if (!document.fullscreenElement && !document.webkitFullscreenElement) {
      const el = presentationStageRef.value || document.documentElement;
      if (el.requestFullscreen) el.requestFullscreen().catch(() => {});
      else if (el.webkitRequestFullscreen) el.webkitRequestFullscreen();
    } else {
      if (document.exitFullscreen) document.exitFullscreen().catch(() => {});
      else if (document.webkitExitFullscreen) document.webkitExitFullscreen();
    }
  }

  function onFullscreenChange() {
    isPresentationFullscreen.value = !!(document.fullscreenElement || document.webkitFullscreenElement);
  }

  let touchStartX = 0, touchStartY = 0;
  function handleTouchStart(e) {
    if (!e.changedTouches?.length) return;
    touchStartX = e.changedTouches[0].screenX;
    touchStartY = e.changedTouches[0].screenY;
  }
  function handleTouchEnd(e) {
    if (!e.changedTouches?.length) return;
    const dx = e.changedTouches[0].screenX - touchStartX;
    const dy = e.changedTouches[0].screenY - touchStartY;
    if (Math.abs(dx) > 45 && Math.abs(dy) < 60) { dx < 0 ? nextSlide() : prevSlide(); }
  }

  // ─── Link Modal ──────────────────────────────────────────
  const openLinkModal = ref(false);
  const linkSearchQuery = ref('');
  const selectedLinkFilename = ref('');
  const selectedLinkTitle = ref('');
  const linkDisplayText = ref('');
  const linkFocusedIndex = ref(0);
  const linkEditorContext = ref({ mode: 'split', start: 0, end: 0, blockIndex: null });
  const linkSearchInputRef = ref(null);
  const linkPagesListRef = ref(null);

  const isExternalUrl = computed(() => isExternalUrlStr(linkSearchQuery.value));

  const computedCustomFilename = computed(() => computeFilenameFromQuery(linkSearchQuery.value));

  const filteredLinkPages = computed(() => {
    const q = (linkSearchQuery.value || '').trim().toLowerCase();
    const nl = _cachedNotesList || [];
    if (!q) return nl;
    return nl.filter(n =>
      (n.title || '').toLowerCase().includes(q) ||
      (n.filename || '').toLowerCase().includes(q) ||
      (n.snippet || '').toLowerCase().includes(q)
    );
  });

  const isExactMatch = computed(() => {
    const q = (linkSearchQuery.value || '').trim().toLowerCase();
    if (!q) return false;
    const targetFn = q.endsWith('.adoc') ? q : `${q}.adoc`;
    return filteredLinkPages.value.some(n =>
      (n.filename || '').toLowerCase() === targetFn || (n.title || '').toLowerCase() === q
    );
  });

  const formattedLinkPreview = computed(() => buildLinkPreview({
    filename: selectedLinkFilename.value,
    title: selectedLinkTitle.value,
    displayText: linkDisplayText.value,
    query: linkSearchQuery.value,
    isExternal: isExternalUrl.value,
    customFilename: computedCustomFilename.value,
  }));

  // Cached notesList reference for filteredLinkPages computed
  let _cachedNotesList = [];
  function setCachedNotesList(list) { _cachedNotesList = list; }

  // ─── Import ──────────────────────────────────────────────
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
    const notesStore = await getNotesStore();
    const aiStore = await getAiStore();
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
    const aiStore = await getAiStore();
    openNewNoteModal.value = false;
    aiStore.openAiDrawer = true;
    aiStore.aiTab = 'import';
  }

  // ─── In-Place Block Editor ───────────────────────────────
  async function loadInPlaceBlocks(text) {
    const notesStore = await getNotesStore();
    const content = text !== undefined ? text : notesStore.rawContent;
    try {
      const data = await apiJson('/api/blocks/parse', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content })
      });
      inPlaceBlocks.value = data.blocks || [];
      nextTick(() => notesStore.setupInteractiveFeatures());
      return;
    } catch (e) { console.error('Failed to parse blocks from server:', e); }
    const raw = parseBlocksFromText(content);
    inPlaceBlocks.value = raw.map((r, i) => ({ index: i, raw: r, html: r }));
  }

  async function switchToInPlaceMode() {
    const notesStore = await getNotesStore();
    await loadInPlaceBlocks(notesStore.rawContent);
    editingBlockIndex.value = -1;
    viewMode.value = 'inplace';
  }

  function editBlock(index) {
    editingBlockIndex.value = index;
    const b = inPlaceBlocks.value[index];
    activeBlockText.value = (typeof b === 'object' && b?.raw !== undefined) ? b.raw : (b || '');
    nextTick(() => { const el = document.querySelector('.inplace-editor-card textarea'); if (el) el.focus(); });
  }

  async function saveBlockEdit(index) {
    const notesStore = await getNotesStore();
    if (index < 0 || index >= inPlaceBlocks.value.length) return;
    if (typeof inPlaceBlocks.value[index] === 'object') {
      inPlaceBlocks.value[index].raw = activeBlockText.value;
      notesStore.rawContent = inPlaceBlocks.value.map(b => typeof b === 'object' ? b.raw : b).join('\n\n');
    } else {
      inPlaceBlocks.value[index] = activeBlockText.value;
      notesStore.rawContent = inPlaceBlocks.value.join('\n\n');
    }
    editingBlockIndex.value = -1;
    notesStore.saveCurrentNote();
    await loadInPlaceBlocks(notesStore.rawContent);
  }

  function cancelBlockEdit() { editingBlockIndex.value = -1; activeBlockText.value = ''; }

  async function insertBlockAfter(index) {
    const notesStore = await getNotesStore();
    inPlaceBlocks.value.splice(index + 1, 0, { index: index + 1, raw: 'New paragraph content...', html: '<p>New paragraph content...</p>' });
    notesStore.rawContent = inPlaceBlocks.value.map(b => typeof b === 'object' ? b.raw : b).join('\n\n');
    notesStore.saveCurrentNote();
    editBlock(index + 1);
  }

  async function deleteBlock(index) {
    const notesStore = await getNotesStore();
    if (!confirm('Delete this block?')) return;
    inPlaceBlocks.value.splice(index, 1);
    notesStore.rawContent = inPlaceBlocks.value.map(b => typeof b === 'object' ? b.raw : b).join('\n\n');
    notesStore.saveCurrentNote();
    await loadInPlaceBlocks(notesStore.rawContent);
  }

  async function addBlockAtEnd() {
    const notesStore = await getNotesStore();
    inPlaceBlocks.value.push({ index: inPlaceBlocks.value.length, raw: 'New paragraph content...', html: '<p>New paragraph content...</p>' });
    notesStore.rawContent = inPlaceBlocks.value.map(b => typeof b === 'object' ? b.raw : b).join('\n\n');
    notesStore.saveCurrentNote();
    editBlock(inPlaceBlocks.value.length - 1);
  }

  // ─── Toolbar Helpers ─────────────────────────────────────
  async function insertPrefix(prefix) {
    const notesStore = await getNotesStore();
    const el = notesStore.editorTextarea;
    if (!el) return;
    const start = el.selectionStart;
    const val = notesStore.rawContent;
    const lineStart = val.lastIndexOf('\n', start - 1) + 1;
    notesStore.rawContent = val.slice(0, lineStart) + prefix + val.slice(lineStart);
    notesStore.onContentChange();
    nextTick(() => { el.focus(); el.setSelectionRange(start + prefix.length, start + prefix.length); });
  }

  async function wrapSelection(before, after) {
    const notesStore = await getNotesStore();
    const el = notesStore.editorTextarea;
    if (!el) return;
    const s = el.selectionStart, e = el.selectionEnd;
    const sel = notesStore.rawContent.slice(s, e);
    notesStore.rawContent = notesStore.rawContent.slice(0, s) + before + sel + after + notesStore.rawContent.slice(e);
    notesStore.onContentChange();
    nextTick(() => { el.focus(); el.setSelectionRange(s + before.length, e + before.length); });
  }

  function insertInPlacePrefix(prefix) { activeBlockText.value = prefix + activeBlockText.value; }
  function wrapInPlaceSelection(before, after) { activeBlockText.value = before + activeBlockText.value + after; }

  async function insertTab() {
    const notesStore = await getNotesStore();
    const el = notesStore.editorTextarea;
    if (!el) return;
    const s = el.selectionStart, end = el.selectionEnd;
    notesStore.rawContent = notesStore.rawContent.slice(0, s) + '  ' + notesStore.rawContent.slice(end);
    notesStore.onContentChange();
    nextTick(() => { el.setSelectionRange(s + 2, s + 2); });
  }

  async function insertTableTemplate() {
    const notesStore = await getNotesStore();
    const el = notesStore.editorTextarea;
    if (!el) return;
    const s = el.selectionStart;
    notesStore.rawContent = notesStore.rawContent.slice(0, s) + '\n|===\n| Header 1 | Header 2 | Header 3\n\n| Row 1 Col 1 | Row 1 Col 2 | Row 1 Col 3\n| Row 2 Col 1 | Row 2 Col 2 | Row 2 Col 3\n|===\n' + notesStore.rawContent.slice(s);
    notesStore.onContentChange();
  }

  // ─── Preview Click ───────────────────────────────────────
  async function handlePreviewClick(e) {
    const notesStore = await getNotesStore();
    const checkbox = e.target.closest('input[type="checkbox"]');
    if (checkbox) {
      const all = Array.from(document.querySelectorAll('.preview-pane input[type="checkbox"], .full-preview-pane input[type="checkbox"], .inplace-container input[type="checkbox"]'));
      const idx = all.indexOf(checkbox);
      if (idx >= 0) { e.preventDefault(); notesStore.toggleChecklistItem(idx, !checkbox.checked); }
      return;
    }
    const link = e.target.closest('a');
    if (link) {
      const href = link.getAttribute('href') || '';
      if (href.startsWith('#')) {
        const target = document.getElementById(href.slice(1));
        if (target) { e.preventDefault(); target.scrollIntoView({ behavior: 'smooth' }); }
      } else if (href.endsWith('.adoc') || href.endsWith('.html')) {
        e.preventDefault();
        notesStore.loadNote(href.replace(/\.html$/, '.adoc'));
      }
    }
  }

  // ─── Link Dialog ─────────────────────────────────────────
  async function openLinkDialog() {
    const notesStore = await getNotesStore();
    let initialText = '';
    let ctx = { mode: 'split', start: 0, end: 0, blockIndex: null };

    if (viewMode.value === 'inplace' && editingBlockIndex.value >= 0) {
      ctx.mode = 'inplace'; ctx.blockIndex = editingBlockIndex.value;
      const blockEl = document.querySelector('.inplace-editor-card textarea');
      if (blockEl) {
        ctx.start = blockEl.selectionStart || 0; ctx.end = blockEl.selectionEnd || 0;
        if (ctx.start !== ctx.end) initialText = (activeBlockText.value || '').substring(ctx.start, ctx.end);
      } else { ctx.start = (activeBlockText.value || '').length; ctx.end = ctx.start; }
    } else {
      ctx.mode = 'split';
      const el = notesStore.editorTextarea;
      if (el) {
        ctx.start = el.selectionStart || 0; ctx.end = el.selectionEnd || 0;
        if (ctx.start !== ctx.end) initialText = (notesStore.rawContent || '').substring(ctx.start, ctx.end);
      } else { ctx.start = (notesStore.rawContent || '').length; ctx.end = ctx.start; }
    }

    linkEditorContext.value = ctx;
    linkDisplayText.value = initialText;
    linkSearchQuery.value = '';
    selectedLinkFilename.value = ''; selectedLinkTitle.value = '';
    linkFocusedIndex.value = 0;
    openLinkModal.value = true;
    nextTick(() => { linkSearchInputRef.value?.focus(); });
  }

  async function confirmLinkInsert() {
    const notesStore = await getNotesStore();
    const linkText = formattedLinkPreview.value;
    if (!linkText) return;
    const ctx = linkEditorContext.value;

    if (ctx.mode === 'inplace') {
      const s = ctx.start, e = ctx.end;
      activeBlockText.value = (activeBlockText.value || '').slice(0, s) + linkText + (activeBlockText.value || '').slice(e);
      openLinkModal.value = false;
      nextTick(() => {
        const blockEl = document.querySelector('.inplace-editor-card textarea');
        if (blockEl) { blockEl.focus(); blockEl.setSelectionRange(s + linkText.length, s + linkText.length); }
      });
    } else {
      const el = notesStore.editorTextarea;
      const s = ctx.start, e = ctx.end;
      notesStore.rawContent = (notesStore.rawContent || '').slice(0, s) + linkText + (notesStore.rawContent || '').slice(e);
      notesStore.onContentChange();
      openLinkModal.value = false;
      nextTick(() => { if (el) { el.focus(); el.setSelectionRange(s + linkText.length, s + linkText.length); } });
    }
  }

  function selectLinkTarget(note) {
    if (!note) return;
    selectedLinkFilename.value = note.filename;
    selectedLinkTitle.value = note.title || note.filename;
    if (!linkDisplayText.value.trim()) linkDisplayText.value = note.title || note.filename;
  }

  function selectCustomLinkTarget(query) {
    selectedLinkFilename.value = ''; selectedLinkTitle.value = query;
    if (!linkDisplayText.value.trim()) linkDisplayText.value = query;
  }

  function handleLinkKeydown(e) {
    const hasFallback = (linkSearchQuery.value || '').trim() && !isExactMatch.value;
    const total = filteredLinkPages.value.length + (hasFallback ? 1 : 0);
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (total > 0) {
        linkFocusedIndex.value = (linkFocusedIndex.value + 1) % total;
        if (linkFocusedIndex.value < filteredLinkPages.value.length) selectLinkTarget(filteredLinkPages.value[linkFocusedIndex.value]);
        else selectCustomLinkTarget((linkSearchQuery.value || '').trim());
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (total > 0) {
        linkFocusedIndex.value = (linkFocusedIndex.value - 1 + total) % total;
        if (linkFocusedIndex.value < filteredLinkPages.value.length) selectLinkTarget(filteredLinkPages.value[linkFocusedIndex.value]);
        else selectCustomLinkTarget((linkSearchQuery.value || '').trim());
      }
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (total > 0 && !selectedLinkFilename.value && !isExternalUrl.value) {
        if (linkFocusedIndex.value < filteredLinkPages.value.length) selectLinkTarget(filteredLinkPages.value[linkFocusedIndex.value]);
      }
      confirmLinkInsert();
    } else if (e.key === 'Escape') {
      e.preventDefault(); openLinkModal.value = false;
    }
  }

  return {
    viewMode, showExportMenu, showAccountMenu,
    openNewNoteModal, newNoteTitle, newNoteTemplate,
    computedNewFilename: computed(() => slugify(newNoteTitle.value) + '.adoc'),
    inPlaceBlocks, editingBlockIndex, activeBlockText,
    loadInPlaceBlocks, switchToInPlaceMode, editBlock, saveBlockEdit, cancelBlockEdit,
    insertBlockAfter, deleteBlock, addBlockAtEnd,
    previousViewMode, slides, currentSlideIndex, showSlideOverview,
    isPresentationFullscreen, presentationStageRef, currentSlideHtml,
    prepareSlides, nextSlide, prevSlide, goToSlide,
    enterPresentationMode, exitPresentationMode,
    togglePresentationFullscreen, onFullscreenChange,
    handleTouchStart, handleTouchEnd, renderAllSlides,
    openLinkModal, linkSearchQuery, selectedLinkFilename, selectedLinkTitle,
    linkDisplayText, linkFocusedIndex, linkEditorContext,
    linkSearchInputRef, linkPagesListRef,
    isExternalUrl, computedCustomFilename, filteredLinkPages,
    isExactMatch, formattedLinkPreview,
    openLinkDialog, confirmLinkInsert, selectLinkTarget, selectCustomLinkTarget,
    handleLinkKeydown,
    importSourceText, importTitle, importMode, importCustomInstruction,
    importUrl, showUrlInput, isFetchingUrl, urlFetchError,
    showFileInput, importFilePath, isLoadingFile, fileFetchError,
    fileInputRef, isDraggingFile,
    toggleUrlInput, toggleFileInput, triggerFilePicker,
    clearImportSource, fetchUrlContent, loadServerFile,
    onFileSelect, onFileDrop, pasteClipboard,
    importText, openImportFromNewModal,
    insertPrefix, wrapSelection, insertInPlacePrefix, wrapInPlaceSelection,
    insertTab, insertTableTemplate,
    insertLink: openLinkDialog,
    handlePreviewClick,
    setCachedNotesList,
  };
});
