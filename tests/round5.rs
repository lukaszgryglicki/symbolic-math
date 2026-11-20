//! The numbers in a base, the walks along the digits of numbers and along the primes, the
//! charts, and the pictures that are saved: runs the program on fixed inputs and compares
//! what it prints. The digits, the paths of the walks and the pixels of the pictures were
//! compared with independent calculations first.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn program() -> Command {
    Command::new(env!("CARGO_BIN_EXE_symbolic-math"))
}

/// A directory of its own for the pictures of a test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("symbolic-math-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a directory for the pictures");
    dir
}

/// What the program prints for an input, with `|` for line ends.
fn answer(dir: Option<&Path>, input: &str) -> String {
    let mut command = program();
    if let Some(dir) = dir {
        command.current_dir(dir);
    }
    let out = command
        .arg(input)
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

/// The size of a JPEG XL file is not a part of what is compared: the encoder may pack a
/// picture a little differently on another kind of processor.
fn sizeless(line: &str) -> String {
    match (line.contains(": JPEG XL, "), line.rfind(", ")) {
        (true, Some(at)) if line.ends_with(" bytes") => format!("{}, N bytes", &line[..at]),
        _ => line.to_string(),
    }
}

fn check_in(dir: Option<&Path>, cases: &[(&str, &str)]) {
    let mut failures = Vec::new();
    for (input, want) in cases {
        let got = sizeless(&answer(dir, input));
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
    check_in(None, cases);
}

const NUMBERS_WRITTEN_IN_A_BASE: &[(&str, &str)] = &[
    (r"16#ff", r"255"),
    (r"16#FF", r"255"),
    (r"16#ff.8", r"511/2|  ~ 255.5"),
    (r"2#101.1", r"11/2|  ~ 5.5"),
    (r"2#0.(01)", r"1/3|  ~ 0.33333333333333333333333333333333"),
    (r"2#0.1(0)", r"1/2|  ~ 0.5"),
    (r"3#0.(1)", r"1/2|  ~ 0.5"),
    (r"10#0.(9)", r"1"),
    (
        r"10#12.3(45)",
        r"679/55|  ~ 12.345454545454545454545454545455",
    ),
    (r"36#z", r"35"),
    (r"36#Az", r"395"),
    (r"37#Az", r"1367"),
    (r"64#Az@_", r"9584575"),
    (r"64#_", r"63"),
    (r"62#Z", r"61"),
    (r"0x1F", r"31"),
    (r"0x1F.8", r"63/2|  ~ 31.5"),
    (r"0xff.(f)", r"256"),
    (r"0b101", r"5"),
    (r"0b0.(01)", r"1/3|  ~ 0.33333333333333333333333333333333"),
    (r"0b0.1", r"1/2|  ~ 0.5"),
    (r"0o17", r"15"),
    (r"0o0.4", r"1/2|  ~ 0.5"),
    (r"0O7.(7)", r"8"),
    (r"1000#001#234", r"1234"),
    (r"1000#1#234", r"1234"),
    (r"1000#001.#500", r"3/2|  ~ 1.5"),
    (
        r"1000#000.(#333)",
        r"1/3|  ~ 0.33333333333333333333333333333333",
    ),
    (r"1000#999#999#999", r"999999999"),
    (r"256#072#105", r"18537"),
    (r"65#64", r"64"),
    (r"65#01#00", r"65"),
    (r"4294967295#4294967294", r"4294967294"),
    (r"-2#101", r"-5"),
    (r"2#101 + 3#12", r"10"),
    (r"16#ff * 2", r"510"),
    (r"16#ff^2", r"65025"),
    (r"2#1/2#11", r"1/3|  ~ 0.33333333333333333333333333333333"),
    (r"16#a.8 + 16#5.8", r"16"),
    (r"16#ffg", r"255g"),
    (r"16#ff x", r"255x"),
    (r"sqrt(16#100)", r"16"),
    (r"[2#1, 2#10, 2#11]", r"[1, 2, 3]"),
    (r"2#0.(01) = 1/3", r"true"),
    (r"16#0.1 = 1/16", r"true"),
    (r"37#", r"error: a digit of base 37 must follow 37#"),
    (r"2#2", r"error: a digit of base 2 must follow 2#"),
    (r"2#", r"error: a digit of base 2 must follow 2#"),
    (
        r"1#0",
        r"error: the base before # is a whole number from 2 to 4294967295, not 1",
    ),
    (
        r"0#1",
        r"error: the base before # is a whole number from 2 to 4294967295, not 0",
    ),
    (r"65#65", r"error: 65 is no digit of base 65"),
    (r"1000#1234", r"error: 1234 is no digit of base 1000"),
    (
        r"1000#001.5",
        r"error: every digit of base 1000 stands after a #, also after the point: .#5",
    ),
    (
        r"4294967296#1",
        r"error: the base before # is a whole number from 2 to 4294967295, not 4294967296",
    ),
    (r"x#1", r"error: unexpected character `#` at position 2"),
    (r"0x", r"0"),
    (r"0b2", r"0"),
];

const FROM_ONE_BASE_TO_ANOTHER: &[(&str, &str)] = &[
    (r"base_conv(15, 10, 16)", r"f"),
    (r"base_conv(fe, 16, 10)", r"254"),
    (r"base_conv (fe, 16, 10)", r"254"),
    (r"base_conv(FE, 16, 10)", r"254"),
    (r"base_conv(ff.8, 16, 2)", r"11111111.1"),
    (r"base_conv(-ff.8, 16, 8)", r"-377.4"),
    (r"base_conv(0.(3), 10, 3)", r"0.1"),
    (r"base_conv(0.1, 10, 2)", r"0.0(0011)"),
    (r"base_conv(0.1(6), 10, 6)", r"0.1"),
    (r"base_conv(101, 2, 10)", r"5"),
    (r"base_conv(   101   , 2, 10)", r"5"),
    (r"base_conv(z, 36, 10)", r"35"),
    (r"base_conv(zz, 36, 2)", r"10100001111"),
    (r"base_conv(Az, 64, 10)", r"2339"),
    (r"base_conv(@_, 64, 10)", r"4031"),
    (r"base_conv(2^64 - 1, 10, 64)", r"f__________"),
    (r"base_conv(1000, 10, 1000)", r"#001#000"),
    (r"base_conv(999, 10, 1000)", r"#999"),
    (r"base_conv(1234, 10, 1000)", r"#001#234"),
    (r"base_conv(#001#234, 1000, 10)", r"1234"),
    (r"base_conv(#1#234, 1000, 10)", r"1234"),
    (r"base_conv(#001#234.#500, 1000, 10)", r"1234.5"),
    (r"base_conv(#000.(#333), 1000, 3)", r"0.1"),
    (r"base_conv(1/3, 10, 1000)", r"#000.(#333)"),
    (r"base_conv(1/7, 10, 65)", r"#00.(#09#18#37)"),
    (r"base_conv(255, 10, 100)", r"#02#55"),
    (r"base_conv(255, 10, 4294967295)", r"#0000000255"),
    (
        r"base_conv(2^40, 10, 4294967295)",
        r"#0000000256#0000000256",
    ),
    (r"base_conv(12345678901234567890, 10, 62)", r"eHZl6hWz5OW"),
    (r"base_conv(1/3, 10, 2)", r"0.(01)"),
    (r"base_conv(11/101, 2, 10)", r"0.6"),
    (r"base_conv(1/11, 2, 2)", r"0.(01)"),
    (r"base_conv(-ff/a, 16, 10)", r"-25.5"),
    (r"base_conv(101 + 11, 2, 10)", r"8"),
    (r"base_conv(ff + 1, 16, 10)", r"256"),
    (r"base_conv(2^10, 10, 2)", r"10000000000"),
    (
        r"base_conv(sqrt(2), 10, 2, 40)",
        r"1.011010100000100111100110011001111111001...",
    ),
    (
        r"base_conv(sqrt(10), 2, 2, 20)",
        r"1.0110101000001001111...",
    ),
    (r"base_conv(64#Hello, 64, 10)", r"725177688"),
    (r"base_conv(64#Hello, 16, 10)", r"725177688"),
    (r"base_conv(16#ff, 16, 2)", r"11111111"),
    (r"base_conv(-16#ff, 16, 10)", r"-255"),
    (r"base_conv(1000#001#234, 1000, 10)", r"1234"),
    (r"base_conv(16#ff + 2#1, 8, 10)", r"256"),
    (r"base_conv(2#101, 256, 10)", r"5"),
    (r"base_conv(16#ff.8, 2, 2)", r"11111111.1"),
    (r"base_conv(64#Hello, 64)", r"Hello"),
    (
        r"base_conv(7#8, 16, 10)",
        r"error: a digit of base 7 must follow 7#",
    ),
    (
        r"base_conv(1#0, 16, 10)",
        r"error: the base before # is a whole number from 2 to 4294967295, not 1",
    ),
    (
        r"base_conv(#8, 7, 10)",
        r##"error: "#8" is no number written in base 7"##,
    ),
    (r"base_conv(pi, 16)", r"3.243f6a8885a308d313198a2e03..."),
    (
        r"base_conv(e, 2)",
        r"10.101101111110000101010001011000101000101011101101001010100110101010111111011100010101100010000000100111001...",
    ),
    (r"base_conv(pi, 10, 16)", r"3.243f6a8885a308d313198a2e03..."),
    (r"base_conv(e, 16, 2)", r"1110"),
    (
        r"base_conv(e, 10, 2, 30)",
        r"10.1011011111100001010100010110...",
    ),
    (r"base_conv(pi, 36, 10)", r"918"),
    (
        r"base_conv(pi, 1000)",
        r"#003.#141#592#653#589#793#238#462#643#383#279...",
    ),
    (
        r"base_conv(pi, 10, 1000, 8)",
        r"#003.#141#592#653#589#793#238#462...",
    ),
    (r"base_conv(pi, 64)", r"3.93ZGy8mz2dcj6oEK0..."),
    (r"base_conv(pi, 16, 50)", r"3.73N44ch1eJL929oh85..."),
    (
        r"base_conv(pi, 10, 16, 50)",
        r"3.243f6a8885a308d313198a2e03707344a4093822299f31d00...",
    ),
    (r"base_conv(golden, 10, 3, 20)", r"1.1212001122021210200..."),
    (r"base_conv(sin(1), 2, 10, 20)", r"0.8414709848078965066..."),
    (r"base_conv(sqrt(2), 60)", r"1.oPa7K64IOsPkyqk4v..."),
    (
        r"base_conv(-pi, 7, 10)",
        r"-3.14159265358979323846264338327950...",
    ),
    (r"base_conv(1 + i, 10, 2)", r"1 + 1*i"),
    (r"base_conv(1/3 + i/7, 10, 2)", r"0.(01) + 0.(001)*i"),
    (
        r"base_conv(e + pi*i, 12, 10)",
        r"2.71828182845904523536028747135266... + 3.14159265358979323846264338327950...*i",
    ),
    (r"base_conv(i, 2)", r"1*i"),
    (r"base_conv(-i/3, 2)", r"-0.(01)*i"),
    (r"base_conv(0, 10, 5)", r"0"),
    (r"base_conv([10, 255, 1/3], 16)", r"[a, ff, 0.(5)]"),
    (r"base_conv([10, 255, 1/3], 10, 16)", r"[a, ff, 0.(5)]"),
    (r"base_conv([1, [2, 3]], 2)", r"[1, [10, 11]]"),
    (
        r"base_conv(Hello, 256, 10)",
        r#"310939249775|note: "Hello" is read character by character: #072#101#108#108#111"#,
    ),
    (
        r#"base_conv("Hello world", 256, 10)"#,
        r#"87521618088882671231069284|note: "Hello world" is read character by character: #072#101#108#108#111#032#119#111#114#108#100"#,
    ),
    (
        r"base_conv(310939249775, 10, 256)",
        r#"#072#101#108#108#111|note: as a text, one character for each digit: "Hello""#,
    ),
    (
        r"base_conv(310939249775, 256)",
        r#"#072#101#108#108#111|note: as a text, one character for each digit: "Hello""#,
    ),
    (
        r"base_conv(255, 256, 10)",
        r#"3290421|note: "255" is read character by character: #050#053#053"#,
    ),
    (r"base_conv(#255, 256, 10)", r"255"),
    (r"base_conv(#072#101, 256, 16)", r"4865"),
    (r"base_conv(1e5, 10, 2)", r"11000011010100000"),
    (r"base_conv(1e5, 16, 10)", r"485"),
    (r#"base_conv("0.(01)", 2, 10)"#, r"0.(3)"),
    (r#"base_conv("fe", 16, 10)"#, r"254"),
    (r"x := 255; base_conv(x, 10, 16)", r"255|ff"),
    (r"a := 255; base_conv(a, 16, 2)", r"255|1010"),
    (r"base_conv(15, 10, 16) + 1", r"16"),
    (r"base_conv(1/7, 10, 10)", r"0.(142857)"),
    (r"base_conv(1/7, 10, 10, 20)", r"0.1428571428571428571..."),
    (r"base_conv(123456789/1000, 10, 10, 3)", r"123456..."),
    (r"base_conv(123456, 10, 10, 3)", r"123456"),
    (r"baseconv(255, 10, 2)", r"11111111"),
    (r"base_convert(255, 10, 2)", r"11111111"),
    (r"convbase(255, 10, 2)", r"11111111"),
    (
        r"base_conv(12, 2, 10)",
        r#"error: "12" is no number written in base 2"#,
    ),
    (
        r"base_conv(fz, 16, 10)",
        r#"error: "fz" is no number written in base 16"#,
    ),
    (
        r"base_conv(12, 1000, 10)",
        r#"error: "12" is no number written in base 1000"#,
    ),
    (
        r"base_conv(#1000, 1000, 10)",
        r"error: 1000 is no digit of base 1000",
    ),
    (
        r"base_conv(x + 1, 10, 2)",
        r#"error: "x + 1" is no number written in base 10"#,
    ),
    (r"base_conv(1/0, 10, 2)", r#"error: "1/0" divides by zero"#),
    (
        r"base_conv(fe, 16)",
        r"error: fe is no number: base_conv(x, to) writes a value in the base to; base_conv(fe, from, to) reads fe as it is written in the base from",
    ),
    (
        r"base_conv(x^2 = 4, 10, 2)",
        r#"error: "x^2 = 4" is no number written in base 10"#,
    ),
    (
        r"base_conv(255, 10, 1)",
        r"error: the base is a whole number from 2 to 4294967295, not 1",
    ),
    (
        r"base_conv(255, 1, 10)",
        r"error: the base is a whole number from 2 to 4294967295, not 1",
    ),
    (
        r"base_conv(255, 10, 4294967296)",
        r"error: the base is a whole number from 2 to 4294967295, not 4294967296",
    ),
    (
        r"base_conv(255, 2.5, 10)",
        r"error: the base is a whole number from 2 to 4294967295, not 5/2",
    ),
    (
        r"base_conv(7, 10, 2, 0)",
        r"error: the number of digits is from 1 to 10000000, not 0",
    ),
    (
        r"base_conv(7)",
        r"error: base_conv takes 2 to 4 argument(s), 1 given",
    ),
    (
        r"base_conv(1, 2, 3, 4, 5)",
        r"error: base_conv takes 2 to 4 argument(s), 5 given",
    ),
];

const DIGITS_AND_BLOCKS_THAT_REPEAT: &[(&str, &str)] = &[
    (r"tobase(1/7)", r"0.(142857)"),
    (r"tobase(1/3, 2)", r"0.(01)"),
    (r"tobase(16#ff.8, 2)", r"11111111.1"),
    (r"tobase(pi, 16)", r"3.243f6a8885a308d313198a2e03..."),
    (r"tobase(pi, 2, 20)", r"11.001001000011111101..."),
    (
        r"tobase(pi, 10, 50)",
        r"3.1415926535897932384626433832795028841971693993751...",
    ),
    (r"tobase(-1/3, 3)", r"-0.1"),
    (r"tobase(1/3 + i/7, 10)", r"0.(3) + 0.(142857)*i"),
    (r"tobase(255, 60)", r"4f"),
    (r"tobase(255, 64)", r"3_"),
    (
        r"tobase(1/1009)",
        r"0.(000991080277502477700693756194251734390485629335976214073339940535183349851337958374628344895936570862239841427155599603567888999008919722497522299306243805748265609514370664023785926660059464816650148662041625371655104063429137760158572844400396432111)",
    ),
    (r"tobase(x, 2)", r"error: x is no number"),
    (
        r"tobase(1/3, 2) + 1",
        r"4/3|  ~ 1.3333333333333333333333333333333",
    ),
    (
        r"tobase(1/3, 2)^2",
        r"1/9|  ~ 0.11111111111111111111111111111111",
    ),
    (r"tobase(1/3, 2) = 1/3", r"true"),
    (
        r"[tobase(1/3, 2), tobase(1/7, 2), tobase(pi, 2, 12)]",
        r"[0.(01), 0.(001), 11.0010010000...]",
    ),
    (r"N(tobase(1/3, 2))", r"0.33333333333333333333333333333333"),
    (
        r"x := tobase(1/3, 2); x; 2x",
        r"0.(01)|0.(01)|2/3|  ~ 0.66666666666666666666666666666667",
    ),
    (r"tobase(tobase(1/3, 2), 3)", r"0.1"),
    (r"tobase(1/7); ans * 7", r"0.(142857)|1"),
    (
        r"tobase(pi, 10, 5) + 0",
        r"pi|  ~ 3.1415926535897932384626433832795",
    ),
    (r"tobase(100, 10, 2)", r"100"),
    (r"tobase(123456789/1000, 10, 8)", r"123456.78..."),
    (r"tobase(pi*10^6, 10, 9)", r"3141592.65..."),
    (
        r"tobase(exp(100), 10, 5)",
        r"26881171418161354484126255515800135873611118...",
    ),
    (
        r"tobase(exp(-100), 10, 50)",
        r"0.0000000000000000000000000000000000000000000372007...",
    ),
    (r"tobase(10^-40, 10, 5)", r"0.0000..."),
    (r"tobase(0.1, 2)", r"0.0(0011)"),
    (r"tobase(1.5, 2)", r"1.1"),
    (
        r"N(sqrt(2)); tobase(ans, 10)",
        r"1.4142135623730950488016887242097|1.4142135623730950488016887242097|note: 1.4142135623730950488016887242097 is known with 32 digits: its digits end there",
    ),
    (r"tobase(0, 5)", r"0"),
    (r"tobase(2^100, 36)", r"3ewfdnca0n6ld1ggvfgg"),
    (
        r"tobase(sqrt(2), 3, 30)",
        r"1.10201122122200121221222001121...",
    ),
    (
        r"tobase(10^40 + 1/3, 16)",
        r"1d6329f1c35ca4bfabb9f5610000000000.(5)",
    ),
    (r"tobase(sqrt(-2), 10, 10)", r"1.414213562...*i"),
    (
        r"tobase(1/3, 1)",
        r"error: the base is a whole number from 2 to 4294967295, not 1",
    ),
    (
        r"tobase(1/3, 2.5)",
        r"error: the base is a whole number from 2 to 4294967295, not 5/2",
    ),
    (r"tobase(inf, 2)", r"error: inf is no number"),
    (r"tobase([1/2, 1/3], 2)", r"[0.1, 0.(01)]"),
    (
        r"tobase(quat(1, 0, 1, 0), 2)",
        r"error: quat(1, 0, 1, 0) is a quaternion: the digits of real and of complex numbers are written",
    ),
    (r"inbase(255, 2)", r"11111111"),
    (r"tofraction(2#0.(01), 2)", r"1/11"),
    (r"tofraction(0.(3))", r"1/3"),
    (r"tofraction(0.(142857))", r"1/7"),
    (r"tofraction(0.75, 16)", r"3/4"),
    (r"tofraction(-5/3 + 2i, 16)", r"-5/3 + 2*i"),
    (r"tofraction(1/3, 1000)", r"#001/#003"),
    (r"tofraction(22/7, 64)", r"m/7"),
    (r"tofraction(tobase(1/3, 2), 3)", r"1/10"),
    (
        r"tofraction(pi, 2)",
        r"error: pi is no fraction of whole numbers",
    ),
    (r"tofraction(7)", r"7"),
    (r"digits(1/7)", r"[[0], [], [1, 4, 2, 8, 5, 7]]"),
    (r"digits(1/4, 10)", r"[[0], [2, 5], []]"),
    (r"digits(255, 16)", r"[[15, 15], [], []]"),
    (r"digits(pi, 60, 5)", r"[[3], [8, 29, 44, 0]]"),
    (
        r"digits(1/7, 10, 10)",
        r"[[0], [1, 4, 2, 8, 5, 7, 1, 4, 2]]",
    ),
    (r"digits(1/3, 60)", r"[[0], [20], []]"),
    (r"digits(123456, 10, 3)", r"[[1, 2, 3, 4, 5, 6], []]"),
    (r"digits(pi*10^6, 10, 3)", r"[[3, 1, 4, 1, 5, 9, 2], []]"),
    (r"digits(1/3, 2, 5)", r"[[0], [0, 1, 0, 1]]"),
    (r"digits(5, 2, 1)", r"[[1, 0, 1], []]"),
    (r"digits(0)", r"[[0], [], []]"),
    (r"digits(-1/7)", r"[[0], [], [1, 4, 2, 8, 5, 7]]"),
    (r"digits(10^-40, 10, 5)", r"[[0], [0, 0, 0, 0]]"),
    (
        r"digits(22/7, 4294967295)",
        r"[[3], [], [613566756, 1840700269, 1227133512, 3681400538, 2454267025, 3067833782]]",
    ),
    (
        r"digits(pi)",
        r"error: the digits of pi do not end: digits(x, b, n) lists the first n",
    ),
    (
        r"digits(1/3 + i)",
        r"error: 1/3 + i is no real number: digits takes re and im one by one",
    ),
    (
        r"digits(sqrt(-2), 10, 10)",
        r"error: i*sqrt(2) is no real number: digits takes re and im one by one",
    ),
    (
        r"digits(pi, 10, 0)",
        r"error: the number of digits is from 1 to 10000000, not 0",
    ),
    (
        r"digits(1/97)",
        r"[[0], [], [0, 1, 0, 3, 0, 9, 2, 7, 8, 3, 5, 0, 5, 1, 5, 4, 6, 3, 9, 1, 7, 5, 2, 5, 7, 7, 3, 1, 9, 5, 8, 7, 6, 2, 8, 8, 6, 5, 9, 7, 9, 3, 8, 1, 4, 4, 3, 2, 9, 8, 9, 6, 9, 0, 7, 2, 1, 6, 4, 9, 4, 8, 4, 5, 3, 6, 0, 8, 2, 4, 7, 4, 2, 2, 6, 8, 0, 4, 1, 2, 3, 7, 1, 1, 3, 4, 0, 2, 0, 6, 1, 8, 5, 5, 6, 7]]",
    ),
    (
        r"digits(1/1000003)",
        r"error: the digits repeat after more than 100000 of them: digits(x, b, n) lists the first n",
    ),
    (
        r"frombase([[0], [], [1,4,2,8,5,7]], 10)",
        r"1/7|  ~ 0.14285714285714285714285714285714",
    ),
    (
        r"frombase([[3], [8, 29, 44]], 60)",
        r"84823/27000|  ~ 3.1415925925925925925925925925926",
    ),
    (r#"frombase("ff.8", 16)"#, r"511/2|  ~ 255.5"),
    (
        r#"frombase("0.(01)", 2)"#,
        r"1/3|  ~ 0.33333333333333333333333333333333",
    ),
    (r#"frombase("-ff", 16)"#, r"-255"),
    (r#"frombase("  12 ", 10)"#, r"12"),
    (r##"frombase("#001#234", 1000)"##, r"1234"),
    (
        r#"frombase("1/11", 2)"#,
        r"1/3|  ~ 0.33333333333333333333333333333333",
    ),
    (
        r#"frombase("Hi", 256)"#,
        r#"18537|note: "Hi" is read character by character: #072#105"#,
    ),
    (r"frombase([1, 0, 1], 2)", r"5"),
    (r"frombase([59, 59], 60)", r"3599"),
    (
        r"frombase(digits(22/7, 5), 5)",
        r"22/7|  ~ 3.1428571428571428571428571428571",
    ),
    (r"frombase([], 4)", r"0"),
    (r"frombase([[], [1]], 2)", r"1/2|  ~ 0.5"),
    (r"frombase([[1], [2], [3]], 4)", r"7/4|  ~ 1.75"),
    (r"frombase([a, b], 2)", r"frombase([a, b], 2)"),
    (r"frombase(x, 2)", r"frombase(x, 2)"),
    (r"frombase([1, 2, 3], 2)", r"error: 2 is no digit of base 2"),
    (r"frombase([-1], 2)", r"error: -1 is no digit of base 2"),
    (
        r"frombase([[1], [5], []], 4)",
        r"error: 5 is no digit of base 4",
    ),
    (
        r"frombase([[1], 2], 2)",
        r"error: frombase([digits], b) or frombase([[whole digits], [digits after the point], [digits that repeat]], b)",
    ),
    (
        r"frombase([[1],[2],[3],[4]], 10)",
        r"error: frombase([digits], b) or frombase([[whole digits], [digits after the point], [digits that repeat]], b)",
    ),
    (
        r#"frombase("xyz", 16)"#,
        r#"error: "xyz" is no number written in base 16"#,
    ),
    (
        r#"frombase("16#ff", 16)"#,
        r#"error: "16#ff" is no number written in base 16"#,
    ),
    (r#"frombase("1/0", 10)"#, r#"error: "1/0" divides by zero"#),
    (r"period(1/7)", r"6"),
    (r"period(1/97)", r"96"),
    (r"period(1/3, 2)", r"2"),
    (r"period(1/8)", r"0"),
    (r"period(0)", r"0"),
    (r"period(5)", r"0"),
    (r"period(1/1000003)", r"166667"),
    (r"period(22/7, 1000)", r"2"),
    (
        r"period(pi)",
        r"error: pi is no fraction of whole numbers: its digits do not repeat",
    ),
    (r"period(1/3 + i/7)", r"error: 1/3 + i/7 is no real number"),
    (
        r"digitcounts(pi, 10, 1000)",
        r"[93, 116, 103, 103, 93, 97, 94, 95, 101, 105]",
    ),
    (r"digitcounts(1/4, 10)", r"[1, 0, 1, 0, 0, 1, 0, 0, 0, 0]"),
    (
        r"digitcounts(1/7, 10, 60)",
        r"[1, 10, 10, 0, 10, 10, 0, 9, 10, 0]",
    ),
    (r"digitcounts(pi, 2, 1000)", r"[511, 489]"),
    (
        r"digitcounts(e, 16, 500)",
        r"[31, 23, 27, 30, 26, 29, 27, 34, 36, 22, 28, 42, 38, 40, 29, 38]",
    ),
    (
        r"digitcounts(1/3)",
        r"error: the digits do not end: digitcounts(x, b, n) counts the first n",
    ),
    (
        r"digitcounts(e + pi*i, 10, 100)",
        r"error: e + i*pi is no real number",
    ),
    (r"base(255, 16)", r"[15, 15]"),
    (r"digitsum(255, 16)", r"30"),
    (r#""abc""#, r#""abc""#),
    (r#""abc" + 1"#, r#""abc" + 1"#),
];

const WALKS_ALONG_DIGITS: &[(&str, &str)] = &[
    (
        r"walk(pi, 10, 1000)",
        r" 28.25 │                  │  ⢠⣠⣀  ⢀⡀                     ⢀⡄|       │                  │  ⠉⠁⡏⣹⣰⠙⢅           ⢀⡠⡀      ⢀⡼⡃|       │                  │  ⢠⢬⠛⣹⣒⣤⣼⣧⣷⣾⣶⠁    ⡠⣴⣉⡀⢣⢴⡤⠤⣖⣲⣿⡿⡾|       │                  │⢀⣀⡘⠾⢿⣿⣵⣶⣿⣿⣟⣿⣿⡦    ⠈⣉⣷⣏⣱⣷⠿⠒⡗⢻⣿⡽⠋|       │                ⣀⣠⢒⠾⣿⣦⣤⣺⣯⢿⣿⣿⣮⣣⣿⣷⣏  ⢀  ⢀⢨⠂⢟⣿⣀⡀⢱⣸⣿⠗|       │                ⢺⣿⣿⣿⣿⣿⣿⣿⣽⠛⠊  ⠡⠟⠋⢉⣵⡄ ⠉⣾⣟⠋ ⠈⡱⠘⠃|       │                 ⠘⠛⣿⢿⣻⣿⡊⠉⠁       ⠠⠧⡤⠊⡏⢀⢑⠢⣠⠧⡄⡀|       │             ⢸⠉⢲⣠ ⠼⢬⣿⣿⣿⣞⠆         ⠐⠁ ⢸⡽⣽⣤⣮⣿⣶⣵  ⢀⣀⣠⢤⡀|       │            ⣤⢼⣶⣷⣣⠚⣦⣤⣭⣟⣿⠜            ⢀⡏⠟⠊  ⠈⣿⣿⣭⣿⣽⣤⡤ ⠱|       │            ⢸⣴⣿⠏⣵⣤⣿⡽⡿⡿⠃              ⠓⠒  ⠠⠊⠁⠉⠙⡏⠻⠿⠃|       │            ⠈⢮⡧⣩⠃⠘⠛⠈⠁|       │            ⠘⠗⠋⢪  │ ⢠      ⣠|       │                ⠙⣺⣶⣄⣯⣦⣄⣠⡴⣧⣠⣿⣶⢶⡤⣄|       │                ⠈⢈⣯⣿⡟⠟⠁⠈⠉⠊⠟⣟⠷⣹⣯⠟⠉⠁|       │            ⢠ ⡀   │⠹     ⣠⠴⠿⠭⠺⠾|       │            ⢼⢾⣷⢤  │ ⢀⣀⡀⣴⣖⠟⠿|       │             ⠣⡬⣾⢿⣼⣽⣄⣠⠝⢿⣿⠁⣤⡤|       │               ⠉⣽⢫⡿⠿⣧⡀⢿⢿⣿⣝|     0 │────────────────⠈⠃⠇──⢙⣿⣻⣝⡭⠂─────────────────────────────────────|-1.981 │                  │  ⠚⠛⠓|       └────────────────────────────────────────────────────────────────|        -13.83                                                     34.54|1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53",
    ),
    (
        r"walk(pi, 10, 200, width = 40, height = 10, ascii)",
        r" 10.24 |              |          ** *|       |              |         **********|       |              |         ********|       |       *      |        ********|       |      *****   |     *****|       |       ********* ******|       |        ********** ******|       |           *************|     0 |------------*-*---*******---------------|-1.542 |              |   ***|       +----------------------------------------|        -8.346                             15.22|200 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (9.107, 9.009), 12.81 away; x from -4.618 to 11.49, y from -1.261 to 9.96",
    ),
    (
        r"walk(pi)",
        r" 28.25 │                  │  ⢠⣠⣀  ⢀⡀                     ⢀⡄|       │                  │  ⠉⠁⡏⣹⣰⠙⢅           ⢀⡠⡀      ⢀⡼⡃|       │                  │  ⢠⢬⠛⣹⣒⣤⣼⣧⣷⣾⣶⠁    ⡠⣴⣉⡀⢣⢴⡤⠤⣖⣲⣿⡿⡾|       │                  │⢀⣀⡘⠾⢿⣿⣵⣶⣿⣿⣟⣿⣿⡦    ⠈⣉⣷⣏⣱⣷⠿⠒⡗⢻⣿⡽⠋|       │                ⣀⣠⢒⠾⣿⣦⣤⣺⣯⢿⣿⣿⣮⣣⣿⣷⣏  ⢀  ⢀⢨⠂⢟⣿⣀⡀⢱⣸⣿⠗|       │                ⢺⣿⣿⣿⣿⣿⣿⣿⣽⠛⠊  ⠡⠟⠋⢉⣵⡄ ⠉⣾⣟⠋ ⠈⡱⠘⠃|       │                 ⠘⠛⣿⢿⣻⣿⡊⠉⠁       ⠠⠧⡤⠊⡏⢀⢑⠢⣠⠧⡄⡀|       │             ⢸⠉⢲⣠ ⠼⢬⣿⣿⣿⣞⠆         ⠐⠁ ⢸⡽⣽⣤⣮⣿⣶⣵  ⢀⣀⣠⢤⡀|       │            ⣤⢼⣶⣷⣣⠚⣦⣤⣭⣟⣿⠜            ⢀⡏⠟⠊  ⠈⣿⣿⣭⣿⣽⣤⡤ ⠱|       │            ⢸⣴⣿⠏⣵⣤⣿⡽⡿⡿⠃              ⠓⠒  ⠠⠊⠁⠉⠙⡏⠻⠿⠃|       │            ⠈⢮⡧⣩⠃⠘⠛⠈⠁|       │            ⠘⠗⠋⢪  │ ⢠      ⣠|       │                ⠙⣺⣶⣄⣯⣦⣄⣠⡴⣧⣠⣿⣶⢶⡤⣄|       │                ⠈⢈⣯⣿⡟⠟⠁⠈⠉⠊⠟⣟⠷⣹⣯⠟⠉⠁|       │            ⢠ ⡀   │⠹     ⣠⠴⠿⠭⠺⠾|       │            ⢼⢾⣷⢤  │ ⢀⣀⡀⣴⣖⠟⠿|       │             ⠣⡬⣾⢿⣼⣽⣄⣠⠝⢿⣿⠁⣤⡤|       │               ⠉⣽⢫⡿⠿⣧⡀⢿⢿⣿⣝|     0 │────────────────⠈⠃⠇──⢙⣿⣻⣝⡭⠂─────────────────────────────────────|-1.981 │                  │  ⠚⠛⠓|       └────────────────────────────────────────────────────────────────|        -13.83                                                     34.54|1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53",
    ),
    (
        r"walk(e, 16, 300, width = 40, height = 12)",
        r" 6.449 │                           ⢰⢄ │|       │                 ⢠⣤⠆     ⣀⣤⠘⢫⣫⣇⣄|       │               ⢠⢢⣽⣻⣹⡅    ⣷⣸⢄⣠⣙⣭⣯⡥|       │              ⣎⣾⣽⣿⠊⣉⣷⣦⣀⡄⠉⠉⠈⣉⣿⣿⣿⣟⢧|     0 │──────────⢰───⠯⣾⠿⠉⠧⣽⠽⠻⠉⢳⡀──⣶⣱⣼⣿⠿⠁───────|       │        ⣀⢀⠞⡏⡀ ⠘⣢⡀ ⢸⠏    ⠈⢱⠿⠚⠉⠁⠉|       │       ⣯⢼⠟⠉⣉⡯⠉⠒⠓⠃             │|       │       ⠘⠇  ⠨⣗⣄⣀⠄              │|       │            ⡱⢻⣿⠣⡀             │|       │            ⣇⢼⠁⣀⣈⠶⡀           │|       │            ⠯⠵⣯⣜⢧⡤            │|-11.27 │             ⠠⠿⠟⠛⠂            │|       └────────────────────────────────────────|        -22.67                             6.867|300 steps along the digits 2.b7e151628aed2a6abf71588... of e in base 16: the digit d is a step at 22.5*d degrees|from the origin to (-11.52, -10.74), 15.75 away; x from -17.46 to 1.663, y from -10.85 to 6.027",
    ),
    (
        r"walk(1/7, 10, 60, width = 30, height = 8)",
        r"  4.251 │   ⢠⡆ ⣠              │|        │   ⡎⠸⣠⠋⡆⢰⢇ ⣠  ⡀      │|        │ ⠈⠉⢹⠿⡻⣒⣚⣇⣘⣴⠁⢇⡸⢣ ⣴ ⢀⡄ │|        │   ⡮⠔⢉⢇⡱⢺⠛⢝⡭⡭⣧⠬⢶⠁⢇⡜⢱ ⡾⡀⢠⡄ ⢀|        │  ⠈  ⠚⠁⢀⠧⠊⢡⣃⠜⡹⢙⡫⡿⢟⠷⣒⣻⣀⣣⡎⠸⣀⠟⡄|        │          ⠊ ⠠⠗⠁⢰⠥⠊⡹⣈⠖⡟⠫⣫⢭⢽⡤⠵|        │               ⠁ ⠐⠋ ⡸⠔⠉⣜⡠⠋⠈⣒⠄|-0.1037 │─────────────────────⠠⠴⠥⠤⠤⠊───|        └──────────────────────────────|         -5.978                   2.187|60 steps along the digits 0.14285714285714285714285... of 1/7 in base 10: the digit d is a step at 36*d degrees|from the origin to (-5.601, 3.196), 6.449 away; x from -5.601 to 1.809, y from 0 to 4.147",
    ),
    (
        r"walk(1/7, 10, 61, width = 30, height = 8)",
        r"  4.251 │    ⣰  ⡄              │|        │   ⢠⠃⢇⡸⢱ ⡼⡀⢀⡄ ⢀       │|        │ ⢰⠉⠉⡹⢿⠳⣒⣳⣁⣣⡜⠸⡀⡟⡄⢠⡆ ⣠  │|        │ ⠇ ⢠⡣⠜⢱⢉⠖⡜⠫⣙⢭⠽⣤⠵⡜⠸⣀⠏⡆⢰⢇ ⣤  ⡀|        │   ⠈  ⠓⠁⢰⠥⠋⡸⣠⠋⡏⣛⢭⠿⡛⢶⣚⣎⣘⣤⠃⢇⡸⢣|        │           ⠋ ⠸⠔⠁⣜⠔⠉⣇⡱⢲⠛⢍⡯⡭⢧⠬⠆|        │                ⠁ ⠘⠊ ⡮⠎⢁⣇⠜⠁⢑⡢|-0.1037 │──────────────────────⠤⠼⠥⠤⠴⠁──|        └──────────────────────────────|         -6.133                   2.032|61 steps along the digits 0.14285714285714285714285... of 1/7 in base 10: the digit d is a step at 36*d degrees|from the origin to (-5.91, 2.245), 6.322 away; x from -5.91 to 1.809, y from 0 to 4.147",
    ),
    (
        r"walk(1/4, 10, 100, width = 20, height = 4)",
        r"  0.9748 │    │  ⠉⠉⠉⠉⠉⠉⠉⢹⠁|         │    │        ⢀⠇|         │    │        ⡸|-0.02378 │────⢀⣀⣀⣀⣀⣀⣀⣀⣠⠃──────|         └────────────────────|          -0.5938        1.903|3 steps along the digits 0.25 of 1/4 in base 10, which end there: the digit d is a step at 36*d degrees|from the origin to (0.309, 0.9511), 1 away; x from 0 to 1.309, y from 0 to 0.9511",
    ),
    (
        r"walk(1/4, 4, 10, width = 20, height = 4, ascii)",
        r" 1.025 |      |      *|       |      |      *|       |      |      *|-0.025 |------********------|       +--------------------|        -0.8125        1.812|2 steps along the digits 0.1 of 1/4 in base 4, which end there: the digit d is a step at 90*d degrees|from the origin to (1, 1), 1.414 away; x from 0 to 1, y from 0 to 1",
    ),
    (
        r"walk(sqrt(2), 3, 500, width = 40, height = 12)",
        r" 3.096 │                            ⡀⣰⡄|     0 │───────────────────────────⡜⡑⠙⡟⠂────────|       │                       ⢀⣀⣦⣼⣿⣓⣀⡀|       │                       ⢨⣊⣳⣞⣹⣗⣴⡅|       │                      ⢀⢀⢀⢋⠹⣿⣗⡴⠁|       │                   ⣔⣶⣴⣉⠋⠊⠋⠙⠙⠓⠁│|       │               ⠠⣤ ⠘⠏⣿⣾⣿⠁      │|       │             ⣴⠷⠧⣽⣛⣿⡯⣯⣽⡤⡧      │|       │          ⢀⢄⡜⢤⢧⡤⠿⣿⡿⡮⣿⣎⠰⡥      │|       │        ⠠⡤⢼⢽⢿⣿⠯⡫⣻⣿⣿⡯⣾⠮⠦⠉⠉⠉⠙⠿⠄ │|       │         ⠘⠞⢍⣬⣿⡵⡧⣿⢿⣟⡏⠉         │|-17.82 │           ⠈⠈⠟⠳⠫⠋⠉⠉⠉          │|       └────────────────────────────────────────|        -26.68                             8.179|500 steps along the digits 1.10201122122200121221222... of sqrt(2) in base 3: the digit d is a step at 120*d degrees|from the origin to (-2.5, -12.99), 13.23 away; x from -19 to 0.5, y from -17.32 to 2.598",
    ),
    (
        r"walk(2^100, 7, width = 40, height = 10)",
        r" 2.631 │                      ⢰⡀ │|       │                    ⣀⠤⣘⡜ │|     0 │───────────────────⡸───⠑⠒⠄──────────────|       │                   ⡇     │|       │                  ⣸      │|       │                  ⢇ ⡠⣤⡄  │|       │                  ⢈⣊⠜⠛⠉  │|       │              ⠐⡆ ⡮⡇      │|       │               ⠑⢲⢟⢶⣆⡀    │|-7.746 │               ⠐⠝⠧⠚⠉⠉    │|       └────────────────────────────────────────|        -13.05                             7.707|36 steps along the digits 322653455556104044451560... of 1267650600228229401496703205376 in base 7, which end there: the digit d is a step at 51.43*d degrees|from the origin to (-5.34, -4.961), 7.289 away; x from -5.34 to 0, y from -7.499 to 2.384",
    ),
    (
        r"walk(-pi, 10, 100, width = 30, height = 8)",
        r" 6.253 │      ⢀⠆⢀      │|       │      ⣸⣦⡜⣇⢄    │|       │       ⢣⠘⠼⣄⣝⣤ ⢲⠒⡖    ⡠⡆|       │       ⠈⠒⢤⢺⡿⢿⣷⣾⣶⣷⢢⣀⠖⠉⠘⢲⠖|       │          ⠈⠈⢫⡹⢋⡯⢿⢝⡼⡀ ⡸⠁|       │            ⠣⣣ ⡎⠁ ⠈⠪⢇⡀|     0 │─────────────⠈─⠘────⢈⣽⡿⡄──────|-1.445 │               │    ⠼⠻⠥⠵|       └──────────────────────────────|        -7.539                   6.893|100 steps along the digits 3.14159265358979323846264... of -pi without its sign in base 10: the digit d is a step at 36*d degrees|from the origin to (2.854, 1.453), 3.203 away; x from -4.618 to 3.972, y from -1.261 to 6.07",
    ),
    (
        r"walk(0, 10, 5, width = 20, height = 3)",
        r" 1.05 │        │|    0 │────────⠠⠤⠤⠄────────|-1.05 │        │|      └────────────────────|       -3                 4|1 step along the digits 0 of 0 in base 10, which end there: the digit d is a step at 36*d degrees|from the origin to (1, 0), 1 away; x from 0 to 1, y from 0 to 0",
    ),
    (
        r"walk(pi, 2, 64, width = 40, height = 3)",
        r" 1.05 │          │|    0 │─⠠⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠄─|-1.05 │          │|      └────────────────────────────────────────|       -3.5                                10.5|64 steps along the digits 11.0010010000111111011010... of pi in base 2: the digit d is a step at 180*d degrees|from the origin to (10, 0), 10 away; x from -3 to 10, y from 0 to 0",
    ),
    (
        r"walk(pi, 4, 200, ascii)",
        r" 6.062 |           ||       |           ||       |           ||       |           ||       |           |                             ****|       |           |                             *  *|       |           |                    *        *******|       |           |                    *******  *  *******|       |           |                    *  *  *  *  *  *  *|     0 |--*******--**********--------*************************----------|       |     *******************  *  **********     **********|       |        *  *  *  *  *  *  *     *     *     *  *     *|       |     *************  *************  ****     ****     **********|       |           |     *           *     *************        *|       |           |                       *  *  *|       |           |                       *******|       |           ||       |           ||       |           ||-7.062 |           ||       +----------------------------------------------------------------|        -3.5                                                        17.5|200 steps along the digits 3.02100333122220202011220... of pi in base 4: the digit d is a step at 90*d degrees|from the origin to (15, -3), 15.3 away; x from -3 to 17, y from -4 to 3",
    ),
    (
        r"walk(pi, 1000, 50, width = 40, height = 10)",
        r" 7.427 │                    │ ⢀⣰⣄|       │                 ⢤⡀ ⡀ ⠈⠁⣹⠇|       │                  ⠈⠢⣇⠴⠊⠉⢘⠮⠛⠁|       │                ⣀⣀⡀⢀⡠⠤⠒⠒⠎|       │             ⣀⠴⣰⠡⠮⠲⡇│|       │             ⠑⢄⠙⣖⡦⢤⡘│|       │               ⢩⠂⠛⠊⠁│  ⢀⡀|     0 │────────────⠠⢤⢤⡇────⠠⡤⠮⠃────────────────|       │              ⠉⠧⡀⣠⠒⢤⡖⠁|-2.299 │                ⠈  ⠸⠃|       └────────────────────────────────────────|        -10.07                             9.378|50 steps along the digits #003.#141#592#653#589#793#238#462... of pi in base 1000: the digit d is a step at 0.36*d degrees|from the origin to (-1.752, 6.011), 6.261 away; x from -3.8 to 3.104, y from -2.067 to 7.196",
    ),
    (
        r"walk(16#ff.8, 16, 10, width = 20, height = 4)",
        r"0.01913 │─⠑⠢⠤⣀───────────────|        │ │   ⠉⠑⠢⠤⣀|        │ │       ⠈⠉⠒⠢⢄⣀|-0.7845 │ │       ⣀⣀⣀⣀⣀⣀⣉⣒⣢⣄|        └────────────────────|         -0.08066       1.928|3 steps along the digits ff.8 of 511/2 in base 16, which end there: the digit d is a step at 22.5*d degrees|from the origin to (0.8478, -0.7654), 1.142 away; x from 0 to 1.848, y from -0.7654 to 0",
    ),
    (
        r"n := 100; walk(pi, 10, n, width = 30, height = 8)",
        r"100| 6.253 │      ⢀⠆⢀      │|       │      ⣸⣦⡜⣇⢄    │|       │       ⢣⠘⠼⣄⣝⣤ ⢲⠒⡖    ⡠⡆|       │       ⠈⠒⢤⢺⡿⢿⣷⣾⣶⣷⢢⣀⠖⠉⠘⢲⠖|       │          ⠈⠈⢫⡹⢋⡯⢿⢝⡼⡀ ⡸⠁|       │            ⠣⣣ ⡎⠁ ⠈⠪⢇⡀|     0 │─────────────⠈─⠘────⢈⣽⡿⡄──────|-1.445 │               │    ⠼⠻⠥⠵|       └──────────────────────────────|        -7.539                   6.893|100 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (2.854, 1.453), 3.203 away; x from -4.618 to 3.972, y from -1.261 to 6.07",
    ),
    (
        r"digitwalk(e, 10, 50, width = 30, height = 8)",
        r" 3.786 │        ⠐⢆⡀    ⡀│|       │          ⢸⠁⣀⠔⢹⠁│⣀⢠⢣|       │          ⡧⠋  ⠓⠶⠿⢮⡏⠈⡆|       │   ⡎⠦⣀   ⡸      ⡜⢺⣌⡹⠉⠙⡄|       │  ⠸⠤⠤⣴⣟⢫⠉⠁     ⠰⠕⡕ ⠁  ⢱|     0 │────⠈⠗⠦⡛⠆───────⢰⠕⠊⢣⡸⢇⠇⢀⢄─⣄───|       │       ⠈⠑⢄⡀     │  ⠈⠃⠘⠻⢍⠙⣶⢽⡄|-1.297 │          ⠈⠣⠄   │       ⠉⠉⠉⠁|       └──────────────────────────────|        -5.279                   4.251|50 steps along the digits 2.71828182845904523536028... of e in base 10: the digit d is a step at 36*d degrees|from the origin to (-1.382, -1.176), 1.814 away; x from -4.5 to 3.472, y from -1.176 to 3.665",
    ),
    (
        r"walk3d(e + pi*i, 12, 2000)",
        r"                                ⣆|                            ⢤⣆ ⡀⣿⣄|                            ⢀⣾⣌⣻⣫⡾⠁|                        ⢀⠤⢤⣴⡎⣴⣾⣿⣿⣿⣾⣿⣽⣶⡀|                     ⢠⣤⣴⣾⡆⠘⣿⣿⢻⣯⣿⣿⣾⣿⣿⣿⠋⠛⡄|                     ⠸⣿⢿⢿⠈⣸⣎⡩⣟⢿⣿⣿⣟⣿⣿⣷⣿⣾|                      ⠘⠚⠉⣿⣿⣤⣴⠈⣿⣿⣿⣿⣿⢿⣿⡎⠽⠇|                         ⢘⣛⢻⣻⡏⡿⢺⢿⣿⣿⠿⣽⡿⠗⣷⠂|                         ⠿⢄⣉⣿⢻⡿⣯⣾⣾⣿⣮⡗⠃ ⠉⡆|                          ⠚⠻⠟⢿⣧⣿⣿⣿⣿⡿⣷⣇⣄⢀⡬⠂|                             ⣿⡻⣹⢽⣿⣷⣿⣿⣿⠿⠋|                             ⠿⡏⣻⣤⣻⣿⣿⣿⡻⣆|                           ⢰⣴⣆⡧⣨⢧⣿⣿⣿⣿⣷⣟⡾|                            ⡯⣟⡟⣯⡦⠇⠛⠃⠙⡏⡽⡧⣶⣇|                           ⠰⠗⠣⡄⢸ ⢀ ⢠⠠⣜⡿⠁ ⢧⡀|                             ⠠⣗⣸⣼⣿⣿⣿⡆⣄⣤⣷⢒⣿⡦|                            ⢠⢲⡼⣆⡍⡽⠿⣿⡏⠁⣿⣷⠟⠹⠋|                           ⠠⣼⡏ ⢀⡡⣿⢹⣿⡇ ⢹⠏⠃|                            ⠻⣷⣿⠉⠈⠙⢳⣿  ⠈|                            ⠈⠈⣿   ⠘⠙⠃|2000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (-13.58, 8.12, 3.562), 16.22 away; x from -15.3 to 4.397, y from -7.175 to 16.44, z from -8.732 to 32.53",
    ),
    (
        r"walk3d(e + pi*i, 12, 300, width = 40, height = 12)",
        r"                ⠠⡶⡆|                ⣠⣷⠁|               ⠐⣯⣼⡆|                ⢮⣿⡇|                  ⠹⢮  ⣔⡄⢢⣼⡁|                   ⠈⢲⣞⡼⠈⠓⢻⠓|              ⡄  ⢰ ⢖⢁⣸⠃  ⠸⠴⡀|             ⣧⣻⣶⣾⣿⣀ ⠁⢀⢠⡗⠔⢺⣶⣅|            ⠊⢿⣿⣿⣿⣏⣇⣸⠒⢿⠿⡟⡢⡞⣿⡿⡄|             ⢠⡿⠇⢧⣿⡁  ⢹⡿⣏⠟⠁⠺⠃|             ⠸⡗ ⢠⢻⡇  ⠘⣿⠛⠆|              ⠁ ⠘⠙⠆   ⡏|300 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (-4.647, 9.843, 4.206), 11.67 away; x from -7.83 to 4.397, y from -1.183 to 11.46, z from -2.928 to 10.4",
    ),
    (
        r"walk3d(e + pi*i, 12, 300, width = 40, height = 8, ascii)",
        r"|                  **|                 ***|                  *******|                * **** **|               ***********|                **********|                **** **|300 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (-4.647, 9.843, 4.206), 11.67 away; x from -7.83 to 4.397, y from -1.183 to 11.46, z from -2.928 to 10.4",
    ),
    (
        r"walk3d(pi, 12, 200, width = 40, height = 10)",
        r"||           ⣀⣀⡠⢤⣀⣀|  ⢀⣀⣀⣠⣤⣴⣶⣮⣍⣠⣤⣴⣶⡿⠟⣿⣯⣽⣷⣤⡄⢀|⠈⠛⠿⠽⠤⢼⣵⠛⠻⠛⠓⡯⣿⣯⣻⣯⣷⣾⣾⡿⣿⣿⡿⢾       ⢀⡤⡤⠖|       ⡇⠠⠽⠷⣷⡞⡯⢿⣽⢯⣭⣥⣿⣟⣿⣿⣿⣿⢯  ⢲⣤⣤⣤⣊⡷⣤⣤⣤⣤⠄|       ⠈⠉⠉⣺⣓⠯⠿⠿⢶⠚⢉⡩⠛⠁  ⠙⢟⣿⣽⣓⣻⠛⠛⠃⣀⣯⣽⣝⡳⢿⣒⣄|       ⠒⠉⠉     ⠈⠛⠛⠏⠉     ⠈⠈ ⠑⠛⠛⠚⠊⠉⠙ ⠈⠉|||200 steps along the digits 3.184809493b918664573a621... of the real part and 0 of the imaginary part of pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (0.7321, 3, 0), 3.088 away; x from -3.634 to 7.366, y from -2.928 to 4.17, z from 0 to 0",
    ),
    (
        r"walk3d(i/4, 4, 10, width = 20, height = 5)",
        r"              ⢰|              ⢸|              ⢸|     ⢀⣀       ⢸|       ⠉⠉⠑⠒⠢⠤⢄⣸|2 steps along the digits 0 of the real part and 0.1 of the imaginary part of i/4 in base 4, which end there: the digits a and c of a place are a step at 90*a degrees around the z-axis, 90*c degrees above the plane|from the origin to (1, 0, 1), 1.414 away; x from 0 to 1, y from 0 to 0, z from 0 to 1",
    ),
    (
        r"walk3d(12.5 + 0.25i, 10, 10, width = 20, height = 5)",
        r"        ⠐⠒⠤⠤⣀⡀|             ⠈⡆|              ⠸⡀|               ⣇|    ⢀⣀⣀⣀⣀⣀⡠⠤⠒⠊⠉|4 steps along the digits 12.5 of the real part and 00.25 of the imaginary part of 25/2 + i/4 in base 10, which end there: the digits a and c of a place are a step at 36*a degrees around the z-axis, 36*c degrees above the plane|from the origin to (-0.191, 1.539, 0.9511), 1.819 away; x from -0.191 to 1.118, y from 0 to 1.539, z from 0 to 0.9511",
    ),
    (
        r"walk3d(sqrt(2) + sqrt(3)*i, 10, 400, width = 40, height = 12)",
        r"                      ⣄⣰⣀⡄⣀|                   ⢰⠒⢢⢈⡥⢻⣿⢿⠄|                    ⢳⣿⠳⠃⡆⣳⠛|                    ⠨⣿⣤⣠⣷⣝|                    ⠘⣿⣿⡞⠈⠇|                    ⢢⣇⡿⠻⣥⣄|                 ⢀⣼⣶⣾⡿⠃⣿⠋⢿|                  ⠁⠁⠻⡀|                    ⢀⣗⡠|             ⢼⡏⢀⣺⢶⠠⡔⠜⡾⠦|             ⠸⣿⣾⣿⣨⡎⢷⣦⡀|               ⠹⢽⢿  ⠉⠁|400 steps along the digits 1.41421356237309504880168... of the real part and 1.73205080756887729352744... of the imaginary part of sqrt(2) + i*sqrt(3) in base 10: the digits a and c of a place are a step at 36*a degrees around the z-axis, 36*c degrees above the plane|from the origin to (-6.849, -13.19, -17.38), 22.87 away; x from -9.236 to 4.832, y from -20.33 to 5.931, z from -19.14 to 2.041",
    ),
    (
        r"walk(1 + i)",
        r"error: 1 + i is no real number: walk3d(z, base, digits) walks along a complex number",
    ),
    (r"walk(x)", r"error: x is no number"),
    (
        r"walk(pi, 1)",
        r"error: the base is a whole number from 2 to 4294967295, not 1",
    ),
    (
        r"walk(pi, 10, 0)",
        r"error: the number of digits is a whole number from 1 to 10000000, not 0",
    ),
    (
        r"walk(pi, 10, 10, 10)",
        r"error: walk takes 3 arguments at most; what else belongs to a picture is written as name = value",
    ),
    (
        r"walk(pi, 10, 20000000)",
        r"error: the number of digits is a whole number from 1 to 10000000, not 20000000",
    ),
    (
        r"walk()",
        r"error: walk takes at least 1 argument(s), 0 given",
    ),
    (
        r"walk(pi, gray)",
        r#"error: gray is chosen for a picture that is saved: save_png("walk.png", walk(...), gray); the picture of characters takes width, height and ascii"#,
    ),
    (
        r"walk(pi, width = 4)",
        r"error: the width of a picture of characters is from 8 to 400, not 4",
    ),
    (
        r"walk(pi, height = 500)",
        r"error: the height of a picture of characters is from 2 to 200, not 500",
    ),
    (
        r"walk(pi, widht = 3)",
        r"error: widht is nothing that is chosen for a picture: width, height, bits, channels, level, quality, distance, effort, frames, fps, turn, slices, thickness, margin, palette, color, colour, background, fog, azimuth, elevation, roll, zoom, scale, center, centre, viewpoint, perspective, stereo, title, xlabel, ylabel are chosen with name = value, gray, grey, alpha, lossless, depth, grow, axes, noaxes, grid, ascii, reverse, views, stereo with the word alone",
    ),
    (
        r"walk(pi, bits = 8)",
        r#"error: bits is chosen for a picture that is saved: save_png("walk.png", walk(...), bits = ...); the picture of characters takes width, height and ascii"#,
    ),
    (r"walk3d(x, 10, 5)", r"error: x is no number"),
    (
        r"1 + walk(pi)",
        r#"error: walk must be a statement of its own, or the drawing of save_png("name.png", walk(...))"#,
    ),
    (
        r"x := walk(pi)",
        r#"error: walk must be a statement of its own, or the drawing of save_png("name.png", walk(...))"#,
    ),
];

const WALKS_ALONG_PRIMES: &[(&str, &str)] = &[
    (
        r"primewalk(200, width = 50, height = 10)",
        r"112.9 │                         ⣶⡆ │|      │                       ⢸⣻⡿  │|      │                       ⢠⣼⣶⣠⣤⡄|      │                       ⠠⢼ ⠈⠛⠛⠃|      │                    ⢠⣤⣤⣤⠼   │|      │                    ⠈⠙⣿⣿⣀⣼⣽ │|      │                      ⣞⣿⠚⣿⢻ │|      │                        ⠰⠿⠟⡆│|      │                           ⣷⠆| -8.9 │───────────────────────────⠿⠛⠃────────────────────|      └──────────────────────────────────────────────────|       -172.2                                       132.2|199 sides along the first 200 primes, from 2 to 1223: a step for every whole number, at every prime a turn to the left by 90 degrees, so that every side is as long as the gap between two primes|from the origin to (1, 76), 76.01 away; x from -47 to 7, y from -6 to 110",
    ),
    (
        r"primewalk(6, width = 20, height = 5, ascii)",
        r" 2.1 |       ******|     |       *  | *|   0 |-------*--***-------|     |       *  ||-2.1 |       ******|     +--------------------|      -4.2             4.2|5 sides along the first 6 primes, from 2 to 13: a step for every whole number, at every prime a turn to the left by 90 degrees, so that every side is as long as the gap between two primes|from the origin to (1, -2), 2.236 away; x from -1 to 1, y from -2 to 2",
    ),
    (
        r"primewalk(2000)",
        r"547.5 │                           │ ⢠⣤⣀⡀⢀⢀⣤⡀|      │                           │ ⠘⠺⢿⣷⣿⣟⣿⠁|      │                           │    ⡀⣼⣿⠯⣤⣤⣀|      │                           │ ⢀⣀⣸⡏⠉⠉⢠⣼⣟⣀⣀|      │                          ⢠⣶⣶⣾⡿  ⢀⣸⣿⣿⠉⠙⠛|      │                          ⠈⠻⢷⣷⢴⣿⣿⣿⣿⣿⣛⣀⣠⣄⣀⡀|      │                         ⢠⢤⣤⠤⠿⠿⣿⣟⠈⠿⢿⣿⣿⠙⠯⢤⣿|      │                         ⣏⣋⣀⡀⢀⣀⡛⠋   ⠏⠽⡄  ⠿|      │                         ⡻⣿⣟⠓⠚⢻⡇⣀⣀ ⢀⣠⣿⡷⠂|      │                        ⢛⣿⠿⠿⠻⣧⣀⠙⠛⢻⢿⣿⣿⣿⠇|      │                        ⣿⣿⣷│⠈⣹⡉  ⠘⠛⠿⠟⠉|      │                        ⢿⣿⣷⠆⢠⣍⡇|      │                        ⠸⣿⡟⣲⡾⠁|      │                        ⠠⣶⣛⣿⠃|      │                      ⣀⣤⣶⣷⣄⣀|      │                      ⢿⢿⣿⢻⣿⡿⣿|      │                         ⢐⡟⣷⡶⢶⣖⡆|      │                        ⠻⣿⣵⡆⠁⠈⠉|      │                         ⠉⠿⣇|-19.5 │───────────────────────────⠛⠃───────────────────────────────────|      └────────────────────────────────────────────────────────────────|       -391.6                                                     515.6|1999 sides along the first 2000 primes, from 2 to 17389: a step for every whole number, at every prime a turn to the left by 90 degrees, so that every side is as long as the gap between two primes|from the origin to (-19, 238), 238.8 away; x from -77 to 201, y from -6 to 534",
    ),
    (
        r"primewalk(2000, 6, width = 50, height = 14)",
        r" 178.1 │             │           ⢀⣶⣀⡀|       │             │           ⢠⣿⣿⣿⣦|       │             │           ⠈⣿⣿⣿⣿⣿⡄|       │             │           ⣠⣿⣿⣿⣿⣫⢤|       │             │     ⣀⣀⣴⣿⣧⡤⣿⢿⣿⣿⣿⣿⠉⣀⣀|     0 │────────────⣠⡄───⢠⣼⣿⣿⡿⠈⠉⢷⡶⣿⣿⣿⣟⣁⣤⣿⡟────────────────|       │           ⠠⣾⣿⣿⣦⡘⠛⢿⣿⣿⣿⡆ ⠈⠛⠋⠉⠻⣷⣿⣿⣿⣟⣆|       │             │⠉⣿⣷⣶⣾⠃⠙⠿⠃     ⢼⣿⣿⠿⣿⣿⣿⣄⣠⣤⡀|       │             │ ⠑⠛⣾⣟⡟             ⢨⡿⢧⣇|       │             │ ⢀⣾⣿⣯            ⢠⣶⣷⣭⣅⠉|       │             │ ⠈⠙⠛⠁            ⣿⡟⠿⢿⣃|       │             │                 ⠉⠁ ⣨⡯⠂|       │             │                   ⠘⣷⣿⣆|-245.6 │             │                    ⠈⠻⠁|       └──────────────────────────────────────────────────|        -197.8                                       558.8|1999 sides along the first 2000 primes, from 2 to 17389: a step for every whole number, at every prime a turn to the left by 60 degrees, so that every side is as long as the gap between two primes|from the origin to (327, -218.2), 393.1 away; x from -20 to 381, y from -235.6 to 168",
    ),
    (
        r"primewalk(500, 5, 2, width = 40, height = 12)",
        r"249.1 │              ⢀⣰⡀      │|      │              ⠸⣿⣷⣄     │|      │                ⢘⣽⡤    │|      │                ⠉⣹⡓    │|      │               ⢀⢠⠃     │|      │             ⢀⣸⣿⢿⣶⣶    │|      │              ⠈⠿⠊ ⢻⣷⣇  │|      │                  ⢸⣉⣶⣦⣀⣀|      │                  ⠿⢿⢿⠯⣾⣿⣥⣄⡀|      │                      ⠈│⡿⠟⠃|      │                       ⢰⣾⣧|-6.82 │───────────────────────⠰⠿⠏──────────────|      └────────────────────────────────────────|       -255.3                             171.3|499 sides along the first 500 primes, from 2 to 3571: a step for every whole number, at every prime a turn to the left by 144 degrees, so that every side is as long as the gap between two primes|from the origin to (-87.23, 243), 258.2 away; x from -108.9 to 24.97, y from -0.7265 to 243",
    ),
    (
        r"primewalk(300, 3, width = 40, height = 12)",
        r" 5.543 │────────⠦⣤⣶⣄⣠⡄──────────────────────────|       │       ⢰⡧⣿⣿⣿⠿⢓⠞|       │       ⠚⣿⣧⣿⣳⣾⣮⢄|       │        ⣷⣿⣿⣦⠝⠉⣩⣺⣾⣧|       │         ⢀⣽⣛⣳⡾⣿⢛⣻⣿⠁⣠⡀     ⣄⣀⣀⣀⡄|       │          ⠉ │⠈⠛⠚⠛⢛⣿⣛⣗    ⢀⣈⢆⣀⠎|       │            │    ⣿⣛⣟⣀⡄  ⣔⣎⣾⣯⣷⡀|       │            │     ⢀⢞⡎  ⠘⢒⣿⣗⣿⣟⢗|       │            │ ⣠  ⢠⡊        ⡠⣠⣌⣆|       │            │⠴⢥⣵⣤⣿⣾⡂      ⢠⣟⣻⣿⣿⣶⣆|       │            │⢠⢿⣺⠎⠐⠿⠦⢤⠆    ⠉⠉⡹⣯⠥⠵|-81.75 │            │⠓⠻⠾⠃   ⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠁|       └────────────────────────────────────────|        -45.25                             100.2|299 sides along the first 300 primes, from 2 to 1987: a step for every whole number, at every prime a turn to the left by 120 degrees, so that every side is as long as the gap between two primes|from the origin to (56, -39.84), 68.72 away; x from -19 to 74, y from -79.67 to 3.464",
    ),
    (
        r"primes_walk(100, width = 30, height = 8)",
        r"51.4 │               ⡖⢲  │|     │          ⢠⠤⡤⢤⣿⣿⣼  │|     │         ⠉⠹⠿⠟⠛⢻⡟⢹  │|     │              ⢶⡶⡾  │|     │             ⠘⠻⠟⠋⢹ │|     │                 ⣿⡤⡄|   0 │─────────────────⣿⣯⣥⣀─────────|-7.4 │                 ⠿⠏⠛⠉|     └──────────────────────────────|      -71.12                   39.13|99 sides along the first 100 primes, from 2 to 541: a step for every whole number, at every prime a turn to the left by 90 degrees, so that every side is as long as the gap between two primes|from the origin to (-37, 36), 51.62 away; x from -37 to 5, y from -6 to 50",
    ),
    (
        r"primewalk3d(2000, width = 50, height = 14)",
        r"                                ⣴⣀⡀|                               ⣶⣾⣾⣷⡄|                             ⢀⣯⣿⣯⣿⣿⡇|              ⣤⡄       ⣀⢠⣤  ⣀⢸⣟⣿⣿⣿⣿⠟|            ⠰⢿⣿⣶⣀ ⢴⣶⣿⣿⡿⠟⠹⣯⢦⣾⣿⣿⣯⣩⣿⣿⡿⠇|              ⠙⣿⡟⢰⣄⣿⣿⣿⣧  ⡾⣶⣤⣿⣿⣿⢷⠭⣥⣠⣾⣷|              ⠸⣿⣅⣬⢿⡏⢹⣿⠋  ⠧⠿⠁⠘⠛⣿⣾⣶⣿⣿⣿|              ⢠⣶⣿⡟⠑⠁         ⠻⠛⠿⢻⣿⣟⡏|              ⠈⠛⠛⠁              ⠘⢿⣾⡇⢀|                              ⣠⣤⣼⣾⣿⣟⠛⠛|                              ⠿⠿⢿⡿⠟⠉|                                ⢀⣷|                              ⢸⣯⣷⡁|                              ⠈⢻⡿⠃|1999 sides along the first 2000 primes, from 2 to 17389: a step for every whole number, at every prime the next of the directions x, y, z, -x, -y, -z, so that every side is as long as the gap between two primes|from the origin to (295, -94, -158), 347.6 away; x from -21 to 331, y from -98 to 104, z from -174 to 116",
    ),
    (
        r"primewalk3d(500, 3, width = 40, height = 12)",
        r"                                 ⣠⠴⠂|                              ⣠⠤⠞⠁|                           ⢀⡤⠖⠃|                         ⣠⡼⠉|                       ⣠⠼⠁|                    ⢀⣠⠟⠁|                  ⣠⠼⠉|              ⣠⠤⠶⠋⠁|            ⣀⡴⠃|         ⢀⡤⠞⠁|      ⢀⣠⠖⠋|     ⡴⠋|499 sides along the first 500 primes, from 2 to 3571: a step for every whole number, at every prime the next of the directions x, y, z, so that every side is as long as the gap between two primes|from the origin to (1231, 1144, 1194), 2061 away; x from 0 to 1231, y from 0 to 1144, z from 0 to 1194",
    ),
    (
        r"primewalk3d(500, 4, width = 40, height = 12)",
        r"           ⣴⣶|          ⠠⣾⠇|           ⣻⣄       ⠠⣶⣤⣠|          ⠘⡛⢋⡆      ⠈⣿⣿⣧⠇|         ⣠⣾⣿⡿⣼ ⣠ ⢴⣄⡠⡪⣽⠧ ⢀⣄|         ⠉⢛⣿⣿⡼⠛⢨⣮⣷⠧ ⠼⢷ ⢀⣵⣾⢄|          ⠘⠃⠈  ⣀⠿⡟  ⣀⣼⠆ ⢺⣿⡿|              ⠛⢧⣠⣿⣆⢾⣿⣷⣷⣿⢵⣿|               ⠐⠟⠻⣿⣿⣾⡻⠻⡛⡭⡞  ⣠⠜⡆|                  ⠛⠻⠿⠃⠐⠋ ⣇⢔⡊  ⢱|                         ⢸⡽⣿⡞⢆⣹⠃|                         ⠉⠚⠽⡌⠙⠈|499 sides along the first 500 primes, from 2 to 3571: a step for every whole number, at every prime the next of the directions to the four corners of a tetrahedron, so that every side is as long as the gap between two primes|from the origin to (92.95, 12.12, -103.3), 139.5 away; x from -7.506 to 109.1, y from -38.68 to 47.92, z from -116 to 4.041",
    ),
    (
        r"primes3d_xyz(300, width = 40, height = 10)",
        r"                  ⣶⣦|               ⢀⣠⣰⣿⠅⣀⡀|               ⠸⠿⢾⣽⣿⣧⡇⢀|                 ⠘⠛⣟⣿⣧⣿|                   ⢠⣿⡽⠉|                   ⢸⢿⡿|                   ⠘⠿⢿⣇⠔⢢|                     ⡠⣸⢭⡥|                   ⡎⣹⣿⣿⣟⠃|                   ⠓⠻⢿⡿⠏|299 sides along the first 300 primes, from 2 to 1987: a step for every whole number, at every prime the next of the directions x, y, z, -x, -y, -z, so that every side is as long as the gap between two primes|from the origin to (59, -68, -66), 111.6 away; x from -21 to 65, y from -68 to 20, z from -94 to 6",
    ),
    (
        r"primes3d(300, 6, width = 40, height = 10, ascii)",
        r"|                  **|                ******|                *******|                   *****|                   ****|                   ******|                     ****|                   ******|                   *****|299 sides along the first 300 primes, from 2 to 1987: a step for every whole number, at every prime the next of the directions x, y, z, -x, -y, -z, so that every side is as long as the gap between two primes|from the origin to (59, -68, -66), 111.6 away; x from -21 to 65, y from -68 to 20, z from -94 to 6",
    ),
    (
        r"primemodwalk(5000, width = 50, height = 14)",
        r" 4.624 │                            ⢠⣢   ⣠     │|       │                           ⢘⢝⢟⣻⡫⣻⣛⣟⢝⠖⣺⡢⣲|     0 │───────────────────⢀⡀──────⣿⣹⡱⢿⣏⣿⣿⣿⣹⣷⣿⠋⠈──────────|       │                 ⢀⢄⢥⣯⡀⡀ ⣴⢴⣄⣨⡪⣦⣼⣕⣽⣭⣯⣪⣯⣝⢑│|       │                 ⠸⣹⣭⣯⣍⣇⣴⣿⣱⣿⣾⢼⣿⣿⡧⣿⣿⣿⢼⣿⡿⠣⠃|       │               ⢴⣷⡦⣺⡆⣷⢕⢷⣾⣾⣺⣶⣷⢵⣷⣾⡮⣾⣶⣷⢵⡗⠚⠂│|       │               ⠠⡿⢶⢼⠾⡿⡣⡿⢿⢿⢼⠿⡿⡺⡿⢿⢗⢿⠿⡿⡺⡷⠄ │|       │          ⢀⠴⢷⡸⠪⢦⢺⣿⢽⣭⣿⡯⣿⣿⣿⢽⣿⣿⢺⣿⣿⡗⣿⠿⣮⢪⠁  │|       │          ⡿⡲⠈  ⢙⠿⡿⡺⡯⢿⢗⢿⠿⡿⡺⡿⢿⢽⢿⠯⠋⠻⠛⢿⠕   │|       │          ⢹⢺⣦⡀⡔⡕⠛⠂ ⠈⢻⡗⣿⣿⣟⢺⣿⣿⣱⠛⢍        │|       │           ⢕⣟⣺⡫⠃    ⠈⠨⣻⣓⣗⢝⣟⣻⣪⣳⣀        │|       │          ⠰⣩⣿⣏⣈⣦⣀⣠⡀   ⠈⠹⡿⠹⡻⢏⢈⣉⣷⡁       │|       │          ⢠⣪⣯⣽⣕⡽⠉⠋⠂     ⣯⣢ ⢰⢽⣽⣭⡯       │|-28.79 │             ⠈⠃         ⠑⠔⠖⠫⠊ ⠙        │|       └──────────────────────────────────────────────────|        -47.86                                        11.8|5000 steps, one for every prime p up to 48611: at 45*(p mod 8) degrees|from the origin to (-30.41, -22.33), 37.73 away; x from -36.06 to 0, y from -27.99 to 3.828",
    ),
    (
        r"primemodwalk(5000, 12, width = 50, height = 14)",
        r" 5.356 │                                                │|       │                ⠠⣔⡤⣀⣄⡀                   ⡀ ⡀⢀⡠⣀⡠⣀|     0 │────────────────⠠⣔⡬⣗⡟⣒⢥⣢⠄─────⡀─⡀─⣠⣀⡠⣀⡠⣴⢽⣺⢽⣺⣗⡯⣗⡯⣟─|       │                  ⠉⠓⣽⢲⣱⠺⣱⢎⠒⣀⡀⣀⢺⣱⢺⣱⢺⡗⢏⡗⣏⣿⣹⢺⣹⢺⡗⣏⡗⣍⠗|       │                ⢀⡤⣖⡯⣿⢽⣺⢽⣺⣗⡯⣗⡯⣾⢽⣺⢽⣺⢽⡯⣗⡯⣗⣿⣺⢽⣺⢽⡯⣗⡯⣗⠦|       │              ⢀⡫⣗⡯⣗⡯⣿⢵⣺⢽⡺⣗⡫⣗⡯⣿⢽⣺⢽⣺⢽⡯⣗⡯⣗⣿⣺⢽⣺⢽⡯⣀⠈ │|       │             ⣠⢽⡯⣗⡯⣗⡯⣿⢽⣺⢽⣲⣗⡯⣗⡯⣿⢽⣺⢽⣺⢽⡯⣗⡯⣗⣿⣺⢽⡚⠝⠋⠑  │|       │       ⢀ ⢀ ⡀ ⡀⢥⡮⣖⡮⣖⡮⠶⠵⣲⢵⣲⣗⡮⣖⡮⣶⢵⣲⢵⣲⢵⡬⣖⡮⣀⢸⡢⠕⠂     │|       │     ⢐⡮⣗⡯⢓⣷⣺⢽⣲⢽⡯⣗⡯⣗⡯⣢⠄⡀⢝⣺⣗⡯⣖⡯⣿⢽⡲⢽⣺⢽⡯⣖⡯⣗⡯        │|       │    ⣀⢴⠤⣔⡯⣗⣿⣺⢽⣺⢽⡯⢗⠫⣓⡮⣳⢽⡚⠙⠺⠗⠪⠖⠫⢿⠽⣲⢽⣚⢵⡬⠁           │|       │ ⣀⢀⡠⠜⠹⠊⠡⢎⠥⣷⢼⣹⢼⣹⣏⡧⣏⡧⣏⣟⡙⠔⣁⢀⡀⣀⠤⢄⠼⣡⢜⣹⢜⣹⠋            │|       │  ⠉⠒⠴⡢⢄⠤⢖⠉⠛⠍⠊⠉⠒⢗⡫⣗⠯⢗⠟⣲⠄ ⢽⡯⣗⠉   ⠁ ⠁⠈             │|       │                ⠈    ⠚⠍⠒⠝⠋⠑                     │|-21.12 │                                                │|       └──────────────────────────────────────────────────|        -45.66                                       1.626|5000 steps, one for every prime p up to 48611: at 30*(p mod 12) degrees|from the origin to (-27.21, -11.13), 29.4 away; x from -44.53 to 0.5, y from -18.63 to 2.866",
    ),
    (
        r"primemodwalk(1000, 10, width = 50, height = 8)",
        r" 44.07 │ │|       │ │|       │ │|       │ │  ⡀ ⢀   ⣀ ⢀⣀ ⡀⣀⣀⣀⣀⡀          ⣀⣠⣄ ⢀⣀⣄⣰⣦⣄⣤⣠⡾⢦⣄⣀|     0 │─⠙⠙⠉⠙⠛⠛⠉⠷⠚⠋⠛⠋⠛⠛⠙⠁⠈⠁⠉⠛⠓⠶⠶⠖⢶⠾⠶⢦⢾⠞⠁⠉⠈⠋⠉⠈─⠉──────⠉⠹⠖⠓─|       │ │|       │ │|-36.96 │ │|       └──────────────────────────────────────────────────|        -7.338                                       245.9|1000 steps, one for every prime p up to 7919: at 36*(p mod 10) degrees|from the origin to (239.9, -0.5878), 239.9 away; x from -1.309 to 239.9, y from -4.617 to 11.72",
    ),
    (
        r"primemodwalk(300, 4, width = 20, height = 8)",
        r" 0.325 │─────────⡖⠒─────────|       │         ⡇│|       │         ⡇│|       │         ⡇│|       │         ⡇│|       │         ⡇│|       │         ⡇│|-13.32 │         ⠇│|       └────────────────────|        -9.031         8.031|300 steps, one for every prime p up to 1987: at 90*(p mod 4) degrees|from the origin to (-1, -9), 9.055 away; x from -1 to 0, y from -13 to 0",
    ),
    (
        r"primemodwalk3d(5000, width = 50, height = 14)",
        r"                             ⢴⣆⣦⣄⣀ ⣠⣀⡀|                            ⡠⣰⣿⣿⣿⣿⣿⣿⣿⣾⠉|               ⢀⣠⣤        ⢀⣴⣿⣿⣷⣿⣿⣿⣿⣿⠿⠈|           ⠠⣶⣴⣦⣼⣿⣿⡆⢠⣀ ⣀⠰⣖⣤⣾⣿⣿⠿⣿⣿⣿⣿⣿⣿⣯⣤⣄|          ⢀ ⠘⣿⣿⣿⣿⣿⣿⣿⣿⣿⣟⣷⣿⣿⢿⣿⣾⣿⣿⣍⠋⣿⣿⢽⠽⡿⡿⡿⠗|          ⢸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡿⢿⣿⣡⣼⣉⣿⣿⣿⣿⣷⡆⠈|         ⠐⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠻⣿⣟⠻⢿⠝⠋|          ⠘⠟⣿⣿⣿⣙⣿⣽⣯⣼⣟⠟⠋⠙⠻⢾⡅⣄|            ⠘⠁⠈⠉⢉⣿⣿⣿⣷⣿⣿⠶⢺⣽⣿⣿⠷|              ⢄⣺⣿⣻⣿⣿⡿⡿⣷⣿⣼⣿⣿⣷⣖⠤|           ⡖⠺⢳⣿⣽⣏⡈⠁⢈⣀⣾⣿⣿⣿⣿⣿⠋⠁|          ⣀⣻⠇⢸⣟⣻⣽⣶⢀⣈⣿⣿⣿⣿⣿⣿⣿|          ⢹⣏⣾⣿⣿⠋⠾⡯⣿⣟⢻⣿⣿⣿⣿⡏⠁|           ⠁⠉    ⠹⡻⠗⠟⠉⠈⠈⠛⠁|5000 steps, one for every prime p up to 48611: at 45*(p mod 8) degrees around the z-axis, 30*(p mod 12) degrees above the plane|from the origin to (-2.449, -21.55, -11.13), 24.38 away; x from -25.72 to 3.062, y from -33.79 to 4.787, z from -18.63 to 2.866",
    ),
    (
        r"primemodwalk3d(2000, 5, 7, width = 50, height = 14)",
        r"||⢀⣀⡀⣀⣤⣀     ⢠⣀⡀|⠸⣿⣷⡼⢿⣿⣻⣤⢀ ⢠⣼⣿⣀⡀            ⡀⡀|   ⠁  ⠈⣿⡏⣸⣿⣿⣿⡿          ⢰⣆⣀⣿⣿⡇ ⡀ ⢀    ⣦⣾⡆|       ⠈⢻⣿⣯⠙⣿⣿⢀⢀⢀    ⣴⣶⣾⡿⣿⣿⡿⡝⠷⣷⣧⣴⣿⡀ ⢠ ⢻⣷⣿⣶|        ⠈⠋  ⠈⣿⣾⣿⣿⣿⢿⣴⣆⡿⡏⠁  ⠘  ⠸⢟⣿⣿⣿⣧⣸⠞⠛⠻⠿⣿⡏ ⢀⣠|             ⠘⠛⠁⠈⠚ ⠳⠙⠆        ⢸⡟⠿⠟⠃⠁    ⠻⢟⣶⣿⣽⣿⣤⣦|                              ⠈          ⠈⠛⠛ ⠁ ⡻|                                              ⣷⢥⣧⡀|                                              ⢺⣾⣿⣯|                                               ⠛⠿⠁|||2000 steps, one for every prime p up to 17389: at 72*(p mod 5) degrees around the z-axis, 51.43*(p mod 7) degrees above the plane|from the origin to (83.59, 8.782, -7.001), 84.34 away; x from -0.9054 to 84.57, y from -5.568 to 19.5, z from -15.48 to 5.469",
    ),
    (
        r"ulamspiral(2000, width = 50, height = 16)",
        r" 69.8 │          ⡄⡀⠠⢀⢄⣄⠠ ⡄⠄⢠ ⣀⠄⡠⡀⡤⡠⢀⡄⡠⡀⠄⠠⡄⢀⠄⡀⠠⢠|      │          ⢺⡙⡃⡎⡋⠷⠖⠅⢭⣰⠸⠻⢂⢏⡽⢏⣊⠅⡩⡣⡜⠡⣖⢪⠍⠩⠨⠩⡮⠩|      │          ⡳⢻⠶ ⣔⡠⡂⣵⡅⡑⠠⢯⣄⢨⣂⢂⠷⠯⢐⠖⠋⢢⡟⢅⡷⣂⠲⡇⢖⡊|      │          ⠤⠦⡱⠿⢁⠊⡰⢄⡮⢊⢴⠃⢦⢶⣥⣯⢣⠃⠣⢯⡗⣕⡧⡰⣴⣓⣄⢎⠣⣷|      │          ⡃⢄⢲⡪⢹⣜⢗⣢⣓⣪⢠⢒⣥⡎⢪⢀⡙⣤⢶⡠⣅⢪⠌⡦⣥⡨⠢⢓⢮⠨|      │          ⠞⠦⢂⠞⠖⣒⠒⡜⢖⡂⡱⣎⠪⡙⣜⣭⠺⡖⣎⢩⢤⠵⡹⢎⡈⠑⣬⠒⠡⠑|      │          ⠩⢏⢍⣇⡁⢐⡮⡩⠫⢁⢾⡨⢭⢨⠚⡷⣶⡏⢚⡍⣓⠚⠏⡢⠑⢈⠒⠓⣒⠠|      │          ⡔⢐⢢⡝ ⠲⡳⣁⣗⠪⡝⣗⣣⣳⣿⣮⠟⡻⡷⣛⠖⡛⡶⠔⠊⡠⢗⢧⠅⡡|    0 │──────────⠮⢁⡍⣃⠪⢄⣬⡭⠏⠜⢧⡊⣖⣼⢿⡯⡫⢎⠿⡼⠰⢎⢽⠡⠦⢲⠡⡿⠘⢼──────────|      │          ⡔⡌⠛⣕⣁⣫⣞⠬⡮⡧⣐⡝⣛⣼⠹⡵⣋⠥⣩⡏⢋⡲⣪⣘⢏⣡⠊⢇⡠⣩|      │          ⠣⡡⢑⢯⢐⢍⡐⢃⢭⡕⣈⠏⠨⢴⢡⢒⡺⢁⣬⢣⠋⢘ ⢡⣳⢈⡧⠃⣉⢛|      │          ⣍⠁⠨⡍⢍⠫⣡⡼⠩⡀⠾⡕⢡⢌⠚⡉⡿⠊⠓⠏⣬⡪⡓⡑⡴⣺⣑⣝⣁⢃|      │          ⠒⡐⠕⣌⠢⢶⡁⢛⢆⡖⢽⠎⢓⢗⡣⢚⠳⡑⡏⢢⡤⣪⣷⡂⠺⠇⠦⠋⡂⡞|      │          ⡁⢭⡂⠵⢙⠴⢎⡤⣚⡕⠤⡣⠼⠚⢭⡁⣣⠤⡤⡖⠳⢹⠳⢵⢕⠈⡥⡬⡨⣫|      │          ⡪⠔⡲⣉⢌⠳⡁⢄⡘⡂⣥⡞⠴⡨⠛⠼⣱⣊⡇⠄⡒⠑⡐⢡⡑⣨⢮⣨⡇⣹|-68.8 │          ⠃⠑⠂⠑ ⠂⠈⠁⠂⠊⠐⠂⠙⠘⠃⠙⠁⠂ ⠙⠘⠚ ⠉⠊⠁ ⠚⠈⠂|      └──────────────────────────────────────────────────|       -107.8                                       108.8|the first 2000 primes up to 17389 on the square spiral of the whole numbers: 1 in the middle, 2 to its right, then around against the clock",
    ),
    (
        r"ulamspiral(300, width = 40, height = 12)",
        r" 23.62 │          ⠠⠠⡀  ⡀  ⡀ │⢄  ⠄⢀ ⠄  ⢀|       │         ⠈⡠⠈ ⡢⡠ ⠂⠠⠊⠢⠄⠕   ⢁ ⠁⢐ ⠔⠄|       │         ⠠⡠  ⡈⠈⡀ ⠈⠠⠊⠁⢔⢐ ⠐⢄⠁⠄⠄⠁|       │        ⠠⡈⠊⠢⡈⠈⡠ ⠨⠊ ⡀│⢅⢀⠐⢅⢐⢀ ⠐⠐⠁|       │        ⠈⠈⡈⠈⡀⠂⡀⠈⡂⡀⠊⡢⠄⠄ ⢀⠅⢁⢄⠑⠄⠄⠑⠅|       │         ⠢ ⠢⠠⠠⠠⠢⠠⠢⠢⡈⢁⢕⠐⠑ ⠄⠐⢐⠐ ⠄⠄|     0 │─────────⠍⢀─⠄⢁⠄⢄⠈⢕⠍⠑⠪⡁⠡⠡⡁⡠⠁⡡⠡─⠠⡁────────|       │         ⠐⠔⢄  ⢄⠐⠔⢀⠁⢐⡂⡀⠢ ⠊ ⠠ ⠂⠢⠈|       │          ⢀⠐⢐⢔⠐⠄⢀⠁⠁⢐⡊⠢⡠⡈⠂⠈ ⠂⠊⡀⠊⠂|       │        ⠐⢄ ⠁⠁⢀⢀⠄⠑⢔ ⠐⠢⠊ ⠠ ⠈⠂⡀⠨⠢|       │        ⢀ ⠅ ⠅⠄ ⠄⠁⢀⠄ │⡀⠈⡀⡨⠂ ⠈⠂ ⠂|-23.62 │           ⠑     ⠁  │⠈       ⠈|       └────────────────────────────────────────|        -39.38                             39.38|the first 300 primes up to 1987 on the square spiral of the whole numbers: 1 in the middle, 2 to its right, then around against the clock",
    ),
    (
        r"ulamspiral(100, width = 30, height = 10, ascii)",
        r" 12.07 |          *   |     *  *|       |     **  *   *| ***  * *|       |        *   *** *    **|       |     * ** * *** *****  *|       |      * ** ***|***    *|     0 |---------*********-*-*--*-----|       |     * ** *** |** * ****|       |     ** ** *  * ***  *|       |      *   * * ***   ** *|-12.07 |       *      |  **|       +------------------------------|        -17.61                   18.61|the first 100 primes up to 541 on the square spiral of the whole numbers: 1 in the middle, 2 to its right, then around against the clock",
    ),
    (
        r"sacksspiral(2000, width = 50, height = 16)",
        r" 137.6 │                    ⣀  ⡤⣠⢤⣄⡄⡀⡀|       │                ⢀⡄⡄⣡⡶⡰⣍⣰⢭⣪⠦⡁⣸⠠⣱⡵⡤⠄|       │              ⣀⣮⡒⠏⠐⢬⢁⢬⠞⠽⣙⢩⠓⡭⡫⠵⠼⣹⢸⢞⢁⣄|       │             ⢜⢃⠅⣛⡻⠻⢯⡠⣥⣥⡃⣀⢴⠆⣁⣃⣈⡶⢯⣇⡘⣌⡂⢑|       │           ⢠⢬⡧⠜⠖⢌⢉⣲⣫⣛⣂⢫⣛⢻⣫⡕⣓⢟⠿⡤⠶⣮⠏⡒⣲⠣⢅⠇|       │          ⢀⡄⢦⢦⢬⠏⠯⠢⠮⣉⣞⣔⣱⠶⡦⠼⣍⣠⡼⢋⣏⡛⡖⢾⣭⡚⠫⡭⣯|       │          ⠦⠴⠶⠠⠢⠇⠽⠫⠮⠯⠿⠤⣵⣹⡼⣟⡞⣯⣛⢐⠷⢦⡇⢽⠸⡙⠧⡮⠯⢥|     0 │─────────⠠⠶⠾⠚⠯⠵⠟⠶⠷⠯⠯⠾⠿⠿⠽⣾⣷⣣⣖⠺⣖⠗⡢⢒⢛⢔⣘⠖⢾⣖⣗──────────|       │         ⠈⢂⣉⣋⢙⣊⣘⣉⡋⡙⣁⣙⢋⣛⡫⡻⣷⢖⡒⣒⣒⢒⡲⡶⣒⡤⢶⢔⢢⣲⠴⠂|       │          ⢉⣥⣉⣌⣉⣅⣜⡉⣈⣊⡣⣯⠭⠷⢾⡯⢫⢡⠇⣺⣊⡑⡇⡹⢝⠫⠂⢹⡓⡕|       │           ⣈⣅⣄⣀⣕⣁⣘⠦⡽⣤⡤⠺⡞⣛⢌⣣⣯⣲⢯⢔⢌⣰⣋⣟⡋⣙⣽⣥|       │            ⣁⢍⠘⠏⡝⣘⢧⡔⡒⡌⣻⣌⡃⣋⣝⡽⢣⡃⡅⠟⢺⢃⡳⢘⡴⡫⠅|       │            ⠈⠜⢩⡧⢮⡾⠦⣶⠚⠸⠽⢦⣻⠜⡫⣝⢓⢺⣩⣊⢉⠪⡎⠠⠞⠈|       │              ⠐⠂⠶⢕⣃⣳⢼⠟⢚⠙⠥⣍⠝⣡⢌⣒⡎⡰⠸⢀⣑⠇|       │                ⠈⠉⠪⣽⢒⡜⢡⠫⡳⡷⣫⣡⣣⢊⠖⠺⠃⠁|-138.8 │                    ⠈⠈⠐⠙⠋⠊⠃⠈ ⠁|       └──────────────────────────────────────────────────|        -216.3                                       215.6|the first 2000 primes up to 17389 on the spiral of Sacks: the whole number k is sqrt(k) away from the middle after sqrt(k) turns, so that the squares lie on the x-axis",
    ),
    (
        r"sacksspiral(300, width = 40, height = 12)",
        r" 45.93 │                 ⡀⠤⡐│⡠⡀⠄|       │             ⠐⠤⠁⡀⡀⠁⢀│⡠⡀⠊ ⡨⢔⡀|       │             ⠈⠍⡁ ⠂⠠⠢│⡂⢂⠂⠅⡈ ⠆⠄|       │          ⡁⢈⢂⠐⠐ ⠂⠅⠄⠈│⣂⢂⠄⢐⠐⠱⠲⠒|       │         ⠄⠄⠠ ⠄⠥⠈⢁⢁⢀⠪│⢂⠤⡄ ⡀⢂ ⠚⢂|     0 │────────⠐⠐⠂⠂⠒⠒⠐⠂⠒⠒⠢⢬⠤⠩⠠⠨⠄⠅⠤─⠈⠄⠅─────────|       │         ⠐⠂⠐ ⠒⠐⠐⠂⠂⡒⡊⢡⢤ ⢄⢀⡀⢀⡀⡀⣀ ⡀|       │         ⢀⢈⠈⢀⠈⡈⡨⠠⠤ ⠂⠰⠠⡌ ⡈⡀⠁⣀⢉⡀⣁⠁|       │         ⠠ ⠤ ⠔⠂⢂⠂⡀⠉⡂⠨⠊⠢⠰⠉⢀⢀⢠|       │           ⠂⢀⣀ ⠄⠐⡂⠨⠂⠨⡠⠜ ⡄ ⡀⡄|       │              ⢀  ⡀⡠⠄⠨⠠⢀⠄⠁⠄⠈|-46.31 │                ⠈ ⠈⠅│ ⠑⠁|       └────────────────────────────────────────|        -77.54                             76.19|the first 300 primes up to 1987 on the spiral of Sacks: the whole number k is sqrt(k) away from the middle after sqrt(k) turns, so that the squares lie on the x-axis",
    ),
    (
        r"primewalk(1)",
        r"error: the number of primes is a whole number from 2 to 5000000, not 1",
    ),
    (
        r"primewalk(10, 1)",
        r"error: the number of directions is a whole number from 2 to 1000000, not 1",
    ),
    (
        r"primewalk(10, 4, 4)",
        r"error: the turn, in directions, is a whole number from 1 to 3, not 4",
    ),
    (
        r"primewalk(6000000)",
        r"error: the number of primes is a whole number from 2 to 5000000, not 6000000",
    ),
    (
        r"primewalk3d(10, 7)",
        r"error: the number of directions is a whole number from 3 to 6, not 7",
    ),
    (
        r"primewalk3d(10, 5)",
        r"error: primewalk3d(n, m): a step for every whole number up to the n-th prime, at every prime the next of m directions of space: 3 (x, y, z), 4 (the corners of a tetrahedron) or 6 (x, y, z, -x, -y, -z); m is 6 when it is left out",
    ),
    (
        r"primemodwalk(0)",
        r"error: the number of primes is a whole number from 1 to 5000000, not 0",
    ),
    (
        r"primemodwalk(10, 1)",
        r"error: the modulus is a whole number from 2 to 4294967295, not 1",
    ),
    (
        r"primemodwalk3d(10, 8, 1)",
        r"error: the second modulus is a whole number from 2 to 4294967295, not 1",
    ),
    (
        r"ulamspiral(x)",
        r"error: the number of primes is a whole number from 1 to 5000000, not x",
    ),
    (
        r"ulamspiral()",
        r"error: ulamspiral takes at least 1 argument(s), 0 given",
    ),
    (
        r"sacksspiral(10, 2)",
        r"error: sacksspiral takes 1 argument at most; what else belongs to a picture is written as name = value",
    ),
    (
        r"primewalk(100, gray)",
        r#"error: gray is chosen for a picture that is saved: save_png("primewalk.png", primewalk(...), gray); the picture of characters takes width, height and ascii"#,
    ),
];

const CHARTS_OF_CHARACTERS: &[(&str, &str)] = &[
    (
        r"complexplot(z^2 - 1, z, width = 40, height = 10)",
        r" 2.1 │⣾⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⡶⡶⡶⡶⡶⡶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣾|     │⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶|     │⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶|     │⣶⣶⣶⣶⣶⣶⣶⣶⣶⡶⡶⡶⡶⡶⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⡶⡶⡶⡶⡶⣶⣶⣶⣶⣶⣶⣶⣶⣶|     │⣶⣶⣶⣶⣶⣶⣶⣶⣶⡶⡶⡶⠶⠶⠴⠴⠴⠶⠶⠶⠶⠶⠶⠴⠴⠴⠶⠶⡶⡶⡶⣶⣶⣶⣶⣶⣶⣶⣶⣶|     │⣶⣶⣶⣶⣶⣶⣶⣶⣶⡶⡶⡶⠶⠶⠴⠴⠴⠶⠶⠶⠶⠶⠶⠴⠴⠴⠶⠶⡶⡶⡶⣶⣶⣶⣶⣶⣶⣶⣶⣶|     │⣶⣶⣶⣶⣶⣶⣶⣶⣶⡶⡶⡶⡶⡶⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⠶⡶⡶⡶⡶⡶⣶⣶⣶⣶⣶⣶⣶⣶⣶|     │⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶|     │⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⡶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶|-2.1 │⣾⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⡶⡶⡶⡶⡶⡶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣾|     └────────────────────────────────────────|      -4.2                                 4.2|the colour is the argument of the value: red for the positive numbers, then yellow, green, cyan for the negative ones, blue, magenta; the lightness is its absolute value: black is 0, the full colour 1, white the infinitely large",
    ),
    (
        r"complexplot(1/z, z, 3, width = 30, height = 8)",
        r" 3.15 │⠤⠤⠤⠤⠤⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠤⠤⠤⠤⠤|      │⠤⠤⠤⠤⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠤⠤⠤⠤|      │⠤⠤⠤⠴⠴⠴⠴⠴⠴⠴⠴⠴⠶⠶⠶⠶⠶⠶⠴⠴⠴⠴⠴⠴⠴⠴⠴⠤⠤⠤|      │⠤⠤⠤⠴⠴⠴⠴⠴⠴⠴⠴⠶⠶⠶⡶⡶⠶⠶⠶⠴⠴⠴⠴⠴⠴⠴⠴⠤⠤⠤|      │⠤⠤⠤⠴⠴⠴⠴⠴⠴⠴⠴⠶⠶⠶⡶⡶⠶⠶⠶⠴⠴⠴⠴⠴⠴⠴⠴⠤⠤⠤|      │⠤⠤⠤⠴⠴⠴⠴⠴⠴⠴⠴⠴⠶⠶⠶⠶⠶⠶⠴⠴⠴⠴⠴⠴⠴⠴⠴⠤⠤⠤|      │⠤⠤⠤⠤⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠤⠤⠤⠤|-3.15 │⠤⠤⠤⠤⠤⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠴⠤⠤⠤⠤⠤|      └──────────────────────────────|       -5.906                   5.906|the colour is the argument of the value: red for the positive numbers, then yellow, green, cyan for the negative ones, blue, magenta; the lightness is its absolute value: black is 0, the full colour 1, white the infinitely large",
    ),
    (
        r"complexplot(sin(z), z, -pi - 2i, pi + 2i, width = 40, height = 8, ascii)",
        r" 2.1 |****************************************|     |++++++++++++++++++++++++++++++++++++++++|     |++++++====++++++++====++++++++====++++++|     |=++====--====++====--====++====--====++=|     |=++====--====++====--====++====--====++=|     |++++++====++++++++====++++++++====++++++|     |++++++++++++++++++++++++++++++++++++++++|-2.1 |****************************************|     +----------------------------------------|      -5.25                               5.25|the colour is the argument of the value: red for the positive numbers, then yellow, green, cyan for the negative ones, blue, magenta; the lightness is its absolute value: black is 0, the full colour 1, white the infinitely large",
    ),
    (
        r"densityplot(sin(x)*cos(y), x, -pi, pi, y, -pi, pi, width = 40, height = 8, ascii)",
        r" 3.142 |++*##%%%@@@@%%%##*++==-::...    ...::-==|       |++++************++++====------------====|       |====------------====++++************++++|       |==-::...    ...::-==++*##%%%@@@@%%%##*++|       |==-::...    ...::-==++*##%%%@@@@%%%##*++|       |====------------====++++************++++|       |++++************++++====------------====|-3.142 |++*##%%%@@@@%%%##*++==-::...    ...::-==|       +----------------------------------------|        -3.142                             3.142|the colours of the palette run from the value -1 to 1",
    ),
    (
        r"densityplot(x + y, x, 0, 1, y, 0, 1, width = 20, height = 5)",
        r"1 │⠶⠶⠶⠶⡶⡶⡶⡶⡶⣶⣶⣶⣶⣶⣾⣾⣾⣾⣾⣿|  │⠴⠴⠴⠶⠶⠶⠶⠶⡶⡶⡶⡶⡶⣶⣶⣶⣶⣶⣾⣾|  │⠤⠤⠴⠴⠴⠴⠴⠶⠶⠶⠶⠶⡶⡶⡶⡶⡶⣶⣶⣶|  │⠄⠤⠤⠤⠤⠤⠴⠴⠴⠴⠴⠶⠶⠶⠶⠶⡶⡶⡶⡶|0 │ ⠄⠄⠄⠄⠤⠤⠤⠤⠤⠴⠴⠴⠴⠴⠶⠶⠶⠶⠶|  └────────────────────|   0                  1|the colours of the palette run from the value 0 to 2",
    ),
    (
        r"plot3d(sin(x)*cos(y), x, -pi, pi, y, -pi, pi, width = 50, height = 14)",
        r"                       ⢀⡤⡲⠲⡀|                      ⣠⠟⢲⠓⢄⠞⢆|                    ⢀⡴⡻ ⡎ ⡎⢣⡈⣦|                   ⢀⡾⢋⠏⢹⠉⢲⢅ ⡽⣈⣆⡀|            ⢀⣀⣀ ⢀⡠⣖⣻⠁⡼ ⡎ ⡎⠈⣾⣾⣻⣏⣟⣶⢄|          ⢀⢔⠝⢫⣨⣿⣯⣢⢠⠃⢉⠏⢲⠓⢼⠤⣴⣿⣻⣷⢻⣿⡹⣟⢦⡀|         ⢀⡾⣛⣶⡚⣯⢏⣿⣳⣟⣄⡜ ⡸ ⣇⣼⣿⠾⡽⣉⡻⣻⣳⣿⣏⣿⣆  ⢀⡤⡞|        ⢀⡧⠟⠁⠉⠙⣾⡞⣾⣿⣻⣢⡓⢢⢧⣴⣿⠟⡇⢠⠃⢀⠗⢞⣿⣻⡞⡼⣎⣻⣗⣫⡗⠁|        ⠈     ⠈⠻⣝⣧⢻⣿⣹⣿⣻⣾⠯⢴⢅⣸⡀⡜ ⡸⠱⡻⣿⡿⠻⣀⢜⠝|                ⠘⢽⣟⡟⣷⣻⣿⡇ ⡎ ⡇⢨⠏⢱⣓⡦⠚⠉⠙⠚⠛⠁|                  ⠈⠛⡟⢫⠎⠘⢼⡀⢸ ⡜⣠⡟⠁|                    ⠸⡎⠳⡀⡇⠈⡏⢩⣫⠟|                     ⠘⣄⡝⢄⣸⣀⡼⠃|                      ⠈⢢⣠⡣⠞⠁|the surface z = cos(y)*sin(x) over 24 by 24 cells; z from -1 to 1",
    ),
    (
        r"plot3d(x^2 - y^2, x, -1, 1, y, -1, 1, width = 40, height = 10, ascii)",
        r"             *|           ******|          ********           **|         ***********       ******|        *** *********** **********|       *************************|        ****** ***************|         ****************|           *************|             *********|the surface z = x^2 - y^2 over 24 by 24 cells; z from -1 to 1",
    ),
    (
        r"parametricplot3d([cos(t), sin(t), t/10], t, 0, 40, width = 40, height = 12)",
        r"                   ⠒⠒⠢⠤⣄⡀|                 ⣀⣀⣀⣀   ⠈⢦|              ⢠⡊⠉⠁   ⠉⠉⢒⡤⡎|               ⣉⠷⠶⠒⠒⠲⠶⠮⣉ ⡹|              ⠘⠤⣀⣀⢀⡀⣀⣀⣠⠤⠋⢧|              ⢠⠖⠉⠉⠉⠉⠉⠉⠙⠒⢤⠎|               ⣑⠶⠦⠤⠤⠤⠶⣖⣉⠁⡱|              ⠘⢅⣀    ⢀⣀⡬⠝⢅|              ⢠⠖⠊⠉⠉⠉⠉⠉⠒⠢⢄⠜|              ⠈⢑⣢⠤⠤⠤⠤⢔⣒⡊⠁⢱|              ⠰⣅⡀     ⣀⡨⠕⢅|                ⠈⠉⠉⠉⠉⠉  ⢀⠜|the curve for t from 0 to 40; x from -1 to 1, y from -1 to 0.9999, z from 0 to 4",
    ),
    (
        r"graphplot(cyclegraph(5), width = 30, height = 10)",
        r"  1.212 │               ⡀|        │            ⢀⡠⠊⠛⢄⡀|        │          ⡠⠔⠁  │ ⠈⠢⢄|        │      ⢀⣀⠔⠉     │    ⠉⠢⣀⡀|        │      ⢹⠁       │      ⠈⡏|      0 │───────⢇───────┼──────⡸───────|        │       ⠘⡄      │     ⢠⠃|        │        ⢱      │     ⡎|        │         ⢇     │    ⡸|-0.9909 │         ⠙⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠋|        └──────────────────────────────|         -1.653                   1.653|5 vertices on a circle, the first at the top and the others with the clock; 5 edges",
    ),
    (
        r"graphplot(petersengraph())",
        r" 1.218 │                                │|       │                               ⢀⣄|       │                                ⣏⠉⠒⠢⠤⢄⣀|       │                     ⠐⡷⡀        ⡟⡄     ⠉⢑⠾⡂|       │                     ⡸⡇⠘⢄       ⡇⢱     ⡠⠃ ⠘⢄|       │                    ⢠⠃⡇ ⠈⠢⡀     ⡇ ⢇  ⢀⠔⠁    ⠣⡀|       │                    ⡜ ⡇   ⠑⡄    ⡇ ⠘⡄⢠⠊       ⠑⡄|       │                ⢺⡦⢄⣰⠁ ⡇    ⠈⢢   ⡇  ⣵⠁       ⣀⡠⠼⡗|       │                ⠈⡞⢄⠇⠉⠑⡗⠤⢄⣀   ⠑⡄ ⡇⢠⠊ ⢇ ⣀⡠⠤⠒⠊⠉   ⡇|       │                 ⠸⡜⠣⡀ ⡇   ⠉⠑⠒⠤⢌⣆⣷⡡⠤⠒⠚⡍         ⡇|     0 │─────────────────⢰⢣─⠑⢄⡇───⣀⡠⠤⠒⢊⠏⡿⡑⠒⠤⢄⣱─────────⡇────────────────|       │                ⢀⠇⠈⡆⣀⡨⣧⠒⠊⠉   ⡠⠃ ⡇⠘⢄   ⡏⠑⠒⠤⢄⣀   ⡇|       │                ⢼⢖⠊⠹⡀ ⡇⠑⡄  ⢀⠜   ⡇  ⠣⡀ ⠸⡀    ⠉⠑⢲⡧|       │                  ⠉⠢⣣ ⡇ ⠈⢢⡠⠃    ⡇   ⠘⢄ ⢣     ⡠⠃|       │                    ⠘⡕⡧⡀⢀⠔⠱⡀    ⡇    ⠈⠢⡈⡆   ⡔⠁|       │                     ⢱⡇⢨⠛⢄⡀⠈⢆   ⡇      ⠑⡼⡀⢠⠊|       │                     ⠠⡷⠁  ⠈⠒⢄⠣⡀ ⡇       ⠈⢷⠅|       │                             ⠉⠺⢄⡇|       │                               ⠈⠋|-1.218 │                                │|       └────────────────────────────────────────────────────────────────|        -1.949                                                     1.949|10 vertices on a circle, the first at the top and the others with the clock; 15 edges",
    ),
    (
        r"graphplot([1 -> 2, 2 -> 3, 3 -> 1], width = 30, height = 8, ascii)",
        r"  1.203 |               ||        |              **|        |             * |*|        |            *  | *|        |          **   |  **|      0 |---------*-----+----*---------|        |        *      |     **|-0.6235 |       ****************|        +------------------------------|         -1.713                   1.713|3 vertices on a circle, the first at the top and the others with the clock; 3 edges",
    ),
    (
        r"matrixplot([[1, 2, 3], [4, 5, 6]], width = 12, height = 4)",
        r" 2.05 │    ⠤⠤⠤⠤⠴⠴⠴⠴|      │    ⠤⠤⠤⠤⠴⠴⠴⠴|      │⡶⡶⡶⡶⣶⣶⣶⣶⣿⣿⣿⣿|-0.05 │⡶⡶⡶⡶⣶⣶⣶⣶⣿⣿⣿⣿|      └────────────|       -0.075 3.075|2 by 3 entries, the first row at the top; the colours of the palette run from the entry 1 to 6",
    ),
    (
        r"matrixplot([[1, 2, 3], [4, 5, 6]], width = 12, height = 4, ascii)",
        r" 2.05 |    ::::====|      |    ::::====|      |++++####@@@@|-0.05 |++++####@@@@|      +------------|       -0.075 3.075|2 by 3 entries, the first row at the top; the colours of the palette run from the entry 1 to 6",
    ),
    (
        r"matrixplot(identity(4), width = 16, height = 8, ascii)",
        r" 4.1 |@@@@|     |@@@@|     |    @@@@|     |    @@@@|     |        @@@@|     |        @@@@|     |            @@@@|-0.1 |            @@@@|     +----------------|      -0.1         4.1|4 by 4 entries, the first row at the top; the colours of the palette run from the entry 0 to 1",
    ),
    (
        r"complexplot(z, z, 0)",
        r"error: complexplot(f, z, a, b) with the corners a and b of a box of the complex plane: complexplot(z^2 - 1, z, -2 - 2i, 2 + 2i); complexplot(f, z, r) is from -r - r*i to r + r*i, complexplot(f, z) from -2 - 2i to 2 + 2i",
    ),
    (
        r"complexplot(z, z, 1, 1 + i)",
        r"error: the corners of the box differ in their real and in their imaginary part: complexplot(f, z, a, b) with the corners a and b of a box of the complex plane: complexplot(z^2 - 1, z, -2 - 2i, 2 + 2i); complexplot(f, z, r) is from -r - r*i to r + r*i, complexplot(f, z) from -2 - 2i to 2 + 2i",
    ),
    (
        r"complexplot(z)",
        r"error: complexplot takes at least 2 argument(s), 1 given",
    ),
    (
        r"densityplot(x, x, 0, 1)",
        r"error: densityplot takes at least 7 argument(s), 4 given",
    ),
    (
        r"densityplot(ln(-1 - x^2 - y^2), x, 0, 1, y, 0, 1)",
        r"error: ln(-x^2 - y^2 - 1) has no real value in these ranges",
    ),
    (
        r"plot3d(x, x, 0, 1, y, 1, 0)",
        r"error: from 1 to 0 is no range of real numbers",
    ),
    (
        r"parametricplot3d([t, t], t, 0, 1)",
        r"error: parametricplot3d([x(t), y(t), z(t)], t, a, b): parametricplot3d([cos(t), sin(t), t/10], t, 0, 20)",
    ),
    (
        r"graphplot(5)",
        r"error: a graph is a list of edges: [[1, 2], [2, 3]], [1 -> 2, 2 -> 3], with weights [[1, 2, 5], [2 -> 3, 4]], a vertex alone as [4]",
    ),
    (
        r"matrixplot([[1, 2], [3]])",
        r"error: the rows of a matrix are equally long: matrixplot(m): matrixplot([[1, 2], [3, 4]])",
    ),
    (r"matrixplot([[1, i]])", r"error: i is no real number"),
    (
        r"complexplot(z, z, gray)",
        r#"error: gray is chosen for a picture that is saved: save_png("complexplot.png", complexplot(...), gray); the picture of characters takes width, height and ascii"#,
    ),
];

const PICTURES_THAT_ARE_SAVED: &[(&str, &str)] = &[
    (
        r#"save_png("walk.png", walk(pi, 10, 10000))"#,
        r"10000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (-65.23, 76.9), 100.8 away; x from -83.73 to 34.73, y from -51.9 to 85.85|saved walk.png: PNG, 1024 x 1024 pixels, 8 bits, RGB, level 9, 256497 bytes",
    ),
    (
        r#"save_png("walk.png", walk(pi), width = 800, height = 600, bits = 16, level = 9)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved walk.png: PNG, 800 x 600 pixels, 16 bits, RGB, level 9, 196509 bytes",
    ),
    (
        r#"save_png("small.png", walk(pi, 10, 100), width = 64)"#,
        r"100 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (2.854, 1.453), 3.203 away; x from -4.618 to 3.972, y from -1.261 to 6.07|saved small.png: PNG, 64 x 64 pixels, 8 bits, RGB, level 9, 2844 bytes",
    ),
    (
        r#"save_png("gray.png", walk(pi), width = 300, gray)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved gray.png: PNG, 300 x 300 pixels, 8 bits, gray, level 9, 18032 bytes",
    ),
    (
        r#"save_png("clear.png", walk(pi), width = 300, alpha)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved clear.png: PNG, 300 x 300 pixels, 8 bits, RGB + alpha, level 9, 41054 bytes",
    ),
    (
        r#"save_png("four.png", walk(pi), width = 300, channels = 4)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved four.png: PNG, 300 x 300 pixels, 8 bits, RGB + alpha, level 9, 41054 bytes",
    ),
    (
        r#"save_png("two.png", walk(pi), width = 300, channels = 2, bits = 16)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved two.png: PNG, 300 x 300 pixels, 16 bits, gray + alpha, level 9, 53429 bytes",
    ),
    (
        r#"save_png("one.png", walk(pi), width = 300, channels = 1)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved one.png: PNG, 300 x 300 pixels, 8 bits, gray, level 9, 18032 bytes",
    ),
    (
        r#"save_png("level0.png", walk(pi), width = 300, level = 0)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved level0.png: PNG, 300 x 300 pixels, 8 bits, RGB, level 0, 270401 bytes",
    ),
    (
        r#"save_png("level1.png", walk(pi), width = 300, level = 1)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved level1.png: PNG, 300 x 300 pixels, 8 bits, RGB, level 1, 42712 bytes",
    ),
    (
        r#"save_png("level5.png", walk(pi), width = 300, level = 5)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved level5.png: PNG, 300 x 300 pixels, 8 bits, RGB, level 5, 39318 bytes",
    ),
    (
        r#"save_jxl("walk.jxl", walk(pi, 10, 10000))"#,
        r"10000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (-65.23, 76.9), 100.8 away; x from -83.73 to 34.73, y from -51.9 to 85.85|saved walk.jxl: JPEG XL, 1024 x 1024 pixels, 8 bits, RGB, lossless, effort 7, N bytes",
    ),
    (
        r#"save_jxl("walk.jxl", walk(pi), width = 300, lossless, bits = 12, gray)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved walk.jxl: JPEG XL, 300 x 300 pixels, 12 bits, gray, lossless, effort 7, N bytes",
    ),
    (
        r#"save_jxl("one.jxl", walk(pi), width = 300, bits = 1, gray)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved one.jxl: JPEG XL, 300 x 300 pixels, 1 bit, gray, lossless, effort 7, N bytes",
    ),
    (
        r#"save_jxl("lossy.jxl", walk(pi), width = 300, quality = 80)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved lossy.jxl: JPEG XL, 300 x 300 pixels, 8 bits, RGB, lossy at the distance 1.9, effort 7, N bytes",
    ),
    (
        r#"save_jxl("lossy.jxl", walk(pi), width = 300, distance = 2, effort = 3)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved lossy.jxl: JPEG XL, 300 x 300 pixels, 8 bits, RGB, lossy at the distance 2, effort 3, N bytes",
    ),
    (
        r#"save_jxl("hundred.jxl", walk(pi), width = 300, quality = 100)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved hundred.jxl: JPEG XL, 300 x 300 pixels, 8 bits, RGB, lossless, effort 7, N bytes",
    ),
    (
        r#"save_jxl("alpha.jxl", walk(pi), width = 300, alpha, bits = 16)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved alpha.jxl: JPEG XL, 300 x 300 pixels, 16 bits, RGB + alpha, lossless, effort 7, N bytes",
    ),
    (
        r#"save("walk.jxl", walk(pi), width = 300)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved walk.jxl: JPEG XL, 300 x 300 pixels, 8 bits, RGB, lossless, effort 7, N bytes",
    ),
    (
        r#"save("walk.png", walk(pi), width = 300)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved walk.png: PNG, 300 x 300 pixels, 8 bits, RGB, level 9, 39605 bytes",
    ),
    (
        r#"save("WALK.PNG", walk(pi), width = 300)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved WALK.PNG: PNG, 300 x 300 pixels, 8 bits, RGB, level 9, 39605 bytes",
    ),
    (
        r#"save_png("fire.png", walk(pi), width = 300, palette = fire, background = white, thickness = 2, margin = 0.1)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved fire.png: PNG, 300 x 300 pixels, 8 bits, RGB, level 9, 34488 bytes",
    ),
    (
        r##"save_png("own.png", walk(pi), width = 300, palette = [blue, "#ff8000", [1, 1, 0]], reverse)"##,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved own.png: PNG, 300 x 300 pixels, 8 bits, RGB, level 9, 41373 bytes",
    ),
    (
        r#"save_png("gold.png", walk(pi), width = 300, color = gold)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved gold.png: PNG, 300 x 300 pixels, 8 bits, RGB, level 9, 31061 bytes",
    ),
    (
        r#"save_png("grey.png", walk(pi), width = 300, grey, colour = white, centre = [0, 0])"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved grey.png: PNG, 300 x 300 pixels, 8 bits, gray, level 9, 5411 bytes",
    ),
    (
        r#"save_png("sea.png", walk(pi), width = 300, palette = "ocean", background = "101010")"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved sea.png: PNG, 300 x 300 pixels, 8 bits, RGB, level 9, 41699 bytes",
    ),
    (
        r#"save_png("axes.png", walk(pi), width = 300, axes, grid)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved axes.png: PNG, 300 x 300 pixels, 8 bits, RGB, level 9, 28965 bytes",
    ),
    (
        r#"save_png("eye.png", walk3d(e + pi*i, 12, 5000), width = 300, azimuth = 60, elevation = 20, roll = 10)"#,
        r"5000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (14.58, 5.387, -8.088), 17.52 away; x from -17.31 to 26.14, y from -13.22 to 17.05, z from -14.25 to 32.53|saved eye.png: PNG, 300 x 300 pixels, 8 bits, RGB, level 9, 51834 bytes",
    ),
    (
        r#"save_png("near.png", walk3d(e + pi*i, 12, 5000), width = 300, viewpoint = [100, -80, 60])"#,
        r"5000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (14.58, 5.387, -8.088), 17.52 away; x from -17.31 to 26.14, y from -13.22 to 17.05, z from -14.25 to 32.53|saved near.png: PNG, 300 x 300 pixels, 8 bits, RGB, level 9, 53784 bytes",
    ),
    (
        r#"save_png("deep.png", walk3d(e + pi*i, 12, 5000), width = 300, perspective = 3, fog = 0.8, zoom = 1.2)"#,
        r"5000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (14.58, 5.387, -8.088), 17.52 away; x from -17.31 to 26.14, y from -13.22 to 17.05, z from -14.25 to 32.53|saved deep.png: PNG, 300 x 300 pixels, 8 bits, RGB, level 9, 71124 bytes",
    ),
    (
        r#"save_png("part.png", walk3d(e + pi*i, 12, 5000), width = 300, center = [0, 0, 0], scale = 8)"#,
        r"5000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (14.58, 5.387, -8.088), 17.52 away; x from -17.31 to 26.14, y from -13.22 to 17.05, z from -14.25 to 32.53|saved part.png: PNG, 300 x 300 pixels, 8 bits, RGB, level 9, 59874 bytes",
    ),
    (
        r#"save_png("box.png", walk3d(e + pi*i, 12, 5000), width = 300, axes)"#,
        r"5000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (14.58, 5.387, -8.088), 17.52 away; x from -17.31 to 26.14, y from -13.22 to 17.05, z from -14.25 to 32.53|saved box.png: PNG, 300 x 300 pixels, 8 bits, RGB, level 9, 32313 bytes",
    ),
    (
        r#"save_png("turn.png", walk3d(e + pi*i, 12, 2000), frames = 12, fps = 6, width = 200)"#,
        r"2000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (-13.58, 8.12, 3.562), 16.22 away; x from -15.3 to 4.397, y from -7.175 to 16.44, z from -8.732 to 32.53|saved turn.png: PNG, 200 x 200 pixels, 8 bits, RGB, 12 frames, 6 a second, level 9, 219566 bytes",
    ),
    (
        r#"save_jxl("turn.jxl", walk3d(e + pi*i, 12, 2000), frames = 12, turn = 90, width = 200)"#,
        r"2000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (-13.58, 8.12, 3.562), 16.22 away; x from -15.3 to 4.397, y from -7.175 to 16.44, z from -8.732 to 32.53|saved turn.jxl: JPEG XL, 200 x 200 pixels, 8 bits, RGB, 12 frames, 12 a second, lossless, effort 7, N bytes",
    ),
    (
        r#"save_jxl("grow.jxl", walk(pi), frames = 10, grow, width = 200)"#,
        r"1000 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (25.36, 15.39), 29.66 away; x from -4.646 to 25.36, y from -1.261 to 27.53|saved grow.jxl: JPEG XL, 200 x 200 pixels, 8 bits, RGB, 10 frames, 12 a second, lossless, effort 7, N bytes",
    ),
    (
        r#"save_png("grow.png", walk3d(e + pi*i, 12, 500), frames = 4, grow, turn = 180, width = 200)"#,
        r"500 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (-8.549, 12.11, 14.77), 20.93 away; x from -14.26 to 4.397, y from -1.183 to 16.44, z from -2.928 to 20.6|saved grow.png: PNG, 200 x 200 pixels, 8 bits, RGB, 4 frames, 12 a second, level 9, 23490 bytes",
    ),
    (
        r#"save_jxl("slices.jxl", walk3d(e + pi*i, 12, 2000), slices = 16, width = 200)"#,
        r"2000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (-13.58, 8.12, 3.562), 16.22 away; x from -15.3 to 4.397, y from -7.175 to 16.44, z from -8.732 to 32.53|saved slices.jxl: JPEG XL, 200 x 200 pixels, 8 bits, RGB, 16 slices as frames, 12 a second, lossless, effort 7, N bytes",
    ),
    (
        r#"save_png("slices.png", walk3d(e + pi*i, 12, 2000), slices = 4, width = 200)"#,
        r"2000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (-13.58, 8.12, 3.562), 16.22 away; x from -15.3 to 4.397, y from -7.175 to 16.44, z from -8.732 to 32.53|saved slices.png: PNG, 200 x 200 pixels, 8 bits, RGB, 4 slices as frames, 12 a second, level 9, 31578 bytes",
    ),
    (
        r#"save_jxl("depth.jxl", walk3d(e + pi*i, 12, 2000), depth, width = 200)"#,
        r"2000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (-13.58, 8.12, 3.562), 16.22 away; x from -15.3 to 4.397, y from -7.175 to 16.44, z from -8.732 to 32.53|saved depth.jxl: JPEG XL, 200 x 200 pixels, 8 bits, RGB + depth, lossless, effort 7, N bytes",
    ),
    (
        r#"save_jxl("all.jxl", walk3d(e + pi*i, 12, 2000), depth, alpha, gray, bits = 10, frames = 3, width = 200)"#,
        r"2000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (-13.58, 8.12, 3.562), 16.22 away; x from -15.3 to 4.397, y from -7.175 to 16.44, z from -8.732 to 32.53|saved all.jxl: JPEG XL, 200 x 200 pixels, 10 bits, gray + alpha + depth, 3 frames, 12 a second, lossless, effort 7, N bytes",
    ),
    (
        r#"save_png("views.png", walk3d(e + pi*i, 12, 2000), views, width = 200)"#,
        r"2000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (-13.58, 8.12, 3.562), 16.22 away; x from -15.3 to 4.397, y from -7.175 to 16.44, z from -8.732 to 32.53|saved views.png: PNG, 200 x 200 pixels, 8 bits, RGB, 3 views as frames (from above, from the front, from the right), 12 a second, level 9, 50137 bytes",
    ),
    (
        r#"save_jxl("views.jxl", primewalk3d(2000), views, width = 200, axes)"#,
        r"1999 sides along the first 2000 primes, from 2 to 17389: a step for every whole number, at every prime the next of the directions x, y, z, -x, -y, -z, so that every side is as long as the gap between two primes|from the origin to (295, -94, -158), 347.6 away; x from -21 to 331, y from -98 to 104, z from -174 to 116|saved views.jxl: JPEG XL, 200 x 200 pixels, 8 bits, RGB, 3 views as frames (from above, from the front, from the right), 12 a second, lossless, effort 7, N bytes",
    ),
    (
        r#"save_png("eyes.png", walk3d(e + pi*i, 12, 2000), stereo, width = 200)"#,
        r"2000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (-13.58, 8.12, 3.562), 16.22 away; x from -15.3 to 4.397, y from -7.175 to 16.44, z from -8.732 to 32.53|saved eyes.png: PNG, 200 x 200 pixels, 8 bits, red for the left eye and cyan for the right one, 4 degrees apart, level 9, 24717 bytes",
    ),
    (
        r#"save_png("eyes6.png", plot3d(x*y, x, -1, 1, y, -1, 1), stereo = 6, width = 200)"#,
        r"the surface z = x*y over 25 by 25 cells; z from -1 to 1|saved eyes6.png: PNG, 200 x 200 pixels, 8 bits, red for the left eye and cyan for the right one, 6 degrees apart, level 9, 17629 bytes",
    ),
    (
        r#"save_jxl("eyes.jxl", walk3d(e + pi*i, 12, 2000), stereo, views, width = 200, background = white)"#,
        r"2000 steps along the digits 2.8752360698219ba71971009... of the real part and 3.184809493b918664573a621... of the imaginary part of e + i*pi in base 12: the digits a and c of a place are a step at 30*a degrees around the z-axis, 30*c degrees above the plane|from the origin to (-13.58, 8.12, 3.562), 16.22 away; x from -15.3 to 4.397, y from -7.175 to 16.44, z from -8.732 to 32.53|saved eyes.jxl: JPEG XL, 200 x 200 pixels, 8 bits, red for the left eye and cyan for the right one, 4 degrees apart, 3 views as frames (from above, from the front, from the right), 12 a second, lossless, effort 7, N bytes",
    ),
    (
        r#"save_png("primes.png", primewalk(20000), width = 400)"#,
        r"19999 sides along the first 20000 primes, from 2 to 224737: a step for every whole number, at every prime a turn to the left by 90 degrees, so that every side is as long as the gap between two primes|from the origin to (975, -304), 1021 away; x from -77 to 1277, y from -554 to 534|saved primes.png: PNG, 400 x 400 pixels, 8 bits, RGB, level 9, 71654 bytes",
    ),
    (
        r#"save_png("space.png", primewalk3d(20000), width = 400, axes)"#,
        r"19999 sides along the first 20000 primes, from 2 to 224737: a step for every whole number, at every prime the next of the directions x, y, z, -x, -y, -z, so that every side is as long as the gap between two primes|from the origin to (1551, -496, 408), 1679 away; x from -487 to 1887, y from -528 to 584, z from -528 to 536|saved space.png: PNG, 400 x 400 pixels, 8 bits, RGB, level 9, 32770 bytes",
    ),
    (
        r#"save_png("mod.png", primemodwalk(50000, 12), width = 400)"#,
        r"50000 steps, one for every prime p up to 611953: at 30*(p mod 12) degrees|from the origin to (-84.37, -12.13), 85.24 away; x from -97.36 to 10.89, y from -50.13 to 2.866|saved mod.png: PNG, 400 x 400 pixels, 8 bits, RGB, level 9, 96176 bytes",
    ),
    (
        r#"save_jxl("mod3d.jxl", primemodwalk3d(20000), frames = 8, width = 200)"#,
        r"20000 steps, one for every prime p up to 224737: at 45*(p mod 8) degrees around the z-axis, 30*(p mod 12) degrees above the plane|from the origin to (-51.44, -10.52, -18.13), 55.55 away; x from -65.52 to 8.573, y from -36.24 to 23.16, z from -34.13 to 2.866|saved mod3d.jxl: JPEG XL, 200 x 200 pixels, 8 bits, RGB, 8 frames, 12 a second, lossless, effort 7, N bytes",
    ),
    (
        r#"save_png("ulam.png", ulamspiral(50000), width = 400)"#,
        r"the first 50000 primes up to 611953 on the square spiral of the whole numbers: 1 in the middle, 2 to its right, then around against the clock|saved ulam.png: PNG, 400 x 400 pixels, 8 bits, RGB, level 9, 286919 bytes",
    ),
    (
        r#"save_jxl("sacks.jxl", sacksspiral(20000), width = 400)"#,
        r"the first 20000 primes up to 224737 on the spiral of Sacks: the whole number k is sqrt(k) away from the middle after sqrt(k) turns, so that the squares lie on the x-axis|saved sacks.jxl: JPEG XL, 400 x 400 pixels, 8 bits, RGB, lossless, effort 7, N bytes",
    ),
    (
        r#"save_png("plot.png", plot([sin(x), cos(x)], x, 0, 2pi, [-2, 2]), grid)"#,
        r"saved plot.png: PNG, 1024 x 768 pixels, 8 bits, RGB, level 9, 31865 bytes",
    ),
    (
        r#"save_png("tan.png", plot(tan(x), x, -3, 3, [-5, 5]), width = 400)"#,
        r"saved tan.png: PNG, 400 x 300 pixels, 8 bits, RGB, level 9, 6922 bytes",
    ),
    (
        r#"save_png("bare.png", plot(sin(x), x, 0, 2pi), width = 400, noaxes)"#,
        r"saved bare.png: PNG, 400 x 300 pixels, 8 bits, RGB, level 9, 6949 bytes",
    ),
    (
        r#"save_png("dark.png", plot(sin(x), x, 0, 2pi), width = 400, background = black, color = yellow)"#,
        r"saved dark.png: PNG, 400 x 300 pixels, 8 bits, RGB, level 9, 6085 bytes",
    ),
    (
        r#"save_png("curve.png", parametricplot([sin(2t), sin(3t)], t, 0, 2pi), width = 400)"#,
        r"saved curve.png: PNG, 400 x 400 pixels, 8 bits, RGB, level 9, 18873 bytes",
    ),
    (
        r#"save_png("polar.png", polarplot(cos(3t), t, 0, pi), width = 400)"#,
        r"saved polar.png: PNG, 400 x 451 pixels, 8 bits, RGB, level 9, 13296 bytes",
    ),
    (
        r#"save_png("circle.png", implicitplot(x^2 + y^2 = 1, x, -1.5, 1.5, y, -1.5, 1.5), width = 400)"#,
        r"saved circle.png: PNG, 400 x 400 pixels, 8 bits, RGB, level 9, 7567 bytes",
    ),
    (
        r#"save_png("points.png", listplot([1, 4, 9, 16, 25]), width = 400)"#,
        r"saved points.png: PNG, 400 x 300 pixels, 8 bits, RGB, level 9, 2131 bytes",
    ),
    (
        r#"save_png("bars.png", histogram([1, 2, 2, 3, 3, 3, 4, 4, 5]), width = 400)"#,
        r"9 values in 5 intervals of the width 0.8 from 1: the bars are as high as the number of values in them, 3 at most|saved bars.png: PNG, 400 x 300 pixels, 8 bits, RGB, level 9, 1654 bytes",
    ),
    (
        r#"save_png("complex.png", complexplot((z^2 - 1)/(z^2 + 1), z), width = 200)"#,
        r"the colour is the argument of the value: red for the positive numbers, then yellow, green, cyan for the negative ones, blue, magenta; the lightness is its absolute value: black is 0, the full colour 1, white the infinitely large|saved complex.png: PNG, 200 x 200 pixels, 8 bits, RGB, level 9, 16591 bytes",
    ),
    (
        r#"save_png("heat.png", densityplot(x*y, x, -1, 1, y, -1, 1), width = 200, palette = heat)"#,
        r"the colours of the palette run from the value -1 to 1|saved heat.png: PNG, 200 x 150 pixels, 8 bits, RGB, level 9, 4966 bytes",
    ),
    (
        r#"save_png("saddle.png", plot3d(x^2 - y^2, x, -1, 1, y, -1, 1), width = 400, azimuth = 45, palette = plasma)"#,
        r"the surface z = x^2 - y^2 over 50 by 50 cells; z from -1 to 1|saved saddle.png: PNG, 400 x 400 pixels, 8 bits, RGB, level 9, 64856 bytes",
    ),
    (
        r#"save_jxl("helix.jxl", parametricplot3d([cos(t), sin(t), t/9], t, 0, 20), frames = 8, width = 200)"#,
        r"the curve for t from 0 to 20; x from -1 to 1, y from -1 to 1, z from 0 to 2.222|saved helix.jxl: JPEG XL, 200 x 200 pixels, 8 bits, RGB, 8 frames, 12 a second, lossless, effort 7, N bytes",
    ),
    (
        r#"save_png("graph.png", graphplot(petersengraph()), width = 400)"#,
        r"10 vertices on a circle, the first at the top and the others with the clock; 15 edges|saved graph.png: PNG, 400 x 400 pixels, 8 bits, RGB, level 9, 23321 bytes",
    ),
    (
        r#"save_png("matrix.png", matrixplot([[1, 2, 3], [4, 5, 6]]), width = 300, palette = ice)"#,
        r"2 by 3 entries, the first row at the top; the colours of the palette run from the entry 1 to 6|saved matrix.png: PNG, 300 x 200 pixels, 8 bits, RGB, level 9, 721 bytes",
    ),
    (
        r#"n := 500; save_png("named.png", walk(pi, 10, n), width = n)"#,
        r"500|500 steps along the digits 3.14159265358979323846264... of pi in base 10: the digit d is a step at 36*d degrees|from the origin to (8.961, 21.32), 23.13 away; x from -4.646 to 11.49, y from -1.261 to 24.17|saved named.png: PNG, 500 x 500 pixels, 8 bits, RGB, level 9, 48938 bytes",
    ),
];

const PICTURES_THAT_ARE_NOT_SAVED: &[(&str, &str)] = &[
    (
        r#"save("a9.gif", plot(sin(x), x, 0, 2pi))"#,
        r#"error: save tells the kind of file by the end of its name, .png or .jxl, which "a9.gif" lacks: save_png and save_jxl take any name"#,
    ),
    (
        r#"save_png("b1.png", walk(pi), bits = 12)"#,
        r"error: a PNG file has 8 or 16 bits, not 12: save_jxl takes 1 to 16",
    ),
    (
        r#"save_png("b2.png", walk(pi), quality = 50)"#,
        r"error: quality belongs to save_jxl: a PNG file is lossless, and its level from 0 to 9 says how hard it is packed",
    ),
    (
        r#"save_png("b2.png", walk(pi), depth)"#,
        r"error: depth belongs to save_jxl: a PNG file is lossless, and its level from 0 to 9 says how hard it is packed",
    ),
    (
        r#"save_jxl("b3.jxl", walk(pi), level = 5)"#,
        r"error: level belongs to save_png: save_jxl takes effort from 1 to 10, and quality or distance for a lossy picture",
    ),
    (
        r#"save_jxl("b4.jxl", walk(pi), quality = 50, distance = 1)"#,
        r"error: quality or distance, not both: quality = 90 is distance = 1",
    ),
    (
        r#"save_jxl("b5.jxl", walk(pi), lossless, quality = 50)"#,
        r"error: lossless, or lossy with quality or distance: not both",
    ),
    (
        r#"save_png("b6.png", walk(pi), width = 5000, height = 5000)"#,
        r"error: a picture has 16777216 pixels at most (4096 by 4096), not 5000 by 5000",
    ),
    (
        r#"save_png("b6.png", walk(pi), width = 20000)"#,
        r"error: width is a whole number from 1 to 16384, not 20000",
    ),
    (
        r#"save_png("b7.png", walk(pi), grow)"#,
        r"error: grow needs frames: frames = n",
    ),
    (
        r#"save_png("b7.png", walk(pi), turn = 90)"#,
        r"error: turn needs frames: frames = n",
    ),
    (
        r#"save_png("b8.png", walk(pi), slices = 3, frames = 2)"#,
        r"error: slices are the frames of the picture: slices = n or frames = n, not both",
    ),
    (
        r#"save_png("b8.png", walk(pi), views)"#,
        r"error: views and stereo show a drawing of space from several points: walk3d, primewalk3d, primemodwalk3d, plot3d, parametricplot3d",
    ),
    (
        r#"save_png("b8.png", walk(pi), stereo)"#,
        r"error: views and stereo show a drawing of space from several points: walk3d, primewalk3d, primemodwalk3d, plot3d, parametricplot3d",
    ),
    (
        r#"save_png("b8.png", plot(sin(x), x, 0, 1), stereo)"#,
        r"error: azimuth, elevation, roll, viewpoint, perspective, center, scale, zoom, fog, frames, turn, grow, slices, views and stereo belong to a drawing of a plane or of space (walk, walk3d, plot3d ...), not to a chart",
    ),
    (
        r#"save_png("b8.png", walk3d(pi + i), views, frames = 3)"#,
        r"error: views are three frames of their own: not with frames, slices, turn or grow",
    ),
    (
        r#"save_png("b8.png", walk3d(pi + i), views, turn = 30)"#,
        r"error: views are three frames of their own: not with frames, slices, turn or grow",
    ),
    (
        r#"save_png("b8.png", walk3d(pi + i), stereo, gray)"#,
        r"error: stereo is a picture of red and cyan for glasses of these colours: this does not go with gray",
    ),
    (
        r#"save_png("b8.png", walk3d(pi + i), stereo = 40)"#,
        r"error: stereo is a number from 0.1 to 30, not 40",
    ),
    (
        r#"save_jxl("b8.jxl", walk3d(pi + i), stereo, depth)"#,
        r"error: stereo is a picture of red and cyan for glasses of these colours: this does not go with depth",
    ),
    (
        r"walk3d(pi + i, 10, 100, stereo)",
        r#"error: stereo is chosen for a picture that is saved: save_png("walk3d.png", walk3d(...), stereo); the picture of characters takes width, height and ascii"#,
    ),
    (
        r#"save_png("b9.png", plot(sin(x), x, 0, 1), azimuth = 30)"#,
        r"error: azimuth, elevation, roll, viewpoint, perspective, center, scale, zoom, fog, frames, turn, grow, slices, views and stereo belong to a drawing of a plane or of space (walk, walk3d, plot3d ...), not to a chart",
    ),
    (
        r#"save_png("b9.png", plot(sin(x), x, 0, 1), slices = 3)"#,
        r"error: azimuth, elevation, roll, viewpoint, perspective, center, scale, zoom, fog, frames, turn, grow, slices, views and stereo belong to a drawing of a plane or of space (walk, walk3d, plot3d ...), not to a chart",
    ),
    (
        r#"save_png("c1.png", plot(sin(x), x, 0, 1, 40, 10))"#,
        r"error: 40 does not belong to a plot that is saved: a range [low, high] of the values may follow its arguments; the size of the picture is chosen with width = ... and height = ...",
    ),
    (
        r#"save_png("c2.png", walk(pi), widht = 3)"#,
        r"error: widht is nothing that is chosen for a picture: width, height, bits, channels, level, quality, distance, effort, frames, fps, turn, slices, thickness, margin, palette, color, colour, background, fog, azimuth, elevation, roll, zoom, scale, center, centre, viewpoint, perspective, stereo, title, xlabel, ylabel are chosen with name = value, gray, grey, alpha, lossless, depth, grow, axes, noaxes, grid, ascii, reverse, views, stereo with the word alone",
    ),
    (
        r#"save_png("c2.png", walk(pi), 800)"#,
        r"error: 800 is nothing that is chosen for a picture: after the drawing come width = 800, gray and the like",
    ),
    (
        r#"save_png("c3.png", sin(x))"#,
        r#"error: save_png("name", drawing, choices): save_png("walk.png", walk(pi, 10, 1000), width = 800); the drawing is one of walk, walk3d, primewalk, primewalk3d, primemodwalk, primemodwalk3d, ulamspiral, sacksspiral, plot, parametricplot, polarplot, listplot, histogram, implicitplot, complexplot, densityplot, plot3d, parametricplot3d, graphplot, matrixplot, lineplot, scatter, bar, barh, stairs, stem, errorbar, pie, loglog, semilogx, semilogy, contour, quiver, plot3, scatter3, surf, spy, boxplot, stem3, quiver3, contour3, waterfall, compass, feather, polarscatter"#,
    ),
    (
        r#"save_png("c3.png", [1, 2, 3])"#,
        r#"error: save_png("name", drawing, choices): save_png("walk.png", walk(pi, 10, 1000), width = 800); the drawing is one of walk, walk3d, primewalk, primewalk3d, primemodwalk, primemodwalk3d, ulamspiral, sacksspiral, plot, parametricplot, polarplot, listplot, histogram, implicitplot, complexplot, densityplot, plot3d, parametricplot3d, graphplot, matrixplot, lineplot, scatter, bar, barh, stairs, stem, errorbar, pie, loglog, semilogx, semilogy, contour, quiver, plot3, scatter3, surf, spy, boxplot, stem3, quiver3, contour3, waterfall, compass, feather, polarscatter"#,
    ),
    (
        r"save_png(c4, walk(pi))",
        r#"error: the name of the file is a text in quotes, not c4: save_png("name", drawing, choices): save_png("walk.png", walk(pi, 10, 1000), width = 800); the drawing is one of walk, walk3d, primewalk, primewalk3d, primemodwalk, primemodwalk3d, ulamspiral, sacksspiral, plot, parametricplot, polarplot, listplot, histogram, implicitplot, complexplot, densityplot, plot3d, parametricplot3d, graphplot, matrixplot, lineplot, scatter, bar, barh, stairs, stem, errorbar, pie, loglog, semilogx, semilogy, contour, quiver, plot3, scatter3, surf, spy, boxplot, stem3, quiver3, contour3, waterfall, compass, feather, polarscatter"#,
    ),
    (
        r#"save_png("c5.png")"#,
        r"error: save_png takes at least 2 argument(s), 1 given",
    ),
    (
        r#"save_png("", walk(pi))"#,
        r"error: the name of the file is empty",
    ),
    (
        r#"save_png("nodir/x.png", walk(pi))"#,
        r"error: cannot write nodir/x.png: No such file or directory (os error 2)",
    ),
    (
        r#"save_png("c6.png", walk(pi), channels = 3, gray)"#,
        r"error: channels = 3 is RGB: this does not go with gray",
    ),
    (
        r#"save_png("c6.png", walk(pi), channels = 5)"#,
        r"error: channels is a whole number from 1 to 4, not 5",
    ),
    (
        r#"save_png("c7.png", walk(pi), ascii)"#,
        r"error: ascii belongs to the picture of characters, which is not saved",
    ),
    (
        r#"save_png("c8.png", walk(pi), color = mauve)"#,
        r##"error: color is a colour: one of black, white, red, green, blue, yellow, cyan, magenta, orange, purple, brown, pink, gray, grey, navy, lime, gold, silver, or "#rrggbb", or [red, green, blue] with numbers from 0 to 1; not mauve"##,
    ),
    (
        r#"save_png("c8.png", walk(pi), palette = mauve)"#,
        r"error: the palette is one of rainbow, fire, ice, viridis, plasma, heat, ocean, sunset, parula, turbo, jet, hot, hsv, bone, copper, pink, spring, summer, autumn, winter, gray, grey, ink, cool, warm, or a list of colours; not mauve",
    ),
    (
        r#"save_png("c8.png", walk(pi), palette = [red])"#,
        r"error: the palette is one of rainbow, fire, ice, viridis, plasma, heat, ocean, sunset, parula, turbo, jet, hot, hsv, bone, copper, pink, spring, summer, autumn, winter, gray, grey, ink, cool, warm, or a list of colours; not [red]",
    ),
    (
        r#"save_png("c8.png", walk(pi), background = [2, 0, 0])"#,
        r##"error: background is a colour: one of black, white, red, green, blue, yellow, cyan, magenta, orange, purple, brown, pink, gray, grey, navy, lime, gold, silver, or "#rrggbb", or [red, green, blue] with numbers from 0 to 1; not [2, 0, 0]"##,
    ),
    (
        r#"save_png("c8.png", walk(pi), thickness = 0)"#,
        r"error: thickness is a number from 0.05 to 200, not 0",
    ),
    (
        r#"save_png("c8.png", walk(pi), margin = 0.5)"#,
        r"error: margin is a number from 0 to 0.45, not 1/2",
    ),
    (
        r#"save_png("c8.png", walk(pi), frames = 0)"#,
        r"error: frames is a whole number from 1 to 10000, not 0",
    ),
    (
        r#"save_png("c8.png", walk(pi), fps = 0, frames = 2)"#,
        r"error: fps is a number from 0.001 to 1000, not 0",
    ),
    (
        r#"save_png("c8.png", walk3d(pi + i), perspective = 1)"#,
        r"error: perspective is the distance of the eye in radii of the drawing, a number above 1, not 1",
    ),
    (
        r#"save_png("c8.png", walk3d(pi + i), viewpoint = 3)"#,
        r"error: viewpoint is a point [x, y, z], not 3",
    ),
    (
        r#"save_png("c8.png", walk3d(pi + i), elevation = 100)"#,
        r"error: elevation is a number from -90 to 90, not 100",
    ),
    (
        r#"save_png("c8.png", walk3d(pi + i, 10, 5), viewpoint = [0, 0, 0], center = [0, 0, 0])"#,
        r"error: the viewpoint is the middle of the drawing: there is no direction to look in",
    ),
    (
        r#"save_jxl("c8.jxl", walk(pi), bits = 17)"#,
        r"error: bits is a whole number from 1 to 16, not 17",
    ),
    (
        r#"save_jxl("c8.jxl", walk(pi), effort = 11)"#,
        r"error: effort is a whole number from 1 to 10, not 11",
    ),
    (
        r#"save_jxl("c8.jxl", walk(pi), quality = 101)"#,
        r"error: quality is a number from 0 to 100, not 101",
    ),
    (
        r#"save_jxl("c8.jxl", walk(pi), distance = 26)"#,
        r"error: distance is a number from 0 to 25, not 26",
    ),
    (
        r#"save_png("c8.png", walk(1 + i))"#,
        r"error: 1 + i is no real number: walk3d(z, base, digits) walks along a complex number",
    ),
    (
        r#"save_png("c8.png", walk(pi, 1))"#,
        r"error: the base is a whole number from 2 to 4294967295, not 1",
    ),
    (
        r#"x := save_png("c8.png", walk(pi))"#,
        r"error: save_png must be a statement of its own",
    ),
    (
        r#"1 + save_png("c8.png", walk(pi))"#,
        r"error: save_png must be a statement of its own",
    ),
];

#[test]
fn numbers_written_in_a_base() {
    check(NUMBERS_WRITTEN_IN_A_BASE);
}

#[test]
fn from_one_base_to_another() {
    check(FROM_ONE_BASE_TO_ANOTHER);
}

#[test]
fn digits_and_blocks_that_repeat() {
    check(DIGITS_AND_BLOCKS_THAT_REPEAT);
}

#[test]
fn walks_along_digits() {
    check(WALKS_ALONG_DIGITS);
}

#[test]
fn walks_along_primes() {
    check(WALKS_ALONG_PRIMES);
}

#[test]
fn charts_of_characters() {
    check(CHARTS_OF_CHARACTERS);
}

#[test]
fn pictures_that_are_saved() {
    let dir = scratch("pictures_that_are_saved");
    check_in(Some(&dir), PICTURES_THAT_ARE_SAVED);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn pictures_that_are_not_saved() {
    let dir = scratch("pictures_that_are_not_saved");
    check_in(Some(&dir), PICTURES_THAT_ARE_NOT_SAVED);
    let _ = std::fs::remove_dir_all(dir);
}

/// The samples of the frames of a PNG file, and its width, height and channels.
fn png_frames(path: &Path) -> (u32, u32, usize, Vec<Vec<u8>>) {
    let file = std::io::BufReader::new(std::fs::File::open(path).expect("the file"));
    let mut reader = png::Decoder::new(file).read_info().expect("a PNG file");
    let count = reader
        .info()
        .animation_control()
        .map_or(1, |a| a.num_frames);
    let mut frames = Vec::new();
    let mut shape = (0, 0, 0);
    for _ in 0..count {
        let mut buffer = vec![0; reader.output_buffer_size().expect("a size")];
        let info = reader.next_frame(&mut buffer).expect("a frame");
        buffer.truncate(info.buffer_size());
        shape = (info.width, info.height, info.color_type.samples());
        frames.push(buffer);
    }
    (shape.0, shape.1, shape.2, frames)
}

/// The samples of the frames of a JPEG XL file as bytes, the channels of a pixel one after
/// the other, and the names of its extra channels.
fn jxl_frames(path: &Path) -> (u32, u32, Vec<Vec<u8>>, Vec<String>) {
    let image = jxl_oxide::JxlImage::builder()
        .pool(jxl_oxide::JxlThreadPool::none())
        .read(std::io::Cursor::new(std::fs::read(path).expect("the file")))
        .expect("a JPEG XL file");
    let mut frames = Vec::new();
    let mut names = Vec::new();
    for k in 0..image.num_loaded_keyframes() {
        let render = image.render_frame(k).expect("a frame");
        names = render
            .extra_channels()
            .0
            .iter()
            .map(|c| {
                if c.is_alpha() {
                    "alpha".to_string()
                } else {
                    c.name().to_string()
                }
            })
            .collect();
        let planes = render.image_planar();
        let pixels = planes[0].buf().len();
        let mut samples = Vec::with_capacity(pixels * planes.len());
        for at in 0..pixels {
            for plane in &planes {
                samples.push((plane.buf()[at] * 255.0 + 0.5) as u8);
            }
        }
        frames.push(samples);
    }
    (image.width(), image.height(), frames, names)
}

/// The files that the program writes are pictures: a PNG file and a JPEG XL file without
/// loss hold the same pixels, frame by frame.
#[test]
fn the_files_are_pictures() {
    let dir = scratch("files");
    let run = |input: &str| {
        let got = answer(Some(&dir), input);
        assert!(
            got.contains("|saved ") || got.starts_with("saved "),
            "{input}: {got}"
        );
    };
    run(r#"save_png("walk.png", walk(pi, 10, 2000), width = 160, height = 120)"#);
    run(r#"save_jxl("walk.jxl", walk(pi, 10, 2000), width = 160, height = 120)"#);
    let (width, height, channels, png) = png_frames(&dir.join("walk.png"));
    assert_eq!((width, height, channels, png.len()), (160, 120, 3, 1));
    let (width, height, jxl, names) = jxl_frames(&dir.join("walk.jxl"));
    assert_eq!((width, height, names.len()), (160, 120, 0));
    assert_eq!(jxl, png);
    // the walk is drawn: many pixels are no background, and they have many colours
    let lit = png[0]
        .chunks(3)
        .filter(|p| p.iter().any(|v| *v > 0))
        .count();
    assert!(lit > 1500, "{lit} pixels of the walk");
    let colours: std::collections::HashSet<&[u8]> = png[0].chunks(3).collect();
    assert!(colours.len() > 500, "{} colours", colours.len());
    // moving pictures: the same frames in both kinds of file, all different
    run(r#"save_png("turn.png", walk3d(e + pi*i, 12, 800), width = 96, frames = 5, alpha)"#);
    run(r#"save_jxl("turn.jxl", walk3d(e + pi*i, 12, 800), width = 96, frames = 5, alpha)"#);
    let (width, height, channels, png) = png_frames(&dir.join("turn.png"));
    assert_eq!((width, height, channels, png.len()), (96, 96, 4, 5));
    let (_, _, jxl, names) = jxl_frames(&dir.join("turn.jxl"));
    assert_eq!(names, ["alpha"]);
    assert_eq!(jxl, png);
    assert!(png.windows(2).all(|w| w[0] != w[1]));
    // slabs of depth and a channel of depths
    run(r#"save_jxl("slabs.jxl", walk3d(e + pi*i, 12, 800), width = 96, slices = 7, depth)"#);
    let (_, _, slabs, names) = jxl_frames(&dir.join("slabs.jxl"));
    assert_eq!((slabs.len(), names), (7, vec!["depth".to_string()]));
    for (k, slab) in slabs.iter().enumerate() {
        // what is drawn in a slab is as deep as the slab
        let (near, far) = (255 * k / 7, 255 * (k + 1) / 7 + 1);
        let mut drawn = 0;
        for pixel in slab.chunks(4).filter(|p| p[..3].iter().any(|v| *v > 0)) {
            let depth = usize::from(pixel[3]);
            assert!(near <= depth + 1 && depth <= far, "slab {k}: depth {depth}");
            drawn += 1;
        }
        assert!(drawn > 0, "slab {k} is empty");
    }
    // a plot is dark lines on a light ground
    run(r#"save_png("plot.png", plot(sin(x), x, 0, 2pi), width = 200)"#);
    let (width, height, _, png) = png_frames(&dir.join("plot.png"));
    assert_eq!((width, height), (200, 150));
    let light = png[0]
        .chunks(3)
        .filter(|p| p.iter().all(|v| *v == 255))
        .count();
    assert!(light > 200 * 150 * 3 / 4, "{light} pixels of the ground");
    // nothing but the pictures is left
    let mut left: Vec<String> = std::fs::read_dir(&dir)
        .expect("the directory")
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .collect();
    left.sort();
    assert_eq!(
        left,
        [
            "plot.png",
            "slabs.jxl",
            "turn.jxl",
            "turn.png",
            "walk.jxl",
            "walk.png"
        ]
    );
    let _ = std::fs::remove_dir_all(dir);
}
