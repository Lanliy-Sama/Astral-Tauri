//! Native Windows desktop lyrics. No second WebView or application instance.
//! A color-keyed layered window keeps only text visible over other apps.

#[cfg(windows)]
mod platform {
    use serde_json::Value;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{LazyLock, Mutex};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        BeginPaint, CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect,
        SelectObject, SetBkMode, SetTextColor, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY,
        DEFAULT_CHARSET, DEFAULT_PITCH, FF_DONTCARE, FW_BOLD, FW_NORMAL, OUT_DEFAULT_PRECIS,
        PAINTSTRUCT, TRANSPARENT,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::*;

    const WM_LYRICS_SHOW: u32 = WM_APP + 11;
    const WM_LYRICS_HIDE: u32 = WM_APP + 12;
    const WM_LYRICS_UPDATE: u32 = WM_APP + 13;
    const WM_LYRICS_CLICK_THROUGH: u32 = WM_APP + 14;

    #[derive(Clone, Default)]
    struct LyricState {
        title: String,
        original: String,
        translation: String,
        romanization: String,
        playing: bool,
        click_through: bool,
    }

    static STATE: LazyLock<Mutex<LyricState>> = LazyLock::new(|| Mutex::new(LyricState::default()));
    static WINDOW: LazyLock<Mutex<isize>> = LazyLock::new(|| Mutex::new(0));
    static STARTED: AtomicBool = AtomicBool::new(false);

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn init() {
        if STARTED.swap(true, Ordering::SeqCst) {
            return;
        }
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || message_loop(sender));
        let _ = receiver.recv_timeout(std::time::Duration::from_secs(2));
    }

    pub fn set_enabled(enabled: bool) {
        if enabled {
            init();
        }
        post(if enabled { WM_LYRICS_SHOW } else { WM_LYRICS_HIDE }, 0);
    }

    pub fn set_click_through(enabled: bool) {
        if let Ok(mut state) = STATE.lock() {
            state.click_through = enabled;
        }
        post(WM_LYRICS_CLICK_THROUGH, usize::from(enabled));
    }

    pub fn update(payload: &Value) {
        let line = payload.get("line");
        let mut state = match STATE.lock() {
            Ok(state) => state,
            Err(_) => return,
        };
        state.title = payload["title"].as_str().unwrap_or("Stellaris").to_string();
        state.playing = payload["playing"].as_bool().unwrap_or(false);
        state.original = line
            .and_then(|line| line["l"].as_str())
            .unwrap_or("")
            .to_string();
        let has_chinese = state.original.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c));
        state.translation = if has_chinese {
            String::new()
        } else {
            line.and_then(|line| line["tr"].as_str()).unwrap_or("").to_string()
        };
        state.romanization = if has_chinese {
            String::new()
        } else {
            line.and_then(|line| line["ro"].as_str()).unwrap_or("").to_string()
        };
        drop(state);
        post(WM_LYRICS_UPDATE, 0);
    }

    fn post(message: u32, wparam: usize) {
        let hwnd = WINDOW.lock().map(|value| *value).unwrap_or(0);
        if hwnd != 0 {
            unsafe {
                let _ = PostMessageW(HWND(hwnd), message, WPARAM(wparam), LPARAM(0));
            }
        }
    }

    fn message_loop(ready: std::sync::mpsc::Sender<isize>) {
        unsafe {
            let class_name = wide("StellarisDesktopLyrics");
            let instance = HINSTANCE(GetModuleHandleW(None).unwrap_or_default().0);
            let mut class = WNDCLASSW::default();
            class.lpfnWndProc = Some(window_proc);
            class.hInstance = instance;
            class.lpszClassName = PCWSTR(class_name.as_ptr());
            class.hbrBackground = CreateSolidBrush(COLORREF(0));
            RegisterClassW(&class);

            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                class.lpszClassName,
                PCWSTR(wide("Stellaris 桌面歌词").as_ptr()),
                WS_POPUP,
                0,
                0,
                880,
                116,
                None,
                None,
                instance,
                None,
            );
            if hwnd.0 == 0 {
                STARTED.store(false, Ordering::SeqCst);
                let _ = ready.send(0);
                return;
            }
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_COLORKEY);
            if let Ok(mut current) = WINDOW.lock() {
                *current = hwnd.0;
            }
            let _ = ready.send(hwnd.0);

            let mut message = MSG::default();
            loop {
                let result = GetMessageW(&mut message, None, 0, 0);
                if result.0 <= 0 {
                    break;
                }
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if let Ok(mut current) = WINDOW.lock() {
                *current = 0;
            }
            STARTED.store(false, Ordering::SeqCst);
        }
    }

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_LYRICS_SHOW => {
                let mut work = RECT::default();
                if SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut work), 0).is_ok() {
                    let work_width = work.right - work.left;
                    let width = 880.min((work_width - 80).max(420));
                    let height = if STATE.lock().map(|s| !s.romanization.is_empty() || !s.translation.is_empty()).unwrap_or(false) { 116 } else { 92 };
                    let x = work.left + (work_width - width) / 2;
                    let y = work.bottom - height - 34;
                    let _ = SetWindowPos(
                        hwnd,
                        HWND_TOPMOST,
                        x,
                        y,
                        width,
                        height,
                        SWP_NOACTIVATE | SWP_SHOWWINDOW,
                    );
                }
                let _ = ShowWindow(hwnd, SW_SHOW);
                LRESULT(0)
            }
            WM_LYRICS_HIDE => {
                let _ = ShowWindow(hwnd, SW_HIDE);
                LRESULT(0)
            }
            WM_LYRICS_UPDATE => {
                let _ = InvalidateRect(hwnd, None, true);
                LRESULT(0)
            }
            WM_LYRICS_CLICK_THROUGH => {
                let mut style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                if wparam.0 != 0 {
                    style |= WS_EX_TRANSPARENT.0 as isize;
                } else {
                    style &= !(WS_EX_TRANSPARENT.0 as isize);
                }
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
                LRESULT(0)
            }
            WM_NCHITTEST => {
                let click_through = STATE.lock().map(|state| state.click_through).unwrap_or(false);
                if click_through { LRESULT(HTTRANSPARENT as isize) } else { LRESULT(HTCAPTION as isize) }
            }
            WM_PAINT => {
                paint(hwnd);
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }

    fn paint(hwnd: HWND) {
        unsafe {
            let mut paint = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut paint);
            let mut client = RECT::default();
            let _ = GetClientRect(hwnd, &mut client);
            FillRect(hdc, &client, CreateSolidBrush(COLORREF(0)));
            let state = match STATE.lock() {
                Ok(state) => state.clone(),
                Err(_) => LyricState::default(),
            };
            let _ = SetBkMode(hdc, TRANSPARENT);
            let text = if state.original.is_empty() { state.title.as_str() } else { state.original.as_str() };
            let has_aux = !state.translation.is_empty() || !state.romanization.is_empty();
            if has_aux {
                draw_text(hdc, text, &client, 5, 35, 26, true);
                if !state.translation.is_empty() {
                    draw_text(hdc, &state.translation, &client, 42, 20, 16, false);
                }
                if !state.romanization.is_empty() {
                    draw_text(hdc, &state.romanization, &client, 64, 18, 13, false);
                }
            } else {
                draw_text(hdc, text, &client, 0, client.bottom - client.top, 26, true);
            }
            let _ = EndPaint(hwnd, &paint);
        }
    }

    unsafe fn draw_text(hdc: windows::Win32::Graphics::Gdi::HDC, text: &str, client: &RECT, top: i32, height: i32, size: i32, bold: bool) {
        if text.is_empty() {
            return;
        }
        let font = CreateFontW(
            -size,
            0,
            0,
            0,
            if bold { FW_BOLD } else { FW_NORMAL },
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            DEFAULT_PITCH | FF_DONTCARE,
            PCWSTR(wide("Segoe UI").as_ptr()),
        );
        let _ = SelectObject(hdc, font);
        let _ = SetTextColor(hdc, COLORREF(0x00ff_ffff));
        let mut rect = RECT { left: 16, top, right: client.right - 16, bottom: top + height };
        let mut text = text.encode_utf16().collect::<Vec<_>>();
        DrawTextW(hdc, &mut text, &mut rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS);
        let _ = DeleteObject(font);
    }
}

#[cfg(windows)]
pub use platform::{init, set_click_through, set_enabled, update};

#[cfg(not(windows))]
pub fn init() {}
#[cfg(not(windows))]
pub fn set_enabled(_enabled: bool) {}
#[cfg(not(windows))]
pub fn set_click_through(_enabled: bool) {}
#[cfg(not(windows))]
pub fn update(_payload: &serde_json::Value) {}
