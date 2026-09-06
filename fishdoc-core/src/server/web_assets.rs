//! Static Web Assets for Fishdoc Web Editor & AI Assistant (Vue 3 ESM, Asciidoctor.js, Minimal CSS).

pub const APP_JS: &str = r#"
import { createApp, ref, computed, watch, nextTick, onMounted } from 'vue';

// Initialize Asciidoctor compiler instance if loaded from CDN
let asciidoctorInstance = null;
try {
  if (typeof Asciidoctor !== 'undefined') {
    asciidoctorInstance = Asciidoctor();
  }
} catch (e) {
  console.warn('Asciidoctor.js initialization error, fallback to server rendering:', e);
}

// Markdown-like mini parser for AI chat responses
function formatMarkdown(text) {
  if (!text) return '';
  let escaped = text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
  
  // Code blocks
  escaped = escaped.replace(/```([a-zA-Z0-9_-]*)\n([\s\S]*?)```/g, '<pre><code class="lang-$1">$2</code></pre>');
  // Inline code
  escaped = escaped.replace(/`([^`]+)`/g, '<code>$1</code>');
  // Bold
  escaped = escaped.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  // Italic
  escaped = escaped.replace(/\*([^*]+)\*/g, '<em>$1</em>');
  // Line breaks
  escaped = escaped.replace(/\n/g, '<br>');
  return escaped;
}

createApp({
  setup() {
    // Note & View State
    const currentFilename = ref('welcome.adoc');
    const notesList = ref([]);
    const rawContent = ref('= Welcome to Fishdoc\n\nStart writing documentation in AsciiDoc.\n');
    const viewMode = ref('split'); // 'split' | 'inplace' | 'preview'
    const isSaving = ref(false);
    const saveStatusText = ref('Saved');
    const saveStatusClass = ref('saved');
    const showExportMenu = ref(false);

    // In-Place Block Editing State
    const inPlaceBlocks = ref([]);
    const editingBlockIndex = ref(-1);
    const activeBlockText = ref('');

    // Modal State
    const openNewNoteModal = ref(false);
    const newNoteTitle = ref('');
    const newNoteTemplate = ref('blank');

    // AI Assistant State
    const openAiDrawer = ref(false);
    const showAiSettings = ref(false);
    const isAiBusy = ref(false);
    const isAiStreaming = ref(false);
    const streamingText = ref('');
    const aiPromptInput = ref('');
    const aiError = ref('');
    const messages = ref([]);
    const pendingAction = ref(null);
    const canUndo = ref(false);

    const aiConfig = ref({
      provider: 'ollama',
      endpoint: '',
      model: 'llama3.2',
      apiKey: '',
      timeout: 90
    });

    // Refs
    const editorTextarea = ref(null);
    const chatMessagesContainer = ref(null);

    // Compute preview HTML via native Rust /api/render
    const renderedHtml = ref('');
    let renderTimer = null;

    async function updateRenderedHtml(text) {
      if (text === undefined || text === null) return;
      try {
        const res = await fetch('/api/render', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ content: text, full: false })
        });
        if (res.ok) {
          renderedHtml.value = await res.text();
          nextTick(() => {
            setupInteractiveFeatures();
          });
        }
      } catch (e) {
        console.error('Render error:', e);
      }
    }

    watch(rawContent, (newVal) => {
      clearTimeout(renderTimer);
      renderTimer = setTimeout(() => {
        updateRenderedHtml(newVal);
      }, 80);
    }, { immediate: true });

    // Computed filename for new note modal
    const computedNewFilename = computed(() => {
      const slug = newNoteTitle.value
        .trim()
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '-')
        .replace(/^-+|-+$/g, '');
      return (slug || 'untitled') + '.adoc';
    });

    async function loadInPlaceBlocks(text) {
      try {
        const res = await fetch('/api/blocks/parse', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ content: text !== undefined ? text : rawContent.value })
        });
        if (res.ok) {
          const data = await res.json();
          inPlaceBlocks.value = data.blocks || [];
          nextTick(() => {
            setupInteractiveFeatures();
          });
          return;
        }
      } catch (e) {
        console.error('Failed to parse blocks from server:', e);
      }
      // Fallback
      const raw = parseBlocksFromText(text !== undefined ? text : rawContent.value);
      inPlaceBlocks.value = raw.map((r, i) => ({ index: i, raw: r, html: r }));
    }

    // Split text into discrete AsciiDoc blocks for in-place editor fallback
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

      if (currentAcc) {
        blocks.push(currentAcc);
      }

      return blocks.length > 0 ? blocks : [text];
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

    function handlePreviewClick(e) {
      // 1. Checklist checkbox click
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

      // 2. Cross-reference or anchor click
      const link = e.target.closest('a');
      if (link) {
        const href = link.getAttribute('href') || '';
        if (href.startsWith('#')) {
          const targetEl = document.getElementById(href.slice(1));
          if (targetEl) {
            e.preventDefault();
            targetEl.scrollIntoView({ behavior: 'smooth' });
          }
        } else if (href.endsWith('.adoc') || href.endsWith('.html')) {
          e.preventDefault();
          const targetNote = href.replace(/\.html$/, '.adoc');
          loadNote(targetNote);
        }
      }
    }

    async function toggleChecklistItem(itemIdx, targetChecked) {
      try {
        const res = await fetch(`/api/notes/${currentFilename.value}/toggle`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ item_index: itemIdx, checked: targetChecked })
        });
        if (res.ok) {
          const data = await res.json();
          if (data.content) {
            rawContent.value = data.content;
            if (data.html) renderedHtml.value = data.html;
            if (viewMode.value === 'inplace') {
              await loadInPlaceBlocks(data.content);
            }
          }
        }
      } catch (e) {
        console.error('Failed to toggle checklist item:', e);
      }
    }

    // Load Note List
    async function fetchNotesList() {
      try {
        const res = await fetch('/api/notes');
        if (res.ok) {
          const list = await res.json();
          notesList.value = list;
          if (list.length > 0 && !list.find(n => n.filename === currentFilename.value)) {
            loadNote(list[0].filename);
          }
        }
      } catch (err) {
        console.error('Failed to fetch notes list:', err);
      }
    }

    // Load Single Note
    async function loadNote(filename) {
      if (!filename) return;
      currentFilename.value = filename;
      try {
        const res = await fetch(`/api/notes/${filename}`);
        if (res.ok) {
          rawContent.value = await res.text();
          saveStatusText.value = 'Saved';
          saveStatusClass.value = 'saved';
          if (viewMode.value === 'inplace') {
            inPlaceBlocks.value = parseBlocksFromText(rawContent.value);
            editingBlockIndex.value = -1;
          }
        }
      } catch (err) {
        console.error(`Failed to load note ${filename}:`, err);
      }
    }

    function onNoteSelect() {
      loadNote(currentFilename.value);
    }

    // Save Note to Server
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
          saveStatusText.value = 'Saved';
          saveStatusClass.value = 'saved';
          // Refresh list snippets
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

    // Create New Note
    async function createNote() {
      const title = newNoteTitle.value.trim();
      if (!title) return;

      let starterContent = `= ${title}\n\n`;
      if (newNoteTemplate.value === 'technical') {
        starterContent = `= ${title}\n:toc: left\n:icons: font\n\n== Overview\nDescribe system architecture and design.\n\n== Requirements\n* [ ] Core functionality\n* [ ] Performance goals\n\n[source,rust]\n----\nfn main() {\n    println!("Hello Fishdoc!");\n}\n----\n`;
      } else if (newNoteTemplate.value === 'meeting') {
        starterContent = `= Meeting: ${title}\n:icons: font\n\nDate: ${new Date().toISOString().slice(0, 10)}\nAttendees: User\n\n== Agenda\n. Topic 1\n. Topic 2\n\n== Action Items\n* [ ] Task 1\n* [ ] Task 2\n`;
      } else if (newNoteTemplate.value === 'journal') {
        starterContent = `= Journal: ${title}\n:icons: font\n\n== ${new Date().toLocaleDateString()}\n\nWrite your thoughts here...\n`;
      }

      try {
        const res = await fetch('/api/notes', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            title: title,
            content: starterContent
          })
        });

        if (res.ok) {
          const data = await res.json();
          openNewNoteModal.value = false;
          newNoteTitle.value = '';
          await fetchNotesList();
          loadNote(data.filename);
        }
      } catch (err) {
        alert('Failed to create note: ' + err.message);
      }
    }

    // AI Configuration & State
    async function fetchAiConfig() {
      try {
        const res = await fetch('/api/ai/config');
        if (res.ok) {
          const data = await res.json();
          aiConfig.value.provider = data.provider || 'ollama';
          aiConfig.value.endpoint = data.endpoint || (data.provider === 'openai' ? 'https://api.mimocode.com' : 'http://192.168.1.1:11434');
          aiConfig.value.model = data.model || 'llama3.2';
          aiConfig.value.timeout = data.timeout || 90;
          canUndo.value = !!data.can_undo;
        }
      } catch (err) {
        console.warn('Could not load AI config:', err);
      }
    }

    async function saveAiSettings() {
      try {
        await fetch('/api/ai/config', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            provider: aiConfig.value.provider,
            endpoint: aiConfig.value.endpoint,
            model: aiConfig.value.model,
            api_key: aiConfig.value.apiKey,
            timeout: Number(aiConfig.value.timeout) || 90
          })
        });
        showAiSettings.value = false;
      } catch (err) {
        alert('Failed to save AI config: ' + err.message);
      }
    }

    function toggleAiDrawer() {
      openAiDrawer.value = !openAiDrawer.value;
      if (openAiDrawer.value) {
        scrollChatToBottom();
      }
    }

    // AI Chat & Execution via SSE Stream
    async function sendUserPrompt(customInstruction = null) {
      const prompt = customInstruction || aiPromptInput.value.trim();
      if (!prompt || isAiBusy.value) return;

      messages.value.push({ role: 'user', content: prompt });
      if (!customInstruction) aiPromptInput.value = '';
      aiError.value = '';
      isAiBusy.value = true;
      isAiStreaming.value = true;
      streamingText.value = '';
      scrollChatToBottom();

      try {
        const response = await fetch('/api/ai/chat', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            prompt: prompt,
            context_filename: currentFilename.value,
            context_content: rawContent.value
          })
        });

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${await response.text()}`);
        }

        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        let fullAnswer = '';

        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          const chunk = decoder.decode(value, { stream: true });

          for (const line of chunk.split('\n')) {
            const trimmed = line.trim();
            if (trimmed.startsWith('data: ') && trimmed !== 'data: [DONE]') {
              try {
                const payload = JSON.parse(trimmed.slice(6));
                if (payload.type === 'token') {
                  streamingText.value += payload.text;
                  fullAnswer += payload.text;
                  scrollChatToBottom();
                } else if (payload.type === 'pending_confirmation') {
                  pendingAction.value = payload.action;
                  openAiDrawer.value = true;
                } else if (payload.type === 'finished') {
                  if (payload.content) fullAnswer = payload.content;
                  if (payload.can_undo !== undefined) canUndo.value = payload.can_undo;
                  // If note was updated or created, reload note
                  if (payload.last_snapshot_id || payload.last_created_note) {
                    await fetchNotesList();
                    await loadNote(currentFilename.value);
                  }
                } else if (payload.type === 'error') {
                  aiError.value = payload.error;
                }
              } catch (_) {}
            }
          }
        }

        if (fullAnswer.trim()) {
          messages.value.push({ role: 'assistant', content: fullAnswer });
        }
      } catch (err) {
        aiError.value = `Server AI Error: ${err.message}`;
      } finally {
        isAiBusy.value = false;
        isAiStreaming.value = false;
        streamingText.value = '';
        scrollChatToBottom();
      }
    }

    // Trigger Quick AI Templates
    async function triggerTemplate(templateId) {
      if (isAiBusy.value) return;
      openAiDrawer.value = true;

      const templatePrompts = {
        summarize: 'Please provide a clear, structured summary of this AsciiDoc document.',
        fix_grammar: 'Review this AsciiDoc document, fixing all grammar and spelling errors while preserving formatting, headings, and structure.',
        add_admonition: 'Analyze this document and add relevant AsciiDoc [NOTE], [TIP], or [WARNING] blocks to highlight key takeaways.',
        format_table: 'Convert the main data or list points in this document into a well-structured AsciiDoc table |=== ... |===',
        continue_writing: 'Continue writing the next logical section of this AsciiDoc document.'
      };

      const prompt = templatePrompts[templateId] || `Run template ${templateId} on this note.`;
      await sendUserPrompt(prompt);
    }

    // Confirm or Deny Pending Tool Call
    async function confirmAction(approved) {
      if (isAiBusy.value) return;
      isAiBusy.value = true;
      isAiStreaming.value = true;
      streamingText.value = '';
      pendingAction.value = null;

      try {
        const response = await fetch('/api/ai/confirm', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ approved })
        });

        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        let fullAnswer = '';

        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          const chunk = decoder.decode(value, { stream: true });

          for (const line of chunk.split('\n')) {
            const trimmed = line.trim();
            if (trimmed.startsWith('data: ') && trimmed !== 'data: [DONE]') {
              try {
                const payload = JSON.parse(trimmed.slice(6));
                if (payload.type === 'token') {
                  streamingText.value += payload.text;
                  fullAnswer += payload.text;
                  scrollChatToBottom();
                } else if (payload.type === 'finished') {
                  if (payload.content) fullAnswer = payload.content;
                  if (payload.can_undo !== undefined) canUndo.value = payload.can_undo;
                  await fetchNotesList();
                  await loadNote(currentFilename.value);
                } else if (payload.type === 'error') {
                  aiError.value = payload.error;
                }
              } catch (_) {}
            }
          }
        }

        if (fullAnswer.trim()) {
          messages.value.push({ role: 'assistant', content: fullAnswer });
        }
      } catch (err) {
        aiError.value = `Failed to confirm action: ${err.message}`;
      } finally {
        isAiBusy.value = false;
        isAiStreaming.value = false;
        streamingText.value = '';
        scrollChatToBottom();
      }
    }

    // Undo AI Action
    async function undoLastAiAction() {
      try {
        const res = await fetch('/api/ai/undo', { method: 'POST' });
        const data = await res.json();
        if (data.ok) {
          canUndo.value = !!data.can_undo;
          await loadNote(currentFilename.value);
          messages.value.push({ role: 'assistant', content: `↩ ${data.message}` });
        } else {
          alert('Undo failed: ' + (data.error || 'Unknown error'));
        }
      } catch (err) {
        alert('Undo error: ' + err.message);
      }
    }

    // Formatting Toolbar Helpers
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
      nextTick(() => {
        el.setSelectionRange(s + 2, s + 2);
      });
    }

    function insertTableTemplate() {
      const tableSnippet = '\n|===\n| Header 1 | Header 2 | Header 3\n\n| Row 1 Col 1 | Row 1 Col 2 | Row 1 Col 3\n| Row 2 Col 1 | Row 2 Col 2 | Row 2 Col 3\n|===\n';
      const el = editorTextarea.value;
      if (!el) return;
      const s = el.selectionStart;
      rawContent.value = rawContent.value.slice(0, s) + tableSnippet + rawContent.value.slice(s);
      onContentChange();
    }

    function insertLink() {
      const url = prompt('Enter URL or note link:', 'https://example.com');
      const text = prompt('Enter link title:', 'Link Description');
      if (url && text) {
        wrapSelection(`${url}[${text}]`, '');
      }
    }

    function scrollChatToBottom() {
      nextTick(() => {
        if (chatMessagesContainer.value) {
          chatMessagesContainer.value.scrollTop = chatMessagesContainer.value.scrollHeight;
        }
      });
    }

    function formatMessageContent(content) {
      return formatMarkdown(content);
    }

    // Keyboard Shortcuts
    function handleGlobalKeyDown(e) {
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        saveCurrentNote();
      }
    }

    onMounted(async () => {
      window.addEventListener('keydown', handleGlobalKeyDown);
      await fetchNotesList();
      await fetchAiConfig();

      // Check if URL has note parameter
      const pathParts = window.location.pathname.split('/');
      if (pathParts[1] === 'page' || pathParts[1] === 'notes' || pathParts[1] === 'edit') {
        const targetNote = pathParts[2];
        if (targetNote) {
          loadNote(targetNote.endsWith('.adoc') ? targetNote : targetNote + '.adoc');
        }
      }
    });

    return {
      currentFilename,
      notesList,
      rawContent,
      viewMode,
      isSaving,
      saveStatusText,
      saveStatusClass,
      showExportMenu,
      inPlaceBlocks,
      editingBlockIndex,
      activeBlockText,
      openNewNoteModal,
      newNoteTitle,
      newNoteTemplate,
      computedNewFilename,
      openAiDrawer,
      showAiSettings,
      isAiBusy,
      isAiStreaming,
      streamingText,
      aiPromptInput,
      aiError,
      messages,
      pendingAction,
      canUndo,
      aiConfig,
      editorTextarea,
      chatMessagesContainer,
      renderedHtml,
      switchToInPlaceMode,
      editBlock,
      saveBlockEdit,
      cancelBlockEdit,
      insertBlockAfter,
      deleteBlock,
      addBlockAtEnd,
      onNoteSelect,
      saveCurrentNote,
      onContentChange,
      createNote,
      saveAiSettings,
      toggleAiDrawer,
      sendUserPrompt,
      triggerTemplate,
      confirmAction,
      undoLastAiAction,
      insertPrefix,
      wrapSelection,
      insertInPlacePrefix,
      wrapInPlaceSelection,
      insertTab,
      insertTableTemplate,
      insertLink,
      formatMessageContent,
      handlePreviewClick,
      toggleChecklistItem
    };
  }
}).mount('#app');
"#;

