//! A broad sweep over everything the program does: every name of every function, numbers of
//! every kind, functions of functions, identities of matrices, lists and polynomials, calculus,
//! distributions, texts, conversions, the ways to write an input, programs, options and
//! refusals, and what MATLAB has area by area. The rows are in `tests/coverage/*.tsv` (options, input, what is printed, with
//! tabs between and `|` for the ends of lines); `gen_cov.py` wrote them and checked most of them
//! against other calculations: the exact arithmetic of Python, mpmath, and identities whose
//! answers are known beforehand.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// A directory of its own for what the rows of a table write.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "symbolic-math-coverage-{name}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("a directory for the files");
    dir
}

/// What the program prints for an input, with `|` for the ends of lines. No program `tet` is
/// found, so that no row depends on it.
fn answer(dir: &Path, flags: &str, input: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_symbolic-math"))
        .current_dir(dir)
        .env("PATH", dir)
        .env("HOME", dir)
        .args(flags.split_whitespace())
        .arg(input)
        .stdin(Stdio::null())
        .output()
        .expect("symbolic-math runs");
    let text = |bytes: &[u8]| {
        String::from_utf8_lossy(bytes)
            .trim_end_matches('\n')
            .replace('\n', "|")
    };
    let all = [text(&out.stdout), text(&out.stderr)]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("|");
    all.split('|').map(sizeless).collect::<Vec<_>>().join("|")
}

/// The size of a JPEG XL file is no part of what is compared: the encoder may pack a picture
/// a little differently on another kind of processor.
fn sizeless(line: &str) -> String {
    match (line.contains(": JPEG XL, "), line.rfind(", ")) {
        (true, Some(at)) if line.ends_with(" bytes") => format!("{}, N bytes", &line[..at]),
        _ => line.to_string(),
    }
}

fn rows(table: &str) -> impl Iterator<Item = (&str, &str, &str)> {
    table.lines().filter(|l| !l.is_empty()).map(|line| {
        let mut parts = line.splitn(3, '\t');
        let mut next = || parts.next().expect("three columns");
        (next(), next(), next())
    })
}

fn check(name: &str, table: &str) {
    let dir = scratch(name);
    let mut failures = Vec::new();
    let mut count = 0;
    for (flags, input, want) in rows(table) {
        count += 1;
        let got = answer(&dir, flags, input);
        if got != want {
            failures.push(format!("{flags} {input}\n  want: {want}\n  got:  {got}"));
        }
    }
    let _ = std::fs::remove_dir_all(dir);
    assert!(count > 0, "the table {name} is empty");
    assert!(
        failures.is_empty(),
        "{} of {count} rows of {name} differ:\n{}",
        failures.len(),
        failures
            .iter()
            .take(25)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

macro_rules! tables {
    ($($name:ident),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                check(stringify!($name), include_str!(concat!("coverage/", stringify!($name), ".tsv")));
            }
        )*
        const TABLES: &[&str] = &[$(include_str!(concat!("coverage/", stringify!($name), ".tsv"))),*];
    };
}

