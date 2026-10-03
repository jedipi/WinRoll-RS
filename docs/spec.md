# WinRoll-RS: portable roll/unroll utility

## Problem Statement

The user wants to temporarily collapse application windows to their title bars, keeping applications open while freeing desktop space. They need this behavior in Microsoft Edge, Google Chrome and Windows File Explorer on modern x64 Windows, without losing window placement or disrupting normal application interactions.

The original WinRoll relies on an older injected-hook and target-window-subclassing design. This repository currently contains planning documents only. A Rust rewrite must establish that a simpler, non-injected approach works on the required applications before promising compatibility.

## Solution

Deliver a manually launched, portable x64 executable for personal testing on Windows 10 22H2 and Windows 11 21H2 or newer. Right-clicking an empty, positively identified caption area toggles an eligible window between rolled and expanded states. The initial release manages ordinary, unsnapped windows at the same privilege level as WinRoll.

A system-tray menu provides Enable/Pause, Unroll all and Exit. Pause keeps managed windows rolled; Exit unrolls them, and failed restoration keeps WinRoll available for retry. Moved windows unroll at their current position using their saved expanded size, adjusted to remain reachable onscreen. Multiple monitors with different scaling are included.

Start with a non-injected feasibility experiment. If a required application cannot meet the behavior contract, revisit the architecture with the user before continuing rather than dropping that application or silently adding injection.

## User Stories

1. As a Windows user, I want a portable executable, so that I can try WinRoll without installing it.
2. As a Windows 10 22H2 user, I want an x64 build, so that I can use WinRoll on the agreed Windows 10 baseline.
3. As a Windows 11 user, I want support from 21H2 onward, so that I can use WinRoll on the agreed Windows 11 baseline.
4. As an Edge user, I want to roll and unroll an ordinary browser window, so that I can temporarily reclaim desktop space.
5. As a Chrome user, I want to roll and unroll an ordinary browser window, so that I can keep it open without its full desktop footprint.
6. As a File Explorer user, I want to roll and unroll an ordinary folder window, so that I can keep my workspace organized.
7. As a user, I want right-clicking an empty caption area to toggle roll/unroll, so that the action is quick and repeatable.
8. As a user, I want a rolled window to retain a usable title bar, so that I can identify, move and unroll it.
9. As a browser user, I want clicks on tabs, buttons, address bars and page content preserved, so that WinRoll does not interfere with browsing.
10. As a user, I want normal right-click menus outside the activation area preserved, so that application interactions remain familiar.
11. As a user, I want maximized and snapped windows left unchanged, so that my existing desktop layout is preserved.
12. As a user, I want an unmoved window to return to its previous expanded size and position, so that a roll/unroll cycle is reversible.
13. As a user, I want a moved rolled window to expand at its new position, so that my repositioning is respected.
14. As a user, I want unrolled windows to remain reachable onscreen, so that restoration does not leave them inaccessible.
15. As a multi-monitor user, I want to move rolled windows between monitors with different scaling, so that caption sizing and unroll geometry remain usable.
16. As a user, I want to manage multiple eligible windows independently, so that each retains its own expanded size.
17. As a user, I want Unroll all to expand all managed windows, so that I can recover my workspace in one action.
18. As a user, I want Pause to keep managed windows rolled and stop intercepting roll and unroll gestures, so that I can temporarily freeze their state.
19. As a user, I want Enable to resume roll gestures, so that I do not have to restart WinRoll after pausing.
20. As a user, I want Exit to unroll managed windows before quitting, so that normal shutdown does not strand rolled windows.
21. As a user, I want a failed unroll during Unroll all or Exit to identify the affected window and allow retry, so that recovery information is not discarded.
22. As a user, I want to close a rolled window normally, so that WinRoll does not require an extra unroll step first.
23. As a user, I want closed-window cleanup to avoid affecting another window, so that stale recovery information cannot alter an unrelated application.
24. As a user, I want WinRoll to run without administrator privileges, so that the first release fits normal desktop use.
25. As a user of older applications, I want the x64 utility to support eligible 32-bit target applications, so that a second WinRoll executable is unnecessary.
26. As a personal tester, I want a compatibility report listing tested versions and unverified environments, so that I can distinguish demonstrated behavior from intended support.
27. As a personal tester, I want concise usage and recovery instructions, so that I understand the controls and the lack of automatic crash recovery.
28. As the project owner, I want required-application failures to trigger an architecture review, so that compatibility is not traded away without a decision.

## Implementation Decisions

