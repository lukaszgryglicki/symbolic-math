//! What a result is does not depend on how many digits are asked for: the numbers that the
//! program prints with its usual 32 digits are the beginnings of the ones that it prints with
//! 50. (A pivot that was a hair off 1, an iteration that stopped at the digits of its misfit
//! and a halving that stopped at the digits of a distribution function gave other numbers.)

use rug::Float;
use std::process::{Command, Stdio};

/// The standard output of a run that succeeds without a word of complaint.
fn printed(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_symbolic-math"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("symbolic-math runs");
    let (text, complaint) = (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    );
    assert_eq!(
        (complaint.as_str(), out.status.code()),
        ("", Some(0)),
        "{args:?}: {text}"
    );
    text
}

/// The numbers of a text, in their order: digits with a point and an exponent, where no name
/// goes on (the 1 of t1 is no number).
fn numbers(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let before = at.checked_sub(1).map(|k| bytes[k]);
        let begins = bytes[at].is_ascii_digit()
            && !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'.');
        if !begins {
            at += 1;
            continue;
        }
        let from = at;
        while at < bytes.len() && (bytes[at].is_ascii_digit() || bytes[at] == b'.') {
            at += 1;
        }
        if at < bytes.len() && bytes[at] == b'e' {
            let mut end = at + 1;
            if end < bytes.len() && bytes[end] == b'-' {
                end += 1;
            }
            if end < bytes.len() && bytes[end].is_ascii_digit() {
                while end < bytes.len() && bytes[end].is_ascii_digit() {
                    end += 1;
                }
                at = end;
            }
        }
        found.push(text[from..at].to_string());
    }
    found
}

fn value(number: &str) -> Float {
    Float::with_val(
        512,
        Float::parse(number).unwrap_or_else(|_| panic!("{number} is no number")),
    )
}

/// The numbers of `input` with the usual digits and with 50: as many, and each the same to 30
/// digits.
fn same_with_more_digits(input: &str) {
    let (usual, more) = (printed(&[input]), printed(&["-d", "50", input]));
    let (few, many) = (numbers(&usual), numbers(&more));
    assert_eq!(few.len(), many.len(), "{input}:\n{usual}\n{more}");
    assert!(!few.is_empty(), "{input}: {usual}");
    let least = Float::with_val(512, Float::i_pow_u(10, 30)).recip();
    for (short, long) in few.iter().zip(&many) {
        let (a, b) = (value(short), value(long));
        let gap = Float::with_val(512, &a - &b).abs();
        assert!(
            gap <= Float::with_val(512, b.abs_ref()) * &least,
            "{input}: {short} with 32 digits, {long} with 50"
        );
    }
}

#[test]
fn factors_without_signs_have_the_digits_that_are_asked_for() {
    // (the rounds ended when the misfit stood still: the factors had 23 right digits then)
    for input in [
        "nnmf([[5, 1, 2], [1, 4, 1], [2, 1, 3]], 2)",
        "nnmf([[1, 2, 3, 4], [2, 4, 6, 9], [3, 6, 10, 12], [5, 1, 1, 1], [1, 1, 5, 2]], 2)",
        "nnmf([[4, 0, 1, 2, 0], [0, 3, 0, 1, 2], [1, 0, 5, 0, 1], [2, 1, 0, 6, 0]], 3)",
        "nnmf([[1, 2], [3, 4], [5, 6]], 2)",
        "nnmf(magic(4), 2)",
    ] {
        same_with_more_digits(input);
    }
}

#[test]
fn a_system_of_rounded_numbers_has_one_solution_with_any_digits() {
    // (a pivot over itself was a hair off 1 with 40 and with 50 digits: the solution of the
    // equation of Lyapunov had a free t1 then, and gram and hsvd failed)
    let balanced = "part(balreal(tf([1], [1, 3, 2])), 1)";
    for input in [
        format!("gram({balanced}, \"c\")"),
        format!("gram({balanced}, \"o\")"),
        format!("hsvd({balanced})"),
        format!("d := ssdata({balanced}); lyap(d[[1]], d[[2]]*transpose(d[[2]]))"),
        "hsvd(part(balreal(tf([1, 2], [1, 6, 11, 6])), 1))".to_string(),
        "inv(N([[10^-40, 1], [3, 1]]))".to_string(),
        "linsolve(N([[10^-60, 1/3, 2], [1/7, 1/9, 5], [1/13, 3, 1/11]]), [1, 2, 3])".to_string(),
        "det(N([[10^-40, 1, 2, 1], [3, 1, 1, 2], [1, 5, 1/7, 3], [2, 2, 1, 1/3]]))".to_string(),
    ] {
        same_with_more_digits(&input);
    }
    for digits in ["33", "36", "40", "44", "50", "57", "64", "80"] {
        let input = format!("d := ssdata({balanced}); lyap(d[[1]], d[[2]]*transpose(d[[2]]))");
        let text = printed(&["-d", digits, &input]);
        let solution = text.lines().last().unwrap_or_default();
        assert!(
            !solution.contains('t') && solution.matches(", 0]").count() == 1,
            "-d {digits}: {solution}"
        );
    }
}

