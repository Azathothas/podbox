[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$experimentRoot = $PSScriptRoot
$projectRoot = Split-Path $experimentRoot -Parent
$logDir = Join-Path $experimentRoot 'logs'
$log = Join-Path $logDir '30-mechanism-consistency.log'
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

$checks = @(
    @{ Name='identity mapping retained'; Path='docs/observations.md'; Pattern='`0 1000 1`' },
    @{ Name='path allowlist retained'; Path='docs/observations.md'; Pattern='`/tmp`, `/dev/shm`, `/workspace`, and `/state`' },
    @{ Name='mount stages separated'; Path='docs/capability-model.md'; Pattern='creation, attachment, and use separately' },
    @{ Name='trace event contract complete'; Path='docs/capability-model.md'; Pattern='begin, syscall, signal, exit, and summary events' },
    @{ Name='outer filter blind spot disclosed'; Path='docs/capability-model.md'; Pattern='already-denied syscalls are invisible' },
    @{ Name='TCG boundary precise'; Path='docs/observations.md'; Pattern='The boundary is the emulator process' },
    @{ Name='policy transition recorded'; Path='experiments/data/policy-transitions.csv'; Pattern='tcp-connect-external-80,later,allowed' },
    @{ Name='route selector fails closed'; Path='docs/architecture.md'; Pattern='A weaker route cannot satisfy a stronger security requirement' },
    @{ Name='Markdown declared canonical'; Path='README.md'; Pattern='Markdown is canonical' }
)

$lines = @(
    '## question: do the core mechanisms agree across the local agent-facing documents and data?'
    "date_utc=$([DateTime]::UtcNow.ToString('o'))"
)
$fail = 0
foreach ($check in $checks) {
    $path = Join-Path $projectRoot $check.Path
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        $lines += "FAIL $($check.Name): missing $($check.Path)"
        $fail++
        continue
    }
    if ((Get-Content -Raw -LiteralPath $path).Contains($check.Pattern)) {
        $lines += "PASS $($check.Name)"
    } else {
        $lines += "FAIL $($check.Name): missing anchor"
        $fail++
    }
}
$rc = if ($fail) { 1 } else { 0 }
$lines += "exit=$rc"
$lines | Set-Content -LiteralPath $log -Encoding utf8
$lines
exit $rc
