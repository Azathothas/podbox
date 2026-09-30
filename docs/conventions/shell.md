# Shell rules

Use the shell that owns the operation.
Do not pass a path found in PowerShell to another shell for removal.
Set the working directory explicitly for each command.
On Windows, use wsl-toolkit with instance podbox. Do not call the WSL executable.

## Arguments and files

Prefer an argument array to a command string.
Keep a prose payload in a file. Use a patch or a file-writing tool.
Do not place backticks or command substitutions inside shell-bound prose.
A quoted heredoc protects its body only in the shell that reads it.
Another shell can expand it first.

Write LF for shell, Python, Rust, and document files.
Follow .gitattributes for PowerShell files.
Use UTF-8 explicitly when Python reads subprocess output on Windows.
Keep literal control bytes out of tracked text. Write their escapes.

Resolve shims before using a native process API.
A .ps1 or .cmd path is not a native executable.
Do not use automatic PowerShell variable names for task variables.

## Exit status

Read a native command's status immediately from LASTEXITCODE in PowerShell.
Read a shell command's status immediately from $?.
Do not read a check's status through a pipeline.
If output must be captured, redirect it to a file and then read that file.

Under pipefail, a producer can fail when head closes its pipe.
Use a full consumer or a file when the producer's status matters.
grep -c returns 1 for zero matches. A zero count is not a command failure
unless the check defines it that way.
A printed failure word does not replace the process status.

## PowerShell

-match is case-insensitive. Use -cmatch when case carries meaning.
An integer cast rounds a floating-point value. Use Floor when required.
Read both child streams before waiting for its exit.
Use ConvertTo-Json with an explicit depth for nested records.
PowerShell aliases can differ from external tools with the same name.
Resolve the actual command type.

Use LiteralPath for file moves and removals.
Before a recursive operation, resolve the target and check that it stays
inside the owned workspace or explicitly named directory.
Do not construct a removal command from unchecked text.

## Windows path conversion

Git Bash can convert POSIX arguments before a Windows process sees them.
For engine payloads, use the shared engine helper.
Do not disable conversion for the toolkit wrapper's Windows script path.
The wrapper and engine have different argument contracts.

A native tool and Git Bash can use different temporary directories.
Use an explicit shared path when one writes a file that the other reads.
Avoid Windows device names as file names.

## Bounds and waits

Bound network, process, and tool-version calls.
Use non-interactive options and closed input where the command needs none.
A timeout means the command did not answer; it does not mean it is absent.
Wait on a job's output or status with a finite limit.
Keep progress updates separate from the process status.
Use a scheduler only when the operator requests later or recurring work.
