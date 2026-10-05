param(
    [string]$Executable = "$PSScriptRoot\..\target\release\winroll.exe",
    [switch]$Fork
)

$ErrorActionPreference = 'Stop'
$Executable = (Resolve-Path -LiteralPath $Executable).Path
$running = $null
if (![Threading.Mutex]::TryOpenExisting('Local\WinRoll-RS.Experiment', [ref]$running)) {
    throw 'Start WinRoll RS and enable gestures before running this check.'
}
$running.Dispose()

# By default drive a disposable fixture. -Fork tests only its identified Close control.
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class TopmostGesture {
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct MouseInput {
        public int X, Y; public uint Data, Flags, Time; public UIntPtr Extra;
    }
    [StructLayout(LayoutKind.Sequential)] public struct Input {
        public uint Type; public MouseInput Mouse;
    }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern int GetWindowLongW(IntPtr hwnd, int index);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out Point point);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(Point point);
    [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr hwnd, uint flags);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd, IntPtr after, int x, int y, int width, int height, uint flags);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    public static void Foreground(IntPtr hwnd) {
        if (!SetForegroundWindow(hwnd)) {
            keybd_event(0x12, 0, 0, UIntPtr.Zero);
            keybd_event(0x12, 0, 2, UIntPtr.Zero);
            SetForegroundWindow(hwnd);
        }
    }
    [DllImport("user32.dll")] public static extern IntPtr SendMessageW(IntPtr hwnd, uint msg, UIntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr hwnd, uint msg, UIntPtr w, IntPtr l);
    [DllImport("user32.dll")] static extern uint SendInput(uint count, Input[] inputs, int size);
    public static void MiddleClick() {
        var inputs = new[] {
            new Input { Mouse = new MouseInput { Flags = 0x20 } },
            new Input { Mouse = new MouseInput { Flags = 0x40 } }
        };
        if (SendInput(2, inputs, Marshal.SizeOf(typeof(Input))) != 2)
            throw new Exception("Cannot send the fixture middle-click.");
    }
}
'@

