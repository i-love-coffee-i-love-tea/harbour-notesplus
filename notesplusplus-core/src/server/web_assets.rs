//! Static Web Assets for Notes++ Web Editor & AI Assistant (Vue 3 ESM, Minimal CSS).

// Force rebuild: code block styles v2
pub const APP_JS: &str = include_str!("../../assets/web/app.js");
pub const VUE_JS: &str = include_str!("../../assets/web/vue.esm-browser.prod.js");
pub const STYLE_CSS: &str = include_str!("../../assets/web/style.css");
pub const INDEX_HTML: &str = include_str!("../../assets/web/index.html");
pub const ICON_PNG: &[u8] = include_bytes!("../../assets/web/icon.png");

// Composable modules
pub const COMPOSABLE_UTILS_JS: &str = include_str!("../../assets/web/composables/utils.js");
pub const COMPOSABLE_USE_AUTH_JS: &str = include_str!("../../assets/web/composables/useAuth.js");
pub const COMPOSABLE_USE_HEALTH_CHECK_JS: &str = include_str!("../../assets/web/composables/useHealthCheck.js");
pub const COMPOSABLE_USE_PRESENTATION_JS: &str = include_str!("../../assets/web/composables/usePresentation.js");
pub const COMPOSABLE_USE_LINK_MODAL_JS: &str = include_str!("../../assets/web/composables/useLinkModal.js");
pub const COMPOSABLE_USE_AI_ASSISTANT_JS: &str = include_str!("../../assets/web/composables/useAiAssistant.js");
pub const COMPOSABLE_USE_IMPORT_JS: &str = include_str!("../../assets/web/composables/useImport.js");
