param()
$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
$root = Join-Path $repo ('target\portable smoke '' ' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$source = Get-Content -LiteralPath (Join-Path $repo 'src\updates\portable.rs') -Raw
$helper = [regex]::Match($source, '(?s)const HELPER: &str = r#"(.*?)"#;').Groups[1].Value
if (!$helper) { throw 'Cannot find the production portable helper.' }
$compiler = Join-Path $env:SystemRoot 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
$processes = @()
function Wait-Until($Test, $Description, $Seconds = 15) {
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    while (!(& $Test)) {
        if ([DateTime]::UtcNow -ge $deadline) { throw "Timed out: $Description" }
        Start-Sleep -Milliseconds 50
    }
}
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class SmokeWindow {
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string c, string t);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
}
'@
try {
    foreach ($mode in @('success', 'startup', 'delayed-startup', 'unconfirmed', 'locked', 'invalid')) {
        $failure = $mode -ne 'success'
        $case = Join-Path $root $mode
        $stage = Join-Path $case '.winroll-update-smoke'
        New-Item -ItemType Directory -Path $stage | Out-Null
        $target = Join-Path $case 'winroll.exe'
        $replacement = Join-Path $stage 'new.exe'
        foreach ($kind in @('original', 'updated')) {
            $output = if ($kind -eq 'original') { $target } else { $replacement }
            $sleep = if ($mode -eq 'startup' -and $kind -eq 'updated') { '' } else { 'System.Threading.Thread.Sleep(60000);' }
            $signal = if ($kind -eq 'updated' -and $mode -eq 'success') {
                'System.Threading.Thread.Sleep(1500); File.WriteAllText(Environment.GetEnvironmentVariable("WINROLL_UPDATE_STATUS"), "ready");'
            } elseif ($kind -eq 'updated' -and $mode -eq 'delayed-startup') {
                'System.Threading.Thread.Sleep(1500); File.WriteAllText(Environment.GetEnvironmentVariable("WINROLL_UPDATE_STATUS"), "failed");'
            } else { '' }
            $code = @"
using System;
using System.IO;
class Program { static void Main() { File.AppendAllText(Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "$kind.txt"), "started\n"); $signal $sleep } }
"@
            $cs = Join-Path $case "$kind.cs"
            Set-Content -LiteralPath $cs -Value $code
            & $compiler /nologo /platform:x64 /target:winexe "/out:$output" $cs
            if ($LASTEXITCODE -ne 0) { throw 'Smoke fixture compilation failed.' }
        }
        $originalHash = (Get-FileHash -LiteralPath $target).Hash
        if ($mode -eq 'invalid') { [IO.File]::WriteAllText($replacement, 'not an executable') }
        $updatedHash = (Get-FileHash -LiteralPath $replacement).Hash
        $payload = Join-Path $case 'payload'
        New-Item -ItemType Directory -Path $payload | Out-Null
        Copy-Item -LiteralPath $replacement -Destination (Join-Path $payload 'winroll.exe')
        $archive = Join-Path $stage 'package.zip'
        Compress-Archive -LiteralPath (Join-Path $payload 'winroll.exe') -DestinationPath $archive
        $script = Join-Path $stage 'install.ps1'
        Set-Content -LiteralPath $script -Value $helper -Encoding UTF8
        if ($mode -eq 'invalid') {
            $message = "The portable package does not contain a Windows executable.`t此便携包不包含 Windows 可执行文件。`n"
            [IO.File]::WriteAllText((Join-Path $stage 'messages.txt'), $message)
        }
        $old = Start-Process -FilePath $target -PassThru -WindowStyle Hidden
        $processes += $old
        Wait-Until { Test-Path -LiteralPath (Join-Path $case 'original.txt') } 'old fixture startup'
        $start = [Diagnostics.ProcessStartInfo]::new('powershell.exe')
        $start.UseShellExecute = $false
        $start.CreateNoWindow = $true
        $start.RedirectStandardError = $true
        # Native ArgumentList preserves apostrophes, spaces and each argument boundary.
        foreach ($arg in @('-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',$script,$target,$archive,$stage,[string]$old.Id,'smoke update failure','Previous version restored.','Previous executable saved at:')) {
            $start.ArgumentList.Add($arg)
        }
        $worker = [Diagnostics.Process]::Start($start)
        $processes += $worker
        if ($mode -eq 'invalid') {
            Wait-Until { Test-Path -LiteralPath (Join-Path $stage 'error') } 'invalid package rejection'
            if (([IO.File]::ReadAllText((Join-Path $stage 'error'))) -notlike '*此便携包不包含 Windows 可执行文件。*') { throw 'The helper did not translate the invalid-package error.' }
            if ($old.HasExited -or (Test-Path -LiteralPath (Join-Path $stage 'ready')) -or
                (Get-FileHash -LiteralPath $target).Hash -ne $originalHash) { throw 'Invalid package affected the running old copy.' }
            Wait-Until { $worker.HasExited } 'rejected helper exit'
            $old.Kill(); $old.WaitForExit()
            continue
        }
        Wait-Until { Test-Path -LiteralPath (Join-Path $stage 'ready') } 'helper readiness'
        if ((Get-FileHash -LiteralPath $target).Hash -ne $originalHash) { throw 'Replacement began before old process exited.' }
        $lock = if ($mode -eq 'locked') { [IO.File]::Open($target, 'Open', 'Read', 'Read') } else { $null }
        $old.Kill()
        $old.WaitForExit()
        if ($mode -ne 'locked') { Wait-Until { Test-Path -LiteralPath (Join-Path $case 'updated.txt') } 'replacement restart' }
        if ($mode -eq 'unconfirmed') {
            Wait-Until { Test-Path -LiteralPath (Join-Path $stage 'error') } 'unconfirmed startup timeout' 45
            $backup = Join-Path $stage 'previous.exe'
            if ((Get-FileHash -LiteralPath $backup).Hash -ne $originalHash -or
                (Get-FileHash -LiteralPath $target).Hash -ne $updatedHash -or
                !(Get-Process -Name winroll -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $target })) {
                throw 'Unconfirmed startup lost its backup or terminated the live replacement.'
            }
            if ((Get-Content -LiteralPath (Join-Path $stage 'error') -Raw) -notlike "*Previous executable saved at: $backup*") {
                throw 'Unconfirmed startup did not report the preserved backup.'
            }
            [SmokeWindow]::PostMessage([SmokeWindow]::FindWindow($null, 'smoke update failure'), 0x10, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
            if (!$worker.WaitForExit(1000)) { $worker.Kill(); $worker.WaitForExit() }
        } elseif ($failure) {
            Wait-Until { (Get-Content -LiteralPath (Join-Path $case 'original.txt')).Count -eq 2 } 'rollback restart'
            if ((Get-FileHash -LiteralPath $target).Hash -ne $originalHash) { throw 'Startup failure did not restore the old executable.' }
            Wait-Until { Test-Path -LiteralPath (Join-Path $stage 'error') } 'reported update error'
            if ((Get-Content -LiteralPath (Join-Path $stage 'error') -Raw) -notlike '*Previous version restored.*') { throw 'Rollback did not report a meaningful error.' }
            [SmokeWindow]::PostMessage([SmokeWindow]::FindWindow($null, 'smoke update failure'), 0x10, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
            # Some execution desktops cannot enumerate the helper's modal error window.
            if (!$worker.WaitForExit(1000)) { $worker.Kill(); $worker.WaitForExit() }
        } elseif ((Get-FileHash -LiteralPath $target).Hash -ne $updatedHash) { throw 'The new executable was not installed.' }
        Wait-Until { $worker.HasExited } 'helper exit'
        if (!$failure -and (Test-Path -LiteralPath $stage)) { throw 'The helper did not remove staging.' }
        Get-Process -Name winroll -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $target } | Stop-Process -Force
        if ($lock) { $lock.Dispose(); $lock = $null }
    }
    'PASS: replacement waits for process exit and confirmed startup, rolls back immediate/delayed startup failures, preserves a live unconfirmed replacement and its backup, recovers locked-target errors, rejects invalid executables, and cleans successful staging (paths with spaces and apostrophes).'
} finally {
    if ($lock) { $lock.Dispose() }
    foreach ($process in $processes) { if (!$process.HasExited) { $process.Kill(); $process.WaitForExit() } }
    Get-Process -Name winroll -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path.StartsWith($root + '\', [StringComparison]::OrdinalIgnoreCase) } | Stop-Process -Force
    $resolved = [IO.Path]::GetFullPath($root)
    if (!$resolved.StartsWith([IO.Path]::GetFullPath((Join-Path $repo 'target')) + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe smoke cleanup path.' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
