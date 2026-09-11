[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$experimentRoot = $PSScriptRoot
$data = Join-Path $experimentRoot 'data/benchmark-samples.csv'
$logDir = Join-Path $experimentRoot 'logs'
$log = Join-Path $logDir '40-tradeoff-model.log'
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

if (-not (Test-Path -LiteralPath $data -PathType Leaf)) {
    Write-Output 'could not run: local benchmark samples are missing'
    exit 2
}

$rows = @(Import-Csv -LiteralPath $data)
$native = @($rows | Where-Object route -eq 'native' | ForEach-Object { [double]$_.mops_per_second })
$chroot = @($rows | Where-Object route -eq 'chroot' | ForEach-Object { [double]$_.mops_per_second })
$guest = @($rows | Where-Object route -eq 'microvm-tcg' | ForEach-Object { [double]$_.mops_per_second })
$checksums = @($rows.checksum | Select-Object -Unique)
$ratios = @()
foreach ($n in @($native + $chroot)) {
    foreach ($g in $guest) { $ratios += $n / $g }
}
$min = ($ratios | Measure-Object -Minimum).Minimum
$max = ($ratios | Measure-Object -Maximum).Maximum

$lines = @(
    '## question: do repository-local benchmark samples derive the reported TCG/native interval?'
    "date_utc=$([DateTime]::UtcNow.ToString('o'))"
    "native_mops=$($native -join ',')"
    "chroot_mops=$($chroot -join ',')"
    "microvm_tcg_mops=$($guest -join ',')"
    ('ratio_min={0:N2}' -f $min)
    ('ratio_max={0:N2}' -f $max)
    "checksums=$($checksums -join ',')"
)

$fail = 0
if ($native.Count -ne 2 -or $chroot.Count -ne 2 -or $guest.Count -ne 2) {
    $lines += 'FAIL unexpected sample count'
    $fail++
} else { $lines += 'PASS sample counts=2/2/2' }
if ($checksums.Count -ne 1 -or $checksums[0] -ne '165be307') {
    $lines += 'FAIL checksum control'
    $fail++
} else { $lines += 'PASS checksum control' }
if ($min -lt 20.0 -or $max -gt 25.0) {
    $lines += 'FAIL ratio outside 20-25x interval'
    $fail++
} else { $lines += 'PASS ratio within 20-25x interval' }

$rc = if ($fail) { 1 } else { 0 }
$lines += "exit=$rc"
$lines | Set-Content -LiteralPath $log -Encoding utf8
$lines
exit $rc
