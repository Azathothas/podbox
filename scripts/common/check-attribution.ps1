# check-attribution.ps1 - does any commit in this repository credit a tool?
#
# ⭐ THE TWIN OF check-attribution.sh. Same schema, same exit codes, same three
# rules. check-twins.sh is what stops the two drifting.
#
# ⛔ THE DEFECT: a commit message that names the tool that wrote it.
# docs/conventions/git.md states the rule, states that it overrides whatever a
# session's own default asks for, and says in as many words that a tool
# enforcing it mechanically beats a rule anyone has to remember. Nothing
# enforced it.
#
# ⛔ THIS CHECK CANNOT PREVENT THE THING IT NAMES, AND THE HOOK IS WHY. It
# reads `git log`, so at the moment a gate runs, the commit being made does not
# exist yet. ⭐ `-Message FILE` applies the same rules to a message that is not
# a commit yet, which is what dotfiles/githooks/commit-msg calls.
#
# ⚠ The emoji rule upstream carries is deliberately not here; the sh twin's
# header carries the measurement that rules it out for this repository.
#
# Usage:
#   pwsh -NoProfile -File scripts/common/check-attribution.ps1
#   pwsh -NoProfile -File scripts/common/check-attribution.ps1 -Json
#   pwsh -NoProfile -File scripts/common/check-attribution.ps1 -Message .git/COMMIT_EDITMSG
#
# Exit codes: 0 clean, 1 something credits a tool, 2 could not run.
#
# ⛔ Read the exit code from this process, unpiped.

[CmdletBinding()]
param(
    [switch]$Json,
    [string]$Message = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    [Console]::Error.WriteLine('check-attribution: git not found')
    exit 2
}
$root = (& git rev-parse --show-toplevel 2>$null)
if ($LASTEXITCODE -ne 0 -or -not $root) {
    [Console]::Error.WriteLine('check-attribution: not a git repository')
    exit 2
}
$root = ($root | Select-Object -First 1).Trim()

# The tokens that make a line a CREDIT rather than prose. ⚠ A list is a thing
# to forget, and this one is short on purpose: the trailer and generated-with
# shapes catch the STRUCTURE of a credit as well as its wording.
$names = @(
    'claude', 'anthropic', 'opus', 'sonnet', 'haiku',
    'copilot', 'chatgpt', 'openai', 'gpt-4', 'gpt-5', 'gemini', 'codex',
    'cursor', 'devin', 'aider'
)
$shapes = @('generated with', 'created with', 'written by ai', 'authored by ai')

$RS = [char]30
$US = [char]31

$problems = New-Object System.Collections.ArrayList
function Add-Problem([string]$Text) { [void]$script:problems.Add('  ' + $Text) }
$script:problems = $problems

function Test-Word([string]$Hay, [string]$Needle) {
    return ($Hay -match ('(^|[^a-z0-9_])' + [regex]::Escape($Needle) + '([^a-z0-9_]|$)'))
}
function Get-Trim48([string]$s) {
    $s = $s.Trim()
    if ($s.Length -gt 48) { return $s.Substring(0, 45) + '...' }
    return $s
}

