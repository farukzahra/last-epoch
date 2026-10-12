# Tag and publish a GitHub Release with the built exe attached, so the app can be
# downloaded instead of built. Reads the PAT from the vault; never prints it.
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

$exe = Join-Path $root 'target\release\le-turbo.exe'
if (-not (Test-Path $exe)) { throw "no exe at $exe" }
$sizeMB = [math]::Round((Get-Item $exe).Length / 1MB, 2)
Write-Host "exe: $exe ($sizeMB MB)"

Push-Location $root
try {
    git tag -f $tag | Out-Null
    $b64 = [Convert]::ToBase64String([System.Text.Encoding]::ASCII.GetBytes("x-access-token:$pat"))
    git -c "http.https://github.com/.extraheader=Authorization: Basic $b64" push -f origin $tag 2>&1 |
        Select-Object -Last 1
} finally {
    Pop-Location
}

$notes = @'
Windows x64 build of LE Turbo.

Download le-turbo.exe, put it in its own folder, and run it. No installer and no runtime
to add: it needs the WebView2 runtime, which ships with Windows 10 and 11.

Rows start disabled. Turn a row ON, then press F2 to arm, F3 to disarm, F9 to quit.

The app writes profile.json and config.json next to the executable, so keep it in a
folder you can write to (not Program Files).

Because the binary is not code-signed, Windows SmartScreen will warn the first time:
choose "More info" then "Run anyway".
'@

$payload = @{
    tag_name   = $tag
    name       = "LE Turbo $tag"
    body       = $notes
    draft      = $false
    prerelease = $false
} | ConvertTo-Json

try {
    $release = Invoke-RestMethod -Method Post -Uri "https://api.github.com/repos/$owner/$repo/releases" `
        -Headers $auth -Body $payload -ContentType 'application/json'
    Write-Host "created release: $($release.html_url)"
} catch {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$owner/$repo/releases/tags/$tag" -Headers $auth
    Write-Host "release already existed: $($release.html_url)"
}

# Clear old assets so re-running this script is idempotent.
$existing = Invoke-RestMethod -Uri "https://api.github.com/repos/$owner/$repo/releases/$($release.id)/assets" -Headers $auth
foreach ($asset in $existing) {
    Invoke-RestMethod -Method Delete -Uri "https://api.github.com/repos/$owner/$repo/releases/assets/$($asset.id)" -Headers $auth | Out-Null
    Write-Host "removed old asset $($asset.name)"
}

$bytes = [IO.File]::ReadAllBytes($exe)
$uploaded = Invoke-RestMethod -Method Post `
    -Uri "https://uploads.github.com/repos/$owner/$repo/releases/$($release.id)/assets?name=le-turbo.exe" `
    -Headers $auth -Body $bytes -ContentType 'application/octet-stream'
Write-Host "uploaded: $($uploaded.browser_download_url)"
Write-Host "downloads: $($uploaded.download_count)"
