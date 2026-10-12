Add-Type -AssemblyName System.Drawing
$src = "C:\repo\last-epoch\research\le_monitor1.png"
$out = "C:\repo\last-epoch\research"

$img = [System.Drawing.Image]::FromFile($src)

function Crop([string]$name, [int]$x, [int]$y, [int]$w, [int]$h, [int]$scale) {
    $bmp = New-Object System.Drawing.Bitmap ($w * $scale), ($h * $scale)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half
    $g.DrawImage($img, (New-Object System.Drawing.Rectangle 0,0,($w*$scale),($h*$scale)), (New-Object System.Drawing.Rectangle $x,$y,$w,$h), [System.Drawing.GraphicsUnit]::Pixel)
    $g.Dispose()
    $p = Join-Path $out "crop_$name.png"
    $bmp.Save($p, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Host "crop_$name.png  <- src($x,$y ${w}x${h}) x$scale"
}

Crop "buffzone" 1380 0 540 400 2
Crop "skillbar" 690 960 420 120 3
Crop "orbs"     380 880 360 200 2
$img.Dispose()
