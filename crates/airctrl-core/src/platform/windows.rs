use super::traits::{PlatformAudioOps, PlatformInputOps, PlatformWindowOps};
use crate::error::CoreError;
use crate::types::*;

#[cfg(target_os = "windows")]
mod win32 {
    use std::ffi::c_void;

    pub type HWND = isize;
    pub type BOOL = i32;
    pub type UINT = u32;
    pub type DWORD = u32;
    pub type WPARAM = usize;
    pub type LPARAM = isize;
    pub type HANDLE = *mut c_void;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct RECT {
        pub left: i32,
        pub top: i32,
        pub right: i32,
        pub bottom: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct POINT {
        pub x: i32,
        pub y: i32,
    }

    pub const SW_MAXIMIZE: i32 = 3;
    pub const SW_MINIMIZE: i32 = 6;
    pub const SW_RESTORE: i32 = 9;
    pub const WM_CLOSE: UINT = 0x0010;
    pub const SPI_GETWORKAREA: UINT = 0x0030;

    pub const KEYEVENTF_KEYUP: DWORD = 0x0002;
    pub const VK_BACK: u8 = 0x08;
    pub const VK_TAB: u8 = 0x09;
    pub const VK_RETURN: u8 = 0x0D;
    pub const VK_SHIFT: u8 = 0x10;
    pub const VK_CONTROL: u8 = 0x11;
    pub const VK_MENU: u8 = 0x12; // Alt
    pub const VK_ESCAPE: u8 = 0x1B;
    pub const VK_SPACE: u8 = 0x20;
    pub const VK_LEFT: u8 = 0x25;
    pub const VK_UP: u8 = 0x26;
    pub const VK_RIGHT: u8 = 0x27;
    pub const VK_DOWN: u8 = 0x28;
    pub const VK_DELETE: u8 = 0x2E;
    pub const VK_LWIN: u8 = 0x5B;

    pub const MOUSEEVENTF_LEFTDOWN: DWORD = 0x0002;
    pub const MOUSEEVENTF_LEFTUP: DWORD = 0x0004;
    pub const MOUSEEVENTF_RIGHTDOWN: DWORD = 0x0008;
    pub const MOUSEEVENTF_RIGHTUP: DWORD = 0x0010;
    pub const MOUSEEVENTF_MIDDLEDOWN: DWORD = 0x0020;
    pub const MOUSEEVENTF_MIDDLEUP: DWORD = 0x0040;
    pub const MOUSEEVENTF_WHEEL: DWORD = 0x0800;
    pub const MOUSEEVENTF_HWHEEL: DWORD = 0x01000;

    pub const CF_UNICODETEXT: UINT = 13;
    pub const GMEM_MOVEABLE: UINT = 0x0002;

    pub type WNDENUMPROC = unsafe extern "system" fn(HWND, LPARAM) -> BOOL;

    #[link(name = "user32")]
    extern "system" {
        pub fn GetForegroundWindow() -> HWND;
        pub fn GetWindowRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
        pub fn GetWindowTextW(hWnd: HWND, lpString: *mut u16, nMaxCount: i32) -> i32;
        pub fn GetWindowThreadProcessId(hWnd: HWND, lpdwProcessId: *mut DWORD) -> DWORD;
        pub fn IsWindowVisible(hWnd: HWND) -> BOOL;
        pub fn IsIconic(hWnd: HWND) -> BOOL;
        pub fn IsZoomed(hWnd: HWND) -> BOOL;
        pub fn MoveWindow(
            hWnd: HWND,
            X: i32,
            Y: i32,
            nWidth: i32,
            nHeight: i32,
            bRepaint: BOOL,
        ) -> BOOL;
        pub fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;
        pub fn SetForegroundWindow(hWnd: HWND) -> BOOL;
        pub fn PostMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> BOOL;
        pub fn SystemParametersInfoW(
            uiAction: UINT,
            uiParam: UINT,
            pvParam: *mut c_void,
            fWinIni: UINT,
        ) -> BOOL;
        pub fn EnumWindows(lpEnumFunc: WNDENUMPROC, lParam: LPARAM) -> BOOL;

        pub fn SetCursorPos(X: i32, Y: i32) -> BOOL;
        pub fn GetCursorPos(lpPoint: *mut POINT) -> BOOL;
        pub fn mouse_event(
            dwFlags: DWORD,
            dx: DWORD,
            dy: DWORD,
            dwData: DWORD,
            dwExtraInfo: usize,
        );
        pub fn keybd_event(bVk: u8, bScan: u8, dwFlags: DWORD, dwExtraInfo: usize);

        pub fn OpenClipboard(hWndNewOwner: HWND) -> BOOL;
        pub fn CloseClipboard() -> BOOL;
        pub fn EmptyClipboard() -> BOOL;
        pub fn SetClipboardData(uFormat: UINT, hMem: HANDLE) -> HANDLE;
        pub fn GetClipboardData(uFormat: UINT) -> HANDLE;
    }

    #[link(name = "kernel32")]
    extern "system" {
        pub fn GlobalAlloc(uFlags: UINT, dwBytes: usize) -> HANDLE;
        pub fn GlobalLock(hMem: HANDLE) -> *mut c_void;
        pub fn GlobalUnlock(hMem: HANDLE) -> BOOL;
    }
}

pub struct WindowsWindowOps;

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_proc(
    hwnd: win32::HWND,
    lparam: win32::LPARAM,
) -> win32::BOOL {
    if win32::IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    let mut title_buf = [0u16; 256];
    let len = win32::GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32);
    if len <= 0 {
        return 1;
    }
    let title = String::from_utf16_lossy(&title_buf[..len as usize]);
    if title.trim().is_empty() || title == "Program Manager" {
        return 1;
    }

