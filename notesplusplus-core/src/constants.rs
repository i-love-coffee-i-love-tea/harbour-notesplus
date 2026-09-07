pub const APP_DIR_NAME: &str = "harbour-notesplusplus";
pub const NOTES_DIR_NAME: &str = "notes";
pub const DB_FILENAME: &str = "notesplusplus.db";
pub const JOURNAL_FILENAME: &str = "journal.adoc";
pub const JOURNAL_TITLE: &str = "Journal";

pub const DEFAULT_SERVER_PORT: u16 = 8080;
pub const DEFAULT_AI_ENDPOINT: &str = "http://localhost:11434";
pub const DEFAULT_AI_MODEL: &str = "llama3.2";
pub const DEFAULT_AI_TIMEOUT_SECS: u64 = 90;
pub const DEFAULT_MAX_TOKENS: u32 = 4096;
pub const DEFAULT_TEMPERATURE: f32 = 0.7;

// Session & Cookie constants
pub const SESSION_COOKIE_NAME: &str = "notesplusplus_session";
pub const SESSION_EXPIRY_SECS: u64 = 7 * 24 * 3600; // 7 days

// Common API Routes
pub const API_ROUTE_PING: &str = "api/ping";
pub const API_ROUTE_AUTH_CONFIG: &str = "api/auth/config";
pub const API_ROUTE_AUTH_LOGIN: &str = "api/auth/login";
pub const API_ROUTE_AUTH_LOGOUT: &str = "api/auth/logout";
pub const API_ROUTE_AUTH_WHOAMI: &str = "api/auth/whoami";
pub const API_ROUTE_AUTH_OAUTH_START: &str = "api/auth/oauth/start";
pub const API_ROUTE_AUTH_OAUTH_LOGIN: &str = "api/auth/oauth/login";
pub const API_ROUTE_AUTH_OAUTH_CALLBACK: &str = "api/auth/oauth/callback";
pub const API_ROUTE_PAGES: &str = "api/pages";
pub const API_ROUTE_SEARCH: &str = "api/search";
pub const API_ROUTE_NOTES: &str = "api/notes";
pub const API_ROUTE_AGENT_CHAT: &str = "api/agent/chat";
pub const API_ROUTE_AGENT_STATUS: &str = "api/agent/status";
pub const API_ROUTE_AGENT_MODELS: &str = "api/agent/models";
pub const API_ROUTE_AGENT_DECIDE: &str = "api/agent/decide";
pub const API_ROUTE_EXPORT_HTML: &str = "api/export/html";
pub const API_ROUTE_EXPORT_ALL: &str = "api/export/all";
pub const API_ROUTE_CERT_INSTALL: &str = "api/cert/install";
pub const API_ROUTE_CERT_STATUS: &str = "api/cert/status";

// Content-Types & MIME types
pub const MIME_JSON: &str = "application/json; charset=utf-8";
pub const MIME_HTML: &str = "text/html; charset=utf-8";
pub const MIME_EVENT_STREAM: &str = "text/event-stream";
pub const MIME_TEXT_PLAIN: &str = "text/plain; charset=utf-8";
