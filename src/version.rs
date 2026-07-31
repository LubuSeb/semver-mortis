use std::cmp::Ordering;
use std::error::Error;
use std::fmt;
use std::str::FromStr;

const MAX_LENGTH: usize = 256;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// A prerelease identifier as exposed by `node-semver`: safe integers are
/// numeric, while all other identifiers remain strings.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Identifier {
    Numeric(u64),
    Text(String),
}

impl Identifier {
    fn parse(value: &str) -> Self {
        match value.parse::<u64>() {
            Ok(number) if number < MAX_SAFE_INTEGER => Self::Numeric(number),
            _ => Self::Text(value.to_owned()),
        }
    }

    fn as_text(&self) -> String {
        match self {
            Self::Numeric(number) => number.to_string(),
            Self::Text(value) => value.clone(),
        }
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Numeric(number) => number.fmt(formatter),
            Self::Text(value) => value.fmt(formatter),
        }
    }
}

impl Ord for Identifier {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_identifier_text(&self.as_text(), &other.as_text())
    }
}

impl PartialOrd for Identifier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Why a version could not be parsed by npm's strict grammar.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    TooLong,
    InvalidVersion,
    InvalidMajor,
    InvalidMinor,
    InvalidPatch,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TooLong => "version is longer than 256 characters",
            Self::InvalidVersion => "invalid version",
            Self::InvalidMajor => "invalid major version",
            Self::InvalidMinor => "invalid minor version",
            Self::InvalidPatch => "invalid patch version",
        })
    }
}

impl Error for ParseError {}

/// npm-compatible semantic version data.
#[derive(Clone, Debug)]
pub struct SemVer {
    raw: String,
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: Vec<Identifier>,
    build: Vec<String>,
    version: String,
}

impl SemVer {
    /// Parse the strict `node-semver` version grammar.
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        if input.encode_utf16().count() > MAX_LENGTH {
            return Err(ParseError::TooLong);
        }

        let raw = input.to_owned();
        let trimmed = input.trim();
        let value = trimmed.strip_prefix('v').unwrap_or(trimmed);

        let (without_build, build_text) = split_once(value, '+');
        if build_text.is_some_and(|build| !valid_build(build)) {
            return Err(ParseError::InvalidVersion);
        }

        let (core, prerelease_text) = split_once(without_build, '-');
        if prerelease_text.is_some_and(|prerelease| !valid_prerelease(prerelease)) {
            return Err(ParseError::InvalidVersion);
        }

        let mut components = core.split('.');
        let major_text = components.next().ok_or(ParseError::InvalidVersion)?;
        let minor_text = components.next().ok_or(ParseError::InvalidVersion)?;
        let patch_text = components.next().ok_or(ParseError::InvalidVersion)?;
        if components.next().is_some()
            || !valid_core_number(major_text)
            || !valid_core_number(minor_text)
            || !valid_core_number(patch_text)
        {
            return Err(ParseError::InvalidVersion);
        }

        let major = parse_core_number(major_text, ParseError::InvalidMajor)?;
        let minor = parse_core_number(minor_text, ParseError::InvalidMinor)?;
        let patch = parse_core_number(patch_text, ParseError::InvalidPatch)?;
        let prerelease: Vec<Identifier> = prerelease_text
            .map(|value| value.split('.').map(Identifier::parse).collect::<Vec<_>>())
            .unwrap_or_default();
        let build: Vec<String> = build_text
            .map(|value| value.split('.').map(str::to_owned).collect::<Vec<_>>())
            .unwrap_or_default();

        let version = format_version(major, minor, patch, &prerelease);
        Ok(Self {
            raw,
            major,
            minor,
            patch,
            prerelease,
            build,
            version,
        })
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn major(&self) -> u64 {
        self.major
    }

    pub fn minor(&self) -> u64 {
        self.minor
    }

    pub fn patch(&self) -> u64 {
        self.patch
    }

    pub fn prerelease(&self) -> &[Identifier] {
        &self.prerelease
    }

    pub fn build(&self) -> &[String] {
        &self.build
    }

    /// Canonical npm version text. Like `SemVer#version`, this excludes build
    /// metadata while preserving prerelease identifiers.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Compare major, minor, and patch only.
    pub fn compare_main(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }

    /// Compare prerelease precedence only.
    pub fn compare_pre(&self, other: &Self) -> Ordering {
        match (self.prerelease.is_empty(), other.prerelease.is_empty()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => compare_identifier_slices(&self.prerelease, &other.prerelease),
        }
    }

