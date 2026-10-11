use super::StagedPackage;
use std::{
    fs,
    os::windows::process::CommandExt,
    process::Command,
    thread,
    time::{Duration, Instant},
};

pub(super) fn prepare(package: StagedPackage) -> Result<(), String> {
    if super::install_mode::installed()? {
        return Err("Installed updates are not supported yet.".into());
    }
    let target = std::env::current_exe().map_err(|e| e.to_string())?;
    let directory = tempfile::Builder::new()
        .prefix(".winroll-update-")
        .tempdir_in(
            target
                .parent()
                .ok_or("Cannot locate the executable directory.")?,
        )
        .map_err(|e| e.to_string())?;
    let archive = directory.path().join("package.zip");
    fs::copy(&package.path, &archive).map_err(|e| e.to_string())?;
    let script = directory.path().join("install.ps1");
    fs::write(&script, HELPER).map_err(|e| e.to_string())?;
    let messages = HELPER_ERRORS
        .iter()
        .map(|key| format!("{key}\t{}\n", crate::localization::text(key)))
        .collect::<String>();
    fs::write(directory.path().join("messages.txt"), messages).map_err(|e| e.to_string())?;
    let powershell = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .ok_or("Cannot locate Windows PowerShell.")?
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let mut child = Command::new(powershell)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(script)
        .arg(&target)
        .arg(&archive)
        .arg(directory.path())
        .arg(std::process::id().to_string())
        .arg(crate::localization::text("Portable update failed."))
        .arg(crate::localization::text(
            "The previous version was restored. Please try the update again.",
        ))
        .arg(crate::localization::text(
            "The previous executable is saved at:",
        ))
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
        .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(30);
    let result = loop {
        if directory.path().join("ready").is_file() {
            break Ok(());
        }
        if let Ok(error) = fs::read_to_string(directory.path().join("error")) {
            break Err(error);
        }
        match child.try_wait() {
            Ok(Some(_)) => break Err("Portable update helper stopped unexpectedly.".into()),
            Err(error) => break Err(error.to_string()),
            Ok(None) => {}
        }
        if Instant::now() >= deadline {
            break Err("Portable update preparation timed out.".into());
        }
        thread::sleep(Duration::from_millis(20));
    };
    if result.is_ok() {
        // The helper owns cleanup after handoff; the caller may now finish the recovered Exit.
        let _ = directory.keep();
    } else {
        // Stop the waiting helper before reporting failure, so a later Exit cannot install.
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

const HELPER_ERRORS: &[&str] = &[
    "The portable staging directory must be an update directory beside the target.",
    "The portable package must contain exactly one root winroll.exe.",
    "The portable package does not contain a Windows executable.",
    "The portable package does not contain an AMD64 PE32+ executable.",
    "The updated executable exited before startup completed.",
    "The updated executable did not confirm startup.",
];

const HELPER: &str = r#"param($Target, $Archive, $Stage, [int]$ParentId, $ErrorTitle, $Restored, $SavedAt)
$ErrorActionPreference = 'Stop'
$ready = $false
$preserve = $false
$backup = Join-Path $Stage 'previous.exe'
$source = Join-Path $Stage 'winroll.exe'
$status = Join-Path $Stage 'startup'
$next = $null
$messages = @{}
$catalog = Join-Path $Stage 'messages.txt'
if ([IO.File]::Exists($catalog)) {
    foreach ($line in [IO.File]::ReadAllLines($catalog)) {
        $pair = $line.Split([char]9, 2)
        if ($pair.Count -eq 2) { $messages[$pair[0]] = $pair[1] }
    }
}
function Error-Text($message) {
    if ($messages.ContainsKey($message)) { return $messages[$message] }
    return $message
}
try {
    $Stage = [IO.Path]::GetFullPath($Stage)
    $Target = [IO.Path]::GetFullPath($Target)
    if ([IO.Path]::GetDirectoryName($Stage) -ne [IO.Path]::GetDirectoryName($Target) -or
        ![IO.Path]::GetFileName($Stage).StartsWith('.winroll-update-', [StringComparison]::Ordinal)) {
        throw 'The portable staging directory must be an update directory beside the target.'
    }
    # Hold the process handle before acknowledging readiness, avoiding PID reuse after Exit.
    $parent = [Diagnostics.Process]::GetProcessById($ParentId)
    $null = $parent.Handle
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($Archive)
    try {
        $entries = @($zip.Entries | Where-Object { $_.FullName -ceq 'winroll.exe' })
        if ($entries.Count -ne 1) { throw 'The portable package must contain exactly one root winroll.exe.' }
        [IO.Compression.ZipFileExtensions]::ExtractToFile($entries[0], $source)
    } finally { $zip.Dispose() }
    $bytes = [IO.File]::ReadAllBytes($source)
    if ($bytes.Length -lt 64 -or $bytes[0] -ne 0x4d -or $bytes[1] -ne 0x5a) {
        throw 'The portable package does not contain a Windows executable.'
    }
    $pe = [BitConverter]::ToInt32($bytes, 0x3c)
    if ($pe -lt 64 -or $pe -gt $bytes.Length - 26 -or
        [BitConverter]::ToUInt32($bytes, $pe) -ne 0x4550 -or
        [BitConverter]::ToUInt16($bytes, $pe + 4) -ne 0x8664 -or
        [BitConverter]::ToUInt16($bytes, $pe + 24) -ne 0x20b) {
        throw 'The portable package does not contain an AMD64 PE32+ executable.'
    }
    [IO.File]::WriteAllText((Join-Path $Stage 'ready'), 'ready')
    $ready = $true
    $parent.WaitForExit()
    $parent.Dispose()
    # The process has exited, including closure of the single-instance mutex and image handle.
    try {
        [IO.File]::Move($Target, $backup)
        [IO.File]::Move($source, $Target)
        $env:WINROLL_UPDATE_STATUS = $status
        try {
            $next = Start-Process -FilePath $Target -WorkingDirectory ([IO.Path]::GetDirectoryName($Target)) -WindowStyle Hidden -PassThru
        } finally { Remove-Item Env:\WINROLL_UPDATE_STATUS }
        $deadline = [DateTime]::UtcNow.AddSeconds(30)
        while ($true) {
            if ([IO.File]::Exists($status)) {
                $result = [IO.File]::ReadAllText($status)
                if ($result -eq 'ready') { break }
                if ($result -eq 'failed') {
                    # Initialization failed before window management began; dismiss its error dialog.
                    if (!$next.HasExited) { $next.Kill(); $next.WaitForExit() }
                    throw 'The updated executable exited before startup completed.'
                }
            }
            if ($next.WaitForExit(50)) { throw 'The updated executable exited before startup completed.' }
            if ([DateTime]::UtcNow -ge $deadline) { throw 'The updated executable did not confirm startup.' }
        }
    } catch {
        $failure = Error-Text $_.Exception.Message
        # An unconfirmed live process may own window state; keep its backup without terminating it.
        if ($next -and !$next.HasExited) {
            $preserve = $true
            throw "$failure`n$SavedAt $backup"
        }
        try {
            if ([IO.File]::Exists($backup)) {
                if ([IO.File]::Exists($Target)) { [IO.File]::Delete($Target) }
                [IO.File]::Move($backup, $Target)
            }
            Start-Process -FilePath $Target -WorkingDirectory ([IO.Path]::GetDirectoryName($Target)) -WindowStyle Hidden
        } catch {
            $preserve = $true
            $previous = if ([IO.File]::Exists($backup)) { $backup } else { $Target }
            throw "$failure`n$SavedAt $previous`n$($_.Exception.Message)"
        }
        throw "$Restored`n$failure"
    }
} catch {
    $message = Error-Text $_.Exception.Message
    if (!$ready) {
        [IO.File]::WriteAllText((Join-Path $Stage 'error'), $message)
    } else {
        [IO.File]::WriteAllText((Join-Path $Stage 'error'), $message)
        Add-Type -AssemblyName System.Windows.Forms
        [Windows.Forms.MessageBox]::Show($message, $ErrorTitle, 'OK', 'Error') | Out-Null
    }
} finally {
    if ($ready -and !$preserve) { Remove-Item -LiteralPath $Stage -Recurse -Force -ErrorAction SilentlyContinue }
}
"#;
