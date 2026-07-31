use crate::SemVer;

const MAX_COMPONENT_LENGTH: usize = 16;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CoerceOptions {
    pub rtl: bool,
    pub include_prerelease: bool,
}

pub fn coerce(input: &str) -> Option<SemVer> {
    coerce_with_options(input, CoerceOptions::default())
}

pub fn coerce_with_options(input: &str, options: CoerceOptions) -> Option<SemVer> {
    let candidates: Vec<_> = (0..input.len())
        .filter_map(|start| candidate_at(input, start, options.include_prerelease))
        .collect();

    let candidate = if options.rtl {
        candidates.into_iter().max_by(|left, right| {
            left.end
                .cmp(&right.end)
                .then_with(|| right.start.cmp(&left.start))
        })
    } else {
        candidates.into_iter().next()
    }?;

    SemVer::parse(&candidate.version).ok()
}

#[derive(Clone, Debug)]
struct Candidate {
    start: usize,
    end: usize,
    version: String,
}

fn candidate_at(input: &str, start: usize, include_prerelease: bool) -> Option<Candidate> {
    let bytes = input.as_bytes();
    if !bytes.get(start).is_some_and(u8::is_ascii_digit)
        || start
            .checked_sub(1)
            .and_then(|index| bytes.get(index))
            .is_some_and(u8::is_ascii_digit)
    {
        return None;
    }

    let (major, mut cursor) = read_component(input, start)?;
    let mut minor = "0";
    let mut patch = "0";

    if bytes.get(cursor) == Some(&b'.')
        && let Some((value, end)) = read_component(input, cursor + 1)
    {
        minor = value;
        cursor = end;
        if bytes.get(cursor) == Some(&b'.')
            && let Some((value, end)) = read_component(input, cursor + 1)
        {
            patch = value;
            cursor = end;
        }
    }

    if bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
        return None;
    }

    let mut suffix = String::new();
    if include_prerelease {
        let (parsed_suffix, end) = read_suffix(&input[cursor..]);
        suffix = parsed_suffix;
        cursor += end;
    }

    Some(Candidate {
        start,
        end: cursor,
        version: format!("{major}.{minor}.{patch}{suffix}"),
    })
}

fn read_component(input: &str, start: usize) -> Option<(&str, usize)> {
    let bytes = input.as_bytes();
    let mut end = start;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    let length = end.checked_sub(start)?;
    (length != 0 && length <= MAX_COMPONENT_LENGTH).then(|| (&input[start..end], end))
}

fn read_suffix(input: &str) -> (String, usize) {
    let bytes = input.as_bytes();
    let mut cursor = 0;
    let mut suffix = String::new();

    if bytes.first() == Some(&b'-')
        && let Some(end) = read_identifier_sequence(input, 1)
    {
        suffix.push_str(&input[..end]);
        cursor = end;
    }

    if bytes.get(cursor) == Some(&b'+')
        && let Some(end) = read_identifier_sequence(input, cursor + 1)
    {
        suffix.push_str(&input[cursor..end]);
        cursor = end;
    }

    (suffix, cursor)
}

fn read_identifier_sequence(input: &str, start: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut cursor = start;
    let mut identifier_start = start;
    while let Some(byte) = bytes.get(cursor) {
        if byte.is_ascii_alphanumeric() || *byte == b'-' {
            cursor += 1;
        } else if *byte == b'.' && cursor != identifier_start {
            cursor += 1;
            identifier_start = cursor;
        } else {
            break;
        }
    }
    (cursor != identifier_start && cursor != start).then_some(cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coerces_leftmost_versions() {
        for (input, expected) in [
            (".1", "1.0.0"),
            ("version1.1", "1.1.0"),
            ("42.6.7.9.3-alpha", "42.6.7"),
            ("v3.4 replaces v3.3.1", "3.4.0"),
        ] {
            assert_eq!(coerce(input).unwrap().version(), expected);
        }
    }

    #[test]
    fn can_coerce_from_the_right() {
        let options = CoerceOptions {
            rtl: true,
            include_prerelease: false,
        };
        assert_eq!(
            coerce_with_options("1.2.3.4.5.6", options)
                .unwrap()
                .version(),
            "4.5.6"
        );
        assert_eq!(
            coerce_with_options("1.2.3.4", options).unwrap().version(),
            "2.3.4"
        );
    }

    #[test]
    fn optionally_preserves_prerelease_and_build() {
        let options = CoerceOptions {
            rtl: false,
            include_prerelease: true,
        };
        let version = coerce_with_options("release 1.2-rc.5+rev.6/a", options).unwrap();
        assert_eq!(version.version(), "1.2.0-rc.5");
        assert_eq!(version.build(), &["rev", "6"]);
    }
}
