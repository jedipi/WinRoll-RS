use super::Language;

pub(super) const LANGUAGE: Language = Language {
    id: 2,
    name: "简体中文",
    windows_ids: &[0x0004, 0x0804, 0x1004],
    messages: &[
        ("Close", "关闭"),
        ("Version {version}", "版本 {version}"),
        ("Created by Kin Tam", "作者：Kin Tam"),
        ("About WinRoll RS", "关于 WinRoll RS"),
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
