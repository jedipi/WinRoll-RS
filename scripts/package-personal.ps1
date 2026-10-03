param(
    [ValidateSet('personal-testing', 'public-release')]
    [string]$Destination = 'personal-testing'
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
$version = (Select-String -LiteralPath (Join-Path $repo 'Cargo.toml') -Pattern '^version = "([^"]+)"$').Matches.Groups[1].Value
if (!$version) { throw 'Cannot read the package version from Cargo.toml.' }
$sysroot = rustc --print sysroot
if ($LASTEXITCODE -ne 0) { throw 'Cannot locate Rust Standard Library notices.' }
$files = @{
    'winroll.exe' = Join-Path $repo 'target\x86_64-pc-windows-msvc\release\winroll.exe'
    'README.md' = Join-Path $repo 'docs\personal-testing.md'
    'compatibility-report.md' = Join-Path $repo 'docs\compatibility-report.md'
    'THIRD-PARTY-NOTICES.txt' = Join-Path $repo 'THIRD-PARTY-NOTICES.txt'
    'RUST-COPYRIGHT.html' = Join-Path $sysroot 'share\doc\rust\COPYRIGHT-library.html'
}
if ($Destination -eq 'public-release') {
    $files.Remove('compatibility-report.md')
}
$package = Join-Path $repo "target\$Destination\winroll-$version-x64"
$zip = "$package.zip"
foreach ($source in $files.Values | Where-Object { $_ -notlike '*.exe' }) {
    if (!(Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "Missing package input: $source"
    }
}
if (Test-Path -LiteralPath $package) {
    $unexpected = Get-ChildItem -LiteralPath $package -Force |
        Where-Object { $_.PSIsContainer -or $_.Name -notin (@($files.Keys) + 'SHA256SUMS.txt', 'compatibility-report.md') }
    if ($unexpected) { throw "Unexpected files in package directory: $package" }
    if ($Destination -eq 'public-release' -and (Test-Path -LiteralPath (Join-Path $package 'compatibility-report.md'))) {
        Remove-Item -LiteralPath (Join-Path $package 'compatibility-report.md')
    }
}
Push-Location $repo
try {
    cargo build --release --locked --offline --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw "Release build failed: $LASTEXITCODE" }
    $exe = [IO.File]::ReadAllBytes($files['winroll.exe'])
    $pe = [BitConverter]::ToInt32($exe, 0x3c)
    if ([BitConverter]::ToUInt32($exe, $pe) -ne 0x4550 -or
        [BitConverter]::ToUInt16($exe, $pe + 4) -ne 0x8664 -or
        [BitConverter]::ToUInt16($exe, $pe + 24) -ne 0x20b) {
        throw 'Release executable is not AMD64 PE32+.'
    }
    New-Item -ItemType Directory -Path $package -Force | Out-Null
    foreach ($name in $files.Keys) {
        Copy-Item -LiteralPath $files[$name] -Destination (Join-Path $package $name) -Force
    }
    $files.Keys | Sort-Object | ForEach-Object {
        $hash = (Get-FileHash -LiteralPath (Join-Path $package $_) -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $_"
    } | Set-Content -LiteralPath (Join-Path $package 'SHA256SUMS.txt') -Encoding ascii
    Compress-Archive -LiteralPath (Get-ChildItem -LiteralPath $package -File).FullName -DestinationPath $zip -Force
    Write-Output $zip
} finally {
    Pop-Location
}
