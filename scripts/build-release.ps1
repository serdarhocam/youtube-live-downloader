param(
    [ValidateSet('portable', 'installer', 'all')]
    [string]$PackageType = 'all'
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if ((Test-Path -LiteralPath (Join-Path $cargoBin 'cargo.exe')) -and (($env:Path -split ';') -notcontains $cargoBin)) {
    $env:Path = "$cargoBin;$env:Path"
}

# One sequence for every distribution; reserve numbers even if a build fails.
$lockPath = Join-Path $projectRoot '.release-build.lock'
try { $buildLock = [System.IO.File]::Open($lockPath, 'OpenOrCreate', 'ReadWrite', 'None') }
catch { throw 'Another release build is running. Wait for it to finish.' }
try {
    $version = (Get-Content -Raw (Join-Path $projectRoot 'package.json') | ConvertFrom-Json).version
    $tauriVersion = (Get-Content -Raw (Join-Path $projectRoot 'src-tauri\tauri.conf.json') | ConvertFrom-Json).version
    $cargo = Get-Content -Raw (Join-Path $projectRoot 'src-tauri\Cargo.toml')
    $cargoVersion = [regex]::Match($cargo, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
    if ($version -ne $tauriVersion -or $version -ne $cargoVersion) { throw 'Package, Tauri and Cargo versions must match.' }
    if ($version -notmatch '^\d+\.\d+\.\d+([-.][0-9A-Za-z.-]+)?$') { throw 'Invalid release version.' }
    $sequencePath = Join-Path $PSScriptRoot 'package-sequence.json'
    $sequence = Get-Content -Raw $sequencePath | ConvertFrom-Json
    $sequence.lastPackage = [int]$sequence.lastPackage + 1
    [System.IO.File]::WriteAllText($sequencePath, ($sequence | ConvertTo-Json) + "`n", [System.Text.UTF8Encoding]::new($false))
    $packageId = 'p{0:D4}' -f $sequence.lastPackage
    $stem = "YouTubeLiveDownloader-v$version-$packageId"
    Write-Output "Building version $version, package $packageId ($PackageType)"
    $started = Get-Date
    if ($PackageType -eq 'portable') { npx tauri build --no-bundle }
    else { npx tauri build --bundles nsis }
    if ($LASTEXITCODE -ne 0) { throw "Release build failed with exit code $LASTEXITCODE. Package $packageId remains reserved." }

    function Export-Package([string]$Source, [string]$Kind) {
        $directory = Join-Path $projectRoot $Kind
        New-Item -ItemType Directory -Path $directory -Force | Out-Null
        $name = "$stem-$Kind.exe"
        $destination = Join-Path $directory $name
        if (Test-Path -LiteralPath $destination) { throw "Package already exists: $destination" }
        Copy-Item -LiteralPath $Source -Destination $destination
        $stream = [System.IO.File]::OpenRead($destination)
        $sha = [System.Security.Cryptography.SHA256]::Create()
        try { $hash = ([System.BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-', '') }
        finally { $sha.Dispose(); $stream.Dispose() }
        [System.IO.File]::WriteAllText("$destination.sha256", "$hash  $name`r`n", [System.Text.UTF8Encoding]::new($false))
        Write-Output "Package: $destination"
        Write-Output "SHA-256: $hash"
    }
    if ($PackageType -in @('portable', 'all')) {
        Export-Package (Join-Path $projectRoot 'src-tauri\target\release\youtube-live-downloader.exe') 'portable'
    }
    if ($PackageType -in @('installer', 'all')) {
        $installers = @(Get-ChildItem -LiteralPath (Join-Path $projectRoot 'src-tauri\target\release\bundle\nsis') -Filter '*.exe' |
            Where-Object { $_.LastWriteTime -ge $started })
        if ($installers.Count -ne 1) { throw 'Expected exactly one freshly built NSIS installer.' }
        Export-Package $installers[0].FullName 'installer'
    }
}
finally { $buildLock.Dispose() }
