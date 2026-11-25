//! Settings as statements and as options (digits, roots, terms, verbose), the steps that
//! verbose tells, programs read from the standard input, and the limit on memory.

use std::io::Write;
use std::process::{Command, Stdio};

fn program() -> Command {
    Command::new(env!("CARGO_BIN_EXE_symbolic-math"))
}

/// What a run prints to the standard output and to the standard error, and its exit code.
fn run_with(
    args: &[&str],
    input: Option<&str>,
    env: &[(&str, &str)],
) -> (String, String, Option<i32>) {
    let mut command = program();
    command
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in env {
        command.env(name, value);
    }
    let mut child = command.spawn().expect("symbolic-math runs");
    if let Some(text) = input {
        child
            .stdin
            .take()
            .expect("a pipe")
            .write_all(text.as_bytes())
            .expect("the lines are written");
    }
    let out = child.wait_with_output().expect("symbolic-math ends");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

fn run(args: &[&str]) -> (String, String, Option<i32>) {
    run_with(args, None, &[])
}

/// The lines of the standard output of a run that succeeds without a word of complaint.
fn lines(args: &[&str]) -> Vec<String> {
    let (out, err, code) = run(args);
    assert_eq!((err.as_str(), code), ("", Some(0)), "{args:?}: {out}");
    out.lines().map(str::to_string).collect()
}

#[test]
fn settings_hold_for_the_lines_of_a_session() {
    let session = "digits = 40\npi\nroots = 2\ncos(x) = x\ndigits = default\npi\nsettings\n\
                   terms = 50\nsum(1/(k^2 + 1), k, 1, 100)\nverbose\nx - 3 = 0\nverbose = off\n\
                   x - 4 = 0\n";
    let (out, err, code) = run_with(&[], Some(session), &[]);
    assert_eq!((err.as_str(), code), ("", Some(0)));
    assert_eq!(
        out,
        "digits = 40\n\
         pi\n  ~ 3.141592653589793238462643383279502884197\n\
         roots = 2\n\
         x ~ 0.7390851332151606416553120876738734040134\n\
         x ~ -2.486885698908560230696957034350657979423 - 1.8093613412957033190162761001940113256i\n\
         x ~ -2.486885698908560230696957034350657979423 + 1.8093613412957033190162761001940113256i\n\
         note: numeric search (roots = 2): the 3 roots nearest to 0 that were found are listed, there may be others\n\
         digits = default (32, or as many as the longest number of an input)\n\
         pi\n  ~ 3.1415926535897932384626433832795\n\
         digits = default (32, or as many as the longest number of an input)\n\
         roots = 2\n\
         terms = default (each sum and each series has its own limit)\n\
         verbose = off\n\
         terms = 50\n\
         sum(1/(k^2 + 1), k, 1, 100)\n\
         verbose = on\n\
         step: the input is read as x - 3 = 0\n\
         step: solve for x: x - 3 = 0\n\
         step:   x - 3 = 0: a polynomial of degree 1 in x\n\
         step:     degree 1: x = 3\n\
         step:   1 solution holds in the equation as it was given\n\
         x = 3\n\
         verbose = off\n\
         x = 4\n"
    );
}

/// `terms = 10^6` once printed `terms = 1000000`, the equation that it was read as, and set
/// nothing; so did `prec = n` with a number for n.
#[test]
fn the_value_of_a_setting_may_be_a_formula() {
    let session = "n := 40\nprec = n\nN(pi)\nterms = 10^6\nroots = 2*2\nsettings\nterms = -5\n\
                   verbose = maybe\ndigits = many\nprec = x + 1\nterms = 2^5, prec = 6*2, N(pi)\n\
                   settings\n";
    let (out, err, code) = run_with(&[], Some(session), &[]);
    assert_eq!(
        out,
        "40\n\
         digits = 40\n\
         3.141592653589793238462643383279502884197\n\
         terms = 1000000\n\
         roots = 4\n\
         digits = 40\n\
         roots = 4\n\
         terms = 1000000\n\
         verbose = off\n\
         x = prec - 1\n\
         3.14159265359\n\
         digits = 12\n\
         roots = 4\n\
         terms = 32\n\
         verbose = off\n"
    );
    // what is no whole number is told as that, and the setting stays as it was
    assert_eq!(
        err,
        "error: terms = N: a whole number from 1 to 1000000000000, or default\n\
         error: verbose = 0, 1 or 2 (off, on, all): no steps, the steps, every detail\n\
         error: digits = N: a whole number from 1 to 1000000, or default\n"
    );
    assert_eq!(code, Some(1));
}

#[test]
fn a_program_is_read_from_the_standard_input_line_by_line() {
    let program = "# the numbers of Fibonacci, three ways\n\
                   fb(n) := if(n < 2, n, fb(n - 1) + fb(n - 2))\n\
                   fb(15)\n\
                   a := 0; b := 1\n\
                   do(block(c := a + b, a := b, b := c), [k, 1, 14])\n\
                   b\n\
                   prec = 50\n\
                   phi^15/sqrt(5)\n\
                   \n\
                   # a loop with a condition\n\
                   n := 27; steps := 0\n\
                   while(n != 1, block(n := if(n mod 2 == 0, n/2, 3n + 1), steps := steps + 1))\n\
                   steps\n\
                   1 +\n\
                   ans + 1\n\
                   quit\n\
                   1/0\n";
    let (out, err, code) = run_with(&[], Some(program), &[]);
    assert_eq!(
        out,
        "fb(n) := if(n < 2, n, fb(n - 1) + fb(n - 2))\n610\n0\n1\n610\n610\ndigits = 50\n\
         305 + 682sqrt(5)/5\n  ~ 609.99967213097131459021128841494607851410034425101\n\
         27\n0\n111\n111\n112\n"
    );
    // a line that fails does not end the program, but the exit code tells of it
    assert!(err.starts_with("error: an operand is missing"), "{err}");
    assert_eq!(err.lines().count(), 1, "{err}");
    assert_eq!(code, Some(1));
}

#[test]
fn options_set_what_statements_set() {
    for (options, input, statements) in [
        (&["--roots", "3"][..], "sin(x) = x", "roots = 3; sin(x) = x"),
        (&["--roots=3"][..], "sin(x) = x", "roots = 3; sin(x) = x"),
        (&["--prec", "40"][..], "pi", "prec = 40; pi"),
        (&["--precision", "40"][..], "pi", "digits = 40; pi"),
        (&["--precision=40"][..], "pi", "precision 40, pi"),
        (&["--digits=40"][..], "pi", "prec=40 pi"),
        (&["-d", "40"][..], "sqrt(2)", "digits 40 sqrt(2)"),
        (
            &["--terms", "50"][..],
            "sum(1/(k^2 + 1), k, 1, 100)",
            "terms = 50; sum(1/(k^2 + 1), k, 1, 100)",
        ),
        (
            &["--terms=20000", "--prec=40"][..],
            "sum(0.99^n/n^2, n, 1, inf)",
            "terms = 20000, prec = 40, sum(0.99^n/n^2, n, 1, inf)",
        ),
        (&["-v"][..], "x^2 - 5x + 6 = 0", "verbose; x^2 - 5x + 6 = 0"),
        (
            &["--verbose"][..],
            "(x^2 - 1)/(x - 1)",
            "verbose = 1; (x^2 - 1)/(x - 1)",
        ),
        (&["--debug"][..], "N(sqrt(2), 50)", "debug; N(sqrt(2), 50)"),
        (&["--debug"][..], "cos(x) = x", "verbose = 2; cos(x) = x"),
        (
            &["-r", "--roots", "4"][..],
            "x*sin(x) = 1",
            "domain(real); roots = 4; x*sin(x) = 1",
        ),
    ] {
        let mut with_options = options.to_vec();
        with_options.push(input);
        let by_options = lines(&with_options);
        let mut by_statements = lines(&[statements]);
        // (domain(real) answers with a line of its own)
        if statements.starts_with("domain(") {
            by_statements.remove(0);
        }
        assert!(!by_options.is_empty(), "{options:?} {input}");
        assert_eq!(by_options, by_statements, "{options:?} {input}");
    }
    // what an option sets a statement may set again
    assert_eq!(
        lines(&["--roots", "9", "roots = 1; cos(x) = x"]),
        [
            "x ~ 0.73908513321516064165531208767387",
            "note: numeric search (roots = 1): the root nearest to 0 that was found is listed, there may be others"
        ]
    );
    assert_eq!(lines(&["-v", "verbose = off; 1 + 1"]), ["2"]);
    assert_eq!(
        lines(&["-d", "50", "digits = 12; pi"]),
        ["pi", "  ~ 3.14159265359"]
    );
}

#[test]
fn an_option_without_its_number_ends_the_run() {
    for (options, message) in [
        (
            &["--roots", "0", "pi"][..],
            "error: --roots needs a whole number from 1 to 100000\n",
        ),
        (
            &["--roots"][..],
            "error: --roots needs a whole number from 1 to 100000\n",
        ),
        (
            &["--roots=many", "pi"][..],
            "error: --roots needs a whole number from 1 to 100000\n",
        ),
        (
            &["--roots", "100001", "pi"][..],
            "error: --roots needs a whole number from 1 to 100000\n",
        ),
        (
            &["--terms", "x", "pi"][..],
            "error: --terms needs a whole number from 1 to 1000000000000\n",
        ),
        (
            &["--terms=0", "pi"][..],
            "error: --terms needs a whole number from 1 to 1000000000000\n",
        ),
        (
            &["--prec", "0", "pi"][..],
            "error: --digits needs a whole number from 1 to 1000000\n",
        ),
        (
            &["--precision", "pi"][..],
            "error: --digits needs a whole number from 1 to 1000000\n",
        ),
        (
            &["--prec=1000001", "pi"][..],
            "error: --digits needs a whole number from 1 to 1000000\n",
        ),
    ] {
        let (out, err, code) = run(options);
        assert_eq!(
            (out.as_str(), err.as_str(), code),
            ("", message, Some(2)),
            "{options:?}"
        );
    }
}

/// `--verbose` and `--debug` take a value as the statement `verbose = ...` does; the options
/// that only switch something on take none (and `--real=no` once switched the real numbers
/// on).
#[test]
fn an_option_with_a_value_means_that_value() {
    let input = "N(sqrt(2), 50)";
    let plain = lines(&[input]);
    let steps = lines(&["-v", input]);
    let all = lines(&["--debug", input]);
    assert!(plain.len() < steps.len() && steps.len() < all.len());
    for (option, want) in [
        ("--verbose=off", &plain),
        ("--verbose=no", &plain),
        ("--verbose=0", &plain),
        ("--verbose=none", &plain),
        ("--debug=off", &plain),
        ("--debug=0", &plain),
        ("--verbose=on", &steps),
        ("--verbose=yes", &steps),
        ("--verbose=1", &steps),
        ("--debug=1", &steps),
        ("--verbose=all", &all),
        ("--verbose=2", &all),
        ("--debug=on", &all),
        ("--debug=all", &all),
        ("--debug=2", &all),
    ] {
        assert_eq!(&lines(&[option, input]), want, "{option}");
    }
    // (the last one holds)
    assert_eq!(lines(&["-v", "--verbose=off", input]), plain);
    assert_eq!(lines(&["--verbose=off", "--debug", input]), all);
    for (option, message) in [
        (
            "--verbose=maybe",
            "error: --verbose=maybe: the value is 0, 1 or 2 (off, on, all): no steps, the steps, every detail\n",
        ),
        (
            "--debug=3",
            "error: --debug=3: the value is 0, 1 or 2 (off, on, all): no steps, the steps, every detail\n",
        ),
        ("--real=no", "error: --real takes no value\n"),
        ("--real=yes", "error: --real takes no value\n"),
        ("--exact=1", "error: --exact takes no value\n"),
        ("--numeric=off", "error: --numeric takes no value\n"),
        ("--quaternions=on", "error: --quaternions takes no value\n"),
        ("--help=all", "error: --help takes no value\n"),
        ("--version=1", "error: --version takes no value\n"),
        ("--functions=sin", "error: --functions takes no value\n"),
    ] {
        let (out, err, code) = run(&[option, "x^2 = -4"]);
        assert_eq!(
            (out.as_str(), err.as_str(), code),
            ("", message, Some(2)),
            "{option}"
        );
    }
}

#[test]
fn the_steps_are_printed_where_the_results_are_and_in_their_order() {
    let (out, err, code) = run(&["verbose; x - 1 = 0; 1/0; 2 + 2; verbose = off; 3 + 3"]);
    assert_eq!((err.as_str(), code), ("", Some(0)));
    assert_eq!(
        out,
        "step: the input is read as x - 1 = 0\n\
         step: solve for x: x - 1 = 0\n\
         step:   x - 1 = 0: a polynomial of degree 1 in x\n\
         step:     degree 1: x = 1\n\
         step:   1 solution holds in the equation as it was given\n\
         x = 1\n\
         step: the input is read as 1/0\n\
         ComplexInfinity\n\
         step: the input is read as 2 + 2\n\
         4\n\
         6\n"
    );
    // the steps of a statement that fails are told as far as they went; the error follows
    let (out, err, code) = run(&["verbose; limit(1/x, x, 0); 5"]);
    assert!(
        out.starts_with(
            "step: the input is read as limit(1/x, x, 0)\nstep: the limit of 1/x as x -> 0\n"
        ),
        "{out}"
    );
    assert!(out.ends_with("goes to -inf\n"), "{out}");
    assert_eq!(
        err,
        "error: the limit does not exist: from the left it is -inf, from the right inf\n"
    );
    assert_eq!(code, Some(1));
    // a statement tells at most 4000 steps
    let (out, err, code) = run(&["debug; s := 0; do(s := s + k, [k, 1, 3000]); verbose = off; s"]);
    assert_eq!((err.as_str(), code), ("", Some(0)));
    let told: Vec<&str> = out.lines().filter(|l| l.starts_with("step: ")).collect();
    assert_eq!(told.last(), Some(&"step: (more steps are not shown)"));
    assert!(told.len() < 4100, "{} steps", told.len());
    assert_eq!(out.lines().last(), Some("4501500"));
    // a reader that stops early is no failure
    let mut child = program()
        .args(["debug; cos(x) = x"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("symbolic-math runs");
    drop(child.stdout.take());
    let ended = child.wait_with_output().expect("symbolic-math ends");
    assert!(ended.status.success() && ended.stderr.is_empty());
}

#[test]
fn each_run_of_tet_is_told() {
    let (out, err, _) = run(&["verbose; 2^^0.5"]);
    if err.contains("could not run tet") || out.contains("could not run tet") {
        // (the program tet may be missing where the tests run)
        return;
    }
    assert_eq!(
        out,
        "step: the input is read as 2^^(1/2)\n\
         step: the program tet is run: tet -q 36 2 0 0.5 0 (base, height, each as its real and \
         its imaginary part, for 36 digits)\n\
         step:   tet gives 1.45878181604\n\
         1.4587818160364217006839716610386\n"
    );
}

#[test]
fn a_function_may_call_itself_thousands_of_times() {
    assert_eq!(
        lines(&["f(n) := if(n <= 1, 1, n*f(n - 1)); f(3000) == 3000!"]),
        ["f(n) := if(n <= 1, 1, n*f(n - 1))", "true"]
    );
    assert_eq!(
        lines(&["-e", "s(n) := if(n == 0, 0, n + s(n - 1)); s(5000)"]),
        ["s(n) := if(n = 0, 0, n + s(n - 1))", "12502500"]
    );
    // one that never ends is an error, not a crash
    for endless in [
        "f(x) := f(x) + 1; f(1)",
        "loop(n) := loop(n + 1); loop(0)",
        "a(n) := b(n); b(n) := a(n) + 1; a(1)",
    ] {
        let (out, err, code) = run(&["-e", endless]);
        assert!(
            err.ends_with("calls itself without end\n") && code == Some(1),
            "{endless}: {out} {err} {code:?}"
        );
    }
}

/// `SYMBOLIC_MATH_MEMORY` is a limit on the address space, which Linux and FreeBSD keep.
#[test]
fn the_memory_of_a_run_can_be_limited() {
    for wrong in ["abc", "32", "0", "-5", "64.5", "", "unlimited 5", "all"] {
        let (out, err, code) = run_with(&["2 + 2"], None, &[("SYMBOLIC_MATH_MEMORY", wrong)]);
        assert_eq!(
            (out.as_str(), err.as_str(), code),
            (
                "",
                "error: SYMBOLIC_MATH_MEMORY needs megabytes: a whole number from 64 on, or unlimited\n",
                Some(2)
            ),
            "{wrong}"
        );
    }
    // no limit at all, on every system
    for free in ["unlimited", "none", " unlimited "] {
        assert_eq!(
            run_with(&["2 + 2"], None, &[("SYMBOLIC_MATH_MEMORY", free)]),
            ("4\n".into(), String::new(), Some(0)),
            "{free}"
        );
    }
    if !cfg!(any(target_os = "linux", target_os = "freebsd")) {
        return;
    }
    let limited = |megabytes: &str, input: &str| {
        run_with(&[input], None, &[("SYMBOLIC_MATH_MEMORY", megabytes)])
    };
    assert_eq!(
        limited("1024", "2 + 2"),
        ("4\n".into(), String::new(), Some(0))
    );
    // (with so little the thread with the long stack cannot be made: the program works without)
    assert_eq!(
        limited("64", "2 + 2"),
        ("4\n".into(), String::new(), Some(0))
    );
    assert_eq!(
        limited("64", "f(n) := if(n <= 1, 1, n*f(n - 1)); f(30) == 30!").0,
        "f(n) := if(n <= 1, 1, n*f(n - 1))\ntrue\n"
    );
    // the inverse sine of this point once took all the memory of the machine
    assert_eq!(
        limited("512", "N(asin(-1.25+0.75i))"),
        (
            "-0.91173829096848763635848956431673 + 1.0317185344477802733636405863101i\n".into(),
            String::new(),
            Some(0)
        )
    );
    // a calculation that needs more than it may take ends as an error like any other, with
    // no result: wherever the memory ends (in a number of GMP that grows, in a list, in the
    // terms of a sum), the program says so itself, and no core is written
    let told = "error: out of memory: the calculation needs more than the 200 megabytes that the program may take (SYMBOLIC_MATH_MEMORY=N sets another limit, SYMBOLIC_MATH_MEMORY=unlimited none)\n";
    for needs_more in [
        "length(string(expand((x + y + z + 1)^120)))",
        "k := 2; while(k > 0, k := k*k)",
        "a := [1]; do(a := join(a, a), [k, 1, 40]); length(a)",
    ] {
        assert_eq!(
            limited("200", needs_more),
            (String::new(), told.to_string(), Some(1)),
            "{needs_more}"
        );
    }
}

/// Without `SYMBOLIC_MATH_MEMORY` the program takes 8192 megabytes at most (less on a small
/// machine), and never more than a limit that it was started with: a calculation without end
/// stops with an error and leaves the machine its memory.
#[test]
fn a_run_has_a_limit_of_memory_of_its_own() {
    if !cfg!(any(target_os = "linux", target_os = "freebsd")) {
        return;
    }
    // (under a limit of the shell that is lower than the usual one: that one is told)
    let script = format!(
        "ulimit -v 307200; exec {} 'k := 2; while(k > 0, k := k*k)'",
        env!("CARGO_BIN_EXE_symbolic-math")
    );
    let out = Command::new("sh")
        .args(["-c", &script])
        .env_remove("SYMBOLIC_MATH_MEMORY")
        .stdin(Stdio::null())
        .output()
        .expect("sh runs");
    assert_eq!(
        (
            String::from_utf8_lossy(&out.stdout).as_ref(),
            String::from_utf8_lossy(&out.stderr).as_ref(),
            out.status.code()
        ),
        (
            "",
            "error: out of memory: the calculation needs more than the 300 megabytes that the program may take (SYMBOLIC_MATH_MEMORY=N sets another limit, SYMBOLIC_MATH_MEMORY=unlimited none)\n",
            Some(1)
        )
    );
    // `unlimited` does not lift a limit of the shell
    let script = format!(
        "ulimit -v 307200; SYMBOLIC_MATH_MEMORY=unlimited exec {} 'a := [1]; do(a := join(a, a), [k, 1, 40]); length(a)'",
        env!("CARGO_BIN_EXE_symbolic-math")
    );
    let out = Command::new("sh")
        .args(["-c", &script])
        .stdin(Stdio::null())
        .output()
        .expect("sh runs");
    assert_eq!(
        (
            String::from_utf8_lossy(&out.stdout).as_ref(),
            String::from_utf8_lossy(&out.stderr).as_ref(),
            out.status.code()
        ),
        (
            "",
            "error: out of memory: the calculation needs more than the 300 megabytes that the program may take (SYMBOLIC_MATH_MEMORY=N sets another limit, SYMBOLIC_MATH_MEMORY=unlimited none)\n",
            Some(1)
        )
    );
}
