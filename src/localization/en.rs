use super::Language;

pub(super) const LANGUAGE: Language = Language {
    id: 1,
    name: "English",
    windows_ids: &[0x0409],
    messages: &[
        (
            "Portable update preparation timed out.",
            "Portable update preparation timed out.",
        ),
        (
            "Portable update helper stopped unexpectedly.",
            "Portable update helper stopped unexpectedly.",
        ),
        (
            "Cannot locate Windows PowerShell.",
            "Cannot locate Windows PowerShell.",
        ),
        (
            "Cannot locate the executable directory.",
            "Cannot locate the executable directory.",
        ),
        (
            "The portable staging directory must be an update directory beside the target.",
            "The portable staging directory must be an update directory beside the target.",
        ),
        (
            "The portable package must contain exactly one root winroll.exe.",
            "The portable package must contain exactly one root winroll.exe.",
        ),
        (
            "The portable package does not contain a Windows executable.",
            "The portable package does not contain a Windows executable.",
        ),
        (
            "The portable package does not contain an AMD64 PE32+ executable.",
            "The portable package does not contain an AMD64 PE32+ executable.",
        ),
        (
            "The updated executable exited before startup completed.",
            "The updated executable exited before startup completed.",
        ),
        (
            "The updated executable did not confirm startup.",
            "The updated executable did not confirm startup.",
        ),
        (
            "Recovering windows before updating...\nRetry recovery or cancel to keep WinRoll running.",
            "Recovering windows before updating...\nRetry recovery or cancel to keep WinRoll running.",
        ),
        (
            "Installing update and restarting WinRoll...",
            "Installing update and restarting WinRoll...",
        ),
        (
            "Could not install the update.\n\n{error}",
            "Could not install the update.\n\n{error}",
        ),
        (
            "Installed updates are not supported yet.",
            "Installed updates are not supported yet.",
        ),
        (
            "Update installation stopped unexpectedly.",
            "Update installation stopped unexpectedly.",
        ),
        ("Portable update failed.", "Portable update failed."),
        (
            "The previous version was restored. Please try the update again.",
            "The previous version was restored. Please try the update again.",
        ),
        (
            "The previous executable is saved at:",
            "The previous executable is saved at:",
        ),
        ("&Update now", "&Update now"),
        (
            "Could not download the update.\n\n{error}",
            "Could not download the update.\n\n{error}",
        ),
        (
            "Downloading update: {percent}% ({bytes} bytes)",
            "Downloading update: {percent}% ({bytes} bytes)",
        ),
        (
            "Downloading update... ({bytes} bytes)",
            "Downloading update... ({bytes} bytes)",
        ),
        ("Verifying update package...", "Verifying update package..."),
        (
            "Update {version} verified and staged.\nYour installation has not changed.\nClosing this window discards the package.\n\n{path}",
            "Update {version} verified and staged.\nYour installation has not changed.\nClosing this window discards the package.\n\n{path}",
        ),
        (
            "Update package has no SHA-256 digest.",
            "Update package has no SHA-256 digest.",
        ),
        (
            "Update package has an invalid SHA-256 digest.",
            "Update package has an invalid SHA-256 digest.",
        ),
        (
            "Update package SHA-256 does not match.",
            "Update package SHA-256 does not match.",
        ),
        (
            "No matching update package was found.",
            "No matching update package was found.",
        ),
        (
            "Update download stopped unexpectedly.",
            "Update download stopped unexpectedly.",
        ),
        (
            "Update check stopped unexpectedly.",
            "Update check stopped unexpectedly.",
        ),
        ("Update canceled.", "Update canceled."),
        ("Close", "Close"),
        ("Version {version}", "Version {version}"),
        ("Created by Kin Tam", "Created by Kin Tam"),
        ("About WinRoll RS", "About WinRoll RS"),
        ("&Check for update...", "&Check for update..."),
        ("&Update available...", "&Update available..."),
        ("WinRoll RS - Updates", "WinRoll RS - Updates"),
        ("Checking for updates...", "Checking for updates..."),
        ("You're up to date.", "You're up to date."),
        ("Update available: {version}", "Update available: {version}"),
        (
            "Could not check for updates.\n\n{error}",
            "Could not check for updates.\n\n{error}",
        ),
        ("&Retry", "&Retry"),
        ("Cancel", "Cancel"),
        (
            "Cannot open the update window",
            "Cannot open the update window",
        ),
        ("WinRoll RS - Options", "WinRoll RS - Options"),
        ("WinRoll RS Options", "WinRoll RS Options"),
        (
            "&Automatically start with Windows",
            "&Automatically start with Windows",
        ),
        ("&Ignore middle mouse button", "&Ignore middle mouse button"),
        ("&Sound effects", "&Sound effects"),
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
