$ErrorActionPreference = 'Continue'
$exe = Join-Path $env:TEMP 'vs_buildtools.exe'
Write-Host "== installing MSVC Build Tools (VCTools workload + recommended)"
Write-Host "   this pulls the MSVC v143 toolset, linker and the Windows SDK (~3-6 GB)"
Write-Host "   A UAC prompt will appear - approve it."

$args = @(
    '--quiet', '--wait', '--norestart', '--nocache',
    '--add', 'Microsoft.VisualStudio.Workload.VCTools',
    '--includeRecommended'
)
$p = Start-Process -FilePath $exe -ArgumentList $args -Wait -PassThru
Write-Host "   bootstrapper exit code: $($p.ExitCode)"
Write-Host "   (0 = success, 3010 = success + reboot suggested, 1602 = user cancelled at UAC)"

Write-Host "== verify with vswhere"
$vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
& $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -format value -property installationPath
& $vswhere -latest -products * -format value -property displayName
Write-Host "== locate link.exe"
Get-ChildItem 'C:\Program Files\Microsoft Visual Studio','C:\Program Files (x86)\Microsoft Visual Studio' -Recurse -Filter link.exe -ErrorAction SilentlyContinue |
    Select-Object -First 5 -ExpandProperty FullName
Write-Host "DONE"
