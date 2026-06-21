param(
    [string[]]$Pages = @("Controls", "Inputs", "Data", "Overlays", "Layout", "NewControls", "Composites", "Stateful"),
    [string[]]$Themes = @("Dark", "Light"),
    [switch]$UpdateBaseline,
    [switch]$Compare,
    [double]$MaxDiffPercent = 0.05
)

$ErrorActionPreference = "Stop"

$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Target = Join-Path $Repo "target\moon-ui-gallery-snapshots"
$CurrentDir = Join-Path $Target "current"
$BaselineDir = Join-Path $Repo "crates\moon-ui-gallery\snapshots\baseline"
New-Item -ItemType Directory -Force -Path $CurrentDir | Out-Null
New-Item -ItemType Directory -Force -Path $BaselineDir | Out-Null

Add-Type -AssemblyName System.Drawing

if ($env:OS -eq "Windows_NT") {
    Add-Type @"
using System.Runtime.InteropServices;
public static class MoonUiGalleryCursor {
    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int x, int y);
}
"@
    [MoonUiGalleryCursor]::SetCursorPos(1, 1) | Out-Null
    Start-Sleep -Milliseconds 200
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

Push-Location $Repo
try {
    Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path $CurrentDir "*.png")

    foreach ($theme in $Themes) {
        $themeDir = Join-Path $CurrentDir $theme
        New-Item -ItemType Directory -Force -Path $themeDir | Out-Null
        Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path $themeDir "*.png")

        if ($env:OS -eq "Windows_NT") {
            [MoonUiGalleryCursor]::SetCursorPos(1, 1) | Out-Null
            Start-Sleep -Milliseconds 200
        }

        cargo run -p moon-ui-gallery --features snapshot -- --snapshot-dir $themeDir --theme $theme
        if ($LASTEXITCODE -ne 0) {
            throw "moon-ui-gallery internal snapshot run failed for theme $theme"
        }

        foreach ($page in $Pages) {
            $capturedPath = Join-Path $themeDir "$page.png"
            $snapshotName = "$theme-$page"
            $outPath = Join-Path $CurrentDir "$snapshotName.png"
            $baselinePath = Join-Path $BaselineDir "$snapshotName.png"
            if (-not (Test-Path $capturedPath)) {
                throw "missing current snapshot for ${snapshotName}: $capturedPath"
            }

            Copy-Item -Force $capturedPath $outPath

            if ($UpdateBaseline) {
                Copy-Item -Force $outPath $baselinePath
                Write-Host "updated baseline $snapshotName -> $baselinePath"
            } elseif ($Compare) {
                if (-not (Test-Path $baselinePath)) {
                    throw "missing baseline for $snapshotName; run with -UpdateBaseline first"
                }
                try {
                    $diff = Compare-Png -Expected $baselinePath -Actual $outPath -MaxDiffPercent $MaxDiffPercent
                } catch {
                    throw "${snapshotName}: $($_.Exception.Message)"
                }
                Write-Host ("PASS {0}: diff {1:N4}%" -f $snapshotName, $diff)
            } else {
                Write-Host "captured $snapshotName -> $outPath"
            }
        }
    }
} finally {
    Pop-Location
}