pub const STYLE_CSS: &str = r#"
:root {
  --bg-main: #f8fafc;
  --bg-card: #ffffff;
  --bg-pane: #ffffff;
  --border-color: #e2e8f0;
  --text-main: #1e293b;
  --text-muted: #64748b;
  --primary: #2563eb;
  --primary-hover: #1d4ed8;
  --accent: #0284c7;
  --accent-light: #e0f2fe;
  --success: #16a34a;
  --success-bg: #dcfce7;
  --danger: #dc2626;
  --danger-bg: #fee2e2;
  --warning: #d97706;
  --warning-bg: #fef3c7;
  --shadow-sm: 0 1px 3px rgba(0,0,0,0.06);
  --shadow-md: 0 4px 6px -1px rgba(0,0,0,0.08), 0 2px 4px -2px rgba(0,0,0,0.05);
  --shadow-lg: 0 10px 15px -3px rgba(0,0,0,0.1), 0 4px 6px -4px rgba(0,0,0,0.05);
  --radius-sm: 6px;
  --radius-md: 8px;
  --radius-lg: 12px;
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg-main: #0f172a;
    --bg-card: #1e293b;
    --bg-pane: #1e293b;
    --border-color: #334155;
    --text-main: #f1f5f9;
    --text-muted: #94a3b8;
    --primary: #3b82f6;
    --primary-hover: #60a5fa;
    --accent-light: #1e3a8a;
    --success-bg: #064e3b;
    --danger-bg: #7f1d1d;
    --warning-bg: #78350f;
  }
}

