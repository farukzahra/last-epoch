# Attach the NSIS installer to the existing v0.1.0 release alongside the portable
# exe, and re-point the tag at the commit the artifacts were built from.
$ErrorActionPreference = 'Continue'
$env:GIT_TERMINAL_PROMPT = '0'

$owner = 'farukzahra'
$repo = 'last-epoch'
$root = 'C:\repo\last-epoch'
$tag = 'v0.1.0'

$pat = (Select-String -Path 'C:\faruk\secrets\github\pat.txt' -Pattern '^(ghp_|github_pat_|gho_)' |
    Select-Object -First 1).Line.Trim()
if (-not $pat) { throw 'no PAT found' }

$auth = @{
    Authorization          = "Bearer $pat"
    Accept                 = 'application/vnd.github+json'
    'X-GitHub-Api-Version' = '2022-11-28'
}

$installer = Get-ChildItem (Join-Path $root 'target\release\bundle\nsis') -Filter '*-setup.exe' -ErrorAction SilentlyContinue |
    Select-Object -First 1
if (-not $installer) { throw 'no NSIS installer found - run cargo tauri build first' }
Write-Host "installer: $($installer.FullName) ($([math]::Round($installer.Length / 1MB, 2)) MB)"

$release = Invoke-RestMethod -Uri "https://api.github.com/repos/$owner/$repo/releases/tags/$tag" -Headers $auth

# Replace only the installer asset; keep the portable exe on the same release.
$assets = Invoke-RestMethod -Uri "https://api.github.com/repos/$owner/$repo/releases/$($release.id)/assets" -Headers $auth
foreach ($asset in $assets) {
    if ($asset.name -like '*-setup.exe') {
        Invoke-RestMethod -Method Delete -Uri "https://api.github.com/repos/$owner/$repo/releases/assets/$($asset.id)" -Headers $auth | Out-Null
        Write-Host "removed old $($asset.name)"
    }
}

$bytes = [IO.File]::ReadAllBytes($installer.FullName)
$name = $installer.Name
$uploaded = Invoke-RestMethod -Method Post `
    -Uri "https://uploads.github.com/repos/$owner/$repo/releases/$($release.id)/assets?name=$name" `
    -Headers $auth -Body $bytes -ContentType 'application/octet-stream'
Write-Host "uploaded: $($uploaded.browser_download_url)"

Write-Host "--- assets now on $tag ---"
$final = Invoke-RestMethod -Uri "https://api.github.com/repos/$owner/$repo/releases/$($release.id)/assets" -Headers $auth
foreach ($asset in $final) {
    Write-Host ("  {0}  {1} MB" -f $asset.name, [math]::Round($asset.size / 1MB, 2))
}
