use super::Language;

pub(super) const LANGUAGE: Language = Language {
    id: 1,
    name: "English",
    windows_ids: &[0x0409],
    messages: &[
        ("Close", "Close"),
        ("Version {version}", "Version {version}"),
        ("Created by Kin Tam", "Created by Kin Tam"),
        ("About WinRoll RS", "About WinRoll RS"),
        ("WinRoll RS - Options", "WinRoll RS - Options"),
        ("WinRoll RS Options", "WinRoll RS Options"),
        (
            "&Automatically start with Windows",
            "&Automatically start with Windows",
        ),
        ("&Ignore middle mouse button", "&Ignore middle mouse button"),
        ("As &icon", "As &icon"),
        ("As &menu", "As &menu"),
        ("Minimize to tray", "Minimize to tray"),
        ("&Transparency", "&Transparency"),
        ("Transparency", "Transparency"),
        (
            "{percent}% (0% opaque, 100% invisible)",
            "{percent}% (0% opaque, 100% invisible)",
        ),
        (
            "Invisible windows: choose Exit in the tray to restore.",
            "Invisible windows: choose Exit in the tray to restore.",
        ),
        ("&Language", "&Language"),
        ("System default", "System default"),
        ("(none)", "(none)"),
        ("(untitled)", "(untitled)"),
        ("&Minimized", "&Minimized"),
        ("&Enable", "&Enable"),
        ("&Retry unroll all", "&Retry unroll all"),
        ("&Unroll all", "&Unroll all"),
        ("&Options...", "&Options..."),
        ("&About...", "&About..."),
        ("E&xit", "E&xit"),
        ("Recovery needed: {name}", "Recovery needed: {name}"),
        (
            "WinRoll RS - unroll pending; retry Unroll all or Exit",
            "WinRoll RS - unroll pending; retry Unroll all or Exit",
        ),
        ("WinRoll RS - paused", "WinRoll RS - paused"),
        ("WinRoll RS - enabled", "WinRoll RS - enabled"),
        (
            "Cannot access the Windows setting.\n\n{error}",
            "Cannot access the Windows setting.\n\n{error}",
        ),
        (
            "Cannot read own integrity level",
            "Cannot read own integrity level",
        ),
        (
            "Run WinRoll RS without administrator privileges",
            "Run WinRoll RS without administrator privileges",
        ),
        (
            "Cannot enable per-monitor DPI awareness",
            "Cannot enable per-monitor DPI awareness",
        ),
        (
            "Cannot create WinRoll RS mutex",
            "Cannot create WinRoll RS mutex",
        ),
        (
            "Another WinRoll RS controller is already running",
            "Another WinRoll RS controller is already running",
        ),
        ("Already initialized", "Already initialized"),
        (
            "Cannot create the WinRoll RS tray icon",
            "Cannot create the WinRoll RS tray icon",
        ),
        ("Cannot install mouse hook", "Cannot install mouse hook"),
        ("Window worker panicked", "Window worker panicked"),
        (
            "Saved minimize as menu setting must be 0 or 1.",
            "Saved minimize as menu setting must be 0 or 1.",
        ),
        (
            "Saved ignore middle mouse button setting must be 0 or 1.",
            "Saved ignore middle mouse button setting must be 0 or 1.",
        ),
        (
            "Saved transparency must be between 0 and 100 in steps of 10.",
            "Saved transparency must be between 0 and 100 in steps of 10.",
        ),
        (
            "Transparency must be between 0 and 100 in steps of 10.",
            "Transparency must be between 0 and 100 in steps of 10.",
        ),
        (
            "The executable path is too long for Windows startup. Move WinRoll RS to a shorter path.",
            "The executable path is too long for Windows startup. Move WinRoll RS to a shorter path.",
        ),
    ],
};
