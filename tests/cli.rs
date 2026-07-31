use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_semver-mortis"))
        .args(args)
        .output()
        .expect("native CLI should start")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone())
        .expect("CLI stdout should be UTF-8")
        .trim()
        .to_owned()
}

#[test]
fn validates_and_compares_versions() {
    let valid = run(&["valid", "1.2.3-beta.1+build.7"]);
    assert!(valid.status.success());
    assert_eq!(stdout(&valid), "1.2.3-beta.1");

    let compare = run(&["compare", "1.2.3-beta.1", "1.2.3"]);
    assert!(compare.status.success());
    assert_eq!(stdout(&compare), "-1");
}

#[test]
fn evaluates_npm_ranges() {
    let output = run(&["satisfies", "1.8.4", "^1.2.3 || >=3"]);
    assert!(output.status.success());
    assert_eq!(stdout(&output), "true");

    let minimum = run(&["min-version", ">1.2.3 <2"]);
    assert!(minimum.status.success());
    assert_eq!(stdout(&minimum), "1.2.4");
}

#[test]
fn supports_coercion_and_increment_options() {
    let coerce = run(&["--rtl", "coerce", "release-1.2.3.4"]);
    assert!(coerce.status.success());
    assert_eq!(stdout(&coerce), "2.3.4");

    let increment = run(&[
        "--identifier",
        "beta",
        "--identifier-base",
        "1",
        "inc",
        "1.2.3",
        "preminor",
    ]);
    assert!(increment.status.success());
    assert_eq!(stdout(&increment), "1.3.0-beta.1");
}

#[test]
fn reports_invalid_input_without_panicking() {
    let output = run(&["compare", "not-a-version", "1.2.3"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}
