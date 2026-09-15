pub const APP_DIR_NAME: &str = "harbour-notesplusplus";
pub const NOTES_DIR_NAME: &str = "notes";
pub const ASSETS_DIR_NAME: &str = "assets";
pub const DB_FILENAME: &str = "notesplusplus.db";
pub const JOURNAL_FILENAME: &str = "journal.adoc";
pub const JOURNAL_TITLE: &str = "Journal";

pub const DEFAULT_SERVER_PORT: u16 = 8080;
pub const DEFAULT_AI_ENDPOINT: &str = "http://localhost:11434";
pub const DEFAULT_AI_MODEL: &str = "llama3.2";
pub const DEFAULT_AI_TIMEOUT_SECS: u64 = 90;
pub const FETCH_URL_TIMEOUT_SECS: u64 = 20;
pub const LLM_CONNECT_TIMEOUT_SECS: u64 = 10;
pub const LLM_WRITE_TIMEOUT_SECS: u64 = 30;
pub const MAX_CONCURRENT_CONNECTIONS: usize = 16;
pub const PORT_SCAN_RANGE: u16 = 20;

// Session & Cookie constants
pub const SESSION_COOKIE_NAME: &str = "notesplusplus_session";
pub const SESSION_EXPIRY_SECS: u64 = 7 * 24 * 3600; // 7 days
pub const AUTH_CHALLENGE_TTL_SECS: u64 = 60;

// Common API Routes
pub const API_ROUTE_PING: &str = "api/ping";
pub const API_ROUTE_AUTH_CONFIG: &str = "api/auth/config";
pub const API_ROUTE_AUTH_LOGOUT: &str = "api/auth/logout";
pub const API_ROUTE_AUTH_WHOAMI: &str = "api/auth/whoami";
pub const API_ROUTE_AUTH_CODE_INITIATE: &str = "api/auth/code/initiate";
pub const API_ROUTE_AUTH_CODE_STATUS: &str = "api/auth/code/status";
pub const API_ROUTE_THEME: &str = "api/theme";
pub const API_ROUTE_SEARCH: &str = "api/search";
pub const API_ROUTE_PAGES: &str = "api/pages";
pub const API_ROUTE_RENDER: &str = "api/render";
pub const API_ROUTE_BLOCKS_PARSE: &str = "api/blocks/parse";
pub const API_ROUTE_BLOCKS_TO_ADOC: &str = "api/blocks/to_adoc";
pub const API_ROUTE_JOURNAL_TODAY: &str = "api/journal/today";
pub const API_ROUTE_JOURNAL_TASK: &str = "api/journal/task";
pub const API_ROUTE_AI_STATUS: &str = "api/ai/status";
pub const API_ROUTE_AI_MODELS: &str = "api/ai/models";
pub const API_ROUTE_AI_CONFIG: &str = "api/ai/config";
pub const API_ROUTE_AI_CHAT: &str = "api/ai/chat";
pub const API_ROUTE_AI_TEMPLATE: &str = "api/ai/template";
pub const API_ROUTE_AI_CONFIRM: &str = "api/ai/confirm";
pub const API_ROUTE_AI_UNDO: &str = "api/ai/undo";
pub const API_ROUTE_AI_FETCH_URL: &str = "api/ai/fetch_url";
pub const API_ROUTE_AI_PREPROCESS_HTML: &str = "api/ai/preprocess_html";
pub const API_ROUTE_AI_READ_FILE: &str = "api/ai/read_file";
pub const API_ROUTE_AI_BACKUPS: &str = "api/ai/backups";

// Content-Types & MIME types
pub const MIME_JSON: &str = "application/json; charset=utf-8";
pub const MIME_HTML: &str = "text/html; charset=utf-8";
pub const MIME_EVENT_STREAM: &str = "text/event-stream";
pub const MIME_TEXT_PLAIN: &str = "text/plain; charset=utf-8";
