//! Iterates of functions for counts of steps that are not whole: runs the program on fixed
//! inputs and compares what it prints. The numbers were checked first: each against the
//! coordinate that makes its map linear (or a power) at the fixed point, computed with other
//! software from the definition, and the tables of sums against each other.

use std::process::{Command, Stdio};

fn program() -> Command {
    Command::new(env!("CARGO_BIN_EXE_symbolic-math"))
}

/// What the program prints for an input, with `|` for line ends: `-x | input` runs it with
/// the option `-x`.
fn answer(options: &[&str], input: &str) -> String {
    let mut args = options.to_vec();
    let mut text = input;
    if input.starts_with('-')
        && let Some((head, tail)) = input.split_once(" | ")
    {
        args.extend(head.split(' '));
        text = tail;
    }
    args.push(text);
    let out = program()
        .args(&args)
        .stdin(Stdio::null())
        .output()
        .expect("symbolic-math runs");
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).trim_end().replace('\n', "|");
    [text(&out.stdout), text(&out.stderr)]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("|")
}

fn check_with(options: &[&str], cases: &[(&str, &str)]) {
    let mut failures = Vec::new();
    for (input, want) in cases {
        let got = answer(options, input);
        if got != *want {
            failures.push(format!("{input}\n  want: {want}\n  got:  {got}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

fn check(cases: &[(&str, &str)]) {
    check_with(&[], cases);
}

const CLOSED_LINEAR_AND_POWERS: &[(&str, &str)] = &[
    (r"iterate(a*x+b, x, t)", r"b*(a^t - 1)/(a - 1) + x*a^t"),
    (r"iterate(2x+1, x, t)", r"x*2^t + 2^t - 1"),
    (r"iterate(2x+1, x, 1/3)", r"2^(1/3)*x - 1 + 2^(1/3)"),
    (r"iterate(2x+1, x, -1/2)", r"sqrt(2)*x/2 - 1 + sqrt(2)/2"),
    (r"iterate(2x+1, x, i)", r"2^i*x - 1 + 2^i"),
    (r"iterate(2x+1, x, 0)", r"x"),
    (r"iterate(2x+1, x, 1)", r"2x + 1"),
    (r"iterate(2x+1, x, 5)", r"32x + 31"),
    (r"iterate(2x+1, x, -3)", r"x/8 - 7/8"),
    (r"iterate(x+3, x, 1/2)", r"x + 3/2"),
    (r"iterate(x+a, x, t)", r"a*t + x"),
    (r"iterate(x-1, x, -5/2)", r"x + 5/2"),
    (r"iterate(-x/3+1, x, t)", r"x*(-1/3)^t - 3(-1/3)^t/4 + 3/4"),
    (
        r"iterate(-x/3+1, x, 1/2)",
        r"i*sqrt(3)*x/3 + 3/4 - i*sqrt(3)/4",
    ),
    (r"iterate(-x, x, 1/2)", r"i*x"),
    (r"iterate(-x, x, 2)", r"x"),
    (r"iterate(1-x, x, 1/2)", r"i*x + 1/2 - i/2"),
    (r"iterate(1-x, x, 2)", r"x"),
    (
        r"iterate((1+i)*x+2, x, 1/2)",
        r"sqrt(1 + i)*x + 2i - 2i*sqrt(1 + i)",
    ),
    (r"iterate(i*x, x, 1/2)", r"(-1)^(1/4)*x"),
    (r"iterate(i*x, x, 4)", r"x"),
    (r"iterate(x, x, 1/2)", r"x"),
    (r"iterate(x/2, x, t)", r"x*2^(-t)"),
    (r"iterate(c*x^p, x, t)", r"c^((p^t - 1)/(p - 1))*x^(p^t)"),
    (r"iterate(x^2, x, 1/3)", r"x^(2^(1/3))"),
    (r"iterate(x^2, x, -1)", r"sqrt(x)"),
    (r"iterate(x^2, x, -1/2)", r"x^(sqrt(2)/2)"),
    (r"iterate(x^2, x, i)", r"x^(2^i)"),
    (r"iterate(3x^2, x, t)", r"3^(2^t)*x^(2^t)/3"),
    (r"iterate(x^3/2, x, t)", r"2^(1/2 - 3^t/2)*x^(3^t)"),
    (
        r"iterate(x^3/2, x, 1/2)",
        r"2^(1/2 - sqrt(3)/2)*x^(sqrt(3))",
    ),
    (r"iterate(sqrt(x), x, t)", r"x^(2^(-t))"),
    (r"iterate(sqrt(x), x, 1/2)", r"x^(sqrt(2)/2)"),
    (r"iterate(x^(1/3), x, -2)", r"x^9"),
    (r"iterate(1/x, x, t)", r"x^((-1)^t)"),
    (r"iterate(1/x, x, 2)", r"x"),
    (r"iterate(1/x, x, 1/2)", r"x^i"),
    (r"iterate(2/x, x, 1/2)", r"2^(1/2 - i/2)*x^i"),
    (r"iterate(x^(-2), x, 1/2)", r"x^(i*sqrt(2))"),
    (r"iterate(x^12, x, 1/2)", r"x^(2sqrt(3))"),
    (r"iterate(x^pi, x, t)", r"x^(pi^t)"),
    (r"iterate(5x^7, x, -1)", r"5^(6/7)*x^(1/7)/5"),
];

const CLOSED_MOEBIUS: &[(&str, &str)] = &[
    (r"iterate(1/(1+x), x, 2)", r"(x + 1)/(x + 2)"),
    (r"iterate(1/(1+x), x, 3)", r"(x + 2)/(2x + 3)"),
    (r"iterate(1/(1+x), x, -1)", r"1/x - 1"),
    (r"iterate(1/(1+x), x, -2)", r"1/(1/x - 1) - 1"),
    (r"iterate(x/(1+x), x, t)", r"x/(t*x + 1)"),
    (r"iterate(x/(1+x), x, 1/3)", r"3x/(x + 3)"),
    (r"iterate(x/(1+x), x, -1)", r"x/(1 - x)"),
    (r"iterate(x/(1+x), x, i)", r"x/(i*x + 1)"),
    (r"iterate(x/(2+x), x, t)", r"x/(-x + x*2^t + 2^t)"),
    (r"iterate(x/(2+x), x, 1/2)", r"x/(sqrt(2)*x - x + sqrt(2))"),
    (r"iterate(x/(1-x), x, t)", r"-x/(t*x - 1)"),
    (r"iterate(x/(1-x), x, 1/2)", r"-2x/(x - 2)"),
    (r"iterate((2x+1)/(x+3), x, 2)", r"(x + 1)/(x + 2)"),
    (r"iterate((2x+1)/(x+3), x, -1)", r"(1 - 3x)/(x - 2)"),
    (
        r"iterate((3x-1)/(x+1), x, t)",
        r"(t*x - t + 2x)/(t*x - t + 2)",
    ),
    (r"iterate((3x-1)/(x+1), x, 1/2)", r"(5x - 1)/(x + 3)"),
    (r"iterate((x-1)/(x+1), x, 2)", r"-1/x"),
    (r"iterate((x-1)/(x+1), x, 4)", r"x"),
    (r"iterate(1/(1-x), x, 3)", r"x"),
    (r"iterate(1/(1-x), x, 2)", r"1 - 1/x"),
    (r"iterate((x+1)/(x-1), x, 2)", r"x"),
    (r"iterate((2x)/(x+2), x, t)", r"2x/(t*x + 2)"),
    (r"iterate(2x/(1+x), x, t)", r"x*2^t/(-x + x*2^t + 1)"),
    (r"iterate((a*x)/(x+a), x, t)", r"a*x/(t*x + a)"),
];

const CLOSED_QUADRATICS: &[(&str, &str)] = &[
    (r"iterate(x^2+2x, x, t)", r"(x + 1)^(2^t) - 1"),
    (r"iterate(x^2+2x, x, 1/3)", r"(x + 1)^(2^(1/3)) - 1"),
    (r"iterate(x^2+2x, x, -1)", r"sqrt(x + 1) - 1"),
    (r"iterate(x^2-2x+2, x, t)", r"(x - 1)^(2^t) + 1"),
    (r"iterate(x^2+4x+2, x, t)", r"(x + 2)^(2^t) - 2"),
    (r"iterate(x^2-6x+12, x, 1/2)", r"(x - 3)^(sqrt(2)) + 3"),
    (r"iterate(2x^2+4x+1, x, 1/2)", r"(2x + 2)^(sqrt(2))/2 - 1"),
    (r"iterate(3x^2+2x, x, 1/2)", r"(3x + 1)^(sqrt(2))/3 - 1/3"),
    (r"iterate(x^2-2, x, t)", r"2cosh(2^t*acosh(x/2))"),
    (r"iterate(x^2-2, x, 1/2)", r"2cosh(sqrt(2)*acosh(x/2))"),
    (r"iterate(x^2-2, x, -1)", r"sqrt(x + 2)"),
    (r"iterate(2x^2-1, x, t)", r"cosh(2^t*acosh(x))"),
    (r"iterate(2x^2-1, x, 1/3)", r"cosh(2^(1/3)*acosh(x))"),
    (
        r"iterate(4x*(1-x), x, t)",
        r"1/2 - cosh(2^t*acosh(1 - 2x))/2",
    ),
    (
        r"iterate(4x*(1-x), x, 1/2)",
        r"1/2 - cosh(sqrt(2)*acosh(1 - 2x))/2",
    ),
    (r"iterate(1-2x^2, x, 1/2)", r"-cosh(sqrt(2)*acosh(-x))"),
    (r"iterate(x^2+x-1/4, x, t)", r"(x + 1/2)^(2^t) - 1/2"),
    (r"iterate(x^2+x, x, 2)", r"x^2 + x + (x^2 + x)^2"),
    (r"iterate(x^2+c, x, 2)", r"c + (x^2 + c)^2"),
    (r"iterate(x^2+1, x, 3)", r"(x^4 + 2x^2 + 2)^2 + 1"),
    (r"iterate(x^2+1, x, -1)", r"sqrt(x - 1)"),
    (r"iterate(x^2+1, x, -2)", r"sqrt(sqrt(x - 1) - 1)"),
    (r"iterate(x^2+1, x, 0)", r"x"),
    (r"iterate(x^2+1, x, 1)", r"x^2 + 1"),
];

const CLOSED_HIGHER_DEGREES: &[(&str, &str)] = &[
    (r"iterate(x^3+3x, x, t)", r"2sinh(3^t*asinh(x/2))"),
    (r"iterate(x^3+3x, x, 1/2)", r"2sinh(sqrt(3)*asinh(x/2))"),
    (r"iterate(x^3-3x, x, t)", r"2cosh(3^t*acosh(x/2))"),
    (r"iterate(4x^3-3x, x, t)", r"cosh(3^t*acosh(x))"),
    (r"iterate(4x^3-3x, x, 1/2)", r"cosh(sqrt(3)*acosh(x))"),
    (r"iterate(4x^3+3x, x, t)", r"sinh(3^t*asinh(x))"),
    (r"iterate(x^3+3x^2+3x, x, t)", r"(x + 1)^(3^t) - 1"),
    (r"iterate(x^3+3x^2+3x, x, 1/2)", r"(x + 1)^(sqrt(3)) - 1"),
    (r"iterate(x^3-3x^2+3x, x, t)", r"(x - 1)^(3^t) + 1"),
    (r"iterate((x-2)^5+2, x, t)", r"(x - 2)^(5^t) + 2"),
    (
        r"iterate(2(x+1)^3-1, x, 1/2)",
        r"2^(1/2 + sqrt(3)/2)*(x + 1)^(sqrt(3))/2 - 1",
    ),
    (r"iterate(x^5+5x^3+5x, x, t)", r"2sinh(5^t*asinh(x/2))"),
    (r"iterate(16x^5-20x^3+5x, x, t)", r"sin(5^t*asin(x))"),
    (
        r"iterate(16x^5+20x^3+5x, x, 1/2)",
        r"sinh(sqrt(5)*asinh(x))",
    ),
    (r"iterate(8x^4-8x^2+1, x, t)", r"cosh(2^(2t)*acosh(x))"),
    (r"iterate(8x^4-8x^2+1, x, 1/2)", r"cosh(2acosh(x))"),
    (r"iterate(x^4+4x^3+6x^2+4x, x, t)", r"(x + 1)^(2^(2t)) - 1"),
    (r"iterate(x^4-4x^2+2, x, t)", r"2cosh(2^(2t)*acosh(x/2))"),
    (r"iterate(32x^6-48x^4+18x^2-1, x, t)", r"cosh(6^t*acosh(x))"),
    (
        r"iterate(64x^7-112x^5+56x^3-7x, x, 1/2)",
        r"cosh(sqrt(7)*acosh(x))",
    ),
    (
        r"iterate(x^7+7x^5+14x^3+7x, x, t)",
        r"2sinh(7^t*asinh(x/2))",
    ),
    (
        r"iterate(128x^8-256x^6+160x^4-32x^2+1, x, t)",
        r"cosh(2^(3t)*acosh(x))",
    ),
    (
        r"iterate(256x^9-576x^7+432x^5-120x^3+9x, x, t)",
        r"sin(3^(2t)*asin(x))",
    ),
    (
        r"iterate(512x^10-1280x^8+1120x^6-400x^4+50x^2-1, x, 1/2)",
        r"cosh(sqrt(10)*acosh(x))",
    ),
    (
        r"iterate(1024x^11-2816x^9+2816x^7-1232x^5+220x^3-11x, x, t)",
        r"cosh(11^t*acosh(x))",
    ),
    (
        r"iterate(2048x^12-6144x^10+6912x^8-3584x^6+840x^4-72x^2+1, x, t)",
        r"cosh(12^t*acosh(x))",
    ),
    (r"iterate((x+1)^12-1, x, 1/2)", r"(x + 1)^(2sqrt(3)) - 1"),
    (r"iterate(x^3+x, x, 2)", r"x^3 + x + (x^3 + x)^3"),
    (
        r"iterate(x^3+x, x, 1/2)",
        r"iterate(x^3 + x, x, 1/2)|note: no closed form for this iterate: at a number, iterate(f, x, t, x0) or f[t](x0), it is computed by regular iteration at a fixed point",
    ),
    (
        r"iterate(x^4+x, x, t)",
        r"iterate(x^4 + x, x, t)|note: no closed form for this iterate: at a number, iterate(f, x, t, x0) or f[t](x0), it is computed by regular iteration at a fixed point",
    ),
];

const CLOSED_EXPONENTIALS_AND_WHOLE_STEPS: &[(&str, &str)] = &[
    (r"iterate(e^x, x, t)", r"e^^(t + slog(e, x))"),
    (r"iterate(e^x, x, 1/2)", r"e^^(slog(e, x) + 1/2)"),
    (r"iterate(e^x, x, -1)", r"ln(x)"),
    (r"iterate(e^x, x, 3)", r"e^(e^(e^x))"),
    (r"iterate(e^x, x, -2)", r"ln(ln(x))"),
    (r"iterate(2^x, x, t)", r"2^^(t + slog(2, x))"),
    (r"iterate(10^x, x, t)", r"10^^(t + slog(10, x))"),
    (r"iterate(10^x, x, -1)", r"ln(x)/(ln(2) + ln(5))"),
    (
        r"iterate(sqrt(2)^x, x, 1/2)",
        r"(sqrt(2))^^(slog(sqrt(2), x) + 1/2)",
    ),
    (r"iterate(a^x, x, 1/2)", r"a^^(slog(a, x) + 1/2)"),
    (r"iterate(3*2^x, x, 1/2)", r"3*8^^(slog(8, x/3) + 1/2)"),
    (r"iterate(e^(2x), x, 1/2)", r"(e^2)^^(slog(e^2, x) + 1/2)"),
    (
        r"iterate(e^(x+1), x, 1/2)",
        r"e*(e^e)^^(slog(e^e, x/e) + 1/2)",
    ),
    (
        r"iterate(6^x/7, x, 1/2)",
        r"(6^(1/7))^^(slog(6^(1/7), 7x) + 1/2)/7",
    ),
    (
        r"iterate(2^(x/3), x, 1/2)",
        r"(2^(1/3))^^(slog(2^(1/3), x) + 1/2)",
    ),
    (r"iterate(ln(x), x, 1/2)", r"e^^(slog(e, x) - 1/2)"),
    (r"iterate(ln(x), x, t)", r"e^^(slog(e, x) - t)"),
    (r"iterate(ln(x), x, -2)", r"e^(e^x)"),
    (
        r"iterate(log(2, x), x, 1/2)",
        r"iterate(ln(2)/ln(x), x, 1/2)|note: no closed form for this iterate: at a number, iterate(f, x, t, x0) or f[t](x0), it is computed by regular iteration at a fixed point",
    ),
    (r"iterate(sin(x), x, 3)", r"sin(sin(sin(x)))"),
    (r"iterate(sin(x), x, -1)", r"asin(x)"),
    (r"iterate(cos(x), x, -2)", r"acos(acos(x))"),
    (r"iterate(tan(x), x, -1)", r"atan(x)"),
    (r"iterate(sinh(x), x, -1)", r"asinh(x)"),
    (r"iterate(atan(x), x, 2)", r"atan(atan(x))"),
    (r"iterate(x!, x, 2)", r"(x!)!"),
    (r"iterate(gamma(x), x, 2)", r"gamma(gamma(x))"),
    (
        r"iterate(sqrt(x+1), x, 3)",
        r"sqrt(sqrt(sqrt(x + 1) + 1) + 1)",
    ),
    (r"iterate(x^3+1, x, -1)", r"(x - 1)^(1/3)"),
    (r"iterate(abs(x), x, 2)", r"abs(x)"),
    (r"iterate(e^x-1, x, -1)", r"ln(x + 1)"),
    (r"iterate(e^x-1, x, 2)", r"e^(e^x - 1) - 1"),
    (r"iterate(x*e^x, x, -1)", r"lambertw(x)"),
    (
        r"iterate(sin(x), x, 1/2)",
        r"iterate(sin(x), x, 1/2)|note: no closed form for this iterate: at a number, iterate(f, x, t, x0) or f[t](x0), it is computed by regular iteration at a fixed point",
    ),
    (
        r"iterate(cos(x), x, t)",
        r"iterate(cos(x), x, t)|note: no closed form for this iterate: at a number, iterate(f, x, t, x0) or f[t](x0), it is computed by regular iteration at a fixed point",
    ),
    (
        r"iterate(5, x, 1/2)",
        r"iterate(5, x, 1/2)|note: no closed form for this iterate: at a number, iterate(f, x, t, x0) or f[t](x0), it is computed by regular iteration at a fixed point",
    ),
    (
        r"iterate(x^2, y, 1/2)",
        r"iterate(x^2, y, 1/2)|note: no closed form for this iterate: at a number, iterate(f, x, t, x0) or f[t](x0), it is computed by regular iteration at a fixed point",
    ),
];

const BRACKETS_AND_USER_FUNCTIONS: &[(&str, &str)] = &[
    (r"exp[t](x)", r"e^^(t + slog(e, x))"),
    (r"exp[1/2](x)", r"e^^(slog(e, x) + 1/2)"),
    (r"exp[1/3](exp[1/3](exp[1/3](x)))", r"e^x"),
    (r"exp[1/2](exp[-1/2](x))", r"x"),
    (r"exp[-1](x)", r"ln(x)"),
    (r"exp[0](x)", r"x"),
    (r"exp[1](x)", r"e^x"),
    (r"exp[2](x)", r"e^(e^x)"),
    (r"ln[1/2](x)", r"e^^(slog(e, x) - 1/2)"),
    (r"ln[2](x)", r"ln(ln(x))"),
    (r"ln[-1](x)", r"e^x"),
    (r"sin[0](x)", r"x"),
    (r"sin[1](x)", r"sin(x)"),
    (r"sin[2](x)", r"sin(sin(x))"),
    (r"sin[-1](x)", r"asin(x)"),
    (r"sin[-2](x)", r"asin(asin(x))"),
    (r"cos[-1](x)", r"acos(x)"),
    (r"tan[-2](x)", r"atan(atan(x))"),
    (r"sqrt[3](x)", r"x^(1/8)"),
    (r"sqrt[1/2](x)", r"x^(sqrt(2)/2)"),
    (r"sqrt[-1](x)", r"x^2"),
    (r"abs[2](x)", r"abs(x)"),
    (r"sinh[-1](x)", r"asinh(x)"),
    (r"cosh[2](x)", r"cosh(cosh(x))"),
    (r"asin[-1](x)", r"sin(x)"),
    (
        r"f(x) := 2x+1; f[1/2](x); f[1/2](f[1/2](x))",
        r"f(x) := 2x + 1|sqrt(2)*x - 1 + sqrt(2)|2x + 1",
    ),
    (
        r"f(x) := 2x+1; f[0.(3)](f[0.(3)](f[0.(3)](x)))",
        r"f(x) := 2x + 1|2x + 1",
    ),
    (
        r"f(x) := 2x+1; f[t](x); f[-t](f[t](x))",
        r"f(x) := 2x + 1|x*2^t + 2^t - 1|x",
    ),
    (
        r"f(x) := x^2-2; f[1/2](x)",
        r"f(x) := x^2 - 2|2cosh(sqrt(2)*acosh(x/2))",
    ),
    (
        r"f(x) := x^2-2; f[1/2](3)",
        r"f(x) := x^2 - 2|2cosh(sqrt(2)*acosh(3/2))|  ~ 4.1567601535073012928327684126346",
    ),
    (
        r"f(x) := 1/(1+x); f[2](x); f[-1](x)",
        r"f(x) := 1/(x + 1)|(x + 1)/(x + 2)|1/x - 1",
    ),
    (
        r"h(x) := (x+1)^2-1; h[1/2](x); h[-1](x); h[1/2](h[1/2](x))",
        r"h(x) := x^2 + 2x|(x + 1)^(sqrt(2)) - 1|sqrt(x + 1) - 1|((x + 1)^(sqrt(2)))^(sqrt(2)) - 1",
    ),
    (
        r"g(x) := sin(x)/2; g[1/2](1)",
        r"g(x) := sin(x)/2|0.62260475375617405272877753879577|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"g(x) := sin(x)/2; g[1/2](g[1/2](1)); g(1)",
        r"g(x) := sin(x)/2|0.42073549240394825332625116081515|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)|sin(1)/2|  ~ 0.42073549240394825332625116081515",
    ),
    (
        r"g(x) := x^2+1; g[1/2](1); g[1/2](g[1/2](1))",
        r"g(x) := x^2 + 1|1.4122853525689035276835226679979|note: iterate: Boettcher's coordinate at infinity (degree 2)|2|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"sin[1/2](x)",
        r"sin[1/2](x)|note: no closed form for this iterate: at a number, iterate(f, x, t, x0) or f[t](x0), it is computed by regular iteration at a fixed point",
    ),
    (
        r"cos[t](1)",
        r"cos[t](1)|note: no closed form for this iterate: at a number, iterate(f, x, t, x0) or f[t](x0), it is computed by regular iteration at a fixed point",
    ),
    (
        r"Γ[1/2](5)",
        r"7.5902884554787652425120448584037|note: iterate: regular iteration at the repelling fixed point 3.562382285 (slope 4.002540886)",
    ),
    (r"iterate(iterate(2x+1, x, 1/2), x, 2)", r"2x + 1"),
    (r"iterate(iterate(x^2, x, 1/2), x, 1/2)", r"x^(2^(1/4))"),
    (r"diff(iterate(2x+1, x, t), t)", r"2^t*ln(2)*(x + 1)"),
    (r"diff(iterate(x^2, x, t), x)", r"2^t*x^(2^t - 1)"),
    (r"limit(iterate(x/(1+x), x, t), t, inf)", r"0"),
    (
        r"solve(iterate(2x+1, x, 1/2) = 5, x)",
        r"x = 3sqrt(2) - 1|  ~ 3.2426406871192851464050661726291",
    ),
    (
        r"solve(iterate(x^2, x, 1/2) = 2, x)",
        r"x = 2^(sqrt(2)/2)|  ~ 1.6325269194381528447734953810247",
    ),
    (
        r"table(iterate(x^2+1, x, k/4, 1), k, 0, 4)",
        r"[1, 1.1927913643206312080330057074488, 1.4122853525689035276835226679979, 1.674103306769920261811824120837, 2]|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"table(iterate(2x+1, x, k/2, 1), k, -2, 2)",
        r"[0, sqrt(2) - 1, 1, 2sqrt(2) - 1, 3]",
    ),
];

const CLOSED_FORMS_AT_NUMBERS: &[(&str, &str)] = &[
    (
        r"iterate(2x+1, x, 1/2, 3)",
        r"4sqrt(2) - 1|  ~ 4.6568542494923801952067548968388",
    ),
    (r"iterate(2x+1, x, t, 3)", r"4*2^t - 1"),
    (
        r"iterate(2x+1, x, -1/2, 3)",
        r"2sqrt(2) - 1|  ~ 1.8284271247461900976033774484194",
    ),
    (
        r"iterate(2x+1, x, i, 1)",
        r"2*2^i - 1|  ~ 0.53847780272794425315665998732254 + 1.2779225526272696023000658229294i",
    ),
    (r"iterate(x+3, x, 1/2, 1)", r"5/2|  ~ 2.5"),
    (
        r"iterate(-x/3+1, x, 1/2, 2)",
        r"3/4 + 5i*sqrt(3)/12|  ~ 0.75 + 0.72168783648703220563643597562745i",
    ),
    (
        r"iterate(x^2, x, 1/2, 3)",
        r"3^(sqrt(2))|  ~ 4.728804387837414947894283340416",
    ),
    (
        r"iterate(x^2, x, 1/2, -2)",
        r"(-2)^(sqrt(2))|  ~ -0.70960886530166095454384989230327 - 2.5689391895491229398133660631043i",
    ),
    (r"iterate(x^2, x, -1, 9)", r"3"),
    (
        r"iterate(x^2, x, i, 2)",
        r"2^(2^i)|  ~ 1.5399244240561272794444062258359 + 0.73041852966302852483884065413469i",
    ),
    (
        r"iterate(3x^2, x, 1/2, 2)",
        r"2^(sqrt(2))*3^(sqrt(2))/3|  ~ 4.200981772057574132496372538147",
    ),
    (
        r"iterate(sqrt(x), x, 1/2, 16)",
        r"2^(2sqrt(2))|  ~ 7.1029933013160154008607800151636",
    ),
    (
        r"iterate(1/x, x, 1/2, 2)",
        r"2^i|  ~ 0.76923890136397212657832999366127 + 0.6389612763136348011500329114647i",
    ),
    (
        r"iterate(x/(1+x), x, 1/2, 1)",
        r"2/3|  ~ 0.66666666666666666666666666666667",
    ),
    (r"iterate(x/(1+x), x, t, 1)", r"1/(t + 1)"),
    (
        r"iterate(1/(1+x), x, 2, 1)",
        r"2/3|  ~ 0.66666666666666666666666666666667",
    ),
    (
        r"iterate(1/(1+x), x, 1/2, 1)",
        r"3/5 + sqrt(1/2 + sqrt(5)/2)*sqrt(1/2 - sqrt(5)/2)/5|  ~ 0.6 + 0.2i",
    ),
    (
        r"iterate((2x+1)/(x+3), x, 1/2, 1)",
        r"7/11 + sqrt(5/2 + sqrt(5)/2)*sqrt(5/2 - sqrt(5)/2)/11|  ~ 0.83964254340907179058265215170284",
    ),
    (
        r"iterate(x^2+2x, x, 1/2, 1)",
        r"2^(sqrt(2)) - 1|  ~ 1.6651441426902251886502972498731",
    ),
    (
        r"iterate(x^2-2, x, 1/2, 3)",
        r"2cosh(sqrt(2)*acosh(3/2))|  ~ 4.1567601535073012928327684126346",
    ),
    (
        r"iterate(x^2-2, x, 1/2, 1)",
        r"2cosh(sqrt(2)*acosh(1/2))|  ~ 0.1794291235643967734077837084637",
    ),
    (
        r"iterate(2x^2-1, x, 1/2, 2)",
        r"cosh(sqrt(2)*acosh(2))|  ~ 3.2974234983248402816961191955714",
    ),
    (
        r"iterate(4x*(1-x), x, 1/2, 3/10)",
        r"1/2 - cosh(sqrt(2)*acosh(2/5))/2|  ~ 0.53430923736158066290632075177437",
    ),
    (
        r"iterate(4x*(1-x), x, 1/2, 1/2)",
        r"1/2 - cos(sqrt(2)*pi/2)/2|  ~ 0.8028499335394067144022181781251",
    ),
    (
        r"iterate(x^3+3x, x, 1/2, 2)",
        r"2sinh(sqrt(3)*asinh(1))|  ~ 4.3851506224376011846552724956723",
    ),
    (
        r"iterate(4x^3-3x, x, 1/2, 2)",
        r"cosh(sqrt(3)*acosh(2))|  ~ 4.9445059279250598136588339201949",
    ),
    (
        r"iterate(x^3+3x^2+3x, x, 1/2, 1)",
        r"2^(sqrt(3)) - 1|  ~ 2.3219970854839128051571831196478",
    ),
    (
        r"iterate(x^5+5x^3+5x, x, 1/3, 1)",
        r"2sinh(5^(1/3)*asinh(1/2))|  ~ 1.8378306972260257582233051643381",
    ),
    (r"iterate(8x^4-8x^2+1, x, 1/2, 2)", r"cosh(2acosh(2))|  ~ 7"),
    (
        r"iterate(sin(x), x, 2, 1)",
        r"sin(sin(1))|  ~ 0.74562414166555788889315107043038",
    ),
    (
        r"iterate(sin(x), x, -1, 1/2)",
        r"pi/6|  ~ 0.52359877559829887307710723054658",
    ),
    (
        r"iterate(cos(x), x, 3, 0)",
        r"cos(cos(1))|  ~ 0.85755321584639341574410627276199",
    ),
    (
        r"iterate(e^x, x, 2, 0)",
        r"e|  ~ 2.7182818284590452353602874713527",
    ),
    (r"iterate(e^x, x, -1, e)", r"1"),
    (r"iterate(e^x, x, -2, e^e)", r"1"),
    (r"iterate(ln(x), x, 2, e^e)", r"1"),
    (r"iterate(x^2+1, x, 3, 1)", r"26"),
    (r"iterate(x^2+1, x, -1, 5)", r"2"),
    (r"iterate(x^2+1, x, 0, 7)", r"7"),
    (r"iterate(x^2+1, x, 1, 7)", r"50"),
    (
        r"cos[3](0)",
        r"cos(cos(1))|  ~ 0.85755321584639341574410627276199",
    ),
    (r"sqrt[2](16)", r"2"),
    (r"exp[-1](e)", r"1"),
    (
        r"-n | iterate(2x+1, x, 1/2, 3)",
        r"4.6568542494923801952067548968388",
    ),
    (
        r"-n | iterate(x^2-2, x, 1/2, 3)",
        r"4.1567601535073012928327684126346",
    ),
    (
        r"-r | iterate(x^2, x, 1/2, 3)",
        r"3^(sqrt(2))|  ~ 4.728804387837414947894283340416",
    ),
    (
        r"-r | iterate(x^2, x, 1/2, -2)",
        r"undefined|note: the value (-2)^(sqrt(2)) is not a real number",
    ),
];

const FIXED_POINT_THAT_IS_ASKED_FOR: &[(&str, &str)] = &[
    (
        r"iterate(sqrt(2)^x, x, 1/2, 3, 2)",
        r"2.9137867346334526079794417475975|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 1/2, 3, 4)",
        r"2.913786734633452607979441916978|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 1/2, 1, 2)",
        r"1.2436216276685218042950989836094|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 1/2, 5, 4)",
        r"5.273647369178106461157851724187|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"iterate(sqrt(2)^x, x, -1/2, 3, 2)",
        r"3.0857905755693308961143633726927|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"iterate(sqrt(2)^x, x, -1/2, 3, 4)",
        r"3.0857905755693308961143635404226|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 1/2, 3, 2) - iterate(sqrt(2)^x, x, 1/2, 3, 4)) < 10^-20",
        r"true|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 1/2, 3, 2) - iterate(sqrt(2)^x, x, 1/2, 3, 4)) > 10^-27",
        r"true|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"iterate(e^x, x, 1/2, 1/2, 3/10+13i/10)",
        r"0.99303919280011931894522795776518 + 0.13114284571242883021571603863683i|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i)",
    ),
    (
        r"iterate(e^x, x, 1/2, 1/2, 3/10-13i/10)",
        r"0.99303919280011931894522795776518 - 0.13114284571242883021571603863683i|note: iterate: regular iteration at the repelling fixed point 0.3181315052 - 1.337235701i (slope 0.3181315052 - 1.337235701i)",
    ),
    (
        r"iterate(e^x, x, 1/2, 1, 2+7i)",
        r"-1.3542739931482792851089420715015 + 0.16181099503187630157531777361401i|note: iterate: regular iteration at the repelling fixed point 2.06227773 + 7.588631178i (slope 2.06227773 + 7.588631178i); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 6.283185307i; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(e^x, x, 1/2, 1, inf)",
        r"error: the orbit of the point does not run to infinity as a fixed point of a quotient of polynomials",
    ),
    (
        r"iterate(e^x, x, 1/2, 1, 5)",
        r"error: the map has no fixed point next to 5",
    ),
    (
        r"iterate(x^2+1, x, 1/2, 1, inf)",
        r"1.4122853525689035276835226679979|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(x^2+1, x, 1/2, 1, 1/2+9i/10)",
        r"1.4063485809471133836658484209548 - 0.32290831034360355928710016190858i|note: iterate: regular iteration at the repelling fixed point 0.5 + 0.8660254038i (slope 1 + 1.732050808i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2+1, x, 1/2, 1, 1/2-9i/10)",
        r"error: the orbit of the point does not run into this fixed point, forward or backward",
    ),
    (
        r"iterate(x^2-1, x, 1/2, -3/10)",
        r"-0.88159462984144529854997076792026|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8366600265",
    ),
    (
        r"iterate(x^2-1, x, 1/2, -3/10, -3/5)",
        r"-0.67429403750566814940818095567885 + 0.41174675597706056375806695302735i|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 1/2, -3/10, 8/5)",
        r"-0.88159462984144529854997076792026|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8366600265",
    ),
    (
        r"iterate(x^2-1, x, 1/2, 1/2, 8/5)",
        r"-0.11172132280572776414076661452449|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 1/2, 3, inf)",
        r"4.4436388563574825638201081884461|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(2x, x, 1/2, 3, 0)",
        r"4.2426406871192851464050661726291|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2x, x, 1/2, 3, inf)",
        r"4.2426406871192851464050661726291|note: iterate: regular iteration at infinity (slope 0.5 in 1/x)",
    ),
    (
        r"iterate(2x+1, x, 1/2, 3, -1)",
        r"4.6568542494923801952067548968388|note: iterate: regular iteration at the repelling fixed point -1 (slope 2)",
    ),
    (
        r"iterate(x/2, x, 1/2, 3, 0)",
        r"2.1213203435596425732025330863145|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x^2, x, 1/2, 3, 1)",
        r"4.728804387837414947894283340416|note: iterate: regular iteration at the repelling fixed point 1 (slope 2)",
    ),
    (
        r"iterate(x^2, x, 1/2, 3, inf)",
        r"4.728804387837414947894283340416|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(x^2, x, 1/2, 3, 0)",
        r"error: the orbit of the point does not run into this fixed point, forward or backward",
    ),
    (
        r"iterate(x^2, x, 1/2, 1/3, 0)",
        r"0.21146994419393222993591154352295|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2+x, x, 1/2, 1/100, 0)",
        r"0.010049752469166316255239097801739|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x^2+x, x, 1/2, 1, 0)",
        r"1.3655261096330619912233668239354|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x^2+x, x, 1/2, 1, inf)",
        r"1.3655259977993247005520607218162|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(4x*(1-x), x, 1/2, 3/10, 0)",
        r"0.53430923736158066290632075177437|note: iterate: regular iteration at the repelling fixed point 0 (slope 4)",
    ),
    (
        r"iterate(4x*(1-x), x, 1/2, 3/10, 3/4)",
        r"1.0023870952402706530425846820713 - 0.75477041568404781212852038534563i|note: iterate: regular iteration at the repelling fixed point 0.75 (slope -2); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.9183300133",
    ),
    (
        r"iterate(x^3+3x, x, 1/2, 1/2, 0)",
        r"0.88373996001655615951283859127481|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(x^3+3x, x, 1/2, 1/2, inf)",
        r"0.88373996001655615951283859127481|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"iterate(x^3-x-1, x, 1/2, 3/10, 8/5)",
        r"-0.89219685146884412662704505752917|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 6.854101966); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 1.390912688",
    ),
    (
        r"iterate(x^3-x-1, x, 1/2, 3/10, -1)",
        r"error: the orbit of the point does not run into this fixed point, forward or backward",
    ),
    (
        r"iterate(x!, x, 1/2, 4, 2)",
        r"6.7025307323287791406936437850177|note: iterate: regular iteration at the repelling fixed point 2 (slope 1.84556867)",
    ),
    (
        r"iterate(x!, x, 1/2, 3/2, 1)",
        r"1.413248045977136087581870415891|note: iterate: regular iteration at the attracting fixed point 1 (slope 0.4227843351)",
    ),
    (
        r"iterate(cos(x), x, 1/2, 1, 7/10)",
        r"0.75308905103837029464247966234978 + 0.21095439744221607041226043321636i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"iterate(gamma(x), x, 1/2, 3, 7/2)",
        r"2.5768899025507045084556987928725|note: iterate: regular iteration at the repelling fixed point 3.562382285 (slope 4.002540886)",
    ),
    (
        r"iterate(gamma(x), x, 1/2, 3, 1)",
        r"1|note: iterate: regular iteration at the attracting fixed point 1 (slope -0.5772156649); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1, which the map takes to the same points",
    ),
    (
        r"iterate(x+1/x, x, 1/2, 2, inf)",
        r"2.2676941608146219556986675663268|note: iterate: regular iteration at infinity (slope 1 in 1/x)",
    ),
    (
        r"iterate(e^x-1, x, 1/2, 1, 0)",
        r"1.2710274138899515214246518766723|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/2, -1, 0)",
        r"-0.77115029972018263753974709071005|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(sin(x), x, 1/2, 1, 0)",
        r"0.90870842974336355496561124039192|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(-x+x^2, x, 1/2, 1/10, 0)",
        r"error: the slope at the fixed point is a root of 1 (its power 2 is 1): the map is no rotation there",
    ),
    (
        r"iterate(-x+x^2, x, 1/2, 3, 2)",
        r"3.9247139068483878247337003634068|note: iterate: regular iteration at the repelling fixed point 2 (slope 3)",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, 1/2, 1/20, 0)",
        r"0.044836303794712618571262068267387 + 0.023126094046315493993809066424038i|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, 1/2, 3, inf)",
        r"5.1510150562072114361552993081563 + 0.59939259953983620717477296803819i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
];

const POINTS_OFF_THE_BRANCH: &[(&str, &str)] = &[
    (
        r"iterate(x/2+x^2/4, x, 1/2, -17/10)",
        r"-0.1917013159269748671039126256453|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at -0.3, which the map takes to the same points",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/2, -3/10)",
        r"-0.1917013159269748671039126256453|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/3, -3)",
        r"0.91686966580286070256101508199015|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1, which the map takes to the same points",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/2, -1)",
        r"-0.40948243823861063351355508853593|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points",
    ),
    (
        r"iterate(cos(x), x, 1/3, 4)",
        r"0.793607727055033515830416762599 + 0.089899016398739752059995040146116i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.8584073464, which the map takes to the same points",
    ),
    (
        r"iterate(cos(x), x, 1/2, -1/2)",
        r"0.7482098722099504605790314493673 - 0.169079025682341043470440461404i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.5, which the map takes to the same points",
    ),
    (
        r"iterate(cos(x), x, 1/2, 1/2)",
        r"0.7482098722099504605790314493673 - 0.169079025682341043470440461404i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"iterate(cos(x), x, 1/2, 100)",
        r"0.74640461895274204903480284084044 - 0.15103492218567984692758163490716i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.5309649149, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)/2, x, 1/2, 5/2)",
        r"0.43228377702417044226381392872244|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.6415926536, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)/2, x, 1/2, pi-5/2)",
        r"0.43228377702417044226381392872244|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(5/2*x*(1-x), x, 1/2, 1/10)",
        r"0.61406750902174611615140307718092 - 0.071154291489336361997748982650702i|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points",
    ),
    (
        r"iterate(5/2*x*(1-x), x, 1/2, 9/10)",
        r"0.61406750902174611615140307718092 - 0.071154291489336361997748982650702i|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points",
    ),
    (
        r"iterate((x^2+1)/3, x, 1/2, -2)",
        r"1.8453249791400491695605278937568|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 2, which the map takes to the same points",
    ),
    (
        r"iterate((x^2+1)/3, x, 1/2, 2)",
        r"1.8453249791400491695605278937568|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075)",
    ),
    (
        r"iterate(x*e^(-x)/2, x, 1/2, 3)",
        r"0.11309289963669972088220931285464|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.1785606279, which the map takes to the same points",
    ),
    (
        r"iterate(x+x^2, x, 1/2, -4/5)",
        r"-0.17746394120091573903217684680391|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at -0.2, which the map takes to the same points",
    ),
    (
        r"iterate(x+x^2, x, 1/2, -1/5)",
        r"-0.17746394120091573903217684680391|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, 1/2, -1/2)",
        r"-0.3087254371867657812986447688061|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x), x, 1/2, 4)",
        r"-0.80223683636499358020360177155301|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at -0.8584073464, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x), x, 1/2, 4-pi)",
        r"0.80223683636499358020360177155301|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, 1/2, -1)",
        r"-0.48866481866503552878680514997834|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points",
    ),
    (
        r"iterate(x^2+x^3, x, 1/2, -9/10)",
        r"-0.032449970272383050682637644323295 - 0.17370097399184511252786864818373i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at -0.3541381265, which the map takes to the same points; the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2+x^3, x, 1/2, -10001/10000)",
        r"-0.00089894707155426176635254893127245 + 0.0011823746437270404839871082949556i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.00005 + 0.01000037499i, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)^2, x, 1/2, 3)",
        r"0.062748296882973421544049458121|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.1415926536, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)^2, x, 1/2, pi-3)",
        r"0.062748296882973421544049458121|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(1/x+x^2, x, 1/2, 3/10)",
        r"2.3137566021843777460068510185924|note: iterate: Boettcher's coordinate at infinity (degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1.681893374, which the map takes to the same points",
    ),
    (
        r"iterate(x+1/x, x, 1/2, 3/10)",
        r"3.4868648429221743829174272379997|note: iterate: regular iteration at infinity (slope 1 in 1/x); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 3.333333333, which the map takes to the same points",
    ),
    (
        r"iterate(x+1/x, x, 1/2, 10/3)",
        r"3.4868648429221743829174272379997|note: iterate: regular iteration at infinity (slope 1 in 1/x)",
    ),
    (
        r"iterate(x^2-1, x, 3/2, 1/2)",
        r"-0.98751834603053837338698148955644|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2-1, x, 1/2, -13/10)",
        r"-0.65324844569828880496882582544264 - 0.79563373382500255003186012077159i|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.5477225575i; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2+1, x, 1/2, -2)",
        r"-0.80873174384614114528151036372511 - 3.1950760544952362799226328811013i|note: iterate: Boettcher's coordinate at infinity (degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2+1, x, 1/2, 2)",
        r"2.9945499170806721420207823353576|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(x^2+x^3, x, 1/2, -1/2)",
        r"-0.037590264253320219401464989424921 - 0.24144162439522375870429001408923i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^3-x, x, 1/2, 3/10)",
        r"-0.27026731890498069101807888560796|note: iterate: regular iteration at the repelling fixed point 1.414213562 (slope 5); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 1.125418783",
    ),
    (
        r"iterate(cos(x)+x, x, 1/2, 1)",
        r"1.7003718204807576203588316542958 - 0.14526280137176207505059350479848i|note: iterate: Boettcher's coordinate at the fixed point 1.570796327 (slope 0, degree 3); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(e^x-1, x, 1/2, 1+3i/10)",
        r"1.2427552538674090045473634254619 + 0.46842303621741052416887261710045i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/2, -1+3i/10)",
        r"-0.78774743039099709967200243168862 + 0.16858517639782691457238108142548i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/2, 6i)",
        r"-0.020048439741352258548320388934239 - 0.2827126615398246420309292817933i|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at -0.2831853072i, which the map takes to the same points",
    ),
];

const ORBITS_THAT_END_ON_THE_FIXED_POINT: &[(&str, &str)] = &[
    (
        r"iterate(sin(x)/2, x, 1/2, pi)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)/2, x, 1/3, -3pi)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)/2, x, -1/2, pi)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)/2, x, i, 2pi)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x), x, 1/2, pi)",
        r"0|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x), x, 1/3, 5pi)",
        r"0|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(gamma(x), x, 1/2, 3)",
        r"1|note: iterate: regular iteration at the attracting fixed point 1 (slope -0.5772156649); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1, which the map takes to the same points",
    ),
    (
        r"iterate(gamma(x), x, 1/2, 2)",
        r"1|note: iterate: regular iteration at the attracting fixed point 1 (slope -0.5772156649); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1, which the map takes to the same points",
    ),
    (
        r"iterate(gamma(x), x, 2/3, 3)",
        r"1|note: iterate: regular iteration at the attracting fixed point 1 (slope -0.5772156649); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1, which the map takes to the same points",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/2, -2)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x/2+x^2/4, x, 7/10, -2)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x^2+x^3, x, 1/2, -1)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x^2+x^3, x, 1/3, -1)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)^2, x, 1/2, pi)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)^2, x, 1/2, -2pi)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x^2-x, x, 1/2, 1)",
        r"error: the slope at the fixed point has size 1 and is not 1: no regular iteration (a number next to another fixed point of the map, as a fifth argument, gives the iterate at that one)",
    ),
    (
        r"iterate(cos(x), x, 1/2, 0.73908513321516064165531208767387340401341175890076)",
        r"0.73908513321516064165531208767387340401341175890076|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/2, 0)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x^2+x^3, x, 1/2, 0)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(e^x-1, x, 1/2, 0)",
        r"0|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x^2-1, x, 1/2, (1+sqrt(5))/2)",
        r"1.6180339887498948482045868343656|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 1/2, 0)",
        r"-0.65533437384651359305942801453463|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(2sinh(x), x, 1/2, 0)",
        r"0|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
];

const NEXT_TO_A_FIXED_POINT_AND_MANY_STEPS: &[(&str, &str)] = &[
    (
        r"iterate(e^x-1, x, 1/2, 10^-80)",
        r"1e-80|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/3, -10^-80)",
        r"-1e-80|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 5/2, 10^-40)",
        r"1e-40|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/2, i/10^45)",
        r"1e-45i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/2, 10^-20)",
        r"1.0000000000000000000025e-20|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/2, -10^-20)",
        r"-9.999999999999999999975e-21|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/2, 10^-60)",
        r"7.0710678118654752440084436210485e-61|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/2, 10^-30)",
        r"7.0710678118654752440084436210506e-31|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, 1/2, -10^-80)",
        r"-7.0710678118654752440084436210485e-81|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate((e^x-1)/3, x, 1/2, i/10^120)",
        r"5.7735026918962576450914878050196e-121i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"iterate(2sinh(x), x, 1/2, 10^-50)",
        r"1.4142135623730950488016887242097e-50|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2x+e^x-1, x, 1/2, -10^-70)",
        r"-1.7320508075688772935274463415059e-70|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(2x+sin(x), x, 1/2, 10^-25)",
        r"1.7320508075688772935274463415059e-25|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(sin(x), x, 1/2, 10^-10)",
        r"9.9999999999999999999916666666667e-11|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(tan(x), x, 1/2, 10^-80)",
        r"1e-80|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(sin(x)/2, x, 1/2, pi-10^-60)",
        r"7.0710678118654752440084436210485e-61|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1e-60, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)/2, x, 1/2, 10^-60)",
        r"7.0710678118654752440084436210485e-61|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x^2+x^3, x, 1/2, 10^-300)",
        r"5.4441651113591190292455249954833e-425|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(1-cos(x), x, 1/2, 10^-100)",
        r"2.8441536577224009075852418703597e-142|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate((x-1)^2*(x+1)+1, x, 1/2, 1+10^-50)",
        r"1|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2+1, x, 1/2, 10^80)",
        r"1.3711500689832883217318040772724e113|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(x+1/x, x, 1/2, 10^30)",
        r"1000000000000000000000000000000|note: iterate: regular iteration at infinity (slope 1 in 1/x)",
    ),
    (
        r"iterate(x^2-1, x, 1/2, (1+sqrt(5))/2-10^-40)",
        r"1.6180339887498948482045868343656|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x/2+x^2/4, x, -150, 10^-50)",
        r"0.000014272273227338351618152345979664",
    ),
    (
        r"iterate(x/2+x^2/4, x, 150, iterate(x/2+x^2/4, x, -150, 10^-50))",
        r"1e-50",
    ),
    (
        r"iterate(x/2+x^2/4, x, -100, 10^-40)",
        r"1.2676506000675355970979646575009e-10",
    ),
    (
        r"iterate(x/2+x^2/4, x, 201/2, 3/10)",
        r"2.2992851310277504398824949136459e-31|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(cos(x), x, -61/2, 37/50)",
        r"0.96170571879597297010438215644194 - 1.1088288692215785244053302062262i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"iterate(cos(x), x, 61/2, iterate(cos(x), x, -61/2, 37/50))",
        r"0.74|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"iterate(cos(x), x, 161/4, 37/50)",
        r"0.73908513329541500089899830556554 + 8.0254359239463742698417109999484e-11i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"iterate(cos(x), x, 601/3, 1)",
        r"0.73908513321516064165531208767387|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"iterate(cos(x), x, 100, 1)",
        r"0.73908513321516064351570128929727",
    ),
    (
        r"iterate(cos(x), x, -12+3i, 37/50)",
        r"0.73908831484643814426060393897152 - 0.0000078404000038614780014297805222348i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"iterate(2sinh(x), x, -61/2, 3/10)",
        r"1.9658634852587340759098996213047e-10|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 161/4, 10^-50)",
        r"1.3075470507794426707358454897762e-38|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(x^2+x^3, x, 21/2, 9/10)",
        r"2.2500786348074012701603977776363e9109|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"iterate(x^2+1, x, 13/2, 1)",
        r"10285613931254970.44509585539764|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(sqrt(x), x, 100, 2)",
        r"2^(1/1267650600228229401496703205376)|  ~ 1.0000000000000000000000000000005",
    ),
];

const WHAT_HAS_NO_ITERATE: &[(&str, &str)] = &[
    (
        r"iterate(x^2, x)",
        r"error: iterate takes 3 to 5 argument(s), 2 given",
    ),
    (
        r"iterate(x^2, x, 1, 2, 3, 4)",
        r"error: iterate takes 3 to 5 argument(s), 6 given",
    ),
    (
        r"iterate(x^2, 3, 1/2)",
        r"error: iterate(f, x, t), iterate(f, x, t, x0) or iterate(f, x, t, x0, p) with a fixed point p, for example iterate(2x + 1, x, 1/2)",
    ),
    (
        r"iterate(sin(x), x, 1/2, 1, y)",
        r"error: y has no numeric value",
    ),
    (
        r"iterate(2x, x, t, 1, 0)",
        r"error: iterate(f, x, t, x0, p) needs numbers for t and x0: iterate(f, x, t), iterate(f, x, t, x0) or iterate(f, x, t, x0, p) with a fixed point p, for example iterate(2x + 1, x, 1/2)",
    ),
    (
        r"iterate(2x, x, 1/2, y, 0)",
        r"error: iterate(f, x, t, x0, p) needs numbers for t and x0: iterate(f, x, t), iterate(f, x, t, x0) or iterate(f, x, t, x0, p) with a fixed point p, for example iterate(2x + 1, x, 1/2)",
    ),
    (
        r"iterate(sin(x), x, 1/2, y)",
        r"iterate(sin(x), x, 1/2, y)|note: no closed form for this iterate: at a number, iterate(f, x, t, x0) or f[t](x0), it is computed by regular iteration at a fixed point",
    ),
    (
        r"iterate(sin(x), x, t, 1)",
        r"iterate(sin(x), x, t, 1)|note: no closed form for this iterate: at a number, iterate(f, x, t, x0) or f[t](x0), it is computed by regular iteration at a fixed point",
    ),
    (
        r"iterate(x+e^x, x, 1/2, 1)",
        r"error: the orbit runs into no fixed point: no regular iteration",
    ),
    (
        r"iterate(x/2+x^2/4, x, -1/2, -1/2)",
        r"error: the iterate would be past a point where the slope of the map is 0, as seen from the fixed point: the branch of the inverse of the map that keeps the fixed point does not go that far",
    ),
    (
        r"iterate(x/2+x^2/4, x, -3/2, 3/10)",
        r"0.59348690823946643437316019759465|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(sin(x)/2, x, -1/2, 1)",
        r"error: the iterate would be past a point where the slope of the map is 0, as seen from the fixed point: the branch of the inverse of the map that keeps the fixed point does not go that far",
    ),
    (
        r"iterate((x^2+1)/3, x, -1/2, 0)",
        r"error: the iterate would be past a point where the slope of the map is 0, as seen from the fixed point: the branch of the inverse of the map that keeps the fixed point does not go that far",
    ),
    (
        r"iterate(sqrt(2)^x, x, -1, -1, 2)",
        r"error: the iterate would be past a point where the slope of the map is 0, as seen from the fixed point: the branch of the inverse of the map that keeps the fixed point does not go that far",
    ),
    (
        r"iterate(x+x^2, x, -1/2, -1/2)",
        r"error: the iterate would be past a point where the slope of the map is 0, as seen from the fixed point: the branch of the inverse of the map that keeps the fixed point does not go that far",
    ),
    (
        r"iterate(x*e^x, x, -1/2, -1)",
        r"error: the iterate would be past a point where the slope of the map is 0, as seen from the fixed point: the branch of the inverse of the map that keeps the fixed point does not go that far",
    ),
    (
        r"iterate(x^2+1, x, -1/2, 1/10)",
        r"error: the iterate would be past a point where the slope of the map is 0, as seen from the fixed point: Boettcher's coordinate has more than one value past it (a number next to another fixed point of the map, as a fifth argument, gives the iterate at that one)",
    ),
    (
        r"iterate(x^2+1, x, i, -2)",
        r"error: the iterate leaves the basin of the fixed point with the slope 0: Boettcher's coordinate would have the size 1 or more",
    ),
    (
        r"iterate(x^2+1, x, i, 3i/2)",
        r"error: the iterate leaves the basin of the fixed point with the slope 0: Boettcher's coordinate would have the size 1 or more",
    ),
    (
        r"iterate(x^2+x^3, x, 3/10-7i/10, -3/10)",
        r"error: the iterate leaves the basin of the fixed point with the slope 0: Boettcher's coordinate would have the size 1 or more",
    ),
    (
        r"iterate(x^2+x^3, x, -4, 3/10)",
        r"0.60396639445566023936218699911976|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2+x, x, 1/2, 1/100)",
        r"error: Boettcher's coordinate at the fixed point with the slope 0 has no value at the point: it is too far, or the coordinate jumps on the line between them (a number next to another fixed point of the map, as a fifth argument, gives the iterate at that one)",
    ),
    (
        r"iterate(x^3-x-1, x, 1/2, 3/10)",
        r"error: the iterate would be past a point where the slope of the map is 0, as seen from the fixed point: Boettcher's coordinate has more than one value past it (a number next to another fixed point of the map, as a fifth argument, gives the iterate at that one)",
    ),
    (
        r"iterate(e^(pi*i*(sqrt(5)-1))*x+x^2, x, i, 1/20)",
        r"error: the iterate would be past the edge of the disk around the fixed point where the map is a rotation, or too close to it for the digits asked for: a count of steps that is not real moves the point across the circles of the rotation",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, 1/2, 1/2+i/3)",
        r"0.3943842143364443270667340605527 + 0.6156585666596157343251029616525i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(2sinh(x), x, 161/4, 3/10)",
        r"error: the map has no value on the way back from its fixed point: the iterate may be too large for a number",
    ),
    (
        r"iterate(e^x-1, x, 30, 1)",
        r"error: the iterate is too large for a number, or the map has no value on the way",
    ),
    (
        r"iterate(-x+x^2, x, 1/2, 1/10, 0)",
        r"error: the slope at the fixed point is a root of 1 (its power 2 is 1): the map is no rotation there",
    ),
    (
        r"iterate(-x+x^2, x, 1/2, 1/10)",
        r"-0.23513967372047610525088236304921|note: iterate: regular iteration at the repelling fixed point 2 (slope 3); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 1.091607978",
    ),
    (
        r"iterate(x^2, x, 1/2, 3, 0)",
        r"error: the orbit of the point does not run into this fixed point, forward or backward",
    ),
    (
        r"iterate(e^x, x, 1/2, 1, inf)",
        r"error: the orbit of the point does not run to infinity as a fixed point of a quotient of polynomials",
    ),
    (
        r"iterate(e^x, x, 1/2, 1, 5)",
        r"error: the map has no fixed point next to 5",
    ),
    (
        r"iterate(x^2+1, x, 1/2, 1, 1/2-9i/10)",
        r"error: the orbit of the point does not run into this fixed point, forward or backward",
    ),
    (r"iterate(x/2+x^2/4, x, -200000, 1/2)", r"2"),
    (r"iterate(cos(x), x, 10^7, 1)", r"error: too many steps"),
    (
        r"iterate(1/x^2+x^2, x, 1/2, 0)",
        r"1.488744252706104902447384797765 - 0.76484220232323246164368301817286i|note: iterate: regular iteration at the repelling fixed point -0.5189127944 - 0.6666098449i (slope -4.075651178 - 2.66643938i)",
    ),
    (
        r"iterate(ln(x), x, 1/2, 0, 3/10+13i/10)",
        r"error: the orbit of the point does not run into this fixed point, forward or backward",
    ),
    (
        r"-r | cos[1/2](1)",
        r"undefined|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)|note: the value 0.75308905103837029464247966234978 + 0.21095439744221607041226043321636i is not a real number",
    ),
    (
        r"-r | iterate(x/2+x^2/4, x, 1/2, 3/10)",
        r"0.22941785117706664604894617111227|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"-r | iterate(e^x-1, x, 1/2, 1)",
        r"1.2710274138899515214246518766723|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"-r | iterate(x^2-1, x, 1/2, -3/10, -3/5)",
        r"undefined|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)|note: the value -0.67429403750566814940818095567885 + 0.41174675597706056375806695302735i is not a real number",
    ),
];

const ATTRACTING_1: &[(&str, &str)] = &[
    (
        r"iterate((e^x-1)/3, x, -1/2, -1e-80)",
        r"-1.7320508075688772935274463415059e-80|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"iterate((e^x-1)/3, x, -1/2, 1e-50)",
        r"1.7320508075688772935274463415059e-50|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"iterate((e^x-1)/3, x, 1/2, 1e-50)",
        r"5.7735026918962576450914878050196e-51|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"iterate((e^x-1)/3, x, 1/3, 1e-50)",
        r"6.9336127435063470484335227478596e-51|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"iterate((e^x-1)/3, x, 3/10-7i/10, 1e-120i)",
        r"-5.0017473395418993075342612801011e-121 + 5.1682404499562699863243658995653e-121i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"iterate((e^x-1)/3, x, 5/2, 1e-120i)",
        r"6.4150029909958418278794308944662e-122i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"iterate((e^x-1)/3, x, 7/4, 1e-120i)",
        r"1.4623044588361027342435765575522e-121i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"iterate((e^x-1)/3, x, i, -1e-80)",
        r"-4.5483242282660975502756514359504e-81 + 8.9057704166774705907492730656518e-81i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"iterate((e^x-1)/3, x, pi/4, -1e-80)",
        r"-4.2195860009209905820062256461243e-81|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"iterate((x^2+1)/3, x, -1, -1+0.5i)",
        r"0.36878304777929095037904182213839 + 2.0337160412234011761227004050727i",
    ),
    (
        r"iterate((x^2+1)/3, x, -1, 2)",
        r"sqrt(5)|  ~ 2.2360679774997896964091736687313",
    ),
    (
        r"iterate((x^2+1)/3, x, 1/2, -1+0.5i)",
        r"0.76692236732753637604222551734658 - 0.4345101922053278083450208617578i|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1 - 0.5i, which the map takes to the same points",
    ),
    (
        r"iterate((x^2+1)/3, x, 1/3, -1+0.5i)",
        r"0.84045638234233428852988532885991 - 0.46114185716731166025817767432705i|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1 - 0.5i, which the map takes to the same points",
    ),
    (
        r"iterate((x^2+1)/3, x, 3/10-7i/10, -1+0.5i)",
        r"0.92037753158724458227121346762618 - 0.17381286368054854876577662675755i|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1 - 0.5i, which the map takes to the same points",
    ),
    (
        r"iterate((x^2+1)/3, x, 5/2, -1+0.5i)",
        r"0.38940943866184991690680638035497 - 0.069084466808210019361517738046152i|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1 - 0.5i, which the map takes to the same points",
    ),
    (
        r"iterate((x^2+1)/3, x, 7/4, -1+0.5i)",
        r"0.43185710994561154059861693831247 - 0.17222771417469680765377782901595i|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1 - 0.5i, which the map takes to the same points",
    ),
    (
        r"iterate((x^2+1)/3, x, i, -1+0.5i)",
        r"0.85735287776484576805939822196041 - 1.1539636859981854564854067762151i|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1 - 0.5i, which the map takes to the same points",
    ),
    (
        r"iterate((x^2+1)/3, x, pi/4, -1+0.5i)",
        r"0.65449415573933915564289029903723 - 0.37978531110865500473076895792858i|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1 - 0.5i, which the map takes to the same points",
    ),
    (
        r"iterate(9/10*atan(x), x, 1/2, 0.3)",
        r"0.28025417147054349552020265614607|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.9)",
    ),
    (
        r"iterate(9/10*atan(x), x, 1/3, 0.3)",
        r"0.2866217297204063693955622240348|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.9)",
    ),
    (
        r"iterate(9/10*atan(x), x, 5/2, 1)",
        r"0.49989700027058620280486944996842|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.9)",
    ),
    (
        r"iterate((1/2+i/3)*x+x^2, x, -1, -0.1+0.05i)",
        r"-0.046066814399912276558065736020695 + 0.16023778721363396785050750058156i",
    ),
    (
        r"iterate((1/2+i/3)*x+x^2, x, -1/2, -0.1+0.05i)",
        r"-0.089062005564752179149378173848899 + 0.10824631887616472594285506095643i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5 + 0.3333333333i)",
    ),
    (
        r"iterate((1/2+i/3)*x+x^2, x, 1/2, -0.1+0.05i)",
        r"-0.084398333789455385954287691889096 + 0.0051545557416057932423841030647239i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5 + 0.3333333333i)",
    ),
    (
        r"iterate((1/2+i/3)*x+x^2, x, 1/3, -0.1+0.05i)",
        r"-0.091583106231882645543963154080927 + 0.017757164055675798222036905890298i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5 + 0.3333333333i)",
    ),
    (
        r"iterate((1/2+i/3)*x+x^2, x, 3/10-7i/10, -0.1+0.05i)",
        r"-0.17453925221944904675955806181612 - 0.041822323976362489068920920146248i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5 + 0.3333333333i)",
    ),
    (
        r"iterate((1/2+i/3)*x+x^2, x, 5/2, -0.1+0.05i)",
        r"-0.0089444338388030360949565378662857 - 0.023540376566517794940601068823891i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5 + 0.3333333333i)",
    ),
    (
        r"iterate((1/2+i/3)*x+x^2, x, 7/4, -0.1+0.05i)",
        r"-0.027848880815157141558788980426324 - 0.027207925155894019922507292716659i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5 + 0.3333333333i)",
    ),
    (
        r"iterate((1/2+i/3)*x+x^2, x, i, -0.1+0.05i)",
        r"-0.030714951058396524605838450079135 + 0.042906619451125328645475755882681i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5 + 0.3333333333i)",
    ),
    (
        r"iterate((1/2+i/3)*x+x^2, x, pi/4, -0.1+0.05i)",
        r"-0.070103993028396901750131039330817 - 0.010662358448106304955684085750896i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5 + 0.3333333333i)",
    ),
    (
        r"iterate(cos(x), x, -1, -0.5)",
        r"2.0943951023931954923084289221863",
    ),
    (
        r"iterate(cos(x), x, -1/2, -0.5)",
        r"0.75752684068034694373074715513071 + 0.24364887709234489830742676757048i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.5, which the map takes to the same points",
    ),
    (
        r"iterate(cos(x), x, 1/2, -0.5)",
        r"0.7482098722099504605790314493673 - 0.169079025682341043470440461404i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.5, which the map takes to the same points",
    ),
    (
        r"iterate(cos(x), x, 1/3, -0.5)",
        r"0.6582303619310517731903673603339 - 0.1691943487451034272439694800229i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.5, which the map takes to the same points",
    ),
    (
        r"iterate(cos(x), x, 3/10-7i/10, -0.5)",
        r"0.79045378161688660713993843012675 - 0.83654692806589381022173125192952i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.5, which the map takes to the same points",
    ),
    (
        r"iterate(cos(x), x, 5/2, -0.5)",
        r"0.74108806785545788696793577529087 - 0.078397641594745170613080660103441i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.5, which the map takes to the same points",
    ),
    (
        r"iterate(cos(x), x, 7/4, -0.5)",
        r"0.66504021352353894172678551957492 + 0.079471009108001027486339944722601i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.5, which the map takes to the same points",
    ),
    (
        r"iterate(cos(x), x, i, -0.5)",
        r"0.73061944996620329816529529160521 + 0.003542469643563702009705443335621i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.5, which the map takes to the same points",
    ),
    (
        r"iterate(cos(x), x, pi/4, -0.5)",
        r"0.85738479761465305059954360415416 - 0.092206045804372131940952754658163i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.5, which the map takes to the same points",
    ),
    (
        r"iterate(cos(x), x, -12+3i, 0.7390851)",
        r"0.73908513309968348169535893662363 + 2.8456905432840934338482678334027e-10i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"iterate(cos(x), x, -61/2, 0.7390851)",
        r"0.7390957314167947909157020022344 + 0.0056862098508054254341386448511798i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"iterate(cos(x), x, 161/4, 0.74)",
        r"0.73908513329541500089899830556554 + 8.0254359239463742698417109999484e-11i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"iterate(gamma(x), x, -1/2, 2)",
        r"1|note: iterate: regular iteration at the attracting fixed point 1 (slope -0.5772156649); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1, which the map takes to the same points",
    ),
    (
        r"iterate(gamma(x), x, 1/3, 3)",
        r"1|note: iterate: regular iteration at the attracting fixed point 1 (slope -0.5772156649); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1, which the map takes to the same points",
    ),
    (
        r"iterate(gamma(x), x, 7/4, 2)",
        r"1|note: iterate: regular iteration at the attracting fixed point 1 (slope -0.5772156649); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1, which the map takes to the same points",
    ),
    (
        r"iterate(ln(1+x)/2, x, -1, 1)",
        r"e^2 - 1|  ~ 6.389056098930650227230427460575",
    ),
    (
        r"iterate(ln(1+x)/2, x, -1/2, 1)",
        r"2.0901753197142005613017641827419|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, 1/2, 1)",
        r"0.56411391353585677718509928993106|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, 1/3, 1)",
        r"0.67451448499909004961215737800669|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, 3/10-7i/10, 1)",
        r"0.46388190091696110014551858439814 + 0.43609573869262849477487288396421i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, 5/2, 1)",
        r"0.10092307644137185639758379003197|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, 7/4, 1)",
        r"0.18187450512733887934095227540104|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, i, 1)",
        r"0.3159366555484494751197242921352 - 0.70689024704176034135942512221934i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, pi/4, 1)",
        r"0.42394757748252799424018151530718|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, -1/2, -1e-80)",
        r"-1.4142135623730950488016887242097e-80|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, -1/2, 1e-50)",
        r"1.4142135623730950488016887242097e-50|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, 1/2, 1e-50)",
        r"7.0710678118654752440084436210485e-51|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, 1/3, 1e-50)",
        r"7.9370052598409973737585281963615e-51|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, 3/10-7i/10, 1e-50)",
        r"7.1850245079836071026692178178826e-51 + 3.7882473993021677573606274392453e-51i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, 5/2, -1e-80)",
        r"-1.7677669529663688110021109052621e-81|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
];

const ATTRACTING_2: &[(&str, &str)] = &[
    (
        r"iterate(ln(1+x)/2, x, 7/4, -1e-80)",
        r"-2.9730177875068026667937499264012e-81|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, i, -1e-80)",
        r"-7.6923890136397212657832999366127e-81 + 6.389612763136348011500329114647e-81i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, pi/4, -1e-80)",
        r"-5.8019181037172444361518420439005e-81|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(5/2*x*(1-x), x, -1, 0.1)",
        r"0.9582575694955840006588047193728",
    ),
    (
        r"iterate(5/2*x*(1-x), x, -1/2, 0.1)",
        r"0.62878023453904651962190849800763 + 0.11050498819795546504894399357855i|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points",
    ),
    (
        r"iterate(5/2*x*(1-x), x, 1/2, 0.1)",
        r"0.61406750902174611615140307718092 - 0.071154291489336361997748982650702i|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points",
    ),
    (
        r"iterate(5/2*x*(1-x), x, 1/3, 0.1)",
        r"0.58573401552176471586303333200094 - 0.084951093874560829887121758260542i|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points",
    ),
    (
        r"iterate(5/2*x*(1-x), x, 3/10-7i/10, 0.1)",
        r"0.66169835108916907820537602079933 - 0.2003727024065821434852534381643i|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points",
    ),
    (
        r"iterate(5/2*x*(1-x), x, 5/2, 0.1)",
        r"0.60148705622605148773083347738698 - 0.02133167426115619142988174006094i|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points",
    ),
    (
        r"iterate(5/2*x*(1-x), x, 7/4, 0.1)",
        r"0.5766024214605125046238629321913 + 0.031354846402811138223515404323477i|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points",
    ),
    (
        r"iterate(5/2*x*(1-x), x, i, 0.1)",
        r"0.59589418607612744156833316776937 + 0.0034977781451384963237408341463289i|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points",
    ),
    (
        r"iterate(5/2*x*(1-x), x, pi/4, 0.1)",
        r"0.6489909981856291478564110238652 - 0.037894897639293342361297774473587i|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points",
    ),
    (
        r"iterate(-x/2+x^3, x, -1, 0.1+0.2i)",
        r"-0.11828523629842643443699386639615 - 0.34612398617927284339265615237285i|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"iterate(-x/2+x^3, x, -1/2, 0.1+0.2i)",
        r"0.21682228033475854689227852246968 - 0.25102455551080060507679385573332i|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"iterate(-x/2+x^3, x, 1/2, 0.1+0.2i)",
        r"-0.13920599259576150871754819150434 + 0.10592674542999287493054935473454i|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"iterate(-x/2+x^3, x, 1/3, 0.1+0.2i)",
        r"-0.072162407767983473255533399953292 + 0.16595879826620859380827858163158i|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"iterate(-x/2+x^3, x, 3/10-7i/10, 0.1+0.2i)",
        r"-0.10086392540468983587417190794319 + 0.59409176683904963723561121008906i|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"iterate(-x/2+x^3, x, 5/2, 0.1+0.2i)",
        r"-0.035923430948256687867525466359377 + 0.023369537611995615242852952189685i|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"iterate(-x/2+x^3, x, 7/4, 0.1+0.2i)",
        r"0.071116746088673588553864660262122 + 0.015836865117278505979614988386988i|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"iterate(-x/2+x^3, x, i, 0.1+0.2i)",
        r"0.0099736740594123242116105029893022 + 0.0031458553832901461420164203248231i|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"iterate(-x/2+x^3, x, pi/4, 0.1+0.2i)",
        r"-0.13702351114624771407499605845221 - 0.051883771264163120529986530326062i|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"iterate(sin(x)/2, x, -1, -0.7)",
        r"-1.5707963267948966192313216916398 + 0.86701472649056510395052026950147i",
    ),
    (
        r"iterate(sin(x)/2, x, -1, 2.5)",
        r"1.5707963267948966192313216916398 - 2.292431669561177687800787311348i",
    ),
    (
        r"iterate(sin(x)/2, x, 1/2, -0.7)",
        r"-0.46703298784910795515092878681948|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(sin(x)/2, x, 1/3, -0.7)",
        r"-0.53157331132539470481546047758955|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(sin(x)/2, x, 3/10-7i/10, -0.7)",
        r"-0.45225482578647811639605713593233 - 0.27025407420097769177031480185217i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(sin(x)/2, x, 5/2, -0.7)",
        r"-0.11161142468057992614321491967205|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(sin(x)/2, x, 7/4, -0.7)",
        r"-0.18867214229635242551760266916272|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(sin(x)/2, x, i, -0.7)",
        r"-0.44823332478856625814229527776367 + 0.44474836075496289591206885438645i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(sin(x)/2, x, pi/4, -0.7)",
        r"-0.37691889920184093083110346002673|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(sin(x)/2, x, -1/2, -3pi)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)/2, x, -1/2, pi)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)/2, x, 1/2, pi)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)/2, x, 1/3, pi)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)/2, x, 3/10-7i/10, pi)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)/2, x, 5/2, -3pi)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)/2, x, 7/4, -3pi)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)/2, x, i, -3pi)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)/2, x, pi/4, -3pi)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x/100+x^2, x, 1/2, -0.2+0.1i)",
        r"0.089484101548177916394542303325929 - 0.068223714377749036534008027169598i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.19 - 0.1i, which the map takes to the same points",
    ),
    (
        r"iterate(x/100+x^2, x, 1/2, 0.5)",
        r"0.37730660739766739829183359447128|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01)",
    ),
    (
        r"iterate(x/100+x^2, x, 1/3, 0.3)",
        r"0.22114652599371326010048051055422|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01)",
    ),
    (
        r"iterate(x/100+x^2, x, 3/10-7i/10, -0.2+0.1i)",
        r"0.13242374946654066650907477568744 + 0.048906535222922651850851796394709i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.19 - 0.1i, which the map takes to the same points",
    ),
    (
        r"iterate(x/100+x^2, x, 3/10-7i/10, 0.5)",
        r"0.43143945785514151294829923705541 - 0.18829366698483818985327825334538i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01)",
    ),
    (
        r"iterate(x/100+x^2, x, 5/2, 0.3)",
        r"0.0016387802261971353627462533674627|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01)",
    ),
    (
        r"iterate(x/100+x^2, x, i, -0.2+0.1i)",
        r"0.19817885534240062266651940179299 - 0.015990235312071671001504031650456i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.19 - 0.1i, which the map takes to the same points",
    ),
    (
        r"iterate(x/100+x^2, x, i, 0.5)",
        r"0.50466745668859805050702756963754 + 0.085194417779149229415314890830278i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01)",
    ),
    (
        r"iterate(x/100+x^2, x, pi/4, 0.3)",
        r"0.12804626658249998384122000701731|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01)",
    ),
    (
        r"iterate(sqrt(2)^x, x, -1, 1+1i, 2)",
        r"1 + 2.2661800709135969048138414728583i|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"iterate(sqrt(2)^x, x, -1/2, 0, 2)",
        r"-1.3372993732370925374604036522649|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 1/2, 0, 2)",
        r"0.62909522347838002831499607034698|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 1/3, 0, 2)",
        r"0.45895567076266692141804269772102|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 3/10-7i/10, 0, 2)",
        r"0.70036838734711260385542467983098 - 0.69904903024489781932572637926355i|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 5/2, -1, 2)",
        r"1.4387637902322512903405709598407|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 7/4, -1, 2)",
        r"1.1742329682334456693207660126783|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"iterate(sqrt(2)^x, x, i, -1, 2)",
        r"0.65281722669928791966283130632414 + 1.6836836259547999946560243895823i|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"iterate(sqrt(2)^x, x, pi/4, -1, 2)",
        r"0.50202089670085763375303356051982|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"iterate(x*e^(-x)/2, x, -1, -0.3)",
        r"-0.40156363678707259466266645379468",
    ),
    (
        r"iterate(x*e^(-x)/2, x, -1, 3)",
        r"-lambertw(-6)|  ~ -0.98161181345761008273444567350906 - 2.022627256933639750675703636623i",
    ),
    (
        r"iterate(x*e^(-x)/2, x, 1/2, -0.3)",
        r"-0.24965150348288953811424997384697|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
];

const ATTRACTING_3: &[(&str, &str)] = &[
    (
        r"iterate(x*e^(-x)/2, x, 1/3, -0.3)",
        r"-0.26618261139444340229603514664416|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x*e^(-x)/2, x, 3/10-7i/10, -0.3)",
        r"-0.26715811830128687664636687176729 - 0.071312399950070720627745909959604i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x*e^(-x)/2, x, 5/2, -0.3)",
        r"-0.094033085378445369247084928130593|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x*e^(-x)/2, x, 7/4, -0.3)",
        r"-0.14130523165087067720738196634738|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x*e^(-x)/2, x, i, -0.3)",
        r"-0.2979210611045035306311099184492 + 0.1056133610822349631113866709872i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x*e^(-x)/2, x, pi/4, -0.3)",
        r"-0.22221155003579952858612771722258|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (r"iterate(x/2+x^2/4, x, -1, -0.5)", r"-1 + i"),
    (
        r"iterate(x/2+x^2/4, x, -1, 1.5)",
        r"1.6457513110645905905016157536393",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/2, -0.5)",
        r"-0.29257788681812253039577530344414|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/3, -0.5)",
        r"-0.34451738410003298321660976057901|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 3/10-7i/10, -0.5)",
        r"-0.25443200673900307612099306973385 - 0.20329283031110297634575294151743i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 5/2, -0.5)",
        r"-0.058544960586121552727956231270914|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 7/4, -0.5)",
        r"-0.10280208074467024224877575796576|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, i, -0.5)",
        r"-0.21052217536441206194961826858724 + 0.32556065376450808029691777057877i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, pi/4, -0.5)",
        r"-0.22551309982442712105393666124989|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, -1/2, -2)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/2, -2)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/3, -2)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x/2+x^2/4, x, 3/10-7i/10, -2)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x/2+x^2/4, x, 5/2, -2)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x/2+x^2/4, x, 7/4, -2)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x/2+x^2/4, x, i, -2)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x/2+x^2/4, x, pi/4, -2)",
        r"0|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x/2+x^2/4, x, -12+3i, 1e-50)",
        r"-1.994729135987840606637301434808e-47 - 3.5774672149498732892553788433769e-47i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, -150, 1e-70+1e-71i)",
        r"1.4272476927059598810582857677829e-25 + 1.4272476927059598810582855620423e-26i",
    ),
    (
        r"iterate(x/2+x^2/4, x, 161/4, 1e-50)",
        r"7.6479083441310153196457165561989e-63|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, -1, -0.02+0.01i)",
        r"-0.04060718299079728369548360538921 + 0.020846518386856085253918342287509i",
    ),
    (
        r"iterate(x/2+x^2/4, x, -1/2, -0.02+0.01i)",
        r"-0.028461348305014131710917979470871 + 0.014384583690929884871409676558747i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/2, -0.02+0.01i)",
        r"-0.014079891127662574665832093454842 + 0.0069875895221398518349862699572864i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/3, -0.02+0.01i)",
        r"-0.015824763083798113907918250919372 + 0.0078707945314848264111094091939883i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 3/10-7i/10, -0.02+0.01i)",
        r"-0.018118986029810273096608617429853 - 0.00058123110164669317588117066388641i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 5/2, -0.02+0.01i)",
        r"-0.0034920029163349132615248081307725 + 0.0017102406837620234048974622066398i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 7/4, -0.02+0.01i)",
        r"-0.0058834751704929095491830640408832 + 0.0028900711075119222873129442080151i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, i, -0.02+0.01i)",
        r"-0.0086850920161646689710593486913618 + 0.020334310417317629224708657549444i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, pi/4, -0.02+0.01i)",
        r"-0.011530713832807676898040682071689 + 0.0057042022369207370454042480706577i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, -1/2, -1e-60)",
        r"-1.4142135623730950488016887242097e-60|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, -1/2, 1e-90i)",
        r"1.4142135623730950488016887242097e-90i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/2, 1e-90i)",
        r"7.0710678118654752440084436210485e-91i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/3, 1e-90i)",
        r"7.9370052598409973737585281963615e-91i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 3/10-7i/10, 1e-30)",
        r"7.185024507983607102669217817886e-31 + 3.7882473993021677573606274392436e-31i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 5/2, 1e-30)",
        r"1.7677669529663688110021109052636e-31|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 7/4, 1e-30)",
        r"2.9730177875068026667937499264033e-31|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, i, -1e-60)",
        r"-7.6923890136397212657832999366127e-61 + 6.389612763136348011500329114647e-61i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, pi/4, -1e-60)",
        r"-5.8019181037172444361518420439005e-61|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
];

const SLOPE_ONE_1: &[(&str, &str)] = &[
    (
        r"iterate(atan(x), x, 1/2, 1)",
        r"0.87431344165394532950964919591062|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(atan(x), x, 1/3, 1)",
        r"0.91104430516205404973972392291961|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(atan(x), x, 5/2, 3)",
        r"0.80098565778351971112684557067541|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/2, -1e-40)",
        r"-1e-40|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/2, -1e-80)",
        r"-1e-80|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/3, -1e-40)",
        r"-1e-40|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/3, -1e-80)",
        r"-1e-80|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 5/2, -1e-40)",
        r"-1e-40|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 5/2, -1e-80)",
        r"-1e-80|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, i, -1e-40)",
        r"-1e-40|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, i, -1e-80)",
        r"-1e-80|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/2, 1e-40)",
        r"1e-40|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/2, 1e-45i)",
        r"1e-45i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/2, 1e-80)",
        r"1e-80|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/3, 1e-45i)",
        r"1e-45i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/3, 1e-80)",
        r"1e-80|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 5/2, 1e-40)",
        r"1e-40|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 5/2, 1e-80)",
        r"1e-80|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, i, 1e-40)",
        r"1e-40|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, i, 1e-45i)",
        r"1e-45i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, -1, -0.5)",
        r"-0.69314718055994530941723212145818",
    ),
    (
        r"iterate(e^x-1, x, -1/2, -0.5)",
        r"-0.58002106526244255170088173174837|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/2, -0.5)",
        r"-0.44011342771640556675909016154872|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/3, -0.5)",
        r"-0.45834190396019757580615706813473|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 3/10-7i/10, -0.5)",
        r"-0.44776473790123669660317090130522 - 0.078551699951875578894463924058508i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 5/2, -0.5)",
        r"-0.29955303505342380981014455846999|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 7/4, -0.5)",
        r"-0.33994634961794840374859131175472|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, i, -0.5)",
        r"-0.46388941797477083902589511865534 + 0.1262249773722657250773518008272i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, pi/4, -0.5)",
        r"-0.41217029838226238258013949658359|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, -1, 0.5)",
        r"0.40546510810816438197801311546435",
    ),
    (
        r"iterate(e^x-1, x, -1/2, 0.5)",
        r"0.4479568589476737980468360477626|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/2, 0.5)",
        r"0.56511117365400814766230566975381|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/3, 0.5)",
        r"0.5416715069746572190909963261349|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 3/10-7i/10, 0.5)",
        r"0.52210139701731083883314490825549 - 0.090567468739110141214913825715132i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 5/2, 0.5)",
        r"1.137513842506579408454386377723|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 7/4, 0.5)",
        r"0.82970895348192472767670819512289|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, i, 0.5)",
        r"0.47532428884575767853907772008963 + 0.11034357396698710260012687967725i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, pi/4, 0.5)",
        r"0.61010671489207018370739429610316|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(ln(1+x), x, -1, 0.5+0.5i)",
        r"0.44688903658416915805158327605191 + 0.79043908321361491184326256704796i",
    ),
    (
        r"iterate(ln(1+x), x, -1/2, 0.5+0.5i)",
        r"0.4947598676451288609600242379574 + 0.63018494658340878809629431773051i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(ln(1+x), x, 1/2, 0.5+0.5i)",
        r"0.48376563691149530311101968475164 + 0.39898399345391630538693916743415i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(ln(1+x), x, 1/3, 0.5+0.5i)",
        r"0.49066845913495732591865889458825 + 0.42972051781428525467250932757404i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(ln(1+x), x, 3/10-7i/10, 0.5+0.5i)",
        r"0.36879705348236689500938706091407 + 0.44057515428126956374895859830694i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(ln(1+x), x, 5/2, 0.5+0.5i)",
        r"0.37392275126651493285411959084726 + 0.18173453207428054539938885420614i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(ln(1+x), x, 7/4, 0.5+0.5i)",
        r"0.41508240486461633221756255129485 + 0.23847983481986373532357623506469i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(ln(1+x), x, i, 0.5+0.5i)",
        r"0.75714150163481070344321041655355 + 0.40314879344093057851463864385983i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(ln(1+x), x, pi/4, 0.5+0.5i)",
        r"0.46978786153422462598480408735676 + 0.3523637234239292609088776117281i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(sin(x), x, 1/2, 0.5)",
        r"0.4893770882878611388411460637411|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(sin(x), x, 1/3, 2)",
        r"1.0400578377389705727950594851904|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1.141592654, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x), x, 5/2, 1)",
        r"0.70944499295284951172909937125521|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(sin(x), x, 1/2, pi)",
        r"0|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x), x, 1/3, pi)",
        r"0|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x), x, 5/2, pi)",
        r"0|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sinh(x), x, 1/2, 0.5)",
        r"0.51023103109429762712006159351208|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(sinh(x), x, 1/3, 0.5)",
        r"0.50675336419254675058612274002501|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(sinh(x), x, 5/2, 2)",
        r"295.18819116567695457998647444911|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(tan(x), x, 1/2, 0.3)",
        r"0.30456150004215605712961478444037|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(tan(x), x, 1/3, 0.3)",
        r"0.30301809750743742480881797513047|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(tan(x), x, 5/2, 0.5)",
        r"0.64768265222209180520690203866673|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, -1, -0.3)",
        r"-0.48940222718021496903623125199629",
    ),
];

const SLOPE_ONE_2: &[(&str, &str)] = &[
    (
        r"iterate(x*e^x, x, -1, -1)",
        r"lambertw(-1)|  ~ -0.31813150520476413531265425158766 + 1.3372357014306894089011621431937i",
    ),
    (
        r"iterate(x*e^x, x, 1/2, -0.3)",
        r"-0.25480753945788813478401727565872|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, 1/3, -0.3)",
        r"-0.26812380088472073094296184895802|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, 3/10-7i/10, -0.3)",
        r"-0.25709569031593042202441676134213 - 0.056904487871415791124770379004195i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, 5/2, -0.3)",
        r"-0.16209918508790904043832170913202|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, 7/4, -0.3)",
        r"-0.18718547789018046162612601920765|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, i, -0.3)",
        r"-0.26344137135629278707229570844074 + 0.092690750051310050418001273049623i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, pi/4, -0.3)",
        r"-0.23505503909395682729165233325555|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, -1, 0.5)",
        r"0.35173371124919582602490930092995",
    ),
    (
        r"iterate(x*e^x, x, -1/2, 0.5)",
        r"0.41386025725399728525468265138383|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, 1/2, 0.5)",
        r"0.62602395130210673371845533179246|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, 1/3, 0.5)",
        r"0.5780935350775984259537972518657|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, 3/10-7i/10, 0.5)",
        r"0.51983382769704553124815356620974 - 0.16833836564032511345116184410532i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, 5/2, 0.5)",
        r"3.7750824085148079995682103159179|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, 7/4, 0.5)",
        r"1.454522022787356393235154257614|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, i, 0.5)",
        r"0.43129541979261486086570529226584 + 0.18024934334657460011281888795029i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, pi/4, 0.5)",
        r"0.72706700160164698110474008762688|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+1/x, x, 1/2, 1)",
        r"1.6682712581427341026136524455363|note: iterate: regular iteration at infinity (slope 1 in 1/x); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points",
    ),
    (
        r"iterate(x+1/x, x, 1/3, 2)",
        r"2.1832527487514173981140186725136|note: iterate: regular iteration at infinity (slope 1 in 1/x)",
    ),
    (
        r"iterate(x+1/x, x, 5/2, 5)",
        r"5.4858434859512021627142930394466|note: iterate: regular iteration at infinity (slope 1 in 1/x)",
    ),
    (
        r"iterate(x+x^2, x, -1, 0.3+0.2i, 0)",
        r"0.25340394063741837301862947159129 + 0.132730922425750615769431896313i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, -1/2, 0.3+0.2i, 0)",
        r"0.27568202045188083207008467662557 + 0.16178959026833187855156948492076i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, 1/2, 0.3+0.2i, 0)",
        r"0.32550672533311736414142597477359 + 0.25099455253484325651201595176209i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, 1/3, 0.3+0.2i, 0)",
        r"0.3169688934202318524118347313448 + 0.23229219730122432814649935209978i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, 3/10-7i/10, 0.3+0.2i, 0)",
        r"0.3855173804934299103833237001849 + 0.17702322228435663733277288523434i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, 5/2, 0.3+0.2i, 0)",
        r"0.33250460265622878024655340888485 + 0.71977418683315589485651064841765i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, 7/4, 0.3+0.2i, 0)",
        r"0.37241696448288058911767048777305 + 0.47410040065016194833937648164659i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, i, 0.3+0.2i, 0)",
        r"0.21445325471109132243293171417422 + 0.22747926173826514703031905503578i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, pi/4, 0.3+0.2i, 0)",
        r"0.33984359186565742458848582889429 + 0.2877587127351794709309354361435i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, -1, -0.2+0.1i)",
        r"-0.21556775949710844636084603665766 + 0.1757887921270714642435286337632i",
    ),
    (
        r"iterate(x+x^2, x, -1, -0.8)",
        r"-0.5 + 0.74161984870956629487113974408007i",
    ),
    (
        r"iterate(x+x^2, x, 1/2, -0.2+0.1i)",
        r"-0.18486286574962956602754834111632 + 0.076507491477770457430665597443225i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, 1/3, -0.2+0.1i)",
        r"-0.18997970650338066343329393622947 + 0.083433458237213046649864346351797i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, 3/10-7i/10, -0.2+0.1i)",
        r"-0.22128643327845493210408596635562 + 0.054458078619740597413667516084242i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, 5/2, -0.2+0.1i)",
        r"-0.13436182668783330279784740095501 + 0.033123574229344308080568016709744i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, 7/4, -0.2+0.1i)",
        r"-0.15042232711300576305774460227789 + 0.043589460765294996525835080327932i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, i, -0.2+0.1i)",
        r"-0.15105955966958543006862326597225 + 0.11476516651112521491947750348409i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, pi/4, -0.2+0.1i)",
        r"-0.17624197670246269859482727826052 + 0.066389742772162437226761145415297i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^3, x, 1/2, 0.5, 0)",
        r"0.55380999398670223325009386810146|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^3, x, 1/3, 0.5, 0)",
        r"0.53428790012952728390446301385663|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^3, x, 5/2, 1, 0)",
        r"54.25420956619169761083706200752|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x-x^3, x, 1/2, 0.3)",
        r"0.28544121920919102020261858604235|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x-x^3, x, 1/3, 0.3)",
        r"0.2900286012427101364881650576726|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x-x^3, x, 5/2, 0.5)",
        r"0.30397656835870299421306493751598|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (r"iterate(x/(1-x), x, -1, -3)", r"3/2|  ~ 1.5"),
    (r"iterate(x/(1-x), x, -1/2, -3)", r"6"),
    (r"iterate(x/(1-x), x, 1/2, -3)", r"-6/5|  ~ -1.2"),
    (r"iterate(x/(1-x), x, 1/3, -3)", r"-3/2|  ~ -1.5"),
    (
        r"iterate(x/(1-x), x, 3/10-7i/10, -3)",
        r"-285/401 - 315i/401|  ~ -0.71072319201995012468827930174564 - 0.78553615960099750623441396508728i",
    ),
    (
        r"iterate(x/(1-x), x, 5/2, -3)",
        r"-6/17|  ~ -0.35294117647058823529411764705882",
    ),
    (r"iterate(x/(1-x), x, 7/4, -3)", r"-12/25|  ~ -0.48"),
    (
        r"iterate(x/(1-x), x, i, -3)",
        r"-3/10 + 9i/10|  ~ -0.3 + 0.9i",
    ),
    (
        r"iterate(x/(1-x), x, pi/4, -3)",
        r"12/(-4 - 3pi)|  ~ -0.89386953252165929405617846057282",
    ),
];

const SLOPE_ZERO_1: &[(&str, &str)] = &[
    (
        r"iterate(1-cos(x), x, -1/2, 0.5)",
        r"0.76443679451187867236155623868763|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(1-cos(x), x, -1/2, 1)",
        r"1.2858680858781937873736133041746|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(1-cos(x), x, 1/2, 1)",
        r"0.71891141386900833234193977823205|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(1-cos(x), x, 1/3, 1)",
        r"0.81124642778587376157689065746234|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(1-cos(x), x, 3/10-7i/10, 0.5i)",
        r"-0.84822048484678176321247570036561 + 0.70644934569083260547947176482026i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(1-cos(x), x, 5/2, 0.5i)",
        r"-0.00071377862677591918813130656950922 + 0.00042690211918627653753648964461064i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(1-cos(x), x, 7/4, 0.5i)",
        r"0.010555533425992040931938774768928 - 0.016428129102498986855365091066778i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(1-cos(x), x, i, 0.5)",
        r"0.41622908079947702674443880948624 - 0.53626724973085463940518539784862i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(1-cos(x), x, pi/4, 0.5)",
        r"0.18025256502239244142812361884976|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(2x^2-3x+4, x, -1/2, 1+2i)",
        r"1.2680539216614042126678188301253 + 1.3782736834864841136153947249637i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(2x^2-3x+4, x, -1/2, 3)",
        r"1.8846562020640812751204459088918|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(2x^2-3x+4, x, 1/2, 3)",
        r"5.4498893937649704729981418097605|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(2x^2-3x+4, x, 1/3, 3)",
        r"4.3634605389647311307373353215614|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(2x^2-3x+4, x, 3/10-7i/10, 10)",
        r"-0.54821192632938697557278096510173 - 12.048516777636358969593175379843i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(2x^2-3x+4, x, 5/2, 10)",
        r"7627062.7190746533470926928884667|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(2x^2-3x+4, x, 7/4, 10)",
        r"9338.4007815746259326367564786037|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(2x^2-3x+4, x, i, 1+2i)",
        r"0.52340814942431961038275335467868 + 1.1203356606566280877657188068506i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(2x^2-3x+4, x, pi/4, 1+2i)",
        r"-2.4445158896223995523211424254586 + 2.9354880179867168544514987822951i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(x^3*(1+x), x, -1/2, -0.3)",
        r"-0.046705031827866716596000045578128 + 0.45134313974801196011985051870695i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^3*(1+x), x, -1/2, 0.5)",
        r"0.60396843697847977939866651457416|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3)",
    ),
    (
        r"iterate(x^3*(1+x), x, 1/2, 0.2i)",
        r"-0.060453718313704710325667445142537 + 0.019731249450158496004365016084071i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^3*(1+x), x, 1/3, 0.2i)",
        r"-0.070019189452050198326965903649627 + 0.073378142663878885979298257457138i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^3*(1+x), x, 3/10-7i/10, 0.5)",
        r"0.46008880330138970338806149664297 + 0.22198698914764539820307809168671i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3)",
    ),
    (
        r"iterate(x^3*(1+x), x, 5/2, 0.5)",
        r"0.0002257332184590923262961759320989|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3)",
    ),
    (
        r"iterate(x^3*(1+x), x, 7/4, 0.2i)",
        r"0.0000032865064237884225466583952011299 - 0.000017065640253313839156942913509969i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^3*(1+x), x, i, 0.2i)",
        r"0.086157184507755650219975848928973 - 0.067230350324035827347784476981697i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^3*(1+x), x, pi/4, -0.3)",
        r"0.017602540848232693360505893570222 + 0.039251662302405553030446863950162i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(5x^4+x^6, x, 1/2, 0.2+0.1i)",
        r"0.051210198911257065686801578926558 + 0.068797915439657185678860982736895i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 4)",
    ),
    (
        r"iterate(5x^4+x^6, x, 1/2, 0.3)",
        r"0.15509711122008106839124153690371|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 4)",
    ),
    (
        r"iterate(5x^4+x^6, x, 1/3, 0.2+0.1i)",
        r"0.094236869158045105486227782625498 + 0.085760422827619995487104587765304i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 4)",
    ),
    (
        r"iterate(5x^4+x^6, x, 1/3, 0.3)",
        r"0.20371911097299359076641711137716|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 4)",
    ),
    (
        r"iterate(5x^4+x^6, x, 3/10-7i/10, 0.2+0.1i)",
        r"-0.013448238223871734877672899826264 + 0.46482005252366376753031052965484i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 4)",
    ),
    (
        r"iterate(5x^4+x^6, x, 3/10-7i/10, 0.3)",
        r"0.2253091532293896224663201249152 + 0.24325806366097032289650074433262i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 4)",
    ),
    (
        r"iterate(5x^4+x^6, x, 5/2, 0.2+0.1i)",
        r"-1.8479075935552114030050970692137e-14 + 1.9320617923409421883704032034477e-14i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 4)",
    ),
    (
        r"iterate(5x^4+x^6, x, 5/2, 0.3)",
        r"3.5714753783656598416976963772893e-10|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 4)",
    ),
    (
        r"iterate(5x^4+x^6, x, i, 0.2+0.1i)",
        r"0.20410690640925752246286307821322 - 0.23392608032027248744424571217895i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 4)",
    ),
    (
        r"iterate((x-1)^2*(x+1)+1, x, -1/2, 0.8)",
        r"0.84919621427957460294445358425339 + 0.21705473950756636699141481456576i|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate((x-1)^2*(x+1)+1, x, -1/2, 1.3)",
        r"1.3325701671068407589361859726259|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2)",
    ),
    (
        r"iterate((x-1)^2*(x+1)+1, x, 1/2, 1.3)",
        r"1.2579890623720207526913875437302|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2)",
    ),
    (
        r"iterate((x-1)^2*(x+1)+1, x, 1/3, 1.1+0.1i)",
        r"1.0558165752432394450872780200282 + 0.087397414671867809776170311091645i|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2)",
    ),
    (
        r"iterate((x-1)^2*(x+1)+1, x, 3/10-7i/10, 1.3)",
        r"1.2857385918604840364815412677173 + 0.06035240979540835076717904626944i|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2)",
    ),
    (
        r"iterate((x-1)^2*(x+1)+1, x, 5/2, 1.1+0.1i)",
        r"0.99995234785299112871101178375775 - 0.00045117339462211816065041375065417i|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2)",
    ),
    (
        r"iterate((x-1)^2*(x+1)+1, x, 7/4, 1.1+0.1i)",
        r"0.99284572159642224841084127421108 + 0.0030499416580589747181113648796868i|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2)",
    ),
    (
        r"iterate((x-1)^2*(x+1)+1, x, i, 0.8)",
        r"0.99282065338022074636768894708039 + 0.031363151054021797242945966317264i|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate((x-1)^2*(x+1)+1, x, pi/4, 0.8)",
        r"1.0622556663856220254671370423283 - 0.070731279928528150847906652975191i|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate((x^3+1)/(x+1), x, 1/2, 3)",
        r"4.2301872484651660274871390338539|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate((x^3+1)/(x+1), x, 1/2, 5+1i)",
        r"8.8287391542496903559947722156576 + 2.6363908979692969534631530175469i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate((x^3+1)/(x+1), x, 1/3, 3)",
        r"3.7173281230190852624361157509564|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate((x^3+1)/(x+1), x, 1/3, 5+1i)",
        r"7.1326250559304992185402449156797 + 1.8617850156561422938876454270695i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate((x^3+1)/(x+1), x, 5/2, 3)",
        r"201.37730691531903699043150260214|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate((x^3+1)/(x+1), x, 5/2, 5+1i)",
        r"1999.3633204044858506422099342158 + 5504.150611814026289794057296742i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate((x^3+1)/(x+1), x, i, 3)",
        r"2.1506030232997679621559493694972 + 1.2014601189554724996175821951902i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate((x^3+1)/(x+1), x, i, 5+1i)",
        r"1.6523083839541074995404431161784 + 2.6213395536913440641081794365161i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(sin(x)^2, x, -1/2, 0.3-0.2i)",
        r"0.44957996302270350848478815316206 - 0.21013053116526576013512834347926i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(sin(x)^2, x, -1/2, 1)",
        r"1.17439880587634027112231727012|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(sin(x)^2, x, 1/2, 1)",
        r"0.85092855090637304353596213656084|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(sin(x)^2, x, 1/3, 1)",
        r"0.89939037808039918761974970168389|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(sin(x)^2, x, 3/10-7i/10, 0.5)",
        r"0.40962836978633639548037577012965 + 0.20072459260130117421537202538714i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(sin(x)^2, x, 5/2, 0.5)",
        r"0.015233221423993482278348805579231|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(sin(x)^2, x, 7/4, 0.5)",
        r"0.083171410709112027627454010220739|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
];

const SLOPE_ZERO_2: &[(&str, &str)] = &[
    (
        r"iterate(sin(x)^2, x, i, 0.3-0.2i)",
        r"0.26572063271589078421675972925214 - 0.56169441144729357243056803089983i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(sin(x)^2, x, pi/4, 0.3-0.2i)",
        r"0.094398457093394265084887518180123 - 0.14119957689490940769984710276315i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(sin(x)^2, x, -1/2, pi)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)^2, x, 1/2, pi)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)^2, x, 1/3, pi)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)^2, x, 3/10-7i/10, pi)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)^2, x, 5/2, pi)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)^2, x, 7/4, pi)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)^2, x, i, pi)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(sin(x)^2, x, pi/4, pi)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x^2*e^x, x, -1/2, -0.5)",
        r"-0.26118693992279192295801364515875 + 0.55489126375621754788020910298947i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2*e^x, x, -1/2, 0.3)",
        r"0.3787105425109698919351595984319|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2*e^x, x, 1/2, 0.3)",
        r"0.20945305014859570120002068167004|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2*e^x, x, 1/3, 0.3)",
        r"0.2402222043949859723056096140582|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2*e^x, x, 3/10-7i/10, 0.3)",
        r"0.25250878317646480119318088133795 + 0.13588600256404702818697082668416i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2*e^x, x, 5/2, 0.3)",
        r"0.0030886412874215808728505686650489|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2*e^x, x, 7/4, 0.3)",
        r"0.031682102345405218377286610140481|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2*e^x, x, i, -0.5)",
        r"-0.015919763727151867070179118860886 + 0.065836640401320954000403972409444i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2*e^x, x, pi/4, -0.5)",
        r"0.13933527120589362180615361149936 - 0.13991442815783447850732926242006i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2+1, x, -1/2, -1+0.5i)",
        r"-0.57901815110490703779828329159843 + 1.47061967027272319817851167334i|note: iterate: Boettcher's coordinate at infinity (degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2+1, x, 1/2, -1+0.5i)",
        r"-0.82746019528410814037092912757101 - 1.7030309649196404105606557614921i|note: iterate: Boettcher's coordinate at infinity (degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2+1, x, 1/2, 3+1i)",
        r"4.8526154885123284929732876377447 + 2.2034483465856505505024348563197i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(x^2+1, x, 1/3, 1.5i)",
        r"-0.51425009472579090892685405035582 + 1.5576039041204560248667550034243i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(x^2+1, x, 3/10-7i/10, 1)",
        r"1.1277861365556399645192798326452 - 0.5387171459237539313470470334136i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(x^2+1, x, 5/2, 0.1)",
        r"3.0313408364236822318988253303766|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(x^2+1, x, 7/4, -2)",
        r"-6.4246668899908459815689929495339 - 14.120600941744163712100179455166i|note: iterate: Boettcher's coordinate at infinity (degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2+1, x, 7/4, 3+1i)",
        r"30.525449973545532573224006631579 + 45.750799528001188839100477137093i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(x^2+1, x, pi/4, -2)",
        r"2.5562813192650326749359192194877 - 3.2048628085749128118020890227628i|note: iterate: Boettcher's coordinate at infinity (degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2+x^3, x, -1/2, -0.2)",
        r"-0.15615130363083920349116773417063 + 0.27656193825404672534480752976926i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2+x^3, x, -1/2, 0.3)",
        r"0.38528934049813540634953258350987|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2+x^3, x, 1/2, 0.3)",
        r"0.20564326010592153148365183708393|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2+x^3, x, 1/3, 0.1+0.1i)",
        r"0.046590707061396390948563510622596 + 0.075172987177286941579197729736604i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2+x^3, x, 3/10-7i/10, 0.1+0.1i)",
        r"-0.069416507700690375006413159571547 + 0.19725383039031129630540847608117i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2+x^3, x, 5/2, 0.1)",
        r"0.000002930744935604804233612642159605|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2+x^3, x, 7/4, 0.1)",
        r"0.00051278401397289236595890414135099|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2+x^3, x, i, -0.2)",
        r"0.0094100682941633703823861960050857 + 0.034543058217453373790054063209606i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2+x^3, x, pi/4, -0.2)",
        r"0.03391334988852760309915982820988 - 0.038537507293176558705531372164919i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2+x^3, x, -1/2, -1)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x^2+x^3, x, 1/2, -1)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x^2+x^3, x, 1/3, -1)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x^2+x^3, x, 3/10-7i/10, -1)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x^2+x^3, x, 5/2, -1)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x^2+x^3, x, 7/4, -1)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x^2+x^3, x, i, -1)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x^2+x^3, x, pi/4, -1)",
        r"0|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0, which the map takes to the same points",
    ),
    (
        r"iterate(x^3+2x, x, -1/2, 0.1)",
        r"0.070647783487991269869498806750988|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"iterate(x^3+2x, x, -1/2, 2)",
        r"1.1877165266939135981944935724991|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"iterate(x^3+2x, x, 1/2, 2)",
        r"4.0509097767086912785577635884455|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"iterate(x^3+2x, x, 1/3, 1+1i)",
        r"1.13667593269491561970871294235 + 1.5424619104660254185249254591358i|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"iterate(x^3+2x, x, 3/10-7i/10, 1+1i)",
        r"2.3261470310041375552907695537538 + 0.25870435288039559354616637306882i|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"iterate(x^3+2x, x, 5/2, 0.1)",
        r"0.59570446519097634771705335770835|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"iterate(x^3+2x, x, 7/4, 0.1)",
        r"0.34216507665430226464268640627558|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"iterate(x^3+2x, x, 7/4, 2)",
        r"291.44031066173104894875172396794|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"iterate(x^3+2x, x, pi/4, 0.1)",
        r"0.17291483258955779449295941025572|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"iterate(x^4-x, x, 1/2, -3)",
        r"9.1681251655045808658967960962681|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
    (
        r"iterate(x^4-x, x, 1/2, 1.5)",
        r"1.9514934023886866467247024559755|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
    (
        r"iterate(x^4-x, x, 1/3, -3)",
        r"1.5670887225665347511214135612223 - 5.5817184805335712593369680146913i|note: iterate: Boettcher's coordinate at infinity (degree 4); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^4-x, x, 1/3, 1.5)",
        r"1.7412684789656265221790832510389|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
    (
        r"iterate(x^4-x, x, 3/10-7i/10, -3)",
        r"34.506356807242910319972201063686 + 126.69484528456602436835242702355i|note: iterate: Boettcher's coordinate at infinity (degree 4); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^4-x, x, 3/10-7i/10, 1.5)",
        r"1.3136552061771012422694882892216 - 0.4116415868746450000697962443164i|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
];

const SLOPE_ZERO_3: &[(&str, &str)] = &[
    (
        r"iterate(x^4-x, x, 5/2, -3)",
        r"2478750546850703.9999999899572282|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
    (
        r"iterate(x^4-x, x, 5/2, 1.5)",
        r"24809.174081736587725589027582093|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
    (
        r"iterate(x^4-x, x, i, 1.5)",
        r"1.1602082013320844625090464026614 + 0.2171260972860886102886259146475i|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
];

const REPELLING_1: &[(&str, &str)] = &[
    (
        r"iterate(-2sinh(x), x, -1, 0.3)",
        r"-0.14944312018495765616028580915059",
    ),
    (
        r"iterate(-2sinh(x), x, -1/2, 0.3)",
        r"-0.21056149065142354694971215489377i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2sinh(x), x, 1/2, 0.3)",
        r"0.41801804434381371521184925079609i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2sinh(x), x, 1/3, 0.3)",
        r"0.18510695094395793278246670497878 + 0.3257027073450097220591809070246i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2sinh(x), x, 3/10-7i/10, 0.3)",
        r"2.3307726189888172478474022862559 + 4.0135252552595668841837901357535i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2sinh(x), x, 5/2, 0.3)",
        r"1.4511916932662031153359461426954i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2sinh(x), x, 7/4, 0.3)",
        r"0.66855372204045163052684072616368 - 0.74796665869020920192835083048208i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2sinh(x), x, i, 0.3)",
        r"0.0099231730914794698940866130256255 + 0.0082427456162304547440265942107414i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2sinh(x), x, pi/4, 0.3)",
        r"-0.39855596545090049789686325759694 + 0.32798033910258305082451500231324i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(2sinh(x), x, -1, 0.3)",
        r"0.14944312018495765616028580915059",
    ),
    (
        r"iterate(2sinh(x), x, -1/2, 0.3)",
        r"0.21160649588157429413911768035283|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 1/2, 0.3)",
        r"0.42637846034101539326131404596098|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 1/3, 0.3)",
        r"0.37908103998368813224138393679751|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 3/10-7i/10, 0.3)",
        r"0.32540283617858726258700148975055 - 0.17415549392196250148958592619479i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 5/2, 0.3)",
        r"1.9928153387509433918370142065853|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 7/4, 0.3)",
        r"1.0628703974367935241301635244581|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, i, 0.3)",
        r"0.22890532769720503826331137634723 + 0.19202936435132599319152813889119i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, pi/4, 0.3)",
        r"0.52216782356187169328464211938802|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, -12+3i, 0.3)",
        r"-0.000035492145952531634362562522555388 + 0.000063653749432728133795352935938613i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, -12+3i, 1e-50)",
        r"-1.188951215736770991466821095233e-54 + 2.1323366254269321496816747447114e-54i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, -61/2, 0.3)",
        r"1.9658634852587340759098996213047e-10|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, -61/2, 1e-50)",
        r"6.5854450798271924667139012562563e-60|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 161/4, 1e-50)",
        r"1.3075470507794426707358454897762e-38|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 1/2, -1e-80+1e-80i)",
        r"-1.4142135623730950488016887242097e-80 + 1.4142135623730950488016887242097e-80i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 1/2, 1e-50)",
        r"1.4142135623730950488016887242097e-50|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 1/3, -1e-80+1e-80i)",
        r"-1.2599210498948731647672106072782e-80 + 1.2599210498948731647672106072782e-80i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 1/3, 1e-50)",
        r"1.2599210498948731647672106072782e-50|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 3/10-7i/10, -1e-80+1e-80i)",
        r"-5.1485513363717485548405011536966e-81 + 1.6632370018696201990047285247294e-80i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 3/10-7i/10, 1e-50)",
        r"1.0890460677533975272443893200495e-50 - 5.7419093411622267176033920467986e-51i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 5/2, -1e-80+1e-80i)",
        r"-5.6568542494923801952067548968388e-80 + 5.6568542494923801952067548968388e-80i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 5/2, 1e-50)",
        r"5.6568542494923801952067548968388e-50|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, i, -1e-80+1e-80i)",
        r"-1.408200177677606927728362905126e-80 + 1.3027762505033732542829708219657e-81i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2x+e^x-1, x, 1/2, -1e-70)",
        r"-1.7320508075688772935274463415059e-70|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(2x+e^x-1, x, 1/2, 1e-50)",
        r"1.7320508075688772935274463415059e-50|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(2x+e^x-1, x, 1/3, -1e-70)",
        r"-1.4422495703074083823216383107801e-70|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(2x+e^x-1, x, 1/3, 1e-50)",
        r"1.4422495703074083823216383107801e-50|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(2x+e^x-1, x, 3/10-7i/10, -1e-70)",
        r"-9.9911496417455156649718222548004e-71 + 9.6692881500876127905050708581122e-71i|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(2x+e^x-1, x, 3/10-7i/10, 1e-50)",
        r"9.9911496417455156649718222548004e-51 - 9.6692881500876127905050708581122e-51i|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(2x+e^x-1, x, 5/2, -1e-70)",
        r"-1.5588457268119895641747017073553e-69|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(2x+e^x-1, x, 5/2, 1e-50)",
        r"1.5588457268119895641747017073553e-49|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(2x+e^x-1, x, i, -1e-70)",
        r"-4.5483242282660975502756514359504e-71 - 8.9057704166774705907492730656518e-71i|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(2x+sin(x), x, -1, 0.3)",
        r"0.1000556204594156281886430753631|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(2x+sin(x), x, 1/3, 0.3)",
        r"0.43238281890391873581940578666536|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(2x+sin(x), x, 7/4, 0.3)",
        r"1.9945200685902459197429378865983|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(e^x, x, 1/2, 0.5, 0.3+1.3i)",
        r"0.99303919280011931894522795776518 + 0.13114284571242883021571603863683i|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i)",
    ),
    (
        r"iterate(e^x, x, 1/2, 1+1i, 0.3+1.3i)",
        r"1.3284294251967888835996268615194 + 1.4200780699102662940971979683309i|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i)",
    ),
    (
        r"iterate(e^x, x, 1/3, 0.5, 0.3+1.3i)",
        r"0.8677077115690522494470005036664 + 0.10271583504752513302494312381968i|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i)",
    ),
    (
        r"iterate(e^x, x, 1/3, 2, 0.3+1.3i)",
        r"2.6351662638411502380291881269236 + 0.29219373617268309639712152213788i|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i)",
    ),
    (
        r"iterate(e^x, x, 3/10-7i/10, 0.5, 0.3+1.3i)",
        r"1.2445059006427093693306193168293 - 0.27024440092273911030412745819953i|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(e^x, x, 3/10-7i/10, 2, 0.3+1.3i)",
        r"2.7182761273312311726510234991575 + 0.0000018369575238977325043253391015039i|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(e^x, x, 5/2, 1+1i, 0.3+1.3i)",
        r"-1.4639622638031581782177343726898 - 0.98173238981176034379934069473398i|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(e^x, x, 5/2, 2, 0.3+1.3i)",
        r"136612709.67027871295499474326561 - 96519611.008486226898941667785552i|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(e^x, x, i, 1+1i, 0.3+1.3i)",
        r"0.55163086971260542675500709200536 + 1.3327882283672447315370416684691i|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i)",
    ),
    (
        r"iterate(-2x+x^3/1000, x, -1, 0.3, 0)",
        r"-0.1500016875569556880227814276088|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2x+x^3/1000, x, -1/2, 0.3, 0)",
        r"-0.21213680752005211053299226629508i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2x+x^3/1000, x, 1/2, 0.3, 0)",
        r"0.42428316162604189049962959192522i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2x+x^3/1000, x, 1/3, 0.3, 0)",
        r"0.18899999287796621923945502672772 + 0.32734200091328606727605072495923i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2x+x^3/1000, x, 3/10-7i/10, 0.3, 0)",
        r"2.9870363176779842615453090782927 + 1.4643694573038120281401230835669i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2x+x^3/1000, x, 5/2, 0.3, 0)",
        r"1.6978965899400659960725921179243i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2x+x^3/1000, x, 7/4, 0.3, 0)",
        r"0.71365605445265056450983998478793 - 0.71341386538807851501928714647183i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
];

const REPELLING_2: &[(&str, &str)] = &[
    (
        r"iterate(-2x+x^3/1000, x, i, 0.3, 0)",
        r"0.0099726978731228810123073989695389 + 0.0082837300518754805799412932732805i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2x+x^3/1000, x, pi/4, 0.3, 0)",
        r"-0.40395764813122602297844675038596 + 0.32277354526920771336708419398562i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(sqrt(2)^x, x, -1, 10, 4)",
        r"6.6438561897747246957406388589788|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"iterate(sqrt(2)^x, x, -1/2, 10, 4)",
        r"7.8152959275784265424981919432039|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 1/2, 10, 4)",
        r"15.007876581580662218836627251525|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 1/3, 10, 4)",
        r"12.777182816284014984688300465627|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 3/10-7i/10, 10, 4)",
        r"8.8090613317946423347480567304732 - 4.9963616065476238757502116941367i|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 5/2, 10, 4)",
        r"2092201254007825209781225321.7645|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 7/4, 10, 4)",
        r"1229.9706127536062362066882514668|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"iterate(sqrt(2)^x, x, i, 10, 4)",
        r"6.823247442146282846843337676011 + 3.729878314695555582059173932407i|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"iterate(sqrt(2)^x, x, pi/4, 10, 4)",
        r"21.655497395099971253129074317121|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"iterate(x^2+1, x, 1/2, 0.5+0.5i, 0.5+0.9i)",
        r"0.72866142145711038169196581584807 + 0.45090655555417975964869528978036i|note: iterate: regular iteration at the repelling fixed point 0.5 + 0.8660254038i (slope 1 + 1.732050808i)",
    ),
    (
        r"iterate(x^2+1, x, 1/2, 1, 0.5+0.9i)",
        r"1.4063485809471133836658484209548 - 0.32290831034360355928710016190858i|note: iterate: regular iteration at the repelling fixed point 0.5 + 0.8660254038i (slope 1 + 1.732050808i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2+1, x, 1/3, 0.5+0.5i, 0.5+0.9i)",
        r"0.64634223901370776534187418008775 + 0.45586650231615405749362396588441i|note: iterate: regular iteration at the repelling fixed point 0.5 + 0.8660254038i (slope 1 + 1.732050808i)",
    ),
    (
        r"iterate(x^2+1, x, 1/3, 2, 0.5+0.9i)",
        r"2.2385307172026957897025595914675 - 0.47681162130697328976628178745515i|note: iterate: regular iteration at the repelling fixed point 0.5 + 0.8660254038i (slope 1 + 1.732050808i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2+1, x, 3/10-7i/10, 0.5+0.5i, 0.5+0.9i)",
        r"0.54002383416793355763074151153513 - 0.026518138583822079472022344815212i|note: iterate: regular iteration at the repelling fixed point 0.5 + 0.8660254038i (slope 1 + 1.732050808i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2+1, x, 3/10-7i/10, 2, 0.5+0.9i)",
        r"40.338872009289686488188117726589 - 72.019926916015225163472854584535i|note: iterate: regular iteration at the repelling fixed point 0.5 + 0.8660254038i (slope 1 + 1.732050808i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2+1, x, 5/2, 1, 0.5+0.9i)",
        r"8.4323639291039188923230568751048 - 5.2197587416125063508318812991527i|note: iterate: regular iteration at the repelling fixed point 0.5 + 0.8660254038i (slope 1 + 1.732050808i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2+1, x, 5/2, 2, 0.5+0.9i)",
        r"44.858880112212705040623344658216 - 88.029810662796322789320279822745i|note: iterate: regular iteration at the repelling fixed point 0.5 + 0.8660254038i (slope 1 + 1.732050808i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2+1, x, i, 1, 0.5+0.9i)",
        r"0.9274861227368351425954649245328 + 0.55030690128651829302326060925444i|note: iterate: regular iteration at the repelling fixed point 0.5 + 0.8660254038i (slope 1 + 1.732050808i)",
    ),
    (
        r"iterate(x^2-1, x, 1/2, -0.3)",
        r"-0.88159462984144529854997076792026|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8366600265",
    ),
    (
        r"iterate(x^2-1, x, 1/2, -1.3)",
        r"-0.65324844569828880496882582544264 - 0.79563373382500255003186012077159i|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.5477225575i; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2-1, x, 1/3, -0.3)",
        r"-0.72075788859794129777751303149648|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8366600265",
    ),
    (
        r"iterate(x^2-1, x, 1/3, 0.2)",
        r"-0.23683586828672368615760190759971|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 3/10-7i/10, -0.5)",
        r"-1.2969492690974837902303069229595 + 0.96358234031770417143263909950157i|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.7071067812",
    ),
    (
        r"iterate(x^2-1, x, 3/10-7i/10, 0.3i)",
        r"-0.36106551508262571822701581934256 + 1.6551967836351092903674530608988i|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 5/2, -0.9)",
        r"-0.95661560651729240644188233553759|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.316227766; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2-1, x, 5/2, 0.5)",
        r"-0.024807516253109876045782650752092|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2-1, x, i, -1.3)",
        r"-1.7250666916179907344022394399216 - 0.094020402324717813290358759193277i|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.5477225575i",
    ),
    (
        r"iterate(x^2-1, x, 1/2, -0.3, -0.6)",
        r"-0.67429403750566814940818095567885 + 0.41174675597706056375806695302735i|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 1/2, -0.5, -0.6)",
        r"-0.62410888599436045092156264299588 + 0.13022922262907587299471778314203i|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 1/3, -0.3, -0.6)",
        r"-0.41595292536347522149809999398709 + 0.33469425681477125607323107731086i|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 1/3, -0.8, -0.6)",
        r"-0.7484783578441068322553430653032 - 0.17394989076603078325335955503345i|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 3/10-7i/10, -0.3, -0.6)",
        r"0.00074335218147069074326743642829766 + 0.0011904951784007023548052231904513i|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2-1, x, 3/10-7i/10, -0.8, -0.6)",
        r"-1.0012162352594955917644032789983 - 0.004357337629751345560128420613458i|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2-1, x, 5/2, -0.5, -0.6)",
        r"-0.63273326521470665984141215586839 + 0.20398882248594895723224855475134i|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 5/2, -0.8, -0.6)",
        r"-0.67211140235010270986293568752664 - 0.40305209951011380338186624711621i|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"iterate(x^2-1, x, i, -0.5, -0.6)",
        r"-0.61316702238884107353022028675862 + 0.0010490739320553862293837367166839i|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 1/2, (1+sqrt(5))/2+1e-60i, 1.6)",
        r"1.6180339887498948482045868343656|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 1/2, (1+sqrt(5))/2-1e-40, 1.6)",
        r"1.6180339887498948482045868343656|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 1/3, (1+sqrt(5))/2+1e-60i, 1.6)",
        r"1.6180339887498948482045868343656|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 1/3, (1+sqrt(5))/2-1e-40, 1.6)",
        r"1.6180339887498948482045868343656|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 3/10-7i/10, (1+sqrt(5))/2+1e-60i, 1.6)",
        r"1.6180339887498948482045868343656|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 3/10-7i/10, (1+sqrt(5))/2-1e-40, 1.6)",
        r"1.6180339887498948482045868343656|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 5/2, (1+sqrt(5))/2+1e-60i, 1.6)",
        r"1.6180339887498948482045868343656|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 5/2, (1+sqrt(5))/2-1e-40, 1.6)",
        r"1.6180339887498948482045868343656|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, i, (1+sqrt(5))/2+1e-60i, 1.6)",
        r"1.6180339887498948482045868343656|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 1/2, -0.2+0.1i, 1.6)",
        r"-0.82165286371088915082536931195652 + 0.070901750218799755941232415999468i|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8961656585 + 0.055793256i",
    ),
    (
        r"iterate(x^2-1, x, 1/2, -0.5, 1.6)",
        r"-0.97401346977127220850461797148625|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.7071067812",
    ),
    (
        r"iterate(x^2-1, x, 1/3, -0.2+0.1i, 1.6)",
        r"-0.63832320284934980479372270901709 + 0.088974058254286823916896061361772i|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8961656585 + 0.055793256i",
    ),
    (
        r"iterate(x^2-1, x, 1/3, -0.5, 1.6)",
        r"-0.86871320329426685467885259544847|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.7071067812",
    ),
    (
        r"iterate(x^2-1, x, 3/10-7i/10, -0.2+0.1i, 1.6)",
        r"-0.7725301925948970020105471718993 + 1.2927280275971942811078418118232i|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8961656585 + 0.055793256i",
    ),
    (
        r"iterate(x^2-1, x, 3/10-7i/10, -0.8, 1.6)",
        r"-1.8740120759169639297625636319403 + 0.47820417897824687730138321892474i|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.4472135955",
    ),
    (
        r"iterate(x^2-1, x, 5/2, -0.3, 1.6)",
        r"-0.95036421102971359663050078780649|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8366600265; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2-1, x, 5/2, -0.8, 1.6)",
        r"-0.99617900613557390903637496660637|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.4472135955; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2-1, x, i, -0.3, 1.6)",
        r"-0.084211459074072225530730967889916 - 1.7763476002604018252502313808144i|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8366600265",
    ),
    (
        r"iterate(x^3+2x, x, -1, 0.1, 0)",
        r"0.049937733209797481227325461932619|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(x^3+2x, x, -1/2, 0.1, 0)",
        r"0.07065195780014993629205699672095|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(x^3+2x, x, 1/2, 0.1, 0)",
        r"0.14165658891736043718794267151236|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(x^3+2x, x, 1/3, 0.1, 0)",
        r"0.12611515512446723592876477948761|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
];

const REPELLING_3: &[(&str, &str)] = &[
    (
        r"iterate(x^3+2x, x, 3/10-7i/10, 0.1, 0)",
        r"0.10875903575046833069982577779599 - 0.057631496312643460466229180412399i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(x^3+2x, x, 5/2, 0.1, 0)",
        r"0.59574337895297592400785289147646|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(x^3+2x, x, 7/4, 0.1, 0)",
        r"0.34218277872229683102294059059487|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(x^3+2x, x, i, 0.1, 0)",
        r"0.076715274989574360759394017690508 + 0.063934848902761097942567638774149i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(x^3+2x, x, pi/4, 0.1, 0)",
        r"0.17292231460251547241815709104674|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
];

const ROTATIONS: &[(&str, &str)] = &[
    (
        r"iterate(e^(i*sqrt(2))*sin(x), x, 1/2, 0.05+0.02i)",
        r"0.025012351423487597973248984537439 + 0.047671136363723159589290695432705i|note: iterate: regular iteration at the fixed point 0 (slope 0.1559436948 + 0.987765946i of size 1)",
    ),
    (
        r"iterate(e^(i*sqrt(2))*sin(x), x, 1/3, 0.05+0.02i)",
        r"0.035454394277324932215605463378254 + 0.040518080623336997911447570021994i|note: iterate: regular iteration at the fixed point 0 (slope 0.1559436948 + 0.987765946i of size 1)",
    ),
    (
        r"iterate(e^(i*sqrt(2))*sin(x), x, 5/2, 0.08)",
        r"-0.073845428901879734117559979699198 - 0.030726126897880780449494468181863i|note: iterate: regular iteration at the fixed point 0 (slope 0.1559436948 + 0.987765946i of size 1)",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, -1, 0.02i)",
        r"0.016448180775607652062905168693498 + 0.010745546831400637239263865693498i",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, -1, 0.03-0.04i)",
        r"-0.017774609053489040422265060225825 - 0.049614718210570159129686673604241i",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, -1/2, 0.02i)",
        r"0.009463663589072936490872913927952 + 0.017395261419689081878948622090717i|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, -1/2, 0.05)",
        r"0.044612027665126600120701593178508 - 0.022895995617875812235790048242823i|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, 1/2, 0.03-0.04i)",
        r"0.044395758032186647887435611044786 - 0.021419533845209898927881039194207i|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, 1/3, 0.02i)",
        r"-0.0066171808786297311692573914853638 + 0.019014150592930036613472819770075i|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, 1/3, 0.05)",
        r"0.047736034975533191900520373515079 + 0.015656944036729600746253483076116i|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, 3/10-7i/10, 0.03-0.04i)",
        r"0.077656743448905898448306593133798 - 0.054898496162422342354019767541447i|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, 5/2, 0.02i)",
        r"-0.011448728354358331907448952124888 - 0.016627689173621549778383589328454i|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, 5/2, 0.05)",
        r"-0.043330865061718179570377397957897 + 0.034160466745376520819045967367011i|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, 7/4, 0.03-0.04i)",
        r"0.036882364664253298318067945726971 + 0.033851849320690948944068854354998i|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, i, 0.02i)",
        r"-0.000095122331316125426610753872964075 + 0.0073497120759117564466588935226284i|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, i, 0.05)",
        r"0.019013760681720398920933205102702 + 0.000070532136771597941087209294487711i|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, pi/4, 0.03-0.04i)",
        r"0.048374865385304365067730146788556 - 0.0087334839811628887668292310765713i|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"iterate(e^(pi*i*(sqrt(5)-1))*x+x^2, x, 1/2, -0.03+0.02i)",
        r"0.0083570259136520319137130482945311 + 0.03568140731475279981251150639536i|note: iterate: regular iteration at the fixed point 0 (slope -0.7373688781 - 0.6754902943i of size 1)",
    ),
    (
        r"iterate(e^(pi*i*(sqrt(5)-1))*x+x^2, x, 1/3, -0.03+0.02i)",
        r"-0.0064090050467414479285137942432856 + 0.035939529773859918387420378635061i|note: iterate: regular iteration at the fixed point 0 (slope -0.7373688781 - 0.6754902943i of size 1)",
    ),
    (
        r"iterate(e^(pi*i*(sqrt(5)-1))*x+x^2, x, 3/10-7i/10, 0.05)",
        r"0.0070240434549253221147119269365372 - 0.0059173082934328612424574811231676i|note: iterate: regular iteration at the fixed point 0 (slope -0.7373688781 - 0.6754902943i of size 1)",
    ),
];

const CLOSED_FORMS_AGAINST_COORDINATES_1: &[(&str, &str)] = &[
    (r"iterate((1+i)*x+2, x, -1, 0.3)", r"-0.85 + 0.85i"),
    (
        r"iterate((1+i)*x+2, x, -1/2, 0.3)",
        r"-0.41052815680107702863381959445874 + 0.34968688803412529894552453132412i",
    ),
    (
        r"iterate((1+i)*x+2, x, 1/2, 1+1i)",
        r"sqrt(1 + i) + 2i - i*sqrt(1 + i)|  ~ 1.5537739740300373073441589530631 + 1.3564057470944173752645565625818i",
    ),
    (
        r"iterate((1+i)*x+2, x, 1/3, 1+1i)",
        r"(1 + i)^(1/3) - i*(1 + i)^(1/3) + 2i|  ~ 1.374729636998602626383479196886 + 1.2062994740159002626241471803638i",
    ),
    (
        r"iterate((1+i)*x+2, x, 5/2, 0.3)",
        r"4.1216825375339034593765901262692 + 4.4795699103295953448413117484343i",
    ),
    (
        r"iterate((1+i)*x+2, x, 7/4, 0.3)",
        r"3.704875439028892639700965293686 + 1.8240359842716700767882902445612i",
    ),
    (
        r"iterate((1+i)*x+2, x, i, 1+1i)",
        r"(1 + i)^i - i*(1 + i)^i + 2i|  ~ 0.58370075875861462751496189677717 + 1.7260427461698789288696578825825i",
    ),
    (r"iterate((2x+1)/(x+3), x, -1, 0)", r"-1/2|  ~ -0.5"),
    (
        r"iterate((2x+1)/(x+3), x, -1/2, 0)",
        r"2 - sqrt(5/2 + sqrt(5)/2)*sqrt(5/2 - sqrt(5)/2)|  ~ -0.23606797749978969640917366873128",
    ),
    (
        r"iterate((2x+1)/(x+3), x, 1/2, 1)",
        r"7/11 + sqrt(5/2 + sqrt(5)/2)*sqrt(5/2 - sqrt(5)/2)/11|  ~ 0.83964254340907179058265215170284",
    ),
    (
        r"iterate((2x+1)/(x+3), x, 1/3, 2+1i)",
        r"-2(6(5/2 + sqrt(5)/2)^(1/3)*(5/2 - sqrt(5)/2)^(1/3) + 2(5/2 + sqrt(5)/2)^(2/3) + 2(5/2 - sqrt(5)/2)^(2/3) + 3sqrt(5)*(5/2 + sqrt(5)/2)^(2/3) - 3sqrt(5)*(5/2 - sqrt(5)/2)^(2/3) + 5i*(5/2 + sqrt(5)/2)^(1/3)*(5/2 - sqrt(5)/2)^(1/3))/(24(5/2 + sqrt(5)/2)^(1/3)*(5/2 - sqrt(5)/2)^(1/3) - 17(5/2 + sqrt(5)/2)^(2/3) - 17(5/2 - sqrt(5)/2)^(2/3) - 5sqrt(5)*(5/2 + sqrt(5)/2)^(2/3) + 5sqrt(5)*(5/2 - sqrt(5)/2)^(2/3))|  ~ 1.5303789334410564938656928233814 + 0.52460884169011424418000060023784i",
    ),
    (
        r"iterate((2x+1)/(x+3), x, 5/2, 0)",
        r"18/29 - (5/2 + sqrt(5)/2)^(5/2)*(5/2 - sqrt(5)/2)^(5/2)/725|  ~ 0.54358386284483483805485608038858",
    ),
    (
        r"iterate((2x+1)/(x+3), x, 7/4, 1)",
        r"-((5/2 + sqrt(5)/2)^(7/2) + 8(5/2 + sqrt(5)/2)^(7/4)*(5/2 - sqrt(5)/2)^(7/4) + (5/2 - sqrt(5)/2)^(7/2) - sqrt(5)*(5/2 + sqrt(5)/2)^(7/2) + sqrt(5)*(5/2 - sqrt(5)/2)^(7/2))/(2((5/2 + sqrt(5)/2)^(7/2) - 7(5/2 + sqrt(5)/2)^(7/4)*(5/2 - sqrt(5)/2)^(7/4) + (5/2 - sqrt(5)/2)^(7/2)))|  ~ 0.68026387564392756656469151796486",
    ),
    (
        r"iterate((2x+1)/(x+3), x, i, 2+1i)",
        r"(sqrt(5)*((5/2 + sqrt(5)/2)^i - (5/2 - sqrt(5)/2)^i) + 2sqrt(5)*((5/2 - sqrt(5)/2)^i*(5/2 + sqrt(5)/2) - (5/2 + sqrt(5)/2)^i*(5/2 - sqrt(5)/2))/5 + 2i*sqrt(5)*((5/2 + sqrt(5)/2)^i - (5/2 - sqrt(5)/2)^i)/5 + i*sqrt(5)*((5/2 - sqrt(5)/2)^i*(5/2 + sqrt(5)/2) - (5/2 + sqrt(5)/2)^i*(5/2 - sqrt(5)/2))/5)/(sqrt(5)*((5/2 + sqrt(5)/2)^i - (5/2 - sqrt(5)/2)^i) + sqrt(5)*((5/2 - sqrt(5)/2)^i*(5/2 + sqrt(5)/2) - (5/2 + sqrt(5)/2)^i*(5/2 - sqrt(5)/2))/5 + i*sqrt(5)*((5/2 + sqrt(5)/2)^i - (5/2 - sqrt(5)/2)^i)/5)|  ~ 1.4314373905548501687496290185655 - 1.2595383020191016604347888301681i",
    ),
    (
        r"iterate(x^3+3x^2+3x, x, -1/2, -0.5)",
        r"-0.32980643898619905541439309596935",
    ),
    (
        r"iterate(x^3+3x^2+3x, x, -1/2, -0.8+0.1i)",
        r"-0.59386020096807619687787714921398 + 0.1113918153989740111998265749354i",
    ),
    (
        r"iterate(x^3+3x^2+3x, x, 1/2, -0.8+0.1i)",
        r"-0.94812574269284285370353848081106 + 0.05373992005763426202289154214887i",
    ),
    (
        r"iterate(x^3+3x^2+3x, x, 1/3, -0.8+0.1i)",
        r"-0.90953918299273620876065285021478 + 0.071476092992652953798463913930215i",
    ),
    (
        r"iterate(x^3+3x^2+3x, x, 3/10-7i/10, -0.5)",
        r"-0.60791826897695834594102320450588 + 0.31077135402920468068121969547276i",
    ),
    (
        r"iterate(x^3+3x^2+3x, x, i, -0.5)",
        r"-0.40505640633425274380562432481892 - 0.42231606475463783545645872282085i",
    ),
    (
        r"iterate(x^3+3x^2+3x, x, pi/4, -0.5)",
        r"-0.80654106638677473357935723284271",
    ),
    (
        r"iterate(x^2-2x+2, x, -1, 0.6)",
        r"1 + 0.63245553203367586639977870888654i",
    ),
    (
        r"iterate(x^2-2x+2, x, -1/2, 0.6)",
        r"0.68313709484219451511949407776519 + 0.41625510119218963328325151911804i",
    ),
    (
        r"iterate(x^2-2x+2, x, 1/2, 1.5)",
        r"1.3752142272464817736730584740494",
    ),
    (
        r"iterate(x^2-2x+2, x, 1/3, 1.5)",
        r"1.4175668100329211334873909232508",
    ),
    (
        r"iterate(x^2-2x+2, x, 5/2, 0.6)",
        r"1.0026536083065716116915683431008 - 0.0049419632857215194139712508966241i",
    ),
    (
        r"iterate(x^2-2x+2, x, 7/4, 0.6)",
        r"0.98093976169269396661060683961823 - 0.041718513149212627366464348421604i",
    ),
    (
        r"iterate(x^2-2x+2, x, i, 1.5)",
        r"1.5301166451927410014486427247973 - 0.251445469974224079747753460641i",
    ),
    (r"iterate(-x/3+1, x, -1, 0.3)", r"2.1"),
    (
        r"iterate(-x/3+1, x, -1/2, 0.3)",
        r"0.75 + 0.77942286340599478208735085367764i",
    ),
    (
        r"iterate(-x/3+1, x, 1/2, 0.3)",
        r"0.75 - 0.25980762113533159402911695122588i",
    ),
    (
        r"iterate(-x/3+1, x, 1/3, 0.3)",
        r"0.59399371327110719141024573817316 - 0.27021081491460061350197971215365i",
    ),
    (
        r"iterate(-x/3+1, x, 5/2, 0.3)",
        r"0.75 - 0.028867513459481288225457439025098i",
    ),
    (
        r"iterate(-x/3+1, x, 7/4, 0.3)",
        r"0.70346975704489502005293906265084 + 0.046530242955104979947060937349162i",
    ),
    (
        r"iterate(-x/3+1, x, i, 0.3)",
        r"0.74115520898531582387501158095226 + 0.017318395568799951540188433481182i",
    ),
    (
        r"iterate(1/(1+x), x, 1/2, 0.3)",
        r"0.59479553903345724907063197026022 - 0.22676579925650557620817843866171i",
    ),
    (
        r"iterate(1/(1+x), x, 1/2, 2+1i)",
        r"(1/39)(19 + 22sqrt(1/2 + sqrt(5)/2)*sqrt(1/2 - sqrt(5)/2) - 4i + 7i*sqrt(1/2 + sqrt(5)/2)*sqrt(1/2 - sqrt(5)/2))|  ~ 0.30769230769230769230769230769231 + 0.46153846153846153846153846153846i",
    ),
    (
        r"iterate(1/(1+x), x, 1/3, 1)",
        r"-(8(1/2 + sqrt(5)/2)^(1/3)*(1/2 - sqrt(5)/2)^(1/3) + (1/2 + sqrt(5)/2)^(2/3) + (1/2 - sqrt(5)/2)^(2/3) - sqrt(5)*(1/2 + sqrt(5)/2)^(2/3) + sqrt(5)*(1/2 - sqrt(5)/2)^(2/3))/(2(-7(1/2 + sqrt(5)/2)^(1/3)*(1/2 - sqrt(5)/2)^(1/3) + (1/2 + sqrt(5)/2)^(2/3) + (1/2 - sqrt(5)/2)^(2/3)))|  ~ 0.72108361969183491226876495208604 + 0.22642508214540558090258778927494i",
    ),
    (
        r"iterate(1/(1+x), x, 3/10-7i/10, 0.3)",
        r"-0.63230708767993458213296933318377 - 1.1621071058411115121629160201402i",
    ),
    (
        r"iterate(1/(1+x), x, 3/10-7i/10, 2+1i)",
        r"(sqrt(5)*((1/2 + sqrt(5)/2)^(3/10 - 7i/10) - (1/2 - sqrt(5)/2)^(3/10 - 7i/10))/5 + 2sqrt(5)*((1/2 - sqrt(5)/2)^(3/10 - 7i/10)*(1/2 + sqrt(5)/2) - (1/2 + sqrt(5)/2)^(3/10 - 7i/10)*(1/2 - sqrt(5)/2))/5 + i*sqrt(5)*((1/2 - sqrt(5)/2)^(3/10 - 7i/10)*(1/2 + sqrt(5)/2) - (1/2 + sqrt(5)/2)^(3/10 - 7i/10)*(1/2 - sqrt(5)/2))/5)/(3sqrt(5)*((1/2 + sqrt(5)/2)^(3/10 - 7i/10) - (1/2 - sqrt(5)/2)^(3/10 - 7i/10))/5 + sqrt(5)*((1/2 - sqrt(5)/2)^(3/10 - 7i/10)*(1/2 + sqrt(5)/2) - (1/2 + sqrt(5)/2)^(3/10 - 7i/10)*(1/2 - sqrt(5)/2))/5 + i*sqrt(5)*((1/2 + sqrt(5)/2)^(3/10 - 7i/10) - (1/2 - sqrt(5)/2)^(3/10 - 7i/10))/5)|  ~ -1.2342943172004914223287131530551 + 0.49245121530073550245116503757557i",
    ),
    (
        r"iterate(1/(1+x), x, 5/2, 1)",
        r"21/34 + (1/2 + sqrt(5)/2)^(5/2)*(1/2 - sqrt(5)/2)^(5/2)/34|  ~ 0.61764705882352941176470588235294 + 0.029411764705882352941176470588235i",
    ),
    (
        r"iterate(1/(1+x), x, i, 0.3)",
        r"0.60883768421934400270428001046059 + 0.013040121703794195414150800987661i",
    ),
    (
        r"iterate(2x+1, x, -1, -2+1i)",
        r"-3/2 + i/2|  ~ -1.5 + 0.5i",
    ),
    (
        r"iterate(2x+1, x, -1/2, -2+1i)",
        r"-1 - sqrt(2)/2 + i*sqrt(2)/2|  ~ -1.7071067811865475244008443621048 + 0.70710678118654752440084436210485i",
    ),
    (
        r"iterate(2x+1, x, 1/2, 0.3)",
        r"0.83847763108502356344219534147261",
    ),
    (
        r"iterate(2x+1, x, 1/3, 0.3)",
        r"0.6378973648633351141973737894617",
    ),
    (
        r"iterate(2x+1, x, 5/2, -2+1i)",
        r"-1 - 4sqrt(2) + 4i*sqrt(2)|  ~ -6.6568542494923801952067548968388 + 5.6568542494923801952067548968388i",
    ),
    (
        r"iterate(2x+1, x, 7/4, -2+1i)",
        r"-1 - 2*2^(3/4) + 2i*2^(3/4)|  ~ -4.3635856610148581721245019049329 + 3.3635856610148581721245019049329i",
    ),
    (
        r"iterate(2x+1, x, i, 0.3)",
        r"0.000010571773163764551828991759651912 + 0.83064965920772524149504278490411i",
    ),
    (
        r"iterate(2x^2-1, x, 1/2, 1.5+1i)",
        r"1.8037075971853787047662070099861 + 2.3319966895316688636920785579511i",
    ),
    (
        r"iterate(2x^2-1, x, 1/2, 3)",
        r"cosh(sqrt(2)*acosh(3))|  ~ 6.0894930018640671213245594391354",
    ),
    (
        r"iterate(2x^2-1, x, 1/3, 3)",
        r"cosh(2^(1/3)*acosh(3))|  ~ 4.6621677089755623347286494975299",
    ),
    (
        r"iterate(2x^2-1, x, 3/10-7i/10, 3)",
        r"cosh(2^(3/10 - 7i/10)*acosh(3))|  ~ 1.8460220549829534864307425836913 - 2.8289942360380079609336339871498i",
    ),
    (
        r"iterate(2x^2-1, x, 5/2, 1.5+1i)",
        r"-509.49134994412830693042535012762 - 361.37962198431862163444927657918i",
    ),
    (
        r"iterate(2x^2-1, x, i, 1.5+1i)",
        r"0.2898764106198789760384970876153 + 0.55405008343476178644985111775747i",
    ),
    (
        r"iterate(2x^2-1, x, pi/4, 1.5+1i)",
        r"1.8361583524170869973340363264919 + 3.9466206164416291872243764128643i",
    ),
    (
        r"iterate(3x^2, x, -1, 0.1)",
        r"0.1825741858350553711523232609336",
    ),
    (
        r"iterate(3x^2, x, -1/2, 0.1)",
        r"0.14228123911205840334271976501363",
    ),
    (
        r"iterate(3x^2, x, 1/2, 3)",
        r"3^(2sqrt(2))/3|  ~ 7.4538636461434629094616635799057",
    ),
    (
        r"iterate(3x^2, x, 1/3, 3)",
        r"3^(2*2^(1/3))/3|  ~ 5.310665769232427659282380488854",
    ),
];

const CLOSED_FORMS_AGAINST_COORDINATES_2: &[(&str, &str)] = &[
    (
        r"iterate(3x^2, x, 5/2, 0.1)",
        r"0.00036730758190179032039693312788529",
    ),
    (
        r"iterate(3x^2, x, 7/4, 0.1)",
        r"0.0058094001982407267793453105615485",
    ),
    (
        r"iterate(3x^2, x, i, 3)",
        r"3^(2*2^i)/3|  ~ 0.30008282975133423629475377055311 + 1.7817445637373474137889612085404i",
    ),
    (
        r"iterate(4x^3-3x, x, 1/2, 1.5+1i)",
        r"1.8322968458613009554153048136147 + 4.0002869376590537395131088207629i",
    ),
    (
        r"iterate(4x^3-3x, x, 1/2, 2)",
        r"cosh(sqrt(3)*acosh(2))|  ~ 4.9445059279250598136588339201949",
    ),
    (
        r"iterate(4x^3-3x, x, 1/3, 1.5+1i)",
        r"1.8172253738197395780277055173992 + 2.4532153320980453778875658924833i",
    ),
    (
        r"iterate(4x^3-3x, x, 1/3, 2)",
        r"cosh(3^(1/3)*acosh(2))|  ~ 3.4157123310696608551449169381737",
    ),
    (
        r"iterate(4x^3-3x, x, 5/2, 1.5+1i)",
        r"-101736350.10507325862051608834848 - 137133476.72989389391523293379425i",
    ),
    (
        r"iterate(4x^3-3x, x, 5/2, 2)",
        r"cosh(9sqrt(3)*acosh(2))|  ~ 411860278.34502022485072727012815",
    ),
    (
        r"iterate(4x^3-3x, x, pi/4, 1.5+1i)",
        r"-0.037916358290287830032058287439871 + 9.8897648425286561093375992875451i",
    ),
    (
        r"iterate(4x^3-3x, x, -1, 0.3)",
        r"0.9122574389279368366370850381393",
    ),
    (
        r"iterate(4x^3-3x, x, -1/2, 0.3)",
        r"0.74451697655551596628582957411649",
    ),
    (
        r"iterate(4x^3-3x, x, 5/2, 0.3)",
        r"0.63170391088197138720371547551972",
    ),
    (
        r"iterate(4x^3-3x, x, 7/4, 1.5+1i)",
        r"-465.19811648250803138590658553841 - 2730.759973968907483894534268899i",
    ),
    (
        r"iterate(4x^3-3x, x, i, 2)",
        r"cosh(3^i*acosh(2))|  ~ 0.45914817194348559890972624339634 + 0.58580747353349407081835909400251i",
    ),
    (
        r"iterate(8x^4-8x^2+1, x, -1, 0.3)",
        r"0.95032251757754718721705817902495",
    ),
    (
        r"iterate(8x^4-8x^2+1, x, -1/2, 0.3)",
        r"0.80622577482985496523666132303038",
    ),
    (
        r"iterate(8x^4-8x^2+1, x, 1/2, 1+1i)",
        r"cosh(2acosh(1 + i))|  ~ -1 + 4i",
    ),
    (
        r"iterate(8x^4-8x^2+1, x, 1/3, 2)",
        r"cosh(2^(2/3)*acosh(2))|  ~ 4.1064526780199773623501739070933",
    ),
    (
        r"iterate(8x^4-8x^2+1, x, 5/2, 0.3)",
        r"-0.94752709327148052710142752325632",
    ),
    (
        r"iterate(8x^4-8x^2+1, x, 7/4, 1+1i)",
        r"cosh(8sqrt(2)*acosh(1 + i))|  ~ -56553.78533055875284112397580119 - 59302.041547503748938854897686481i",
    ),
    (
        r"iterate(8x^4-8x^2+1, x, i, 2)",
        r"cosh(4^i*acosh(2))|  ~ 0.28068993082559222432306148512475 + 0.23471666268373881434929097875265i",
    ),
    (r"iterate(sqrt(x), x, -1, 0.3)", r"0.09"),
    (
        r"iterate(sqrt(x), x, -1/2, 0.3)",
        r"0.18219555902936464153636414816419",
    ),
    (
        r"iterate(sqrt(x), x, 1/2, 2i)",
        r"(2i)^(sqrt(2)/2)|  ~ 0.72486781198949564804964225017138 + 1.4627750332268412004090438586708i",
    ),
    (
        r"iterate(sqrt(x), x, 1/3, 3)",
        r"3^(2^(2/3)/2)|  ~ 2.3916156730285356391011072349423",
    ),
    (
        r"iterate(sqrt(x), x, 5/2, 0.3)",
        r"0.80829003264464806160197790414192",
    ),
    (
        r"iterate(sqrt(x), x, 7/4, 2i)",
        r"(2i)^(2^(1/4)/4)|  ~ 1.0972626883893798851585084824418 + 0.55323789454575558533289225457048i",
    ),
    (
        r"iterate(sqrt(x), x, i, 3)",
        r"3^((1/2)^i)|  ~ 1.7777460946140607211549918431813 - 1.503373768449330841428893046969i",
    ),
    (r"iterate(x+3, x, -1, 0.3)", r"-2.7"),
    (r"iterate(x+3, x, -1/2, 0.3)", r"-1.2"),
    (r"iterate(x+3, x, 1/2, 1+1i)", r"5/2 + i|  ~ 2.5 + i"),
    (r"iterate(x+3, x, 1/3, 1+1i)", r"2 + i"),
    (r"iterate(x+3, x, 5/2, 0.3)", r"7.8"),
    (r"iterate(x+3, x, 7/4, 0.3)", r"5.55"),
    (r"iterate(x+3, x, i, 1+1i)", r"1 + 4i"),
    (
        r"iterate(x/(2+x), x, -1, 0.3)",
        r"0.85714285714285714285714285714286",
    ),
    (
        r"iterate(x/(2+x), x, -1/2, 0.3)",
        r"0.48446575452020335697527059764356",
    ),
    (
        r"iterate(x/(2+x), x, 1/2, 2+1i)",
        r"(1/233)(71 + 105sqrt(2) + 28i + 25i*sqrt(2))|  ~ 0.94202757102650206061878676412883 + 0.27191132643488144300447303907829i",
    ),
    (
        r"iterate(x/(2+x), x, 1/3, 3)",
        r"3/(4*2^(1/3) - 3)|  ~ 1.4708159236701882051455589361098",
    ),
    (
        r"iterate(x/(2+x), x, 5/2, 0.3)",
        r"0.042529600987257423154855240040139",
    ),
    (
        r"iterate(x/(2+x), x, 7/4, 2+1i)",
        r"(-5 + 14*2^(3/4) + 2i*2^(3/4))/(5 - 28*2^(3/4) + 80sqrt(2))|  ~ 0.26102621438058529956186767657171 + 0.047343182268826268773458035348526i",
    ),
    (
        r"iterate(x/(2+x), x, i, 3)",
        r"3/(4*2^i - 3)|  ~ 0.035310096499517550528554771952855 - 1.1727168770842516907634889624909i",
    ),
    (
        r"iterate(x^2, x, -1, -2)",
        r"i*sqrt(2)|  ~ 1.4142135623730950488016887242097i",
    ),
    (
        r"iterate(x^2, x, -1/2, 0.3)",
        r"0.42684371733617521002815929504089",
    ),
    (
        r"iterate(x^2, x, 1/2, 1+1i)",
        r"(1 + i)^(sqrt(2))|  ~ 0.72486781198949564804964225017138 + 1.4627750332268412004090438586708i",
    ),
    (
        r"iterate(x^2, x, 1/3, 3)",
        r"3^(2^(1/3))|  ~ 3.991490612252180028650352579072",
    ),
    (
        r"iterate(x^2, x, 5/2, -2)",
        r"(-2)^(4sqrt(2))|  ~ 23.867572875308513955865149571678 - 44.449916959089207790807630525659i",
    ),
    (
        r"iterate(x^2, x, 7/4, 0.3)",
        r"0.017428200594722180338035931684645",
    ),
    (
        r"iterate(x^2, x, i, 1+1i)",
        r"(1 + i)^(2^i)|  ~ 0.53596614058978534119715007442804 + 0.58089691827183278120989187563468i",
    ),
    (
        r"iterate(x^2+2x, x, -1, -0.5)",
        r"-0.29289321881345247559915563789515",
    ),
    (
        r"iterate(x^2+2x, x, -1/2, -0.5)",
        r"-0.38745267346393407536833178625433",
    ),
    (
        r"iterate(x^2+2x, x, 1/2, -0.7+0.1i)",
        r"-0.82368442506247900757877550695993 + 0.086265304351016147317501189020813i",
    ),
    (
        r"iterate(x^2+2x, x, 1/3, 0.5)",
        r"0.66671400223449412536885807473404",
    ),
    (
        r"iterate(x^2+2x, x, 5/2, -0.5)",
        r"-0.98017938207796278971888847523099",
    ),
    (
        r"iterate(x^2+2x, x, 7/4, -0.7+0.1i)",
        r"-0.99023417123331322353915620846858 + 0.018372676903244453025457422303624i",
    ),
    (
        r"iterate(x^2+2x, x, i, 0.5)",
        r"0.32042931943596532416600660236333 + 0.34995725634605395876001462954016i",
    ),
    (
        r"iterate(x^2-2, x, 1/2, 2.5+1i)",
        r"2.8894583526469099830870342788358 + 2.1676303195344825851329452461374i",
    ),
];

const CLOSED_FORMS_AGAINST_COORDINATES_3: &[(&str, &str)] = &[
    (
        r"iterate(x^2-2, x, 1/2, 5)",
        r"2cosh(sqrt(2)*acosh(5/2))|  ~ 9.2777052997465309364311742088708",
    ),
    (
        r"iterate(x^2-2, x, 1/3, 5)",
        r"2cosh(2^(1/3)*acosh(5/2))|  ~ 7.3386227862338520695585797459183",
    ),
    (
        r"iterate(x^2-2, x, 3/10-7i/10, 3)",
        r"2cosh(2^(3/10 - 7i/10)*acosh(3/2))|  ~ 2.7261546246199471274399329579444 - 1.3131788541653502622067449136202i",
    ),
    (
        r"iterate(x^2-2, x, 5/2, 3)",
        r"2cosh(4sqrt(2)*acosh(3/2))|  ~ 231.43729780799699057070780591151",
    ),
    (
        r"iterate(x^2-2, x, i, 2.5+1i)",
        r"1.2134919312098680017377455364254 + 0.61557134330204356596872424759019i",
    ),
    (
        r"iterate(x^2-2, x, pi/4, 2.5+1i)",
        r"3.1414182368572209169223015287266 + 3.4603502572403232284506449626628i",
    ),
    (
        r"iterate(x^3+3x, x, 1/2, 0.1)",
        r"0.1733493102125437517060983390429",
    ),
    (
        r"iterate(x^3+3x, x, 1/2, 2)",
        r"2sinh(sqrt(3)*asinh(1))|  ~ 4.3851506224376011846552724956723",
    ),
    (
        r"iterate(x^3+3x, x, 1/3, 2)",
        r"2sinh(3^(1/3)*asinh(1))|  ~ 3.2844821060393835426354263622204",
    ),
    (
        r"iterate(x^3+3x, x, 3/10-7i/10, 1+1i)",
        r"2sinh(3^(3/10 - 7i/10)*asinh(1/2 + i/2))|  ~ 2.2470764816049543398521062142964 - 0.18421223347474387446096235202458i",
    ),
    (
        r"iterate(x^3+3x, x, 5/2, 1+1i)",
        r"2sinh(9sqrt(3)*asinh(1/2 + i/2))|  ~ 2816.2953507828204652462093228013 + 2715.276739359232726974242775222i",
    ),
    (
        r"iterate(x^3+3x, x, i, 0.1)",
        r"0.045423195266965942391936925932406 + 0.089014241382346135989394676476184i",
    ),
    (
        r"iterate(x^3+3x, x, pi/4, 0.1)",
        r"0.23744573507485853882265444682407",
    ),
    (
        r"iterate(x^3+3x, x, 1/2, 1+1i)",
        r"2sinh(sqrt(3)*asinh(1/2 + i/2))|  ~ 1.493694039952040448779614122318 + 2.0506029513536330145496818127734i",
    ),
    (
        r"iterate(x^3+3x, x, 1/3, 0.1)",
        r"0.14428980723187361934788504845331",
    ),
    (
        r"iterate(x^3+3x, x, 5/2, 0.1)",
        r"1.7206870211714469216486331743861",
    ),
    (
        r"iterate(x^3+3x, x, 5/2, 2)",
        r"2sinh(9sqrt(3)*asinh(1))|  ~ 926578.91688182873557369855593294",
    ),
    (
        r"iterate(x^3+3x, x, pi/4, 1+1i)",
        r"2sinh(3^(pi/4)*asinh(1/2 + i/2))|  ~ 1.5467275920226262214649304289564 + 3.3377691931784669131324612171714i",
    ),
    (
        r"iterate(x^3+3x, x, -1, -0.3)",
        r"-0.099669956223525738235545251907291",
    ),
    (
        r"iterate(x^3+3x, x, -1/2, -0.3)",
        r"-0.17277623497448992807723260156081",
    ),
    (
        r"iterate(x^3+3x, x, 5/2, -0.3)",
        r"-10.17636886839916017770871100392",
    ),
    (
        r"iterate(x^3+3x, x, 7/4, 1+1i)",
        r"2sinh(3*3^(3/4)*asinh(1/2 + i/2))|  ~ -37.59555105128800242362454935717 + 1.8340658938756342815792715850283i",
    ),
    (
        r"iterate(x^3+3x, x, i, 2)",
        r"2sinh(3^i*asinh(1))|  ~ 0.5825048712050989482967871081117 + 1.5286628944339738876334978229646i",
    ),
    (
        r"iterate(x^3/2, x, -1, 0.5+0.5i)",
        r"1.0842150814913511818796660082611 + 0.29051455550725144450381318862493i",
    ),
    (
        r"iterate(x^3/2, x, -1/2, 0.5+0.5i)",
        r"0.85201359964656371153121582663853 + 0.41520072792087408343251223205355i",
    ),
    (
        r"iterate(x^3/2, x, 1/2, 3)",
        r"2^(1/2 - sqrt(3)/2)*3^(sqrt(3))|  ~ 5.2025184473721668454211668501237",
    ),
    (
        r"iterate(x^3/2, x, 1/3, 3)",
        r"2^(1/2 - 3^(1/3)/2)*3^(3^(1/3))|  ~ 4.1837211587552711386172878659831",
    ),
    (
        r"iterate(x^3/2, x, 5/2, 0.5+0.5i)",
        r"0.000027216299149323563688776021422369 - 0.0000091167065466445309627116366694653i",
    ),
    (
        r"iterate(x^3/2, x, 7/4, 0.5+0.5i)",
        r"0.007562380832408335737839903180388 - 0.0097727794561208557771470209437298i",
    ),
    (
        r"iterate(x^3/2, x, i, 3)",
        r"2^(1/2 - 3^i/2)*3^(3^i)|  ~ 1.5608814633284795371316202884172 + 1.2359757673613164880962185496351i",
    ),
    (
        r"iterate(x^5+5x^3+5x, x, -1, -0.3)",
        r"-0.059786148604840175981224355650846",
    ),
    (
        r"iterate(x^5+5x^3+5x, x, -1/2, -0.3)",
        r"-0.13376551894215325240757218252782",
    ),
    (
        r"iterate(x^5+5x^3+5x, x, 1/2, 1+1i)",
        r"2sinh(sqrt(5)*asinh(1/2 + i/2))|  ~ 1.5765344202290561362429444929898 + 3.0350331899698899700390015158388i",
    ),
    (
        r"iterate(x^5+5x^3+5x, x, 1/3, 2)",
        r"2sinh(5^(1/3)*asinh(1))|  ~ 4.2922015974791436414418835755867",
    ),
    (
        r"iterate(x^5+5x^3+5x, x, 5/2, -0.3)",
        r"-4247.6634410428974953714168649014",
    ),
    (
        r"iterate(x^5+5x^3+5x, x, 7/4, 1+1i)",
        r"2sinh(5*5^(3/4)*asinh(1/2 + i/2))|  ~ 2055.0986791883384532492979724495 + 6822.9476367082097348790243686519i",
    ),
    (
        r"iterate(x^5+5x^3+5x, x, i, 2)",
        r"2sinh(5^i*asinh(1))|  ~ -0.043359775216038192642334396031864 + 1.5432834836967763975875912270199i",
    ),
];

const WITH_50_DIGITS_1: &[(&str, &str)] = &[
    (
        r"iterate((e^x-1)/3, x, -1/2, -1e-80)",
        r"-1.7320508075688772935274463415058723669428052538104e-80|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"iterate((e^x-1)/3, x, 1/3, 1e-50)",
        r"6.9336127435063470484335227478596179544593511345776e-51|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"iterate((e^x-1)/3, x, 7/4, 1e-120i)",
        r"1.4623044588361027342435765575522211724000762113398e-121i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"iterate((x^2+1)/3, x, -1, -1+0.5i)",
        r"0.36878304777929095037904182213839182414616843440627 + 2.0337160412234011761227004050726570443212977894504i",
    ),
    (
        r"iterate((x^2+1)/3, x, 1/3, -1+0.5i)",
        r"0.84045638234233428852988532885990872676822776896094 - 0.46114185716731166025817767432704971723013209428175i|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1 - 0.5i, which the map takes to the same points",
    ),
    (
        r"iterate((x^2+1)/3, x, 7/4, -1+0.5i)",
        r"0.43185710994561154059861693831247317036809892591304 - 0.17222771417469680765377782901594700524246872539798i|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1 - 0.5i, which the map takes to the same points",
    ),
    (
        r"iterate((1/2+i/3)*x+x^2, x, -1, -0.1+0.05i)",
        r"-0.046066814399912276558065736020694953223965609080119 + 0.16023778721363396785050750058155771435271968522359i",
    ),
    (
        r"iterate((1/2+i/3)*x+x^2, x, 1/3, -0.1+0.05i)",
        r"-0.091583106231882645543963154080926606475391072058516 + 0.017757164055675798222036905890298499519634064353027i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5 + 0.3333333333i)",
    ),
    (
        r"iterate((1/2+i/3)*x+x^2, x, 7/4, -0.1+0.05i)",
        r"-0.027848880815157141558788980426324073376260831003382 - 0.027207925155894019922507292716658999132980395471212i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5 + 0.3333333333i)",
    ),
    (
        r"iterate(cos(x), x, -1, -0.5)",
        r"2.0943951023931954923084289221863352561314462662501",
    ),
    (
        r"iterate(cos(x), x, 1/3, -0.5)",
        r"0.65823036193105177319036736033389520179798214299386 - 0.169194348745103427243969480022898953270590672633i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.5, which the map takes to the same points",
    ),
    (
        r"iterate(cos(x), x, 7/4, -0.5)",
        r"0.66504021352353894172678551957491804367964919944003 + 0.079471009108001027486339944722600942933696473123095i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.5, which the map takes to the same points",
    ),
    (
        r"iterate(ln(1+x)/2, x, -1, 1)",
        r"e^2 - 1|  ~ 6.3890560989306502272304274605750078131803155705518",
    ),
    (
        r"iterate(ln(1+x)/2, x, 1/3, 1)",
        r"0.67451448499909004961215737800669146513273401101315|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, 7/4, 1)",
        r"0.18187450512733887934095227540103650735397467835144|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, -1/2, -1e-80)",
        r"-1.4142135623730950488016887242096980785696718753769e-80|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, 1/3, 1e-50)",
        r"7.9370052598409973737585281963615413019574666394992e-51|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, 7/4, -1e-80)",
        r"-2.9730177875068026667937499264011897882324302311595e-81|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(5/2*x*(1-x), x, -1, 0.1)",
        r"0.9582575694955840006588047193728008488984456576768",
    ),
    (
        r"iterate(5/2*x*(1-x), x, 1/3, 0.1)",
        r"0.58573401552176471586303333200093730097408631566111 - 0.084951093874560829887121758260541626158510040798617i|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points",
    ),
    (
        r"iterate(5/2*x*(1-x), x, 7/4, 0.1)",
        r"0.57660242146051250462386293219130185274551933929549 + 0.031354846402811138223515404323476616667160818415582i|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points",
    ),
    (
        r"iterate(-x/2+x^3, x, -1, 0.1+0.2i)",
        r"-0.11828523629842643443699386639614804861802612618364 - 0.34612398617927284339265615237284682268487718837728i|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"iterate(-x/2+x^3, x, 1/3, 0.1+0.2i)",
        r"-0.072162407767983473255533399953292059748778394676105 + 0.16595879826620859380827858163158413220533157419971i|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"iterate(-x/2+x^3, x, 7/4, 0.1+0.2i)",
        r"0.071116746088673588553864660262122172285302511094031 + 0.015836865117278505979614988386988184977094529979652i|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"iterate(sin(x)/2, x, -1, -0.7)",
        r"-1.5707963267948966192313216916397514420985846996876 + 0.86701472649056510395052026950146737546449648994564i",
    ),
    (
        r"iterate(sin(x)/2, x, 1/3, -0.7)",
        r"-0.53157331132539470481546047758955349403941426701597|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(sin(x)/2, x, 7/4, -0.7)",
        r"-0.18867214229635242551760266916271931048580031496641|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/100+x^2, x, 1/2, -0.2+0.1i)",
        r"0.089484101548177916394542303325928516773941029986921 - 0.068223714377749036534008027169597648892326265617921i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.19 - 0.1i, which the map takes to the same points",
    ),
    (
        r"iterate(x/100+x^2, x, 3/10-7i/10, -0.2+0.1i)",
        r"0.13242374946654066650907477568743620828745358316752 + 0.048906535222922651850851796394709124185955097148401i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.19 - 0.1i, which the map takes to the same points",
    ),
    (
        r"iterate(x/100+x^2, x, i, -0.2+0.1i)",
        r"0.19817885534240062266651940179299207918110572637288 - 0.015990235312071671001504031650456067322261258631647i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.19 - 0.1i, which the map takes to the same points",
    ),
    (
        r"iterate(sqrt(2)^x, x, -1, 1+1i, 2)",
        r"1 + 2.2661800709135969048138414728583334050859307338619i|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 1/3, 0, 2)",
        r"0.45895567076266692141804269772101734454931224288344|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 7/4, -1, 2)",
        r"1.1742329682334456693207660126782811836567463368567|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"iterate(x*e^(-x)/2, x, -1, -0.3)",
        r"-0.40156363678707259466266645379468368662300271600635",
    ),
    (
        r"iterate(x*e^(-x)/2, x, 1/3, -0.3)",
        r"-0.2661826113944434022960351466441560124786049451223|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x*e^(-x)/2, x, 7/4, -0.3)",
        r"-0.14130523165087067720738196634738414977857183950528|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (r"iterate(x/2+x^2/4, x, -1, -0.5)", r"-1 + i"),
    (
        r"iterate(x/2+x^2/4, x, 1/3, -0.5)",
        r"-0.34451738410003298321660976057900683474383533176374|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 7/4, -0.5)",
        r"-0.10280208074467024224877575796575641123895326616829|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, -1, -0.02+0.01i)",
        r"-0.040607182990797283695483605389209955826393393085167 + 0.020846518386856085253918342287508958537349301507473i",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/3, -0.02+0.01i)",
        r"-0.015824763083798113907918250919371642967411778111064 + 0.0078707945314848264111094091939882990787184987611235i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 7/4, -0.02+0.01i)",
        r"-0.0058834751704929095491830640408831658561256323783435 + 0.002890071107511922287312944208015084085961918375503i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, -1/2, -1e-60)",
        r"-1.4142135623730950488016887242096980785696718753769e-60|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/3, 1e-90i)",
        r"7.9370052598409973737585281963615413019574666394993e-91i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 7/4, 1e-30)",
        r"2.9730177875068026667937499264032789225434538494208e-31|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(1-cos(x), x, -1/2, 0.5)",
        r"0.76443679451187867236155623868762856570660695514559|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(1-cos(x), x, 1/3, 1)",
        r"0.81124642778587376157689065746233528128456244307611|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(1-cos(x), x, 7/4, 0.5i)",
        r"0.01055553342599204093193877476892805059473947894138 - 0.016428129102498986855365091066777581852053522966384i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(2x^2-3x+4, x, -1/2, 1+2i)",
        r"1.2680539216614042126678188301253301765007846353058 + 1.3782736834864841136153947249636867516528609311027i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(2x^2-3x+4, x, 1/3, 3)",
        r"4.3634605389647311307373353215613543357329225592578|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(2x^2-3x+4, x, 7/4, 10)",
        r"9338.400781574625932636756478603704649920943446377|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(x^3*(1+x), x, -1/2, -0.3)",
        r"-0.046705031827866716596000045578127774765626563219856 + 0.45134313974801196011985051870695033166991616460525i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^3*(1+x), x, 1/3, 0.2i)",
        r"-0.070019189452050198326965903649627262369306794751936 + 0.07337814266387888597929825745713804427158236964156i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^3*(1+x), x, 7/4, 0.2i)",
        r"0.0000032865064237884225466583952011299258770917595691566 - 0.000017065640253313839156942913509969090422461127140796i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(5x^4+x^6, x, 1/2, 0.2+0.1i)",
        r"0.051210198911257065686801578926557510184687138604355 + 0.068797915439657185678860982736894620095099812803944i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 4)",
    ),
    (
        r"iterate(5x^4+x^6, x, 1/3, 0.3)",
        r"0.20371911097299359076641711137715517163717653530032|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 4)",
    ),
    (
        r"iterate(5x^4+x^6, x, 5/2, 0.2+0.1i)",
        r"-1.8479075935552114030050970692137363922814928846682e-14 + 1.9320617923409421883704032034477086044090795289433e-14i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 4)",
    ),
    (
        r"iterate((x-1)^2*(x+1)+1, x, -1/2, 0.8)",
        r"0.84919621427957460294445358425338760476532324985462 + 0.21705473950756636699141481456575548679312466476227i|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate((x-1)^2*(x+1)+1, x, 1/3, 1.1+0.1i)",
        r"1.055816575243239445087278020028211550754912100872 + 0.087397414671867809776170311091645497502467075281202i|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2)",
    ),
    (
        r"iterate((x-1)^2*(x+1)+1, x, 7/4, 1.1+0.1i)",
        r"0.99284572159642224841084127421108482565127955909183 + 0.0030499416580589747181113648796868202126195913052151i|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2)",
    ),
];

const WITH_50_DIGITS_2: &[(&str, &str)] = &[
    (
        r"iterate((x^3+1)/(x+1), x, 1/2, 3)",
        r"4.2301872484651660274871390338538941194453926203219|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate((x^3+1)/(x+1), x, 1/3, 3)",
        r"3.7173281230190852624361157509564066449040750108115|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate((x^3+1)/(x+1), x, 5/2, 5+1i)",
        r"1999.3633204044858506422099342157648726758458147131 + 5504.1506118140262897940572967419911493821785124344i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(sin(x)^2, x, -1/2, 0.3-0.2i)",
        r"0.4495799630227035084847881531620615111375366616737 - 0.21013053116526576013512834347926304580019542091643i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(sin(x)^2, x, 1/3, 1)",
        r"0.89939037808039918761974970168389090011988681272605|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(sin(x)^2, x, 7/4, 0.5)",
        r"0.083171410709112027627454010220738580724928327548109|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2*e^x, x, -1/2, -0.5)",
        r"-0.26118693992279192295801364515874507562767734637652 + 0.55489126375621754788020910298947304590639771615093i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2*e^x, x, 1/3, 0.3)",
        r"0.24022220439498597230560961405819555058856058774321|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2*e^x, x, 7/4, 0.3)",
        r"0.031682102345405218377286610140480608841484230764372|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2+1, x, -1/2, -1+0.5i)",
        r"-0.5790181511049070377982832915984299753367529915804 + 1.4706196702727231981785116733400354023934846677363i|note: iterate: Boettcher's coordinate at infinity (degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2+1, x, 1/3, 1.5i)",
        r"-0.51425009472579090892685405035581530011350824651571 + 1.5576039041204560248667550034242880734562212148683i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(x^2+1, x, 7/4, -2)",
        r"-6.4246668899908459815689929495338702897164193683066 - 14.12060094174416371210017945516633086770148040038i|note: iterate: Boettcher's coordinate at infinity (degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2+x^3, x, -1/2, -0.2)",
        r"-0.15615130363083920349116773417062822420558122078847 + 0.27656193825404672534480752976926068144643192493025i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2+x^3, x, 1/3, 0.1+0.1i)",
        r"0.04659070706139639094856351062259602207722353686063 + 0.075172987177286941579197729736603799390827475863895i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2+x^3, x, 7/4, 0.1)",
        r"0.00051278401397289236595890414135099421030415583337148|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^3+2x, x, -1/2, 0.1)",
        r"0.07064778348799126986949880675098767895541343125876|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"iterate(x^3+2x, x, 1/3, 1+1i)",
        r"1.136675932694915619708712942349977239215394007229 + 1.5424619104660254185249254591358180774936938238768i|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"iterate(x^3+2x, x, 7/4, 0.1)",
        r"0.34216507665430226464268640627558157918254691156103|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"iterate(x^4-x, x, 1/2, -3)",
        r"9.1681251655045808658967960962681349495493900088832|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
    (
        r"iterate(x^4-x, x, 1/3, 1.5)",
        r"1.7412684789656265221790832510388833989289014524914|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
    (
        r"iterate(x^4-x, x, 5/2, -3)",
        r"2478750546850703.9999999899572282367712794081481469|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
    (r"iterate((1+i)*x+2, x, -1, 0.3)", r"-0.85 + 0.85i"),
    (
        r"iterate((1+i)*x+2, x, 1/3, 0.3)",
        r"0.90629363546190824357152617972818310853249045431286 - 0.081275796330526930408188059934687685654712012124813i",
    ),
    (
        r"iterate((1+i)*x+2, x, 7/4, 0.3)",
        r"3.7048754390288926397009652936860267252300319505722 + 1.8240359842716700767882902445611778220794686464948i",
    ),
    (r"iterate((2x+1)/(x+3), x, -1, 0)", r"-1/2|  ~ -0.5"),
    (
        r"iterate((2x+1)/(x+3), x, 1/3, 0)",
        r"sqrt(5)*((5/2 + sqrt(5)/2)^(1/3) - (5/2 - sqrt(5)/2)^(1/3))/(5(3sqrt(5)*((5/2 + sqrt(5)/2)^(1/3) - (5/2 - sqrt(5)/2)^(1/3))/5 + sqrt(5)*((5/2 - sqrt(5)/2)^(1/3)*(5/2 + sqrt(5)/2) - (5/2 + sqrt(5)/2)^(1/3)*(5/2 - sqrt(5)/2))/5))|  ~ 0.13280579048374381258580579072271085039794996686728",
    ),
    (
        r"iterate((2x+1)/(x+3), x, 7/4, 0)",
        r"sqrt(5)*((5/2 + sqrt(5)/2)^(7/4) - (5/2 - sqrt(5)/2)^(7/4))/(5(3sqrt(5)*((5/2 + sqrt(5)/2)^(7/4) - (5/2 - sqrt(5)/2)^(7/4))/5 + sqrt(5)*((5/2 - sqrt(5)/2)^(7/4)*(5/2 + sqrt(5)/2) - (5/2 + sqrt(5)/2)^(7/4)*(5/2 - sqrt(5)/2))/5))|  ~ 0.47001779133636196809359305249810566249879810245948",
    ),
    (
        r"iterate(x^3+3x^2+3x, x, -1/2, -0.5)",
        r"-0.32980643898619905541439309596935199531944350039123",
    ),
    (
        r"iterate(x^3+3x^2+3x, x, 1/3, -0.5)",
        r"-0.63200694869776868196387167891334793282959480414765",
    ),
    (
        r"iterate(x^3+3x^2+3x, x, i, -0.5)",
        r"-0.40505640633425274380562432481891776324773547169306 - 0.42231606475463783545645872282085202721540546576771i",
    ),
    (
        r"iterate(x^2-2x+2, x, -1, 0.6)",
        r"1 + 0.63245553203367586639977870888654370674391102786504i",
    ),
    (
        r"iterate(x^2-2x+2, x, 1/3, 0.6)",
        r"0.78415349329606925673449147127457794539040499582138 - 0.22973896793516321300735956590191414403777082676895i",
    ),
    (
        r"iterate(x^2-2x+2, x, 7/4, 0.6)",
        r"0.98093976169269396661060683961823164434744395135826 - 0.041718513149212627366464348421603680022649873797681i",
    ),
    (r"iterate(-x/3+1, x, -1, 0.3)", r"2.1"),
    (
        r"iterate(-x/3+1, x, 1/3, 0.3)",
        r"0.59399371327110719141024573817315859602466459947201 - 0.27021081491460061350197971215364714041714166910085i",
    ),
    (
        r"iterate(-x/3+1, x, 7/4, 0.3)",
        r"0.70346975704489502005293906265083829561243978871612 + 0.046530242955104979947060937349161704387560211283883i",
    ),
    (
        r"iterate(1/(1+x), x, 1/2, 0.3)",
        r"0.5947955390334572490706319702602230483271375464684 - 0.22676579925650557620817843866171003717472118959108i",
    ),
    (
        r"iterate(1/(1+x), x, 1/3, 2+1i)",
        r"-2(6(1/2 + sqrt(5)/2)^(1/3)*(1/2 - sqrt(5)/2)^(1/3) + 2(1/2 + sqrt(5)/2)^(2/3) + 2(1/2 - sqrt(5)/2)^(2/3) + 3sqrt(5)*(1/2 + sqrt(5)/2)^(2/3) - 3sqrt(5)*(1/2 - sqrt(5)/2)^(2/3) + 5i*(1/2 + sqrt(5)/2)^(1/3)*(1/2 - sqrt(5)/2)^(1/3))/(24(1/2 + sqrt(5)/2)^(1/3)*(1/2 - sqrt(5)/2)^(1/3) - 17(1/2 + sqrt(5)/2)^(2/3) - 17(1/2 - sqrt(5)/2)^(2/3) - 5sqrt(5)*(1/2 + sqrt(5)/2)^(2/3) + 5sqrt(5)*(1/2 - sqrt(5)/2)^(2/3))|  ~ 0.49729261617498487516577951994376933962435757389707 + 0.72763487763421006275947955550373399412894491344926i",
    ),
    (
        r"iterate(1/(1+x), x, 5/2, 1)",
        r"21/34 + (1/2 + sqrt(5)/2)^(5/2)*(1/2 - sqrt(5)/2)^(5/2)/34|  ~ 0.61764705882352941176470588235294117647058823529412 + 0.029411764705882352941176470588235294117647058823529i",
    ),
    (
        r"iterate(2x+1, x, -1, -2+1i)",
        r"-3/2 + i/2|  ~ -1.5 + 0.5i",
    ),
    (
        r"iterate(2x+1, x, 1/3, -2+1i)",
        r"-1 - 2^(1/3) + i*2^(1/3)|  ~ -2.2599210498948731647672106072782283505702514647015 + 1.2599210498948731647672106072782283505702514647015i",
    ),
    (
        r"iterate(2x+1, x, 7/4, -2+1i)",
        r"-1 - 2*2^(3/4) + 2i*2^(3/4)|  ~ -4.3635856610148581721245019049328595801601370494271 + 3.3635856610148581721245019049328595801601370494271i",
    ),
    (
        r"iterate(2x^2-1, x, 1/2, 1.5+1i)",
        r"1.8037075971853787047662070099861449175082393169521 + 2.3319966895316688636920785579511454726730176727317i",
    ),
    (
        r"iterate(2x^2-1, x, 3/10-7i/10, 1.5+1i)",
        r"2.975989693856434751053128702794470906592010650518 - 0.00047324211225049929161439311918888646975332417148639i",
    ),
    (
        r"iterate(2x^2-1, x, i, 1.5+1i)",
        r"0.28987641061987897603849708761530148459051159067439 + 0.55405008343476178644985111775747087539006526499875i",
    ),
    (
        r"iterate(3x^2, x, -1, 0.1)",
        r"0.18257418583505537115232326093360071131758156499933",
    ),
    (
        r"iterate(3x^2, x, 1/3, 0.1)",
        r"0.073129534262836901039734147459677418564798461004009",
    ),
    (
        r"iterate(3x^2, x, 7/4, 0.1)",
        r"0.0058094001982407267793453105615484933996949995480819",
    ),
    (
        r"iterate(4x^3-3x, x, 1/2, 1.5+1i)",
        r"1.8322968458613009554153048136147228854589663200872 + 4.00028693765905373951310882076291978386030197337i",
    ),
    (
        r"iterate(4x^3-3x, x, 1/3, 1.5+1i)",
        r"1.8172253738197395780277055173992085893129273025441 + 2.4532153320980453778875658924833460109147363008117i",
    ),
    (
        r"iterate(4x^3-3x, x, 5/2, 2)",
        r"cosh(9sqrt(3)*acosh(2))|  ~ 411860278.34502022485072727012815213243939691032483",
    ),
    (
        r"iterate(4x^3-3x, x, -1, 0.3)",
        r"0.91225743892793683663708503813930337646526646290379",
    ),
    (
        r"iterate(4x^3-3x, x, 1/3, 0.3)",
        r"-0.25247875449734262112416205118469441950576299068865",
    ),
    (
        r"iterate(4x^3-3x, x, 7/4, 0.3)",
        r"-0.72034191539638395033947475422474530948884402439735",
    ),
    (
        r"iterate(8x^4-8x^2+1, x, -1, 0.3)",
        r"0.95032251757754718721705817902494932995847439523981",
    ),
    (
        r"iterate(8x^4-8x^2+1, x, 1/3, 0.3)",
        r"-0.42505077132436733995226426115070246001250258477181",
    ),
    (
        r"iterate(8x^4-8x^2+1, x, 7/4, 0.3)",
        r"-0.18607017078181717978138340417902154746601226843358",
    ),
    (r"iterate(sqrt(x), x, -1, 0.3)", r"0.09"),
    (
        r"iterate(sqrt(x), x, 1/3, 0.3)",
        r"0.38458369244554859205674078445253522017197383143847",
    ),
    (
        r"iterate(sqrt(x), x, 7/4, 0.3)",
        r"0.69911274412935997651527316074532919613197836680617",
    ),
];

const WITH_50_DIGITS_3: &[(&str, &str)] = &[
    (r"iterate(x+3, x, -1, 0.3)", r"-2.7"),
    (r"iterate(x+3, x, 1/3, 0.3)", r"1.3"),
    (r"iterate(x+3, x, 7/4, 0.3)", r"5.55"),
    (
        r"iterate(x/(2+x), x, -1, 0.3)",
        r"0.85714285714285714285714285714285714285714285714286",
    ),
    (
        r"iterate(x/(2+x), x, 1/3, 0.3)",
        r"0.22423244703127485550191857113867941493821995388489",
    ),
    (
        r"iterate(x/(2+x), x, 7/4, 0.3)",
        r"0.07366190643705778396356073699038963691187643537104",
    ),
    (
        r"iterate(x^2, x, -1, -2)",
        r"i*sqrt(2)|  ~ 1.4142135623730950488016887242096980785696718753769i",
    ),
    (
        r"iterate(x^2, x, 1/3, -2)",
        r"(-2)^(2^(1/3))|  ~ -1.639804394960498720868327865797500620757137767722 - 1.7453466125838821332593195471670258716379723797578i",
    ),
    (
        r"iterate(x^2, x, 7/4, -2)",
        r"(-2)^(2*2^(3/4))|  ~ -4.2773403664443137307270408050745106719873900282362 - 9.3621222066654372800642867071107479497461870570479i",
    ),
    (
        r"iterate(x^2+2x, x, -1, -0.5)",
        r"-0.29289321881345247559915563789515096071516406231153",
    ),
    (
        r"iterate(x^2+2x, x, 1/3, -0.5)",
        r"-0.58243318996707886651260907674919143327312570049124",
    ),
    (
        r"iterate(x^2+2x, x, 7/4, -0.5)",
        r"-0.90284619303908843373245926926685206121135273237929",
    ),
    (
        r"iterate(x^2-2, x, 1/2, 2.5+1i)",
        r"2.8894583526469099830870342788357789798834210751646 + 2.16763031953448258513294524613735755274105228346i",
    ),
    (
        r"iterate(x^2-2, x, 3/10-7i/10, 2.5+1i)",
        r"3.8438708623227279224779750431213071697718712517219 + 0.12763742779425673482352362417155326096496660014189i",
    ),
    (
        r"iterate(x^2-2, x, i, 2.5+1i)",
        r"1.21349193120986800173774553642544315117887268292 + 0.61557134330204356596872424759018817213532016305056i",
    ),
    (
        r"iterate(x^3+3x, x, 1/2, 0.1)",
        r"0.17334931021254375170609833904290234615174635117947",
    ),
    (
        r"iterate(x^3+3x, x, 3/10-7i/10, 0.1)",
        r"0.099794777317530151884278430467988394735714780697191 - 0.096735501293458431448503583496866899237045715108785i",
    ),
    (
        r"iterate(x^3+3x, x, i, 0.1)",
        r"0.045423195266965942391936925932405906775860144488036 + 0.089014241382346135989394676476184267946761493364163i",
    ),
    (
        r"iterate(x^3+3x, x, 1/3, 1+1i)",
        r"2sinh(3^(1/3)*asinh(1/2 + i/2))|  ~ 1.3386371157013764636156763113266254976133302898505 + 1.5872539341312731308117019158861712626126338692205i",
    ),
    (
        r"iterate(x^3+3x, x, 5/2, 2)",
        r"2sinh(9sqrt(3)*asinh(1))|  ~ 926578.91688182873557369855593294393258930517527303",
    ),
    (
        r"iterate(x^3+3x, x, -1, -0.3)",
        r"-0.099669956223525738235545251907290838754627465497828",
    ),
    (
        r"iterate(x^3+3x, x, 1/3, -0.3)",
        r"-0.43441386261429817894407454458218978360838226113689",
    ),
    (
        r"iterate(x^3+3x, x, 7/4, -0.3)",
        r"-2.4187778905637355785254985557988173781475850586877",
    ),
    (
        r"iterate(x^3/2, x, -1, 0.5+0.5i)",
        r"1.0842150814913511818796660082610832038799929487881 + 0.29051455550725144450381318862492907368424628483821i",
    ),
    (
        r"iterate(x^3/2, x, 1/3, 0.5+0.5i)",
        r"0.22075206500856147438578232508995498600283195852158 + 0.47128154791791832915462402771581119502856791301891i",
    ),
    (
        r"iterate(x^3/2, x, 7/4, 0.5+0.5i)",
        r"0.0075623808324083357378399031803880381351638776049221 - 0.0097727794561208557771470209437297861925406990453884i",
    ),
    (
        r"iterate(x^5+5x^3+5x, x, -1, -0.3)",
        r"-0.059786148604840175981224355650846350782906817654509",
    ),
    (
        r"iterate(x^5+5x^3+5x, x, 1/3, -0.3)",
        r"-0.51666905602540757698345397528359299792776358853947",
    ),
    (
        r"iterate(x^5+5x^3+5x, x, 7/4, -0.3)",
        r"-12.081608644668413195988687025494464514270006035165",
    ),
    (
        r"iterate(e^x-1, x, 1/2, -1e-40)",
        r"-9.99999999999999999999999999999999999999975e-41|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/3, -1e-40)",
        r"-9.9999999999999999999999999999999999999998333333333e-41|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 5/2, -1e-80)",
        r"-1e-80|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/2, 1e-40)",
        r"1.000000000000000000000000000000000000000025e-40|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 1/3, 1e-45i)",
        r"-1.6666666666666666666666666666666666666666666666667e-91 + 1e-45i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 5/2, 1e-80)",
        r"1e-80|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, -1, -0.5)",
        r"-0.69314718055994530941723212145817656807550013436026",
    ),
    (
        r"iterate(e^x-1, x, 1/3, -0.5)",
        r"-0.45834190396019757580615706813473262189929531462872|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 7/4, -0.5)",
        r"-0.33994634961794840374859131175472320750788733068378|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, -1, 0.5)",
        r"0.40546510810816438197801311546434913657199042346249",
    ),
    (
        r"iterate(e^x-1, x, 1/3, 0.5)",
        r"0.54167150697465721909099632613490316126806299012052|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(e^x-1, x, 7/4, 0.5)",
        r"0.8297089534819247276767081951228904093422718582169|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(ln(1+x), x, -1, 0.5+0.5i)",
        r"0.44688903658416915805158327605190924180283522054537 + 0.7904390832136149118432625670479557246822600697689i",
    ),
    (
        r"iterate(ln(1+x), x, 1/3, 0.5+0.5i)",
        r"0.49066845913495732591865889458825355207144468226256 + 0.42972051781428525467250932757403590296794611773952i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(ln(1+x), x, 7/4, 0.5+0.5i)",
        r"0.41508240486461633221756255129485063144471655890806 + 0.23847983481986373532357623506468560836724697698246i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, -1, -0.3)",
        r"-0.48940222718021496903623125199629336892341000601636",
    ),
    (
        r"iterate(x*e^x, x, 1/3, -0.3)",
        r"-0.26812380088472073094296184895801665539140908711888|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, 7/4, -0.3)",
        r"-0.1871854778901804616261260192076502063275365365047|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, -1, 0.5)",
        r"0.35173371124919582602490930092995106517146421551711",
    ),
    (
        r"iterate(x*e^x, x, 1/3, 0.5)",
        r"0.57809353507759842595379725186570399459149562091894|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x*e^x, x, 7/4, 0.5)",
        r"1.4545220227873563932351542576140452665948364337748|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, -1, 0.3+0.2i, 0)",
        r"0.25340394063741837301862947159129461366839297291765 + 0.13273092242575061576943189631299741712337217933985i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, 1/3, 0.3+0.2i, 0)",
        r"0.31696889342023185241183473134479571249578056948342 + 0.23229219730122432814649935209978232138688530222882i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, 7/4, 0.3+0.2i, 0)",
        r"0.37241696448288058911767048777305356962622259320545 + 0.47410040065016194833937648164658604665392419890084i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, -1, -0.2+0.1i)",
        r"-0.21556775949710844636084603665765906492381478424845 + 0.17578879212707146424352863376320240539080940258425i",
    ),
    (
        r"iterate(x+x^2, x, 1/3, -0.2+0.1i)",
        r"-0.18997970650338066343329393622947161848956848097058 + 0.083433458237213046649864346351796503849837568348729i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x+x^2, x, 7/4, -0.2+0.1i)",
        r"-0.15042232711300576305774460227789125643828638800296 + 0.043589460765294996525835080327931881610260058978865i|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (r"iterate(x/(1-x), x, -1, -3)", r"3/2|  ~ 1.5"),
    (r"iterate(x/(1-x), x, 1/3, -3)", r"-3/2|  ~ -1.5"),
    (r"iterate(x/(1-x), x, 7/4, -3)", r"-12/25|  ~ -0.48"),
];

const WITH_50_DIGITS_4: &[(&str, &str)] = &[
    (
        r"iterate(-2sinh(x), x, -1, 0.3)",
        r"-0.14944312018495765616028580915059136911214723204303",
    ),
    (
        r"iterate(-2sinh(x), x, 1/3, 0.3)",
        r"0.18510695094395793278246670497878418095632311584258 + 0.32570270734500972205918090702459586127244684243385i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2sinh(x), x, 7/4, 0.3)",
        r"0.66855372204045163052684072616367871985160505925545 - 0.74796665869020920192835083048208167925368429738279i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(2sinh(x), x, -1, 0.3)",
        r"0.14944312018495765616028580915059136911214723204303",
    ),
    (
        r"iterate(2sinh(x), x, 1/3, 0.3)",
        r"0.37908103998368813224138393679751390595097252161686|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 7/4, 0.3)",
        r"1.0628703974367935241301635244580904254575013908305|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, -12+3i, 0.3)",
        r"-0.000035492145952531634362562522555388276056798666241152 + 0.000063653749432728133795352935938613025838496497968621i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, -12+3i, 1e-50)",
        r"-1.1889512157367709914668210952329622102399949304093e-54 + 2.132336625426932149681674744711440495991853429937e-54i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, -61/2, 1e-50)",
        r"6.5854450798271924667139012562562622063312301196947e-60|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 1/2, -1e-80+1e-80i)",
        r"-1.4142135623730950488016887242096980785696718753769e-80 + 1.4142135623730950488016887242096980785696718753769e-80i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 1/3, 1e-50)",
        r"1.2599210498948731647672106072782283505702514647015e-50|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 5/2, -1e-80+1e-80i)",
        r"-5.6568542494923801952067548968387923142786875015078e-80 + 5.6568542494923801952067548968387923142786875015078e-80i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2x+e^x-1, x, 1/2, -1e-70)",
        r"-1.7320508075688772935274463415058723669428052538104e-70|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(2x+e^x-1, x, 1/3, 1e-50)",
        r"1.4422495703074083823216383107801095883918692534994e-50|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(2x+e^x-1, x, 5/2, -1e-70)",
        r"-1.5588457268119895641747017073552851302485247284293e-69|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(e^x, x, 1/2, 0.5, 0.3+1.3i)",
        r"0.99303919280011931894522795776518410261932514162395 + 0.13114284571242883021571603863682936722508416286448i|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i)",
    ),
    (
        r"iterate(e^x, x, 1/3, 2, 0.3+1.3i)",
        r"2.6351662638411502380291881269236110232476289579599 + 0.29219373617268309639712152213788451835948956419507i|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i)",
    ),
    (
        r"iterate(e^x, x, 5/2, 1+1i, 0.3+1.3i)",
        r"-1.4639622638031581782177343726897956434773108314294 - 0.98173238981176034379934069473397694044528366502909i|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(-2x+x^3/1000, x, -1, 0.3, 0)",
        r"-0.15000168755695568802278142760880132553845658043438|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2x+x^3/1000, x, 1/3, 0.3, 0)",
        r"0.188999992877966219239455026727722633883446736784 + 0.32734200091328606727605072495923332220651896031107i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2x+x^3/1000, x, 7/4, 0.3, 0)",
        r"0.71365605445265056450983998478793439264367624952799 - 0.71341386538807851501928714647183284808429635327642i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(sqrt(2)^x, x, -1, 10, 4)",
        r"6.6438561897747246957406388589787803517296627860492|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 1/3, 10, 4)",
        r"12.777182816284014984688300465627395224896802107502|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 7/4, 10, 4)",
        r"1229.9706127536062362066882514668272428803661647975|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"iterate(x^2+1, x, 1/2, 0.5+0.5i, 0.5+0.9i)",
        r"0.72866142145711038169196581584806817482641643398877 + 0.45090655555417975964869528978035760926628729902604i|note: iterate: regular iteration at the repelling fixed point 0.5 + 0.8660254038i (slope 1 + 1.732050808i)",
    ),
    (
        r"iterate(x^2+1, x, 1/3, 2, 0.5+0.9i)",
        r"2.2385307172026957897025595914674764657931972939971 - 0.47681162130697328976628178745515128391723899776415i|note: iterate: regular iteration at the repelling fixed point 0.5 + 0.8660254038i (slope 1 + 1.732050808i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2+1, x, 5/2, 1, 0.5+0.9i)",
        r"8.4323639291039188923230568751047530272733686822532 - 5.2197587416125063508318812991527286140106515924327i|note: iterate: regular iteration at the repelling fixed point 0.5 + 0.8660254038i (slope 1 + 1.732050808i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2-1, x, 1/2, -0.3)",
        r"-0.88159462984144529854997076792025711841574021329567|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8366600265",
    ),
    (
        r"iterate(x^2-1, x, 1/3, 0.2)",
        r"-0.2368358682867236861576019075997140380058361039373|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 5/2, -0.9)",
        r"-0.95661560651729240644188233553759053987775886314812|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.316227766; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2-1, x, 1/2, -0.3, -0.6)",
        r"-0.67429403750566814940818095567885006090241621894186 + 0.41174675597706056375806695302735047455411032583929i|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 1/3, -0.8, -0.6)",
        r"-0.7484783578441068322553430653031989938303459162128 - 0.17394989076603078325335955503345017472195963277655i|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 5/2, -0.5, -0.6)",
        r"-0.63273326521470665984141215586838753769790214530396 + 0.20398882248594895723224855475133672096475218012273i|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 1/2, (1+sqrt(5))/2+1e-60i, 1.6)",
        r"1.6180339887498948482045868343656381177203091798058|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 1/3, (1+sqrt(5))/2-1e-40, 1.6)",
        r"1.6180339887498948482045868343656381177201612669648|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 5/2, (1+sqrt(5))/2+1e-60i, 1.6)",
        r"1.6180339887498948482045868343656381177203091798058|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 1/2, -0.2+0.1i, 1.6)",
        r"-0.82165286371088915082536931195651561419181569775899 + 0.070901750218799755941232415999468374588617081095561i|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8961656585 + 0.055793256i",
    ),
    (
        r"iterate(x^2-1, x, 1/3, -0.5, 1.6)",
        r"-0.86871320329426685467885259544847157800555886453578|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.7071067812",
    ),
    (
        r"iterate(x^2-1, x, 5/2, -0.3, 1.6)",
        r"-0.95036421102971359663050078780649197668790262840225|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8366600265; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^3+2x, x, -1, 0.1, 0)",
        r"0.049937733209797481227325461932619434297659117124833|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(x^3+2x, x, 1/3, 0.1, 0)",
        r"0.12611515512446723592876477948760817045539953168278|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(x^3+2x, x, 7/4, 0.1, 0)",
        r"0.3421827787222968310229405905948661576038115965541|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, -1, 0.02i)",
        r"0.016448180775607652062905168693497840069862311581673 + 0.010745546831400637239263865693498201557844433441637i",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, 1/3, 0.02i)",
        r"-0.0066171808786297311692573914853638483373329573801962 + 0.019014150592930036613472819770074949573155295463905i|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, 7/4, 0.02i)",
        r"-0.019950473811479540239710990486430705695915241946428 - 0.0041525597685876202931302580989651123019022896808438i|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
];

const WITH_100_DIGITS_1: &[(&str, &str)] = &[
    (
        r"iterate((e^x-1)/3, x, -1/2, -1e-80)",
        r"-1.732050807568877293527446341505872366942805253810380628055806979451933016908800046590765129990668874e-80|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"iterate((e^x-1)/3, x, 5/2, -1e-80)",
        r"-6.41500299099584182787943089446619395164001945855696528909558140537752969225481490712629060490371188e-82|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"iterate((x^2+1)/3, x, -1, -1+0.5i)",
        r"0.3687830477792909503790418221383918241461684344062692527898760177214753587197384821446943042165956586 + 2.033716041223401176122700405072657044321297789450391206784112642759011859389378662198424517348811364i",
    ),
    (
        r"iterate((x^2+1)/3, x, 3/10-7i/10, 1)",
        r"0.8569095484865215023815578140817753873097516566545343723380920776270119643901745708796740342231941073 + 0.2592260005174725636047040765102946513544364078322814915875725670406390291020097852454663018008432206i|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075)",
    ),
    (
        r"iterate((1/2+i/3)*x+x^2, x, -1, -0.1+0.05i)",
        r"-0.0460668143999122765580657360206949532239656090801191570225926337269411231771315585258774089066594612 + 0.1602377872136339678505075005815577143527196852235864858032072477379719565749247441605724481256842512i",
    ),
    (
        r"iterate((1/2+i/3)*x+x^2, x, 3/10-7i/10, 0.1)",
        r"0.1174835750263675362273053258684167078067782848922592881458374685697766565296525719974515824492755978 + 0.0470527957002668744427152622468698410377007075503340337298921919353422034594143574954560772091033649i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5 + 0.3333333333i)",
    ),
    (
        r"iterate(cos(x), x, -1, -0.5)",
        r"2.094395102393195492308428922186335256131446266250070547316629728205210937524139332418689883561411379",
    ),
    (
        r"iterate(cos(x), x, 3/10-7i/10, 1)",
        r"0.9798119117464271170651294281490735621400895370697404140653650357952703628453332981052869550186372036 + 0.8499907714166447317405851905849743361807024688371189319902617277047369231475788087274179755524034021i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"iterate(ln(1+x)/2, x, -1, 1)",
        r"e^2 - 1|  ~ 6.389056098930650227230427460575007813180315570551847324087127822522573796079057763384312485079121795",
    ),
    (
        r"iterate(ln(1+x)/2, x, 3/10-7i/10, 3)",
        r"0.7099697044231711517638946198191561573847904120421169644185100470239760496325001754227117800926147893 + 1.170301891517249973482288478098152643868127107462728376539067984365519088802894371337701222520899391i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, -1/2, -1e-80)",
        r"-1.414213562373095048801688724209698078569671875376948073176679737990732478462107032992523158058592061e-80|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(ln(1+x)/2, x, 5/2, -1e-80)",
        r"-1.767766952966368811002110905262122598212089844221185091470849672488415598077633813115653947573240076e-81|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(5/2*x*(1-x), x, -1, 0.1)",
        r"0.9582575694955840006588047193728008488984456576767971902607242123906868425547770886604361559493445033",
    ),
    (
        r"iterate(5/2*x*(1-x), x, 3/10-7i/10, 0.55)",
        r"0.6332035597320710850715641109455811917466770175487315271978885682192952114425632641465964882218627461 - 0.1444142590303736473167718744001628348609166828258406944652204411999258818567421563839689483264503114i|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5)",
    ),
    (
        r"iterate(-x/2+x^3, x, -1, 0.1+0.2i)",
        r"-0.1182852362984264344369938663961480486180261261836385700561064240096388444685267725641681219605361136 - 0.3461239861792728433926561523728468226848771883772777678855778338592403406852544356971195217477370936i|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"iterate(-x/2+x^3, x, 3/10-7i/10, 0.3)",
        r"0.01752133604311086523820540748151767289178710873236195529720742645296928024071541293919674085658349969 + 0.5611921488022505954764552215159738203820177210889605256472879942602744677985692502576442126869512508i|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"iterate(sin(x)/2, x, -1, -0.7)",
        r"-1.570796326794896619231321691639751442098584699687552910487472296153908203143104499314017412671058534 + 0.8670147264905651039505202695014673754644964899456437313235336374113288604675326775564747843240066777i",
    ),
    (
        r"iterate(sin(x)/2, x, 3/10-7i/10, 2+1i)",
        r"1.150136412010211514842622266776873316777810896987218194119441258681466925232898188094326598209843356 + 0.6810547640635615577359537226323126145933135555877155874211341747665523785591146089721126814386179989i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1.141592654 - i, which the map takes to the same points",
    ),
    (
        r"iterate(x/100+x^2, x, 1/2, -0.2+0.1i)",
        r"0.08948410154817791639454230332592851677394102998692125936514090201045842209500358414530269094899940344 - 0.06822371437774903653400802716959764889232626561792117239837653737258393558225021850664514076018018591i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.19 - 0.1i, which the map takes to the same points",
    ),
    (
        r"iterate(x/100+x^2, x, 5/2, -0.2+0.1i)",
        r"-0.0001056853157975352302103562315312108212205087659481071750652296252562278372357839045495604051498698429 - 0.0002384465926323781622905784686464645049678748654270865937262546240335077407530236184741200699651733448i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.19 - 0.1i, which the map takes to the same points",
    ),
    (
        r"iterate(sqrt(2)^x, x, -1, 1+1i, 2)",
        r"1 + 2.266180070913596904813841472858333405085930733861897792093008273970300476860652551129541939802008045i|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 3/10-7i/10, 1, 2)",
        r"1.237495609964295159049267452368659992609500036802579284405186978021079080759883230295980452575764403 - 0.3058173848146085490241991315550215616144660953817118273533322078092953036669023681387271648761681951i|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"iterate(x*e^(-x)/2, x, -1, -0.3)",
        r"-0.4015636367870725946626664537946836866230027160063511402379921560604975325206585381386527082762303647",
    ),
    (
        r"iterate(x*e^(-x)/2, x, 3/10-7i/10, 3)",
        r"0.1051723234182097985255222893369227110620724033029540402220098251191409130074984432945961359506387923 + 0.07439102101820424415009935761196671452901993636990902376734204917652891672398647703248996473429711395i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.1785606279, which the map takes to the same points",
    ),
    (r"iterate(x/2+x^2/4, x, -1, -0.5)", r"-1 + i"),
    (
        r"iterate(x/2+x^2/4, x, 3/10-7i/10, 0.3)",
        r"0.2419941165751391148589118972029086433711672849004488453009971308654088293018017996980871366533350276 + 0.09647037120500395473986451983371696710533137663413764101790992571609348088464725121879943606212979342i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, -1, -0.02+0.01i)",
        r"-0.04060718299079728369548360538920995582639339308516715555727009944075355983658642732024033477053652968 + 0.02084651838685608525391834228750895853734930150747289214568920494360949973752300337450411665324620971i",
    ),
    (
        r"iterate(x/2+x^2/4, x, 3/10-7i/10, 0.03)",
        r"0.02186129873010365881266428352876943199060029476651196506470232154756774832140264650237540387025288543 + 0.01121247170160807994293047236543126613325573010696200953708670446538923529205863654270230135105268728i|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, -1/2, -1e-60)",
        r"-1.414213562373095048801688724209698078569671875376948073176680323777170105367058237161663324629563003e-60|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 5/2, -1e-60)",
        r"-1.767766952966368811002110905262122598212089844221185091470848217221462631708822796452079155786953754e-61|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(1-cos(x), x, -1/2, 0.5)",
        r"0.7644367945118786723615562386876285657066069551455870161327389790565367234156508051846524544899618005|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(1-cos(x), x, 5/2, 0.5)",
        r"0.0007393520216436025043872832065155633903048220092099360728861554491745901071109768732038621883748245542|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(2x^2-3x+4, x, -1/2, 1+2i)",
        r"1.268053921661404212667818830125330176500784635305833988310898160516095197560400232168812168000254551 + 1.378273683486484113615394724963686751652860931102659490661761727603264857804920206557129298191553087i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(2x^2-3x+4, x, 5/2, 1+2i)",
        r"-65.69411464297143247384705426925861364192086663989471263354898833955257352648765474365724626920765346 + 603.0626580178535464620081324266673973598048513422009634505003445943480565166934892105752802670776441i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(x^3*(1+x), x, -1/2, -0.3)",
        r"-0.04670503182786671659600004557812777476562656321985566079184481888086922212128590100482310073445106089 + 0.4513431397480119601198505187069503316699161646052465506623578018485382187504612494826084801154180894i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^3*(1+x), x, 5/2, 0.2i)",
        r"1.318111254400925074031907909486134151195555484205047879632752021970466476503968937939553010327531427e-11 + 5.043046972090108462147382257094716787996846160618460607671160326405138650086560085401116019381696959e-12i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(5x^4+x^6, x, 1/2, 0.2+0.1i)",
        r"0.05121019891125706568680157892655751018468713860435450912018026370737263976443960811582072667109094609 + 0.06879791543965718567886098273689462009509981280394359686517172335044418032961176717367043709854196876i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 4)",
    ),
    (
        r"iterate(5x^4+x^6, x, 3/10-7i/10, 0.3)",
        r"0.2253091532293896224663201249151976395981434010968381252349720080580078210312215789745051913186752347 + 0.2432580636609703228965007443326244399663097589927487376108921733090341197223204687583484677209072175i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 4)",
    ),
    (
        r"iterate((x-1)^2*(x+1)+1, x, -1/2, 0.8)",
        r"0.8491962142795746029444535842533876047653232498546214914431191937074125385494218819976754141557527867 + 0.2170547395075663669914148145657554867931246647622721735515195136364777113847479436085572757874405044i|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate((x-1)^2*(x+1)+1, x, 5/2, 0.8)",
        r"1.001040036601236654434179127027165333860663739074014003188291620741302076905746680845907494465083683 - 0.001934669170292869410719480010227114525120553911355538100056709385239171912231547499857699880663224067i|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate((x^3+1)/(x+1), x, 1/2, 3)",
        r"4.23018724846516602748713903385389411944539262032192648237250609640669564123647651895486961133159152|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate((x^3+1)/(x+1), x, 5/2, 3)",
        r"201.377306915319036990431502602139022289840793059861008655419470165082434513121586598538179014728505|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(sin(x)^2, x, -1/2, 0.3-0.2i)",
        r"0.449579963022703508484788153162061511137536661673701078361084612376926442215567988296872252599650344 - 0.2101305311652657601351283434792630458001954209164255241591048199219367805553182445079768556006566353i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(sin(x)^2, x, 5/2, 0.3-0.2i)",
        r"-0.002983886541867071093975893510814133110891067884031994500396789036850298974576173536422954594431093418 + 0.0001951739215524648565249608100766391915400634682020249796490603056400465862259004163499010412418215511i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"iterate(x^2*e^x, x, -1/2, -0.5)",
        r"-0.2611869399227919229580136451587450756276773463765225799197798980688246548110921538157220508082621583 + 0.5548912637562175478802091029894730459063977161509325269274340840684643155495669898706990626188193098i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2*e^x, x, 5/2, -0.5)",
        r"0.002889710536297465441256245158629184469557936195307789708950140998060394201600925345464738754415425771 - 0.005347219498567048848445333974374875869588248006471220528867464655048561909715697070315639249801374431i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2+1, x, -1/2, -1+0.5i)",
        r"-0.57901815110490703779828329159842997533675299158040175032838455576938345776090451214806932960890759 + 1.470619670272723198178511673340035402393484667736269071110230306873788033579646208915676875452263326i|note: iterate: Boettcher's coordinate at infinity (degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2+1, x, 3/10-7i/10, 3+1i)",
        r"3.931455049948579590981093797285692995477744751872384837601496852893980473685254322601527146460206266 - 1.594706939239391414156450802540795427848666367602544955392085763154246747334163150214327842024384413i|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"iterate(x^2+x^3, x, -1/2, -0.2)",
        r"-0.1561513036308392034911677341706282242055812207884715796347386653814457475492934200599822098076892502 + 0.2765619382540467253448075297692606814464319249302476259905225680414041308547854913679808973826705279i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^2+x^3, x, 5/2, -0.2)",
        r"0.00002927772507703915804257816950381723227468856564395291564470438765513828407421831106037338741840172867 - 0.00005452198003642715831118548577026630792099455874355341829031037446518142098609322071007456736612735464i|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2); the power of the coordinate is a principal power, and at this point it does not commute with the map",
    ),
    (
        r"iterate(x^3+2x, x, -1/2, 0.1)",
        r"0.07064778348799126986949880675098767895541343125876024567570275742879021225302595663074996544680854745|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"iterate(x^3+2x, x, 3/10-7i/10, 2)",
        r"1.391247035167570749171195039790347803627975363531494479813838484705050948008800549220150969041316184 - 1.85501682084360740134104661207873623692246632258098998137453352637371912474734584484196306340534712i|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"iterate(x^4-x, x, 1/2, -3)",
        r"9.168125165504580865896796096268134949549390008883207319372707907109192340782673032187973022003475685|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
    (
        r"iterate(x^4-x, x, 3/10-7i/10, 1.5)",
        r"1.313655206177101242269488289221622953165911419444463646530425740453978787178208883678021237280425108 - 0.4116415868746450000697962443164019248976340689222025604939496971906757498183009166305817804536537947i|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
    (r"iterate((1+i)*x+2, x, -1, 0.3)", r"-0.85 + 0.85i"),
    (
        r"iterate((1+i)*x+2, x, 3/10-7i/10, 1+1i)",
        r"(1 + i)^(3/10 - 7i/10) - i*(1 + i)^(3/10 - 7i/10) + 2i|  ~ 1.909267669751405361151393131480289855463555495553023610988901048962651351851391532051403400607770986 + 0.06388317261205187181194275941536068201568626945807313406806236154297511725173359333734087524954770454i",
    ),
    (r"iterate((2x+1)/(x+3), x, -1, 0)", r"-1/2|  ~ -0.5"),
    (
        r"iterate((2x+1)/(x+3), x, 3/10-7i/10, 1)",
        r"(3sqrt(5)*((5/2 + sqrt(5)/2)^(3/10 - 7i/10) - (5/2 - sqrt(5)/2)^(3/10 - 7i/10))/5 + sqrt(5)*((5/2 - sqrt(5)/2)^(3/10 - 7i/10)*(5/2 + sqrt(5)/2) - (5/2 + sqrt(5)/2)^(3/10 - 7i/10)*(5/2 - sqrt(5)/2))/5)/(4sqrt(5)*((5/2 + sqrt(5)/2)^(3/10 - 7i/10) - (5/2 - sqrt(5)/2)^(3/10 - 7i/10))/5 + sqrt(5)*((5/2 - sqrt(5)/2)^(3/10 - 7i/10)*(5/2 + sqrt(5)/2) - (5/2 + sqrt(5)/2)^(3/10 - 7i/10)*(5/2 - sqrt(5)/2))/5)|  ~ 0.8133795285864354918076343564700402956255385645468407192331031241462843611219264649183992507213361023 + 0.1813000844380041403600285812062947253369974844839218482449947608302421269061748711583806569572772279i",
    ),
    (
        r"iterate(x^3+3x^2+3x, x, -1/2, -0.5)",
        r"-0.3298064389861990554143930959693519953194435003912302046000999176962168500560748032478160264951332879",
    ),
    (
        r"iterate(x^3+3x^2+3x, x, 3/10-7i/10, -0.5)",
        r"-0.6079182689769583459410232045058794259380815136190248262853152083132351688580382554252546918855885989 + 0.3107713540292046806812196954727606741894095292125673159042084154526810095900087891908928278229598346i",
    ),
];

const WITH_100_DIGITS_2: &[(&str, &str)] = &[
    (
        r"iterate(x^2-2x+2, x, -1, 0.6)",
        r"1 + 0.632455532033675866399778708886543706743911027865043365371500970558518887727847644268849621675860059i",
    ),
    (
        r"iterate(x^2-2x+2, x, 3/10-7i/10, 1.5)",
        r"1.433330524109519654455146985972008163563021535733577804139997051476724012759744592415161506479300265 + 0.1821878904383977641894326068138744923712198649861295711753828398722678054439412046405525100423786656i",
    ),
    (r"iterate(-x/3+1, x, -1, 0.3)", r"2.1"),
    (
        r"iterate(-x/3+1, x, 3/10-7i/10, 0.3)",
        r"1.159289589631601511133142254991600200903627209142302283908938727195858255576734810037106187959354219 - 2.889521655543428650799637409486363476002840723367822298046688748251712445987171889249578702691711466i",
    ),
    (
        r"iterate(1/(1+x), x, 1/2, 0.3)",
        r"0.594795539033457249070631970260223048327137546468401486988847583643122676579925650557620817843866171 - 0.2267657992565055762081784386617100371747211895910780669144981412639405204460966542750929368029739777i",
    ),
    (
        r"iterate(1/(1+x), x, 3/10-7i/10, 1)",
        r"(sqrt(5)*((1/2 + sqrt(5)/2)^(3/10 - 7i/10) - (1/2 - sqrt(5)/2)^(3/10 - 7i/10))/5 + sqrt(5)*((1/2 - sqrt(5)/2)^(3/10 - 7i/10)*(1/2 + sqrt(5)/2) - (1/2 + sqrt(5)/2)^(3/10 - 7i/10)*(1/2 - sqrt(5)/2))/5)/(2sqrt(5)*((1/2 + sqrt(5)/2)^(3/10 - 7i/10) - (1/2 - sqrt(5)/2)^(3/10 - 7i/10))/5 + sqrt(5)*((1/2 - sqrt(5)/2)^(3/10 - 7i/10)*(1/2 + sqrt(5)/2) - (1/2 + sqrt(5)/2)^(3/10 - 7i/10)*(1/2 - sqrt(5)/2))/5)|  ~ -0.4845362712342496018615117624443392360505246573276464622126668450873230682690777891384530624662270858 + 1.068310224040452716567235791025700118212701441367948035912766308363936936730145038859516027122920516i",
    ),
    (
        r"iterate(2x+1, x, -1, -2+1i)",
        r"-3/2 + i/2|  ~ -1.5 + 0.5i",
    ),
    (
        r"iterate(2x+1, x, 3/10-7i/10, 0.3)",
        r"0.4157598880794167854177061160643806819169466505540878119955953349741737073777489401861472244687649215 - 0.7464482143510894732884409660838242570302346310795864090666297826652203271880445309913945164314232061i",
    ),
    (
        r"iterate(2x^2-1, x, 1/2, 1.5+1i)",
        r"1.803707597185378704766207009986144917508239316952051490888502742675740771831547770266338039459984324 + 2.331996689531668863692078557951145472673017672731710142886224362938238057640215441970720813049689483i",
    ),
    (
        r"iterate(2x^2-1, x, 5/2, 1.5+1i)",
        r"-509.4913499441283069304253501276248515379805199496671076466771931601788186371461327851808815381288226 - 361.3796219843186216344492765791821036001876531774439921574784867884529173976160899715928744903698861i",
    ),
    (
        r"iterate(3x^2, x, -1, 0.1)",
        r"0.1825741858350553711523232609336007113175815649993277514089648165774977590409075779336194787212902086",
    ),
    (
        r"iterate(3x^2, x, 3/10-7i/10, 3)",
        r"3^(2*2^(3/10 - 7i/10))/3|  ~ 1.110066685047394580926397933512001303445720545210473580353419531582907713442887640080021025369121692 - 3.475339270666784305497312332473094737661337184917218435558308481178959849359335977180162387775344086i",
    ),
    (
        r"iterate(4x^3-3x, x, 1/2, 1.5+1i)",
        r"1.832296845861300955415304813614722885458966320087182878544078075590923911513720159793363868052932542 + 4.000286937659053739513108820762919783860301973370043670949277963363291159477683707034819534392756175i",
    ),
    (
        r"iterate(4x^3-3x, x, 5/2, 1.5+1i)",
        r"-101736350.1050732586205160883484822893531315945759690546288068555279632447038242740493389373368455431 - 137133476.7298938939152329337942505412366433993061942816884035500136334840039000040949590749632459835i",
    ),
    (
        r"iterate(4x^3-3x, x, -1, 0.3)",
        r"0.912257438927936836637085038139303376465266462903788228641996521757601415803469443830513513884166887",
    ),
    (
        r"iterate(4x^3-3x, x, 3/10-7i/10, 1.5+1i)",
        r"2.90983251334591065683391920470765964789088998083347706061072344978842336244055317718024650624240092 - 1.725274750117168619949373444404632421431517361280584458433050088608667464581066106121150870450455401i",
    ),
    (
        r"iterate(8x^4-8x^2+1, x, -1, 0.3)",
        r"0.9503225175775471872170581790249493299584743952398100910069970160190615468836519031072662680151829807",
    ),
    (
        r"iterate(8x^4-8x^2+1, x, 3/10-7i/10, 1+1i)",
        r"cosh(4^(3/10 - 7i/10)*acosh(1 + i))|  ~ 3.328392758810388895441750241397620235753962087980488746360647912817901543928361993991745950064478336 - 1.984683891731306190766563229434688920261933086457689250610306580579038842372021439343486671847071357i",
    ),
    (r"iterate(sqrt(x), x, -1, 0.3)", r"0.09"),
    (
        r"iterate(sqrt(x), x, 3/10-7i/10, 2i)",
        r"(2i)^((1/2)^(3/10 - 7i/10))|  ~ 0.1621122204624011448061988781252256107826679895175562894216460432558208878746874037629925867880637082 + 0.8929336386202198026474229407690803322295768555361810629102454117746420311018379670561278178649489587i",
    ),
    (r"iterate(x+3, x, -1, 0.3)", r"-2.7"),
    (
        r"iterate(x+3, x, 3/10-7i/10, 1+1i)",
        r"19/10 - 11i/10|  ~ 1.9 - 1.1i",
    ),
    (
        r"iterate(x/(2+x), x, -1, 0.3)",
        r"0.8571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571",
    ),
    (
        r"iterate(x/(2+x), x, 3/10-7i/10, 2+1i)",
        r"2^(7i/10)*(7*2^(3/10) - 5*2^(7i/10) + i*2^(3/10))/(-14*2^(3/10 + 7i/10) + 10*2^(3/5) + 5*2^(7i/5))|  ~ 0.3382017636827491052412397630216174232007290620249121650862623933309140407299307056081070997127688871 + 0.8431201088117464669569155210980156212095964036318784437813474683221755502873421720564899260442287348i",
    ),
    (
        r"iterate(x^2, x, -1, -2)",
        r"i*sqrt(2)|  ~ 1.414213562373095048801688724209698078569671875376948073176679737990732478462107038850387534327641573i",
    ),
    (
        r"iterate(x^2, x, 3/10-7i/10, 1+1i)",
        r"(1 + i)^(2^(3/10 - 7i/10))|  ~ 1.813945566174843507317965514030523541868507130633909671547306111694723853164952676459922407345708513 + 1.397190770191190670239597894921080388133929805669316951455509460621533882110763954573351540400251464i",
    ),
    (
        r"iterate(x^2+2x, x, -1, -0.5)",
        r"-0.2928932188134524755991556378951509607151640623115259634116601310046337607689464805748062328361792136",
    ),
    (
        r"iterate(x^2+2x, x, 3/10-7i/10, -0.7+0.1i)",
        r"-0.817822000722659305526912120983038513794889097519717828625454842371438341350782359133394812709847793 + 0.2910092587997520809868010749420858131317271011555533149242799629071297524277545049461136222776252405i",
    ),
    (
        r"iterate(x^2-2, x, 1/2, 2.5+1i)",
        r"2.889458352646909983087034278835778979883421075164568592534643356226816788060853163957307527840875342 + 2.167630319534482585132945246137357552741052283460007464893999494736515948687516484562991304113688599i",
    ),
    (
        r"iterate(x^2-2, x, 5/2, 2.5+1i)",
        r"-156.1909320421649256431281996685323038151370963911694557887781609731864071412471037280024749492502448 + 41.34635945256105809598763924423578672328500953608158219050426484415655625809680999783328976641990266i",
    ),
    (
        r"iterate(x^3+3x, x, 1/2, 0.1)",
        r"0.1733493102125437517060983390429023461517463511794747920497068376125547742721284743646212045278995742",
    ),
    (
        r"iterate(x^3+3x, x, 5/2, 0.1)",
        r"1.720687021171446921648633174386092098917450285846989538104435861106082025973000929159349678998890273",
    ),
    (
        r"iterate(x^3+3x, x, -1, -0.3)",
        r"-0.09966995622352573823554525190729083875462746549782754079607767391665253350590932449491503815836739557",
    ),
    (
        r"iterate(x^3+3x, x, 3/10-7i/10, 1+1i)",
        r"2sinh(3^(3/10 - 7i/10)*asinh(1/2 + i/2))|  ~ 2.247076481604954339852106214296389120555805413525797512311881148020037541732307967949295672072099947 - 0.1842122334747438744609623520245788721171831015200893193813652873705260328677449180511191408295633039i",
    ),
    (
        r"iterate(x^3/2, x, -1, 0.5+0.5i)",
        r"1.0842150814913511818796660082610832038799929487881379698228290997936227373473917322952413513880637 + 0.2905145555072514445038131886249290736842462848382114649186862188810144845352821456584692447569532219i",
    ),
    (
        r"iterate(x^3/2, x, 3/10-7i/10, 3)",
        r"2^(1/2 - 3^(3/10 - 7i/10)/2)*3^(3^(3/10 - 7i/10))|  ~ 2.239688992531978337349496468446229303885373199693012852143985119570590672976775544244019040999085866 - 1.992942730170708710965922346085819322645263557881818298730390579956276204348232394273750197526785989i",
    ),
    (
        r"iterate(x^5+5x^3+5x, x, -1, -0.3)",
        r"-0.05978614860484017598122435565084635078290681765450851919533730341679648099411661731733635231858952977",
    ),
    (
        r"iterate(x^5+5x^3+5x, x, 3/10-7i/10, 1+1i)",
        r"2sinh(5^(3/10 - 7i/10)*asinh(1/2 + i/2))|  ~ 2.192330684312049774627805316987009316936521626357143205609306170469618566924539346743591098520716848 - 1.407928124878750960481796571738722024368583115001341475995339725522593637636367819329607541498904319i",
    ),
    (r"iterate(x/(1-x), x, -1, -3)", r"3/2|  ~ 1.5"),
    (
        r"iterate(x/(1-x), x, 3/10-7i/10, 0.3)",
        r"0.3130016051364365971107544141252006420545746388443017656500802568218298555377207062600321027287319422 - 0.07223113964686998394863563402889245585874799357945425361155698234349919743178170144462279293739967897i",
    ),
    (
        r"iterate(-2sinh(x), x, -1, 0.3)",
        r"-0.1494431201849576561602858091505913691121472320430276753884640525194980696796467976181611495603339009",
    ),
    (
        r"iterate(-2sinh(x), x, 3/10-7i/10, 1)",
        r"-913.7343677130317299969817657958508739861306176326329904786683157616583269127757038917618834711947184 + 625.5815826264436110710340440089741348717568504873028125043345624346472883984828563294887991654322904i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(2sinh(x), x, -1, 0.3)",
        r"0.1494431201849576561602858091505913691121472320430276753884640525194980696796467976181611495603339009",
    ),
    (
        r"iterate(2sinh(x), x, 3/10-7i/10, 1)",
        r"1.041009350128969922075084654246829086390637862562851752132993398192600871281809351825102720171521194 - 0.637494406437668651137771565895964840327826522913102000193748663395899978059587450368652662996007351i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, -12+3i, 0.3)",
        r"-0.00003549214595253163436256252255538827605679866624115225908947005192155369058951464327942234925002454326 + 0.00006365374943272813379535293593861302583849649796862093135003116926625216066882207706172679980241179826i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, -61/2, 0.3)",
        r"1.965863485258734075909899621304676909733795635879619700450017600861405262751630725713465305807200209e-10|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 1/2, -1e-80+1e-80i)",
        r"-1.414213562373095048801688724209698078569671875376948073176679737990732478462107038850387534327641573e-80 + 1.414213562373095048801688724209698078569671875376948073176679737990732478462107038850387534327641573e-80i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2sinh(x), x, 3/10-7i/10, 1e-50)",
        r"1.089046067753397527244389320049523601474574346580067547688919488441672082598268415527805557283665324e-50 - 5.741909341162226717603392046798648131001804854458356992820229097424771747600342546087650126395563124e-51i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(2x+e^x-1, x, 1/2, -1e-70)",
        r"-1.732050807568877293527446341505872366942805253810380628055806979451933006342556766821790299485968088e-70|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(2x+e^x-1, x, 3/10-7i/10, 1e-50)",
        r"9.991149641745515664971822254800375876300753412510698919545364263621193437365399231389848309463627087e-51 - 9.669288150087612790505070858112196135216173899632556089603777070719534269910681107001312597924908271e-51i|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"iterate(e^x, x, 1/2, 0.5, 0.3+1.3i)",
        r"0.9930391928001193189452279577651841026193251416239540378972259558530393576085194185378117021025524037 + 0.1311428457124288302157160386368293672250841628644831088702997614511362486722807324672609476266720421i|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i)",
    ),
    (
        r"iterate(e^x, x, 3/10-7i/10, 1+1i, 0.3+1.3i)",
        r"1.763184134458171920758564039826377889701447147075664236105754721915149454000354947272733803218324292 + 0.3752662247760010661475281066454587869054580451401479980573212802609474127885922814321562031202320611i|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i)",
    ),
    (
        r"iterate(-2x+x^3/1000, x, -1, 0.3, 0)",
        r"-0.1500016875569556880227814276088013255384565804343821039962339924537728869573543301177421123436287952|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(-2x+x^3/1000, x, 3/10-7i/10, 1, 0)",
        r"9.91545467546977882888354672575202717863205527303373444559374724718876507290874167629869774693483909 + 4.680569740842369531076408408139591012627435832612884192588770520714896551916244497083685964017419427i|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"iterate(sqrt(2)^x, x, -1, 10, 4)",
        r"6.64385618977472469574063885897878035172966278604916122410951279163186955321725043170027948671874031|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"iterate(sqrt(2)^x, x, 3/10-7i/10, 4.5+0.5i, 4)",
        r"4.729417501288061624150978421766888570412500751887551774761701690666827803968644900061191956377103458 + 0.4118900490335895278959049279678462933642863791560973859452002112688934107304838272929697143836456352i|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"iterate(x^2+1, x, 1/2, 0.5+0.5i, 0.5+0.9i)",
        r"0.7286614214571103816919658158480681748264164339887705479526159580189398139764326657565000979194828815 + 0.4509065555541797596486952897803576092662872990260359685979962628516205059203188800002458197581736151i|note: iterate: regular iteration at the repelling fixed point 0.5 + 0.8660254038i (slope 1 + 1.732050808i)",
    ),
    (
        r"iterate(x^2+1, x, 3/10-7i/10, 1, 0.5+0.9i)",
        r"-7.791096329722537179988107323904338189071511613458791768119540254140419265550973569291768083557675788 + 4.621937906303621393824980863875977283007305230745831141909127341073709419825224174812700277621271448i|note: iterate: regular iteration at the repelling fixed point 0.5 + 0.8660254038i (slope 1 + 1.732050808i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
];

const WITH_100_DIGITS_3: &[(&str, &str)] = &[
    (
        r"iterate(x^2-1, x, 1/2, -0.3)",
        r"-0.881594629841445298549970767920257118415740213295674736306021801155255528412331321236535821660644897|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8366600265",
    ),
    (
        r"iterate(x^2-1, x, 3/10-7i/10, -1.3)",
        r"-5.63969993210599264563639309978731745680137986891688434048686845315974190442385136753145857579474595 - 1.055225559807679621161182441229309390330783996576488892454458616912278376798314716063023619294989065i|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.5477225575i; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2-1, x, 1/2, -0.3, -0.6)",
        r"-0.674294037505668149408180955678850060902416218941856502103009680626492361130504709077639568652327735 + 0.4117467559770605637580669530273504745541103258392853238785406359229432544180772214186081630023761085i|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 3/10-7i/10, -0.5, -0.6)",
        r"0.1144533780349492244186765635782752441735045672165718026291101564517227755766214959637818939013276465 - 0.1215981108518542656932997158418105806803511201037266706939777460960706491430682297551876104303118168i|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"iterate(x^2-1, x, 1/2, (1+sqrt(5))/2+1e-60i, 1.6)",
        r"1.618033988749894848204586834365638117720309179805762862135448622705260462818902449707207204189391137 + 1.798907439947867272261227583624255638327544983625213870595405432296607439800533079612582401441931907e-60i|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 3/10-7i/10, (1+sqrt(5))/2-1e-40, 1.6)",
        r"1.618033988749894848204586834365638117720212357865213830132029263247169632048503909120779220296667168 + 1.041930733207431883741629052588545671368641145026937805262651392822679101597385217129425366585794645e-40i|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)",
    ),
    (
        r"iterate(x^2-1, x, 1/2, -0.2+0.1i, 1.6)",
        r"-0.8216528637108891508253693119565156141918156977589920801661443511769734960326077732475323527379255889 + 0.07090175021879975594123241599946837458861708109556112700320112703841735493914625674851579928031345397i|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8961656585 + 0.055793256i",
    ),
    (
        r"iterate(x^2-1, x, 3/10-7i/10, -0.5, 1.6)",
        r"-1.296949269097483790230306922959512825716171347046614651037505720055783887261565849357370775010329059 + 0.9635823403177041714326390995015704910130349238633272125089476518038576164100231873406246964820589344i|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.7071067812",
    ),
    (
        r"iterate(x^3+2x, x, -1, 0.1, 0)",
        r"0.0499377332097974812273254619326194342976591171248325726468396259714843204471048341235519108388370605|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(x^3+2x, x, 3/10-7i/10, 0.5+0.5i, 0)",
        r"0.9264565273206569528473610847116638670363506374565036368169289329075497209153914674330631073491869318 + 0.2596485329644569882075840267995835312176164785622490993136462228869778531098763108483127663790778869i|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, -1, 0.02i)",
        r"0.01644818077560765206290516869349784006986231158167296572504151069320280384032283843001057167165891982 + 0.01074554683140063723926386569349820155784443344163692281435552633705806430436304596906294262542559463i",
    ),
    (
        r"iterate(e^(i)*x+x^2, x, 3/10-7i/10, 0.02i)",
        r"-0.0113510477792098461811547954665603085233363896497979856370885956007711818008706393435204844379670951 + 0.03919522470776867872962709440377715881033304147767293920902913080482041898297268361789667945290104723i|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
];

const STEPS_ADD_UP_1: &[(&str, &str)] = &[
    (
        r"abs(iterate(2x+1, x, 1/2, iterate(2x+1, x, 1/2, 0.3)) - (iterate(2x+1, x, 1, 0.3))) < 10^-27*(1 + abs(iterate(2x+1, x, 1, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate(2x+1, x, 5/2, iterate(2x+1, x, -3/2, 0.3)) - (iterate(2x+1, x, 1, 0.3))) < 10^-27*(1 + abs(iterate(2x+1, x, 1, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate(2x+1, x, 1/2, iterate(2x+1, x, 1/2, -2)) - (iterate(2x+1, x, 1, -2))) < 10^-27*(1 + abs(iterate(2x+1, x, 1, -2)))",
        r"true",
    ),
    (
        r"abs(iterate(2x+1, x, 5/2, iterate(2x+1, x, -3/2, -2)) - (iterate(2x+1, x, 1, -2))) < 10^-27*(1 + abs(iterate(2x+1, x, 1, -2)))",
        r"true",
    ),
    (
        r"abs(iterate(2x+1, x, 1/2, iterate(2x+1, x, 1/2, 1+i)) - (iterate(2x+1, x, 1, 1+i))) < 10^-27*(1 + abs(iterate(2x+1, x, 1, 1+i)))",
        r"true",
    ),
    (
        r"abs(iterate(2x+1, x, 5/2, iterate(2x+1, x, -3/2, 1+i)) - (iterate(2x+1, x, 1, 1+i))) < 10^-27*(1 + abs(iterate(2x+1, x, 1, 1+i)))",
        r"true",
    ),
    (
        r"abs(iterate(x/3-2, x, 1/2, iterate(x/3-2, x, 1/2, 0.3)) - (iterate(x/3-2, x, 1, 0.3))) < 10^-27*(1 + abs(iterate(x/3-2, x, 1, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate(x/3-2, x, 1/2, iterate(x/3-2, x, 3/2, 0.3)) - (iterate(x/3-2, x, 2, 0.3))) < 10^-27*(1 + abs(iterate(x/3-2, x, 2, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate(x/3-2, x, 1, iterate(x/3-2, x, 1/2, 0.3)) - (iterate(x/3-2, x, 3/2, 0.3))) < 10^-27*(1 + abs(iterate(x/3-2, x, 3/2, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate(x/3-2, x, 1/2, iterate(x/3-2, x, 1/2, 7)) - (iterate(x/3-2, x, 1, 7))) < 10^-27*(1 + abs(iterate(x/3-2, x, 1, 7)))",
        r"true",
    ),
    (
        r"abs(iterate(x/3-2, x, 1/2, iterate(x/3-2, x, 3/2, 7)) - (iterate(x/3-2, x, 2, 7))) < 10^-27*(1 + abs(iterate(x/3-2, x, 2, 7)))",
        r"true",
    ),
    (
        r"abs(iterate(x/3-2, x, 1, iterate(x/3-2, x, 1/2, 7)) - (iterate(x/3-2, x, 3/2, 7))) < 10^-27*(1 + abs(iterate(x/3-2, x, 3/2, 7)))",
        r"true",
    ),
    (
        r"abs(iterate((1+i)*x+2, x, 1/2, iterate((1+i)*x+2, x, 1/2, 0.3)) - (iterate((1+i)*x+2, x, 1, 0.3))) < 10^-27*(1 + abs(iterate((1+i)*x+2, x, 1, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate((1+i)*x+2, x, 1/2, iterate((1+i)*x+2, x, 3/2, 0.3)) - (iterate((1+i)*x+2, x, 2, 0.3))) < 10^-27*(1 + abs(iterate((1+i)*x+2, x, 2, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate((1+i)*x+2, x, 1, iterate((1+i)*x+2, x, 1/2, 0.3)) - (iterate((1+i)*x+2, x, 3/2, 0.3))) < 10^-27*(1 + abs(iterate((1+i)*x+2, x, 3/2, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate((1+i)*x+2, x, 1/2, iterate((1+i)*x+2, x, 1/2, 1-i)) - (iterate((1+i)*x+2, x, 1, 1-i))) < 10^-27*(1 + abs(iterate((1+i)*x+2, x, 1, 1-i)))",
        r"true",
    ),
    (
        r"abs(iterate((1+i)*x+2, x, 1/2, iterate((1+i)*x+2, x, 3/2, 1-i)) - (iterate((1+i)*x+2, x, 2, 1-i))) < 10^-27*(1 + abs(iterate((1+i)*x+2, x, 2, 1-i)))",
        r"true",
    ),
    (
        r"abs(iterate((1+i)*x+2, x, 1, iterate((1+i)*x+2, x, 1/2, 1-i)) - (iterate((1+i)*x+2, x, 3/2, 1-i))) < 10^-27*(1 + abs(iterate((1+i)*x+2, x, 3/2, 1-i)))",
        r"true",
    ),
    (
        r"abs(iterate(x+3, x, 1/2, iterate(x+3, x, 1/2, 0.3)) - (iterate(x+3, x, 1, 0.3))) < 10^-27*(1 + abs(iterate(x+3, x, 1, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate(x+3, x, 1/2, iterate(x+3, x, 3/2, 0.3)) - (iterate(x+3, x, 2, 0.3))) < 10^-27*(1 + abs(iterate(x+3, x, 2, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate(x+3, x, 1, iterate(x+3, x, 1/2, 0.3)) - (iterate(x+3, x, 3/2, 0.3))) < 10^-27*(1 + abs(iterate(x+3, x, 3/2, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate(x+3, x, 1/2, iterate(x+3, x, 1/2, 1+i)) - (iterate(x+3, x, 1, 1+i))) < 10^-27*(1 + abs(iterate(x+3, x, 1, 1+i)))",
        r"true",
    ),
    (
        r"abs(iterate(x+3, x, 1/2, iterate(x+3, x, 3/2, 1+i)) - (iterate(x+3, x, 2, 1+i))) < 10^-27*(1 + abs(iterate(x+3, x, 2, 1+i)))",
        r"true",
    ),
    (
        r"abs(iterate(x+3, x, 1, iterate(x+3, x, 1/2, 1+i)) - (iterate(x+3, x, 3/2, 1+i))) < 10^-27*(1 + abs(iterate(x+3, x, 3/2, 1+i)))",
        r"true",
    ),
    (
        r"abs(iterate(x^2, x, 1/2, iterate(x^2, x, 1/2, 3)) - (iterate(x^2, x, 1, 3))) < 10^-27*(1 + abs(iterate(x^2, x, 1, 3)))",
        r"true",
    ),
    (
        r"abs(iterate(x^2, x, 1/2, iterate(x^2, x, -1/2, 3)) - (3)) < 10^-27*(1 + abs(3))",
        r"true",
    ),
    (
        r"abs(iterate(x^2, x, 1/2, iterate(x^2, x, 1/2, 1.5)) - (iterate(x^2, x, 1, 1.5))) < 10^-27*(1 + abs(iterate(x^2, x, 1, 1.5)))",
        r"true",
    ),
    (
        r"abs(iterate(x^2, x, 1/2, iterate(x^2, x, -1/2, 1.5)) - (1.5)) < 10^-27*(1 + abs(1.5))",
        r"true",
    ),
    (
        r"abs(iterate(x^2, x, 1/2, iterate(x^2, x, 1/2, 0.3)) - (iterate(x^2, x, 1, 0.3))) < 10^-27*(1 + abs(iterate(x^2, x, 1, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate(x^2, x, 1/2, iterate(x^2, x, -1/2, 0.3)) - (0.3)) < 10^-27*(1 + abs(0.3))",
        r"true",
    ),
    (
        r"abs(iterate(3x^2, x, 1/2, iterate(3x^2, x, 1/2, 3)) - (iterate(3x^2, x, 1, 3))) < 10^-27*(1 + abs(iterate(3x^2, x, 1, 3)))",
        r"true",
    ),
    (
        r"abs(iterate(3x^2, x, 3/10, iterate(3x^2, x, 9/20, 3)) - (iterate(3x^2, x, 3/4, 3))) < 10^-27*(1 + abs(iterate(3x^2, x, 3/4, 3)))",
        r"true",
    ),
    (
        r"abs(iterate(3x^2, x, 1/3, iterate(3x^2, x, -1/3, 3)) - (3)) < 10^-27*(1 + abs(3))",
        r"true",
    ),
    (
        r"abs(iterate(3x^2, x, 1/2, iterate(3x^2, x, 1/2, 0.1)) - (iterate(3x^2, x, 1, 0.1))) < 10^-27*(1 + abs(iterate(3x^2, x, 1, 0.1)))",
        r"true",
    ),
    (
        r"abs(iterate(3x^2, x, 3/10, iterate(3x^2, x, 9/20, 0.1)) - (iterate(3x^2, x, 3/4, 0.1))) < 10^-27*(1 + abs(iterate(3x^2, x, 3/4, 0.1)))",
        r"true",
    ),
    (
        r"abs(iterate(3x^2, x, 1/3, iterate(3x^2, x, -1/3, 0.1)) - (0.1)) < 10^-27*(1 + abs(0.1))",
        r"true",
    ),
    (
        r"abs(iterate(sqrt(x), x, 1/2, iterate(sqrt(x), x, 1/2, 3)) - (iterate(sqrt(x), x, 1, 3))) < 10^-27*(1 + abs(iterate(sqrt(x), x, 1, 3)))",
        r"true",
    ),
    (
        r"abs(iterate(sqrt(x), x, 3/10, iterate(sqrt(x), x, 9/20, 3)) - (iterate(sqrt(x), x, 3/4, 3))) < 10^-27*(1 + abs(iterate(sqrt(x), x, 3/4, 3)))",
        r"true",
    ),
    (
        r"abs(iterate(sqrt(x), x, 1/3, iterate(sqrt(x), x, -1/3, 3)) - (3)) < 10^-27*(1 + abs(3))",
        r"true",
    ),
    (
        r"abs(iterate(sqrt(x), x, 1/2, iterate(sqrt(x), x, 1/2, 0.3)) - (iterate(sqrt(x), x, 1, 0.3))) < 10^-27*(1 + abs(iterate(sqrt(x), x, 1, 0.3)))",
        r"true",
    ),
];

const STEPS_ADD_UP_2: &[(&str, &str)] = &[
    (
        r"abs(iterate(sqrt(x), x, 3/10, iterate(sqrt(x), x, 9/20, 0.3)) - (iterate(sqrt(x), x, 3/4, 0.3))) < 10^-27*(1 + abs(iterate(sqrt(x), x, 3/4, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate(sqrt(x), x, 1/3, iterate(sqrt(x), x, -1/3, 0.3)) - (0.3)) < 10^-27*(1 + abs(0.3))",
        r"true",
    ),
    (
        r"abs(iterate(x^3/2, x, 1/2, iterate(x^3/2, x, 1/2, 3)) - (iterate(x^3/2, x, 1, 3))) < 10^-27*(1 + abs(iterate(x^3/2, x, 1, 3)))",
        r"true",
    ),
    (
        r"abs(iterate(x^3/2, x, 3/10, iterate(x^3/2, x, 9/20, 3)) - (iterate(x^3/2, x, 3/4, 3))) < 10^-27*(1 + abs(iterate(x^3/2, x, 3/4, 3)))",
        r"true",
    ),
    (
        r"abs(iterate(x^3/2, x, 1/3, iterate(x^3/2, x, -1/3, 3)) - (3)) < 10^-27*(1 + abs(3))",
        r"true",
    ),
    (
        r"abs(iterate(x^3/2, x, 1/2, iterate(x^3/2, x, 1/2, 2)) - (iterate(x^3/2, x, 1, 2))) < 10^-27*(1 + abs(iterate(x^3/2, x, 1, 2)))",
        r"true",
    ),
    (
        r"abs(iterate(x^3/2, x, 3/10, iterate(x^3/2, x, 9/20, 2)) - (iterate(x^3/2, x, 3/4, 2))) < 10^-27*(1 + abs(iterate(x^3/2, x, 3/4, 2)))",
        r"true",
    ),
    (
        r"abs(iterate(x^3/2, x, 1/3, iterate(x^3/2, x, -1/3, 2)) - (2)) < 10^-27*(1 + abs(2))",
        r"true",
    ),
    (
        r"abs(iterate(1/(1+x), x, 1/2, iterate(1/(1+x), x, 1/2, 1)) - (iterate(1/(1+x), x, 1, 1))) < 10^-27*(1 + abs(iterate(1/(1+x), x, 1, 1)))",
        r"true",
    ),
    (
        r"abs(iterate(1/(1+x), x, 3/10, iterate(1/(1+x), x, 9/20, 1)) - (iterate(1/(1+x), x, 3/4, 1))) < 10^-27*(1 + abs(iterate(1/(1+x), x, 3/4, 1)))",
        r"true",
    ),
    (
        r"abs(iterate(1/(1+x), x, 1/3, iterate(1/(1+x), x, -1/3, 1)) - (1)) < 10^-27*(1 + abs(1))",
        r"true",
    ),
    (
        r"abs(iterate(1/(1+x), x, 1/2, iterate(1/(1+x), x, 1/2, 0.3)) - (iterate(1/(1+x), x, 1, 0.3))) < 10^-27*(1 + abs(iterate(1/(1+x), x, 1, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate(1/(1+x), x, 3/10, iterate(1/(1+x), x, 9/20, 0.3)) - (iterate(1/(1+x), x, 3/4, 0.3))) < 10^-27*(1 + abs(iterate(1/(1+x), x, 3/4, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate(1/(1+x), x, 1/3, iterate(1/(1+x), x, -1/3, 0.3)) - (0.3)) < 10^-27*(1 + abs(0.3))",
        r"true",
    ),
    (
        r"abs(iterate((2x+1)/(x+3), x, 1/2, iterate((2x+1)/(x+3), x, 1/2, 1)) - (iterate((2x+1)/(x+3), x, 1, 1))) < 10^-27*(1 + abs(iterate((2x+1)/(x+3), x, 1, 1)))",
        r"true",
    ),
    (
        r"abs(iterate((2x+1)/(x+3), x, 1/3, iterate((2x+1)/(x+3), x, -1/3, 1)) - (1)) < 10^-27*(1 + abs(1))",
        r"true",
    ),
    (
        r"abs(iterate((2x+1)/(x+3), x, 1/3+i/5, iterate((2x+1)/(x+3), x, 2/3-i/5, 1)) - (iterate((2x+1)/(x+3), x, 1, 1))) < 10^-27*(1 + abs(iterate((2x+1)/(x+3), x, 1, 1)))",
        r"true",
    ),
    (
        r"abs(iterate((2x+1)/(x+3), x, 1/3, iterate((2x+1)/(x+3), x, -1/3, 0)) - (0)) < 10^-27*(1 + abs(0))",
        r"true",
    ),
    (
        r"abs(iterate((2x+1)/(x+3), x, 1/3+i/5, iterate((2x+1)/(x+3), x, 2/3-i/5, 0)) - (iterate((2x+1)/(x+3), x, 1, 0))) < 10^-27*(1 + abs(iterate((2x+1)/(x+3), x, 1, 0)))",
        r"true",
    ),
    (
        r"abs(iterate((2x+1)/(x+3), x, 1/3, iterate((2x+1)/(x+3), x, -1/3, 2+i)) - (2+i)) < 10^-27*(1 + abs(2+i))",
        r"true",
    ),
    (
        r"abs(iterate(x/(2+x), x, 1/2, iterate(x/(2+x), x, 1/2, 1)) - (iterate(x/(2+x), x, 1, 1))) < 10^-27*(1 + abs(iterate(x/(2+x), x, 1, 1)))",
        r"true",
    ),
    (
        r"abs(iterate(x/(2+x), x, 1/2, iterate(x/(2+x), x, 3/2, 1)) - (iterate(x/(2+x), x, 2, 1))) < 10^-27*(1 + abs(iterate(x/(2+x), x, 2, 1)))",
        r"true",
    ),
    (
        r"abs(iterate(x/(2+x), x, 1, iterate(x/(2+x), x, 1/2, 1)) - (iterate(x/(2+x), x, 3/2, 1))) < 10^-27*(1 + abs(iterate(x/(2+x), x, 3/2, 1)))",
        r"true",
    ),
    (
        r"abs(iterate(x/(2+x), x, 1/2, iterate(x/(2+x), x, 1/2, 0.3)) - (iterate(x/(2+x), x, 1, 0.3))) < 10^-27*(1 + abs(iterate(x/(2+x), x, 1, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate(x/(2+x), x, 1/2, iterate(x/(2+x), x, 3/2, 0.3)) - (iterate(x/(2+x), x, 2, 0.3))) < 10^-27*(1 + abs(iterate(x/(2+x), x, 2, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate(x/(2+x), x, 1, iterate(x/(2+x), x, 1/2, 0.3)) - (iterate(x/(2+x), x, 3/2, 0.3))) < 10^-27*(1 + abs(iterate(x/(2+x), x, 3/2, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate(x/(1+x), x, 1/2, iterate(x/(1+x), x, 1/2, 1)) - (iterate(x/(1+x), x, 1, 1))) < 10^-27*(1 + abs(iterate(x/(1+x), x, 1, 1)))",
        r"true",
    ),
    (
        r"abs(iterate(x/(1+x), x, 5/2, iterate(x/(1+x), x, -3/2, 1)) - (iterate(x/(1+x), x, 1, 1))) < 10^-27*(1 + abs(iterate(x/(1+x), x, 1, 1)))",
        r"true",
    ),
    (
        r"abs(iterate(x/(1+x), x, 1/2, iterate(x/(1+x), x, 1/2, 0.3)) - (iterate(x/(1+x), x, 1, 0.3))) < 10^-27*(1 + abs(iterate(x/(1+x), x, 1, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate(x/(1+x), x, 5/2, iterate(x/(1+x), x, -3/2, 0.3)) - (iterate(x/(1+x), x, 1, 0.3))) < 10^-27*(1 + abs(iterate(x/(1+x), x, 1, 0.3)))",
        r"true",
    ),
    (
        r"abs(iterate(x/(1+x), x, 1/2, iterate(x/(1+x), x, 1/2, 1+i)) - (iterate(x/(1+x), x, 1, 1+i))) < 10^-27*(1 + abs(iterate(x/(1+x), x, 1, 1+i)))",
        r"true",
    ),
    (
        r"abs(iterate(x/(1+x), x, 5/2, iterate(x/(1+x), x, -3/2, 1+i)) - (iterate(x/(1+x), x, 1, 1+i))) < 10^-27*(1 + abs(iterate(x/(1+x), x, 1, 1+i)))",
        r"true",
    ),
    (
        r"abs(iterate(x^2+2x, x, 1/2, iterate(x^2+2x, x, 1/2, 0.5)) - (iterate(x^2+2x, x, 1, 0.5))) < 10^-27*(1 + abs(iterate(x^2+2x, x, 1, 0.5)))",
        r"true",
    ),
    (
        r"abs(iterate(x^2+2x, x, 3/10, iterate(x^2+2x, x, 9/20, 0.5)) - (iterate(x^2+2x, x, 3/4, 0.5))) < 10^-27*(1 + abs(iterate(x^2+2x, x, 3/4, 0.5)))",
        r"true",
    ),
    (
        r"abs(iterate(x^2+2x, x, 1/3, iterate(x^2+2x, x, -1/3, 0.5)) - (0.5)) < 10^-27*(1 + abs(0.5))",
        r"true",
    ),
    (
        r"abs(iterate(x^2+2x, x, 1/2, iterate(x^2+2x, x, 1/2, 2)) - (iterate(x^2+2x, x, 1, 2))) < 10^-27*(1 + abs(iterate(x^2+2x, x, 1, 2)))",
        r"true",
    ),
    (
        r"abs(iterate(x^2+2x, x, 3/10, iterate(x^2+2x, x, 9/20, 2)) - (iterate(x^2+2x, x, 3/4, 2))) < 10^-27*(1 + abs(iterate(x^2+2x, x, 3/4, 2)))",
        r"true",
    ),
    (
        r"abs(iterate(x^2+2x, x, 1/3, iterate(x^2+2x, x, -1/3, 2)) - (2)) < 10^-27*(1 + abs(2))",
        r"true",
    ),
    (
        r"abs(iterate(x^2-2, x, 1/2, iterate(x^2-2, x, 1/2, 3)) - (iterate(x^2-2, x, 1, 3))) < 10^-27*(1 + abs(iterate(x^2-2, x, 1, 3)))",
        r"true",
    ),
    (
        r"abs(iterate(x^2-2, x, 3/10, iterate(x^2-2, x, 9/20, 3)) - (iterate(x^2-2, x, 3/4, 3))) < 10^-27*(1 + abs(iterate(x^2-2, x, 3/4, 3)))",
        r"true",
    ),
];

const STEPS_ADD_UP_3: &[(&str, &str)] = &[
    (
        r"abs(iterate(x^2-2, x, 1/3, iterate(x^2-2, x, -1/3, 3)) - (3)) < 10^-27*(1 + abs(3))",
        r"true",
    ),
    (
        r"abs(iterate(x^2-2, x, 1/2, iterate(x^2-2, x, 1/2, 5)) - (iterate(x^2-2, x, 1, 5))) < 10^-27*(1 + abs(iterate(x^2-2, x, 1, 5)))",
        r"true",
    ),
    (
        r"abs(iterate(x^2-2, x, 3/10, iterate(x^2-2, x, 9/20, 5)) - (iterate(x^2-2, x, 3/4, 5))) < 10^-27*(1 + abs(iterate(x^2-2, x, 3/4, 5)))",
        r"true",
    ),
    (
        r"abs(iterate(x^2-2, x, 1/3, iterate(x^2-2, x, -1/3, 5)) - (5)) < 10^-27*(1 + abs(5))",
        r"true",
    ),
    (
        r"abs(iterate(2x^2-1, x, 1/2, iterate(2x^2-1, x, 1/2, 3)) - (iterate(2x^2-1, x, 1, 3))) < 10^-27*(1 + abs(iterate(2x^2-1, x, 1, 3)))",
        r"true",
    ),
    (
        r"abs(iterate(2x^2-1, x, 3/10, iterate(2x^2-1, x, 9/20, 3)) - (iterate(2x^2-1, x, 3/4, 3))) < 10^-27*(1 + abs(iterate(2x^2-1, x, 3/4, 3)))",
        r"true",
    ),
    (
        r"abs(iterate(2x^2-1, x, 1/3, iterate(2x^2-1, x, -1/3, 3)) - (3)) < 10^-27*(1 + abs(3))",
        r"true",
    ),
    (
        r"abs(iterate(2x^2-1, x, 1/2, iterate(2x^2-1, x, 1/2, 1.5)) - (iterate(2x^2-1, x, 1, 1.5))) < 10^-27*(1 + abs(iterate(2x^2-1, x, 1, 1.5)))",
        r"true",
    ),
    (
        r"abs(iterate(2x^2-1, x, 3/10, iterate(2x^2-1, x, 9/20, 1.5)) - (iterate(2x^2-1, x, 3/4, 1.5))) < 10^-27*(1 + abs(iterate(2x^2-1, x, 3/4, 1.5)))",
        r"true",
    ),
    (
        r"abs(iterate(2x^2-1, x, 1/3, iterate(2x^2-1, x, -1/3, 1.5)) - (1.5)) < 10^-27*(1 + abs(1.5))",
        r"true",
    ),
    (
        r"abs(iterate(4x^3-3x, x, 1/2, iterate(4x^3-3x, x, 1/2, 2)) - (iterate(4x^3-3x, x, 1, 2))) < 10^-27*(1 + abs(iterate(4x^3-3x, x, 1, 2)))",
        r"true",
    ),
    (
        r"abs(iterate(4x^3-3x, x, 3/10, iterate(4x^3-3x, x, 9/20, 2)) - (iterate(4x^3-3x, x, 3/4, 2))) < 10^-27*(1 + abs(iterate(4x^3-3x, x, 3/4, 2)))",
        r"true",
    ),
    (
        r"abs(iterate(4x^3-3x, x, 1/3, iterate(4x^3-3x, x, -1/3, 2)) - (2)) < 10^-27*(1 + abs(2))",
        r"true",
    ),
    (
        r"abs(iterate(4x^3-3x, x, 1/2, iterate(4x^3-3x, x, 1/2, 1.5)) - (iterate(4x^3-3x, x, 1, 1.5))) < 10^-27*(1 + abs(iterate(4x^3-3x, x, 1, 1.5)))",
        r"true",
    ),
    (
        r"abs(iterate(4x^3-3x, x, 3/10, iterate(4x^3-3x, x, 9/20, 1.5)) - (iterate(4x^3-3x, x, 3/4, 1.5))) < 10^-27*(1 + abs(iterate(4x^3-3x, x, 3/4, 1.5)))",
        r"true",
    ),
    (
        r"abs(iterate(4x^3-3x, x, 1/3, iterate(4x^3-3x, x, -1/3, 1.5)) - (1.5)) < 10^-27*(1 + abs(1.5))",
        r"true",
    ),
    (
        r"abs(iterate(x^3+3x, x, 1/2, iterate(x^3+3x, x, 1/2, 2)) - (iterate(x^3+3x, x, 1, 2))) < 10^-27*(1 + abs(iterate(x^3+3x, x, 1, 2)))",
        r"true",
    ),
    (
        r"abs(iterate(x^3+3x, x, 5/2, iterate(x^3+3x, x, -3/2, 2)) - (iterate(x^3+3x, x, 1, 2))) < 10^-27*(1 + abs(iterate(x^3+3x, x, 1, 2)))",
        r"true",
    ),
    (
        r"abs(iterate(x^3+3x, x, 1/2, iterate(x^3+3x, x, 1/2, 0.1)) - (iterate(x^3+3x, x, 1, 0.1))) < 10^-27*(1 + abs(iterate(x^3+3x, x, 1, 0.1)))",
        r"true",
    ),
    (
        r"abs(iterate(x^3+3x, x, 5/2, iterate(x^3+3x, x, -3/2, 0.1)) - (iterate(x^3+3x, x, 1, 0.1))) < 10^-27*(1 + abs(iterate(x^3+3x, x, 1, 0.1)))",
        r"true",
    ),
    (
        r"abs(iterate(x^3+3x, x, 1/2, iterate(x^3+3x, x, 1/2, -0.7)) - (iterate(x^3+3x, x, 1, -0.7))) < 10^-27*(1 + abs(iterate(x^3+3x, x, 1, -0.7)))",
        r"true",
    ),
    (
        r"abs(iterate(x^3+3x, x, 5/2, iterate(x^3+3x, x, -3/2, -0.7)) - (iterate(x^3+3x, x, 1, -0.7))) < 10^-27*(1 + abs(iterate(x^3+3x, x, 1, -0.7)))",
        r"true",
    ),
    (
        r"abs(iterate(x^5+5x^3+5x, x, 1/2, iterate(x^5+5x^3+5x, x, 1/2, 0.4)) - (iterate(x^5+5x^3+5x, x, 1, 0.4))) < 10^-27*(1 + abs(iterate(x^5+5x^3+5x, x, 1, 0.4)))",
        r"true",
    ),
    (
        r"abs(iterate(x^5+5x^3+5x, x, 3/10, iterate(x^5+5x^3+5x, x, 9/20, 0.4)) - (iterate(x^5+5x^3+5x, x, 3/4, 0.4))) < 10^-27*(1 + abs(iterate(x^5+5x^3+5x, x, 3/4, 0.4)))",
        r"true",
    ),
    (
        r"abs(iterate(x^5+5x^3+5x, x, 1/3, iterate(x^5+5x^3+5x, x, -1/3, 0.4)) - (0.4)) < 10^-27*(1 + abs(0.4))",
        r"true",
    ),
    (
        r"abs(iterate(x^5+5x^3+5x, x, 1/2, iterate(x^5+5x^3+5x, x, 1/2, -1)) - (iterate(x^5+5x^3+5x, x, 1, -1))) < 10^-27*(1 + abs(iterate(x^5+5x^3+5x, x, 1, -1)))",
        r"true",
    ),
    (
        r"abs(iterate(x^5+5x^3+5x, x, 3/10, iterate(x^5+5x^3+5x, x, 9/20, -1)) - (iterate(x^5+5x^3+5x, x, 3/4, -1))) < 10^-27*(1 + abs(iterate(x^5+5x^3+5x, x, 3/4, -1)))",
        r"true",
    ),
    (
        r"abs(iterate(x^5+5x^3+5x, x, 1/3, iterate(x^5+5x^3+5x, x, -1/3, -1)) - (-1)) < 10^-27*(1 + abs(-1))",
        r"true",
    ),
    (
        r"abs(iterate(x^3+3x^2+3x, x, 1/2, iterate(x^3+3x^2+3x, x, 1/2, 0.5)) - (iterate(x^3+3x^2+3x, x, 1, 0.5))) < 10^-27*(1 + abs(iterate(x^3+3x^2+3x, x, 1, 0.5)))",
        r"true",
    ),
    (
        r"abs(iterate(x^3+3x^2+3x, x, 3/10, iterate(x^3+3x^2+3x, x, 9/20, 0.5)) - (iterate(x^3+3x^2+3x, x, 3/4, 0.5))) < 10^-27*(1 + abs(iterate(x^3+3x^2+3x, x, 3/4, 0.5)))",
        r"true",
    ),
    (
        r"abs(iterate(x^3+3x^2+3x, x, 1/3, iterate(x^3+3x^2+3x, x, -1/3, 0.5)) - (0.5)) < 10^-27*(1 + abs(0.5))",
        r"true",
    ),
    (
        r"abs(iterate(x^3+3x^2+3x, x, 1/2, iterate(x^3+3x^2+3x, x, 1/2, 2)) - (iterate(x^3+3x^2+3x, x, 1, 2))) < 10^-27*(1 + abs(iterate(x^3+3x^2+3x, x, 1, 2)))",
        r"true",
    ),
    (
        r"abs(iterate(x^3+3x^2+3x, x, 3/10, iterate(x^3+3x^2+3x, x, 9/20, 2)) - (iterate(x^3+3x^2+3x, x, 3/4, 2))) < 10^-27*(1 + abs(iterate(x^3+3x^2+3x, x, 3/4, 2)))",
        r"true",
    ),
    (
        r"abs(iterate(x^3+3x^2+3x, x, 1/3, iterate(x^3+3x^2+3x, x, -1/3, 2)) - (2)) < 10^-27*(1 + abs(2))",
        r"true",
    ),
    (
        r"abs(iterate(e^x, x, 1/2, iterate(e^x, x, 1/2, 0.5)) - (iterate(e^x, x, 1, 0.5))) < 10^-27*(1 + abs(iterate(e^x, x, 1, 0.5)))",
        r"true",
    ),
    (
        r"abs(iterate(e^x, x, 1/3, iterate(e^x, x, 2/3, 0.5)) - (iterate(e^x, x, 1, 0.5))) < 10^-27*(1 + abs(iterate(e^x, x, 1, 0.5)))",
        r"true",
    ),
    (
        r"abs(iterate(e^x, x, 1/2, iterate(e^x, x, 1/2, 1)) - (iterate(e^x, x, 1, 1))) < 10^-27*(1 + abs(iterate(e^x, x, 1, 1)))",
        r"true",
    ),
    (
        r"abs(iterate(e^x, x, 1/3, iterate(e^x, x, 2/3, 1)) - (iterate(e^x, x, 1, 1))) < 10^-27*(1 + abs(iterate(e^x, x, 1, 1)))",
        r"true",
    ),
    (
        r"abs(iterate(e^x, x, 1/2, iterate(e^x, x, 1/2, 2)) - (iterate(e^x, x, 1, 2))) < 10^-27*(1 + abs(iterate(e^x, x, 1, 2)))",
        r"true",
    ),
    (
        r"abs(iterate(e^x, x, 1/3, iterate(e^x, x, 2/3, 2)) - (iterate(e^x, x, 1, 2))) < 10^-27*(1 + abs(iterate(e^x, x, 1, 2)))",
        r"true",
    ),
];

const STEPS_ADD_UP_4: &[(&str, &str)] = &[
    (
        r"abs(iterate(2^x, x, 1/2, iterate(2^x, x, 1/2, 0.5)) - (iterate(2^x, x, 1, 0.5))) < 10^-27*(1 + abs(iterate(2^x, x, 1, 0.5)))",
        r"true",
    ),
    (
        r"abs(iterate(2^x, x, 1/2, iterate(2^x, x, -1/2, 0.5)) - (0.5)) < 10^-27*(1 + abs(0.5))",
        r"true",
    ),
    (
        r"abs(iterate(2^x, x, 1/2, iterate(2^x, x, 1/2, 1)) - (iterate(2^x, x, 1, 1))) < 10^-27*(1 + abs(iterate(2^x, x, 1, 1)))",
        r"true",
    ),
    (
        r"abs(iterate(2^x, x, 1/2, iterate(2^x, x, -1/2, 1)) - (1)) < 10^-27*(1 + abs(1))",
        r"true",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 1/2, iterate(sqrt(2)^x, x, 1/2, 1)) - (iterate(sqrt(2)^x, x, 1, 1))) < 10^-27*(1 + abs(iterate(sqrt(2)^x, x, 1, 1)))",
        r"true",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 1/3, iterate(sqrt(2)^x, x, 2/3, 1)) - (iterate(sqrt(2)^x, x, 1, 1))) < 10^-27*(1 + abs(iterate(sqrt(2)^x, x, 1, 1)))",
        r"true",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 1/2, iterate(sqrt(2)^x, x, 1/2, 1.5)) - (iterate(sqrt(2)^x, x, 1, 1.5))) < 10^-27*(1 + abs(iterate(sqrt(2)^x, x, 1, 1.5)))",
        r"true",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 1/3, iterate(sqrt(2)^x, x, 2/3, 1.5)) - (iterate(sqrt(2)^x, x, 1, 1.5))) < 10^-27*(1 + abs(iterate(sqrt(2)^x, x, 1, 1.5)))",
        r"true",
    ),
    (
        r"abs(iterate(x/2+x^2/4, x, 1/2, iterate(x/2+x^2/4, x, 1/2, 0.3)) - (iterate(x/2+x^2/4, x, 1, 0.3))) < 10^-27*(1 + abs(iterate(x/2+x^2/4, x, 1, 0.3)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"abs(iterate(x/2+x^2/4, x, 1/2, iterate(x/2+x^2/4, x, 1, 0.3)) - (iterate(x/2+x^2/4, x, 3/2, 0.3))) < 10^-27*(1 + abs(iterate(x/2+x^2/4, x, 3/2, 0.3)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"abs(iterate(x/2+x^2/4, x, 3/10, iterate(x/2+x^2/4, x, 9/20, -0.5)) - (iterate(x/2+x^2/4, x, 3/4, -0.5))) < 10^-27*(1 + abs(iterate(x/2+x^2/4, x, 3/4, -0.5)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"abs(iterate(x/2+x^2/4, x, 1/3, iterate(x/2+x^2/4, x, 2/3, 1.5)) - (iterate(x/2+x^2/4, x, 1, 1.5))) < 10^-27*(1 + abs(iterate(x/2+x^2/4, x, 1, 1.5)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"abs(iterate(x/2+x^2/4, x, i, iterate(x/2+x^2/4, x, -i, 1.5)) - (1.5)) < 10^-27*(1 + abs(1.5))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"abs(iterate(x/2+x^2/4, x, 1/2, iterate(x/2+x^2/4, x, -1/2, 0.2+0.3i)) - (0.2+0.3i)) < 10^-27*(1 + abs(0.2+0.3i))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"abs(iterate(cos(x), x, 1/2, iterate(cos(x), x, 1/2, 1)) - (iterate(cos(x), x, 1, 1))) < 10^-27*(1 + abs(iterate(cos(x), x, 1, 1)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"abs(iterate(cos(x), x, 1/2, iterate(cos(x), x, 1, 1)) - (iterate(cos(x), x, 3/2, 1))) < 10^-27*(1 + abs(iterate(cos(x), x, 3/2, 1)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"abs(iterate(cos(x), x, 1/2, iterate(cos(x), x, 1/2, 0.5)) - (iterate(cos(x), x, 1, 0.5))) < 10^-27*(1 + abs(iterate(cos(x), x, 1, 0.5)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"abs(iterate(cos(x), x, 1/2, iterate(cos(x), x, 1, 0.5)) - (iterate(cos(x), x, 3/2, 0.5))) < 10^-27*(1 + abs(iterate(cos(x), x, 3/2, 0.5)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"abs(iterate(cos(x), x, 1/2, iterate(cos(x), x, 1/2, 0.2+0.3i)) - (iterate(cos(x), x, 1, 0.2+0.3i))) < 10^-27*(1 + abs(iterate(cos(x), x, 1, 0.2+0.3i)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"abs(iterate(cos(x), x, 1/2, iterate(cos(x), x, 1, 0.2+0.3i)) - (iterate(cos(x), x, 3/2, 0.2+0.3i))) < 10^-27*(1 + abs(iterate(cos(x), x, 3/2, 0.2+0.3i)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"abs(iterate(sin(x)/2, x, 1/2, iterate(sin(x)/2, x, 1/2, 1)) - (iterate(sin(x)/2, x, 1, 1))) < 10^-27*(1 + abs(iterate(sin(x)/2, x, 1, 1)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"abs(iterate(sin(x)/2, x, 1/2, iterate(sin(x)/2, x, 3/2, 1)) - (iterate(sin(x)/2, x, 2, 1))) < 10^-27*(1 + abs(iterate(sin(x)/2, x, 2, 1)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"abs(iterate(sin(x)/2, x, 1/2+i/3, iterate(sin(x)/2, x, 1/2-i/3, 1)) - (iterate(sin(x)/2, x, 1, 1))) < 10^-27*(1 + abs(iterate(sin(x)/2, x, 1, 1)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"abs(iterate(sin(x)/2, x, 1/3, iterate(sin(x)/2, x, 2/3, -0.7)) - (iterate(sin(x)/2, x, 1, -0.7))) < 10^-27*(1 + abs(iterate(sin(x)/2, x, 1, -0.7)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"abs(iterate(sin(x)/2, x, 1/2, iterate(sin(x)/2, x, -1/2, -0.7)) - (-0.7)) < 10^-27*(1 + abs(-0.7))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"abs(iterate(sin(x)/2, x, i, iterate(sin(x)/2, x, -i, -0.7)) - (-0.7)) < 10^-27*(1 + abs(-0.7))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"abs(iterate((1/2+i/3)*x+x^2, x, 1/2, iterate((1/2+i/3)*x+x^2, x, 1/2, 0.1)) - (iterate((1/2+i/3)*x+x^2, x, 1, 0.1))) < 10^-27*(1 + abs(iterate((1/2+i/3)*x+x^2, x, 1, 0.1)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5 + 0.3333333333i)",
    ),
    (
        r"abs(iterate((1/2+i/3)*x+x^2, x, 1/2, iterate((1/2+i/3)*x+x^2, x, 3/2, 0.1)) - (iterate((1/2+i/3)*x+x^2, x, 2, 0.1))) < 10^-27*(1 + abs(iterate((1/2+i/3)*x+x^2, x, 2, 0.1)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5 + 0.3333333333i)",
    ),
    (
        r"abs(iterate((1/2+i/3)*x+x^2, x, 1, iterate((1/2+i/3)*x+x^2, x, 1/2, 0.1)) - (iterate((1/2+i/3)*x+x^2, x, 3/2, 0.1))) < 10^-27*(1 + abs(iterate((1/2+i/3)*x+x^2, x, 3/2, 0.1)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5 + 0.3333333333i)",
    ),
    (
        r"abs(iterate((1/2+i/3)*x+x^2, x, 1/2, iterate((1/2+i/3)*x+x^2, x, 1/2, 0.1i)) - (iterate((1/2+i/3)*x+x^2, x, 1, 0.1i))) < 10^-27*(1 + abs(iterate((1/2+i/3)*x+x^2, x, 1, 0.1i)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5 + 0.3333333333i)",
    ),
    (
        r"abs(iterate((1/2+i/3)*x+x^2, x, 1/2, iterate((1/2+i/3)*x+x^2, x, 3/2, 0.1i)) - (iterate((1/2+i/3)*x+x^2, x, 2, 0.1i))) < 10^-27*(1 + abs(iterate((1/2+i/3)*x+x^2, x, 2, 0.1i)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5 + 0.3333333333i)",
    ),
    (
        r"abs(iterate((1/2+i/3)*x+x^2, x, 1, iterate((1/2+i/3)*x+x^2, x, 1/2, 0.1i)) - (iterate((1/2+i/3)*x+x^2, x, 3/2, 0.1i))) < 10^-27*(1 + abs(iterate((1/2+i/3)*x+x^2, x, 3/2, 0.1i)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5 + 0.3333333333i)",
    ),
    (
        r"abs(iterate(x/100+x^2, x, 1/2, iterate(x/100+x^2, x, 1/2, 0.3)) - (iterate(x/100+x^2, x, 1, 0.3))) < 10^-27*(1 + abs(iterate(x/100+x^2, x, 1, 0.3)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01)",
    ),
    (
        r"abs(iterate(x/100+x^2, x, 1/4, iterate(x/100+x^2, x, 1/2, 0.3)) - (iterate(x/100+x^2, x, 3/4, 0.3))) < 10^-27*(1 + abs(iterate(x/100+x^2, x, 3/4, 0.3)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01)",
    ),
    (
        r"abs(iterate(x/100+x^2, x, 1/2, iterate(x/100+x^2, x, 3/2, 0.3)) - (iterate(x/100+x^2, x, 2, 0.3))) < 10^-27*(1 + abs(iterate(x/100+x^2, x, 2, 0.3)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01)",
    ),
    (
        r"abs(iterate(x/100+x^2, x, 1/2, iterate(x/100+x^2, x, 1/2, 0.5)) - (iterate(x/100+x^2, x, 1, 0.5))) < 10^-27*(1 + abs(iterate(x/100+x^2, x, 1, 0.5)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01)",
    ),
    (
        r"abs(iterate(x/100+x^2, x, 1/4, iterate(x/100+x^2, x, 1/2, 0.5)) - (iterate(x/100+x^2, x, 3/4, 0.5))) < 10^-27*(1 + abs(iterate(x/100+x^2, x, 3/4, 0.5)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01)",
    ),
    (
        r"abs(iterate(x/100+x^2, x, 1/2, iterate(x/100+x^2, x, 3/2, 0.5)) - (iterate(x/100+x^2, x, 2, 0.5))) < 10^-27*(1 + abs(iterate(x/100+x^2, x, 2, 0.5)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.01)",
    ),
    (
        r"abs(iterate((x^2+1)/3, x, 1/2, iterate((x^2+1)/3, x, 1/2, 1)) - (iterate((x^2+1)/3, x, 1, 1))) < 10^-27*(1 + abs(iterate((x^2+1)/3, x, 1, 1)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075)",
    ),
    (
        r"abs(iterate((x^2+1)/3, x, 3/10, iterate((x^2+1)/3, x, 9/20, 1)) - (iterate((x^2+1)/3, x, 3/4, 1))) < 10^-27*(1 + abs(iterate((x^2+1)/3, x, 3/4, 1)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075)",
    ),
];

const STEPS_ADD_UP_5: &[(&str, &str)] = &[
    (
        r"abs(iterate((x^2+1)/3, x, 1/2, iterate((x^2+1)/3, x, 1/2, 2)) - (iterate((x^2+1)/3, x, 1, 2))) < 10^-27*(1 + abs(iterate((x^2+1)/3, x, 1, 2)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075)",
    ),
    (
        r"abs(iterate((x^2+1)/3, x, 3/10, iterate((x^2+1)/3, x, 9/20, 2)) - (iterate((x^2+1)/3, x, 3/4, 2))) < 10^-27*(1 + abs(iterate((x^2+1)/3, x, 3/4, 2)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075)",
    ),
    (
        r"abs(iterate((x^2+1)/3, x, 1/2, iterate((x^2+1)/3, x, 1/2, 0)) - (iterate((x^2+1)/3, x, 1, 0))) < 10^-27*(1 + abs(iterate((x^2+1)/3, x, 1, 0)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075)",
    ),
    (
        r"abs(iterate((x^2+1)/3, x, 3/10, iterate((x^2+1)/3, x, 9/20, 0)) - (iterate((x^2+1)/3, x, 3/4, 0))) < 10^-27*(1 + abs(iterate((x^2+1)/3, x, 3/4, 0)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points|note: iterate: regular iteration at the attracting fixed point 0.3819660113 (slope 0.2546440075)",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 1/2, iterate(sqrt(2)^x, x, 1/2, 1, 2), 2) - (iterate(sqrt(2)^x, x, 1, 1, 2))) < 10^-27*(1 + abs(iterate(sqrt(2)^x, x, 1, 1, 2)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 1/2, iterate(sqrt(2)^x, x, 3/2, 1, 2), 2) - (iterate(sqrt(2)^x, x, 2, 1, 2))) < 10^-27*(1 + abs(iterate(sqrt(2)^x, x, 2, 1, 2)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 1, iterate(sqrt(2)^x, x, 1/2, 1, 2), 2) - (iterate(sqrt(2)^x, x, 3/2, 1, 2))) < 10^-27*(1 + abs(iterate(sqrt(2)^x, x, 3/2, 1, 2)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 1/2, iterate(sqrt(2)^x, x, 3/2, 3, 2), 2) - (iterate(sqrt(2)^x, x, 2, 3, 2))) < 10^-27*(1 + abs(iterate(sqrt(2)^x, x, 2, 3, 2)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 1, iterate(sqrt(2)^x, x, 1/2, 3, 2), 2) - (iterate(sqrt(2)^x, x, 3/2, 3, 2))) < 10^-27*(1 + abs(iterate(sqrt(2)^x, x, 3/2, 3, 2)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 1/2, iterate(sqrt(2)^x, x, 3/2, 0, 2), 2) - (iterate(sqrt(2)^x, x, 2, 0, 2))) < 10^-27*(1 + abs(iterate(sqrt(2)^x, x, 2, 0, 2)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 2 (slope 0.6931471806)",
    ),
    (
        r"abs(iterate(-x/2+x^3, x, 1/2, iterate(-x/2+x^3, x, 1/2, 0.3)) - (iterate(-x/2+x^3, x, 1, 0.3))) < 10^-27*(1 + abs(iterate(-x/2+x^3, x, 1, 0.3)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"abs(iterate(-x/2+x^3, x, 1/3, iterate(-x/2+x^3, x, 2/3, 0.3)) - (iterate(-x/2+x^3, x, 1, 0.3))) < 10^-27*(1 + abs(iterate(-x/2+x^3, x, 1, 0.3)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"abs(iterate(-x/2+x^3, x, 3/10, iterate(-x/2+x^3, x, 9/20, 0.3)) - (iterate(-x/2+x^3, x, 3/4, 0.3))) < 10^-27*(1 + abs(iterate(-x/2+x^3, x, 3/4, 0.3)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"abs(iterate(-x/2+x^3, x, 1/2, iterate(-x/2+x^3, x, -1/2, 0.3)) - (0.3)) < 10^-27*(1 + abs(0.3))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"abs(iterate(-x/2+x^3, x, 1/3, iterate(-x/2+x^3, x, -1/3, 0.3)) - (0.3)) < 10^-27*(1 + abs(0.3))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"abs(iterate(-x/2+x^3, x, 1/2, iterate(-x/2+x^3, x, 1, 0.3)) - (iterate(-x/2+x^3, x, 3/2, 0.3))) < 10^-27*(1 + abs(iterate(-x/2+x^3, x, 3/2, 0.3)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope -0.5)",
    ),
    (
        r"abs(iterate(5/2*x*(1-x), x, 1/2, iterate(5/2*x*(1-x), x, 1/2, 0.3)) - (iterate(5/2*x*(1-x), x, 1, 0.3))) < 10^-27*(1 + abs(iterate(5/2*x*(1-x), x, 1, 0.3)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.7, which the map takes to the same points|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5)",
    ),
    (
        r"abs(iterate(5/2*x*(1-x), x, 1/4, iterate(5/2*x*(1-x), x, 1/2, 0.3)) - (iterate(5/2*x*(1-x), x, 3/4, 0.3))) < 10^-27*(1 + abs(iterate(5/2*x*(1-x), x, 3/4, 0.3)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.7, which the map takes to the same points|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5)",
    ),
    (
        r"abs(iterate(5/2*x*(1-x), x, 1/2, iterate(5/2*x*(1-x), x, 3/2, 0.3)) - (iterate(5/2*x*(1-x), x, 2, 0.3))) < 10^-27*(1 + abs(iterate(5/2*x*(1-x), x, 2, 0.3)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 0.7, which the map takes to the same points|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5)",
    ),
    (
        r"abs(iterate(5/2*x*(1-x), x, 1/2, iterate(5/2*x*(1-x), x, 1/2, 0.7)) - (iterate(5/2*x*(1-x), x, 1, 0.7))) < 10^-27*(1 + abs(iterate(5/2*x*(1-x), x, 1, 0.7)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5)",
    ),
    (
        r"abs(iterate(5/2*x*(1-x), x, 1/4, iterate(5/2*x*(1-x), x, 1/2, 0.7)) - (iterate(5/2*x*(1-x), x, 3/4, 0.7))) < 10^-27*(1 + abs(iterate(5/2*x*(1-x), x, 3/4, 0.7)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5)",
    ),
    (
        r"abs(iterate(5/2*x*(1-x), x, 1/2, iterate(5/2*x*(1-x), x, 3/2, 0.7)) - (iterate(5/2*x*(1-x), x, 2, 0.7))) < 10^-27*(1 + abs(iterate(5/2*x*(1-x), x, 2, 0.7)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.6 (slope -0.5)",
    ),
    (
        r"abs(iterate(erf(x), x, 1/2, iterate(erf(x), x, 1/2, 1)) - (iterate(erf(x), x, 1, 1))) < 10^-27*(1 + abs(iterate(erf(x), x, 1, 1)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.6174468791 (slope 0.7706982084)",
    ),
    (
        r"abs(iterate(erf(x), x, 1/2, iterate(erf(x), x, 1/2, 0.2)) - (iterate(erf(x), x, 1, 0.2))) < 10^-27*(1 + abs(iterate(erf(x), x, 1, 0.2)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.6174468791 (slope 0.7706982084)",
    ),
    (
        r"abs(iterate(besselj(0,x), x, 1/2, iterate(besselj(0,x), x, 1/2, 1)) - (iterate(besselj(0,x), x, 1, 1))) < 10^-27*(1 + abs(iterate(besselj(0,x), x, 1, 1)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.8336568528 (slope -0.3816508417)",
    ),
    (
        r"abs(iterate(besselj(0,x), x, 1/2, iterate(besselj(0,x), x, 1/2, 0.5)) - (iterate(besselj(0,x), x, 1, 0.5))) < 10^-27*(1 + abs(iterate(besselj(0,x), x, 1, 0.5)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0.8336568528 (slope -0.3816508417)",
    ),
    (
        r"abs(iterate(x^2-1, x, 1/2, iterate(x^2-1, x, 1/2, 0.5)) - (iterate(x^2-1, x, 1, 0.5))) < 10^-27*(1 + abs(iterate(x^2-1, x, 1, 0.5)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.9424853724",
    ),
    (
        r"abs(iterate(x^2-1, x, 1/4, iterate(x^2-1, x, 1/2, 0.5)) - (iterate(x^2-1, x, 3/4, 0.5))) < 10^-27*(1 + abs(iterate(x^2-1, x, 3/4, 0.5)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.9424853724",
    ),
    (
        r"abs(iterate(x^2-1, x, 1/2, iterate(x^2-1, x, 1, 0.5)) - (iterate(x^2-1, x, 3/2, 0.5))) < 10^-27*(1 + abs(iterate(x^2-1, x, 3/2, 0.5)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.5; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"abs(iterate(x^2-1, x, 1/2, iterate(x^2-1, x, 1/2, 0.2)) - (iterate(x^2-1, x, 1, 0.2))) < 10^-27*(1 + abs(iterate(x^2-1, x, 1, 0.2)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.7351297854",
    ),
    (
        r"abs(iterate(x^2-1, x, 1/4, iterate(x^2-1, x, 1/2, 0.2)) - (iterate(x^2-1, x, 3/4, 0.2))) < 10^-27*(1 + abs(iterate(x^2-1, x, 3/4, 0.2)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977)|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.7351297854",
    ),
    (
        r"abs(iterate(x^2-1, x, 1/2, iterate(x^2-1, x, 1, 0.2)) - (iterate(x^2-1, x, 3/2, 0.2))) < 10^-27*(1 + abs(iterate(x^2-1, x, 3/2, 0.2)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.2; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"abs(iterate(2sinh(x), x, 1/2, iterate(2sinh(x), x, 1/2, 0.3)) - (iterate(2sinh(x), x, 1, 0.3))) < 10^-27*(1 + abs(iterate(2sinh(x), x, 1, 0.3)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"abs(iterate(2sinh(x), x, 3/10, iterate(2sinh(x), x, 9/20, 0.3)) - (iterate(2sinh(x), x, 3/4, 0.3))) < 10^-27*(1 + abs(iterate(2sinh(x), x, 3/4, 0.3)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"abs(iterate(2sinh(x), x, 1/3, iterate(2sinh(x), x, -1/3, 0.3)) - (0.3)) < 10^-27*(1 + abs(0.3))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"abs(iterate(2sinh(x), x, 1/2, iterate(2sinh(x), x, 1/2, 1)) - (iterate(2sinh(x), x, 1, 1))) < 10^-27*(1 + abs(iterate(2sinh(x), x, 1, 1)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"abs(iterate(2sinh(x), x, 3/10, iterate(2sinh(x), x, 9/20, 1)) - (iterate(2sinh(x), x, 3/4, 1))) < 10^-27*(1 + abs(iterate(2sinh(x), x, 3/4, 1)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"abs(iterate(2sinh(x), x, 1/3, iterate(2sinh(x), x, -1/3, 1)) - (1)) < 10^-27*(1 + abs(1))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"abs(iterate(2sinh(x), x, 1/2, iterate(2sinh(x), x, 1/2, 0.3, 0), 0) - (iterate(2sinh(x), x, 1, 0.3, 0))) < 10^-27*(1 + abs(iterate(2sinh(x), x, 1, 0.3, 0)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"abs(iterate(2sinh(x), x, 1/2, iterate(2sinh(x), x, 3/2, 0.3, 0), 0) - (iterate(2sinh(x), x, 2, 0.3, 0))) < 10^-27*(1 + abs(iterate(2sinh(x), x, 2, 0.3, 0)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
];

const STEPS_ADD_UP_6: &[(&str, &str)] = &[
    (
        r"abs(iterate(2sinh(x), x, 1, iterate(2sinh(x), x, 1/2, 0.3, 0), 0) - (iterate(2sinh(x), x, 3/2, 0.3, 0))) < 10^-27*(1 + abs(iterate(2sinh(x), x, 3/2, 0.3, 0)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"abs(iterate(2sinh(x), x, 1/2, iterate(2sinh(x), x, 1/2, 1, 0), 0) - (iterate(2sinh(x), x, 1, 1, 0))) < 10^-27*(1 + abs(iterate(2sinh(x), x, 1, 1, 0)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"abs(iterate(2sinh(x), x, 1/2, iterate(2sinh(x), x, 3/2, 1, 0), 0) - (iterate(2sinh(x), x, 2, 1, 0))) < 10^-27*(1 + abs(iterate(2sinh(x), x, 2, 1, 0)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"abs(iterate(2sinh(x), x, 1, iterate(2sinh(x), x, 1/2, 1, 0), 0) - (iterate(2sinh(x), x, 3/2, 1, 0))) < 10^-27*(1 + abs(iterate(2sinh(x), x, 3/2, 1, 0)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"abs(iterate(e^x, x, 1/2, iterate(e^x, x, 1/2, 0.5, 0.3+1.3i), 0.3+1.3i) - (iterate(e^x, x, 1, 0.5, 0.3+1.3i))) < 10^-27*(1 + abs(iterate(e^x, x, 1, 0.5, 0.3+1.3i)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i)|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"abs(iterate(e^x, x, 1/4, iterate(e^x, x, 1/2, 0.5, 0.3+1.3i), 0.3+1.3i) - (iterate(e^x, x, 3/4, 0.5, 0.3+1.3i))) < 10^-27*(1 + abs(iterate(e^x, x, 3/4, 0.5, 0.3+1.3i)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i)",
    ),
    (
        r"abs(iterate(e^x, x, 1/2, iterate(e^x, x, 3/2, 0.5, 0.3+1.3i), 0.3+1.3i) - (iterate(e^x, x, 2, 0.5, 0.3+1.3i))) < 10^-27*(1 + abs(iterate(e^x, x, 2, 0.5, 0.3+1.3i)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i)|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"abs(iterate(e^x, x, 1/2, iterate(e^x, x, 1/2, 2, 0.3+1.3i), 0.3+1.3i) - (iterate(e^x, x, 1, 2, 0.3+1.3i))) < 10^-27*(1 + abs(iterate(e^x, x, 1, 2, 0.3+1.3i)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i)|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"abs(iterate(e^x, x, 1/4, iterate(e^x, x, 1/2, 2, 0.3+1.3i), 0.3+1.3i) - (iterate(e^x, x, 3/4, 2, 0.3+1.3i))) < 10^-27*(1 + abs(iterate(e^x, x, 3/4, 2, 0.3+1.3i)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i)|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"abs(iterate(e^x, x, 1/2, iterate(e^x, x, 3/2, 2, 0.3+1.3i), 0.3+1.3i) - (iterate(e^x, x, 2, 2, 0.3+1.3i))) < 10^-27*(1 + abs(iterate(e^x, x, 2, 2, 0.3+1.3i)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i)|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 2.983926437 + 0.2908540453i; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on|note: iterate: regular iteration at the repelling fixed point 0.3181315052 + 1.337235701i (slope 0.3181315052 + 1.337235701i); the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"abs(iterate(2x+sin(x), x, 1/2, iterate(2x+sin(x), x, 1/2, 0.3)) - (iterate(2x+sin(x), x, 1, 0.3))) < 10^-27*(1 + abs(iterate(2x+sin(x), x, 1, 0.3)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"abs(iterate(2x+sin(x), x, 1/2, iterate(2x+sin(x), x, -1/2, 1)) - (1)) < 10^-27*(1 + abs(1))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"abs(iterate(x^3+2x, x, 1/2, iterate(x^3+2x, x, 1/2, 0.1, 0), 0) - (iterate(x^3+2x, x, 1, 0.1, 0))) < 10^-27*(1 + abs(iterate(x^3+2x, x, 1, 0.1, 0)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"abs(iterate(x^3+2x, x, 3/10, iterate(x^3+2x, x, 9/20, 0.1, 0), 0) - (iterate(x^3+2x, x, 3/4, 0.1, 0))) < 10^-27*(1 + abs(iterate(x^3+2x, x, 3/4, 0.1, 0)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"abs(iterate(x^3+2x, x, 1/3, iterate(x^3+2x, x, -1/3, 0.1, 0), 0) - (0.1)) < 10^-27*(1 + abs(0.1))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"abs(iterate(x^3+2x, x, 1/2, iterate(x^3+2x, x, 1/2, 2, 0), 0) - (iterate(x^3+2x, x, 1, 2, 0))) < 10^-27*(1 + abs(iterate(x^3+2x, x, 1, 2, 0)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"abs(iterate(x^3+2x, x, 3/10, iterate(x^3+2x, x, 9/20, 2, 0), 0) - (iterate(x^3+2x, x, 3/4, 2, 0))) < 10^-27*(1 + abs(iterate(x^3+2x, x, 3/4, 2, 0)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"abs(iterate(x^3+2x, x, 1/3, iterate(x^3+2x, x, -1/3, 2, 0), 0) - (2)) < 10^-27*(1 + abs(2))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 2)",
    ),
    (
        r"abs(iterate(-2sinh(x), x, 1/2, iterate(-2sinh(x), x, 1/2, 0.3)) - (iterate(-2sinh(x), x, 1, 0.3))) < 10^-27*(1 + abs(iterate(-2sinh(x), x, 1, 0.3)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"abs(iterate(-2sinh(x), x, 1/3, iterate(-2sinh(x), x, 2/3, 0.3)) - (iterate(-2sinh(x), x, 1, 0.3))) < 10^-27*(1 + abs(iterate(-2sinh(x), x, 1, 0.3)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"abs(iterate(-2sinh(x), x, 1/4, iterate(-2sinh(x), x, 1/2, 0.3)) - (iterate(-2sinh(x), x, 3/4, 0.3))) < 10^-27*(1 + abs(iterate(-2sinh(x), x, 3/4, 0.3)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"abs(iterate(-2sinh(x), x, 3/10, iterate(-2sinh(x), x, 9/20, 0.3)) - (iterate(-2sinh(x), x, 3/4, 0.3))) < 10^-27*(1 + abs(iterate(-2sinh(x), x, 3/4, 0.3)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"abs(iterate(-2sinh(x), x, 1/2, iterate(-2sinh(x), x, 3/2, 0.3)) - (iterate(-2sinh(x), x, 2, 0.3))) < 10^-27*(1 + abs(iterate(-2sinh(x), x, 2, 0.3)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"abs(iterate(-2sinh(x), x, 1/2, iterate(-2sinh(x), x, 1, 0.3)) - (iterate(-2sinh(x), x, 3/2, 0.3))) < 10^-27*(1 + abs(iterate(-2sinh(x), x, 3/2, 0.3)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope -2)",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 1/2, iterate(sqrt(2)^x, x, 1/2, 3, 4), 4) - (iterate(sqrt(2)^x, x, 1, 3, 4))) < 10^-27*(1 + abs(iterate(sqrt(2)^x, x, 1, 3, 4)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 3/10, iterate(sqrt(2)^x, x, 9/20, 3, 4), 4) - (iterate(sqrt(2)^x, x, 3/4, 3, 4))) < 10^-27*(1 + abs(iterate(sqrt(2)^x, x, 3/4, 3, 4)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 1/3, iterate(sqrt(2)^x, x, -1/3, 3, 4), 4) - (3)) < 10^-27*(1 + abs(3))",
        r"true|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 1/2, iterate(sqrt(2)^x, x, 1/2, 5, 4), 4) - (iterate(sqrt(2)^x, x, 1, 5, 4))) < 10^-27*(1 + abs(iterate(sqrt(2)^x, x, 1, 5, 4)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 3/10, iterate(sqrt(2)^x, x, 9/20, 5, 4), 4) - (iterate(sqrt(2)^x, x, 3/4, 5, 4))) < 10^-27*(1 + abs(iterate(sqrt(2)^x, x, 3/4, 5, 4)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"abs(iterate(sqrt(2)^x, x, 1/3, iterate(sqrt(2)^x, x, -1/3, 5, 4), 4) - (5)) < 10^-27*(1 + abs(5))",
        r"true|note: iterate: regular iteration at the repelling fixed point 4 (slope 1.386294361)",
    ),
    (
        r"abs(iterate(x!, x, 1/2, iterate(x!, x, 1/2, 3)) - (iterate(x!, x, 1, 3))) < 10^-27*(1 + abs(iterate(x!, x, 1, 3)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 2 (slope 1.84556867)",
    ),
    (
        r"abs(iterate(x!, x, 1/2, iterate(x!, x, 1/2, 4)) - (iterate(x!, x, 1, 4))) < 10^-27*(1 + abs(iterate(x!, x, 1, 4)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 2 (slope 1.84556867)",
    ),
    (
        r"abs(iterate(x^2+x^3, x, 1/2, iterate(x^2+x^3, x, 1/2, 0.1)) - (iterate(x^2+x^3, x, 1, 0.1))) < 10^-27*(1 + abs(iterate(x^2+x^3, x, 1, 0.1)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(x^2+x^3, x, 3/10, iterate(x^2+x^3, x, 9/20, 0.1)) - (iterate(x^2+x^3, x, 3/4, 0.1))) < 10^-27*(1 + abs(iterate(x^2+x^3, x, 3/4, 0.1)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(x^2+x^3, x, 1/3, iterate(x^2+x^3, x, -1/3, 0.1)) - (0.1)) < 10^-27*(1 + abs(0.1))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(x^2+x^3, x, 1/2, iterate(x^2+x^3, x, 1/2, 0.3)) - (iterate(x^2+x^3, x, 1, 0.3))) < 10^-27*(1 + abs(iterate(x^2+x^3, x, 1, 0.3)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(x^2+x^3, x, 3/10, iterate(x^2+x^3, x, 9/20, 0.3)) - (iterate(x^2+x^3, x, 3/4, 0.3))) < 10^-27*(1 + abs(iterate(x^2+x^3, x, 3/4, 0.3)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(x^2+x^3, x, 1/3, iterate(x^2+x^3, x, -1/3, 0.3)) - (0.3)) < 10^-27*(1 + abs(0.3))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(sin(x)^2, x, 1/2, iterate(sin(x)^2, x, 1/2, 0.5)) - (iterate(sin(x)^2, x, 1, 0.5))) < 10^-27*(1 + abs(iterate(sin(x)^2, x, 1, 0.5)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(sin(x)^2, x, 3/10, iterate(sin(x)^2, x, 9/20, 0.5)) - (iterate(sin(x)^2, x, 3/4, 0.5))) < 10^-27*(1 + abs(iterate(sin(x)^2, x, 3/4, 0.5)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
];

const STEPS_ADD_UP_7: &[(&str, &str)] = &[
    (
        r"abs(iterate(sin(x)^2, x, 1/3, iterate(sin(x)^2, x, -1/3, 0.5)) - (0.5)) < 10^-27*(1 + abs(0.5))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(sin(x)^2, x, 1, iterate(sin(x)^2, x, 1/2, 0.5)) - (iterate(sin(x)^2, x, 3/2, 0.5))) < 10^-27*(1 + abs(iterate(sin(x)^2, x, 3/2, 0.5)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(sin(x)^2, x, 1/4, iterate(sin(x)^2, x, 1/2, 1)) - (iterate(sin(x)^2, x, 3/4, 1))) < 10^-27*(1 + abs(iterate(sin(x)^2, x, 3/4, 1)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(sin(x)^2, x, 1/2, iterate(sin(x)^2, x, -1/2, 1)) - (1)) < 10^-27*(1 + abs(1))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate((x-1)^2*(x+1)+1, x, 1/2, iterate((x-1)^2*(x+1)+1, x, 1/2, 1.3)) - (iterate((x-1)^2*(x+1)+1, x, 1, 1.3))) < 10^-27*(1 + abs(iterate((x-1)^2*(x+1)+1, x, 1, 1.3)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate((x-1)^2*(x+1)+1, x, 3/10, iterate((x-1)^2*(x+1)+1, x, 9/20, 1.3)) - (iterate((x-1)^2*(x+1)+1, x, 3/4, 1.3))) < 10^-27*(1 + abs(iterate((x-1)^2*(x+1)+1, x, 3/4, 1.3)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate((x-1)^2*(x+1)+1, x, 1/3, iterate((x-1)^2*(x+1)+1, x, -1/3, 1.3)) - (1.3)) < 10^-27*(1 + abs(1.3))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate((x-1)^2*(x+1)+1, x, 1/2, iterate((x-1)^2*(x+1)+1, x, 1/2, 1.1)) - (iterate((x-1)^2*(x+1)+1, x, 1, 1.1))) < 10^-27*(1 + abs(iterate((x-1)^2*(x+1)+1, x, 1, 1.1)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate((x-1)^2*(x+1)+1, x, 3/10, iterate((x-1)^2*(x+1)+1, x, 9/20, 1.1)) - (iterate((x-1)^2*(x+1)+1, x, 3/4, 1.1))) < 10^-27*(1 + abs(iterate((x-1)^2*(x+1)+1, x, 3/4, 1.1)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate((x-1)^2*(x+1)+1, x, 1/3, iterate((x-1)^2*(x+1)+1, x, -1/3, 1.1)) - (1.1)) < 10^-27*(1 + abs(1.1))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 1 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(1-cos(x), x, 1/2, iterate(1-cos(x), x, 1/2, 1)) - (iterate(1-cos(x), x, 1, 1))) < 10^-27*(1 + abs(iterate(1-cos(x), x, 1, 1)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(1-cos(x), x, 3/10, iterate(1-cos(x), x, 9/20, 1)) - (iterate(1-cos(x), x, 3/4, 1))) < 10^-27*(1 + abs(iterate(1-cos(x), x, 3/4, 1)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(1-cos(x), x, 1/3, iterate(1-cos(x), x, -1/3, 1)) - (1)) < 10^-27*(1 + abs(1))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(1-cos(x), x, 1/2, iterate(1-cos(x), x, 1/2, 0.5)) - (iterate(1-cos(x), x, 1, 0.5))) < 10^-27*(1 + abs(iterate(1-cos(x), x, 1, 0.5)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(1-cos(x), x, 3/10, iterate(1-cos(x), x, 9/20, 0.5)) - (iterate(1-cos(x), x, 3/4, 0.5))) < 10^-27*(1 + abs(iterate(1-cos(x), x, 3/4, 0.5)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(1-cos(x), x, 1/3, iterate(1-cos(x), x, -1/3, 0.5)) - (0.5)) < 10^-27*(1 + abs(0.5))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(x^3*(1+x), x, 1/2, iterate(x^3*(1+x), x, 1/2, 0.5)) - (iterate(x^3*(1+x), x, 1, 0.5))) < 10^-27*(1 + abs(iterate(x^3*(1+x), x, 1, 0.5)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3)",
    ),
    (
        r"abs(iterate(x^3*(1+x), x, 1/3, iterate(x^3*(1+x), x, 2/3, 0.5)) - (iterate(x^3*(1+x), x, 1, 0.5))) < 10^-27*(1 + abs(iterate(x^3*(1+x), x, 1, 0.5)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3)",
    ),
    (
        r"abs(iterate(x^3*(1+x), x, 3/10, iterate(x^3*(1+x), x, 9/20, 0.5)) - (iterate(x^3*(1+x), x, 3/4, 0.5))) < 10^-27*(1 + abs(iterate(x^3*(1+x), x, 3/4, 0.5)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3)",
    ),
    (
        r"abs(iterate(x^3*(1+x), x, 1/2, iterate(x^3*(1+x), x, -1/2, 0.5)) - (0.5)) < 10^-27*(1 + abs(0.5))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3)",
    ),
    (
        r"abs(iterate(x^3*(1+x), x, 1/3, iterate(x^3*(1+x), x, -1/3, 0.5)) - (0.5)) < 10^-27*(1 + abs(0.5))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3)",
    ),
    (
        r"abs(iterate(x^3*(1+x), x, 1/2, iterate(x^3*(1+x), x, 1, 0.5)) - (iterate(x^3*(1+x), x, 3/2, 0.5))) < 10^-27*(1 + abs(iterate(x^3*(1+x), x, 3/2, 0.5)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 3)",
    ),
    (
        r"abs(iterate(x^2+1, x, 1/2, iterate(x^2+1, x, 1/2, 1)) - (iterate(x^2+1, x, 1, 1))) < 10^-27*(1 + abs(iterate(x^2+1, x, 1, 1)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"abs(iterate(x^2+1, x, 1/3, iterate(x^2+1, x, -1/3, 1)) - (1)) < 10^-27*(1 + abs(1))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"abs(iterate(x^2+1, x, 3/10, iterate(x^2+1, x, 9/20, 2)) - (iterate(x^2+1, x, 3/4, 2))) < 10^-27*(1 + abs(iterate(x^2+1, x, 3/4, 2)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"abs(iterate(x^2+1, x, 1, iterate(x^2+1, x, 1/2, 2)) - (iterate(x^2+1, x, 3/2, 2))) < 10^-27*(1 + abs(iterate(x^2+1, x, 3/2, 2)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"abs(iterate(x^2+1, x, 1/2, iterate(x^2+1, x, 1, 0.5)) - (iterate(x^2+1, x, 3/2, 0.5))) < 10^-27*(1 + abs(iterate(x^2+1, x, 3/2, 0.5)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"abs(iterate(x^2+1, x, 1/2, iterate(x^2+1, x, 3/2, 3+i)) - (iterate(x^2+1, x, 2, 3+i))) < 10^-27*(1 + abs(iterate(x^2+1, x, 2, 3+i)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"abs(iterate(x^3+2x, x, 1/2, iterate(x^3+2x, x, 1/2, 0.1)) - (iterate(x^3+2x, x, 1, 0.1))) < 10^-27*(1 + abs(iterate(x^3+2x, x, 1, 0.1)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"abs(iterate(x^3+2x, x, 3/10, iterate(x^3+2x, x, 9/20, 0.1)) - (iterate(x^3+2x, x, 3/4, 0.1))) < 10^-27*(1 + abs(iterate(x^3+2x, x, 3/4, 0.1)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"abs(iterate(x^3+2x, x, 1/3, iterate(x^3+2x, x, -1/3, 0.1)) - (0.1)) < 10^-27*(1 + abs(0.1))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"abs(iterate(x^3+2x, x, 1/2, iterate(x^3+2x, x, 1/2, 2)) - (iterate(x^3+2x, x, 1, 2))) < 10^-27*(1 + abs(iterate(x^3+2x, x, 1, 2)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"abs(iterate(x^3+2x, x, 3/10, iterate(x^3+2x, x, 9/20, 2)) - (iterate(x^3+2x, x, 3/4, 2))) < 10^-27*(1 + abs(iterate(x^3+2x, x, 3/4, 2)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"abs(iterate(x^3+2x, x, 1/3, iterate(x^3+2x, x, -1/3, 2)) - (2)) < 10^-27*(1 + abs(2))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 3)",
    ),
    (
        r"abs(iterate(2x^2-3x+4, x, 1/2, iterate(2x^2-3x+4, x, 1/2, 3)) - (iterate(2x^2-3x+4, x, 1, 3))) < 10^-27*(1 + abs(iterate(2x^2-3x+4, x, 1, 3)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"abs(iterate(2x^2-3x+4, x, 3/10, iterate(2x^2-3x+4, x, 9/20, 3)) - (iterate(2x^2-3x+4, x, 3/4, 3))) < 10^-27*(1 + abs(iterate(2x^2-3x+4, x, 3/4, 3)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"abs(iterate(2x^2-3x+4, x, 1/3, iterate(2x^2-3x+4, x, -1/3, 3)) - (3)) < 10^-27*(1 + abs(3))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"abs(iterate(2x^2-3x+4, x, 1/2, iterate(2x^2-3x+4, x, 1/2, 10)) - (iterate(2x^2-3x+4, x, 1, 10))) < 10^-27*(1 + abs(iterate(2x^2-3x+4, x, 1, 10)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"abs(iterate(2x^2-3x+4, x, 3/10, iterate(2x^2-3x+4, x, 9/20, 10)) - (iterate(2x^2-3x+4, x, 3/4, 10))) < 10^-27*(1 + abs(iterate(2x^2-3x+4, x, 3/4, 10)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"abs(iterate(2x^2-3x+4, x, 1/3, iterate(2x^2-3x+4, x, -1/3, 10)) - (10)) < 10^-27*(1 + abs(10))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
];

const STEPS_ADD_UP_8: &[(&str, &str)] = &[
    (
        r"abs(iterate(x^4-x, x, 1/2, iterate(x^4-x, x, 1/2, 2)) - (iterate(x^4-x, x, 1, 2))) < 10^-27*(1 + abs(iterate(x^4-x, x, 1, 2)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
    (
        r"abs(iterate(x^4-x, x, 3/10, iterate(x^4-x, x, 9/20, 2)) - (iterate(x^4-x, x, 3/4, 2))) < 10^-27*(1 + abs(iterate(x^4-x, x, 3/4, 2)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
    (
        r"abs(iterate(x^4-x, x, 1/3, iterate(x^4-x, x, -1/3, 2)) - (2)) < 10^-27*(1 + abs(2))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
    (
        r"abs(iterate(x^4-x, x, 1/2, iterate(x^4-x, x, 1/2, 1.5)) - (iterate(x^4-x, x, 1, 1.5))) < 10^-27*(1 + abs(iterate(x^4-x, x, 1, 1.5)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
    (
        r"abs(iterate(x^4-x, x, 3/10, iterate(x^4-x, x, 9/20, 1.5)) - (iterate(x^4-x, x, 3/4, 1.5))) < 10^-27*(1 + abs(iterate(x^4-x, x, 3/4, 1.5)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
    (
        r"abs(iterate(x^4-x, x, 1/3, iterate(x^4-x, x, -1/3, 1.5)) - (1.5)) < 10^-27*(1 + abs(1.5))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 4)",
    ),
    (
        r"abs(iterate(2x+3/x, x, 1/2, iterate(2x+3/x, x, 1/2, 1)) - (iterate(2x+3/x, x, 1, 1))) < 10^-27*(1 + abs(iterate(2x+3/x, x, 1, 1)))",
        r"true|note: iterate: regular iteration at infinity (slope 0.5 in 1/x); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1.5, which the map takes to the same points|note: iterate: regular iteration at infinity (slope 0.5 in 1/x)",
    ),
    (
        r"abs(iterate(2x+3/x, x, 1/4, iterate(2x+3/x, x, 1/2, 1)) - (iterate(2x+3/x, x, 3/4, 1))) < 10^-27*(1 + abs(iterate(2x+3/x, x, 3/4, 1)))",
        r"true|note: iterate: regular iteration at infinity (slope 0.5 in 1/x); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1.5, which the map takes to the same points|note: iterate: regular iteration at infinity (slope 0.5 in 1/x)",
    ),
    (
        r"abs(iterate(2x+3/x, x, 1/2, iterate(2x+3/x, x, 1, 1)) - (iterate(2x+3/x, x, 3/2, 1))) < 10^-27*(1 + abs(iterate(2x+3/x, x, 3/2, 1)))",
        r"true|note: iterate: regular iteration at infinity (slope 0.5 in 1/x)|note: iterate: regular iteration at infinity (slope 0.5 in 1/x); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at 1.5, which the map takes to the same points",
    ),
    (
        r"abs(iterate(2x+3/x, x, 1/3, iterate(2x+3/x, x, 2/3, 5)) - (iterate(2x+3/x, x, 1, 5))) < 10^-27*(1 + abs(iterate(2x+3/x, x, 1, 5)))",
        r"true|note: iterate: regular iteration at infinity (slope 0.5 in 1/x)",
    ),
    (
        r"abs(iterate(2x+3/x, x, 3/10, iterate(2x+3/x, x, 9/20, 5)) - (iterate(2x+3/x, x, 3/4, 5))) < 10^-27*(1 + abs(iterate(2x+3/x, x, 3/4, 5)))",
        r"true|note: iterate: regular iteration at infinity (slope 0.5 in 1/x)",
    ),
    (
        r"abs(iterate(2x+3/x, x, 1/3, iterate(2x+3/x, x, -1/3, 5)) - (5)) < 10^-27*(1 + abs(5))",
        r"true|note: iterate: regular iteration at infinity (slope 0.5 in 1/x)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2, iterate(e^x-1, x, 1/2, -1)) - (iterate(e^x-1, x, 1, -1))) < 10^-27*(1 + abs(iterate(e^x-1, x, 1, -1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/4, iterate(e^x-1, x, 1/2, -1)) - (iterate(e^x-1, x, 3/4, -1))) < 10^-27*(1 + abs(iterate(e^x-1, x, 3/4, -1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2, iterate(e^x-1, x, 1, -1)) - (iterate(e^x-1, x, 3/2, -1))) < 10^-27*(1 + abs(iterate(e^x-1, x, 3/2, -1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2, iterate(e^x-1, x, 1/2, -0.5)) - (iterate(e^x-1, x, 1, -0.5))) < 10^-27*(1 + abs(iterate(e^x-1, x, 1, -0.5)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/4, iterate(e^x-1, x, 1/2, -0.5)) - (iterate(e^x-1, x, 3/4, -0.5))) < 10^-27*(1 + abs(iterate(e^x-1, x, 3/4, -0.5)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2, iterate(e^x-1, x, 1, -0.5)) - (iterate(e^x-1, x, 3/2, -0.5))) < 10^-27*(1 + abs(iterate(e^x-1, x, 3/2, -0.5)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2, iterate(e^x-1, x, 1/2, 1)) - (iterate(e^x-1, x, 1, 1))) < 10^-27*(1 + abs(iterate(e^x-1, x, 1, 1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 3/10, iterate(e^x-1, x, 9/20, 1)) - (iterate(e^x-1, x, 3/4, 1))) < 10^-27*(1 + abs(iterate(e^x-1, x, 3/4, 1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/3, iterate(e^x-1, x, -1/3, 1)) - (1)) < 10^-27*(1 + abs(1))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2, iterate(e^x-1, x, 1/2, 0.5)) - (iterate(e^x-1, x, 1, 0.5))) < 10^-27*(1 + abs(iterate(e^x-1, x, 1, 0.5)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 3/10, iterate(e^x-1, x, 9/20, 0.5)) - (iterate(e^x-1, x, 3/4, 0.5))) < 10^-27*(1 + abs(iterate(e^x-1, x, 3/4, 0.5)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/3, iterate(e^x-1, x, -1/3, 0.5)) - (0.5)) < 10^-27*(1 + abs(0.5))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2, iterate(e^x-1, x, 1/2, 1, 0.1), 0.1) - (iterate(e^x-1, x, 1, 1, 0.1))) < 10^-27*(1 + abs(iterate(e^x-1, x, 1, 1, 0.1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2, iterate(e^x-1, x, 3/2, 1, 0.1), 0.1) - (iterate(e^x-1, x, 2, 1, 0.1))) < 10^-27*(1 + abs(iterate(e^x-1, x, 2, 1, 0.1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1, iterate(e^x-1, x, 1/2, 1, 0.1), 0.1) - (iterate(e^x-1, x, 3/2, 1, 0.1))) < 10^-27*(1 + abs(iterate(e^x-1, x, 3/2, 1, 0.1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2, iterate(e^x-1, x, 1/2, 0.5, 0.1), 0.1) - (iterate(e^x-1, x, 1, 0.5, 0.1))) < 10^-27*(1 + abs(iterate(e^x-1, x, 1, 0.5, 0.1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2, iterate(e^x-1, x, 3/2, 0.5, 0.1), 0.1) - (iterate(e^x-1, x, 2, 0.5, 0.1))) < 10^-27*(1 + abs(iterate(e^x-1, x, 2, 0.5, 0.1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1, iterate(e^x-1, x, 1/2, 0.5, 0.1), 0.1) - (iterate(e^x-1, x, 3/2, 0.5, 0.1))) < 10^-27*(1 + abs(iterate(e^x-1, x, 3/2, 0.5, 0.1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2, iterate(e^x-1, x, 1/2, -1, -0.1), -0.1) - (iterate(e^x-1, x, 1, -1, -0.1))) < 10^-27*(1 + abs(iterate(e^x-1, x, 1, -1, -0.1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 3/10, iterate(e^x-1, x, 9/20, -1, -0.1), -0.1) - (iterate(e^x-1, x, 3/4, -1, -0.1))) < 10^-27*(1 + abs(iterate(e^x-1, x, 3/4, -1, -0.1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, i, iterate(e^x-1, x, -i, -1, -0.1), -0.1) - (-1)) < 10^-27*(1 + abs(-1))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2, iterate(e^x-1, x, 1/2, -0.5+0.2i, -0.1), -0.1) - (iterate(e^x-1, x, 1, -0.5+0.2i, -0.1))) < 10^-27*(1 + abs(iterate(e^x-1, x, 1, -0.5+0.2i, -0.1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 3/10, iterate(e^x-1, x, 9/20, -0.5+0.2i, -0.1), -0.1) - (iterate(e^x-1, x, 3/4, -0.5+0.2i, -0.1))) < 10^-27*(1 + abs(iterate(e^x-1, x, 3/4, -0.5+0.2i, -0.1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, i, iterate(e^x-1, x, -i, -0.5+0.2i, -0.1), -0.1) - (-0.5+0.2i)) < 10^-27*(1 + abs(-0.5+0.2i))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(sin(x), x, i/2, iterate(sin(x), x, -i/2, 0.5i, 0.1i), 0.1i) - (0.5i)) < 10^-27*(1 + abs(0.5i))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, i, iterate(e^x-1, x, -i, -1)) - (-1)) < 10^-27*(1 + abs(-1))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2+i/3, iterate(e^x-1, x, 1/2-i/3, -1)) - (iterate(e^x-1, x, 1, -1))) < 10^-27*(1 + abs(iterate(e^x-1, x, 1, -1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
];

const STEPS_ADD_UP_9: &[(&str, &str)] = &[
    (
        r"abs(iterate(e^x-1, x, i/2, iterate(e^x-1, x, i/2, -1)) - (iterate(e^x-1, x, i, -1))) < 10^-27*(1 + abs(iterate(e^x-1, x, i, -1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, i, iterate(e^x-1, x, -i, -0.5+0.2i)) - (-0.5+0.2i)) < 10^-27*(1 + abs(-0.5+0.2i))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2+i/3, iterate(e^x-1, x, 1/2-i/3, -0.5+0.2i)) - (iterate(e^x-1, x, 1, -0.5+0.2i))) < 10^-27*(1 + abs(iterate(e^x-1, x, 1, -0.5+0.2i)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, i/2, iterate(e^x-1, x, i/2, -0.5+0.2i)) - (iterate(e^x-1, x, i, -0.5+0.2i))) < 10^-27*(1 + abs(iterate(e^x-1, x, i, -0.5+0.2i)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(ln(1+x), x, i, iterate(ln(1+x), x, -i, 1)) - (1)) < 10^-27*(1 + abs(1))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(ln(1+x), x, 1/2+i/3, iterate(ln(1+x), x, 1/2-i/3, 1)) - (iterate(ln(1+x), x, 1, 1))) < 10^-27*(1 + abs(iterate(ln(1+x), x, 1, 1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(ln(1+x), x, i/2, iterate(ln(1+x), x, i/2, 1)) - (iterate(ln(1+x), x, i, 1))) < 10^-27*(1 + abs(iterate(ln(1+x), x, i, 1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(ln(1+x), x, i, iterate(ln(1+x), x, -i, 0.5+0.5i)) - (0.5+0.5i)) < 10^-27*(1 + abs(0.5+0.5i))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(ln(1+x), x, 1/2+i/3, iterate(ln(1+x), x, 1/2-i/3, 0.5+0.5i)) - (iterate(ln(1+x), x, 1, 0.5+0.5i))) < 10^-27*(1 + abs(iterate(ln(1+x), x, 1, 0.5+0.5i)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(ln(1+x), x, i/2, iterate(ln(1+x), x, i/2, 0.5+0.5i)) - (iterate(ln(1+x), x, i, 0.5+0.5i))) < 10^-27*(1 + abs(iterate(ln(1+x), x, i, 0.5+0.5i)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x+x^2, x, 1/2, iterate(x+x^2, x, 1/2, 0.5, 0), 0) - (iterate(x+x^2, x, 1, 0.5, 0))) < 10^-27*(1 + abs(iterate(x+x^2, x, 1, 0.5, 0)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x+x^2, x, 1/2, iterate(x+x^2, x, 3/2, 0.5, 0), 0) - (iterate(x+x^2, x, 2, 0.5, 0))) < 10^-27*(1 + abs(iterate(x+x^2, x, 2, 0.5, 0)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x+x^2, x, 1, iterate(x+x^2, x, 1/2, 0.5, 0), 0) - (iterate(x+x^2, x, 3/2, 0.5, 0))) < 10^-27*(1 + abs(iterate(x+x^2, x, 3/2, 0.5, 0)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x+x^2, x, 1/2, iterate(x+x^2, x, 1/2, 0.1, 0), 0) - (iterate(x+x^2, x, 1, 0.1, 0))) < 10^-27*(1 + abs(iterate(x+x^2, x, 1, 0.1, 0)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x+x^2, x, 1/2, iterate(x+x^2, x, 3/2, 0.1, 0), 0) - (iterate(x+x^2, x, 2, 0.1, 0))) < 10^-27*(1 + abs(iterate(x+x^2, x, 2, 0.1, 0)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x+x^2, x, 1, iterate(x+x^2, x, 1/2, 0.1, 0), 0) - (iterate(x+x^2, x, 3/2, 0.1, 0))) < 10^-27*(1 + abs(iterate(x+x^2, x, 3/2, 0.1, 0)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(2x+sin(x), x, i, iterate(2x+sin(x), x, -i, 0.3)) - (0.3)) < 10^-27*(1 + abs(0.3))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"abs(iterate(2x+sin(x), x, i/2, iterate(2x+sin(x), x, i/2, 0.3)) - (iterate(2x+sin(x), x, i, 0.3))) < 10^-27*(1 + abs(iterate(2x+sin(x), x, i, 0.3)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 0 (slope 3)",
    ),
    (
        r"abs(iterate(x^2-1, x, 1/2, iterate(x^2-1, x, 1/2, -0.3)) - (iterate(x^2-1, x, 1, -0.3))) < 10^-27*(1 + abs(iterate(x^2-1, x, 1, -0.3)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8366600265|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.3441008139; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"abs(iterate(x^2-1, x, 1/3, iterate(x^2-1, x, 2/3, -0.3)) - (iterate(x^2-1, x, 1, -0.3))) < 10^-27*(1 + abs(iterate(x^2-1, x, 1, -0.3)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8366600265|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.1413613723; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"abs(iterate(x^2-1, x, 3/10, iterate(x^2-1, x, 9/20, -0.3)) - (iterate(x^2-1, x, 3/4, -0.3))) < 10^-27*(1 + abs(iterate(x^2-1, x, 3/4, -0.3)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8366600265|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.4014491727",
    ),
    (
        r"abs(iterate(x^2-1, x, 1, iterate(x^2-1, x, 1/2, -0.3)) - (iterate(x^2-1, x, 3/2, -0.3))) < 10^-27*(1 + abs(iterate(x^2-1, x, 3/2, -0.3)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8366600265|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.8366600265; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"abs(iterate(x^2-1, x, 1/4, iterate(x^2-1, x, 1/2, -0.5)) - (iterate(x^2-1, x, 3/4, -0.5))) < 10^-27*(1 + abs(iterate(x^2-1, x, 3/4, -0.5)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.7071067812|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.1612033816; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.7071067812; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"abs(iterate(x^2-1, x, 3/10, iterate(x^2-1, x, 9/20, -0.5)) - (iterate(x^2-1, x, 3/4, -0.5))) < 10^-27*(1 + abs(iterate(x^2-1, x, 3/4, -0.5)))",
        r"true|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.7071067812|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.2233284255; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on|note: iterate: regular iteration at the repelling fixed point 1.618033989 (slope 3.236067977); the point is past a fold of the map, as seen from the fixed point: it is taken as the image of 0.7071067812; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on",
    ),
    (
        r"abs(iterate(x^2-1, x, 1/2, iterate(x^2-1, x, 1/2, -0.3, -0.6), -0.6) - (iterate(x^2-1, x, 1, -0.3, -0.6))) < 10^-27*(1 + abs(iterate(x^2-1, x, 1, -0.3, -0.6)))",
        r"true|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"abs(iterate(x^2-1, x, 1/2, iterate(x^2-1, x, 3/2, -0.3, -0.6), -0.6) - (iterate(x^2-1, x, 2, -0.3, -0.6))) < 10^-27*(1 + abs(iterate(x^2-1, x, 2, -0.3, -0.6)))",
        r"true|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"abs(iterate(x^2-1, x, 1/2, iterate(x^2-1, x, 1, -0.3, -0.6), -0.6) - (iterate(x^2-1, x, 3/2, -0.3, -0.6))) < 10^-27*(1 + abs(iterate(x^2-1, x, 3/2, -0.3, -0.6)))",
        r"true|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"abs(iterate(x^2-1, x, 1/2, iterate(x^2-1, x, 1/2, -0.5, -0.6), -0.6) - (iterate(x^2-1, x, 1, -0.5, -0.6))) < 10^-27*(1 + abs(iterate(x^2-1, x, 1, -0.5, -0.6)))",
        r"true|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"abs(iterate(x^2-1, x, 1/2, iterate(x^2-1, x, 3/2, -0.5, -0.6), -0.6) - (iterate(x^2-1, x, 2, -0.5, -0.6))) < 10^-27*(1 + abs(iterate(x^2-1, x, 2, -0.5, -0.6)))",
        r"true|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"abs(iterate(x^2-1, x, 1/2, iterate(x^2-1, x, 1, -0.5, -0.6), -0.6) - (iterate(x^2-1, x, 3/2, -0.5, -0.6))) < 10^-27*(1 + abs(iterate(x^2-1, x, 3/2, -0.5, -0.6)))",
        r"true|note: iterate: regular iteration at the repelling fixed point -0.6180339887 (slope -1.236067977)",
    ),
    (
        r"abs(iterate(x^2+x^3, x, i, iterate(x^2+x^3, x, -i, 0.1)) - (0.1)) < 10^-27*(1 + abs(0.1))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(x^2+x^3, x, 1/2+i/3, iterate(x^2+x^3, x, 1/2-i/3, 0.1)) - (iterate(x^2+x^3, x, 1, 0.1))) < 10^-27*(1 + abs(iterate(x^2+x^3, x, 1, 0.1)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(x^2+x^3, x, i/2, iterate(x^2+x^3, x, i/2, 0.1)) - (iterate(x^2+x^3, x, i, 0.1))) < 10^-27*(1 + abs(iterate(x^2+x^3, x, i, 0.1)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(x^2+x^3, x, i, iterate(x^2+x^3, x, -i, 0.2+0.1i)) - (0.2+0.1i)) < 10^-27*(1 + abs(0.2+0.1i))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(x^2+x^3, x, 1/2+i/3, iterate(x^2+x^3, x, 1/2-i/3, 0.2+0.1i)) - (iterate(x^2+x^3, x, 1, 0.2+0.1i))) < 10^-27*(1 + abs(iterate(x^2+x^3, x, 1, 0.2+0.1i)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(x^2+x^3, x, i/2, iterate(x^2+x^3, x, i/2, 0.2+0.1i)) - (iterate(x^2+x^3, x, i, 0.2+0.1i))) < 10^-27*(1 + abs(iterate(x^2+x^3, x, i, 0.2+0.1i)))",
        r"true|note: iterate: Boettcher's coordinate at the fixed point 0 (slope 0, degree 2)",
    ),
    (
        r"abs(iterate(x^2+1, x, i, iterate(x^2+1, x, -i, 2)) - (2)) < 10^-27*(1 + abs(2))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"abs(iterate(x^2+1, x, 1/2+i/3, iterate(x^2+1, x, 1/2-i/3, 2)) - (iterate(x^2+1, x, 1, 2))) < 10^-27*(1 + abs(iterate(x^2+1, x, 1, 2)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"abs(iterate(x^2+1, x, i/2, iterate(x^2+1, x, i/2, 2)) - (iterate(x^2+1, x, i, 2))) < 10^-27*(1 + abs(iterate(x^2+1, x, i, 2)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"abs(iterate(x^2+1, x, i, iterate(x^2+1, x, -i, 3+i)) - (3+i)) < 10^-27*(1 + abs(3+i))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
];

const STEPS_ADD_UP_10: &[(&str, &str)] = &[
    (
        r"abs(iterate(x^2+1, x, 1/2+i/3, iterate(x^2+1, x, 1/2-i/3, 3+i)) - (iterate(x^2+1, x, 1, 3+i))) < 10^-27*(1 + abs(iterate(x^2+1, x, 1, 3+i)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"abs(iterate(x^2+1, x, i/2, iterate(x^2+1, x, i/2, 3+i)) - (iterate(x^2+1, x, i, 3+i))) < 10^-27*(1 + abs(iterate(x^2+1, x, i, 3+i)))",
        r"true|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (
        r"abs(iterate(x/2+x^2/4, x, -100, iterate(x/2+x^2/4, x, 100, 1e-40)) - (1e-40)) < 10^-27*(1 + abs(1e-40))",
        r"true",
    ),
    (
        r"abs(iterate(x/2+x^2/4, x, -100, iterate(x/2+x^2/4, x, 100, 0.3)) - (0.3)) < 10^-27*(1 + abs(0.3))",
        r"true",
    ),
    (
        r"abs(iterate(gamma(x), x, 1/2, iterate(gamma(x), x, 1/2, 1.3)) - (iterate(gamma(x), x, 1, 1.3))) < 10^-27*(1 + abs(iterate(gamma(x), x, 1, 1.3)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 1 (slope -0.5772156649)",
    ),
    (
        r"abs(iterate(gamma(x), x, 1/2, iterate(gamma(x), x, 1/2, 0.8+0.2i)) - (iterate(gamma(x), x, 1, 0.8+0.2i))) < 10^-27*(1 + abs(iterate(gamma(x), x, 1, 0.8+0.2i)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 1 (slope -0.5772156649)",
    ),
    (
        r"abs(iterate((e^x-1)/3, x, 1/2, iterate((e^x-1)/3, x, 1/2, 1e-60)) - (iterate((e^x-1)/3, x, 1, 1e-60))) < 10^-27*(1 + abs(iterate((e^x-1)/3, x, 1, 1e-60)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"abs(iterate((e^x-1)/3, x, 3/10, iterate((e^x-1)/3, x, 9/20, 1e-60)) - (iterate((e^x-1)/3, x, 3/4, 1e-60))) < 10^-27*(1 + abs(iterate((e^x-1)/3, x, 3/4, 1e-60)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"abs(iterate((e^x-1)/3, x, 1/3, iterate((e^x-1)/3, x, -1/3, 1e-60)) - (1e-60)) < 10^-27*(1 + abs(1e-60))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"abs(iterate((e^x-1)/3, x, 1/2, iterate((e^x-1)/3, x, 1/2, -1e-90)) - (iterate((e^x-1)/3, x, 1, -1e-90))) < 10^-27*(1 + abs(iterate((e^x-1)/3, x, 1, -1e-90)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"abs(iterate((e^x-1)/3, x, 3/10, iterate((e^x-1)/3, x, 9/20, -1e-90)) - (iterate((e^x-1)/3, x, 3/4, -1e-90))) < 10^-27*(1 + abs(iterate((e^x-1)/3, x, 3/4, -1e-90)))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"abs(iterate((e^x-1)/3, x, 1/3, iterate((e^x-1)/3, x, -1/3, -1e-90)) - (-1e-90)) < 10^-27*(1 + abs(-1e-90))",
        r"true|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.3333333333)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2, iterate(e^x-1, x, 1/2, 1e-50)) - (iterate(e^x-1, x, 1, 1e-50))) < 10^-27*(1 + abs(iterate(e^x-1, x, 1, 1e-50)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/3, iterate(e^x-1, x, 2/3, 1e-50)) - (iterate(e^x-1, x, 1, 1e-50))) < 10^-27*(1 + abs(iterate(e^x-1, x, 1, 1e-50)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2, iterate(e^x-1, x, -1/2, 1e-50)) - (1e-50)) < 10^-27*(1 + abs(1e-50))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2, iterate(e^x-1, x, 1/2, -1e-70)) - (iterate(e^x-1, x, 1, -1e-70))) < 10^-27*(1 + abs(iterate(e^x-1, x, 1, -1e-70)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/3, iterate(e^x-1, x, 2/3, -1e-70)) - (iterate(e^x-1, x, 1, -1e-70))) < 10^-27*(1 + abs(iterate(e^x-1, x, 1, -1e-70)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^x-1, x, 1/2, iterate(e^x-1, x, -1/2, -1e-70)) - (-1e-70)) < 10^-27*(1 + abs(-1e-70))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(e^(i*sqrt(2))*sin(x), x, 1/2, iterate(e^(i*sqrt(2))*sin(x), x, 1/2, 0.08)) - (iterate(e^(i*sqrt(2))*sin(x), x, 1, 0.08))) < 10^-27*(1 + abs(iterate(e^(i*sqrt(2))*sin(x), x, 1, 0.08)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 0.1559436948 + 0.987765946i of size 1)",
    ),
    (
        r"abs(iterate(e^(i*sqrt(2))*sin(x), x, 1/2, iterate(e^(i*sqrt(2))*sin(x), x, 1/2, 0.05+0.02i)) - (iterate(e^(i*sqrt(2))*sin(x), x, 1, 0.05+0.02i))) < 10^-27*(1 + abs(iterate(e^(i*sqrt(2))*sin(x), x, 1, 0.05+0.02i)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 0.1559436948 + 0.987765946i of size 1)",
    ),
    (
        r"abs(iterate(x+x^2, x, 1/2, iterate(x+x^2, x, 1/2, -0.3)) - (iterate(x+x^2, x, 1, -0.3))) < 10^-27*(1 + abs(iterate(x+x^2, x, 1, -0.3)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x+x^2, x, 1/4, iterate(x+x^2, x, 1/2, -0.3)) - (iterate(x+x^2, x, 3/4, -0.3))) < 10^-27*(1 + abs(iterate(x+x^2, x, 3/4, -0.3)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x+x^2, x, 1/2, iterate(x+x^2, x, 3/2, -0.3)) - (iterate(x+x^2, x, 2, -0.3))) < 10^-27*(1 + abs(iterate(x+x^2, x, 2, -0.3)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x+x^2, x, 1/2, iterate(x+x^2, x, 1/2, -0.5)) - (iterate(x+x^2, x, 1, -0.5))) < 10^-27*(1 + abs(iterate(x+x^2, x, 1, -0.5)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x+x^2, x, 1/4, iterate(x+x^2, x, 1/2, -0.5)) - (iterate(x+x^2, x, 3/4, -0.5))) < 10^-27*(1 + abs(iterate(x+x^2, x, 3/4, -0.5)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x+x^2, x, 1/2, iterate(x+x^2, x, 3/2, -0.5)) - (iterate(x+x^2, x, 2, -0.5))) < 10^-27*(1 + abs(iterate(x+x^2, x, 2, -0.5)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(sin(x), x, 1/2, iterate(sin(x), x, 1/2, 1)) - (iterate(sin(x), x, 1, 1))) < 10^-27*(1 + abs(iterate(sin(x), x, 1, 1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(sin(x), x, 1/3, iterate(sin(x), x, 2/3, 1)) - (iterate(sin(x), x, 1, 1))) < 10^-27*(1 + abs(iterate(sin(x), x, 1, 1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(ln(1+x), x, 1/2, iterate(ln(1+x), x, 1/2, 1)) - (iterate(ln(1+x), x, 1, 1))) < 10^-27*(1 + abs(iterate(ln(1+x), x, 1, 1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(ln(1+x), x, 1/4, iterate(ln(1+x), x, 1/2, 1)) - (iterate(ln(1+x), x, 3/4, 1))) < 10^-27*(1 + abs(iterate(ln(1+x), x, 3/4, 1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(ln(1+x), x, 1/2, iterate(ln(1+x), x, 3/2, 1)) - (iterate(ln(1+x), x, 2, 1))) < 10^-27*(1 + abs(iterate(ln(1+x), x, 2, 1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(ln(1+x), x, 1/2, iterate(ln(1+x), x, 1/2, 3)) - (iterate(ln(1+x), x, 1, 3))) < 10^-27*(1 + abs(iterate(ln(1+x), x, 1, 3)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(ln(1+x), x, 1/4, iterate(ln(1+x), x, 1/2, 3)) - (iterate(ln(1+x), x, 3/4, 3))) < 10^-27*(1 + abs(iterate(ln(1+x), x, 3/4, 3)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(ln(1+x), x, 1/2, iterate(ln(1+x), x, 3/2, 3)) - (iterate(ln(1+x), x, 2, 3))) < 10^-27*(1 + abs(iterate(ln(1+x), x, 2, 3)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x*e^x, x, 1/2, iterate(x*e^x, x, 1/2, 0.5)) - (iterate(x*e^x, x, 1, 0.5))) < 10^-27*(1 + abs(iterate(x*e^x, x, 1, 0.5)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x*e^x, x, 1/4, iterate(x*e^x, x, 1/2, 0.5)) - (iterate(x*e^x, x, 3/4, 0.5))) < 10^-27*(1 + abs(iterate(x*e^x, x, 3/4, 0.5)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x*e^x, x, 1/2, iterate(x*e^x, x, 3/2, 0.5)) - (iterate(x*e^x, x, 2, 0.5))) < 10^-27*(1 + abs(iterate(x*e^x, x, 2, 0.5)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x*e^x, x, 1/2, iterate(x*e^x, x, 1/2, -1)) - (iterate(x*e^x, x, 1, -1))) < 10^-27*(1 + abs(iterate(x*e^x, x, 1, -1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x*e^x, x, 1/4, iterate(x*e^x, x, 1/2, -1)) - (iterate(x*e^x, x, 3/4, -1))) < 10^-27*(1 + abs(iterate(x*e^x, x, 3/4, -1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x*e^x, x, 1/2, iterate(x*e^x, x, 3/2, -1)) - (iterate(x*e^x, x, 2, -1))) < 10^-27*(1 + abs(iterate(x*e^x, x, 2, -1)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1); the point is not on the branch of the map that goes on from the fixed point: this is the iterate at the point there, which the map takes to the same points|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
];

const STEPS_ADD_UP_11: &[(&str, &str)] = &[
    (
        r"abs(iterate(tan(x), x, 1/2, iterate(tan(x), x, 1/2, 0.5)) - (iterate(tan(x), x, 1, 0.5))) < 10^-27*(1 + abs(iterate(tan(x), x, 1, 0.5)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(tan(x), x, 1/3, iterate(tan(x), x, 2/3, 0.5)) - (iterate(tan(x), x, 1, 0.5))) < 10^-27*(1 + abs(iterate(tan(x), x, 1, 0.5)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(sinh(x), x, 1/2, iterate(sinh(x), x, 1/2, 0.5)) - (iterate(sinh(x), x, 1, 0.5))) < 10^-27*(1 + abs(iterate(sinh(x), x, 1, 0.5)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(sinh(x), x, 1/2, iterate(sinh(x), x, -1/2, 0.5)) - (0.5)) < 10^-27*(1 + abs(0.5))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"abs(iterate(x^x, x, 1/2, iterate(x^x, x, 1/2, 2)) - (iterate(x^x, x, 1, 2))) < 10^-27*(1 + abs(iterate(x^x, x, 1, 2)))",
        r"true|note: iterate: regular iteration at the fixed point 1 (slope 1)",
    ),
    (
        r"abs(iterate(x^x, x, 1/2, iterate(x^x, x, 1/2, 1.5)) - (iterate(x^x, x, 1, 1.5))) < 10^-27*(1 + abs(iterate(x^x, x, 1, 1.5)))",
        r"true|note: iterate: regular iteration at the fixed point 1 (slope 1)",
    ),
    (
        r"abs(iterate(x+1/x, x, 1/2, iterate(x+1/x, x, 1/2, 2)) - (iterate(x+1/x, x, 1, 2))) < 10^-27*(1 + abs(iterate(x+1/x, x, 1, 2)))",
        r"true|note: iterate: regular iteration at infinity (slope 1 in 1/x)",
    ),
    (
        r"abs(iterate(x+1/x, x, 1/2, iterate(x+1/x, x, 1/2, 5)) - (iterate(x+1/x, x, 1, 5))) < 10^-27*(1 + abs(iterate(x+1/x, x, 1, 5)))",
        r"true|note: iterate: regular iteration at infinity (slope 1 in 1/x)",
    ),
    (
        r"abs(iterate(e^(i)*x+x^2, x, 1/2, iterate(e^(i)*x+x^2, x, 1/2, 0.05)) - (iterate(e^(i)*x+x^2, x, 1, 0.05))) < 10^-27*(1 + abs(iterate(e^(i)*x+x^2, x, 1, 0.05)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"abs(iterate(e^(i)*x+x^2, x, 3/10, iterate(e^(i)*x+x^2, x, 9/20, 0.05)) - (iterate(e^(i)*x+x^2, x, 3/4, 0.05))) < 10^-27*(1 + abs(iterate(e^(i)*x+x^2, x, 3/4, 0.05)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"abs(iterate(e^(i)*x+x^2, x, 1/3, iterate(e^(i)*x+x^2, x, -1/3, 0.05)) - (0.05)) < 10^-27*(1 + abs(0.05))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"abs(iterate(e^(i)*x+x^2, x, 1/2, iterate(e^(i)*x+x^2, x, 1/2, 0.02i)) - (iterate(e^(i)*x+x^2, x, 1, 0.02i))) < 10^-27*(1 + abs(iterate(e^(i)*x+x^2, x, 1, 0.02i)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"abs(iterate(e^(i)*x+x^2, x, 3/10, iterate(e^(i)*x+x^2, x, 9/20, 0.02i)) - (iterate(e^(i)*x+x^2, x, 3/4, 0.02i))) < 10^-27*(1 + abs(iterate(e^(i)*x+x^2, x, 3/4, 0.02i)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"abs(iterate(e^(i)*x+x^2, x, 1/3, iterate(e^(i)*x+x^2, x, -1/3, 0.02i)) - (0.02i)) < 10^-27*(1 + abs(0.02i))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope 0.5403023059 + 0.8414709848i of size 1)",
    ),
    (
        r"abs(iterate(e^(pi*i*(sqrt(5)-1))*x+x^2, x, 1/2, iterate(e^(pi*i*(sqrt(5)-1))*x+x^2, x, 1/2, 0.05)) - (iterate(e^(pi*i*(sqrt(5)-1))*x+x^2, x, 1, 0.05))) < 10^-27*(1 + abs(iterate(e^(pi*i*(sqrt(5)-1))*x+x^2, x, 1, 0.05)))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope -0.7373688781 - 0.6754902943i of size 1)",
    ),
    (
        r"abs(iterate(e^(pi*i*(sqrt(5)-1))*x+x^2, x, 1/2, iterate(e^(pi*i*(sqrt(5)-1))*x+x^2, x, -1/2, 0.05)) - (0.05)) < 10^-27*(1 + abs(0.05))",
        r"true|note: iterate: regular iteration at the fixed point 0 (slope -0.7373688781 - 0.6754902943i of size 1)",
    ),
];

#[test]
fn closed_linear_and_powers() {
    check(CLOSED_LINEAR_AND_POWERS);
}

#[test]
fn closed_moebius() {
    check(CLOSED_MOEBIUS);
}

#[test]
fn closed_quadratics() {
    check(CLOSED_QUADRATICS);
}

#[test]
fn closed_higher_degrees() {
    check(CLOSED_HIGHER_DEGREES);
}

#[test]
fn closed_exponentials_and_whole_steps() {
    check(CLOSED_EXPONENTIALS_AND_WHOLE_STEPS);
}

#[test]
fn brackets_and_user_functions() {
    check(BRACKETS_AND_USER_FUNCTIONS);
}

#[test]
fn closed_forms_at_numbers() {
    check(CLOSED_FORMS_AT_NUMBERS);
}

#[test]
fn fixed_point_that_is_asked_for() {
    check(FIXED_POINT_THAT_IS_ASKED_FOR);
}

#[test]
fn points_off_the_branch() {
    check(POINTS_OFF_THE_BRANCH);
}

#[test]
fn orbits_that_end_on_the_fixed_point() {
    check(ORBITS_THAT_END_ON_THE_FIXED_POINT);
}

#[test]
fn next_to_a_fixed_point_and_many_steps() {
    check(NEXT_TO_A_FIXED_POINT_AND_MANY_STEPS);
}

#[test]
fn what_has_no_iterate() {
    check(WHAT_HAS_NO_ITERATE);
}

#[test]
fn attracting_1() {
    check(ATTRACTING_1);
}

#[test]
fn attracting_2() {
    check(ATTRACTING_2);
}

#[test]
fn attracting_3() {
    check(ATTRACTING_3);
}

#[test]
fn slope_one_1() {
    check(SLOPE_ONE_1);
}

#[test]
fn slope_one_2() {
    check(SLOPE_ONE_2);
}

#[test]
fn slope_zero_1() {
    check(SLOPE_ZERO_1);
}

#[test]
fn slope_zero_2() {
    check(SLOPE_ZERO_2);
}

#[test]
fn slope_zero_3() {
    check(SLOPE_ZERO_3);
}

#[test]
fn repelling_1() {
    check(REPELLING_1);
}

#[test]
fn repelling_2() {
    check(REPELLING_2);
}

#[test]
fn repelling_3() {
    check(REPELLING_3);
}

#[test]
fn rotations() {
    check(ROTATIONS);
}

#[test]
fn closed_forms_against_coordinates_1() {
    check(CLOSED_FORMS_AGAINST_COORDINATES_1);
}

#[test]
fn closed_forms_against_coordinates_2() {
    check(CLOSED_FORMS_AGAINST_COORDINATES_2);
}

#[test]
fn closed_forms_against_coordinates_3() {
    check(CLOSED_FORMS_AGAINST_COORDINATES_3);
}

#[test]
fn with_50_digits_1() {
    check_with(&["-d", "50"], WITH_50_DIGITS_1);
}

#[test]
fn with_50_digits_2() {
    check_with(&["-d", "50"], WITH_50_DIGITS_2);
}

#[test]
fn with_50_digits_3() {
    check_with(&["-d", "50"], WITH_50_DIGITS_3);
}

#[test]
fn with_50_digits_4() {
    check_with(&["-d", "50"], WITH_50_DIGITS_4);
}

#[test]
fn with_100_digits_1() {
    check_with(&["-d", "100"], WITH_100_DIGITS_1);
}

#[test]
fn with_100_digits_2() {
    check_with(&["-d", "100"], WITH_100_DIGITS_2);
}

#[test]
fn with_100_digits_3() {
    check_with(&["-d", "100"], WITH_100_DIGITS_3);
}

#[test]
fn steps_add_up_1() {
    check(STEPS_ADD_UP_1);
}

#[test]
fn steps_add_up_2() {
    check(STEPS_ADD_UP_2);
}

#[test]
fn steps_add_up_3() {
    check(STEPS_ADD_UP_3);
}

#[test]
fn steps_add_up_4() {
    check(STEPS_ADD_UP_4);
}

#[test]
fn steps_add_up_5() {
    check(STEPS_ADD_UP_5);
}

#[test]
fn steps_add_up_6() {
    check(STEPS_ADD_UP_6);
}

#[test]
fn steps_add_up_7() {
    check(STEPS_ADD_UP_7);
}

#[test]
fn steps_add_up_8() {
    check(STEPS_ADD_UP_8);
}

#[test]
fn steps_add_up_9() {
    check(STEPS_ADD_UP_9);
}

#[test]
fn steps_add_up_10() {
    check(STEPS_ADD_UP_10);
}

#[test]
fn steps_add_up_11() {
    check(STEPS_ADD_UP_11);
}