$records = @()
if ($Message) {
    if (-not (Test-Path -LiteralPath $Message -PathType Leaf)) {
        [Console]::Error.WriteLine("check-attribution: no such message file: $Message")
        exit 2
    }
    # ⛔ COMMENT LINES ARE STRIPPED, AND ONLY COMMENT LINES. git's own template
    # puts the branch name and the staged file list behind `#`, none of which
    # reaches the stored message. Refusing a message over git's own scaffolding
    # is how a hook becomes something people turn off.
    $kept = @([System.IO.File]::ReadAllText($Message) -split "`r?`n" | Where-Object { -not $_.StartsWith('#') })
    $body = ($kept -join "`n")
    $subject = ($kept | Where-Object { $_ -ne '' } | Select-Object -First 1)
    if ($null -eq $subject) { $subject = '' }
    $records = @(, @('pending', $subject, $body))
}
else {
    # ⛔ THE RECORDS ARE SEPARATED BY CONTROL BYTES, NOT BY NEWLINES. A commit
    # body holds newlines and blank lines by design, so any line-oriented split
    # tears one in half and the halves are then read as two messages.
    $raw = (& git log --format="%x1e%H%x1f%s%x1f%B" 2>$null) -join "`n"
    if ($LASTEXITCODE -ne 0) {
        [Console]::Error.WriteLine('check-attribution: git log gave no history')
        exit 2
    }
    foreach ($rec in ($raw -split $RS)) {
        if (-not $rec) { continue }
        $f = $rec -split $US
        if ($f.Count -lt 3) {
            Add-Problem 'a commit record did not split into three fields, so it was not read'
            continue
        }
        $bodyParts = $f[2..($f.Count - 1)]
        $records += , @($f[0], $f[1], ($bodyParts -join $US))
    }
}

# ⛔ AN ABSENCE IS NOT A ZERO. A run that read no message at all would report a
# clean history because it had nothing to disagree with, which is the shape
# every dead check in this tree has had.
$checked = 0
foreach ($rec in $records) {
    $hash = $rec[0]; $subject = $rec[1]; $body = $rec[2]
    if (-not $hash) { continue }
    $checked++
    $short = if ($hash.Length -gt 8) { $hash.Substring(0, 8) } else { $hash }
    $lower = $body.ToLowerInvariant()

    # 1. The lines a tool appends.
    foreach ($s in $shapes) {
        if ($lower.Contains($s)) {
            Add-Problem ($short + ' (' + (Get-Trim48 $subject) + ') carries "' + $s + '"')
        }
    }

    # 2. A co-author trailer, and ONLY when it names a tool. A human co-author
    #    is a legitimate thing to record.
    $at = $lower.IndexOf('co-authored-by:')
    if ($at -ge 0) {
        $rest = $lower.Substring($at + 15)
        $nl = $rest.IndexOf("`n")
        if ($nl -ge 0) { $rest = $rest.Substring(0, $nl) }
        foreach ($n in $names) {
            if (Test-Word $rest $n) {
                Add-Problem ($short + ' (' + (Get-Trim48 $subject) + ') has a co-author trailer naming "' + $n + '"')
                break
            }
        }
    }

    # 3. A tool name anywhere in the body, trailer or not.
    foreach ($n in $names) {
        if (Test-Word $lower $n) {
            Add-Problem ($short + ' (' + (Get-Trim48 $subject) + ') names "' + $n + '" in its message')
        }
    }
}

if ($checked -eq 0) {
    [Console]::Error.WriteLine('check-attribution: read no commit at all, so it agreed with nothing')
    exit 2
}

# ⚠ THE HOOK STATE IS A NOTE, NOT A VERDICT. Hooks are not cloned, so a fresh
# checkout has none, and failing the gate over a local git setting would make
# every first run of this repository red.
$hooks = (& git config core.hooksPath 2>$null)
$hookOk = 0
if ($hooks) {
    $hooks = ($hooks | Select-Object -First 1).Trim()
    if (Test-Path -LiteralPath (Join-Path $root (Join-Path $hooks 'commit-msg')) -PathType Leaf) { $hookOk = 1 }
}

# ⛔ A SHALLOW CLONE MAKES THIS CHECK LOOK CLEAN OVER A HISTORY IT NEVER READ.
# The default checkout in a CI job fetches ONE commit, so a run there scans one
# message and reports success about the whole history. ⚠ Reported rather than
# failed: a shallow clone is legitimate, and failing would make every default
# workflow red.
$shallow = $false
if (-not $Message) {
    $shallow = ((& git rev-parse --is-shallow-repository 2>$null) -eq 'true')
}

