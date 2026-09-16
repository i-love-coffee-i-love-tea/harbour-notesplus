//! C-compatible FFI interface for the Notes Plus core library.
//!
//! Provides `extern "C"` functions consumed by the Sailfish OS C++ bridge
//! (`NotesBridge.cpp`, `AgentBridge.cpp`, `SpeechBridge.cpp`).
//!
//! Memory management:
//! - All string returns (`*mut c_char`) are allocated on the Rust heap and
//!   MUST be freed by calling `notes_core_free_string()`.
//! - Opaque pointers (`*mut AppPaths`, `*mut FfiSearchEngine`, etc.) must be
//!   freed with their respective `notes_core_*_free()` functions.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod agent;
pub mod common;
pub mod consts;
pub mod db;
pub mod export;
pub mod groups;
pub mod journal;
pub mod pages;
pub mod paths;
pub mod search;
pub mod server;
pub mod stt;
pub mod util;

pub use agent::*;
pub use common::*;
pub use consts::*;
pub use db::*;
pub use export::*;
pub use groups::*;
pub use journal::*;
pub use pages::*;
pub use paths::*;
pub use search::*;
pub use server::*;
pub use stt::*;
pub use util::*;
