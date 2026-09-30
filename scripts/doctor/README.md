# Environment probe

The doctor reports the host, tools, repository, and probe limits.
It is read-only. It does not install a tool or prove a build.

```sh
sh scripts/doctor/doctor.sh
```

```powershell
pwsh -NoProfile -File scripts/doctor/doctor.ps1
```

Use the PowerShell form for a native Windows probe.
Use the shell form where the POSIX tools are present.
The twins share the agent-doctor/1 output schema.

A timed-out tool is reported separately from a missing tool.
A shim is resolved through its correct interpreter.
Tool presence does not prove that its external service is available.

The sh and PowerShell probes can find different PATH entries.
Do not infer one shell's tool access from the other shell.
The twin check compares stable host, repository, and schema fields;
it does not require every tool lookup to have the same result.

[Tool map](../../docs/agent-tooling.md) gives project commands.
[Container lanes](../../docs/containers.md) gives the build route.
