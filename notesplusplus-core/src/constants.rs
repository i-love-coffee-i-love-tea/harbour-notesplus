pub const APP_DIR_NAME: &str = "harbour-notesplusplus";
pub const NOTES_DIR_NAME: &str = "notes";
pub const DB_FILENAME: &str = "notesplusplus.db";
pub const JOURNAL_FILENAME: &str = "journal.adoc";
pub const JOURNAL_TITLE: &str = "Journal";

pub const DEFAULT_SERVER_PORT: u16 = 8080;
pub const DEFAULT_AI_ENDPOINT: &str = "http://localhost:11434";
pub const DEFAULT_AI_MODEL: &str = "llama3.2";
pub const DEFAULT_AI_TIMEOUT_SECS: u64 = 90;

// Session & Cookie constants
pub const SESSION_COOKIE_NAME: &str = "notesplusplus_session";
pub const SESSION_EXPIRY_SECS: u64 = 7 * 24 * 3600; // 7 days
pub const AUTH_CHALLENGE_TTL_SECS: u64 = 60;

// Common API Routes
pub const API_ROUTE_PING: &str = "api/ping";

// Content-Types & MIME types
pub const MIME_JSON: &str = "application/json; charset=utf-8";
pub const MIME_HTML: &str = "text/html; charset=utf-8";
pub const MIME_EVENT_STREAM: &str = "text/event-stream";
pub const MIME_TEXT_PLAIN: &str = "text/plain; charset=utf-8";
