use super::Language;

pub(super) const LANGUAGE: Language = Language {
    id: 2,
    name: "简体中文",
    windows_ids: &[0x0004, 0x0804, 0x1004],
    messages: &[
        (
            "Portable update preparation timed out.",
            "便携版更新准备超时。",
        ),
        (
            "Portable update helper stopped unexpectedly.",
            "便携版更新助手意外停止。",
        ),
        (
            "Cannot locate Windows PowerShell.",
            "无法找到 Windows PowerShell。",
        ),
        (
            "Cannot locate the executable directory.",
            "无法找到程序目录。",
        ),
        (
            "The portable staging directory must be an update directory beside the target.",
            "便携版暂存目录必须位于目标程序旁的更新目录中。",
        ),
        (
            "The portable package must contain exactly one root winroll.exe.",
            "便携版更新包的根目录必须包含且仅包含一个 winroll.exe。",
        ),
        (
            "The portable package does not contain a Windows executable.",
            "便携版更新包不包含 Windows 可执行文件。",
        ),
        (
            "The portable package does not contain an AMD64 PE32+ executable.",
            "便携版更新包不包含 AMD64 PE32+ 可执行文件。",
        ),
        (
            "The updated executable exited before startup completed.",
            "更新后的程序在启动完成前退出。",
        ),
        (
            "The updated executable did not confirm startup.",
            "更新后的程序未确认启动成功。",
        ),
        (
            "Recovering windows before updating...\nRetry recovery or cancel to keep WinRoll running.",
            "正在恢复窗口以便更新...\n请重试恢复，或取消以保持 WinRoll 运行。",
        ),
        (
            "Installing update and restarting WinRoll...",
            "正在安装更新并重启 WinRoll...",
        ),
        (
            "Could not install the update.\n\n{error}",
            "无法安装更新。\n\n{error}",
        ),
        (
            "Installed updates are not supported yet.",
            "暂不支持安装版更新。",
        ),
        (
            "Update installation stopped unexpectedly.",
            "更新安装意外停止。",
        ),
        ("Portable update failed.", "便携版更新失败。"),
        (
            "The previous version was restored. Please try the update again.",
            "已恢复之前的版本。请重试更新。",
        ),
        (
            "The previous executable is saved at:",
            "之前的可执行文件保存在：",
        ),
        ("&Update now", "立即更新(&U)"),
        (
            "Could not download the update.\n\n{error}",
            "无法下载更新。\n\n{error}",
        ),
        (
            "Downloading update: {percent}% ({bytes} bytes)",
            "正在下载更新：{percent}%（{bytes} 字节）",
        ),
        (
            "Downloading update... ({bytes} bytes)",
            "正在下载更新...（{bytes} 字节）",
        ),
        ("Verifying update package...", "正在验证更新包..."),
        (
            "Update {version} verified and staged.\nYour installation has not changed.\nClosing this window discards the package.\n\n{path}",
            "更新 {version} 已验证并暂存。\n当前安装未改变。\n关闭此窗口将丢弃更新包。\n\n{path}",
        ),
        (
            "Update package has no SHA-256 digest.",
            "更新包没有 SHA-256 摘要。",
        ),
        (
            "Update package has an invalid SHA-256 digest.",
            "更新包的 SHA-256 摘要无效。",
        ),
        (
            "Update package SHA-256 does not match.",
            "更新包的 SHA-256 摘要不匹配。",
        ),
        (
            "No matching update package was found.",
            "未找到匹配的更新包。",
        ),
        (
            "Update download stopped unexpectedly.",
            "更新下载意外停止。",
        ),
        ("Update check stopped unexpectedly.", "更新检查意外停止。"),
        ("Update canceled.", "更新已取消。"),
        ("Close", "关闭"),
        ("Version {version}", "版本 {version}"),
        ("Created by Kin Tam", "作者：Kin Tam"),
        ("About WinRoll RS", "关于 WinRoll RS"),
        ("&Check for update...", "检查更新(&C)..."),
        ("&Update available...", "有可用更新(&U)..."),
        ("WinRoll RS - Updates", "WinRoll RS - 更新"),
        ("Checking for updates...", "正在检查更新..."),
        ("You're up to date.", "已是最新版本。"),
        ("Update available: {version}", "有可用更新：{version}"),
        (
            "Could not check for updates.\n\n{error}",
            "无法检查更新。\n\n{error}",
        ),
        ("&Retry", "重试(&R)"),
        ("Cancel", "取消"),
        ("Cannot open the update window", "无法打开更新窗口"),
        ("WinRoll RS - Options", "WinRoll RS - 选项"),
        ("WinRoll RS Options", "WinRoll RS 选项"),
        (
            "&Automatically start with Windows",
            "随 Windows 自动启动(&A)",
        ),
        ("&Ignore middle mouse button", "忽略鼠标中键(&I)"),
        ("&Sound effects", "音效(&S)"),
        ("As &icon", "显示为图标(&I)"),
        ("As &menu", "显示为菜单项(&M)"),
        ("Minimize to tray", "最小化到托盘"),
        ("&Transparency", "透明度(&T)"),
        ("Transparency", "透明度"),
        (
            "{percent}% (0% opaque, 100% invisible)",
            "{percent}%（0% 不透明，100% 不可见）",
        ),
        (
            "Invisible windows: choose Exit in the tray to restore.",
            "窗口不可见时，请在托盘菜单中选择“退出”以恢复。",
        ),
        ("&Language", "语言(&L)"),
        ("System default", "跟随系统"),
        ("(none)", "（无）"),
        ("(untitled)", "（无标题）"),
        ("&Minimized", "已最小化(&M)"),
        ("&Enable", "启用(&E)"),
        ("&Retry unroll all", "重试展开所有窗口(&R)"),
        ("&Unroll all", "展开所有窗口(&U)"),
        ("&Options...", "选项(&O)..."),
        ("&About...", "关于(&A)..."),
        ("E&xit", "退出(&X)"),
        ("Recovery needed: {name}", "需要恢复：{name}"),
        (
            "WinRoll RS - unroll pending; retry Unroll all or Exit",
            "WinRoll RS - 等待展开；请重试“展开所有窗口”或“退出”",
        ),
        ("WinRoll RS - paused", "WinRoll RS - 已暂停"),
        ("WinRoll RS - enabled", "WinRoll RS - 已启用"),
        (
            "Cannot access the Windows setting.\n\n{error}",
            "无法访问 Windows 设置。\n\n{error}",
        ),
        (
            "Cannot read own integrity level",
            "无法读取程序的完整性级别",
        ),
        (
            "Run WinRoll RS without administrator privileges",
            "请勿以管理员身份运行 WinRoll RS",
        ),
        (
            "Cannot enable per-monitor DPI awareness",
            "无法启用每显示器 DPI 感知",
        ),
        (
            "Cannot create WinRoll RS mutex",
            "无法创建 WinRoll RS 互斥锁",
        ),
        (
            "Another WinRoll RS controller is already running",
            "另一个 WinRoll RS 实例正在运行",
        ),
        ("Already initialized", "已初始化"),
        (
            "Cannot create the WinRoll RS tray icon",
            "无法创建 WinRoll RS 托盘图标",
        ),
        ("Cannot install mouse hook", "无法安装鼠标钩子"),
        ("Window worker panicked", "窗口工作线程异常终止"),
        (
            "Saved minimize as menu setting must be 0 or 1.",
            "保存的最小化菜单设置必须为 0 或 1。",
        ),
        (
            "Saved ignore middle mouse button setting must be 0 or 1.",
            "保存的忽略鼠标中键设置必须为 0 或 1。",
        ),
        (
            "Saved transparency must be between 0 and 100 in steps of 10.",
            "保存的透明度必须在 0 到 100 之间，且为 10 的倍数。",
        ),
        (
            "Transparency must be between 0 and 100 in steps of 10.",
            "透明度必须在 0 到 100 之间，且为 10 的倍数。",
        ),
        (
            "The executable path is too long for Windows startup. Move WinRoll RS to a shorter path.",
            "程序路径太长，无法随 Windows 启动。请将 WinRoll RS 移至更短的路径。",
        ),
    ],
};
