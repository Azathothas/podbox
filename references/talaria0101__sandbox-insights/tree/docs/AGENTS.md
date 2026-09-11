# AGENTS.md — sandbox-insights

Router, not a rulebook. Everything binding lives in the file a row
names; the link is the authority. Find the row for the work in front
of you and read what it names, in full.

**sandbox-insights** is a measured characterization of one class of
Linux runtime — a container sandbox that presents uid 0 with a full
capability set while refusing most of what container, virtualization
and debugging tooling is built on — plus the instrument library that
produced the measurements, and the working recipes for everything that
still runs there. `docs/paper.md` is the distilled write-up.

---

## Start here, every session

1. **Probe the host before believing any claim about it.** This
   runtime class is reconfigured by its operator on a day scale; a
   fact measured last week may be false today (the mount-API filter
   gap was closed between two sessions; the namespace gap outlived
   it). Run the census first:

   ```sh
   sh experiments/10-environment-census.sh    # identity + mechanism census
   sh experiments/15-network-policy.sh        # the network envelope
   ```

   Compare against `docs/environment.md`. Where they disagree, the
   host is right and the document gets amended in the same change.

2. **The gates** (see *The gate*) before claiming anything works.

## The routing table

| the task | read, in this order |
| --- | --- |
| **Understanding the environment** | [`docs/environment.md`](environment.md) (the measured contract), `experiments/logs/10-` (the census that produced it) |
| **Attributing a denial to a mechanism** | [`docs/attribution.md`](attribution.md) — the N/F/M/P model, the bogus-argument discriminator, the probe protocol |
| **A tool fails and the errno misleads** | [`docs/walls.md`](walls.md) — the four recurring walls, their signatures, their fixes |
| **Making something run here** | [`docs/recipes.md`](recipes.md) — TCG microVMs, chroot appliances, ptrace-free tracing, LD_PRELOAD shims |
| **Designing a tool for this class** | [`docs/design-lessons.md`](design-lessons.md) — honest degradation, mode ladder, diagnostics |
| **The distillation** | [`docs/paper.md`](paper.md) |
| **Adding an experiment** | [`experiments/README.md`](../experiments/README.md) — numbered scripts, pinned inputs, committed logs, exit 0/1/2 |
| **Reusing an instrument** | `scripts/` — every instrument is standalone, self-asserting, and prints its conditions |
| **Writing a commit** | as the configured bot identity; message says what changed and why |

## The absolutes

1. ⛔ **Never claim what a committed log does not back.** Every claim
   carries a tag: **[V]** verified in this tree by a runnable script,
   **[S]** established from pinned source read, **[C]** cross-checked
   against another instance of this runtime class. [V] outranks [S]
   outranks memory.
2. ⛔ **A probe must report the verdict of the operation it names.**
   Not a child's exit code; not a proxy; not "could not run" read as
   "denied". Exit 0 ran-and-held, 1 ran-and-failed, 2 could-not-run.
3. ⛔ **Negative results are committed** with the same conditions
   block as positive ones.
4. ⛔ **The host is re-audited continuously by its operator.** Filter
   gaps close without notice. Anything built on a gap must probe the
   gap at runtime, and any document that states a gap states its date.
5. ⛔ **Numbers carry conditions** — host, kernel, date, versions. A
   number without them is a rumour.
6. ⛔ **No secrets, no credentials, no tokens** in tree, logs, commits.
7. ⛔ **Edit documents in place.** A superseded claim is corrected
   where it stands, not annotated to death.
8. **Exit codes are read unpiped** from the process that produced them.

## The gate

A change is done when the experiments it touches pass live, with
output actually inspected:

```sh
sh experiments/10-environment-census.sh
sh experiments/20-mechanism-attribution.sh
sh experiments/45-senotif-primitives.sh
```

and every new experiment script has been run once, its log committed,
and its README entry written.

## The tree

| path | what is in it |
| --- | --- |
| `docs/` | the distillation: environment contract, mechanism attribution, walls, recipes, design lessons, the paper, the reviews |
| `docs/reviews/` | documented review passes (REVIEW-1..N), each a fresh read with its own lens |
| `scripts/` | the instrument library: census, discriminators, network policy probe, namespace-privilege probe, seccomp-notify primitive probe, spawn matrix, shims, benchmark, cpio writer |
| `experiments/` | numbered, runnable experiments + committed `logs/` (the evidence) |
| `work/` | regenerable artefacts — never committed (gitignored) |

## What a session owes at its end

* the census re-run and compared against `docs/environment.md`;
* new findings recorded where the routing table says they live;
* an honest verdict written down: what was verified, what was assumed,
  what remains. "Assume more remain" is the default closing state.
