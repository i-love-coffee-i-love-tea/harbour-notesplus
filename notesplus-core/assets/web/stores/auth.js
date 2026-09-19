import { defineStore } from 'pinia';
import { ref } from 'vue';
import { apiFetch, apiJson } from './api.js';
import { formatSessionRemaining, formatSessionRemainingFull } from '/composables/utils.js';

export const useAuthStore = defineStore('auth', () => {
  const isAuthenticated = ref(false);
  const authUser = ref('');
  const authError = ref('');
  const authStatus = ref('');
  const authVerificationCode = ref('');
  const authChallengeId = ref('');
  const authCanRetry = ref(false);
  let authPollTimer = null;

  const sessionExpiresAt = ref(0);
  const sessionRemainingText = ref('');
  const sessionRemainingFullText = ref('');
  let sessionCountdownTimer = null;

  function updateSessionCountdown() {
    if (!isAuthenticated.value || !sessionExpiresAt.value) {
      sessionRemainingText.value = '';
      sessionRemainingFullText.value = '';
      return;
    }
    const diff = sessionExpiresAt.value - Math.floor(Date.now() / 1000);
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
    if (expiresAt) sessionExpiresAt.value = expiresAt;
    updateSessionCountdown();
    sessionCountdownTimer = setInterval(updateSessionCountdown, 1000);
  }

  function stopSessionCountdown() {
    if (sessionCountdownTimer) { clearInterval(sessionCountdownTimer); sessionCountdownTimer = null; }
  }

  function stopAuthPolling() {
    if (authPollTimer) { clearInterval(authPollTimer); authPollTimer = null; }
  }

  async function fetchAuthConfig() {
    try {
      const res = await apiFetch('/api/auth/config', { cache: 'no-store' });
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
        if (!isAuthenticated.value) startPhoneAuth();
        return data.authenticated;
      }
    } catch (err) {
      console.warn('Failed to fetch auth config:', err);
    }
    return false;
  }

  async function logout() {
    try { await apiFetch('/api/auth/logout', { method: 'POST' }); } catch (_) {}
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

  async function startPhoneAuth() {
    stopAuthPolling();
    authError.value = '';
    authStatus.value = 'Connecting to phone...';
    authCanRetry.value = false;
    try {
      const res = await apiFetch('/api/auth/code/initiate', { method: 'POST' });
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
    authPollTimer = setInterval(async () => {
      if (!authChallengeId.value) { stopAuthPolling(); return; }
      try {
        const res = await apiFetch('/api/auth/code/status?challenge_id=' + encodeURIComponent(authChallengeId.value));
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
          if (data.expires_at) startSessionCountdown(data.expires_at);
          // Trigger post-auth actions via other stores
          const { useNotesStore } = await import('./notes.js');
          const { useAiStore } = await import('./ai.js');
          const notesStore = useNotesStore();
          const aiStore = useAiStore();
          await notesStore.fetchNotesList();
          await aiStore.fetchAiConfig();
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

  return {
    isAuthenticated, authUser, authError, authStatus,
    authVerificationCode, authChallengeId, authCanRetry,
    sessionExpiresAt, sessionRemainingText, sessionRemainingFullText,
    fetchAuthConfig, logout, startPhoneAuth, stopAuthPolling,
    startSessionCountdown, stopSessionCountdown,
  };
});
