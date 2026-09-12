#!/usr/bin/env bash
# Question: can a `build.rs` invoke cargo, which is what embedding the two
# interposer objects by the shape TODO/interpose.md T-0702 recommends requires?
#
# ⭐ T-0702's `Approach` says "embed both", and the two objects are built by
# `scripts/build-interpose.sh` rather than by cargo, because the musl object
# needs `zig cc` as its linker and `podbox-interpose` is not a workspace member.
# So `include_bytes!` in `podbox-cli` needs the script to have run, and the
# entry's `Status note` recommends a `build.rs` that runs it.
#
# ⛔ THAT RECOMMENDATION HAS A HAZARD NOBODY HAS MEASURED. A `build.rs` runs
# INSIDE a cargo invocation, and cargo takes a lock on its package cache. A
# nested cargo that wants the same lock waits for a lock its own parent holds,
# which is a deadlock rather than a slow build. ⚠ A separate
# `CARGO_TARGET_DIR` does not help: the target directory and the package cache
# are different locks.
#
# So this measures the mechanism before the entry commits to it, on a fixture
# rather than on podbox: crate `outer` whose `build.rs` builds crate `inner`.
#
#   1. the nested build with no help                      the recommendation as written
#   2. the nested build with `CARGO_TARGET_DIR` separate   the usual first workaround
#
# ⛔ EVERY CLAUSE RUNS UNDER `timeout`. A deadlock is the answer this script
# exists to find, and a hang with no upper bound costs a session instead of
# reporting one. `AGENTS.md` requires the bound of anything that can wait.
#
#   ./158-interpose-embedding.sh
#
# Exit: 0 the measurement ran and a nested build completed, 1 it ran and every
#       shape failed or hung, 2 it could not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
OUT="$REPO/experiments/results/interpose-embedding.txt"
BOUND="${PODBOX_EMBED_TIMEOUT:-120}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT INT TERM

command -v cargo >/dev/null 2>&1 || {
	echo "SKIP: no cargo on PATH." >&2
	exit 2
}
command -v timeout >/dev/null 2>&1 || {
	echo "SKIP: no timeout(1), and an unbounded wait is not permitted." >&2
	exit 2
}

fail=0
completed=0
REPORT="$WORK/report"
say() { printf '%s\n' "$*" >>"$REPORT"; }

{
	echo "== conditions"
	printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'host kernel       %s\n' "$(uname -sr)"
	printf 'cargo             %s\n' "$(cargo --version 2>/dev/null || echo -)"
	printf 'commit            %s\n' "$(git -C "$REPO" rev-parse --short HEAD 2>/dev/null || echo -)"
	printf 'bound per clause  %s s\n' "$BOUND"
	echo
} >"$REPORT"

# ⚠ The fixture is two crates and no workspace, so nothing here shares a lock
# with podbox's own build and a deadlock cannot be blamed on this tree.
mk_fixture() {
	mkdir -p "$WORK/inner/src" "$WORK/outer/src"
	cat >"$WORK/inner/Cargo.toml" <<'EOF'
[package]
name = "inner"
version = "0.0.0"
edition = "2021"

[workspace]
EOF
	echo 'pub fn one() -> u8 { 1 }' >"$WORK/inner/src/lib.rs"

	cat >"$WORK/outer/Cargo.toml" <<'EOF'
[package]
name = "outer"
version = "0.0.0"
edition = "2021"
build = "build.rs"

[workspace]
EOF
	echo 'fn main() { println!("{}", inner_bytes().len()); }' >"$WORK/outer/src/main.rs"
	echo 'fn inner_bytes() -> &'"'"'static [u8] { b"placeholder" }' >>"$WORK/outer/src/main.rs"
}

