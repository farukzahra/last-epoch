Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class W {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern int GetWindowTextW(IntPtr hWnd, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern int GetClassNameW(IntPtr hWnd, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
  [DllImport("user32.dll")] public static extern int GetWindowLongW(IntPtr hWnd, int n);
  [DllImport("user32.dll")] public static extern int GetWindowLongPtrW(IntPtr hWnd, int n);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
}
"@

$out = "C:\repo\last-epoch\research"
$exe = "C:\Program Files\D4Turbo\D4Turbo.exe"

if (-not (Get-Process D4Turbo -ErrorAction SilentlyContinue)) {
    Write-Host "launching..."
    Start-Process -FilePath $exe
    Start-Sleep -Seconds 10
} else {
    Write-Host "already running"
}

$pids = (Get-Process D4Turbo -ErrorAction SilentlyContinue).Id
Write-Host "pids: $($pids -join ',')"

$rows = New-Object System.Collections.ArrayList
$cb = [W+EnumWindowsProc]{
    param($h, $l)
    $pid2 = 0
    [W]::GetWindowThreadProcessId($h, [ref]$pid2) | Out-Null
    if ($pids -contains $pid2) {
        $sb = New-Object Text.StringBuilder 512
        [W]::GetWindowTextW($h, $sb, 512) | Out-Null
        $title = $sb.ToString()
        $cb2 = New-Object Text.StringBuilder 256
        [W]::GetClassNameW($h, $cb2, 256) | Out-Null
        $r = New-Object W+RECT
        [W]::GetWindowRect($h, [ref]$r) | Out-Null
        [void]$rows.Add([pscustomobject]@{
            hwnd   = ("0x{0:X}" -f [int64]$h)
            pid    = $pid2
            visible= [W]::IsWindowVisible($h)
            style  = ("0x{0:X8}" -f ([W]::GetWindowLongW($h, -16) -band 0xFFFFFFFF))
            exStyle= ("0x{0:X8}" -f ([W]::GetWindowLongPtrW($h, -20) -band 0xFFFFFFFF))
            class  = $cb2.ToString()
            title  = $title
            rect   = "$($r.L),$($r.T) $($r.R),$($r.B)"
        })
    }
    return $true
}
[W]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
$rows | Format-Table -AutoSize | Out-String -Width 300

# screenshot the primary screen
$b = [System.Windows.Forms.SystemInformation]::VirtualScreen
$bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($b.X, $b.Y, 0, 0, $bmp.Size)
$bmp.Save("$out\d4turbo_screen.png", [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Host "saved $out\d4turbo_screen.png"
