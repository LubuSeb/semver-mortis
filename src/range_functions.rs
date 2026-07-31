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

/// Return whether every version admitted by `sub` is also admitted by `domain`.
pub fn subset(sub: &str, domain: &str, options: RangeOptions) -> Result<bool, RangeError> {
    if sub == domain {
        return Ok(true);
    }
    let sub = Range::parse_with_options(sub, options)?;
    let domain = Range::parse_with_options(domain, options)?;
    let mut saw_non_null = false;

    'outer: for simple_sub in sub.sets() {
        for simple_domain in domain.sets() {
            let result = simple_subset(simple_sub, simple_domain, options);
            saw_non_null |= result.is_some();
            if result == Some(true) {
                continue 'outer;
            }
        }
        if saw_non_null {
            return Ok(false);
        }
    }
    Ok(true)
}

fn simple_subset(sub: &[Comparator], domain: &[Comparator], options: RangeOptions) -> Option<bool> {
    if sub == domain {
        return Some(true);
    }

    let minimum = || {
        Comparator::parse(
            if options.include_prerelease {
                ">=0.0.0-0"
            } else {
                ">=0.0.0"
            },
            false,
        )
        .expect("static comparator")
    };
    let mut sub_storage = Vec::new();
    let sub = if is_any_set(sub) {
        if is_any_set(domain) {
            return Some(true);
        }
        sub_storage.push(minimum());
        &sub_storage
    } else {
        sub
    };
    let mut domain_storage = Vec::new();
    let domain = if is_any_set(domain) {
        if options.include_prerelease {
            return Some(true);
        }
        domain_storage.push(minimum());
        &domain_storage
    } else {
        domain
    };

    let mut exact = Vec::new();
    let mut greatest_lower: Option<&Comparator> = None;
    let mut least_upper: Option<&Comparator> = None;
    for comparator in sub {
        match comparator.operator() {
            ComparatorOperator::Greater | ComparatorOperator::GreaterOrEqual => {
                greatest_lower = Some(higher_gt(greatest_lower, comparator));
            }
            ComparatorOperator::Less | ComparatorOperator::LessOrEqual => {
                least_upper = Some(lower_lt(least_upper, comparator));
            }
            ComparatorOperator::Exact => exact.push(comparator),
            ComparatorOperator::Any => {}
        }
    }
    if exact.len() > 1 {
        return None;
    }

    let bound_ordering = match (greatest_lower, least_upper) {
        (Some(lower), Some(upper)) => {
            let ordering = lower
                .semver()
                .expect("bound")
                .compare(upper.semver().expect("bound"));
            if ordering == Ordering::Greater
                || (ordering == Ordering::Equal
                    && (lower.operator() != ComparatorOperator::GreaterOrEqual
                        || upper.operator() != ComparatorOperator::LessOrEqual))
            {
                return None;
            }
            Some(ordering)
        }
        _ => None,
    };

    if let Some(exact) = exact.first() {
        let version = exact.semver().expect("exact version");
        if greatest_lower.is_some_and(|bound| !bound.test_version(version))
            || least_upper.is_some_and(|bound| !bound.test_version(version))
        {
            return None;
        }
        return Some(
            domain
                .iter()
                .all(|comparator| single_comparator_satisfies(version, comparator, options)),
        );
    }

    let mut need_lower_prerelease = greatest_lower
        .and_then(Comparator::semver)
        .filter(|version| !options.include_prerelease && !version.prerelease().is_empty());
    let mut need_upper_prerelease = least_upper
        .and_then(Comparator::semver)
        .filter(|version| !options.include_prerelease && !version.prerelease().is_empty());
    if let (Some(bound), Some(version)) = (least_upper, need_upper_prerelease)
        && bound.operator() == ComparatorOperator::Less
        && version.prerelease().len() == 1
        && version.prerelease()[0] == crate::Identifier::Numeric(0)
    {
        need_upper_prerelease = None;
    }

    let mut has_domain_lower = false;
    let mut has_domain_upper = false;
    for comparator in domain {
        has_domain_lower |= matches!(
            comparator.operator(),
            ComparatorOperator::Greater | ComparatorOperator::GreaterOrEqual
        );
        has_domain_upper |= matches!(
            comparator.operator(),
            ComparatorOperator::Less | ComparatorOperator::LessOrEqual
        );

        if let Some(lower) = greatest_lower {
            clear_matching_prerelease(&mut need_lower_prerelease, comparator);
            if matches!(
                comparator.operator(),
                ComparatorOperator::Greater | ComparatorOperator::GreaterOrEqual
            ) {
                if std::ptr::eq(higher_gt(Some(lower), comparator), comparator) {
                    return Some(false);
                }
            } else if lower.operator() == ComparatorOperator::GreaterOrEqual
                && !comparator.test_version(lower.semver().expect("bound"))
            {
                return Some(false);
            }
        }

        if let Some(upper) = least_upper {
            clear_matching_prerelease(&mut need_upper_prerelease, comparator);
            if matches!(
                comparator.operator(),
                ComparatorOperator::Less | ComparatorOperator::LessOrEqual
            ) {
                if std::ptr::eq(lower_lt(Some(upper), comparator), comparator) {
                    return Some(false);
                }
            } else if upper.operator() == ComparatorOperator::LessOrEqual
                && !comparator.test_version(upper.semver().expect("bound"))
            {
                return Some(false);
            }
        }

        if comparator.operator() == ComparatorOperator::Exact
            && (least_upper.is_some() || greatest_lower.is_some())
            && bound_ordering != Some(Ordering::Equal)
        {
            return Some(false);
        }
    }

    if greatest_lower.is_some()
        && has_domain_upper
        && least_upper.is_none()
        && bound_ordering != Some(Ordering::Equal)
    {
        return Some(false);
    }
    if least_upper.is_some()
        && has_domain_lower
        && greatest_lower.is_none()
        && bound_ordering != Some(Ordering::Equal)
    {
        return Some(false);
    }
    if need_lower_prerelease.is_some() || need_upper_prerelease.is_some() {
        return Some(false);
    }
    Some(true)
}

