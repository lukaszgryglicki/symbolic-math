//! The text of `--help` shows an example of every kind of operation: every example line runs
//! without an error, and every function and every constant of the program is in one of them.

use std::process::{Command, Stdio};

fn program() -> Command {
    Command::new(env!("CARGO_BIN_EXE_symbolic-math"))
}

/// A directory for the pictures that examples save: nothing is written into the repository.
/// Every test has its own, and removes it at its end.
fn scratch() -> std::path::PathBuf {
    let thread: String = format!("{:?}", std::thread::current().id())
        .chars()
        .filter(char::is_ascii_digit)
        .collect();
    let dir = std::env::temp_dir().join(format!(
        "symbolic-math-help-{}-{thread}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("a directory for the pictures");
    dir
}

fn tidy() {
    let _ = std::fs::remove_dir_all(scratch());
}

fn output(args: &[&str]) -> (String, String, bool) {
    let out = program()
        .args(args)
        .current_dir(scratch())
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
    // (an input that is too long for a line is cut into pieces in quotes, with a backslash after
    // all but the last: the shell joins them into one argument)
    let help = help.replace("'\\\n'", "");
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
    tidy();
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
    // the examples that save a picture wrote a file each
    let saved: Vec<String> = std::fs::read_dir(scratch())
        .expect("the directory of the pictures")
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .collect();
    for name in [
        "walk.png",
        "walk.jxl",
        "turn.png",
        "slices.jxl",
        "plot.png",
        "complex.png",
        "bar.png",
        "pie.png",
        "p11.png",
        "log.jxl",
        "gray.png",
        "rgb.jxl",
        "deep.png",
        "m.csv",
    ] {
        assert!(
            saved.iter().any(|s| s == name),
            "{name} was not saved: {saved:?}"
        );
    }
    assert!(saved.iter().all(|s| !s.ends_with(".part")), "{saved:?}");
    tidy();
}

/// The pictures of the repository as the help text lists them after the word `Pictures`:
/// the name of every file on a line of its own, the input that made it on the lines after it.
fn pictures(help: &str) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = Vec::new();
    for line in help
        .lines()
        .skip_while(|l| !l.starts_with("Pictures:"))
        .skip(1)
        .take_while(|l| !l.trim().is_empty())
    {
        if let Some(name) = line.strip_prefix("    ").filter(|l| !l.starts_with(' ')) {
            found.push((name.to_string(), String::new()));
        } else if line.starts_with("        ")
            && let Some((_, input)) = found.last_mut()
        {
            if !input.is_empty() {
                input.push(' ');
            }
            input.push_str(line.trim());
        }
    }
    found
}

/// The arguments of a call `name(a, b, ...)`, apart at the commas outside of its brackets
/// and quotes.
fn arguments_of(call: &str) -> Vec<String> {
    let inner = &call[call.find('(').map_or(0, |at| at + 1)..call.len() - 1];
    let (mut depth, mut quoted) = (0, false);
    let (mut all, mut one) = (Vec::new(), String::new());
    for c in inner.chars() {
        match c {
            '"' => quoted = !quoted,
            '(' | '[' | '{' if !quoted => depth += 1,
            ')' | ']' | '}' if !quoted => depth -= 1,
            ',' if !quoted && depth == 0 => {
                all.push(one.trim().to_string());
                one.clear();
                continue;
            }
            _ => {}
        }
        one.push(c);
    }
    all.push(one.trim().to_string());
    all
}

/// Every picture of the directory `jxls` stands in the help text with the input that made
/// it. The inputs take minutes each as they are; with small sizes every one of them runs.
#[test]
fn the_pictures_of_the_repository_have_their_inputs() {
    let (help, _, _) = output(&["--help"]);
    let listed = pictures(&help);
    assert!(listed.len() >= 80, "only {} pictures", listed.len());
    let names: std::collections::BTreeSet<&str> =
        listed.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names.len(), listed.len(), "a name stands twice");
    // the files of the directory are these, when the directory came with its pictures
    let there: std::collections::BTreeSet<String> =
        std::fs::read_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("jxls"))
            .map(|entries| {
                entries
                    .flatten()
                    .filter_map(|e| e.file_name().into_string().ok())
                    .filter(|name| name.ends_with(".jxl"))
                    .collect()
            })
            .unwrap_or_default();
    if !there.is_empty() {
        let missing: Vec<&&str> = names.iter().filter(|n| !there.contains(**n)).collect();
        let unlisted: Vec<&String> = there.iter().filter(|n| !names.contains(n.as_str())).collect();
        assert!(
            missing.is_empty() && unlisted.is_empty(),
            "not in jxls: {missing:?}; not in the help text: {unlisted:?}"
        );
    }
    let sizes = ["width", "height", "frames", "effort", "fps", "slices", "voxels"];
    for (name, input) in &listed {
        assert!(
            input.starts_with(&format!("save_jxl(\"{name}\", ")) && input.ends_with(')'),
            "{name}: {input}"
        );
        let all = arguments_of(input);
        let chosen = |choice: &str| all.iter().any(|a| a.starts_with(&format!("{choice} = ")));
        // (the values of tetration come from the program tet, which is not asked here)
        let drawing = &all[1];
        if ["^^", "slog(", "ssrt(", "tet("].iter().any(|t| drawing.contains(t)) {
            continue;
        }
        let mut small: Vec<String> = all
            .iter()
            .filter(|a| !sizes.iter().any(|s| a.starts_with(&format!("{s} = "))))
            .cloned()
            .collect();
        let more: &[&str] = if drawing.starts_with("plot4d(") {
            &["width = 10", "height = 10", "frames = 3"]
        } else if drawing.starts_with("view4d(") {
            &["width = 24", "voxels = 8"]
        } else {
            &["width = 48", "height = 27"]
        };
        small.extend(more.iter().map(|m| m.to_string()));
        small.push("effort = 1".to_string());
        if chosen("frames") && !drawing.starts_with("plot4d(") {
            small.push("frames = 2".to_string());
        }
        if chosen("slices") {
            small.push("slices = 2".to_string());
        }
        let small = format!("save_jxl({})", small.join(", "));
        let (out, err, ok) = output(&[&small]);
        assert!(ok && err.is_empty(), "{small}: {err}");
        let last = out.lines().last().unwrap_or_default();
        assert!(
            last.starts_with(&format!("saved {name}: JPEG XL, ")),
            "{small}: {last}"
        );
    }
    tidy();
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
    tidy();
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
        ":=",
        "->",
        ";",
        "[[",
        "==",
        "!=",
        "<=",
        ">=",
        "^^",
        "+/-",
        "±",
        "!",
        "!!",
        "%",
        "'(",
        "°",
        "|",
        "[1/2](",
        " and ",
        " or ",
        "∑",
        "∏",
        "∫",
        "∂",
        "√",
        "π",
        "∞",
        "²",
        "≤",
        "0x",
        "0b",
        "0.(3)",
        "d/dx",
        "**",
        " mod ",
        "∈",
        "0o",
        "16#",
        "1000#001#234",
        ".(#",
        "\"#",
        ".*",
        "./",
        ".^",
        ".\\",
        "1:5",
        "0:0.25:1",
        "] \\ [",
        "[1, 2; 3, 4]",
        "@(x)",
        "@plus",
        "~=",
        "~(",
        "!true",
        " & ",
        " || ",
    ] {
        assert!(text.contains(form), "no example with {form}");
    }
    // everything that is chosen for a picture
    for name in symbolic_math::picture::NAMED {
        assert!(
            has_word(&text, name, &[' ']) && text.contains(&format!("{name} = ")),
            "no example with {name} = ..."
        );
    }
    for word in symbolic_math::picture::WORDS {
        assert!(has_word(&text, word, &[')', ',']), "no example with {word}");
    }
    for (palette, _) in symbolic_math::raster::PALETTES {
        assert!(
            *palette == "rainbow" || *palette == "grey" || has_word(&help, palette, &[]),
            "the palette {palette} is not named"
        );
    }
    tidy();
}
