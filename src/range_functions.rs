use std::cmp::Ordering;

use crate::{Comparator, ComparatorOperator, Range, RangeError, RangeOptions, SemVer};

pub fn satisfies(version: &str, range: &str, options: RangeOptions) -> bool {
    Range::parse_with_options(range, options).is_ok_and(|range| range.test(version))
}

pub fn valid_range(input: &str, options: RangeOptions) -> Option<String> {
    let range = Range::parse_with_options(input, options).ok()?;
    Some(if range.range().is_empty() {
        "*".to_owned()
    } else {
        range.range().to_owned()
    })
}

pub fn max_satisfying(versions: &[&str], range: &str, options: RangeOptions) -> Option<String> {
    satisfying_extreme(versions, range, options, true)
}

pub fn min_satisfying(versions: &[&str], range: &str, options: RangeOptions) -> Option<String> {
    satisfying_extreme(versions, range, options, false)
}

fn satisfying_extreme(
    versions: &[&str],
    range: &str,
    options: RangeOptions,
    maximum: bool,
) -> Option<String> {
    let range = Range::parse_with_options(range, options).ok()?;
    let mut selected: Option<(&str, SemVer)> = None;
    for input in versions {
        let Ok(version) = parse_mode(input, options.loose) else {
            continue;
        };
        if !range.test_version(&version) {
            continue;
        }
        let replace = selected.as_ref().is_none_or(|(_, current)| {
            let ordering = version.compare(current);
            if maximum {
                ordering == Ordering::Greater
            } else {
                ordering == Ordering::Less
            }
        });
        if replace {
            selected = Some((input, version));
        }
    }
    selected.map(|(input, _)| input.to_owned())
}

pub fn min_version(input: &str, options: RangeOptions) -> Option<SemVer> {
    let range = Range::parse_with_options(input, options).ok()?;
    for floor in ["0.0.0", "0.0.0-0"] {
        let version = SemVer::parse(floor).expect("static version");
        if range.test_version(&version) {
            return Some(version);
        }
    }

    let mut minimum: Option<SemVer> = None;
    for set in range.sets() {
        let mut set_minimum: Option<SemVer> = None;
        for comparator in set {
            let Some(target) = comparator.semver() else {
                continue;
            };
            let candidate = match comparator.operator() {
                ComparatorOperator::Greater => increment_minimum(target)?,
                ComparatorOperator::Exact | ComparatorOperator::GreaterOrEqual => target.clone(),
                ComparatorOperator::Any
                | ComparatorOperator::Less
                | ComparatorOperator::LessOrEqual => continue,
            };
            if set_minimum
                .as_ref()
                .is_none_or(|current| candidate.compare(current) == Ordering::Greater)
            {
                set_minimum = Some(candidate);
            }
        }
        if let Some(candidate) = set_minimum
            && minimum
                .as_ref()
                .is_none_or(|current| candidate.compare(current) == Ordering::Less)
        {
            minimum = Some(candidate);
        }
    }
    minimum.filter(|version| range.test_version(version))
}

pub fn intersects(left: &str, right: &str, options: RangeOptions) -> Result<bool, RangeError> {
    let left = Range::parse_with_options(left, options)?;
    let right = Range::parse_with_options(right, options)?;
    Ok(left.intersects(&right, options.include_prerelease))
}

pub fn greater_than_range(
    version: &str,
    range: &str,
    options: RangeOptions,
) -> Result<bool, RangeError> {
    outside(version, range, OutsideDirection::Higher, options)
}

pub fn less_than_range(
    version: &str,
    range: &str,
    options: RangeOptions,
) -> Result<bool, RangeError> {
    outside(version, range, OutsideDirection::Lower, options)
}

pub fn to_comparators(input: &str, options: RangeOptions) -> Result<Vec<Vec<String>>, RangeError> {
    Ok(Range::parse_with_options(input, options)?
        .sets()
        .iter()
        .map(|set| set.iter().map(ToString::to_string).collect())
        .collect())
}

pub fn simplify(versions: &[&str], input: &str, options: RangeOptions) -> Option<String> {
    let range = Range::parse_with_options(input, options).ok()?;
    let mut parsed = versions
        .iter()
        .filter_map(|value| {
            parse_mode(value, options.loose)
                .ok()
                .map(|parsed| (*value, parsed))
        })
        .collect::<Vec<_>>();
    parsed.sort_by(|(_, left), (_, right)| left.compare(right));
    let sorted: Vec<_> = parsed.iter().map(|(value, _)| *value).collect();

    let mut groups: Vec<(&str, Option<&str>)> = Vec::new();
    let mut first = None;
    let mut previous = None;
    for version in &sorted {
        if range.test(version) {
            previous = Some(*version);
            first.get_or_insert(*version);
        } else if let Some(start) = first.take() {
            groups.push((start, previous));
            previous = None;
        }
    }
    if let Some(start) = first {
        groups.push((start, None));
    }

    let mut parts = Vec::new();
    for (minimum, maximum) in groups {
        if maximum == Some(minimum) {
            parts.push(minimum.to_owned());
        } else if maximum.is_none() && sorted.first() == Some(&minimum) {
            parts.push("*".to_owned());
        } else if maximum.is_none() {
            parts.push(format!(">={minimum}"));
        } else if let Some(maximum) = maximum {
            if sorted.first() == Some(&minimum) {
                parts.push(format!("<={maximum}"));
            } else {
                parts.push(format!("{minimum} - {maximum}"));
            }
        }
    }
    let simplified = parts.join(" || ");
    Some(if simplified.len() < input.len() {
        simplified
    } else {
        input.to_owned()
    })
}

