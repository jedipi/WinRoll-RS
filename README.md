# WinRoll RS

WinRoll RS is a Rust reimplementation of WinRoll 2.0 for Windows 10 and Windows 11.

Visit the [WinRoll RS website](https://jedipi.github.io/WinRoll-RS/) to see how it works.

[![Download WinRoll RS](https://img.shields.io/badge/Download-WinRoll%20RS-2563eb?style=for-the-badge)](https://github.com/jedipi/WinRoll-RS/releases/latest)

Direct downloads for the latest release, v0.1.0:

- [Portable version (ZIP, x64)](https://github.com/jedipi/WinRoll-RS/releases/download/v0.1.0/winroll-0.1.0-x64-portable.zip)
- [Installer (EXE, x64)](https://github.com/jedipi/WinRoll-RS/releases/download/v0.1.0/winroll-0.1.0-x64-setup.exe)

WinRoll lets you roll a window up into its title bar, like a window shade. It also has a few small window-management features: Always on Top, Send to Back, transparency, and minimizing windows to the system tray.

I want to keep the original WinRoll's simplicity and feel, using modern Windows APIs and code that's easier to maintain.

## Background

Back in the early 2000s, I was obsessed with a tiny Windows utility called WinRoll.

Its main feature was simple and useful:

> Right-click a window's title bar and the entire window rolls up, leaving only the title bar visible.

Wil Palma created the original WinRoll, which reached version 2.0 in 2004.

Development eventually stopped. FelixHuoEZ later uploaded the original source code to GitHub:

https://github.com/FelixHuoEZ/winroll

The original was written almost entirely in 32-bit x86 Assembly using MASM, for much older versions of Windows.

I'm rebuilding it for modern Windows while keeping it small and out of the way.

## Goals

I want WinRoll RS to:

- Support Windows 10 x64
- Support Windows 11 x64
- Run as a lightweight background utility
- Preserve the original WinRoll user experience
- Support both 32-bit and 64-bit target applications where possible
- Handle DPI scaling correctly on modern Windows
- Support multi-monitor environments
- Avoid DLL injection whenever possible
- Use modern Win32 APIs
- Keep memory and CPU usage very low
- Maintain a small and understandable codebase

Windows XP, Vista, 7, 8, and 32-bit Windows are outside the project's scope.

## Core feature

### Roll up / roll down

Right-click a window's title bar to roll it up. Right-click again to restore it.

Normal window:

![File Explorer at its normal size](assets/screenshots/normal-window.png)

Rolled window:

![File Explorer rolled up to its title bar](assets/screenshots/rolled-window.png)

WinRoll saves the window's original size and position so it can restore them exactly when you roll it down.

### Always on Top

Middle-click a window's **Close (X)** button to toggle Always on Top. Middle-click
it again to turn it off. Exiting WinRoll RS through its tray restores the original
Always on Top state of windows changed with this gesture. Pause and Unroll all
leave Always on Top unchanged.

### Send to Back

Right-click a window's **Close (X)** button to send it behind other windows without
minimizing it or changing its size or position. 

### Transparency

Middle-click an empty title-bar area to toggle the configured transparency. Middle-click
again to restore the window's original appearance. Pause keeps the current appearance
and disables the gesture; Unroll all leaves transparency unchanged.

At 100% the window is invisible and cannot be clicked. Choose **Exit** in the
WinRoll RS tray menu to restore affected windows before quitting. Applications
that use per-pixel layered rendering are excluded.

### Minimize to Tray

Middle-click a window's **Minimize** button to hide it from the desktop and taskbar
and give it a notification-area icon. Click that icon to show the window again.
Check the hidden-icons overflow if the icon is not immediately visible.

Pause disables the gesture but keeps hidden windows available through their icons.
Unroll all leaves them hidden. Exit shows them before quitting, preserving their
normal or maximized state; restoring an individual rolled window keeps it rolled.
If a window cannot be shown, WinRoll keeps its icon and lets you retry.

The button must report the native Minimize hit region. Unrecognized custom controls
and windows running at a different integrity level retain their normal clicks.
Exit WinRoll through its tray before ending its process: hidden-window state is held
in memory and cannot be recovered by restarting WinRoll after a forced termination.

## Options

Open **Options...** from the WinRoll RS tray menu to open **Options**. Select
**Automatically start with Windows** to launch WinRoll RS when you sign in, or
clear it to disable startup.

The **Transparency** slider has 10% steps from **0%** (no transparency) to **100%**
(fully transparent). The default is **50%**. Changes are saved for the next run and
apply the next time transparency is enabled on a window.

## Planned features

### Minimize to Tray

Hide selected windows from the desktop and taskbar, then restore them from the WinRoll system tray menu.

### System tray

WinRoll runs in the Windows notification area.

WinRoll starts enabled. Left-click its tray icon to pause; left-click again to
enable it. Right-click the icon to open the menu. While Exit recovery is pending,
use the menu to retry restoration before enabling gestures again.

The planned tray menu includes:

- Options (available now)
- Rolled windows
- Hidden windows
- Enable / Disable WinRoll
- Start with Windows (available in Options)
- About
- Exit

### Mouse Controls

The classic WinRoll interaction model will be preserved where practical.

Initial planned controls:

| Action | Mouse Gesture |
|---|---|
| Roll Up / Roll Down | Right-click title bar |
| Transparency (available now) | Middle-click title bar |
| Always on Top (available now) | Middle-click Close button |
| Send to Back (available now) | Right-click Close button |
| Minimize to Tray | Middle-click Minimize button |

Mouse actions will eventually be configurable.

### Polish

Other planned additions include:

- configurable mouse gestures
- excluded applications
- keyboard shortcuts
- optional animations
- logging
- portable configuration
- installer
- automatic update support

## Non-goals

WinRoll RS is a small window utility, with no plans to become a full desktop window manager or replace:

- Windows Snap
- PowerToys FancyZones
- virtual desktop managers
- tiling window managers

The focus is on small, useful window-management actions built around the classic Roll Up behaviour.

## Original WinRoll

Original WinRoll source mirror:

https://github.com/FelixHuoEZ/winroll

Original application: WinRoll 2.0

Copyright © 2003–2004 Wil Palma.

The original project's permissive license allows modification and redistribution as long as:

1. The origin of the original software is not misrepresented.
2. Altered versions are clearly identified as altered versions.
3. The original license notice is preserved.

WinRoll RS is a modern reimplementation inspired by WinRoll. It is not the original WinRoll 2.0.

## Credits

Thanks to Wil Palma for creating the original WinRoll.

Once I got used to rolling windows up, it was hard to do without it.

Thanks also to FelixHuoEZ for preserving the WinRoll source code on GitHub.
