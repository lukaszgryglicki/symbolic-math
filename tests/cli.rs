//! Runs the program on fixed inputs and compares what it prints.

use std::process::{Command, Stdio};

fn program() -> Command {
    Command::new(env!("CARGO_BIN_EXE_symbolic-math"))
}

/// Standard output and standard error with `|` for line ends, and whether the program succeeded.
fn run(args: &[&str]) -> (String, String, bool) {
    let out = program()
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("symbolic-math runs");
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).trim_end().replace('\n', "|");
    (text(&out.stdout), text(&out.stderr), out.status.success())
}

fn check(cases: &[(&str, &str)]) {
    check_with(&[], cases);
}

fn check_with(options: &[&str], cases: &[(&str, &str)]) {
    let mut failures = Vec::new();
    for (input, want) in cases {
        let mut args = options.to_vec();
        args.push(input);
        let (out, err, _) = run(&args);
        let got = [out, err]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("|");
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

const ALGEBRA_AND_EQUATIONS: &[(&str, &str)] = &[
    (r"10*x-(2+3)*x", r"5x"),
    (r"i^2", r"-1"),
    (
        r"4*(x^2)+2x+2=0",
        r"x = -1/4 - i*sqrt(7)/4|  ~ -0.25 - 0.66143782776614764762540393840982i|x = -1/4 + i*sqrt(7)/4|  ~ -0.25 + 0.66143782776614764762540393840982i",
    ),
    (r"sin(x)=0", r"x = k*pi, k is an integer"),
    (
        r"sqrt(8)",
        r"2sqrt(2)|  ~ 2.8284271247461900976033774484194",
    ),
    (r"1/(1+i)", r"1/2 - i/2|  ~ 0.5 - 0.5i"),
    (r"e^(i*pi)", r"-1"),
    (
        r"cos(pi/5)",
        r"1/4 + sqrt(5)/4|  ~ 0.80901699437494742410229341718282",
    ),
    (r"0.5x+0.25x", r"0.75x"),
    (r"phi^2-phi-1", r"0"),
    (r"ln(8)/ln(2)", r"3"),
    (r"2^^4", r"65536"),
    (
        r"N(pi, 50)",
        r"3.1415926535897932384626433832795028841971693993751",
    ),
    (r"sin(x)^2+cos(x)^2", r"1"),
    (r"(x^2-1)/(x-1)", r"x + 1"),
    (r"(1+2i)*(3-4i)", r"11 + 2i"),
    (r"(1+i)^10", r"32i"),
    (r"sqrt(-4)", r"2i"),
    (r"abs(3+4i)", r"5"),
    (r"conj(2+3i)", r"2 - 3i"),
    (r"re((1+2i)^2)", r"-3"),
    (r"arg(-1)", r"pi|  ~ 3.1415926535897932384626433832795"),
    (
        r"e^(i*pi/3)",
        r"(-1)^(1/3)|  ~ 0.5 + 0.86602540378443864676372317075294i",
    ),
    (
        r"sqrt(i)",
        r"(-1)^(1/4)|  ~ 0.70710678118654752440084436210485 + 0.70710678118654752440084436210485i",
    ),
    (
        r"(-8)^(1/3)",
        r"2(-1)^(1/3)|  ~ 1 + 1.7320508075688772935274463415059i",
    ),
    (r"1/3+1/6", r"1/2|  ~ 0.5"),
    (r"2^100", r"1267650600228229401496703205376"),
    (
        r"100!",
        r"93326215443944152681699238856266700490715968264381621468592963895217599993229915608941463976156518286253697920827223758251185210916864000000000000000000000000",
    ),
    (r"1.5+2.25", r"3.75"),
    (r"1/3.0", r"0.33333333333333333333333333333333"),
    (r"0.1+0.2", r"0.3"),
    (
        r"123456789.123456789123456789123456789123456789*2",
        r"246913578.246913578246913578246913578246913578",
    ),
    (r"sqrt(2)*sqrt(8)", r"4"),
    (
        r"sqrt(12)+sqrt(27)",
        r"5sqrt(3)|  ~ 8.6602540378443864676372317075294",
    ),
    (r"(sqrt(2)+1)*(sqrt(2)-1)", r"1"),
    (
        r"1/(sqrt(2)+1)",
        r"sqrt(2) - 1|  ~ 0.4142135623730950488016887242097",
    ),
    (
        r"2^(1/2)*2^(1/3)",
        r"2^(5/6)|  ~ 1.781797436280678609480452411181",
    ),
    (r"ln(e^5)", r"5"),
    (r"e^(ln(7))", r"7"),
    (r"ln(1)", r"0"),
    (r"sin(pi/6)", r"1/2|  ~ 0.5"),
    (r"cos(pi/3)", r"1/2|  ~ 0.5"),
    (r"tan(pi/4)", r"1"),
    (
        r"sin(pi/12)",
        r"sqrt(6)/4 - sqrt(2)/4|  ~ 0.25881904510252076234889883762405",
    ),
    (r"asin(1/2)", r"pi/6|  ~ 0.52359877559829887307710723054658"),
    (r"atan(1)", r"pi/4|  ~ 0.78539816339744830961566084581988"),
    (r"acos(-1)", r"pi|  ~ 3.1415926535897932384626433832795"),
    (r"sinh(0)", r"0"),
    (r"x+x+x", r"3x"),
    (r"x*x*x", r"x^3"),
    (r"(x+1)^2-(x^2+2x+1)", r"0"),
    (r"(a+b)^3", r"(a + b)^3"),
    (r"expand((a+b)^3)", r"a^3 + 3a^2*b + 3a*b^2 + b^3"),
    (r"expand((x+1)*(x-1)*(x^2+1))", r"x^4 - 1"),
    (r"factor(x^4-1)", r"(x + 1)*(x - 1)*(x^2 + 1)"),
    (r"factor(x^2+2x+1)", r"(x + 1)^2"),
    (r"factor(360)", r"2^3*3^2*5"),
    (r"factor(2x+2)", r"2(x + 1)"),
    (r"factor(x^3-1)", r"(x - 1)*(x^2 + x + 1)"),
    (r"factor(x^2-2)", r"x^2 - 2"),
    (r"factor(6x^2+5x-6)", r"(2x + 3)*(3x - 2)"),
    (r"together(1/x+1/y)", r"(x + y)/(x*y)"),
    (r"cancel((x^2-1)/(x+1))", r"x - 1"),
    (r"(x^2-4)/(x-2)", r"x + 2"),
    (r"x/x", r"1"),
    (r"x^2/x", r"x"),
    (r"(x^3-1)/(x-1)", r"x^2 + x + 1"),
    (r"tan(x)*cos(x)", r"sin(x)"),
    (r"1-cos(x)^2", r"sin(x)^2"),
    (r"(sin(x)+cos(x))^2", r"2cos(x)*sin(x) + 1"),
    (r"sin(2x)/(2cos(x))", r"sin(2x)/(2cos(x))"),
    (r"e^x*e^y", r"e^(x + y)"),
    (r"e^(2x)/e^x", r"e^x"),
    (r"ln(x)+ln(2)", r"ln(x) + ln(2)"),
    (r"x^a*x^b", r"x^(a + b)"),
    (r"(x^a)^2", r"x^(2a)"),
    (r"sqrt(x)^2", r"x"),
    (r"x^(1/2)*x^(1/2)", r"x"),
    (r"2x+3y-x+y", r"x + 4y"),
    (r"x*y*x", r"x^2*y"),
    (r"subst(x^2+y, x, 3)", r"y + 9"),
    (r"subst(x^2+y, x = 2)", r"y + 4"),
    (r"x^2=4", r"x = -2|x = 2"),
    (r"x^2+1=0", r"x = -i|x = i"),
    (
        r"x^3=8",
        r"x = 2|x = -1 - i*sqrt(3)|  ~ -1 - 1.7320508075688772935274463415059i|x = -1 + i*sqrt(3)|  ~ -1 + 1.7320508075688772935274463415059i",
    ),
    (r"x^2-5x+6=0", r"x = 2|x = 3"),
    (r"2x+3=7", r"x = 2"),
    (r"x^3-6x^2+11x-6=0", r"x = 1|x = 2|x = 3"),
    (r"x^4-5x^2+4=0", r"x = -2|x = -1|x = 1|x = 2"),
    (
        r"x^3-3x+1=0",
        r"x = -2cos(pi/9)|  ~ -1.8793852415718167681082185546495|x = 2cos(4pi/9)|  ~ 0.34729635533386069770343325353863|x = 2cos(2pi/9)|  ~ 1.5320888862379560704047853011108",
    ),
    (
        r"x^5-x-1=0",
        r"x ~ 1.1673039782614186842560458998548|x ~ -0.76488443360058472602982318770854 - 0.35247154603172624931794709140258i|x ~ -0.76488443360058472602982318770854 + 0.35247154603172624931794709140258i|x ~ 0.18123244446987538390180023778112 - 1.0839541013177106684303444929808i|x ~ 0.18123244446987538390180023778112 + 1.0839541013177106684303444929808i",
    ),
    (r"(x-1)^2*(x+2)=0", r"x = -2|x = 1 (multiplicity 2)"),
    (
        r"a*x^2+b*x+c=0",
        r"x = (-b - sqrt(b^2 - 4a*c))/(2a)|x = (sqrt(b^2 - 4a*c) - b)/(2a)",
    ),
    (r"solve(a*x+b=0, x)", r"x = -b/a"),
    (
        r"solve(x^2+y^2=1, y)",
        r"y = -sqrt(1 - x^2)|y = sqrt(1 - x^2)",
    ),
    (
        r"cos(x)=1/2",
        r"x = -pi/3 + 2n*pi, n is an integer|x = pi/3 + 2k*pi, k is an integer",
    ),
    (r"tan(x)=1", r"x = pi/4 + k*pi, k is an integer"),
    (
        r"sin(x)+cos(x)=1",
        r"x = 2k*pi, k is an integer|x = pi/2 + 2n*pi, n is an integer",
    ),
    (
        r"sin(x)=2",
        r"x = -asin(2) + pi + 2n*pi, n is an integer|x = asin(2) + 2k*pi, k is an integer",
    ),
    (r"2^x=8", r"x = 3 + 2i*k*pi/ln(2), k is an integer"),
    (r"e^x=5", r"x = ln(5) + 2i*k*pi, k is an integer"),
    (r"ln(x)=2", r"x = e^2|  ~ 7.389056098930650227230427460575"),
    (
        r"x*e^x=1",
        r"x = lambertw(1)|  ~ 0.56714329040978387299996866221036|note: Lambert W: the real branches are given; lambertw(k, z) with other integers k gives complex solutions",
    ),
    (r"sqrt(x)+sqrt(x+5)=5", r"x = 4"),
    (r"sqrt(x)=3", r"x = 9"),
    (
        r"abs(x)=3",
        r"x = -3|x = 3|note: abs: the real solutions are given",
    ),
    (r"x^x=27", r"x = 3"),
    (
        r"e^x=x+2",
        r"x = -2 - lambertw(-1/e^2)|  ~ -1.8414056604369606378466046580125|x = -2 - lambertw(-1, -1/e^2)|  ~ 1.1461932206205825852370610285214|note: Lambert W: the real branches are given; lambertw(k, z) with other integers k gives complex solutions",
    ),
    (
        r"cos(x)=x",
        r"x ~ 0.73908513321516064165531208767387|x ~ -9.1099874539365630067992197228856 - 2.9501708616994369577625248695066i|x ~ -9.1099874539365630067992197228856 + 2.9501708616994369577625248695066i|x ~ -2.4868856989085602306969570343507 - 1.809361341295703319016276100194i|x ~ -2.4868856989085602306969570343507 + 1.809361341295703319016276100194i|x ~ 5.8695603773476143940881725681928 - 2.5448857668857093265559704256731i|x ~ 5.8695603773476143940881725681928 + 2.5448857668857093265559704256731i|x ~ 12.308557602869556231953449009541 - 3.2355040686191297238524490154616i|x ~ 12.308557602869556231953449009541 + 3.2355040686191297238524490154616i|note: numeric search: the roots found are listed, there may be others",
    ),
    (r"1/x=0", r"no solution"),
    (r"x=x", r"true"),
    (r"x+1=x", r"false"),
    (r"3=3", r"true"),
    (r"3=4", r"false"),
    (r"x+y=3, x-y=1", r"x = 2, y = 1"),
    (r"x^2+y^2=25, x+y=7", r"x = 4, y = 3|x = 3, y = 4"),
    (
        r"a*x+b*y=1, c*x+d*y=0",
        r"x = d/(a*d - b*c), y = -c/(a*d - b*c)",
    ),
    (r"x+y+z=6, x-y+z=2, x+y-z=0", r"x = 1, y = 2, z = 3"),
    (r"x+y=2, 2x+2y=4", r"x = 2 - y|note: free: y (any value)"),
    (r"x+y=2, x+y=3", r"no solution"),
    (r"x*y=6, x+y=5", r"x = 3, y = 2|x = 2, y = 3"),
    (
        r"solve([x+y=3, x*y=2], [x, y])",
        r"x = 2, y = 1|x = 1, y = 2",
    ),
];

const CALCULUS_AND_REAL_LINE: &[(&str, &str)] = &[
    (r"diff(sin(x)^2, x)", r"2cos(x)*sin(x)"),
    (r"diff(x^3, x)", r"3x^2"),
    (r"diff(x^x, x)", r"x^x*(ln(x) + 1)"),
    (r"diff(e^(2x)*cos(x), x)", r"e^(2x)*(2cos(x) - sin(x))"),
    (r"diff(ln(x^2+1), x)", r"2x/(x^2 + 1)"),
    (r"diff(x^3*y^2, x, y)", r"6x^2*y"),
    (r"diff(x^5, x, 3)", r"60x^2"),
    (r"diff(atan(x), x)", r"1/(x^2 + 1)"),
    (r"diff(sqrt(x), x)", r"1/(2sqrt(x))"),
    (r"diff(tan(x), x)", r"1/cos(x)^2"),
    (r"d/dx x^2", r"2x"),
    (r"sin(x)'", r"cos(x)"),
    (r"grad(x^2*y+z)", r"[2x*y, x^2, 1]"),
    (r"hessian(x^2*y, [x, y])", r"[[2y, 2x], [2x, 0]]"),
    (r"laplacian(x^2+y^2+z^2)", r"6"),
    (r"jacobian([x*y, x+y], [x, y])", r"[[y, x], [1, 1]]"),
    (r"divergence([x^2, y^2, z^2], [x, y, z])", r"2x + 2y + 2z"),
    (r"curl([y, -x, 0], [x, y, z])", r"[0, 0, -2]"),
    (r"integrate(x*e^x, x)", r"e^x*(x - 1)"),
    (r"integrate(sin(x)^2, x)", r"x/2 - sin(2x)/4"),
    (r"integrate(x^2, x)", r"x^3/3"),
    (r"integrate(1/x, x)", r"ln(x)"),
    (r"integrate(1/(x^2+1), x)", r"atan(x)"),
    (r"integrate(1/(x^2-1), x)", r"ln(x - 1)/2 - ln(x + 1)/2"),
    (r"integrate(ln(x), x)", r"x*ln(x) - x"),
    (r"integrate(x*sin(x), x)", r"sin(x) - x*cos(x)"),
    (r"integrate(e^(-x^2), x)", r"sqrt(pi)*erf(x)/2"),
    (r"integrate(1/sqrt(1-x^2), x)", r"asin(x)"),
    (r"integrate(sin(x)*cos(x), x)", r"-cos(2x)/4"),
    (r"integrate(tan(x), x)", r"-ln(cos(x))"),
    (r"integrate(x/(x^2+1), x)", r"ln(x^2 + 1)/2"),
    (r"integrate(e^x*sin(x), x)", r"-e^x*(cos(x) - sin(x))/2"),
    (
        r"integrate(1/(x^3+1), x)",
        r"ln(x + 1)/3 - ln(x^2 - x + 1)/6 + sqrt(3)*atan(sqrt(3)*(2x - 1)/3)/3",
    ),
    (r"integrate(sqrt(x), x)", r"2x^(3/2)/3"),
    (r"integrate(x^2, x, 0, 3)", r"9"),
    (r"integrate(sin(x), x, 0, pi)", r"2"),
    (
        r"integrate(e^(-x^2), x, -inf, inf)",
        r"sqrt(pi)|  ~ 1.7724538509055160272981674833411",
    ),
    (
        r"integrate(sin(x)/x, x, 0, inf)",
        r"pi/2|  ~ 1.5707963267948966192313216916398",
    ),
    (r"integrate(1/x^2, x, 1, inf)", r"1"),
    (r"integrate(1/x, x, 0, 1)", r"inf"),
    (r"integrate(x*e^(-x^2), x, 0, inf)", r"1/2|  ~ 0.5"),
    (r"integrate(e^(-x), x, 0, inf)", r"1"),
    (
        r"integrate(1/(1+x^2), x, -inf, inf)",
        r"pi|  ~ 3.1415926535897932384626433832795",
    ),
    (
        r"integrate(sqrt(1-x^2), x, -1, 1)",
        r"pi/2|  ~ 1.5707963267948966192313216916398",
    ),
    (r"integrate(ln(x), x, 0, 1)", r"-1"),
    (r"integrate(x^n, x)", r"x^(n + 1)/(n + 1)"),
    (r"integrate(e^(a*x), x)", r"e^(a*x)/a"),
    (
        r"nintegrate(sin(x^2), x, 0, 1)",
        r"0.3102683017233811018081524231654",
    ),
    (
        r"integrate(e^(sin(x)), x, 0, 1)",
        r"1.6318696084180513481371617237447",
    ),
    (r"limit(sin(x)/x, x, 0)", r"1"),
    (
        r"limit((1+1/x)^x, x, inf)",
        r"e|  ~ 2.7182818284590452353602874713527",
    ),
    (r"limit((1-cos(x))/x^2, x, 0)", r"1/2|  ~ 0.5"),
    (r"limit(1/x, x, 0, +)", r"inf"),
    (r"limit(1/x, x, 0, -)", r"-inf"),
    (
        r"limit(1/x, x, 0)",
        r"error: the limit does not exist: from the left it is -inf, from the right inf",
    ),
    (r"limit((x^2-1)/(x-1), x, 1)", r"2"),
    (r"limit(x*ln(x), x, 0, +)", r"0"),
    (r"limit((e^x-1)/x, x, 0)", r"1"),
    (r"limit(x^2/e^x, x, inf)", r"0"),
    (
        r"limit(sin(x), x, inf)",
        r"error: no limit found for sin(x) as x -> inf",
    ),
    (
        r"limit((2x^2+1)/(3x^2-x), x, inf)",
        r"2/3|  ~ 0.66666666666666666666666666666667",
    ),
    (r"limit(sqrt(x^2+x)-x, x, inf)", r"1/2|  ~ 0.5"),
    (
        r"limit(atan(x), x, inf)",
        r"pi/2|  ~ 1.5707963267948966192313216916398",
    ),
    (r"limit((1+a/x)^x, x, inf)", r"e^a"),
    (r"limit(tan(x)/x, x -> 0)", r"1"),
    (
        r"series(sin(x), x, 0, 7)",
        r"x - x^3/6 + x^5/120 - x^7/5040 + O(x^8)",
    ),
    (
        r"series(e^x, x, 0, 4)",
        r"1 + x + x^2/2 + x^3/6 + x^4/24 + O(x^5)",
    ),
    (
        r"series(1/(1-x), x, 0, 5)",
        r"1 + x + x^2 + x^3 + x^4 + x^5 + O(x^6)",
    ),
    (
        r"series(ln(1+x), x, 0, 4)",
        r"x - x^2/2 + x^3/3 - x^4/4 + O(x^5)",
    ),
    (
        r"series(cos(x), x, 0, 6)",
        r"1 - x^2/2 + x^4/24 - x^6/720 + O(x^7)",
    ),
    (
        r"series(sqrt(1+x), x, 0, 3)",
        r"1 + x/2 - x^2/8 + x^3/16 + O(x^4)",
    ),
    (
        r"series(e^x, x, 1, 2)",
        r"e + e*(x - 1) + e*(x - 1)^2/2 + O((x - 1)^3)",
    ),
    (r"series(tan(x), x, 0, 5)", r"x + x^3/3 + 2x^5/15 + O(x^6)"),
    (r"sum(k, k, 1, n)", r"n^2/2 + n/2"),
    (r"sum(k^2, k, 1, n)", r"n^3/3 + n^2/2 + n/6"),
    (r"sum(k^3, k, 1, 10)", r"3025"),
    (
        r"sum(1/k^2, k, 1, inf)",
        r"pi^2/6|  ~ 1.644934066848226436472415166646",
    ),
    (
        r"sum(1/k!, k, 0, inf)",
        r"e|  ~ 2.7182818284590452353602874713527",
    ),
    (r"sum(1/2^k, k, 0, inf)", r"2"),
    (r"sum(x^k, k, 0, inf)", r"1/(1 - x)"),
    (
        r"sum((-1)^(k+1)/k, k, 1, inf)",
        r"ln(2)|  ~ 0.69314718055994530941723212145818",
    ),
    (
        r"sum(1/k^4, k, 1, inf)",
        r"pi^4/90|  ~ 1.0823232337111381915160036965412",
    ),
    (r"sum(1/(k*(k+1)), k, 1, inf)", r"1"),
    (r"sum(k/2^k, k, 1, inf)", r"2"),
    (r"sum(1/k, k, 1, inf)", r"inf"),
    (
        r"sum(1/k^3, k, 1, inf)",
        r"zeta(3)|  ~ 1.2020569031595942853997381615114",
    ),
    (r"sum(2^k, k, 0, n)", r"2*2^n - 1"),
    (r"product(k, k, 1, 5)", r"120"),
    (r"product(1-1/k^2, k, 2, inf)", r"1/2|  ~ 0.5"),
    (r"product(k, k, 1, n)", r"n!"),
    (r"x^2-4>0", r"x < -2 or x > 2"),
    (r"x^2-4<=0", r"-2 <= x <= 2"),
    (r"(x-1)/(x+2)<=0", r"-2 < x <= 1"),
    (
        r"sin(x)>=1/2",
        r"pi/6 + 2k*pi <= x <= 5pi/6 + 2k*pi, k is an integer",
    ),
    (
        r"tan(x)>1",
        r"pi/4 + k*pi < x < pi/2 + k*pi, k is an integer",
    ),
    (r"1<x<3", r"1 < x < 3"),
    (r"x^2>4, x<5", r"x < -2 or 2 < x < 5"),
    (r"2<3", r"true"),
    (r"x^2!=4", r"x != -2 and x != 2"),
    (r"abs(x)<3", r"-3 < x < 3"),
    (r"sqrt(x)<2", r"0 <= x < 4"),
    (r"minimize(x^2-2x+3)", r"[2, x = 1]"),
    (r"maximize(-x^2+4x)", r"[4, x = 2]"),
    (
        r"minimize(x^3)",
        r"-inf|note: x^3 is unbounded below (x -> -inf)",
    ),
    (r"minimize(x^4-2x^2)", r"[-1, x = -1, x = 1]"),
    (r"maximize(x*e^(-x))", r"[1/e, x = 1]"),
    (r"minimize(x+1/x, x, 0, inf)", r"[2, x = 1]"),
];

const MATRICES_NUMBERS_NAMES: &[(&str, &str)] = &[
    (r"sum(1/k, k, 1, inf)", r"inf"),
    (r"sum(1/sqrt(k), k, 1, inf)", r"inf"),
    (r"det([1,2;3,4])", r"-2"),
    (r"det([a,b;c,d])", r"a*d - b*c"),
    (r"inv([1,2;3,4])", r"[[-2, 1], [3/2, -1/2]]"),
    (r"[1,2;3,4]*[5,6;7,8]", r"[[19, 22], [43, 50]]"),
    (r"[1,2;3,4]^2", r"[[7, 10], [15, 22]]"),
    (r"[1,2;3,4]*[1,1]", r"[3, 7]"),
    (r"[1,2,3]+[4,5,6]", r"[5, 7, 9]"),
    (r"2*[1,2,3]", r"[2, 4, 6]"),
    (r"transpose([1,2;3,4])", r"[[1, 3], [2, 4]]"),
    (r"trace([1,2;3,4])", r"5"),
    (r"rank([1,2;2,4])", r"1"),
    (r"rref([1,2,3;4,5,6])", r"[[1, 0, -1], [0, 1, 2]]"),
    (r"nullspace([1,2;2,4])", r"[[-2, 1]]"),
    (r"charpoly([1,2;3,4])", r"lambda^2 - 5lambda - 2"),
    (
        r"eigenvalues([1,2;3,4])",
        r"[5/2 - sqrt(33)/2, 5/2 + sqrt(33)/2]",
    ),
    (r"eigenvalues([2,0;0,3])", r"[2, 3]"),
    (
        r"eigenvectors([2,1;1,2])",
        r"[[1, [[-1, 1]]], [3, [[1, 1]]]]",
    ),
    (r"dot([1,2,3],[4,5,6])", r"32"),
    (r"cross([1,0,0],[0,1,0])", r"[0, 0, 1]"),
    (r"identity(2)", r"[[1, 0], [0, 1]]"),
    (r"linsolve([1,2;3,4],[5,6])", r"[-4, 9/2]"),
    (r"norm([3,4])", r"5"),
    (r"isprime(97)", r"true"),
    (r"isprime(2^127-1)", r"true"),
    (r"nextprime(100)", r"101"),
    (r"prevprime(100)", r"97"),
    (r"prime(100)", r"541"),
    (r"primepi(1000)", r"168"),
    (r"factor(2^64+1)", r"274177*67280421310721"),
    (
        r"factor(10^30+1)",
        r"61*101*3541*9901*27961*4188901*39526741",
    ),
    (r"divisors(28)", r"[1, 2, 4, 7, 14, 28]"),
    (r"totient(100)", r"40"),
    (r"sigma(28)", r"56"),
    (r"mobius(30)", r"-1"),
    (r"gcd(12, 18)", r"6"),
    (r"lcm(4, 6)", r"12"),
    (r"gcd(x^2-1, x^2-2x+1)", r"x - 1"),
    (r"fibonacci(50)", r"12586269025"),
    (r"lucas(10)", r"123"),
    (
        r"bernoulli(10)",
        r"5/66|  ~ 0.075757575757575757575757575757576",
    ),
    (r"binomial(10, 3)", r"120"),
    (r"10!", r"3628800"),
    (r"5!!", r"15"),
    (r"catalannumber(5)", r"42"),
    (r"bell(5)", r"52"),
    (r"partitions(100)", r"190569292"),
    (r"stirling2(5, 2)", r"15"),
    (r"powmod(2, 100, 97)", r"16"),
    (r"modinv(3, 11)", r"4"),
    (r"crt([2,3,2],[3,5,7])", r"23"),
    (r"egcd(240, 46)", r"[2, -9, 47]"),
    (r"jacobi(5, 21)", r"1"),
    (r"isqrt(99)", r"9"),
    (r"base(255, 16)", r"[15, 15]"),
    (r"cfrac(pi, 6)", r"[3, 7, 15, 1, 292, 1]"),
    (r"convergents(pi, 4)", r"[3, 22/7, 333/106, 355/113]"),
    (r"mod(17, 5)", r"2"),
    (r"floor(7/2)", r"3"),
    (r"ceil(7/2)", r"4"),
    (r"round(5/2)", r"3"),
    (r"17 mod 5", r"2"),
    (r"gamma(5)", r"24"),
    (
        r"gamma(1/2)",
        r"sqrt(pi)|  ~ 1.7724538509055160272981674833411",
    ),
    (
        r"gamma(3/2)",
        r"sqrt(pi)/2|  ~ 0.88622692545275801364908374167057",
    ),
    (r"zeta(2)", r"pi^2/6|  ~ 1.644934066848226436472415166646"),
    (r"zeta(4)", r"pi^4/90|  ~ 1.0823232337111381915160036965412"),
    (r"zeta(0)", r"-1/2|  ~ -0.5"),
    (
        r"zeta(-1)",
        r"-1/12|  ~ -0.083333333333333333333333333333333",
    ),
    (r"erf(0)", r"0"),
    (r"erf(inf)", r"1"),
    (r"lambertw(e)", r"1"),
    (r"lambertw(0)", r"0"),
    (
        r"digamma(1)",
        r"-eulergamma|  ~ -0.5772156649015328606065120900824",
    ),
    (r"binomial(1/2, 2)", r"-1/8|  ~ -0.125"),
    (r"N(gamma(1/3))", r"2.6789385347077476336556929409747"),
    (
        r"N(zeta(3), 40)",
        r"1.202056903159594285399738161511449990765",
    ),
    (r"N(erf(1))", r"0.84270079294971486934122063508261"),
    (r"N(lambertw(1))", r"0.56714329040978387299996866221036"),
    (r"N(ei(1))", r"1.8951178163559367554665209343316"),
    (r"N(si(1))", r"0.94608307036718301494135331382318"),
    (r"N(li(2))", r"1.0451637801174927848445888891946"),
    (
        r"N(gamma(1+i))",
        r"0.4980156681183560427136911174622 - 0.15494982830181068512495513048389i",
    ),
    (
        r"N(zeta(1/2+14i))",
        r"0.022241142609993589246213199203969 - 0.10325812326645005790236309555257i",
    ),
    (
        r"N(e, 60)",
        r"2.71828182845904523536028747135266249775724709369995957496697",
    ),
    (
        r"N(sqrt(2), 100)",
        r"1.414213562373095048801688724209698078569671875376948073176679737990732478462107038850387534327641573",
    ),
    (r"π", r"pi|  ~ 3.1415926535897932384626433832795"),
    (r"2π", r"2pi|  ~ 6.283185307179586476925286766559"),
    (r"√2", r"sqrt(2)|  ~ 1.4142135623730950488016887242097"),
    (r"√(x²)", r"sqrt(x^2)"),
    (r"x²+2x+1", r"(x + 1)^2"),
    (r"∛27", r"3"),
    (r"∞", r"inf"),
    (r"1/∞", r"0"),
    (r"x ≤ 3", r"x <= 3"),
    (r"π ≥ 3", r"true"),
    (r"3 × 4 ÷ 2", r"6"),
    (r"5 − 2", r"3"),
    (r"2·3", r"6"),
    (r"½ + ⅓", r"5/6|  ~ 0.83333333333333333333333333333333"),
    (r"x₁ + x₂", r"x1 + x2"),
    (r"α + β", r"alpha + beta"),
    (r"sin(π/2)", r"1"),
    (r"∑(k, k, 1, 10)", r"55"),
    (r"∏(k, k, 1, 5)", r"120"),
    (r"∫(x, x)", r"x^2/2"),
    (r"∂(x^2, x)", r"2x"),
    (r"|−5|", r"5"),
    (r"⌊3.7⌋", r"3"),
    (r"⌈3.2⌉", r"4"),
    (r"90°", r"pi/2|  ~ 1.5707963267948966192313216916398"),
    (r"e^(iπ) + 1", r"0"),
    (
        r"φ",
        r"1/2 + sqrt(5)/2|  ~ 1.6180339887498948482045868343656",
    ),
    (r"γ", r"eulergamma|  ~ 0.5772156649015328606065120900824"),
    (r"τ", r"2pi|  ~ 6.283185307179586476925286766559"),
    (r"2↑↑3", r"16"),
    (r"Γ(5)", r"24"),
    (r"ζ(2)", r"pi^2/6|  ~ 1.644934066848226436472415166646"),
    (r"tet(2, 3)", r"16"),
    (r"2^^3", r"16"),
    (r"2^^0", r"1"),
    (r"2^^-1", r"0"),
    (r"slog(2, 16)", r"3"),
    (r"ssrt(27, 2)", r"3"),
    (r"2^^x=16", r"x = 3"),
    (r"x^^2=4", r"x = 2"),
    (
        r"identify(3.1415926535897932384626433832795)",
        r"pi|  ~ 3.1415926535897932384626433832795",
    ),
    (
        r"identify(1.4142135623730950488016887242097)",
        r"sqrt(2)|  ~ 1.4142135623730950488016887242097",
    ),
    (r"rationalize(0.75)", r"3/4|  ~ 0.75"),
    (
        r"rationalize(pi, 0.001)",
        r"333/106|  ~ 3.1415094339622641509433962264151",
    ),
    (r"dsolve(y' = y, y, x)", r"y = C1*e^x"),
    (r"dsolve(y'' + y = 0, y, x)", r"y = C1*cos(x) + C2*sin(x)"),
    (
        r"dsolve([y'' + y = 0, y(0) = 1, y'(0) = 2], y, x)",
        r"y = cos(x) + 2sin(x)",
    ),
    (
        r"dsolve(y'' - 3y' + 2y = e^x)",
        r"y = C1*e^x + C2*e^(2x) - x*e^x",
    ),
    (r"dsolve(y' + y/x = x, y, x)", r"y = x^2/3 + C1/x"),
    (r"dsolve(y' = x*y)", r"y = C1*e^(x^2/2)"),
    (
        r"rsolve([a(n+2) = a(n+1) + a(n), a(0) = 0, a(1) = 1], a)",
        r"a(n) = -sqrt(5)*((1/2 - sqrt(5)/2)^n - (1/2 + sqrt(5)/2)^n)/5",
    ),
    (r"rsolve([a(n+1) = 2a(n), a(0) = 3], a)", r"a(n) = 3*2^n"),
    (r"f(x) := x^2 + 1; f(3); f'(x)", r"f(x) := x^2 + 1|10|2x"),
    (r"a := 5; a^2 + 1", r"5|26"),
    (r"x := 2; x^3", r"2|8"),
    (r"assume(x > 0); sqrt(x^2)", r"true|x"),
    (r"assume(n, integer); sin(n*pi)", r"true|0"),
    (r"1 + 1; ans * 3", r"2|6"),
    (r"trigexpand(sin(2x))", r"2cos(x)*sin(x)"),
    (r"trigreduce(cos(x)^2)", r"cos(2x)/2 + 1/2"),
    (r"apart(1/(x^2-1))", r"1/(2(x - 1)) - 1/(2(x + 1))"),
    (r"collect(a*x + b*x + c, x)", r"c + x*(a + b)"),
    (r"coeff(3x^2 + 2x + 1, x, 2)", r"3"),
    (r"degree(x^5 + x)", r"5"),
    (r"quo(x^3 - 1, x - 1)", r"x^2 + x + 1"),
    (r"rem(x^3 + 2, x - 1)", r"3"),
    (r"resultant(x^2 - 1, x - a, x)", r"a^2 - 1"),
    (r"discriminant(a*x^2 + b*x + c, x)", r"b^2 - 4a*c"),
    (r"roots(x^3 - 3x^2 + 3x - 1)", r"[1, 1, 1]"),
    (
        r"nsolve(cos(x) = x, x)",
        r"0.73908513321516064165531208767387",
    ),
    (r"rectform(e^(i*pi/4))", r"sqrt(2)/2 + i*sqrt(2)/2"),
    (r"polarform(1+i)", r"sqrt(2)*e^(i*pi/4)"),
    (r"hypot(3, 4)", r"5"),
    (r"avg(1, 2, 3, 4)", r"5/2|  ~ 2.5"),
    (r"max(3, 7, 5)", r"7"),
    (r"min(sqrt(2), 1.5)", r"1.4142135623730950488016887242097"),
    (
        r"agm(1, 2)",
        r"agm(1, 2)|  ~ 1.4567910310469068691864323832651",
    ),
    (r"cis(pi)", r"-1"),
    (r"complex(1, 2)^2", r"-3 + 4i"),
];

const EDGES_AND_LATER_FEATURES: &[(&str, &str)] = &[
    (r"integrate(abs(x), x, -1, 1)", r"1"),
    (r"integrate(abs(x)*e^(-x^2), x, -inf, inf)", r"1"),
    (r"integrate(frac(x), x, 0, 5/2)", r"9/8|  ~ 1.125"),
    (
        r"integrate(min(x, x^2), x, 0, 2)",
        r"11/6|  ~ 1.8333333333333333333333333333333",
    ),
    (r"integrate(abs(x), x, 1, -1)", r"-1"),
    (
        r"integrate(1/abs(x), x, 1, 2)",
        r"ln(2)|  ~ 0.69314718055994530941723212145818",
    ),
    (r"integrate(floor(x)^2, x, 0, 4)", r"14"),
    (r"integrate(abs(sin(x)), x, 0, 2pi)", r"4"),
    (r"9^9^9", r"9^387420489"),
    (
        r"2^(10^9)",
        r"2^1000000000|  ~ 4.6129760011690693931161192210373e301029995",
    ),
    (r"x'''", r"0"),
    (r"1/0", r"ComplexInfinity"),
    (r"0/0", r"undefined"),
    (r"0^0", r"undefined"),
    (r"inf-inf", r"undefined"),
    (r"ln(0)", r"-inf"),
    (r"i^i", r"e^(-pi/2)|  ~ 0.20787957635076190854695561983498"),
    (r"i^x", r"i^x"),
    (r"simplify(i^x + i^x)", r"2i^x"),
    (
        r"factor(a + b*u^2 + c*w*u + d*t*u)",
        r"b*u^2 + c*u*w + d*t*u + a",
    ),
    (
        r"factor(expand((x+y+z+1)*(x*y-z^2+w)*(u+v*w-3)))",
        r"(v*w + u - 3)*(x + y + z + 1)*(x*y - z^2 + w)",
    ),
    (r"17 mod 5", r"2"),
    (r"polarform(1+i)", r"sqrt(2)*e^(i*pi/4)"),
    (r"rectform(e^(i*pi/4))", r"sqrt(2)/2 + i*sqrt(2)/2"),
    (
        r"dsolve(y'''' = y)",
        r"y = C1*e^(-x) + C2*e^x + C3*cos(x) + C4*sin(x)",
    ),
    (
        r"dsolve(y' = x/y)",
        r"y = -sqrt(x^2 + C1)|y = sqrt(x^2 + C1)",
    ),
    (
        r"dsolve(y'' + w^2*y = 0, y, x)",
        r"y = C1*cos(w*x) + C2*sin(w*x)",
    ),
    (r"dsolve(y' = 1 + y^2)", r"y = tan(C1 + x)"),
    (r"dsolve(x^2*y'' + x*y' - y = 0)", r"y = C2*x + C1/x"),
    (
        r"dsolve(y'' + y = sin(x))",
        r"y = C1*cos(x) + C2*sin(x) - x*cos(x)/2",
    ),
    (r"dsolve(y' = e^(x-y))", r"y = ln(C1 + e^x)"),
    (r"dsolve(y' = (x+y)/x)", r"y = x*(C1 + ln(x))"),
    (r"dsolve(2x*y + (x^2 + 3y^2)*y' = 0)", r"x^2*y + y^3 = C1"),
    (r"dsolve(y' + y = y^2)", r"y = 1/(C1*e^x + 1)"),
    (
        r"dsolve([y'' = -g, y(0) = h, y'(0) = v], y, t)",
        r"y = -g*t^2/2 + t*v + h",
    ),
    (r"dsolve(y'' = -9.81, y, t)", r"y = C2*t - 4.905t^2 + C1"),
    (
        r"dsolve(y' = sqrt(1-y^2))",
        r"y = sin(C1 + x)|note: y = sin(C1 + x) satisfies the equation on a part of the real line only",
    ),
    (
        r"rsolve(a(n+2) - 4a(n+1) + 4a(n) = 0, a)",
        r"a(n) = C2*n*2^n + C1*2^n",
    ),
    (
        r"rsolve([a(n) = 3a(n-1) + 2^n, a(0) = 1], a)",
        r"a(n) = 3*3^n - 2*2^n",
    ),
    (r"rsolve(a(n+1) = a(n) + n, a)", r"a(n) = n^2/2 + C1 - n/2"),
    (
        r"0<=sin(x)<1/2",
        r"5pi/6 + 2k*pi < x <= pi + 2k*pi or 2k*pi <= x < pi/6 + 2k*pi, k is an integer",
    ),
    (
        r"cos(x)<0",
        r"pi/2 + 2k*pi < x < 3pi/2 + 2k*pi, k is an integer",
    ),
    (r"x^2=4, x>0", r"x = 2"),
    (r"x^3-x>0", r"-1 < x < 0 or x > 1"),
    (r"(x-1)*(x-2)*(x-3)>=0", r"1 <= x <= 2 or x >= 3"),
    (r"1/x>1", r"0 < x < 1"),
    (r"ln(x)>1", r"x > e"),
    (r"e^x>2", r"x > ln(2)"),
    (r"sin(x)>2", r"no real solution"),
    (r"sin(x)<2", r"x is any real number"),
    (r"x^2>=0", r"x is any real number"),
    (r"x^2<=0", r"x = 0"),
    (r"minimize(abs(x)+abs(x-2))", r"[2, x = 0, x = 2]"),
    (r"maximize(sin(x)+cos(x))", r"[sqrt(2), x = pi/4]"),
    (r"minimize(x^3-3x, x, 0, 3)", r"[-2, x = 1]"),
    (
        r"minimize(1/(1+x^2))",
        r"[0]|note: the value 0 is approached (x -> -inf) but not taken",
    ),
    (
        r"minimize((x-a)^2+b, x)",
        r"[b, x = a]|note: x = a is the only stationary point; it is a local minimum",
    ),
    (
        r"f(x) := f(x) + 1; f(1)",
        r"error: f calls itself without end",
    ),
    (r"assume(w > 0); sqrt(-w^2)", r"true|i*w"),
    (r"assume(x, real); sqrt(x^2)", r"true|abs(x)"),
    (
        r"[1,2;3,4]^(1/2)",
        r"error: a matrix power needs a whole exponent",
    ),
    (r"inv([2, 4])", r"[1/2, 1/4]"),
    (r"sum(isprime(k), k, 1, 10)", r"6false + 4true"),
    (
        r"erfc(100)",
        r"erfc(100)|  ~ 6.4059614249217320390213391485864e-4346",
    ),
    (r"N(erfc(5))", r"1.5374597944280348501883434853834e-12"),
    (
        r"li2(2)",
        r"pi^2/4 - i*pi*ln(2)|  ~ 2.467401100272339654708622749969 - 2.1775860903036021305006888982376i",
    ),
    (
        r"N(polygamma(1, 1/3), 40)",
        r"10.09559712542709408179200409989251636052",
    ),
    (
        r"N(gamma(1/3+2i))",
        r"0.062959426851031394514118228764171 - 0.073342946031507627850028125931902i",
    ),
    (
        r"N(zeta(2+3i))",
        r"0.79802198514627572062229450072481 - 0.11374430805293850021591336585732i",
    ),
    (
        r"N(erf(1+i))",
        r"1.3161512816979476448802710802437 + 0.19045346923783468628410886196916i",
    ),
    (
        r"N(ei(1+i))",
        r"1.7646259855638540684267381613512 + 2.3877698515105224192627920891038i",
    ),
    (
        r"N(si(2+3i))",
        r"4.5475138895622892198532043408795 + 1.3991965806460547894598387158988i",
    ),
    (
        r"N(li2(3+4i))",
        r"-0.60480701206119998349190645023618 + 3.7336195322943858855807109244644i",
    ),
    (
        r"N(lgamma(3+4i))",
        r"-1.7566267846037841105306041816233 + 4.74266443803465792819488940755i",
    ),
    (
        r"N(digamma(1+i))",
        r"0.094650320622476977271878482721911 + 1.07667404746858117413405079475i",
    ),
    (
        r"N(agm(1+i, 2-3i))",
        r"1.8867928351475901316549388018554 - 0.64509296243909443583100887754275i",
    ),
    (
        r"N(binomial(5.5, 2.25))",
        r"13.630408300801710100818396101375",
    ),
    (r"factorial2(5.5)", r"26.577756623752689661120732150406"),
];

const TYPICAL_QUESTIONS: &[(&str, &str)] = &[
    (
        r"x^4+x^2+1=0",
        r"x = -1/2 - i*sqrt(3)/2|  ~ -0.5 - 0.86602540378443864676372317075294i|x = -1/2 + i*sqrt(3)/2|  ~ -0.5 + 0.86602540378443864676372317075294i|x = 1/2 - i*sqrt(3)/2|  ~ 0.5 - 0.86602540378443864676372317075294i|x = 1/2 + i*sqrt(3)/2|  ~ 0.5 + 0.86602540378443864676372317075294i",
    ),
    (
        r"x^4+1=0",
        r"x = -sqrt(2)/2 - i*sqrt(2)/2|  ~ -0.70710678118654752440084436210485 - 0.70710678118654752440084436210485i|x = -sqrt(2)/2 + i*sqrt(2)/2|  ~ -0.70710678118654752440084436210485 + 0.70710678118654752440084436210485i|x = sqrt(2)/2 - i*sqrt(2)/2|  ~ 0.70710678118654752440084436210485 - 0.70710678118654752440084436210485i|x = sqrt(2)/2 + i*sqrt(2)/2|  ~ 0.70710678118654752440084436210485 + 0.70710678118654752440084436210485i",
    ),
    (
        r"x^6=64",
        r"x = -2|x = 2|x = -1 - i*sqrt(3)|  ~ -1 - 1.7320508075688772935274463415059i|x = -1 + i*sqrt(3)|  ~ -1 + 1.7320508075688772935274463415059i|x = 1 - i*sqrt(3)|  ~ 1 - 1.7320508075688772935274463415059i|x = 1 + i*sqrt(3)|  ~ 1 + 1.7320508075688772935274463415059i",
    ),
    (
        r"factor(x^6-1)",
        r"(x + 1)*(x - 1)*(x^2 + x + 1)*(x^2 - x + 1)",
    ),
    (
        r"expand((x+y+z)^3)",
        r"x^3 + 3x^2*y + 3x^2*z + 3x*y^2 + 6x*y*z + 3x*z^2 + y^3 + 3y^2*z + 3y*z^2 + z^3",
    ),
    (
        r"integrate(1/(x^4+1), x)",
        r"sqrt(2)*(2atan(sqrt(2)*x + 1) + 2atan(sqrt(2)*x - 1) + ln(x^2 + sqrt(2)*x + 1) - ln(x^2 - sqrt(2)*x + 1))/8",
    ),
    (
        r"integrate(sqrt(x^2+1), x)",
        r"x*sqrt(x^2 + 1)/2 + asinh(x)/2",
    ),
    (r"integrate(x^2*e^(-x), x, 0, inf)", r"2"),
    (r"integrate(sin(x)^3, x)", r"cos(3x)/12 - 3cos(x)/4"),
    (r"integrate(1/(1+e^x), x)", r"x - ln(e^x + 1)"),
    (r"integrate(x*ln(x), x)", r"x^2*(2ln(x) - 1)/4"),
    (r"integrate(cos(x)^2*sin(x), x)", r"-cos(3x)/12 - cos(x)/4"),
    (r"integrate(1/(x*ln(x)), x)", r"ln(ln(x))"),
    (r"integrate(atan(x), x)", r"x*atan(x) - ln(x^2 + 1)/2"),
    (r"integrate(x^3/(x^2+1), x)", r"x^2/2 - ln(x^2 + 1)/2"),
    (r"integrate(1/sqrt(x^2+4), x)", r"asinh(x/2)"),
    (r"integrate(sec(x)^2, x)", r"tan(x)"),
    (r"integrate(1/sin(x), x)", r"ln(tan(x/2))"),
    (
        r"integrate(x^2*sin(x), x)",
        r"-x^2*cos(x) + 2x*sin(x) + 2cos(x)",
    ),
    (r"integrate(1/(x^2+2x+5), x)", r"atan(x/2 + 1/2)/2"),
    (
        r"integrate(sqrt(4-x^2), x, 0, 2)",
        r"pi|  ~ 3.1415926535897932384626433832795",
    ),
    (r"integrate(x^3*e^(x^2), x)", r"e^(x^2)*(x^2 - 1)/2"),
    (r"integrate(ln(x)^2, x)", r"2x - 2x*ln(x) + x*ln(x)^2"),
    (r"integrate(1/(e^x+e^(-x)), x)", r"atan(e^x)"),
    (
        r"integrate(cos(x)^4, x, 0, pi/2)",
        r"3pi/16|  ~ 0.58904862254808623221174563436491",
    ),
    (r"limit(x^(1/x), x, inf)", r"1"),
    (r"limit((x^3-8)/(x-2), x, 2)", r"12"),
    (r"limit((sqrt(1+x)-1)/x, x, 0)", r"1/2|  ~ 0.5"),
    (r"limit(ln(x)/x, x, inf)", r"0"),
    (
        r"limit((1+2/x)^(3x), x, inf)",
        r"e^6|  ~ 403.42879349273512260838718054339",
    ),
    (r"limit(sin(3x)/sin(5x), x, 0)", r"3/5|  ~ 0.6"),
    (r"limit(x*sin(1/x), x, inf)", r"1"),
    (
        r"limit((tan(x)-x)/x^3, x, 0)",
        r"1/3|  ~ 0.33333333333333333333333333333333",
    ),
    (r"limit((e^x-1-x)/x^2, x, 0)", r"1/2|  ~ 0.5"),
    (r"limit(x^x, x, 0, +)", r"1"),
    (r"limit((x+1)^(1/3)-x^(1/3), x, inf)", r"0"),
    (r"diff(sin(x)/x, x)", r"cos(x)/x - sin(x)/x^2"),
    (r"diff(ln(sin(x)), x)", r"cos(x)/sin(x)"),
    (r"diff(asin(x), x)", r"1/sqrt(1 - x^2)"),
    (
        r"diff(x^2*e^x*sin(x), x)",
        r"x*e^x*(x*cos(x) + x*sin(x) + 2sin(x))",
    ),
    (r"diff(sqrt(1+x^2), x, 2)", r"1/(x^2 + 1)^(3/2)"),
    (r"diff(e^(x*y), x, y)", r"e^(x*y)*(x*y + 1)"),
    (
        r"series(1/(1+x^2), x, 0, 6)",
        r"1 - x^2 + x^4 - x^6 + O(x^7)",
    ),
    (
        r"series(ln(x), x, 1, 3)",
        r"(x - 1) - (x - 1)^2/2 + (x - 1)^3/3 + O((x - 1)^4)",
    ),
    (
        r"series(e^x*sin(x), x, 0, 5)",
        r"x + x^2 + x^3/3 - x^5/30 + O(x^6)",
    ),
    (
        r"series(1/sin(x), x, 0, 3)",
        r"1/x + x/6 + 7x^3/360 + O(x^4)",
    ),
    (r"sum(k*x^k, k, 1, inf)", r"x/(x - 1)^2"),
    (r"sum(1/(k^2+k), k, 1, n)", r"n/(n + 1)"),
    (r"sum(binomial(n,k), k, 0, n)", r"2^n"),
    (r"sum(k^4, k, 1, n)", r"n^5/5 + n^4/2 + n^3/3 - n/30"),
    (
        r"sum((-1)^k/(2k+1), k, 0, inf)",
        r"pi/4|  ~ 0.78539816339744830961566084581988",
    ),
    (
        r"sum(1/(2k-1)^2, k, 1, inf)",
        r"pi^2/8|  ~ 1.2337005501361698273543113749845",
    ),
    (r"sum(x^k/k!, k, 0, inf)", r"e^x"),
    (
        r"x^2+y^2=1, y=x",
        r"x = -sqrt(2)/2, y = -sqrt(2)/2|  ~ x = -0.70710678118654752440084436210485, y = -0.70710678118654752440084436210485|x = sqrt(2)/2, y = sqrt(2)/2|  ~ x = 0.70710678118654752440084436210485, y = 0.70710678118654752440084436210485",
    ),
    (r"x+2y+3z=1, 2x-y+z=2, x+y+z=3", r"x = 3, y = 2, z = -2"),
    (
        r"x*y=1, x+y=a",
        r"x = a/2 - sqrt(a^2 - 4)/2, y = a/2 + sqrt(a^2 - 4)/2|x = a/2 + sqrt(a^2 - 4)/2, y = a/2 - sqrt(a^2 - 4)/2",
    ),
    (r"simplify((x^2+2x+1)/(x+1))", r"x + 1"),
    (r"(a+b)^2-(a-b)^2", r"4a*b"),
    (r"e^(ln(x)+ln(y))", r"x*y"),
    (r"sin(asin(x))", r"x"),
    (r"cos(asin(x))", r"sqrt(1 - x^2)"),
    (r"tan(atan(x))", r"x"),
    (r"cosh(x)^2-sinh(x)^2", r"1"),
    (r"cosh(x)+sinh(x)", r"cosh(x) + sinh(x)"),
    (r"(e^x-e^(-x))/2", r"e^x/2 - e^(-x)/2"),
    (r"sin(x+pi)", r"-sin(x)"),
    (r"cos(x-pi/2)", r"sin(x)"),
    (r"sin(-x)", r"-sin(x)"),
    (r"ln(e^x)", r"ln(e^x)"),
    (r"sqrt(x^4)", r"sqrt(x^4)"),
    (r"(x^2)^(1/2)", r"sqrt(x^2)"),
    (r"abs(-x)", r"abs(x)"),
    (r"abs(x*y)", r"abs(x*y)"),
    (r"conj(x+i*y)", r"conj(x) - i*conj(y)"),
    (r"re(e^(i*x))", r"re(e^(i*x))"),
    (r"(1+x)^2/(1+x)", r"x + 1"),
    (r"1/(1/x)", r"x"),
    (r"x^(1/2)^2", r"x^(1/4)"),
    (r"2^(3x)/8^x", r"1"),
    (r"log(100, 10)", r"2"),
    (r"log2(1024)", r"10"),
    (
        r"sin(pi/8)",
        r"sqrt(2 - sqrt(2))/2|  ~ 0.3826834323650897717284599840304",
    ),
    (
        r"tan(pi/3)",
        r"sqrt(3)|  ~ 1.7320508075688772935274463415059",
    ),
    (r"cos(2pi/3)", r"-1/2|  ~ -0.5"),
    (r"asin(sin(1))", r"1"),
    (
        r"acos(cos(4))",
        r"2pi - 4|  ~ 2.283185307179586476925286766559",
    ),
    (
        r"atan(tan(2))",
        r"2 - pi|  ~ -1.1415926535897932384626433832795",
    ),
    (r"sum(1/(4k^2-1), k, 1, n)", r"n/(2n + 1)"),
    (r"sum(1/k, k, 1, n)", r"harmonic(n)"),
    (r"sum(1/k^2, k, 1, n)", r"pi^2/6 - polygamma(1, n + 1)"),
    (r"sum(binomial(n,k)*x^k, k, 0, n)", r"(x + 1)^n"),
    (
        r"sum((-1)^k/(3k+1), k, 0, inf)",
        r"ln(2)/3 + sqrt(3)*pi/9|  ~ 0.83564884826472105333710345970011",
    ),
    (
        r"sum(1/((3k+1)*(3k+2)), k, 0, inf)",
        r"sqrt(3)*pi/9|  ~ 0.60459978807807261686469275254739",
    ),
    (
        r"digamma(1/4)",
        r"-eulergamma - 3ln(2) - pi/2|  ~ -4.2274535333762654080895301460967",
    ),
    (
        r"digamma(1/3)",
        r"-eulergamma - 3ln(3)/2 - sqrt(3)*pi/6|  ~ -3.1320337800208063229964190742873",
    ),
    (r"sinh(x)^2 + 1", r"cosh(x)^2"),
    (
        r"ln(sqrt(3)/2)",
        r"ln(3)/2 - ln(2)|  ~ -0.14384103622589046371960950299691",
    ),
    (r"assume(x>0); ln(x^2)", r"true|2ln(x)"),
    (
        r"x^2 = 2 ± 1",
        r"x = -sqrt(3)|  ~ -1.7320508075688772935274463415059|x = sqrt(3)|  ~ 1.7320508075688772935274463415059|x = -1|x = 1",
    ),
    (r"sqrt([1, 4, 9])", r"[1, 2, 3]"),
    (r"sin²(x) + cos²(x)", r"1"),
    (r"isprime([2, 3, 4, 5])", r"[true, true, false, true]"),
];

const NUMBER_THEORY_ROUND_TWO: &[(&str, &str)] = &[
    (
        r"cfrac(-47723556207783/7869431960)",
        r"[-6065, 1, 1, 2, 1, 2, 2, 1, 1, 2, 6, 1, 5, 1, 1, 3, 2, 1, 1, 1, 10, 3, 1, 23, 2, 4]",
    ),
    (r"convergents(415/93)", r"[4, 9/2, 58/13, 415/93]"),
    (r"sqrtmod(4, 15)", r"2"),
    (r"sqrtmod(2, 15)", r"undefined"),
    (r"sqrtmod(17, 2^64)", r"405959429219100393"),
    (r"sqrtmodall(1, 24)", r"[1, 5, 7, 11, 13, 17, 19, 23]"),
    (r"sqrtmods(4, 15)", r"[2, 7, 8, 13]"),
    (
        r"discretelog(3, 3^(2^60) mod (2^127-1), 2^127-1)",
        r"1152921504606846976",
    ),
    (r"discretelog(3, 5, 2^127-1)", r"undefined"),
    (r"discretelog(7, 7^123 mod 1000003, 1000003)", r"123"),
    (r"discretelog(2, 3, 0)", r"undefined"),
    (r"17 mod (5)", r"2"),
    (r"2 mod(7, 5)", r"4"),
    (r"3^(2^60) mod 1000", r"921"),
    (r"3^(2^60) mod -1000", r"-79"),
    (r"10 mod (3 + 1)", r"2"),
    (r"totient(0)", r"0"),
    (r"totient(-5)", r"4"),
    (r"mobius(0)", r"0"),
    (r"sigma(-6)", r"12"),
    (r"carmichael(561)", r"80"),
    (r"carmichaellambda(8)", r"2"),
    (r"liouville(12)", r"-1"),
    (
        r"mangoldt(27)",
        r"ln(3)|  ~ 1.0986122886681096913952452369225",
    ),
    (r"mangoldt(12)", r"0"),
    (r"isprimepower(27)", r"true"),
    (r"iscomposite(9)", r"true"),
    (r"isperfect(8128)", r"true"),
    (r"coprime(4, 9, 25)", r"true"),
    (r"iscoprime(4, 6)", r"false"),
    (r"primes(30)", r"[2, 3, 5, 7, 11, 13, 17, 19, 23, 29]"),
    (r"primes(100, 130)", r"[101, 103, 107, 109, 113, 127]"),
    (
        r"primes(10^30, 10^30+200)",
        r"[1000000000000000000000000000057, 1000000000000000000000000000099]",
    ),
    (r"primefactors(360)", r"[2, 3, 5]"),
    (r"ifactors(360)", r"[[2, 3], [3, 2], [5, 1]]"),
    (r"primorial(10)", r"210"),
    (r"legendre(2, 7)", r"1"),
    (r"legendre(3, 7)", r"-1"),
    (r"legendre(3, 9)", r"undefined"),
    (
        r"farey(5)",
        r"[0, 1/5, 1/4, 1/3, 2/5, 1/2, 3/5, 2/3, 3/4, 4/5, 1]",
    ),
    (r"squaresr(2, 25)", r"12"),
    (r"squaresr(4, 10^20)", r"2861022949218744"),
    (r"squaresr(3, 1000)", r"144"),
    (r"squaresr(24, 2)", r"1104"),
    (r"partitionsq(100)", r"444793"),
    (r"eulerian(5, 2)", r"66"),
    (r"superfactorial(5)", r"34560"),
    (r"hyperfactorial(4)", r"27648"),
    (r"pell(61)", r"[1766319049, 226153980]"),
    (r"pell(2, 5)", r"[3363, 2378]"),
    (r"pell(4)", r"undefined"),
    (r"egyptian(4/13)", r"[1/4, 1/18, 1/468]"),
    (r"egyptian(7/2)", r"[3, 1/2]"),
    (r"primitiveroots(14)", r"[3, 5]"),
    (r"primitiveroots(8)", r"[]"),
    (r"pochhammer(x, 3)", r"x^3 + 3x^2 + 2x"),
    (r"pochhammer(3, 4)", r"360"),
    (r"pochhammer(x, n)", r"gamma(n + x)/gamma(x)"),
    (r"pochhammer(x, -2)", r"1/(x^2 - 3x + 2)"),
    (r"falling(x, 3)", r"x^3 - 3x^2 + 2x"),
    (r"fallingfactorial(10, 3)", r"720"),
    (r"primepi(10^10)", r"455052511"),
    (r"0.(3)", r"1/3|  ~ 0.33333333333333333333333333333333"),
    (r"0.1(6)", r"1/6|  ~ 0.16666666666666666666666666666667"),
    (r"2.(142857)", r"15/7|  ~ 2.1428571428571428571428571428571"),
    (r"0.(9)", r"1"),
    (r"0.5(x)", r"0.5x"),
];

const REAL_DOMAIN: &[(&str, &str)] = &[
    (r"x^2+1=0", r"no real solution"),
    (r"x^3=8", r"x = 2"),
    (r"x^3=-8", r"x = -2"),
    (r"x^4=16", r"x = -2|x = 2"),
    (r"(-8)^(1/3)", r"-2"),
    (r"cbrt(-27)", r"-3"),
    (r"(-32)^(3/5)", r"-8"),
    (
        r"sqrt(-4)",
        r"undefined|note: the value 2i is not a real number",
    ),
    (
        r"ln(-1)",
        r"undefined|note: the value i*pi is not a real number",
    ),
    (r"sqrt(x^2)", r"abs(x)"),
    (r"conj(x+2)", r"x + 2"),
    (r"re(x*(1+i))", r"x"),
    (r"e^x=1", r"x = 0"),
    (r"e^x=-1", r"no real solution"),
    (r"sin(x)=0", r"x = k*pi, k is an integer"),
    (r"cosh(x)=0", r"no real solution"),
    (
        r"x^2=2",
        r"x = -sqrt(2)|  ~ -1.4142135623730950488016887242097|x = sqrt(2)|  ~ 1.4142135623730950488016887242097",
    ),
    (r"integrate(cbrt(x), x, -8, 8)", r"0"),
    (r"integrate(x^(1/3), x, -8, 1)", r"-45/4|  ~ -11.25"),
    (r"(x^3)^(1/3)", r"x"),
    (r"(-x)^(1/3)", r"-x^(1/3)"),
    (r"N((-2)^(1/3))", r"-1.2599210498948731647672106072782"),
    (r"x+y=3, x*y=5", r"no real solution"),
    (r"x+y=3, x*y=2", r"x = 2, y = 1|x = 1, y = 2"),
    (r"abs(x)^2", r"x^2"),
    (r"roots(x^4-1)", r"[-1, 1]"),
    (r"nsolve(x^2+1, x)", r"error: no root found for x^2 + 1"),
    (
        r"nsolve(x^3+x+1, x)",
        r"-0.68232780382801932736948373971105",
    ),
    (r"2^x=-4", r"no real solution"),
    (r"i^2", r"-1"),
    (r"re((x+i*y)^2)", r"x^2 - y^2"),
    (r"im(sin(x+i*y))", r"cos(x)*sinh(y)"),
    (r"domain()", r"real"),
];

const QUATERNION_DOMAIN: &[(&str, &str)] = &[
    (r"i*j", r"k"),
    (r"j*i", r"-k"),
    (r"j*k", r"i"),
    (r"k*j", r"-i"),
    (r"k*i", r"j"),
    (r"i*k", r"-j"),
    (r"j^2", r"-1"),
    (r"i*j*k", r"-1"),
    (r"(1+2i+3j+4k)*(5+6i+7j+8k)", r"-60 + 12i + 30j + 24k"),
    (r"(5+6i+7j+8k)*(1+2i+3j+4k)", r"-60 + 20i + 14j + 32k"),
    (r"(1+2i+3j+4k)^2", r"-28 + 4i + 6j + 8k"),
    (r"1/(1+2i+3j+4k)", r"1/30 - i/15 - j/10 - 2k/15"),
    (r"conj(1+2i+3j+4k)", r"1 - 2i - 3j - 4k"),
    (
        r"abs(1+2i+3j+4k)",
        r"sqrt(30)|  ~ 5.477225575051661134569697828008",
    ),
    (r"q := 1+j; p := 2-k; q*p - p*q", r"1 + j|2 - k|-2i"),
    (r"exp(j*pi)", r"-1"),
    (r"ln(j)", r"pi*j/2"),
    (r"sqrt(j)", r"sqrt(2)/2 + sqrt(2)*j/2"),
    (r"sin(j)", r"sinh(1)*j"),
    (
        r"cos(1+i+j+k)",
        r"cos(1)*cosh(sqrt(3)) - sqrt(3)*sin(1)*sinh(sqrt(3))*i/3 - sqrt(3)*sin(1)*sinh(sqrt(3))*j/3 - sqrt(3)*sin(1)*sinh(sqrt(3))*k/3",
    ),
    (r"re(1+2i+3j+4k)", r"1"),
    (r"im(1+2i+3j+4k)", r"[2, 3, 4]"),
    (r"sign(3j+4k)", r"3j/5 + 4k/5"),
    (r"x*j + y*k", r"x*j + y*k"),
    (r"(a+b*i+c*j+d*k)*(a-b*i-c*j-d*k)", r"a^2 + b^2 + c^2 + d^2"),
    (r"(1+j)/(1-k)", r"1/2 + i/2 + j/2 + k/2"),
    (
        r"N(sqrt(j))",
        r"0.70710678118654752440084436210485 + 0.70710678118654752440084436210485j",
    ),
    (r"exp(1+2i+3j+4k) * exp(-1-2i-3j-4k)", r"1"),
    (r"2^j", r"cos(ln(2)) + sin(ln(2))*j"),
    (r"(i+j)^3", r"-2i - 2j"),
    (r"quat(1,2,3,4)", r"1 + 2i + 3j + 4k"),
    (r"quat(1,2,0,0)", r"1 + 2i"),
    (r"[j, k]*2", r"[2j, 2k]"),
    (r"inv(j)", r"-j"),
    (r"arg(1+j)", r"pi/4|  ~ 0.78539816339744830961566084581988"),
    (r"(1+j)^10", r"32j"),
    (r"k^(1/3)", r"sqrt(3)/2 + k/2"),
    (r"(x+1)*j - k/2 + 2.5i", r"2.5i + (x + 1)*j - 0.5k"),
    (r"expand((x+j)^2)", r"x^2 - 1 + 2x*j"),
    (r"diff(x^2*j + sin(x)*k, x)", r"2x*j + cos(x)*k"),
    (r"integrate(x*j + k, x, 0, 2)", r"2j + 2k"),
    (r"sum(k^2, k, 1, 10)", r"385"),
    (r"sum(k, k, 1, 4)*j", r"10j"),
    (r"x*j + y*k = 4j + 2k", r"x = -i*y + 4 + 2i"),
    (r"x*j + x*k = 4j + 2k", r"x = 3 - i"),
    (
        r"assume(x, real); assume(y, real); x*j + y*k = 4j + 2k",
        r"true|true|x = 4, y = 2",
    ),
    (r"assume(x, real); x*j + x*k = 4j + 2k", r"true|no solution"),
    (
        r"quat(a, b, c, d)*i = j + 2k",
        r"a = 0, b = 0, c = -2, d = 1",
    ),
    (r"solve(k^2 = 4, k)", r"k = -2|k = 2"),
    (
        r"rotate([1,0,0], rotationquat(pi/2, [0,0,1]))",
        r"[0, 1, 0]",
    ),
    (
        r"rotationmatrix(1+k)",
        r"[[0, -1, 0], [1, 0, 0], [0, 0, 1]]",
    ),
    (r"domain()", r"quaternion"),
];

const COMPLEX_PARTS_AND_DOMAINS: &[(&str, &str)] = &[
    (
        r"complexexpand((x+i*y)^3)",
        r"x^3 - 3x*y^2 + i*(3x^2*y - y^3)",
    ),
    (r"complexexpand(exp(x+i*y))", r"e^x*cos(y) + i*e^x*sin(y)"),
    (
        r"complexexpand(ln(x+i*y))",
        r"ln(x^2 + y^2)/2 + i*atan2(y, x)",
    ),
    (
        r"complexexpand(sqrt(x+i*y))",
        r"cos(atan2(y, x)/2)*(x^2 + y^2)^(1/4) + i*sin(atan2(y, x)/2)*(x^2 + y^2)^(1/4)",
    ),
    (r"complexexpand(abs(x+i*y))", r"sqrt(x^2 + y^2)"),
    (r"complexexpand(abs((x+i*y)^2))", r"x^2 + y^2"),
    (r"complexexpand(arg(x+i*y))", r"atan2(y, x)"),
    (
        r"complexexpand(1/(x+i*y))",
        r"x/(x^2 + y^2) - i*y/(x^2 + y^2)",
    ),
    (r"complexexpand(abs(sin(x+i*y))^2)", r"cosh(y)^2 - cos(x)^2"),
    (r"complexexpand(conj(x+i*y)*(x+i*y))", r"x^2 + y^2"),
    (
        r"complexexpand(re(exp((x+i*y)^2)))",
        r"e^(x^2 - y^2)*cos(2x*y)",
    ),
    (
        r"complexexpand(2^(x+i*y))",
        r"2^x*cos(y*ln(2)) + i*2^x*sin(y*ln(2))",
    ),
    (
        r"complexexpand(tan(x+i*y))",
        r"cos(x)*sin(x)/(cos(x)^2 + cosh(y)^2 - 1) + i*cosh(y)*sinh(y)/(cos(x)^2 + cosh(y)^2 - 1)",
    ),
    (r"complexexpand(sin(z))", r"sin(z)"),
    (r"rectform(sqrt(3+4i))", r"2 + i"),
    (r"rectform(ln(-1+i))", r"ln(2)/2 + 3i*pi/4"),
    (
        r"rectform((1+i)^(1/3))",
        r"2^(2/3)*(1 + sqrt(3))/4 + i*2^(2/3)*(sqrt(3) - 1)/4",
    ),
    (r"rectform(asin(2))", r"pi/2 - i*ln(2 + sqrt(3))"),
    (
        r"rectform(acosh(1/2))",
        r"i*pi/3|  ~ 1.0471975511965977461542144610932i",
    ),
    (r"rectform(atan(1+i))", r"pi/4 + atan(1/2)/2 + i*ln(5)/4"),
    (
        r"rectform((1+i)^i)",
        r"e^(-pi/4)*cos(ln(2)/2) + i*e^(-pi/4)*sin(ln(2)/2)",
    ),
    (r"rectform(e^(i*pi/3))", r"1/2 + i*sqrt(3)/2"),
    (
        r"N(asin(2))",
        r"1.5707963267948966192313216916398 - 1.316957896924816708625046347308i",
    ),
    (r"N(acos(2))", r"1.316957896924816708625046347308i"),
    (
        r"N(atanh(2))",
        r"0.54930614433405484569762261846126 - 1.5707963267948966192313216916398i",
    ),
    (
        r"N(asinh(-2i))",
        r"-1.316957896924816708625046347308 - 1.5707963267948966192313216916398i",
    ),
    (
        r"N(atan(-2i))",
        r"-1.5707963267948966192313216916398 - 0.54930614433405484569762261846126i",
    ),
    (r"re((x+i*y)^2)", r"re((x + i*y)^2)"),
    (r"quat(1,2,3,4)*quat(0,1,0,0)", r"quat(-2, 1, 4, -3)"),
    (r"quat(1,2,3,4)^2", r"quat(-28, 4, 6, 8)"),
    (r"conj(quat(1,2,3,4))", r"quat(1, -2, -3, -4)"),
    (r"j*k", r"i*k"),
    (r"domain(quaternion); i*j", r"quaternion|k"),
    (
        r"domain(real); x^2+1=0; domain(complex); x^2+1=0",
        r"real|no real solution|complex|x = -i|x = i",
    ),
    (r"domain()", r"complex"),
    (
        r"domain(foo)",
        r"error: domain(complex), domain(real) or domain(quaternion)",
    ),
    (r"simplify(sqrt(sin(x)^2+cos(x)^2))", r"1"),
    (r"simplify(ln(sin(x)^2+cos(x)^2))", r"0"),
    (r"simplify(sin((x^2-1)/(x-1)))", r"sin(x + 1)"),
];

const REGIONS_AREAS_VOLUMES: &[(&str, &str)] = &[
    (r"integrate(x*y, x, 0, 1, y, 0, x)", r"1/8|  ~ 0.125"),
    (r"integrate(x*y, [x, 0, 1], [y, 0, x])", r"1/8|  ~ 0.125"),
    (
        r"integrate(x^2+y^2, x, 0, 1, y, 0, 1)",
        r"2/3|  ~ 0.66666666666666666666666666666667",
    ),
    (
        r"integrate(1, x, -1, 1, y, -sqrt(1-x^2), sqrt(1-x^2))",
        r"pi|  ~ 3.1415926535897932384626433832795",
    ),
    (
        r"integrate(x*y*z, x, 0, 1, y, 0, 2, z, 0, 3)",
        r"9/2|  ~ 4.5",
    ),
    (
        r"integrate(e^(-x^2-y^2), x, -inf, inf, y, -inf, inf)",
        r"pi|  ~ 3.1415926535897932384626433832795",
    ),
    (
        r"integrate(sin(x*y), x, 0, 1, y, 0, 1)",
        r"0.23981174200056472594386588619325",
    ),
    (
        r"integrate(1/(1+x*y), x, 0, 1, y, 0, 1)",
        r"pi^2/12|  ~ 0.82246703342411321823620758332301",
    ),
    (
        r"integrate(abs(x-y), x, 0, 1, y, 0, 1)",
        r"1/3|  ~ 0.33333333333333333333333333333333",
    ),
    (
        r"integrate(abs(x-y)*abs(y-1/2), x, 0, 1, y, 0, 1)",
        r"3/32|  ~ 0.09375",
    ),
    (r"integrate(abs(x*y), x, -1, 1, y, -1, 2)", r"5/2|  ~ 2.5"),
    (
        r"nintegrate(sin(x*y*z), x, 0, 1, y, 0, 1, z, 0, 1)",
        r"0.12243402879673783876395550551984",
    ),
    (
        r"nintegrate(cos(x+y+z+w), x, 0, 1, y, 0, 1, z, 0, 1, w, 0, 1)",
        r"-0.35176387721724327714203281094918",
    ),
    (
        r"nintegrate(ln(x+y), x, 0, 1, y, 0, 1)",
        r"-0.11370563888010938116553575708365",
    ),
    (
        r"area(disk(2))",
        r"4pi|  ~ 12.566370614359172953850573533118",
    ),
    (r"area(disk(1+i, r))", r"pi*r^2"),
    (r"area(triangle(0, 4, 3i))", r"6"),
    (r"area(polygon([0,0],[4,0],[4,3],[0,3]))", r"12"),
    (r"area(polygon(0, 2, 2+2i, 1+i, 2i))", r"3"),
    (
        r"area(ellipse(0, 2, 3))",
        r"6pi|  ~ 18.849555921538759430775860299677",
    ),
    (
        r"area(annulus(0, 1, 2))",
        r"3pi|  ~ 9.4247779607693797153879301498385",
    ),
    (
        r"area(sector(0, 2, 0, pi/2))",
        r"pi|  ~ 3.1415926535897932384626433832795",
    ),
    (r"area(rectangle(0, 3+2i))", r"6"),
    (r"area(triangle([0,0,0],[1,0,0],[0,1,0]))", r"1/2|  ~ 0.5"),
    (r"volume(ball([0,0,0], r))", r"4pi*r^3/3"),
    (r"volume(ball([0,0,0,0], r))", r"pi^2*r^4/2"),
    (
        r"volume(ball([0,0,0,0,0], 1))",
        r"8pi^2/15|  ~ 5.2637890139143245967117285332673",
    ),
    (r"volume(cone([0,0,0], r, h))", r"h*pi*r^2/3"),
    (
        r"volume(cylinder([0,0,0], 2, 5))",
        r"20pi|  ~ 62.83185307179586476925286766559",
    ),
    (
        r"volume(tetrahedron([0,0,0],[1,0,0],[0,1,0],[0,0,1]))",
        r"1/6|  ~ 0.16666666666666666666666666666667",
    ),
    (r"volume(box([0,0,0,0],[1,2,3,4]))", r"24"),
    (
        r"volume(ellipsoid([0,0,0],[1,2,3]))",
        r"8pi|  ~ 25.132741228718345907701147066236",
    ),
    (
        r"volume(shell([0,0,0], 1, 2))",
        r"28pi/3|  ~ 29.321531433504736892318004910609",
    ),
    (
        r"perimeter(disk(3))",
        r"6pi|  ~ 18.849555921538759430775860299677",
    ),
    (r"perimeter(triangle(0, 3, 4i))", r"12"),
    (
        r"perimeter(ellipse(0, 2, 1))",
        r"9.6884482205476761984285031963918",
    ),
    (r"surfacearea(ball([0,0,0], r))", r"4pi*r^2"),
    (r"surfacearea(ball([0,0,0,0], r))", r"2pi^2*r^3"),
    (r"surfacearea(box([0,0,0],[1,2,3]))", r"22"),
    (
        r"surfacearea(tetrahedron([0,0,0],[1,0,0],[0,1,0],[0,0,1]))",
        r"3/2 + sqrt(3)/2|  ~ 2.3660254037844386467637231707529",
    ),
    (
        r"surfacearea(cone([0,0,0], 3, 4))",
        r"24pi|  ~ 75.398223686155037723103441198708",
    ),
    (r"centroid(triangle(0, 3, 3i))", r"[1, 1]"),
    (r"centroid(sector(0, 1, 0, pi))", r"[0, 4/(3pi)]"),
    (r"centroid(cone([0,0,0], 1, 4))", r"[0, 0, 1]"),
    (r"volume(re(z^2), z, disk(0, 1))", r"0"),
    (
        r"volume(abs(z)^2, z, disk(0, 2))",
        r"8pi|  ~ 25.132741228718345907701147066236",
    ),
    (
        r"volume(re(z)^2, z, disk(0, 1))",
        r"pi/4|  ~ 0.78539816339744830961566084581988",
    ),
    (r"volume(im(z^2), z, rectangle(0, 1+i))", r"1/2|  ~ 0.5"),
    (
        r"volume(re(e^z), z, rectangle(0, 1+i))",
        r"sin(1)*(e - 1)|  ~ 1.4458843023709458845556695850702",
    ),
    (
        r"volume(im(e^z), z, rectangle(0, 1+pi*i))",
        r"2e - 2|  ~ 3.4365636569180904707205749427053",
    ),
    (
        r"volume(abs(z), z, disk(0, 1))",
        r"2pi/3|  ~ 2.0943951023931954923084289221863",
    ),
    (
        r"volume(arg(z), z, sector(0, 1, 0, pi/2))",
        r"pi^2/16|  ~ 0.61685027506808491367715568749226",
    ),
    (
        r"volume(arg(z)^2, z, disk(0, 1))",
        r"pi^3/3|  ~ 10.335425560099940058492105022367",
    ),
    (
        r"volume(arg(z), z, sector(0, 1, 0, 3pi/2))",
        r"pi^2/16|  ~ 0.61685027506808491367715568749226",
    ),
    (r"volume(abs(im(z^2)), z, disk(0, 1))", r"1"),
    (
        r"volume(abs(re(z)*im(z)), z, rectangle(-1-i, 1+2i))",
        r"5/2|  ~ 2.5",
    ),
    (
        r"volume(abs(e^z), z, rectangle(0, 1+i))",
        r"e - 1|  ~ 1.7182818284590452353602874713527",
    ),
    (
        r"volume(re(sin(z)), z, triangle(0, 1, i))",
        r"-1/(4e) + e/4 - sin(1)/2|  ~ 0.16686510441795247511493976448265",
    ),
    (r"volume(re(1/z), z, annulus(0, 1, 2))", r"0"),
    (
        r"volume(e^(-abs(z)^2), z, disk(0, 3))",
        r"pi - pi/e^9|  ~ 3.1412049502558935703467659349176",
    ),
    (
        r"volume(ln(abs(z)), z, disk(0, 1))",
        r"-pi/2|  ~ -1.5707963267948966192313216916398",
    ),
    (r"volume(abs(z)^a, z, disk(0, 1))", r"2pi/(a + 2)"),
    (
        r"volume(abs(sin(z)), z, disk(0, 1))",
        r"2.100628697330130024451715148702",
    ),
    (
        r"volume(re(gamma(z)), z, rectangle(1, 2+i))",
        r"0.79058149346063531290843057083793",
    ),
    (
        r"integrate(z^2, z, disk(1, 1))",
        r"pi|  ~ 3.1415926535897932384626433832795",
    ),
    (
        r"integrate(conj(z)*z, z in disk(0, 1))",
        r"pi/2|  ~ 1.5707963267948966192313216916398",
    ),
    (
        r"volume(x*y, [x, y], triangle([0,0],[1,0],[0,1]))",
        r"1/24|  ~ 0.041666666666666666666666666666667",
    ),
    (
        r"integrate(x^2+y^2+z^2, [x,y,z], ball([0,0,0], 1))",
        r"4pi/5|  ~ 2.5132741228718345907701147066236",
    ),
    (r"integrate(x^2, [x,y,z], ball([0,0,0], R))", r"4R^5*pi/15"),
    (
        r"integrate(z, [x,y,z], cone([0,0,0], 1, 1))",
        r"pi/12|  ~ 0.26179938779914943653855361527329",
    ),
    (
        r"integrate(x^2+y^2+z^2+w^2, [x,y,z,w], ball([0,0,0,0], 1))",
        r"pi^2/3|  ~ 3.2898681336964528729448303332921",
    ),
    (
        r"integrate(x+y+z+w, [x,y,z,w], simplex([0,0,0,0],[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]))",
        r"1/30|  ~ 0.033333333333333333333333333333333",
    ),
    (
        r"integrate(x^2+y^2, [x,y,z], cylinder([0,0,0], R, h))",
        r"R^4*h*pi/2",
    ),
    (
        r"integrate(e^(-x^2-y^2-z^2), [x,y,z], ball([0,0,0], 2))",
        r"pi^(3/2)*erf(2) - 4pi/e^4|  ~ 5.3121197278603800156166304603088",
    ),
    (
        r"surfacearea(x^2+y^2, [x,y], disk(0,1))",
        r"pi*(5sqrt(5) - 1)/6|  ~ 5.3304135002689731212198716387908",
    ),
    (
        r"surfacearea(re(z^2), z, disk(0, 1))",
        r"pi*(5sqrt(5) - 1)/6|  ~ 5.3304135002689731212198716387908",
    ),
    (r"area(sin(x), x, 0, 2pi)", r"4"),
    (r"area(x^2, x+2, x)", r"9/2|  ~ 4.5"),
    (
        r"area(x^2-1, x)",
        r"4/3|  ~ 1.3333333333333333333333333333333",
    ),
    (
        r"area(sin(x), cos(x), x, 0, pi)",
        r"2sqrt(2)|  ~ 2.8284271247461900976033774484194",
    ),
    (
        r"arclength(x^2, x, 0, 1)",
        r"asinh(2)/4 + sqrt(5)/2|  ~ 1.4789428575445974338279060194339",
    ),
    (
        r"arclength([cos(t), sin(t), t], t, 0, 2pi)",
        r"2sqrt(2)*pi|  ~ 8.8857658763167324940317619801214",
    ),
    (
        r"arclength(cosh(x), x, 0, 1)",
        r"sinh(1)|  ~ 1.1752011936438014568823818505956",
    ),
    (r"revolutionvolume(sqrt(r^2-x^2), x, -r, r)", r"4pi*r^3/3"),
    (
        r"revolutionarea(x, x, 0, 1)",
        r"sqrt(2)*pi|  ~ 4.4428829381583662470158809900607",
    ),
];

const FUNCTIONAL_EQUATIONS: &[(&str, &str)] = &[
    (r"funsolve(f(x) + 2f(1/x) = 3x, f)", r"f(x) = 2/x - x"),
    (
        r"funsolve(f(x) + 2f(-x) = sin(x) + x^2)",
        r"f(x) = x^2/3 - sin(x)",
    ),
    (
        r"funsolve(2f(x) + f(1-x) = x^2)",
        r"f(x) = x^2/3 + 2x/3 - 1/3",
    ),
    (
        r"funsolve(f(x) + x*f(1-x) = x)",
        r"f(x) = x^2/(x^2 - x + 1)",
    ),
    (
        r"funsolve(f((x-1)/x) + f(x) = 1 + x)",
        r"f(x) = (x^3 - x^2 - 1)/(2x*(x - 1))",
    ),
    (r"funsolve(f(2x+1) = 4x^2)", r"f(x) = (x - 1)^2"),
    (
        r"funsolve(g(x) + g(1/(1-x)) = x, g)",
        r"g(x) = (x^3 - x + 1)/(2x*(x - 1))",
    ),
    (r"funsolve(f(x+y) = f(x) + f(y))", r"f(x) = C1*x"),
    (r"funsolve(f(x+y) = f(x)*f(y))", r"f(x) = 0|f(x) = e^(C1*x)"),
    (r"funsolve(f(x*y) = f(x) + f(y))", r"f(x) = C1*ln(x)"),
    (
        r"funsolve(f(x*y) = f(x)*f(y))",
        r"f(x) = 0|f(x) = x^C1|f(x) = abs(x)",
    ),
    (
        r"funsolve(f(x+y) + f(x-y) = 2f(x)*f(y))",
        r"f(x) = 0|f(x) = cosh(C1*x)",
    ),
    (
        r"funsolve(f(x+y) + f(x-y) = 2f(x) + 2f(y))",
        r"f(x) = C1*x^2",
    ),
    (r"funsolve(f((x+y)/2) = (f(x)+f(y))/2)", r"f(x) = C2*x + C1"),
    (
        r"funsolve(f(x+y) = f(x) + f(y) + x*y)",
        r"f(x) = C1*x + x^2/2",
    ),
    (
        r"funsolve(f(x+y) = f(x) + f(y) + f(x)*f(y))",
        r"f(x) = -1|f(x) = e^(C1*x) - 1",
    ),
    (
        r"funsolve(f(x+y) = f(x)*cos(y) + f(y)*cos(x))",
        r"f(x) = C1*sin(x)",
    ),
    (
        r"funsolve(f(x+y) + f(x-y) = 2f(x)*cos(y))",
        r"f(x) = C1*cos(x) + C2*sin(x)",
    ),
    (
        r"funsolve(f(x+y) = e^y*f(x) + e^x*f(y))",
        r"f(x) = C1*x*e^x",
    ),
    (r"funsolve(f(x*y) = y*f(x) + x*f(y))", r"f(x) = C1*x*ln(x)"),
    (
        r"funsolve(f(sqrt(x^2+y^2)) = f(x)*f(y))",
        r"f(x) = 0|f(x) = e^(C1*x^2)",
    ),
    (
        r"funsolve(f(x+y)*f(x-y) = f(x)^2 - f(y)^2)",
        r"f(x) = C1*x|f(x) = C2*sinh(C3*x)",
    ),
    (
        r"funsolve(f(x+y) = f(x)*f(y) - sin(x)*sin(y))",
        r"f(x) = cos(x)",
    ),
    (
        r"funsolve(f(x)*f(y) = f(x+y) + x*y)",
        r"f(x) = 1 - x|f(x) = x + 1",
    ),
    (r"funsolve(f(x+y) = f(x)*f(y)/(f(x)+f(y)))", r"f(x) = C1/x"),
    (r"funsolve(f(x+y+z) = f(x)+f(y)+f(z))", r"f(x) = C1*x"),
    (
        r"funsolve(f(x+1) = 2f(x))",
        r"f(x) = C1*2^x|note: the constants may also be functions of period 1",
    ),
    (
        r"funsolve(f(x+1) - f(x) = x)",
        r"f(x) = x^2/2 + C1 - x/2|note: the constants may also be functions of period 1",
    ),
    (
        r"funsolve(f(x+2) = f(x+1) + f(x))",
        r"f(x) = C1*(1/2 - sqrt(5)/2)^x + C2*(1/2 + sqrt(5)/2)^x|note: the constants may also be functions of period 1",
    ),
    (
        r"funsolve(f(x+1) = x*f(x))",
        r"f(x) = C1*gamma(x)|note: the constants may also be functions of period 1",
    ),
    (
        r"funsolve(f(x+1) = 2x*f(x)/(x+3))",
        r"f(x) = C1*2^x/(x^3 + 3x^2 + 2x)|note: the constants may also be functions of period 1",
    ),
    (
        r"funsolve(f(x+pi) = -f(x))",
        r"f(x) = C1*(-1)^(x/pi)|note: the constants may also be functions of period pi",
    ),
    (
        r"funsolve(f(2x) = 4f(x))",
        r"f(x) = C1*x^2|note: the constants may also be functions unchanged when x is multiplied by 2",
    ),
    (
        r"funsolve(f(2x) = f(x) + 1)",
        r"f(x) = C1 + ln(x)/ln(2)|note: the constants may also be functions unchanged when x is multiplied by 2",
    ),
    (
        r"funsolve(f(3x) = 2f(x) + x)",
        r"f(x) = x + C1*x^(ln(2)/ln(3))|note: the constants may also be functions unchanged when x is multiplied by 3",
    ),
    (
        r"funsolve(f(x^2) = 2f(x))",
        r"f(x) = C1*ln(x)|note: the constants may also be functions unchanged when x is raised to the power 2",
    ),
    (
        r"funsolve(f(2x) = 2f(x)^2 - 1)",
        r"f(x) = -1/2|f(x) = cosh(C1*x)",
    ),
    (
        r"funsolve(h(t+1) - 3h(t) = 2^t, h)",
        r"h(t) = C1*3^t - 2^t|note: the constants may also be functions of period 1",
    ),
    (
        r"funsolve(f(x) + f(1/x) = 1)",
        r"f(x) = C1*ln(x) + 1/2|note: the equation does not fix f: these are some of its solutions",
    ),
    (
        r"funsolve(f(x) + f(-x) = 0)",
        r"f(x) = C2*x^3 + C1*x|f(x) = C3*sinh(C4*x)|f(x) = C5*x|note: the equation does not fix f: these are some of its solutions",
    ),
    (
        r"funsolve(f(x)^2 = x^2)",
        r"f(x) = -x|f(x) = x|f(x) = abs(x)",
    ),
    (
        r"funsolve(f(x+y) = f(x)*cosh(y) + f(y)*cosh(x))",
        r"f(x) = C1*sinh(x)",
    ),
    (
        r"funsolve(f(x+y) - f(x-y) = 2cos(x)*sin(y))",
        r"f(x) = C1 + sin(x)",
    ),
];

const ITERATES: &[(&str, &str)] = &[
    (r"iterate(2x+1, x, 1/2)", r"sqrt(2)*x - 1 + sqrt(2)"),
    (r"iterate(2x+1, x, 0)", r"x"),
    (r"iterate(2x+1, x, 2)", r"4x + 3"),
    (r"iterate(2x+1, x, -1)", r"x/2 - 1/2"),
    (r"iterate(2x+1, x, t)", r"x*2^t + 2^t - 1"),
    (r"iterate(x+3, x, t)", r"3t + x"),
    (r"iterate(x^2, x, 1/2)", r"x^(sqrt(2))"),
    (r"iterate(x^2, x, t)", r"x^(2^t)"),
    (r"iterate(3x^2, x, 1/2)", r"3^(sqrt(2))*x^(sqrt(2))/3"),
    (r"iterate(sqrt(x), x, -1)", r"x^2"),
    (r"iterate(1/(1+x), x, 2)", r"(x + 1)/(x + 2)"),
    (r"iterate(x/(1+x), x, t)", r"x/(t*x + 1)"),
    (r"iterate(x/(1+x), x, 1/2)", r"2x/(x + 2)"),
    (r"iterate(x^2+2x, x, 1/2)", r"(x + 1)^(sqrt(2)) - 1"),
    (r"iterate(x^2+2x, x, 3)", r"(x + 1)^8 - 1"),
    (r"iterate(2x^2-1, x, 1/2)", r"cosh(sqrt(2)*acosh(x))"),
    (
        r"iterate(4x*(1-x), x, t)",
        r"1/2 - cosh(2^t*acosh(1 - 2x))/2",
    ),
    (r"iterate(e^x, x, -1)", r"ln(x)"),
    (r"iterate(e^x, x, 2)", r"e^(e^x)"),
    (r"iterate(e^x, x, 1/2)", r"e^^(slog(e, x) + 1/2)"),
    (r"iterate(2^x, x, 1/2)", r"2^^(slog(2, x) + 1/2)"),
    (r"iterate(ln(x), x, 1/2)", r"e^^(slog(e, x) - 1/2)"),
    (r"iterate(x^3, x, i)", r"x^(3^i)"),
    (r"iterate(2x, x, i)", r"2^i*x"),
    (r"exp[1/2](x)", r"e^^(slog(e, x) + 1/2)"),
    (r"exp[1/3](exp[1/3](exp[1/3](x)))", r"e^x"),
    (r"exp[-1](x)", r"ln(x)"),
    (r"exp[0](x)", r"x"),
    (r"exp[1](x)", r"e^x"),
    (r"exp[2](x)", r"e^(e^x)"),
    (r"sin[2](x)", r"sin(sin(x))"),
    (r"sin[-1](x)", r"asin(x)"),
    (r"cos[-1](x)", r"acos(x)"),
    (r"sqrt[3](x)", r"x^(1/8)"),
    (r"ln[2](x)", r"ln(ln(x))"),
    (
        r"cos[3](0)",
        r"cos(cos(1))|  ~ 0.85755321584639341574410627276199",
    ),
    (
        r"f(x) := 2x+1; f[1/2](x); f[1/2](f[1/2](x))",
        r"f(x) := 2x + 1|sqrt(2)*x - 1 + sqrt(2)|2x + 1",
    ),
    (
        r"f(x) := 2x+1; f[0.(3)](f[0.(3)](f[0.(3)](x)))",
        r"f(x) := 2x + 1|2x + 1",
    ),
    (
        r"sin[1/2](x)",
        r"sin[1/2](x)|note: no closed form for this iterate: at a number, iterate(f, x, t, x0) or f[t](x0), it is computed by regular iteration at a fixed point",
    ),
    (
        r"cos[1/2](1)",
        r"0.75308905103837029464247966234978 + 0.21095439744221607041226043321636i|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"cos[1/2](cos[1/2](1))",
        r"0.54030230586813971740093660744298|note: iterate: regular iteration at the attracting fixed point 0.7390851332 (slope -0.6736120292)",
    ),
    (
        r"iterate(e^x-1, x, 1/2, 1)",
        r"1.2710274138899515214246518766723|note: iterate: regular iteration at the fixed point 0 (slope 1)",
    ),
    (
        r"iterate(x/2+x^2/4, x, 1/2, 0.3)",
        r"0.22941785117706664604894617111227|note: iterate: regular iteration at the attracting fixed point 0 (slope 0.5)",
    ),
    (
        r"iterate(x^2+1, x, 1/2, 1)",
        r"1.4122853525689035276835226679979|note: iterate: Boettcher's coordinate at infinity (degree 2)",
    ),
    (r"diff(iterate(2x+1, x, t), t)", r"2^t*ln(2)*(x + 1)"),
    (
        r"solve(iterate(2x+1, x, 1/2) = 5, x)",
        r"x = 3sqrt(2) - 1|  ~ 3.2426406871192851464050661726291",
    ),
];

const TENSORS: &[(&str, &str)] = &[
    (r"dimensions([[1,2,3],[4,5,6]])", r"[2, 3]"),
    (r"tensorrank([[[1,2],[3,4]],[[5,6],[7,8]]])", r"3"),
    (r"flatten([[1,2],[3,[4,5]]])", r"[1, 2, 3, 4, 5]"),
    (r"reshape([1,2,3,4,5,6], [2,3])", r"[[1, 2, 3], [4, 5, 6]]"),
    (
        r"transpose([[[1,2],[3,4]],[[5,6],[7,8]]], [3,1,2])",
        r"[[[1, 3], [5, 7]], [[2, 4], [6, 8]]]",
    ),
    (r"outer([1,2],[x,y])", r"[[x, y], [2x, 2y]]"),
    (
        r"outer([1,2],[3,4],[5,6])",
        r"[[[15, 18], [20, 24]], [[30, 36], [40, 48]]]",
    ),
    (r"contract(outer([a,b],[c,d]), 1, 2)", r"a*c + b*d"),
    (r"inner([[1,2],[3,4]], [5,6])", r"[17, 39]"),
    (r"inner([[1,2],[3,4]], [[5,6],[7,8]], 2)", r"70"),
    (
        r"kron([[1,2],[3,4]], [[0,1],[1,0]])",
        r"[[0, 1, 0, 2], [1, 0, 2, 0], [0, 3, 0, 4], [3, 0, 4, 0]]",
    ),
    (
        r"levicivita(3)",
        r"[[[0, 0, 0], [0, 0, 1], [0, -1, 0]], [[0, 0, -1], [0, 0, 0], [1, 0, 0]], [[0, 1, 0], [-1, 0, 0], [0, 0, 0]]]",
    ),
    (r"levicivita(1,3,2)", r"-1"),
    (r"signature([2,1,3,4])", r"-1"),
    (r"kroneckerdelta(2,2)", r"1"),
    (r"kroneckerdelta(1,2)", r"0"),
    (r"kroneckerdelta(m,n)", r"kroneckerdelta(m, n)"),
    (r"symmetrize([[1,2],[3,4]])", r"[[1, 5/2], [5/2, 4]]"),
    (r"antisymmetrize([[1,2],[3,4]])", r"[[0, -1/2], [1/2, 0]]"),
    (
        r"wedge([1,0,0],[0,1,0])",
        r"[[0, 1, 0], [-1, 0, 0], [0, 0, 0]]",
    ),
    (r"hodge(wedge([1,0,0],[0,1,0]))", r"[0, 0, 1]"),
    (r"hodge([a,b,c])", r"[[0, c, -b], [-c, 0, a], [b, -a, 0]]"),
    (
        r"christoffel([[1,0],[0,r^2]], [r,t])",
        r"[[[0, 0], [0, -r]], [[0, 1/r], [1/r, 0]]]",
    ),
    (r"ricciscalar([[R^2,0],[0,R^2*sin(t)^2]], [t,p])", r"2/R^2"),
    (
        r"ricci([[R^2,0],[0,R^2*sin(t)^2]], [t,p])",
        r"[[1, 0], [0, sin(t)^2]]",
    ),
    (
        r"riemann([[R^2,0],[0,R^2*sin(t)^2]], [t,p])",
        r"[[[[0, 0], [0, 0]], [[0, sin(t)^2], [-sin(t)^2, 0]]], [[[0, -1], [1, 0]], [[0, 0], [0, 0]]]]",
    ),
    (
        r"einstein([[R^2,0],[0,R^2*sin(t)^2]], [t,p])",
        r"[[0, 0], [0, 0]]",
    ),
    (r"kretschmann([[R^2,0],[0,R^2*sin(t)^2]], [t,p])", r"4/R^4"),
    (
        r"geodesics([[1,0],[0,r^2]], [r,t])",
        r"[r'' -> r*(t')^2, t'' -> -2r'*t'/r]",
    ),
    (
        r"laplacian(u^2, [u,v,w], [[1,0,0],[0,u^2,0],[0,0,u^2*sin(v)^2]])",
        r"6",
    ),
    (r"divergence([r, 0], [r,t], [[1,0],[0,r^2]])", r"2"),
    (
        r"covariant([r, 0], [[1,0],[0,r^2]], [r,t])",
        r"[[1, 0], [0, 1]]",
    ),
    (
        r"raiseindex([[1,2],[3,4]], [[1,0],[0,r^2]], 1)",
        r"[[1, 2], [3/r^2, 4/r^2]]",
    ),
    (r"lowerindex([1,1], [[1,0],[0,r^2]], 1)", r"[1, r^2]"),
    (
        r"matrixexp([[0,1],[-1,0]], t)",
        r"[[cos(t), sin(t)], [-sin(t), cos(t)]]",
    ),
    (r"matrixexp([[1,1],[0,1]])", r"[[e, e], [0, e]]"),
    (
        r"matrixexp([[0,1,0],[0,0,1],[0,0,0]], t)",
        r"[[1, t, t^2/2], [0, 1, t], [0, 0, 1]]",
    ),
    (r"matrixpower([[4,0],[0,9]], 1/2)", r"[[2, 0], [0, 3]]"),
    (r"matrixpower([[1,1],[0,1]], 1/2)", r"[[1, 1/2], [0, 1]]"),
    (
        r"matrixsqrt([[2,1],[1,2]])",
        r"[[1/2 + sqrt(3)/2, sqrt(3)/2 - 1/2], [sqrt(3)/2 - 1/2, 1/2 + sqrt(3)/2]]",
    ),
    (r"matrixlog([[1,1],[0,1]])", r"[[0, 1], [0, 0]]"),
    (
        r"matrixfunction(sin(x), x, [[0,1],[0,0]])",
        r"[[0, 1], [0, 0]]",
    ),
    (r"adjugate([[1,2],[3,4]])", r"[[4, -2], [-3, 1]]"),
    (
        r"gramschmidt([[1,1,0],[1,0,1],[0,1,1]])",
        r"[[1, 1, 0], [1/2, -1/2, 1], [-2/3, 2/3, 2/3]]",
    ),
    (r"cholesky([[4,2],[2,3]])", r"[[2, 0], [1, sqrt(2)]]"),
    (
        r"qr([[1,1],[1,0]])",
        r"[[[sqrt(2)/2, sqrt(2)/2], [sqrt(2)/2, -sqrt(2)/2]], [[sqrt(2), sqrt(2)/2], [0, sqrt(2)/2]]]",
    ),
    (
        r"pinv([[1,2],[3,4],[5,6]])",
        r"[[-4/3, -1/3, 2/3], [13/12, 1/3, -5/12]]",
    ),
    (r"singularvalues([[3,0],[0,4]])", r"[4, 3]"),
    (r"diag([1,2,3])", r"[[1, 0, 0], [0, 2, 0], [0, 0, 3]]"),
    (r"diag([[1,2],[3,4]])", r"[1, 4]"),
    (r"zeros(2,3)", r"[[0, 0, 0], [0, 0, 0]]"),
    (r"ones(2)", r"[[1, 1], [1, 1]]"),
    (
        r"hilbert(3)",
        r"[[1, 1/2, 1/3], [1/2, 1/3, 1/4], [1/3, 1/4, 1/5]]",
    ),
    (r"det(vandermonde([x,y,z]))", r"-(x - y)*(x - z)*(y - z)"),
];

const GEOMETRY: &[(&str, &str)] = &[
    (r"distance([0,0],[3,4])", r"5"),
    (r"distance([1,2,3,4],[5,6,7,8])", r"8"),
    (r"distance(1+i, 4+5i)", r"5"),
    (r"distance([0,0,5], plane([0,0,0],[1,0,0],[0,1,0]))", r"5"),
    (r"distance([1,1], line([0,0],[1,0]))", r"1"),
    (
        r"distance(line([0,0,0],[1,0,0]), line([0,1,1],[0,2,1]))",
        r"1",
    ),
    (
        r"angle([1,0],[0,1])",
        r"pi/2|  ~ 1.5707963267948966192313216916398",
    ),
    (
        r"angle([1,0,0],[1,1,0])",
        r"pi/4|  ~ 0.78539816339744830961566084581988",
    ),
    (
        r"angle([1,0,0,0],[1,1,1,1])",
        r"pi/3|  ~ 1.0471975511965977461542144610932",
    ),
    (
        r"angle([1,0],[0,0],[0,1])",
        r"pi/2|  ~ 1.5707963267948966192313216916398",
    ),
    (
        r"angle(plane([0,0,0],[1,0,0],[0,1,0]), plane([0,0,0],[1,0,0],[0,0,1]))",
        r"pi/2|  ~ 1.5707963267948966192313216916398",
    ),
    (
        r"angle(line([0,0,0],[1,1,1]), plane([0,0,0],[0,0,1]))",
        r"pi/2 - acos(sqrt(3)/3)|  ~ 0.61547970867038734106746458912399",
    ),
    (r"projection([1,2,3],[1,0,0])", r"[1, 0, 0]"),
    (r"projection([1,2,3], plane([0,0,0],[0,0,1]))", r"[1, 2, 0]"),
    (r"projection([3,4], line([0,0],[1,1]))", r"[7/2, 7/2]"),
    (
        r"intersection(line([0,0],[1,1]), line([0,2],[2,0]))",
        r"[1, 1]",
    ),
    (
        r"intersection(line([0,0,0],[1,1,1]), plane([0,0,2],[0,0,1]))",
        r"[2, 2, 2]",
    ),
    (
        r"intersection(plane([0,0,0],[0,0,1]), plane([0,0,0],[0,1,0]))",
        r"line([0, 0, 0], [-1, 0, 0])",
    ),
    (
        r"intersection(line([0,0],[1,0]), line([0,1],[1,1]))",
        r"undefined",
    ),
    (r"equation(line([0,0],[1,2]))", r"2x - y = 0"),
    (
        r"equation(plane([1,0,0],[0,1,0],[0,0,1]))",
        r"x + y + z = 1",
    ),
    (r"equation(circle([1,2],3))", r"(x - 1)^2 + (y - 2)^2 = 9"),
    (r"equation(sphere([0,0,0],r))", r"x^2 + y^2 + z^2 = r^2"),
    (
        r"equation(line([0,0,0],[1,2,3]))",
        r"[x -> t, y -> 2t, z -> 3t]",
    ),
    (r"midpoint([0,0],[2,4])", r"[1, 2]"),
    (r"centroid([0,0],[3,0],[0,3])", r"[1, 1]"),
    (
        r"centroid([[0,0,0],[1,0,0],[0,1,0],[0,0,1]])",
        r"[1/4, 1/4, 1/4]",
    ),
    (r"normalize([3,4])", r"[3/5, 4/5]"),
    (r"norm([3,4], 1)", r"7"),
    (r"norm([3,-4], inf)", r"4"),
    (
        r"norm([1,2,3], 3)",
        r"6^(2/3)|  ~ 3.3019272488946266838746099524091",
    ),
    (r"cross([1,0,0],[0,1,0])", r"[0, 0, 1]"),
    (r"cross([1,2])", r"[-2, 1]"),
    (r"cross([1,0,0,0],[0,1,0,0],[0,0,1,0])", r"[0, 0, 0, 1]"),
    (r"triple([1,0,0],[0,1,0],[0,0,1])", r"1"),
    (r"collinear([0,0],[1,1],[2,2])", r"true"),
    (r"coplanar([0,0,0],[1,0,0],[0,1,0],[1,1,1])", r"false"),
    (r"circumcircle([0,0],[2,0],[0,2])", r"[[1, 1], sqrt(2)]"),
    (
        r"circumsphere([0,0,0],[1,0,0],[0,1,0],[0,0,1])",
        r"[[1/2, 1/2, 1/2], sqrt(3)/2]",
    ),
    (r"incircle([0,0],[3,0],[0,4])", r"[[1, 1], 1]"),
    (
        r"solidangle([1,0,0],[0,1,0],[0,0,1])",
        r"pi/2|  ~ 1.5707963267948966192313216916398",
    ),
    (
        r"solidangle(pi/2)",
        r"2pi|  ~ 6.283185307179586476925286766559",
    ),
    (
        r"solidangle([0,0,0],[1,1,1],[1,-1,1],[-1,-1,1])",
        r"pi/3|  ~ 1.0471975511965977461542144610932",
    ),
    (r"ballvolume(4, r)", r"pi^2*r^4/2"),
    (r"ballvolume(n, r)", r"pi^(n/2)*r^n/gamma(n/2 + 1)"),
    (
        r"spherearea(4)",
        r"2pi^2|  ~ 19.739208802178717237668981999752",
    ),
    (
        r"ballvolume(3)",
        r"4pi/3|  ~ 4.1887902047863909846168578443727",
    ),
    (
        r"rotationmatrix(t)",
        r"[[cos(t), -sin(t)], [sin(t), cos(t)]]",
    ),
    (
        r"rotationmatrix(pi/2, [0,0,1])",
        r"[[0, -1, 0], [1, 0, 0], [0, 0, 1]]",
    ),
    (
        r"rotationmatrix(pi/2, [1,0,0,0], [0,0,0,1])",
        r"[[0, 0, 0, -1], [0, 1, 0, 0], [0, 0, 1, 0], [1, 0, 0, 0]]",
    ),
    (r"rotate([1,0], pi/2)", r"[0, 1]"),
    (r"rotate([1,0,0], pi/2, [0,0,1])", r"[0, 1, 0]"),
    (
        r"rotationquat(pi/2, [0,0,1])",
        r"quat(sqrt(2)/2, 0, 0, sqrt(2)/2)",
    ),
    (
        r"rotationmatrix(quat(1,0,0,1))",
        r"[[0, -1, 0], [1, 0, 0], [0, 0, 1]]",
    ),
    (r"reflect([1,2,3],[0,0,1])", r"[1, 2, -3]"),
    (r"topolar([1,1])", r"[sqrt(2), pi/4]"),
    (r"tospherical([1,1,1])", r"[sqrt(3), acos(sqrt(3)/3), pi/4]"),
    (r"frompolar([2, pi/2])", r"[0, 2]"),
    (r"fromspherical([1, pi/2, pi/2])", r"[0, 0, 1]"),
    (r"tospherical([1,0,0,0])", r"[1, 0, 0, 0]"),
    (
        r"regularpolygon(6, 2)",
        r"[sides -> 6, diagonals -> 9, area -> 6sqrt(3), perimeter -> 12, circumradius -> 2, inradius -> sqrt(3), angle -> 2pi/3]",
    ),
    (
        r"polytope(tesseract)",
        r"[vertices -> 16, edges -> 32, faces -> 24, cells -> 8, volume -> a^4, surface -> 8a^3, circumradius -> a, inradius -> a/2, dihedral -> pi/2]",
    ),
    (
        r"polytope(cell24, 2)",
        r"[vertices -> 24, edges -> 96, faces -> 96, cells -> 24, volume -> 32, surface -> 64sqrt(2), circumradius -> 2, inradius -> sqrt(2), dihedral -> 2pi/3]",
    ),
    (
        r"polytope(cell120)",
        r"[vertices -> 600, edges -> 1200, faces -> 720, cells -> 120, volume -> 15a^4*(105 + 47sqrt(5))/4, surface -> 30a^3*(15 + 7sqrt(5)), circumradius -> sqrt(2)*a*(3 + sqrt(5))/2, inradius -> a*(7 + 3sqrt(5))/4, dihedral -> 4pi/5]",
    ),
    (
        r"polytope(cell600)",
        r"[vertices -> 120, edges -> 720, faces -> 1200, cells -> 600, volume -> 25a^4*(2 + sqrt(5))/4, surface -> 50sqrt(2)*a^3, circumradius -> a/2 + sqrt(5)*a/2, inradius -> a*(sqrt(10) + 2sqrt(2))/4, dihedral -> acos(-1/8 - 3sqrt(5)/8)]",
    ),
    (
        r"polytope(icosahedron)",
        r"[vertices -> 12, edges -> 30, faces -> 20, volume -> 5a^3*(3 + sqrt(5))/12, surface -> 5sqrt(3)*a^2, circumradius -> a*sqrt(10 + 2sqrt(5))/4, inradius -> sqrt(3)*a*(3 + sqrt(5))/12, dihedral -> acos(-sqrt(5)/3)]",
    ),
    (
        r"polytope(dodecahedron, 1)",
        r"[vertices -> 20, edges -> 30, faces -> 12, volume -> 15/4 + 7sqrt(5)/4, surface -> 3sqrt(25 + 10sqrt(5)), circumradius -> sqrt(15)/4 + sqrt(3)/4, inradius -> sqrt(250 + 110sqrt(5))/20, dihedral -> acos(-sqrt(5)/5)]",
    ),
    (
        r"polytope(simplex, 4)",
        r"[vertices -> 5, edges -> 10, facets -> 5, volume -> sqrt(5)*a^4/96, surface -> 5sqrt(2)*a^3/12, circumradius -> sqrt(10)*a/5, inradius -> sqrt(10)*a/20, dihedral -> acos(1/4)]",
    ),
    (
        r"polytope(hypercube, 5, 2)",
        r"[vertices -> 32, edges -> 80, facets -> 10, volume -> 32, surface -> 160, circumradius -> sqrt(5), inradius -> 1, dihedral -> pi/2]",
    ),
    (
        r"polytope(orthoplex, 4)",
        r"[vertices -> 8, edges -> 24, facets -> 16, volume -> a^4/6, surface -> 4sqrt(2)*a^3/3, circumradius -> sqrt(2)*a/2, inradius -> sqrt(2)*a/4, dihedral -> 2pi/3]",
    ),
    (
        r"polytope(foo)",
        r"error: polytope(foo): the names are tetrahedron, cube, octahedron, dodecahedron, icosahedron; cell5, cell8 (tesseract), cell16, cell24, cell120, cell600; and simplex, cube, orthoplex with the dimension, as in polytope(simplex, 5)",
    ),
];

#[test]
fn number_theory_round_two() {
    check(NUMBER_THEORY_ROUND_TWO);
}

#[test]
fn real_domain() {
    check_with(&["-r"], REAL_DOMAIN);
}

#[test]
fn quaternion_domain() {
    check_with(&["-q"], QUATERNION_DOMAIN);
}

#[test]
fn complex_parts_and_domains() {
    check(COMPLEX_PARTS_AND_DOMAINS);
}

#[test]
fn regions_areas_volumes() {
    check(REGIONS_AREAS_VOLUMES);
}

#[test]
fn functional_equations() {
    check(FUNCTIONAL_EQUATIONS);
}

#[test]
fn iterates() {
    check(ITERATES);
}

#[test]
fn tensors() {
    check(TENSORS);
}

#[test]
fn geometry() {
    check(GEOMETRY);
}

#[test]
fn algebra_and_equations() {
    check(ALGEBRA_AND_EQUATIONS);
}

#[test]
fn calculus_and_real_line() {
    check(CALCULUS_AND_REAL_LINE);
}

#[test]
fn matrices_numbers_names() {
    check(MATRICES_NUMBERS_NAMES);
}

#[test]
fn edges_and_later_features() {
    check(EDGES_AND_LATER_FEATURES);
}

#[test]
fn typical_questions() {
    check(TYPICAL_QUESTIONS);
}

#[test]
fn options() {
    assert_eq!(
        run(&["-n", "sqrt(2)+pi"]).0,
        "4.5558062159628882872643321074892"
    );
    assert_eq!(
        run(&["--numeric", "x^2=2"]).0,
        "x = -1.4142135623730950488016887242097|x = 1.4142135623730950488016887242097"
    );
    assert_eq!(run(&["-d", "10", "pi"]).0, "pi|  ~ 3.141592654");
    assert_eq!(
        run(&["--digits=50", "-n", "pi"]).0,
        "3.1415926535897932384626433832795028841971693993751"
    );
    assert_eq!(run(&["-e", "sqrt(8)"]).0, "2sqrt(2)");
    assert_eq!(run(&["--exact", "x^2=2"]).0, "x = -sqrt(2)|x = sqrt(2)");
    assert!(run(&["--help"]).0.starts_with("Usage: symbolic-math"));
    assert_eq!(
        run(&["--version"]).0,
        format!("symbolic-math {}", env!("CARGO_PKG_VERSION"))
    );
    let functions = run(&["--functions"]).0;
    for name in [
        "sqrt, √",
        "integrate, int, integral, ∫",
        "slog, ate",
        "ssrt, sroot",
        "lambertw, w, productlog",
    ] {
        assert!(functions.contains(name), "--functions lacks {name}");
    }
    let constants = run(&["--constants"]).0;
    for name in [
        "pi, π",
        "phi, golden, goldenratio, φ, ϕ",
        "eulergamma, gamma, γ",
        "inf, infinity, oo, ∞",
    ] {
        assert!(constants.contains(name), "--constants lacks {name}");
    }
    let units = run(&["--units"]).0;
    for name in ["km", "degC", "kWh", "lightyear", "Prefixes", "kibi"] {
        assert!(units.contains(name), "--units lacks {name}");
    }
    let (out, err, ok) = run(&["-d", "0", "pi"]);
    assert!(
        out.is_empty() && !ok && err.contains("--digits needs a whole number"),
        "{err}"
    );
}

#[test]
fn precision_follows_the_longest_number() {
    // 40 significant digits in the input give 40 digits in the answer
    assert_eq!(
        run(&["1.234567890123456789012345678901234567891*2"]).0,
        "2.469135780246913578024691357802469135782"
    );
    assert_eq!(
        run(&["sqrt(3.000000000000000000000000000000000000000)"]).0,
        "1.732050807568877293527446341505872366943"
    );
    assert_eq!(run(&["sqrt(2.0)"]).0, "1.4142135623730950488016887242097");
}

#[test]
fn one_session_for_all_arguments() {
    assert_eq!(
        run(&["a := 5", "a^2 + 1", "f(x) := x^3", "f(a)", "f'(2)"]).0,
        "5|26|f(x) := x^3|125|12"
    );
    assert_eq!(
        run(&["assume(x > 0)", "sqrt(x^2)", "forget(x)", "sqrt(x^2)"]).0,
        "true|x|true|sqrt(x^2)"
    );
}

#[test]
fn errors_go_to_standard_error() {
    for (input, message) in [
        ("1 +", "error: an operand is missing"),
        ("foo(2)", "error: unknown function `foo`"),
        ("sin", "error: expected `(` after the function name `sin`"),
        ("(1", "error: "),
        (
            "limit(1/x, x, 0)",
            "error: the limit does not exist: from the left it is -inf, from the right inf",
        ),
        (
            "det([1,2,3;4,5,6])",
            "error: the determinant needs a square matrix",
        ),
        ("inv([1,2;2,4])", "error: the matrix is singular"),
    ] {
        let (out, err, ok) = run(&[input]);
        assert!(
            out.is_empty() && !ok && err.starts_with(message),
            "{input}: stdout `{out}`, stderr `{err}`"
        );
    }
    // the statements before a failing one still print, and the status tells of the failure
    let (out, err, ok) = run(&["1+1", "foo(2)", "2+2"]);
    assert_eq!((out.as_str(), ok), ("2|4", false), "{err}");
}

#[test]
fn reads_lines_from_standard_input() {
    use std::io::Write;
    let mut child = program()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("symbolic-math runs");
    child
        .stdin
        .take()
        .unwrap()
        .write_all("# a comment\nx := 3\n\nx^2 + 1\nquit\n5\n".as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), "3\n10\n");
}

/// `plot4d(.., fill = between)` in the files that are saved: over the place x + i*y of the
/// identity the voxels from the level of x to that of y, in the colours from the one to the
/// other, and nothing else.
#[test]
fn the_body_between_the_parts_of_a_value_is_in_the_files() {
    let dir =
        std::env::temp_dir().join(format!("symbolic-math-test-{}-between", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ask = |input: &str| -> String {
        let out = program()
            .arg(input)
            .current_dir(&dir)
            .stdin(Stdio::null())
            .output()
            .expect("symbolic-math runs");
        assert!(out.status.success(), "{input}");
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    // the nine grays of the palette, the first at the real part, the last at the imaginary part
    const GRAYS: [u32; 9] = [64, 88, 112, 136, 159, 183, 207, 231, 255];
    for file in ["body.png", "body.jxl"] {
        let saved = ask(&format!(
            "save(\"{file}\", plot4d(z, fill = between, width = 8, height = 8, frames = 8, colors = 9, palette = gray))"
        ));
        assert!(
            saved.contains("232 voxels of the box of 8 x 8 x 8"),
            "{saved}"
        );
        assert_eq!(ask(&format!("imframes(\"{file}\")")).trim(), "8");
        for k in 0..8usize {
            // red, green, blue and alpha, each row by row from the top
            let samples: Vec<u32> = ask(&format!("imread(\"{file}\", {})", k + 1))
                .split(|c: char| !c.is_ascii_digit())
                .filter(|part| !part.is_empty())
                .map(|part| part.parse().unwrap())
                .collect();
            assert_eq!(samples.len(), 4 * 64, "{file}, frame {k}");
            for row in 0..8usize {
                for across in 0..8usize {
                    let (i, j) = (across, 7 - row);
                    let inside = (i.min(j)..=i.max(j)).contains(&k);
                    let share = match i == j {
                        true => 0.5,
                        false => (k as f64 - i as f64) / (j as f64 - i as f64),
                    };
                    let gray = match inside {
                        true => GRAYS[(share * 8.0).round() as usize],
                        false => 0,
                    };
                    let at = row * 8 + across;
                    assert_eq!(
                        [0, 64, 128, 192].map(|channel| samples[channel + at]),
                        [gray, gray, gray, if inside { 255 } else { 0 }],
                        "{file}, frame {k}, row {row}, place {across}"
                    );
                }
            }
        }
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[cfg(unix)]
mod tetration {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    /// A directory with a `tet` that logs its arguments and answers with fixed numbers.
    fn stand_in(name: &str, script: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("symbolic-math-test-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tet");
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        dir
    }

    fn run_with_path(dir: &std::path::Path, input: &str) -> (String, String, bool) {
        let out = program()
            .arg(input)
            .env("PATH", dir)
            .stdin(Stdio::null())
            .output()
            .expect("symbolic-math runs");
        let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).trim_end().replace('\n', "|");
        (text(&out.stdout), text(&out.stderr), out.status.success())
    }

    const ANSWERS: &str = "#!/bin/sh\necho \"$SILENT $*\" >> \"$0.log\"\ncase \"$2\" in\n--slog) echo 3.5; echo 0;;\n--sroot) echo 1.75; echo 0;;\n*) echo 1.25; echo 0.5;;\nesac\n";

    #[test]
    fn runs_tet_for_heights_that_are_not_whole() {
        let dir = stand_in("answers", ANSWERS);
        assert_eq!(run_with_path(&dir, "2^^0.5").0, "1.25 + 0.5i");
        assert_eq!(
            run_with_path(&dir, "tet(2, 1/2)").0,
            "2^^(1/2)|  ~ 1.25 + 0.5i"
        );
        assert_eq!(
            run_with_path(&dir, "slog(2, 100)").0,
            "slog(2, 100)|  ~ 3.5"
        );
        assert_eq!(run_with_path(&dir, "ate(2, 100)").0, "slog(2, 100)|  ~ 3.5");
        assert_eq!(run_with_path(&dir, "ssrt(100, 2.5)").0, "1.75");
        assert_eq!(
            run_with_path(&dir, "sroot(100, 5/2)").0,
            "ssrt(100, 5/2)|  ~ 1.75"
        );
        assert_eq!(
            run_with_path(&dir, "2^^x = 100").0,
            "x = slog(2, 100)|  ~ 3.5"
        );
        let log = std::fs::read_to_string(dir.join("tet.log")).unwrap();
        let calls: Vec<&str> = log.lines().collect();
        // SILENT=1, -q, the mode, the digits, then base and height (or value) as real and imaginary parts
        assert!(
            calls[0].starts_with("1 -q ") && calls[0].ends_with(" 2 0 0.5 0"),
            "{}",
            calls[0]
        );
        assert!(
            calls
                .iter()
                .any(|c| c.starts_with("1 -q --slog ") && c.ends_with(" 2 0 100 0")),
            "{log}"
        );
        assert!(
            calls
                .iter()
                .any(|c| c.starts_with("1 -q --sroot ") && c.ends_with(" 100 0 2.5 0")),
            "{log}"
        );
        let digits: u32 = calls[0].split(' ').nth(2).unwrap().parse().unwrap();
        assert!(digits >= 32, "tet was asked for {digits} digits");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn whole_heights_need_no_tet() {
        let dir = stand_in("unused", "#!/bin/sh\nexit 7\n");
        assert_eq!(run_with_path(&dir, "2^^4").0, "65536");
        assert_eq!(run_with_path(&dir, "2^^x = 65536").0, "x = 4");
        assert_eq!(run_with_path(&dir, "x^^2 = 27").0, "x = 3");
        assert_eq!(run_with_path(&dir, "slog(2, 65536)").0, "4");
        assert_eq!(run_with_path(&dir, "ssrt(256, 2)").0, "4");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn reports_a_failing_or_missing_tet() {
        let dir = stand_in(
            "failing",
            "#!/bin/sh\necho 'error: no convergence' >&2\nexit 1\n",
        );
        let (out, err, ok) = run_with_path(&dir, "N(2^^0.5)");
        assert!(
            out.is_empty() && !ok && err.contains("tet: no convergence"),
            "stdout `{out}`, stderr `{err}`"
        );
        let (out, err, ok) = run_with_path(&dir, "2^^x = 100");
        assert_eq!(
            (out.as_str(), err.as_str(), ok),
            ("x = slog(2, 100)|note: tet: no convergence", "", true)
        );
        std::fs::remove_file(dir.join("tet")).unwrap();
        let (out, err, ok) = run_with_path(&dir, "2^^0.5");
        assert!(
            out.is_empty() && !ok && err.contains("could not run tet") && err.contains("on PATH"),
            "stdout `{out}`, stderr `{err}`"
        );
        let (out, err, ok) = run_with_path(&dir, "2^^(1/2)");
        assert!(
            ok && err.is_empty() && out.starts_with("2^^(1/2)|note: could not run tet"),
            "stdout `{out}`, stderr `{err}`"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// A stand-in that answers with the height it was asked for: `b^^h` is then `h` itself.
    /// It takes its time and writes down when it starts and when it ends.
    const HEIGHTS: &str = "#!/bin/sh\necho \"S $*\" >> \"$0.log\"\n/bin/sleep 0.05\necho E >> \"$0.log\"\necho \"$5\"; echo \"$6\"\n";

    #[test]
    fn a_grid_asks_tet_once_for_every_place_and_each_place_gets_its_value() {
        let dir = stand_in("grid", HEIGHTS);
        // (the text below a drawing names the function)
        for (with_tet, plain, name, plain_name) in [
            (
                "plot(1.5^^x, x, 0.1, 0.9, 24, 8)",
                "plot(x, x, 0.1, 0.9, 24, 8)",
                "",
                "",
            ),
            (
                "plot4d(1.5^^z, z, 0.1 + 0.1i, 0.9 + 0.9i, width = 8, height = 4, frames = 3)",
                "plot4d(z, z, 0.1 + 0.1i, 0.9 + 0.9i, width = 8, height = 4, frames = 3)",
                "(3/2)^^z",
                "z",
            ),
            (
                "complexplot(1.5^^z, z, 0.15 + 0.15i, 0.85 + 0.85i, width = 16, height = 6)",
                "complexplot(z, z, 0.15 + 0.15i, 0.85 + 0.85i, width = 16, height = 6)",
                "",
                "",
            ),
            (
                "plot3d(re(1.5^^(x + i*y)), x, 0.2, 0.8, y, 0.3, 0.7, width = 20, height = 8)",
                "plot3d(re(x + i*y), x, 0.2, 0.8, y, 0.3, 0.7, width = 20, height = 8)",
                "re((3/2)^^(x + i*y))",
                "re(x) - im(y)",
            ),
            (
                "densityplot(re(1.5^^(x + i*y)), x, 0.2, 0.8, y, 0.3, 0.7, width = 12, height = 5)",
                "densityplot(re(x + i*y), x, 0.2, 0.8, y, 0.3, 0.7, width = 12, height = 5)",
                "",
                "",
            ),
        ] {
            let (got, err, ok) = run_with_path(&dir, with_tet);
            let (want, _, _) = run_with_path(&dir, plain);
            assert!(ok && err.is_empty(), "{with_tet}: stderr `{err}`");
            let got = match name {
                "" => got,
                _ => got.replace(name, plain_name),
            };
            assert_eq!(got, want, "{with_tet}");
            // no place is asked twice, none again by the drawing
            let log = std::fs::read_to_string(dir.join("tet.log")).unwrap();
            std::fs::remove_file(dir.join("tet.log")).unwrap();
            let lines: Vec<&str> = log.lines().collect();
            let mut asked: Vec<&str> = lines
                .iter()
                .copied()
                .filter(|l| l.starts_with('S'))
                .collect();
            let runs = asked.len();
            asked.sort_unstable();
            asked.dedup();
            assert!(
                runs > 40 && asked.len() == runs,
                "{with_tet}: {runs} runs of tet for {} places",
                asked.len()
            );
            // and the runs are made many at a time: one starts before another has ended
            let together = lines
                .windows(2)
                .any(|w| w[0].starts_with('S') && w[1].starts_with('S'));
            let alone = std::thread::available_parallelism().map_or(1, |n| n.get()) < 2;
            assert!(
                together || alone,
                "{with_tet}: the runs of tet were made one by one"
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_drawing_says_that_tet_is_missing() {
        let dir = stand_in("missing", ANSWERS);
        std::fs::remove_file(dir.join("tet")).unwrap();
        for input in [
            "plot(1.5^^x, x, 0.1, 0.9, 24, 8)",
            "plot([sin(x), 1.5^^x], x, 0.1, 0.9, 24, 8)",
            "plot3d(b^^h, b, 1.1, 1.4, h, 0.1, 0.9, width = 20, height = 8)",
            "densityplot(slog(b, x), b, 1.1, 1.4, x, 0.1, 0.9, width = 12, height = 5)",
            "complexplot(1.5^^z, z, 0.15 + 0.15i, 0.85 + 0.85i, width = 16, height = 6)",
            "plot4d(1.5^^z, z, 0.1 + 0.1i, 0.9 + 0.9i, width = 8, height = 4, frames = 3)",
            "view4d(ssrt(z, 2.5), z, 2 + 0.1i, 3 + 0.9i, width = 24, height = 8, voxels = 8)",
            "polarplot(1.5^^t, t, 0.1, 0.9, 20, 8)",
            "save_png(\"t.png\", complexplot(1.5^^z, z, 0.15 + 0.15i, 0.85 + 0.85i), width = 8)",
        ] {
            let (out, err, ok) = run_with_path(&dir, input);
            assert!(
                out.is_empty()
                    && !ok
                    && err.starts_with("error: could not run tet: ")
                    && err.ends_with("; ensure the tet executable is on PATH"),
                "{input}: stdout `{out}`, stderr `{err}`"
            );
        }
        // what needs no value of tet is as ever, and an exact answer comes with a note
        let (out, err, ok) = run_with_path(&dir, "plot(x, x, 0.1, 0.9, 24, 3)");
        assert!(ok && err.is_empty() && !out.contains("tet"), "{out} {err}");
        let (out, _, ok) = run_with_path(&dir, "[2^^(1/2), 2^^3]");
        assert!(ok && out == "[2^^(1/2), 16]", "{out}");
        let (out, _, ok) = run_with_path(&dir, "2^^(1/2)");
        assert!(
            ok && out.starts_with("2^^(1/2)|note: could not run tet: "),
            "{out}"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn real_tet_when_installed() {
        let installed = std::env::var_os("PATH")
            .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join("tet").is_file()));
        if !installed {
            eprintln!("tet is not installed: skipped");
            return;
        }
        let (out, err, ok) = run(&["2^^0.5"]);
        assert!(
            ok && out.starts_with("1.45878181603642170068397166"),
            "stdout `{out}`, stderr `{err}`"
        );
        let (out, err, ok) = run(&["2^^x = 100"]);
        assert!(
            ok && out.starts_with("x = slog(2, 100)|  ~ 3.49090471067046825666007243"),
            "stdout `{out}`, stderr `{err}`"
        );
    }
}

/// What a pass of odd inputs found in round 7: these crashed, hung or took all the memory.
#[test]
fn odd_inputs_that_once_broke_the_program() {
    check(&[
        // a power of a thousand million of one exponential in the simplifier
        ("e^(-t) + e^(-t/1000000000)", "e^(-t) + e^(-t/1000000000)"),
        (
            "step(1/((s + 1)*(s + 1/1000000000)))",
            "1000000000e^(-t)/999999999 - 1000000000000000000e^(-t/1000000000)/999999999 + 1000000000",
        ),
        ("(e^(2t) - 1)/(e^t - 1)", "e^t + 1"),
        // a decimal exponent of a thousand beside a small one: the exact way took for ever
        (
            "softmax([-7, 1000.5])",
            "[2.8074335297474330303768726824136e-438, 1]",
        ),
        ("e^1000.5/(e^(-7) + e^1000.5)", "1"),
        // the quantile of whole numbers whose upper end is none
        ("quantile(discreteuniformdist(1, 5/2), 9/10)", "2"),
        ("quantile(discreteuniformdist(1, 6), 1/2)", "3"),
        // an observation that is no even list
        (
            "trainnet(feedforwardnet([2, 3, 1]), [[0, 0], [1, []]], [[0], [1]], \"solver\", \"sgdm\", \"epochs\", 2)",
            "error: trainnet: an observation is a list of 2 values; several of them are the rows of a matrix",
        ),
        // a number of a part far beyond the pixels of the picture
        (
            "regionprops([[1, 1, 0], [0, 0, 0], [0, 1, 1000000000]])",
            "error: regionprops: the numbers of the parts are whole numbers, no more than the picture has pixels",
        ),
        // a weight between states and inputs of the wrong size
        (
            "lqr([[0, 1], [0, 0]], [[0], [1]], [[1, 0], [0, 1]], [[1]], [[1]])",
            "error: lqr(A, B, Q, R, N): the weight N between states and inputs has a row for every state and a column for every input",
        ),
        // rows of conditions without their right sides; a lower bound above its upper one
        (
            "fmincon(@(x, y) x^2 + y^2, [1, 1], [1, 2])",
            "error: fmincon(f, start, A, b, Aeq, beq, lower, upper, conditions): the least f(x) with A*x <= b, Aeq*x = beq, lower <= x <= upper and conditions(x) = [[values that are <= 0], [values that are 0]]; [] leaves a part out",
        ),
        (
            "ga(@(x, y) x^2 + y^2, 2, [5, 5], [-5, -5])",
            "error: ga: a lower bound lies above its upper bound",
        ),
        // e is a number in a formula of blocks, also when a signal has that name
        (
            "sim([[\"e\", \"constant\", 1], [\"y\", \"fcn\", 2*e]], 1)",
            "error: sim: the block y: e in its formula is a number, and a signal of this diagram has that name too: give the signal another name",
        ),
        (
            "getfield(sim([[\"u\", \"constant\", 1], [\"y\", \"fcn\", 2*e^u]], 1, \"points\", 2), \"y\")",
            "[5.43656365691809, 5.43656365691809]",
        ),
    ]);
}

/// What a scan of larger systems found in round 7: sampling that hung or gave wrong systems.
#[test]
fn systems_that_once_hung_or_went_wrong() {
    check(&[
        // sampling: closed forms only where they are short, decimals otherwise (these hung)
        (
            "order(c2d(ss([[0, 1, 0], [0, 0, 1], [-1, -2, -3]], [[0, 0], [0, 1], [1, 0]], [[1, 0, 0], [0, 1, 0]], [[0, 0], [0, 0]]), 1/10))",
            "3",
        ),
        (
            "c2d(ss([[-1, 0], [0, -2]], [[1, 0], [0, 1]], [[1, 0], [0, 1]], [[0, 0], [0, 0]]), 1/10)",
            "ss([[1/e^(1/10), 0], [0, 1/e^(1/5)]], [[1 - 1/e^(1/10), 0], [0, 1/2 - 1/(2e^(1/5))]], [[1, 0], [0, 1]], [[0, 0], [0, 0]], 1/10)",
        ),
        (
            "c2d(1/((s^2 + 2s + 5)*(s^2 + 10s + 74)*(s + 3)), 1/10, \"impulse\")",
            "tf([3.0433480639301867484348889692682e-7, 0.0000024210380307937531529956573504475, 0.0000017884296150445891575936457104819, 1.2352083503291179624799442035126e-7, 0], [1, -3.4422205171355621847233674182834, 4.8334068474491114086688299185168, -3.5102052740679598547598516392927, 1.3472965299099051973974126626299, -0.22313016014842982893328047076401], 1/10)",
        ),
        (
            "c2d(tf([1], [1, 6, 11, 6]), 1/10, \"impulse\")",
            "tf([(e^(1/10) - 1)^2/(20e^(3/10)), (e^(1/10) - 1)^2/(20sqrt(e)), 0], [1, (-1 - e^(1/10) - e^(1/5))/e^(3/10), (1 + e^(1/10) + e^(1/5))/sqrt(e), -1/e^(3/5)], 1/10)",
        ),
        (
            "c2d(tf([1, 2], [1, 1]), 1/10, \"impulse\")",
            "error: c2d(system, Ts, \"impulse\"): the answer to a blow is sampled, so the system has more poles than zeros",
        ),
        // a matrix of systems in decimal numbers: every entry with the states of its own
        (
            "d2c(c2d([[1/(s + 1), 2/(s + 2)], [0, s/(s^2 + 2s + 5)]], 1/10))",
            "[[tf([1], [1, 1]), tf([2], [1, 2])], [tf([0], [1]), tf([1, 0], [1, 2, 5])]]",
        ),
        (
            "part(c2d([[1/(s + 1), 2/(s + 2)], [0, s/(s^2 + 2s + 5)]], 1/10), 1, 1)",
            "tf([0.095162581964040426835750940553563], [1, -0.90483741803595957316424905944644], 1/10)",
        ),
        // coefficients with names
        ("roots([1, a])", "[-a]"),
        (
            "roots([1, a, b])",
            "[-a/2 - sqrt(a^2 - 4b)/2, sqrt(a^2 - 4b)/2 - a/2]",
        ),
        (
            "roots([1, 0, -a], x)",
            "error: roots([coefficients]) or roots(expression, variable)",
        ),
        ("zpk(tf([1, b], [1, a]))", "zpk([-b], [-a], 1)"),
        ("pole(tf([1], [1, a]))", "[-a]"),
        // a system without a delay is no delay T
        ("pade(tf([1], [1, 2]), 1)", "tf([1], [1, 2])"),
        ("pade(1/(s + 2), 2)", "tf([1], [1, 2])"),
        ("pade(a, 1)", "[[-1, 2/a], [1, 2/a]]"),
        // objects of systems that are none: these overflowed the stack
        (
            "zpk([-1], [-2, -3], 4) + zpk([], [-1])",
            "error: zpk([zeros], [poles], gain[, Ts]) or zpk(system)",
        ),
        (
            "tf([1], [1, -1/2], 1/10) + tf([1], [1, -1/3], [1, -1/3], 1/10)",
            "error: tf([numerator], [denominator][, Ts]), tf(\"s\"), tf(system); with a delay: tf([numerator], [denominator], \"InputDelay\", T)",
        ),
        (
            "ss([[-1]], [[1]], [[0]])*ss([[-2]], [[1]], [[1]], [[0]])",
            "error: ss(A, B, C, D[, Ts]) or ss(system)",
        ),
        (
            "tf([[1, 2], [3]], [1, 2])",
            "error: tf: tf([numerator], [denominator][, Ts]): the coefficients of both, the highest power first",
        ),
        // a gain alone: a system without states
        ("ss(tf([2], [1]))", "ss([[]], [], [[]], [[2]])"),
        ("dcgain(ss([[]], [], [[]], [[2]]))", "2"),
        ("ss([], [], [], 2)", "ss([[]], [], [[]], [[2]])"),
        (
            "ss(tf([1], [1, 1]))*ss(tf([2], [1])) + ss(tf([2], [1]))",
            "tf([2, 4], [1, 1])",
        ),
        ("c2d(tf([2], [1]), 1/10)", "tf([2], [1], 1/10)"),
        ("d2c(tf([2], [1], 1/10))", "tf([2], [1])"),
        ("d2d(tf([2], [1], 1/10), 1/5)", "tf([2], [1], 1/5)"),
        ("h2norm(tf([1], [0, -1/2], 1))", "2"),
        (
            "gram(tf([2], [1]), \"c\")",
            "error: gram: a system with states, or gram(A, B[, \"o\"]) with a square matrix A",
        ),
        (
            "gram([[0], [1]], [[0, 1], [-2, -3]])",
            "error: gram: a system with states, or gram(A, B[, \"o\"]) with a square matrix A",
        ),
        (
            "stepinfo(tf([2], [1]))",
            "struct(\"RiseTime\", 0, \"SettlingTime\", 0, \"SettlingMin\", 2, \"SettlingMax\", 2, \"Overshoot\", 0, \"Undershoot\", 0, \"Peak\", 2, \"PeakTime\", 0)",
        ),
        // the matched way: names, zeros at 0, pairs of roots
        (
            "c2d(tf([1], [1, a]), 1/10, \"matched\")",
            "tf([(1 - e^(-a/10))/a], [1, -e^(-a/10)], 1/10)",
        ),
        (
            "c2d(tf([1, 0], [1, 1]), 1/10, \"matched\")",
            "tf([1/2 + 1/(2e^(1/10)), -1/2 - 1/(2e^(1/10))], [1, -1/e^(1/10)], 1/10)",
        ),
        (
            "c2d(tf([1], [1, 2, 5]), 1/10, \"matched\")",
            "tf([1/10 + 1/(10e^(1/5)) - cos(1/5)/(5e^(1/10)), 1/10 + 1/(10e^(1/5)) - cos(1/5)/(5e^(1/10))], [1, -2cos(1/5)/e^(1/10), 1/e^(1/5)], 1/10)",
        ),
        (
            "c2d(tf([1, -1], [1, 3, 2, 0]), 1/10, \"matched\")",
            "tf([(1 + e^(1/10))^2*(e^(1/10) - 1)^2*(1 + e^(1/10) + e^(1/5))/(24e^(2/5)), -(1 + e^(1/10))^2*(e^(1/10) - 1)^3*(1 + e^(1/10) + e^(1/5))/(24e^(2/5)), -(1 + e^(1/10))^2*(e^(1/10) - 1)^2*(1 + e^(1/10) + e^(1/5))/(24e^(3/10))], [1, -1 - 1/e^(1/10) - 1/e^(1/5), (1 + e^(1/10) + e^(1/5))/e^(3/10), -1/e^(3/10)], 1/10)",
        ),
        (
            "c2d(tf([1], [1, 3, 2, 1]), 1/10, \"matched\")",
            "tf([0.00021562552986360568098776298883099, 0.00043125105972721136197552597766198, 0.00021562552986360568098776298883099], [1, -2.72311537903558240815012283657, 2.4647961018367546969409476678431, -0.74081822068171786606687377931782], 1/10)",
        ),
        (
            "c2d(tf([1, 1], [1, 3, 2]), 1/5, \"matched\")",
            "tf([1/2 - 1/(2e^(2/5))], [1, -1/e^(2/5)], 1/5)",
        ),
    ]);
}

/// What the pass over every older example with one value changed found in round 7.
#[test]
fn odd_values_in_older_examples() {
    check(&[
        // odd parameters of distributions: these hung or gave numbers that are none
        (
            "rng(7); binornd(i, 1/2)",
            "error: binomialdist: the parameter n is a real number or a name, not i",
        ),
        ("rng(7); geornd([])", "7|[]"),
        (
            "unidinv(1/2, true)",
            "error: discreteuniformdist: the parameter b is a real number or a name, not true",
        ),
        (
            "rng(1); exprnd(-1)",
            "error: exponentialdist(r) needs r > 0",
        ),
        ("rng(1); normrnd(3, 0)", "1|3"),
        // lists among the parameters: a distribution of every entry
        ("rng(7); poissrnd([1, 2, 3])", "7|[0, 3, 3]"),
        ("poisspdf(2, [1, 2])", "[1/(2e), 2/e^2]"),
        (
            "rng(7); binornd([5, 6, 7], [1/2, 1/3])",
            "error: binomialdist: the lists among the parameters are of one length",
        ),
        // large laws of whole numbers: quick, and the same draws as the slow way gave
        ("rng(7); poissrnd(10000)", "7|9857"),
        ("rng(7); poissrnd(1000000)", "7|998570"),
        ("rng(7); binornd(10000, 1/2)", "7|4928"),
        ("quantile(geometricdist(1/1000000), 1/2)", "693146"),
        ("quantile(negativebinomialdist(20000, 1/2), 1/2)", "19999"),
        // the incomplete beta and gamma functions far out: the series cancelled itself to garbage or never ended
        (
            "N(betaregularized(1/2, 50000, 50001))",
            "0.50126156310709836994276897365706",
        ),
        (
            "N(binocdf(50000, 100000, 1/2))",
            "0.50126156310709836994276897365706",
        ),
        ("N(gammap(5/2, 500000000))", "1"),
        (
            "chi2gof([16, 18, 16, 14, 12, 12], [1000000000, 16, 16, 16, 16, 8])",
            "[1, 3.0278415258651586812365834368744e-186302445, 857954522.13636384581818181818182]",
        ),
        // values that do not differ; a list as an entry of a matrix; an order of a thousand million
        (
            "fitlm([1, 2, 3, 4], [1, 1, 1, 1])",
            "error: fitlm: the values that are to be explained do not differ (regress gives the coefficients)",
        ),
        (
            "getfield(fitlm([1, 2, 3, 4], [1, 2, 3, 4]), \"tstat\")",
            "[undefined, inf]",
        ),
        (
            "lyap([[-2, 1], [[], -1]], [[1, 0], [0, 1]])",
            "error: lyap(A, Q), lyap(A, B, C) or dlyap(A, Q): square matrices",
        ),
        (
            "odetovectorfield(diff(y, x, 1000000000) = -y, y, x)",
            "error: odetovectorfield: a derivative of the order 1000 at most",
        ),
        // Student's law far out: the series in 1/z
        ("N(tcdf(100, 3))", "0.99999889773903840754444131816197"),
        ("N(tcdf(10^20, 4))", "1"),
        (
            "N(hyp2f1(1/2, 2, 3/2, -10^40))",
            "7.8539816339744830961566084581988e-21",
        ),
        // systems where numbers belong; huge delays, poles and counts
        (
            "initial(ss([[0, 1], [-2, -3]], [[0], [1]], [[1, 0], [0, 1]], [[0], [0]]), ss([[0, 1], [-2, -3]], [[0], [1]], [[1, 0], [0, 1]], [[0], [0]]), [1, 0])",
            "error: initial: only the first argument is a system",
        ),
        (
            "impulse(e^(-s)/(s + 1000000000))",
            "e^((-1000000000)(t - 1))*heaviside(t - 1)",
        ),
        (
            "tf([1], [1, -1/2], 1, \"InputDelay\", 1000000000)",
            "error: tf: the delay of a sampled system is a whole number of samples (100000 at most)",
        ),
        ("part(step(tf([1], [1, -1/2], 1), 100000), 100000)", "2"),
        (
            "pdepe(0, @(x, t, u, ux) [1, ux, 0], @(x) sin(pi*x), @(xl, ul, xr, ur, t) [ul, 0, ur, 0], linspace(0, 1, 100000), [0, 1/10])",
            "error: pdepe: 2000 points of the mesh at most",
        ),
    ]);
}
