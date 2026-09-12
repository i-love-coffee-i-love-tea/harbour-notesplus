import { ref } from 'vue';

export function useAuth({ onAuthenticated }) {
  const isAuthenticated = ref(false);
  const authUser = ref('');
  const authError = ref('');
  const authStatus = ref('');
  const authVerificationCode = ref('');
  const authChallengeId = ref('');
  const authCanRetry = ref(false);
  const authPollTimer = ref(null);

  // Session Expiry & Timer State
  const sessionExpiresAt = ref(0);
  const sessionRemainingText = ref('');
  const sessionRemainingFullText = ref('');
  const sessionCountdownTimer = ref(null);

  function formatSessionRemaining(seconds) {
    if (seconds <= 0) return 'Expired';
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);
    if (days >= 1) return `${days}d ${hours}h`;
    if (hours >= 1) return `${hours}h ${minutes}m`;
    if (minutes >= 1) return `${minutes}m ${secs}s`;
    return `${secs}s`;
  }

  function formatSessionRemainingFull(seconds) {
    if (seconds <= 0) return 'Expired';
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);
    const parts = [];
    if (days > 0) parts.push(`${days} day${days > 1 ? 's' : ''}`);
    if (hours > 0) parts.push(`${hours} hr${hours > 1 ? 's' : ''}`);
    if (minutes > 0) parts.push(`${minutes} min`);
    if (secs > 0 || parts.length === 0) parts.push(`${secs} sec`);
    return parts.join(' ');
  }

  function updateSessionCountdown() {
    if (!isAuthenticated.value || !sessionExpiresAt.value) {
      sessionRemainingText.value = '';
      sessionRemainingFullText.value = '';
      return;
    }
    const now = Math.floor(Date.now() / 1000);
    const diff = sessionExpiresAt.value - now;
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
    sessionCountdownTimer.value = setInterval(updateSessionCountdown, 1000);
  }

  function stopSessionCountdown() {
    if (sessionCountdownTimer.value) {
      clearInterval(sessionCountdownTimer.value);
      sessionCountdownTimer.value = null;
    }
  }

  function stopAuthPolling() {
    if (authPollTimer.value) {
      clearInterval(authPollTimer.value);
      authPollTimer.value = null;
    }
  }

  async function fetchAuthConfig() {
    try {
      const res = await fetch('/api/auth/config', { cache: 'no-store' });
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
    try { await fetch('/api/auth/logout', { method: 'POST' }); } catch (_) {}
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
      const res = await fetch('/api/auth/code/initiate', { method: 'POST' });
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
    authPollTimer.value = setInterval(async () => {
      if (!authChallengeId.value) { stopAuthPolling(); return; }
      try {
        const res = await fetch('/api/auth/code/status?challenge_id=' + encodeURIComponent(authChallengeId.value));
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
          if (onAuthenticated) await onAuthenticated();
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
}
