$ErrorActionPreference = 'Stop'
& (Join-Path $PSScriptRoot 'build-release.ps1') -PackageType portable
