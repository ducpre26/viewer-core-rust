[CmdletBinding()]
param(
    [string]$Image = 'viewer-core-rust:test',
    [string]$ExportPath
)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
foreach ($command in @('git', 'docker')) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
        throw "$command is required. On Windows, install Docker Desktop and select Linux containers."
    }
}
if ($ExportPath) {
    $ExportPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($ExportPath)
    if (Test-Path -LiteralPath $ExportPath) { throw "Export already exists: $ExportPath" }
    if (-not (Test-Path -LiteralPath (Split-Path -Parent $ExportPath) -PathType Container)) {
        throw 'The export parent directory must already exist.'
    }
}
Push-Location $repoRoot
try {
    $dockerOs = & docker info --format '{{.OSType}}'
    if ($LASTEXITCODE -ne 0 -or "$dockerOs".Trim() -ne 'linux') {
        throw 'Docker must be running with a Linux container engine.'
    }
    & docker buildx version
    if ($LASTEXITCODE -ne 0) { throw 'Docker Buildx is required.' }
    $revision = & git rev-parse --verify HEAD
    if ($LASTEXITCODE -ne 0) { throw 'Cannot read source revision.' }
    $changes = & git status --porcelain
    if ($LASTEXITCODE -ne 0) { throw 'Cannot read source working-tree state.' }
    $dirty = if ($changes) { 'true' } else { 'false' }
    & docker buildx build --platform linux/amd64 --load --tag $Image `
        --build-arg "SOURCE_REVISION=$revision" --build-arg "SOURCE_DIRTY=$dirty" .
    if ($LASTEXITCODE -ne 0) { throw 'Docker image build failed.' }
    if ($ExportPath) {
        & docker image save --output $ExportPath $Image
        if ($LASTEXITCODE -ne 0) { throw 'Docker image export failed.' }
    }
    Write-Host "Built $Image (linux/amd64); revision=$revision; dirty=$dirty"
} finally {
    Pop-Location
}
