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

export async function apiFetch(url, options = {}) {
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
