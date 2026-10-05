use std::{
    collections::HashMap,
    io::{self, Write},
    mem::size_of,
    ptr::null_mut,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering},
        mpsc::{self, SyncSender},
    },
    thread,
    time::{Duration, Instant},
};
use windows_sys::{
    Win32::{
        Foundation::*,
        Graphics::{Dwm::*, Gdi::*},
        Security::*,
        System::{Console::*, LibraryLoader::*, Threading::*},
        UI::{
            Controls::*,
            HiDpi::*,
            Input::KeyboardAndMouse::{
                GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
                SendInput, SetFocus, VK_CONTROL, VK_F8, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
                keybd_event,
            },
            Shell::*,
            WindowsAndMessaging::*,
        },
    },
    core::w,
};

mod tray_windows;

static COMMANDS: OnceLock<SyncSender<Command>> = OnceLock::new();
static CONTROL_REQUESTS: AtomicU32 = AtomicU32::new(0);
static RECOVERY_PENDING: AtomicBool = AtomicBool::new(false);
static RECOVERY_WINDOWS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static WORKER_FINISHED: AtomicBool = AtomicBool::new(false);
static TASKBAR_CREATED: OnceLock<u32> = OnceLock::new();
static TRAY_ICONS: OnceLock<Option<(isize, isize)>> = OnceLock::new();
static HAS_ROLLED_WINDOWS: AtomicBool = AtomicBool::new(false);
static ABOUT_WINDOW: AtomicIsize = AtomicIsize::new(0);
static OPTIONS_WINDOW: AtomicIsize = AtomicIsize::new(0);
static TRANSPARENCY: AtomicU32 = AtomicU32::new(50);
static RESTORE_TRAY_REQUESTS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
const ENABLE: u32 = 1;
const UNROLL_ALL: u32 = 2;
const PAUSE: u32 = 4;
const EXIT: u32 = 8;
const ABOUT: u32 = 16;
const SETTINGS: u32 = 32;
const RECREATE_TRAY: u32 = 256;
const TOGGLE_ENABLED: u32 = 512;
// TBM_GETPOS is WM_USER, omitted by the windows-sys metadata.
const TBM_GETPOS: u32 = WM_USER;
const TRAY_CALLBACK: u32 = WM_APP + 1;
const TRAY_UPDATE: u32 = WM_APP + 2;
const WORKER_DONE: u32 = WM_APP + 3;
const WINDOW_TRAY_CALLBACK: u32 = WM_APP + 4;
const NIN_KEYSELECT: u32 = NIN_SELECT | NINF_KEY;
const ALT_TAP_TAG: usize = 0x0057_494e_414c_5421;
static SUPPRESS_UP: AtomicBool = AtomicBool::new(false);
static SUPPRESS_MIDDLE_UP: AtomicBool = AtomicBool::new(false);
static STOPPING: AtomicBool = AtomicBool::new(false);
static DRAG: Mutex<Option<Drag>> = Mutex::new(None);
static DRAG_POSITION: Mutex<Option<DragPosition>> = Mutex::new(None);
static DRAG_ACTIVE: AtomicBool = AtomicBool::new(false);
// Only the self-test's process-owned fixture consults this switch.
static FIXTURE_REFUSES_EXPANSION: AtomicIsize = AtomicIsize::new(0);
static FIXTURE_MIN_TRACK: AtomicBool = AtomicBool::new(false);
static FIXTURE_CAPTION_EVERYWHERE: AtomicBool = AtomicBool::new(false);
static FIXTURE_REFUSES_SHOW: AtomicIsize = AtomicIsize::new(0);
static FIXTURE_DELAYS_HIDE: AtomicIsize = AtomicIsize::new(0);

#[derive(Clone, Copy)]
struct Target {
    hwnd: isize,
    pid: u32,
    tid: u32,
    rect: RECT,
}
enum Command {
    ProbeRight(POINT, SyncSender<Option<(Target, u32)>>),
    ProbeMiddle(POINT, SyncSender<Option<(Target, u32)>>),
    ProbeRolled(POINT, SyncSender<Option<Target>>),
    FocusRolled(isize, Instant),
    Toggle(Target),
    ToggleTopmost(Target),
    ToggleTransparency(Target),
    SendToBack(Target),
    MinimizeToTray(Target),
}
struct Drag {
    target: Target,
    start: POINT,
    moved: bool,
}
struct DragPosition {
    key: isize,
    x: i32,
    y: i32,
    finished: bool,
}
struct Saved {
    target: Target,
    marker: usize,
    dpi: u32,
}
struct Manager {
    hidden: HashMap<u32, tray_windows::Hidden>,
    hidden_property: Vec<u16>,
    tray_owner: isize,
    windows: HashMap<isize, Saved>,
    topmost: HashMap<isize, (Target, usize, bool)>,
    topmost_property: Vec<u16>,
    transparent: HashMap<isize, Transparent>,
    transparency_property: Vec<u16>,
    roll_recovery: bool,
    exit_pending: bool,
    property: Vec<u16>,
    next_marker: usize,
    integrity: u32,
}

#[derive(Clone, Copy)]
struct Transparent {
    target: Target,
    marker: usize,
    layered: bool,
    color: u32,
    alpha: u8,
    flags: u32,
}

fn log(message: impl std::fmt::Display) {
    // Logging must never panic and abandon recovery because stdout was closed.
    let _ = writeln!(io::stdout(), "{message}");
}

fn rect(hwnd: HWND) -> Option<RECT> {
    let mut value = RECT::default();
    // SAFETY: value is writable and Windows validates the opaque window handle.
    (unsafe { GetWindowRect(hwnd, &mut value) } != 0).then_some(value)
}
fn dimensions(r: RECT) -> (i32, i32) {
    (r.right - r.left, r.bottom - r.top)
}
fn bounds(r: RECT) -> [i32; 4] {
    [r.left, r.top, r.right, r.bottom]
}

fn transparency_alpha(percent: u32) -> u8 {
    ((100 - percent.min(100)) * 255 / 100) as u8
}

fn caption_height(dpi: u32) -> i32 {
    unsafe {
        GetSystemMetricsForDpi(SM_CYCAPTION, dpi)
            + 2 * (GetSystemMetricsForDpi(SM_CYFRAME, dpi)
                + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi))
    }
}

fn rolled_height(dpi: u32) -> i32 {
    caption_height(dpi) + (dpi as i32 + 48) / 96
}

fn integrity(pid: u32) -> Option<u32> {
    // SAFETY: handles are checked and closed; aligned buffer outlives its SID pointers.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let mut token = null_mut();
        let opened = OpenProcessToken(process, TOKEN_QUERY, &mut token);
        CloseHandle(process);
        if opened == 0 {
            return None;
        }
        let mut buffer = [0usize; 128];
        let mut needed = 0;
        let ok = GetTokenInformation(
            token,
            TokenIntegrityLevel,
            buffer.as_mut_ptr().cast(),
            size_of_val(&buffer) as u32,
            &mut needed,
        );
        CloseHandle(token);
        if ok == 0 {
            return None;
        }
        let label = &*(buffer.as_ptr().cast::<TOKEN_MANDATORY_LABEL>());
        if IsValidSid(label.Label.Sid) == 0 {
            return None;
        }
        let count = *GetSidSubAuthorityCount(label.Label.Sid);
        if count == 0 {
            return None;
        }
        Some(*GetSidSubAuthority(label.Label.Sid, u32::from(count - 1)))
    }
}

fn visible_caption(hwnd: HWND) -> bool {
    // SAFETY: query-only Win32 calls on an opaque handle; failure rejects the window.
    unsafe {
        let mut cloaked: u32 = 0;
        IsWindowVisible(hwnd) != 0
            && IsIconic(hwnd) == 0
            && GetAncestor(hwnd, GA_ROOT) == hwnd
            && GetWindowLongW(hwnd, GWL_STYLE) as u32 & WS_CAPTION == WS_CAPTION
            && DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED as u32,
                (&mut cloaked as *mut u32).cast(),
                4,
            ) >= 0
            && cloaked == 0
    }
}

fn ordinary(hwnd: HWND) -> bool {
    visible_caption(hwnd) && unsafe { IsZoomed(hwnd) == 0 && IsWindowArranged(hwnd) == 0 }
}

fn same_target(target: Target) -> bool {
    let mut pid = 0;
    // SAFETY: pid is writable; a closed handle returns zero.
    let tid = unsafe { GetWindowThreadProcessId(target.hwnd as HWND, &mut pid) };
    tid != 0 && tid == target.tid && pid == target.pid
}

impl Manager {
    fn new() -> Result<Self, String> {
        let pid = unsafe { GetCurrentProcessId() };
        let level = integrity(pid).ok_or("Cannot read own integrity level")?;
        if level >= 0x3000
        /* SECURITY_MANDATORY_HIGH_RID */
        {
            return Err("Run WinRoll RS without administrator privileges".into());
        }
        Ok(Self {
            hidden: HashMap::new(),
            hidden_property: format!("WinRoll-RS.Hidden.{pid}\0")
                .encode_utf16()
                .collect(),
            tray_owner: 0,
            windows: HashMap::new(),
            topmost: HashMap::new(),
            transparent: HashMap::new(),
            roll_recovery: false,
            transparency_property: format!("WinRoll-RS.Transparency.{pid}\0")
                .encode_utf16()
                .collect(),
            topmost_property: format!("WinRoll-RS.Topmost.{pid}\0")
                .encode_utf16()
                .collect(),
            exit_pending: false,
            property: format!("WinRoll-RS.Experiment.{pid}\0")
                .encode_utf16()
                .collect(),
            next_marker: 1,
            integrity: level,
        })
    }

    fn probe(&self, point: POINT) -> Option<Target> {
        self.probe_hit(point, HTCAPTION)
    }

    fn probe_hit(&self, point: POINT, expected_hit: u32) -> Option<Target> {
        self.probe_hits(point, &[expected_hit])
            .map(|(target, _)| target)
    }

    fn probe_hits(&self, point: POINT, expected_hits: &[u32]) -> Option<(Target, u32)> {
        // SAFETY: Windows validates window handles and the writable output pointers.
        unsafe {
            let under_pointer = WindowFromPoint(point);
            let hwnd = GetAncestor(under_pointer, GA_ROOT);
            if hwnd.is_null()
                || hwnd == GetConsoleWindow()
                || !visible_caption(hwnd)
                || (expected_hits == [HTCAPTION] && !ordinary(hwnd))
            {
                log(format!(
                    "PASS THROUGH: ineligible hwnd={:#x} snapped={} maximized={} minimized={}",
                    hwnd as usize,
                    IsWindowArranged(hwnd),
                    IsZoomed(hwnd),
                    IsIconic(hwnd)
                ));
                return None;
            }
            let mut pid = 0;
            let tid = GetWindowThreadProcessId(hwnd, &mut pid);
            let target_integrity = integrity(pid);
            if tid == 0 || target_integrity != Some(self.integrity) {
                log(format!(
                    "PASS THROUGH: integrity own={} target={target_integrity:?}",
                    self.integrity
                ));
                return None;
            }
            let before = rect(hwnd)?;
            let caption = caption_height(GetDpiForWindow(hwnd));
            if caption <= 0 || point.y < before.top || point.y.saturating_sub(before.top) >= caption
            {
                log("PASS THROUGH: below caption band");
                return None;
            }
            // WM_NCHITTEST packs signed 16-bit screen coordinates. Fail closed outside its range.
            let x = i16::try_from(point.x).ok()?;
            let y = i16::try_from(point.y).ok()?;
            let packed = ((u32::from(y as u16) << 16) | u32::from(x as u16)) as isize;
            let mut hit = 0;
            // Child hits normally win, so controls cannot be overridden by a parent caption.
            if SendMessageTimeoutW(
                under_pointer,
                WM_NCHITTEST,
                0,
                packed,
                SMTO_ABORTIFHUNG | SMTO_BLOCK,
                20,
                &mut hit,
            ) == 0
            {
                return None;
            }
            // UWP's CoreWindow covers the host caption but reports HTCLIENT there.
            // Only this direct child/frame pair delegates caption recognition to the host.
            if hit == HTCLIENT as usize
                && expected_hits.contains(&HTCAPTION)
                && GetParent(under_pointer) == hwnd
                && [
                    (under_pointer, "Windows.UI.Core.CoreWindow"),
                    (hwnd, "ApplicationFrameWindow"),
                ]
                .iter()
                .all(|(window, expected)| {
                    let mut class = [0u16; 64];
                    let length = GetClassNameW(*window, class.as_mut_ptr(), class.len() as i32);
                    String::from_utf16_lossy(&class[..length as usize]) == *expected
                })
            {
                let mut frame_hit = 0;
                if SendMessageTimeoutW(
                    hwnd,
                    WM_NCHITTEST,
                    0,
                    packed,
                    SMTO_ABORTIFHUNG | SMTO_BLOCK,
                    20,
                    &mut frame_hit,
                ) != 0
                    && frame_hit == HTCAPTION as usize
                {
                    hit = frame_hit;
                }
            }
            let recognized = expected_hits
                .iter()
                .copied()
                .find(|expected| hit == *expected as usize);
            let Some(recognized) = recognized else {
                log(format!(
                    "PASS THROUGH: hit-test={hit} expected={expected_hits:?}"
                ));
                return None;
            };
            if recognized == HTCAPTION && !ordinary(hwnd) {
                return None;
            }
            let target = Target {
                hwnd: hwnd as isize,
                pid,
                tid,
                rect: before,
            };
            (same_target(target) && bounds(rect(hwnd)?) == bounds(before))
                .then_some((target, recognized))
        }
    }

