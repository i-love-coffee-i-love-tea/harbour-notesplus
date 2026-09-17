import { defineStore } from 'pinia';
import { ref, computed, nextTick } from 'vue';
import { isExternalUrlStr, computeFilenameFromQuery, buildLinkPreview } from '/composables/utils.js';
import { useNotesStore } from './notes.js';
import { useUiStore } from './ui.js';
import { useEditorStore } from './editorStore.js';

export const useLinkStore = defineStore('link', () => {
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

  // Cached notesList reference for filteredLinkPages computed
  let _cachedNotesList = [];
  function setCachedNotesList(list) { _cachedNotesList = list; }

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

  // ─── Link Dialog ─────────────────────────────────────────
  async function openLinkDialog() {
    const notesStore = useNotesStore();
    const uiStore = useUiStore();
    const editorStore = useEditorStore();
    let initialText = '';
    let ctx = { mode: 'split', start: 0, end: 0, blockIndex: null };

    if (uiStore.viewMode === 'inplace' && editorStore.editingBlockIndex >= 0) {
      ctx.mode = 'inplace'; ctx.blockIndex = editorStore.editingBlockIndex;
      const blockEl = document.querySelector('.inplace-editor-card textarea');
      if (blockEl) {
        ctx.start = blockEl.selectionStart || 0; ctx.end = blockEl.selectionEnd || 0;
        if (ctx.start !== ctx.end) initialText = (editorStore.activeBlockText || '').substring(ctx.start, ctx.end);
      } else { ctx.start = (editorStore.activeBlockText || '').length; ctx.end = ctx.start; }
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
    const notesStore = useNotesStore();
    const editorStore = useEditorStore();
    const linkText = formattedLinkPreview.value;
    if (!linkText) return;
    const ctx = linkEditorContext.value;

    if (ctx.mode === 'inplace') {
      const s = ctx.start, e = ctx.end;
      editorStore.activeBlockText = (editorStore.activeBlockText || '').slice(0, s) + linkText + (editorStore.activeBlockText || '').slice(e);
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
    openLinkModal, linkSearchQuery, selectedLinkFilename, selectedLinkTitle,
    linkDisplayText, linkFocusedIndex, linkEditorContext,
    linkSearchInputRef, linkPagesListRef,
    isExternalUrl, computedCustomFilename, filteredLinkPages,
    isExactMatch, formattedLinkPreview,
    openLinkDialog, confirmLinkInsert, selectLinkTarget, selectCustomLinkTarget,
    handleLinkKeydown,
    insertLink: openLinkDialog,
    setCachedNotesList,
  };
});