#[test]
fn a_polynomial_of_rounded_roots_has_no_rounding_for_coefficients() {
    for input in [
        "poly(buttap(2)[[2]])",
        "poly(buttap(5)[[2]])",
        "poly(buttap(8)[[2]])",
        "zp2ss(buttap(2))",
        "poly(cheb1ap(4, 1)[[2]])",
        "poly(cheb2ap(5, 40)[[2]])",
        "poly(ellipap(4, 1, 40)[[2]])",
        "poly(besselap(5)[[2]])",
        "poly(roots([1, 2, 3, 4, 5, 6]))",
    ] {
        same_with_more_digits(input);
        for digits in ["32", "40", "50", "64"] {
            let text = printed(&["-d", digits, input]);
            assert!(!text.contains('i'), "-d {digits} {input}: {text}");
        }
    }
    assert_eq!(
        printed(&["poly(roots([1, 0, 0, 0, 0, 0, 3, 1]))"]).trim(),
        "[1, 0, 0, 0, 0, 0, 3, 1]"
    );
}

#[test]
fn a_place_of_a_distribution_has_the_digits_that_are_asked_for() {
    // (far out in a tail the distribution function has fewer digits than the place needs)
    for input in [
        "tinv(1 - 10^-30, 7)",
        "tinv(10^-30, 7)",
        "tinv(1 - 10^-20, 7)",
        "tinv(0.50000000000000001, 5)",
        "chi2inv(1 - 10^-25, 3)",
        "finv(1 - 10^-20, 3, 4)",
        "betainv(1 - 10^-20, 2, 3)",
        "tinv(0.975, 10)",
        "chi2inv(0.95, 4)",
        "gaminv(0.5, 2)",
    ] {
        same_with_more_digits(input);
    }
    // the two tails of an even distribution are the same place with the other sign
    let (upper, lower) = (
        printed(&["tinv(1 - 10^-30, 7)"]),
        printed(&["tinv(10^-30, 7)"]),
    );
    assert_eq!(format!("-{}", upper.trim()), lower.trim());
    for digits in ["32", "40", "50"] {
        assert_eq!(printed(&["-d", digits, "tinv(0.5, 7)"]).trim(), "0");
        assert_eq!(printed(&["-d", digits, "bohmanwin(2)"]).trim(), "[0, 0]");
    }
}

#[test]
fn a_search_and_a_path_have_the_digits_that_are_asked_for() {
    for input in [
        // the frequency of the largest gain (a search that compared gains told it to half
        // its digits)
        "hinfnorm(ss([[-5/2, 2], [0, -3]], [[1], [1]], [[-2, -1], [-1, 1]], [[1], [0]]))",
        // paths over a kink of the slopes (a step as short as these need was taken for a
        // singular point from 40 digits on)
        "bvp4c(@(x, y, z) [z, -abs(y)], @(ya, za, yb, zb) [ya, yb + 2], [0, 4], [-1, 0])",
        "ode45(@(t, y) abs(y - 1) + 1/10, [0, 30], [0])[[-1]]",
        "ode45(@(t, x, v) [v, -abs(x)], [0, 3], [1, 0])[[-1]]",
    ] {
        same_with_more_digits(input);
    }
    assert_eq!(
        printed(&["-d", "64", "ode45(@(t, y) abs(t - 1), [0, 2], [0])[[-1]]"]).trim(),
        "[2, 1]"
    );
}

#[test]
fn a_list_that_falls_away_keeps_its_small_values() {
    // (everything below 10^-40 of the largest value of a list was shown as 0)
    let answer = printed(&["-e", "impz([1], [1, -1/1000], 20)"]);
    assert!(answer.trim().ends_with("1e-51, 1e-54, 1e-57]"), "{answer}");
    let window = printed(&["gausswin(5, 30)"]);
    assert!(
        window.starts_with("[3.69388306848725621879342752457") && !window.contains("[0,"),
        "{window}"
    );
    assert_eq!(
        printed(&["-e", "freqs([1], [1, 1], [1, 10^40])"]).trim(),
        "[0.5 - 0.5i, 1e-80 - 1e-40i]"
    );
    // the delay at a zero of the response: what both sides of it have, not the rounding of
    // 0 over the rounding of 0 (which made the 1.5 of the other places a 0 as well)
    for digits in ["32", "40", "50"] {
        let delays = printed(&["-d", digits, "-e", "grpdelay([1, 0, 0, 1], [1], 6)[[1]]"]);
        assert_eq!(
            delays.trim(),
            "[1.5, 1.5, 1.5, 1.5, 1.5, 1.5]",
            "-d {digits}"
        );
    }
}

#[test]
fn a_trained_network_has_the_digits_that_are_asked_for() {
    // (the slope that ended a training left the weights with 21 of their 32 digits. Only
    // trainings with one least loss are asked: a bias that may run away along a flat valley
    // has no digits to speak of)
    for input in [
        "trainnet(feedforwardnet([2, 1]), [[-0.762, -0.717], [1, 0.63], [0.51, -0.624], [-0.111, -0.072]], \
         [[0.898], [0.832], [-0.259], [-0.27]])",
        "trainnet(feedforwardnet([1, 1]), [1, 2, 3, 4], [3, 5, 7, 9], \"l2\", 0.1)",
        "n := trainnet(patternnet([2, 3, 2]), [[0, 0], [0, 1], [1, 0], [1, 1]], [[1, 0], [0, 1], [0, 1], [1, 0]]); \
         predict(n, [[0, 0], [0, 1], [1, 0], [1, 1]])",
        "n := trainnet(patternnet([2, 3], \"tanh\", 5), [[0.037, -0.754], [-0.633, 0.908], [0.559, -0.562], \
         [0.107, -0.829], [-0.503, -0.433], [0.743, 0.728]], [[1, 0, 0], [0, 1, 0], [0, 0, 1], [1, 0, 0], \
         [0, 1, 0], [0, 0, 1]], \"l2\", 0.05); n",
    ] {
        same_with_more_digits(input);
    }
}