* { box-sizing: border-box; }

html, body {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  background-color: var(--bg-main);
  color: var(--text-main);
}

#app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  height: 100dvh;
  width: 100vw;
  max-width: 100%;
  overflow: hidden;
  position: relative;
}

[v-cloak] { display: none; }

/* Navigation Bar */
.navbar {
  height: 56px;
  background-color: var(--bg-card);
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  z-index: 100;
  flex-shrink: 0;
}

.nav-left, .nav-center, .nav-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.brand {
  display: flex;
  align-items: center;
  gap: 6px;
  text-decoration: none;
  color: var(--text-main);
  font-size: 1.15rem;
  font-weight: 700;
}

.brand .logo { font-size: 1.3rem; }
.brand .badge {
  font-size: 0.72rem;
  background-color: var(--primary);
  color: #fff;
  padding: 2px 6px;
  border-radius: var(--radius-sm);
}

.note-selector {
  display: flex;
  align-items: center;
  gap: 6px;
}

.note-selector select {
  padding: 6px 10px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  background-color: var(--bg-main);
  color: var(--text-main);
  font-size: 0.88rem;
  font-weight: 600;
  max-width: 240px;
  outline: none;
}

button {
  cursor: pointer;
  border: 1px solid var(--border-color);
  background-color: var(--bg-card);
  color: var(--text-main);
  padding: 6px 12px;
  border-radius: var(--radius-sm);
  font-size: 0.85rem;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  transition: all 0.15s ease;
}

