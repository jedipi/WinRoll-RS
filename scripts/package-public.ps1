param([string]$IsccPath = 'ISCC.exe')

$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
Get-Command $IsccPath -ErrorAction Stop | Out-Null
$zip = & (Join-Path $PSScriptRoot 'package-personal.ps1') -Destination public-release
$output = Split-Path $zip -Parent
$package = Join-Path $output ([IO.Path]::GetFileNameWithoutExtension($zip))
$version = ([regex]::Match((Split-Path $package -Leaf), '^winroll-(.+)-x64$')).Groups[1].Value
& $IsccPath "/DAppVersion=$version" "/DPackageDir=$package" "/DOutputDir=$output" (Join-Path $repo 'installer\winroll.iss')
if ($LASTEXITCODE -ne 0) { throw "Installer compilation failed: $LASTEXITCODE" }
$assets = @($zip, "$package-setup.exe")
$assets | ForEach-Object {
    $hash = (Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $(Split-Path $_ -Leaf)"
} | Set-Content -LiteralPath (Join-Path $output 'SHA256SUMS.txt') -Encoding ascii
$assets
