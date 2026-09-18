import { createApp, onMounted, watch, nextTick } from 'vue';
import { storeToRefs } from 'pinia';
import { pinia } from '/stores/index.js';
import { setHealthCallbacks } from '/stores/api.js';
import { formatMarkdown, getRequestedNote } from '/composables/utils.js';
import { useNotesStore } from '/stores/notes.js';
import { useAuthStore } from '/stores/auth.js';
import { useThemeStore } from '/stores/theme.js';
import { useHealthStore } from '/stores/health.js';
import { useAiStore } from '/stores/ai.js';
import { useUiStore } from '/stores/ui.js';
import { usePresentationStore } from '/stores/presentationStore.js';
import { useImportStore } from '/stores/importStore.js';
import { useLinkStore } from '/stores/linkStore.js';
import { useEditorStore } from '/stores/editorStore.js';

const app = createApp({
  setup() {
    const notes = useNotesStore();
    const auth = useAuthStore();
    const theme = useThemeStore();
    const health = useHealthStore();
    const ai = useAiStore();
    const ui = useUiStore();
    const pres = usePresentationStore();
    const imp = useImportStore();
    const link = useLinkStore();
    const editor = useEditorStore();

    // Wire API health callbacks
    setHealthCallbacks(
      () => { health.markPhoneReachable(); },
      (err) => health.markPhoneUnreachable(err)
    );

    // Watch rawContent for render updates
    let renderTimer = null;
    watch(() => notes.rawContent, (newVal) => {
      clearTimeout(renderTimer);
      renderTimer = setTimeout(() => {
        notes.updateRenderedHtml(newVal);
        if (ui.viewMode === 'present') pres.prepareSlides(newVal);
      }, 80);
    }, { immediate: true });

    // Keep link store's cached notesList in sync
    watch(() => notes.notesList, (list) => link.setCachedNotesList(list), { immediate: true });

    // Global keyboard shortcuts
    function handleGlobalKeyDown(e) {
      if (link.openLinkModal) {
        if (e.key === 'Escape') { e.preventDefault(); link.openLinkModal = false; }
        return;
      }
      if ((e.ctrlKey || e.metaKey) && (e.key === 's' || e.key === 'S')) {
        e.preventDefault(); notes.saveCurrentNote(); return;
      }
      if ((e.ctrlKey || e.metaKey) && (e.key === 'k' || e.key === 'K')) {
        e.preventDefault(); link.openLinkDialog(); return;
      }
      if (ui.viewMode === 'present') {
        if (['INPUT', 'TEXTAREA', 'SELECT'].includes(e.target?.tagName)) return;
        switch (e.key) {
          case 'ArrowRight': case 'ArrowDown': case 'PageDown': case ' ': case 'Enter': case 'l': case 'L': case 'j': case 'J':
            e.preventDefault(); pres.nextSlide(); break;
          case 'ArrowLeft': case 'ArrowUp': case 'PageUp': case 'Backspace': case 'h': case 'H': case 'k': case 'K':
            e.preventDefault(); pres.prevSlide(); break;
          case 'Home': e.preventDefault(); pres.goToSlide(0); break;
          case 'End': e.preventDefault(); pres.goToSlide(pres.slides.length - 1); break;
          case 'f': case 'F': case 'F11': e.preventDefault(); pres.togglePresentationFullscreen(); break;
          case 'o': case 'O': case 'g': case 'G': e.preventDefault(); pres.showSlideOverview = !pres.showSlideOverview; break;
          case 'Escape':
            e.preventDefault();
            if (pres.showSlideOverview) pres.showSlideOverview = false;
            else pres.exitPresentationMode();
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
      document.addEventListener('fullscreenchange', pres.onFullscreenChange);
      document.addEventListener('webkitfullscreenchange', pres.onFullscreenChange);
      document.addEventListener('click', (e) => {
        if (!e.target.closest('.export-dropdown')) ui.showExportMenu = false;
        if (!e.target.closest('.account-dropdown-wrapper')) ui.showAccountMenu = false;
        if (!e.target.closest('.note-color-badge-wrapper')) ui.showColorMenu = false;
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

      heartbeatTimer = setInterval(() => health.checkConnection(true), 10000);
      theme.initTheme();
    });

    // Expose store references and template bindings directly
    const stores = [notes, auth, theme, health, ai, ui, pres, imp, link, editor];
    const bindings = {
      // Direct store references
      notes,
      auth,
      theme,
      health,
      ai,
      ui,
      pres,
      imp,
      link,
      editor,

      // App-level template helpers
      formatMessageContent: (content) => formatMarkdown(content),
      createNote: () => notes.createNote(ui.newNoteTitle, ui.newNoteTemplate, ui.newNoteColor),
    };

    // Expose reactive state/getters and bound actions for seamless template access
    for (const store of stores) {
      Object.assign(bindings, storeToRefs(store));
      for (const key of Object.keys(store)) {
        if (typeof store[key] === 'function' && !key.startsWith('$')) {
          bindings[key] = store[key].bind(store);
        }
      }
    }

    return bindings;
  }
});

app.use(pinia);
app.mount('#app');
