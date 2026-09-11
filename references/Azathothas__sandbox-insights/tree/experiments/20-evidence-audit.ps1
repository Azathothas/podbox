[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$experimentRoot = $PSScriptRoot
$projectRoot = Split-Path $experimentRoot -Parent
$logDir = Join-Path $experimentRoot 'logs'
$log = Join-Path $logDir '20-evidence-audit.log'
$checker = Join-Path $projectRoot 'scripts/check-evidence.ps1'
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

$result = @(& $checker)
$rc = $LASTEXITCODE
if ($null -eq $rc) { $rc = 0 }
$lines = @(
    '## question: does every evidence row resolve to a repository-local literal anchor and condition?'
    "date_utc=$([DateTime]::UtcNow.ToString('o'))"
) + $result + "exit=$rc"
$lines | Set-Content -LiteralPath $log -Encoding utf8
$lines
exit $rc