    fn owns(&self, saved: &Saved) -> bool {
        same_target(saved.target)
            && unsafe {
                GetPropW(saved.target.hwnd as HWND, self.property.as_ptr()) as usize == saved.marker
            }
    }

    fn owns_topmost(&self, target: Target, marker: usize) -> bool {
        same_target(target)
            && unsafe {
                GetPropW(target.hwnd as HWND, self.topmost_property.as_ptr()) as usize == marker
            }
    }

    fn restore_topmost(&mut self) {
        for (key, (target, marker, original)) in self.topmost.clone() {
            if !self.owns_topmost(target, marker) {
                self.topmost.remove(&key);
                continue;
            }
            let hwnd = key as HWND;
            unsafe {
                if SetWindowPos(
                    hwnd,
                    if original {
                        HWND_TOPMOST
                    } else {
                        HWND_NOTOPMOST
                    },
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE
                        | SWP_NOSIZE
                        | SWP_NOACTIVATE
                        | SWP_NOOWNERZORDER
                        | SWP_ASYNCWINDOWPOS,
                ) == 0
                {
                    continue;
                }
            }
            let deadline = Instant::now() + Duration::from_millis(350);
            let mut stable = 0;
            loop {
                if !self.owns_topmost(target, marker) {
                    self.topmost.remove(&key);
                    break;
                }
                if (unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST != 0)
                    == original
                {
                    stable += 1;
                    if stable == 5 {
                        unsafe {
                            RemovePropW(hwnd, self.topmost_property.as_ptr());
                        }
                        self.topmost.remove(&key);
                        break;
                    }
                } else {
                    stable = 0;
                }
                if Instant::now() >= deadline {
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }
        }
    }

    fn toggle_topmost(&mut self, target: Target) {
        let hwnd = target.hwnd as HWND;
        if !same_target(target)
            || !visible_caption(hwnd)
            || rect(hwnd).map(bounds) != Some(bounds(target.rect))
            || integrity(target.pid) != Some(self.integrity)
        {
            log("CANCEL: topmost target changed after hit-test");
            return;
        }
        // Use the current native state, including topmost set by the target or another utility.
        unsafe {
            let topmost = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST != 0;
            if !self
                .topmost
                .get(&target.hwnd)
                .is_some_and(|(saved, marker, _)| self.owns_topmost(*saved, *marker))
            {
                let marker = self.next_marker;
                self.next_marker += 1;
                if SetPropW(hwnd, self.topmost_property.as_ptr(), marker as HANDLE) == 0 {
                    log("REJECT: could not mark topmost window identity");
                    return;
                }
                self.topmost.insert(target.hwnd, (target, marker, topmost));
            }
            if SetWindowPos(
                hwnd,
                if topmost {
                    HWND_NOTOPMOST
                } else {
                    HWND_TOPMOST
                },
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_ASYNCWINDOWPOS,
            ) == 0
            {
                log(format!("FAIL topmost hwnd={:#x}", target.hwnd));
            }
        }
    }

    fn owns_transparency(&self, saved: Transparent) -> bool {
        same_target(saved.target)
            && unsafe {
                GetPropW(
                    saved.target.hwnd as HWND,
                    self.transparency_property.as_ptr(),
                ) as usize
                    == saved.marker
            }
    }

    fn restore_transparency(&mut self, key: isize) {
        let Some(saved) = self.transparent.get(&key).copied() else {
            return;
        };
        if !self.owns_transparency(saved) {
            self.transparent.remove(&key);
            return;
        }
        // Preserve unrelated extended styles and the target's original layered attributes.
        unsafe {
            let hwnd = key as HWND;
            let restored = if saved.layered {
                SetLayeredWindowAttributes(hwnd, saved.color, saved.alpha, saved.flags) != 0
            } else {
                let style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
                SetLastError(0);
                SetWindowLongW(hwnd, GWL_EXSTYLE, (style & !WS_EX_LAYERED) as i32);
                GetLastError() == 0 && GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_LAYERED == 0
            };
            if restored {
                RemovePropW(hwnd, self.transparency_property.as_ptr());
                self.transparent.remove(&key);
                RedrawWindow(
                    hwnd,
                    std::ptr::null(),
                    null_mut(),
                    RDW_INVALIDATE | RDW_FRAME | RDW_ALLCHILDREN,
                );
            }
        }
    }

    fn restore_all_transparency(&mut self) {
        for key in self.transparent.keys().copied().collect::<Vec<_>>() {
            self.restore_transparency(key);
        }
    }

    fn toggle_transparency(&mut self, target: Target) {
        let hwnd = target.hwnd as HWND;
        if !same_target(target)
            || !visible_caption(hwnd)
            || rect(hwnd).map(bounds) != Some(bounds(target.rect))
            || integrity(target.pid) != Some(self.integrity)
        {
            return;
        }
        if let Some(saved) = self.transparent.get(&target.hwnd).copied() {
            let owned = self.owns_transparency(saved);
            self.restore_transparency(target.hwnd);
            if owned {
                return;
            }
        }
        let percent = TRANSPARENCY.load(Ordering::Relaxed);
        // Zero means no transparency; do not change windows with their own alpha effects.
        if percent == 0 {
            return;
        }
        unsafe {
            let style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            let mut saved = Transparent {
                target,
                marker: self.next_marker,
                layered: style & WS_EX_LAYERED != 0,
                color: 0,
                alpha: 255,
                flags: 0,
            };
            // Per-pixel layered windows cannot be safely restored with this API.
            if saved.layered
                && GetLayeredWindowAttributes(
                    hwnd,
                    &mut saved.color,
                    &mut saved.alpha,
                    &mut saved.flags,
                ) == 0
            {
                return;
            }
            self.next_marker += 1;
            if SetPropW(
                hwnd,
                self.transparency_property.as_ptr(),
                saved.marker as HANDLE,
            ) == 0
            {
                return;
            }
            self.transparent.insert(target.hwnd, saved);
            SetLastError(0);
            SetWindowLongW(hwnd, GWL_EXSTYLE, (style | WS_EX_LAYERED) as i32);
            if GetLastError() != 0
                || SetLayeredWindowAttributes(
                    hwnd,
                    saved.color,
                    transparency_alpha(percent),
                    saved.flags | LWA_ALPHA,
                ) == 0
            {
                self.restore_transparency(target.hwnd);
            }
        }
    }

    fn send_to_back(&self, target: Target) {
        let hwnd = target.hwnd as HWND;
        if !same_target(target)
            || !visible_caption(hwnd)
            || rect(hwnd).map(bounds) != Some(bounds(target.rect))
            || integrity(target.pid) != Some(self.integrity)
        {
            log("CANCEL: send-to-back target changed after hit-test");
            return;
        }
        // HWND_BOTTOM also clears Always on Top; keep geometry and activation unchanged.
        unsafe {
            if SetWindowPos(
                hwnd,
                HWND_BOTTOM,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_ASYNCWINDOWPOS,
            ) == 0
            {
                log(format!("FAIL send-to-back hwnd={:#x}", target.hwnd));
            }
        }
    }

    fn toggle(&mut self, target: Target) {
        let hwnd = target.hwnd as HWND;
        if !same_target(target)
            || !ordinary(hwnd)
            || rect(hwnd).map(bounds) != Some(bounds(target.rect))
            || integrity(target.pid) != Some(self.integrity)
        {
            log("CANCEL: target changed after hit-test");
            return;
        }
        if self.windows.contains_key(&target.hwnd) {
            self.unroll(target.hwnd);
            return;
        }
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        if dpi == 0 {
            return;
        }
        let height = rolled_height(dpi);
        let (width, old_height) = dimensions(target.rect);
        if height <= 0 || height >= old_height {
            return;
        }
        let marker = self.next_marker;
        self.next_marker += 1;
        if unsafe { SetPropW(hwnd, self.property.as_ptr(), marker as HANDLE) } == 0 {
            log("REJECT: could not mark window identity");
            return;
        }
        self.windows.insert(
            target.hwnd,
            Saved {
                target,
                marker,
                dpi,
            },
        );
        if self.resize(
            target.hwnd,
            target.rect.left,
            target.rect.top,
            width,
            height,
        ) {
            log(format!(
                "ROLL hwnd={:#x} pid={} dpi={} expanded={}x{} rolled={}x{}",
                target.hwnd, target.pid, dpi, width, old_height, width, height
            ));
        } else {
            log(format!("FAIL roll hwnd={:#x}; restoring", target.hwnd));
            self.unroll(target.hwnd);
        }
    }

    fn resize(&self, key: isize, x: i32, y: i32, width: i32, height: i32) -> bool {
        let Some(saved) = self.windows.get(&key) else {
            return false;
        };
        if !self.owns(saved) {
            return false;
        }
        // ASYNC avoids blocking on a hung target; retain recovery state until geometry is observed.
        if unsafe {
            SetWindowPos(
                key as HWND,
                null_mut(),
                x,
                y,
                width,
                height,
                SWP_NOZORDER
                    | SWP_NOACTIVATE
                    | SWP_NOOWNERZORDER
                    | SWP_NOSENDCHANGING
                    | SWP_ASYNCWINDOWPOS,
            )
        } == 0
        {
            return false;
        }
        let deadline = Instant::now() + Duration::from_millis(350);
        let mut stable = 0;
        while Instant::now() < deadline {
            if !self.owns(saved) {
                return false;
            }
            if rect(key as HWND)
                .is_some_and(|r| r.left == x && r.top == y && dimensions(r) == (width, height))
            {
                stable += 1;
                if stable == 5 {
                    return true;
                }
            } else {
                stable = 0;
            }
            thread::sleep(Duration::from_millis(20));
        }
        false
    }

    fn maintain_rolled_size(&mut self) {
        for key in self.windows.keys().copied().collect::<Vec<_>>() {
            let saved = &self.windows[&key];
            if !self.owns(saved) {
                continue;
            }
            let dpi = unsafe { GetDpiForWindow(key as HWND) };
            if dpi == 0 {
                continue;
            }
            let Some(current) = rect(key as HWND) else {
                continue;
            };
            let height = rolled_height(dpi);
            // Keep the saved physical width: DPI-driven width changes shift the
            // monitor boundary while a rolled window is being dragged.
            let width = dimensions(saved.target.rect).0;
            if height > 0 && dimensions(current) != (width, height) {
                if DRAG_ACTIVE.load(Ordering::Relaxed) {
                    // The target can resize itself in WM_DPICHANGED. Do not rewind its
                    // position or block subsequent mouse updates while correcting height.
                    unsafe {
                        SetWindowPos(
                            key as HWND,
                            null_mut(),
                            0,
                            0,
                            width,
                            height,
                            SWP_NOMOVE
                                | SWP_NOZORDER
                                | SWP_NOACTIVATE
                                | SWP_NOOWNERZORDER
                                | SWP_NOSENDCHANGING
                                | SWP_ASYNCWINDOWPOS,
                        );
                    }
                    continue;
                }
                if !self.resize(key, current.left, current.top, width, height) {
                    continue;
                }
                log(format!(
                    "ROLLED height corrected hwnd={key:#x} dpi={dpi} height={height}"
                ));
            }
            if dpi != saved.dpi {
                self.windows.get_mut(&key).unwrap().dpi = dpi;
                log(format!(
                    "DPI hwnd={key:#x} dpi={dpi} rolled_height={height}"
                ));
            }
        }
    }

    fn move_rolled(&self, position: DragPosition) {
        let Some(saved) = self.windows.get(&position.key) else {
            return;
        };
        if !self.owns(saved) {
            return;
        }
        let hwnd = position.key as HWND;
        let width = dimensions(saved.target.rect).0;
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        let height = rolled_height(dpi);
        if dpi == 0 || height <= 0 {
            return;
        }
        if position.finished {
            if self.resize(position.key, position.x, position.y, width, height) {
                log(format!(
                    "DRAG verified hwnd={:#x} position={},{}",
                    position.key, position.x, position.y
                ));
            }
        } else {
            // Keep the target out of its native move loop, which enforces its minimum height.
            unsafe {
                SetWindowPos(
                    hwnd,
                    HWND_TOP,
                    position.x,
                    position.y,
                    width,
                    height,
                    SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOSENDCHANGING | SWP_ASYNCWINDOWPOS,
                );
            }
        }
    }

    fn focus_rolled(&self, key: isize) {
        let Some(saved) = self.windows.get(&key) else {
            return;
        };
        if !self.owns(saved) {
            return;
        }
        unsafe {
            let mut activated = SetForegroundWindow(key as HWND);
            if activated == 0
                && GetAsyncKeyState(VK_MENU as i32) >= 0
                && [VK_CONTROL, VK_SHIFT, VK_LWIN, VK_RWIN]
                    .into_iter()
                    .all(|key| GetAsyncKeyState(key as i32) >= 0)
            {
                let alt = [0, KEYEVENTF_KEYUP].map(|flags| INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_MENU,
                            dwFlags: flags,
                            dwExtraInfo: ALT_TAP_TAG,
                            ..Default::default()
                        },
                    },
                });
                let sent = SendInput(2, alt.as_ptr(), size_of::<INPUT>() as i32);
                if sent == 1 {
                    let mut released = false;
                    for _ in 0..3 {
                        if SendInput(1, alt[1..].as_ptr(), size_of::<INPUT>() as i32) == 1 {
                            released = true;
                            break;
                        }
                        thread::sleep(Duration::from_millis(5));
                    }
                    if !released {
                        keybd_event(VK_MENU as u8, 0, KEYEVENTF_KEYUP, ALT_TAP_TAG);
                    }
                }
                if sent == 2 {
                    activated = SetForegroundWindow(key as HWND);
                }
            }
            if activated == 0 {
                log(format!("FOCUS could not activate hwnd={key:#x}"));
            }
        }
    }

    fn unroll(&mut self, key: isize) {
        let Some(saved) = self.windows.get(&key) else {
            return;
        };
        if !self.owns(saved) {
            self.windows.remove(&key);
            log("CLEAN: closed or replaced window");
            return;
        }
        let Some(current) = rect(key as HWND) else {
            return;
        };
        let (width, height) = dimensions(saved.target.rect);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let monitor = unsafe { MonitorFromWindow(key as HWND, MONITOR_DEFAULTTONEAREST) };
        if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
            return;
        }
        let work = info.rcWork;
        let current_dpi = unsafe { GetDpiForWindow(key as HWND) };
        if current_dpi == 0 {
            return;
        }
        let (x, y) = crate::geometry::restore_position(
            current.left,
            current.top,
            width,
            rolled_height(current_dpi),
            [work.left, work.top, work.right, work.bottom],
        );
        log(format!(
            "UNROLL attempt hwnd={key:#x} dpi={}->{} size={width}x{height} position={x},{y}",
            saved.dpi, current_dpi
        ));
        if self.resize(key, x, y, width, height) {
            unsafe {
                RemovePropW(key as HWND, self.property.as_ptr());
            }
            self.windows.remove(&key);
            log(format!("UNROLL verified hwnd={key:#x}"));
        } else {
            log(format!(
                "RECOVERY NEEDED hwnd={key:#x}; use Unroll all or Exit to retry"
            ));
        }
    }

    fn restore_all(&mut self) -> bool {
        for key in self.windows.keys().copied().collect::<Vec<_>>() {
            self.unroll(key);
        }
        self.windows.is_empty()
    }

    fn publish_recovery(&self) {
        let mut affected = Vec::new();
        for target in self
            .windows
            .values()
            .filter(|_| self.exit_pending || self.roll_recovery)
            .map(|saved| saved.target)
            .chain(
                self.topmost
                    .values()
                    .filter(|_| self.exit_pending)
                    .map(|(target, _, _)| *target),
            )
            .chain(
                self.transparent
                    .values()
                    .filter(|_| self.exit_pending)
                    .map(|saved| saved.target),
            )
            .chain(
                self.hidden
                    .values()
                    .filter(|_| self.exit_pending)
                    .map(|saved| saved.target),
            )
        {
            let mut title = [0u16; 128];
            let length = unsafe { GetWindowTextW(target.hwnd as HWND, title.as_mut_ptr(), 128) };
            let name = if length > 0 {
                String::from_utf16_lossy(&title[..length as usize])
            } else {
                "(untitled)".into()
            };
            affected.push(format!(
                "{name} (PID {}, HWND {:#x})",
                target.pid, target.hwnd
            ));
        }
        affected.sort();
        affected.dedup();
        let pending = !affected.is_empty();
        *RECOVERY_WINDOWS.lock().unwrap() = affected;
        RECOVERY_PENDING.store(pending, Ordering::Relaxed);
    }

    // Returns true only when Exit has verified every surviving managed window.
    fn control(&mut self, action: u32) -> bool {
        if action == TOGGLE_ENABLED {
            return self.control(if STOPPING.load(Ordering::Relaxed) {
                ENABLE
            } else {
                PAUSE
            });
        }
        if action == ENABLE {
            if !self.exit_pending
                && (self.windows.is_empty() || !RECOVERY_PENDING.load(Ordering::Relaxed))
            {
                STOPPING.store(false, Ordering::Relaxed);
            }
            return false;
        }
        if action == PAUSE {
            STOPPING.store(true, Ordering::Relaxed);
            return false;
        }
        if action == EXIT {
            STOPPING.store(true, Ordering::Relaxed);
            self.exit_pending = true;
        }
        if self.exit_pending {
            self.restore_all_hidden();
        }
        let mut restored = self.restore_all();
        self.roll_recovery = !restored;
        if self.exit_pending {
            self.restore_topmost();
            self.restore_all_transparency();
            restored &=
                self.topmost.is_empty() && self.transparent.is_empty() && self.hidden.is_empty();
        }
        self.publish_recovery();
        log(format!("CONTROL action={action} restored={restored}"));
        self.exit_pending && restored
    }
}

