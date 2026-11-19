//! The text of `--help` shows an example of every kind of operation: every example line runs
//! without an error, and every function and every constant of the program is in one of them.

use std::process::{Command, Stdio};

fn program() -> Command {
    Command::new(env!("CARGO_BIN_EXE_symbolic-math"))
}

fn output(args: &[&str]) -> (String, String, bool) {
    let out = program()
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("symbolic-math runs");
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
    (text(&out.stdout), text(&out.stderr), out.status.success())
}

/// The arguments of an example line: options and inputs in quotes, up to the remark after `#`.
fn arguments(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            ' ' => {
                chars.next();
            }
            '#' => break,
            '\'' | '"' => {
                chars.next();
                args.push(chars.by_ref().take_while(|x| *x != c).collect());
            }
            _ => {
                let mut word = String::new();
                while let Some(x) = chars.next_if(|x| *x != ' ') {
                    word.push(x);
                }
                args.push(word);
            }
        }
    }
    args
}

/// The example lines of the help text: those after the word `Examples` that start with two
/// spaces and a quote or an option.
fn examples(help: &str) -> Vec<Vec<String>> {
    help.lines()
        .skip_while(|l| !l.starts_with("Examples"))
        .filter(|l| l.starts_with("  ") && l[2..].starts_with(['\'', '"', '-']))
        .map(arguments)
        .collect()
}

/// The first name of every entry of a list like the one of `--functions`.
fn first_names(list: &str) -> Vec<String> {
    list.lines()
        .filter(|l| l.starts_with("    "))
        .filter_map(|l| {
            l.split([',', ' '])
                .find(|w| !w.is_empty())
                .map(str::to_string)
        })
        .collect()
}

/// Whether `name` stands in `text` as a whole word, followed by one of `after` when given.
fn has_word(text: &str, name: &str, after: &[char]) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(name).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let next = text[at + name.len()..].chars().next();
        !before.is_some_and(word)
            && if after.is_empty() {
                !next.is_some_and(word)
            } else {
                next.is_some_and(|c| after.contains(&c))
            }
    })
}

#[test]
fn both_spellings_print_the_same_text() {
    let (long, _, ok) = output(&["--help"]);
    assert!(ok && long.starts_with("Usage: symbolic-math"));
    assert_eq!(output(&["-h"]).0, long);
    // (lines fit a terminal of 100 columns)
    let wide: Vec<&str> = long.lines().filter(|l| l.chars().count() > 100).collect();
    assert!(wide.is_empty(), "lines longer than 100 columns: {wide:?}");
}

#[test]
fn every_example_runs() {
    let (help, _, _) = output(&["--help"]);
    let lines = examples(&help);
    assert!(lines.len() > 150, "only {} example lines", lines.len());
    let mut failures = Vec::new();
    for args in &lines {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let (out, err, ok) = output(&refs);
        // (the tet program may be missing where the tests run)
        let no_tet = err.contains("could not run tet") || out.contains("could not run tet");
        if (!ok || !err.is_empty() || out.trim().is_empty()) && !no_tet {
            failures.push(format!("{args:?}\n  out: {out}\n  err: {err}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} example lines fail:\n{}",
        failures.len(),
        lines.len(),
        failures.join("\n")
    );
}

#[test]
fn every_function_and_constant_has_an_example() {
    let (help, _, _) = output(&["--help"]);
    let text: String = examples(&help)
        .iter()
        .flatten()
        .map(|a| format!("{a} "))
        .collect();
    let functions = first_names(&output(&["--functions"]).0);
    assert!(functions.len() > 500, "only {} functions", functions.len());
    let missing: Vec<&String> = functions
        .iter()
        .filter(|f| !has_word(&text, f, &['(', '[']))
        .collect();
    assert!(
        missing.is_empty(),
        "functions without an example: {missing:?}"
    );
    let constants = first_names(&output(&["--constants"]).0);
    assert!(constants.len() > 40, "only {} constants", constants.len());
    let missing: Vec<&String> = constants
        .iter()
        .filter(|c| !has_word(&text, c, &[]))
        .collect();
    assert!(
        missing.is_empty(),
        "constants without an example: {missing:?}"
    );
}

#[test]
fn the_options_and_the_forms_of_input_have_examples() {
    let (help, _, _) = output(&["--help"]);
    let lines = examples(&help);
    for option in ["-r", "-q", "-n", "-e", "-d"] {
        assert!(
            lines.iter().any(|l| l[0] == option),
            "no example with the option {option}"
        );
    }
    let text: String = lines.iter().flatten().map(|a| format!("{a} ")).collect();
    for form in [
        ":=", "->", ";", "[[", "==", "!=", "<=", ">=", "^^", "+/-", "±", "!", "!!", "%", "'(", "°",
        "|", "[1/2](", " and ", " or ", "∑", "∏", "∫", "∂", "√", "π", "∞", "²", "≤", "0x", "0b",
        "0.(3)", "d/dx", "**", " mod ", "∈",
    ] {
        assert!(text.contains(form), "no example with {form}");
    }
}
