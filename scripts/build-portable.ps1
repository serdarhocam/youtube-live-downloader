$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot

$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if ((Test-Path -LiteralPath (Join-Path $cargoBin 'cargo.exe')) -and (($env:Path -split ';') -notcontains $cargoBin)) {
    $env:Path = "$cargoBin;$env:Path"
}

npx tauri build --no-bundle
if ($LASTEXITCODE -ne 0) { throw "Tauri release build failed with exit code $LASTEXITCODE." }

$portableDir = Join-Path $projectRoot 'portable'
New-Item -ItemType Directory -Path $portableDir -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $projectRoot 'src-tauri\target\release\youtube-live-downloader.exe') `
    -Destination (Join-Path $portableDir 'YouTubeLiveDownloader.exe') -Force

$artifact = Get-Item -LiteralPath (Join-Path $portableDir 'YouTubeLiveDownloader.exe')
$stream = [System.IO.File]::OpenRead($artifact.FullName)
try {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try { $hash = ([System.BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-', '') }
    finally { $sha.Dispose() }
}
finally { $stream.Dispose() }
Write-Output "Portable executable: $($artifact.FullName)"
Write-Output "Size: $($artifact.Length) bytes"
Write-Output "SHA-256: $hash"
$checksumPath = Join-Path $portableDir 'SHA256SUMS.txt'
[System.IO.File]::WriteAllText($checksumPath, "$hash  YouTubeLiveDownloader.exe`r`n", [System.Text.UTF8Encoding]::new($false))
Write-Output "Checksum file: $checksumPath"
