import { defineStore } from 'pinia';
import { ref, computed, nextTick, watch } from 'vue';
import { apiFetch, apiJson, apiText } from './api.js';
import { getRequestedNote } from '/composables/utils.js';

export const useNotesStore = defineStore('notes', () => {
  const currentFilename = ref(getRequestedNote() || 'welcome.adoc');
  const notesList = ref([]);
  const rawContent = ref('= Welcome to Notes Plus\n\nStart writing documentation in AsciiDoc.\n');
  const isSaving = ref(false);
  const saveStatusText = ref('Saved');
  const saveStatusClass = ref('saved');
  const renderedHtml = ref('');
  let renderTimer = null;

  const editorTextarea = ref(null);

  async function updateRenderedHtml(text) {
    if (text === undefined || text === null) return;
    try {
      renderedHtml.value = await apiText('/api/render', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content: text, full: false })
      });
      nextTick(() => { setupInteractiveFeatures(); });
    } catch (e) {
      console.error('Render error:', e);
    }
  }

  async function fetchNotesList() {
    try {
      const list = await apiJson('/api/notes');
      notesList.value = list;
      if (list.length > 0) {
        const requested = getRequestedNote();
        let target = null;
        if (requested && list.find(n => n.filename === requested)) {
          target = requested;
        } else if (currentFilename.value && list.find(n => n.filename === currentFilename.value)) {
          target = currentFilename.value;
        } else {
          target = list[0].filename;
        }
        if (target && (target !== currentFilename.value || !rawContent.value || rawContent.value.startsWith('= Welcome to Notes Plus\n\nStart writing'))) {
          await loadNote(target);
        }
      }
    } catch (err) {
      console.error('Failed to fetch notes list:', err);
    }
  }

  async function loadNote(filename, updateHistory = true) {
    if (!filename) return;
    currentFilename.value = filename;
    try { localStorage.setItem('notesplus_last_note', filename); } catch (_) {}

    if (updateHistory && typeof history !== 'undefined' && history.replaceState) {
      const desiredHash = '#' + encodeURIComponent(filename);
      if (window.location.hash !== desiredHash) history.replaceState(null, '', desiredHash);
    }

    try {
      rawContent.value = await apiText(`/api/notes/${filename}`);
      saveStatusText.value = 'Saved';
      saveStatusClass.value = 'saved';
    } catch (err) {
      console.error(`Failed to load note ${filename}:`, err);
    }
  }

  function onNoteSelect() {
    loadNote(currentFilename.value);
  }

  async function saveCurrentNote() {
    if (!currentFilename.value || isSaving.value) return;
    isSaving.value = true;
    saveStatusText.value = 'Saving...';
    saveStatusClass.value = 'saving';
    try {
      const res = await apiFetch(`/api/notes/${currentFilename.value}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'text/plain; charset=utf-8' },
        body: rawContent.value
      });
      if (res.ok) {
        saveStatusText.value = 'Saved';
        saveStatusClass.value = 'saved';
        fetchNotesList();
      } else {
        saveStatusText.value = 'Error saving';
        saveStatusClass.value = 'unsaved';
      }
    } catch (err) {
      saveStatusText.value = 'Save failed';
      saveStatusClass.value = 'unsaved';
      console.error('Save error:', err);
    } finally {
      isSaving.value = false;
    }
  }

  function onContentChange() {
    saveStatusText.value = 'Unsaved changes';
    saveStatusClass.value = 'unsaved';
  }

  async function createNote(title, template) {
    if (!title) return;
    let starterContent = `= ${title}\n\n`;
    if (template === 'technical') {
      starterContent = `= ${title}\n:toc: left\n:icons: font\n\n== Overview\nDescribe system architecture and design.\n\n== Requirements\n* [ ] Core functionality\n* [ ] Performance goals\n\n[source,rust]\n----\nfn main() {\n    println!("Hello Notes Plus!");\n}\n----\n`;
    } else if (template === 'meeting') {
      starterContent = `= Meeting: ${title}\n:icons: font\n\nDate: ${new Date().toISOString().slice(0, 10)}\nAttendees: User\n\n== Agenda\n. Topic 1\n. Topic 2\n\n== Action Items\n* [ ] Task 1\n* [ ] Task 2\n`;
    } else if (template === 'journal') {
      starterContent = `= Journal: ${title}\n:icons: font\n\n== ${new Date().toLocaleDateString()}\n\nWrite your thoughts here...\n`;
    } else if (template === 'presentation') {
      starterContent = `= ${title}\n:icons: font\n\nWelcome to ${title}.\n\n== Agenda\n* Introduction\n* Key Architecture\n* Demonstration\n* Summary\n\n== Key Architecture\n[source,rust]\n----\n// Clean & Modular\npub fn present_deck() {\n    println!("Presenting slides offline");\n}\n----\n\n== Summary\n* Responsive presentation view\n* AsciiDoc page break & heading support\n* Pure local execution\n`;
    }
    try {
      const data = await apiJson('/api/notes', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ title, content: starterContent })
      });
      await fetchNotesList();
      loadNote(data.filename);
      return true;
    } catch (err) {
      alert('Failed to create note: ' + err.message);
      return false;
    }
  }

  async function toggleChecklistItem(itemIdx, targetChecked) {
    try {
      const data = await apiJson(`/api/notes/${currentFilename.value}/toggle`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ item_index: itemIdx, checked: targetChecked })
      });
      if (data.content) {
        rawContent.value = data.content;
        if (data.html) renderedHtml.value = data.html;
      }
    } catch (e) {
      console.error('Failed to toggle checklist item:', e);
    }
  }

  function setupInteractiveFeatures() {
    const codeBlocks = document.querySelectorAll('.preview-pane pre, .full-preview-pane pre, .inplace-rendered-card pre');
    codeBlocks.forEach(pre => {
      if (pre.querySelector('.copy-code-btn')) return;
      pre.style.position = 'relative';
      const copyBtn = document.createElement('button');
      copyBtn.className = 'copy-code-btn';
      copyBtn.innerText = '📋 Copy';
      copyBtn.title = 'Copy code to clipboard';
      copyBtn.onclick = (e) => {
        e.stopPropagation();
        const codeEl = pre.querySelector('code') || pre;
        const text = codeEl.innerText || codeEl.textContent;
        navigator.clipboard.writeText(text).then(() => {
          copyBtn.innerText = '✓ Copied!';
          setTimeout(() => { copyBtn.innerText = '📋 Copy'; }, 2000);
        });
      };
      pre.appendChild(copyBtn);
    });
  }

  return {
    currentFilename, notesList, rawContent,
    isSaving, saveStatusText, saveStatusClass,
    renderedHtml, editorTextarea,
    updateRenderedHtml, fetchNotesList, loadNote,
    onNoteSelect, saveCurrentNote, onContentChange,
    createNote, toggleChecklistItem, setupInteractiveFeatures,
  };
});