unsafe extern "system" fn mouse_hook(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
    // SAFETY: for HC_ACTION Windows supplies a valid MSLLHOOKSTRUCT during this callback.
    unsafe {
        if code == HC_ACTION as i32 {
            if message as u32 == WM_RBUTTONUP && SUPPRESS_UP.swap(false, Ordering::Relaxed) {
                return 1;
            }
            if message as u32 == WM_MBUTTONUP && SUPPRESS_MIDDLE_UP.swap(false, Ordering::Relaxed) {
                return 1;
            }
            let event = &*(data as *const MSLLHOOKSTRUCT);
            if matches!(message as u32, WM_MOUSEMOVE | WM_LBUTTONUP)
                && let Ok(mut active) = DRAG.lock()
                && let Some(drag) = active.as_mut()
            {
                if !drag.moved
                    && (event.pt.x.abs_diff(drag.start.x)
                        >= GetSystemMetrics(SM_CXDRAG).max(1) as u32
                        || event.pt.y.abs_diff(drag.start.y)
                            >= GetSystemMetrics(SM_CYDRAG).max(1) as u32)
                {
                    drag.moved = true;
                }
                if drag.moved
                    && let Ok(mut pending) = DRAG_POSITION.lock()
                {
                    *pending = Some(DragPosition {
                        key: drag.target.hwnd,
                        x: drag
                            .target
                            .rect
                            .left
                            .saturating_add(event.pt.x.saturating_sub(drag.start.x)),
                        y: drag
                            .target
                            .rect
                            .top
                            .saturating_add(event.pt.y.saturating_sub(drag.start.y)),
                        finished: message as u32 == WM_LBUTTONUP,
                    });
                }
                if message as u32 == WM_LBUTTONUP {
                    // ponytail: a plain rolled-caption click only focuses; replay clicks if double-click is needed.
                    *active = None;
                    DRAG_ACTIVE.store(false, Ordering::Relaxed);
                }
                if message as u32 == WM_MOUSEMOVE {
                    return CallNextHookEx(null_mut(), code, message, data);
                }
                return 1;
            }
            if message as u32 == WM_LBUTTONDOWN && event.flags & LLMHF_LOWER_IL_INJECTED == 0 {
                let (reply, receive) = mpsc::sync_channel(1);
                if let Some(sender) = COMMANDS.get()
                    && sender
                        .try_send(Command::ProbeRolled(event.pt, reply))
                        .is_ok()
                    && let Ok(Some(target)) = receive.recv_timeout(Duration::from_millis(40))
                    && let Ok(mut active) = DRAG.lock()
                {
                    *active = Some(Drag {
                        target,
                        start: event.pt,
                        moved: false,
                    });
                    if sender
                        .try_send(Command::FocusRolled(target.hwnd, Instant::now()))
                        .is_ok()
                    {
                        DRAG_ACTIVE.store(true, Ordering::Relaxed);
                        return 1;
                    }
                    *active = None;
                }
            }
            if matches!(message as u32, WM_RBUTTONDOWN | WM_MBUTTONDOWN)
                && event.flags & LLMHF_LOWER_IL_INJECTED == 0
                && !STOPPING.load(Ordering::Relaxed)
            {
                let (reply, receive) = mpsc::sync_channel(1);
                if let Some(sender) = COMMANDS.get() {
                    // A timed-out probe cannot change a window: only the subsequent command can.
                    let middle = message as u32 == WM_MBUTTONDOWN;
                    let probe = if middle {
                        Command::ProbeMiddle(event.pt, reply)
                    } else {
                        Command::ProbeRight(event.pt, reply)
                    };
                    if sender.try_send(probe).is_ok()
                        && let Ok(Some((target, hit))) =
                            receive.recv_timeout(Duration::from_millis(40))
                        && sender
                            .try_send(if middle && hit == HTMINBUTTON {
                                Command::MinimizeToTray(target)
                            } else if middle && hit == HTCAPTION {
                                Command::ToggleTransparency(target)
                            } else if middle {
                                Command::ToggleTopmost(target)
                            } else if hit == HTCLOSE {
                                Command::SendToBack(target)
                            } else {
                                Command::Toggle(target)
                            })
                            .is_ok()
                    {
                        if middle {
                            SUPPRESS_MIDDLE_UP.store(true, Ordering::Relaxed);
                        } else {
                            SUPPRESS_UP.store(true, Ordering::Relaxed);
                        }
                        return 1;
                    }
                }
            }
        }
        CallNextHookEx(null_mut(), code, message, data)
    }
}

fn request_control(action: u32) {
    if action == PAUSE || action == EXIT {
        STOPPING.store(true, Ordering::Relaxed);
    }
    // Lifecycle requests cannot be lost behind a full mouse-probe queue.
    CONTROL_REQUESTS.fetch_or(action, Ordering::Relaxed);
}

fn tray_icon(hwnd: HWND, operation: u32) -> bool {
    let Some(&(expanded, rolled)) = TRAY_ICONS
        .get_or_init(|| {
            let expanded = icon_from_ico(include_bytes!("../assets/icons/winroll-expanded.ico"))?;
            let Some(rolled) = icon_from_ico(include_bytes!("../assets/icons/winroll-rolled.ico"))
            else {
                unsafe { DestroyIcon(expanded) };
                return None;
            };
            Some((expanded as isize, rolled as isize))
        })
        .as_ref()
    else {
        return false;
    };
    let mut icon = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
        uCallbackMessage: TRAY_CALLBACK,
        hIcon: if HAS_ROLLED_WINDOWS.load(Ordering::Relaxed) {
            rolled as HICON
        } else {
            expanded as HICON
        },
        ..Default::default()
    };
    let tip = if RECOVERY_PENDING.load(Ordering::Relaxed) {
        "WinRoll RS - unroll pending; retry Unroll all or Exit"
    } else if STOPPING.load(Ordering::Relaxed) {
        "WinRoll RS - paused"
    } else {
        "WinRoll RS - enabled"
    };
    for (slot, value) in icon.szTip.iter_mut().zip(tip.encode_utf16()) {
        *slot = value;
    }
    // SAFETY: icon is initialized and lives through each shell call; the stock icon is shared.
    unsafe {
        if Shell_NotifyIconW(operation, &icon) == 0 {
            return false;
        }
        if operation == NIM_ADD {
            icon.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            return Shell_NotifyIconW(NIM_SETVERSION, &icon) != 0;
        }
    }
    true
}

fn icon_from_ico(data: &[u8]) -> Option<HICON> {
    let count = u16::from_le_bytes(data.get(4..6)?.try_into().ok()?) as usize;
    let entries = data.get(6..6 + count * 16)?;
    let entry = entries
        .as_chunks::<16>()
        .0
        .iter()
        .find(|entry| entry[0] == 32)?;
    let length = u32::from_le_bytes(entry[8..12].try_into().ok()?) as usize;
    let offset = u32::from_le_bytes(entry[12..16].try_into().ok()?) as usize;
    let bits = data.get(offset..offset.checked_add(length)?)?;
    let icon =
        unsafe { CreateIconFromResourceEx(bits.as_ptr(), length as u32, 1, 0x30000, 32, 32, 0) };
    (!icon.is_null()).then_some(icon)
}

#[cfg(test)]
#[test]
fn embedded_tray_icons_load() {
    for data in [
        &include_bytes!("../assets/icons/winroll-expanded.ico")[..],
        &include_bytes!("../assets/icons/winroll-rolled.ico")[..],
    ] {
        let icon = icon_from_ico(data).expect("embedded tray icon should load");
        unsafe { DestroyIcon(icon) };
    }
}

