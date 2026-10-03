# WinRoll RS — usage and recovery

WinRoll RS collapses ordinary application windows to their title bars. This portable x64 build is intended for Windows 10 22H2 and Windows 11 21H2 or newer. See the repository's [compatibility report](https://github.com/jedipi/WinRoll-RS/blob/main/docs/compatibility-report.md) for tested versions, observed results and unverified environments; intended support is not a claim that every environment passed.

For the portable ZIP, extract it to a folder and double-click `winroll.exe`. Alternatively, run the setup executable to install for your Windows account, then launch **WinRoll RS** from the Start menu. The installer requires no administrator privileges and creates an uninstall entry in Windows Settings. Exit WinRoll RS through its tray before installing over an existing version or uninstalling; the installer does not force it to quit.

Run without administrator privileges. Find **WinRoll RS** in the notification area, including the hidden-icons menu. Only one controller can run at a time. Startup and updates are manual.

Right-click empty title-bar space to roll a window; right-click again to expand it. Use ordinary, unsnapped windows at the same privilege level. Maximized, snapped and elevated windows are excluded. Unrecognized title areas keep their normal clicks, as do tabs, buttons, address bars and page content.

Left-drag a rolled window's empty title bar to move it; a plain left click focuses it. When expanded, it recovers its saved size at the new position, adjusted to keep it reachable. Native title-bar double-click behavior is unavailable while rolled.

Open the WinRoll RS tray menu:

- **Pause** keeps windows rolled and blocks roll/unroll gestures. Rolled windows remain draggable. **Enable** resumes gestures.
- **Unroll all** expands every managed window while preserving the enabled or paused mode.
- **Exit** expands managed windows before quitting.
- **About WinRoll RS** shows version and creator information.

If expansion fails, WinRoll RS stays running and keeps the window's saved geometry. The tray menu identifies affected windows by title, PID and handle. When the application responds again, choose **Retry unroll all**. A successful retry also completes a pending Exit; closing the affected window normally can allow Exit to finish. Do not force WinRoll RS to quit while recovery is pending.

Saved geometry exists only in memory. There is no automatic recovery after a crash, forced termination or logoff. Use **Unroll all** or **Exit** before ending your session. Report failures with the application and Windows versions, display scaling, steps and visible result; distinguish a rejected gesture from a window that rolled but could not expand.

`SHA256SUMS.txt` contains SHA256 hashes for the supplied executable and documents. Third-party notices are in `THIRD-PARTY-NOTICES.txt`.
