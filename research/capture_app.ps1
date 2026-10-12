param(
    [string]$Title = "LE Turbo",
    [string]$Out = "C:\repo\last-epoch\research\app_window.png"
)

Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class CA {
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

[void][CA]::SetProcessDPIAware()

$found = @()
[CA]::EnumWindows({
        param($h, $p)
        if ([CA]::IsWindowVisible($h)) {
            $n = [CA]::GetWindowTextLengthW($h)
            if ($n -gt 0) {
                $sb = New-Object Text.StringBuilder ($n + 1)
                [CA]::GetWindowTextW($h, $sb, $n + 1) | Out-Null
                $cl = New-Object Text.StringBuilder 256
                [CA]::GetClassNameW($h, $cl, 256) | Out-Null
                if ($sb.ToString() -eq $Title) {
                    Write-Host "MATCH hwnd=0x$('{0:X}' -f [int64]$h) class='$($cl.ToString())' title='$($sb.ToString())'"
                    $script:found += , @($h, $cl.ToString())
                }
            }
        }
        return $true
    }, [IntPtr]::Zero) | Out-Null

if ($found.Count -eq 0) { Write-Host "WINDOW '$Title' NOT FOUND"; exit 1 }

$target = $found[0][0]
[CA]::ShowWindow($target, 9) | Out-Null
[CA]::SetWindowPos($target, [IntPtr](-1), 0, 0, 0, 0, 0x43) | Out-Null
[CA]::BringWindowToTop($target) | Out-Null
[CA]::SetForegroundWindow($target) | Out-Null
Start-Sleep -Seconds 3

$r = New-Object CA+RECT
[CA]::GetWindowRect($target, [ref]$r) | Out-Null
$w = $r.R - $r.L; $h = $r.B - $r.T
Write-Host "capturing $w x $h at $($r.L),$($r.T)"
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $h))
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Host "saved $Out"