# ⛔ AND WHETHER GIT WILL ACTUALLY RUN IT. git invokes a hook directly rather
# than through an interpreter, so a commit-msg without the executable bit is
# SKIPPED IN SILENCE on any POSIX host. ⚠ The bit that ships is the one in the
# INDEX: a Windows checkout does not carry it in the working tree, so asking
# the filesystem here would answer about the wrong thing.
$hookExec = $false
$hookNoExec = ''
foreach ($h in 'dotfiles/githooks/commit-msg', '.githooks/commit-msg') {
    $entry = (& git ls-files -s -- $h 2>$null)
    if (-not $entry) { continue }
    $mode = ($entry | Select-Object -First 1).Split(' ')[0]
    if ($mode -eq '100755') { $hookExec = $true }
    elseif ($mode -eq '100644') { $hookExec = $false; $hookNoExec = $h }
}

$count = $problems.Count

if ($Json) {
    # ⚠ CONCATENATED, NOT `-f`. PowerShell's format operator needs a doubled
    # brace to emit a literal one, which is the shape check-placeholders looks
    # for. Concatenation keeps the source honest and the output byte-identical
    # to the sh twin.
    Write-Output ('{"schema":"check-attribution/1","problems":' + $count +
        ',"commits":' + $checked + ',"hook_installed":' + $hookOk +
        ',"hook_executable":' + $(if ($hookExec) { 'true' } else { 'false' }) +
        ',"shallow":' + $(if ($shallow) { 'true' } else { 'false' }) + '}')
    if ($count -gt 0) { exit 1 }
    exit 0
}

if ($count -gt 0) {
    Write-Output ('a tool is credited in ' + $count + ' place(s):')
    Write-Output ''
    $problems | ForEach-Object { Write-Output $_ }
    Write-Output ''
    Write-Output 'docs/conventions/git.md: no tool is credited in a commit. The identity'
    Write-Output 'is the operator alone, and this overrides whatever default asked for'
    Write-Output 'the trailer. Edit the message and commit again.'
    exit 1
}

if ($Message) {
    Write-Output 'no tool credited in the pending message'
    exit 0
}

Write-Output ('no tool credited in ' + $checked + ' commit(s)')
if ($hookNoExec) {
    Write-Output ''
    Write-Output ('⚠ ' + $hookNoExec + ' is tracked WITHOUT the executable bit, so git skips it in')
    Write-Output '  silence on any POSIX host. Fix the mode in the index, not on disk:'
    Write-Output ''
    Write-Output ('    git update-index --chmod=+x ' + $hookNoExec)
}
if ($shallow) {
    Write-Output ''
    Write-Output '⚠ this clone is SHALLOW, so that is every commit that was fetched'
    Write-Output '  rather than every commit there is. A default CI checkout fetches'
    Write-Output '  one. Fetch the full history where the whole record is the scope.'
}
if ($hookOk -eq 0) {
    # ⚠ NAME THE DIRECTORY THAT IS ACTUALLY THERE. This template keeps the hook
    # under dotfiles/, and a project that copied it keeps its own; printing one
    # of those into the other's tree is an instruction that does nothing.
    $where = ''
    if (Test-Path -LiteralPath (Join-Path $root 'dotfiles/githooks/commit-msg')) { $where = 'dotfiles/githooks' }
    if (Test-Path -LiteralPath (Join-Path $root '.githooks/commit-msg')) { $where = '.githooks' }
    Write-Output ''
    Write-Output '⚠ the commit-msg hook is not installed here, so this can only report'
    if ($where) {
        Write-Output '  after the fact. Install it once, per checkout:'
        Write-Output ''
        Write-Output ('    git config core.hooksPath ' + $where)
    }
    else {
        Write-Output '  after the fact, and no commit-msg hook is in this tree to install.'
        Write-Output '  docs/conventions/git.md says where it belongs.'
    }
}
exit 0
