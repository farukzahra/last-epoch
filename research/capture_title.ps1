param([string]$Match = "LE Turbo", [string]$Out = "C:\repo\last-epoch\research\le_turbo_ui.png")

Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class CW {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr p);
  public delegate bool EnumWindowsProc(IntPtr h, IntPtr p);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextLengthW(IntPtr h);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int n);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint f);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
}
"@

[void][CW]::SetProcessDPIAware()

$target = [IntPtr]::Zero
[CW]::EnumWindows({
  param($h, $p)
  if ([CW]::IsWindowVisible($h)) {
    $n = [CW]::GetWindowTextLengthW($h)
    if ($n -gt 0) {
      $sb = New-Object Text.StringBuilder ($n + 1)
      [CW]::GetWindowTextW($h, $sb, $n + 1) | Out-Null
      if ($sb.ToString() -like "*$Match*") {
        $cl = New-Object Text.StringBuilder 256
        [CW]::GetClassNameW($h, $cl, 256) | Out-Null
        if ($cl.ToString() -notlike 'Tauri*') { Write-Host "found: '$($sb.ToString())' class=$($cl.ToString())"; $script:target = $h }
      }
    }
  }
  return $true
}, [IntPtr]::Zero) | Out-Null

if ($target -eq [IntPtr]::Zero) { Write-Host "WINDOW NOT FOUND"; exit 1 }

[CW]::ShowWindow($target, 9) | Out-Null
[CW]::SetWindowPos($target, [IntPtr](-1), 60, 60, 0, 0, 0x43) | Out-Null
[CW]::BringWindowToTop($target) | Out-Null
[CW]::SetForegroundWindow($target) | Out-Null
Start-Sleep -Seconds 2

$r = New-Object CW+RECT
[CW]::GetWindowRect($target, [ref]$r) | Out-Null
$w = $r.R - $r.L; $h = $r.B - $r.T
Write-Host "capturing $w x $h at $($r.L),$($r.T)"
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $h))
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Host "saved $Out"
