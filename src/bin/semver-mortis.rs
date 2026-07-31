use std::cmp::Ordering;
use std::env;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use semver_mortis::{
    CoerceOptions, Comparator, Identifier, IdentifierBase, Range, RangeOptions, ReleaseType, clean,
    clean_loose, coerce, coerce_with_options, compare, compare_build, compare_loose, diff,
    greater_than_range, inc, inc_with_options, intersects, less_than_range, max_satisfying,
    min_satisfying, min_version, parse, parse_loose, subset, truncate, valid, valid_range,
};

fn main() -> ExitCode {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.first().is_some_and(|argument| argument == "serve") {
        return serve();
    }
    match run(args) {
        Ok(Some(output)) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Ok(None) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn serve() -> ExitCode {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            return ExitCode::from(2);
        };
        let decoded = line
            .split('\t')
            .map(decode_hex)
            .collect::<Result<Vec<_>, _>>();
        let response = match decoded.and_then(run) {
            Ok(Some(value)) => format!("some\t{}", encode_hex(&value)),
            Ok(None) => "none".to_owned(),
            Err(error) => format!("error\t{}", encode_hex(&error)),
        };
        if writeln!(stdout, "{response}").is_err() || stdout.flush().is_err() {
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

fn run(mut args: Vec<String>) -> Result<Option<String>, String> {
    let loose = take_flag(&mut args, "--loose");
    let include_prerelease = take_flag(&mut args, "--include-prerelease");
    let rtl = take_flag(&mut args, "--rtl");
    let identifier = take_option(&mut args, "--identifier");
    let identifier_base = take_option(&mut args, "--identifier-base")
        .as_deref()
        .map(identifier_base)
        .transpose()?
        .unwrap_or_default();
    let command = args.first().map(String::as_str).ok_or_else(usage)?;
    match command {
        "valid" => {
            let input = required(&args, 1)?;
            Ok(if loose {
                parse_loose(input)
                    .ok()
                    .map(|value| value.version().to_owned())
            } else {
                valid(input)
            })
        }
        "clean" => Ok(if loose {
            clean_loose(required(&args, 1)?)
        } else {
            clean(required(&args, 1)?)
        }),
        "parse" => Ok(if loose {
            parse_loose(required(&args, 1)?)
        } else {
            parse(required(&args, 1)?)
        }
        .ok()
        .map(|value| value.version().to_owned())),
        "inspect" => {
            let version = if loose {
                parse_loose(required(&args, 1)?)
            } else {
                parse(required(&args, 1)?)
            }
            .map_err(|error| error.to_string())?;
            Ok(Some(inspect_version(&version)))
        }
        "coerce" | "coerce-full" => {
            let value = if rtl || include_prerelease {
                coerce_with_options(
                    required(&args, 1)?,
                    CoerceOptions {
                        rtl,
                        include_prerelease,
                    },
                )
            } else {
                coerce(required(&args, 1)?)
            };
            Ok(value.map(|value| {
                if command == "coerce-full" {
                    value.raw().to_owned()
                } else {
                    value.version().to_owned()
                }
            }))
        }
        "compare" => Ok(Some(ordering_text(
            (if loose {
                compare_loose(required(&args, 1)?, required(&args, 2)?)
            } else {
                compare(required(&args, 1)?, required(&args, 2)?)
            })
            .map_err(|error| error.to_string())?,
        ))),
        "compare-build" => Ok(Some(ordering_text(if loose {
            let left = parse_loose(required(&args, 1)?).map_err(|error| error.to_string())?;
            let right = parse_loose(required(&args, 2)?).map_err(|error| error.to_string())?;
            left.compare(&right)
                .then_with(|| left.compare_build(&right))
        } else {
            compare_build(required(&args, 1)?, required(&args, 2)?)
                .map_err(|error| error.to_string())?
        }))),
        "diff" => Ok(diff(required(&args, 1)?, required(&args, 2)?)
            .map_err(|error| error.to_string())?
            .map(release_name)
            .map(str::to_owned)),
        "comparator" => Ok(Some(
            Comparator::parse(required(&args, 1)?, loose)
                .map_err(|error| error.to_string())?
                .value()
                .to_owned(),
        )),
        "inc" => {
            let release = release_type(required(&args, 2)?)?;
            Ok(
                if identifier.is_some() || identifier_base != IdentifierBase::Zero || loose {
                    inc_with_options(
                        required(&args, 1)?,
                        release,
                        loose,
                        identifier.as_deref(),
                        identifier_base,
                    )
                } else {
                    inc(required(&args, 1)?, release)
                },
            )
        }
        "truncate" => Ok(truncate(
            required(&args, 1)?,
            release_type(required(&args, 2)?)?,
            loose,
        )),
        "range" => Ok(Some(
            Range::parse_with_options(
                required(&args, 1)?,
                RangeOptions {
                    loose,
                    include_prerelease,
                },
            )
            .map_err(|error| error.to_string())?
            .range()
            .to_owned(),
        )),
        "satisfies" => Ok(Some(
            Range::parse_with_options(
                required(&args, 2)?,
                RangeOptions {
                    loose,
                    include_prerelease,
                },
            )
            .map_err(|error| error.to_string())?
            .test(required(&args, 1)?)
            .to_string(),
        )),
        "valid-range" => Ok(valid_range(
            required(&args, 1)?,
            RangeOptions {
                loose,
                include_prerelease,
            },
        )),
        "min-version" => Ok(min_version(
            required(&args, 1)?,
            RangeOptions {
                loose,
                include_prerelease,
            },
        )
        .map(|version| version.version().to_owned())),
        "gtr" | "ltr" => {
            let options = RangeOptions {
                loose,
                include_prerelease,
            };
            let result = if command == "gtr" {
                greater_than_range(required(&args, 1)?, required(&args, 2)?, options)
            } else {
                less_than_range(required(&args, 1)?, required(&args, 2)?, options)
            };
            Ok(Some(result.map_err(|error| error.to_string())?.to_string()))
        }
        "intersects" => Ok(Some(
            intersects(
                required(&args, 1)?,
                required(&args, 2)?,
                RangeOptions {
                    loose,
                    include_prerelease,
                },
            )
            .map_err(|error| error.to_string())?
            .to_string(),
        )),
        "subset" => Ok(Some(
            subset(
                required(&args, 1)?,
                required(&args, 2)?,
                RangeOptions {
                    loose,
                    include_prerelease,
                },
            )
            .map_err(|error| error.to_string())?
            .to_string(),
        )),
        "max-satisfying" | "min-satisfying" => {
            let range = required(&args, 1)?;
            let versions: Vec<_> = args.iter().skip(2).map(String::as_str).collect();
            let options = RangeOptions {
                loose,
                include_prerelease,
            };
            Ok(if command == "max-satisfying" {
                max_satisfying(&versions, range, options)
            } else {
                min_satisfying(&versions, range, options)
            })
        }
        _ => Err(usage()),
    }
}

fn take_flag(args: &mut Vec<String>, flag: &str) -> bool {
    if let Some(index) = args.iter().position(|argument| argument == flag) {
        args.remove(index);
        true
    } else {
        false
    }
}

fn take_option(args: &mut Vec<String>, flag: &str) -> Option<String> {
    let index = args.iter().position(|argument| argument == flag)?;
    args.remove(index);
    (index < args.len()).then(|| args.remove(index))
}

fn required(args: &[String], index: usize) -> Result<&str, String> {
    args.get(index).map(String::as_str).ok_or_else(usage)
}

fn ordering_text(ordering: Ordering) -> String {
    match ordering {
        Ordering::Less => "-1",
        Ordering::Equal => "0",
        Ordering::Greater => "1",
    }
    .to_owned()
}

fn release_type(value: &str) -> Result<ReleaseType, String> {
    match value {
        "major" => Ok(ReleaseType::Major),
        "premajor" => Ok(ReleaseType::Premajor),
        "minor" => Ok(ReleaseType::Minor),
        "preminor" => Ok(ReleaseType::Preminor),
        "patch" => Ok(ReleaseType::Patch),
        "prepatch" => Ok(ReleaseType::Prepatch),
        "prerelease" => Ok(ReleaseType::Prerelease),
        "release" => Ok(ReleaseType::Release),
        _ => Err(format!("invalid release type: {value}")),
    }
}

fn release_name(value: ReleaseType) -> &'static str {
    match value {
        ReleaseType::Major => "major",
        ReleaseType::Premajor => "premajor",
        ReleaseType::Minor => "minor",
        ReleaseType::Preminor => "preminor",
        ReleaseType::Patch => "patch",
        ReleaseType::Prepatch => "prepatch",
        ReleaseType::Prerelease => "prerelease",
        ReleaseType::Release => "release",
    }
}

fn inspect_version(version: &semver_mortis::SemVer) -> String {
    let prerelease = version
        .prerelease()
        .iter()
        .map(|identifier| match identifier {
            Identifier::Numeric(value) => format!("n:{value}"),
            Identifier::Text(value) => format!("s:{value}"),
        })
        .collect::<Vec<_>>()
        .join(",");
    [
        version.raw().to_owned(),
        version.version().to_owned(),
        version.major().to_string(),
        version.minor().to_string(),
        version.patch().to_string(),
        version.is_loose().to_string(),
        prerelease,
        version.build().join(","),
    ]
    .join("\u{1f}")
}

fn identifier_base(value: &str) -> Result<IdentifierBase, String> {
    match value {
        "0" => Ok(IdentifierBase::Zero),
        "1" => Ok(IdentifierBase::One),
        "false" | "omit" => Ok(IdentifierBase::Omit),
        _ => Err(format!("invalid identifier base: {value}")),
    }
}

fn usage() -> String {
    "usage: semver-mortis [--loose] [--rtl] [--include-prerelease] [--identifier ID] [--identifier-base 0|1|false] <valid|clean|parse|inspect|coerce|coerce-full|compare|compare-build|diff|comparator|inc|truncate|range|satisfies|valid-range|min-version|gtr|ltr|intersects|subset|max-satisfying|min-satisfying> ...".into()
}

fn encode_hex(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn decode_hex(value: &str) -> Result<String, String> {
    if value.len() % 2 != 0 {
        return Err("invalid protocol hex".into());
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = hex_digit(pair[0])?;
            let low = hex_digit(pair[1])?;
            Ok((high << 4) | low)
        })
        .collect::<Result<Vec<_>, String>>()?;
    String::from_utf8(bytes).map_err(|_| "invalid protocol utf-8".into())
}

fn hex_digit(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err("invalid protocol hex".into()),
    }
}
