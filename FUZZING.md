# Differential fuzzing

`scripts/fuzz-differential.js` deterministically generates strict versions,
invalid candidates, prereleases, build metadata, and composed npm ranges. Each
case compares the Rust CLI directly with the pinned `npm/node-semver` oracle.

Kickoff run (seed `0x5eedc0de`, `FUZZ_CASES=2000`):

```text
differential fuzz: 6000/6000 generated oracle checks passed
```

The 6,000 checks comprise 2,000 each of validation, precedence comparison, and
range satisfaction. To reproduce:

```sh
cargo build
NODE_SEMVER_ORACLE=/path/to/node-semver FUZZ_CASES=2000 npm run fuzz:differential
```
