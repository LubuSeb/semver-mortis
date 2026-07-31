use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

use crate::SemVer;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComparatorOperator {
    Any,
    Exact,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
}

impl ComparatorOperator {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Any | Self::Exact => "",
            Self::Greater => ">",
            Self::GreaterOrEqual => ">=",
            Self::Less => "<",
            Self::LessOrEqual => "<=",
        }
    }

    fn is_greater(self) -> bool {
        matches!(self, Self::Greater | Self::GreaterOrEqual)
    }

    fn is_less(self) -> bool {
        matches!(self, Self::Less | Self::LessOrEqual)
    }

    fn is_inclusive(self) -> bool {
        matches!(self, Self::Exact | Self::GreaterOrEqual | Self::LessOrEqual)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComparatorError(pub String);

impl fmt::Display for ComparatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid comparator: {}", self.0)
    }
}

impl Error for ComparatorError {}

/// A normalized npm comparator such as `>=1.2.3`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Comparator {
    operator: ComparatorOperator,
    semver: Option<SemVer>,
    loose: bool,
    value: String,
}

impl Comparator {
    pub fn parse(input: &str, loose: bool) -> Result<Self, ComparatorError> {
        let compact = input.split_whitespace().collect::<Vec<_>>().join(" ");
        let (operator_text, version_text) = split_operator(&compact);
        let operator = match operator_text {
            "" | "=" => ComparatorOperator::Exact,
            ">" => ComparatorOperator::Greater,
            ">=" => ComparatorOperator::GreaterOrEqual,
            "<" => ComparatorOperator::Less,
            "<=" => ComparatorOperator::LessOrEqual,
            _ => return Err(ComparatorError(compact)),
        };
        let version_text = version_text.trim();
        if version_text.is_empty() {
            if matches!(
                operator,
                ComparatorOperator::Exact | ComparatorOperator::Greater
            ) {
                return Ok(Self {
                    operator: ComparatorOperator::Any,
                    semver: None,
                    loose,
                    value: String::new(),
                });
            }
            return Err(ComparatorError(compact));
        }

        let semver = if loose {
            SemVer::parse_loose(version_text)
        } else {
            SemVer::parse(version_text)
        }
        .map_err(|_| ComparatorError(compact))?;
        let value = format!("{}{}", operator.as_str(), semver.version());
        Ok(Self {
            operator,
            semver: Some(semver),
            loose,
            value,
        })
    }

    pub fn any(loose: bool) -> Self {
        Self {
            operator: ComparatorOperator::Any,
            semver: None,
            loose,
            value: String::new(),
        }
    }

    pub fn operator(&self) -> ComparatorOperator {
        self.operator
    }

    pub fn semver(&self) -> Option<&SemVer> {
        self.semver.as_ref()
    }

    pub fn is_loose(&self) -> bool {
        self.loose
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn test(&self, input: &str) -> bool {
        if self.operator == ComparatorOperator::Any {
            return true;
        }
        let Ok(version) = (if self.loose {
            SemVer::parse_loose(input)
        } else {
            SemVer::parse(input)
        }) else {
            return false;
        };
        self.test_version(&version)
    }

    pub fn test_version(&self, version: &SemVer) -> bool {
        let Some(target) = &self.semver else {
            return true;
        };
        let ordering = version.compare(target);
        match self.operator {
            ComparatorOperator::Any => true,
            ComparatorOperator::Exact => ordering == Ordering::Equal,
            ComparatorOperator::Greater => ordering == Ordering::Greater,
            ComparatorOperator::GreaterOrEqual => ordering != Ordering::Less,
            ComparatorOperator::Less => ordering == Ordering::Less,
            ComparatorOperator::LessOrEqual => ordering != Ordering::Greater,
        }
    }

    pub fn intersects(&self, other: &Self, include_prerelease: bool) -> bool {
        if self.operator == ComparatorOperator::Any || other.operator == ComparatorOperator::Any {
            return true;
        }
        if self.operator == ComparatorOperator::Exact {
            return self
                .semver
                .as_ref()
                .is_some_and(|version| other.test_version(version));
        }
        if other.operator == ComparatorOperator::Exact {
            return other
                .semver
                .as_ref()
                .is_some_and(|version| self.test_version(version));
        }

        if is_impossible_lower_bound(self, include_prerelease)
            || is_impossible_lower_bound(other, include_prerelease)
        {
            return false;
        }
        if (self.operator.is_greater() && other.operator.is_greater())
            || (self.operator.is_less() && other.operator.is_less())
        {
            return true;
        }

        let left = self.semver.as_ref().expect("non-any comparator");
        let right = other.semver.as_ref().expect("non-any comparator");
        let ordering = left.compare(right);
        if ordering == Ordering::Equal {
            return self.operator.is_inclusive() && other.operator.is_inclusive();
        }
        (ordering == Ordering::Less && self.operator.is_greater() && other.operator.is_less())
            || (ordering == Ordering::Greater
                && self.operator.is_less()
                && other.operator.is_greater())
    }
}

impl fmt::Display for Comparator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(formatter)
    }
}

fn split_operator(value: &str) -> (&str, &str) {
    for operator in [">=", "<=", ">", "<", "="] {
        if let Some(rest) = value.strip_prefix(operator) {
            return (operator, rest);
        }
    }
    ("", value)
}

fn is_impossible_lower_bound(comparator: &Comparator, include_prerelease: bool) -> bool {
    if comparator.operator != ComparatorOperator::Less {
        return false;
    }
    if include_prerelease {
        comparator.value == "<0.0.0-0"
    } else {
        comparator.value.starts_with("<0.0.0")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_normalizes_and_tests() {
        let comparator = Comparator::parse(">= v1.2.3", false).unwrap();
        assert_eq!(comparator.to_string(), ">=1.2.3");
        assert!(comparator.test("1.2.4"));
        assert!(!comparator.test("not a version string"));
        assert!(Comparator::parse("", false).unwrap().test("1.2.3"));
        assert_eq!(
            Comparator::parse("=1.2.3", false).unwrap(),
            Comparator::parse("1.2.3", false).unwrap()
        );
    }

    #[test]
    fn matches_upstream_intersection_edges() {
        for (left, right, expected, include_prerelease) in [
            ("1.3.0", ">=1.3.0", true, false),
            ("1.3.0", ">1.3.0", false, false),
            (">1.3.0", ">1.2.0", true, false),
            (">=1.3.0", "<=1.3.0", true, false),
            (">1.3.0", "<=1.3.0", false, false),
            (">1.0.0", "<2.0.0", true, false),
            ("<=1.0.0", ">=2.0.0", false, false),
            ("<0.0.0", "<0.1.0", false, false),
            ("<0.0.0-0", "<0.1.0", false, true),
        ] {
            let left = Comparator::parse(left, false).unwrap();
            let right = Comparator::parse(right, false).unwrap();
            assert_eq!(
                left.intersects(&right, include_prerelease),
                expected,
                "{left} {right}"
            );
            assert_eq!(right.intersects(&left, include_prerelease), expected);
        }
    }
}
