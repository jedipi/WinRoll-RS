use super::*;
use crate::updates::{Checker, State};

pub(super) const TIMER: usize = 0x5550;
static CHECKER: Mutex<Checker> = Mutex::new(Checker::new());
static WINDOW: AtomicIsize = AtomicIsize::new(0);

pub(super) fn menu_label() -> &'static str {
    CHECKER.lock().unwrap().menu_label()
}

pub(super) fn startup(owner: HWND) {
    let mut checker = CHECKER.lock().unwrap();
    if unsafe { SetTimer(owner, TIMER, 100, None) } == 0 {
        checker.cancel();
        checker.state = State::Failed(io::Error::last_os_error().to_string());
    } else {
        checker.start(crate::updates::check);
    }
}

pub(super) fn poll(owner: HWND) {
    let mut checker = CHECKER.lock().unwrap();
    let changed = checker.poll();
    if !matches!(checker.state, State::Checking) {
        unsafe { KillTimer(owner, TIMER) };
    }
    drop(checker);
    if changed {
        refresh();
    }
}

pub(super) fn refresh() {
    let hwnd = WINDOW.load(Ordering::Relaxed) as HWND;
    if hwnd.is_null() {
        return;
    }
    let state = CHECKER.lock().unwrap().state.clone();
    let label = match &state {
        State::Idle | State::Checking => text("Checking for updates...").to_owned(),
        State::Current => text("You're up to date.").to_owned(),
        State::Available(version) => {
            text("Update available: {version}").replace("{version}", version)
        }
        State::Failed(error) => {
            text("Could not check for updates.\n\n{error}").replace("{error}", error)
        }
    };
    let wide: Vec<u16> = label
        .replace("\r\n", "\n")
        .replace('\n', "\r\n")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    // SAFETY: the UI thread owns this window and its controls.
    unsafe {
        SetWindowTextW(hwnd, wide_text("WinRoll RS - Updates").as_ptr());
        SetWindowTextW(GetDlgItem(hwnd, 10), wide.as_ptr());
        SetWindowTextW(GetDlgItem(hwnd, IDOK), wide_text("&Retry").as_ptr());
        ShowWindow(
            GetDlgItem(hwnd, IDOK),
            if matches!(state, State::Failed(_)) {
                SW_SHOW
            } else {
                SW_HIDE
            },
        );
        SetWindowTextW(
            GetDlgItem(hwnd, IDCANCEL),
            wide_text(if matches!(state, State::Checking) {
                "Cancel"
            } else {
                "Close"
            })
            .as_ptr(),
        );
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: this thread owns the window and native child controls.
    unsafe {
        match message {
            WM_CREATE => {
                let dpi = GetDpiForWindow(hwnd) as i32;
                let scale = |n| n * dpi / 96;
                for (id, class, style, x, y, width, height) in [
                    (
                        10,
                        w!("EDIT"),
                        ES_MULTILINE as u32 | ES_READONLY as u32 | WS_VSCROLL | WS_TABSTOP,
                        20,
                        20,
                        400,
                        140,
                    ),
                    (
                        IDOK,
                        w!("BUTTON"),
                        BS_PUSHBUTTON as u32 | WS_TABSTOP,
                        220,
                        182,
                        90,
                        28,
                    ),
                    (
                        IDCANCEL,
                        w!("BUTTON"),
                        BS_DEFPUSHBUTTON as u32 | WS_TABSTOP,
                        330,
                        182,
                        90,
                        28,
                    ),
                ] {
                    let control = CreateWindowExW(
                        0,
                        class,
                        w!(""),
                        WS_CHILD | WS_VISIBLE | style,
                        scale(x),
                        scale(y),
                        scale(width),
                        scale(height),
                        hwnd,
                        id as HMENU,
                        GetModuleHandleW(null_mut()),
                        null_mut(),
                    );
                    if control.is_null() {
                        return -1;
                    }
                    SendMessageW(
                        control,
                        WM_SETFONT,
                        GetStockObject(DEFAULT_GUI_FONT) as usize,
                        1,
                    );
                }
                for (id, after) in [(IDOK, 10), (IDCANCEL, IDOK)] {
                    SetWindowPos(
                        GetDlgItem(hwnd, id),
                        GetDlgItem(hwnd, after),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                }
                return 0;
            }
            WM_COMMAND if w & 0xffff == IDOK as usize => {
                if matches!(CHECKER.lock().unwrap().state, State::Failed(_)) {
                    startup(GetWindow(hwnd, GW_OWNER));
                    refresh();
                    SetFocus(GetDlgItem(hwnd, IDCANCEL));
                }
                return 0;
            }
            WM_COMMAND if w & 0xffff == IDCANCEL as usize => {
                DestroyWindow(hwnd);
                return 0;
            }
            WM_CLOSE => {
                DestroyWindow(hwnd);
                return 0;
            }
            WM_DESTROY => {
                CHECKER.lock().unwrap().cancel();
                KillTimer(GetWindow(hwnd, GW_OWNER), TIMER);
                WINDOW.store(0, Ordering::Relaxed);
                return 0;
            }
            _ => {}
        }
        DefWindowProcW(hwnd, message, w, l)
    }
}

pub(super) fn show(owner: HWND) {
    poll(owner);
    unsafe {
        let existing = WINDOW.load(Ordering::Relaxed) as HWND;
        if !existing.is_null() {
            SetForegroundWindow(existing);
            return;
        }
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: GetModuleHandleW(null_mut()),
            hbrBackground: GetSysColorBrush(COLOR_BTNFACE),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            lpszClassName: w!("WinRollUpdates"),
            ..Default::default()
        };
        RegisterClassW(&class);
        let mut cursor = POINT::default();
        GetCursorPos(&mut cursor);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(
            MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST),
            &mut info,
        );
        let mut dpi = 96;
        let mut dpi_y = 96;
        if GetDpiForMonitor(
            MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST),
            MDT_EFFECTIVE_DPI,
            &mut dpi,
            &mut dpi_y,
        ) < 0
        {
            dpi = GetDpiForSystem();
        }
        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU;
        let mut bounds = RECT {
            right: 440 * dpi as i32 / 96,
            bottom: 230 * dpi as i32 / 96,
            ..Default::default()
        };
        AdjustWindowRectExForDpi(&mut bounds, style, 0, WS_EX_DLGMODALFRAME, dpi);
        let width = bounds.right - bounds.left;
        let height = bounds.bottom - bounds.top;
        let hwnd = CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            class.lpszClassName,
            wide_text("WinRoll RS - Updates").as_ptr(),
            style,
            info.rcWork.left + (info.rcWork.right - info.rcWork.left - width) / 2,
            info.rcWork.top + (info.rcWork.bottom - info.rcWork.top - height) / 2,
            width,
            height,
            owner,
            null_mut(),
            class.hInstance,
            null_mut(),
        );
        if hwnd.is_null() {
            MessageBoxW(
                owner,
                wide_text("Cannot open the update window").as_ptr(),
                w!("WinRoll RS"),
                MB_OK | MB_ICONERROR,
            );
            return;
        }
        WINDOW.store(hwnd as isize, Ordering::Relaxed);
        if !matches!(
            CHECKER.lock().unwrap().state,
            State::Checking | State::Available(_)
        ) {
            startup(owner);
        }
        refresh();
        if let Some(Some((expanded, _))) = TRAY_ICONS.get() {
            SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, *expanded);
        }
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        SetFocus(GetDlgItem(hwnd, IDCANCEL));
    }
}

