param()
$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
$release = Join-Path $repo 'target\release\winroll.exe'
if (!(Test-Path -LiteralPath $release)) { throw 'Build target/release/winroll.exe first.' }
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class NativeUpdateSmoke {
    public delegate bool Callback(IntPtr h, IntPtr p);
    [DllImport("user32.dll")] static extern bool EnumWindows(Callback cb, IntPtr p);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
    public static Dictionary<int, IntPtr> Trays() {
        var windows = new Dictionary<int, IntPtr>();
        EnumWindows((h,p) => { var s = new StringBuilder(256); GetClassName(h,s,256);
            if (s.ToString() == "WinRollTray") { uint pid; GetWindowThreadProcessId(h,out pid); windows[(int)pid] = h; }
            return true; }, IntPtr.Zero);
        return windows;
    }
}
'@
if ([NativeUpdateSmoke]::Trays().Count -ne 0) {
    'SKIP: an existing WinRoll controller is active; it was left running.'
    return
}
function Wait-Until($Test, $Description) {
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    while (!(& $Test)) {
        if ([DateTime]::UtcNow -ge $deadline) { throw "Timed out: $Description" }
        Start-Sleep -Milliseconds 50
    }
}
function Registry-Snapshot {
    $values = @{}
    foreach ($path in @('Software\WinRoll RS', 'Software\Microsoft\Windows\CurrentVersion\Run')) {
        $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($path)
        $values[$path] = if ($key) {
            try {
                $names = if ($path.EndsWith('\Run')) { @('WinRoll RS') } else { @($key.GetValueNames() | Sort-Object) }
                @($names | ForEach-Object {
                    $value = $key.GetValue($_, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
                    if ($null -ne $value) { @{ Name = $_; Kind = [string]$key.GetValueKind($_); Value = $value } }
                })
            } finally { $key.Dispose() }
        } else { $null }
    }
    $values | ConvertTo-Json -Depth 5 -Compress
}
$before = Registry-Snapshot
$root = Join-Path $repo ('target\native portable smoke '' ' + [Guid]::NewGuid().ToString('N'))
$stage = Join-Path $root '.winroll-update-smoke'
New-Item -ItemType Directory -Path $stage | Out-Null
$target = Join-Path $root 'winroll.exe'
$old = $null
$worker = $null
try {
    Copy-Item -LiteralPath $release -Destination $target
    # A harmless PE overlay makes the old copy distinct without changing executable code.
    $overlay = [IO.File]::Open($target, 'Append', 'Write')
    try { $bytes = [Text.Encoding]::UTF8.GetBytes('old disposable portable copy'); $overlay.Write($bytes,0,$bytes.Length) } finally { $overlay.Dispose() }
    $oldHash = (Get-FileHash -LiteralPath $target).Hash
    $newHash = (Get-FileHash -LiteralPath $release).Hash
    $payload = Join-Path $root 'payload'
    New-Item -ItemType Directory -Path $payload | Out-Null
    Copy-Item -LiteralPath $release -Destination (Join-Path $payload 'winroll.exe')
    $archive = Join-Path $stage 'package.zip'
    Compress-Archive -LiteralPath (Join-Path $payload 'winroll.exe') -DestinationPath $archive
    $source = Get-Content -LiteralPath (Join-Path $repo 'src\updates\portable.rs') -Raw
    $helper = [regex]::Match($source, '(?s)const HELPER: &str = r#"(.*?)"#;').Groups[1].Value
    if (!$helper) { throw 'Cannot find the production portable helper.' }
    $script = Join-Path $stage 'install.ps1'
    Set-Content -LiteralPath $script -Value $helper -Encoding UTF8
    $old = Start-Process -FilePath $target -PassThru -WindowStyle Hidden
    Wait-Until { [NativeUpdateSmoke]::Trays().ContainsKey($old.Id) } 'old WinRoll tray startup'
    $start = [Diagnostics.ProcessStartInfo]::new('powershell.exe')
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    foreach ($arg in @('-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',$script,$target,$archive,$stage,[string]$old.Id,'Native update smoke error','Previous version restored.','Previous executable saved at:')) { $start.ArgumentList.Add($arg) }
    $worker = [Diagnostics.Process]::Start($start)
    Wait-Until { Test-Path -LiteralPath (Join-Path $stage 'ready') } 'helper readiness'
    if ((Get-FileHash -LiteralPath $target).Hash -ne $oldHash) { throw 'The old running image was changed before Exit.' }
    if (![NativeUpdateSmoke]::PostMessage([NativeUpdateSmoke]::Trays()[$old.Id], 0x10, [IntPtr]::Zero, [IntPtr]::Zero)) { throw 'Cannot request normal Exit on the old tray.' }
    Wait-Until { $old.HasExited } 'normal old Exit and mutex release'
    $script:replacement = $null
    Wait-Until {
        foreach ($entry in [NativeUpdateSmoke]::Trays().GetEnumerator()) {
            if ($entry.Key -ne $old.Id) {
                $candidate = Get-Process -Id $entry.Key -ErrorAction SilentlyContinue
                if ($candidate -and $candidate.Path -eq $target) { $script:replacement = $candidate; return $true }
            }
        }
        $false
    } 'replacement WinRoll tray startup'
    Wait-Until { $worker.HasExited } 'helper cleanup'
    if ((Get-FileHash -LiteralPath $target).Hash -ne $newHash) { throw 'The actual WinRoll replacement hash is incorrect.' }
    if (Test-Path -LiteralPath $stage) { throw 'Successful native update left staging behind.' }
    if ((Registry-Snapshot) -ne $before) { throw 'Preferences or startup registration changed.' }
    [NativeUpdateSmoke]::PostMessage([NativeUpdateSmoke]::Trays()[$replacement.Id], 0x10, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
    Wait-Until { $replacement.HasExited } 'normal replacement Exit'
    'PASS: actual WinRoll portable replacement, normal Exit, mutex release, automatic tray restart, preserved preferences/startup registration, and cleanup.'
} finally {
    # Only ask disposable controllers to exit normally; never terminate managed window state.
    foreach ($entry in [NativeUpdateSmoke]::Trays().GetEnumerator()) {
        $candidate = Get-Process -Id $entry.Key -ErrorAction SilentlyContinue
        if ($candidate -and $candidate.Path -eq $target) {
            [NativeUpdateSmoke]::PostMessage($entry.Value, 0x10, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
            if (!$candidate.WaitForExit(5000)) { throw "Disposable WinRoll still needs recovery; preserved $root" }
        }
    }
    if ($worker -and !$worker.HasExited) { $worker.Kill(); $worker.WaitForExit() }
    $resolved = [IO.Path]::GetFullPath($root)
    if (!$resolved.StartsWith([IO.Path]::GetFullPath((Join-Path $repo 'target')) + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe smoke cleanup path.' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
