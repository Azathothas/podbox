[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$required = @(
    'README.md','AGENTS.md','LICENSE','docs/evidence-model.md',
    'docs/observations.md','docs/research-method.md','docs/capability-model.md',
    'docs/architecture.md','docs/failure-field-guide.md','docs/open-questions.md',
    'experiments/README.md','experiments/evidence-ledger.csv',
    'experiments/data/benchmark-samples.csv',
    'experiments/data/policy-transitions.csv',
    'experiments/10-artifact-inventory.ps1',
    'experiments/20-evidence-audit.ps1',
    'experiments/30-mechanism-consistency.ps1',
    'experiments/40-tradeoff-model.ps1',
    'paper/README.md','paper/metadata.json','paper/paper.md',
    'paper/main.tex','paper/sandbox-insights.pdf',
    'reviews/REVIEW-1-structure.md','reviews/REVIEW-2-claims.md',
    'reviews/REVIEW-3-usability.md','reviews/REVIEW-4-deep-release.md',
    'scripts/build-paper.py','scripts/check-evidence.ps1',
    'scripts/check-structure.ps1','scripts/render-paper.ps1',
    'scripts/run-gate.ps1'
)
$fail = 0
foreach ($relative in $required) {
    $path = Join-Path $projectRoot $relative
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        Write-Output "FAIL missing $relative"
        $fail++
    } else {
        Write-Output "PASS exists $relative"
    }
}

$markdown = Get-ChildItem -LiteralPath $projectRoot -Recurse -File -Filter '*.md'
$linkPattern = '\[[^\]]+\]\(([^)]+)\)'
foreach ($file in $markdown) {
    $content = Get-Content -Raw -LiteralPath $file.FullName
    foreach ($match in [regex]::Matches($content, $linkPattern)) {
        $target = $match.Groups[1].Value.Trim('<','>')
        if ($target -match '^(https?://|mailto:|#)') { continue }
        $pathPart = ($target -split '#', 2)[0]
        if ([string]::IsNullOrWhiteSpace($pathPart)) { continue }
        $resolved = [IO.Path]::GetFullPath((Join-Path $file.DirectoryName $pathPart))
        if (-not (Test-Path -LiteralPath $resolved)) {
            $relativeFile = $file.FullName.Substring($projectRoot.Length + 1)
            Write-Output "FAIL broken link $relativeFile -> $target"
            $fail++
        }
    }
}

$readme = Get-Content -Raw -LiteralPath (Join-Path $projectRoot 'README.md')
$agentRules = Get-Content -Raw -LiteralPath (Join-Path $projectRoot 'AGENTS.md')
if (-not $readme.Contains('Markdown is canonical')) {
    Write-Output 'FAIL README does not declare Markdown canonical'
    $fail++
}
if (-not $agentRules.Contains('written for agents first')) {
    Write-Output 'FAIL AGENTS.md does not declare agent-first intent'
    $fail++
}

if ($fail) {
    Write-Output "SUMMARY FAIL=$fail"
    exit 1
}
Write-Output "SUMMARY PASS files=$($required.Count) markdown=$($markdown.Count)"
exit 0
