import { createApp, ref, computed, watch, nextTick, onMounted } from 'vue';
import { formatMarkdown, getRequestedNote, consumeSseStream } from '/composables/utils.js';
import { useAuth } from '/composables/useAuth.js';
import { useTheme } from '/composables/useTheme.js';
import { useHealthCheck } from '/composables/useHealthCheck.js';
import { usePresentation } from '/composables/usePresentation.js';
import { useLinkModal } from '/composables/useLinkModal.js';
import { useAiAssistant } from '/composables/useAiAssistant.js';
import { useImport } from '/composables/useImport.js';

// Initialize Asciidoctor compiler instance if available
let asciidoctorInstance = null;
try {
  if (typeof Asciidoctor !== 'undefined') {
    asciidoctorInstance = Asciidoctor();
  }
} catch (e) {
  console.warn('Asciidoctor.js initialization error, fallback to server rendering:', e);
}

createApp({
  setup() {
    // Core Note & View State
    const currentFilename = ref(getRequestedNote() || 'welcome.adoc');
    const notesList = ref([]);
    const rawContent = ref('= Welcome to Notes++\n\nStart writing documentation in AsciiDoc.\n');
    const viewMode = ref('split');
    const isSaving = ref(false);
    const saveStatusText = ref('Saved');
    const saveStatusClass = ref('saved');
    const showExportMenu = ref(false);
    const showAccountMenu = ref(false);

    // Modal State
    const openNewNoteModal = ref(false);
    const newNoteTitle = ref('');
    const newNoteTemplate = ref('blank');

    // In-Place Block Editing State
    const inPlaceBlocks = ref([]);
    const editingBlockIndex = ref(-1);
    const activeBlockText = ref('');

    // Compute preview HTML via native Rust /api/render
    const renderedHtml = ref('');
    let renderTimer = null;

    // Editor ref
    const editorTextarea = ref(null);

    // ─── Composables ───────────────────────────────────────────────────

    const {
      isAuthenticated, authUser, authError, authStatus,
      authVerificationCode, authChallengeId, authCanRetry,
      sessionRemainingText, sessionRemainingFullText,
      fetchAuthConfig, logout, startPhoneAuth, stopAuthPolling,
    } = useAuth({
      onAuthenticated: async () => {
        await fetchNotesList();
        await ai.fetchAiConfig();
      }
    });

    const {
      isPhoneReachable, isCheckingConnection, connectionError,
      markPhoneReachable, markPhoneUnreachable, checkConnection,
    } = useHealthCheck({
      onReachable: () => {
        fetchNotesList();
        updateRenderedHtml(rawContent.value);
      }
    });

    const presentation = usePresentation({
      rawContent, viewMode, markPhoneReachable, markPhoneUnreachable
    });

    const linkModal = useLinkModal({ notesList });

    const ai = useAiAssistant({
      currentFilename, rawContent, notesList,
      markPhoneReachable, markPhoneUnreachable,
      fetchNotesList: () => fetchNotesList(),
      loadNote: (fn) => loadNote(fn),
    });

    const imp = useImport({
      rawContent, currentFilename,
      saveCurrentNote: () => saveCurrentNote(),
      viewMode,
      markPhoneReachable, markPhoneUnreachable,
      fetchNotesList: () => fetchNotesList(),
      loadNote: (fn) => loadNote(fn),
    });

    const theme = useTheme();

    // ─── Core App Logic ────────────────────────────────────────────────

    async function updateRenderedHtml(text) {
      if (text === undefined || text === null) return;
      try {
        const res = await fetch('/api/render', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ content: text, full: false })
        });
        if (res.ok) {
          markPhoneReachable();
          renderedHtml.value = await res.text();
          nextTick(() => { setupInteractiveFeatures(); });
        } else {
          markPhoneUnreachable(new Error(`HTTP ${res.status}`));
        }
      } catch (e) {
        console.error('Render error:', e);
        markPhoneUnreachable(e);
      }
    }

    watch(rawContent, (newVal) => {
      clearTimeout(renderTimer);
      renderTimer = setTimeout(() => {
        updateRenderedHtml(newVal);
        if (viewMode.value === 'present') {
          presentation.prepareSlides(newVal);
        }
      }, 80);
    }, { immediate: true });

    // Computed filename for new note modal
    const computedNewFilename = computed(() => {
      const slug = newNoteTitle.value.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
      return (slug || 'untitled') + '.adoc';
    });

    // ─── Note CRUD ─────────────────────────────────────────────────────

    async function fetchNotesList() {
      try {
        const res = await fetch('/api/notes');
        if (res.ok) {
          markPhoneReachable();
          const list = await res.json();
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
            if (target && (target !== currentFilename.value || !rawContent.value || rawContent.value.startsWith('= Welcome to Notes++\n\nStart writing'))) {
              await loadNote(target);
            }
          }
        } else {
          markPhoneUnreachable(new Error(`HTTP ${res.status}`));
        }
      } catch (err) {
        console.error('Failed to fetch notes list:', err);
        markPhoneUnreachable(err);
      }
    }

    async function loadNote(filename, updateHistory = true) {
      if (!filename) return;
      currentFilename.value = filename;
      try { localStorage.setItem('notesplusplus_last_note', filename); } catch (_) {}

      if (updateHistory && typeof history !== 'undefined' && history.replaceState) {
        const desiredHash = '#' + encodeURIComponent(filename);
        if (window.location.hash !== desiredHash) {
          history.replaceState(null, '', desiredHash);
        }
      }

      try {
        const res = await fetch(`/api/notes/${filename}`);
        if (res.ok) {
          markPhoneReachable();
          rawContent.value = await res.text();
          saveStatusText.value = 'Saved';
          saveStatusClass.value = 'saved';
          if (viewMode.value === 'inplace') {
            await loadInPlaceBlocks(rawContent.value);
          } else if (viewMode.value === 'present') {
            presentation.prepareSlides(rawContent.value);
            presentation.renderAllSlides();
          }
        } else {
          markPhoneUnreachable(new Error(`HTTP ${res.status}`));
        }
      } catch (err) {
        console.error(`Failed to load note ${filename}:`, err);
        markPhoneUnreachable(err);
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
        const res = await fetch(`/api/notes/${currentFilename.value}`, {
          method: 'PUT',
          headers: { 'Content-Type': 'text/plain; charset=utf-8' },
          body: rawContent.value
        });
        if (res.ok) {
          markPhoneReachable();
          saveStatusText.value = 'Saved';
          saveStatusClass.value = 'saved';
          fetchNotesList();
        } else {
          saveStatusText.value = 'Error saving';
          saveStatusClass.value = 'unsaved';
          markPhoneUnreachable(new Error(`HTTP ${res.status}`));
        }
      } catch (err) {
        saveStatusText.value = 'Save failed';
        saveStatusClass.value = 'unsaved';
        console.error('Save error:', err);
        markPhoneUnreachable(err);
      } finally {
        isSaving.value = false;
      }
    }

    function onContentChange() {
      saveStatusText.value = 'Unsaved changes';
      saveStatusClass.value = 'unsaved';
    }

    async function createNote() {
      const title = newNoteTitle.value.trim();
      if (!title) return;

      let starterContent = `= ${title}\n\n`;
      if (newNoteTemplate.value === 'technical') {
        starterContent = `= ${title}\n:toc: left\n:icons: font\n\n== Overview\nDescribe system architecture and design.\n\n== Requirements\n* [ ] Core functionality\n* [ ] Performance goals\n\n[source,rust]\n----\nfn main() {\n    println!("Hello Notes++!");\n}\n----\n`;
      } else if (newNoteTemplate.value === 'meeting') {
        starterContent = `= Meeting: ${title}\n:icons: font\n\nDate: ${new Date().toISOString().slice(0, 10)}\nAttendees: User\n\n== Agenda\n. Topic 1\n. Topic 2\n\n== Action Items\n* [ ] Task 1\n* [ ] Task 2\n`;
      } else if (newNoteTemplate.value === 'journal') {
        starterContent = `= Journal: ${title}\n:icons: font\n\n== ${new Date().toLocaleDateString()}\n\nWrite your thoughts here...\n`;
      } else if (newNoteTemplate.value === 'presentation') {
        starterContent = `= ${title}\n:icons: font\n\nWelcome to ${title}.\n\n== Agenda\n* Introduction\n* Key Architecture\n* Demonstration\n* Summary\n\n== Key Architecture\n[source,rust]\n----\n// Clean & Modular\npub fn present_deck() {\n    println!("Presenting slides offline");\n}\n----\n\n== Summary\n* Responsive presentation view\n* AsciiDoc page break & heading support\n* Pure local execution\n`;
      }

      try {
        const res = await fetch('/api/notes', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ title, content: starterContent })
        });
        if (res.ok) {
          markPhoneReachable();
          const data = await res.json();
          openNewNoteModal.value = false;
          newNoteTitle.value = '';
          await fetchNotesList();
          loadNote(data.filename);
        } else {
          markPhoneUnreachable(new Error(`HTTP ${res.status}`));
        }
      } catch (err) {
        alert('Failed to create note: ' + err.message);
        markPhoneUnreachable(err);
      }
    }

    // ─── In-Place Block Editor ─────────────────────────────────────────

    function parseBlocksFromText(text) {
      if (!text || !text.trim()) return [''];
      const rawBlocks = text.split(/\n\s*\n/);
      const blocks = [];
      let currentAcc = '';
      for (let i = 0; i < rawBlocks.length; i++) {
        const b = rawBlocks[i].trim();
        if (!b) continue;
        if (currentAcc) {
          currentAcc += '\n\n' + rawBlocks[i];
          const delimMatches = currentAcc.match(/^(=|--|-|\*|\.|_){4,}|^\|===/gm);
          if (delimMatches && delimMatches.length % 2 === 0) {
            blocks.push(currentAcc);
            currentAcc = '';
          }
        } else {
          const delimMatches = b.match(/^(=|--|-|\*|\.|_){4,}|^\|===/gm);
          if (delimMatches && delimMatches.length % 2 === 1) {
            currentAcc = rawBlocks[i];
          } else {
            blocks.push(rawBlocks[i]);
          }
        }
      }
      if (currentAcc) blocks.push(currentAcc);
      return blocks.length > 0 ? blocks : [text];
    }

    async function loadInPlaceBlocks(text) {
      try {
        const res = await fetch('/api/blocks/parse', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ content: text !== undefined ? text : rawContent.value })
        });
        if (res.ok) {
          markPhoneReachable();
          const data = await res.json();
          inPlaceBlocks.value = data.blocks || [];
          nextTick(() => { setupInteractiveFeatures(); });
          return;
        } else {
          markPhoneUnreachable(new Error(`HTTP ${res.status}`));
        }
      } catch (e) {
        console.error('Failed to parse blocks from server:', e);
        markPhoneUnreachable(e);
      }
      const raw = parseBlocksFromText(text !== undefined ? text : rawContent.value);
      inPlaceBlocks.value = raw.map((r, i) => ({ index: i, raw: r, html: r }));
    }

    async function switchToInPlaceMode() {
      await loadInPlaceBlocks(rawContent.value);
      editingBlockIndex.value = -1;
      viewMode.value = 'inplace';
    }

    function editBlock(index) {
      editingBlockIndex.value = index;
      const b = inPlaceBlocks.value[index];
      activeBlockText.value = (typeof b === 'object' && b !== null && b.raw !== undefined) ? b.raw : (b || '');
      nextTick(() => {
        const el = document.querySelector('.inplace-editor-card textarea');
        if (el) el.focus();
      });
    }

    async function saveBlockEdit(index) {
      if (index >= 0 && index < inPlaceBlocks.value.length) {
        if (typeof inPlaceBlocks.value[index] === 'object') {
          inPlaceBlocks.value[index].raw = activeBlockText.value;
          rawContent.value = inPlaceBlocks.value.map(b => typeof b === 'object' ? b.raw : b).join('\n\n');
        } else {
          inPlaceBlocks.value[index] = activeBlockText.value;
          rawContent.value = inPlaceBlocks.value.join('\n\n');
        }
        editingBlockIndex.value = -1;
        saveCurrentNote();
        await loadInPlaceBlocks(rawContent.value);
      }
    }

    function cancelBlockEdit() {
      editingBlockIndex.value = -1;
      activeBlockText.value = '';
    }

    async function insertBlockAfter(index) {
      const newBlock = { index: index + 1, raw: 'New paragraph content...', html: '<p>New paragraph content...</p>' };
      inPlaceBlocks.value.splice(index + 1, 0, newBlock);
      rawContent.value = inPlaceBlocks.value.map(b => typeof b === 'object' ? b.raw : b).join('\n\n');
      saveCurrentNote();
      editBlock(index + 1);
    }

    async function deleteBlock(index) {
      if (confirm('Delete this block?')) {
        inPlaceBlocks.value.splice(index, 1);
        rawContent.value = inPlaceBlocks.value.map(b => typeof b === 'object' ? b.raw : b).join('\n\n');
        saveCurrentNote();
        await loadInPlaceBlocks(rawContent.value);
      }
    }

    async function addBlockAtEnd() {
      const newBlock = { index: inPlaceBlocks.value.length, raw: 'New paragraph content...', html: '<p>New paragraph content...</p>' };
      inPlaceBlocks.value.push(newBlock);
      rawContent.value = inPlaceBlocks.value.map(b => typeof b === 'object' ? b.raw : b).join('\n\n');
      saveCurrentNote();
      editBlock(inPlaceBlocks.value.length - 1);
    }

    // ─── Interactive Features ──────────────────────────────────────────

    function setupInteractiveFeatures() {
      const codeBlocks = document.querySelectorAll('.preview-pane pre, .full-preview-pane pre, .inplace-rendered-card pre');
      codeBlocks.forEach(pre => {
        if (pre.querySelector('.copy-code-btn')) return;
        pre.style.position = 'relative';
        const copyBtn = document.createElement('button');
        copyBtn.className = 'copy-code-btn';
        copyBtn.innerText = '\ud83d\udccb Copy';
        copyBtn.title = 'Copy code to clipboard';
        copyBtn.onclick = (e) => {
          e.stopPropagation();
          const codeEl = pre.querySelector('code') || pre;
          const text = codeEl.innerText || codeEl.textContent;
          navigator.clipboard.writeText(text).then(() => {
            copyBtn.innerText = '\u2713 Copied!';
            setTimeout(() => { copyBtn.innerText = '\ud83d\udccb Copy'; }, 2000);
          });
        };
        pre.appendChild(copyBtn);
      });
    }

    async function toggleChecklistItem(itemIdx, targetChecked) {
      try {
        const res = await fetch(`/api/notes/${currentFilename.value}/toggle`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ item_index: itemIdx, checked: targetChecked })
        });
        if (res.ok) {
          markPhoneReachable();
          const data = await res.json();
          if (data.content) {
            rawContent.value = data.content;
            if (data.html) renderedHtml.value = data.html;
            if (viewMode.value === 'inplace') await loadInPlaceBlocks(data.content);
          }
        } else {
          markPhoneUnreachable(new Error(`HTTP ${res.status}`));
        }
      } catch (e) {
        console.error('Failed to toggle checklist item:', e);
        markPhoneUnreachable(e);
      }
    }

    function handlePreviewClick(e) {
      const checkbox = e.target.closest('input[type="checkbox"]');
      if (checkbox) {
        const allCheckboxes = Array.from(document.querySelectorAll('.preview-pane input[type="checkbox"], .full-preview-pane input[type="checkbox"], .inplace-container input[type="checkbox"]'));
        const itemIdx = allCheckboxes.indexOf(checkbox);
        if (itemIdx >= 0) {
          e.preventDefault();
          toggleChecklistItem(itemIdx, !checkbox.checked);
        }
        return;
      }
      const link = e.target.closest('a');
      if (link) {
        const href = link.getAttribute('href') || '';
        if (href.startsWith('#')) {
          const targetEl = document.getElementById(href.slice(1));
          if (targetEl) { e.preventDefault(); targetEl.scrollIntoView({ behavior: 'smooth' }); }
        } else if (href.endsWith('.adoc') || href.endsWith('.html')) {
          e.preventDefault();
          loadNote(href.replace(/\.html$/, '.adoc'));
        }
      }
    }

    // ─── Import with SSE Streaming ─────────────────────────────────────

    async function importText() {
      if (ai.isAiBusy.value || !imp.importSourceText.value.trim()) return;
      ai.openAiDrawer.value = true;
      ai.isAiBusy.value = true;
      ai.isAiStreaming.value = true;
      ai.streamingText.value = '';
      ai.aiError.value = '';

      const userMsg = `Import: ${imp.importSourceText.value.substring(0, 80)}${imp.importSourceText.value.length > 80 ? '...' : ''}`;
      ai.messages.value.push({ role: 'user', content: userMsg });

      try {
        const response = await fetch('/api/ai/template', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            template_id: 'import_convert',
            content: imp.importSourceText.value,
            context_filename: imp.importTitle.value ? imp.importTitle.value.replace(/\s+/g, '-').toLowerCase() + '.adoc' : 'imported-note.adoc',
            target_title: imp.importTitle.value || undefined,
            mode: imp.importMode.value,
            custom_instruction: imp.importCustomInstruction.value || undefined
          })
        });

        if (!response.ok) {
          markPhoneUnreachable(new Error(`HTTP ${response.status}`));
          throw new Error(`HTTP ${response.status}`);
        }
        markPhoneReachable();

        let fullAnswer = '';
        let createdNote = null;

        await consumeSseStream(
          response,
          (token) => { ai.streamingText.value += token; fullAnswer += token; ai.scrollChatToBottom(); },
          (payload) => {
            if (payload.type === 'finished') {
              if (payload.content) fullAnswer = payload.content;
              ai.canUndo.value = payload.can_undo || false;
              if (payload.last_created_note) createdNote = payload.last_created_note;
            } else if (payload.type === 'pending_confirmation') {
              ai.pendingAction.value = payload.action;
            } else if (payload.type === 'error') {
              ai.aiError.value = payload.error;
            }
          }
        );

        if (fullAnswer.trim()) ai.messages.value.push({ role: 'assistant', content: fullAnswer });
        await fetchNotesList();
        if (createdNote) {
          await loadNote(createdNote);
        } else if (currentFilename.value) {
          await loadNote(currentFilename.value);
        }
        imp.importSourceText.value = '';
      } catch (err) {
        ai.aiError.value = `Import Error: ${err.message}`;
      } finally {
        ai.isAiBusy.value = false;
        ai.isAiStreaming.value = false;
        ai.streamingText.value = '';
        ai.scrollChatToBottom();
      }
    }

    function openImportFromNewModal() {
      openNewNoteModal.value = false;
      ai.openAiDrawer.value = true;
      ai.aiTab.value = 'import';
    }

    // ─── Link Insertion ────────────────────────────────────────────────

    function openLinkDialog() {
      let initialText = '';
      let ctx = { mode: 'split', start: 0, end: 0, blockIndex: null };

      if (viewMode.value === 'inplace' && editingBlockIndex.value !== null && editingBlockIndex.value >= 0) {
        ctx.mode = 'inplace';
        ctx.blockIndex = editingBlockIndex.value;
        const blockEl = document.querySelector('.inplace-editor-card textarea');
        if (blockEl) {
          ctx.start = blockEl.selectionStart || 0;
          ctx.end = blockEl.selectionEnd || 0;
          if (ctx.start !== ctx.end) initialText = (activeBlockText.value || '').substring(ctx.start, ctx.end);
        } else {
          ctx.start = (activeBlockText.value || '').length;
          ctx.end = ctx.start;
        }
      } else {
        ctx.mode = 'split';
        const el = editorTextarea.value;
        if (el) {
          ctx.start = el.selectionStart || 0;
          ctx.end = el.selectionEnd || 0;
          if (ctx.start !== ctx.end) initialText = (rawContent.value || '').substring(ctx.start, ctx.end);
        } else {
          ctx.start = (rawContent.value || '').length;
          ctx.end = ctx.start;
        }
      }

      linkModal.linkEditorContext.value = ctx;
      linkModal.linkDisplayText.value = initialText;
      linkModal.linkSearchQuery.value = '';
      linkModal.selectedLinkFilename.value = '';
      linkModal.selectedLinkTitle.value = '';
      linkModal.linkFocusedIndex.value = 0;
      linkModal.openLinkModal.value = true;

      nextTick(() => {
        if (linkModal.linkSearchInputRef.value) linkModal.linkSearchInputRef.value.focus();
      });
    }

    function confirmLinkInsert() {
      const linkText = linkModal.formattedLinkPreview.value;
      if (!linkText) return;

      const ctx = linkModal.linkEditorContext.value;
      if (ctx.mode === 'inplace') {
        const s = ctx.start, e = ctx.end;
        activeBlockText.value = (activeBlockText.value || '').slice(0, s) + linkText + (activeBlockText.value || '').slice(e);
        linkModal.openLinkModal.value = false;
        nextTick(() => {
          const blockEl = document.querySelector('.inplace-editor-card textarea');
          if (blockEl) { blockEl.focus(); blockEl.setSelectionRange(s + linkText.length, s + linkText.length); }
        });
      } else {
        const el = editorTextarea.value;
        const s = ctx.start, e = ctx.end;
        rawContent.value = (rawContent.value || '').slice(0, s) + linkText + (rawContent.value || '').slice(e);
        onContentChange();
        linkModal.openLinkModal.value = false;
        nextTick(() => {
          if (el) { el.focus(); el.setSelectionRange(s + linkText.length, s + linkText.length); }
        });
      }
    }

    function selectLinkTarget(note) {
      if (!note) return;
      linkModal.selectedLinkFilename.value = note.filename;
      linkModal.selectedLinkTitle.value = note.title || note.filename;
      if (!linkModal.linkDisplayText.value.trim()) {
        linkModal.linkDisplayText.value = note.title || note.filename;
      }
    }

    function selectCustomLinkTarget(query) {
      linkModal.selectedLinkFilename.value = '';
      linkModal.selectedLinkTitle.value = query;
      if (!linkModal.linkDisplayText.value.trim()) {
        linkModal.linkDisplayText.value = query;
      }
    }

    // ─── Toolbar Helpers ───────────────────────────────────────────────

    function insertPrefix(prefix) {
      const el = editorTextarea.value;
      if (!el) return;
      const start = el.selectionStart;
      const val = rawContent.value;
      const lineStart = val.lastIndexOf('\n', start - 1) + 1;
      rawContent.value = val.slice(0, lineStart) + prefix + val.slice(lineStart);
      onContentChange();
      nextTick(() => {
        el.focus();
        el.setSelectionRange(start + prefix.length, start + prefix.length);
      });
    }

    function wrapSelection(before, after) {
      const el = editorTextarea.value;
      if (!el) return;
      const s = el.selectionStart, e = el.selectionEnd;
      const sel = rawContent.value.slice(s, e);
      rawContent.value = rawContent.value.slice(0, s) + before + sel + after + rawContent.value.slice(e);
      onContentChange();
      nextTick(() => {
        el.focus();
        el.setSelectionRange(s + before.length, e + before.length);
      });
    }

    function insertInPlacePrefix(prefix) {
      activeBlockText.value = prefix + activeBlockText.value;
    }

    function wrapInPlaceSelection(before, after) {
      activeBlockText.value = before + activeBlockText.value + after;
    }

    function insertTab(e) {
      const el = editorTextarea.value;
      if (!el) return;
      const s = el.selectionStart, end = el.selectionEnd;
      rawContent.value = rawContent.value.slice(0, s) + '  ' + rawContent.value.slice(end);
      onContentChange();
      nextTick(() => { el.setSelectionRange(s + 2, s + 2); });
    }

    function insertTableTemplate() {
      const tableSnippet = '\n|===\n| Header 1 | Header 2 | Header 3\n\n| Row 1 Col 1 | Row 1 Col 2 | Row 1 Col 3\n| Row 2 Col 1 | Row 2 Col 2 | Row 2 Col 3\n|===\n';
      const el = editorTextarea.value;
      if (!el) return;
      const s = el.selectionStart;
      rawContent.value = rawContent.value.slice(0, s) + tableSnippet + rawContent.value.slice(s);
      onContentChange();
    }

    // ─── Link Keydown & Helpers ────────────────────────────────────────

    function handleLinkKeydown(e) {
      const hasFallback = (linkModal.linkSearchQuery.value || '').trim() && !linkModal.isExactMatch.value;
      const totalCount = linkModal.filteredLinkPages.value.length + (hasFallback ? 1 : 0);

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        if (totalCount > 0) {
          linkModal.linkFocusedIndex.value = (linkModal.linkFocusedIndex.value + 1) % totalCount;
          if (linkModal.linkFocusedIndex.value < linkModal.filteredLinkPages.value.length) {
            selectLinkTarget(linkModal.filteredLinkPages.value[linkModal.linkFocusedIndex.value]);
          } else {
            selectCustomLinkTarget((linkModal.linkSearchQuery.value || '').trim());
          }
        }
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        if (totalCount > 0) {
          linkModal.linkFocusedIndex.value = (linkModal.linkFocusedIndex.value - 1 + totalCount) % totalCount;
          if (linkModal.linkFocusedIndex.value < linkModal.filteredLinkPages.value.length) {
            selectLinkTarget(linkModal.filteredLinkPages.value[linkModal.linkFocusedIndex.value]);
          } else {
            selectCustomLinkTarget((linkModal.linkSearchQuery.value || '').trim());
          }
        }
      } else if (e.key === 'Enter') {
        e.preventDefault();
        if (totalCount > 0 && !linkModal.selectedLinkFilename.value && !linkModal.isExternalUrl.value) {
          if (linkModal.linkFocusedIndex.value < linkModal.filteredLinkPages.value.length) {
            selectLinkTarget(linkModal.filteredLinkPages.value[linkModal.linkFocusedIndex.value]);
          }
        }
        confirmLinkInsert();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        linkModal.openLinkModal.value = false;
      }
    }

    // ─── Keyboard Shortcuts ────────────────────────────────────────────

    function handleGlobalKeyDown(e) {
      if (linkModal.openLinkModal.value) {
        if (e.key === 'Escape') { e.preventDefault(); linkModal.openLinkModal.value = false; }
        return;
      }

      if ((e.ctrlKey || e.metaKey) && (e.key === 's' || e.key === 'S')) {
        e.preventDefault();
        saveCurrentNote();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && (e.key === 'k' || e.key === 'K')) {
        e.preventDefault();
        openLinkDialog();
        return;
      }

      if (viewMode.value === 'present') {
        if (['INPUT', 'TEXTAREA', 'SELECT'].includes(e.target?.tagName)) return;
        switch (e.key) {
          case 'ArrowRight': case 'ArrowDown': case 'PageDown': case ' ': case 'Enter': case 'l': case 'L': case 'j': case 'J':
            e.preventDefault(); presentation.nextSlide(); break;
          case 'ArrowLeft': case 'ArrowUp': case 'PageUp': case 'Backspace': case 'h': case 'H': case 'k': case 'K':
            e.preventDefault(); presentation.prevSlide(); break;
          case 'Home': e.preventDefault(); presentation.goToSlide(0); break;
          case 'End': e.preventDefault(); presentation.goToSlide(presentation.slides.value.length - 1); break;
          case 'f': case 'F': case 'F11': e.preventDefault(); presentation.togglePresentationFullscreen(); break;
          case 'o': case 'O': case 'g': case 'G': e.preventDefault(); presentation.showSlideOverview.value = !presentation.showSlideOverview.value; break;
          case 'Escape':
            e.preventDefault();
            if (presentation.showSlideOverview.value) { presentation.showSlideOverview.value = false; }
            else { presentation.exitPresentationMode(); }
            break;
        }
      }
    }

    // ─── Message Formatting ────────────────────────────────────────────

    function formatMessageContent(content) {
      return formatMarkdown(content);
    }

    // ─── Lifecycle ─────────────────────────────────────────────────────

    let heartbeatTimer = null;

    onMounted(async () => {
      window.addEventListener('keydown', handleGlobalKeyDown);
      document.addEventListener('fullscreenchange', presentation.onFullscreenChange);
      document.addEventListener('webkitfullscreenchange', presentation.onFullscreenChange);
      document.addEventListener('click', (e) => {
        if (!e.target.closest('.export-dropdown')) showExportMenu.value = false;
        if (!e.target.closest('.account-dropdown-wrapper')) showAccountMenu.value = false;
      });
      window.addEventListener('hashchange', () => {
        const req = getRequestedNote();
        if (req && req !== currentFilename.value) loadNote(req, false);
      });

      const authed = await fetchAuthConfig();
      if (authed) {
        await fetchNotesList();
        await ai.fetchAiConfig();
      }

      heartbeatTimer = setInterval(() => checkConnection(true), 4000);
      theme.initTheme();
      setInterval(() => theme.fetchTheme(), 10000);
    });

    // ─── Return All Template Bindings ──────────────────────────────────

    return {
      // Theme
      themePreference: theme.themePreference,
      effectiveTheme: theme.effectiveTheme,
      osTheme: theme.osTheme,
      setThemePreference: theme.setThemePreference,
      // Auth
      isAuthenticated, authUser, authError, authStatus,
      authVerificationCode, authChallengeId, authCanRetry,
      sessionRemainingText, sessionRemainingFullText,
      fetchAuthConfig, logout, startPhoneAuth,
      // Health
      isPhoneReachable, isCheckingConnection, connectionError, checkConnection,
      // Notes
      currentFilename, notesList, rawContent, viewMode,
      isSaving, saveStatusText, saveStatusClass, showExportMenu, showAccountMenu,
      computedNewFilename, renderedHtml, editorTextarea,
      // Presentation
      previousViewMode: presentation.previousViewMode,
      slides: presentation.slides,
      currentSlideIndex: presentation.currentSlideIndex,
      showSlideOverview: presentation.showSlideOverview,
      isPresentationFullscreen: presentation.isPresentationFullscreen,
      presentationStageRef: presentation.presentationStageRef,
      currentSlideHtml: presentation.currentSlideHtml,
      prepareSlides: presentation.prepareSlides,
      nextSlide: presentation.nextSlide,
      prevSlide: presentation.prevSlide,
      goToSlide: presentation.goToSlide,
      enterPresentationMode: presentation.enterPresentationMode,
      exitPresentationMode: presentation.exitPresentationMode,
      togglePresentationFullscreen: presentation.togglePresentationFullscreen,
      handleTouchStart: presentation.handleTouchStart,
      handleTouchEnd: presentation.handleTouchEnd,
      // Link Modal
      openLinkModal: linkModal.openLinkModal,
      linkSearchQuery: linkModal.linkSearchQuery,
      selectedLinkFilename: linkModal.selectedLinkFilename,
      selectedLinkTitle: linkModal.selectedLinkTitle,
      linkDisplayText: linkModal.linkDisplayText,
      linkFocusedIndex: linkModal.linkFocusedIndex,
      linkEditorContext: linkModal.linkEditorContext,
      linkSearchInputRef: linkModal.linkSearchInputRef,
      linkPagesListRef: linkModal.linkPagesListRef,
      isExternalUrl: linkModal.isExternalUrl,
      computedCustomFilename: linkModal.computedCustomFilename,
      filteredLinkPages: linkModal.filteredLinkPages,
      isExactMatch: linkModal.isExactMatch,
      formattedLinkPreview: linkModal.formattedLinkPreview,
      // AI
      openAiDrawer: ai.openAiDrawer,
      showAiSettings: ai.showAiSettings,
      isAiBusy: ai.isAiBusy,
      isAiStreaming: ai.isAiStreaming,
      streamingText: ai.streamingText,
      aiPromptInput: ai.aiPromptInput,
      aiError: ai.aiError,
      messages: ai.messages,
      pendingAction: ai.pendingAction,
      canUndo: ai.canUndo,
      availableModels: ai.availableModels,
      isLoadingModels: ai.isLoadingModels,
      modelsError: ai.modelsError,
      isCurrentModelInList: ai.isCurrentModelInList,
      aiConfig: ai.aiConfig,
      aiTab: ai.aiTab,
      chatMessagesContainer: ai.chatMessagesContainer,
      fetchAvailableModels: ai.fetchAvailableModels,
      onProviderChange: ai.onProviderChange,
      onModelSelect: ai.onModelSelect,
      saveAiSettings: ai.saveAiSettings,
      toggleAiDrawer: ai.toggleAiDrawer,
      toggleAiDrawerImport: ai.toggleAiDrawerImport,
      sendUserPrompt: ai.sendUserPrompt,
      triggerTemplate: ai.triggerTemplate,
      confirmAction: ai.confirmAction,
      undoLastAiAction: ai.undoLastAiAction,
      // Import
      importSourceText: imp.importSourceText,
      importTitle: imp.importTitle,
      importMode: imp.importMode,
      importCustomInstruction: imp.importCustomInstruction,
      importUrl: imp.importUrl,
      showUrlInput: imp.showUrlInput,
      isFetchingUrl: imp.isFetchingUrl,
      urlFetchError: imp.urlFetchError,
      showFileInput: imp.showFileInput,
      importFilePath: imp.importFilePath,
      isLoadingFile: imp.isLoadingFile,
      fileFetchError: imp.fileFetchError,
      fileInputRef: imp.fileInputRef,
      isDraggingFile: imp.isDraggingFile,
      toggleUrlInput: imp.toggleUrlInput,
      toggleFileInput: imp.toggleFileInput,
      triggerFilePicker: imp.triggerFilePicker,
      clearImportSource: imp.clearImportSource,
      fetchUrlContent: imp.fetchUrlContent,
      onFileSelect: imp.onFileSelect,
      onFileDrop: imp.onFileDrop,
      loadServerFile: imp.loadServerFile,
      pasteClipboard: imp.pasteClipboard,
      importText,
      openImportFromNewModal,
      // Block editor
      inPlaceBlocks, editingBlockIndex, activeBlockText,
      switchToInPlaceMode, editBlock, saveBlockEdit, cancelBlockEdit,
      insertBlockAfter, deleteBlock, addBlockAtEnd,
      // Toolbar
      insertPrefix, wrapSelection, insertInPlacePrefix, wrapInPlaceSelection,
      insertTab, insertTableTemplate, insertLink: openLinkDialog,
      // Modals
      openNewNoteModal, newNoteTitle, newNoteTemplate, createNote,
      // Actions
      onNoteSelect, saveCurrentNote, onContentChange,
      handlePreviewClick, handleLinkKeydown, confirmLinkInsert,
      selectLinkTarget, selectCustomLinkTarget,
      formatMessageContent,
    };
  }
}).mount('#app');