unsafe extern "system" fn about_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        match message {
            WM_CREATE => {
                let dpi = GetDpiForWindow(hwnd) as i32;
                let scale = |n| n * dpi / 96;
                let button = CreateWindowExW(
                    0,
                    w!("BUTTON"),
                    w!("Close"),
                    WS_CHILD | WS_VISIBLE | BS_DEFPUSHBUTTON as u32,
                    scale(216),
                    scale(104),
                    scale(80),
                    scale(28),
                    hwnd,
                    1usize as HMENU,
                    GetModuleHandleW(null_mut()),
                    null_mut(),
                );
                SendMessageW(
                    button,
                    WM_SETFONT,
                    GetStockObject(DEFAULT_GUI_FONT) as usize,
                    1,
                );
                SetFocus(button);
                return 0;
            }
            WM_PAINT => {
                let mut paint = PAINTSTRUCT::default();
                let dc = BeginPaint(hwnd, &mut paint);
                let dpi = GetDpiForWindow(hwnd) as i32;
                let scale = |n| n * dpi / 96;
                if let Some(Some((expanded, _))) = TRAY_ICONS.get() {
                    DrawIconEx(
                        dc,
                        scale(20),
                        scale(20),
                        *expanded as HICON,
                        scale(32),
                        scale(32),
                        0,
                        null_mut(),
                        DI_NORMAL,
                    );
                }
                SelectObject(dc, GetStockObject(DEFAULT_GUI_FONT));
                SetBkMode(dc, TRANSPARENT as i32);
                for (line, y) in [
                    ("WinRoll RS", 22),
                    (concat!("Version ", env!("CARGO_PKG_VERSION")), 48),
                    ("Created by Kin Tam", 74),
                ] {
                    let wide: Vec<u16> = line.encode_utf16().collect();
                    TextOutW(dc, scale(68), scale(y), wide.as_ptr(), wide.len() as i32);
                }
                EndPaint(hwnd, &paint);
                return 0;
            }
            WM_COMMAND if w & 0xffff == 1 => {
                DestroyWindow(hwnd);
                return 0;
            }
            WM_CLOSE => {
                DestroyWindow(hwnd);
                return 0;
            }
            WM_DESTROY => {
                ABOUT_WINDOW.store(0, Ordering::Relaxed);
                return 0;
            }
            _ => {}
        }
        DefWindowProcW(hwnd, message, w, l)
    }
}

fn options_error(owner: HWND, error: &io::Error) {
    let text: Vec<u16> = format!("Cannot access the Windows startup setting.\n\n{error}")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    unsafe {
        MessageBoxW(
            owner,
            text.as_ptr(),
            w!("WinRoll RS Options"),
            MB_OK | MB_ICONERROR,
        )
    };
}

fn update_transparency_label(hwnd: HWND) {
    let text: Vec<u16> = format!(
        "&Transparency: {}% (0% opaque, 100% invisible)\0",
        TRANSPARENCY.load(Ordering::Relaxed)
    )
    .encode_utf16()
    .collect();
    unsafe {
        SetWindowTextW(GetDlgItem(hwnd, 5), text.as_ptr());
    }
}

unsafe extern "system" fn options_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: this thread owns the options window and its standard child controls.
    unsafe {
        match message {
            WM_CREATE => {
                let enabled = match crate::startup::enabled() {
                    Ok(enabled) => enabled,
                    Err(error) => {
                        options_error(hwnd, &error);
                        return -1;
                    }
                };
                let dpi = GetDpiForWindow(hwnd) as i32;
                let scale = |n| n * dpi / 96;
                for (id, text, style, x, y, width, height) in [
                    (
                        3,
                        w!("&Automatically start with Windows"),
                        BS_AUTOCHECKBOX,
                        20,
                        22,
                        300,
                        28,
                    ),
                    (2, w!("Close"), BS_DEFPUSHBUTTON, 236, 180, 80, 28),
                ] {
                    let control = CreateWindowExW(
                        0,
                        w!("BUTTON"),
                        text,
                        WS_CHILD | WS_VISIBLE | WS_TABSTOP | style as u32,
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
                let controls = INITCOMMONCONTROLSEX {
                    dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
                    dwICC: ICC_BAR_CLASSES,
                };
                if InitCommonControlsEx(&controls) == 0 {
                    return -1;
                }
                for (id, class, text, style, y, height) in [
                    (5, w!("STATIC"), w!(""), 0, 62, 24),
                    (
                        4,
                        TRACKBAR_CLASSW,
                        w!("Transparency"),
                        WS_TABSTOP | TBS_AUTOTICKS,
                        88,
                        36,
                    ),
                    (
                        6,
                        w!("STATIC"),
                        w!("Invisible windows: choose Exit in the tray to restore."),
                        0,
                        130,
                        40,
                    ),
                ] {
                    let control = CreateWindowExW(
                        0,
                        class,
                        text,
                        WS_CHILD | WS_VISIBLE | style,
                        scale(20),
                        scale(y),
                        scale(300),
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
                let slider = GetDlgItem(hwnd, 4);
                SendMessageW(slider, TBM_SETRANGE, 1, 10 << 16);
                SendMessageW(slider, TBM_SETPAGESIZE, 0, 1);
                SendMessageW(
                    slider,
                    TBM_SETPOS,
                    1,
                    (TRANSPARENCY.load(Ordering::Relaxed) / 10) as isize,
                );
                // Keep keyboard order checkbox, labelled slider, Close.
                SetWindowPos(
                    GetDlgItem(hwnd, 5),
                    GetDlgItem(hwnd, 3),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
                SetWindowPos(
                    slider,
                    GetDlgItem(hwnd, 5),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
                update_transparency_label(hwnd);
                let checkbox = GetDlgItem(hwnd, 3);
                SendMessageW(
                    checkbox,
                    BM_SETCHECK,
                    if enabled { BST_CHECKED } else { BST_UNCHECKED } as usize,
                    0,
                );
                SetFocus(checkbox);
                return 0;
            }
            WM_HSCROLL if l == GetDlgItem(hwnd, 4) as isize => {
                let slider = GetDlgItem(hwnd, 4);
                let percent = SendMessageW(slider, TBM_GETPOS, 0, 0) as u32 * 10;
                if percent != TRANSPARENCY.load(Ordering::Relaxed) {
                    match crate::transparency_settings::save(percent) {
                        Ok(()) => {
                            TRANSPARENCY.store(percent, Ordering::Relaxed);
                        }
                        Err(error) => {
                            SendMessageW(
                                slider,
                                TBM_SETPOS,
                                1,
                                (TRANSPARENCY.load(Ordering::Relaxed) / 10) as isize,
                            );
                            options_error(hwnd, &error);
                        }
                    }
                    update_transparency_label(hwnd);
                }
                return 0;
            }
            WM_COMMAND if w & 0xffff == 3 && (w >> 16) == BN_CLICKED as usize => {
                let checkbox = GetDlgItem(hwnd, 3);
                let enabled = SendMessageW(checkbox, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
                if let Err(error) = crate::startup::set_enabled(enabled) {
                    SendMessageW(
                        checkbox,
                        BM_SETCHECK,
                        if enabled { BST_UNCHECKED } else { BST_CHECKED } as usize,
                        0,
                    );
                    options_error(hwnd, &error);
                }
                return 0;
            }
            WM_COMMAND if matches!(w & 0xffff, 1 | 2) => {
                DestroyWindow(hwnd);
                return 0;
            }
            WM_CLOSE => {
                DestroyWindow(hwnd);
                return 0;
            }
            WM_DESTROY => {
                OPTIONS_WINDOW.store(0, Ordering::Relaxed);
                return 0;
            }
            _ => {}
        }
        DefWindowProcW(hwnd, message, w, l)
    }
}

fn show_auxiliary(owner: HWND, options: bool) {
    unsafe {
        let window = if options {
            &OPTIONS_WINDOW
        } else {
            &ABOUT_WINDOW
        };
        let existing = window.load(Ordering::Relaxed) as HWND;
        if !existing.is_null() {
            SetForegroundWindow(existing);
            return;
        }
        let class = WNDCLASSW {
            lpfnWndProc: Some(if options { options_proc } else { about_proc }),
            hInstance: GetModuleHandleW(null_mut()),
            hbrBackground: GetSysColorBrush(if options { COLOR_BTNFACE } else { COLOR_WINDOW }),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            lpszClassName: if options {
                w!("WinRollOptions")
            } else {
                w!("WinRollAbout")
            },
            ..Default::default()
        };
        RegisterClassW(&class);
        let mut cursor = POINT::default();
        GetCursorPos(&mut cursor);
        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(monitor, &mut info);
        let mut dpi = 96;
        let mut dpi_y = 96;
        if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut dpi_y) < 0 {
            dpi = GetDpiForSystem();
        }
        let mut bounds = RECT {
            left: 0,
            top: 0,
            right: (if options { 340 } else { 320 }) * dpi as i32 / 96,
            bottom: (if options { 228 } else { 150 }) * dpi as i32 / 96,
        };
        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU;
        AdjustWindowRectExForDpi(&mut bounds, style, 0, WS_EX_DLGMODALFRAME, dpi);
        let width = bounds.right - bounds.left;
        let height = bounds.bottom - bounds.top;
        let hwnd = CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            class.lpszClassName,
            if options {
                w!("Options")
            } else {
                w!("About WinRoll RS")
            },
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
        if !hwnd.is_null() {
            window.store(hwnd as isize, Ordering::Relaxed);
            if let Some(Some((expanded, _))) = TRAY_ICONS.get() {
                SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, *expanded);
            }
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            if options {
                SetFocus(GetDlgItem(hwnd, 3));
            }
        }
    }
}

unsafe extern "system" fn tray_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: this thread owns the hidden tray window, menu and message loop.
    unsafe {
        if TASKBAR_CREATED.get().is_some_and(|id| message == *id) {
            tray_icon(hwnd, NIM_ADD);
            request_control(RECREATE_TRAY);
            return 0;
        }
        match message {
            WINDOW_TRAY_CALLBACK => {
                // These icons use the original callback format: w is the full icon ID.
                if matches!(
                    l as u32,
                    WM_LBUTTONUP | WM_RBUTTONUP | NIN_SELECT | NIN_KEYSELECT
                ) {
                    RESTORE_TRAY_REQUESTS.lock().unwrap().push(w as u32);
                }
                return 0;
            }
            WORKER_DONE => {
                // TrackPopupMenu has its own message loop. Completion must also close that loop.
                EndMenu();
                return 0;
            }
            TRAY_CALLBACK if matches!(l as u32 & 0xffff, NIN_SELECT | NIN_KEYSELECT) => {
                if !RECOVERY_PENDING.load(Ordering::Relaxed) {
                    // Preserve click parity even when two clicks arrive before the worker wakes.
                    CONTROL_REQUESTS.fetch_xor(TOGGLE_ENABLED, Ordering::Relaxed);
                }
                return 0;
            }
            TRAY_CALLBACK if l as u32 & 0xffff == WM_CONTEXTMENU => {
                let menu = CreatePopupMenu();
                if menu.is_null() {
                    return 0;
                }
                let paused = STOPPING.load(Ordering::Relaxed);
                let flags = if paused && RECOVERY_PENDING.load(Ordering::Relaxed) {
                    MF_STRING | MF_GRAYED
                } else {
                    MF_STRING
                };
                AppendMenuW(
                    menu,
                    flags,
                    if paused { ENABLE } else { PAUSE } as usize,
                    if paused { w!("&Enable") } else { w!("&Pause") },
                );
                let affected = RECOVERY_WINDOWS.lock().unwrap();
                if !affected.is_empty() {
                    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
                    for name in affected.iter() {
                        let label: Vec<u16> = format!("Recovery needed: {name}")
                            .encode_utf16()
                            .chain(Some(0))
                            .collect();
                        AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, label.as_ptr());
                    }
                }
                let retry = !affected.is_empty();
                drop(affected);
                AppendMenuW(
                    menu,
                    MF_STRING,
                    UNROLL_ALL as usize,
                    if retry {
                        w!("&Retry unroll all")
                    } else {
                        w!("&Unroll all")
                    },
                );
                AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
                AppendMenuW(menu, MF_STRING, SETTINGS as usize, w!("&Options..."));
                AppendMenuW(menu, MF_STRING, ABOUT as usize, w!("&About..."));
                AppendMenuW(menu, MF_STRING, EXIT as usize, w!("E&xit"));
                // Version 4 supplies the icon anchor for keyboard as well as mouse activation.
                let x = w as i16 as i32;
                let y = (w >> 16) as i16 as i32;
                SetForegroundWindow(hwnd);
                let action = TrackPopupMenu(
                    menu,
                    TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                    x,
                    y,
                    0,
                    hwnd,
                    std::ptr::null(),
                ) as u32;
                DestroyMenu(menu);
                PostMessageW(hwnd, WM_NULL, 0, 0);
                tray_icon(hwnd, NIM_SETFOCUS);
                if action == ABOUT {
                    show_auxiliary(hwnd, false);
                }
                if action == SETTINGS {
                    show_auxiliary(hwnd, true);
                }
                if matches!(action, ENABLE | PAUSE | UNROLL_ALL | EXIT) {
                    request_control(action);
                }
                return 0;
            }
            TRAY_UPDATE => {
                tray_icon(hwnd, NIM_MODIFY);
                return 0;
            }
            WM_CLOSE => {
                request_control(EXIT);
                return 0;
            }
            _ => {}
        }
        DefWindowProcW(hwnd, message, w, l)
    }
}

pub fn report_error(error: &str) {
    log(error);
    if std::env::args().len() == 1 {
        let text: Vec<u16> = error.encode_utf16().chain(Some(0)).collect();
        unsafe {
            MessageBoxW(
                null_mut(),
                text.as_ptr(),
                w!("WinRoll RS"),
                MB_OK | MB_ICONERROR,
            );
        }
    }
}

pub fn run() -> Result<(), String> {
    unsafe {
        if std::env::args().len() > 1 {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
        if SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) == 0 {
            return Err("Cannot enable per-monitor DPI awareness".into());
        }
        EnumDisplayMonitors(null_mut(), std::ptr::null(), Some(report_monitor), 0);
    }
    if std::env::args().any(|arg| arg == "--inventory") {
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--self-test") {
        return self_test(false);
    }
    if std::env::args().any(|arg| arg == "--fixture") {
        return self_test(true);
    }
    let mut manager = Manager::new()?;
    TRANSPARENCY.store(
        crate::transparency_settings::load().map_err(|error| error.to_string())?,
        Ordering::Relaxed,
    );
    // Keep the experiment's mutex name so an old controller cannot run alongside the tray build.
    let mutex = unsafe { CreateMutexW(null_mut(), 0, w!("Local\\WinRoll-RS.Experiment")) };
    if mutex.is_null() {
        return Err("Cannot create WinRoll RS mutex".into());
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            CloseHandle(mutex);
        }
        return Err("Another WinRoll RS controller is already running".into());
    }
    let (sender, receiver) = mpsc::sync_channel(8);
    COMMANDS.set(sender).map_err(|_| "Already initialized")?;
    let tray = unsafe {
        let class = WNDCLASSW {
            lpfnWndProc: Some(tray_proc),
            hInstance: GetModuleHandleW(null_mut()),
            lpszClassName: w!("WinRollTray"),
            ..Default::default()
        };
        RegisterClassW(&class);
        let _ = TASKBAR_CREATED.set(RegisterWindowMessageW(w!("TaskbarCreated")));
        CreateWindowExW(
            0,
            class.lpszClassName,
            w!("WinRoll RS"),
            0,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            class.hInstance,
            null_mut(),
        )
    };
    if tray.is_null() || !tray_icon(tray, NIM_ADD) {
        unsafe {
            tray_icon(tray, NIM_DELETE);
            DestroyWindow(tray);
            CloseHandle(mutex);
        }
        return Err("Cannot create the WinRoll RS tray icon".into());
    }
    let hook = unsafe {
        SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(mouse_hook),
            GetModuleHandleW(null_mut()),
            0,
        )
    };
    if hook.is_null() {
        unsafe {
            tray_icon(tray, NIM_DELETE);
            DestroyWindow(tray);
            CloseHandle(mutex);
        }
        return Err("Cannot install mouse hook".into());
    }
    let tray_key = tray as isize;
    manager.tray_owner = tray_key;
    let worker = thread::spawn(move || {
        loop {
            let requests = CONTROL_REQUESTS.swap(0, Ordering::Relaxed);
            if requests & RECREATE_TRAY != 0 {
                manager.recreate_hidden_icons();
            }
            if requests & !RECREATE_TRAY != 0 {
                // A stop request wins over Enable if multiple actions arrive while restoring.
                let action = [EXIT, PAUSE, UNROLL_ALL, ENABLE, TOGGLE_ENABLED]
                    .into_iter()
                    .find(|action| requests & action != 0)
                    .unwrap();
                if manager.control(action) {
                    break;
                }
                unsafe {
                    PostMessageW(tray_key as HWND, TRAY_UPDATE, 0, 0);
                }
            }
            let restore = std::mem::take(&mut *RESTORE_TRAY_REQUESTS.lock().unwrap());
            let restored_hidden = !restore.is_empty();
            for id in restore {
                manager.restore_hidden(id, true);
            }
            let hidden_changed = manager.clean_hidden() || restored_hidden;
            // Closing a rolled window needs no further user action to release its state.
            let closed: Vec<_> = manager
                .windows
                .iter()
                .filter(|(_, saved)| !manager.owns(saved))
                .map(|(key, _)| *key)
                .collect();
            let mut changed = hidden_changed || !closed.is_empty();
            for key in closed {
                manager.windows.remove(&key);
            }
            let closed: Vec<_> = manager
                .topmost
                .iter()
                .filter(|(_, (target, marker, _))| !manager.owns_topmost(*target, *marker))
                .map(|(key, _)| *key)
                .collect();
            changed |= !closed.is_empty();
            for key in closed {
                manager.topmost.remove(&key);
            }
            let stale: Vec<_> = manager
                .transparent
                .iter()
                .filter(|(_, saved)| !manager.owns_transparency(**saved))
                .map(|(key, _)| *key)
                .collect();
            changed |= !stale.is_empty();
            for key in stale {
                manager.transparent.remove(&key);
            }
            if changed && RECOVERY_PENDING.load(Ordering::Relaxed) {
                manager.publish_recovery();
                unsafe {
                    PostMessageW(tray_key as HWND, TRAY_UPDATE, 0, 0);
                }
            }
            if let Ok(mut pending) = DRAG_POSITION.lock()
                && let Some(position) = pending.take()
            {
                drop(pending);
                manager.move_rolled(position);
            }
            manager.maintain_rolled_size();
            let rolled = !manager.windows.is_empty();
            if HAS_ROLLED_WINDOWS.swap(rolled, Ordering::Relaxed) != rolled {
                unsafe { PostMessageW(tray_key as HWND, TRAY_UPDATE, 0, 0) };
            }
            if manager.exit_pending
                && manager.windows.is_empty()
                && manager.topmost.is_empty()
                && manager.transparent.is_empty()
                && manager.hidden.is_empty()
            {
                break;
            }
            let wait = if DRAG_ACTIVE.load(Ordering::Relaxed) {
                10
            } else {
                50
            };
            match receiver.recv_timeout(Duration::from_millis(wait)) {
                Ok(Command::ProbeRight(point, reply)) => {
                    let started = Instant::now();
                    let target = if STOPPING.load(Ordering::Relaxed) {
                        None
                    } else {
                        manager.probe_hits(point, &[HTCAPTION, HTCLOSE])
                    };
                    let _ = reply.try_send(target);
                    log(format!(
                        "PROBE eligible={} elapsed={}ms",
                        target.is_some(),
                        started.elapsed().as_millis()
                    ));
                }
                Ok(Command::Toggle(target)) if !STOPPING.load(Ordering::Relaxed) => {
                    manager.toggle(target)
                }
                Ok(Command::ProbeMiddle(point, reply)) => {
                    let target = if STOPPING.load(Ordering::Relaxed) {
                        None
                    } else {
                        manager.probe_hits(point, &[HTCAPTION, HTCLOSE, HTMINBUTTON])
                    };
                    let _ = reply.try_send(target);
                }
                Ok(Command::ToggleTopmost(target)) if !STOPPING.load(Ordering::Relaxed) => {
                    manager.toggle_topmost(target)
                }
                Ok(Command::MinimizeToTray(target)) if !STOPPING.load(Ordering::Relaxed) => {
                    manager.minimize_to_tray(target)
                }
                Ok(Command::ToggleTransparency(target)) if !STOPPING.load(Ordering::Relaxed) => {
                    manager.toggle_transparency(target)
                }
                Ok(Command::SendToBack(target)) if !STOPPING.load(Ordering::Relaxed) => {
                    manager.send_to_back(target)
                }
                Ok(Command::ProbeRolled(point, reply)) => {
                    let target = manager.probe(point).filter(|target| {
                        manager
                            .windows
                            .get(&target.hwnd)
                            .is_some_and(|saved| manager.owns(saved))
                    });
                    let _ = reply.try_send(target);
                }
                Ok(Command::FocusRolled(key, pressed))
                    if pressed.elapsed() <= Duration::from_millis(100) =>
                {
                    manager.focus_rolled(key)
                }
                _ => {}
            }
        }
        WORKER_FINISHED.store(true, Ordering::Relaxed);
        unsafe {
            PostMessageW(tray_key as HWND, WORKER_DONE, 0, 0);
        }
    });
    log("WinRoll RS enabled. Use the tray menu to Pause, Unroll all or Exit.");
    pump_until_restored();
    let result = worker
        .join()
        .map_err(|_| "Window worker panicked".to_owned());
    unsafe {
        UnhookWindowsHookEx(hook);
        tray_icon(tray, NIM_DELETE);
        let about = ABOUT_WINDOW.load(Ordering::Relaxed) as HWND;
        if !about.is_null() {
            DestroyWindow(about);
        }
        let options = OPTIONS_WINDOW.load(Ordering::Relaxed) as HWND;
        if !options.is_null() {
            DestroyWindow(options);
        }
        if let Some(Some((expanded, rolled))) = TRAY_ICONS.get() {
            DestroyIcon(*expanded as HICON);
            DestroyIcon(*rolled as HICON);
        }
        DestroyWindow(tray);
        CloseHandle(mutex);
    }
    result
}

