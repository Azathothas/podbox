[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$paperDir = Join-Path $projectRoot 'paper'
$pdf = Join-Path $paperDir 'sandbox-insights.pdf'
$renderDir = Join-Path $paperDir 'rendered'

if (-not (Test-Path -LiteralPath $pdf -PathType Leaf)) {
    Write-Error "missing PDF: $pdf"
    exit 2
}
$pdftoppm = Get-Command pdftoppm -ErrorAction SilentlyContinue
if (-not $pdftoppm) {
    Write-Error 'pdftoppm is required for visual rendering'
    exit 2
}

$resolvedPaper = [IO.Path]::GetFullPath($paperDir)
$resolvedRender = [IO.Path]::GetFullPath($renderDir)
if (-not $resolvedRender.StartsWith($resolvedPaper + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    Write-Error "unsafe render directory: $resolvedRender"
    exit 1
}
if (Test-Path -LiteralPath $renderDir) {
    Remove-Item -LiteralPath $renderDir -Recurse -Force
}
New-Item -ItemType Directory -Path $renderDir | Out-Null

& $pdftoppm.Source -png -r 144 $pdf (Join-Path $renderDir 'page')
$rc = $LASTEXITCODE
if ($rc -ne 0) {
    Write-Output "FAIL pdftoppm exit=$rc"
    exit 1
}
$pages = @(Get-ChildItem -LiteralPath $renderDir -File -Filter 'page-*.png' | Sort-Object Name)
if ($pages.Count -lt 4) {
    Write-Output "FAIL rendered pages=$($pages.Count)"
    exit 1
}
Write-Output "PASS rendered pages=$($pages.Count) directory=$renderDir"
exit 0
