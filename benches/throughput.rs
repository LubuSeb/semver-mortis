use std::hint::black_box;
use std::time::{Duration, Instant};

use semver_mortis::{Range, SemVer};

const ITERATIONS: u64 = 1_000_000;
const SAMPLES: usize = 5;

fn main() {
    benchmark("parse strict", || {
        black_box(SemVer::parse(black_box("2.17.4-rc.12+build.99")).unwrap());
    });

    let left = SemVer::parse("2.17.4-rc.12+build.99").unwrap();
    let right = SemVer::parse("2.17.4").unwrap();
    benchmark("compare", || {
        black_box(black_box(&left).compare(black_box(&right)));
    });

    let range = Range::parse("^2.4.0 || >=3.1.0 <4.0.0").unwrap();
    benchmark("range test", || {
        black_box(black_box(&range).test(black_box("3.5.7-rc.1")));
    });
}

fn benchmark(name: &str, mut operation: impl FnMut()) {
    for _ in 0..10_000 {
        operation();
    }
    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            operation();
        }
        samples.push(start.elapsed());
    }
    samples.sort();
    report(name, samples[SAMPLES / 2]);
}

fn report(name: &str, elapsed: Duration) {
    let nanoseconds = elapsed.as_nanos() as f64 / ITERATIONS as f64;
    let operations_per_second = 1_000_000_000.0 / nanoseconds;
    println!(
        "{name:12} {nanoseconds:10.1} ns/op  {operations_per_second:12.0} ops/s  (median of {SAMPLES})"
    );
}