#[derive(Clone, Copy)]
enum OutsideDirection {
    Higher,
    Lower,
}

fn outside(
    version: &str,
    range: &str,
    direction: OutsideDirection,
    options: RangeOptions,
) -> Result<bool, RangeError> {
    let version = parse_mode(version, options.loose).map_err(|_| RangeError(version.to_owned()))?;
    let range = Range::parse_with_options(range, options)?;
    if range.test_version(&version) {
        return Ok(false);
    }

    for set in range.sets() {
        let boundaries: Vec<_> = set
            .iter()
            .map(|comparator| {
                if comparator.operator() == ComparatorOperator::Any {
                    Comparator::parse(">=0.0.0", options.loose).expect("static comparator")
                } else {
                    comparator.clone()
                }
            })
            .collect();
        let Some(mut high) = boundaries.first() else {
            continue;
        };
        let mut low = high;
        for comparator in &boundaries[1..] {
            let comparator_version = comparator.semver().expect("concrete boundary");
            let high_version = high.semver().expect("concrete boundary");
            let low_version = low.semver().expect("concrete boundary");
            let high_ordering = comparator_version.compare(high_version);
            let low_ordering = comparator_version.compare(low_version);
            match direction {
                OutsideDirection::Higher => {
                    if high_ordering == Ordering::Greater {
                        high = comparator;
                    } else if low_ordering == Ordering::Less {
                        low = comparator;
                    }
                }
                OutsideDirection::Lower => {
                    if high_ordering == Ordering::Less {
                        high = comparator;
                    } else if low_ordering == Ordering::Greater {
                        low = comparator;
                    }
                }
            }
        }

        let (open_edge, closed_edge) = match direction {
            OutsideDirection::Higher => (
                ComparatorOperator::Greater,
                ComparatorOperator::GreaterOrEqual,
            ),
            OutsideDirection::Lower => (ComparatorOperator::Less, ComparatorOperator::LessOrEqual),
        };
        if matches!(operator_pair(high.operator()), pair if pair == open_edge || pair == closed_edge)
        {
            return Ok(false);
        }

        let ordering = version.compare(low.semver().expect("concrete boundary"));
        let beyond_or_equal = match direction {
            OutsideDirection::Higher => ordering != Ordering::Greater,
            OutsideDirection::Lower => ordering != Ordering::Less,
        };
        let strictly_beyond = match direction {
            OutsideDirection::Higher => ordering == Ordering::Less,
            OutsideDirection::Lower => ordering == Ordering::Greater,
        };
        if matches!(low.operator(), ComparatorOperator::Exact) && beyond_or_equal
            || low.operator() == open_edge && beyond_or_equal
            || low.operator() == closed_edge && strictly_beyond
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn operator_pair(operator: ComparatorOperator) -> ComparatorOperator {
    operator
}

fn increment_minimum(version: &SemVer) -> Option<SemVer> {
    if version.prerelease().is_empty() {
        SemVer::parse(&format!(
            "{}.{}.{}",
            version.major(),
            version.minor(),
            version.patch().checked_add(1)?
        ))
        .ok()
    } else {
        SemVer::parse(&format!("{}.0", version.version())).ok()
    }
}

fn parse_mode(input: &str, loose: bool) -> Result<SemVer, crate::ParseError> {
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
    fn finds_satisfying_extremes() {
        let versions = ["1.2.3", "1.2.6", "1.3.0", "bad"];
        assert_eq!(
            min_satisfying(&versions, "~1.2.3", RangeOptions::default()).as_deref(),
            Some("1.2.3")
        );
        assert_eq!(
            max_satisfying(&versions, "~1.2.3", RangeOptions::default()).as_deref(),
            Some("1.2.6")
        );
    }

    #[test]
    fn finds_minimum_versions() {
        for (range, expected) in [
            ("*", Some("0.0.0")),
            ("1.0.x", Some("1.0.0")),
            ("~1.1.1-beta", Some("1.1.1-beta")),
            (">1.0.0", Some("1.0.1")),
            (">1.0.0-beta", Some("1.0.0-beta.0")),
            (">4 <3", None),
        ] {
            assert_eq!(
                min_version(range, RangeOptions::default())
                    .as_ref()
                    .map(SemVer::version),
                expected,
                "{range}"
            );
        }
    }

    #[test]
    fn detects_versions_outside_ranges() {
        let options = RangeOptions::default();
        assert_eq!(greater_than_range("3.0.0", "^1.0.0", options), Ok(true));
        assert_eq!(greater_than_range("0.5.0", "^1.0.0", options), Ok(false));
        assert_eq!(less_than_range("0.5.0", "^1.0.0", options), Ok(true));
        assert_eq!(less_than_range("3.0.0", "^1.0.0", options), Ok(false));
    }
}
