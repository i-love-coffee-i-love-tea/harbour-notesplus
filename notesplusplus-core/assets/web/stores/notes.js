import { defineStore } from 'pinia';
import { ref, computed, nextTick, watch } from 'vue';
import { apiFetch, apiJson, apiText } from './api.js';
import { getRequestedNote, highlightSearchTerms } from '/composables/utils.js';
import { useUiStore } from './ui.js';

export const useNotesStore = defineStore('notes', () => {
  const currentFilename = ref(getRequestedNote() || 'welcome.adoc');
  const notesList = ref([]);
  const groupTree = ref([]);
  const gallerySearchQuery = ref('');
  const searchResults = ref([]);
  const isSearching = ref(false);
  const activeSearchTerm = ref('');
  let searchTimer = null;
  const collapsedGroups = ref(new Set());
  const rawContent = ref('= Welcome to Notes Plus\n\nStart writing documentation in AsciiDoc.\n');
  const isSaving = ref(false);
  const saveStatusText = ref('Saved');
  const saveStatusClass = ref('saved');
  const renderedHtml = ref('');
  let renderTimer = null;

  const editorTextarea = ref(null);

  function flattenTreeNodes(nodes, parentPrefix = '') {
    let result = [];
    if (!Array.isArray(nodes)) return result;
    for (const node of nodes) {
      const displayName = node.display_name || node.name || (node.path ? node.path.split('/').pop() : 'Notes');
      const fullDisplayName = parentPrefix ? `${parentPrefix} / ${displayName}` : displayName;
      
      result.push({
        ...node,
        display_name: displayName,
        full_display_name: fullDisplayName,
      });
      
      if (node.children && node.children.length > 0) {
        result = result.concat(flattenTreeNodes(node.children, fullDisplayName));
      }
    }
    return result;
  }

  async function updateRenderedHtml(text) {
    if (text === undefined || text === null) return;
    try {
      let html = await apiText('/api/render', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content: text, full: false })
      });
      if (activeSearchTerm.value) {
        html = highlightSearchTerms(html, activeSearchTerm.value);
      }
      renderedHtml.value = html;
      nextTick(() => { setupInteractiveFeatures(); });
    } catch (e) {
      console.error('Render error:', e);
    }
  }

  async function fetchGroupTree() {
    try {
      const tree = await apiJson('/api/tree');
      groupTree.value = Array.isArray(tree) ? tree : [];
      if (Array.isArray(tree)) {
        const flattened = flattenTreeNodes(tree);
        const nextCollapsed = new Set(collapsedGroups.value);
        flattened.forEach(g => {
          if (g.collapsed && g.path !== undefined) {
            nextCollapsed.add(g.path);
          }
        });
        collapsedGroups.value = nextCollapsed;
      }
    } catch (err) {
      console.error('Failed to fetch group tree:', err);
    }
  }

  function toggleGroupCollapse(path) {
    const nextSet = new Set(collapsedGroups.value);
    if (nextSet.has(path)) {
      nextSet.delete(path);
    } else {
      nextSet.add(path);
    }
    collapsedGroups.value = nextSet;
  }

  function isGroupCollapsed(path) {
    return collapsedGroups.value.has(path);
  }

  const filteredGroupTree = computed(() => {
    const q = (gallerySearchQuery.value || '').trim().toLowerCase();
    const allGroups = flattenTreeNodes(groupTree.value);

    return allGroups.map(group => {
      const matchingPages = (group.pages || []).filter(p => {
        if (!q) return true;
        const title = (p.name || p.title || '').toLowerCase();
        const filename = (p.filename || p.full_path || '').toLowerCase();
        const snippet = (p.snippet || '').toLowerCase();
        return title.includes(q) || filename.includes(q) || snippet.includes(q);
      });
      return {
        ...group,
        pages: matchingPages
      };
    }).filter(group => (group.pages && group.pages.length > 0) || (q && (group.full_display_name || group.display_name || '').toLowerCase().includes(q)));
  });

  const currentNoteTitle = computed(() => {
    const fn = currentFilename.value;
    if (!fn) return '';
    const found = notesList.value.find(n => n.filename === fn || n.full_path === fn || fn.endsWith('/' + n.filename));
    if (found) return found.title || found.name || found.filename;
    const allGroups = flattenTreeNodes(groupTree.value);
    for (const g of allGroups) {
      const p = (g.pages || []).find(page => page.filename === fn || page.full_path === fn || fn.endsWith('/' + page.filename));
      if (p) return p.name || p.title || p.filename;
    }
    return fn.split('/').pop().replace(/\.adoc$/, '').replace(/_/g, ' ');
  });

  const currentNoteColor = computed(() => {
    const fn = currentFilename.value;
    if (!fn) return '';
    const found = notesList.value.find(n => n.filename === fn || n.full_path === fn || fn.endsWith('/' + n.filename));
    if (found && found.color) return found.color;
    const allGroups = flattenTreeNodes(groupTree.value);
    for (const g of allGroups) {
      const p = (g.pages || []).find(page => page.filename === fn || page.full_path === fn || fn.endsWith('/' + page.filename));
      if (p && p.color) return p.color;
    }
    return '';
  });

  const notePalette = [
    "#e67e22", "#3498db", "#2ecc71", "#9b59b6",
    "#f1c40f", "#e74c3c", "#1abc9c", "#e84393",
    "#00cec9", "#6c5ce7", "#fdcb6e", "#00b894"
  ];

  async function fetchNotesList() {
    try {
      const [list] = await Promise.all([
        apiJson('/api/notes'),
        fetchGroupTree(),
      ]);
      notesList.value = list;
      if (list.length > 0) {
        const requested = getRequestedNote();
        let target = null;
        if (requested) {
          const found = list.find(n => n.filename === requested || n.full_path === requested);
          if (found) {
            target = found.full_path || found.filename;
          } else {
            target = requested;
          }
        } else if (currentFilename.value && list.find(n => n.filename === currentFilename.value || n.full_path === currentFilename.value)) {
          target = currentFilename.value;
        } else {
          target = list[0].full_path || list[0].filename;
        }
        if (target) {
          await loadNote(target, false);
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
      rawContent.value = await apiText(`/api/notes/${encodeURI(filename)}`);
      saveStatusText.value = 'Saved';
      saveStatusClass.value = 'saved';
    } catch (err) {
      console.error(`Failed to load note ${filename}:`, err);
    }
  }

  function onNoteSelect() {
    loadNote(currentFilename.value);
  }

  async function selectNote(filename, targetViewMode = 'split') {
    if (!filename) return;
    if (!isSearchActive.value) {
      activeSearchTerm.value = '';
    }
    await loadNote(filename);
    const ui = useUiStore();
    if (ui.viewMode === 'gallery') {
      ui.viewMode = targetViewMode;
    }
  }

  async function selectSearchResultNote(page) {
    if (!page) return;
    activeSearchTerm.value = (gallerySearchQuery.value || '').trim();
    const target = page.full_path || page.filename;
    await selectNote(target);
  }

  const isSearchActive = computed(() => !!(gallerySearchQuery.value || '').trim());

  async function performSearch(query) {
    const q = (query !== undefined ? query : gallerySearchQuery.value || '').trim();
    if (searchTimer) {
      clearTimeout(searchTimer);
      searchTimer = null;
    }
    if (!q) {
      searchResults.value = [];
      isSearching.value = false;
      return;
    }

    isSearching.value = true;
    searchTimer = setTimeout(async () => {
      try {
        const results = await apiJson('/api/search?q=' + encodeURIComponent(q));
        if ((gallerySearchQuery.value || '').trim() === q) {
          searchResults.value = Array.isArray(results) ? results : [];
        }
      } catch (err) {
        console.error('Search error:', err);
        if ((gallerySearchQuery.value || '').trim() === q) {
          searchResults.value = [];
        }
      } finally {
        if ((gallerySearchQuery.value || '').trim() === q) {
          isSearching.value = false;
        }
      }
    }, 250);
  }

  watch(gallerySearchQuery, (newVal) => {
    performSearch(newVal);
  });

  function clearSearch() {
    if (searchTimer) {
      clearTimeout(searchTimer);
      searchTimer = null;
    }
    gallerySearchQuery.value = '';
    searchResults.value = [];
    isSearching.value = false;
  }

  function clearDocumentHighlight() {
    activeSearchTerm.value = '';
    updateRenderedHtml(rawContent.value);
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

  async function createNote(title, template, color) {
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
      const payload = { title, content: starterContent };
      if (color) {
        payload.color = color;
      }
      const data = await apiJson('/api/notes', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });
      await fetchNotesList();
      loadNote(data.filename);
      const ui = useUiStore();
      if (ui.viewMode === 'gallery') {
        ui.viewMode = 'split';
      }
      return true;
    } catch (err) {
      alert('Failed to create note: ' + err.message);
      return false;
    }
  }

  async function setNoteColor(filename, color) {
    const target = filename || currentFilename.value;
    if (!target) return;
    try {
      await apiJson(`/api/notes/${encodeURI(target)}/color`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ color: color || '' })
      });
      await Promise.all([
        fetchNotesList(),
        loadNote(target, false)
      ]);
      return true;
    } catch (err) {
      console.error('Failed to set note color:', err);
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
    const codeBlocks = document.querySelectorAll('.preview-pane pre, .full-preview-pane pre, .mini-doc-preview pre');
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

    if (activeSearchTerm.value) {
      const firstMatch = document.querySelector('.preview-pane mark.search-match, .full-preview-pane mark.search-match');
      if (firstMatch) {
        firstMatch.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }
    }
  }

  return {
    currentFilename, notesList, groupTree, gallerySearchQuery, collapsedGroups,
    searchResults, isSearching, activeSearchTerm, isSearchActive,
    rawContent, isSaving, saveStatusText, saveStatusClass,
    renderedHtml, editorTextarea, filteredGroupTree, currentNoteTitle, currentNoteColor, notePalette,
    updateRenderedHtml, fetchNotesList, fetchGroupTree, toggleGroupCollapse, isGroupCollapsed,
    performSearch, clearSearch, selectSearchResultNote, clearDocumentHighlight,
    loadNote, selectNote, onNoteSelect, saveCurrentNote, onContentChange,
    createNote, setNoteColor, toggleChecklistItem, setupInteractiveFeatures,
  };
});
