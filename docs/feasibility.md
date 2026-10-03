# Ticket #2 feasibility results

Date: 2026-09-24. Status: runnable experiment implemented; core roll/unroll demonstrated on all three required applications. Maximized, snapped and elevated-target exclusion have observed checks, along with normal-exit recovery, restoration retry and closed-window cleanup. This is feasibility evidence, not a universal compatibility claim; environment and coverage gaps remain listed below.

## Environment

- OS: Windows build 26200.9168, DisplayVersion 25H2, x64. The registry's legacy ProductName value says Windows 10 Pro; build/display-version are recorded rather than relying on that misleading name.
- Rust 1.98.1, Cargo 1.98.1, MSVC 14.51.36231, Windows SDK 10.0.26100.0.
- Dependency: windows-sys 0.61.2, with Cargo.lock committed.
- Display A: 2560x1440 at (0,0), work area (0,0)-(2560,1392), effective DPI 96x96.
- Display B: 1080x1920 at (2560,0), work area (2560,0)-(3640,1872), effective DPI 96x96.
- Tests against real applications ran outside the Codex sandbox at normal user privilege. Native fixture and build checks also ran inside the sandbox. The program refuses high-integrity execution.
- Ordinary-window desktop gestures were generated through the computer-use tool and the normal mouse-hook path, not direct calls to the application's toggle function. The dedicated elevated-fixture gesture was performed by the user; input-device details were not recorded.

## Observed results

| Target/check | Result | Evidence |
| --- | --- | --- |
| File Explorer 10.0.26100.8117 | Pass for tested ordinary window | Caption gesture rolled 1080x872 to 1080x39; a second gesture restored and verified 1080x872. |
| Microsoft Edge 152.0.4191.53 | Pass for tested ordinary window | Caption gesture rolled 1094x912 to 1094x39; a second gesture restored and verified 1094x912. |
| Google Chrome 153.0.8010.48 | Pass for tested ordinary window | Caption gesture rolled 1265x956 to 1265x39; a second gesture restored and verified 1265x956. |
| Chrome movement at 96 DPI | Pass for horizontal drag | Window remained rolled after dragging its caption; unroll verified 1265x956 at its new position (605,278). |
| Exit with all three applications rolled | Pass on reviewed release build | `q` restored and verified all three saved sizes before the process exited with code 0. |
| Maximized Chrome | Pass | Gesture passed through, no roll; normal window menu remained available. |
| Snapped native window | Pass | Persistent fixture snapped using a real title-bar drag. Caption right-click logged `snapped=1 maximized=0 minimized=0`, passed through to the normal system menu and left the snapped geometry unchanged. |
| Elevated Task Manager manual attempt | Inconclusive | User confirmed completing the requested caption gesture. The controller recorded no Task Manager event or roll operation. Captured pass-through handles mapped to Explorer, while another rolled/unrolled target belonged to ApplicationFrameHost. These unrelated events do not verify elevated-target rejection. |
| Dedicated elevated native fixture | Pass for observed exclusion | User repeated the caption gesture on a dedicated fixture. PID 73316, HWND 4001472, TokenElevation=1 before/after; controller PID 60544 had TokenElevation=0. Rectangle remained exactly (0,100)-(800,700), or 800x600. Fixture was neither snapped nor maximized. No controller event/roll was captured, so this verifies unchanged elevated-window behavior, not which internal rejection path handled the input. |
| Edge tab right-click | Pass in observed sample | Hit-test returned HTCLIENT; gesture passed through without rolling. |
| Chrome address-bar right-click | Pass on reviewed release build | HTCLIENT; normal edit menu appeared and geometry stayed unchanged. |
| Edge close-button right-click | Pass in observed sample | Probe rejected the point; window stayed rolled and was subsequently restored by normal exit. |
| Native fixture | Pass | 20 measured geometry cycles, client-area rejection, maximized exclusion. |
| Native restoration refusal/retry | Pass | Fixture enforces a small height after an external expansion request. First restoration fails and retains state; disabling the refusal and retrying restores the original 800x600 geometry. |
| Closed rolled native window | Pass | Fixture rolled, then destroyed through WM_CLOSE. Restore-all discarded its obsolete state without attempting to resize another window. |
| Child-control regression | Pass | A native child button returns HTCLIENT while its parent reports HTCAPTION behind it. Test failed with parent fallback, passes with authoritative child hit-testing. |
| Geometry regression | Pass | New position retained, offscreen placement corrected, negative monitor coordinates and oversized windows covered. |
| Formatting, Clippy, unit test and release build | Pass | Commands in README; no lint allowances added. |

