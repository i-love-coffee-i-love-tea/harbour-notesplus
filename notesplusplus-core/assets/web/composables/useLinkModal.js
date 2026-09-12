import { ref, computed } from 'vue';

export function useLinkModal({ notesList }) {
  const openLinkModal = ref(false);
  const linkSearchQuery = ref('');
  const selectedLinkFilename = ref('');
  const selectedLinkTitle = ref('');
  const linkDisplayText = ref('');
  const linkFocusedIndex = ref(0);
  const linkEditorContext = ref({ mode: 'split', start: 0, end: 0, blockIndex: null });
  const linkSearchInputRef = ref(null);
  const linkPagesListRef = ref(null);

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
    if (!q) return notesList.value;
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
      if (isExternalUrl.value) return `${q}[${text}]`;
      const fn = computedCustomFilename.value;
      return `xref:${fn}[${text}]`;
    }
    return '';
  });

  function openLinkDialog(context) {
    linkEditorContext.value = context || { mode: 'split', start: 0, end: 0, blockIndex: null };
    linkSearchQuery.value = '';
    selectedLinkFilename.value = '';
    selectedLinkTitle.value = '';
    linkDisplayText.value = '';
    linkFocusedIndex.value = 0;
    openLinkModal.value = true;
  }

  function selectLinkTarget(filename, title) {
    selectedLinkFilename.value = filename;
    selectedLinkTitle.value = title || filename;
    linkDisplayText.value = '';
  }

  function handleLinkKeydown(e) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      linkFocusedIndex.value = Math.min(linkFocusedIndex.value + 1, filteredLinkPages.value.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      linkFocusedIndex.value = Math.max(linkFocusedIndex.value - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const page = filteredLinkPages.value[linkFocusedIndex.value];
      if (page) selectLinkTarget(page.filename, page.title);
    }
  }

  return {
    openLinkModal, linkSearchQuery, selectedLinkFilename, selectedLinkTitle,
    linkDisplayText, linkFocusedIndex, linkEditorContext,
    linkSearchInputRef, linkPagesListRef,
    isExternalUrl, computedCustomFilename, filteredLinkPages,
    isExactMatch, formattedLinkPreview,
    openLinkDialog, selectLinkTarget, handleLinkKeydown,
  };
}
