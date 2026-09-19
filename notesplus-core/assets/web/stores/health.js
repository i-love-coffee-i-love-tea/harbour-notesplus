import { defineStore } from 'pinia';
import { ref } from 'vue';
import { apiFetch } from './api.js';

export const useHealthStore = defineStore('health', () => {
  const isPhoneReachable = ref(true);
  const isCheckingConnection = ref(false);
  const connectionError = ref('');

  function markPhoneReachable() {
    isPhoneReachable.value = true;
    connectionError.value = '';
  }

  function markPhoneUnreachable(err) {
    isPhoneReachable.value = false;
    if (err) connectionError.value = typeof err === 'string' ? err : (err.message || 'Phone unreachable');
  }

  async function checkConnection(quiet) {
    if (isCheckingConnection.value) return;
    if (!quiet) isCheckingConnection.value = true;
    try {
      let signal, timer;
      if (typeof AbortController !== 'undefined') {
        const ctrl = new AbortController();
        timer = setTimeout(() => ctrl.abort(), 3000);
        signal = ctrl.signal;
      }
      const res = await fetch('/api/ping', { method: 'GET', signal, cache: 'no-store' });
      if (timer) clearTimeout(timer);
      if (res.ok) markPhoneReachable();
      else markPhoneUnreachable(new Error(`HTTP ${res.status}`));
    } catch (err) {
      markPhoneUnreachable(err);
    } finally {
      if (!quiet) isCheckingConnection.value = false;
    }
  }

  return {
    isPhoneReachable, isCheckingConnection, connectionError,
    markPhoneReachable, markPhoneUnreachable, checkConnection,
  };
});
