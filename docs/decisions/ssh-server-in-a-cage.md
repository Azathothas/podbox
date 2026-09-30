# SSH server selection in a restricted host

Status: reference evidence and current server contract.

A server configuration check does not prove a session.
The selected server must complete a real SSH handshake and run the command.

The reference cage at sandssh commit `4fc7f8c` rejected OpenSSH privilege
separation during execution. A configuration test had returned zero.
That result applies to that cage and build. It is not a universal statement
about every OpenSSH build.

The reference used a dynamic Dropbear build, a passwd shim, and an
`initgroups` tolerance patch. Static binaries cannot use the preload shim.
A change to the patch must retain the uid and gid checks.

podbox accepts an explicit server and configuration.
Read the current helper's usage before invocation.
The transport does not create a PTY or add host privileges.
The interactive session layer provides line handling where its proof applies;
it does not create a missing terminal device.

[T-1402](../../TODO/podssh.md) records the interactive proof.
[T-1404](../../TODO/podssh.md) records the remote and machine proof.
The copied shim and its notice are listed in
[THIRD_PARTY](../../THIRD_PARTY.md).
Do not infer scp, arbitrary server compatibility, or a cage session from
a separate transport test.
