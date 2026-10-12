Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class Cap2 {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr p);
  public delegate bool EnumWindowsProc(IntPtr h, IntPtr p);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr h, uint cmd);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
}
"@

[Cap2]::SetProcessDPIAware() | Out-Null
$out = "C:\repo\last-epoch\research"

$game = [IntPtr]::Zero
[Cap2]::EnumWindows({
  param($h, $p)
  if ([Cap2]::IsWindowVisible($h)) {
    $c = New-Object Text.StringBuilder 256; [Cap2]::GetClassNameW($h, $c, 256) | Out-Null
    if ($c.ToString() -eq 'UnityWndClass') { $script:game = $h }
  }
  return $true
}, [IntPtr]::Zero) | Out-Null

# walk up to the root owner (the real top-level game window)
$root = $game
while ($true) {
  $owner = [Cap2]::GetWindow($root, 4)  # GW_OWNER
  if ($owner -eq [IntPtr]::Zero) { break }
  $root = $owner
}

$r = New-Object Cap2+RECT
[Cap2]::GetWindowRect($root, [ref]$r) | Out-Null
Write-Host "game hwnd=0x$('{0:X}' -f [int64]$root) rect=$($r.L),$($r.T) $($r.R - $r.L)x$($r.B - $r.T)"

Write-Host "=== SCREENS (physically aware) ==="
foreach ($s in [System.Windows.Forms.Screen]::AllScreens) {
  Write-Host ("  {0}  bounds={1}  primary={2}" -f $s.DeviceName, $s.Bounds, $s.Primary)
}

# screen that contains the game centre
$cx = [int](($r.L + $r.R) / 2); $cy = [int](($r.T + $r.B) / 2)
$screen = [System.Windows.Forms.Screen]::AllScreens |
  Where-Object { $_.Bounds.Contains($cx, $cy) } | Select-Object -First 1
if (-not $screen) { $screen = [System.Windows.Forms.Screen]::PrimaryScreen }
$b = $screen.Bounds
Write-Host "capturing screen $($screen.DeviceName) bounds=$b"

$bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($b.X, $b.Y, 0, 0, (New-Object System.Drawing.Size $b.Width, $b.Height))
$bmp.Save("$out\le_monitor1.png", [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Host "saved $out\le_monitor1.png  ($($b.Width)x$($b.Height))"
