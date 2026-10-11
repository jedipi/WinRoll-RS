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
    if UPDATE_RECOVERY_CANCELLED.load(Ordering::Acquire) && checker.recover() {
        UPDATE_RECOVERED.store(false, Ordering::Release);
        request_control(UPDATE_RECOVER);
    }
    if checker.state == State::Recovering && UPDATE_RECOVERED.load(Ordering::Acquire) {
        checker.recovered();
    }
    if checker.state == State::ReadyToExit {
        request_control(EXIT);
    }
    if changed && matches!(checker.state, State::InstallFailed(_)) {
        request_control(CANCEL_UPDATE_RECOVERY);
    }
    if !checker.busy() && !matches!(checker.state, State::Staged(_)) {
        unsafe { KillTimer(owner, TIMER) };
    }
    drop(checker);
    if changed
        || matches!(
            CHECKER.lock().unwrap().state,
            State::Recovering | State::Installing
        )
    {
        refresh();
    }
}

pub(super) fn refresh() {
    let hwnd = WINDOW.load(Ordering::Relaxed) as HWND;
    if hwnd.is_null() {
        return;
    }
    let checker = CHECKER.lock().unwrap();
    let state = checker.state.clone();
    drop(checker);
    let label = match &state {
        State::Idle | State::Checking => text("Checking for updates...").to_owned(),
        State::Current => text("You're up to date.").to_owned(),
        State::Available(version) => {
            text("Update available: {version}").replace("{version}", version)
        }
        State::Failed(error) => {
            text("Could not check for updates.\n\n{error}").replace("{error}", text(error))
        }
        State::DownloadFailed(error) => {
            text("Could not download the update.\n\n{error}").replace("{error}", text(error))
        }
        State::Downloading { downloaded, total } => match total.filter(|total| *total > 0) {
            Some(total) => text("Downloading update: {percent}% ({bytes} bytes)")
                .replace(
                    "{percent}",
                    &(u128::from(*downloaded) * 100 / u128::from(total))
                        .min(100)
                        .to_string(),
                )
                .replace("{bytes}", &downloaded.to_string()),
            None => text("Downloading update... ({bytes} bytes)")
                .replace("{bytes}", &downloaded.to_string()),
        },
        State::Verifying => text("Verifying update package...").to_owned(),
        State::Staged(_) => text("Verifying update package...").to_owned(),
        State::Recovering => {
            let mut label = text("Recovering windows before updating...\nRetry recovery or cancel to keep WinRoll running.").to_owned();
            for (name, pid, hwnd) in RECOVERY_WINDOWS.lock().unwrap().iter() {
                label.push_str(&format!("\n{name} (PID {pid}, HWND {hwnd:#x})"));
            }
            label
        }
        State::Installing | State::ReadyToExit => {
            text("Installing update and restarting WinRoll...").to_owned()
        }
        State::InstallFailed(error) => {
            text("Could not install the update.\n\n{error}").replace("{error}", text(error))
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
        SetWindowTextW(
            GetDlgItem(hwnd, IDOK),
            wide_text(if matches!(state, State::Available(_)) {
                "&Update now"
            } else {
                "&Retry"
            })
            .as_ptr(),
        );
        ShowWindow(
            GetDlgItem(hwnd, IDOK),
            if matches!(
                state,
                State::Available(_)
                    | State::Failed(_)
                    | State::DownloadFailed(_)
                    | State::InstallFailed(_)
                    | State::Recovering
            ) {
                SW_SHOW
            } else {
                SW_HIDE
            },
        );
        SetWindowTextW(
            GetDlgItem(hwnd, IDCANCEL),
            wide_text(
                if matches!(
                    state,
                    State::Checking
                        | State::Downloading { .. }
                        | State::Verifying
                        | State::Recovering
                ) {
                    "Cancel"
                } else {
                    "Close"
                },
            )
            .as_ptr(),
        );
        windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
            GetDlgItem(hwnd, IDCANCEL),
            (!matches!(state, State::Installing | State::ReadyToExit)) as i32,
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
                        190,
                        182,
                        120,
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
                let state = CHECKER.lock().unwrap().state.clone();
                let owner = GetWindow(hwnd, GW_OWNER);
                if matches!(state, State::Failed(_)) {
                    startup(owner);
                    refresh();
                    SetFocus(GetDlgItem(hwnd, IDCANCEL));
                } else if state == State::Recovering {
                    request_control(UPDATE_RECOVER);
                } else if matches!(
                    state,
                    State::Available(_) | State::DownloadFailed(_) | State::InstallFailed(_)
                ) {
                    let mut checker = CHECKER.lock().unwrap();
                    if SetTimer(owner, TIMER, 100, None) == 0 {
                        checker.state =
                            State::DownloadFailed(io::Error::last_os_error().to_string());
                    } else {
                        checker.update_now();
                    }
                    drop(checker);
                    refresh();
                    SetFocus(GetDlgItem(hwnd, IDCANCEL));
                }
                return 0;
            }
            WM_COMMAND if w & 0xffff == IDCANCEL as usize => {
                if can_close() {
                    DestroyWindow(hwnd);
                }
                return 0;
            }
            WM_CLOSE => {
                if can_close() {
                    DestroyWindow(hwnd);
                }
                return 0;
            }
            WM_DESTROY => {
                cancel_recovery();
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
            State::Checking
                | State::Available(_)
                | State::Downloading { .. }
                | State::Verifying
                | State::Staged(_)
                | State::DownloadFailed(_)
                | State::Recovering
                | State::Installing
                | State::ReadyToExit
                | State::InstallFailed(_)
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
                    if can_close() {
                        DestroyWindow(hwnd);
                    }
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

fn can_close() -> bool {
    !matches!(
        CHECKER.lock().unwrap().state,
        State::Installing | State::ReadyToExit
    )
}

pub(super) fn cancel_recovery() -> bool {
    let mut checker = CHECKER.lock().unwrap();
    if checker.state == State::Installing {
        return false;
    }
    if checker.state == State::ReadyToExit {
        return true;
    }
    if checker.state == State::Recovering {
        CONTROL_REQUESTS.fetch_and(!UPDATE_RECOVER, Ordering::Relaxed);
        request_control(CANCEL_UPDATE_RECOVERY);
    }
    checker.cancel();
    true
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
            CHECKER.lock().unwrap().state = State::Available("1.1.0".into());
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
            for (state, expected, action, cancel) in [
                (
                    State::Available("1.1.0".into()),
                    text("Update available: {version}").replace("{version}", "1.1.0"),
                    Some("&Update now"),
                    "Close",
                ),
                (
                    State::Downloading {
                        downloaded: 2,
                        total: Some(5),
                    },
                    text("Downloading update: {percent}% ({bytes} bytes)")
                        .replace("{percent}", "40")
                        .replace("{bytes}", "2"),
                    None,
                    "Cancel",
                ),
                (
                    State::Downloading {
                        downloaded: 2,
                        total: None,
                    },
                    text("Downloading update... ({bytes} bytes)").replace("{bytes}", "2"),
                    None,
                    "Cancel",
                ),
                (
                    State::Verifying,
                    text("Verifying update package...").to_owned(),
                    None,
                    "Cancel",
                ),
                (
                    State::Staged("1.1.0".into()),
                    text("Verifying update package...").to_owned(),
                    None,
                    "Close",
                ),
                (
                    State::Recovering,
                    text("Recovering windows before updating...\nRetry recovery or cancel to keep WinRoll running.").to_owned(),
                    Some("&Retry"),
                    "Cancel",
                ),
                (
                    State::Installing,
                    text("Installing update and restarting WinRoll...").to_owned(),
                    None,
                    "Close",
                ),
                (
                    State::InstallFailed("offline".into()),
                    text("Could not install the update.\n\n{error}").replace("{error}", "offline"),
                    Some("&Retry"),
                    "Close",
                ),
                (
                    State::DownloadFailed("Update package SHA-256 does not match.".into()),
                    text("Could not download the update.\n\n{error}")
                        .replace("{error}", text("Update package SHA-256 does not match.")),
                    Some("&Retry"),
                    "Close",
                ),
            ] {
                let installing = state == State::Installing;
                CHECKER.lock().unwrap().state = state;
                refresh();
                let control_text = |id| {
                    let mut buffer = [0u16; 1024];
                    let len = unsafe {
                        GetWindowTextW(
                            GetDlgItem(hwnd, id),
                            buffer.as_mut_ptr(),
                            buffer.len() as i32,
                        )
                    };
                    String::from_utf16_lossy(&buffer[..len as usize]).replace("\r\n", "\n")
                };
                if control_text(10) != expected || control_text(IDCANCEL) != text(cancel) {
                    return Err("Update progress or cancellation did not translate".into());
                }
                let visible = unsafe { IsWindowVisible(GetDlgItem(hwnd, IDOK)) != 0 };
                if visible != action.is_some()
                    || action.is_some_and(|action| control_text(IDOK) != text(action))
                {
                    return Err("Update consent or retry action is incorrect".into());
                }
                if installing {
                    unsafe { SendMessageW(hwnd, WM_CLOSE, 0, 0); }
                    let escape = MSG {
                        hwnd: unsafe { GetDlgItem(hwnd, 10) },
                        message: WM_KEYDOWN,
                        wParam: 27,
                        ..Default::default()
                    };
                    if !dialog_message(&escape) || unsafe { IsWindow(hwnd) } == 0 || can_close() {
                        return Err("Installation could be interrupted by closing its UI".into());
                    }
                }
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
