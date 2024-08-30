use std::sync::OnceLock;

/// Application.
pub mod app;

/// Terminal events handler.
pub mod event;

/// Widget renderer.
pub mod ui;

/// Terminal user interface.
pub mod tui;

/// Event handler.
pub mod handler;

pub mod text_wrap;
pub mod utils;

pub static MAX_PATH_CHARS: OnceLock<usize> = OnceLock::new();
