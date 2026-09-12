import { ref } from 'vue';

export function useHealthCheck({ onReachable, onUnreachable }) {
  const isPhoneReachable = ref(true);
  const isCheckingConnection = ref(false);
  const connectionError = ref('');
  let heartbeatTimer = null;

  function markPhoneReachable() {
    if (!isPhoneReachable.value) {
      isPhoneReachable.value = true;
      connectionError.value = '';
      if (onReachable) onReachable();
    } else {
      isPhoneReachable.value = true;
      connectionError.value = '';
    }
  }

  function markPhoneUnreachable(err) {
    isPhoneReachable.value = false;
    if (err) {
      connectionError.value = typeof err === 'string' ? err : (err.message || 'Phone unreachable');
    }
  }

  async function checkConnection(quiet) {
    if (isCheckingConnection.value) return;
    if (!quiet) isCheckingConnection.value = true;
    try {
      let signal = undefined;
      let timer = null;
      if (typeof AbortController !== 'undefined') {
        const ctrl = new AbortController();
        timer = setTimeout(() => ctrl.abort(), 3000);
        signal = ctrl.signal;
      }
      const res = await fetch('/api/ping', { method: 'GET', signal, cache: 'no-store' });
      if (timer) clearTimeout(timer);
      if (res.ok) {
        markPhoneReachable();
      } else {
        markPhoneUnreachable(new Error(`HTTP ${res.status}`));
      }
    } catch (err) {
      markPhoneUnreachable(err);
    } finally {
      if (!quiet) isCheckingConnection.value = false;
    }
  }

  function startHeartbeat() {
    if (heartbeatTimer) clearInterval(heartbeatTimer);
    heartbeatTimer = setInterval(() => checkConnection(true), 30000);
  }

  function stopHeartbeat() {
    if (heartbeatTimer) {
      clearInterval(heartbeatTimer);
      heartbeatTimer = null;
    }
  }

  return {
    isPhoneReachable, isCheckingConnection, connectionError,
    markPhoneReachable, markPhoneUnreachable, checkConnection,
    startHeartbeat, stopHeartbeat,
  };
}
