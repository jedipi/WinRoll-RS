use super::*;

pub(super) struct Hidden {
    pub target: Target,
    marker: usize,
    icon: isize,
    as_menu: bool,
    operation: Option<thread::JoinHandle<()>>,
    restoring: bool,
}

impl Manager {
    fn owns_hidden(&self, saved: &Hidden) -> bool {
        same_target(saved.target)
            && unsafe {
                GetPropW(saved.target.hwnd as HWND, self.hidden_property.as_ptr()) as usize
                    == saved.marker
            }
    }

    fn hidden_icon(&self, id: u32, operation: u32) -> bool {
        let saved = &self.hidden[&id];
        if saved.as_menu {
            return true;
        }
        let mut icon = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.tray_owner as HWND,
            uID: id,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WINDOW_TRAY_CALLBACK,
            hIcon: saved.icon as HICON,
            ..Default::default()
        };
        unsafe {
            GetWindowTextW(saved.target.hwnd as HWND, icon.szTip.as_mut_ptr(), 128);
            Shell_NotifyIconW(operation, &icon) != 0
        }
    }

    fn forget_hidden(&mut self, id: u32) {
        self.hidden_icon(id, NIM_DELETE);
        if let Some(saved) = self.hidden.remove(&id) {
            unsafe {
                if self.owns_hidden(&saved) {
                    RemovePropW(saved.target.hwnd as HWND, self.hidden_property.as_ptr());
                }
                if !saved.as_menu {
                    DestroyIcon(saved.icon as HICON);
                }
            }
        }
        self.publish_hidden_menu();
    }

    pub(super) fn publish_hidden_menu(&self) {
        let mut minimized = self
            .hidden
            .iter()
            .filter(|(_, saved)| saved.as_menu && self.owns_hidden(saved))
            .map(|(id, saved)| {
                let mut title = [0u16; 512];
                let length = unsafe {
                    GetWindowTextW(
                        saved.target.hwnd as HWND,
                        title.as_mut_ptr(),
                        title.len() as i32,
                    )
                };
                // Keep titles untranslated; the UI supplies its current-language fallback.
                let title = String::from_utf16_lossy(&title[..length as usize]);
                (*id, title)
            })
            .collect::<Vec<_>>();
        minimized.sort_by_key(|(id, _)| *id);
        *MINIMIZED_WINDOWS.lock().unwrap() = minimized;
    }

    pub(super) fn minimize_to_tray(&mut self, target: Target, as_menu: bool) {
        let hwnd = target.hwnd as HWND;
        if self.tray_owner == 0
            || !same_target(target)
            || !visible_caption(hwnd)
            || rect(hwnd).map(bounds) != Some(bounds(target.rect))
            || integrity(target.pid) != Some(self.integrity)
            || unsafe { GetWindowLongW(hwnd, GWL_STYLE) as u32 & WS_MINIMIZEBOX == 0 }
            || self
                .hidden
                .values()
                .any(|saved| saved.target.hwnd == target.hwnd)
        {
            return;
        }
        // ID 1 belongs to WinRoll. Never recycle IDs: delayed shell callbacks must be harmless.
        let Some(id) = self
            .next_marker
            .checked_add(1)
            .and_then(|id| u32::try_from(id).ok())
        else {
            return;
        };
        self.next_marker += 2;
        unsafe {
            let icon = if as_menu {
                null_mut()
            } else {
                let mut source = 0;
                SendMessageTimeoutW(
                    hwnd,
                    WM_GETICON,
                    ICON_SMALL2 as usize,
                    0,
                    SMTO_ABORTIFHUNG | SMTO_BLOCK,
                    20,
                    &mut source,
                );
                if source == 0 {
                    source = GetClassLongPtrW(hwnd, GCLP_HICONSM);
                }
                if source == 0 {
                    source = GetClassLongPtrW(hwnd, GCLP_HICON);
                }
                if source == 0 {
                    source = LoadIconW(null_mut(), IDI_APPLICATION) as usize;
                }
                let icon = CopyIcon(source as HICON);
                if icon.is_null() {
                    return;
                }
                icon
            };
            if !same_target(target)
                || SetPropW(hwnd, self.hidden_property.as_ptr(), id as HANDLE) == 0
            {
                if !icon.is_null() {
                    DestroyIcon(icon);
                }
                return;
            }
            self.hidden.insert(
                id,
                Hidden {
                    target,
                    marker: id as usize,
                    icon: icon as isize,
                    as_menu,
                    operation: None,
                    restoring: false,
                },
            );
            // Install the recovery path before hiding the target.
            self.publish_hidden_menu();
            if !self.hidden_icon(id, NIM_ADD) {
                self.forget_hidden(id);
                return;
            }
        }
        if !self.start_hidden_operation(id, false) {
            self.forget_hidden(id);
            return;
        }
        self.wait_hidden_operation(id);
    }

    fn start_hidden_operation(&mut self, id: u32, show: bool) -> bool {
        let saved = &self.hidden[&id];
        let target = saved.target;
        let marker = saved.marker;
        let property = self.hidden_property.clone();
        // A synchronous call on its own thread gives an actual completion signal.
        // Never queue a show until the preceding hide has finished, even if it was refused.
        let operation = thread::Builder::new().spawn(move || unsafe {
            let hwnd = target.hwnd as HWND;
            if same_target(target) && GetPropW(hwnd, property.as_ptr()) as usize == marker {
                SetWindowPos(
                    hwnd,
                    null_mut(),
                    0,
                    0,
                    0,
                    0,
                    (if show { SWP_SHOWWINDOW } else { SWP_HIDEWINDOW })
                        | SWP_NOMOVE
                        | SWP_NOSIZE
                        | SWP_NOZORDER
                        | SWP_NOACTIVATE
                        | SWP_NOOWNERZORDER,
                );
            }
        });
        let Ok(operation) = operation else {
            return false;
        };
        let saved = self.hidden.get_mut(&id).unwrap();
        saved.operation = Some(operation);
        saved.restoring = show;
        true
    }

    fn wait_hidden_operation(&mut self, id: u32) -> bool {
        let deadline = Instant::now() + Duration::from_millis(350);
        loop {
            let Some(saved) = self.hidden.get(&id) else {
                return false;
            };
            if !self.owns_hidden(saved) {
                self.forget_hidden(id);
                return false;
            }
            if saved
                .operation
                .as_ref()
                .is_none_or(|operation| operation.is_finished())
            {
                if let Some(operation) = self.hidden.get_mut(&id).unwrap().operation.take() {
                    let _ = operation.join();
                }
                return true;
            }
            if Instant::now() >= deadline {
                // Keep the operation and recovery entry until this target responds.
                return false;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub(super) fn restore_hidden(&mut self, id: u32, activate: bool) {
        if !self.wait_hidden_operation(id) {
            return;
        }
        // Showing without SW_RESTORE preserves maximized, snapped and rolled geometry.
        if !self.hidden[&id].restoring
            && (!self.start_hidden_operation(id, true) || !self.wait_hidden_operation(id))
        {
            return;
        }
        let saved = &self.hidden[&id];
        let hwnd = saved.target.hwnd as HWND;
        if !self.owns_hidden(saved) || unsafe { IsWindowVisible(hwnd) } != 0 {
            if activate && self.owns_hidden(saved) {
                unsafe { SetForegroundWindow(hwnd) };
            }
            self.forget_hidden(id);
        } else {
            // A completed show was refused; allow the next recovery attempt to retry.
            self.hidden.get_mut(&id).unwrap().restoring = false;
        }
    }

    pub(super) fn restore_all_hidden(&mut self) {
        for id in self.hidden.keys().copied().collect::<Vec<_>>() {
            self.restore_hidden(id, false);
        }
    }

    pub(super) fn clean_hidden(&mut self) -> bool {
        let stale: Vec<_> = self
            .hidden
            .iter()
            .filter(|(_, saved)| !self.owns_hidden(saved))
            .map(|(id, _)| *id)
            .collect();
        let changed = !stale.is_empty();
        for id in stale {
            self.forget_hidden(id);
        }
        changed
    }

    pub(super) fn recreate_hidden_icons(&mut self) {
        self.clean_hidden();
        for id in self
            .hidden
            .iter()
            .filter(|(_, saved)| !saved.as_menu)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>()
        {
            if !self.hidden_icon(id, NIM_ADD) {
                // Explorer could not recreate the recovery icon; expose the window instead.
                self.restore_hidden(id, false);
            }
        }
    }
}

pub(super) fn self_test() -> Result<(), String> {
    let (key, fixture) = spawn_fixture(false, false)?;
    let hwnd = key as HWND;
    let owner = unsafe {
        CreateWindowExW(
            0,
            w!("STATIC"),
            w!("Tray test owner"),
            0,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            GetModuleHandleW(null_mut()),
            null_mut(),
        )
    };
    let mut manager = Manager::new()?;
    manager.tray_owner = owner as isize;
    let wait = |visible: bool| -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(1);
        while (unsafe { IsWindowVisible(hwnd) } != 0) != visible {
            if Instant::now() >= deadline {
                return Err(format!("Tray fixture visibility did not become {visible}"));
            }
            thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    };
    let result: Result<(), String> = (|| {
        if owner.is_null() {
            return Err("Cannot create tray test owner".into());
        }
        let initial = rect(hwnd).ok_or("Missing tray fixture bounds")?;
        // Find the actual native Minimize hit region at the current DPI/theme.
        let point = (initial.top..initial.top + caption_height(unsafe { GetDpiForWindow(hwnd) }))
            .step_by(4)
            .find_map(|y| {
                (initial.right - 200..initial.right)
                    .step_by(4)
                    .find_map(|x| {
                        let point = POINT { x, y };
                        manager
                            .probe_hit(point, HTMINBUTTON)
                            .map(|target| (point, target))
                    })
            })
            .ok_or("Minimize button was not recognized")?;
        if manager.probe_hits(point.0, &[HTCAPTION, HTCLOSE]).is_some() {
            return Err("Minimize button was accepted by the right-click probe".into());
        }
        let target = point.1;
        let mut stale = target;
        stale.pid = 0;
        manager.minimize_to_tray(stale, false);
        if !manager.hidden.is_empty() {
            return Err("Stale target was hidden".into());
        }
        manager.minimize_to_tray(target, false);
        wait(false)?;
        let id = *manager
            .hidden
            .keys()
            .next()
            .ok_or("Missing hidden window state")?;
        manager.control(PAUSE);
        if unsafe { IsWindowVisible(hwnd) } != 0 {
            return Err("Pause showed a hidden window".into());
        }
        manager.control(UNROLL_ALL);
        if unsafe { IsWindowVisible(hwnd) } != 0 {
            return Err("Unroll all showed a hidden window".into());
        }
        // Simulate Explorer losing the icon, then exercise the recreation path.
        manager.hidden_icon(id, NIM_DELETE);
        manager.recreate_hidden_icons();
        unsafe {
            tray_proc(
                owner,
                WINDOW_TRAY_CALLBACK,
                id as usize,
                WM_LBUTTONUP as isize,
            );
        }
        let requests = std::mem::take(&mut *RESTORE_TRAY_REQUESTS.lock().unwrap());
        if requests != [id] {
            return Err("Tray click did not queue restoration".into());
        }
        manager.restore_hidden(id, true);
        wait(true)?;
        if !manager.hidden.is_empty() || rect(hwnd).map(bounds) != Some(bounds(initial)) {
            return Err("Tray restore lost state or changed geometry".into());
        }
        if !MINIMIZED_WINDOWS.lock().unwrap().is_empty() {
            return Err("Icon-mode window was published in the Minimized menu".into());
        }
        manager.minimize_to_tray(target, true);
        wait(false)?;
        let menu_id = *manager.hidden.keys().next().ok_or("Missing menu window")?;
        if manager.hidden[&menu_id].icon != 0
            || *MINIMIZED_WINDOWS.lock().unwrap()
                != [(menu_id, "WinRoll RS native test fixture".into())]
        {
            return Err("Menu mode copied an icon or omitted the window title/ID".into());
        }
        unsafe { SetWindowTextW(hwnd, w!("Renamed & menu fixture")) };
        manager.publish_hidden_menu();
        let menu_entries = MINIMIZED_WINDOWS.lock().unwrap().clone();
        if menu_entries != [(menu_id, "Renamed & menu fixture".into())] {
            return Err("Minimized menu did not refresh the window title".into());
        }
        let menu_valid = unsafe {
            let menu = CreatePopupMenu();
            let submenu = append_minimized_menu(menu, &menu_entries);
            let mut text = [0u16; 128];
            let length = GetMenuStringW(submenu, 0, text.as_mut_ptr(), 128, MF_BYPOSITION);
            let valid = !menu.is_null()
                && !submenu.is_null()
                && GetMenuItemCount(submenu) == 1
                && GetMenuItemID(submenu, 0) == MINIMIZED_MENU_FIRST
                && String::from_utf16_lossy(&text[..length.max(0) as usize])
                    == "Renamed && menu fixture";
            DestroyMenu(menu);
            valid
        };
        if !menu_valid {
            return Err("Minimized submenu omitted the title or restore command".into());
        }
        unsafe { SetWindowTextW(hwnd, w!("")) };
        manager.publish_hidden_menu();
        if *MINIMIZED_WINDOWS.lock().unwrap() != [(menu_id, String::new())] {
            return Err("Minimized menu omitted the untitled-window fallback".into());
        }
        manager.recreate_hidden_icons();
        if manager.hidden[&menu_id].icon != 0 || unsafe { IsWindowVisible(hwnd) } != 0 {
            return Err("Icon recreation changed a menu-mode window".into());
        }
        restore_minimized_selection(MINIMIZED_MENU_FIRST, &menu_entries);
        let requests = std::mem::take(&mut *RESTORE_TRAY_REQUESTS.lock().unwrap());
        if requests != [menu_id] {
            return Err("Minimized submenu selection did not queue restoration".into());
        }
        for id in requests {
            manager.restore_hidden(id, true);
        }
        wait(true)?;
        if !manager.hidden.is_empty()
            || !MINIMIZED_WINDOWS.lock().unwrap().is_empty()
            || rect(hwnd).map(bounds) != Some(bounds(initial))
        {
            return Err("Menu restoration retained its entry or changed geometry".into());
        }
        FIXTURE_REFUSES_HIDE.store(key, Ordering::Relaxed);
        manager.minimize_to_tray(target, true);
        if unsafe { IsWindowVisible(hwnd) } == 0 || manager.hidden.is_empty() {
            return Err("Refused hide did not retain a visible recovery entry".into());
        }
        if !manager.control(EXIT) || !manager.hidden.is_empty() {
            return Err("Refused hide prevented Exit despite a visible window".into());
        }
        FIXTURE_REFUSES_HIDE.store(0, Ordering::Relaxed);
        STOPPING.store(false, Ordering::Relaxed);
        manager.control(ENABLE);
        manager.toggle(target);
        let rolled = rect(hwnd).ok_or("Missing rolled bounds")?;
        manager.minimize_to_tray(
            Target {
                rect: rolled,
                ..target
            },
            false,
        );
        wait(false)?;
        manager.restore_all_hidden();
        if rect(hwnd).map(bounds) != Some(bounds(rolled)) || manager.windows.is_empty() {
            return Err("Tray restore did not preserve rolled geometry".into());
        }
        manager.restore_all();
        unsafe {
            ShowWindowAsync(hwnd, SW_MAXIMIZE);
        }
        let deadline = Instant::now() + Duration::from_secs(1);
        while unsafe { IsZoomed(hwnd) } == 0 {
            if Instant::now() >= deadline {
                return Err("Fixture did not maximize".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
        thread::sleep(Duration::from_millis(100));
        let maximized = rect(hwnd).ok_or("Missing maximized bounds")?;
        FIXTURE_DELAYS_HIDE.store(key, Ordering::Relaxed);
        FIXTURE_REFUSES_SHOW.store(key, Ordering::Relaxed);
        manager.minimize_to_tray(
            Target {
                rect: maximized,
                ..target
            },
            true,
        );
        if manager.control(EXIT)
            || manager.hidden.is_empty()
            || !RECOVERY_PENDING.load(Ordering::Relaxed)
        {
            return Err(
                "Exit discarded recovery before delayed hide/refused show completed".into(),
            );
        }
        FIXTURE_REFUSES_SHOW.store(0, Ordering::Relaxed);
        if !manager.control(EXIT) {
            return Err("Exit did not restore hidden windows".into());
        }
        if !MINIMIZED_WINDOWS.lock().unwrap().is_empty() {
            return Err("Exit retained a minimized menu entry".into());
        }
        wait(true)?;
        if unsafe { IsZoomed(hwnd) } == 0 || rect(hwnd).map(bounds) != Some(bounds(maximized)) {
            return Err("Tray Exit changed maximized state".into());
        }
        manager.minimize_to_tray(
            Target {
                rect: maximized,
                ..target
            },
            true,
        );
        wait(false)?;
        unsafe {
            PostMessageW(hwnd, WM_CLOSE, 0, 0);
        }
        let deadline = Instant::now() + Duration::from_secs(1);
        while unsafe { IsWindow(hwnd) } != 0 {
            if Instant::now() >= deadline {
                return Err("Hidden fixture did not close".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
        if !manager.clean_hidden()
            || !manager.hidden.is_empty()
            || !MINIMIZED_WINDOWS.lock().unwrap().is_empty()
        {
            return Err("Closed hidden window retained its recovery entry".into());
        }
        Ok(())
    })();
    FIXTURE_REFUSES_HIDE.store(0, Ordering::Relaxed);
    FIXTURE_REFUSES_SHOW.store(0, Ordering::Relaxed);
    FIXTURE_DELAYS_HIDE.store(0, Ordering::Relaxed);
    manager.restore_all_hidden();
    manager.restore_all();
    unsafe {
        PostMessageW(hwnd, WM_CLOSE, 0, 0);
        DestroyWindow(owner);
    }
    let _ = fixture.join();
    STOPPING.store(false, Ordering::Relaxed);
    RECOVERY_PENDING.store(false, Ordering::Relaxed);
    result?;
    log(
        "PASS Minimize hit-test, tray icon/menu hide/restore, live menu titles, Pause, Unroll all, icon recreation, rolled/maximized geometry, refused hide, delayed hide/refused show recovery, Exit and closed-window cleanup",
    );
    Ok(())
}

#[test]
fn native_minimize_to_tray() {
    self_test().unwrap();
}
