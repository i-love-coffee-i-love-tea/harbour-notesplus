//! Static Web Assets for Notes++ Web Editor & AI Assistant (Vue 3 ESM, Minimal CSS).

// Force rebuild: code block styles v2
pub const APP_JS: &str = include_str!("../../assets/web/app.js");
pub const VUE_JS: &str = include_str!("../../assets/web/vue.esm-browser.prod.js");
pub const STYLE_CSS: &str = include_str!("../../assets/web/style.css");
pub const INDEX_HTML: &str = include_str!("../../assets/web/index.html");
pub const ICON_PNG: &[u8] = include_bytes!("../../assets/web/icon.png");