    let mut rect = win32::RECT::default();
    win32::GetWindowRect(hwnd, &mut rect);
    let width = (rect.right - rect.left).max(0) as u32;
    let height = (rect.bottom - rect.top).max(0) as u32;
    if width < 40 || height < 40 {
        return 1;
    }

    let mut pid: win32::DWORD = 0;
    win32::GetWindowThreadProcessId(hwnd, &mut pid);
    let fg = win32::GetForegroundWindow();

    let list = &mut *(lparam as *mut Vec<WindowInfo>);
    list.push(WindowInfo {
        id: hwnd as u64,
        process_id: pid,
        process_name: String::new(),
        title,
        bounds: WindowRect {
            x: rect.left,
            y: rect.top,
            width,
            height,
        },
        is_focused: hwnd == fg,
        is_minimized: win32::IsIconic(hwnd) != 0,
    });
    1
}

impl PlatformWindowOps for WindowsWindowOps {
    fn get_active_window(&self) -> Result<WindowInfo, CoreError> {
        #[cfg(target_os = "windows")]
        unsafe {
            let hwnd = win32::GetForegroundWindow();
            if hwnd == 0 {
                return Ok(WindowInfo {
                    id: 1,
                    process_id: std::process::id(),
                    process_name: "airctrl.exe".to_string(),
                    title: "airctrl Windows Host".to_string(),
                    bounds: WindowRect {
                        x: 0,
                        y: 0,
                        width: 1920,
                        height: 1080,
                    },
                    is_focused: true,
                    is_minimized: false,
                });
            }

            let mut rect = win32::RECT::default();
            win32::GetWindowRect(hwnd, &mut rect);
            let mut title_buf = [0u16; 256];
            let len = win32::GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32);
            let title = if len > 0 {
                String::from_utf16_lossy(&title_buf[..len as usize])
            } else {
                "Active Window".to_string()
            };
            let mut pid: win32::DWORD = 0;
            win32::GetWindowThreadProcessId(hwnd, &mut pid);

            Ok(WindowInfo {
                id: hwnd as u64,
                process_id: pid,
                process_name: String::new(),
                title,
                bounds: WindowRect {
                    x: rect.left,
                    y: rect.top,
                    width: (rect.right - rect.left).max(100) as u32,
                    height: (rect.bottom - rect.top).max(100) as u32,
                },
                is_focused: true,
                is_minimized: win32::IsIconic(hwnd) != 0,
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(CoreError::PlatformUnsupported(
                "WindowsWindowOps requires Windows OS".into(),
            ))
        }
    }

    fn get_work_area(&self) -> Result<WindowRect, CoreError> {
        #[cfg(target_os = "windows")]
        unsafe {
            let mut rect = win32::RECT::default();
            let ok = win32::SystemParametersInfoW(
                win32::SPI_GETWORKAREA,
                0,
                (&mut rect as *mut win32::RECT).cast(),
                0,
            );
            if ok != 0 && rect.right > rect.left && rect.bottom > rect.top {
                return Ok(WindowRect {
                    x: rect.left,
                    y: rect.top,
                    width: (rect.right - rect.left) as u32,
                    height: (rect.bottom - rect.top) as u32,
                });
            }
        }
        Ok(WindowRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        })
    }