fn pump_until_restored() {
    let mut msg = MSG::default();
    // SAFETY: this thread owns the hook and runs its required message loop until recovery completes.
    unsafe {
        while !WORKER_FINISHED.load(Ordering::Relaxed) {
            if GetMessageW(&mut msg, null_mut(), 0, 0) <= 0 {
                request_control(EXIT);
                thread::sleep(Duration::from_millis(50));
                continue;
            }
            let options = OPTIONS_WINDOW.load(Ordering::Relaxed) as HWND;
            if !options.is_null() && IsDialogMessageW(options, &msg) != 0 {
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn report_monitor(
    monitor: HMONITOR,
    _: HDC,
    _: *mut RECT,
    _: LPARAM,
) -> i32 {
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let (mut x, mut y) = (0, 0);
    // SAFETY: callback supplies a monitor handle; all output buffers are writable.
    unsafe {
        if GetMonitorInfoW(monitor, &mut info) != 0 {
            let dpi_result = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y);
            log(format!(
                "MONITOR bounds={:?} work={:?} dpi={x}x{y} dpi_result={dpi_result}",
                bounds(info.rcMonitor),
                bounds(info.rcWork)
            ));
        }
    }
    1
}

fn spawn_fixture(
    interactive: bool,
    second: bool,
) -> Result<(isize, thread::JoinHandle<()>), String> {
    let (ready, receive) = mpsc::sync_channel(1);
    let fixture = thread::spawn(move || unsafe {
        // SAFETY: this thread owns the fixture and its message loop until WM_DESTROY.
        let class = WNDCLASSW {
            lpfnWndProc: Some(fixture_proc),
            hInstance: GetModuleHandleW(null_mut()),
            hbrBackground: GetSysColorBrush(COLOR_WINDOW),
            lpszClassName: if second {
                w!("WinRollRecoveryFixture")
            } else {
                w!("WinRollExperimentFixture")
            },
            ..Default::default()
        };
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST,
            class.lpszClassName,
            if second {
                w!("WinRoll RS second native test fixture")
            } else if interactive {
                w!("WinRoll RS native test fixture (F8: refuse expansion)")
            } else {
                w!("WinRoll RS native test fixture")
            },
            WS_OVERLAPPEDWINDOW,
            if second {
                1000
            } else if interactive {
                0
            } else {
                100
            },
            100,
            if second { 600 } else { 800 },
            if second { 500 } else { 600 },
            null_mut(),
            null_mut(),
            class.hInstance,
            null_mut(),
        );
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        if !second {
            CreateWindowExW(
                0,
                w!("BUTTON"),
                w!("Do not roll from this control"),
                WS_CHILD | WS_VISIBLE,
                50,
                40,
                220,
                40,
                hwnd,
                42usize as HMENU,
                class.hInstance,
                null_mut(),
            );
        }
        if interactive {
            // The first ShowWindow may honor a launcher's hidden-console startup flag.
            // The explicitly requested fixture must still expose its test window.
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        let _ = ready.send(hwnd as isize);
        if hwnd.is_null() {
            return;
        }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
    let key = receive.recv().map_err(|e| e.to_string())?;
    if key == 0 {
        return Err("Could not create native fixture".into());
    }
    Ok((key, fixture))
}

fn self_test(interactive: bool) -> Result<(), String> {
    let (key, fixture) = spawn_fixture(interactive, false)?;
    let hwnd = key as HWND;
    if interactive {
        log(format!(
            "FIXTURE hwnd={key:#x} integrity={:?}; close its window when finished",
            integrity(unsafe { GetCurrentProcessId() })
        ));
        return fixture
            .join()
            .map_err(|_| "Fixture thread panicked".to_owned());
    }
    let (second_key, second_fixture) = spawn_fixture(false, true)?;
    let second_hwnd = second_key as HWND;
    let mut manager = Manager::new()?;
    let result: Result<(), String> = (|| {
        let initial = rect(hwnd).ok_or("Fixture rectangle unavailable")?;
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        let point = POINT {
            x: initial.left + 100,
            y: initial.top
                + unsafe {
                    GetSystemMetricsForDpi(SM_CYFRAME, dpi)
                        + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi)
                        + 10
                },
        };
        let close_point = POINT {
            x: initial.right - 25,
            y: point.y,
        };
        for (point, expected) in [(point, HTCAPTION), (close_point, HTCLOSE)] {
            if manager
                .probe_hits(point, &[HTCAPTION, HTCLOSE])
                .map(|(_, hit)| hit)
                != Some(expected)
            {
                return Err("Right-click probe did not distinguish caption from Close".into());
            }
        }
        if manager.probe_hit(point, HTCLOSE).is_some()
            || manager
                .probe_hit(
                    POINT {
                        x: initial.left + 200,
                        y: initial.top + 200,
                    },
                    HTCLOSE,
                )
                .is_some()
            || manager.probe(close_point).is_some()
        {
            return Err("Close and caption gestures did not reject other hit regions".into());
        }
        self_test_topmost_exit(close_point)?;
        self_test_transparency(point)?;
        self_test_topmost(&mut manager, close_point, false)?;
        self_test_topmost(&mut manager, close_point, true)?;
        self_test_send_to_back(&manager, close_point, second_hwnd, false)?;
        self_test_send_to_back(&manager, close_point, second_hwnd, true)?;
        let mut stale = manager
            .probe_hit(close_point, HTCLOSE)
            .ok_or("Close unavailable")?;
        stale.pid = 0;
        manager.toggle_topmost(stale);
        manager.send_to_back(stale);
        if unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST == 0 {
            return Err("Stale target changed topmost state".into());
        }
        if manager
            .probe(POINT {
                x: initial.left + 200,
                y: initial.top + 200,
            })
            .is_some()
        {
            return Err("Client area incorrectly eligible".into());
        }
        let child = unsafe { GetDlgItem(hwnd, 42) };
        let child_rect = rect(child).ok_or("Fixture child unavailable")?;
        if manager
            .probe(POINT {
                x: child_rect.left + 10,
                y: child_rect.top + 10,
            })
            .is_some()
        {
            return Err("Child control incorrectly overridden by parent caption hit".into());
        }
        FIXTURE_CAPTION_EVERYWHERE.store(true, Ordering::Relaxed);
        let content = manager.probe(POINT {
            x: initial.left + 300,
            y: initial.top + 200,
        });
        let caption = manager.probe(point);
        FIXTURE_CAPTION_EVERYWHERE.store(false, Ordering::Relaxed);
        if content.is_some() || caption.is_none() {
            return Err("Caption hit-test escaped the title-bar band".into());
        }
        for cycle in 1..=20 {
            let target = manager
                .probe(point)
                .ok_or("Fixture caption was not eligible")?;
            manager.toggle(target);
            if manager.windows.is_empty()
                || dimensions(rect(hwnd).ok_or("Fixture closed")?).1 >= 600
            {
                return Err("Fixture did not stay rolled".into());
            }
            if !manager.restore_all() || rect(hwnd).map(bounds) != Some(bounds(initial)) {
                return Err("Fixture geometry not restored".into());
            }
            log(format!("PASS native cycle {cycle}/20"));
        }
        manager.toggle(manager.probe(point).ok_or("Fixture caption unavailable")?);
        self_test_topmost(&mut manager, close_point, false)?;
        self_test_topmost(&mut manager, close_point, true)?;
        self_test_send_to_back(&manager, close_point, second_hwnd, true)?;
        if !manager.restore_all() || rect(hwnd).map(bounds) != Some(bounds(initial)) {
            return Err("Rolled caption gesture checks did not restore geometry".into());
        }
        self_test_dpi_move(&mut manager, key, point)?;
        let mut monitor_info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
        if unsafe { GetMonitorInfoW(monitor, &mut monitor_info) } == 0 {
            return Err("Fixture work area unavailable".into());
        }
        let work = monitor_info.rcWork;
        let moved_x = (initial.left + 100).min(work.right - 800).max(work.left);
        let moved_y = (initial.top + 80).min(work.bottom - 600).max(work.top);
        manager.toggle(manager.probe(point).ok_or("Fixture caption unavailable")?);
        FIXTURE_MIN_TRACK.store(true, Ordering::Relaxed);
        manager.move_rolled(DragPosition {
            key,
            x: moved_x,
            y: moved_y,
            finished: false,
        });
        manager.move_rolled(DragPosition {
            key,
            x: moved_x,
            y: moved_y,
            finished: true,
        });
        if dimensions(rect(hwnd).ok_or("Fixture closed")?) != (800, rolled_height(dpi))
            || !manager.restore_all()
            || rect(hwnd).map(bounds) != Some([moved_x, moved_y, moved_x + 800, moved_y + 600])
        {
            return Err("Moved rolled fixture did not expand at its new position".into());
        }
        FIXTURE_MIN_TRACK.store(false, Ordering::Relaxed);
        let moved_point = POINT {
            x: moved_x + 100,
            y: moved_y + point.y - initial.top,
        };
        manager.toggle(
            manager
                .probe(moved_point)
                .ok_or("Moved fixture caption unavailable")?,
        );
        let current_dpi = unsafe { GetDpiForWindow(hwnd) };
        let expected_height = rolled_height(current_dpi);
        manager.windows.get_mut(&key).unwrap().dpi = current_dpi + 1;
        if !manager.resize(key, moved_x, moved_y, 800, expected_height + 10) {
            return Err("Could not simulate stale rolled caption height".into());
        }
        manager.maintain_rolled_size();
        if dimensions(rect(hwnd).ok_or("Fixture closed")?).1 != expected_height
            || manager.windows[&key].dpi != current_dpi
            || !manager.restore_all()
        {
            return Err("Rolled caption did not update for current DPI".into());
        }
        manager.toggle(
            manager
                .probe(moved_point)
                .ok_or("Moved fixture caption unavailable")?,
        );
        unsafe {
            SetWindowPos(
                hwnd,
                null_mut(),
                moved_x + 20,
                moved_y + 20,
                800,
                expected_height + 100,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOSENDCHANGING,
            );
        }
        DRAG_ACTIVE.store(true, Ordering::Relaxed);
        manager.maintain_rolled_size();
        DRAG_ACTIVE.store(false, Ordering::Relaxed);
        let deadline = Instant::now() + Duration::from_millis(350);
        while rect(hwnd).is_some_and(|r| dimensions(r).1 != expected_height)
            && Instant::now() < deadline
        {
            thread::sleep(Duration::from_millis(5));
        }
        if rect(hwnd).map(bounds)
            != Some([
                moved_x + 20,
                moved_y + 20,
                moved_x + 820,
                moved_y + 20 + expected_height,
            ])
        {
            return Err("Moved rolled fixture exposed client area".into());
        }
        log("PASS moved rolled fixture re-collapsed after external height growth");
        if !manager.restore_all() {
            return Err("Moved fixture did not unroll after height correction".into());
        }
        unsafe {
            SetWindowPos(
                hwnd,
                null_mut(),
                initial.left,
                initial.top,
                800,
                600,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOSENDCHANGING,
            );
        }
        log("PASS moved rolled fixture expanded at its new position");
        log("PASS synthetic DPI refresh corrected rolled caption height");
        let second_initial = rect(second_hwnd).ok_or("Second fixture rectangle unavailable")?;
        let second_dpi = unsafe { GetDpiForWindow(second_hwnd) };
        let second_point = POINT {
            x: second_initial.left + 100,
            y: second_initial.top
                + unsafe {
                    GetSystemMetricsForDpi(SM_CYFRAME, second_dpi)
                        + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, second_dpi)
                        + 10
                },
        };
        manager.toggle(
            manager
                .probe(point)
                .ok_or("First fixture caption unavailable")?,
        );
        manager.toggle(
            manager
                .probe(second_point)
                .ok_or("Second fixture caption unavailable")?,
        );
        unsafe {
            SendMessageW(hwnd, WM_KEYDOWN, VK_F8 as usize, 0);
        }
        if FIXTURE_REFUSES_EXPANSION.load(Ordering::Relaxed) != key {
            return Err("Fixture F8 did not enable expansion refusal".into());
        }
        if manager.control(UNROLL_ALL)
            || manager.windows.len() != 1
            || !manager.windows.contains_key(&key)
            || rect(second_hwnd).map(bounds) != Some(bounds(second_initial))
            || STOPPING.load(Ordering::Relaxed)
        {
            return Err("Mixed Unroll all outcome lost recovery or failed to restore peer".into());
        }
        let affected = RECOVERY_WINDOWS.lock().unwrap().clone();
        if affected.len() != 1 || !affected[0].contains("WinRoll RS native test fixture") {
            return Err("Recovery flow did not identify the failed window".into());
        }
        unsafe {
            SendMessageW(hwnd, WM_KEYDOWN, VK_F8 as usize, 0);
        }
        if FIXTURE_REFUSES_EXPANSION.load(Ordering::Relaxed) != 0 {
            return Err("Fixture F8 did not allow expansion retry".into());
        }
        if manager.control(UNROLL_ALL)
            || rect(hwnd).map(bounds) != Some(bounds(initial))
            || !RECOVERY_WINDOWS.lock().unwrap().is_empty()
        {
            return Err("Unroll all retry did not clear recovery".into());
        }
        let painted = unsafe {
            let dc = GetDC(hwnd);
            let painted = SendMessageW(hwnd, WM_ERASEBKGND, dc as usize, 0);
            ReleaseDC(hwnd, dc);
            painted
        };
        if painted == 0 {
            return Err("Fixture did not paint its client background after retry".into());
        }
        manager.control(ENABLE);
        log("PASS mixed native Unroll all outcome and identified recovery retry");
        manager.toggle(manager.probe(point).ok_or("Fixture caption unavailable")?);
        FIXTURE_REFUSES_EXPANSION.store(key, Ordering::Relaxed);
        if manager.restore_all() {
            return Err("Refused restoration incorrectly discarded recovery state".into());
        }
        FIXTURE_REFUSES_EXPANSION.store(0, Ordering::Relaxed);
        if !manager.restore_all() || rect(hwnd).map(bounds) != Some(bounds(initial)) {
            return Err("Restoration retry did not recover saved geometry".into());
        }
        log("PASS native restoration refusal retained state; retry restored original geometry");
        manager.toggle(manager.probe(point).ok_or("Fixture caption unavailable")?);
        manager.toggle(
            manager
                .probe(second_point)
                .ok_or("Second fixture caption unavailable")?,
        );
        let rolled = rect(hwnd).ok_or("Fixture closed")?;
        let second_rolled = rect(second_hwnd).ok_or("Second fixture closed")?;
        if manager.control(PAUSE)
            || !STOPPING.load(Ordering::Relaxed)
            || manager.windows.len() != 2
            || rect(hwnd).map(bounds) != Some(bounds(rolled))
            || rect(second_hwnd).map(bounds) != Some(bounds(second_rolled))
        {
            return Err("Pause did not preserve rolled windows and disable gestures".into());
        }
        manager.move_rolled(DragPosition {
            key,
            x: rolled.left + 60,
            y: rolled.top + 40,
            finished: true,
        });
        if rect(hwnd).map(|r| (r.left, r.top, dimensions(r)))
            != Some((rolled.left + 60, rolled.top + 40, dimensions(rolled)))
        {
            return Err("Paused drag exposed window content or failed to move".into());
        }
        manager.move_rolled(DragPosition {
            key,
            x: rolled.left,
            y: rolled.top,
            finished: true,
        });
        manager.control(ENABLE);
        if STOPPING.load(Ordering::Relaxed) || manager.windows.len() != 2 {
            return Err("Enable did not resume gestures with rolled windows".into());
        }
        if manager.control(UNROLL_ALL)
            || STOPPING.load(Ordering::Relaxed)
            || rect(hwnd).map(bounds) != Some(bounds(initial))
            || rect(second_hwnd).map(bounds) != Some(bounds(second_initial))
        {
            return Err("Unroll all did not restore while remaining enabled".into());
        }
        manager.toggle(manager.probe(point).ok_or("Fixture caption unavailable")?);
        FIXTURE_REFUSES_EXPANSION.store(key, Ordering::Relaxed);
        if manager.control(EXIT) || !STOPPING.load(Ordering::Relaxed) {
            return Err("Failed Exit restoration permitted exit or gestures".into());
        }
        manager.control(ENABLE);
        if !STOPPING.load(Ordering::Relaxed) {
            return Err("Enable discarded pending recovery".into());
        }
        FIXTURE_REFUSES_EXPANSION.store(0, Ordering::Relaxed);
        if !manager.control(UNROLL_ALL) || rect(hwnd).map(bounds) != Some(bounds(initial)) {
            return Err("Exit retry did not restore before permitting shutdown".into());
        }
        manager.exit_pending = false; // The self-test continues after verifying Exit completion.
        manager.control(ENABLE);
        log("PASS Pause, Enable, Unroll all and failed Exit/retry lifecycle geometry");
        manager.toggle(manager.probe(point).ok_or("Fixture caption unavailable")?);
        let rolled = rect(hwnd).ok_or("Fixture closed")?;
        unsafe {
            RemovePropW(hwnd, manager.property.as_ptr());
        }
        if !manager.restore_all() || rect(hwnd).map(bounds) != Some(bounds(rolled)) {
            return Err("Stale identity changed an unowned window".into());
        }
        // The test owns this fixture; reset it after deliberately invalidating WinRoll's marker.
        unsafe {
            SetWindowPos(
                hwnd,
                null_mut(),
                initial.left,
                initial.top,
                800,
                600,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOSENDCHANGING,
            );
        }
        log("PASS stale identity discarded without resizing the unowned window");
        unsafe {
            PostMessageW(hwnd, WM_SYSCOMMAND, SC_MAXIMIZE as usize, 0);
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while unsafe { IsZoomed(hwnd) } == 0 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        if unsafe { IsZoomed(hwnd) } == 0 || ordinary(hwnd) {
            return Err("Maximized fixture not excluded".into());
        }
        let maximized = rect(hwnd).ok_or("Maximized fixture unavailable")?;
        let maximized_close = POINT {
            x: maximized.right - 25,
            y: maximized.top + caption_height(unsafe { GetDpiForWindow(hwnd) }) / 2,
        };
        self_test_topmost(&mut manager, maximized_close, false)?;
        self_test_topmost(&mut manager, maximized_close, true)?;
        self_test_send_to_back(&manager, maximized_close, second_hwnd, true)?;
        unsafe {
            PostMessageW(hwnd, WM_SYSCOMMAND, SC_RESTORE as usize, 0);
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while !ordinary(hwnd) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        manager.toggle(
            manager
                .probe(point)
                .ok_or("Restored fixture caption unavailable")?,
        );
        if manager.windows.is_empty() {
            return Err("Fixture failed to roll before close check".into());
        }
        unsafe {
            PostMessageW(hwnd, WM_CLOSE, 0, 0);
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while unsafe { IsWindow(hwnd) } != 0 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        if unsafe { IsWindow(hwnd) } != 0 || !manager.restore_all() {
            return Err("Closed rolled fixture was not cleaned up".into());
        }
        log("PASS closed rolled window cleanup");
        manager.toggle(
            manager
                .probe(second_point)
                .ok_or("Second fixture caption unavailable for close check")?,
        );
        FIXTURE_REFUSES_EXPANSION.store(second_key, Ordering::Relaxed);
        if manager.control(EXIT) || !RECOVERY_PENDING.load(Ordering::Relaxed) {
            return Err("Failed Exit did not retain closed-window recovery".into());
        }
        unsafe {
            PostMessageW(second_hwnd, WM_CLOSE, 0, 0);
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while unsafe { IsWindow(second_hwnd) } != 0 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        if unsafe { IsWindow(second_hwnd) } != 0
            || !manager.control(UNROLL_ALL)
            || RECOVERY_PENDING.load(Ordering::Relaxed)
        {
            return Err("Closed pending window did not permit Exit to finish".into());
        }
        log("PASS closed pending recovery permits Exit completion");
        log(
            "PASS maximized-window exclusion; client-area preservation; 20 native geometry cycles. This does not prove browser compatibility or physical mouse-hook behavior.",
        );
        Ok(())
    })();
    FIXTURE_REFUSES_EXPANSION.store(0, Ordering::Relaxed);
    manager.restore_all();
    unsafe {
        PostMessageW(hwnd, WM_CLOSE, 0, 0);
        PostMessageW(second_hwnd, WM_CLOSE, 0, 0);
    }
    fixture.join().map_err(|_| "Fixture thread panicked")?;
    second_fixture
        .join()
        .map_err(|_| "Second fixture thread panicked")?;
    result?;
    tray_windows::self_test()?;
    self_test_options()?;
    self_test_menu_exit()
}

fn self_test_transparency(point: POINT) -> Result<(), String> {
    let mut manager = Manager::new()?;
    let target = manager
        .probe(point)
        .ok_or("Transparency fixture caption not found")?;
    let hwnd = target.hwnd as HWND;
    let original_percent = TRANSPARENCY.load(Ordering::Relaxed);
    let result = (|| -> Result<(), String> {
        for percent in (0..=100).step_by(10) {
            TRANSPARENCY.store(percent, Ordering::Relaxed);
            manager.toggle_transparency(target);
            unsafe {
                let mut alpha = 255;
                let mut flags = 0;
                if percent > 0
                    && (GetLayeredWindowAttributes(hwnd, null_mut(), &mut alpha, &mut flags) == 0
                        || alpha != transparency_alpha(percent)
                        || flags & LWA_ALPHA == 0)
                {
                    return Err(format!("Transparency {percent}% was not applied"));
                }
            }
            if percent > 0 {
                manager.toggle_transparency(target);
            }
            if !manager.transparent.is_empty()
                || unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32 & WS_EX_LAYERED != 0
            {
                return Err("Transparency toggle did not restore non-layered style".into());
            }
        }
        // Preserve a pre-existing alpha and color key, including on Exit.
        unsafe {
            let style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            SetWindowLongW(hwnd, GWL_EXSTYLE, (style | WS_EX_LAYERED) as i32);
            if SetLayeredWindowAttributes(hwnd, 0x123456, 201, LWA_ALPHA | LWA_COLORKEY) == 0 {
                return Err("Cannot configure original layered fixture".into());
            }
        }
        TRANSPARENCY.store(100, Ordering::Relaxed);
        manager.toggle_transparency(target);
        manager.control(PAUSE);
        if manager.transparent.is_empty() {
            return Err("Pause discarded transparency".into());
        }
        if !manager.control(EXIT) {
            return Err("Exit did not restore transparency".into());
        }
        unsafe {
            let mut color = 0;
            let mut alpha = 0;
            let mut flags = 0;
            if GetLayeredWindowAttributes(hwnd, &mut color, &mut alpha, &mut flags) == 0
                || color != 0x123456
                || alpha != 201
                || flags != LWA_ALPHA | LWA_COLORKEY
            {
                return Err("Original layered attributes were not restored".into());
            }
        }
        Ok(())
    })();
    manager.restore_all_transparency();
    unsafe {
        let style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongW(hwnd, GWL_EXSTYLE, (style & !WS_EX_LAYERED) as i32);
    }
    TRANSPARENCY.store(original_percent, Ordering::Relaxed);
    STOPPING.store(false, Ordering::Relaxed);
    result?;
    log("PASS transparency 0–100%, toggle, Pause and original layered attributes on Exit");
    Ok(())
}

fn self_test_topmost_exit(point: POINT) -> Result<(), String> {
    let mut manager = Manager::new()?;
    let target = manager
        .probe_hit(point, HTCLOSE)
        .ok_or("Fixture Close unavailable")?;
    let hwnd = target.hwnd as HWND;
    for original in [false, true] {
        unsafe {
            SetWindowPos(
                hwnd,
                if original {
                    HWND_TOPMOST
                } else {
                    HWND_NOTOPMOST
                },
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
        self_test_topmost(&mut manager, point, !original)?;
        manager.control(PAUSE);
        manager.control(UNROLL_ALL);
        if (unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST != 0) == original {
            return Err("Pause or Unroll all changed Always on Top".into());
        }
        manager.control(ENABLE);
        self_test_topmost(&mut manager, point, original)?;
        self_test_topmost(&mut manager, point, !original)?;
        if !manager.control(EXIT) {
            return Err("Exit did not complete topmost restoration".into());
        }
        if (unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST != 0) != original {
            return Err("Exit left the window's Always on Top state changed".into());
        }
        manager.exit_pending = false;
        manager.control(ENABLE);
    }
    log("PASS Exit restores original Always on Top state");
    Ok(())
}

fn self_test_topmost(manager: &mut Manager, point: POINT, expected: bool) -> Result<(), String> {
    let target = manager
        .probe_hit(point, HTCLOSE)
        .ok_or("Fixture Close button was not eligible")?;
    manager.toggle_topmost(target);
    let hwnd = target.hwnd as HWND;
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let topmost = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST != 0;
        if topmost == expected {
            break;
        }
        if Instant::now() >= deadline {
            return Err("Always on top did not toggle the native flag".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    if rect(hwnd).map(bounds) != Some(bounds(target.rect)) {
        return Err("Always on top changed window geometry".into());
    }
    log(format!(
        "PASS Close-button Always on top={expected} with unchanged geometry"
    ));
    Ok(())
}

fn self_test_send_to_back(
    manager: &Manager,
    point: POINT,
    other: HWND,
    topmost: bool,
) -> Result<(), String> {
    let target = manager
        .probe_hit(point, HTCLOSE)
        .ok_or("Close unavailable")?;
    let hwnd = target.hwnd as HWND;
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_ASYNCWINDOWPOS;
    let behind_other = || {
        let mut next = unsafe { GetWindow(other, GW_HWNDNEXT) };
        while !next.is_null() && next != hwnd {
            next = unsafe { GetWindow(next, GW_HWNDNEXT) };
        }
        next == hwnd
    };
    unsafe {
        SetWindowPos(other, HWND_NOTOPMOST, 0, 0, 0, 0, flags);
        SetWindowPos(
            hwnd,
            if topmost {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            },
            0,
            0,
            0,
            0,
            flags,
        );
        SetWindowPos(hwnd, HWND_TOP, 0, 0, 0, 0, flags);
    }
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let ready = unsafe {
            (GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST != 0) == topmost
                && GetWindowLongW(other, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST == 0
                && GetAncestor(WindowFromPoint(point), GA_ROOT) == hwnd
        };
        if ready && !behind_other() {
            break;
        }
        if Instant::now() >= deadline {
            return Err("Send to Back fixture setup did not settle".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    let foreground = unsafe { GetForegroundWindow() };
    manager.send_to_back(target);
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if behind_other()
            && unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST == 0
        {
            break;
        }
        if Instant::now() >= deadline {
            return Err("Send to Back did not lower the fixture or clear Always on Top".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    if rect(hwnd).map(bounds) != Some(bounds(target.rect))
        || !visible_caption(hwnd)
        || unsafe { GetForegroundWindow() } != foreground
    {
        return Err("Send to Back changed geometry, visibility or activation".into());
    }
    // Restore fixture order so subsequent pointer probes still reach its caption.
    unsafe {
        SetWindowPos(other, HWND_TOPMOST, 0, 0, 0, 0, flags);
        SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, flags);
    }
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let ready = unsafe {
            GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST != 0
                && GetWindowLongW(other, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST != 0
                && GetAncestor(WindowFromPoint(point), GA_ROOT) == hwnd
        };
        if ready && !behind_other() {
            break;
        }
        if Instant::now() >= deadline {
            return Err("Send to Back fixture cleanup did not settle".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    log(format!(
        "PASS Close-button Send to Back (initial topmost={topmost}) with unchanged geometry and activation"
    ));
    Ok(())
}

fn self_test_options() -> Result<(), String> {
    let enabled = crate::startup::enabled().map_err(|error| error.to_string())?;
    // Read the real preference, but never click the checkbox or change startup in this UI check.
    unsafe {
        show_auxiliary(null_mut(), true);
        let hwnd = OPTIONS_WINDOW.load(Ordering::Relaxed) as HWND;
        if hwnd.is_null() {
            return Err("Cannot create Options window".into());
        }
        let result = (|| -> Result<(), String> {
            let checkbox = GetDlgItem(hwnd, 3);
            let close = GetDlgItem(hwnd, 2);
            let slider = GetDlgItem(hwnd, 4);
            if slider.is_null()
                || SendMessageW(slider, TBM_GETRANGEMIN, 0, 0) != 0
                || SendMessageW(slider, TBM_GETRANGEMAX, 0, 0) != 10
                || SendMessageW(slider, TBM_GETPOS, 0, 0)
                    != (TRANSPARENCY.load(Ordering::Relaxed) / 10) as isize
            {
                return Err("Options transparency slider has incorrect range or preference".into());
            }
            if checkbox.is_null()
                || close.is_null()
                || windows_sys::Win32::UI::Input::KeyboardAndMouse::GetFocus() != checkbox
                || (SendMessageW(checkbox, BM_GETCHECK, 0, 0) == BST_CHECKED as isize) != enabled
            {
                return Err("Options does not reflect the saved startup preference".into());
            }
            show_auxiliary(null_mut(), true);
            if OPTIONS_WINDOW.load(Ordering::Relaxed) != hwnd as isize {
                return Err("Options menu item opened a duplicate Options window".into());
            }
            SetFocus(checkbox);
            let tab = MSG {
                hwnd: checkbox,
                message: WM_KEYDOWN,
                wParam: 9, // VK_TAB
                ..Default::default()
            };
            if IsDialogMessageW(hwnd, &tab) == 0
                || windows_sys::Win32::UI::Input::KeyboardAndMouse::GetFocus() != slider
            {
                return Err("Options keyboard navigation did not focus transparency".into());
            }
            let tab = MSG {
                hwnd: slider,
                ..tab
            };
            if IsDialogMessageW(hwnd, &tab) == 0
                || windows_sys::Win32::UI::Input::KeyboardAndMouse::GetFocus() != close
            {
                return Err("Options keyboard navigation did not focus Close".into());
            }
            SendMessageW(hwnd, WM_COMMAND, 2, close as isize);
            if OPTIONS_WINDOW.load(Ordering::Relaxed) != 0 || IsWindow(hwnd) != 0 {
                return Err("Close did not destroy the Options window".into());
            }
            Ok(())
        })();
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        result?;
    }
    log("PASS Options startup preference, window reuse, keyboard navigation and Close");
    Ok(())
}

fn self_test_menu_exit() -> Result<(), String> {
    let mut manager = Manager::new()?;
    STOPPING.store(false, Ordering::Relaxed);
    CONTROL_REQUESTS.store(0, Ordering::Relaxed);
    for (event, paused) in [
        (NIN_SELECT, true),
        (NIN_SELECT, false),
        (NIN_KEYSELECT, true),
        (NIN_KEYSELECT, false),
    ] {
        unsafe {
            tray_proc(null_mut(), TRAY_CALLBACK, 0, ((1 << 16) | event) as isize);
        }
        let action = CONTROL_REQUESTS.swap(0, Ordering::Relaxed);
        if action != TOGGLE_ENABLED {
            return Err("Tray activation did not request a toggle".into());
        }
        manager.control(action);
        if STOPPING.load(Ordering::Relaxed) != paused {
            return Err("Tray activation did not toggle enabled state".into());
        }
    }
    for _ in 0..2 {
        unsafe {
            tray_proc(null_mut(), TRAY_CALLBACK, 0, NIN_SELECT as isize);
        }
    }
    if CONTROL_REQUESTS.swap(0, Ordering::Relaxed) != 0 {
        return Err("Two queued tray clicks did not cancel".into());
    }
    RECOVERY_PENDING.store(true, Ordering::Relaxed);
    unsafe {
        tray_proc(null_mut(), TRAY_CALLBACK, 0, NIN_SELECT as isize);
    }
    RECOVERY_PENDING.store(false, Ordering::Relaxed);
    if CONTROL_REQUESTS.swap(0, Ordering::Relaxed) != 0 {
        return Err("Tray click bypassed pending recovery".into());
    }
    log("PASS tray left-click and keyboard toggle, rapid-click parity and recovery guard");
    // Exercise the real native modal menu and outer message loop, with no target windows at risk.
    unsafe {
        let class = WNDCLASSW {
            lpfnWndProc: Some(tray_proc),
            hInstance: GetModuleHandleW(null_mut()),
            lpszClassName: w!("WinRollTrayExitTest"),
            ..Default::default()
        };
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(
            0,
            class.lpszClassName,
            w!("WinRoll RS exit test"),
            0,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            class.hInstance,
            null_mut(),
        );
        if hwnd.is_null() {
            return Err("Cannot create tray test window".into());
        }
        let key = hwnd as isize;
        let tid = GetCurrentThreadId();
        let completion = thread::spawn(move || {
            let mut info = GUITHREADINFO {
                cbSize: size_of::<GUITHREADINFO>() as u32,
                ..Default::default()
            };
            let deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < deadline {
                GetGUIThreadInfo(tid, &mut info);
                if info.flags & GUI_INMENUMODE != 0 {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
            let finished = Instant::now();
            WORKER_FINISHED.store(true, Ordering::Relaxed);
            PostMessageW(key as HWND, WORKER_DONE, 0, 0);
            // Bound a regression: cancel the menu if completion was swallowed by its modal loop.
            thread::sleep(Duration::from_secs(1));
            PostMessageW(key as HWND, WM_CANCELMODE, 0, 0);
            PostMessageW(key as HWND, WM_NULL, 0, 0);
            (info.flags & GUI_INMENUMODE != 0, finished)
        });
        PostMessageW(
            hwnd,
            TRAY_CALLBACK,
            (100 << 16) | 100,
            WM_CONTEXTMENU as isize,
        );
        pump_until_restored();
        let returned = Instant::now();
        let (menu_opened, finished) = completion.join().map_err(|_| "Menu test thread panicked")?;
        DestroyWindow(hwnd);
        WORKER_FINISHED.store(false, Ordering::Relaxed);
        if !menu_opened || returned.duration_since(finished) >= Duration::from_millis(900) {
            return Err(
                "Completed Exit did not dismiss the open tray menu and finish its message loop"
                    .into(),
            );
        }
    }
    log("PASS completed Exit dismisses an open tray menu and finishes the message loop");
    Ok(())
}

fn self_test_dpi_move(manager: &mut Manager, key: isize, point: POINT) -> Result<(), String> {
    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        data: LPARAM,
    ) -> i32 {
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let (mut dpi, mut y) = (0, 0);
        // SAFETY: enumeration is synchronous; data points to the caller's live vector.
        unsafe {
            if GetMonitorInfoW(monitor, &mut info) != 0
                && GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut y) >= 0
            {
                (*(data as *mut Vec<(RECT, u32)>)).push((info.rcWork, dpi));
            }
        }
        1
    }
    let hwnd = key as HWND;
    let initial = rect(hwnd).ok_or("DPI fixture rectangle unavailable")?;
    let initial_dpi = unsafe { GetDpiForWindow(hwnd) };
    let mut monitors: Vec<(RECT, u32)> = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            null_mut(),
            std::ptr::null(),
            Some(collect),
            &mut monitors as *mut _ as LPARAM,
        );
    }
    let Some((destination, destination_dpi)) =
        monitors.into_iter().find(|(_, dpi)| *dpi != initial_dpi)
    else {
        log("UNVERIFIED native mixed-DPI move: no differently scaled monitor available");
        return Ok(());
    };
    manager.toggle(
        manager
            .probe(point)
            .ok_or("DPI fixture caption unavailable")?,
    );
    FIXTURE_MIN_TRACK.store(true, Ordering::Relaxed);
    for (x, y, dpi) in [
        (destination.left + 40, destination.top + 80, destination_dpi),
        (initial.left, initial.top, initial_dpi),
    ] {
        manager.move_rolled(DragPosition {
            key,
            x,
            y,
            finished: false,
        });
        let deadline = Instant::now() + Duration::from_secs(1);
        while unsafe { GetDpiForWindow(hwnd) } != dpi && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        if unsafe { GetDpiForWindow(hwnd) } != dpi {
            return Err(format!(
                "Native fixture did not reach destination DPI {dpi}"
            ));
        }
        // Let the target apply its DPI rectangle and minimum size, as real applications do.
        thread::sleep(Duration::from_millis(30));
        manager.move_rolled(DragPosition {
            key,
            x,
            y,
            finished: false,
        });
        thread::sleep(Duration::from_millis(50));
        let current = rect(hwnd).ok_or("DPI fixture closed")?;
        log(format!(
            "DPI MOVE dpi={dpi} rect={:?} expected_height={}",
            bounds(current),
            rolled_height(dpi)
        ));
        if dimensions(current) != (dimensions(initial).0, rolled_height(dpi)) {
            return Err(
                "Unfinished mixed-DPI drag changed saved width or exposed fixture client area"
                    .into(),
            );
        }
    }
    // Hold a fixed drag position around a horizontal monitor boundary. A target's
    // suggested DPI rectangle must not cause the monitor choice to oscillate.
    if destination.left > initial.left {
        for offset in [-120, -80, -40, 0, 40, 80, 120] {
            let x = destination.left - dimensions(initial).0 / 2 + offset;
            let mut previous_dpi = unsafe { GetDpiForWindow(hwnd) };
            let mut transitions = 0;
            DRAG_ACTIVE.store(true, Ordering::Relaxed);
            for _ in 0..40 {
                manager.move_rolled(DragPosition {
                    key,
                    x,
                    y: initial.top,
                    finished: false,
                });
                manager.maintain_rolled_size();
                thread::sleep(Duration::from_millis(10));
                let dpi = unsafe { GetDpiForWindow(hwnd) };
                if dpi != previous_dpi {
                    transitions += 1;
                    previous_dpi = dpi;
                    log(format!(
                        "DPI BOUNDARY x={x} dpi={dpi} rect={:?}",
                        rect(hwnd).map(bounds)
                    ));
                }
            }
            DRAG_ACTIVE.store(false, Ordering::Relaxed);
            if transitions > 2 {
                return Err(format!(
                    "Fixed rolled position oscillated between DPIs {transitions} times"
                ));
            }
            if rect(hwnd).map(dimensions)
                != Some((dimensions(initial).0, rolled_height(previous_dpi)))
            {
                return Err(
                    "Held boundary position did not settle at saved width and caption height"
                        .into(),
                );
            }
        }
        manager.move_rolled(DragPosition {
            key,
            x: initial.left,
            y: initial.top,
            finished: true,
        });
    }
    FIXTURE_MIN_TRACK.store(false, Ordering::Relaxed);
    if !manager.restore_all() || rect(hwnd).map(bounds) != Some(bounds(initial)) {
        return Err("Mixed-DPI fixture did not recover its original expanded rectangle".into());
    }
    log("PASS native mixed-DPI moves in both directions before release");
    Ok(())
}

unsafe extern "system" fn fixture_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: invoked by Windows only for this process-owned fixture.
    unsafe {
        if message == WM_DPICHANGED {
            let suggested = *(l as *const RECT);
            SetWindowPos(
                hwnd,
                null_mut(),
                suggested.left,
                suggested.top,
                suggested.right - suggested.left,
                suggested.bottom - suggested.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            return 0;
        }
        if message == WM_GETMINMAXINFO && FIXTURE_MIN_TRACK.load(Ordering::Relaxed) {
            (*(l as *mut MINMAXINFO)).ptMinTrackSize.y = 243;
            return 0;
        }
        if message == WM_KEYDOWN && w == VK_F8 as usize {
            let refusing = FIXTURE_REFUSES_EXPANSION.load(Ordering::Relaxed) != hwnd as isize;
            FIXTURE_REFUSES_EXPANSION
                .store(if refusing { hwnd as isize } else { 0 }, Ordering::Relaxed);
            SetWindowTextW(
                hwnd,
                if refusing {
                    w!("WinRoll RS native test fixture - refusing expansion (F8: allow)")
                } else {
                    w!("WinRoll RS native test fixture - expansion allowed (F8: refuse)")
                },
            );
            return 0;
        }
        if message == WM_WINDOWPOSCHANGING {
            let position = &mut *(l as *mut WINDOWPOS);
            if position.flags & SWP_HIDEWINDOW != 0
                && FIXTURE_DELAYS_HIDE
                    .compare_exchange(hwnd as isize, 0, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
            {
                thread::sleep(Duration::from_millis(600));
            }
            if FIXTURE_REFUSES_SHOW.load(Ordering::Relaxed) == hwnd as isize {
                position.flags &= !SWP_SHOWWINDOW;
            }
        }
        if message == WM_WINDOWPOSCHANGED
            && FIXTURE_REFUSES_EXPANSION.load(Ordering::Relaxed) == hwnd as isize
        {
            let position = &*(l as *const WINDOWPOS);
            if position.cy > 100 && position.flags & SWP_NOSIZE == 0 {
                // Simulate an application enforcing its own size after an external resize request.
                SetWindowPos(
                    hwnd,
                    null_mut(),
                    0,
                    0,
                    position.cx,
                    39,
                    SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOSENDCHANGING,
                );
                return 0;
            }
        }
        if message == WM_DESTROY {
            PostQuitMessage(0);
            return 0;
        }
        if message == WM_NCHITTEST {
            if FIXTURE_CAPTION_EVERYWHERE.load(Ordering::Relaxed) {
                return HTCAPTION as isize;
            }
            // A custom frame may report caption behind a child control. Its result must not win.
            if let Some(child) = rect(GetDlgItem(hwnd, 42)) {
                let (x, y) = (l as i16 as i32, (l >> 16) as i16 as i32);
                if x >= child.left && x < child.right && y >= child.top && y < child.bottom {
                    return HTCAPTION as isize;
                }
            }
        }
        DefWindowProcW(hwnd, message, w, l)
    }
}
