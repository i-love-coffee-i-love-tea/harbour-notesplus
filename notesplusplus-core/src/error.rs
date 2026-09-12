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
