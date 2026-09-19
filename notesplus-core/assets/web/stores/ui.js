import { defineStore } from 'pinia';
import { ref, computed, watch } from 'vue';
import { slugify } from '/composables/utils.js';

export const useUiStore = defineStore('ui', () => {
  const viewMode = ref((() => { try { return localStorage.getItem('np_view_mode') || 'gallery'; } catch (_) { return 'gallery'; } })());
  const showExportMenu = ref(false);
  const showAccountMenu = ref(false);
  const showColorMenu = ref(false);
  const openNewNoteModal = ref(false);
  const newNoteTitle = ref('');
  const newNoteTemplate = ref('blank');
  const newNoteColor = ref('');

  watch(viewMode, (v) => { try { localStorage.setItem('np_view_mode', v); } catch (_) {} });

  return {
    viewMode, showExportMenu, showAccountMenu, showColorMenu,
    openNewNoteModal, newNoteTitle, newNoteTemplate, newNoteColor,
    computedNewFilename: computed(() => slugify(newNoteTitle.value) + '.adoc'),
  };
});
