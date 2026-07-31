use std::cmp::Ordering;

use semver_mortis::{Range, RangeOptions, SemVer, min_version};

#[derive(Clone)]
struct Generator(u64);

impl Generator {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn below(&mut self, limit: u64) -> u64 {
        self.next() % limit
    }

    fn version(&mut self) -> String {
        let core = format!("{}.{}.{}", self.below(40), self.below(40), self.below(40));
        let prerelease = match self.below(5) {
            0 => "-alpha".to_owned(),
            1 => format!("-rc.{}", self.below(20)),
            2 => format!("-{}.beta", self.below(20)),
            _ => String::new(),
        };
        let build = match self.below(4) {
            0 => format!("+build.{}", self.below(20)),
            _ => String::new(),
        };
        format!("{core}{prerelease}{build}")
    }
}

#[test]
fn generated_versions_round_trip_and_order_antisymmetrically() {
    let mut generator = Generator(0x5eed_cafe_f00d_beef);
    for _ in 0..10_000 {
        let left_input = generator.version();
        let right_input = generator.version();
        let left = SemVer::parse(&left_input).unwrap();
        let right = SemVer::parse(&right_input).unwrap();
        let round_trip = SemVer::parse(left.version()).unwrap();

        assert_eq!(left.compare(&round_trip), Ordering::Equal);
        assert_eq!(left.major(), round_trip.major());
        assert_eq!(left.minor(), round_trip.minor());
        assert_eq!(left.patch(), round_trip.patch());
        assert_eq!(left.prerelease(), round_trip.prerelease());
        assert_eq!(left.compare(&right), right.compare(&left).reverse());
    }
}

#[test]
fn generated_ordering_is_transitive() {
    let mut generator = Generator(0xd1ff_3e12_3456_7890);
    for _ in 0..5_000 {
        let mut versions = [
            SemVer::parse(&generator.version()).unwrap(),
            SemVer::parse(&generator.version()).unwrap(),
            SemVer::parse(&generator.version()).unwrap(),
        ];
        versions.sort();
        assert!(versions[0] <= versions[1]);
        assert!(versions[1] <= versions[2]);
        assert!(versions[0] <= versions[2]);
    }
}

#[test]
fn generated_ranges_preserve_behavior_after_normalization() {
    let mut generator = Generator(0xa11c_e55e_1234_5678);
    for _ in 0..2_000 {
        let major = generator.below(8);
        let minor = generator.below(8);
        let patch = generator.below(8);
        let range_text = match generator.below(6) {
            0 => format!("^{major}.{minor}.{patch}"),
            1 => format!("~{major}.{minor}"),
            2 => format!("{major}.{minor}.x"),
            3 => format!(">={major}.{minor}.{patch} <{}.0.0", major + 1),
            4 => format!("{major}.{minor}.0 - {major}.{}.9", minor + 1),
            _ => format!("^{major}.{minor}.{patch} || {}.x", major + 2),
        };
        let options = RangeOptions {
            include_prerelease: generator.below(2) == 1,
            ..RangeOptions::default()
        };
        let range = Range::parse_with_options(&range_text, options).unwrap();
        let normalized = Range::parse_with_options(range.range(), options).unwrap();
        for _ in 0..8 {
            let candidate = generator.version();
            assert_eq!(range.test(&candidate), normalized.test(&candidate));
        }
        if let Some(minimum) = min_version(&range_text, options) {
            assert!(range.test_version(&minimum));
        }
    }
}
