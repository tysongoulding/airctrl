//! airctrl Tauri 2.0 shell library
pub mod commands;
pub mod ipc;
pub mod state;

pub use ipc::{IpcBridge, IpcError};
pub use state::AppState;
