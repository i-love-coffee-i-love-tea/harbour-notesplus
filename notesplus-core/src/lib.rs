pub mod block;
pub mod constants;
pub mod db;
pub mod error;
pub mod ffi;
pub mod inline;
pub mod journal;
pub mod net;
pub mod page;
pub mod group;
pub mod tree;
pub mod xref;
pub mod parser;
pub mod paths;
pub mod repository;
pub mod html;
pub mod search;
pub mod server;
pub mod agent;
pub mod stt;
pub mod escape;
pub mod diagram;
pub mod highlight;
pub mod search_index;

pub use constants::*;
pub use error::NotesError;
pub use paths::AppPaths;
pub use stt::*;

/// Extension trait to recover from poisoned mutexes without repeating the
/// `unwrap_or_else(|e| e.into_inner())` pattern everywhere.
pub trait MutexResultExt<T> {
    /// Acquire the inner value, recovering from poison if necessary.
    fn recover(self) -> T;
}

impl<T> MutexResultExt<T> for Result<T, std::sync::PoisonError<T>> {
    fn recover(self) -> T {
        self.unwrap_or_else(|e| e.into_inner())
    }
}
