#!/usr/bin/env sh
# Bisect v2: dump the ground-truth file set plus the script side's
# values/conditions JSON, and a from-scratch reimplementation of the
# binary's walk+render over hashlib, to isolate why the two digests
# differ on a scratch root while agreeing on /work.
set -u

OUT=/out/bisect2-proof.log
: >"$OUT" || exit 2

say() { printf '%s\n' "$*" >>"$OUT"; }
fail=0
miss() { fail=1; say "  FAIL: $1"; }

BIN1=""
cargo build --release -p podbox-buildstate >>"$OUT" 2>&1
for cand in /work/target/release/podbox-buildstate /work/target/x86_64-unknown-linux-musl/release/podbox-buildstate /work/target/debug/podbox-buildstate; do
  [ -x "$cand" ] && BIN1="$cand" && break
done
[ -n "$BIN1" ] || { miss "no podbox-buildstate binary"; say "verdict BISECT2-FAIL"; exit 1; }
git show HEAD:scripts/build-state.py >"$OUT.orig.py" 2>&1

S4="$(mktemp -d)"
mkdir -p "$S4/crates/a"
echo "scratch-input" >"$S4/crates/a/f.rs"
for n in podbox node operator proxy shell; do echo "scratch-out" >"$S4/$n"; done

say "== ground truth: every entry under the scratch root"
find "$S4" -mindepth 1 | sort >>"$OUT" 2>&1
say "== hidden entries, if any"
find "$S4" -mindepth 1 -name ".*" >>"$OUT" 2>&1 || true
say "== symlinks, if any"
find "$S4" -mindepth 1 -type l >>"$OUT" 2>&1 || true
say ""

say "== script side values+conditions as JSON"
python3 - "$S4" >>"$OUT" 2>&1 <<'PYEOF'
import hashlib, json, os, subprocess, sys
from pathlib import Path
root = Path(sys.argv[1]).resolve()
paths = set()
for name in ("crates", "vendor", "scripts", ".cargo"):
    base = root / name
    if base.exists():
        for path in base.rglob("*"):
            relative = path.relative_to(root)
            if path.is_file() and not any(part in ("target", "__pycache__") for part in relative.parts):
                paths.add(relative.as_posix())
for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
    if (root / name).is_file():
        paths.add(name)
def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()
values = {name: digest(root / name) for name in sorted(paths)}
conditions = {"target": "t"}
keys = {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CC", "AR", "TARGETS"}
keys.update(key for key in os.environ if key.startswith(("CARGO_TARGET_", "CC_", "AR_")))
for key in sorted(keys):
    conditions[key] = os.environ.get(key, "")
conditions['build_commit'] = 'script-placeholder'
print("VALUES " + json.dumps(values, sort_keys=True))
print("CONDS " + json.dumps(conditions, sort_keys=True))
PYEOF
say ""

say "== reimplemented binary walk+render over hashlib"
python3 - "$S4" >>"$OUT" 2>&1 <<'PYEOF'
import hashlib, json, os, sys
from pathlib import Path
root = Path(sys.argv[1])
paths = set()
def walk(base):
    for entry in sorted(os.listdir(base)):
        path = base / entry
        try:
            st = os.lstat(path)
        except OSError:
            continue
        import stat as statmod
        if statmod.S_ISDIR(st.st_mode):
            walk(path)
        elif statmod.S_ISREG(st.st_mode):
            relative = path.relative_to(root)
            if all(part not in ("target", "__pycache__") for part in relative.parts):
                paths.add(relative.as_posix().replace("\\", "/"))
for name in ("crates", "vendor", "scripts", ".cargo"):
    base = root / name
    if base.is_dir() and not base.is_symlink():
        walk(base)
for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
    if (root / name).is_file():
        paths.add(name)
def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()
values = {name: digest(root / name) for name in sorted(paths)}
print("VALUES " + json.dumps(values, sort_keys=True))
PYEOF
say ""

say "== the two digests again"
A="$("$BIN1" inputs --root "$S4" --target t 2>>"$OUT")"
say "  binary inputs: [$A]"
B="$(python3 "$OUT.orig.py" inputs --root "$S4" --target t 2>>"$OUT")"
say "  script inputs: [$B]"
rm -rf "$S4"
say ""

if [ "$fail" -eq 0 ]; then say "verdict BISECT2-OK"; else say "verdict BISECT2-FAIL"; fi
[ "$fail" -eq 0 ] || exit 1
exit 0
