# Semver Mortis

A from-scratch Rust port of [`npm/node-semver`](https://github.com/npm/node-semver), built during the 72-hour Port Mortem 2026 hackathon.

- Track G: JavaScript to Rust
- Kickoff: 2026-07-31 18:00 UTC
- Upstream commit: `6e05b7637396ac66522cff8731f07cfe0ef49a29`
- Goal: behavioral equivalence demonstrated by unchanged upstream tests, differential fuzzing, and benchmarks

This repository was created after kickoff. Implementation will land in small, auditable commits.

## Verification

```sh
cargo test
cargo clippy --all-targets -- -D warnings
npm test
```

`npm test` runs the Rust suite and then drives the compiled CLI through the
unchanged upstream fixtures and test vectors. It currently checks 1,027 core parsing, comparison,
increment, truncation, range parsing, inclusion/exclusion, outside-range, and
intersection/subset cases without requiring npm dependencies.