button:hover:not(:disabled) {
  border-color: var(--primary);
  color: var(--primary);
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-new {
  background-color: var(--accent-light);
  color: var(--primary);
  border-color: transparent;
}

.btn-primary, .btn-small-primary {
  background-color: var(--primary);
  color: #ffffff;
  border-color: var(--primary);
}
.btn-primary:hover:not(:disabled), .btn-small-primary:hover:not(:disabled) {
  background-color: var(--primary-hover);
  color: #ffffff;
}

.btn-save {
  background-color: var(--primary);
  color: #ffffff;
  border-color: var(--primary);
}
.btn-save:hover:not(:disabled) {
  background-color: var(--primary-hover);
}

.save-indicator {
  font-size: 0.8rem;
  font-weight: 500;
}
.save-indicator.saved { color: var(--success); }
.save-indicator.saving { color: var(--warning); }
.save-indicator.unsaved { color: var(--text-muted); }

.view-mode-tabs {
  display: flex;
  background-color: var(--bg-main);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  padding: 2px;
  gap: 2px;
}

.view-mode-tabs button {
  border: none;
  background: transparent;
  padding: 4px 10px;
  font-size: 0.82rem;
  border-radius: 4px;
}

.view-mode-tabs button.active {
  background-color: var(--bg-card);
  color: var(--primary);
  box-shadow: var(--shadow-sm);
}

.btn-ai {
  background: linear-gradient(135deg, #4f46e5 0%, #7c3aed 100%);
  color: #ffffff;
  border: none;
  position: relative;
}
.btn-ai:hover { opacity: 0.95; color: #ffffff; }
.btn-ai .sparkle { font-size: 1rem; }
.pulse-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: #ef4444;
  position: absolute;
  top: 4px;
  right: 4px;
  animation: pulse 1.5s infinite;
}

@keyframes pulse {
  0% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(239, 68, 68, 0.7); }
  70% { transform: scale(1); box-shadow: 0 0 0 6px rgba(239, 68, 68, 0); }
  100% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(239, 68, 68, 0); }
}

.export-dropdown { position: relative; }
.dropdown-menu {
  position: absolute;
  right: 0;
  top: calc(100% + 4px);
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-md);
  min-width: 160px;
  display: flex;
  flex-direction: column;
  z-index: 200;
}
.dropdown-menu a {
  padding: 8px 14px;
  text-decoration: none;
  color: var(--text-main);
  font-size: 0.85rem;
}
.dropdown-menu a:hover {
  background-color: var(--bg-main);
  color: var(--primary);
}

/* Main Workspace */
.workspace {
  display: flex;
  flex: 1;
  min-height: 0;
  min-width: 0;
  height: calc(100vh - 56px);
  height: calc(100dvh - 56px);
  overflow: hidden;
  position: relative;
}

/* Split Mode */
.workspace.split .editor-pane {
  flex: 1;
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  height: 100%;
  overflow: hidden;
}

.workspace.split .preview-pane {
  flex: 1;
  min-width: 0;
  min-height: 0;
  height: 100%;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 28px 40px;
  background-color: var(--bg-card);
  -webkit-overflow-scrolling: touch;
}

.editor-toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 12px;
  background-color: var(--bg-main);
  border-bottom: 1px solid var(--border-color);
  flex-wrap: wrap;
  flex-shrink: 0;
}

.toolbar-group { display: flex; gap: 4px; }
.toolbar-divider { width: 1px; height: 18px; background-color: var(--border-color); margin: 0 4px; }

.editor-toolbar button {
  padding: 3px 8px;
  font-size: 0.78rem;
  background-color: var(--bg-card);
}

.editor-pane textarea {
  flex: 1;
  min-height: 0;
  height: 100%;
  border: none;
  padding: 16px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 14px;
  line-height: 1.55;
  background-color: var(--bg-pane);
  color: var(--text-main);
  resize: none;
  outline: none;
  overflow-y: auto;
}

/* In-Place Block Mode */
.inplace-container {
  flex: 1;
  min-height: 0;
  height: 100%;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 24px 32px;
  max-width: 900px;
  width: 100%;
  margin: 0 auto;
  -webkit-overflow-scrolling: touch;
}

.inplace-header { margin-bottom: 20px; }
.inplace-header h3 { margin: 0 0 4px 0; font-size: 1.3rem; }
.inplace-header .subtitle { margin: 0; color: var(--text-muted); font-size: 0.88rem; }

.blocks-list { display: flex; flex-direction: column; gap: 14px; }

.inplace-block-wrapper {
  position: relative;
  border-radius: var(--radius-md);
  transition: all 0.15s ease;
}

.inplace-rendered-card {
  position: relative;
  padding: 14px 18px;
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-sm);
}
.inplace-rendered-card:hover {
  border-color: var(--primary);
}

.block-actions {
  position: absolute;
  top: 8px;
  right: 8px;
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.15s ease;
  z-index: 10;
}
.inplace-rendered-card:hover .block-actions { opacity: 1; }

.block-actions button {
  padding: 2px 8px;
  font-size: 0.72rem;
  background-color: var(--bg-card);
  box-shadow: var(--shadow-sm);
}

.inplace-editor-card {
  background-color: var(--bg-card);
  border: 2px solid var(--primary);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
  overflow: hidden;
}

.inplace-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 10px;
  background-color: var(--bg-main);
  border-bottom: 1px solid var(--border-color);
}
.inplace-toolbar-left, .inplace-toolbar-right { display: flex; gap: 4px; }

