# Decision log

## D001 — Preserve behavior before optimizing

**Time:** 2026-07-31, immediately after kickoff

**Decision:** Port the public behavior in vertical slices, beginning with strict version parsing and comparison. Keep the upstream test corpus byte-for-byte unchanged and use the JavaScript implementation as a differential oracle.

**Why:** The judging rubric rewards defensible equivalence more than code volume. Small slices keep every commit testable and make semantic divergences easy to isolate.

**Consequences:** Optimization and ergonomic Rust-only APIs come after compatibility. The core crate forbids unsafe Rust.

