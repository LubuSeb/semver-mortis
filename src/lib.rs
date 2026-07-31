#![forbid(unsafe_code)]
//! A behaviorally compatible Rust port of `npm/node-semver`.

mod version;

pub use version::{Identifier, ParseError, SemVer};

/// The exact upstream revision used as the behavioral oracle.
pub const UPSTREAM_COMMIT: &str = "6e05b7637396ac66522cff8731f07cfe0ef49a29";

/// Parse a strict npm-compatible semantic version.
pub fn parse(input: &str) -> Result<SemVer, ParseError> {
    SemVer::parse(input)
}

/// Return npm's canonical version string when `input` is valid.
pub fn valid(input: &str) -> Option<String> {
    parse(input)
        .ok()
        .map(|version| version.version().to_owned())
}
