#![forbid(unsafe_code)]
//! A behaviorally compatible Rust port of `npm/node-semver`.

mod coerce;
mod comparator;
mod functions;
mod range;
mod version;

pub use coerce::{CoerceOptions, coerce, coerce_with_options};
pub use comparator::{Comparator, ComparatorError, ComparatorOperator};
pub use functions::{
    ComparisonOperator, InvalidComparisonOperator, cmp, compare, compare_build, compare_loose,
    diff, eq, gt, gte, lt, lte, major, minor, neq, patch, prerelease, rcompare, rsort, sort,
    truncate,
};
pub use range::{Range, RangeError, RangeOptions};
pub use version::{Identifier, IdentifierBase, IncrementError, ParseError, ReleaseType, SemVer};

/// The exact upstream revision used as the behavioral oracle.
pub const UPSTREAM_COMMIT: &str = "6e05b7637396ac66522cff8731f07cfe0ef49a29";

/// Parse a strict npm-compatible semantic version.
pub fn parse(input: &str) -> Result<SemVer, ParseError> {
    SemVer::parse(input)
}

/// Parse using npm's permissive grammar.
pub fn parse_loose(input: &str) -> Result<SemVer, ParseError> {
    SemVer::parse_loose(input)
}

/// Return npm's canonical version string when `input` is valid.
pub fn valid(input: &str) -> Option<String> {
    parse(input)
        .ok()
        .map(|version| version.version().to_owned())
}

/// Normalize a version after removing npm's conventional leading `=`/`v`.
pub fn clean(input: &str) -> Option<String> {
    clean_with_mode(input, false)
}

/// Normalize a version using npm's permissive grammar.
pub fn clean_loose(input: &str) -> Option<String> {
    clean_with_mode(input, true)
}

fn clean_with_mode(input: &str, loose: bool) -> Option<String> {
    let cleaned = input.trim().trim_start_matches(['=', 'v']);
    let parsed = if loose {
        SemVer::parse_loose(cleaned)
    } else {
        SemVer::parse(cleaned)
    };
    parsed.ok().map(|version| version.version().to_owned())
}

/// Increment a strict version with npm's default prerelease numbering.
pub fn inc(input: &str, release: ReleaseType) -> Option<String> {
    inc_with_options(input, release, false, None, IdentifierBase::Zero)
}

/// Increment a version with full control over npm's prerelease behavior.
pub fn inc_with_options(
    input: &str,
    release: ReleaseType,
    loose: bool,
    identifier: Option<&str>,
    identifier_base: IdentifierBase,
) -> Option<String> {
    let mut version = if loose {
        SemVer::parse_loose(input).ok()?
    } else {
        SemVer::parse(input).ok()?
    };
    version
        .increment(release, identifier, identifier_base)
        .ok()?;
    Some(version.version().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_upstream_examples() {
        for (input, expected) in [
            ("1.2.3", Some("1.2.3")),
            ("  =v1.2.3   ", Some("1.2.3")),
            ("0.12.0-dev.1150+3c22cecee", Some("0.12.0-dev.1150")),
            (">1.2.3", None),
            ("1.2.x", None),
        ] {
            assert_eq!(clean(input).as_deref(), expected);
        }
    }

    #[test]
    fn accepts_npm_loose_forms_only_in_loose_mode() {
        for (input, expected) in [
            ("= 1.2.3", "1.2.3"),
            ("01.02.03", "1.2.3"),
            ("1.2.3tag", "1.2.3-tag"),
            ("1.2.3-01", "1.2.3-1"),
        ] {
            assert!(parse(input).is_err(), "strictly accepted {input:?}");
            assert_eq!(parse_loose(input).unwrap().version(), expected);
        }
    }
}
