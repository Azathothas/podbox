[CmdletBinding()]
param([switch]$Quiet)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$ledgerPath = Join-Path $projectRoot 'experiments/evidence-ledger.csv'
if (-not (Test-Path -LiteralPath $ledgerPath -PathType Leaf)) {
    Write-Error "missing evidence ledger: $ledgerPath"
    exit 2
}

$rows = @(Import-Csv -LiteralPath $ledgerPath)
$allowed = @('OBSERVED','CODE','DERIVED','DESIGN','LIMIT','OPEN')
$ids = @{}
$fail = 0
$rootPrefix = [IO.Path]::GetFullPath($projectRoot) + [IO.Path]::DirectorySeparatorChar
foreach ($row in $rows) {
    if ($ids.ContainsKey($row.id)) {
        Write-Output "FAIL duplicate id $($row.id)"
        $fail++
        continue
    }
    $ids[$row.id] = $true
    if ($row.class -notin $allowed) {
        Write-Output "FAIL $($row.id) invalid class $($row.class)"
        $fail++
    }
    if ([IO.Path]::IsPathRooted($row.path) -or $row.path -match '(^|[\\/])\.\.([\\/]|$)') {
        Write-Output "FAIL $($row.id) non-local path $($row.path)"
        $fail++
        continue
    }
    $path = [IO.Path]::GetFullPath((Join-Path $projectRoot $row.path))
    if (-not $path.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        Write-Output "FAIL $($row.id) path escapes repository"
        $fail++
        continue
    }
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        Write-Output "FAIL $($row.id) missing path $($row.path)"
        $fail++
        continue
    }
    $content = Get-Content -Raw -LiteralPath $path
    if (-not $content.Contains($row.pattern)) {
        Write-Output "FAIL $($row.id) anchor missing in $($row.path): $($row.pattern)"
        $fail++
    } elseif (-not $Quiet) {
        Write-Output "PASS $($row.id) $($row.class) $($row.path)"
    }
    if ([string]::IsNullOrWhiteSpace($row.condition)) {
        Write-Output "FAIL $($row.id) missing condition"
        $fail++
    }
}
if ($rows.Count -lt 30) {
    Write-Output "FAIL evidence ledger unexpectedly small: $($rows.Count)"
    $fail++
}
if ($fail) {
    Write-Output "SUMMARY evidence=$($rows.Count) FAIL=$fail"
    exit 1
}
Write-Output "SUMMARY PASS evidence=$($rows.Count)"
exit 0
