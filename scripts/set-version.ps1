param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^\d+\.\d+\.\d+([-.][0-9A-Za-z.-]+)?$')]
    [string]$Version
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot

function Update-JsonVersion([string]$Path, [bool]$UpdateLockRoot = $false) {
    $fullPath = Join-Path $projectRoot $Path
    $json = Get-Content -LiteralPath $fullPath -Raw | ConvertFrom-Json
    $json.version = $Version
    if ($UpdateLockRoot -and $null -ne $json.packages -and $null -ne $json.packages.'') {
        $json.packages.''.version = $Version
    }
    $content = $json | ConvertTo-Json -Depth 100
    [System.IO.File]::WriteAllText($fullPath, "$content`r`n", [System.Text.UTF8Encoding]::new($false))
}

Update-JsonVersion 'package.json'
Update-JsonVersion 'package-lock.json' $true
Update-JsonVersion 'src-tauri\tauri.conf.json'

$cargoPath = Join-Path $projectRoot 'src-tauri\Cargo.toml'
$cargo = Get-Content -LiteralPath $cargoPath -Raw
$cargo = [regex]::Replace($cargo, '(?m)^(version\s*=\s*")[^"]+("\s*)$', "`${1}$Version`${2}", 1)
[System.IO.File]::WriteAllText($cargoPath, $cargo, [System.Text.UTF8Encoding]::new($false))

Write-Output "Project version updated to $Version."
Write-Output 'Run npm install --package-lock-only, checks, and npm run portable before publishing.'