    fn focus_window(&self, id: u64) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        unsafe {
            let hwnd = id as win32::HWND;
            if hwnd != 0 {
                if win32::IsIconic(hwnd) != 0 {
                    win32::ShowWindow(hwnd, win32::SW_RESTORE);
                }
                win32::SetForegroundWindow(hwnd);
            }
        }
        let _ = id;
        Ok(())
    }

    fn enumerate_windows(&self) -> Result<Vec<WindowInfo>, CoreError> {
        #[cfg(target_os = "windows")]
        unsafe {
            let mut windows: Vec<WindowInfo> = Vec::new();
            win32::EnumWindows(
                enum_windows_proc,
                (&mut windows as *mut Vec<WindowInfo>) as win32::LPARAM,
            );
            return Ok(windows);
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(vec![])
        }
    }

    fn minimize_window(&self, id: u64) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        unsafe {
            let hwnd = if id <= 1 {
                win32::GetForegroundWindow()
            } else {
                id as win32::HWND
            };
            if hwnd != 0 {
                win32::ShowWindow(hwnd, win32::SW_MINIMIZE);
            }
        }
        let _ = id;
        Ok(())
    }

    fn maximize_window(&self, id: u64) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        unsafe {
            let hwnd = if id <= 1 {
                win32::GetForegroundWindow()
            } else {
                id as win32::HWND
            };
            if hwnd != 0 {
                win32::ShowWindow(hwnd, win32::SW_MAXIMIZE);
            }
        }
        let _ = id;
        Ok(())
    }

    fn restore_window(&self, id: u64) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        unsafe {
            let hwnd = if id <= 1 {
                win32::GetForegroundWindow()
            } else {
                id as win32::HWND
            };
            if hwnd != 0 {
                win32::ShowWindow(hwnd, win32::SW_RESTORE);
            }
        }
        let _ = id;
        Ok(())
    }

    fn set_window_bounds(&self, id: u64, bounds: WindowRect) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        unsafe {
            let hwnd = if id <= 1 {
                win32::GetForegroundWindow()
            } else {
                id as win32::HWND
            };
            if hwnd != 0 {
                if win32::IsZoomed(hwnd) != 0 || win32::IsIconic(hwnd) != 0 {
                    win32::ShowWindow(hwnd, win32::SW_RESTORE);
                }
                win32::MoveWindow(
                    hwnd,
                    bounds.x,
                    bounds.y,
                    bounds.width.max(120) as i32,
                    bounds.height.max(80) as i32,
                    1,
                );
            }
        }
        let _ = (id, bounds);
        Ok(())
    }

    fn center_window(&self, id: u64) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        unsafe {
            let hwnd = if id <= 1 {
                win32::GetForegroundWindow()
            } else {
                id as win32::HWND
            };
            if hwnd != 0 {
                if win32::IsZoomed(hwnd) != 0 || win32::IsIconic(hwnd) != 0 {
                    win32::ShowWindow(hwnd, win32::SW_RESTORE);
                }
                let work = self.get_work_area().unwrap_or(WindowRect {
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                });
                let mut rect = win32::RECT::default();
                win32::GetWindowRect(hwnd, &mut rect);
                let w = (rect.right - rect.left).clamp(400, work.width as i32);
                let h = (rect.bottom - rect.top).clamp(300, work.height as i32);
                let x = work.x + ((work.width as i32 - w) / 2).max(0);
                let y = work.y + ((work.height as i32 - h) / 2).max(0);
                win32::MoveWindow(hwnd, x, y, w, h, 1);
            }
        }
        let _ = id;
        Ok(())
    }

    fn close_window(&self, id: u64) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        unsafe {
            let hwnd = if id <= 1 {
                win32::GetForegroundWindow()
            } else {
                id as win32::HWND
            };
            if hwnd != 0 {
                win32::PostMessageW(hwnd, win32::WM_CLOSE, 0, 0);
            }
        }
        let _ = id;
        Ok(())
    }
}

