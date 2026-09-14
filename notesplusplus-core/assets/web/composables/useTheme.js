import { ref, computed, watch, onMounted } from 'vue';

const THEME_STORAGE_KEY = 'notesplusplus_theme_preference';

export function useTheme() {
  const themePreference = ref(localStorage.getItem(THEME_STORAGE_KEY) || 'os');
  const osTheme = ref(null);
  const browserPrefersDark = ref(
    typeof window !== 'undefined' && window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches
  );

  const effectiveTheme = computed(() => {
    if (themePreference.value === 'dark') return 'dark';
    if (themePreference.value === 'light') return 'light';
    if (themePreference.value === 'browser') {
      return browserPrefersDark.value ? 'dark' : 'light';
    }
    // 'os' mode (default): Follow OS ambiance if available, fallback to browser
    if (osTheme.value && (osTheme.value.colorScheme === 'dark' || osTheme.value.colorScheme === 'light')) {
      return osTheme.value.colorScheme;
    }
    return browserPrefersDark.value ? 'dark' : 'light';
  });

  function applyTheme() {
    const root = document.documentElement;
    const theme = effectiveTheme.value;
    root.setAttribute('data-theme', theme);

    if (osTheme.value) {
      if (theme === 'dark') {
        if (osTheme.value.highlightColor) {
          root.style.setProperty('--primary', osTheme.value.highlightColor);
          root.style.setProperty('--primary-hover', osTheme.value.highlightColor);
          root.style.setProperty('--accent', osTheme.value.highlightColor);
          root.style.setProperty('--silica-highlight', osTheme.value.highlightColor);
        }
        if (osTheme.value.highlightBackgroundColor) {
          root.style.setProperty('--accent-light', osTheme.value.highlightBackgroundColor);
          root.style.setProperty('--silica-highlight-bg', osTheme.value.highlightBackgroundColor);
        }
      } else {
        root.style.removeProperty('--primary');
        root.style.removeProperty('--primary-hover');
        root.style.removeProperty('--accent');
        root.style.removeProperty('--accent-light');
        if (osTheme.value.highlightColor) {
          root.style.setProperty('--silica-highlight', osTheme.value.highlightColor);
        }
        if (osTheme.value.highlightBackgroundColor) {
          root.style.setProperty('--silica-highlight-bg', osTheme.value.highlightBackgroundColor);
        }
      }
    }
  }

  function setThemePreference(pref) {
    if (['os', 'browser', 'dark', 'light'].includes(pref)) {
      themePreference.value = pref;
      try {
        localStorage.setItem(THEME_STORAGE_KEY, pref);
      } catch (_) {}
      applyTheme();
    }
  }

  async function fetchTheme() {
    try {
      const res = await fetch('/api/theme', { cache: 'no-store' });
      if (!res.ok) return;
      const data = await res.json();
      if (data && typeof data === 'object') {
        osTheme.value = data;
        applyTheme();
      }
    } catch (_) {}
  }

  function initTheme() {
    // Listen for browser color scheme changes
    if (typeof window !== 'undefined' && window.matchMedia) {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      const listener = (e) => {
        browserPrefersDark.value = e.matches;
        applyTheme();
      };
      if (mediaQuery.addEventListener) {
        mediaQuery.addEventListener('change', listener);
      } else if (mediaQuery.addListener) {
        mediaQuery.addListener(listener);
      }
    }

    applyTheme();
    fetchTheme();
  }

  watch(effectiveTheme, () => {
    applyTheme();
  });

  return {
    themePreference,
    effectiveTheme,
    osTheme,
    setThemePreference,
    fetchTheme,
    initTheme,
    applyTheme,
  };
}
