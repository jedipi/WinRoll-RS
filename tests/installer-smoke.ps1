param([string]$SetupPath)

$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
if (!$SetupPath) {
    $version = (Select-String -LiteralPath (Join-Path $repo 'Cargo.toml') -Pattern '^version = "([^"]+)"$').Matches.Groups[1].Value
    $SetupPath = Join-Path $repo "target\public-release\winroll-$version-x64-setup.exe"
}
$SetupPath = (Resolve-Path -LiteralPath $SetupPath).Path
$installDir = Join-Path $repo ("target\installer-smoke-" + [guid]::NewGuid().ToString('N'))
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{4A9106DA-A87A-4CD2-8BD8-2177786DA580}_is1'
$menuDir = Join-Path ([Environment]::GetFolderPath('StartMenu')) 'Programs\WinRoll RS'
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if ($principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run this test without administrator privileges.'
}
if ((Test-Path -LiteralPath $uninstallKey) -or (Test-Path -LiteralPath $menuDir)) {
    throw 'An existing WinRoll RS installation or Start menu folder must not be changed by this test.'
}
$running = $null
if ([Threading.Mutex]::TryOpenExisting('Local\WinRoll-RS.Experiment', [ref]$running)) {
    $running.Dispose()
    throw 'Exit WinRoll RS through its tray before testing the installer.'
}
function Invoke-Installer([string]$Path, [string[]]$Arguments) {
    $process = Start-Process -FilePath $Path -ArgumentList $Arguments -WindowStyle Hidden -Wait -PassThru
    return $process.ExitCode
}
$quiet = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
$setupArgs = $quiet + @('/SP-', "/DIR=`"$installDir`"")
$uninstaller = Join-Path $installDir 'unins000.exe'
try {
    $mutex = [Threading.Mutex]::new($false, 'Local\WinRoll-RS.Experiment')
    try {
        if ((Invoke-Installer $SetupPath $setupArgs) -eq 0 -or (Test-Path -LiteralPath $uninstallKey)) {
            throw 'Setup did not reject the running-app mutex.'
        }
    } finally { $mutex.Dispose() }
    if ((Invoke-Installer $SetupPath $setupArgs) -ne 0) { throw 'Installation failed.' }
    $entry = Get-ItemProperty -LiteralPath $uninstallKey
    if ($entry.DisplayName -notlike 'WinRoll RS*' -or $entry.InstallLocation.TrimEnd('\') -ne $installDir) {
        throw 'Per-user uninstall registration is incorrect.'
    }
    foreach ($name in @('WinRoll RS.lnk', 'Uninstall WinRoll RS.lnk')) {
        if (!(Test-Path -LiteralPath (Join-Path $menuDir $name))) { throw "Missing shortcut: $name" }
    }
    foreach ($line in Get-Content -LiteralPath (Join-Path $installDir 'SHA256SUMS.txt')) {
        $expected, $name = $line -split '  ', 2
        if ((Get-FileHash -LiteralPath (Join-Path $installDir $name)).Hash.ToLowerInvariant() -ne $expected) {
            throw "Installed file hash mismatch: $name"
        }
    }
    $selfTest = Start-Process -FilePath (Join-Path $installDir 'winroll.exe') -ArgumentList '--self-test' -WindowStyle Hidden -Wait -PassThru -RedirectStandardOutput "$installDir-self-test.log" -RedirectStandardError "$installDir-self-test-error.log"
    if ($selfTest.ExitCode -ne 0) { throw 'Installed executable native self-test failed.' }
    if ((Invoke-Installer $SetupPath $setupArgs) -ne 0) { throw 'Reinstallation failed.' }
    $mutex = [Threading.Mutex]::new($false, 'Local\WinRoll-RS.Experiment')
    try {
        if ((Invoke-Installer $uninstaller $quiet) -eq 0 -or !(Test-Path -LiteralPath (Join-Path $installDir 'winroll.exe'))) {
            throw 'Uninstall did not preserve files while the running-app mutex exists.'
        }
    } finally { $mutex.Dispose() }
    if ((Invoke-Installer $uninstaller $quiet) -ne 0) { throw 'Uninstall failed.' }
    if ((Test-Path -LiteralPath $uninstallKey) -or (Test-Path -LiteralPath $menuDir) -or
        (Test-Path -LiteralPath (Join-Path $installDir 'winroll.exe'))) {
        throw 'Uninstall left application files, shortcuts or registration behind.'
    }
    Write-Output 'PASS: per-user install, file hashes, native self-test, shortcuts, reinstall, running-app guards and uninstall cleanup.'
} finally {
    if (Test-Path -LiteralPath $uninstaller) {
        Invoke-Installer $uninstaller $quiet | Out-Null
    }
}
