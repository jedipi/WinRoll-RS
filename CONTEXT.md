# WinRoll

A desktop utility for changing how application windows occupy the user's workspace.

## Language

**Target application**:
An application whose windows the user wants WinRoll to manage. Its bitness is separate from that of WinRoll or the operating system.

**Roll up**:
Collapse a window to its title bar while keeping the application open.
_Avoid_: Minimize, hide

**Unroll**:
Return a rolled-up window to its expanded size.
_Avoid_: Restore (ambiguous with restoring a minimized window)

**Managed window**:
A target window currently rolled up by WinRoll, with its expanded size retained for unrolling.

**Pause**:
A mode in which WinRoll keeps its managed windows rolled and stops responding to roll and unroll gestures while remaining available in the system tray.
