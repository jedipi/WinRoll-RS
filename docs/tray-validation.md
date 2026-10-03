# Ticket #3 validation

Implementation: one `winroll.exe`, using the proven worker, mouse hook, geometry observation and per-window identity/recovery state. Native notification-area menu: Enable/Pause, Unroll all, Exit. No new dependency; the existing windows-sys dependency enables its Shell API bindings. The single-instance mutex remains compatible with the old experiment.

The GUI executable needs no console or stdin. Pause/Exit disable interception before requesting worker restoration. The process and tray remain available on failure, and Enable cannot discard pending recovery. Retry using Unroll all or Exit. Detailed affected-window reporting and the fuller recovery experience remain #4.

## Environment

2026-09-24: Windows 11 25H2 build 26200.9168, x64. Edge 152.0.4191.53, Chrome 153.0.8010.48, Explorer 10.0.26100.8117. Both monitors remain at 96 DPI: 2560x1440 at (0,0), work area (0,0)-(2560,1392); 1080x1920 at (2560,0), work area (2560,0)-(3640,1872). Desktop controller ran outside the sandbox without administrator elevation. No previous WinRoll process or elevated fixture was found before starting.

## Checks

- Native self-test: 20 roll/unroll geometry cycles; child-control/client-area rejection; successful Pause, Enable and Unroll all; failed Pause/Exit preserve state and inhibit Enable; successful Exit retry restores before allowing shutdown. These checks call the lifecycle/state boundary against a real native fixture, not the visible menu or hook.
- Stale identity: invalidating the native fixture's ownership marker discards saved state without changing its geometry. Closed rolled-window cleanup also passes.
- Native menu shutdown: Exit completion while the popup menu is open dismisses the menu and finishes the outer message loop. The regression failed when `EndMenu` was removed and passed with the fix restored. It tests the native modal loop, not notification-area activation or menu-action selection.
- Final checks passed: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`, `cargo build --release --locked`, and the release executable's `--self-test` (exit code 0).
- Real caption gestures through the installed hook: Explorer 1080x872 to 1080x39 and back at (2560,0); Edge 1080x1039 to 1080x39 and back at (2560,0); Chrome 1265x956 to 1265x39 and back at (615,288). Each restoration was measured by the worker, in addition to visible inspection.
- **User-confirmed pass:** after the reviewed build was launched for manual testing, the user reported "Tested all good" for the requested multiple-window checks: Unroll all, Pause/Enable, and Exit, including unrolling managed windows on Pause and Exit. This completes #3's remaining live tray-action acceptance check. This is user-reported evidence; no additional per-window measurements or compatibility environments were supplied.

All three desktop targets were verified unrolled before the test controller was stopped to rebuild the reviewed executable. No controller was left running. That stop is not evidence of the user-facing Exit action.

The reviewed executable was subsequently launched again at the user's request for the successful manual tray tests above. Ticket #3 is complete within its agreed scope; the compatibility limits below remain unchanged.

## Review against 92a5fc4

- Standards: no code findings; clarified the self-test documentation to distinguish native menu-loop coverage from notification-area/action-selection coverage.
- Spec: found an Exit-completion message lost inside `TrackPopupMenu`'s nested loop. Fixed by retaining worker completion and dismissing an open menu; the reviewer confirmed resolution. No remaining blocking code findings.

## Repeat the desktop checks

1. Launch `winroll.exe` at normal privilege. Open ordinary, unsnapped target windows of different sizes.
2. Right-click a positively identified empty caption twice; verify rolled height, then restored position and expanded size. Repeat in Edge, Chrome and Explorer. Check that ordinary right-clicks elsewhere retain their behavior.
3. Roll multiple targets, choose Unroll all, and verify each expands to its own saved size. Roll them again and choose Pause; verify expansion, then verify caption gestures no longer roll windows.
4. Choose Enable and verify gestures resume. Roll multiple targets and choose Exit; verify each expands before the process and tray icon disappear.
5. Close a rolled disposable fixture; ensure normal Exit succeeds. Run `--self-test` for controlled refusal/retry and stale-identity regression checks.

## Limits

This is implementation validation, not the full #6 compatibility matrix. Twenty cycles per real application, mixed-DPI movement, Windows 10 22H2, Windows 11 24H2, x86 targets, hung-thread recovery and actual HWND reuse remain unverified. Invalidating the native fixture's identity marker tests the stale-identity guard, not actual OS handle reuse. The previous elevated fixture's unchanged geometry remains historical evidence only; no new elevated rejection-path claim is made. Automatic crash recovery is out of scope.