$cursor = [TopmostGesture+Point]::new()
[void][TopmostGesture]::GetCursorPos([ref]$cursor)
$foreground = [TopmostGesture]::GetForegroundWindow()
$fixture = $null
if (!$Fork) {
    $fixture = Start-Process -FilePath $Executable -ArgumentList '--fixture' -WindowStyle Hidden -PassThru
}
$hwnd = [IntPtr]::Zero
$initialTopmost = $null
try {
    if ($Fork) {
        $hwnd = (Get-Process Fork).MainWindowHandle
    } else {
        $deadline = [DateTime]::UtcNow.AddSeconds(3)
        do {
            $fixture.Refresh()
            $hwnd = $fixture.MainWindowHandle
            if ($hwnd -ne [IntPtr]::Zero) { break }
            Start-Sleep -Milliseconds 20
        } while ([DateTime]::UtcNow -lt $deadline)
    }
    if ($hwnd -eq [IntPtr]::Zero) { throw 'Fixture did not open.' }
    $rect = [TopmostGesture+Rect]::new()
    if (![TopmostGesture]::GetWindowRect($hwnd, [ref]$rect)) { throw 'Fixture rectangle unavailable.' }
    $point = [TopmostGesture+Point]::new()
    if ($Fork) {
        # Run -Fork with Windows PowerShell for its built-in UI Automation assemblies.
        Add-Type -AssemblyName UIAutomationClient
        Add-Type -AssemblyName UIAutomationTypes
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
        $condition = [System.Windows.Automation.PropertyCondition]::new(
            [System.Windows.Automation.AutomationElement]::AutomationIdProperty, 'PART_CloseButton')
        $close = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
        if (!$close) { throw 'Fork Close control unavailable.' }
        $bounds = $close.Current.BoundingRectangle
        $point.X = [int]($bounds.Left + $bounds.Width / 2)
        $point.Y = [int]($bounds.Top + $bounds.Height / 2)
        [TopmostGesture]::Foreground($hwnd)
        Start-Sleep -Milliseconds 100
    } else {
        $point.X = $rect.Right - 25
        $point.Y = $rect.Top + 18
    }
    $packed = [IntPtr](($point.Y -shl 16) -bor ($point.X -band 0xffff))
    if (!$Fork -and [TopmostGesture]::SendMessageW($hwnd, 0x84, [UIntPtr]::Zero, $packed).ToInt64() -ne 20) {
        throw 'Fixture point is not the Close button.'
    }
    [void][TopmostGesture]::SetCursorPos($point.X, $point.Y)
    $initialTopmost = ([TopmostGesture]::GetWindowLongW($hwnd, -20) -band 8) -ne 0
    # An even number of clicks returns the target to its original state.
    foreach ($expected in @(!$initialTopmost, $initialTopmost)) {
        if ([TopmostGesture]::GetAncestor([TopmostGesture]::WindowFromPoint($point), 2) -ne $hwnd) {
            throw 'Target Close button is obscured; no click sent.'
        }
        [TopmostGesture]::MiddleClick()
        $deadline = [DateTime]::UtcNow.AddSeconds(2)
        do {
            $topmost = ([TopmostGesture]::GetWindowLongW($hwnd, -20) -band 8) -ne 0
            if ($topmost -eq $expected) { break }
            Start-Sleep -Milliseconds 20
        } while ([DateTime]::UtcNow -lt $deadline)
        if ($topmost -ne $expected) { throw "Middle-click failed: expected Always on Top=$expected." }
        $current = [TopmostGesture+Rect]::new()
        [void][TopmostGesture]::GetWindowRect($hwnd, [ref]$current)
        if ($current.Left -ne $rect.Left -or $current.Top -ne $rect.Top -or
            $current.Right -ne $rect.Right -or $current.Bottom -ne $rect.Bottom) {
            throw 'Middle-click changed window geometry.'
        }
        Write-Output "PASS middle-click Close: Always on Top=$expected"
        Start-Sleep -Milliseconds 100
    }
    if ($Fork) {
        foreach ($id in @('PART_MinimizeButton', 'PART_MaximizeButton', 'FeedbackButton')) {
            $condition = [System.Windows.Automation.PropertyCondition]::new(
                [System.Windows.Automation.AutomationElement]::AutomationIdProperty, $id)
            $button = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
            if (!$button) { throw "Fork control $id unavailable." }
            $bounds = $button.Current.BoundingRectangle
            $point.X = [int]($bounds.Left + $bounds.Width / 2)
            $point.Y = [int]($bounds.Top + $bounds.Height / 2)
            if ([TopmostGesture]::GetAncestor([TopmostGesture]::WindowFromPoint($point), 2) -ne $hwnd) {
                throw 'Target control is obscured; no click sent.'
            }
            [void][TopmostGesture]::SetCursorPos($point.X, $point.Y)
            [TopmostGesture]::MiddleClick()
            Start-Sleep -Milliseconds 150
            if ((([TopmostGesture]::GetWindowLongW($hwnd, -20) -band 8) -ne 0) -ne $initialTopmost) {
                throw "Fork control $id incorrectly toggled Always on Top."
            }
            Write-Output "PASS Fork $id does not toggle Always on Top"
        }
    }
} finally {
    if ($Fork -and $null -ne $initialTopmost) {
        $currentTopmost = ([TopmostGesture]::GetWindowLongW($hwnd, -20) -band 8) -ne 0
        if ($currentTopmost -ne $initialTopmost) {
            $after = if ($initialTopmost) { [IntPtr](-1) } else { [IntPtr](-2) }
            [void][TopmostGesture]::SetWindowPos($hwnd, $after, 0, 0, 0, 0, 0x213)
        }
    }
    if ($fixture -and $hwnd -ne [IntPtr]::Zero) {
        [void][TopmostGesture]::PostMessageW($hwnd, 0x10, [UIntPtr]::Zero, [IntPtr]::Zero)
        [void]$fixture.WaitForExit(3000)
    }
    [void][TopmostGesture]::SetCursorPos($cursor.X, $cursor.Y)
    [void][TopmostGesture]::SetForegroundWindow($foreground)
}
