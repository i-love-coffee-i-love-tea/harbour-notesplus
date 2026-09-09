import { createApp, ref, computed, watch, nextTick, onMounted } from 'vue';

// Initialize Asciidoctor compiler instance if available
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
  if (!text || typeof text !== 'string') return '';
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

// Extract slides from AsciiDoc text based on headings (=, ==, ===) and page breaks (<<<)
function extractSlides(text) {
  if (!text || !text.trim()) {
    return [{
      index: 0,
      title: 'Slide 1',
      raw: '= Untitled',
      html: ''
    }];
  }

  const lines = text.split('\n');
  const slides = [];
  let currentLines = [];

  function flushSlide() {
    if (currentLines.length === 0 && slides.length > 0) return;
    const raw = currentLines.join('\n').trim();
    if (!raw && slides.length > 0) return;

    // Detect slide title from headings or first text line
    let title = '';
    for (const l of currentLines) {
      const trimmed = l.trim();
      if (trimmed.startsWith('= ') || trimmed.startsWith('== ') || trimmed.startsWith('=== ') || trimmed.startsWith('==== ')) {
        title = trimmed.replace(/^=+\s+/, '').trim();
        break;
      } else if (!title && trimmed && !trimmed.startsWith('//') && !trimmed.startsWith(':') && !trimmed.startsWith('[')) {
        title = trimmed;
      }
    }
    if (!title) {
      title = `Slide ${slides.length + 1}`;
    }

    slides.push({
      index: slides.length,
      title: title.length > 50 ? title.slice(0, 47) + '...' : title,
      raw: raw || ' ',
      html: ''
    });
    currentLines = [];
  }

  const hasLevel1Or2 = lines.some(l => /^={1,2}\s+\S+/.test(l.trim()));

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();

    // Explicit AsciiDoc Page Break <<<
    if (trimmed === '<<<') {
      flushSlide();
      continue;
    }

    // Heading-based slide boundaries
    const isSlideHeading = hasLevel1Or2
      ? (trimmed.startsWith('= ') || trimmed.startsWith('== '))
      : (trimmed.startsWith('= ') || trimmed.startsWith('== ') || trimmed.startsWith('=== '));

    if (isSlideHeading && currentLines.some(l => l.trim().length > 0)) {
      flushSlide();
    }

    currentLines.push(line);
  }

  flushSlide();

  if (slides.length === 0) {
    slides.push({
      index: 0,
      title: 'Slide 1',
      raw: text,
      html: ''
    });
  }

  return slides;
}

function getRequestedNote() {
  // 1. URL hash: #foo.adoc or #note=foo.adoc or #/page/foo.adoc
  if (typeof window !== 'undefined' && window.location && window.location.hash) {
    let hash = window.location.hash.slice(1).trim();
    if (hash.startsWith('/page/')) hash = hash.slice(6);
    else if (hash.startsWith('page/')) hash = hash.slice(5);
    else if (hash.startsWith('note=')) hash = hash.slice(5);
    try {
      hash = decodeURIComponent(hash);
    } catch (_) {}
    if (hash) {
      return hash.endsWith('.adoc') ? hash : hash + '.adoc';
    }
  }

  // 2. URL query: ?note=foo.adoc or ?page=foo.adoc
  if (typeof window !== 'undefined' && window.location && window.location.search) {
    try {
      const params = new URLSearchParams(window.location.search);
      const qNote = params.get('note') || params.get('page');
      if (qNote) {
        const decoded = decodeURIComponent(qNote.trim());
        if (decoded) {
          return decoded.endsWith('.adoc') ? decoded : decoded + '.adoc';
        }
      }
    } catch (_) {}
  }

  // 3. URL path: /page/foo.adoc or /notes/foo.adoc or /edit/foo.adoc
  if (typeof window !== 'undefined' && window.location && window.location.pathname) {
    const pathParts = window.location.pathname.split('/');
    if (pathParts[1] === 'page' || pathParts[1] === 'notes' || pathParts[1] === 'edit') {
      const target = pathParts[2];
      if (target) {
        try {
          const decoded = decodeURIComponent(target.trim());
          return decoded.endsWith('.adoc') ? decoded : decoded + '.adoc';
        } catch (_) {
          return target.endsWith('.adoc') ? target : target + '.adoc';
        }
      }
    }
  }

  // 4. LocalStorage
  try {
    const saved = localStorage.getItem('notesplusplus_last_note');
    if (saved && saved.trim()) {
      const trimmed = saved.trim();
      return trimmed.endsWith('.adoc') ? trimmed : trimmed + '.adoc';
    }
  } catch (_) {}

  return null;
}

