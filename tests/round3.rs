//! The lists, programs, statistics, distributions, transforms, special functions and the other
//! additions of the third round: runs the program on fixed inputs and compares what it prints.
//! The values of the special functions were compared with other software first.

use std::process::{Command, Stdio};

fn program() -> Command {
    Command::new(env!("CARGO_BIN_EXE_symbolic-math"))
}

/// What the program prints for an input, with `|` for line ends.
fn answer(options: &[&str], input: &str) -> String {
    let mut args = options.to_vec();
    args.push(input);
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

const LISTS: &[(&str, &str)] = &[
    (r"range(5)", r"[1, 2, 3, 4, 5]"),
    (r"range(2, 10, 3)", r"[2, 5, 8]"),
    (r"range(0)", r"[]"),
    (r"range(1, 2, 1/4)", r"[1, 5/4, 3/2, 7/4, 2]"),
    (r"range(5, 1, -2)", r"[5, 3, 1]"),
    (r"table(k^2, k, 1, 5)", r"[1, 4, 9, 16, 25]"),
    (r"table(k^2, [k, 1, 5])", r"[1, 4, 9, 16, 25]"),
    (r"table(a+b, [a,1,2], [b,1,3])", r"[[2, 3, 4], [3, 4, 5]]"),
    (r"table(k, [k, [2,3,5]])", r"[2, 3, 5]"),
    (r"table(x, 3)", r"[x, x, x]"),
    (r"table(i^2, i, 1, 3)", r"[1, 4, 9]"),
    (r"table(i+k, [i,1,2], [k,1,3])", r"[[2, 3, 4], [3, 4, 5]]"),
    (
        r"table(i+j, [i,1,2], [j,1,3])",
        r"error: i and j are two names of the imaginary unit, and both are indices here: a loop inside a loop needs another name for its index",
    ),
    (r"length([1,2,3])", r"3"),
    (r"length([])", r"0"),
    (r"first([a,b,c])", r"a"),
    (r"last([a,b,c])", r"c"),
    (r"rest([a,b,c])", r"[b, c]"),
    (r"most([a,b,c])", r"[a, b]"),
    (
        r"first([])",
        r"error: there is no position 1 in a list of 0 entries",
    ),
    (r"part([a,b,c], 2)", r"b"),
    (r"part([a,b,c], -1)", r"c"),
    (r"part([[1,2],[3,4]], 2, 1)", r"3"),
    (r"[a,b,c][[2]]", r"b"),
    (r"[[1,2],[3,4]][[2]][[1]]", r"3"),
    (
        r"[a,b,c][[5]]",
        r"error: there is no position 5 in a list of 3 entries",
    ),
    (r"take([1,2,3,4,5], 2)", r"[1, 2]"),
    (r"take([1,2,3,4,5], -2)", r"[4, 5]"),
    (r"drop([1,2,3,4,5], 2)", r"[3, 4, 5]"),
    (r"drop([1,2,3,4,5], -2)", r"[1, 2, 3]"),
    (r"reverse([1,2,3])", r"[3, 2, 1]"),
    (r"sort([3,1,2])", r"[1, 2, 3]"),
    (
        r"sort([3, pi, e, sqrt(2), 1/7+sqrt(2)/7])",
        r"[1/7 + sqrt(2)/7, sqrt(2), e, 3, pi]",
    ),
    (r"sort([b, a, 3, 1+i, 1-i])", r"[1 - i, 1 + i, 3, a, b]"),
    (r"sortby([3,-1,2], abs)", r"[-1, 2, 3]"),
    (r"sortby([[2,a],[1,b]], first)", r"[[1, b], [2, a]]"),
    (r"sortby([3,-1,2], x -> -x)", r"[3, 2, -1]"),
    (r"rotateleft([1,2,3,4])", r"[2, 3, 4, 1]"),
    (r"rotateright([1,2,3,4], 2)", r"[3, 4, 1, 2]"),
    (r"append([1,2], 3)", r"[1, 2, 3]"),
    (r"prepend([1,2], 0)", r"[0, 1, 2]"),
    (r"join([1,2],[3],[4,5])", r"[1, 2, 3, 4, 5]"),
    (r"insert([1,2,3], x, 2)", r"[1, x, 2, 3]"),
    (r"delete([1,2,3], 2)", r"[1, 3]"),
    (r"union([1,2],[2,3])", r"[1, 2, 3]"),
    (r"union([3,1],[2,1],[5])", r"[1, 2, 3, 5]"),
    (r"intersection([1,2,3],[2,3,4])", r"[2, 3]"),
    (r"complement([1,2,3,4],[2,4])", r"[1, 3]"),
    (r"unique([1,1,2,3,3])", r"[1, 2, 3]"),
    (r"contains([1,2,3], 2)", r"true"),
    (r"contains([1,2,3], 5)", r"false"),
    (r"count([1,2,1,1], 1)", r"3"),
    (r"position([a,b,a], a)", r"[1, 3]"),
    (r"total([1,2,3,4])", r"10"),
    (r"total([a, b, 2a])", r"3a + b"),
    (r"accumulate([1,2,3,4])", r"[1, 3, 6, 10]"),
    (r"differences([1,4,9,16])", r"[3, 5, 7]"),
    (r"ratios([1,2,4,8])", r"[2, 2, 2]"),
    (r"tally([a,b,a,c,a])", r"[[a, 3], [b, 1], [c, 1]]"),
    (r"zip([1,2],[a,b])", r"[[1, a], [2, b]]"),
    (r"zip([1,2,3],[a,b],[x,y,z])", r"[[1, a, x], [2, b, y]]"),
    (
        r"permutations([1,2,3])",
        r"[[1, 2, 3], [1, 3, 2], [2, 1, 3], [2, 3, 1], [3, 1, 2], [3, 2, 1]]",
    ),
    (r"length(permutations([1,2,3,4,5]))", r"120"),
    (
        r"subsets([1,2,3])",
        r"[[], [1], [2], [3], [1, 2], [1, 3], [2, 3], [1, 2, 3]]",
    ),
    (r"subsets([1,2,3], 2)", r"[[1, 2], [1, 3], [2, 3]]"),
    (r"tuples([0,1], 2)", r"[[0, 0], [0, 1], [1, 0], [1, 1]]"),
    (
        r"cartesianproduct([1,2],[a,b])",
        r"[[1, a], [1, b], [2, a], [2, b]]",
    ),
    (r"partition([1,2,3,4,5,6], 2)", r"[[1, 2], [3, 4], [5, 6]]"),
    (r"partition([1,2,3,4,5], 2)", r"[[1, 2], [3, 4]]"),
    (r"map(x -> x^2, [1,2,3])", r"[1, 4, 9]"),
    (r"map(sin, [0, pi/2])", r"[0, 1]"),
    (r"map(x -> x^2, [a,b])", r"[a^2, b^2]"),
    (r"select([1,2,3,4,5,6], iseven)", r"[2, 4, 6]"),
    (r"select([1,2,3,4,5,6], x -> x > 3)", r"[4, 5, 6]"),
    (r"select(x -> x > 3, [1,2,3,4,5,6])", r"[4, 5, 6]"),
    (r"reject([1,2,3,4], isodd)", r"[2, 4]"),
    (r"countif([1,2,3,4], isprime)", r"2"),
    (r"anytrue([1,2,3], x -> x > 2)", r"true"),
    (r"alltrue([1,2,3], ispositive)", r"true"),
    (r"alltrue([1,-2,3], ispositive)", r"false"),
    (r"findfirst([4,6,7,9], isprime)", r"7"),
    (r"findfirst([4,6,8], isprime)", r"[]"),
    (r"fold((a, b) -> a + b, 0, [1,2,3])", r"6"),
    (r"fold((a, b) -> a*x + b, 0, [1,2,3])", r"x^2 + 2x + 3"),
    (
        r"foldlist((a, b) -> a*b, 1, [1,2,3,4])",
        r"[1, 1, 2, 6, 24]",
    ),
    (r"nestlist(x -> 2x, 1, 5)", r"[1, 2, 4, 8, 16, 32]"),
    (r"nestlist(sin, x, 2)", r"[x, sin(x), sin(sin(x))]"),
    (r"nest(x -> x^2, 2, 3)", r"256"),
    (r"nest(x -> x^2, x, 3)", r"x^8"),
    (r"nest(sin, x, 3)", r"sin(sin(sin(x)))"),
    (r"nest(x -> 2x+1, 1, 10)", r"2047"),
    (
        r"fixedpoint(cos, 1.0)",
        r"0.73908513321516064165531208767387",
    ),
    (
        r"fixedpoint(x -> (x + 2/x)/2, 1.0)",
        r"1.4142135623730950488016887242097",
    ),
    (
        r"fixedpoint(x -> x/2 + 1, 0, 5)",
        r"31/16|  ~ 1.9375|note: no fixed point after 5 steps: the last value is shown",
    ),
    (r"apply((a,b) -> a+b, [1,2])", r"3"),
    (r"array(k -> k^2, 3)", r"[1, 4, 9]"),
    (r"array((a,b) -> a+b, [2,3])", r"[[2, 3, 4], [3, 4, 5]]"),
    (r"max([1,5,3])", r"5"),
    (r"min([1,5,3])", r"1"),
    (r"max(1, 5, 3)", r"5"),
    (r"mean([1,2,3,4])", r"5/2|  ~ 2.5"),
    (r"[1,2,3] + [10,20,30]", r"[11, 22, 33]"),
    (r"2*[1,2,3]", r"[2, 4, 6]"),
    (r"[1,2,3]^2", r"[1, 4, 9]"),
    (r"sin([0, pi/2, pi])", r"[0, 1, 0]"),
    (r"[1,2,3] == [1,2,3]", r"true"),
    (r"[1,2,3] == [1,2,4]", r"false"),
    (r"[1,2] == [1,2,3]", r"false"),
    (r"[1,2,3] != [1,2,4]", r"true"),
    (r"[[1,2],[3,4]] == [[1,2],[3,4]]", r"true"),
    (r"[1,2] == 3", r"false"),
    (r"[x, 2] == [1, 2]", r"x = 1"),
    (r"[x, 2] != [1, 3]", r"true"),
    (r"[x, 2] != [1, 2]", r"[x, 2] != [1, 2]"),
    (r"[[x,2],[3,y]] == [[1,2],[3,4]]", r"x = 1, y = 4"),
    (r"[x+y, x-y] = [2, 0]", r"x = 1, y = 1"),
    (r"[x^2, y] = [4, x]", r"x = -2, y = -2|x = 2, y = 2"),
    (r"[x, x] = [1, 2]", r"no solution"),
    (r"[x+y, x*y] = [3, 2]", r"x = 2, y = 1|x = 1, y = 2"),
    (r"[sin(x), 1] = [0, 1]", r"x = k*pi, k is an integer"),
    (
        r"[x, y, z] = [1, 2, 3] + t*[1, 1, 1]",
        r"x = t + 1, y = t + 2, z = t + 3",
    ),
    (r"x^2 = [1, 4]", r"x = -1|x = 1|x = -2|x = 2"),
    (r"3 +/- 1", r"[4, 2]"),
    (r"3 -/+ 1", r"[2, 4]"),
    (r"3 ± 1", r"[4, 2]"),
    (r"3 ∓ 1", r"[2, 4]"),
    (r"x = +/-2", r"x = 2|x = -2"),
    (r"x^2 = ±4", r"x = -2|x = 2|x = -2i|x = 2i"),
    (r"2*(±3)", r"[6, -6]"),
    (r"a - ±b", r"[a - b, a + b]"),
    (r"x = (-3 +/- sqrt(9-8))/2", r"x = -1|x = -2"),
];

const PROGRAMS_AND_LOGIC: &[(&str, &str)] = &[
    (r"if(1 < 2, a, b)", r"a"),
    (r"if(2 < 1, a, b)", r"b"),
    (r"if(x > 0, a, b)", r"piecewise(x > 0, a, b)"),
    (r"if(1 < 2, a)", r"a"),
    (r"piecewise(x < 0, x^2, x)", r"piecewise(x < 0, x^2, x)"),
    (r"subst(piecewise(x < 0, x^2, x), x, -2)", r"4"),
    (r"subst(piecewise(x < 0, x^2, x), x, 3)", r"3"),
    (
        r"diff(piecewise(x < 0, x^2, x), x)",
        r"piecewise(x < 0, 2x, 1)",
    ),
    (
        r"integrate(piecewise(x < 0, x^2, x), x, -1, 1)",
        r"5/6|  ~ 0.83333333333333333333333333333333",
    ),
    (r"limit(piecewise(x < 0, x^2, x), x, 0)", r"0"),
    (
        r"piecewise(x < 0, x^2, x < 1, x, 2)",
        r"piecewise(x < 0, x^2, x < 1, x, 2)",
    ),
    (
        r"subst(piecewise(x < 0, x^2, x < 1, x, 2), x, 1/2)",
        r"1/2|  ~ 0.5",
    ),
    (r"subst(piecewise(x < 0, x^2, x < 1, x, 2), x, 5)", r"2"),
    (
        r"f(x) := piecewise(x < 0, x^2, x); f(-3); f(2)",
        r"f(x) := piecewise(x < 0, x^2, x)|9|2",
    ),
    (r"k := 1; while(k < 100, k := 2k); k", r"1|128|128"),
    (
        r"while(k < 100, k := 2k)",
        r"error: while: the condition k < 100 is neither true nor false",
    ),
    (r"s := 0; do(s := s + k^2, k, 1, 10); s", r"0|385|385"),
    (r"do(k^2, k, 1, 3)", r"9"),
    (r"for(k, 1, 3, k^2)", r"9"),
    (r"block(a := 2, b := 3, a*b)", r"6"),
    (r"a := 5; a^2; a := a + 1; a", r"5|25|6|6"),
    (
        r"f(x) := x^2 + 1; f(3); f(f(x))",
        r"f(x) := x^2 + 1|10|x^4 + 2x^2 + 2",
    ),
    (r"g(x, y) := x*y + 1; g(2, 3)", r"g(x, y) := x*y + 1|7"),
    (r"and(true, false)", r"false"),
    (r"true and false", r"false"),
    (r"true and true", r"true"),
    (r"or(true, false)", r"true"),
    (r"false or false", r"false"),
    (r"not(true)", r"false"),
    (r"not true", r"false"),
    (r"xor(true, true)", r"false"),
    (r"xor(true, false)", r"true"),
    (r"implies(true, false)", r"false"),
    (r"implies(false, false)", r"true"),
    (r"1 < 2 and 2 < 3", r"true"),
    (r"1 < 2 or 2 < 1", r"true"),
    (r"not(1 < 2)", r"false"),
    (r"x > 1 and x < 3", r"1 < x < 3"),
    (r"2 < 3", r"true"),
    (r"3 <= 3", r"true"),
    (r"3 == 3", r"true"),
    (r"3 != 3", r"false"),
    (r"x == x", r"true"),
    (r"pi > 3", r"true"),
    (r"e^pi > pi^e", r"true"),
    (r"sqrt(2) < 1.5", r"true"),
    (r"0 < 1/10^27 + sqrt(13)/10^27", r"true"),
    (r"(1+sqrt(13))/3 < 2", r"true"),
    (r"1/3 + sqrt(2)/3 == (1+sqrt(2))/3", r"true"),
    (
        r"1 + i < 2",
        r"error: 1 + i < 2 compares values that are not real",
    ),
    (r"x < y", r"x < y"),
    (r"islist([1])", r"true"),
    (r"islist(1)", r"false"),
    (r"isnumber(pi)", r"true"),
    (r"isnumber(x)", r"false"),
    (r"isinteger(4/2)", r"true"),
    (r"isinteger(1/2)", r"false"),
    (r"isrational(1/3)", r"true"),
    (r"isrational(sqrt(2))", r"false"),
    (r"iseven(4)", r"true"),
    (r"isodd(4)", r"false"),
    (r"isreal(1+i)", r"false"),
    (r"isreal(pi)", r"true"),
    (r"ispositive(-1)", r"false"),
    (r"isnegative(-1)", r"true"),
    (r"isnonnegative(0)", r"true"),
    (r"if(isprime(7), yes, no)", r"yes"),
    (r"isrational(pi)", r"false"),
    (r"isrational(e)", r"false"),
    (r"isrational(pi+e)", r"isrational(e + pi)"),
    (r"isrational(1/3+sqrt(2)/7)", r"false"),
    (r"isrational(sqrt(2)+sqrt(3))", r"false"),
    (r"isrational(sqrt(2)*sqrt(3))", r"false"),
    (r"isrational(2^(1/3))", r"false"),
    (r"isrational(ln(2))", r"false"),
    (r"isrational(sin(1))", r"false"),
    (r"isrational(pi^2/6)", r"false"),
    (r"isrational(zeta(3))", r"isrational(zeta(3))"),
    (r"isrational(i)", r"false"),
    (r"isrational(sqrt(2)^2)", r"true"),
    (r"isrational(cos(1)^2+sin(1)^2)", r"true"),
    (r"isrational((1+sqrt(5))/2)", r"false"),
    (
        r"isrational(sqrt(2)+2^(1/3))",
        r"isrational(2^(1/3) + sqrt(2))",
    ),
    (r"isrational(1+pi+sqrt(2))", r"false"),
    (r"isinteger(sqrt(2))", r"false"),
    (r"isinteger(pi)", r"false"),
    (r"isinteger(sqrt(3+2sqrt(2)) - sqrt(2))", r"true"),
    (r"isinteger(e^(i*pi))", r"true"),
    (r"isinteger(zeta(3))", r"false"),
    (r"isinteger(x)", r"isinteger(x)"),
    (r"isinteger(gamma(5))", r"true"),
    (r"isinteger(2.5)", r"false"),
    (r"iseven(sqrt(16))", r"true"),
    (r"isodd(sin(pi/2))", r"true"),
    (
        r"select([1, sqrt(2), 2/3, pi, 4], isrational)",
        r"[1, 2/3, 4]",
    ),
    (r"select([1, sqrt(2), 2/3, pi, 4], isinteger)", r"[1, 4]"),
    (
        r"assume(n, integer); isinteger(n); isrational(n); isinteger(n/2)",
        r"true|true|true|isinteger(n/2)",
    ),
    (
        r"select(range(30), isprime)",
        r"[2, 3, 5, 7, 11, 13, 17, 19, 23, 29]",
    ),
    (
        r"table(if(iseven(k), k/2, 3k+1), k, 1, 6)",
        r"[4, 1, 10, 2, 16, 3]",
    ),
    (r"sum(if(isprime(k), k, 0), k, 1, 20)", r"77"),
];

const STATISTICS: &[(&str, &str)] = &[
    (r"mean([1,2,3,4])", r"5/2|  ~ 2.5"),
    (r"mean([a,b])", r"a/2 + b/2"),
    (r"median([3,1,2])", r"2"),
    (r"median([4,1,3,2])", r"5/2|  ~ 2.5"),
    (r"mode([1,2,2,3])", r"2"),
    (r"mode([1,1,2,2,3])", r"[1, 2]"),
    (
        r"variance([1,2,3,4])",
        r"5/3|  ~ 1.6666666666666666666666666666667",
    ),
    (r"variance([a,b])", r"(a - b)^2/2"),
    (
        r"stddev([1,2,3,4])",
        r"sqrt(15)/3|  ~ 1.2909944487358056283930884665941",
    ),
    (r"pvariance([1,2,3,4])", r"5/4|  ~ 1.25"),
    (r"pstddev([2,4,4,4,5,5,7,9])", r"2"),
    (r"covariance([1,2,3],[2,4,7])", r"5/2|  ~ 2.5"),
    (
        r"correlation([1,2,3],[2,4,7])",
        r"5sqrt(57)/38|  ~ 0.99339926779878285489956379038765",
    ),
    (r"correlation([1,2,3],[2,4,6])", r"1"),
    (r"correlation([1,2,3],[6,4,2])", r"-1"),
    (r"quantile([1,2,3,4,5], 1/4)", r"2"),
    (r"quantile([1,2,3,4,5], 1/2)", r"3"),
    (r"quartiles([1,2,3,4,5,6,7,8])", r"[11/4, 9/2, 25/4]"),
    (r"iqr([1,2,3,4,5,6,7,8])", r"7/2|  ~ 3.5"),
    (
        r"moment([1,2,3], 2)",
        r"14/3|  ~ 4.6666666666666666666666666666667",
    ),
    (r"centralmoment([1,2,3,4], 2)", r"5/4|  ~ 1.25"),
    (r"centralmoment([1,2,3,4], 3)", r"0"),
    (
        r"skewness([1,2,3,10])",
        r"18sqrt(2)/25|  ~ 1.018233764908628435137215881431",
    ),
    (r"kurtosis([1,2,3,10])", r"1394/625|  ~ 2.2304"),
    (r"geometricmean([1,2,4])", r"2"),
    (
        r"harmonicmean([1,2,4])",
        r"12/7|  ~ 1.7142857142857142857142857142857",
    ),
    (
        r"rms([3,4])",
        r"5sqrt(2)/2|  ~ 3.5355339059327376220042218105242",
    ),
    (r"meandeviation([1,2,3,4])", r"1"),
    (
        r"mean([1.5, 2.5, 4.0])",
        r"2.6666666666666666666666666666667",
    ),
    (
        r"stddev([1.5, 2.5, 4.0])",
        r"1.258305739211791616206114134491",
    ),
    (r"variance([1+i, 1-i])", r"2"),
    (r"mean([])", r"undefined"),
    (
        r"variance([1])",
        r"error: variance needs two values or more",
    ),
    (r"median([x, y])", r"error: median needs real numbers"),
    (r"total([1,2,3])/length([1,2,3])", r"2"),
];

const DISTRIBUTIONS: &[(&str, &str)] = &[
    (
        r"pdf(normaldist(0,1), x)",
        r"sqrt(2)*e^(-x^2/2)/(2sqrt(pi))",
    ),
    (
        r"pdf(normaldist(m,s), x)",
        r"sqrt(2)*e^(-(x - m)^2/(2s^2))/(2sqrt(pi)*s)",
    ),
    (
        r"cdf(normaldist(0,1), 1)",
        r"1/2 + erf(sqrt(2)/2)/2|  ~ 0.84134474606854294858523254563204",
    ),
    (
        r"cdf(normaldist(m,s), x)",
        r"erf(sqrt(2)*(x - m)/(2s))/2 + 1/2",
    ),
    (
        r"quantile(normaldist(0,1), 0.975)",
        r"1.9599639845400542355245944305206",
    ),
    (r"quantile(normaldist(0,1), 1/2)", r"0"),
    (r"mean(normaldist(m,s))", r"m"),
    (r"variance(normaldist(m,s))", r"s^2"),
    (r"stddev(normaldist(m,s))", r"s"),
    (r"skewness(normaldist(0,1))", r"0"),
    (r"kurtosis(normaldist(0,1))", r"3"),
    (r"mgf(normaldist(m,s), t)", r"e^(s^2*t^2/2 + m*t)"),
    (r"expectation(x^2, x, normaldist(0,1))", r"1"),
    (r"expectation(x, x, exponentialdist(2))", r"1/2|  ~ 0.5"),
    (
        r"probability(x > 1, x, normaldist(0,1))",
        r"1/2 - erf(sqrt(2)/2)/2|  ~ 0.15865525393145705141476745436796",
    ),
    (r"probability(x < 1/2, x, uniformdist(0,1))", r"1/2|  ~ 0.5"),
    (
        r"probability(x > 1 and x < 2, x, exponentialdist(1))",
        r"1/e - 1/e^2|  ~ 0.23254415793482962970152427518898",
    ),
    (
        r"pdf(normaldist(), 0)",
        r"sqrt(2)/(2sqrt(pi))|  ~ 0.39894228040143267793994605993438",
    ),
    (
        r"pdf(uniformdist(a,b), x)",
        r"piecewise(a <= x and x <= b, 1/(b - a), 0)",
    ),
    (
        r"cdf(uniformdist(0,1), 1/3)",
        r"1/3|  ~ 0.33333333333333333333333333333333",
    ),
    (r"mean(uniformdist(a,b))", r"a/2 + b/2"),
    (r"variance(uniformdist(a,b))", r"(b - a)^2/12"),
    (
        r"stddev(uniformdist(0,1))",
        r"sqrt(3)/6|  ~ 0.28867513459481288225457439025098",
    ),
    (
        r"pdf(exponentialdist(2), x)",
        r"piecewise(x >= 0, 2e^(-2x), 0)",
    ),
    (
        r"cdf(exponentialdist(2), 1)",
        r"1 - 1/e^2|  ~ 0.86466471676338730810600050502752",
    ),
    (r"mean(exponentialdist(l))", r"1/l"),
    (r"variance(exponentialdist(l))", r"1/l^2"),
    (
        r"median(exponentialdist(1))",
        r"ln(2)|  ~ 0.69314718055994530941723212145818",
    ),
    (r"skewness(exponentialdist(1))", r"2"),
    (
        r"quantile(exponentialdist(1), 1/2)",
        r"ln(2)|  ~ 0.69314718055994530941723212145818",
    ),
    (r"mean(gammadist(a,b))", r"a*b"),
    (r"variance(gammadist(a,b))", r"a*b^2"),
    (r"variance(gammadist(2))", r"2"),
    (r"pdf(gammadist(2,1), x)", r"piecewise(x > 0, x*e^(-x), 0)"),
    (
        r"pdf(chisquaredist(3), x)",
        r"piecewise(x > 0, sqrt(2)*sqrt(x)*e^(-x/2)/(2sqrt(pi)), 0)",
    ),
    (r"mean(chisquaredist(k))", r"k"),
    (r"variance(chisquaredist(k))", r"2k"),
    (
        r"cdf(studenttdist(5), 2)",
        r"1/2 + 16sqrt(5)*hyp2f1(1/2, 3, 3/2, -4/5)/(15pi)|  ~ 0.94903026058507082187731944707886",
    ),
    (r"pdf(studenttdist(1), x)", r"1/(pi*x^2 + pi)"),
    (
        r"variance(studenttdist(5))",
        r"5/3|  ~ 1.6666666666666666666666666666667",
    ),
    (
        r"pdf(betadist(2,3), x)",
        r"piecewise(0 < x and x < 1, 12x*(1 - x)^2, 0)",
    ),
    (r"mean(betadist(2,3))", r"2/5|  ~ 0.4"),
    (r"variance(betadist(a,b))", r"a*b/((a + b + 1)*(a + b)^2)"),
    (r"pdf(cauchydist(0,1), x)", r"1/(pi*x^2 + pi)"),
    (r"cdf(cauchydist(0,1), 1)", r"3/4|  ~ 0.75"),
    (r"mean(cauchydist(0,1))", r"undefined"),
    (
        r"mean(lognormaldist(0,1))",
        r"sqrt(e)|  ~ 1.6487212707001281468486507878142",
    ),
    (r"median(lognormaldist(m,s))", r"e^m"),
    (
        r"mean(weibulldist(2,1))",
        r"sqrt(pi)/2|  ~ 0.88622692545275801364908374167057",
    ),
    (
        r"mean(weibulldist(2))",
        r"sqrt(pi)/2|  ~ 0.88622692545275801364908374167057",
    ),
    (r"variance(laplacedist(0,b))", r"2b^2"),
    (r"pdf(laplacedist(0,1), x)", r"e^(-abs(x))/2"),
    (r"mean(rayleighdist(s))", r"sqrt(2)*sqrt(pi)*s/2"),
    (
        r"cdf(rayleighdist(1), x)",
        r"piecewise(x >= 0, 1 - e^(-x^2/2), 0)",
    ),
    (r"cdf(logisticdist(0,1), x)", r"e^x/(e^x + 1)"),
    (
        r"variance(logisticdist(0,1))",
        r"pi^2/3|  ~ 3.2898681336964528729448303332921",
    ),
    (r"mean(paretodist(1,3))", r"3/2|  ~ 1.5"),
    (r"variance(paretodist(1,3))", r"3/4|  ~ 0.75"),
    (
        r"mean(fdist(3,5))",
        r"5/3|  ~ 1.6666666666666666666666666666667",
    ),
    (
        r"pdf(binomialdist(10, 1/3), 4)",
        r"4480/19683|  ~ 0.22760758014530305339633185998069",
    ),
    (
        r"cdf(binomialdist(10, 1/3), 4)",
        r"15488/19683|  ~ 0.78687191993090484174160443021897",
    ),
    (r"mean(binomialdist(n, p))", r"n*p"),
    (r"variance(binomialdist(n, p))", r"n*p*(1 - p)"),
    (
        r"pdf(binomialdist(n, p), k)",
        r"p^k*binomial(n, k)*(1 - p)^(n - k)",
    ),
    (
        r"table(pdf(binomialdist(4, 1/2), k), k, 0, 4)",
        r"[1/16, 1/4, 3/8, 1/4, 1/16]",
    ),
    (
        r"pdf(poissondist(3), 2)",
        r"9/(2e^3)|  ~ 0.22404180765538774340704087042528",
    ),
    (
        r"cdf(poissondist(3), 2)",
        r"17/(2e^3)|  ~ 0.42319008112684351532441053302553",
    ),
    (r"mean(poissondist(l))", r"l"),
    (r"variance(poissondist(l))", r"l"),
    (r"mgf(poissondist(l), t)", r"e^(l*e^t - l)"),
    (
        r"pdf(geometricdist(1/3), 2)",
        r"4/27|  ~ 0.14814814814814814814814814814815",
    ),
    (r"mean(geometricdist(p))", r"1/p - 1"),
    (r"mean(bernoullidist(p))", r"p"),
    (r"variance(bernoullidist(p))", r"p - p^2"),
    (
        r"pdf(discreteuniformdist(1,6), 3)",
        r"1/6|  ~ 0.16666666666666666666666666666667",
    ),
    (r"mean(discreteuniformdist(1,6))", r"7/2|  ~ 3.5"),
    (
        r"variance(discreteuniformdist(1,6))",
        r"35/12|  ~ 2.9166666666666666666666666666667",
    ),
    (r"mean(negativebinomialdist(3,1/2))", r"3"),
    (r"pdf(hypergeometricdist(3,4,10), 1)", r"1/2|  ~ 0.5"),
    (r"mean(hypergeometricdist(3,4,10))", r"6/5|  ~ 1.2"),
    (
        r"table(pdf(hypergeometricdist(3,4,10), k), [k,0,3])",
        r"[1/6, 1/2, 3/10, 1/30]",
    ),
    (r"pdf(binomialdist(10, 1/3), 11)", r"0"),
    (r"pdf(binomialdist(10, 1/3), 1/2)", r"0"),
    (r"pdf(poissondist(3), -1)", r"0"),
    (
        r"mean(binomialdist(10, 2))",
        r"error: binomialdist(n, p) needs n >= 0 and 0 <= p and p <= 1",
    ),
    (
        r"pdf(normaldist(0,-1), x)",
        r"error: normaldist(m, s) needs s > 0",
    ),
    (
        r"mean(exponentialdist(-2))",
        r"error: exponentialdist(r) needs r > 0",
    ),
    (
        r"pdf(poissondist(-1), 2)",
        r"error: poissondist(l) needs l > 0",
    ),
    (
        r"mean(uniformdist(3,1))",
        r"error: uniformdist(a, b) needs a < b",
    ),
    (
        r"pdf(hypergeometricdist(10,4,3), 1)",
        r"error: hypergeometricdist(n, s, u) needs 0 <= n and n <= u and 0 <= s and s <= u",
    ),
    (
        r"mean(geometricdist(0))",
        r"error: geometricdist(p) needs 0 < p and p <= 1",
    ),
    (
        r"mean(bernoullidist(3/2))",
        r"error: bernoullidist(p) needs 0 <= p and p <= 1",
    ),
    (
        r"pdf(normaldist(1,2,3), x)",
        r"error: normaldist takes 0 to 2 argument(s), 3 given",
    ),
    (
        r"mean(betadist(2))",
        r"error: betadist takes 2 argument(s), 1 given",
    ),
    (
        r"quantile(normaldist(0,1), 2)",
        r"error: quantile(dist, q): q must be a number from 0 to 1",
    ),
    (
        r"quantile(normaldist(0,1), -1/2)",
        r"error: quantile(dist, q): q must be a number from 0 to 1",
    ),
    (
        r"quantile(normaldist(0,1), i)",
        r"error: quantile(dist, q): q must be a number from 0 to 1",
    ),
    (r"quantile(normaldist(0,1), 0)", r"-inf"),
    (r"quantile(normaldist(0,1), 1)", r"inf"),
    (r"quantile(normaldist(0,1), q)", r"sqrt(2)*erfinv(2q - 1)"),
    (
        r"quantile(normaldist(0,1), [0.025, 0.5, 0.975])",
        r"[-1.9599639845400542355245944305206, 0, 1.9599639845400542355245944305206]",
    ),
    (r"quantile(binomialdist(10,1/2), 0.5)", r"5"),
    (r"quantile(binomialdist(10,1/2), 1)", r"10"),
    (r"quantile(binomialdist(10,1/2), 0)", r"0"),
    (r"quantile(poissondist(3), 0.5)", r"3"),
    (r"quantile(poissondist(3), 1)", r"inf"),
    (r"quantile(geometricdist(1/3), 9/10)", r"5"),
    (
        r"quantile(studenttdist(5), 0.975)",
        r"2.5705818356363155146962462174396",
    ),
    (
        r"quantile(studenttdist(1), 0.975)",
        r"12.706204736174704646021679978842",
    ),
    (
        r"quantile(studenttdist(30), 0.975)",
        r"2.0422724563012383099580422320339",
    ),
    (
        r"quantile(studenttdist(5), 0.025)",
        r"-2.5705818356363155146962462174396",
    ),
    (
        r"quantile(chisquaredist(3), 0.95)",
        r"7.8147279032511799552689948735205",
    ),
    (
        r"quantile(fdist(3,5), 0.95)",
        r"5.4094513180564914681652924434708",
    ),
    (
        r"quantile(gammadist(2,3), 0.5)",
        r"5.0350409700499819602386535362836",
    ),
    (
        r"quantile(betadist(2,3), 1/2)",
        r"0.38572756813238954827550275115735",
    ),
    (
        r"quantile(cauchydist(0,1), 0.975)",
        r"12.706204736174704646021679978842",
    ),
    (
        r"quantile(logisticdist(0,1), 0.9)",
        r"2.1972245773362193827904904738451",
    ),
    (
        r"quantile(weibulldist(2,1), 0.5)",
        r"0.8325546111576977563531646448952",
    ),
    (
        r"quantile(paretodist(1,3), 0.5)",
        r"1.2599210498948731647672106072782",
    ),
    (
        r"quantile(laplacedist(0,1), 0.9)",
        r"1.6094379124341003746007593332262",
    ),
    (
        r"quantile(rayleighdist(1), 0.5)",
        r"1.1774100225154746910115693264597",
    ),
    (r"median(binomialdist(10,1/2))", r"5"),
    (r"stddev(exponentialdist(l))", r"1/l"),
    (r"stddev(gammadist(a,b))", r"sqrt(a)*b"),
    (r"stddev(uniformdist(a,b))", r"sqrt(3)*abs(b - a)/6"),
    (r"stddev(poissondist(l))", r"sqrt(l)"),
    (r"stddev(laplacedist(m,b))", r"sqrt(2)*b"),
    (r"stddev(normaldist(0,2))", r"2"),
    (
        r"cdf(studenttdist(5), -2.0)",
        r"0.050969739414929178122680552921144",
    ),
    (
        r"cdf(studenttdist(5), 2.5)",
        r"0.97275495032881187944224362041969",
    ),
];

const TRANSFORMS: &[(&str, &str)] = &[
    (r"laplace(1, t, s)", r"1/s"),
    (r"laplace(t^2, t, s)", r"2/s^3"),
    (r"laplace(t^n, t, s)", r"s^(-n - 1)*gamma(n + 1)"),
    (r"laplace(e^(-2t)*sin(3t), t, s)", r"3/(s^2 + 4s + 13)"),
    (r"laplace(cos(a*t), t, s)", r"s/(a^2 + s^2)"),
    (r"laplace(sinh(a*t), t, s)", r"-a/(a^2 - s^2)"),
    (r"laplace(t*e^t, t, s)", r"1/(s - 1)^2"),
    (r"laplace(t*sin(t), t, s)", r"2s/(s^2 + 1)^2"),
    (r"laplace(heaviside(t-2), t, s)", r"e^(-2s)/s"),
    (r"laplace(diracdelta(t-1), t, s)", r"e^(-s)"),
    (r"laplace(sqrt(t), t, s)", r"sqrt(pi)/(2s^(3/2))"),
    (r"laplace(1/sqrt(t), t, s)", r"sqrt(pi)/sqrt(s)"),
    (r"laplace(besselj(0, t), t, s)", r"1/sqrt(s^2 + 1)"),
    (r"laplace(erf(sqrt(t)), t, s)", r"1/(s*sqrt(s + 1))"),
    (r"invlaplace(1/s, s, t)", r"1"),
    (r"invlaplace(1/(s^2+1), s, t)", r"sin(t)"),
    (r"invlaplace(s/(s^2+1), s, t)", r"cos(t)"),
    (r"invlaplace(1/(s*(s+1)), s, t)", r"1 - e^(-t)"),
    (
        r"invlaplace((s+3)/(s^2+2s+5), s, t)",
        r"e^(-t)*(cos(2t) + sin(2t))",
    ),
    (r"invlaplace(1/s^3, s, t)", r"t^2/2"),
    (r"invlaplace(1/(s-a)^2, s, t)", r"t*e^(a*t)"),
    (r"invlaplace(e^(-2s)/s, s, t)", r"heaviside(t - 2)"),
    (r"invlaplace(1/(s^2*(s+1)), s, t)", r"t + e^(-t) - 1"),
    (r"invlaplace(1/sqrt(s), s, t)", r"1/(sqrt(pi)*sqrt(t))"),
    (r"fourier(e^(-x^2), x, w)", r"sqrt(pi)*e^(-w^2/4)"),
    (r"fourier(e^(-abs(x)), x, w)", r"2/(w^2 + 1)"),
    (r"fourier(1/(1+x^2), x, w)", r"pi*e^(-abs(w))"),
    (r"fourier(diracdelta(x), x, w)", r"1"),
    (r"fourier(diracdelta(x-a), x, w)", r"e^(-i*a*w)"),
    (
        r"fourier(e^(-a*x^2), x, w)",
        r"fourier(e^(-a*x^2), x, w)|note: no closed form found for this fourier",
    ),
    (r"invfourier(e^(-w^2/4), w, x)", r"e^(-x^2)/sqrt(pi)"),
    (r"invfourier(1, w, x)", r"diracdelta(x)"),
    (r"diracdelta(-x)", r"diracdelta(x)"),
    (r"diracdelta(1-x)", r"diracdelta(x - 1)"),
    (r"diracdelta(-2x)", r"diracdelta(2x)"),
    (r"fourier(1, x, w)", r"2pi*diracdelta(w)"),
    (
        r"fourier(cos(x), x, w)",
        r"pi*(diracdelta(w + 1) + diracdelta(w - 1))",
    ),
    (
        r"fourier(sin(3x), x, w)",
        r"-i*pi*(diracdelta(w - 3) - diracdelta(w + 3))",
    ),
    (r"fourier(e^(2i*x), x, w)", r"2pi*diracdelta(w - 2)"),
    (r"invfourier(e^(-3i*w), w, x)", r"diracdelta(x - 3)"),
    (
        r"assume(a, real); fourier(e^(i*a*x), x, w)",
        r"true|2pi*diracdelta(a - w)",
    ),
    (
        r"assume(a > 0); fourier(e^(-a*x^2), x, w)",
        r"true|sqrt(pi)*e^(-w^2/(4a))/sqrt(a)",
    ),
    (
        r"assume(a > 0); fourier(e^(-a*abs(x)), x, w)",
        r"true|2a/(a^2 + w^2)",
    ),
    (
        r"fourier(e^(-2*x^2), x, w)",
        r"sqrt(2)*sqrt(pi)*e^(-w^2/8)/2",
    ),
    (r"fourier(x*e^(-x^2), x, w)", r"-i*sqrt(pi)*w*e^(-w^2/4)/2"),
    (
        r"fourier(x^2*e^(-x^2), x, w)",
        r"-sqrt(pi)*e^(-w^2/4)*(w^2 - 2)/4",
    ),
    (
        r"fourier(e^(-x^2)*cos(x), x, w)",
        r"sqrt(pi)*e^(-(w + 1)^2/4)*(e^w + 1)/2",
    ),
    (
        r"fourier(heaviside(x)*e^(-x), x, w)",
        r"(1 - i*w)/(w^2 + 1)",
    ),
    (r"fourier(sign(x), x, w)", r"-2i/w"),
    (r"fourier(heaviside(x), x, w)", r"pi*diracdelta(w) - i/w"),
    (r"fourier(x, x, w)", r"2i*pi*diff(diracdelta(w), w)"),
    (r"fourier(1/(x^2+4), x, w)", r"pi*e^(-2abs(w))/2"),
    (
        r"invfourier(2/(w^2+1), w, x)",
        r"e^(-x)*heaviside(x) + e^x*heaviside(-x)",
    ),
    (r"invfourier(pi*e^(-abs(w)), w, x)", r"1/(x^2 + 1)"),
    (r"invfourier(2pi*diracdelta(w), w, x)", r"1"),
    (
        r"laplace(t^2*e^(-3t)*cos(t), t, s)",
        r"2(s + 3)*(s^2 + 6s + 6)/(s^2 + 6s + 10)^3",
    ),
    (r"laplace(ln(t), t, s)", r"(-ln(s) - eulergamma)/s"),
    (r"laplace(sin(t)/t, t, s)", r"pi/2 - atan(s)"),
    (r"laplace(t^(3/2), t, s)", r"3sqrt(pi)/(4s^(5/2))"),
    (
        r"laplace(cosh(2t)*sin(t), t, s)",
        r"(s^2 + 5)/(s^4 - 6s^2 + 25)",
    ),
    (r"laplace(heaviside(t-1)*(t-1)^2, t, s)", r"2e^(-s)/s^3"),
    (
        r"invlaplace(e^(-s)/(s^2+1), s, t)",
        r"heaviside(t - 1)*sin(t - 1)",
    ),
    (r"invlaplace(s/(s^2+4)^2, s, t)", r"t*sin(2t)/4"),
    (
        r"invlaplace(1/(s^4-1), s, t)",
        r"-e^(-t)/4 + e^t/4 - sin(t)/2",
    ),
    (
        r"invlaplace(1/(s^2+s+1), s, t)",
        r"2sqrt(3)*e^(-t/2)*sin(sqrt(3)*t/2)/3",
    ),
    (r"invlaplace(atan(1/s), s, t)", r"sin(t)/t"),
    (r"fourierseries(x, x, 3)", r"2sin(x) - sin(2x) + 2sin(3x)/3"),
    (r"fourierseries(x^2, x, 2)", r"pi^2/3 - 4cos(x) + cos(2x)"),
    (
        r"fourierseries(abs(x), x, 3)",
        r"pi/2 - 4cos(x)/pi - 4cos(3x)/(9pi)",
    ),
    (r"fourierseries(sin(x)^2, x, 3)", r"1/2 - cos(2x)/2"),
    (r"ztransform(1, n, z)", r"z/(z - 1)"),
    (r"ztransform(n, n, z)", r"z/(z - 1)^2"),
    (r"ztransform(2^n, n, z)", r"z/(z - 2)"),
    (r"ztransform(n^2, n, z)", r"z*(z + 1)/(z - 1)^3"),
    (r"ztransform(a^n, n, z)", r"-z/(a - z)"),
    (r"invztransform(z/(z-2), z, n)", r"2^n"),
    (r"invztransform(z/(z-1)^2, z, n)", r"n"),
    (r"invztransform(z/(z-1), z, n)", r"1"),
    (r"residue(1/(z^2+1), z, i)", r"-i/2|  ~ -0.5i"),
    (r"residue(e^z/z^3, z, 0)", r"1/2|  ~ 0.5"),
    (r"residue(1/sin(z), z, pi)", r"-1"),
    (r"residue(cot(z), z, 0)", r"1"),
    (r"residue(1/(z^2*(z-1)), z, 0)", r"-1"),
    (r"residue(1/(z^2*(z-1)), z, 1)", r"1"),
    (r"residue(gamma(z), z, -2)", r"1/2|  ~ 0.5"),
    (r"residue(z^2, z, 0)", r"0"),
    (r"pade(e^x, x, 2, 2)", r"(x^2 + 6x + 12)/(x^2 - 6x + 12)"),
    (r"pade(sin(x), x, 3, 2)", r"(20x - 7x^3/3)/(x^2 + 20)"),
    (r"pade(ln(1+x), x, 2, 2)", r"(3x^2 + 6x)/(x^2 + 6x + 6)"),
    (r"pade(1/(1-x), x, 1, 1)", r"-1/(x - 1)"),
    (
        r"seriescoeff(e^x, x, 5)",
        r"1/120|  ~ 0.0083333333333333333333333333333333",
    ),
    (r"seriescoeff(1/(1-x-x^2), x, 10)", r"89"),
    (
        r"seriescoeff(tan(x), x, 7)",
        r"17/315|  ~ 0.053968253968253968253968253968254",
    ),
    (
        r"seriescoeff(sin(x), x, pi, 3)",
        r"1/6|  ~ 0.16666666666666666666666666666667",
    ),
    (
        r"seriescoeff(1/(1-x), x, n)",
        r"error: seriescoeff: the power must be a whole number",
    ),
];

const DIFFERENTIAL_EQUATIONS_AND_SYSTEMS: &[(&str, &str)] = &[
    (
        r"dsolve([x'(t) = y(t), y'(t) = -x(t)], [x, y], t)",
        r"x = C1*cos(t) + C2*sin(t)|y = C2*cos(t) - C1*sin(t)",
    ),
    (
        r"dsolve([x'(t) = 2x(t) + y(t), y'(t) = x(t) + 2y(t)], [x, y], t)",
        r"x = e^t*(C1 - C2 + C1*e^(2t) + C2*e^(2t))/2|y = e^t*(-C1 + C2 + C1*e^(2t) + C2*e^(2t))/2",
    ),
    (
        r"dsolve([x'(t) = y(t), y'(t) = -x(t), x(0) = 1, y(0) = 0], [x, y], t)",
        r"x = cos(t)|y = -sin(t)",
    ),
    (
        r"dsolve([x'(t) = x(t) + y(t), y'(t) = y(t)], [x, y], t)",
        r"x = e^t*(C2*t + C1)|y = C2*e^t",
    ),
    (
        r"dsolve([x'(t) = 3x(t) - y(t) + e^t, y'(t) = x(t) + y(t)], [x, y], t)",
        r"x = e^(2t)*(C1*t - C2*t + C1)|y = e^t*(C1*t*e^t - C2*t*e^t + C2*e^t + 1)",
    ),
    (r"dsolve(y'' + y = 0, y, x)", r"y = C1*cos(x) + C2*sin(x)"),
    (r"dsolve([y' = y, y(0) = 2], y, x)", r"y = 2e^x"),
    (
        r"dsolve(x^2*y'' + x*y' + (x^2 - 4)*y = 0, y, x)",
        r"y = C1*besselj(2, x) + C2*bessely(2, x)",
    ),
    (
        r"dsolve(x^2*y'' + x*y' - (x^2 + 1)*y = 0, y, x)",
        r"y = C1*besseli(1, x) + C2*besselk(1, x)",
    ),
    (
        r"dsolve(y'' - x*y = 0, y, x)",
        r"y = C1*airyai(x) + C2*airybi(x)",
    ),
    (
        r"dsolve(x^2*y'' + x*y' + (x^2 - 1/4)*y = 0, y, x)",
        r"y = sqrt(2)*C1*sin(x)/(sqrt(pi)*sqrt(x)) - sqrt(2)*C2*cos(x)/(sqrt(pi)*sqrt(x))",
    ),
    (
        r"ndsolve([y' = y, y(0) = 1], y, [x, 0, 1])",
        r"y(1) = 2.7182818284590452353602874713527",
    ),
    (
        r"ndsolve([y'' = -y, y(0) = 0, y'(0) = 1], y, [x, 0, pi/2])",
        r"y(pi/2) = 1",
    ),
    (
        r"ndsolve([u' = v, v' = -u, u(0) = 1, v(0) = 0], [u, v], [t, 0, 1])",
        r"u(1) = 0.54030230586813971740093660744298|v(1) = -0.8414709848078965066525023216303",
    ),
    (
        r"ndsolve([y' = x^2 + y^2, y(0) = 0], y, [x, 0, 1])",
        r"y(1) = 0.35023184431675577784938227410474",
    ),
    (
        r"ndsolve([y' = y, y(0) = 1], y, [x, 0, 1, 4])",
        r"[[0.25, 1.2840254166877414840734205680624], [0.5, 1.6487212707001281468486507878142], [0.75, 2.1170000166126746685453698198371], [1, 2.7182818284590452353602874713527]]",
    ),
    (
        r"ndsolve([y' = cos(x)*y, y(0) = 1], y, [x, 0, 10])",
        r"y(10) = 0.58040966204724130577881311863589",
    ),
    (
        r"ndsolve([y'' + y = 0, y(0) = 0, y'(0) = 1], y, [x, 0, 10])",
        r"y(10) = -0.54402111088936981340474766185138",
    ),
    (
        r"ndsolve([y' = -2y, y(0) = 3], y, [x, 0, -1])",
        r"y(-1) = 22.167168296791950681691282381725",
    ),
    (
        r"ndsolve([y' = 1/(x-1), y(0) = 0], y, [x, 0, 2])",
        r"error: the numeric solution does not get past 1: a singular point",
    ),
    (
        r"ndsolve([y' = y, y(0) = 1], y, [x, 1])",
        r"error: ndsolve([y'' + y = 0, y(0) = 0, y'(0) = 1], y, [x, 0, 10]) or ndsolve([...], [x, y], [t, 0, 1, 10]) for 10 steps",
    ),
    (
        r"ndsolve([y' = y], y, [x, 0, 1])",
        r"error: the start value y(0) is missing",
    ),
];

const LATTICES_RELATIONS_AND_FITS: &[(&str, &str)] = &[
    (
        r"latticereduce([[1,1,1],[-1,0,2],[3,5,6]])",
        r"[[0, 1, 0], [1, 0, 1], [-1, 0, 2]]",
    ),
    (
        r"latticereduce([[1,0,0,1345],[0,1,0,35],[0,0,1,154]])",
        r"[[0, 9, -2, 7], [1, 1, -9, -6], [1, -3, -8, 8]]",
    ),
    (r"latticereduce([[2,0],[0,2]])", r"[[2, 0], [0, 2]]"),
    (
        r"lll([[1,1,1],[-1,0,2],[3,5,6]])",
        r"[[0, 1, 0], [1, 0, 1], [-1, 0, 2]]",
    ),
    (r"findrelation([ln(2), ln(3), ln(6)])", r"[1, 1, -1]"),
    (r"findrelation([pi, atan(1), 1])", r"[1, -4, 0]"),
    (
        r"findrelation([sqrt(2)+sqrt(3), sqrt(2), sqrt(3)])",
        r"[1, -1, -1]",
    ),
    (r"findrelation([zeta(2), pi^2])", r"[6, -1]"),
    (
        r"findrelation([pi, e, 1])",
        r"error: no whole-number relation found between these numbers with 128 digits",
    ),
    (
        r"findrelation([sqrt(2), sqrt(3), sqrt(6), 1])",
        r"error: no whole-number relation found between these numbers with 128 digits",
    ),
    (
        r"minpoly(sqrt(2)+sqrt(3), x)",
        r"x^4 - 10x^2 + 1|note: the polynomial was found with numbers and confirmed with twice the digits",
    ),
    (
        r"minpoly(cos(2pi/7), x)",
        r"8x^3 + 4x^2 - 4x - 1|note: the polynomial was found with numbers and confirmed with twice the digits",
    ),
    (
        r"minpoly(2^(1/3) + 1, x)",
        r"x^3 - 3x^2 + 3x - 3|note: the polynomial was found with numbers and confirmed with twice the digits",
    ),
    (
        r"minpoly((1+sqrt(5))/2, x)",
        r"x^2 - x - 1|note: the polynomial was found with numbers and confirmed with twice the digits",
    ),
    (
        r"minpoly(i + sqrt(2), x)",
        r"x^4 - 2x^2 + 9|note: the polynomial was found with numbers and confirmed with twice the digits",
    ),
    (
        r"minpoly(pi, x)",
        r"error: no polynomial of degree 12 or less with whole coefficients has this root",
    ),
    (
        r"identify(3.3302775637731994646559610633735)",
        r"63/20 + sqrt(13)/20|  ~ 3.3302775637731994646559610633735",
    ),
    (
        r"identify(3.3027756377319946465596106337352)",
        r"3/2 + sqrt(13)/2|  ~ 3.3027756377319946465596106337352",
    ),
    (
        r"identify(1.4142135623730950488)",
        r"sqrt(2)|  ~ 1.4142135623730950488016887242097",
    ),
    (
        r"identify(0.7853981633974483096156608458198757)",
        r"pi/4|  ~ 0.7853981633974483096156608458198757",
    ),
    (
        r"identify(4.8104773809653516554730356667038)",
        r"e^(pi/2)|  ~ 4.8104773809653516554730356667038",
    ),
    (
        r"identify(0.6931471805599453)",
        r"ln(2)|  ~ 0.69314718055994530941723212145818",
    ),
    (r"identify(0.5)", r"1/2|  ~ 0.5"),
    (r"identify(1.2345678)", r"6172839/5000000|  ~ 1.2345678"),
    (r"identify(pi)", r"pi|  ~ 3.1415926535897932384626433832795"),
    (
        r"identify(1.2020569031595942853997381615114)",
        r"zeta(3)|  ~ 1.2020569031595942853997381615114",
    ),
    (
        r"identify(1.6449340668482264364724151666460)",
        r"pi^2/6|  ~ 1.644934066848226436472415166646",
    ),
    (
        r"identify(0.3678794411714423215955237701615)",
        r"1/e|  ~ 0.36787944117144232159552377016146",
    ),
    (
        r"identify(2.6651441426902251886502972498731)",
        r"26651441426902251886502972498731/10000000000000000000000000000000|  ~ 2.6651441426902251886502972498731|note: no closed form found for this number",
    ),
    (r"interpolate([[0,1],[1,3],[2,7]], x)", r"x^2 + x + 1"),
    (r"interpolate([[1,1],[2,4],[3,9],[4,16]], x)", r"x^2"),
    (r"interpolate([[0,0],[1,1]], x)", r"x"),
    (
        r"interpolate([[a,b],[c,d]], x)",
        r"(a*d - b*c + b*x - d*x)/(a - c)",
    ),
    (
        r"interpolate([[0,1],[0,2]], x)",
        r"error: two points have the same abscissa 0",
    ),
    (r"fit([[1,2],[2,4.1],[3,5.9]], [1, x], x)", r"1.95x + 0.1"),
    (
        r"fit([[0,1],[1,2.7],[2,7.4]], [1, x, x^2], x)",
        r"1.5x^2 + 0.2x + 1",
    ),
    (r"fit([[0,1],[1,3],[2,5]], [1, x], x)", r"2x + 1"),
    (r"fit([[1,1],[2,4],[3,9]], [x^2], x)", r"x^2"),
    (r"leastsquares([[1,1],[1,2],[1,3]], [1,2,2])", r"[2/3, 1/2]"),
    (r"leastsquares([[1,0],[0,1],[1,1]], [1,1,3])", r"[4/3, 4/3]"),
    (
        r"criticalpoints(x^3 - 3x, x)",
        r"[[-1, 2, maximum], [1, -2, minimum]]",
    ),
    (
        r"criticalpoints(x^4 - 2x^2, x)",
        r"[[-1, -1, minimum], [0, 0, maximum], [1, -1, minimum]]",
    ),
    (
        r"criticalpoints(x^2 + y^2 - 2x, [x, y])",
        r"[[[1, 0], -1, minimum]]",
    ),
    (
        r"criticalpoints(x^2 - y^2, [x, y])",
        r"[[[0, 0], 0, saddle]]",
    ),
    (r"criticalpoints(x^3, x)", r"[[0, 0, degenerate]]"),
    (
        r"nsolve([x^2 + y^2 = 4, x*y = 1], [x, y], [2, 0.5])",
        r"[x -> 1.9318516525781365734994863994578, y -> 0.5176380902050415246977976752481]",
    ),
    (
        r"nsolve([x + y = 3, x - y = 1], [x, y], [0, 0])",
        r"[x -> 2, y -> 1]",
    ),
    (
        r"nsolve(cos(x) = x, x, 1)",
        r"0.73908513321516064165531208767387",
    ),
];

const DIOPHANTINE_AND_MODULAR: &[(&str, &str)] = &[
    (
        r"diophantine(3x + 5y = 1, [x, y])",
        r"[x -> 5t + 2, y -> -3t - 1]|note: t is any whole number",
    ),
    (
        r"diophantine(6x + 9y = 5, [x, y])",
        r"[]|note: no solution: the right side is not a multiple of 3, the greatest common divisor of the coefficients",
    ),
    (
        r"diophantine(6x + 9y = 12, [x, y])",
        r"[x -> 3t - 4, y -> 4 - 2t]|note: t is any whole number",
    ),
    (
        r"diophantine(2x + 3y + 5z = 7, [x, y, z])",
        r"[x -> t1, y -> -4t1 + 5t2 + 14, z -> 2t1 - 3t2 - 7]|note: t1, t2 are any whole numbers",
    ),
    (
        r"diophantine(x^2 + y^2 = 25, [x, y])",
        r"[[-5, 0], [-4, -3], [-4, 3], [-3, -4], [-3, 4], [0, -5], [0, 5], [3, -4], [3, 4], [4, -3], [4, 3], [5, 0]]",
    ),
    (r"diophantine(x^2 + y^2 = 3, [x, y])", r"[]"),
    (
        r"diophantine(x^2 - 2y^2 = 1, [x, y])",
        r"[x -> (3 + 2sqrt(2))^n/2 + (3 - 2sqrt(2))^n/2, y -> sqrt(2)*((3 + 2sqrt(2))^n - (3 - 2sqrt(2))^n)/4]|note: n is any whole number; with every solution both signs changed is one too; the least solution of x^2 - 2 y^2 = 1 is (3, 2)",
    ),
    (
        r"diophantine(x^2 - 61y^2 = 1, [x, y])",
        r"[x -> (1766319049 + 226153980sqrt(61))^n/2 + (1766319049 - 226153980sqrt(61))^n/2, y -> sqrt(61)*((1766319049 + 226153980sqrt(61))^n - (1766319049 - 226153980sqrt(61))^n)/122]|note: n is any whole number; with every solution both signs changed is one too; the least solution of x^2 - 61 y^2 = 1 is (1766319049, 226153980)",
    ),
    (r"diophantine(x^2 - 4y^2 = 1, [x, y])", r"[[-1, 0], [1, 0]]"),
    (
        r"diophantine(x*y = 12, [x, y])",
        r"[[-12, -1], [-6, -2], [-4, -3], [-3, -4], [-2, -6], [-1, -12], [1, 12], [2, 6], [3, 4], [4, 3], [6, 2], [12, 1]]",
    ),
    (r"diophantine(x^2 = 49, [x])", r"[-7, 7]"),
    (
        r"diophantine(x^2 - y^2 = 15, [x, y])",
        r"[[-8, -7], [-8, 7], [-4, -1], [-4, 1], [4, -1], [4, 1], [8, -7], [8, 7]]",
    ),
    (r"modsolve(x^2 = 1, x, 8)", r"[1, 3, 5, 7]"),
    (r"modsolve(3x = 4, x, 7)", r"[6]"),
    (r"modsolve(x^2 + x + 1 = 0, x, 7)", r"[2, 4]"),
    (r"modsolve(x^2 = 2, x, 5)", r"[]"),
    (r"modsolve(6x = 4, x, 8)", r"[2, 6]"),
    (r"modsolve(x^3 = 1, x, 13)", r"[1, 3, 9]"),
    (r"modsolve(2x + 1 = 0, x, 4)", r"[]"),
    (r"modsolve(x^2 = -1, x, 10^9+7)", r"[]"),
    (r"modsolve(x^2 = -1, x, 10^9+9)", r"[430477711, 569522298]"),
    (r"modsolve(x^2 = 2, x, 10^9+7)", r"[59713600, 940286407]"),
    (r"modsolve(x^3 = 1, x, 1000003)", r"[1, 499501, 500501]"),
    (r"modsolve(x^5 - x + 1 = 0, x, 1000003)", r"[]"),
    (r"modsolve(x^2 = 5, x, 2^127-1)", r"[]"),
    (
        r"modsolve(x^2 - 1 = 0, x, (10^9+7)^2)",
        r"[1, 1000000014000000048]",
    ),
    (
        r"modsolve(x^2 = 0, x, (10^9+7)^2)",
        r"error: more than 200000 solutions",
    ),
    (
        r"modsolve(x^4 = 1, x, 100003*100019)",
        r"[1, 3750812520, 6251387537, 10002200056]",
    ),
    (
        r"modsolve((x-1)*(x-2)*(x-3)*(x-4)*(x-5) = 0, x, 1000003)",
        r"[1, 2, 3, 4, 5]",
    ),
    (r"modsolve(3x = 4, x, 10^18+9)", r"[666666666666666674]"),
    (
        r"modsolve(0*x = 0, x, 1000003)",
        r"error: more than 200000 solutions",
    ),
    (
        r"modsolve(x^2+x+1 = 0, x, 1000003*7)",
        r"[499501, 2500507, 4499513, 6500519]",
    ),
    (r"modsolve(x^2 = 1, x, 2^61-1)", r"[1, 2305843009213693950]"),
    (r"modsolve(x^1000 = 1, x, 1000003)", r"[1, 1000002]"),
    (r"modsolve(x^2 = 4, x, 99991)", r"[2, 99989]"),
    (r"modsolve(x^2 = 4, x, 100003)", r"[2, 100001]"),
    (r"length(modsolve(x^1000 = 1, x, 1000003*1000033))", r"16"),
];

const SPECIAL_FUNCTIONS_IN_CLOSED_FORM: &[(&str, &str)] = &[
    (r"besselj(1/2, x)", r"sqrt(2)*sin(x)/(sqrt(pi)*sqrt(x))"),
    (r"besselj(-1/2, x)", r"sqrt(2)*cos(x)/(sqrt(pi)*sqrt(x))"),
    (
        r"besselj(3/2, x)",
        r"sqrt(2)*(sin(x)/x - cos(x))/(sqrt(pi)*sqrt(x))",
    ),
    (r"besselj(0, 0)", r"1"),
    (r"besselj(1, 0)", r"0"),
    (r"besselj(n, 0)", r"besselj(n, 0)"),
    (r"diff(besselj(0, x), x)", r"-besselj(1, x)"),
    (
        r"diff(besselj(n, x), x)",
        r"besselj(n - 1, x)/2 - besselj(n + 1, x)/2",
    ),
    (r"integrate(besselj(1, x), x)", r"-besselj(0, x)"),
    (r"besseli(1/2, x)", r"sqrt(2)*sinh(x)/(sqrt(pi)*sqrt(x))"),
    (r"besselk(1/2, x)", r"sqrt(2)*sqrt(pi)*e^(-x)/(2sqrt(x))"),
    (r"bessely(1/2, x)", r"-sqrt(2)*cos(x)/(sqrt(pi)*sqrt(x))"),
    (r"diff(besseli(0, x), x)", r"besseli(1, x)"),
    (r"diff(besselk(0, x), x)", r"-besselk(1, x)"),
    (r"hankel1(1/2, x)", r"hankel1(1/2, x)"),
    (r"sphericalbesselj(0, x)", r"sin(x)/x"),
    (r"sphericalbesselj(1, x)", r"sin(x)/x^2 - cos(x)/x"),
    (r"sphericalbessely(0, x)", r"-cos(x)/x"),
    (
        r"airyai(0)",
        r"3^(1/3)/(3gamma(2/3))|  ~ 0.35502805388781723926006318600418",
    ),
    (
        r"airybi(0)",
        r"3^(5/6)/(3gamma(2/3))|  ~ 0.61492662744600073515092236909361",
    ),
    (r"diff(airyai(x), x)", r"airyaiprime(x)"),
    (r"diff(airyaiprime(x), x)", r"x*airyai(x)"),
    (r"diff(airybi(x), x)", r"airybiprime(x)"),
    (r"hyp1f1(1, 1, x)", r"e^x"),
    (r"hyp1f1(a, a, x)", r"e^x"),
    (r"hyp1f1(0, b, x)", r"1"),
    (r"hyp2f1(1, 1, 2, x)", r"-ln(1 - x)/x"),
    (r"hyp2f1(a, b, b, x)", r"(1 - x)^(-a)"),
    (r"hyp2f1(0, b, c, x)", r"1"),
    (r"hyp2f1(a, b, c, 0)", r"1"),
    (r"hypergeometric([], [], x)", r"e^x"),
    (r"hypergeometric([1], [], x)", r"1/(1 - x)"),
    (r"hypergeometric([1,1],[2],-x)", r"ln(x + 1)/x"),
    (
        r"diff(hyp2f1(a,b,c,x), x)",
        r"a*b*hyp2f1(a + 1, b + 1, c + 1, x)/c",
    ),
    (r"diff(hyp1f1(a,b,x), x)", r"a*hyp1f1(a + 1, b + 1, x)/b"),
    (r"polylog(1, x)", r"-ln(1 - x)"),
    (r"polylog(0, x)", r"x/(1 - x)"),
    (r"polylog(-1, x)", r"x/(x - 1)^2"),
    (
        r"polylog(2, 1)",
        r"pi^2/6|  ~ 1.644934066848226436472415166646",
    ),
    (
        r"polylog(2, -1)",
        r"-pi^2/12|  ~ -0.82246703342411321823620758332301",
    ),
    (
        r"polylog(2, 1/2)",
        r"pi^2/12 - ln(2)^2/2|  ~ 0.58224052646501250590265632015968",
    ),
    (
        r"polylog(3, -1)",
        r"-3zeta(3)/4|  ~ -0.90154267736969571404980362113359",
    ),
    (
        r"polylog(4, 1)",
        r"pi^4/90|  ~ 1.0823232337111381915160036965412",
    ),
    (r"polylog(-2, x)", r"-x*(x + 1)/(x - 1)^3"),
    (r"li2(0)", r"0"),
    (r"li2(1)", r"pi^2/6|  ~ 1.644934066848226436472415166646"),
    (
        r"li2(-1)",
        r"-pi^2/12|  ~ -0.82246703342411321823620758332301",
    ),
    (
        r"li2(1/2)",
        r"pi^2/12 - ln(2)^2/2|  ~ 0.58224052646501250590265632015968",
    ),
    (
        r"li2(2)",
        r"pi^2/4 - i*pi*ln(2)|  ~ 2.467401100272339654708622749969 - 2.1775860903036021305006888982376i",
    ),
    (r"li2(1/2) - (pi^2/12 - ln(2)^2/2)", r"0"),
    (r"integrate(-ln(1-x)/x, x)", r"li2(x)"),
    (
        r"integrate(ln(1-x)/x, x, 0, 1)",
        r"-pi^2/6|  ~ -1.644934066848226436472415166646",
    ),
    (
        r"integrate(-ln(1-x)/x, x, 0, 1/2)",
        r"pi^2/12 - ln(2)^2/2|  ~ 0.58224052646501250590265632015968",
    ),
    (r"integrate(ln(x)/(1-x), x)", r"li2(1 - x)"),
    (
        r"integrate(ln(x)/(1-x), x, 0, 1)",
        r"-pi^2/6|  ~ -1.644934066848226436472415166646",
    ),
    (
        r"integrate(ln(1+x)/x, x, 0, 1)",
        r"pi^2/12|  ~ 0.82246703342411321823620758332301",
    ),
    (r"integrate(ln(1+x)/x, x)", r"-li2(-x)"),
    (r"integrate(ln(2+3x)/x, x)", r"ln(2)*ln(x) - li2(-3x/2)"),
    (r"integrate(ln(x)/(x+1), x)", r"li2(-x) + ln(x + 1)*ln(x)"),
    (
        r"integrate(ln(x)/(x+1), x, 0, 1)",
        r"-pi^2/12|  ~ -0.82246703342411321823620758332301",
    ),
    (
        r"integrate(ln(x)/(2x+3), x)",
        r"li2(-2x/3)/2 + ln(2x + 3)*ln(x)/2 - ln(3)*ln(x)/2",
    ),
    (r"diff(integrate(ln(x)/(2x+3), x), x) - ln(x)/(2x+3)", r"0"),
    (r"integrate(ln(x)/x, x)", r"ln(x)^2/2"),
    (
        r"polylog(3, 1)",
        r"zeta(3)|  ~ 1.2020569031595942853997381615114",
    ),
    (r"polylog(s, 0)", r"0"),
    (r"diff(polylog(2, x), x)", r"-ln(1 - x)/x"),
    (
        r"zeta(2, 1)",
        r"pi^2/6|  ~ 1.644934066848226436472415166646",
    ),
    (
        r"zeta(2, 2)",
        r"pi^2/6 - 1|  ~ 0.64493406684822643647241516664603",
    ),
    (r"zeta(0, a)", r"1/2 - a"),
    (
        r"zeta(-1)",
        r"-1/12|  ~ -0.083333333333333333333333333333333",
    ),
    (r"zeta(-2)", r"0"),
    (
        r"dirichleteta(1)",
        r"ln(2)|  ~ 0.69314718055994530941723212145818",
    ),
    (
        r"dirichleteta(2)",
        r"pi^2/12|  ~ 0.82246703342411321823620758332301",
    ),
    (r"dirichleteta(0)", r"1/2|  ~ 0.5"),
    (
        r"elliptick(0)",
        r"pi/2|  ~ 1.5707963267948966192313216916398",
    ),
    (
        r"elliptice(0)",
        r"pi/2|  ~ 1.5707963267948966192313216916398",
    ),
    (r"elliptice(1)", r"1"),
    (r"ellipticf(x, 0)", r"x"),
    (r"ellipticf(0, m)", r"0"),
    (r"ellipticpi(0, m)", r"elliptick(m)"),
    (
        r"diff(elliptick(m), m)",
        r"(elliptice(m) - elliptick(m)*(1 - m))/(2m*(1 - m))",
    ),
    (
        r"diff(elliptice(m), m)",
        r"(elliptice(m) - elliptick(m))/(2m)",
    ),
    (r"jacobisn(u, 0)", r"sin(u)"),
    (r"jacobicn(u, 0)", r"cos(u)"),
    (r"jacobidn(u, 0)", r"1"),
    (r"jacobidn(u, 1)", r"1/cosh(u)"),
    (r"jacobisn(u, 1)", r"tanh(u)"),
    (r"jacobisn(0, m)", r"0"),
    (r"diff(jacobisn(u, m), u)", r"jacobicn(u, m)*jacobidn(u, m)"),
    (
        r"diff(jacobicn(u, m), u)",
        r"-jacobidn(u, m)*jacobisn(u, m)",
    ),
    (r"elliptictheta(3, 0, 0)", r"1"),
    (r"ellipticnome(0)", r"0"),
    (r"legendrep(3, x)", r"5x^3/2 - 3x/2"),
    (r"legendrep(2, 1)", r"1"),
    (r"legendrep(n, 1)", r"1"),
    (r"chebyshevu(n, 1)", r"n + 1"),
    (r"laguerrel(n, 0)", r"1"),
    (r"chebyshevt(n, 0)", r"cos(n*pi/2)"),
    (
        r"chebyshevt(1/2, 0)",
        r"sqrt(2)/2|  ~ 0.70710678118654752440084436210485",
    ),
    (r"legendrep(n, x)", r"legendrep(n, x)"),
    (r"legendreq(0, x)", r"atanh(x)"),
    (r"chebyshevt(4, x)", r"8x^4 - 8x^2 + 1"),
    (r"chebyshevt(n, 1)", r"1"),
    (r"chebyshevu(3, x)", r"8x^3 - 4x"),
    (r"hermiteh(3, x)", r"8x^3 - 12x"),
    (r"hermiteh(0, x)", r"1"),
    (r"laguerrel(2, x)", r"x^2/2 - 2x + 1"),
    (
        r"laguerrel(2, a, x)",
        r"a^2/2 - a*x + x^2/2 + 3a/2 - 2x + 1",
    ),
    (r"gegenbauerc(2, a, x)", r"2a^2*x^2 + 2a*x^2 - a"),
    (r"jacobip(1, a, b, x)", r"a*x/2 + b*x/2 + a/2 - b/2 + x"),
    (
        r"sphericalharmonicy(0, 0, t, p)",
        r"1/(2sqrt(pi))|  ~ 0.28209479177387814347403972578039",
    ),
    (
        r"sphericalharmonicy(1, 0, t, p)",
        r"sqrt(3)*cos(t)/(2sqrt(pi))",
    ),
    (
        r"sphericalharmonicy(1, 1, t, p)",
        r"-sqrt(6)*e^(i*p)*sin(t)/(4sqrt(pi))",
    ),
    (
        r"sphericalharmonicy(2, 0, t, p)",
        r"sqrt(5)*(3cos(t)^2 - 1)/(4sqrt(pi))",
    ),
    (
        r"diff(legendrep(n, x), x)",
        r"n*(x*legendrep(n, x) - legendrep(n - 1, x))/(x^2 - 1)",
    ),
    (r"integrate(legendrep(2,x)*legendrep(3,x), x, -1, 1)", r"0"),
    (r"integrate(legendrep(2,x)^2, x, -1, 1)", r"2/5|  ~ 0.4"),
    (
        r"integrate(hermiteh(2,x)*hermiteh(3,x)*e^(-x^2), x, -inf, inf)",
        r"0",
    ),
    (r"uppergamma(1, x)", r"e^(-x)"),
    (r"uppergamma(a, 0)", r"gamma(a)"),
    (r"uppergamma(2, x)", r"e^(-x)*(x + 1)"),
    (r"lowergamma(1, x)", r"1 - e^(-x)"),
    (r"gammaq(1, x)", r"e^(-x)"),
    (r"gammap(1, x)", r"1 - e^(-x)"),
    (r"expint(1, x)", r"expint(1, x)"),
    (r"expint(0, x)", r"e^(-x)/x"),
    (r"betainc(1, 1, 1)", r"1"),
    (r"betaregularized(1, a, b)", r"betaregularized(1, a, b)"),
    (r"erfinv(0)", r"0"),
    (r"erfcinv(1)", r"0"),
    (r"erf(erfinv(x))", r"x"),
    (r"erfc(erfcinv(x))", r"x"),
    (r"erf(erfcinv(x))", r"1 - x"),
    (r"erfinv(erf(x))", r"erfinv(erf(x))"),
    (r"erf(erfinv(1/2))", r"1/2|  ~ 0.5"),
    (r"fresnels(0)", r"0"),
    (r"fresnelc(0)", r"0"),
    (r"diff(fresnels(x), x)", r"sin(pi*x^2/2)"),
    (r"diff(fresnelc(x), x)", r"cos(pi*x^2/2)"),
    (r"sinc(0)", r"1"),
    (r"diff(sinc(x), x)", r"cos(x)/x - sin(x)/x^2"),
    (r"heaviside(2)", r"1"),
    (r"heaviside(-2)", r"0"),
    (r"heaviside(0)", r"1/2|  ~ 0.5"),
    (r"diff(heaviside(x), x)", r"diracdelta(x)"),
    (r"diracdelta(1)", r"0"),
    (r"integrate(diracdelta(x-1)*x^2, x, -inf, inf)", r"1"),
    (r"integrate(diracdelta(x)*cos(x), x, -1, 1)", r"1"),
    (r"integrate(heaviside(x), x, -1, 2)", r"2"),
    (r"struveh(0, 0)", r"0"),
    (r"diff(erfi(x), x)", r"2e^(x^2)/sqrt(pi)"),
    (r"diff(ei(x), x)", r"e^x/x"),
    (r"diff(li(x), x)", r"1/ln(x)"),
    (r"diff(si(x), x)", r"sin(x)/x"),
    (r"diff(ci(x), x)", r"cos(x)/x"),
    (r"diff(shi(x), x)", r"sinh(x)/x"),
    (r"diff(chi(x), x)", r"cosh(x)/x"),
    (r"diff(lambertw(x), x)", r"lambertw(x)/(x + x*lambertw(x))"),
    (r"diff(digamma(x), x)", r"polygamma(1, x)"),
    (r"diff(gamma(x), x)", r"digamma(x)*gamma(x)"),
    (
        r"diff(beta(x, y), x)",
        r"gamma(x)*gamma(y)*(digamma(x) - digamma(x + y))/gamma(x + y)",
    ),
    (
        r"harmonic(10)",
        r"7381/2520|  ~ 2.9289682539682539682539682539683",
    ),
    (
        r"harmonic(n) - harmonic(n-1)",
        r"harmonic(n) - harmonic(n - 1)",
    ),
    (
        r"polygamma(1, 1)",
        r"pi^2/6|  ~ 1.644934066848226436472415166646",
    ),
    (
        r"digamma(1)",
        r"-eulergamma|  ~ -0.5772156649015328606065120900824",
    ),
    (r"lgamma(1)", r"0"),
    (r"factorial2(7)", r"105"),
    (r"binomial(n, 2)", r"binomial(n, 2)"),
    (
        r"beta(2, 3)",
        r"1/12|  ~ 0.083333333333333333333333333333333",
    ),
];

const PRODUCTS_GAMMA_QUOTIENTS_AND_ENDLESS_TOWERS: &[(&str, &str)] = &[
    (r"gamma(n+2)/gamma(n+1)", r"n + 1"),
    (r"gamma(n+3)/gamma(n)", r"n^3 + 3n^2 + 2n"),
    (r"gamma(n+1/2)/gamma(n-1/2)", r"n - 1/2"),
    (r"n!/(n-2)!", r"n^2 - n"),
    (r"factorial(n+1)/factorial(n)", r"n + 1"),
    (r"(n+1)!/(n-1)!", r"n^2 + n"),
    (r"gamma(n)/gamma(n+2)", r"1/(n^2 + n)"),
    (r"n!/gamma(n)", r"n"),
    (r"gamma(x)^2/gamma(x+1)^2", r"1/x^2"),
    (r"gamma(x)*gamma(1-x)", r"pi/sin(pi*x)"),
    (
        r"gamma(1/3)*gamma(2/3)",
        r"2sqrt(3)*pi/3|  ~ 3.6275987284684357011881565152843",
    ),
    (
        r"gamma(1/4)*gamma(3/4)",
        r"sqrt(2)*pi|  ~ 4.4428829381583662470158809900607",
    ),
    (
        r"gamma(1/6)*gamma(5/6)",
        r"2pi|  ~ 6.283185307179586476925286766559",
    ),
    (
        r"gamma(1+i)*gamma(1-i)",
        r"pi/sinh(pi)|  ~ 0.27202905498213316295023658367204",
    ),
    (
        r"gamma(i)*gamma(-i)",
        r"pi/sinh(pi)|  ~ 0.27202905498213316295023658367204",
    ),
    (
        r"gamma(1/2+i)*gamma(1/2-i)",
        r"pi/cosh(pi)|  ~ 0.27101495139941834788656083097138",
    ),
    (
        r"gamma(3/2+2i)*gamma(-1/2-2i)",
        r"-pi/cosh(2pi)|  ~ -0.011733447815317395070140326640539",
    ),
    (r"gamma(x+1)*gamma(1-x)", r"pi*x/sin(pi*x)"),
    (r"gamma(x)*gamma(-x)", r"-pi/(x*sin(pi*x))"),
    (r"gamma(1/2+x)*gamma(1/2-x)", r"pi/cos(pi*x)"),
    (r"gamma(x+1/2)*gamma(3/2-x)", r"pi*(1/2 - x)/cos(pi*x)"),
    (
        r"gamma(2/3)*gamma(4/3)",
        r"2sqrt(3)*pi/9|  ~ 1.2091995761561452337293855050948",
    ),
    (r"gamma(x+3)*gamma(-x-1)", r"pi*(x + 2)/sin(pi*x)"),
    (
        r"gamma(7/2)*gamma(-5/2)",
        r"-pi|  ~ -3.1415926535897932384626433832795",
    ),
    (
        r"gamma(5/2)*gamma(-3/2)",
        r"pi|  ~ 3.1415926535897932384626433832795",
    ),
    (r"1/(gamma(x)*gamma(1-x))", r"sin(pi*x)/pi"),
    (r"gamma(x)^2*gamma(1-x)^2", r"pi^2/sin(pi*x)^2"),
    (
        r"gamma(x)*gamma(1-x)*sin(pi*x)",
        r"pi|  ~ 3.1415926535897932384626433832795",
    ),
    (
        r"beta(1/3, 2/3)",
        r"2sqrt(3)*pi/3|  ~ 3.6275987284684357011881565152843",
    ),
    (r"beta(x, 1-x)", r"pi/sin(pi*x)"),
    (
        r"gamma(1/4)^2",
        r"gamma(1/4)^2|  ~ 13.145047206596874412856136719589",
    ),
    (r"gamma(5)*gamma(-3+x)", r"24gamma(x - 3)"),
    (r"product(k, k, 1, n)", r"n!"),
    (r"product(k, k, m, n)", r"n!/(m - 1)!"),
    (r"product(2k, k, 1, n)", r"2^n*n!"),
    (r"product(a*k, k, 1, n)", r"a^n*n!"),
    (r"product(k^2, k, 1, n)", r"(n!)^2"),
    (r"product(2^k, k, 1, n)", r"2^(n^2/2 + n/2)"),
    (r"product(k*2^k, k, 1, n)", r"2^(n^2/2 + n/2)*n!"),
    (r"product(k+1, k, 1, n)", r"(n + 1)!"),
    (r"product(1/k, k, 1, n)", r"1/n!"),
    (r"product(k/(k+1), k, 1, n)", r"1/(n + 1)"),
    (r"product((k+2)/k, k, 1, n)", r"n^2/2 + 3n/2 + 1"),
    (r"product(1 + 1/k, k, 1, n)", r"n + 1"),
    (r"product(1 - 1/k^2, k, 2, n)", r"(n + 1)/(2n)"),
    (
        r"product((2k-1)/(2k), k, 1, n)",
        r"gamma(n + 1/2)/(sqrt(pi)*n!)",
    ),
    (r"product(x+k, k, 1, n)", r"gamma(n + x + 1)/gamma(x + 1)"),
    (
        r"product(k^2+1, k, 1, n)",
        r"gamma(n + 1 + i)*gamma(n + 1 - i)*sinh(pi)/pi",
    ),
    (r"product(k-3, k, 1, n)", r"product(k - 3, k, 1, n)"),
    (r"product(k-3, k, 4, n)", r"(n - 3)!"),
    (r"product(a, k, 1, n)", r"a^n"),
    (r"product(x, k, 1, 5)", r"x^5"),
    (r"product(k, k, 1, 5)", r"120"),
    (r"product(1 - 1/k^2, k, 2, inf)", r"1/2|  ~ 0.5"),
    (r"product(k/(k+1), k, 1, inf)", r"0"),
    (
        r"product(1 - 1/(4k^2), k, 1, inf)",
        r"2/pi|  ~ 0.63661977236758134307553505349006",
    ),
    (
        r"product(4k^2/(4k^2-1), k, 1, inf)",
        r"pi/2|  ~ 1.5707963267948966192313216916398",
    ),
    (
        r"product(1 - 1/(9k^2), k, 1, inf)",
        r"3sqrt(3)/(2pi)|  ~ 0.82699334313268807426698974746945",
    ),
    (
        r"product(1 - 4/k^2, k, 3, inf)",
        r"1/6|  ~ 0.16666666666666666666666666666667",
    ),
    (
        r"product(1 + 1/k^2, k, 1, inf)",
        r"sinh(pi)/pi|  ~ 3.6760779103749777206956974920283",
    ),
    (
        r"product(1 + 4/k^2, k, 1, inf)",
        r"sinh(2pi)/(2pi)|  ~ 42.61292337424352992695363312163",
    ),
    (
        r"product(1 + 1/k^3, k, 1, inf)",
        r"cosh(sqrt(3)*pi/2)/pi|  ~ 2.4281897920988703287360414361791",
    ),
    (
        r"product((k^2+1)/(k^2+4), k, 1, inf)",
        r"2sinh(pi)/sinh(2pi)|  ~ 0.086266738334054414696554046331337",
    ),
    (r"product(1 - x^2/k^2, k, 1, inf)", r"sin(pi*x)/(pi*x)"),
    (
        r"product(e^(1/k^2), k, 1, inf)",
        r"5.1806683178971157484166262011169",
    ),
    (
        r"product(1 + 1/k, k, 1, inf)",
        r"error: no value found for the product of 1/k + 1 (it may diverge)",
    ),
    (
        r"product(2, k, 1, inf)",
        r"error: no value found for the product of 2 (it may diverge)",
    ),
    (r"tet(sqrt(2), inf)", r"2"),
    (r"sqrt(2)^^inf", r"2"),
    (r"eta^^inf", r"e|  ~ 2.7182818284590452353602874713527"),
    (
        r"towermin^^inf",
        r"1/e|  ~ 0.36787944117144232159552377016146",
    ),
    (r"2^^inf", r"inf"),
    (
        r"(1/2)^^inf",
        r"lambertw(ln(2))/ln(2)|  ~ 0.64118574450498598448620048211482",
    ),
    (r"(1/4)^^inf", r"1/2|  ~ 0.5"),
    (r"(1/100)^^inf", r"undefined"),
    (r"tet(1.2, inf)", r"1.2577345413765264210509828226289"),
    (
        r"i^^inf",
        r"2i*lambertw(-i*pi/2)/pi|  ~ 0.43828293672703211162697516355126 + 0.360592471871385485952940526906i",
    ),
    (
        r"(1+i)^^inf",
        r"-lambertw(-ln(2)/2 - i*pi/4)/(ln(2)/2 + i*pi/4)|  ~ 0.64102647882048916445892350851539 + 0.52362846125716326516257255521972i",
    ),
    (r"1^^inf", r"1"),
    (r"0^^inf", r"undefined"),
    (r"x^^inf", r"x^^inf"),
    (
        r"(3^(1/3))^^inf",
        r"-3lambertw(-ln(3)/3)/ln(3)|  ~ 2.4780526802883024118937365168947",
    ),
    (
        r"ssrt(2, inf)",
        r"sqrt(2)|  ~ 1.4142135623730950488016887242097",
    ),
    (r"ssrt(4, inf)", r"undefined"),
    (r"ssrt(i, inf)", r"undefined"),
    (
        r"x^^inf = 2",
        r"x = sqrt(2)|  ~ 1.4142135623730950488016887242097",
    ),
    (r"x^^inf = 4", r"no solution"),
    (
        r"x^^inf = e",
        r"x = e^(1/e)|  ~ 1.4446678610097661336583391085964",
    ),
    (
        r"x^^inf = 1/e",
        r"x = e^(-e)|  ~ 0.065988035845312537076790187596846",
    ),
    (
        r"x^^inf = 3/2",
        r"x = 2^(1/3)*3^(2/3)/2|  ~ 1.310370697104448303570830640221",
    ),
    (r"x^^inf = 1/2", r"x = 1/4|  ~ 0.25"),
    (r"x^^inf = 1/3", r"no solution"),
    (r"x^^inf = i", r"no solution"),
    (r"sinh(ln(2))", r"3/4|  ~ 0.75"),
    (r"cosh(ln(3))", r"5/3|  ~ 1.6666666666666666666666666666667"),
    (r"tanh(ln(2))", r"3/5|  ~ 0.6"),
    (r"sinh(2ln(2))", r"15/8|  ~ 1.875"),
    (r"sinh(ln(x))", r"x/2 - 1/(2x)"),
    (r"cosh(ln(x))", r"x/2 + 1/(2x)"),
    (r"tanh(ln(x))", r"(x^2 - 1)/(x^2 + 1)"),
    (r"tanh(ln(x)/2)", r"(x - 1)/(x + 1)"),
    (r"coth(ln(x))", r"(x^2 + 1)/(x^2 - 1)"),
    (r"sech(ln(2))", r"4/5|  ~ 0.8"),
    (
        r"cosh(ln(1+sqrt(2)))",
        r"sqrt(2)|  ~ 1.4142135623730950488016887242097",
    ),
    (
        r"sinh(1 + ln(2))",
        r"sinh(1 + ln(2))|  ~ 2.6263119681661846549614065288123",
    ),
];

const SPECIAL_FUNCTIONS_AGAINST_MPMATH_1: &[(&str, &str)] = &[
    (r"airyai((-0.7))", r"0.51100039757501014939201074751941"),
    (
        r"airyai((-2.1+(-1.7)*i))",
        r"1.7238863404206863301900963259645 - 2.2121346787729288632612051262133i",
    ),
    (r"airyai((-48.25))", r"-0.19700365416542031401332752531312"),
    (
        r"airyai((0+(200.5)*i))",
        r"1.173053642942420270038109557483e580 - 5.1076607464643202843635946714054e579i",
    ),
    (
        r"airyai((0.5+(0.8)*i))",
        r"0.18784305976964109191794167870769 - 0.18891819995823327058168447497392i",
    ),
    (
        r"airyai((150.5))",
        r"2.2096950811830475032177011772395e-536",
    ),
    (
        r"airyai((3.5+(400.5)*i))",
        r"-1.2407548357251767072006294150331e1617 - 1.3802735277903817201571217948657e1618i",
    ),
    (
        r"airyaiprime((-0.7))",
        r"-0.14464128564332102740172035753993",
    ),
    (
        r"airyaiprime((-2.1+(-1.7)*i))",
        r"2.5579047892777924703709663017537 + 3.5601065459199119945399344126086i",
    ),
    (
        r"airyaiprime((-48.25))",
        r"0.58073976512292286731006305889727",
    ),
    (
        r"airyaiprime((0+(200.5)*i))",
        r"-1.6858595047223384444049542407446e581 - 6.6296861225982735402682443786449e580i",
    ),
    (
        r"airyaiprime((0.5+(0.8)*i))",
        r"-0.25547482874487249669710774899769 + 0.12677448309908152479494043735026i",
    ),
    (
        r"airyaiprime((150.5))",
        r"-2.7111864215289772561705255042077e-535",
    ),
    (
        r"airyaiprime((3.5+(400.5)*i))",
        r"-1.7682712572629800645022563167871e1619 + 2.1365807647861301701645504133326e1619i",
    ),
    (r"airybi((-0.7))", r"0.27526801198787969865258707702465"),
    (
        r"airybi((-2.1+(-1.7)*i))",
        r"-2.2311778447833267063070375713704 - 1.6949697525965873848461909907179i",
    ),
    (r"airybi((-48.25))", r"-0.083751957850410480660243100437398"),
    (
        r"airybi((0+(200.5)*i))",
        r"5.1076607464643202843635946714054e579 + 1.173053642942420270038109557483e580i",
    ),
    (
        r"airybi((0.5+(0.8)*i))",
        r"0.73800276393560246009718754973691 + 0.34426691931462055916544153634556i",
    ),
    (r"airybi((150.5))", r"5.8711010145795759887247242568722e533"),
    (
        r"airybi((3.5+(400.5)*i))",
        r"1.3802735277903817201571217948657e1618 - 1.2407548357251767072006294150331e1617i",
    ),
    (
        r"airybiprime((-0.7))",
        r"0.54499912006918190665687101013048",
    ),
    (
        r"airybiprime((-2.1+(-1.7)*i))",
        r"3.5948974610288243466473455300719 - 2.5096809787018814031190580268041i",
    ),
    (
        r"airybiprime((-48.25))",
        r"-1.3688669633849530025376476745197",
    ),
    (
        r"airybiprime((0+(200.5)*i))",
        r"6.6296861225982735402682443786449e580 - 1.6858595047223384444049542407446e581i",
    ),
    (
        r"airybiprime((0.5+(0.8)*i))",
        r"0.21300938198449947172590496063183 + 0.24408533242674259513756425586368i",
    ),
    (
        r"airybiprime((150.5))",
        r"7.2015996250967279830175128557587e534",
    ),
    (
        r"airybiprime((3.5+(400.5)*i))",
        r"-2.1365807647861301701645504133326e1619 - 1.7682712572629800645022563167871e1619i",
    ),
    (
        r"besseli((-0.5), (0+(-4.4)*i))",
        r"-0.082662347970357263311460632973644 - 0.082662347970357263311460632973644i",
    ),
    (
        r"besseli((-3), (0.5+(0.8)*i))",
        r"-0.017048681595750384546829404445103 + 0.00093449828546497530165615728983174i",
    ),
    (
        r"besseli((0), (0+(-4.4)*i))",
        r"-0.34225679000388561443890465489298",
    ),
    (
        r"besseli((0.5), (-2.5+(-500.5)*i))",
        r"-0.04541708850473086951247731949639 + 0.21303119060281284451763211148985i",
    ),
    (
        r"besseli((1), (20.5+(-17.5)*i))",
        r"-7447886.7675423119847577852337826 + 60359513.916227790505528925217833i",
    ),
    (
        r"besseli((1.5), (-11.2))",
        r"-7939.2851591751831405751510835762i",
    ),
    (
        r"besseli((2.5), (-48.25))",
        r"48594571985542080698.814410966097i",
    ),
    (
        r"besselj((-0.5), (0+(-4.4)*i))",
        r"10.955442094897829997747505365466 + 10.955442094897829997747505365466i",
    ),
    (
        r"besselj((-3), (0.5+(0.8)*i))",
        r"0.017712886273804324716868329468144 - 0.0027632907835374958843355922770826i",
    ),
    (
        r"besselj((0), (0+(-4.4)*i))",
        r"16.010435524946996723557906664704",
    ),
    (
        r"besselj((0.5), (-2.5+(-500.5)*i))",
        r"-4.0854792963531805921982053045925e215 + 5.8118453879909405948512782274966e214i",
    ),
    (
        r"besselj((1), (20.5+(-17.5)*i))",
        r"2857618.9215990702081894673161401 - 1016000.928961244330167862419979i",
    ),
    (
        r"besselj((1.5), (-11.2))",
        r"0.069242810701832303665498386868885i",
    ),
    (
        r"besselj((2.5), (-48.25))",
        r"0.10663341924307070031872844020387i",
    ),
    (
        r"besselk((-0.5), (0+(-4.4)*i))",
        r"0.27219865504671868580610864040633 - 0.53189008015887621991894274233518i",
    ),
    (
        r"besselk((-3), (0.5+(0.8)*i))",
        r"-9.9574618844784359784349349272223 - 0.017468826425568677798014918446037i",
    ),
    (
        r"besselk((0), (0+(-4.4)*i))",
        r"0.25656831580457943961055244465513 - 0.5376157085587158141679370747893i",
    ),
    (
        r"besselk((0.5), (-2.5+(-500.5)*i))",
        r"0.13817409918604970846509847500759 - 0.66834871895710686453575087593179i",
    ),
    (
        r"besselk((1), (20.5+(-17.5)*i))",
        r"1.679929276867869604594719346951e-10 - 2.5455707465574199625815378509222e-10i",
    ),
    (
        r"besselk((1.5), (-11.2))",
        r"-24941.999925241021735723641523652i",
    ),
    (
        r"besselk((2.5), (-48.25))",
        r"-152664350354119372928.78736945463i",
    ),
    (
        r"bessely((-0.5), (0+(-4.4)*i))",
        r"10.95213989769618001393489411035 - 10.95213989769618001393489411035i",
    ),
    (
        r"bessely((-3), (0.5+(0.8)*i))",
        r"-5.6455532590907602447496709670015 - 1.1306188577981327861519487712836i",
    ),
    (
        r"bessely((0), (0+(-4.4)*i))",
        r"-0.0045512651776405215685526543541217 - 16.010435524946996723557906664704i",
    ),
    (
        r"bessely((0.5), (-2.5+(-500.5)*i))",
        r"5.8118453879909405948512782274966e214 + 4.0854792963531805921982053045925e215i",
    ),
    (
        r"bessely((1), (20.5+(-17.5)*i))",
        r"-1016000.9289612477055943777656211 - 2857618.9215990682662605648257804i",
    ),
    (
        r"bessely((1.5), (-11.2))",
        r"0.22912795038950893212435077152556i",
    ),
    (
        r"bessely((2.5), (-48.25))",
        r"0.042901402682606887492827154864309i",
    ),
    (
        r"betainc((-0.5), (0.5), (-0.5))",
        r"1.1547005383792515290182975610039i",
    ),
    (
        r"betainc((-1), (1), (2.5))",
        r"-1.8627416997969520780827019587355",
    ),
    (
        r"betainc((0+(1)*i), (1.5), (-0.5))",
        r"-0.50012348749683786901523137090258 + 0.025203054549039146071702768269474i",
    ),
    (
        r"betainc((0.5), (3.25), (2))",
        r"0.019976046063719463126305016969341",
    ),
    (
        r"betainc((0.9+(0.1)*i), (1.5), (0.5))",
        r"0.88254885795333862284175388880707 + 0.27109178752856090578531122645659i",
    ),
    (
        r"betainc((1.0001), (3.25), (2.5))",
        r"0.043013931170889183493613226355015 - 4.0006428883930702076586549200308e-11i",
    ),
    (
        r"betainc((1.5), (0.5), (2.5))",
        r"1.1780972450961724644234912687298 - 0.06084650945458694235253079486402i",
    ),
    (
        r"betaregularized((0.1), (0.5), (0.5))",
        r"0.20483276469913345164919784755052",
    ),
    (
        r"betaregularized((0.1), (2), (0.5))",
        r"0.0038825370469605104203485335036937",
    ),
    (
        r"betaregularized((0.3+(0.4)*i), (1), (0.5))",
        r"0.13217923082301868510189557603678 + 0.2304623340481638934890134820885i",
    ),
    (
        r"betaregularized((0.3+(0.4)*i), (3.25), (0.5))",
        r"-0.034570819875480925848513457043483 - 0.0022012510835554456256150399247654i",
    ),
    (
        r"betaregularized((0.5), (0.5), (2.5))",
        r"0.9244131815783875620503567023267",
    ),
    (
        r"betaregularized((0.5), (3.25), (0.5))",
        r"0.040598368388116080862367665921606",
    ),
    (
        r"betaregularized((0.9), (1), (0.5))",
        r"0.68377223398316206680011064555673",
    ),
];

const SPECIAL_FUNCTIONS_AGAINST_MPMATH_2: &[(&str, &str)] = &[
    (
        r"chebyshevt((-0.3), (-0.7))",
        r"0.76235101445429822117810966505855",
    ),
    (
        r"chebyshevt((-2.5), (-2.5))",
        r"25.114736709748720756147971951835i",
    ),
    (
        r"chebyshevt((0.5), (0))",
        r"0.70710678118654752440084436210485",
    ),
    (
        r"chebyshevt((0.5+(1)*i), (0.3))",
        r"1.5434923494000774106111409076952 - 0.96582290256690890361542951376672i",
    ),
    (r"chebyshevt((10), (0.95))", r"-0.99942165995"),
    (r"chebyshevt((2.5), (1))", r"1"),
    (r"chebyshevt((3), (1.5))", r"9"),
    (
        r"chebyshevu((-0.3), (-0.7))",
        r"1.3966983802836207364997552155948",
    ),
    (
        r"chebyshevu((-2.5), (-2.5))",
        r"-2.3094010767585030580365951220078i",
    ),
    (
        r"chebyshevu((0.5), (0))",
        r"0.70710678118654752440084436210485",
    ),
    (
        r"chebyshevu((0.5+(1)*i), (0.3))",
        r"1.8996828915693260438290965017787 - 0.55189880146680508778024543643812i",
    ),
    (r"chebyshevu((10), (0.95))", r"-1.1028799099"),
    (r"chebyshevu((2.5), (1))", r"3.5"),
    (r"chebyshevu((3), (1.5))", r"21"),
    (r"dirichleteta((-1))", r"0.25"),
    (
        r"dirichleteta((-2.5))",
        r"-0.087841120721362842395232450051557",
    ),
    (r"dirichleteta((0))", r"0.5"),
    (
        r"dirichleteta((0.5))",
        r"0.60489864342163037024726591423596",
    ),
    (r"dirichleteta((1))", r"0.69314718055994530941723212145818"),
    (
        r"dirichleteta((1+(1)*i))",
        r"0.72655977506246326320149572854724 + 0.15809586390120732435542628554432i",
    ),
    (r"dirichleteta((2))", r"0.82246703342411321823620758332301"),
    (r"elliptice((-0.5))", r"1.7517712756948178620265018389845"),
    (
        r"elliptice((-2.5+(0.75)*i), (0.9))",
        r"-1.448467344346614822881944619419 + 0.66597123477659554092718116605824i",
    ),
    (
        r"elliptice((0+(0.9)*i), (-3.5))",
        r"-0.38563595876521335632366016345976 + 0.40622216546550161367426955411293i",
    ),
    (
        r"elliptice((0.3), (25.5))",
        r"0.15630587076551634120247265258996 + 0.071497491477740968675772976637331i",
    ),
    (r"elliptice((1.1+(-0.6)*i), (0))", r"1.1 - 0.6i"),
    (
        r"elliptice((1.2), (3.5))",
        r"0.43670380060644718236369986528883 + 0.61859764486548442245337297293434i",
    ),
    (
        r"elliptice((2.5), (0.5+(-0.85)*i))",
        r"2.2511414843381290319622475317116 + 0.72308710615265831043647314330209i",
    ),
    (
        r"ellipticf((-0.8), (-0.5))",
        r"-0.76639593485329275758767290533561",
    ),
    (
        r"ellipticf((-2.5+(0.75)*i), (0+(1)*i))",
        r"-2.2780664182738775419538031595103 + 0.070895726354138222022383897428245i",
    ),
    (
        r"ellipticf((-7.5), (1.2+(0.3)*i))",
        r"-9.2412086438052142860982438181343 - 4.3009949278687421436915369866414i",
    ),
    (
        r"ellipticf((0.3), (0.5))",
        r"0.30225466857501761840492419512755",
    ),
    (
        r"ellipticf((1.1+(-0.6)*i), (-5.5+(4.5)*i))",
        r"0.79337663825161668181147298073342 - 0.090123861598396056599979041281705i",
    ),
    (
        r"ellipticf((1.2), (25.5))",
        r"0.31418283245947452484238108573208 - 0.52241106328109269791657840096293i",
    ),
    (
        r"ellipticf((2.5), (0.1))",
        r"2.5791682943801087480489378715131",
    ),
    (r"elliptick((-0.5))", r"1.4157372084259561988921659654236"),
    (r"elliptick((-1))", r"1.3110287771460599052324197949456"),
    (r"elliptick((0))", r"1.5707963267948966192313216916398"),
    (r"elliptick((0.1))", r"1.6124413487202193982299163630854"),
    (
        r"elliptick((0.5+(-0.85)*i))",
        r"1.5496843519475142395971652425844 - 0.41110272286722925237904828913864i",
    ),
    (r"elliptick((0.99))", r"3.6956373629898746778099541952626"),
    (
        r"elliptick((1.5))",
        r"1.6566381702365941664484683729331 - 1.4157372084259561988921659654236i",
    ),
    (
        r"ellipticnome((-0.5))",
        r"-0.02531991336628518686662697111068",
    ),
    (
        r"ellipticnome((-0.6+(0.7)*i))",
        r"-0.034883728505427129043888126036643 + 0.025673256213686702243792527420003i",
    ),
    (
        r"ellipticnome((-3.5))",
        r"-0.092926517038166385538752692888428",
    ),
    (
        r"ellipticnome((0.1))",
        r"0.0065846515538583702744730596706544",
    ),
    (
        r"ellipticnome((0.5))",
        r"0.043213918263772249774417737171728",
    ),
    (
        r"ellipticnome((0.5+(-0.85)*i))",
        r"0.00088644042333972055562305177629701 - 0.065311855586565999394453254099053i",
    ),
    (r"ellipticnome((0.9))", r"0.1401731269542615524091054546102"),
    (
        r"ellipticpi((-0.5), (-0.5))",
        r"1.167847517129878574684334559323",
    ),
    (
        r"ellipticpi((-2.5), (-1))",
        r"0.73887771552126029343614106396938",
    ),
    (
        r"ellipticpi((0), (-3.5))",
        r"1.0415607119696200478517739952254",
    ),
    (
        r"ellipticpi((0.3), (-7.5), (0))",
        r"-8.8901783660438901873461109668928",
    ),
    (
        r"ellipticpi((0.3+(0.4)*i), (-7.5), (1.3+(-0.4)*i))",
        r"-11.279396715979924782719308760947 + 2.6344771392895385580236176822626i",
    ),
    (
        r"ellipticpi((0.5+(-1.5)*i), (0+(1)*i))",
        r"1.1385392981017401907453617454266 - 0.50303029820190804023537093823118i",
    ),
    (
        r"ellipticpi((0.9), (0+(0.9)*i), (-5.5+(4.5)*i))",
        r"0.28959036385860147200905258486858 + 0.53272838551030534974465917508834i",
    ),
    (
        r"elliptictheta((1), (-0.8), (-0.3))",
        r"-0.68660629028971399198645042155567 - 0.68660629028971399198645042155567i",
    ),
    (
        r"elliptictheta((1), (0.4+(0.3)*i), (0.5))",
        r"0.17865900330128882346811853416913 + 0.29358018984563537498199116298325i",
    ),
    (
        r"elliptictheta((2), (-0.8), (0.5))",
        r"0.84482344429384970803858698276586",
    ),
    (
        r"elliptictheta((2), (1.2), (0+(0.7)*i))",
        r"1.1517312494122832659637839538837 + 0.47706270371547748485375806741476i",
    ),
    (
        r"elliptictheta((3), (0+(0.9)*i), (-0.4+(0.5)*i))",
        r"-6.3136836171819846314927964234149 + 8.5633280326521015653259997424947i",
    ),
    (
        r"elliptictheta((3), (1.2), (0.9))",
        r"0.0000063325315689698691291027990336076",
    ),
    (
        r"elliptictheta((4), (0.3), (0.5))",
        r"0.22082416809403158431531747950602",
    ),
    (
        r"erfcinv((0.0000000001))",
        r"4.5728249673894852787410436731407",
    ),
    (r"erfcinv((0.001))", r"2.3267537655135246705602202009842"),
    (r"erfcinv((0.1))", r"1.1630871536766740867262542605629"),
    (r"erfcinv((0.5))", r"0.47693627620446987338141835364313"),
    (r"erfcinv((1.5))", r"-0.47693627620446987338141835364313"),
    (r"erfcinv((1.999))", r"-2.3267537655135246705602202009842"),
    (r"erfinv((-0.5))", r"-0.47693627620446987338141835364313"),
];

const SPECIAL_FUNCTIONS_AGAINST_MPMATH_3: &[(&str, &str)] = &[
    (r"erfinv((-0.99999))", r"-3.1234132743408750302472925681804"),
    (
        r"erfinv((0.00001))",
        r"0.0000088622692547595938030382475277907",
    ),
    (r"erfinv((0.1))", r"0.088855990494257687015737250567792"),
    (r"erfinv((0.5))", r"0.47693627620446987338141835364313"),
    (r"erfinv((0.9))", r"1.1630871536766740867262542605629"),
    (r"erfinv((0.999))", r"2.3267537655135246705602202009842"),
    (
        r"expint((-1), (-0.7))",
        r"1.2329098209002917479333975848468",
    ),
    (
        r"expint((-1), (55.5))",
        r"1.4458350074472788566569943458062e-26",
    ),
    (
        r"expint((0), (55.5))",
        r"1.4202450073154686114064280741991e-26",
    ),
    (
        r"expint((0.5), (4.2))",
        r"0.0032451699020521312422991685445899",
    ),
    (
        r"expint((1), (55.5))",
        r"1.3955306227669237300371213112768e-26",
    ),
    (
        r"expint((1+(1)*i), (4.2))",
        r"0.002891753703785384756792605649593 - 0.00049993510470806480569149786301512i",
    ),
    (
        r"expint((2), (40.5+(30.5)*i))",
        r"4.7172944611929214903668653569765e-20 + 1.422032835498560157880881190065e-20i",
    ),
    (r"fresnelc((-0.7))", r"-0.65965235190451039091616930626994"),
    (
        r"fresnelc((-2.1+(-1.7)*i))",
        r"-392.78161858935498152609096580644 + 4573.6873152618713386828751837885i",
    ),
    (
        r"fresnelc((-48.25))",
        r"-0.50064573079661591422727259866922",
    ),
    (
        r"fresnelc((0+(60.5)*i))",
        r"0.50201299739959060068183804141129i",
    ),
    (r"fresnelc((0.3))", r"0.29940097605204721038191383185205"),
    (
        r"fresnelc((20.5+(-17.5)*i))",
        r"-1.1318596314569412881665798665353e487 + 1.3260777284991551698107732869999e487i",
    ),
    (r"fresnelc((4.2))", r"0.54171920316341528151747635114754"),
    (r"fresnels((-0.7))", r"-0.17213645786347745335589208939427"),
    (
        r"fresnels((-2.1+(-1.7)*i))",
        r"-4574.1873154576924099349889629812 - 392.28161707637399269409353586582i",
    ),
    (r"fresnels((-48.25))", r"-0.4934345826905256760444586787688"),
    (
        r"fresnels((0+(60.5)*i))",
        r"-0.49513899875344637292762351394622i",
    ),
    (r"fresnels((0.3))", r"0.01411699800657658580731856630369"),
    (
        r"fresnels((20.5+(-17.5)*i))",
        r"1.3260777284991551698107732869999e487 + 1.1318596314569412881665798665353e487i",
    ),
    (r"fresnels((4.2))", r"0.56319888839661130347762122195946"),
    (
        r"gammap((0.5), (-0.7))",
        r"1.2195045923706355012144381341171i",
    ),
    (
        r"gammap((0.5), (20.5+(-17.5)*i))",
        r"0.99999999992846115044032675443639 + 1.1327653492988969380327568366765e-10i",
    ),
    (
        r"gammap((0.5+(1)*i), (12.5))",
        r"1.0000017851735839045295751620966 + 7.7829635329393454668914288420706e-7i",
    ),
    (
        r"gammap((1), (-9.5+(3.25)*i))",
        r"13282.300905696339763181284681242 - 1445.4574416207645528961762979736i",
    ),
    (r"gammap((1), (4.2))", r"0.98500442317952229378801563977127"),
    (
        r"gammap((2.5), (0.002+(-0.003)*i))",
        r"-1.8137866086081015518587811270393e-7 - 1.4871026454683971890092486569027e-7i",
    ),
    (
        r"gammap((4.75), (-2.1+(-1.7)*i))",
        r"-5.7746021308425184535773576617788 + 5.8867697628592687389449375287616i",
    ),
    (
        r"gammaq((0.5), (-0.7))",
        r"1 - 1.2195045923706355012144381341171i",
    ),
    (
        r"gammaq((0.5), (20.5+(-17.5)*i))",
        r"7.1538849559673245563606452262838e-11 - 1.1327653492988969380327568366765e-10i",
    ),
    (
        r"gammaq((0.5+(1)*i), (12.5))",
        r"-0.0000017851735839045295751620966187621 - 7.7829635329393454668914288420706e-7i",
    ),
    (
        r"gammaq((1), (-9.5+(3.25)*i))",
        r"-13281.300905696339763181284681242 + 1445.4574416207645528961762979736i",
    ),
    (
        r"gammaq((1), (4.2))",
        r"0.014995576820477706211984360228729",
    ),
    (
        r"gammaq((2.5), (0.002+(-0.003)*i))",
        r"1.0000001813786608608101551858781 + 1.4871026454683971890092486569027e-7i",
    ),
    (
        r"gammaq((4.75), (-2.1+(-1.7)*i))",
        r"6.7746021308425184535773576617788 - 5.8867697628592687389449375287616i",
    ),
    (
        r"gegenbauerc((-0.3), (-0.25), (-0.7))",
        r"0.069912643643383428889489963111043",
    ),
    (r"gegenbauerc((-2.5), (0.5), (1))", r"1"),
    (
        r"gegenbauerc((0.5), (2.25), (1.5))",
        r"2.8124608736802323237253263407182",
    ),
    (
        r"gegenbauerc((0.5+(1)*i), (2.25), (6.5))",
        r"-7.0831806436906761346919670521585 - 0.84131958739006370762023084668713i",
    ),
    (r"gegenbauerc((10), (1), (1.5))", r"17711"),
    (r"gegenbauerc((3), (-0.25), (-2.5))", r"5.8984375"),
    (r"gegenbauerc((3), (2.25), (6.5))", r"11284.7109375"),
    (
        r"hankel1((-0.5), (0+(-4.4)*i))",
        r"21.907581992594010011682399475816 + 21.907581992594010011682399475816i",
    ),
    (
        r"hankel1((-3), (0.5+(0.8)*i))",
        r"1.1483317440719371108688171007518 - 5.6483165498742977406340065592786i",
    ),
    (
        r"hankel1((0), (0+(-4.4)*i))",
        r"32.020871049893993447115813329408 - 0.0045512651776405215685526543541217i",
    ),
    (
        r"hankel1((0.5), (-2.5+(-500.5)*i))",
        r"-8.170958592706361184396410609185e215 + 1.1623690775981881189702556454993e215i",
    ),
    (
        r"hankel1((1), (20.5+(-17.5)*i))",
        r"5715237.8431981384744500321419205 - 2032001.8579224920357622401856001i",
    ),
    (
        r"hankel1((1.5), (-11.2))",
        r"-0.22912795038950893212435077152556 + 0.069242810701832303665498386868885i",
    ),
    (
        r"hankel1((2.5), (-48.25))",
        r"-0.042901402682606887492827154864309 + 0.10663341924307070031872844020387i",
    ),
    (
        r"hankel2((-0.5), (0+(-4.4)*i))",
        r"0.0033021972016499838126112551154415 + 0.0033021972016499838126112551154415i",
    ),
    (
        r"hankel2((-3), (0.5+(0.8)*i))",
        r"-1.1129059715243284614350804418155 + 5.6427899683072227488653353747244i",
    ),
    (
        r"hankel2((0), (0+(-4.4)*i))",
        r"0.0045512651776405215685526543541217i",
    ),
    (
        r"hankel2((0.5), (-2.5+(-500.5)*i))",
        r"2.2467249311808313729102315195708e-220 - 1.5246933297603455526057861259038e-219i",
    ),
    (
        r"hankel2((1), (20.5+(-17.5)*i))",
        r"1.9419289024903596606578118619658e-9 + 3.3754265153456420895310620412495e-9i",
    ),
    (
        r"hankel2((1.5), (-11.2))",
        r"0.22912795038950893212435077152556 + 0.069242810701832303665498386868885i",
    ),
    (
        r"hankel2((2.5), (-48.25))",
        r"0.042901402682606887492827154864309 + 0.10663341924307070031872844020387i",
    ),
    (
        r"hermiteh((-0.3), (-0.7))",
        r"1.6311679257257645566661946531867",
    ),
    (
        r"hermiteh((-2.5), (-2.5))",
        r"2813.5091229361845424120896950235",
    ),
    (
        r"hermiteh((0.5), (-2.5+(0.75)*i))",
        r"39.470994616186501494010711353843 + 0.10433053415730201658165940544409i",
    ),
    (
        r"hermiteh((0.5+(1)*i), (-9.5))",
        r"1.0147520758438991689701896024675e38 + 1.7845321655056416117056920811175e38i",
    ),
    (r"hermiteh((10), (1))", r"8224"),
    (
        r"hermiteh((2.5), (1.1+(-0.6)*i))",
        r"-2.6869927752508990356774610017817 - 8.0258536963014560263873874772652i",
    ),
    (r"hermiteh((3), (1.5))", r"9"),
    (
        r"hyp0f1((-0.5), (-11.2))",
        r"3.5856694833816344546455161291854",
    ),
];

const SPECIAL_FUNCTIONS_AGAINST_MPMATH_4: &[(&str, &str)] = &[
    (
        r"hyp0f1((0.25), (-2.1+(-1.7)*i))",
        r"-3.6217739196182803967032791941531 + 0.87708029750256108919423110597888i",
    ),
    (
        r"hyp0f1((0.5), (0.002+(-0.003)*i))",
        r"1.0039966625770222566664327426123 - 0.0060080007992379267654072816262655i",
    ),
    (
        r"hyp0f1((1), (-2.1+(-1.7)*i))",
        r"-0.54677774695121221764404608030436 - 0.36358082620776824191549587196545i",
    ),
    (
        r"hyp0f1((1+(-1)*i), (-2.1+(-1.7)*i))",
        r"0.085330347530813793415621311807816 - 1.2146071533700942711323233791828i",
    ),
    (
        r"hyp0f1((1.5), (12.5))",
        r"83.255018326089616307345864464528",
    ),
    (
        r"hyp0f1((3), (-2.1+(-1.7)*i))",
        r"0.38299451939656713257602284530454 - 0.31537990039901707148332347640156i",
    ),
    (
        r"hyp1f1((-0.5), (-0.5), (-48.25))",
        r"1.1099189036438807004830616461537e-21",
    ),
    (
        r"hyp1f1((-2.5), (-0.5), (0.002+(-0.003)*i))",
        r"1.0100374616617087697207658122508 - 0.014910007494997867583467155842235i",
    ),
    (
        r"hyp1f1((0.25), (-0.5), (12.5))",
        r"-1664147.4207075161362705174320614",
    ),
    (
        r"hyp1f1((0.5), (1+(-1)*i), (0+(-4.4)*i))",
        r"-0.10543816609036153305518944056892 - 1.2609201335644701820964003220621i",
    ),
    (
        r"hyp1f1((1), (1+(-1)*i), (3.1+(-2.6)*i))",
        r"14.356592601056104043532152073141 - 18.026345002799923567902006510073i",
    ),
    (
        r"hyp1f1((1.5), (3), (-0.7))",
        r"0.71553384356437690053848319102281",
    ),
    (
        r"hyp1f1((3), (1), (-2.1+(-1.7)*i))",
        r"0.059141995455510176227448480919231 + 0.2936209507205647120151127778478i",
    ),
    (
        r"hyp2f1((-0.5), (-0.5), (0.5), (8.5+(-9.5)*i))",
        r"2.3712077791457778605134999709285 - 4.8885396149851039711884095971966i",
    ),
    (
        r"hyp2f1((-2.5), (0.5), (1), (1.3+(-0.4)*i))",
        r"0.29401226709302189383035687834673 + 0.01720714614183091101704047060861i",
    ),
    (
        r"hyp2f1((0.5), (-2.5), (4.75), (0.99))",
        r"0.78664487490269213039972465980513",
    ),
    (
        r"hyp2f1((0.5+(1)*i), (3), (0.5), (0+(-2.5)*i))",
        r"-4.4970758518605100848342544403177 + 4.0983247360693754015919263736495i",
    ),
    (
        r"hyp2f1((1+(-1)*i), (0.5), (0.25), (8.5+(-9.5)*i))",
        r"-0.1759565628445450424233240408137 + 0.12060488802887530158109568879267i",
    ),
    (
        r"hyp2f1((1.5), (4.75), (2), (0.99))",
        r"178998427.53488441064869253019361",
    ),
    (
        r"hyp2f1((3), (0.5), (4.75), (1.01+(0.001)*i))",
        r"2.0437488921571033605512155312497 + 0.045750522562554137203383480712569i",
    ),
    (
        r"hypergeometric([(-2), (1.5)], [(0.75), (1.25)], (0+(1.9)*i))",
        r"-2.6673015873015873015873015873016 - 6.08i",
    ),
    (
        r"hypergeometric([(-2), (1.5)], [(3), (1.25)], (0.3))",
        r"0.77",
    ),
    (
        r"hypergeometric([(-2), (2)], [(3), (1.25)], (-1.2+(2.3)*i))",
        r"1.5955555555555555555555555555556 - 3.4346666666666666666666666666667i",
    ),
    (
        r"hypergeometric([(0.5), (1.5)], [(3), (1.25)], (-2.1+(-1.7)*i))",
        r"0.67157818801476528825977875281924 - 0.14724120843613773730039389512411i",
    ),
    (
        r"hypergeometric([(0.5), (2)], [(0.75), (1.25)], (0.3))",
        r"1.3815351548916300454968949327273",
    ),
    (
        r"hypergeometric([(1), (1.5)], [(0.75), (1.25)], (-9.5+(3.25)*i))",
        r"-0.019347032401087324840148771834709 - 0.01047389269430825403819622548138i",
    ),
    (
        r"hypergeometric([(1), (1.5)], [(3), (1.25)], (-2.1+(-1.7)*i))",
        r"0.40732392474134372114059720401811 - 0.21000467852151677499696211693216i",
    ),
    (
        r"hypergeometric([(0.5), (1.5), (-3)], [(0.75), (1.25)], (-0.5+(0.6)*i))",
        r"1.6390143190143190143190143190143 - 2.985974025974025974025974025974i",
    ),
    (
        r"hypergeometric([(0.5), (1.5), (-3)], [(3), (1.25)], (-0.5+(0.6)*i))",
        r"1.2538782051282051282051282051282 - 0.5205i",
    ),
    (
        r"hypergeometric([(0.5), (1.5), (2)], [(0.75), (1.25)], (-0.7))",
        r"0.45624476363420843478322883136241",
    ),
    (
        r"hypergeometric([(0.5), (1.5), (2)], [(3), (1.25)], (0.3))",
        r"1.1488324890535095027988882667709",
    ),
    (
        r"hypergeometric([(1), (1.5), (-3)], [(0.75), (1.25)], (0.3))",
        r"0.044123876123876123876123876123876",
    ),
    (
        r"hypergeometric([(1), (1.5), (-3)], [(3), (1.25)], (0.3+(0.4)*i))",
        r"0.61013333333333333333333333333333 - 0.32631794871794871794871794871795i",
    ),
    (
        r"hypergeometric([(1), (1.5), (2)], [(0.75), (1.25)], (0.9))",
        r"506.93019644895419313378298657182",
    ),
    (
        r"hyperu((-0.5), (-0.5), (55.5))",
        r"7.5163588219385290645390483930983",
    ),
    (r"hyperu((-2), (1.5), (40.5+(30.5)*i))", r"511.25 + 2318i"),
    (
        r"hyperu((-2.5), (2), (3.1+(-2.6)*i))",
        r"-12.246111899286758068691573146066 + 17.220534152157791468128641412462i",
    ),
    (
        r"hyperu((0.5), (2), (0.3))",
        r"2.7883195950732136766974518659888",
    ),
    (
        r"hyperu((1+(-1)*i), (-1), (-0.7))",
        r"0.66709291939317683798205808784881 + 0.061319499222871339575533642428101i",
    ),
    (
        r"hyperu((1.5), (2), (30.5))",
        r"0.005798996101994328445898218142835",
    ),
    (
        r"hyperu((3), (-2.5), (-9.5+(3.25)*i))",
        r"0.0047053171927555651648883884529115 + 0.00016294614919623192690458044759531i",
    ),
    (
        r"jacobicn((-0.8), (-0.5))",
        r"0.66908385114429149530347043011593",
    ),
    (
        r"jacobicn((-2.5+(0.75)*i), (0.1))",
        r"-0.95038023205386713737167289114825 + 0.538517230348103914166156634668i",
    ),
    (
        r"jacobicn((0+(0.9)*i), (0.9))",
        r"1.5881752390722298804955425012913",
    ),
    (
        r"jacobicn((0.3), (1.5))",
        r"0.95726269902598487840312566864535",
    ),
    (
        r"jacobicn((1.1+(-0.6)*i), (-0.6+(0.7)*i))",
        r"0.78822373761207818273086633421932 + 0.75764075276685681718103879444338i",
    ),
    (
        r"jacobicn((1.2), (0.3+(0.4)*i))",
        r"0.42354106376037989916313992463624 + 0.078242152155719310279401343000297i",
    ),
    (
        r"jacobicn((2.5), (0.99))",
        r"0.14915512819773536042199190786111",
    ),
    (
        r"jacobidn((-0.8), (-0.5))",
        r"1.1296740238090640411631431089576",
    ),
    (
        r"jacobidn((-2.5+(0.75)*i), (0.1))",
        r"0.98185499854113345392184549437191 - 0.052125429019934436989275575304134i",
    ),
    (
        r"jacobidn((0+(0.9)*i), (0.9))",
        r"1.5395033390681298170910180594475",
    ),
    (
        r"jacobidn((0.3), (1.5))",
        r"0.93516191775529972443832088274223",
    ),
    (
        r"jacobidn((1.1+(-0.6)*i), (-0.6+(0.7)*i))",
        r"1.0729593232663168944436847328262 - 0.64472831164118742548984888826958i",
    ),
    (
        r"jacobidn((1.2), (0.3+(0.4)*i))",
        r"0.87026251274447846405263712417292 - 0.17857293489520406513045419687294i",
    ),
    (
        r"jacobidn((2.5), (0.99))",
        r"0.17895468628959131375499286549724",
    ),
    (
        r"jacobip((-0.3), (0), (-0.5), (1.1+(-0.6)*i))",
        r"0.995173891081932323637223765205 + 0.017025705676793466730659114676512i",
    ),
    (r"jacobip((-2.5), (0), (-0.5), (1.5))", r"2.5234375"),
    (
        r"jacobip((0.5), (0.5), (1.5), (1.1+(-0.6)*i))",
        r"1.3967349456018420942494534227245 - 0.41556210986465178852714324946997i",
    ),
    (
        r"jacobip((0.5+(1)*i), (0.5), (1.5), (0.3))",
        r"0.98228663874369301971256892954322 - 1.3168183800160122968241187813814i",
    ),
    (
        r"jacobip((2.5), (0), (0), (-2.5))",
        r"-0.0026469327699178030889106689603704 + 17.550419919448576465313358843475i",
    ),
    (
        r"jacobip((3), (0), (-0.5), (-2.5+(0.75)*i))",
        r"-15.062744140625 + 21.9100341796875i",
    ),
    (r"jacobip((3), (2), (1.5), (6.5))", r"3520.7080078125"),
    (
        r"jacobisn((-0.8), (-0.5))",
        r"-0.74318692139859645157285539413",
    ),
    (
        r"jacobisn((-2.5+(0.75)*i), (0.1))",
        r"-0.86052538130507259978790750244151 - 0.59474844259334561885252882290297i",
    ),
    (
        r"jacobisn((0+(0.9)*i), (0.9))",
        r"1.2338154602703495611774987692461i",
    ),
    (
        r"jacobisn((0.3), (1.5))",
        r"0.28921985591153088971313959061679",
    ),
    (
        r"jacobisn((1.1+(-0.6)*i), (-0.6+(0.7)*i))",
        r"1.1136741879163425035575119026433 - 0.53623441433122303262720630955688i",
    ),
    (
        r"jacobisn((1.2), (0.3+(0.4)*i))",
        r"0.90997857422250206932927576558252 - 0.036417081999154830504933958425696i",
    ),
    (
        r"jacobisn((2.5), (0.99))",
        r"0.98881380842518432640676794723362",
    ),
    (
        r"laguerrel((-0.3), (-0.5), (-0.7))",
        r"1.36488011598665941258880031024",
    ),
];

const SPECIAL_FUNCTIONS_AGAINST_MPMATH_5: &[(&str, &str)] = &[
    (
        r"laguerrel((-2.5), (0), (1))",
        r"7.2794797198301424175157049556993",
    ),
    (
        r"laguerrel((0.5), (2), (1.5))",
        r"1.3545034653832010583327690354219",
    ),
    (
        r"laguerrel((0.5+(1)*i), (2), (6.5))",
        r"-9.850744785855242376720732016454 - 6.8603793319061626491961528042888i",
    ),
    (
        r"laguerrel((10), (0.5), (1.5))",
        r"0.97541643415178571428571428571429",
    ),
    (
        r"laguerrel((3), (-0.5), (-2.5))",
        r"15.416666666666666666666666666667",
    ),
    (
        r"laguerrel((3), (2), (6.5))",
        r"4.8541666666666666666666666666667",
    ),
    (
        r"legendrep((-0.3), (-0.7))",
        r"1.4310762216964090821533349657617",
    ),
    (
        r"legendrep((-2.5), (-2.5))",
        r"0.015204469100885964050740422050525 - 4.6079403921451440061994128546835i",
    ),
    (
        r"legendrep((0.5), (0))",
        r"0.53935260118837935666793572235555",
    ),
    (
        r"legendrep((0.5+(1)*i), (0.3))",
        r"0.92855320107440114349016650363658 - 0.89019628153281157821208657665693i",
    ),
    (
        r"legendrep((10), (0.95))",
        r"-0.3548802948408550262451171875",
    ),
    (r"legendrep((2.5), (1))", r"1"),
    (r"legendrep((3), (1.5))", r"6.1875"),
    (
        r"lowergamma((0.25), (-0.7))",
        r"3.0328546159610994464906813202712 + 3.0328546159610994464906813202712i",
    ),
    (
        r"lowergamma((0.5), (-9.5+(3.25)*i))",
        r"350.26769232835149083700511977677 - 4430.7607623039522159488291098671i",
    ),
    (
        r"lowergamma((0.5+(1)*i), (20.5+(-17.5)*i))",
        r"0.30069461757911574712335650095548 - 0.42496787977976890940938375429617i",
    ),
    (
        r"lowergamma((1+(-1)*i), (0+(1.9)*i))",
        r"3.3787386316270368165142250170925 + 1.7227698189394390878375908873852i",
    ),
    (
        r"lowergamma((1.5), (0.5+(0.8)*i))",
        r"0.21581604691578067985408690771649 + 0.39053050299619494917311254564458i",
    ),
    (
        r"lowergamma((2), (4.2))",
        r"0.92202300053351592769768132681061",
    ),
    (
        r"lowergamma((4.75), (0.3))",
        r"0.00054003993925012075750839514288184",
    ),
    (
        r"polylog((-0.5), (-0.5))",
        r"-0.28301281074650602317460047408289",
    ),
    (
        r"polylog((-1), (25.5))",
        r"0.042482299042065805914202415660142",
    ),
    (r"polylog((0), (0.99))", r"99"),
    (
        r"polylog((1), (0.5))",
        r"0.69314718055994530941723212145818",
    ),
    (
        r"polylog((1.5), (0+(-2.5)*i))",
        r"-0.91374882292436776678921571480816 - 1.5478088858195605897714051040343i",
    ),
    (
        r"polylog((2+(-3)*i), (-3.5))",
        r"-3.1766030981277115306250954166138 + 1.7890889657198947109008886997403i",
    ),
    (
        r"polylog((3), (-0.6+(0.7)*i))",
        r"-0.59994572561277392699979625295007 + 0.61075240922784263059712945625712i",
    ),
    (
        r"sphericalbesselj((-1.5), (-0.7))",
        r"-0.49283460534965412383610929853685i",
    ),
    (
        r"sphericalbesselj((-1.5), (30.5))",
        r"0.032564519117765034839193506427443",
    ),
    (
        r"sphericalbesselj((0), (20.5+(-17.5)*i))",
        r"521933.49284949257952972407994243 + 522835.95518297919091518005680678i",
    ),
    (
        r"sphericalbesselj((0.5), (0.002+(-0.003)*i))",
        r"0.033176054046581059885868492274741 - 0.017755221278923336463455960449467i",
    ),
    (
        r"sphericalbesselj((1), (-3.3))",
        r"-0.28475092254876085720652857585279",
    ),
    (
        r"sphericalbesselj((1), (4.2))",
        r"0.067319709592820304202648038768432",
    ),
    (
        r"sphericalbesselj((2), (1.5))",
        r"0.12734928368840821564724094723019",
    ),
    (
        r"sphericalbessely((0), (-0.7))",
        r"1.0926316961206977517940857002741",
    ),
    (
        r"sphericalbessely((0), (20.5+(-17.5)*i))",
        r"522835.955182978644354884433491 - 521933.49284949182511106334154762i",
    ),
    (
        r"sphericalbessely((0.5), (12.5))",
        r"-0.054534242043296928585407179387196",
    ),
    (
        r"sphericalbessely((1), (-9.5+(3.25)*i))",
        r"0.60047021743044931001353201103349 - 1.0927891597486122447196415841071i",
    ),
    (
        r"sphericalbessely((1), (4.2))",
        r"0.2353106046189211694860299712355",
    ),
    (
        r"sphericalbessely((2), (0.002+(-0.003)*i))",
        r"62812849.794925238003887771505923 - 12289601.046507112158369885548315i",
    ),
    (
        r"sphericalbessely((5), (-2.1+(-1.7)*i))",
        r"2.2163550473736971886487803588991 - 1.3413443417873983121850172060138i",
    ),
    (
        r"struveh((-0.5), (0+(-4.4)*i))",
        r"10.95213989769618001393489411035 - 10.95213989769618001393489411035i",
    ),
    (
        r"struveh((-3), (0.5+(0.8)*i))",
        r"-0.74157423424116328890605532508726 - 1.8912063845694130288169469793636i",
    ),
    (
        r"struveh((0), (0+(-4.4)*i))",
        r"-15.856423126982456552622552866293i",
    ),
    (
        r"struveh((0.5), (-2.5+(-500.5)*i))",
        r"5.8118453879909405948512782274966e214 + 4.0854792963531805921982053045925e215i",
    ),
    (
        r"struveh((1), (20.5+(-17.5)*i))",
        r"-1016000.2922005682110160498646444 - 2857618.9207348263229033456678477i",
    ),
    (
        r"struveh((1.5), (-11.2))",
        r"1.5855311157916008809631512532982i",
    ),
    (
        r"struveh((2.5), (-48.25))",
        r"-33.441575669595035896423262987728i",
    ),
    (
        r"struvel((-0.5), (0+(-4.4)*i))",
        r"-0.25594939378496110962291227407703 + 0.25594939378496110962291227407703i",
    ),
    (
        r"struvel((-3), (0.5+(0.8)*i))",
        r"-1.1719049709838545795542132475292 - 1.8987680933280909324241872795123i",
    ),
    (
        r"struvel((0), (0+(-4.4)*i))",
        r"0.024279788356651105001496950652163i",
    ),
    (
        r"struvel((0.5), (-2.5+(-500.5)*i))",
        r"0.017391774835360109374173706516574 - 0.23773428461580759837773396001869i",
    ),
    (
        r"struvel((1), (20.5+(-17.5)*i))",
        r"-7447887.404028052253859613404185 + 60359513.917094271249153901303954i",
    ),
    (
        r"struvel((1.5), (-11.2))",
        r"7937.9713263181392671528699726133i",
    ),
    (
        r"struvel((2.5), (-48.25))",
        r"-48594571985542080665.444799868648i",
    ),
    (
        r"uppergamma((-0.5), (-0.7))",
        r"-3.5449077018110320545963349666823 - 0.49075847429802636905408480318271i",
    ),
    (
        r"uppergamma((-1), (7.5+(6.5)*i))",
        r"1.3441427365685486415701699395246e-7 - 0.0000048662274765538789238566478183014i",
    ),
    (
        r"uppergamma((-2.5), (4.2))",
        r"0.000056795447232868086151254767824017",
    ),
    (
        r"uppergamma((0.5), (-3.3))",
        r"1.7724538509055160272981674833411 - 18.581911120727161003701814560852i",
    ),
    (
        r"uppergamma((1), (-9.5+(3.25)*i))",
        r"-13281.300905696339763181284681242 + 1445.4574416207645528961762979736i",
    ),
    (
        r"uppergamma((1.5), (-33.5+(-41.5)*i))",
        r"-2304269208417167.1249017941726142 + 1136612551979874.7326338519975889i",
    ),
    (
        r"uppergamma((3), (0+(1.9)*i))",
        r"4.1164365358624155661298187732921 + 0.2950427870954243218088644334271i",
    ),
    (
        r"zeta((-1), (0.25))",
        r"0.010416666666666666666666666666667",
    ),
    (
        r"zeta((-2.5), (1+(1)*i))",
        r"0.30072752212648779532566750095775 + 0.4063933773036421103749371066044i",
    ),
    (
        r"zeta((-7.5), (3.5))",
        r"-985.98524504958712913659430469339",
    ),
    (
        r"zeta((0.5), (0.5+(-2.5)*i))",
        r"-2.2322707963509233644259645358432 + 2.2322706456492118845271891486655i",
    ),
    (
        r"zeta((0.5+(14.5)*i), (25.75))",
        r"-0.094138373246476582331151838295877 + 0.3382976676386524419518883253525i",
    ),
    (
        r"zeta((2+(1)*i), (0.5))",
        r"3.5074988482018306761645130726647 + 2.0313988089339878255198260755764i",
    ),
    (
        r"zeta((3), (10.5))",
        r"0.0049875721238575846426529597285471",
    ),
];

const TOWERS_AND_WHOLE_STEPS: &[(&str, &str)] = &[
    (
        r"iterate(x^2+1, x, 12)",
        r"((((((((((x^4 + 2x^2 + 2)^2 + 1)^2 + 1)^2 + 1)^2 + 1)^2 + 1)^2 + 1)^2 + 1)^2 + 1)^2 + 1)^2 + 1",
    ),
    (
        r"iterate(x^2+1, x, 5)",
        r"(((x^4 + 2x^2 + 2)^2 + 1)^2 + 1)^2 + 1",
    ),
    (r"iterate(x^2+c, x, 4)", r"c + (c + (c + (x^2 + c)^2)^2)^2"),
    (
        r"iterate(x^3+x, x, 3)",
        r"x^3 + x + (x^3 + x + (x^3 + x)^3)^3 + (x^3 + x)^3",
    ),
    (
        r"iterate(sqrt(x+1), x, 6)",
        r"sqrt(sqrt(sqrt(sqrt(sqrt(sqrt(x + 1) + 1) + 1) + 1) + 1) + 1)",
    ),
    (
        r"iterate(e^x-1, x, 6)",
        r"e^(e^(e^(e^(e^(e^x - 1) - 1) - 1) - 1) - 1) - 1",
    ),
    (
        r"iterate(1/(1+x), x, 20)",
        r"(4181x + 6765)/(6765x + 10946)",
    ),
    (r"iterate(2x+1, x, 20)", r"1048576x + 1048575"),
    (r"iterate(x^2+1, x, 3, 1)", r"26"),
    (
        r"iterate(x^2+1, x, 12, 1)",
        r"4.2738617653645554487425669417955e724",
    ),
    (
        r"iterate(x^2+1, x, 30, 1)",
        r"6.3326959742865570597041426882266e189957621",
    ),
    (
        r"iterate(x^2+1, x, 100, 1)",
        r"error: the iterate is too large for a number, or the map has no value on the way",
    ),
    (r"iterate(x^2-2, x, 3, 3)", r"2207"),
    (r"iterate(x^2+1, x, -2, 5)", r"1"),
    (
        r"iterate(sin(x), x, 3, 1)",
        r"sin(sin(sin(1)))|  ~ 0.67843047736074022897916012648039",
    ),
    (
        r"iterate(sin(x), x, 100, 1)",
        r"0.16885248872798112003378368717892",
    ),
    (
        r"iterate(sin(x), x, -1, 1/2)",
        r"pi/6|  ~ 0.52359877559829887307710723054658",
    ),
    (
        r"iterate(cos(x), x, 1000, 1)",
        r"0.73908513321516064165531208767387",
    ),
    (
        r"iterate(2x+1, x, 100, 1)",
        r"2535301200456458802993406410751",
    ),
    (
        r"iterate(2x+1, x, 1000, 1)",
        r"2.14301721437253464189685009812e301",
    ),
    (
        r"iterate(x/2, x, 1000, 1)",
        r"9.3326361850321887899008954472382e-302",
    ),
    (
        r"iterate(4x*(1-x), x, 5, 3/10)",
        r"2047637583328478647296/23283064365386962890625|  ~ 0.08794536454456290615570638831616",
    ),
    (
        r"iterate(4x*(1-x), x, 200, 3/10)",
        r"0.87627570890335016476282619708722",
    ),
    (
        r"iterate(x/2+x^2/4, x, 100, 0.3)",
        r"3.2516802160622435686030857962617e-31",
    ),
    (
        r"iterate(x/2+x^2/4, x, -100, iterate(x/2+x^2/4, x, 100, 0.3))",
        r"0.3",
    ),
    (
        r"iterate(e^x-1, x, 30, 1)",
        r"error: the iterate is too large for a number, or the map has no value on the way",
    ),
    (r"iterate(e^x-1, x, 200, iterate(e^x-1, x, -200, 1))", r"1"),
    (
        r"iterate(x^2-2x+2, x, -1, 0.6)",
        r"1 + 0.63245553203367586639977870888654i",
    ),
    (
        r"iterate(8x^4-8x^2+1, x, -1, 2)",
        r"cosh(acosh(2)/4)|  ~ 1.0546906824732048065659303083325",
    ),
    (r"iterate(x/2+x^2/4, x, -200000, 1/2)", r"2"),
    (
        r"diff(iterate(x^2+1, x, 5), x)",
        r"8(((x^4 + 2x^2 + 2)^2 + 1)^2 + 1)*((x^4 + 2x^2 + 2)^2 + 1)*(4x^3 + 4x)*(x^4 + 2x^2 + 2)",
    ),
    (
        r"expand(iterate(x^2+1, x, 4))",
        r"x^16 + 8x^14 + 32x^12 + 80x^10 + 138x^8 + 168x^6 + 144x^4 + 80x^2 + 26",
    ),
    (r"iterate(x^2+1, x, 40) - iterate(x^2+1, x, 40)", r"0"),
    (r"(x+1)^100", r"(x + 1)^100"),
    (r"(x^100-1)/(x-1)", r"(x^100 - 1)/(x - 1)"),
    (
        r"expand((x+1)^20)",
        r"x^20 + 20x^19 + 190x^18 + 1140x^17 + 4845x^16 + 15504x^15 + 38760x^14 + 77520x^13 + 125970x^12 + 167960x^11 + 184756x^10 + 167960x^9 + 125970x^8 + 77520x^7 + 38760x^6 + 15504x^5 + 4845x^4 + 1140x^3 + 190x^2 + 20x + 1",
    ),
    (
        r"factor(x^64-1)",
        r"(x + 1)*(x - 1)*(x^16 + 1)*(x^2 + 1)*(x^32 + 1)*(x^4 + 1)*(x^8 + 1)",
    ),
    (
        r"e^(e^(e^(e^(e^(e^(e^(e^(e^(e^(e^(e^(e^x-1)-1)-1)-1)-1)-1)-1)-1)-1)-1)-1)-1)-1",
        r"e^(e^(e^(e^(e^(e^(e^(e^(e^(e^(e^(e^(e^x - 1) - 1) - 1) - 1) - 1) - 1) - 1) - 1) - 1) - 1) - 1) - 1) - 1",
    ),
    (
        r"e^(-210000.5 + 1070000.5i) - 1",
        r"-1 - 6.4224656915184844031338613761594e-91203i",
    ),
    (
        r"N(e^(10^-100000+i))",
        r"0.54030230586813971740093660744298 + 0.8414709848078965066525023216303i",
    ),
    (
        r"N(e^(1+10^-100000*i))",
        r"2.7182818284590452353602874713527 + 2.7182818284590452353602874713527e-100000i",
    ),
    (r"(-2.0)^3", r"-8"),
    (r"(-2.5)^2", r"6.25"),
    (r"(-2.0)^-2", r"0.25"),
    (r"(-1.5)^(1/2)", r"1.2247448713915890490986420373529i"),
    (r"(-8.0)^(1/3)", r"1 + 1.7320508075688772935274463415059i"),
    (r"(-1.0)^100001", r"-1"),
    (r"(1.0+i)^8", r"16"),
    (r"(-0.5)^-3", r"-8"),
    (r"asin[-1](x)", r"sin(x)"),
    (r"acos[-1](x)", r"cos(x)"),
    (
        r"erf[-1](1/2)",
        r"erfinv(1/2)|  ~ 0.47693627620446987338141835364313",
    ),
];

#[test]
fn lists() {
    check(LISTS);
}

#[test]
fn programs_and_logic() {
    check(PROGRAMS_AND_LOGIC);
}

#[test]
fn statistics() {
    check(STATISTICS);
}

#[test]
fn distributions() {
    check(DISTRIBUTIONS);
}

#[test]
fn transforms() {
    check(TRANSFORMS);
}

#[test]
fn differential_equations_and_systems() {
    check(DIFFERENTIAL_EQUATIONS_AND_SYSTEMS);
}

#[test]
fn lattices_relations_and_fits() {
    check(LATTICES_RELATIONS_AND_FITS);
}

#[test]
fn diophantine_and_modular() {
    check(DIOPHANTINE_AND_MODULAR);
}

#[test]
fn special_functions_in_closed_form() {
    check(SPECIAL_FUNCTIONS_IN_CLOSED_FORM);
}

#[test]
fn products_gamma_quotients_and_endless_towers() {
    check(PRODUCTS_GAMMA_QUOTIENTS_AND_ENDLESS_TOWERS);
}

#[test]
fn special_functions_against_mpmath_1() {
    check_with(&["-n"], SPECIAL_FUNCTIONS_AGAINST_MPMATH_1);
}

#[test]
fn special_functions_against_mpmath_2() {
    check_with(&["-n"], SPECIAL_FUNCTIONS_AGAINST_MPMATH_2);
}

#[test]
fn special_functions_against_mpmath_3() {
    check_with(&["-n"], SPECIAL_FUNCTIONS_AGAINST_MPMATH_3);
}

#[test]
fn special_functions_against_mpmath_4() {
    check_with(&["-n"], SPECIAL_FUNCTIONS_AGAINST_MPMATH_4);
}

#[test]
fn special_functions_against_mpmath_5() {
    check_with(&["-n"], SPECIAL_FUNCTIONS_AGAINST_MPMATH_5);
}

#[test]
fn towers_and_whole_steps() {
    check(TOWERS_AND_WHOLE_STEPS);
}
