use std::cmp::Ordering;
use std::env;
use std::process::ExitCode;

use semver_mortis::{
    Range, RangeOptions, ReleaseType, clean, clean_loose, coerce, compare, greater_than_range, inc,
    intersects, less_than_range, max_satisfying, min_satisfying, min_version, parse, parse_loose,
    valid, valid_range,
};

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
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

fn run(mut args: Vec<String>) -> Result<Option<String>, String> {
    let loose = take_flag(&mut args, "--loose");
    let include_prerelease = take_flag(&mut args, "--include-prerelease");
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
        "coerce" => Ok(coerce(required(&args, 1)?).map(|value| value.version().to_owned())),
        "compare" => Ok(Some(ordering_text(
            compare(required(&args, 1)?, required(&args, 2)?).map_err(|error| error.to_string())?,
        ))),
        "inc" => Ok(inc(required(&args, 1)?, release_type(required(&args, 2)?)?)),
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

fn usage() -> String {
    "usage: semver-mortis [--loose] [--include-prerelease] <valid|clean|parse|coerce|compare|inc|range|satisfies|valid-range|min-version|gtr|ltr|intersects|max-satisfying|min-satisfying> ...".into()
}
