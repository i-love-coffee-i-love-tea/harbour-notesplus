// vue-demi shim for Vue 3 — Pinia 2.x imports vue-demi for Vue 2/3 compat.
// With Vue 3 we just re-export everything from vue and add the compat markers.
export * from 'vue';
export const isVue2 = false;
export function set(target, key, val) { target[key] = val; }
export function del(target, key) { delete target[key]; }
