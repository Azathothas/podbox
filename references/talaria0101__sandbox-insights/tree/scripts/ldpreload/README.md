# ldpreload — the interposition-reach fixture

One shim, three victim payload classes, the same two syscalls each.
Experiment `../../experiments/35-interposition-reach.sh` builds and
drives them; the build lines are also here:

```sh
cc -O2 -shared -fPIC -o shim.so shim.c -ldl
cc -O2 -o victim_dyn victim_dyn.c
cc -O2 -static -o victim_static victim_static.c
go build -o victim_go victim_go.go

LD_PRELOAD=$PWD/shim.so ./victim_dyn      # intercepted
LD_PRELOAD=$PWD/shim.so ./victim_static   # invisible: no loader
LD_PRELOAD=$PWD/shim.so ./victim_go       # invisible: raw syscalls
```

The lesson the fixture proves: interposition coverage is a property of
the payload, and seeing a call is not clearing it — the intercepted
`lchown(0, 42)` still answers `EINVAL` from the mapping check.
