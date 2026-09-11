[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$experimentRoot = $PSScriptRoot
$projectRoot = Split-Path $experimentRoot -Parent
$logDir = Join-Path $experimentRoot 'logs'
$log = Join-Path $logDir '10-artifact-inventory.log'
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

$required = @(
    'README.md','AGENTS.md','LICENSE',
    'docs/observations.md','docs/capability-model.md','docs/evidence-model.md',
    'docs/research-method.md','docs/architecture.md',
    'docs/failure-field-guide.md','docs/open-questions.md',
    'experiments/README.md','experiments/evidence-ledger.csv',
    'experiments/data/benchmark-samples.csv',
    'experiments/data/policy-transitions.csv','paper/paper.md'
)
$lines = @(
    '## question: is the agent-first standalone artifact complete and locally anchored?'
    "date_utc=$([DateTime]::UtcNow.ToString('o'))"
    "powershell=$($PSVersionTable.PSVersion)"
)
$fail = 0
foreach ($relative in $required) {
    $path = Join-Path $projectRoot $relative
    if (Test-Path -LiteralPath $path -PathType Leaf) {
        $lines += "PASS exists $relative"
    } else {
        $lines += "FAIL missing $relative"
        $fail++
    }
}

$license = Join-Path $projectRoot 'LICENSE'
if ((Test-Path -LiteralPath $license) -and
    (Get-Content -Raw -LiteralPath $license).Contains('Zero-Clause BSD') -and
    (Get-Content -Raw -LiteralPath $license).Contains('Permission to use, copy, modify, and/or distribute')) {
    $lines += 'PASS license=0BSD'
} else {
    $lines += 'FAIL license is not canonical 0BSD text'
    $fail++
}

$textFiles = Get-ChildItem -LiteralPath $projectRoot -Recurse -File | Where-Object {
    $_.FullName -notmatch '\\.git\\|\\paper\\rendered\\' -and
    ($_.Extension -in '.md','.ps1','.py','.json','.csv','.tex','.log' -or $_.Name -in 'LICENSE','.gitignore')
}
$revisionPattern = '(?<![0-9a-f])[0-9a-f]{40}(?![0-9a-f])'
$revisionHits = @($textFiles | Select-String -Pattern $revisionPattern -CaseSensitive:$false)
if ($revisionHits.Count) {
    foreach ($hit in $revisionHits) {
        $relative = $hit.Path.Substring($projectRoot.Length + 1)
        $lines += "FAIL embedded external revision ${relative}:$($hit.LineNumber)"
    }
    $fail += $revisionHits.Count
} else {
    $lines += 'PASS no embedded 40-character external revisions'
}

$rc = if ($fail) { 1 } else { 0 }
$lines += "exit=$rc"
$lines | Set-Content -LiteralPath $log -Encoding utf8
$lines
exit $rc
