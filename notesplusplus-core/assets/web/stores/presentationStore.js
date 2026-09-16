import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { apiText } from './api.js';
import { extractSlides } from '/composables/utils.js';
import { useNotesStore } from './notes.js';
import { useUiStore } from './ui.js';

export const usePresentationStore = defineStore('presentation', () => {
  const previousViewMode = ref('split');
  const slides = ref([]);
  const currentSlideIndex = ref(0);
  const showSlideOverview = ref(false);
  const isPresentationFullscreen = ref(false);
  const presentationStageRef = ref(null);
  const slideHtmlCache = new Map();

  const currentSlideHtml = computed(() => {
    if (!slides.value.length) return '';
    const slide = slides.value[currentSlideIndex.value];
    return slide ? (slide.html || '') : '';
  });

  async function renderSlideHtml(idx) {
    if (idx < 0 || idx >= slides.value.length) return;
    const slide = slides.value[idx];
    if (slideHtmlCache.has(slide.raw)) { slide.html = slideHtmlCache.get(slide.raw); return; }
    try {
      const html = await apiText('/api/render', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content: slide.raw, full: false })
      });
      slideHtmlCache.set(slide.raw, html);
      slide.html = html;
    } catch (e) { console.warn('Failed to render slide:', e); }
  }

  async function renderAllSlides() {
    for (let i = 0; i < slides.value.length; i++) {
      if (!slides.value[i].html) await renderSlideHtml(i);
    }
  }

  async function prepareSlides(text) {
    const notesStore = useNotesStore();
    const content = text !== undefined ? text : notesStore.rawContent;
    const parsed = extractSlides(content);
    for (const s of parsed) {
      if (slideHtmlCache.has(s.raw)) s.html = slideHtmlCache.get(s.raw);
    }
    slides.value = parsed;
    if (currentSlideIndex.value >= parsed.length) currentSlideIndex.value = Math.max(0, parsed.length - 1);
    else if (currentSlideIndex.value < 0) currentSlideIndex.value = 0;
    renderSlideHtml(currentSlideIndex.value);
    if (currentSlideIndex.value + 1 < parsed.length) renderSlideHtml(currentSlideIndex.value + 1);
    if (currentSlideIndex.value - 1 >= 0) renderSlideHtml(currentSlideIndex.value - 1);
  }

  function nextSlide() {
    if (currentSlideIndex.value < slides.value.length - 1) {
      currentSlideIndex.value++;
      renderSlideHtml(currentSlideIndex.value);
      if (currentSlideIndex.value + 1 < slides.value.length) renderSlideHtml(currentSlideIndex.value + 1);
    }
  }

  function prevSlide() {
    if (currentSlideIndex.value > 0) {
      currentSlideIndex.value--;
      renderSlideHtml(currentSlideIndex.value);
      if (currentSlideIndex.value - 1 >= 0) renderSlideHtml(currentSlideIndex.value - 1);
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
    const uiStore = useUiStore();
    if (uiStore.viewMode !== 'present') previousViewMode.value = uiStore.viewMode;
    uiStore.viewMode = 'present';
    await prepareSlides();
    renderAllSlides();
  }

  function exitPresentationMode() {
    const uiStore = useUiStore();
    if (isPresentationFullscreen.value) {
      if (document.exitFullscreen) document.exitFullscreen().catch(() => {});
      else if (document.webkitExitFullscreen) document.webkitExitFullscreen();
    }
    uiStore.viewMode = previousViewMode.value || 'split';
    showSlideOverview.value = false;
  }

  function togglePresentationFullscreen() {
    if (!document.fullscreenElement && !document.webkitFullscreenElement) {
      const el = presentationStageRef.value || document.documentElement;
      if (el.requestFullscreen) el.requestFullscreen().catch(() => {});
      else if (el.webkitRequestFullscreen) el.webkitRequestFullscreen();
    } else {
      if (document.exitFullscreen) document.exitFullscreen().catch(() => {});
      else if (document.webkitExitFullscreen) document.webkitExitFullscreen();
    }
  }

  function onFullscreenChange() {
    isPresentationFullscreen.value = !!(document.fullscreenElement || document.webkitFullscreenElement);
  }

  let touchStartX = 0, touchStartY = 0;
  function handleTouchStart(e) {
    if (!e.changedTouches?.length) return;
    touchStartX = e.changedTouches[0].screenX;
    touchStartY = e.changedTouches[0].screenY;
  }
  function handleTouchEnd(e) {
    if (!e.changedTouches?.length) return;
    const dx = e.changedTouches[0].screenX - touchStartX;
    const dy = e.changedTouches[0].screenY - touchStartY;
    if (Math.abs(dx) > 45 && Math.abs(dy) < 60) { dx < 0 ? nextSlide() : prevSlide(); }
  }

  return {
    previousViewMode, slides, currentSlideIndex, showSlideOverview,
    isPresentationFullscreen, presentationStageRef, currentSlideHtml,
    prepareSlides, nextSlide, prevSlide, goToSlide,
    enterPresentationMode, exitPresentationMode,
    togglePresentationFullscreen, onFullscreenChange,
    handleTouchStart, handleTouchEnd, renderAllSlides,
  };
});
