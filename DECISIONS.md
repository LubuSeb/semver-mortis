# Decision log

This log records consequential choices made during the timed Port Mortem build.

## D001 — Preserve behavior before optimizing

**Time:** 2026-07-31, immediately after kickoff

**Decision:** Port public behavior in vertical slices, beginning with strict
version parsing and comparison. Keep the upstream test corpus byte-for-byte
unchanged and use the JavaScript implementation as a differential oracle.

**Why:** Behavioral equivalence is easier to defend when each commit adds one
coherent slice and its proof. It also makes divergences easy to isolate.

**Consequence:** Optimization and Rust-only ergonomics follow compatibility;
the git history remains an audit trail rather than one large import.

## D002 — Use an explicit parser, not a regex-table transliteration

**Time:** 2026-07-31, strict and loose parsing phase

**Decision:** Implement bounded component parsing and range expansion directly
in Rust instead of recreating node-semver's generated JavaScript regex table.

**Why:** The data model and invariants become visible in types and branches,
and long-input behavior does not depend on regex backtracking.

**Consequence:** Some JavaScript reflection fields (`re`, `src`, `tokens`) have
only a small test-adapter compatibility surface. They are not part of the Rust
API. The upstream 500,000-character whitespace regression suite still runs
unchanged against the port.

## D003 — Make safe, dependency-free Rust a hard constraint

**Time:** 2026-07-31, repository bootstrap

**Decision:** Use only the standard library and enforce `unsafe_code =
"forbid"` in both crate source and Cargo lint configuration.

**Why:** SemVer is small, security-sensitive parsing infrastructure. A tiny
supply-chain and auditable memory-safety boundary are meaningful advantages of
the port.

**Consequence:** The implementation owns its parsing, error, CLI, property-test,
and benchmark machinery. Tap is a pinned development-only dependency used to
execute the historical upstream JavaScript tests; it is not linked or shipped.

## D004 — Run original tests through a persistent test-only bridge

**Time:** 2026-07-31, after the public Rust API reached range parity

**Decision:** Leave `tests/original/` untouched and place matching CommonJS
module paths in `tests/`. A synchronous worker bridge holds one Rust CLI process
open and translates the JavaScript-facing calls.

**Why:** Direct fixture extraction proves values; unchanged test execution also
proves constructor behavior, mutation, errors, caching, and JavaScript API edge
semantics. Keeping a process alive avoids turning thousands of assertions into
thousands of process starts.

**Consequence:** The bridge is proof infrastructure, never production code.
Every run verifies SHA-256 first. The result is reported as 49 unchanged
non-CLI behavior suites, not as a drop-in npm implementation.

## D005 — Separate semantic parity from Node CLI parity

**Time:** 2026-07-31, proof-harness completion

**Decision:** Exclude upstream `bin/semver.js`, whose snapshots specify the
Node wrapper, and `map.js`, which checks node-semver's JavaScript file topology.
Replace the former with native Rust CLI integration tests.

**Why:** Passing either unchanged would require preserving delivery-language
machinery rather than ported behavior. Counting them would overstate the claim.

**Consequence:** [ORIGINAL_TESTS.md](ORIGINAL_TESTS.md) lists the exact 49/51
scope. Native CLI behavior is independently exercised by `tests/cli.rs`.

## D006 — Optimize only with comparable pre-parsed workloads

**Time:** 2026-07-31, benchmark phase

**Decision:** Compare parse, pre-parsed comparison, and pre-parsed range test
using identical data sets, warmup, and five one-million-operation samples.

**Why:** Mixing parse cost into only one side or comparing process startup would
produce an attractive but meaningless number.

**Consequence:** The reported 1.19x–2.35x figures are labeled focused
microbenchmarks and retain raw reproduction commands in `BENCHMARKS.md`.
