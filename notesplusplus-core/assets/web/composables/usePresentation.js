import { ref, computed } from 'vue';
import { extractSlides } from './utils.js';

export function usePresentation({ rawContent, viewMode, markPhoneReachable, markPhoneUnreachable }) {
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
        if (markPhoneReachable) markPhoneReachable();
        const html = await res.text();
        slideHtmlCache.set(slide.raw, html);
        slide.html = html;
      } else {
        if (markPhoneUnreachable) markPhoneUnreachable(new Error(`HTTP ${res.status}`));
      }
    } catch (e) {
      console.warn('Failed to render slide:', e);
      if (markPhoneUnreachable) markPhoneUnreachable(e);
    }
  }

  async function renderAllSlides() {
    for (let i = 0; i < slides.value.length; i++) {
      if (!slides.value[i].html) await renderSlideHtml(i);
    }
  }

  function prepareSlides(text) {
    const content = text !== undefined ? text : rawContent.value;
    const parsed = extractSlides(content);
    for (const s of parsed) {
      if (slideHtmlCache.has(s.raw)) s.html = slideHtmlCache.get(s.raw);
    }
    slides.value = parsed;
    if (currentSlideIndex.value >= parsed.length) {
      currentSlideIndex.value = Math.max(0, parsed.length - 1);
    } else if (currentSlideIndex.value < 0) {
      currentSlideIndex.value = 0;
    }
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
    if (viewMode.value !== 'present') previousViewMode.value = viewMode.value;
    viewMode.value = 'present';
    prepareSlides();
    renderAllSlides();
  }

  function exitPresentationMode() {
    if (isPresentationFullscreen.value) {
      if (document.exitFullscreen) document.exitFullscreen().catch(() => {});
      else if (document.webkitExitFullscreen) document.webkitExitFullscreen();
    }
    viewMode.value = previousViewMode.value || 'split';
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

  // Touch Swipe Navigation
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
      if (deltaX < 0) nextSlide(); else prevSlide();
    }
  }

  return {
    previousViewMode, slides, currentSlideIndex, showSlideOverview,
    isPresentationFullscreen, presentationStageRef, currentSlideHtml,
    prepareSlides, nextSlide, prevSlide, goToSlide,
    enterPresentationMode, exitPresentationMode,
    togglePresentationFullscreen, onFullscreenChange,
    handleTouchStart, handleTouchEnd, renderAllSlides,
  };
}