tables!(
    names_first_1,
    names_first_2,
    names_first_3,
    names_other,
    names_too_few_1,
    names_too_few_2,
    names_too_few_3,
    names_too_many_1,
    names_too_many_2,
    names_too_many_3,
    fractions_1,
    fractions_2,
    fractions_printed,
    rounding_1,
    rounding_2,
    whole_numbers_1,
    whole_numbers_2,
    remainders,
    complex_numbers_1,
    complex_numbers_2,
    decimals,
    decimals_printed,
    digits_of_bases_1,
    digits_of_bases_2,
    kinds_and_bits_1,
    kinds_and_bits_2,
    values_of_functions_1,
    values_of_functions_2,
    values_of_two_arguments,
    functions_of_functions_1,
    functions_of_functions_2,
    functions_of_functions_3,
    deep_compositions,
    special_places_1,
    special_places_2,
    special_places_3,
    special_places_4,
    special_places_5,
    special_places_6,
    special_places_7,
    special_places_8,
    special_places_9,
    special_places_10,
    special_places_11,
    special_places_12,
    special_places_13,
    special_places_14,
    special_places_15,
    special_places_16,
    special_places_17,
    special_places_18,
    special_places_19,
    special_places_20,
    special_places_21,
    special_places_22,
    special_places_23,
    special_places_24,
    special_places_25,
    special_places_26,
    special_places_27,
    matrices_1,
    matrices_2,
    matrices_of_any_shape,
    matrices_with_names,
    lists_of_numbers_1,
    lists_of_numbers_2,
    lists_of_numbers_3,
    lists_of_numbers_4,
    lists_of_numbers_5,
    lists_of_numbers_6,
    polynomials_1,
    polynomials_2,
    derivatives_of_integrals,
    rules_of_derivatives,
    partial_derivatives,
    known_values,
    series,
    transforms,
    whole_numbers_known_1,
    whole_numbers_known_2,
    distributions_known,
    distributions_there_and_back,
    texts_1,
    texts_2,
    conversions,
    ways_to_write,
    programs,
    quaternions,
    quaternions_known,
    options,
    refused,
    matlab_arrays,
    matlab_statistics,
    matlab_matrices,
    matlab_polynomials,
    matlab_numbers,
    matlab_special,
    matlab_texts,
    matlab_distributions,
    matlab_functions,
    matlab_expressions,
    matlab_graphs,
    matlab_more,
    matlab_calendar,
    matlab_patterns,
    matlab_tests,
    matlab_records,
    matlab_functionals,
    matlab_conditions,
    matlab_gallery,
    matlab_pencils,
    matlab_multivar,
    matlab_solvers,
    matlab_tables,
    matlab_code,
    matlab_control,
    matlab_signal,
    matlab_mapping,
    matlab_networks,
    plot4d_characters,
    plot4d_files,
    plot4d_jxl,
    quick_looks,
    palettes,
    plot4d_all,
    view4d,
    matlab_signal_more,
    matlab_control_more,
    plot4d_between,
    r7_control_mimo,
    r7_control_delays,
    r7_filters,
    r7_rasters_geoid,
    r7_convnets,
    r7_walks_of_chance,
    r7_solids4d,
    r7_cells,
    r7_streams_units,
    r7_matfiles,
    r7_blocks,
    r7_images,
    r7_odd_and_large_values,
    matlab_charts,
    checked_by_octave_1,
    checked_by_octave_2,
    checked_by_octave_maps,
    checked_by_scipy,
    checked_by_scipy_signal,
    checked_by_numpy,
    checked_by_python,
    checked_by_geographiclib_1,
    checked_by_geographiclib_2,
    checked_by_mpmath,
    checked_by_scipy_filters,
    checked_by_octave_filters,
    checked_by_scipy_control,
    checked_by_scipy_fir,
    checked_by_scipy_mimo,
    checked_by_scipy_delays,
    checked_by_numpy_rasters,
    checked_by_numpy_convnets,
    checked_by_qhull_solids,
    checked_by_scipy_blocks,
    checked_by_scipy_images,
    checked_by_scipy_sampling,
    checked_by_mpmath_laws,
    checked_by_mpmath_gauss,
    matlab_charts6,
    matlab_special_values,
    matlab_files,
    matlab_files6,
    matlab_pictures,
    matlab_pictures_known,
    settings_as_statements,
    roots_nearest_to_zero,
    sums_by_numbers,
    repeated_roots_and_no_roots,
    inverse_sines_on_the_hyperbola,
    steps_told,
    programs_of_statements,
);

/// Whether `name(` or `name[` stands in a text, as the call of that function and of no longer
/// name.
fn calls(text: &str, name: &str) -> bool {
    text.match_indices(name).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + name.len()..].chars().next();
        !before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
            && matches!(after, Some('(' | '['))
    })
}

