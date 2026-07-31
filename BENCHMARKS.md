# Benchmark snapshot

Recorded 2026-07-31 on an AMD Ryzen 9 3950X with Rust 1.97.1 and Node 24.13.0.
Each result is the median of five 1,000,000-operation samples after warmup.

| Operation | Rust port | Pinned node-semver | Relative throughput |
| --- | ---: | ---: | ---: |
| strict parse | 793.9 ns/op | 947.7 ns/op | 1.19x |
| pre-parsed compare | 2.6 ns/op | 6.1 ns/op | 2.35x |
| pre-parsed range test | 620.0 ns/op | 955.7 ns/op | 1.54x |

These are focused microbenchmarks, not application-level latency claims. Re-run
the Rust side with `cargo bench --bench throughput`. To compare the pinned
JavaScript oracle, set `NODE_SEMVER_ORACLE` to its checkout and run
`npm run bench:node`.