pub struct WindowsInputOps;

#[cfg(target_os = "windows")]
impl WindowsInputOps {
    fn modifier_to_vk(m: ModifierKey) -> u8 {
        match m {
            ModifierKey::Control => win32::VK_CONTROL,
            ModifierKey::Alt => win32::VK_MENU,
            ModifierKey::Shift => win32::VK_SHIFT,
            ModifierKey::Super => win32::VK_LWIN,
        }
    }

    fn key_to_vk(k: &InputKey) -> u8 {
        match k {
            InputKey::Char(c) => {
                let up = c.to_ascii_uppercase();
                if up.is_ascii_alphanumeric() {
                    up as u8
                } else {
                    win32::VK_SPACE
                }
            }
            InputKey::Space => win32::VK_SPACE,
            InputKey::Enter => win32::VK_RETURN,
            InputKey::Escape => win32::VK_ESCAPE,
            InputKey::Tab => win32::VK_TAB,
            InputKey::Backspace => win32::VK_BACK,
            InputKey::Delete => win32::VK_DELETE,
            InputKey::ArrowUp => win32::VK_UP,
            InputKey::ArrowDown => win32::VK_DOWN,
            InputKey::ArrowLeft => win32::VK_LEFT,
            InputKey::ArrowRight => win32::VK_RIGHT,
            InputKey::FKey(n) => 0x6F + n.clamp(&1, &12),
            InputKey::Raw(code) => (*code & 0xFF) as u8,
        }
    }
}