/// A function that is added without a row here fails this test: every name of every function
/// that `--functions` lists is called in some row.
#[test]
fn every_name_of_every_function_has_a_row() {
    let listed = Command::new(env!("CARGO_BIN_EXE_symbolic-math"))
        .arg("--functions")
        .output()
        .expect("symbolic-math runs");
    let listed = String::from_utf8_lossy(&listed.stdout).into_owned();
    let inputs: Vec<&str> = TABLES
        .iter()
        .flat_map(|t| rows(t))
        .map(|(_, input, _)| input)
        .collect();
    let all = inputs.join("\n");
    let mut names = 0;
    let mut missing = Vec::new();
    for line in listed.lines().skip(1).filter(|l| l.starts_with("    ")) {
        // the names stand first, with a comma after each but the last
        let mut words = line.split_whitespace();
        let mut of_line = Vec::new();
        for word in words.by_ref() {
            match word.strip_suffix(',') {
                Some(name) => of_line.push(name),
                None => {
                    of_line.push(word);
                    break;
                }
            }
        }
        for name in of_line {
            names += 1;
            // (signs that are brackets are no names that are called)
            if !name.chars().any(|c| c.is_alphanumeric()) || matches!(name, "‖" | "⌊" | "⌈") {
                continue;
            }
            if !calls(&all, name) {
                missing.push(name.to_string());
            }
        }
    }
    assert!(names > 1500, "only {names} names were listed");
    assert!(inputs.len() > 30000, "only {} rows", inputs.len());
    assert!(missing.is_empty(), "names without a row: {missing:?}");
}

/// The lists of names can be cut off by the one who reads them.
#[test]
fn a_reader_may_stop_early() {
    for option in ["--help", "--functions", "--constants", "--units"] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_symbolic-math"))
            .arg(option)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("symbolic-math runs");
        // (the pipe is closed before anything is read)
        drop(child.stdout.take());
        let out = child.wait_with_output().expect("symbolic-math ends");
        assert!(out.status.success(), "{option}: {:?}", out.status);
        assert!(
            out.stderr.is_empty(),
            "{option}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// Without an input the statements are read from the standard input, a line each; `quit`
/// ends them, and a line that fails does not end them.
#[test]
fn statements_from_the_standard_input() {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_symbolic-math"))
        .arg("-e")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("symbolic-math runs");
    child
        .stdin
        .take()
        .expect("a pipe")
        .write_all(b"a := 6\n# a remark\n\na*7\n1 +\nans + 1\nquit\n1/0\n")
        .expect("the lines are written");
    let out = child.wait_with_output().expect("symbolic-math ends");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "6\n42\n43\n");
    assert!(String::from_utf8_lossy(&out.stderr).starts_with("error: "));
    assert!(!out.status.success());
}

/// Several inputs share their definitions, and an option that is not known is an input.
#[test]
fn several_inputs_and_the_exit_code() {
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_symbolic-math"))
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("symbolic-math runs");
        (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
            out.status.code(),
        )
    };
    assert_eq!(
        run(&["-e", "a := 2", "a^10", "ans + 1"]).0,
        "2\n1024\n1025\n"
    );
    assert_eq!(run(&["-e", "1 + 1", "1 +", "2 + 2"]).0, "2\n4\n");
    assert_eq!(run(&["-e", "1 + 1", "1 +", "2 + 2"]).2, Some(1));
    assert_eq!(run(&["-e", "1 + 1"]).2, Some(0));
    assert_eq!(run(&["-d", "0", "pi"]).2, Some(2));
    assert_eq!(run(&["--digits=x", "pi"]).2, Some(2));
    assert_eq!(run(&["-d"]).2, Some(2));
    assert_eq!(run(&["-e", "--", "-1 + 3"]).0, "2\n");
    let (version, _, code) = run(&["--version"]);
    assert!(
        version.starts_with("symbolic-math ") && code == Some(0),
        "{version}"
    );
    assert_eq!(run(&["-V"]).0, version);
    assert_eq!(run(&["-h"]).0, run(&["--help"]).0);
    for (option, head) in [
        ("--functions", "Functions"),
        ("--constants", "Constants"),
        ("--units", "Units"),
    ] {
        let (text, errors, code) = run(&[option]);
        assert!(
            text.starts_with(head) && text.lines().count() > 20,
            "{option}"
        );
        assert!(errors.is_empty() && code == Some(0), "{option}");
    }
}
