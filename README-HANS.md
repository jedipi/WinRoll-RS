[English](README.md) | [繁體中文](README-HANT.md) | [简体中文](README-HANS.md)

# WinRoll RS

WinRoll RS 用 Rust 重新打造 WinRoll 2.0，适用于 Windows 10 和 Windows 11。

欢迎访问 [WinRoll RS 网站](https://jedipi.github.io/WinRoll-RS/)，了解它的操作方式。

[![下载 WinRoll RS](https://img.shields.io/badge/Download-WinRoll%20RS-2563eb?style=for-the-badge)](https://github.com/jedipi/WinRoll-RS/releases/latest)

最新版本 v1.0.0 的直接下载链接：

- [便携版 (ZIP, x64)](https://github.com/jedipi/WinRoll-RS/releases/download/v1.0.0/winroll-1.0.0-x64-portable.zip)
- [安装程序 (EXE, x64)](https://github.com/jedipi/WinRoll-RS/releases/download/v1.0.0/winroll-1.0.0-x64-setup.exe)

WinRoll 能像卷起窗帘一样，把窗口收起来，只留下标题栏。它还提供几项实用的窗口管理功能：窗口置顶、移至最底层、调整透明度，以及最小化到系统托盘。

我希望延续原版 WinRoll 简单直观的操作体验，并采用现代 Windows API，让代码更容易维护。

## 背景

2000 年代初期，我非常喜欢一款叫作 WinRoll 的 Windows 小工具。

它的核心功能既简单又实用：

> 右键单击窗口标题栏，窗口就会卷起，只留下标题栏。

原版 WinRoll 由 Wil Palma 开发，并于 2004 年推出 2.0 版。

这个项目后来停止开发。之后，FelixHuoEZ 将源代码上传到 GitHub：

https://github.com/FelixHuoEZ/winroll

原版几乎全部使用 32 位 x86 汇编语言编写，使用 MASM，针对的是年代久远的 Windows 版本。

我重新打造这款工具，让它适用于现代 Windows，同时保持小巧、不打扰日常操作的特色。

## 目标

我希望 WinRoll RS 能做到：

- 支持 Windows 10 x64
- 支持 Windows 11 x64
- 以轻量级工具的形式在后台运行
- 延续原版 WinRoll 的使用体验
- 尽可能同时支持对 32 位和 64 位应用程序进行操作
- 正确处理现代 Windows 的 DPI 缩放
- 支持多显示器环境
- 尽可能避免使用 DLL 注入
- 采用现代 Win32 API
- 将内存与 CPU 占用保持在极低水平

本项目不支持 Windows XP、Vista、7、8，也不支持 32 位 Windows。

## 核心功能

### 卷起 / 展开

右键单击窗口标题栏，即可卷起窗口；再单击一次即可还原。

普通窗口：

![正常大小的文件资源管理器](assets/screenshots/normal-window.png)

卷起后的窗口：

![卷起后只留下标题栏的文件资源管理器](assets/screenshots/rolled-window.png)

WinRoll 会记住窗口原本的大小与位置，让窗口展开时能准确恢复原状。

### 窗口置顶

用鼠标中键单击窗口的 **关闭 (X)** 按钮，即可开启窗口置顶；再单击一次
即可关闭。通过系统托盘退出 WinRoll RS 时，凡是通过这个操作更改置顶状态的窗口，
都会恢复原本的置顶设置。“暂停”和“全部展开”
不会影响窗口置顶状态。

### 移至最底层

右键单击窗口的 **关闭 (X)** 按钮，即可把窗口移到其他窗口后面，
而不会将它最小化，也不会改变它的大小或位置。

### 透明度

用鼠标中键单击标题栏的空白处，即可应用设置好的透明度；再单击一次，
即可恢复窗口原本的外观。“暂停”会保留当前的外观，
并禁用这项鼠标操作；“全部展开”不会影响透明度。

透明度设为 100% 时，窗口会完全不可见，也无法点击。请从
WinRoll RS 系统托盘菜单中选择 **退出**，让受影响的窗口在程序退出前恢复原状。使用
逐像素分层渲染的应用程序不适用此功能。

### 最小化到系统托盘

用鼠标中键单击窗口的 **最小化** 按钮，即可将窗口从桌面和任务栏隐藏，
并按照“选项”中的设置，以以下方式收纳：

- **以图标显示** (默认) 会为每个窗口创建一个通知区域图标。单击
  图标即可还原窗口；如果找不到，请查看通知区域中的隐藏图标。
- **以菜单显示** 会将窗口标题列在 WinRoll 系统托盘菜单的 **已最小化** 子菜单中。
  单击标题即可还原窗口。没有任何窗口时，子菜单会显示 **(无)** 。

### 鼠标操作

延续 WinRoll 经典的操作方式。

| 操作 | 鼠标操作 |
|---|---|
| 卷起 / 展开 | 右键单击标题栏 |
| 透明度 | 中键单击标题栏 |
| 窗口置顶 | 中键单击关闭按钮 |
| 移至最底层 | 右键单击关闭按钮 |
| 最小化到系统托盘 | 中键单击最小化按钮 |

### 多语言支持

支持多种语言

如果你想添加语言或改进现有翻译，请参阅 [贡献翻译](src/localization/README.md)。
每种语言都有独立的源代码文件，并会一并编译到可执行文件中。

### 检查更新

WinRoll RS 会在启动时静默检查新的稳定版本。在托盘菜单中选择 **检查更新** 可手动检查；
发现新版本时，菜单会显示 **有可用更新**。对话框会显示可用版本或 **已是最新版本。**，
手动检查支持重试和取消。下载和安装更新功能仍在规划中。

### 后续改进

其他规划中的功能包括：

- 指定不应用功能的应用程序
- 可自行选择是否启用的动画
- 便携式配置
- 自动更新支持

## 不在开发范围内的项目

WinRoll RS 定位为小巧的窗口工具，并不打算发展成完整的桌面窗口管理器，也不会取代以下工具或功能：

- Windows Snap
- PowerToys FancyZones
- 虚拟桌面管理器
- 平铺式窗口管理器

本项目以经典的窗口卷起功能为核心，专注于提供简单、实用的窗口管理操作。

## 原版 WinRoll

原版 WinRoll 的源代码镜像：

https://github.com/FelixHuoEZ/winroll

原版应用程序：WinRoll 2.0

版权所有 © 2003–2004 Wil Palma。

原始项目采用宽松的许可条款，允许修改及再分发，但必须遵守以下条件：

1. 不得对原始软件的来源作出不实陈述。
2. 修改过的版本必须清楚标注，让人知道这是经过修改的版本。
3. 必须保留原始许可声明。

WinRoll RS 是受 WinRoll 启发、为现代环境重新打造的版本，并非原版 WinRoll 2.0。

## 致谢

感谢 Wil Palma 开发了原版 WinRoll。

习惯了卷起窗口的便利之后，就很难再舍弃这项功能。

也感谢 FelixHuoEZ 将 WinRoll 的源代码保存在 GitHub 上。