The reviewed release build re-demonstrated roll-up of all three targets simultaneously and verified restoration of each on exit. Earlier exploratory builds also tested a full caption-toggle cycle on each. The 20-cycle test here is the native fixture; 20 cycles on each real application belong to the release acceptance matrix and were not claimed as run.

## Findings that affected implementation

File Explorer's root window returned HTCLIENT for the tested empty title-bar point, while the child window directly beneath the pointer returned HTCAPTION. Root-only hit-testing missed this caption. The experiment therefore asks the actual window under the pointer and only accepts its explicit caption result. It never substitutes the root's caption result for a child's interactive result.

The hook waits at most 40ms for a worker's classification. Cross-process hit-testing has a 20ms timeout. No resize occurs in the hook. A timed-out probe cannot cause a delayed resize: a separate Toggle command is sent only if classification arrives within the hook's deadline. Busy or unresponsive paths pass clicks through.

Resize requests use SWP_NOSENDCHANGING with asynchronous SetWindowPos. Success requires five matching geometry samples 20ms apart within 350ms. Failed roll attempts trigger restoration; failed restoration retains state for retry. This bounded observation does not prove that an application will retain a rolled size indefinitely or after changing its own layout.

## Unverified and remaining work

- The dedicated elevated fixture remained unchanged across the user's gesture, with identity, privilege and geometry measured before/after. The hook produced no event for that gesture; direct execution of the token-comparison rejection branch is not established by this observation. The earlier Task Manager attempt remains inconclusive and is superseded by the dedicated fixture's observed exclusion result.
- Both available monitors are 96 DPI. Mixed-DPI transitions, automatic rolled-caption resizing after a DPI change, monitor removal and vertical movement are unverified. Production handling belongs to ticket #5.
- Windows 10 22H2, Windows 11 24H2 and 32-bit targets were not available/tested in this run.
- Broader combinations of tab layouts, browser themes, caption buttons, page menus, application self-resizing and long-lived rolled state remain unverified.
- Restoration refusal/retry and closed-window cleanup are tested against the native fixture. Hung-thread recovery and actual handle-reuse scenarios remain unverified.
- Console closure, forced termination and crashes cannot promise restoration; automatic crash recovery remains out of scope.
- There is no tray menu or settings UI in this ticket.

No required application failed the tested roll/unroll path. Any failure in the remaining required behavior must reopen the architecture discussion before downstream feature work; do not infer injection is authorized or silently exclude a required application.

After each manual attempt, the controller was stopped through its normal q exit path and returned exit code 0. No experiment controller was left running. The elevated disposable fixture is left for the user to close; it contains no user data and installs no hook.

## Repeat the desktop experiment

1. Run the release executable from a normal terminal. Record its monitor inventory and your OS/application versions.
2. Open ordinary unsnapped Edge, Chrome and File Explorer windows. Right-click an empty caption, inspect the rolled result, then right-click again. Match ROLL and UNROLL verified output to the observed window.
3. Try tab, address-bar, page and caption-button right-clicks. They should retain their normal behavior without a ROLL entry.
4. Repeat with maximized, snapped and elevated test windows; these should remain unchanged.
5. Move a rolled window, unroll it, and verify its new position and original expanded size. Repeat across monitors with different scaling where available.
6. Roll all three windows and enter q. Verify each restored before the process exits. If recovery fails, keep the process open and retry q after the target responds.

## References

- [IsWindowArranged and snapped-window state](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-iswindowarranged)
- [LowLevelMouseProc lifetime and timeout constraints](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc)
- [SetWindowPos flags](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos)

## Review

Standards review: no actionable findings. Spec review: one child-hit-test fallback defect identified, reproduced in the native fixture and fixed; reviewer confirmed resolution. Remaining verification gaps are listed above. Review used the empty starting tree because the repository had no commits.

Follow-up review against a8f3e48: both Standards and Spec reviewers found no blocking code findings in the persistent fixture, explicit exclusion diagnostics and closed-window cleanup test. They confirmed that closed-window cleanup does not establish actual replacement-handle behavior, and that elevated-input and mixed-DPI claims must remain unverified without evidence.

Elevated-fixture launch fix reviewed against b66fc38: both reviewers found no actionable findings. The interactive fixture makes a second ShowWindow call after the first consumes the launcher's hidden startup flag, exposing the requested test window while its console stays hidden. Release build, formatting, Clippy and unit tests passed. The elevated fixture was successfully displayed and used in the manual check above.
