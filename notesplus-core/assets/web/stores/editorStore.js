import { defineStore } from 'pinia';
import { nextTick } from 'vue';
import { useNotesStore } from './notes.js';

export const useEditorStore = defineStore('editor', () => {
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
      const all = Array.from(document.querySelectorAll('.preview-pane input[type="checkbox"], .full-preview-pane input[type="checkbox"]'));
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
    insertPrefix, wrapSelection,
    insertTab, insertTableTemplate,
    handlePreviewClick,
  };
});
