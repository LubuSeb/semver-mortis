# 2–3 minute demo storyboard

Run `npm run demo` before recording. Keep the repository page visible beside
the terminal so every spoken claim has an inspectable artifact.

## 0:00–0:25 — The resurrection

“This is Semver Mortis: npm's node-semver 7.8.5 behavior, ported from JavaScript
to dependency-free, safe Rust during Port Mortem. It handles the full npm range
grammar—not just SemVer core—and forbids unsafe code.”

Show `Cargo.toml`, the `#![forbid(unsafe_code)]` line, and the top proof table.

## 0:25–1:05 — Behavior, including the awkward parts

Run `npm run demo`. Point out strict normalization, prerelease range gating,
right-to-left coercion, named prerelease increments, and range subset checks.

“These are native Rust calls. The CLI is intentionally command-oriented rather
than pretending to remain an npm binary.”

## 1:05–1:50 — Proof, not a promise

Run `npm run test:original` and show the final two lines:

```text
49 passed, 49 of 49 completed
8,787 passed, of 8,787
```

“Before Tap starts, the harness verifies all 66 copied upstream files against
the committed SHA-256 manifest. The JavaScript tests themselves are unchanged;
a thin test-only adapter drives one persistent Rust process. Separately, 1,144
frozen vectors, 17,000 property scenarios, and 6,000 deterministic oracle fuzz
checks all pass.”

Open `ORIGINAL_TESTS.md` briefly to show the two explicit exclusions.

## 1:50–2:20 — Why port it?

Show the benchmark table.

“On this machine the Rust port is 1.19 times faster at parsing, 2.35 times at
pre-parsed comparison, and 1.54 times at range evaluation. More importantly,
the shipped crate has no runtime dependencies and no unsafe escape hatch.”

## 2:20–2:35 — Close

“The commit history tells the build story slice by slice; the decision log
tells you where I traded scope, compatibility, and honesty. Clone it and one
`npm test` reproduces the complete evidence.”
