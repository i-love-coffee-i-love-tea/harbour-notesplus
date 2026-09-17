//! Static Web Assets for Notes++ Web Editor & AI Assistant (Vue 3 ESM, Minimal CSS).

pub const APP_JS: &str = include_str!("../../assets/web/app.js");
pub const VUE_JS: &str = include_str!("../../assets/web/vue.esm-browser.prod.js");
pub const VUE_DEMI_JS: &str = include_str!("../../assets/web/vue-demi.esm-browser.js");
pub const PINIA_JS: &str = include_str!("../../assets/web/pinia.esm-browser.prod.js");
pub const STYLE_CSS: &str = include_str!("../../assets/web/style.css");
pub const INDEX_HTML: &str = include_str!("../../assets/web/index.html");
pub const ICON_PNG: &[u8] = include_bytes!("../../assets/web/icon.png");

// Store modules
pub const STORES_INDEX_JS: &str = include_str!("../../assets/web/stores/index.js");
pub const STORES_API_JS: &str = include_str!("../../assets/web/stores/api.js");
pub const STORES_NOTES_JS: &str = include_str!("../../assets/web/stores/notes.js");
pub const STORES_AUTH_JS: &str = include_str!("../../assets/web/stores/auth.js");
pub const STORES_THEME_JS: &str = include_str!("../../assets/web/stores/theme.js");
pub const STORES_HEALTH_JS: &str = include_str!("../../assets/web/stores/health.js");
pub const STORES_AI_JS: &str = include_str!("../../assets/web/stores/ai.js");
pub const STORES_UI_JS: &str = include_str!("../../assets/web/stores/ui.js");
pub const STORES_PRESENTATION_STORE_JS: &str = include_str!("../../assets/web/stores/presentationStore.js");
pub const STORES_IMPORT_STORE_JS: &str = include_str!("../../assets/web/stores/importStore.js");
pub const STORES_LINK_STORE_JS: &str = include_str!("../../assets/web/stores/linkStore.js");
pub const STORES_EDITOR_STORE_JS: &str = include_str!("../../assets/web/stores/editorStore.js");

// Shared utilities (still used by stores)
pub const COMPOSABLE_UTILS_JS: &str = include_str!("../../assets/web/composables/utils.js");

// Shims for Pinia bare-specifier dependencies
pub const VUE_DEVTOOLS_API_JS: &str = include_str!("../../assets/web/vue-devtools-api-stub.js");
