use std::fmt;

#[derive(Debug)]
pub enum CoreError {
    Io(std::io::Error),
    Db(rusqlite::Error),
    Json(serde_json::Error),
    Msg(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Io(e) => write!(f, "{}", e),
            CoreError::Db(e) => write!(f, "{}", e),
            CoreError::Json(e) => write!(f, "{}", e),
            CoreError::Msg(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for CoreError {}

impl From<std::io::Error> for CoreError {
    fn from(e: std::io::Error) -> Self { CoreError::Io(e) }
}

impl From<rusqlite::Error> for CoreError {
    fn from(e: rusqlite::Error) -> Self { CoreError::Db(e) }
}

impl From<serde_json::Error> for CoreError {
    fn from(e: serde_json::Error) -> Self { CoreError::Json(e) }
}

impl From<String> for CoreError {
    fn from(s: String) -> Self { CoreError::Msg(s) }
}

impl From<&str> for CoreError {
    fn from(s: &str) -> Self { CoreError::Msg(s.to_string()) }
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
