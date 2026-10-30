param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^\d+\.\d+\.\d+([-.][0-9A-Za-z.-]+)?$')]
    [string]$Version
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot

function Update-JsonVersion([string]$Path) {
    $fullPath = Join-Path $projectRoot $Path
    $json = Get-Content -LiteralPath $fullPath -Raw | ConvertFrom-Json
    $json.version = $Version
    $content = $json | ConvertTo-Json -Depth 100
    [System.IO.File]::WriteAllText($fullPath, "$content`r`n", [System.Text.UTF8Encoding]::new($false))
}

Push-Location -LiteralPath $projectRoot
try {
    # npm handles the empty root-package key in lockfiles on Windows PowerShell 5.1.
    & npm version $Version --no-git-tag-version --allow-same-version --ignore-scripts
    if ($LASTEXITCODE -ne 0) { throw 'Could not synchronize npm package versions.' }
}
finally { Pop-Location }
Update-JsonVersion 'src-tauri\tauri.conf.json'

$cargoPath = Join-Path $projectRoot 'src-tauri\Cargo.toml'
$cargo = Get-Content -LiteralPath $cargoPath -Raw
$cargo = [regex]::Replace($cargo, '(?m)^(version\s*=\s*")[^"]+("\s*)$', "`${1}$Version`${2}", 1)
[System.IO.File]::WriteAllText($cargoPath, $cargo, [System.Text.UTF8Encoding]::new($false))

Write-Output "Project version updated to $Version."
Write-Output 'Run npm install --package-lock-only, checks, and npm run portable before publishing.'
