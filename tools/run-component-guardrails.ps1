param(
    [string]$DonorRoot = "",
    [switch]$WithSnapshots
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if ($DonorRoot -eq "") {
    $DonorRoot = Join-Path $Repo "vendor\longbridge-gpui-component-ui-src"
}
if (-not (Test-Path $DonorRoot)) {
    throw "Longbridge donor root is missing: $DonorRoot"
}

function Run-Step {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [scriptblock]$Body
    )

    Write-Host ""
    Write-Host "== $Name =="
    & $Body
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
}

Run-Step "cargo fmt" {
    cargo fmt --all -- --check
}

Run-Step "gallery check" {
    cargo check -p moon-ui-gallery
}

Run-Step "component tests" {
    cargo test -p moon-ui-components
}

Run-Step "gallery tests" {
    cargo test -p moon-ui-gallery
}

Run-Step "component audit" {
    cargo xtask component-audit --check-baseline
}

Run-Step "component API" {
    cargo xtask component-api --check-baseline
}

Run-Step "component mirror" {
    cargo xtask component-mirror --donor-root $DonorRoot --check-baseline
}

if ($WithSnapshots) {
    Run-Step "gallery snapshots" {
        powershell -ExecutionPolicy Bypass -File tools\capture-gallery-snapshots.ps1 -Compare
    }
}

Write-Host ""
Write-Host "MoonUI guardrails PASS"