impl PlatformInputOps for WindowsInputOps {
    fn set_clipboard_text(&self, text: &str) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        unsafe {
            if win32::OpenClipboard(0) != 0 {
                win32::EmptyClipboard();
                let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
                let bytes = utf16.len() * std::mem::size_of::<u16>();
                let hmem = win32::GlobalAlloc(win32::GMEM_MOVEABLE, bytes);
                if !hmem.is_null() {
                    let ptr = win32::GlobalLock(hmem) as *mut u16;
                    if !ptr.is_null() {
                        std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());
                        win32::GlobalUnlock(hmem);
                        win32::SetClipboardData(win32::CF_UNICODETEXT, hmem);
                    }
                }
                win32::CloseClipboard();
            }
        }
        let _ = text;
        Ok(())
    }

    fn get_clipboard_text(&self) -> Result<Option<String>, CoreError> {
        #[cfg(target_os = "windows")]
        unsafe {
            let mut result = None;
            if win32::OpenClipboard(0) != 0 {
                let hmem = win32::GetClipboardData(win32::CF_UNICODETEXT);
                if !hmem.is_null() {
                    let ptr = win32::GlobalLock(hmem) as *const u16;
                    if !ptr.is_null() {
                        let mut len = 0usize;
                        while *ptr.add(len) != 0 && len < 65536 {
                            len += 1;
                        }
                        let slice = std::slice::from_raw_parts(ptr, len);
                        result = Some(String::from_utf16_lossy(slice));
                        win32::GlobalUnlock(hmem);
                    }
                }
                win32::CloseClipboard();
            }
            return Ok(result);
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(None)
        }
    }

    fn type_text(&self, text: &str) -> Result<(), CoreError> {
        self.paste_text(text)
    }

    fn paste_text(&self, text: &str) -> Result<(), CoreError> {
        self.set_clipboard_text(text)?;
        #[cfg(target_os = "windows")]
        unsafe {
            win32::keybd_event(win32::VK_CONTROL, 0, 0, 0);
            win32::keybd_event(b'V', 0, 0, 0);
            win32::keybd_event(b'V', 0, win32::KEYEVENTF_KEYUP, 0);
            win32::keybd_event(win32::VK_CONTROL, 0, win32::KEYEVENTF_KEYUP, 0);
        }
        Ok(())
    }

    fn send_key_combination(
        &self,
        modifiers: &[ModifierKey],
        key: InputKey,
    ) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        unsafe {
            for &m in modifiers {
                let vk = Self::modifier_to_vk(m);
                win32::keybd_event(vk, 0, 0, 0);
            }
            let vk_key = Self::key_to_vk(&key);
            win32::keybd_event(vk_key, 0, 0, 0);
            win32::keybd_event(vk_key, 0, win32::KEYEVENTF_KEYUP, 0);
            for &m in modifiers.iter().rev() {
                let vk = Self::modifier_to_vk(m);
                win32::keybd_event(vk, 0, win32::KEYEVENTF_KEYUP, 0);
            }
        }
        let _ = (modifiers, key);
        Ok(())
    }

    fn move_pointer(&self, x: i32, y: i32) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        unsafe {
            win32::SetCursorPos(x, y);
        }
        let _ = (x, y);
        Ok(())
    }

    fn get_pointer_position(&self) -> Result<PointerPosition, CoreError> {
        #[cfg(target_os = "windows")]
        unsafe {
            let mut pt = win32::POINT::default();
            if win32::GetCursorPos(&mut pt) != 0 {
                return Ok(PointerPosition { x: pt.x, y: pt.y });
            }
        }
        Ok(PointerPosition { x: 0, y: 0 })
    }

    fn click_pointer(&self, action: ClickAction) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        unsafe {
            match action {
                ClickAction::LeftClick => {
                    win32::mouse_event(win32::MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
                    win32::mouse_event(win32::MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
                }
                ClickAction::RightClick => {
                    win32::mouse_event(win32::MOUSEEVENTF_RIGHTDOWN, 0, 0, 0, 0);
                    win32::mouse_event(win32::MOUSEEVENTF_RIGHTUP, 0, 0, 0, 0);
                }
                ClickAction::MiddleClick => {
                    win32::mouse_event(win32::MOUSEEVENTF_MIDDLEDOWN, 0, 0, 0, 0);
                    win32::mouse_event(win32::MOUSEEVENTF_MIDDLEUP, 0, 0, 0, 0);
                }
                ClickAction::DoubleClick => {
                    win32::mouse_event(win32::MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
                    win32::mouse_event(win32::MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
                    win32::mouse_event(win32::MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
                    win32::mouse_event(win32::MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
                }
                ClickAction::LeftDown => {
                    win32::mouse_event(win32::MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
                }
                ClickAction::LeftUp => {
                    win32::mouse_event(win32::MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
                }
            }
        }
        let _ = action;
        Ok(())
    }

    fn scroll(&self, direction: ScrollDirection, amount: u32) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        unsafe {
            let ticks = (amount.max(1) as i32) * 120;
            match direction {
                ScrollDirection::Up => {
                    win32::mouse_event(win32::MOUSEEVENTF_WHEEL, 0, 0, ticks as u32, 0);
                }
                ScrollDirection::Down => {
                    win32::mouse_event(win32::MOUSEEVENTF_WHEEL, 0, 0, (-ticks) as u32, 0);
                }
                ScrollDirection::Right => {
                    win32::mouse_event(win32::MOUSEEVENTF_HWHEEL, 0, 0, ticks as u32, 0);
                }
                ScrollDirection::Left => {
                    win32::mouse_event(win32::MOUSEEVENTF_HWHEEL, 0, 0, (-ticks) as u32, 0);
                }
                ScrollDirection::PageUp => {
                    win32::keybd_event(0x21, 0, 0, 0);
                    win32::keybd_event(0x21, 0, win32::KEYEVENTF_KEYUP, 0);
                }
                ScrollDirection::PageDown => {
                    win32::keybd_event(0x22, 0, 0, 0);
                    win32::keybd_event(0x22, 0, win32::KEYEVENTF_KEYUP, 0);
                }
            }
        }
        let _ = (direction, amount);
        Ok(())
    }
}

pub struct WindowsAudioOps;
impl PlatformAudioOps for WindowsAudioOps {
    fn play_audio_stream(&self, _samples: &[f32], _sample_rate: u32) -> Result<(), CoreError> {
        Ok(())
    }

    fn capture_audio_chunk(&self) -> Result<Vec<f32>, CoreError> {
        Ok(vec![0.0f32; 1600])
    }

    fn is_audio_output_active(&self) -> Result<bool, CoreError> {
        Ok(false)
    }

    fn list_audio_devices(&self) -> Result<Vec<String>, CoreError> {
        Ok(vec!["Windows Default Audio Device".to_string()])
    }
}