#   $1 clause number   $2 what it changes   $3 an extra .env() call, or empty
nested() {
	n="$1"
	label="$2"
	extra="${3:-}"
	rm -rf "$WORK/outer/target" "$WORK/inner/target"
	cat >"$WORK/outer/build.rs" <<EOF
fn main() {
    // ⛔ The nested cargo, which is the whole subject. Its output goes to
    // stderr so a caller sees it even when cargo swallows stdout.
    let out = std::process::Command::new(std::env::var("CARGO").unwrap_or("cargo".into()))
        .args(["build", "--release", "--manifest-path", "$WORK/inner/Cargo.toml"])
        $extra
        .output()
        .expect("spawning the nested cargo");
    eprintln!("nested cargo exited {:?}", out.status.code());
    eprintln!("{}", String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "the nested cargo did not succeed");
}
EOF
	# ⛔ THE GUARD, AND IT IS HERE BECAUSE THIS SCRIPT NEEDED IT. The first
	# version referred to `$3` after a `shift`, so under `set -u` the heredoc
	# aborted, `build.rs` was written truncated, and the clause reported
	# `main function not found` as though a lock had been measured. A fixture
	# that did not compile for a reason of its own is not a measurement, and
	# `scripts/plant.sh` exists for the same class of mistake one layer up.
	if ! grep -q "^fn main() {" "$WORK/outer/build.rs" ||
		! grep -q "nested cargo exited" "$WORK/outer/build.rs"; then
		say "== clause $n  $label"
		say "  ⛔ COULD NOT RUN: the fixture's build.rs was not written whole,"
		say "  so nothing below would have been about cargo's locks."
		say ""
		fail=1
		return
	fi
	say "== clause $n  $label"
	s=$(date +%s)
	timeout "$BOUND" cargo build --release \
		--manifest-path "$WORK/outer/Cargo.toml" >"$WORK/clause$n.log" 2>&1
	# ⛔ Read here, unpiped, from the process that produced it.
	rc=$?
	e=$(date +%s)
	say "  exit             $rc  after $((e - s)) s"
	case "$rc" in
	0)
		say "  ⭐ the nested build COMPLETED"
		completed=$((completed + 1))
		;;
	124)
		say "  ⛔ TIMED OUT at $BOUND s, which is what a lock held by the parent"
		say "     looks like from outside"
		;;
	*)
		say "  ⚠ it failed rather than hung, and the reason is below"
		grep -E "error|panicked|nested cargo exited" "$WORK/clause$n.log" |
			head -8 | sed 's/^/    /' >>"$REPORT"
		;;
	esac
	say ""
}

mk_fixture
nested 1 "the nested build with no help" ""
nested 2 "the nested build with CARGO_TARGET_DIR separate" \
	".env(\"CARGO_TARGET_DIR\", \"$WORK/nested-target\")"

say "== what the clauses support"
if [ "$completed" -gt 0 ]; then
	say "  a build.rs CAN invoke cargo on this host, in $completed of 2 shapes."
	say "  ⚠ That is this host on this day. The hazard is real on others, and"
	say "  TODO/interpose.md T-0702 carries what podbox decided to do with it."
else
	say "  ⛔ NO SHAPE COMPLETED, so the recommendation in T-0702's Status note"
	say "  cannot be implemented as written, and the entry records that rather"
	say "  than a shape nobody ran."
	fail=1
fi
say ""
say "⚠ What this cannot say: that podbox's own script behaves the same. It"
say "  builds two targets and invokes a linker, so it is slower and it needs"
say "  zig; this fixture isolates the LOCK question and nothing else."

cat "$REPORT"
mkdir -p "$(dirname "$OUT")"
cp "$REPORT" "$OUT"
echo
echo "written to ${OUT#"$REPO"/}"
if [ -d /out ]; then
	cp "$REPORT" /out/interpose-embedding.txt 2>/dev/null
fi
[ -x "$REPO/scripts/common/result-diff.sh" ] && "$REPO/scripts/common/result-diff.sh" "$OUT"

[ "$fail" -eq 0 ] || exit 1
exit 0