fn is_any_set(set: &[Comparator]) -> bool {
    set.len() == 1 && set[0].operator() == ComparatorOperator::Any
}

fn higher_gt<'a>(current: Option<&'a Comparator>, candidate: &'a Comparator) -> &'a Comparator {
    let Some(current) = current else {
        return candidate;
    };
    match current
        .semver()
        .expect("bound")
        .compare(candidate.semver().expect("bound"))
    {
        Ordering::Greater => current,
        Ordering::Less => candidate,
        Ordering::Equal
            if candidate.operator() == ComparatorOperator::Greater
                && current.operator() == ComparatorOperator::GreaterOrEqual =>
        {
            candidate
        }
        Ordering::Equal => current,
    }
}

fn lower_lt<'a>(current: Option<&'a Comparator>, candidate: &'a Comparator) -> &'a Comparator {
    let Some(current) = current else {
        return candidate;
    };
    match current
        .semver()
        .expect("bound")
        .compare(candidate.semver().expect("bound"))
    {
        Ordering::Less => current,
        Ordering::Greater => candidate,
        Ordering::Equal
            if candidate.operator() == ComparatorOperator::Less
                && current.operator() == ComparatorOperator::LessOrEqual =>
        {
            candidate
        }
        Ordering::Equal => current,
    }
}

fn clear_matching_prerelease(needed: &mut Option<&SemVer>, comparator: &Comparator) {
    let (Some(expected), Some(candidate)) = (*needed, comparator.semver()) else {
        return;
    };
    if !candidate.prerelease().is_empty()
        && candidate.major() == expected.major()
        && candidate.minor() == expected.minor()
        && candidate.patch() == expected.patch()
    {
        *needed = None;
    }
}

fn single_comparator_satisfies(
    version: &SemVer,
    comparator: &Comparator,
    options: RangeOptions,
) -> bool {
    if !comparator.test_version(version) {
        return false;
    }
    if version.prerelease().is_empty() || options.include_prerelease {
        return true;
    }
    comparator.semver().is_some_and(|allowed| {
        !allowed.prerelease().is_empty()
            && allowed.major() == version.major()
            && allowed.minor() == version.minor()
            && allowed.patch() == version.patch()
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

    #[test]
    fn checks_range_subsets() {
        let options = RangeOptions::default();
        assert_eq!(subset("1.2.x", "^1.0.0", options), Ok(true));
        assert_eq!(subset("^1.0.0", "1.2.x", options), Ok(false));
        assert_eq!(subset(">2 <1", "1.x", options), Ok(true));
        assert_eq!(subset("1.2.3 || 2.0.0", ">=1.0.0", options), Ok(true));
    }
}
