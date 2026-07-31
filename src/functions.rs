use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

use crate::{Identifier, ParseError, ReleaseType, SemVer};

/// Operators accepted by npm's `cmp` helper.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComparisonOperator {
    Equal,
    StrictEqual,
    NotEqual,
    StrictNotEqual,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
}

impl ComparisonOperator {
    pub fn parse(value: &str) -> Result<Self, InvalidComparisonOperator> {
        match value {
            "" | "=" | "==" => Ok(Self::Equal),
            "===" => Ok(Self::StrictEqual),
            "!=" => Ok(Self::NotEqual),
            "!==" => Ok(Self::StrictNotEqual),
            ">" => Ok(Self::Greater),
            ">=" => Ok(Self::GreaterOrEqual),
            "<" => Ok(Self::Less),
            "<=" => Ok(Self::LessOrEqual),
            _ => Err(InvalidComparisonOperator(value.to_owned())),
        }
    }

    fn evaluate(self, ordering: Ordering) -> bool {
        match self {
            Self::Equal | Self::StrictEqual => ordering == Ordering::Equal,
            Self::NotEqual | Self::StrictNotEqual => ordering != Ordering::Equal,
            Self::Greater => ordering == Ordering::Greater,
            Self::GreaterOrEqual => ordering != Ordering::Less,
            Self::Less => ordering == Ordering::Less,
            Self::LessOrEqual => ordering != Ordering::Greater,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvalidComparisonOperator(pub String);

impl fmt::Display for InvalidComparisonOperator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid comparison operator: {}", self.0)
    }
}

impl Error for InvalidComparisonOperator {}

pub fn compare(left: &str, right: &str) -> Result<Ordering, ParseError> {
    Ok(SemVer::parse(left)?.compare(&SemVer::parse(right)?))
}

pub fn compare_loose(left: &str, right: &str) -> Result<Ordering, ParseError> {
    Ok(SemVer::parse_loose(left)?.compare(&SemVer::parse_loose(right)?))
}

pub fn compare_build(left: &str, right: &str) -> Result<Ordering, ParseError> {
    let left = SemVer::parse(left)?;
    let right = SemVer::parse(right)?;
    Ok(left
        .compare(&right)
        .then_with(|| left.compare_build(&right)))
}

pub fn rcompare(left: &str, right: &str) -> Result<Ordering, ParseError> {
    compare(right, left)
}

pub fn eq(left: &str, right: &str) -> Result<bool, ParseError> {
    Ok(compare(left, right)? == Ordering::Equal)
}

pub fn neq(left: &str, right: &str) -> Result<bool, ParseError> {
    Ok(compare(left, right)? != Ordering::Equal)
}

pub fn gt(left: &str, right: &str) -> Result<bool, ParseError> {
    Ok(compare(left, right)? == Ordering::Greater)
}

pub fn gte(left: &str, right: &str) -> Result<bool, ParseError> {
    Ok(compare(left, right)? != Ordering::Less)
}

pub fn lt(left: &str, right: &str) -> Result<bool, ParseError> {
    Ok(compare(left, right)? == Ordering::Less)
}

pub fn lte(left: &str, right: &str) -> Result<bool, ParseError> {
    Ok(compare(left, right)? != Ordering::Greater)
}

pub fn cmp(left: &str, operator: ComparisonOperator, right: &str) -> Result<bool, ParseError> {
    Ok(operator.evaluate(compare(left, right)?))
}

pub fn major(input: &str, loose: bool) -> Option<u64> {
    parse_mode(input, loose).ok().map(|version| version.major())
}

pub fn minor(input: &str, loose: bool) -> Option<u64> {
    parse_mode(input, loose).ok().map(|version| version.minor())
}

pub fn patch(input: &str, loose: bool) -> Option<u64> {
    parse_mode(input, loose).ok().map(|version| version.patch())
}

pub fn prerelease(input: &str, loose: bool) -> Option<Vec<Identifier>> {
    let version = parse_mode(input, loose).ok()?;
    (!version.prerelease().is_empty()).then(|| version.prerelease().to_vec())
}

/// Return npm's semantic change classification.
pub fn diff(left: &str, right: &str) -> Result<Option<ReleaseType>, ParseError> {
    let left = SemVer::parse(left)?;
    let right = SemVer::parse(right)?;
    let ordering = left.compare(&right);
    if ordering == Ordering::Equal {
        return Ok(None);
    }

    let (high, low) = if ordering == Ordering::Greater {
        (&left, &right)
    } else {
        (&right, &left)
    };
    let high_has_pre = !high.prerelease().is_empty();
    let low_has_pre = !low.prerelease().is_empty();

    if low_has_pre && !high_has_pre {
        if low.patch() == 0 && low.minor() == 0 {
            return Ok(Some(ReleaseType::Major));
        }
        if low.compare_main(high) == Ordering::Equal {
            return Ok(Some(if low.patch() == 0 && low.minor() != 0 {
                ReleaseType::Minor
            } else {
                ReleaseType::Patch
            }));
        }
    }

    let release = if left.major() != right.major() {
        if high_has_pre {
            ReleaseType::Premajor
        } else {
            ReleaseType::Major
        }
    } else if left.minor() != right.minor() {
        if high_has_pre {
            ReleaseType::Preminor
        } else {
            ReleaseType::Minor
        }
    } else if left.patch() != right.patch() {
        if high_has_pre {
            ReleaseType::Prepatch
        } else {
            ReleaseType::Patch
        }
    } else {
        ReleaseType::Prerelease
    };
    Ok(Some(release))
}

/// Drop less-significant components without incrementing the version.
pub fn truncate(input: &str, release: ReleaseType, loose: bool) -> Option<String> {
    let version = parse_mode(input, loose).ok()?;
    if matches!(
        release,
        ReleaseType::Premajor
            | ReleaseType::Preminor
            | ReleaseType::Prepatch
            | ReleaseType::Prerelease
    ) {
        return Some(version.version().to_owned());
    }

    let (minor, patch) = match release {
        ReleaseType::Major => (0, 0),
        ReleaseType::Minor => (version.minor(), 0),
        ReleaseType::Patch | ReleaseType::Release => (version.minor(), version.patch()),
        _ => unreachable!(),
    };
    Some(format!("{}.{}.{}", version.major(), minor, patch))
}

pub fn sort(versions: &[&str]) -> Result<Vec<String>, ParseError> {
    sort_by(versions, false)
}

pub fn rsort(versions: &[&str]) -> Result<Vec<String>, ParseError> {
    sort_by(versions, true)
}

fn sort_by(versions: &[&str], reverse: bool) -> Result<Vec<String>, ParseError> {
    let mut parsed = versions
        .iter()
        .map(|value| SemVer::parse(value).map(|version| ((*value).to_owned(), version)))
        .collect::<Result<Vec<_>, _>>()?;
    parsed.sort_by(|(_, left), (_, right)| {
        let ordering = left.compare(right).then_with(|| left.compare_build(right));
        if reverse {
            ordering.reverse()
        } else {
            ordering
        }
    });
    Ok(parsed.into_iter().map(|(original, _)| original).collect())
}

fn parse_mode(input: &str, loose: bool) -> Result<SemVer, ParseError> {
    if loose {
        SemVer::parse_loose(input)
    } else {
        SemVer::parse(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implements_comparison_helpers() {
        assert_eq!(compare("1.2.3", "1.2.4"), Ok(Ordering::Less));
        assert_eq!(compare_build("1.2.3+1", "1.2.3+2"), Ok(Ordering::Less));
        assert_eq!(rcompare("1.2.3", "1.2.4"), Ok(Ordering::Greater));
        assert_eq!(eq("1.2.3+one", "1.2.3+two"), Ok(true));
        assert_eq!(gt("2.0.0", "1.9.9"), Ok(true));
        assert_eq!(lte("2.0.0", "1.9.9"), Ok(false));
        assert_eq!(
            cmp("1.2.3", ComparisonOperator::GreaterOrEqual, "1.2.3"),
            Ok(true)
        );
    }

    #[test]
    fn classifies_upstream_diff_examples() {
        for (left, right, expected) in [
            ("1.2.3", "2.0.0-pre", ReleaseType::Premajor),
            ("1.0.1", "1.1.0-pre", ReleaseType::Preminor),
            ("1.2.3", "1.2.4-pre", ReleaseType::Prepatch),
            ("1.0.0-1", "1.0.0", ReleaseType::Major),
            ("1.1.0-1", "1.1.0", ReleaseType::Minor),
            ("1.1.1-1", "1.1.1", ReleaseType::Patch),
            ("1.1.1-pre-1", "1.1.1-pre-2", ReleaseType::Prerelease),
        ] {
            assert_eq!(diff(left, right), Ok(Some(expected)), "{left} {right}");
        }
        assert_eq!(diff("1.2.3+one", "1.2.3+two"), Ok(None));
    }

    #[test]
    fn extracts_truncates_and_sorts() {
        assert_eq!(major("01.2.3", true), Some(1));
        assert_eq!(minor("1.2.3", false), Some(2));
        assert_eq!(patch("1.2.3", false), Some(3));
        assert_eq!(
            prerelease("1.2.3-alpha.1", false),
            Some(vec![
                Identifier::Text("alpha".into()),
                Identifier::Numeric(1)
            ])
        );
        assert_eq!(
            truncate("1.2.3-foo+bar", ReleaseType::Major, false).as_deref(),
            Some("1.0.0")
        );
        assert_eq!(
            sort(&["1.2.3", "1.2.3-alpha", "2.0.0"]).unwrap(),
            ["1.2.3-alpha", "1.2.3", "2.0.0"]
        );
        assert_eq!(
            rsort(&["1.2.3", "1.2.3-alpha", "2.0.0"]).unwrap(),
            ["2.0.0", "1.2.3", "1.2.3-alpha"]
        );
    }
}
