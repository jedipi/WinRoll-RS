# Ticket #5 placement validation

## Environment and results

2026-09-27: Windows 11 25H2 build 26200.9168, x64. The available monitors are (0,0)-(2560,1440), work area (0,0)-(2560,1392), and (2560,0)-(3640,1920), work area (2560,0)-(3640,1872). Both report 96 DPI (100% scale) through `--inventory`.

- **Pass:** `cargo test --locked` checks unchanged and moved placement, negative monitor coordinates, oversized windows, and a 60-pixel caption near the work-area bottom.
- **Pass:** `--self-test` rolled an 800x600 native fixture to 800x39, moved it from (100,100) to (200,180) while rolled, and verified that it expanded to 800x600 at (200,180). It also forced a stale DPI/height state and verified that the rolled caption returned to the current 96-DPI height of 39 pixels. This is a synthetic DPI check, not a physical scale transition.
- **User-reported pass on the manual-drag build:** Real caption gestures and moved-window placement in Edge, Chrome and File Explorer, as detailed below. The fixture test alone does not exercise the mouse hook or application captions.
- **Mixed-DPI follow-up pass on the tested configuration:** Native 96/120-DPI transitions and the user's Explorer boundary retest passed after preserving the rolled width. See the follow-up below; other scale combinations remain unverified.

On 2026-09-27, live validation reported that dragging rolled Edge, Chrome and Explorer windows exposed their address/favorites or toolbar area, and it remained visible after release. The previous DPI-only maintenance missed same-DPI height growth. A native fixture now reproduces that geometry change and verifies re-collapse.

A retest of the height-correction build found that the extra bars still appeared **during** the drag in all three required applications, then disappeared on mouse release. A caption-region clipping experiment passed the native fixture but did not fix the live drag; it was discarded.

The diagnostic trace showed Explorer's actual outer height change from 39 to 243 pixels during the held caption drag at 96 DPI. Windows' native move loop applies the target's minimum tracking height. A subsequent non-injected build intercepts left dragging only on a managed window's positively identified caption and moves that window through the existing worker with position-only requests. On Windows 11 25H2 build 26200.9168, the user reported that Edge 153.0.4234.48, Chrome 153.0.8010.53, and Explorer 10.0.26100.8117 followed the pointer while remaining rolled and unrolled at their new positions. The user also reported normal left clicks on other windows, tabs and caption buttons. These are live user observations; mixed-DPI movement remains unverified.

The user also moved a rolled window between the two 96-DPI monitors; it stayed rolled and unrolled at a reachable position. A later correction clarified that clicking a rolled window **already behind** another window must both raise and activate it. Live diagnostics showed `SetForegroundWindow` denied activation (`activated=0`) even after Z-order requests and temporary thread-input attachment. A topmost-then-normal Z-order experiment raised it visually, but the previous window still received Alt+Space. A tagged synthetic mouse click activated it but exposed client content during drag; adding `WM_CANCELMODE` kept the caption short but prevented dragging. These experiments were discarded.

After explicit architecture review, WinRoll instead sends a tagged Alt key tap only when Windows denies foreground activation on a managed rolled-caption click, then retries `SetForegroundWindow`. This injects keyboard input, not code into the target. The user reported that the rolled window came forward and received Alt+Space while remaining caption-height through the drag, then unrolled normally. Edge and Chrome passed the same roll-drag-unroll check; the previously active browser showed no menu response in the observed sample. Broader menu and modifier combinations and physical mixed-DPI movement remain unverified.

The final release build records the caption press before queueing foreground activation and bounds how long a queued press may inject input. It passed a repeat live Explorer check: a quick click on a rolled window behind another brought it forward and gave it Alt+Space, and a short drag stayed collapsed before unrolling at the new position.

While rolled, a plain left click on empty caption space focuses the window; native caption double-click behavior is not preserved by the manual drag path.

## Mixed-DPI follow-up

The right-hand portrait display was subsequently changed to 125% (120 DPI), with work area (2560,0)-(3640,1860); the primary remained at 100% (96 DPI). The user initially observed transient client content in Explorer while dragging across the boundary in either direction, disappearing again before mouse release. That failed live check superseded the earlier unavailable-configuration status and reopened #5.

The native fixture now handles `WM_DPICHANGED` by applying Windows' suggested rectangle, and the mixed-monitor regression applies a 243-pixel minimum tracking height. On the previous implementation an unfinished move left the fixture at 243 pixels on the 120-DPI monitor, where caption height was 47. Explicit caption-height requests on every drag update correct that growth in both directions; active-drag maintenance also issues position-preserving asynchronous corrections. These native checks verify correction before release, not the absence of transient visible frames.

The first live retest still exposed content at a particular boundary position, with alternating 96/120-DPI logs. A diagnostic sample showed a 1,350-pixel rolled width despite a 1,080-pixel drag-start width. The final change preserves the saved physical width during movement and maintenance. Native checks now recover 800x47 at 120 DPI and 800x39 at 96 DPI, and held-boundary positions settle. The fixture still observes the application's own transient resize at the initial DPI change; this alone does not establish a flash-free transition.

**User-reported live pass:** After testing the width-stability candidate at the troublesome Explorer boundary position, the user reported "all good." This validates the reported symptom on the available 100%/125% setup. Other scale pairs and a complete mixed-DPI matrix across all target applications remain #6 validation work. Temporary diagnostics were removed; formatting, Clippy, unit tests and the native mixed-DPI self-test passed.

## Repeat the live checks

1. Run `winroll.exe` normally and `winroll.exe --inventory`. Record the OS build, each target application's version and architecture, and each monitor's arrangement, work area and scaling.
2. In each of Edge, Chrome and File Explorer, use an ordinary unsnapped window. Record its expanded rectangle. Right-click an empty caption to roll it, then right-click again without moving it. Verify the original rectangle returns.
3. Roll again, drag its caption to a different position, and unroll by right-clicking the caption. Verify the saved expanded width and height return at the new position.
4. Move a rolled window near each work-area edge and unroll. Verify the caption remains reachable and the expanded rectangle matches the saved size.
5. On monitors with different scaling, drag a rolled window across monitors. Check that its rolled caption takes the destination DPI's height and remains usable; unroll and verify size, position and reachability. Record measured rectangles and DPI values. If no mixed-scale setup is available, leave this check unverified.

Use `--fixture` for disposable manual geometry checks. Close the fixture and exit WinRoll through its tray menu after testing.