.inplace-toolbar button { padding: 3px 8px; font-size: 0.75rem; }
.btn-block-save { background-color: var(--success); color: #fff; border-color: var(--success); }
.btn-block-cancel { background-color: var(--danger); color: #fff; border-color: var(--danger); }

.inplace-editor-card textarea {
  width: 100%;
  border: none;
  padding: 12px 16px;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 14px;
  line-height: 1.5;
  background-color: var(--bg-pane);
  color: var(--text-main);
  outline: none;
  resize: vertical;
}

.add-bottom-block { text-align: center; margin: 24px 0 48px 0; }
.btn-add-block-large {
  padding: 10px 20px;
  font-size: 0.92rem;
  background-color: var(--accent-light);
  color: var(--primary);
  border: 1px dashed var(--primary);
}

/* Full Preview Mode */
.full-preview-pane {
  flex: 1;
  min-height: 0;
  height: 100%;
  overflow-y: auto;
  overflow-x: hidden;
  background-color: var(--bg-card);
  padding: 32px;
  -webkit-overflow-scrolling: touch;
}
.preview-container {
  max-width: 860px;
  margin: 0 auto;
}

/* AI Assistant Drawer */
.ai-drawer {
  position: absolute;
  top: 56px;
  right: 0;
  bottom: 0;
  width: 440px;
  max-width: 90vw;
  height: calc(100vh - 56px);
  height: calc(100dvh - 56px);
  min-height: 0;
  overflow: hidden;
  background-color: var(--bg-card);
  border-left: 1px solid var(--border-color);
  box-shadow: -5px 0 25px rgba(0,0,0,0.08);
  display: flex;
  flex-direction: column;
  z-index: 500;
  transform: translateX(100%);
  transition: transform 0.25s cubic-bezier(0.16, 1, 0.3, 1);
}

.ai-drawer.is-open { transform: translateX(0); }

.ai-drawer-header {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color);
  display: flex;
  justify-content: space-between;
  align-items: center;
  background-color: var(--bg-main);
}
.ai-title { display: flex; align-items: center; gap: 6px; font-size: 1rem; }
.ai-backend-badge {
  font-size: 0.7rem;
  font-weight: 500;
  background-color: var(--accent-light);
  color: var(--primary);
  padding: 2px 6px;
  border-radius: var(--radius-sm);
}

.ai-settings-panel {
  padding: 14px;
  background-color: var(--bg-main);
  border-bottom: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  gap: 8px;
  font-size: 0.82rem;
  max-height: 45vh;
  overflow-y: auto;
  flex-shrink: 0;
}
.ai-settings-panel h4 { margin: 0 0 4px 0; font-size: 0.9rem; }
.ai-settings-panel label { display: flex; flex-direction: column; gap: 4px; }
.ai-settings-panel input, .ai-settings-panel select {
  padding: 6px 8px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  background-color: var(--bg-card);
  color: var(--text-main);
  font-size: 0.82rem;
}
.settings-actions { display: flex; gap: 8px; margin-top: 4px; }

.ai-quick-templates {
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-color);
  display: flex;
  gap: 6px;
  overflow-x: auto;
  white-space: nowrap;
}
.ai-quick-templates button {
  padding: 4px 8px;
  font-size: 0.74rem;
  background-color: var(--bg-main);
  border-radius: 12px;
}