pub(super) fn dialog_message(msg: &MSG) -> bool {
    let hwnd = WINDOW.load(Ordering::Relaxed) as HWND;
    if hwnd.is_null() {
        return false;
    }
    unsafe {
        // A multiline edit claims Tab/Escape even when read-only; keep dialog navigation working.
        if msg.hwnd == GetDlgItem(hwnd, 10) && msg.message == WM_KEYDOWN {
            match msg.wParam {
                9 => {
                    SetFocus(GetNextDlgTabItem(
                        hwnd,
                        msg.hwnd,
                        (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyState(
                            VK_SHIFT as i32,
                        ) < 0) as i32,
                    ));
                    return true;
                }
                27 => {
                    DestroyWindow(hwnd);
                    return true;
                }
                _ => {}
            }
        }
        IsDialogMessageW(hwnd, msg) != 0
    }
}

pub(super) fn close(owner: HWND) {
    CHECKER.lock().unwrap().cancel();
    unsafe {
        KillTimer(owner, TIMER);
        let hwnd = WINDOW.load(Ordering::Relaxed) as HWND;
        if !hwnd.is_null() {
            DestroyWindow(hwnd);
        }
    }
}

#[cfg_attr(test, test)]
pub(super) fn self_test() -> Result<(), String> {
    #[cfg(test)]
    let _guard = UI_TEST_LOCK.lock().unwrap();
    let original_language = localization::preference();
    CHECKER.lock().unwrap().state = State::Available("1.1.0".into());
    show(null_mut());
    let hwnd = WINDOW.load(Ordering::Relaxed) as HWND;
    if hwnd.is_null() {
        return Err("Cannot create update dialog".into());
    }
    let result = (|| {
        for language in [1, 2, 3] {
            localization::set_preference(language);
            refresh();
            let mut label = [0u16; 512];
            let len = unsafe {
                GetWindowTextW(GetDlgItem(hwnd, 10), label.as_mut_ptr(), label.len() as i32)
            };
            if String::from_utf16_lossy(&label[..len as usize])
                != text("Update available: {version}").replace("{version}", "1.1.0")
            {
                return Err("Update dialog did not translate the available version".into());
            }
        }
        CHECKER.lock().unwrap().state = State::Failed("offline".into());
        refresh();
        unsafe {
            if IsWindowVisible(GetDlgItem(hwnd, IDOK)) == 0 {
                return Err("Update error did not offer Retry".into());
            }
            SetFocus(GetDlgItem(hwnd, 10));
            let tab = MSG {
                hwnd: GetDlgItem(hwnd, 10),
                message: WM_KEYDOWN,
                wParam: 9,
                ..Default::default()
            };
            let navigated = dialog_message(&tab);
            let focused = windows_sys::Win32::UI::Input::KeyboardAndMouse::GetFocus();
            if !navigated || focused != GetDlgItem(hwnd, IDOK) {
                return Err("Retry is not keyboard accessible".into());
            }
            let escape = MSG {
                hwnd: GetDlgItem(hwnd, IDOK),
                message: WM_KEYDOWN,
                wParam: 27,
                ..Default::default()
            };
            if !dialog_message(&escape)
                || IsWindow(hwnd) != 0
                || WINDOW.load(Ordering::Relaxed) != 0
            {
                return Err("Escape did not cancel the update dialog".into());
            }
        }
        Ok(())
    })();
    close(null_mut());
    *CHECKER.lock().unwrap() = Checker::new();
    localization::set_preference(original_language);
    result
}