createApp({
  setup() {
    // Note & View State
    const currentFilename = ref(getRequestedNote() || 'welcome.adoc');
    const notesList = ref([]);
    const rawContent = ref('= Welcome to Notes++\n\nStart writing documentation in AsciiDoc.\n');
    const viewMode = ref('split'); // 'split' | 'inplace' | 'preview'
    const isSaving = ref(false);
    const saveStatusText = ref('Saved');
    const saveStatusClass = ref('saved');
    const showExportMenu = ref(false);
    const showAccountMenu = ref(false);

    // Authentication State
    const isAuthenticated = ref(false);
    const authUser = ref('');
    const authError = ref('');
    const authStatus = ref('');
    const authVerificationCode = ref('');
    const authChallengeId = ref('');
    const authCanRetry = ref(false);
    const authPollTimer = ref(null);

    // Session Expiry & Timer State
    const sessionExpiresAt = ref(0);
    const sessionRemainingText = ref('');
    const sessionRemainingFullText = ref('');
    const sessionCountdownTimer = ref(null);

    function formatSessionRemaining(seconds) {
      if (seconds <= 0) return 'Expired';
      const days = Math.floor(seconds / 86400);
      const hours = Math.floor((seconds % 86400) / 3600);
      const minutes = Math.floor((seconds % 3600) / 60);
      const secs = Math.floor(seconds % 60);

      if (days >= 1) {
        return `${days}d ${hours}h`;
      } else if (hours >= 1) {
        return `${hours}h ${minutes}m`;
      } else if (minutes >= 1) {
        return `${minutes}m ${secs}s`;
      } else {
        return `${secs}s`;
      }
    }

    function formatSessionRemainingFull(seconds) {
      if (seconds <= 0) return 'Expired';
      const days = Math.floor(seconds / 86400);
      const hours = Math.floor((seconds % 86400) / 3600);
      const minutes = Math.floor((seconds % 3600) / 60);
      const secs = Math.floor(seconds % 60);
      const parts = [];
      if (days > 0) parts.push(`${days} day${days > 1 ? 's' : ''}`);
      if (hours > 0) parts.push(`${hours} hr${hours > 1 ? 's' : ''}`);
      if (minutes > 0) parts.push(`${minutes} min`);
      if (secs > 0 || parts.length === 0) parts.push(`${secs} sec`);
      return parts.join(' ');
    }

    function updateSessionCountdown() {
      if (!isAuthenticated.value || !sessionExpiresAt.value) {
        sessionRemainingText.value = '';
        sessionRemainingFullText.value = '';
        return;
      }
      const now = Math.floor(Date.now() / 1000);
      const diff = sessionExpiresAt.value - now;
      if (diff <= 0) {
        sessionRemainingText.value = 'Expired';
        sessionRemainingFullText.value = 'Session expired';
        stopSessionCountdown();
        logout();
      } else {
        sessionRemainingText.value = formatSessionRemaining(diff);
        sessionRemainingFullText.value = formatSessionRemainingFull(diff);
      }
    }

    function startSessionCountdown(expiresAt) {
      stopSessionCountdown();
      if (expiresAt) {
        sessionExpiresAt.value = expiresAt;
      }
      updateSessionCountdown();
      sessionCountdownTimer.value = setInterval(updateSessionCountdown, 1000);
    }

    function stopSessionCountdown() {
      if (sessionCountdownTimer.value) {
        clearInterval(sessionCountdownTimer.value);
        sessionCountdownTimer.value = null;
      }
    }

    async function fetchAuthConfig() {
      try {
        const res = await fetch('/api/auth/config', { cache: 'no-store' });
        if (res.ok) {
          const data = await res.json();
          isAuthenticated.value = !!data.authenticated;
          authUser.value = data.user || '';
          if (isAuthenticated.value && data.expires_at) {
            startSessionCountdown(data.expires_at);
          } else {
            stopSessionCountdown();
            sessionRemainingText.value = '';
            sessionRemainingFullText.value = '';
          }
          if (!isAuthenticated.value) {
            startPhoneAuth();
          }
          return data.authenticated;
        }
      } catch (err) {
        console.warn('Failed to fetch auth config:', err);
      }
      return false;
    }

    async function logout() {
      try {
        await fetch('/api/auth/logout', { method: 'POST' });
      } catch (_) {}
      isAuthenticated.value = false;
      authUser.value = '';
      stopAuthPolling();
      stopSessionCountdown();
      sessionRemainingText.value = '';
      sessionRemainingFullText.value = '';
      sessionExpiresAt.value = 0;
      authChallengeId.value = '';
      authVerificationCode.value = '';
      await fetchAuthConfig();
    }

    // Phone authentication challenge flow
    async function startPhoneAuth() {
      stopAuthPolling();
      authError.value = '';
      authStatus.value = 'Connecting to phone...';
      authCanRetry.value = false;
      try {
        const res = await fetch('/api/auth/code/initiate', { method: 'POST' });
        const data = await res.json();
        if (res.ok && data.ok) {
          authChallengeId.value = data.challenge_id;
          authVerificationCode.value = data.verification_code || '';
          authStatus.value = 'Please tap Accept on your phone';
          startAuthPolling();
        } else {
          authStatus.value = '';
          authError.value = data.error || 'Failed to initiate login authorization';
          authCanRetry.value = true;
        }
      } catch (err) {
        authStatus.value = '';
        authError.value = 'Authorization request failed: ' + err.message;
        authCanRetry.value = true;
      }
    }

    function startAuthPolling() {
      stopAuthPolling();
      authPollTimer.value = setInterval(async () => {
        if (!authChallengeId.value) {
          stopAuthPolling();
          return;
        }
        try {
          const res = await fetch('/api/auth/code/status?challenge_id=' + encodeURIComponent(authChallengeId.value));
          const data = await res.json();
          if (data.status === 'approved') {
            stopAuthPolling();
            isAuthenticated.value = true;
            authUser.value = data.user || 'phone-user';
            authChallengeId.value = '';
            authVerificationCode.value = '';
            authStatus.value = '';
            authError.value = '';
            authCanRetry.value = false;
            if (data.expires_at) {
              startSessionCountdown(data.expires_at);
            }
            await fetchNotesList();
            await fetchAiConfig();
          } else if (data.status === 'denied') {
            stopAuthPolling();
            authStatus.value = 'Login request was denied on the phone.';
            authCanRetry.value = true;
          } else if (data.status === 'expired') {
            stopAuthPolling();
            authStatus.value = 'Code expired. Requesting a new code...';
            setTimeout(() => { startPhoneAuth(); }, 1200);
          }
        } catch (_) {}
      }, 1000);
    }

    function stopAuthPolling() {
      if (authPollTimer.value) {
        clearInterval(authPollTimer.value);
        authPollTimer.value = null;
      }
    }

    // Connection & Health Check State
    const isPhoneReachable = ref(true);
    const isCheckingConnection = ref(false);
    const connectionError = ref('');
    let heartbeatTimer = null;

    function markPhoneReachable() {
      if (!isPhoneReachable.value) {
        isPhoneReachable.value = true;
        connectionError.value = '';
        fetchNotesList();
        updateRenderedHtml(rawContent.value);
      } else {
        isPhoneReachable.value = true;
        connectionError.value = '';
      }
    }

    function markPhoneUnreachable(err) {
      isPhoneReachable.value = false;
      if (err) {
        connectionError.value = typeof err === 'string' ? err : (err.message || 'Phone unreachable');
      }
    }

    async function checkConnection(quiet) {
      if (isCheckingConnection.value) return;
      if (!quiet) isCheckingConnection.value = true;
      try {
        let signal = undefined;
        let timer = null;
        if (typeof AbortController !== 'undefined') {
          const ctrl = new AbortController();
          timer = setTimeout(() => ctrl.abort(), 3000);
          signal = ctrl.signal;
        }
        const res = await fetch('/api/ping', {
          method: 'GET',
          signal: signal,
          cache: 'no-store'
        });
        if (timer) clearTimeout(timer);
        if (res.ok) {
          markPhoneReachable();
        } else {
          markPhoneUnreachable(new Error(`HTTP ${res.status}`));
        }
      } catch (err) {
        markPhoneUnreachable(err);
      } finally {
        if (!quiet) isCheckingConnection.value = false;
      }
    }

    // In-Place Block Editing State
    const inPlaceBlocks = ref([]);
    const editingBlockIndex = ref(-1);
    const activeBlockText = ref('');

    // Presentation Mode State
    const previousViewMode = ref('split');
    const slides = ref([]);
    const currentSlideIndex = ref(0);
    const showSlideOverview = ref(false);
    const isPresentationFullscreen = ref(false);
    const presentationStageRef = ref(null);
    const slideHtmlCache = new Map();

    const currentSlideHtml = computed(() => {
      if (!slides.value || slides.value.length === 0) return '';
      const slide = slides.value[currentSlideIndex.value];
      return slide ? (slide.html || '') : '';
    });

    async function renderSlideHtml(idx) {
      if (idx < 0 || idx >= slides.value.length) return;
      const slide = slides.value[idx];
      if (slideHtmlCache.has(slide.raw)) {
        slide.html = slideHtmlCache.get(slide.raw);
        return;
      }
      try {
        const res = await fetch('/api/render', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ content: slide.raw, full: false })
        });
        if (res.ok) {
          markPhoneReachable();
          const html = await res.text();
          slideHtmlCache.set(slide.raw, html);
          slide.html = html;
        } else {
          markPhoneUnreachable(new Error(`HTTP ${res.status}`));
        }
      } catch (e) {
        console.warn('Failed to render slide:', e);
        markPhoneUnreachable(e);
      }
    }

    async function renderAllSlides() {
      for (let i = 0; i < slides.value.length; i++) {
        if (!slides.value[i].html) {
          await renderSlideHtml(i);
        }
      }
    }

    function prepareSlides(text) {
      const content = text !== undefined ? text : rawContent.value;
      const parsed = extractSlides(content);
      
      for (const s of parsed) {
        if (slideHtmlCache.has(s.raw)) {
          s.html = slideHtmlCache.get(s.raw);
        }
      }

      slides.value = parsed;
      if (currentSlideIndex.value >= parsed.length) {
        currentSlideIndex.value = Math.max(0, parsed.length - 1);
      } else if (currentSlideIndex.value < 0) {
        currentSlideIndex.value = 0;
      }

      renderSlideHtml(currentSlideIndex.value);
      if (currentSlideIndex.value + 1 < parsed.length) {
        renderSlideHtml(currentSlideIndex.value + 1);
      }
      if (currentSlideIndex.value - 1 >= 0) {
        renderSlideHtml(currentSlideIndex.value - 1);
      }
    }

    function nextSlide() {
      if (currentSlideIndex.value < slides.value.length - 1) {
        currentSlideIndex.value++;
        renderSlideHtml(currentSlideIndex.value);
        if (currentSlideIndex.value + 1 < slides.value.length) {
          renderSlideHtml(currentSlideIndex.value + 1);
        }
      }
    }

    function prevSlide() {
      if (currentSlideIndex.value > 0) {
        currentSlideIndex.value--;
        renderSlideHtml(currentSlideIndex.value);
        if (currentSlideIndex.value - 1 >= 0) {
          renderSlideHtml(currentSlideIndex.value - 1);
        }
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
      if (viewMode.value !== 'present') {
        previousViewMode.value = viewMode.value;
      }
      viewMode.value = 'present';
      prepareSlides();
      renderAllSlides();
    }

    function exitPresentationMode() {
      if (isPresentationFullscreen.value) {
        if (document.exitFullscreen) {
          document.exitFullscreen().catch(() => {});
        } else if (document.webkitExitFullscreen) {
          document.webkitExitFullscreen();
        }
      }
      viewMode.value = previousViewMode.value || 'split';
      showSlideOverview.value = false;
    }

    function togglePresentationFullscreen() {
      if (!document.fullscreenElement && !document.webkitFullscreenElement) {
        const el = presentationStageRef.value || document.documentElement;
        if (el.requestFullscreen) {
          el.requestFullscreen().catch(() => {});
        } else if (el.webkitRequestFullscreen) {
          el.webkitRequestFullscreen();
        }
      } else {
        if (document.exitFullscreen) {
          document.exitFullscreen().catch(() => {});
        } else if (document.webkitExitFullscreen) {
          document.webkitExitFullscreen();
        }
      }
    }

    function onFullscreenChange() {
      isPresentationFullscreen.value = !!(document.fullscreenElement || document.webkitFullscreenElement);
    }

    // Touch Swipe Navigation for Presentation Mode
    let touchStartX = 0;
    let touchStartY = 0;

    function handleTouchStart(e) {
      if (!e.changedTouches || !e.changedTouches.length) return;
      touchStartX = e.changedTouches[0].screenX;
      touchStartY = e.changedTouches[0].screenY;
    }

    function handleTouchEnd(e) {
      if (!e.changedTouches || !e.changedTouches.length) return;
      const deltaX = e.changedTouches[0].screenX - touchStartX;
      const deltaY = e.changedTouches[0].screenY - touchStartY;
      if (Math.abs(deltaX) > 45 && Math.abs(deltaY) < 60) {
        if (deltaX < 0) {
          nextSlide();
        } else {
          prevSlide();
        }
      }
    }

    // Modal State
    const openNewNoteModal = ref(false);
    const newNoteTitle = ref('');
    const newNoteTemplate = ref('blank');

    // Link Modal State
    const openLinkModal = ref(false);
    const linkSearchQuery = ref('');
    const selectedLinkFilename = ref('');
    const selectedLinkTitle = ref('');
    const linkDisplayText = ref('');
    const linkFocusedIndex = ref(0);
    const linkEditorContext = ref({ mode: 'split', start: 0, end: 0, blockIndex: null });
    const linkSearchInputRef = ref(null);
    const linkPagesListRef = ref(null);

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
    const availableModels = ref([]);
    const isLoadingModels = ref(false);
    const modelsError = ref('');

    const aiConfig = ref({
      provider: 'ollama',
      model: 'llama3.2',
      system_prompt: ''
    });

    // Import Assistant State
    const aiTab = ref('chat'); // 'chat' or 'import'
    const importSourceText = ref('');
    const importTitle = ref('');
    const importMode = ref('convert_full');
    const importCustomInstruction = ref('');

    const isCurrentModelInList = computed(() => {
      if (!aiConfig.value.model) return false;
      return availableModels.value.some(m => m.id === aiConfig.value.model || m.name === aiConfig.value.model);
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
          markPhoneReachable();
          renderedHtml.value = await res.text();
          nextTick(() => {
            setupInteractiveFeatures();
          });
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
          prepareSlides(newVal);
        }
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

    // Link Search & Selection Dialog State & Computeds
    const isExternalUrl = computed(() => {
      const q = (linkSearchQuery.value || '').trim();
      return /^(https?:\/\/|mailto:|ftp:\/\/)/i.test(q);
    });

    const computedCustomFilename = computed(() => {
      const q = (linkSearchQuery.value || '').trim();
      if (!q) return '';
      if (isExternalUrl.value) return q;
      if (q.toLowerCase().endsWith('.adoc')) return q;
      const slug = q.toLowerCase().replace(/[^a-z0-9_-]+/g, '-').replace(/^-+|-+$/g, '');
      return (slug || 'untitled') + '.adoc';
    });

    const filteredLinkPages = computed(() => {
      const q = (linkSearchQuery.value || '').trim().toLowerCase();
      if (!q) {
        return notesList.value;
      }
      return notesList.value.filter(n => {
        const titleMatch = (n.title || '').toLowerCase().includes(q);
        const fileMatch = (n.filename || '').toLowerCase().includes(q);
        const snippetMatch = (n.snippet || '').toLowerCase().includes(q);
        return titleMatch || fileMatch || snippetMatch;
      });
    });

    const isExactMatch = computed(() => {
      const q = (linkSearchQuery.value || '').trim().toLowerCase();
      if (!q) return false;
      const targetFn = q.endsWith('.adoc') ? q : `${q}.adoc`;
      return filteredLinkPages.value.some(n => 
        (n.filename || '').toLowerCase() === targetFn || 
        (n.title || '').toLowerCase() === q
      );
    });

    const formattedLinkPreview = computed(() => {
      if (selectedLinkFilename.value) {
        const text = (linkDisplayText.value || '').trim() || selectedLinkTitle.value || selectedLinkFilename.value;
        return `xref:${selectedLinkFilename.value}[${text}]`;
      }
      const q = (linkSearchQuery.value || '').trim();
      if (q) {
        const text = (linkDisplayText.value || '').trim() || q;
        if (isExternalUrl.value) {
          return `${q}[${text}]`;
        }
        const fn = computedCustomFilename.value;
        return `xref:${fn}[${text}]`;
      }
      return '';
    });

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
          nextTick(() => {
            setupInteractiveFeatures();
          });
          return;
        } else {
          markPhoneUnreachable(new Error(`HTTP ${res.status}`));
        }
      } catch (e) {
        console.error('Failed to parse blocks from server:', e);
        markPhoneUnreachable(e);
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
          markPhoneReachable();
          const data = await res.json();
          if (data.content) {
            rawContent.value = data.content;
            if (data.html) renderedHtml.value = data.html;
            if (viewMode.value === 'inplace') {
              await loadInPlaceBlocks(data.content);
            }
          }
        } else {
          markPhoneUnreachable(new Error(`HTTP ${res.status}`));
        }
      } catch (e) {
        console.error('Failed to toggle checklist item:', e);
        markPhoneUnreachable(e);
      }
    }

    // Load Note List
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

    // Load Single Note
    async function loadNote(filename, updateHistory = true) {
      if (!filename) return;
      currentFilename.value = filename;

      try {
        localStorage.setItem('notesplusplus_last_note', filename);
      } catch (_) {}

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
            inPlaceBlocks.value = parseBlocksFromText(rawContent.value);
            editingBlockIndex.value = -1;
          } else if (viewMode.value === 'present') {
            prepareSlides(rawContent.value);
            renderAllSlides();
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
          markPhoneReachable();
          saveStatusText.value = 'Saved';
          saveStatusClass.value = 'saved';
          // Refresh list snippets
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

    // Create New Note
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
          body: JSON.stringify({
            title: title,
            content: starterContent
          })
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

    // AI Configuration & State
    async function fetchAiConfig() {
      try {
        const res = await fetch('/api/ai/config');
        if (res.ok) {
          markPhoneReachable();
          const data = await res.json();
          if (data.provider) {
            aiConfig.value.provider = data.provider;
          }
          if (data.model) {
            aiConfig.value.model = data.model;
          }
          aiConfig.value.system_prompt = data.system_prompt || '';
          canUndo.value = !!data.can_undo;
          await fetchAvailableModels();
        } else {
          markPhoneUnreachable(new Error(`HTTP ${res.status}`));
        }
      } catch (err) {
        console.warn('Could not load AI config:', err);
        markPhoneUnreachable(err);
      }
    }

    async function fetchAvailableModels() {
      isLoadingModels.value = true;
      modelsError.value = '';
      try {
        const res = await fetch('/api/ai/models');
        if (res.ok) {
          markPhoneReachable();
          const data = await res.json();
          if (data && Array.isArray(data.models)) {
            availableModels.value = data.models;
          } else {
            availableModels.value = [];
          }
        } else {
          console.warn('Could not fetch models from server:', res.status);
        }
      } catch (err) {
        console.warn('Failed to fetch models from server:', err);
      } finally {
        isLoadingModels.value = false;
      }
    }

    async function onProviderChange() {
      await saveAiSettings();
    }

    async function onModelSelect() {
      await saveAiSettings();
    }

    async function saveAiSettings() {
      try {
        const res = await fetch('/api/ai/config', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            provider: aiConfig.value.provider,
            model: aiConfig.value.model,
            system_prompt: aiConfig.value.system_prompt
          })
        });
        if (res.ok) {
          markPhoneReachable();
          showAiSettings.value = false;
          await fetchAvailableModels();
        } else {
          markPhoneUnreachable(new Error(`HTTP ${res.status}`));
        }
      } catch (err) {
        alert('Failed to save AI config: ' + err.message);
        markPhoneUnreachable(err);
      }
    }

    function toggleAiDrawer() {
      openAiDrawer.value = !openAiDrawer.value;
      if (openAiDrawer.value) {
        scrollChatToBottom();
        if (availableModels.value.length === 0) {
          fetchAvailableModels();
        }
      }
    }

    async function consumeSseStream(response, onToken, onPayload) {
      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });

        const lines = buffer.split('\n');
        buffer = lines.pop();

        for (const line of lines) {
          const trimmed = line.trim();
          if (!trimmed || trimmed === 'data: [DONE]') continue;
          if (trimmed.startsWith('data: ')) {
            const dataStr = trimmed.slice(6).trim();
            if (dataStr === '[DONE]') continue;
            try {
              const payload = JSON.parse(dataStr);
              if (payload.type === 'token' && typeof payload.text === 'string') {
                onToken(payload.text);
              }
              if (onPayload) {
                await onPayload(payload);
              }
            } catch (e) {
              console.warn('Error parsing SSE payload JSON:', dataStr, e);
            }
          }
        }
      }

      if (buffer && buffer.trim()) {
        const trimmed = buffer.trim();
        if (trimmed.startsWith('data: ')) {
          const dataStr = trimmed.slice(6).trim();
          if (dataStr && dataStr !== '[DONE]') {
            try {
              const payload = JSON.parse(dataStr);
              if (payload.type === 'token' && typeof payload.text === 'string') {
                onToken(payload.text);
              }
              if (onPayload) {
                await onPayload(payload);
              }
            } catch (_) {}
          }
        }
      }
    }

    // AI Chat & Execution via SSE Stream
    async function sendUserPrompt(customInstruction = null) {
      const instructionText = typeof customInstruction === 'string' ? customInstruction : null;
      const prompt = (instructionText || aiPromptInput.value || '').trim();
      if (!prompt || isAiBusy.value) return;

      messages.value.push({ role: 'user', content: prompt });
      if (!instructionText) aiPromptInput.value = '';
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
          markPhoneUnreachable(new Error(`HTTP ${response.status}`));
          throw new Error(`HTTP ${response.status}: ${await response.text()}`);
        }
        markPhoneReachable();

        let fullAnswer = '';

        await consumeSseStream(
          response,
          (token) => {
            streamingText.value += token;
            fullAnswer += token;
            scrollChatToBottom();
          },
          async (payload) => {
            if (payload.type === 'pending_confirmation') {
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
          }
        );

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

    // Import External Text as Note
    async function importText() {
      if (isAiBusy.value || !importSourceText.value.trim()) return;
      openAiDrawer.value = true;
      isAiBusy.value = true;
      isAiStreaming.value = true;
      streamingText.value = '';
      aiError.value = '';

      const userMsg = `Import: ${importSourceText.value.substring(0, 80)}${importSourceText.value.length > 80 ? '...' : ''}`;
      messages.value.push({ role: 'user', content: userMsg });

      try {
        const response = await fetch('/api/ai/template', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            template_id: 'import_convert',
            content: importSourceText.value,
            context_filename: importTitle.value ? importTitle.value.replace(/\s+/g, '-').toLowerCase() + '.adoc' : 'imported-note.adoc',
            target_title: importTitle.value || undefined,
            mode: importMode.value,
            custom_instruction: importCustomInstruction.value || undefined
          })
        });

        if (!response.ok) {
          markPhoneUnreachable(new Error(`HTTP ${response.status}`));
          throw new Error(`HTTP ${response.status}`);
        }
        markPhoneReachable();

        let fullAnswer = '';

        await consumeSseStream(
          response,
          (token) => { streamingText.value += token; fullAnswer += token; scrollChatToBottom(); },
          (payload) => {
            if (payload.type === 'finished') {
              canUndo.value = payload.can_undo || false;
              if (payload.last_created_note) loadNotesList();
            } else if (payload.type === 'pending_confirmation') {
              pendingAction.value = payload.action;
            } else if (payload.type === 'error') {
              aiError.value = payload.error;
            }
          }
        );

        if (fullAnswer) messages.value.push({ role: 'assistant', content: fullAnswer });
        await loadNotesList();
        importSourceText.value = '';
      } catch (err) {
        aiError.value = `Import Error: ${err.message}`;
      } finally {
        isAiBusy.value = false;
        isAiStreaming.value = false;
        streamingText.value = '';
        scrollChatToBottom();
      }
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

        if (!response.ok) {
          markPhoneUnreachable(new Error(`HTTP ${response.status}`));
          throw new Error(`HTTP ${response.status}`);
        }
        markPhoneReachable();

        let fullAnswer = '';

        await consumeSseStream(
          response,
          (token) => {
            streamingText.value += token;
            fullAnswer += token;
            scrollChatToBottom();
          },
          async (payload) => {
            if (payload.type === 'finished') {
              if (payload.content) fullAnswer = payload.content;
              if (payload.can_undo !== undefined) canUndo.value = payload.can_undo;
              await fetchNotesList();
              await loadNote(currentFilename.value);
            } else if (payload.type === 'error') {
              aiError.value = payload.error;
            }
          }
        );

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
        if (res.ok) {
          markPhoneReachable();
          const data = await res.json();
          canUndo.value = !!data.can_undo;
          await loadNote(currentFilename.value);
          messages.value.push({ role: 'assistant', content: `↩ ${data.message}` });
        } else {
          markPhoneUnreachable(new Error(`HTTP ${res.status}`));
          alert('Undo failed: Server error');
        }
      } catch (err) {
        alert('Undo error: ' + err.message);
        markPhoneUnreachable(err);
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

    function selectLinkTarget(note) {
      if (!note) return;
      selectedLinkFilename.value = note.filename;
      selectedLinkTitle.value = note.title || note.filename;
      if (!linkDisplayText.value.trim()) {
        linkDisplayText.value = note.title || note.filename;
      }
    }

    function selectCustomLinkTarget(query) {
      selectedLinkFilename.value = '';
      selectedLinkTitle.value = query;
      if (!linkDisplayText.value.trim()) {
        linkDisplayText.value = query;
      }
    }

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
          if (ctx.start !== ctx.end) {
            initialText = (activeBlockText.value || '').substring(ctx.start, ctx.end);
          }
        } else {
          ctx.start = (activeBlockText.value || '').length;
          ctx.end = (activeBlockText.value || '').length;
        }
      } else {
        ctx.mode = 'split';
        const el = editorTextarea.value;
        if (el) {
          ctx.start = el.selectionStart || 0;
          ctx.end = el.selectionEnd || 0;
          if (ctx.start !== ctx.end) {
            initialText = (rawContent.value || '').substring(ctx.start, ctx.end);
          }
        } else {
          ctx.start = (rawContent.value || '').length;
          ctx.end = (rawContent.value || '').length;
        }
      }

      linkEditorContext.value = ctx;
      linkDisplayText.value = initialText;
      linkSearchQuery.value = '';
      selectedLinkFilename.value = '';
      selectedLinkTitle.value = '';
      linkFocusedIndex.value = 0;
      openLinkModal.value = true;

      nextTick(() => {
        if (linkSearchInputRef.value) {
          linkSearchInputRef.value.focus();
        }
      });
    }

    function handleLinkKeydown(e) {
      const hasFallback = (linkSearchQuery.value || '').trim() && !isExactMatch.value;
      const totalCount = filteredLinkPages.value.length + (hasFallback ? 1 : 0);

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        if (totalCount > 0) {
          linkFocusedIndex.value = (linkFocusedIndex.value + 1) % totalCount;
          if (linkFocusedIndex.value < filteredLinkPages.value.length) {
            selectLinkTarget(filteredLinkPages.value[linkFocusedIndex.value]);
          } else {
            selectCustomLinkTarget((linkSearchQuery.value || '').trim());
          }
        }
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        if (totalCount > 0) {
          linkFocusedIndex.value = (linkFocusedIndex.value - 1 + totalCount) % totalCount;
          if (linkFocusedIndex.value < filteredLinkPages.value.length) {
            selectLinkTarget(filteredLinkPages.value[linkFocusedIndex.value]);
          } else {
            selectCustomLinkTarget((linkSearchQuery.value || '').trim());
          }
        }
      } else if (e.key === 'Enter') {
        e.preventDefault();
        if (totalCount > 0 && !selectedLinkFilename.value && !isExternalUrl.value) {
          if (linkFocusedIndex.value < filteredLinkPages.value.length) {
            selectLinkTarget(filteredLinkPages.value[linkFocusedIndex.value]);
          }
        }
        confirmLinkInsert();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        openLinkModal.value = false;
      }
    }

    function confirmLinkInsert() {
      const linkText = formattedLinkPreview.value;
      if (!linkText) return;

      const ctx = linkEditorContext.value;
      if (ctx.mode === 'inplace') {
        const s = ctx.start;
        const e = ctx.end;
        activeBlockText.value = (activeBlockText.value || '').slice(0, s) + linkText + (activeBlockText.value || '').slice(e);
        openLinkModal.value = false;
        nextTick(() => {
          const blockEl = document.querySelector('.inplace-editor-card textarea');
          if (blockEl) {
            blockEl.focus();
            blockEl.setSelectionRange(s + linkText.length, s + linkText.length);
          }
        });
      } else {
        const el = editorTextarea.value;
        const s = ctx.start;
        const e = ctx.end;
        rawContent.value = (rawContent.value || '').slice(0, s) + linkText + (rawContent.value || '').slice(e);
        onContentChange();
        openLinkModal.value = false;
        nextTick(() => {
          if (el) {
            el.focus();
            el.setSelectionRange(s + linkText.length, s + linkText.length);
          }
        });
      }
    }

    function insertLink() {
      openLinkDialog();
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
      if (openLinkModal.value) {
        if (e.key === 'Escape') {
          e.preventDefault();
          openLinkModal.value = false;
        }
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
          case 'ArrowRight':
          case 'ArrowDown':
          case 'PageDown':
          case ' ':
          case 'Enter':
          case 'l':
          case 'L':
          case 'j':
          case 'J':
            e.preventDefault();
            nextSlide();
            break;
          case 'ArrowLeft':
          case 'ArrowUp':
          case 'PageUp':
          case 'Backspace':
          case 'h':
          case 'H':
          case 'k':
          case 'K':
            e.preventDefault();
            prevSlide();
            break;
          case 'Home':
            e.preventDefault();
            goToSlide(0);
            break;
          case 'End':
            e.preventDefault();
            goToSlide(slides.value.length - 1);
            break;
          case 'f':
          case 'F':
          case 'F11':
            e.preventDefault();
            togglePresentationFullscreen();
            break;
          case 'o':
          case 'O':
          case 'g':
          case 'G':
            e.preventDefault();
            showSlideOverview.value = !showSlideOverview.value;
            break;
          case 'Escape':
            e.preventDefault();
            if (showSlideOverview.value) {
              showSlideOverview.value = false;
            } else {
              exitPresentationMode();
            }
            break;
        }
      }
    }

    onMounted(async () => {
      window.addEventListener('keydown', handleGlobalKeyDown);
      document.addEventListener('fullscreenchange', onFullscreenChange);
      document.addEventListener('webkitfullscreenchange', onFullscreenChange);
      document.addEventListener('click', (e) => {
        if (!e.target.closest('.export-dropdown')) {
          showExportMenu.value = false;
        }
        if (!e.target.closest('.account-dropdown-wrapper')) {
          showAccountMenu.value = false;
        }
      });
      window.addEventListener('hashchange', () => {
        const req = getRequestedNote();
        if (req && req !== currentFilename.value) {
          loadNote(req, false);
        }
      });
      const authed = await fetchAuthConfig();
      if (authed) {
        await fetchNotesList();
        await fetchAiConfig();
      }

      // Start periodic health check heartbeat (every 4 seconds)
      heartbeatTimer = setInterval(() => checkConnection(true), 4000);

      // Fetch Sailfish ambience theme and apply as CSS overrides
      fetchTheme();
      setInterval(fetchTheme, 10000);
    });

    async function fetchTheme() {
      try {
        const res = await fetch('/api/theme', { cache: 'no-store' });
        if (!res.ok) return;
        const colors = await res.json();
        if (!colors || Object.keys(colors).length === 0) return;
        const root = document.documentElement;
        for (const [key, value] of Object.entries(colors)) {
          root.style.setProperty('--' + key, value);
        }
      } catch (_) {}
    }

    return {
      isAuthenticated,
      authUser,
      authError,
      authStatus,
      authVerificationCode,
      authChallengeId,
      authCanRetry,
      sessionRemainingText,
      sessionRemainingFullText,
      fetchAuthConfig,
      logout,
      startPhoneAuth,
      isPhoneReachable,
      isCheckingConnection,
      connectionError,
      checkConnection,
      currentFilename,
      notesList,
      rawContent,
      viewMode,
      previousViewMode,
      slides,
      currentSlideIndex,
      showSlideOverview,
      isPresentationFullscreen,
      presentationStageRef,
      currentSlideHtml,
      prepareSlides,
      nextSlide,
      prevSlide,
      goToSlide,
      enterPresentationMode,
      exitPresentationMode,
      togglePresentationFullscreen,
      handleTouchStart,
      handleTouchEnd,
      isSaving,
      saveStatusText,
      saveStatusClass,
      showExportMenu,
      showAccountMenu,
      inPlaceBlocks,
      editingBlockIndex,
      activeBlockText,
      openNewNoteModal,
      newNoteTitle,
      newNoteTemplate,
      computedNewFilename,
      openLinkModal,
      linkSearchQuery,
      selectedLinkFilename,
      selectedLinkTitle,
      linkDisplayText,
      linkFocusedIndex,
      linkEditorContext,
      linkSearchInputRef,
      linkPagesListRef,
      isExternalUrl,
      computedCustomFilename,
      filteredLinkPages,
      isExactMatch,
      formattedLinkPreview,
      selectLinkTarget,
      selectCustomLinkTarget,
      openLinkDialog,
      handleLinkKeydown,
      confirmLinkInsert,
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
      availableModels,
      isLoadingModels,
      modelsError,
      isCurrentModelInList,
      fetchAvailableModels,
      onProviderChange,
      onModelSelect,
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
      aiTab,
      importSourceText,
      importTitle,
      importMode,
      importCustomInstruction,
      importText,
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
