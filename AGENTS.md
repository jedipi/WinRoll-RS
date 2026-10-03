## Agent skills

### Issue tracker
Use GitHub Issues for jedipi/WinRoll-RS. See docs/agents/issue-tracker.md.

### Triage labels
Use the five default triage labels. See docs/agents/triage-labels.md.

### Domain docs
Use the single-context layout. See docs/agents/domain.md.

## Build and run

Requires Rust's `x86_64-pc-windows-msvc` toolchain and Visual Studio C++ build tools/Windows SDK. Intended baseline: Windows 10 22H2 and Windows 11 21H2 or newer, x64.

The target configuration links the C runtime statically so the executable does not require a separate VC++ runtime installation.

```powershell
cargo build --release --locked
.\target\release\winroll.exe
```

Run normally, **not as administrator**. Right-click an empty caption area to roll/unroll a window. The window directly under the pointer must positively return `HTCAPTION`; unrecognized areas and ineligible windows keep their normal clicks. Maximized, snapped, hidden/cloaked and different-integrity windows are excluded.

Left-drag a rolled window's empty caption area to move it. WinRoll RS moves managed rolled windows directly so their contents stay collapsed during the drag. A plain left click there focuses the window; native caption double-click behavior is unavailable while rolled.

The executable runs in the notification area without a console. Open its **WinRoll RS** tray menu (check the hidden-icons arrow if necessary):

- **Pause** keeps managed windows rolled and blocks roll/unroll gestures. Rolled windows can still be dragged by their captions. **Enable** resumes roll/unroll gestures.
- **Unroll all** expands managed windows without changing the enabled/paused mode.
- **About WinRoll RS** shows the app icon, version and creator (Kin Tam).
- **Exit** stops gestures, unrolls managed windows and exits only after restoration succeeds or the windows have closed.

If an unroll fails during Unroll all or Exit, WinRoll RS stays in the tray. Open its menu to see each affected window's title, PID and handle, then choose **Retry unroll all** when it responds. A successful retry finishes a pending Exit automatically. Closing an affected window removes it from recovery and can finish a pending Exit. The tray tooltip also shows when recovery is pending.

Saved geometry is in memory. Ending the process in Task Manager, logging off and crashes are not recovery paths. Unroll first. The utility does not capture window contents. An identity property on each managed window guards against stale handles. The former console `q`/Ctrl+C controls are replaced by the tray menu.

## Checks

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
.\target\release\winroll.exe --self-test
.\target\release\winroll.exe --inventory
.\target\release\winroll.exe --fixture
```

`--self-test` creates and closes two native windows. It checks client/child-control rejection, 20 measured roll/unroll cycles, mixed unroll outcomes, identified recovery, Pause preservation, Exit retry and fixture background painting, stale identity, maximized exclusion, and closed-window cleanup. It also briefly opens the native tray menu and verifies that completed Exit dismisses it and finishes the message loop. It does not exercise the global mouse hook, notification-area activation or user selection of menu actions. `--inventory` reports connected monitor bounds, work areas and effective DPI.

Diagnostic modes attach to the invoking terminal. To wait for the GUI executable and capture its exit code in PowerShell, use:

```powershell
$p = Start-Process .\target\release\winroll.exe -ArgumentList '--self-test' -NoNewWindow -Wait -PassThru
$p.ExitCode
```

`--fixture` keeps a disposable native window open for manual tests; it does not install a hook. Exit any older WinRoll RS process through its tray, rebuild, then launch the controller and fixture from the same new executable. Press **F8** while the fixture is focused to make it refuse expansion; its title must change to **refusing expansion** before continuing. If the title does not change, refocus the fixture or check that both processes use the new build. Press **F8** again to allow expansion. To test recovery through the live tray:

1. Press F8 to refuse expansion, then right-click an empty part of the fixture caption to roll it.
2. Choose **Pause** from the WinRoll RS tray menu. The fixture should stay rolled, WinRoll RS should stay running, and right-click caption gestures should neither unroll it nor roll other windows. Drag the rolled fixture by its caption and verify its content stays hidden. Choose **Enable** to resume roll/unroll gestures.
3. Choose **Unroll all** while the fixture refuses expansion. The menu should identify the fixture and offer **Retry unroll all**. Press F8 to allow expansion, then select **Retry unroll all**; the fixture should expand.
4. Roll the fixture again, press F8 to refuse expansion, then choose **Exit**. After F8 allows expansion, a successful retry should expand the fixture and remove the WinRoll RS tray icon.

For the snapped-window check, drag the fixture title bar to the left screen edge, then right-click its caption: it should remain unchanged and show its normal system menu. Close the fixture when finished. It can also serve as an elevated test target when launched separately as administrator; keep the controller unelevated.

If Cargo is installed but missing from PATH, add `$env:USERPROFILE\.cargo\bin` to the current terminal's PATH. For a workspace-local dependency cache, set `$env:CARGO_HOME = Join-Path $PWD '.cargo-cache'` before Cargo commands.

## Personal-testing package

With the locked dependencies cached and Rust's documentation component installed, run:

```powershell
./scripts/package-personal.ps1
```

This creates `target/personal-testing/winroll-0.1.0-x64.zip` with one x64 executable, concise usage/recovery instructions, the compatibility report, license notices and SHA256 hashes. It publishes nothing. The separate `tests/native-x86.c` target is for cross-bitness validation only and is excluded from the ZIP.

## Public package and installer

With Inno Setup 6 installed, run:

```powershell
./scripts/package-public.ps1 -IsccPath 'D:/Program Files/Inno Setup 6/ISCC.exe'
```

The compiler path may be omitted when `ISCC.exe` is on PATH. This builds the portable ZIP, `winroll-0.1.0-x64-setup.exe` and release checksums under `target/public-release/`. The installer installs for the current user, adds Start menu and Windows uninstall entries, and requires Win10 22H2 or Win11 21H2+ on x64. Exit WinRoll RS through its tray before upgrading or uninstalling so managed windows can unroll first. Startup and updates remain manual.

The public packaging and installer are the user's 2026-10-01 follow-up to ticket #6. Packages include the accepted compatibility report and its remaining validation gaps. Artifacts are unsigned and are built locally; the script does not publish a GitHub release.

See [installer validation](docs/installer-validation.md) for the tested install/reinstall/uninstall behavior and `tests/installer-smoke.ps1` for the runnable check.

See the [current compatibility report](docs/compatibility-report.md) for passed, failed and unverified checks. [Placement validation](docs/placement-validation.md), [recovery validation](docs/recovery-validation.md), [tray implementation checks](docs/tray-validation.md) and [feasibility results](docs/feasibility.md) retain historical evidence, including superseded Pause behavior. Unavailable OS baselines and mixed-DPI configurations are not claimed as verified.
