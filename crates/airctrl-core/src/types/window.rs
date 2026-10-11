use serde::{Deserialize, Serialize};

/// 2D rectangular bounds of an operating system window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Metadata describing an enumerated open window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowInfo {
    /// Unique OS-level window identifier (HWND on Windows, XID on X11, WindowID on macOS)
    pub id: u64,
    /// Owning process identifier
    pub process_id: u32,
    /// Process binary name (e.g. "code.exe", "ms-teams.exe")
    pub process_name: String,
    /// Window title text
    pub title: String,
    /// Window geometry on the virtual desktop
    pub bounds: WindowRect,
    /// Whether this window currently has foreground focus
    pub is_focused: bool,
    /// Whether the window is currently minimized to the taskbar/dock
    pub is_minimized: bool,
}

/// Rich, comprehensive window details extracted by the crawler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowDetails {
    /// Unique OS-level window identifier (HWND on Windows, XID on X11, WindowID on macOS)
    pub id: u64,
    /// Owning process identifier
    pub process_id: u32,
    /// Process binary name (e.g. "code.exe", "ms-teams.exe")
    pub process_name: String,
    /// Window title text
    pub title: String,
    /// Window class name (e.g. "Chrome_WidgetWin_1", "Notepad", or X11 WM_CLASS)
    pub class_name: String,
    /// Window geometry on the virtual desktop
    pub bounds: WindowRect,
    /// Whether the window currently has foreground focus
    pub is_focused: bool,
    /// Whether the window is currently minimized to taskbar/dock
    pub is_minimized: bool,
    /// Whether the window is currently maximized to fill the display
    pub is_maximized: bool,
}

impl From<WindowInfo> for WindowDetails {
    fn from(w: WindowInfo) -> Self {
        Self {
            id: w.id,
            process_id: w.process_id,
            process_name: w.process_name,
            title: w.title,
            class_name: String::new(),
            bounds: w.bounds,
            is_focused: w.is_focused,
            is_minimized: w.is_minimized,
            is_maximized: false,
        }
    }
}

impl From<WindowDetails> for WindowInfo {
    fn from(d: WindowDetails) -> Self {
        Self {
            id: d.id,
            process_id: d.process_id,
            process_name: d.process_name,
            title: d.title,
            bounds: d.bounds,
            is_focused: d.is_focused,
            is_minimized: d.is_minimized,
        }
    }
}

/// Snapshot metadata describing a running operating system process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub exe_path: Option<String>,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub window_ids: Vec<u64>,
}

/// Classification of how a window was matched against a query string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchType {
    ExactProcess,
    ExactTitle,
    SubstringProcess,
    SubstringTitle,
    TokenOverlap,
    FuzzyTitle,
    FuzzyProcess,
}

/// Candidate result from matching a search query against active windows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowMatch {
    pub window: WindowDetails,
    pub match_type: MatchType,
    pub score: f32,
}
