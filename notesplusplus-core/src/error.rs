use std::fmt;

/// Unified error type for the entire Notes++ application.
#[derive(Debug)]
pub enum NotesError {
    /// Standard I/O error.
    Io(std::io::Error),
    /// SQLite database error.
    Db(rusqlite::Error),
    /// JSON serialization/deserialization error.
    Json(serde_json::Error),
    /// TLS configuration or certificate error.
    Tls(String),
    /// TCP listener bind failure.
    Bind(String),
    /// HTTP-level error (status codes, download failures, etc.).
    Http(String),
    /// Authentication/authorization error.
    Auth(String),
    /// LLM network connectivity error.
    LlmNetwork(String),
    /// LLM HTTP error with status code and response body.
    LlmHttp { status: u16, body: String },
    /// LLM request timed out.
    LlmTimeout,
    /// LLM returned an invalid or unexpected response.
    LlmResponse(String),
    /// STT model load/lookup/engine error.
    SttModel(String),
    /// STT audio format or decoding error.
    SttAudio(String),
    /// STT inference error.
    SttInference(String),
    /// STT model file checksum mismatch.
    SttChecksum { expected: String, actual: String },
    /// STT download was cancelled.
    SttCancelled,
    /// Generic string error message.
    Msg(String),
}

impl fmt::Display for NotesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotesError::Io(e) => write!(f, "{}", e),
            NotesError::Db(e) => write!(f, "{}", e),
            NotesError::Json(e) => write!(f, "{}", e),
            NotesError::Tls(s) => write!(f, "TLS error: {}", s),
            NotesError::Bind(s) => write!(f, "Bind error: {}", s),
            NotesError::Http(s) => write!(f, "HTTP error: {}", s),
            NotesError::Auth(s) => write!(f, "Auth error: {}", s),
            NotesError::LlmNetwork(s) => write!(f, "LLM network error: {}", s),
            NotesError::LlmHttp { status, body } => {
                write!(f, "LLM HTTP error ({}): {}", status, body)
            }
            NotesError::LlmTimeout => write!(f, "Timeout waiting for LLM response"),
            NotesError::LlmResponse(s) => write!(f, "Invalid LLM response: {}", s),
            NotesError::SttModel(s) => write!(f, "STT model error: {}", s),
            NotesError::SttAudio(s) => write!(f, "STT audio error: {}", s),
            NotesError::SttInference(s) => write!(f, "STT inference error: {}", s),
            NotesError::SttChecksum { expected, actual } => {
                write!(
                    f,
                    "Checksum mismatch: expected {}, calculated {}",
                    expected, actual
                )
            }
            NotesError::SttCancelled => write!(f, "Download cancelled"),
            NotesError::Msg(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for NotesError {}

impl From<std::io::Error> for NotesError {
    fn from(e: std::io::Error) -> Self {
        NotesError::Io(e)
    }
}

impl From<rusqlite::Error> for NotesError {
    fn from(e: rusqlite::Error) -> Self {
        NotesError::Db(e)
    }
}

impl From<serde_json::Error> for NotesError {
    fn from(e: serde_json::Error) -> Self {
        NotesError::Json(e)
    }
}

impl From<String> for NotesError {
    fn from(s: String) -> Self {
        NotesError::Msg(s)
    }
}

impl From<&str> for NotesError {
    fn from(s: &str) -> Self {
        NotesError::Msg(s.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApiErrorCode {
    Unauthorized,
    Forbidden,
    NotFound,
    BadRequest,
    MethodNotAllowed,
    InternalError,
    RateLimited,
    AiError,
}

impl ApiErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiErrorCode::Unauthorized => "ERR_UNAUTHORIZED",
            ApiErrorCode::Forbidden => "ERR_FORBIDDEN",
            ApiErrorCode::NotFound => "ERR_NOT_FOUND",
            ApiErrorCode::BadRequest => "ERR_BAD_REQUEST",
            ApiErrorCode::MethodNotAllowed => "ERR_METHOD_NOT_ALLOWED",
            ApiErrorCode::InternalError => "ERR_INTERNAL",
            ApiErrorCode::RateLimited => "ERR_RATE_LIMITED",
            ApiErrorCode::AiError => "ERR_AI",
        }
    }

    pub fn status_code(&self) -> u16 {
        match self {
            ApiErrorCode::Unauthorized => 401,
            ApiErrorCode::Forbidden => 403,
            ApiErrorCode::NotFound => 404,
            ApiErrorCode::BadRequest => 400,
            ApiErrorCode::MethodNotAllowed => 405,
            ApiErrorCode::InternalError => 500,
            ApiErrorCode::RateLimited => 429,
            ApiErrorCode::AiError => 502,
        }
    }

    pub fn reason(&self) -> &'static str {
        match self {
            ApiErrorCode::Unauthorized => "Unauthorized",
            ApiErrorCode::Forbidden => "Forbidden",
            ApiErrorCode::NotFound => "Not Found",
            ApiErrorCode::BadRequest => "Bad Request",
            ApiErrorCode::MethodNotAllowed => "Method Not Allowed",
            ApiErrorCode::InternalError => "Internal Server Error",
            ApiErrorCode::RateLimited => "Too Many Requests",
            ApiErrorCode::AiError => "Bad Gateway",
        }
    }
}

impl fmt::Display for ApiErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
