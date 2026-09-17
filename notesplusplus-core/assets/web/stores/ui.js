import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { slugify } from '/composables/utils.js';

export const useUiStore = defineStore('ui', () => {
  const viewMode = ref('gallery');
  const showExportMenu = ref(false);
  const showAccountMenu = ref(false);
  const openNewNoteModal = ref(false);
  const newNoteTitle = ref('');
  const newNoteTemplate = ref('blank');

  return {
    viewMode, showExportMenu, showAccountMenu,
    openNewNoteModal, newNoteTitle, newNoteTemplate,
    computedNewFilename: computed(() => slugify(newNoteTitle.value) + '.adoc'),
  };
});
