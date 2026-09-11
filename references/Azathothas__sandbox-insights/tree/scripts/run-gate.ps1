[CmdletBinding()]
param([string]$PythonExecutable)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent

function Find-Python {
    param([string]$Requested)
    if ($Requested) {
        if (Test-Path -LiteralPath $Requested -PathType Leaf) { return (Resolve-Path $Requested).Path }
        $cmd = Get-Command $Requested -ErrorAction SilentlyContinue
        if ($cmd) { return $cmd.Source }
        return $null
    }
    foreach ($name in 'python','python3') {
        $cmd = Get-Command $name -ErrorAction SilentlyContinue
        if ($cmd) { return $cmd.Source }
    }
    $userRoot = Split-Path (Split-Path (Split-Path $projectRoot -Parent) -Parent) -Parent
    $bundled = Join-Path $userRoot '.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe'
    if (Test-Path -LiteralPath $bundled -PathType Leaf) { return $bundled }
    return $null
}

function Invoke-GateStep {
    param([string]$Name, [scriptblock]$Action)
    Write-Output "=== $Name ==="
    & $Action
    $rc = $LASTEXITCODE
    if ($null -eq $rc) { $rc = 0 }
    if ($rc -ne 0) {
        Write-Output "GATE FAIL step=$Name exit=$rc"
        exit $rc
    }
    Write-Output "GATE PASS step=$Name"
}

$python = Find-Python $PythonExecutable
if (-not $python) {
    Write-Output 'GATE CANNOT RUN: Python with reportlab and pypdf is required'
    exit 2
}

Invoke-GateStep 'artifact inventory' { & (Join-Path $projectRoot 'experiments/10-artifact-inventory.ps1') }
Invoke-GateStep 'local evidence ledger' { & (Join-Path $projectRoot 'experiments/20-evidence-audit.ps1') }
Invoke-GateStep 'mechanism consistency' { & (Join-Path $projectRoot 'experiments/30-mechanism-consistency.ps1') }
Invoke-GateStep 'derived tradeoff model' { & (Join-Path $projectRoot 'experiments/40-tradeoff-model.ps1') }
Invoke-GateStep 'paper build' { & $python (Join-Path $PSScriptRoot 'build-paper.py') }
Invoke-GateStep 'structure and links' { & (Join-Path $PSScriptRoot 'check-structure.ps1') }
Invoke-GateStep 'paper render' { & (Join-Path $PSScriptRoot 'render-paper.ps1') }

$pdfinfo = Get-Command pdfinfo -ErrorAction SilentlyContinue
if (-not $pdfinfo) {
    Write-Output 'GATE CANNOT RUN: pdfinfo is required'
    exit 2
}
$pdf = Join-Path $projectRoot 'paper/sandbox-insights.pdf'
& $pdfinfo.Source $pdf
$rc = $LASTEXITCODE
if ($rc -ne 0) {
    Write-Output "GATE FAIL step=pdfinfo exit=$rc"
    exit 1
}
Write-Output 'GATE PASS all steps'
exit 0
