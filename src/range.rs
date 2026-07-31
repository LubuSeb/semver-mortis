use std::error::Error;
use std::fmt;

use crate::{Comparator, ComparatorError, SemVer};

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const NULL_SET: &str = "<0.0.0-0";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RangeOptions {
    pub loose: bool,
    pub include_prerelease: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RangeError(pub String);

impl fmt::Display for RangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid SemVer range: {}", self.0)
    }
}

impl Error for RangeError {}

impl From<ComparatorError> for RangeError {
    fn from(error: ComparatorError) -> Self {
        Self(error.0)
    }
}

/// An OR-of-AND comparator set implementing npm's range grammar.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Range {
    raw: String,
    sets: Vec<Vec<Comparator>>,
    options: RangeOptions,
    formatted: String,
}

impl Range {
    pub fn parse(input: &str) -> Result<Self, RangeError> {
        Self::parse_with_options(input, RangeOptions::default())
    }

    pub fn parse_with_options(input: &str, options: RangeOptions) -> Result<Self, RangeError> {
        let raw = input.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut sets = Vec::new();
        for alternative in raw.split("||") {
            match parse_set(alternative.trim(), options) {
                Ok(set) => {
                    if !set.is_empty() {
                        sets.push(set);
                    }
                }
                Err(_) if options.loose => {}
                Err(_) => return Err(RangeError(raw)),
            }
        }
        if sets.is_empty() {
            return Err(RangeError(raw));
        }

        if sets.len() > 1 {
            let first = sets[0].clone();
            sets.retain(|set| !is_null_set(&set[0]));
            if sets.is_empty() {
                sets.push(first);
            } else if let Some(any) = sets
                .iter()
                .find(|set| set.len() == 1 && set[0].operator() == crate::ComparatorOperator::Any)
                .cloned()
            {
                sets = vec![any];
            }
        }

        let formatted = sets
            .iter()
            .map(|set| {
                set.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("||");
        Ok(Self {
            raw,
            sets,
            options,
            formatted,
        })
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn sets(&self) -> &[Vec<Comparator>] {
        &self.sets
    }

    pub fn options(&self) -> RangeOptions {
        self.options
    }

    pub fn range(&self) -> &str {
        &self.formatted
    }

    pub fn test(&self, input: &str) -> bool {
        let Ok(version) = (if self.options.loose {
            SemVer::parse_loose(input)
        } else {
            SemVer::parse(input)
        }) else {
            return false;
        };
        self.test_version(&version)
    }

    pub fn test_version(&self, version: &SemVer) -> bool {
        self.sets
            .iter()
            .any(|set| test_set(set, version, self.options.include_prerelease))
    }

    pub fn intersects(&self, other: &Self, include_prerelease: bool) -> bool {
        self.sets.iter().any(|left| {
            is_satisfiable(left, include_prerelease)
                && other.sets.iter().any(|right| {
                    is_satisfiable(right, include_prerelease)
                        && left.iter().all(|left_comparator| {
                            right.iter().all(|right_comparator| {
                                left_comparator.intersects(right_comparator, include_prerelease)
                            })
                        })
                })
        })
    }
}

impl fmt::Display for Range {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.formatted.fmt(formatter)
    }
}

#[derive(Clone, Debug)]
struct PartialVersion {
    major: Option<u64>,
    minor: Option<u64>,
    patch: Option<u64>,
    prerelease: Option<String>,
}

impl PartialVersion {
    fn exact(&self) -> bool {
        self.major.is_some() && self.minor.is_some() && self.patch.is_some()
    }

    fn exact_text(&self) -> Option<String> {
        Some(format!(
            "{}.{}.{}{}",
            self.major?,
            self.minor?,
            self.patch?,
            self.prerelease
                .as_ref()
                .map(|value| format!("-{value}"))
                .unwrap_or_default()
        ))
    }
}

fn parse_set(input: &str, options: RangeOptions) -> Result<Vec<Comparator>, RangeError> {
    let expanded = if let Some(hyphen) = expand_hyphen(input, options)? {
        hyphen
    } else {
        let tokens = join_spaced_operators(input);
        let had_tokens = !tokens.is_empty();
        let mut expanded = Vec::new();
        for token in tokens {
            match expand_token(&token, options) {
                Ok(values) => expanded.extend(values),
                Err(_) if options.loose => {}
                Err(error) => return Err(error),
            }
        }
        if expanded.is_empty() && had_tokens {
            return Ok(Vec::new());
        }
        expanded
    };

    let mut comparators = Vec::new();
    for value in expanded {
        let value = normalize_floor(&value, options.include_prerelease);
        let comparator = Comparator::parse(&value, options.loose)?;
        if is_null_set(&comparator) {
            return Ok(vec![comparator]);
        }
        if !comparators
            .iter()
            .any(|existing: &Comparator| existing.value() == comparator.value())
        {
            comparators.push(comparator);
        }
    }
    if comparators.len() > 1 {
        comparators.retain(|comparator| !comparator.value().is_empty());
    }
    if comparators.is_empty() {
        comparators.push(Comparator::any(options.loose));
    }
    Ok(comparators)
}

fn join_spaced_operators(input: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut words = input.split_whitespace().peekable();
    while let Some(word) = words.next() {
        if matches!(word, ">" | ">=" | "<" | "<=" | "=" | "~" | "~>" | "^") {
            if let Some(next) = words.peek() {
                result.push(format!("{word}{}", next));
                words.next();
            } else {
                result.push(word.to_owned());
            }
        } else {
            result.push(word.to_owned());
        }
    }
    result
}

fn expand_hyphen(input: &str, options: RangeOptions) -> Result<Option<Vec<String>>, RangeError> {
    let words: Vec<_> = input.split_whitespace().collect();
    if words.len() != 3 || words[1] != "-" {
        return Ok(None);
    }
    let from = parse_partial(words[0], options.loose)?;
    let to = parse_partial(words[2], options.loose)?;
    let mut result = Vec::new();

    if let Some(major) = from.major {
        let lower = match (from.minor, from.patch) {
            (None, _) => format!(">={major}.0.0{}", prerelease_floor(options)),
            (Some(minor), None) => {
                format!(">={major}.{minor}.0{}", prerelease_floor(options))
            }
            (Some(minor), Some(patch)) => {
                if let Some(prerelease) = from.prerelease {
                    format!(">={major}.{minor}.{patch}-{prerelease}")
                } else {
                    format!(">={major}.{minor}.{patch}{}", prerelease_floor(options))
                }
            }
        };
        result.push(lower);
    }

    if let Some(major) = to.major {
        let upper = match (to.minor, to.patch) {
            (None, _) => format!("<{}.0.0-0", increment(major)?),
            (Some(minor), None) => format!("<{major}.{}.0-0", increment(minor)?),
            (Some(minor), Some(patch)) => {
                if let Some(prerelease) = to.prerelease {
                    format!("<={major}.{minor}.{patch}-{prerelease}")
                } else if options.include_prerelease {
                    format!("<{major}.{minor}.{}-0", increment(patch)?)
                } else {
                    format!("<={major}.{minor}.{patch}")
                }
            }
        };
        result.push(upper);
    }
    Ok(Some(result))
}

fn expand_token(input: &str, options: RangeOptions) -> Result<Vec<String>, RangeError> {
    let token = strip_build(input);
    if token.is_empty() || token == "*" || token.eq_ignore_ascii_case("x") {
        return Ok(vec![String::new()]);
    }
    if let Some(value) = token.strip_prefix("~>") {
        return expand_tilde(value, options);
    }
    if let Some(value) = token.strip_prefix('~') {
        return expand_tilde(value, options);
    }
    if let Some(value) = token.strip_prefix('^') {
        return expand_caret(value, options);
    }
    expand_xrange(token, options)
}

fn expand_tilde(input: &str, options: RangeOptions) -> Result<Vec<String>, RangeError> {
    let version = parse_partial(input, options.loose)?;
    let Some(major) = version.major else {
        return Ok(vec![String::new()]);
    };
    let lower_floor = prerelease_floor(options);
    let (lower, upper) = match (version.minor, version.patch) {
        (None, _) => (
            format!(">={major}.0.0{lower_floor}"),
            format!("<{}.0.0-0", increment(major)?),
        ),
        (Some(minor), None) => (
            format!(">={major}.{minor}.0{lower_floor}"),
            format!("<{major}.{}.0-0", increment(minor)?),
        ),
        (Some(minor), Some(patch)) => {
            let lower = if let Some(prerelease) = version.prerelease {
                format!(">={major}.{minor}.{patch}-{prerelease}")
            } else {
                format!(">={major}.{minor}.{patch}")
            };
            (lower, format!("<{major}.{}.0-0", increment(minor)?))
        }
    };
    Ok(vec![lower, upper])
}

fn expand_caret(input: &str, options: RangeOptions) -> Result<Vec<String>, RangeError> {
    let version = parse_partial(input, options.loose)?;
    let Some(major) = version.major else {
        return Ok(vec![String::new()]);
    };
    let lower_floor = prerelease_floor(options);
    let (lower, upper) = match (version.minor, version.patch) {
        (None, _) => (
            format!(">={major}.0.0{lower_floor}"),
            format!("<{}.0.0-0", increment(major)?),
        ),
        (Some(minor), None) => {
            let upper = if major == 0 {
                format!("<0.{}.0-0", increment(minor)?)
            } else {
                format!("<{}.0.0-0", increment(major)?)
            };
            (format!(">={major}.{minor}.0{lower_floor}"), upper)
        }
        (Some(minor), Some(patch)) => {
            let lower = if let Some(prerelease) = version.prerelease {
                format!(">={major}.{minor}.{patch}-{prerelease}")
            } else {
                format!(">={major}.{minor}.{patch}")
            };
            let upper = if major != 0 {
                format!("<{}.0.0-0", increment(major)?)
            } else if minor != 0 {
                format!("<0.{}.0-0", increment(minor)?)
            } else {
                format!("<0.0.{}-0", increment(patch)?)
            };
            (lower, upper)
        }
    };
    Ok(vec![lower, upper])
}

fn expand_xrange(input: &str, options: RangeOptions) -> Result<Vec<String>, RangeError> {
    let (operator, version_text) = split_operator(input);
    let version = parse_partial(version_text, options.loose)?;
    let Some(mut major) = version.major else {
        return if matches!(operator, ">" | "<") {
            Ok(vec![NULL_SET.to_owned()])
        } else {
            Ok(vec![String::new()])
        };
    };

    if version.exact() {
        let exact = version.exact_text().expect("exact partial version");
        let operator = if operator == "=" { "" } else { operator };
        return Ok(vec![format!("{operator}{exact}")]);
    }

    let mut minor = version.minor.unwrap_or(0);
    let patch = 0;
    if !operator.is_empty() && operator != "=" {
        let result = match operator {
            ">" => {
                if version.minor.is_none() {
                    major = increment(major)?;
                    minor = 0;
                } else {
                    minor = increment(minor)?;
                }
                format!(">={major}.{minor}.{patch}")
            }
            ">=" => format!(">={major}.{minor}.{patch}{}", prerelease_floor(options)),
            "<" => format!("<{major}.{minor}.{patch}-0"),
            "<=" => {
                if version.minor.is_none() {
                    major = increment(major)?;
                } else {
                    minor = increment(minor)?;
                }
                format!("<{major}.{minor}.{patch}-0")
            }
            _ => return Err(RangeError(input.to_owned())),
        };
        return Ok(vec![result]);
    }

    let lower_floor = prerelease_floor(options);
    let (lower, upper) = if version.minor.is_none() {
        (
            format!(">={major}.0.0{lower_floor}"),
            format!("<{}.0.0-0", increment(major)?),
        )
    } else {
        (
            format!(">={major}.{minor}.0{lower_floor}"),
            format!("<{major}.{}.0-0", increment(minor)?),
        )
    };
    Ok(vec![lower, upper])
}

fn parse_partial(input: &str, loose: bool) -> Result<PartialVersion, RangeError> {
    let value = strip_build(input.trim());
    let value = value
        .strip_prefix('v')
        .or_else(|| value.strip_prefix('V'))
        .unwrap_or(value);
    if value.is_empty() || value == "*" || value.eq_ignore_ascii_case("x") {
        return Ok(PartialVersion {
            major: None,
            minor: None,
            patch: None,
            prerelease: None,
        });
    }
    if loose && let Ok(version) = SemVer::parse_loose(value) {
        let prerelease = (!version.prerelease().is_empty()).then(|| {
            version
                .prerelease()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(".")
        });
        return Ok(PartialVersion {
            major: Some(version.major()),
            minor: Some(version.minor()),
            patch: Some(version.patch()),
            prerelease,
        });
    }
    let (core, prerelease) = value
        .split_once('-')
        .map_or((value, None), |(core, prerelease)| (core, Some(prerelease)));
    let parts: Vec<_> = core.split('.').collect();
    if parts.is_empty() || parts.len() > 3 {
        return Err(RangeError(input.to_owned()));
    }
    let mut values = [None; 3];
    let mut wildcard_seen = false;
    let mut explicit_wildcard = false;
    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() || *part == "*" || part.eq_ignore_ascii_case("x") {
            wildcard_seen = true;
            explicit_wildcard = true;
            continue;
        }
        if wildcard_seen || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(RangeError(input.to_owned()));
        }
        if !loose && part.len() > 1 && part.starts_with('0') {
            return Err(RangeError(input.to_owned()));
        }
        let number = part
            .parse::<u64>()
            .map_err(|_| RangeError(input.to_owned()))?;
        if number > MAX_SAFE_INTEGER {
            return Err(RangeError(input.to_owned()));
        }
        values[index] = Some(number);
    }
    if prerelease.is_some() && values.iter().any(Option::is_none) && !explicit_wildcard {
        return Err(RangeError(input.to_owned()));
    }
    if let Some(prerelease) = prerelease.filter(|_| values.iter().all(Option::is_some)) {
        let full = format!(
            "{}.{}.{}-{prerelease}",
            values[0].expect("complete version"),
            values[1].expect("complete version"),
            values[2].expect("complete version")
        );
        let valid = if loose {
            SemVer::parse_loose(&full).is_ok()
        } else {
            SemVer::parse(&full).is_ok()
        };
        if !valid {
            return Err(RangeError(input.to_owned()));
        }
    }
    Ok(PartialVersion {
        major: values[0],
        minor: values[1],
        patch: values[2],
        prerelease: prerelease
            .filter(|_| values.iter().all(Option::is_some))
            .map(str::to_owned),
    })
}

