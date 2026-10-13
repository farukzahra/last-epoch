# Build and publish the Windows packages for the current version.
#
# Run this on every /commit-push that ships something: it reads the version from
# Cargo.toml (so the tag can never drift from the build), builds the installer and
# the portable exe, and uploads both to the GitHub release for that version.
#
# The hosted assets use stable names with no version in them, so the download
# links in the README keep working from one release to the next.
#
# Usage:  powershell -File scripts/publish.ps1
$ErrorActionPreference = 'Continue'
$env:GIT_TERMINAL_PROMPT = '0'

$root = Split-Path $PSScriptRoot -Parent
$owner = 'farukzahra'
$repo = 'last-epoch'
$portableName = 'le-turbo.exe'
$installerName = 'LE-Turbo-setup.exe'

# --- version, straight from the crate manifest -------------------------------
$cargo = Get-Content (Join-Path $root 'Cargo.toml') -Raw
if ($cargo -notmatch '(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"') {
    throw 'could not read the version from Cargo.toml'
}
$version = $Matches[1]
$tag = "v$version"
Write-Host "version: $version  (tag $tag)"

# --- secrets -----------------------------------------------------------------
$pat = (Select-String -Path 'C:\faruk\secrets\github\pat.txt' -Pattern '^(ghp_|github_pat_|gho_)' |
    Select-Object -First 1).Line.Trim()
if (-not $pat) { throw 'no PAT found' }
$auth = @{
    Authorization          = "Bearer $pat"
    Accept                 = 'application/vnd.github+json'
    'X-GitHub-Api-Version' = '2022-11-28'
}
$b64 = [Convert]::ToBase64String([System.Text.Encoding]::ASCII.GetBytes("x-access-token:$pat"))
$gitAuth = "http.https://github.com/.extraheader=Authorization: Basic $b64"

# --- build both packages -----------------------------------------------------
Push-Location $root
try {
    & "$env:USERPROFILE\.cargo\bin\cargo.exe" tauri build 2>&1 |
        Select-String -Pattern 'Finished 1 bundle|error' | Select-Object -First 3
    if ($LASTEXITCODE -ne 0) { throw 'cargo tauri build failed' }
} finally {
    Pop-Location
}

$portable = Join-Path $root 'target\release\le-turbo.exe'
$installer = Get-ChildItem (Join-Path $root 'target\release\bundle\nsis') -Filter '*-setup.exe' -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -like "*$version*" } |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not (Test-Path $portable)) { throw "no portable exe at $portable" }
if (-not $installer) { throw "no installer for $version under bundle\nsis" }
Write-Host ("installer: {0} ({1:N2} MB)" -f $installer.Name, ($installer.Length / 1MB))

# --- tag ---------------------------------------------------------------------
Push-Location $root
try {
    git tag -f $tag | Out-Null
    git -c $gitAuth push -f origin $tag 2>&1 | Select-String -Pattern $tag
} finally {
    Pop-Location
}

# --- release notes from the release history ---------------------------------
$notes = "LE Turbo $version."
$historyPath = Join-Path $root 'docs\release-history.json'
if (Test-Path $historyPath) {
    $history = Get-Content $historyPath -Raw | ConvertFrom-Json
    $entry = $history.entries | Where-Object { $_.version -eq $version } | Select-Object -First 1
    if ($entry) { $notes = "$($entry.title).`n`n$($entry.summary)" }
}
$notes += @'

Two downloads:
  LE-Turbo-setup.exe  installs for the current user, no admin prompt, and the
                      finish page offers a desktop shortcut.
  le-turbo.exe        portable, no install, just run it.

Both need the WebView2 runtime, which ships with Windows 10 and 11. Neither is
code signed, so SmartScreen warns the first time: More info, then Run anyway.

Rows start disabled. Turn one on, then press the toggle key, F2 by default.
'@

# --- create or reuse the release, then upload both assets --------------------
$release = $null
try {
    $release = Invoke-RestMethod -Method Post -Uri "https://api.github.com/repos/$owner/$repo/releases" `
        -Headers $auth -ContentType 'application/json' -Body (@{
            tag_name = $tag; name = "LE Turbo $version"; body = $notes
            draft = $false; prerelease = $false
        } | ConvertTo-Json)
    Write-Host "created release: $($release.html_url)"
} catch {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$owner/$repo/releases/tags/$tag" -Headers $auth
    Write-Host "release exists: $($release.html_url)"
    Invoke-RestMethod -Method Patch -Uri "https://api.github.com/repos/$owner/$repo/releases/$($release.id)" `
        -Headers $auth -ContentType 'application/json' -Body (@{ body = $notes } | ConvertTo-Json) | Out-Null
}

$existing = Invoke-RestMethod -Uri "https://api.github.com/repos/$owner/$repo/releases/$($release.id)/assets" -Headers $auth
foreach ($asset in $existing) {
    if ($asset.name -eq $portableName -or $asset.name -eq $installerName) {
        Invoke-RestMethod -Method Delete -Uri "https://api.github.com/repos/$owner/$repo/releases/assets/$($asset.id)" -Headers $auth | Out-Null
        Write-Host "replaced $($asset.name)"
    }
}

foreach ($pair in @(@($portable, $portableName), @($installer.FullName, $installerName))) {
    $uploaded = Invoke-RestMethod -Method Post `
        -Uri "https://uploads.github.com/repos/$owner/$repo/releases/$($release.id)/assets?name=$($pair[1])" `
        -Headers $auth -Body ([IO.File]::ReadAllBytes($pair[0])) -ContentType 'application/octet-stream'
    Write-Host ("uploaded {0}  ({1:N2} MB)" -f $uploaded.name, ($uploaded.size / 1MB))
}

# --- prove the links work, so a broken release is never handed over ----------
foreach ($name in @($installerName, $portableName)) {
    $url = "https://github.com/$owner/$repo/releases/latest/download/$name"
    try {
        $head = Invoke-WebRequest -Uri $url -Method Head -UseBasicParsing -MaximumRedirection 5
        Write-Host "OK  $url  ($($head.Headers.'Content-Length') bytes)"
    } catch {
        Write-Host "BROKEN  $url  :: $($_.Exception.Message)"
    }
}
Write-Host "done: https://github.com/$owner/$repo/releases/tag/$tag"
