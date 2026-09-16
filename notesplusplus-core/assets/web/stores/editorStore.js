import { defineStore } from 'pinia';
import { ref, nextTick } from 'vue';
import { apiJson } from './api.js';
import { parseBlocksFromText } from '/composables/utils.js';
import { useNotesStore } from './notes.js';
import { useUiStore } from './ui.js';

export const useEditorStore = defineStore('editor', () => {
  const inPlaceBlocks = ref([]);
  const editingBlockIndex = ref(-1);
  const activeBlockText = ref('');

  // ─── In-Place Block Editor ───────────────────────────────
  async function loadInPlaceBlocks(text) {
    const notesStore = useNotesStore();
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
    const notesStore = useNotesStore();
    const uiStore = useUiStore();
    await loadInPlaceBlocks(notesStore.rawContent);
    editingBlockIndex.value = -1;
    uiStore.viewMode = 'inplace';
  }

  function editBlock(index) {
    editingBlockIndex.value = index;
    const b = inPlaceBlocks.value[index];
    activeBlockText.value = (typeof b === 'object' && b?.raw !== undefined) ? b.raw : (b || '');
    nextTick(() => { const el = document.querySelector('.inplace-editor-card textarea'); if (el) el.focus(); });
  }

  async function saveBlockEdit(index) {
    const notesStore = useNotesStore();
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
    const notesStore = useNotesStore();
    inPlaceBlocks.value.splice(index + 1, 0, { index: index + 1, raw: 'New paragraph content...', html: '<p>New paragraph content...</p>' });
    notesStore.rawContent = inPlaceBlocks.value.map(b => typeof b === 'object' ? b.raw : b).join('\n\n');
    notesStore.saveCurrentNote();
    editBlock(index + 1);
  }

  async function deleteBlock(index) {
    const notesStore = useNotesStore();
    if (!confirm('Delete this block?')) return;
    inPlaceBlocks.value.splice(index, 1);
    notesStore.rawContent = inPlaceBlocks.value.map(b => typeof b === 'object' ? b.raw : b).join('\n\n');
    notesStore.saveCurrentNote();
    await loadInPlaceBlocks(notesStore.rawContent);
  }

  async function addBlockAtEnd() {
    const notesStore = useNotesStore();
    inPlaceBlocks.value.push({ index: inPlaceBlocks.value.length, raw: 'New paragraph content...', html: '<p>New paragraph content...</p>' });
    notesStore.rawContent = inPlaceBlocks.value.map(b => typeof b === 'object' ? b.raw : b).join('\n\n');
    notesStore.saveCurrentNote();
    editBlock(inPlaceBlocks.value.length - 1);
  }

  // ─── Toolbar Helpers ─────────────────────────────────────
  async function insertPrefix(prefix) {
    const notesStore = useNotesStore();
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
    const notesStore = useNotesStore();
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
    const notesStore = useNotesStore();
    const el = notesStore.editorTextarea;
    if (!el) return;
    const s = el.selectionStart, end = el.selectionEnd;
    notesStore.rawContent = notesStore.rawContent.slice(0, s) + '  ' + notesStore.rawContent.slice(end);
    notesStore.onContentChange();
    nextTick(() => { el.setSelectionRange(s + 2, s + 2); });
  }

  async function insertTableTemplate() {
    const notesStore = useNotesStore();
    const el = notesStore.editorTextarea;
    if (!el) return;
    const s = el.selectionStart;
    notesStore.rawContent = notesStore.rawContent.slice(0, s) + '\n|===\n| Header 1 | Header 2 | Header 3\n\n| Row 1 Col 1 | Row 1 Col 2 | Row 1 Col 3\n| Row 2 Col 1 | Row 2 Col 2 | Row 2 Col 3\n|===\n' + notesStore.rawContent.slice(s);
    notesStore.onContentChange();
  }

  // ─── Preview Click ───────────────────────────────────────
  async function handlePreviewClick(e) {
    const notesStore = useNotesStore();
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

  return {
    inPlaceBlocks, editingBlockIndex, activeBlockText,
    loadInPlaceBlocks, switchToInPlaceMode, editBlock, saveBlockEdit, cancelBlockEdit,
    insertBlockAfter, deleteBlock, addBlockAtEnd,
    insertPrefix, wrapSelection, insertInPlacePrefix, wrapInPlaceSelection,
    insertTab, insertTableTemplate,
    handlePreviewClick,
  };
});
