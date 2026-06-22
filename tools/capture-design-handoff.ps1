param(
    [string[]]$Pages = @("Controls", "Inputs", "Data", "Overlays", "Layout", "NewControls", "Composites", "Stateful"),
    [string[]]$Themes = @("Dark", "Light"),
    [switch]$Compare,
    [switch]$FailOnDiff,
    [double]$MaxDiffPercent = 0.05
)

$ErrorActionPreference = "Stop"

$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$HandoffDir = Join-Path $Repo "design-handoff"
$CurrentDir = Join-Path $HandoffDir "screenshots\current"
$BaselineDir = Join-Path $Repo "crates\moon-ui-gallery\snapshots\baseline"

New-Item -ItemType Directory -Force -Path $CurrentDir | Out-Null
Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path $CurrentDir "*.png")

Add-Type -AssemblyName System.Drawing

function Find-Chrome {
    $candidates = @(
        "$env:ProgramFiles\Google\Chrome\Application\chrome.exe",
        "${env:ProgramFiles(x86)}\Google\Chrome\Application\chrome.exe",
        "$env:LocalAppData\Google\Chrome\Application\chrome.exe",
        "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe",
        "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe"
    )
    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path $candidate)) {
            return $candidate
        }
    }
    throw "Chrome or Edge executable was not found"
}

function Compare-Png {
    param(
        [string]$Expected,
        [string]$Actual,
        [double]$MaxDiffPercent
    )

    $a = [System.Drawing.Bitmap]::FromFile($Expected)
    $b = [System.Drawing.Bitmap]::FromFile($Actual)
    try {
        if ($a.Width -ne $b.Width -or $a.Height -ne $b.Height) {
            throw "size mismatch expected $($a.Width)x$($a.Height), actual $($b.Width)x$($b.Height)"
        }

        $changed = 0
        $total = $a.Width * $a.Height
        for ($y = 0; $y -lt $a.Height; $y++) {
            for ($x = 0; $x -lt $a.Width; $x++) {
                if ($a.GetPixel($x, $y).ToArgb() -ne $b.GetPixel($x, $y).ToArgb()) {
                    $changed++
                }
            }
        }

        $percent = $changed * 100.0 / $total
        if ($percent -gt $MaxDiffPercent) {
            throw ("pixel diff {0:N4}% exceeds {1:N4}% ({2}/{3} px)" -f $percent, $MaxDiffPercent, $changed, $total)
        }
        return $percent
    } finally {
        $a.Dispose()
        $b.Dispose()
    }
}

$chrome = Find-Chrome
foreach ($theme in $Themes) {
    foreach ($page in $Pages) {
        $name = "$theme-$page"
        $htmlPath = Join-Path $HandoffDir "pages\$name.html"
        if (-not (Test-Path $htmlPath)) {
            throw "missing handoff page for ${name}: $htmlPath"
        }
        $url = ([System.Uri](Resolve-Path $htmlPath).Path).AbsoluteUri
        $out = Join-Path $CurrentDir "$name.png"
        $args = @(
            "--headless=new",
            "--disable-gpu",
            "--hide-scrollbars",
            "--force-device-scale-factor=1",
            "--window-size=1280,900",
            "--virtual-time-budget=500",
            "--screenshot=$out",
            $url
        )
        $p = Start-Process -FilePath $chrome -ArgumentList $args -Wait -PassThru -WindowStyle Hidden
        if ($p.ExitCode -ne 0) {
            throw "Chrome screenshot failed for $name with exit code $($p.ExitCode)"
        }
        Write-Host "captured $name -> $out"

        if ($Compare) {
            $baseline = Join-Path $BaselineDir "$name.png"
            if (-not (Test-Path $baseline)) {
                throw "missing GPUI baseline for ${name}: $baseline"
            }
            if ($FailOnDiff) {
                $diff = Compare-Png -Expected $baseline -Actual $out -MaxDiffPercent $MaxDiffPercent
                Write-Host ("COMPARE {0}: diff {1:N4}%" -f $name, $diff)
            } else {
                $diff = Compare-Png -Expected $baseline -Actual $out -MaxDiffPercent 100.0
                Write-Host ("REPORT {0}: diff {1:N4}%" -f $name, $diff)
            }
        }
    }
}