.ai-undo-banner {
  padding: 8px 12px;
  background-color: var(--success-bg);
  color: var(--success);
  font-size: 0.8rem;
  font-weight: 600;
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.btn-undo-link {
  background: transparent;
  border: none;
  color: var(--success);
  text-decoration: underline;
  padding: 0;
  font-size: 0.8rem;
}

/* Diff Confirmation Card */
.diff-confirmation-card {
  margin: 10px 12px;
  padding: 12px;
  background-color: var(--warning-bg);
  border: 1px solid var(--warning);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
}
.diff-card-header { display: flex; justify-content: space-between; font-size: 0.85rem; margin-bottom: 4px; }
.diff-reason { font-size: 0.82rem; margin: 4px 0 8px 0; color: var(--text-main); }
.diff-view-box {
  max-height: 160px;
  overflow-y: auto;
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 4px;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 11px;
  line-height: 1.4;
  padding: 6px;
  margin-bottom: 8px;
}
.diff-line { display: flex; white-space: pre-wrap; word-break: break-all; }
.diff-line.diff-added { background-color: #ecfdf5; color: #047857; }
.diff-line.diff-removed { background-color: #fef2f2; color: #b91c1c; }
.diff-prefix { width: 16px; user-select: none; font-weight: bold; }
.diff-card-actions { display: flex; gap: 8px; }
.btn-diff-approve { background-color: var(--success); color: #fff; border-color: var(--success); flex: 1; }
.btn-diff-deny { background-color: var(--danger); color: #fff; border-color: var(--danger); flex: 1; }

.ai-chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.chat-welcome {
  background-color: var(--bg-main);
  padding: 14px;
  border-radius: var(--radius-md);
  font-size: 0.85rem;
  color: var(--text-muted);
  line-height: 1.45;
}
.chat-welcome p { margin: 0 0 6px 0; }

.chat-bubble {
  padding: 10px 14px;
  border-radius: var(--radius-md);
  font-size: 0.88rem;
  line-height: 1.5;
  max-width: 92%;
}
.chat-bubble.user {
  background-color: var(--accent-light);
  color: var(--text-main);
  align-self: flex-end;
  border-bottom-right-radius: 2px;
}
.chat-bubble.assistant {
  background-color: var(--bg-main);
  border: 1px solid var(--border-color);
  align-self: flex-start;
  border-bottom-left-radius: 2px;
}
.chat-bubble-header { font-size: 0.74rem; color: var(--text-muted); margin-bottom: 4px; }
.chat-bubble-body pre {
  background-color: #1e293b;
  color: #f8fafc;
  padding: 8px;
  border-radius: 4px;
  overflow-x: auto;
  font-size: 12px;
}
.chat-bubble-body code {
  background-color: rgba(0,0,0,0.06);
  padding: 2px 4px;
  border-radius: 3px;
  font-size: 0.84em;
}

.typing-indicator { font-style: italic; color: var(--primary); font-size: 0.78rem; margin-left: 6px; }
.chat-error-alert {
  padding: 8px 12px;
  background-color: var(--danger-bg);
  color: var(--danger);
  border-radius: var(--radius-sm);
  font-size: 0.82rem;
}

.ai-input-container {
  padding: 10px 14px;
  border-top: 1px solid var(--border-color);
  background-color: var(--bg-main);
  display: flex;
  gap: 8px;
  align-items: flex-end;
}
.ai-input-container textarea {
  flex: 1;
  padding: 8px 10px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background-color: var(--bg-card);
  color: var(--text-main);
  font-size: 0.88rem;
  resize: none;
  outline: none;
  font-family: inherit;
}
.btn-send-ai {
  padding: 8px 14px;
  background-color: var(--primary);
  color: #fff;
  border-color: var(--primary);
  border-radius: var(--radius-md);
  font-size: 1rem;
}

/* Modal Overlay */
.modal-overlay {
  position: fixed;
  inset: 0;
  background-color: rgba(0,0,0,0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}
.modal-card {
  width: 440px;
  max-width: 90vw;
  background-color: var(--bg-card);
  border-radius: var(--radius-lg);
  padding: 24px;
  box-shadow: var(--shadow-lg);
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.modal-card h3 { margin: 0; font-size: 1.25rem; }
.modal-card label { display: flex; flex-direction: column; gap: 6px; font-size: 0.88rem; font-weight: 500; }
.modal-card input, .modal-card select {
  padding: 8px 10px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  background-color: var(--bg-main);
  color: var(--text-main);
  font-size: 0.9rem;
}
.modal-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 10px; }

/* Code copy button */
.copy-code-btn {
  position: absolute;
  top: 8px;
  right: 8px;
  background-color: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.2);
  color: #e2e8f0;
  border-radius: 4px;
  padding: 3px 8px;
  font-size: 0.74rem;
  cursor: pointer;
  opacity: 0.8;
  transition: opacity 0.2s, background-color 0.2s;
  z-index: 5;
}
.copy-code-btn:hover {
  opacity: 1;
  background-color: rgba(255, 255, 255, 0.25);
}

/* Admonition SVGs & Cards */
.admonitionblock {
  margin: 1.5em 0;
  padding: 14px 18px;
  border-radius: 8px;
  border-left: 5px solid;
  box-shadow: var(--shadow-sm);
}
.admonitionblock.note { background-color: var(--note-bg, #e0f2fe); border-color: var(--note-border, #0284c7); }
.admonitionblock.tip { background-color: var(--tip-bg, #f0fdf4); border-color: var(--tip-border, #16a34a); }
.admonitionblock.warning { background-color: var(--warning-bg, #fffbeb); border-color: var(--warning-border, #d97706); }
.admonitionblock.caution { background-color: var(--caution-bg, #fff1f2); border-color: var(--caution-border, #e11d48); }
.admonitionblock.important { background-color: #faf5ff; border-color: #9333ea; }
.admonition-header { display: flex; align-items: center; gap: 8px; font-weight: 700; margin-bottom: 8px; }
.admonition-icon { display: flex; align-items: center; }
.admonition-icon-svg { display: inline-block; vertical-align: middle; }

/* Table Frames & Grids */
table.table { width: 100%; border-collapse: collapse; margin: 1.5em 0; font-size: 0.95rem; }
table.table th, table.table td { padding: 10px 14px; border: 1px solid var(--border-color); text-align: left; vertical-align: top; }
table.table th { background-color: var(--bg-main); font-weight: 600; }
table.table tr:nth-child(even) td { background-color: rgba(0,0,0,0.02); }

table.frame-all { border: 1px solid var(--border-color); }
table.frame-topbot { border-top: 1px solid var(--border-color); border-bottom: 1px solid var(--border-color); border-left: none; border-right: none; }
table.frame-sides { border-left: 1px solid var(--border-color); border-right: 1px solid var(--border-color); border-top: none; border-bottom: none; }
table.frame-none { border: none; }

table.grid-all th, table.grid-all td { border: 1px solid var(--border-color); }
table.grid-rows th, table.grid-rows td { border-top: 1px solid var(--border-color); border-bottom: 1px solid var(--border-color); border-left: none; border-right: none; }
table.grid-cols th, table.grid-cols td { border-left: 1px solid var(--border-color); border-right: 1px solid var(--border-color); border-top: none; border-bottom: none; }
table.grid-none th, table.grid-none td { border: none; }

/* Table of Contents */
.toc { margin: 1.5em 0 2em 0; padding: 16px 20px; background-color: var(--bg-main); border: 1px solid var(--border-color); border-radius: 8px; }
.toc .toctitle { font-size: 1.1rem; font-weight: 700; color: var(--text-main); margin-bottom: 12px; }
.toc ul { margin: 0.3em 0; padding-left: 20px; list-style-type: none; }
.toc ul.sectlevel1 { padding-left: 4px; }
.toc li { margin: 0.3em 0; }
.toc li::before { content: "•"; color: var(--primary); display: inline-block; width: 1em; margin-left: -1em; }
.toc a { color: var(--primary); text-decoration: none; font-size: 0.95rem; }
.toc a:hover { text-decoration: underline; }

/* Inline macros */
.conum {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background-color: var(--primary);
  color: #ffffff;
  border-radius: 50%;
  width: 1.35em;
  height: 1.35em;
  font-size: 0.8em;
  font-weight: 700;
  vertical-align: 0.1em;
  margin: 0 3px;
}
.keyseq kbd {
  display: inline-block;
  padding: 2px 6px;
  font-size: 0.85em;
  font-family: ui-monospace, monospace;
  background: var(--bg-main);
  border: 1px solid var(--border-color);
  border-radius: 4px;
  box-shadow: 0 1px 0 var(--border-color);
}
.btn {
  display: inline-block;
  padding: 2px 8px;
  font-size: 0.85em;
  background: var(--bg-main);
  border: 1px solid var(--border-color);
  border-radius: 4px;
  font-weight: 500;
}
.menuseq { font-weight: 500; }
.checklist-checkbox { margin-right: 8px; cursor: pointer; }

/* Responsive adjustments */
@media (max-width: 768px) {
  .workspace.split { flex-direction: column; }
  .workspace.split .editor-pane { min-height: 40vh; border-right: none; border-bottom: 1px solid var(--border-color); }
  .ai-drawer { width: 100vw; max-width: 100vw; }
}
"#;

pub const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Fishdoc Web - AsciiDoc Editor & AI</title>
  <link rel="stylesheet" href="/style.css">
  
  <!-- Vue 3 ES Module Import Map -->
  <script type="importmap">
    {
      "imports": {
        "vue": "https://cdn.jsdelivr.net/npm/vue@3.4.21/dist/vue.esm-browser.prod.js"
      }
    }
  </script>
</head>
<body>
  <div id="app" v-cloak>
    <!-- Top Navigation Bar -->
    <header class="navbar">
      <div class="nav-left">
        <a href="/" class="brand">
          <span class="logo">🐟</span> <strong>Fishdoc</strong> <span class="badge">Web</span>
        </a>
        
        <div class="note-selector">
          <select v-model="currentFilename" @change="onNoteSelect">
            <option v-for="n in notesList" :key="n.filename" :value="n.filename">
              {{ n.title }} ({{ n.filename }})
            </option>
          </select>
          <button @click="openNewNoteModal = true" class="btn-new" title="Create New Note">+ New</button>
        </div>
      </div>

      <div class="nav-center">
        <div class="view-mode-tabs">
          <button :class="{ active: viewMode === 'split' }" @click="viewMode = 'split'" title="Side-by-side Editor & Preview">
            ✏️ Split
          </button>
          <button :class="{ active: viewMode === 'inplace' }" @click="switchToInPlaceMode" title="Block-by-Block In-Place Editing">
            🧱 Blocks
          </button>
          <button :class="{ active: viewMode === 'preview' }" @click="viewMode = 'preview'" title="Full Rendered Document Preview">
            👁️ Preview
          </button>
        </div>
      </div>

      <div class="nav-right">
        <span class="save-indicator" :class="saveStatusClass">
          {{ saveStatusText }}
        </span>
        <button @click="saveCurrentNote" class="btn-save" :disabled="isSaving" title="Save document (Ctrl+S)">
          {{ isSaving ? 'Saving...' : '💾 Save' }}
        </button>
        <div class="export-dropdown">
          <button class="btn-export" @click="showExportMenu = !showExportMenu">
            📥 Export ▾
          </button>
          <div v-if="showExportMenu" class="dropdown-menu">
            <a :href="'/export/' + currentFilename" target="_blank" @click="showExportMenu = false">Download HTML5</a>
            <a :href="'/raw/' + currentFilename" target="_blank" @click="showExportMenu = false">View Raw AsciiDoc</a>
          </div>
        </div>
        <button @click="toggleAiDrawer" class="btn-ai" :class="{ 'has-pending': pendingAction, 'is-open': openAiDrawer }">
          <span class="sparkle">✨</span> Assistant
          <span v-if="pendingAction" class="pulse-dot"></span>
        </button>
      </div>
    </header>

    <!-- Main Workspace -->
    <main class="workspace" :class="[viewMode, { 'drawer-open': openAiDrawer }]">
      
      <!-- MODE 1: Split View (Source on Left, Preview on Right) -->
      <template v-if="viewMode === 'split'">
        <section class="editor-pane">
          <div class="editor-toolbar">
            <div class="toolbar-group">
              <button @click="insertPrefix('= ')" title="Heading 1">H1</button>
              <button @click="insertPrefix('== ')" title="Heading 2">H2</button>
              <button @click="insertPrefix('=== ')" title="Heading 3">H3</button>
              <button @click="insertPrefix('==== ')" title="Heading 4">H4</button>
            </div>
            <div class="toolbar-divider"></div>
            <div class="toolbar-group">
              <button @click="insertPrefix('* ')" title="Bullet list item">• List</button>
              <button @click="insertPrefix('. ')" title="Numbered list item">1. Num</button>
              <button @click="insertPrefix('* [ ] ')" title="Checklist task item">☑ Task</button>
            </div>
            <div class="toolbar-divider"></div>
            <div class="toolbar-group">
              <button @click="wrapSelection('*', '*')" title="Bold text"><strong>B</strong></button>
              <button @click="wrapSelection('_', '_')" title="Italic text"><em>I</em></button>
              <button @click="wrapSelection('`', '`')" title="Monospace code"><code>C</code></button>
              <button @click="wrapSelection('[NOTE]\n====\n', '\n====\n')" title="Note Admonition">Note</button>
              <button @click="wrapSelection('[source,rust]\n----\n', '\n----\n')" title="Source Code block">Code</button>
              <button @click="insertTableTemplate" title="Insert AsciiDoc Table">Table</button>
              <button @click="insertLink" title="Insert Link / Cross reference">Link</button>
            </div>
          </div>
          <textarea 
            ref="editorTextarea" 
            v-model="rawContent" 
            @input="onContentChange"
            @keydown.tab.prevent="insertTab"
            placeholder="Write AsciiDoc content here..."
            spellcheck="false"
          ></textarea>
        </section>

        <section class="preview-pane" @click="handlePreviewClick">
          <article class="fishdoc-body" v-html="renderedHtml"></article>
        </section>
      </template>

      <!-- MODE 2: In-Place Block Editor (Fishdoc style) -->
      <template v-else-if="viewMode === 'inplace'">
        <section class="inplace-container" @click="handlePreviewClick">
          <div class="inplace-header">
            <h3>Block Editor: <code>{{ currentFilename }}</code></h3>
            <p class="subtitle">Click any block to edit in place. Changes sync directly with the AsciiDoc source.</p>
          </div>

          <div class="blocks-list">
            <div 
              v-for="(block, idx) in inPlaceBlocks" 
              :key="idx" 
              class="inplace-block-wrapper"
              :class="{ 'is-editing': editingBlockIndex === idx }"
            >
              <!-- Block Rendered Mode -->
              <div v-if="editingBlockIndex !== idx" class="inplace-rendered-card" @dblclick="editBlock(idx)">
                <div class="block-actions">
                  <button @click="editBlock(idx)" class="btn-block-edit" title="Edit this block">✏️ Edit</button>
                  <button @click="insertBlockAfter(idx)" class="btn-block-add" title="Insert block below">➕</button>
                  <button @click="deleteBlock(idx)" class="btn-block-del" title="Delete block">🗑️</button>
                </div>
                <div class="block-html fishdoc-body" v-html="typeof block === 'object' ? block.html : block"></div>
              </div>

              <!-- Block Active Edit Mode -->
              <div v-else class="inplace-editor-card">
                <div class="inplace-toolbar">
                  <div class="inplace-toolbar-left">
                    <button @click="insertInPlacePrefix('= ')" title="Heading 1">H1</button>
                    <button @click="insertInPlacePrefix('== ')" title="Heading 2">H2</button>
                    <button @click="insertInPlacePrefix('=== ')" title="Heading 3">H3</button>
                    <button @click="insertInPlacePrefix('* ')" title="Bullet">•</button>
                    <button @click="insertInPlacePrefix('. ')" title="Numbered">1.</button>
                    <button @click="insertInPlacePrefix('* [ ] ')" title="Task Checkbox">☑</button>
                    <button @click="wrapInPlaceSelection('[NOTE]\n====\n', '\n====')" title="Admonition">Note</button>
                    <button @click="wrapInPlaceSelection('[source,rust]\n----\n', '\n----')" title="Code">Code</button>
                  </div>
                  <div class="inplace-toolbar-right">
                    <button @click="saveBlockEdit(idx)" class="btn-block-save" title="Save block">✔ Done</button>
                    <button @click="cancelBlockEdit" class="btn-block-cancel" title="Cancel edit">✖ Cancel</button>
                  </div>
                </div>
                <textarea 
                  :ref="'blockEditor_' + idx"
                  v-model="activeBlockText" 
                  @keydown.esc="cancelBlockEdit"
                  @keydown.ctrl.enter="saveBlockEdit(idx)"
                  placeholder="Block content..."
                  rows="6"
                ></textarea>
              </div>
            </div>

            <!-- Add Block at Bottom -->
            <div class="add-bottom-block">
              <button @click="addBlockAtEnd" class="btn-add-block-large">➕ Add New Block</button>
            </div>
          </div>
        </section>
      </template>

      <!-- MODE 3: Full Document Preview -->
      <template v-else-if="viewMode === 'preview'">
        <section class="full-preview-pane" @click="handlePreviewClick">
          <div class="preview-container">
            <article class="fishdoc-body" v-html="renderedHtml"></article>
          </div>
        </section>
      </template>

    </main>

    <!-- AI Assistant Drawer -->
    <aside class="ai-drawer" :class="{ 'is-open': openAiDrawer }">
      <div class="ai-drawer-header">
        <div class="ai-title">
          <span class="sparkle">✨</span> <strong>Fishdoc AI</strong>
          <span class="ai-backend-badge">{{ aiConfig.provider }}: {{ aiConfig.model }}</span>
        </div>
        <div class="ai-header-actions">
          <button @click="showAiSettings = !showAiSettings" class="btn-icon" title="AI Settings">⚙️</button>
          <button @click="openAiDrawer = false" class="btn-icon" title="Close Drawer">&times;</button>
        </div>
      </div>

      <!-- Collapsible AI Settings -->
      <div v-if="showAiSettings" class="ai-settings-panel">
        <h4>AI Configuration (Backend Brokered)</h4>
        <label>Provider:
          <select v-model="aiConfig.provider">
            <option value="ollama">Ollama (Local / LAN)</option>
            <option value="openai">OpenAI / Compatible</option>
          </select>
        </label>
        <label>Endpoint URL:
          <input v-model="aiConfig.endpoint" placeholder="http://localhost:11434">
        </label>
        <label>Model Name:
          <input v-model="aiConfig.model" placeholder="llama3.2">
        </label>
        <label>API Key (Optional):
          <input v-model="aiConfig.apiKey" type="password" placeholder="Leave empty for local Ollama">
        </label>
        <div class="settings-actions">
          <button @click="saveAiSettings" class="btn-small-primary">Save Settings</button>
          <button @click="showAiSettings = false" class="btn-small">Close</button>
        </div>
      </div>

      <!-- Quick Template Action Chips -->
      <div class="ai-quick-templates">
        <button @click="triggerTemplate('summarize')" :disabled="isAiBusy" title="Summarize active document">
          📝 Summarize
        </button>
        <button @click="triggerTemplate('fix_grammar')" :disabled="isAiBusy" title="Fix grammar and polish AsciiDoc">
          ✨ Polish
        </button>
        <button @click="triggerTemplate('add_admonition')" :disabled="isAiBusy" title="Add helpful notes and warnings">
          💡 Add Tips
        </button>
        <button @click="triggerTemplate('format_table')" :disabled="isAiBusy" title="Convert unstructured data to table">
          📊 Table
        </button>
        <button @click="triggerTemplate('continue_writing')" :disabled="isAiBusy" title="Continue writing next section">
          ✍️ Continue
        </button>
      </div>

      <!-- Undo Banner if Snapshot available -->
      <div v-if="canUndo" class="ai-undo-banner">
        <span>AI modified document.</span>
        <button @click="undoLastAiAction" class="btn-undo-link">↩ Undo Last Change</button>
      </div>

      <!-- Pending Action Confirmation Card -->
      <div v-if="pendingAction" class="diff-confirmation-card">
        <div class="diff-card-header">
          <strong>⚠️ Approval Required</strong>
          <span class="diff-action-type">{{ pendingAction.tool_name }}</span>
        </div>
        <p class="diff-reason">{{ pendingAction.reason || ('Modify ' + pendingAction.filename) }}</p>

        <div v-if="pendingAction.diff && pendingAction.diff.length" class="diff-view-box">
          <div 
            v-for="(line, idx) in pendingAction.diff" 
            :key="idx" 
            :class="['diff-line', 'diff-' + line.diff_type]"
          >
            <span class="diff-prefix">{{ line.diff_type === 'added' ? '+' : line.diff_type === 'removed' ? '-' : ' ' }}</span>
            <span class="diff-text">{{ line.text }}</span>
          </div>
        </div>

        <div class="diff-card-actions">
          <button @click="confirmAction(true)" class="btn-diff-approve" :disabled="isAiBusy">✔ Approve & Apply</button>
          <button @click="confirmAction(false)" class="btn-diff-deny" :disabled="isAiBusy">✖ Deny</button>
        </div>
      </div>

      <!-- Chat History Stream -->
      <div class="ai-chat-messages" ref="chatMessagesContainer">
        <div v-if="messages.length === 0" class="chat-welcome">
          <p><strong>👋 Welcome to Fishdoc Assistant!</strong></p>
          <p>I can edit notes, answer questions, format tables, search your library, and summarize documentation directly via your configured Ollama backend.</p>
        </div>

        <div 
          v-for="(msg, idx) in messages" 
          :key="idx" 
          :class="['chat-bubble', msg.role]"
        >
          <div class="chat-bubble-header">
            <strong>{{ msg.role === 'user' ? 'You' : 'Fishdoc AI' }}</strong>
          </div>
          <div class="chat-bubble-body" v-html="formatMessageContent(msg.content)"></div>
        </div>

        <!-- Live Streaming Output -->
        <div v-if="isAiStreaming" class="chat-bubble assistant streaming">
          <div class="chat-bubble-header"><strong>Fishdoc AI</strong> <span class="typing-indicator">typing...</span></div>
          <div class="chat-bubble-body" v-html="formatMessageContent(streamingText)"></div>
        </div>

        <div v-if="aiError" class="chat-error-alert">
          {{ aiError }}
        </div>
      </div>

      <!-- Chat Input Area -->
      <div class="ai-input-container">
        <textarea 
          v-model="aiPromptInput" 
          @keydown.enter.exact.prevent="sendUserPrompt"
          placeholder="Ask AI to modify note or answer questions... (Enter to send)"
          :disabled="isAiBusy"
          rows="2"
        ></textarea>
        <button @click="sendUserPrompt" :disabled="isAiBusy || !aiPromptInput.trim()" class="btn-send-ai">
          {{ isAiBusy ? '⏳' : '➤' }}
        </button>
      </div>
    </aside>

    <!-- Create New Note Modal -->
    <div v-if="openNewNoteModal" class="modal-overlay" @click.self="openNewNoteModal = false">
      <div class="modal-card">
        <h3>Create New AsciiDoc Note</h3>
        <label>Note Title:
          <input v-model="newNoteTitle" placeholder="e.g. Project Roadmap" autofocus>
        </label>
        <label>Filename Preview:
          <code>{{ computedNewFilename }}</code>
        </label>
        <label>Template:
          <select v-model="newNoteTemplate">
            <option value="blank">Blank Note</option>
            <option value="technical">Technical Documentation</option>
            <option value="meeting">Meeting Notes</option>
            <option value="journal">Journal Entry</option>
          </select>
        </label>
        <div class="modal-actions">
          <button @click="createNote" class="btn-primary" :disabled="!newNoteTitle.trim()">Create Note</button>
          <button @click="openNewNoteModal = false" class="btn-cancel">Cancel</button>
        </div>
      </div>
    </div>

  </div>

  <script type="module" src="/app.js"></script>
</body>
</html>
"#;