- Build in Rust for `x86_64-pc-windows-msvc`. Ship one x64 executable; x86 and x64 target applications remain within the intended compatibility scope.
- Begin with native Win32 APIs and `WH_MOUSE_LL` in the WinRoll process. Injection is not part of the initial approach and must not be introduced automatically if the experiment fails.
- Keep low-level hook processing bounded and perform window operations outside the callback. Prove caption identification and event consumption together; a rectangle that merely resembles a title bar is insufficient.
- Only consume the agreed right-click caption gesture for eligible windows. Preserve interactions on tabs, caption buttons, address bars, page content and other non-activation areas.
- Initially exclude maximized, snapped and elevated targets. Reliable eligibility detection is part of feasibility, not an assumed capability.
- Retain per-window recovery state in memory, including the saved expanded size. Validate the target window's identity before acting on stored state; a handle alone must not permit acting on a replacement window.
- Verify observed geometry after roll/unroll requests. Successful API return values alone do not establish successful behavior.
- On unroll, retain the current window position and recover its saved expanded size, adjusting placement for screen reachability. Account for destination-monitor scaling when moving a rolled window.
- Enable resumes gesture handling. Unroll all expands managed windows. Pause blocks roll and unroll gestures, keeps managed windows rolled and allows their managed caption drag while retaining the tray utility. Exit stops new roll interception, unrolls managed windows and quits only after recovery succeeds or affected windows have closed.
- If restoration during Unroll all or Exit fails, keep the process running, identify the affected window, retain its recovery state and allow retry. Remove recovery state for windows that have actually closed.
- Use fixed gestures and the three agreed tray actions. No settings framework, installation flow or automatic startup is required.
- Responsibilities are gesture/eligibility handling, window manipulation with recovery state, and tray/lifecycle control. Keep their implementation minimal; this specification does not mandate separate modules or abstraction layers for each responsibility.
- There are no existing application interfaces, database schemas or implementation modules to modify. Final internal boundaries and library versions are implementation choices after feasibility is established.
- Preserve applicable original-source notices and identify altered versions if original source is reused.

## Testing Decisions

- Primary test boundary: externally observable behavior of the running WinRoll process against real native windows, using caption gestures and tray actions. This carries forward the user's accepted native-window acceptance checks rather than introducing a new product interface solely for tests.
- A good test asserts visible geometry, preserved input behavior, lifecycle behavior or recovery outcomes. It does not assert internal call order, map layout, helper names or use of a particular API flag.
- Exercise gesture/eligibility handling, manipulation/recovery and tray/lifecycle behavior together at that boundary. Retain one small runnable regression check for geometry and recovery-state logic where deterministic failure cases cannot be established reliably through live application windows.
- No existing tests or test seams are present in this repository. There is no prior test framework or fixture pattern to reuse, and the specification does not require a new general-purpose testing framework.
- Record OS version, target-application version and architecture, monitor arrangement and scaling for each run. Record passes, failures and unverified environments separately.
- Edge, Chrome and File Explorer must each pass 20 roll/unroll cycles on eligible ordinary windows. Verify restored expanded size and position, not just that API requests returned successfully.
- Test a moved rolled window, including movement between monitors with different scaling. Verify usable caption sizing, expansion at the new position and reachable onscreen placement.
- Confirm tabs, buttons, address bars, page content and ordinary right-click menus retain normal behavior. Confirm maximized and snapped windows remain unchanged.
- Exercise Pause, Unroll all and Exit with multiple managed windows. Verify that Pause preserves rolled geometry, Unroll all and Exit restore it, and Enable resumes gestures.
- Exercise restoration failure during Unroll all and Exit. Verify the process remains available, the affected window is identified, recovery state is retained and retry is possible.
- Close a rolled window and confirm cleanup causes neither an error nor an action on an unrelated window, including the stale-identity case.
- Use a small 32-bit native test application to verify cross-bitness behavior. This is a test fixture, not a second WinRoll release binary.
- Missing Windows versions, applications or monitor configurations remain explicitly unverified. A personal-testing build may report such gaps, but cannot claim those acceptance checks passed or broad compatibility was demonstrated.

## Out of Scope

- Windows XP, Vista, 7, 8/8.1, 32-bit Windows and native ARM64 builds.
- Windows releases earlier than the agreed baseline.
- Rolling maximized or snapped windows and managing elevated target applications.
- Automatic restoration following a crash or forced process termination.
- Always-on-top, send-to-back and transparency/opacity controls.
- Minimize-to-tray and restore-from-tray behavior for target applications.
- Bulk roll, minimize, maximize, restore, close or transparency operations beyond Unroll all for currently managed windows.
- Additional original gestures on minimize, maximize or close buttons and modifier-based shortcuts.
- Public distribution packaging, installers, automatic startup, automatic updates and configurable gestures/settings.
- Universal application compatibility or an automatic fallback to injected hooks.

## Further Notes

### Delivery milestones

1. Prove non-injected feasibility on the three required applications. Demonstrate caption targeting, eligible-window detection, roll/unroll geometry, preserved unrelated interactions and mixed-DPI movement on available environments. Record reproduction steps and failures. Required-app incompatibility or unsafe eligibility detection triggers architecture review before further implementation.
2. Build the portable tray utility from the proven approach. Add managed-window recovery, the agreed tray actions, failure retry and closed-window cleanup. Validate the lifecycle and restoration checks.
3. Run the acceptance matrix and package the personal-testing executable with concise usage/recovery instructions and a compatibility report. Clearly distinguish passed checks from unavailable environments.

### Evidence and limitations

The original source subclasses target windows and intercepts sizing messages. Replacing that mechanism with an external resize is a hypothesis requiring testing, not established feature parity. The prior conversation's single-executable recommendation does not remove this feasibility gate.

- [Original WinRoll repository and license](https://github.com/FelixHuoEZ/winroll)
- [Original window-management implementation](https://raw.githubusercontent.com/FelixHuoEZ/winroll/master/winrolldll.asm)
- [Microsoft low-level mouse hook documentation](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc)
- [Microsoft caption hit-testing documentation](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-nchittest)
- [Microsoft SetWindowPos documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos)
- [Microsoft sizing-message documentation](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-windowposchanging)

This specification synthesizes the accepted planning decisions. No implementation, feasibility experiment or compatibility test has been completed.

Published on 2026-09-24 as [GitHub issue #1](https://github.com/jedipi/WinRoll-RS/issues/1) with the `ready-for-agent` label.
