import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { slugify } from '/composables/utils.js';

export const useUiStore = defineStore('ui', () => {
  const viewMode = ref('gallery');
  const showExportMenu = ref(false);
  const showAccountMenu = ref(false);
  const showColorMenu = ref(false);
  const openNewNoteModal = ref(false);
  const newNoteTitle = ref('');
  const newNoteTemplate = ref('blank');
  const newNoteColor = ref('');

  return {
    viewMode, showExportMenu, showAccountMenu, showColorMenu,
    openNewNoteModal, newNoteTitle, newNoteTemplate, newNoteColor,
    computedNewFilename: computed(() => slugify(newNoteTitle.value) + '.adoc'),
  };
});
