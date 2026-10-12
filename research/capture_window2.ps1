Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class W3 {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr hWnd, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr hWnd, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int n);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool SetWindowDisplayAffinity(IntPtr hWnd, uint aff);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
}
"@

$out = "C:\repo\last-epoch\research"
$pids = (Get-Process D4Turbo -ErrorAction SilentlyContinue).Id
$target = [IntPtr]::Zero
$cb = [W3+EnumWindowsProc]{
    param($h, $l)
    $p = 0
    [W3]::GetWindowThreadProcessId($h, [ref]$p) | Out-Null
    if ($pids -contains $p -and [W3]::IsWindowVisible($h)) {
        $cls = New-Object Text.StringBuilder 256
        [W3]::GetClassNameW($h, $cls, 256) | Out-Null
        if ($cls.ToString() -eq 'Tauri Window') { $script:target = $h }
    }
    return $true
}
[W3]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
if ($target -eq [IntPtr]::Zero) { Write-Host "NO WINDOW"; exit 1 }

[W3]::ShowWindow($target, 9) | Out-Null
# HWND_TOPMOST = -1 ; SWP_SHOWWINDOW = 0x40
[W3]::SetWindowPos($target, [IntPtr](-1), 0, 0, 1040, 849, 0x40) | Out-Null
[W3]::BringWindowToTop($target) | Out-Null
[W3]::SetForegroundWindow($target) | Out-Null
Start-Sleep -Seconds 2

# WDA_EXCLUDEFROMCAPTURE = 0x11 ; temporarily re-enable capture for our screenshot
$ok = [W3]::SetWindowDisplayAffinity($target, 0)
Write-Host "SetWindowDisplayAffinity(0) -> $ok"
Start-Sleep -Milliseconds 800

$r = New-Object W3+RECT
[W3]::GetWindowRect($target, [ref]$r) | Out-Null
$w = $r.R - $r.L; $h = $r.B - $r.T
Write-Host "rect $($r.L),$($r.T) $($w)x$($h)"

$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $h))
$bmp.Save("$out\d4turbo_window.png", [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Host "saved"
