// Thin fetch wrapper with shared error handling.
// Stores import this instead of calling raw fetch().

const healthCallbacks = { markPhoneReachable: null, markPhoneUnreachable: null };

export function setHealthCallbacks(reachable, unreachable) {
  healthCallbacks.markPhoneReachable = reachable;
  healthCallbacks.markPhoneUnreachable = unreachable;
}

function markOk() {
  if (healthCallbacks.markPhoneReachable) healthCallbacks.markPhoneReachable();
}

function markErr(err) {
  if (healthCallbacks.markPhoneUnreachable) healthCallbacks.markPhoneUnreachable(err);
}

// Auth gate: blocks API calls to protected endpoints until auth flow completes.
// Prevents stores from leaking document content or making wasted requests before login.
let authReady = false;

const PUBLIC_API_ROUTES = new Set([
  '/api/ping',
  '/api/auth/config',
  '/api/auth/logout',
  '/api/auth/whoami',
  '/api/auth/code/initiate',
  '/api/auth/code/status',
  '/api/theme',
]);

function isPublicRoute(url) {
  const path = url.split('?')[0];
  return PUBLIC_API_ROUTES.has(path) || !path.startsWith('/api/');
}

export function setAuthReady(ready = true) {
  authReady = ready;
}

export function checkAuthReady() {
  return authReady;
}

export async function apiFetch(url, options = {}) {
  if (!authReady && !isPublicRoute(url)) {
    return new Response(null, { status: 401, statusText: 'Auth not ready' });
  }
  try {
    const res = await fetch(url, options);
    if (res.ok) {
      markOk();
      return res;
    }
    markErr(new Error(`HTTP ${res.status}`));
    return res;
  } catch (err) {
    markErr(err);
    throw err;
  }
}

export async function apiJson(url, options = {}) {
  const res = await apiFetch(url, options);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function apiText(url, options = {}) {
  const res = await apiFetch(url, options);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.text();
}
