# WinRoll-RS planning

Status: planning decisions accepted and synthesized in [the specification](spec.md) for GitHub Issues. No implementation or compatibility tests have been performed.

Reference: ChatGPT conversation "Analyse Rust Rewrite", ID `6ab4aee7-7d1c-83ec-bdb1-950037eab3ba`.

## Settled

- Rust rewrite of the original WinRoll utility.
- Platform scope: [Windows 10/11 x64](adr/0001-supported-platforms.md).
- Vocabulary: [CONTEXT.md](../CONTEXT.md).
- First release: roll/unroll, tray controls, and an exit action that unrolls managed windows.
- Primary acceptance targets: Microsoft Edge, Google Chrome, and Windows File Explorer.
- Delivery: portable executable for personal testing.
- Activation: right-click an empty area positively identified as the window caption toggles roll/unroll. Preserve normal clicks on tabs, caption buttons, address bars and page content.
- Initial eligibility: ordinary, unsnapped windows only. Maximized and snapped windows remain unchanged; reliable eligibility detection is part of the feasibility gate.
- Privileges: run normally and initially manage applications at the same privilege level; elevated targets are outside the first-release scope.
- Recovery: unroll managed windows on normal exit. Automatic recovery after a crash or forced termination is deferred.
- Tray actions: Enable/Pause, Unroll all, Exit. Pause keeps managed windows rolled and stops intercepting gestures; Exit unrolls before quitting.
- Compatibility gate: if non-injected roll-up fails on Edge, Chrome or File Explorer, revisit architecture with the user before implementation continues. Do not silently drop a required application or introduce injection.
- Unroll at the window's current position using its saved expanded size; adjust placement to keep the window reachable onscreen.
- Multiple monitors with different DPI scaling are in scope. Recalculate rolled title-bar size for the destination monitor.
- If restoration fails during Exit or Unroll all, keep WinRoll running, identify the failed window and allow retry. Retain its recovery state; discard state for windows that have closed. Stop intercepting new roll gestures while pausing or exiting.

## Acceptance checks

Record the OS version, application version, architecture and monitor/scaling setup for each run. Mark missing environments unverified; intended support is not a claim that testing occurred.

- Edge, Chrome and File Explorer each pass 20 roll/unroll cycles on ordinary windows.
- Expanded size and position restore correctly; a moved rolled window expands at its new position, adjusted for screen reachability.
- Tabs, buttons and ordinary right-click menus retain their behavior.
- Maximized and snapped windows remain unchanged.
- Pause preserves multiple managed windows; Unroll all and Exit restore them.
- Restoration failure leaves WinRoll available for retry and preserves recovery state.
- Closing a rolled window causes no error or accidental action on another window.
- Moving a rolled window between monitors with different scaling preserves usable caption sizing and correct unroll geometry.
- A small 32-bit test application verifies cross-bitness behavior without adding a 32-bit WinRoll release binary.

## Deferred features

These are candidates for later releases, not commitments for the first release:

- Always-on-top toggle.
- Send window to back.
- Transparency toggle and opacity settings.
- Minimize to tray and restore from tray.
- Bulk actions across eligible windows (roll/unroll, minimize, maximize, restore, close, transparency), except the first-release Unroll all action for managed windows.
- Additional original gestures on minimize, maximize and close buttons, plus modifier-based shortcuts.
- Roll-up of maximized or snapped windows.
- Elevated target applications.
- Automatic crash recovery.

Feature inventory source: [original WinRoll DLL](https://raw.githubusercontent.com/FelixHuoEZ/winroll/master/winrolldll.asm). Public packaging, installation, automatic startup and updates are also outside the currently accepted scope.

## Feasibility findings

- The original implementation subclasses target windows and handles minimum-size and window-position messages to maintain the rolled state. A simple external resize does not reproduce that mechanism. [Original source](https://raw.githubusercontent.com/FelixHuoEZ/winroll/master/winrolldll.asm)
- A low-level mouse hook runs in the installing process, requires a message loop, and risks silent removal if callbacks take too long. [Microsoft documentation](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc)
- Target windows can influence resizing. Testing `SetWindowPos` with `SWP_NOSENDCHANGING` is a candidate experiment, not proof of persistent roll-up compatibility. [SetWindowPos](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos), [WM_WINDOWPOSCHANGING](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-windowposchanging)
- Caption hit-testing requires application cooperation; browser title-bar and tab-strip behavior must be tested separately. [WM_NCHITTEST](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-nchittest)
- Cross-integrity message sending and window-style changes have restrictions, supporting the initial same-privilege scope. [SendMessage](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendmessage), [SetWindowLongPtrW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowlongptrw)
- Original source carries a permissive license requiring attribution, preservation of its notice and identification of altered source versions if reused. [Original repository](https://github.com/FelixHuoEZ/winroll)

## Implementation approach to validate

- Start with one Rust executable using native Win32 APIs and a low-level mouse hook, avoiding injected DLLs.
- Keep hook processing bounded and move window operations out of the callback. Caption identification and mouse-event consumption must be proven together so unrelated clicks remain intact.
- Keep per-window recovery state in memory. Validate window identity before restoration so a reused window handle cannot affect another application window.
- Verify actual window geometry after roll/unroll requests; an accepted API call alone is not a successful roll.
- Keep the first version manually launched with fixed gestures and tray actions. Add no settings framework or installer for this scope.
- Final module boundaries and library versions are implementation choices, to be kept minimal after the feasibility experiment.

## Milestones

### 1. Prove non-injected feasibility

Build a minimal x64 experiment to identify eligible captions, roll/unroll and restore Edge, Chrome and File Explorer. Test ordinary versus maximized/snapped windows, target minimum-size behavior, preservation of unrelated mouse actions and mixed-DPI movement. Record results and reproduction steps.

Exit criterion: demonstrate the required behavior on available environments. Any required-app incompatibility or inability to safely identify eligible windows triggers an architecture review before proceeding. Missing environments remain explicit validation gaps.

### 2. Build the personal-testing utility

Turn the proven path into the portable tray utility. Add managed-window state, Enable/Pause, Unroll all, Exit, restoration retry and closed-window cleanup. Keep a small runnable regression check for geometry/state logic and use native-window integration checks for behavior that pure tests cannot establish.

Exit criterion: implemented tray actions and recovery semantics pass their acceptance checks, including multiple managed windows and restoration failure.

### 3. Validate and package

Run the acceptance matrix on available Windows versions, application versions, x86 fixture and mixed-DPI monitors. Produce the x64 executable, concise usage/recovery instructions and a compatibility report listing passes, failures and unverified environments. Preserve applicable original-source notices if source is reused.

Exit criterion: a portable personal-testing build with reproducible results and explicit limitations. Do not call an unverified acceptance check passed.

## Remaining gate

The product-decision interview is complete and specification publication is authorized. Feasibility findings may require revisiting the architecture under the accepted compatibility policy.
