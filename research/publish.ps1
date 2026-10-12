# Publish LE Turbo to GitHub as a PUBLIC repository and push main.
# Reads the PAT from the documented vault; never prints it.
$ErrorActionPreference = 'Stop'
$env:GIT_TERMINAL_PROMPT = '0'

$repoName = 'last-epoch'
$repoRoot = 'C:\repo\last-epoch'

$patLine = (Select-String -Path 'C:\faruk\secrets\github\pat.txt' -Pattern '^(ghp_|github_pat_|gho_)' |
    Select-Object -First 1).Line
$pat = $patLine.Trim()
if (-not $pat) { throw 'no PAT found' }

$headers = @{
    Authorization          = "Bearer $pat"
    Accept                 = 'application/vnd.github+json'
    'X-GitHub-Api-Version' = '2022-11-28'
}

$owner = (Invoke-RestMethod -Uri 'https://api.github.com/user' -Headers $headers).login
Write-Host "owner: $owner"

$description = 'Presses your Last Epoch skills on a clock, holds the left button so your character runs, and never leaves a key stuck.'
$body = @{
    name        = $repoName
    description = $description
    private     = $false
    auto_init   = $false
    has_issues  = $true
} | ConvertTo-Json

try {
    $created = Invoke-RestMethod -Method Post -Uri 'https://api.github.com/user/repos' -Headers $headers -Body $body -ContentType 'application/json'
    Write-Host "created: $($created.html_url)  private=$($created.private)"
} catch {
    Write-Host "create failed (may already exist): $($_.Exception.Message)"
}

Push-Location $repoRoot
try {
    git checkout -q main
    git merge -q --ff-only "feat/le-turbo-v1"
    git remote remove origin 2>&1 | Out-Null
    git remote add origin "https://github.com/$owner/$repoName.git"

    $b64 = [Convert]::ToBase64String([System.Text.Encoding]::ASCII.GetBytes("x-access-token:$pat"))
    git -c "http.https://github.com/.extraheader=Authorization: Basic $b64" push -u origin main 2>&1 | Select-Object -Last 4
    Write-Host "push exit: $LASTEXITCODE"
    git log --oneline -1
} finally {
    Pop-Location
}
