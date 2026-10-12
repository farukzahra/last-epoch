Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class W2 {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr hWnd, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr hWnd, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int n);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
}
"@

$out = "C:\repo\last-epoch\research"
$pids = (Get-Process D4Turbo -ErrorAction SilentlyContinue).Id
Write-Host "pids: $($pids -join ',')"

$target = [IntPtr]::Zero
$cb = [W2+EnumWindowsProc]{
    param($h, $l)
    $p = 0
    [W2]::GetWindowThreadProcessId($h, [ref]$p) | Out-Null
    if ($pids -contains $p -and [W2]::IsWindowVisible($h)) {
        $sb = New-Object Text.StringBuilder 512
        [W2]::GetWindowTextW($h, $sb, 512) | Out-Null
        $r = New-Object W2+RECT
        [W2]::GetWindowRect($h, [ref]$r) | Out-Null
        $w = $r.R - $r.L; $hh = $r.B - $r.T
        $cls = New-Object Text.StringBuilder 256
        [W2]::GetClassNameW($h, $cls, 256) | Out-Null
        Write-Host ("hwnd=0x{0:X} class='{1}' title='{2}' size={3}x{4} at {5},{6}" -f [int64]$h, $cls.ToString(), $sb.ToString(), $w, $hh, $r.L, $r.T)
        if ($w -gt 300) { $script:target = $h }
    }
    return $true
}
[W2]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null

if ($target -eq [IntPtr]::Zero) { Write-Host "NO VISIBLE WINDOW"; exit 1 }

[W2]::ShowWindow($target, 9) | Out-Null   # SW_RESTORE
[W2]::SetForegroundWindow($target) | Out-Null
Start-Sleep -Seconds 3

$r = New-Object W2+RECT
[W2]::GetWindowRect($target, [ref]$r) | Out-Null
$w = $r.R - $r.L; $h = $r.B - $r.T
Write-Host "capturing $w x $h at $($r.L),$($r.T)"

$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $h))
$bmp.Save("$out\d4turbo_window.png", [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Host "saved $out\d4turbo_window.png"
