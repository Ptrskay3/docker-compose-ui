use std::sync::OnceLock;

pub mod app;
pub mod event;
pub mod handler;
pub mod text_wrap;
pub mod tui;
pub mod ui;
pub mod utils;

/// Maximum number of characters in a path before starting to truncate it.
pub static MAX_PATH_CHARS: OnceLock<usize> = OnceLock::new();
/// Whether the light mode is enabled.
pub static LIGHT_MODE: OnceLock<bool> = OnceLock::new();