    /// Compare build identifiers using `node-semver`'s extension semantics.
    pub fn compare_build(&self, other: &Self) -> Ordering {
        compare_text_identifier_slices(&self.build, &other.build)
    }

    pub fn compare(&self, other: &Self) -> Ordering {
        self.compare_main(other)
            .then_with(|| self.compare_pre(other))
    }
}

impl FromStr for SemVer {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.version.fmt(formatter)
    }
}

impl PartialEq for SemVer {
    fn eq(&self, other: &Self) -> bool {
        self.compare(other) == Ordering::Equal
    }
}

impl Eq for SemVer {}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare(other)
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn split_once(value: &str, separator: char) -> (&str, Option<&str>) {
    match value.split_once(separator) {
        Some((head, tail)) => (head, Some(tail)),
        None => (value, None),
    }
}

fn valid_core_number(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && (value == "0" || !value.starts_with('0'))
}

fn parse_core_number(value: &str, overflow: ParseError) -> Result<u64, ParseError> {
    let number = value.parse::<u64>().map_err(|_| overflow.clone())?;
    (number <= MAX_SAFE_INTEGER)
        .then_some(number)
        .ok_or(overflow)
}

fn valid_prerelease(value: &str) -> bool {
    !value.is_empty()
        && value.split('.').all(|identifier| {
            valid_identifier(identifier)
                && (!identifier.bytes().all(|byte| byte.is_ascii_digit())
                    || identifier == "0"
                    || !identifier.starts_with('0'))
        })
}

fn valid_build(value: &str) -> bool {
    !value.is_empty() && value.split('.').all(valid_identifier)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn format_version(major: u64, minor: u64, patch: u64, prerelease: &[Identifier]) -> String {
    let mut version = format!("{major}.{minor}.{patch}");
    if !prerelease.is_empty() {
        version.push('-');
        for (index, identifier) in prerelease.iter().enumerate() {
            if index != 0 {
                version.push('.');
            }
            version.push_str(&identifier.to_string());
        }
    }
    version
}

fn compare_identifier_text(left: &str, right: &str) -> Ordering {
    match (is_numeric(left), is_numeric(right)) {
        (true, true) => compare_decimal_strings(left, right),
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => left.cmp(right),
    }
}

fn is_numeric(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn compare_decimal_strings(left: &str, right: &str) -> Ordering {
    let left = left.trim_start_matches('0');
    let right = right.trim_start_matches('0');
    let left = if left.is_empty() { "0" } else { left };
    let right = if right.is_empty() { "0" } else { right };
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

fn compare_identifier_slices(left: &[Identifier], right: &[Identifier]) -> Ordering {
    for (left_identifier, right_identifier) in left.iter().zip(right) {
        let ordering = left_identifier.cmp(right_identifier);
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.len().cmp(&right.len())
}

fn compare_text_identifier_slices(left: &[String], right: &[String]) -> Ordering {
    for (left_identifier, right_identifier) in left.iter().zip(right) {
        let ordering = compare_identifier_text(left_identifier, right_identifier);
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.len().cmp(&right.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats_upstream_examples() {
        let version = SemVer::parse(" v1.2.3-alpha.1+build.5 ").unwrap();
        assert_eq!(version.major(), 1);
        assert_eq!(version.minor(), 2);
        assert_eq!(version.patch(), 3);
        assert_eq!(
            version.prerelease(),
            &[Identifier::Text("alpha".into()), Identifier::Numeric(1)]
        );
        assert_eq!(version.build(), &["build", "5"]);
        assert_eq!(version.version(), "1.2.3-alpha.1");
        assert_eq!(version.raw(), " v1.2.3-alpha.1+build.5 ");
    }

    #[test]
    fn rejects_invalid_strict_versions() {
        for invalid in [
            "1.2",
            "1.2.3.4",
            "01.2.3",
            "1.02.3",
            "1.2.03",
            "1.2.3-01",
            "1.2.3-alpha..beta",
            "1.2.3+",
            "V1.2.3",
        ] {
            assert!(SemVer::parse(invalid).is_err(), "accepted {invalid:?}");
        }
    }

    #[test]
    fn follows_semver_precedence() {
        let ordered = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
        ];
        let parsed: Vec<_> = ordered
            .iter()
            .map(|value| SemVer::parse(value).unwrap())
            .collect();
        assert!(parsed.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn ignores_build_in_normal_comparison_but_can_compare_it() {
        let left = SemVer::parse("1.2.3+build.2").unwrap();
        let right = SemVer::parse("1.2.3+build.10").unwrap();
        assert_eq!(left, right);
        assert_eq!(left.compare_build(&right), Ordering::Less);
    }
}
