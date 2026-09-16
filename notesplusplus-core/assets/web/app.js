import { createApp, onMounted, watch, nextTick } from 'vue';
import { pinia } from '/stores/index.js';
import { setHealthCallbacks } from '/stores/api.js';
import { formatMarkdown, getRequestedNote } from '/composables/utils.js';
import { useNotesStore } from '/stores/notes.js';
import { useAuthStore } from '/stores/auth.js';
import { useThemeStore } from '/stores/theme.js';
import { useHealthStore } from '/stores/health.js';
import { useAiStore } from '/stores/ai.js';
import { useUiStore } from '/stores/ui.js';

const app = createApp({
  setup() {
    const notes = useNotesStore();
    const auth = useAuthStore();
    const theme = useThemeStore();
    const health = useHealthStore();
    const ai = useAiStore();
    const ui = useUiStore();

    // Wire API health callbacks
    setHealthCallbacks(
      () => { health.markPhoneReachable(); notes.fetchNotesList(); },
      (err) => health.markPhoneUnreachable(err)
    );

    // Watch rawContent for render updates
    let renderTimer = null;
    watch(() => notes.rawContent, (newVal) => {
      clearTimeout(renderTimer);
      renderTimer = setTimeout(() => {
        notes.updateRenderedHtml(newVal);
        if (ui.viewMode === 'present') ui.prepareSlides(newVal);
      }, 80);
    }, { immediate: true });

    // Keep ui store's cached notesList in sync
    watch(() => notes.notesList, (list) => ui.setCachedNotesList(list), { immediate: true });

    // Global keyboard shortcuts
    function handleGlobalKeyDown(e) {
      if (ui.openLinkModal) {
        if (e.key === 'Escape') { e.preventDefault(); ui.openLinkModal = false; }
        return;
      }
      if ((e.ctrlKey || e.metaKey) && (e.key === 's' || e.key === 'S')) {
        e.preventDefault(); notes.saveCurrentNote(); return;
      }
      if ((e.ctrlKey || e.metaKey) && (e.key === 'k' || e.key === 'K')) {
        e.preventDefault(); ui.openLinkDialog(); return;
      }
      if (ui.viewMode === 'present') {
        if (['INPUT', 'TEXTAREA', 'SELECT'].includes(e.target?.tagName)) return;
        switch (e.key) {
          case 'ArrowRight': case 'ArrowDown': case 'PageDown': case ' ': case 'Enter': case 'l': case 'L': case 'j': case 'J':
            e.preventDefault(); ui.nextSlide(); break;
          case 'ArrowLeft': case 'ArrowUp': case 'PageUp': case 'Backspace': case 'h': case 'H': case 'k': case 'K':
            e.preventDefault(); ui.prevSlide(); break;
          case 'Home': e.preventDefault(); ui.goToSlide(0); break;
          case 'End': e.preventDefault(); ui.goToSlide(ui.slides.length - 1); break;
          case 'f': case 'F': case 'F11': e.preventDefault(); ui.togglePresentationFullscreen(); break;
          case 'o': case 'O': case 'g': case 'G': e.preventDefault(); ui.showSlideOverview = !ui.showSlideOverview; break;
          case 'Escape':
            e.preventDefault();
            if (ui.showSlideOverview) ui.showSlideOverview = false;
            else ui.exitPresentationMode();
            break;
        }
      }
    }

    // Live companion sync (SSE)
    function initEventSource() {
      if (typeof EventSource === 'undefined') return;
      try {
        const es = new EventSource('/api/events');
        es.onmessage = (e) => {
          try {
            const data = JSON.parse(e.data);
            if (data.type === 'page_updated' || data.type === 'notes_changed') notes.fetchNotesList();
          } catch (_) {}
        };
        es.onerror = () => es.close();
      } catch (_) {}
    }

    let heartbeatTimer = null;

    onMounted(async () => {
      window.addEventListener('keydown', handleGlobalKeyDown);
      document.addEventListener('fullscreenchange', ui.onFullscreenChange);
      document.addEventListener('webkitfullscreenchange', ui.onFullscreenChange);
      document.addEventListener('click', (e) => {
        if (!e.target.closest('.export-dropdown')) ui.showExportMenu = false;
        if (!e.target.closest('.account-dropdown-wrapper')) ui.showAccountMenu = false;
      });
      window.addEventListener('hashchange', () => {
        const req = getRequestedNote();
        if (req && req !== notes.currentFilename) notes.loadNote(req, false);
      });

      const authed = await auth.fetchAuthConfig();
      if (authed) {
        await notes.fetchNotesList();
        await ai.fetchAiConfig();
        initEventSource();
      }

      heartbeatTimer = setInterval(() => health.checkConnection(true), 4000);
      theme.initTheme();
      setInterval(() => theme.fetchTheme(), 10000);
    });

    // Return template bindings — proxy to all stores
    return new Proxy({}, {
      get(_, prop) {
        if (prop in notes) return notes[prop];
        if (prop in auth) return auth[prop];
        if (prop in theme) return theme[prop];
        if (prop in health) return health[prop];
        if (prop in ai) return ai[prop];
        if (prop in ui) return ui[prop];
        if (prop === 'formatMessageContent') return (content) => formatMarkdown(content);
        if (prop === 'createNote') return () => notes.createNote(ui.newNoteTitle, ui.newNoteTemplate);
        return undefined;
      }
    });
  }
});

app.use(pinia);
app.mount('#app');