fn split_operator(value: &str) -> (&str, &str) {
    for operator in [">=", "<=", ">", "<", "="] {
        if let Some(rest) = value.strip_prefix(operator) {
            return (operator, rest);
        }
    }
    ("", value)
}

fn strip_build(value: &str) -> &str {
    value.split_once('+').map_or(value, |(version, _)| version)
}

fn prerelease_floor(options: RangeOptions) -> &'static str {
    if options.include_prerelease { "-0" } else { "" }
}

fn increment(value: u64) -> Result<u64, RangeError> {
    value
        .checked_add(1)
        .ok_or_else(|| RangeError("version component overflow".into()))
}

fn normalize_floor(value: &str, include_prerelease: bool) -> String {
    if (!include_prerelease && value == ">=0.0.0") || (include_prerelease && value == ">=0.0.0-0") {
        String::new()
    } else {
        value.to_owned()
    }
}

fn is_null_set(comparator: &Comparator) -> bool {
    comparator.value() == NULL_SET
}

fn is_satisfiable(comparators: &[Comparator], include_prerelease: bool) -> bool {
    comparators.iter().enumerate().all(|(index, comparator)| {
        comparators[index + 1..]
            .iter()
            .all(|other| comparator.intersects(other, include_prerelease))
    })
}

fn test_set(set: &[Comparator], version: &SemVer, include_prerelease: bool) -> bool {
    if !set
        .iter()
        .all(|comparator| comparator.test_version(version))
    {
        return false;
    }
    if version.prerelease().is_empty() || include_prerelease {
        return true;
    }
    set.iter().any(|comparator| {
        comparator.semver().is_some_and(|allowed| {
            !allowed.prerelease().is_empty()
                && allowed.major() == version.major()
                && allowed.minor() == version.minor()
                && allowed.patch() == version.patch()
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_core_range_forms() {
        for (input, expected) in [
            ("1.2.3 - 2.0.0", ">=1.2.3 <=2.0.0"),
            ("1 - 2", ">=1.0.0 <3.0.0-0"),
            ("~1.2.3", ">=1.2.3 <1.3.0-0"),
            ("~1.2", ">=1.2.0 <1.3.0-0"),
            ("^1.2.3", ">=1.2.3 <2.0.0-0"),
            ("^0.2.3", ">=0.2.3 <0.3.0-0"),
            ("^0.0.3", ">=0.0.3 <0.0.4-0"),
            ("1.2.x", ">=1.2.0 <1.3.0-0"),
            (">1", ">=2.0.0"),
            ("<=1.2.x", "<1.3.0-0"),
            ("*", ""),
            (">=0.0.0", ""),
        ] {
            assert_eq!(Range::parse(input).unwrap().range(), expected, "{input}");
        }
    }

    #[test]
    fn applies_prerelease_gating() {
        let range = Range::parse("^1.2.3").unwrap();
        assert!(range.test("1.4.0"));
        assert!(!range.test("1.4.0-alpha"));

        let range = Range::parse("^1.2.3-beta.1").unwrap();
        assert!(range.test("1.2.3-beta.2"));
        assert!(!range.test("1.2.4-beta.1"));

        let options = RangeOptions {
            include_prerelease: true,
            ..RangeOptions::default()
        };
        let range = Range::parse_with_options("1.2.x", options).unwrap();
        assert_eq!(range.range(), ">=1.2.0-0 <1.3.0-0");
        assert!(range.test("1.2.0-alpha"));
    }

    #[test]
    fn supports_or_sets_and_intersections() {
        let range = Range::parse("1.2.x || >=2.0.0").unwrap();
        assert!(range.test("1.2.9"));
        assert!(range.test("3.0.0"));
        assert!(!range.test("1.3.0"));
        assert!(range.intersects(&Range::parse("^1.2.4").unwrap(), false));
        assert!(!range.intersects(&Range::parse("1.3.x").unwrap(), false));
    }
}
