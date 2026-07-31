# Semver Mortis

**npm's `node-semver` 7.8.5, resurrected as safe, dependency-free Rust.**

Built from scratch during Port Mortem 2026 (JavaScript to Rust) from upstream
commit [`6e05b76`](https://github.com/npm/node-semver/commit/6e05b7637396ac66522cff8731f07cfe0ef49a29).

| Proof | Result |
| --- | ---: |
| Unchanged upstream behavior suites | **49/49 suites, 8,787/8,787 assertions** |
| Frozen fixture and vector checks | **1,144/1,144** |
| Deterministic differential fuzz checks | **6,000/6,000** |
| Generated property scenarios | **17,000** |
| Runtime dependencies / unsafe blocks | **0 / 0** |

The port covers strict and loose parsing, precedence and build comparison,
coercion (LTR/RTL and prerelease), every npm increment mode, comparators, the
full npm range grammar, prerelease gating, intersections, subsets, min/max
satisfying, minimum versions, outside-range checks, simplification, sorting,
truncation, and the functional helpers exposed by `node-semver`.

## See it work

```sh
cargo run --bin semver-mortis -- valid 1.2.3-beta.1+build.7
# 1.2.3-beta.1

cargo run --bin semver-mortis -- satisfies 1.8.4 "^1.2.3 || >=3"
# true

cargo run --bin semver-mortis -- --identifier beta --identifier-base 1 inc 1.2.3 preminor
# 1.3.0-beta.1
```

For the paced demonstration used by the video storyboard:

```sh
npm run demo
```

See [DEMO.md](DEMO.md) for the 2–3 minute narrative.

## Reproduce the evidence

Rust-only verification needs no third-party crate:

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

The complete local proof run uses the same Tap version as the pinned upstream:

```sh
npm ci
npm test
```

`npm test` performs three independent layers:

1. Rust unit, integration, and deterministic property tests.
2. SHA-256 verification of all 66 frozen upstream files, followed by 49
   unchanged suites through a thin test-only JavaScript-to-Rust adapter.
3. 1,144 frozen fixture/vector comparisons driven directly through the CLI.

The adapter is test infrastructure, not part of the shipped Rust library. It
keeps one native process alive and forwards calls over a deterministic line
protocol, so the test code under `tests/original/` stays byte-identical. The
exact scope and the two upstream infrastructure/Node-CLI exclusions are in
[ORIGINAL_TESTS.md](ORIGINAL_TESTS.md).

For randomized oracle testing, point `NODE_SEMVER_ORACLE` at the pinned
checkout and run:

```sh
cargo build
NODE_SEMVER_ORACLE=/path/to/node-semver FUZZ_CASES=2000 npm run fuzz:differential
```

The kickoff run passed all 6,000 checks. Seed, categories, and reproduction
details are recorded in [FUZZING.md](FUZZING.md).

## Performance

Median time per operation, five 1,000,000-operation samples after warmup:

| Operation | Rust | node-semver | Rust throughput |
| --- | ---: | ---: | ---: |
| Strict parse | 793.9 ns | 947.7 ns | **1.19x** |
| Pre-parsed compare | 2.6 ns | 6.1 ns | **2.35x** |
| Pre-parsed range test | 620.0 ns | 955.7 ns | **1.54x** |

These are focused microbenchmarks rather than application-latency claims.
Machine details and commands are in [BENCHMARKS.md](BENCHMARKS.md).

## Design

- `SemVer` owns parsed identifiers and preserves raw/build metadata while
  keeping normal precedence build-agnostic.
- `Comparator` models a normalized operator/version pair; `Range` is an OR of
  AND comparator sets with npm-compatible prerelease gating.
- Parsing is bounded and explicit rather than a transliteration of the
  JavaScript regular-expression table.
- `#![forbid(unsafe_code)]` is enforced at crate root and by Cargo lint policy.
- The crate has no runtime dependencies; `Cargo.lock` contains only this
  package.

Tradeoffs and choices made during the timed build are recorded in
[DECISIONS.md](DECISIONS.md). Original-test provenance and hashes are in
[tests/PROVENANCE.md](tests/PROVENANCE.md).

## Honest boundary

This is a behavioral port with an idiomatic Rust library and native command
interface; it is not a drop-in npm package. JavaScript-only module-reflection
exports exist solely in the upstream test adapter. The upstream Node CLI
snapshot test is replaced by direct Rust CLI integration tests, not counted as
an unchanged-suite pass.
