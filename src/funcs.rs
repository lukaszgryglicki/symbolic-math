//! Automatic evaluation of functions: special values, parity, periodicity, inverse compositions.

use crate::assume;
use crate::canon::{add, coeff_and_rest, div, mul, neg, pow, powi, sqrt, sub};
use crate::complex;
use crate::expr::Expr;
use crate::ntheory;
use crate::num::{Flt, Int, Num, Rat};
use crate::numeric;
use crate::tetration;
use rug::Complete;
use rug::ops::Pow as _;
use std::collections::HashMap;

pub fn func(name: &str, args: Vec<Expr>) -> Expr {
    match name {
        "piecewise" => {
            return crate::logic::piecewise(&args).unwrap_or_else(|| Expr::call(name, args));
        }
        "and" | "or" | "not" | "xor" | "implies" => {
            return crate::logic::connective(name, &args).unwrap_or_else(|| Expr::call(name, args));
        }
        _ => {}
    }
    if args.iter().any(Expr::is_undefined) {
        return Expr::undefined();
    }
    if name == "quat" && args.len() == 4 {
        let [a, b, c, d] = <[Expr; 4]>::try_from(args).expect("four components");
        return crate::quat::make([a, b, c, d]);
    }
    if args.first().is_some_and(crate::quat::is_quat)
        && let Some(r) = crate::quat::apply(name, &args)
    {
        return r;
    }
    if let Some(r) = rewrite(name, &args) {
        return r;
    }
    if let Some(r) = float_eval(name, &args) {
        return r;
    }
    let done = match (name, args.len()) {
        ("sin" | "cos" | "tan", 1) => trig(name, &args[0]),
        ("sinh" | "cosh" | "tanh", 1) => hyperbolic(name, &args[0]),
        ("asin" | "acos" | "atan" | "asinh" | "acosh" | "atanh", 1) => inverse(name, &args[0]),
        ("ln", 1) => ln(&args[0]),
        ("abs", 1) => abs(&args[0]),
        ("sign", 1) => sign(&args[0]),
        ("re" | "im", 1) => re_im(&args[0], name == "re"),
        ("conj", 1) => conj(&args[0]),
        ("arg", 1) => arg(&args[0]),
        ("floor" | "ceil" | "round" | "trunc" | "frac", 1) => rounding(name, &args[0]),
        ("factorial", 1) => factorial(&args[0]),
        ("gamma", 1) => gamma(&args[0]),
        ("binomial", 2) => binomial(&args[0], &args[1]),
        ("min" | "max", _) => min_max(name, &args),
        ("mod", 2) => modulo(&args[0], &args[1]),
        ("atan2", 2) => atan2(&args[0], &args[1]),
        ("erf", 1) => match &args[0] {
            Expr::Fn(inner, u) if &**inner == "erfinv" && u.len() == 1 => Some(u[0].clone()),
            Expr::Fn(inner, u) if &**inner == "erfcinv" && u.len() == 1 => {
                Some(sub(Expr::one(), u[0].clone()))
            }
            _ => odd_special(
                name,
                &args[0],
                &[(Expr::zero(), Expr::zero()), (Expr::inf(), Expr::one())],
            ),
        },
        ("si", 1) => odd_special(
            name,
            &args[0],
            &[(Expr::zero(), Expr::zero()), (Expr::inf(), Expr::pi() / 2)],
        ),
        ("shi", 1) => odd_special(
            name,
            &args[0],
            &[(Expr::zero(), Expr::zero()), (Expr::inf(), Expr::inf())],
        ),
        ("ci", 1) => table(
            &args[0],
            &[(Expr::inf(), Expr::zero()), (Expr::zero(), Expr::neg_inf())],
        ),
        ("chi", 1) => table(
            &args[0],
            &[(Expr::inf(), Expr::inf()), (Expr::zero(), Expr::neg_inf())],
        ),
        ("ei", 1) => table(
            &args[0],
            &[
                (Expr::zero(), Expr::neg_inf()),
                (Expr::inf(), Expr::inf()),
                (Expr::neg_inf(), Expr::zero()),
            ],
        ),
        ("li", 1) => table(
            &args[0],
            &[
                (Expr::zero(), Expr::zero()),
                (Expr::one(), Expr::neg_inf()),
                (Expr::inf(), Expr::inf()),
            ],
        ),
        ("zeta", 1) => zeta(&args[0]),
        ("gamma", 2 | 3) => Some(func("uppergamma", args.clone())),
        ("beta", 3) => Some(func("betainc", args.clone())),
        ("lambertw", 1) => lambertw(&args[0]),
        ("lambertw", 2) if args[0].is_zero() => Some(func("lambertw", vec![args[1].clone()])),
        ("harmonic", 1) => args[0]
            .as_i64()
            .filter(|n| (0..=100_000).contains(n))
            .map(|n| ntheory::harmonic(n as u32).into()),
        ("digamma", 1) => digamma(&args[0]),
        ("polygamma", 2) => polygamma(&args[0], &args[1]),
        ("tet" | "slog" | "ssrt", 2) => tetration::simplify(name, &args[0], &args[1]),
        _ => number_theory(name, &args).or_else(|| crate::specfn::value(name, &args)),
    };
    done.unwrap_or_else(|| Expr::call(name, args))
}

/// Whole-number functions of exact arguments.
fn number_theory(name: &str, a: &[Expr]) -> Option<Expr> {
    use crate::ntheory as nt;
    let int = |i: usize| a.get(i).and_then(Expr::as_int);
    let index = |i: usize, max: i64| {
        a.get(i)
            .and_then(Expr::as_i64)
            .filter(|n| (0..=max).contains(n))
            .map(|n| n as u32)
    };
    let whole = |i: usize| {
        a.get(i)
            .and_then(Expr::as_rat)
            .map(|r| r.clone().floor().numer().clone())
    };
    let nonzero = |i: usize| int(i).filter(|n| !n.is_zero());
    let radix = |i: usize| {
        a.get(i)
            .and_then(Expr::as_i64)
            .filter(|b| (2..=1_000_000_000).contains(b))
            .map(|b| b as u32)
    };
    let num = Expr::integer;
    let small = |n: i32| Expr::int(i64::from(n));
    let or_undefined = |v: Option<Int>| v.map_or_else(Expr::undefined, Expr::integer);
    let list = |v: Vec<Int>| Expr::list(v.into_iter().map(Expr::integer).collect());
    Some(match (name, a.len()) {
        ("isprime", 1) => Expr::Bool(a[0].as_num()?.as_int().is_some_and(nt::is_prime)),
        ("issquare", 1) => Expr::Bool(a[0].as_num()?.as_int().is_some_and(nt::is_square)),
        ("issquarefree", 1) => Expr::Bool(nt::is_squarefree(int(0)?)?),
        ("nextprime", 1) => num(nt::next_prime(&whole(0)?)),
        ("prevprime", 1) => or_undefined(nt::prev_prime(a[0].as_rat()?.clone().ceil().numer())),
        ("prime", 1) => num(nt::nth_prime(
            int(0)?
                .to_u64()
                .filter(|n| (1..=nt::NTH_PRIME_MAX).contains(n))?,
        )?),
        ("primepi", 1) => num(nt::prime_pi(&whole(0)?)?),
        ("divisors", 1) => list(nt::divisors(nonzero(0)?)?),
        ("totient", 1) if a[0].is_zero() => Expr::zero(),
        ("totient", 1) => num(nt::totient(int(0)?)?),
        ("sigma", 1 | 2) => {
            let k = if a.len() == 2 { index(1, 10_000)? } else { 1 };
            num(nt::divisor_sigma(k, nonzero(0)?)?)
        }
        ("mobius", 1) if a[0].is_zero() => Expr::zero(),
        ("mobius", 1) => small(nt::mobius(int(0)?)?),
        ("primenu", 1) => Expr::int(i64::from(nt::omega(nonzero(0)?)?)),
        ("primeomega", 1) => Expr::int(i64::from(nt::big_omega(nonzero(0)?)?)),
        ("radical", 1) => num(nt::radical(nonzero(0)?)?),
        ("perfectpower", 1) => {
            let n = int(0)?;
            let (b, e) = nt::is_perfect_power(n).unwrap_or((n.clone(), 1));
            Expr::list(vec![num(b), Expr::int(i64::from(e))])
        }
        ("fibonacci" | "lucas", 1) => {
            let n = a[0].as_i64().filter(|n| n.abs() <= 10_000_000)?;
            num(if name == "fibonacci" {
                nt::fibonacci(n)
            } else {
                nt::lucas(n)
            })
        }
        ("bernoulli", 1) => Expr::rational(nt::bernoulli(index(0, 6000)?)),
        ("eulernumber", 1) => num(nt::euler_number(index(0, 6000)?)),
        ("catalannumber", 1) => num(nt::catalan_number(index(0, 1_000_000)?)),
        ("bell", 1) => num(nt::bell_number(index(0, 5000)?)),
        ("stirling1" | "stirling2", 2) => {
            let (n, k) = (index(0, 3000)?, index(1, 3000)?);
            num(if name == "stirling1" {
                nt::stirling1(n, k)
            } else {
                nt::stirling2(n, k)
            })
        }
        ("partitions", 1) => num(nt::partitions(index(0, 200_000)?)),
        ("derangements", 1) => num(nt::derangements(index(0, 100_000)?)),
        ("multinomial", n) if n >= 1 => {
            let ks: Vec<u32> = (0..n).map(|i| index(i, 100_000)).collect::<Option<_>>()?;
            num(nt::multinomial(&ks))
        }
        ("powmod", 3) => or_undefined(nt::pow_mod(int(0)?, int(1)?, int(2)?)),
        ("modinv", 2) => or_undefined(nt::mod_inverse(int(0)?, int(1)?)),
        ("jacobi", 2) => nt::jacobi(int(0)?, int(1)?).map_or_else(Expr::undefined, small),
        ("kronecker", 2) => small(nt::kronecker(int(0)?, int(1)?)),
        ("sqrtmod", 2) => or_undefined(nt::sqrt_mod(int(0)?, int(1)?)),
        ("ord", 2) => or_undefined(nt::multiplicative_order(int(0)?, int(1)?)),
        ("primitiveroot", 1) => or_undefined(nt::primitive_root(int(0)?)),
        ("discretelog", 3) => match nt::discrete_log_search(int(0)?, int(1)?, int(2)?) {
            Ok(x) => num(x),
            Err(true) => Expr::undefined(),
            Err(false) => return None,
        },
        ("crt", 2) => {
            let (rs, ms) = (a[0].as_list()?, a[1].as_list()?);
            if rs.len() != ms.len() {
                return None;
            }
            let pairs: Vec<(Int, Int)> = rs
                .iter()
                .zip(ms)
                .map(|(r, m)| Some((r.as_int()?.clone(), m.as_int()?.clone())))
                .collect::<Option<_>>()?;
            or_undefined(nt::crt(&pairs).map(|(x, _)| x))
        }
        ("egcd", 2) => {
            let (g, x, y) = nt::extended_gcd(int(0)?, int(1)?);
            list(vec![g, x, y])
        }
        ("isqrt", 1) => num(nt::isqrt(&whole(0)?)?),
        ("iroot", 2) => num(nt::iroot(&whole(0)?, index(1, 1_000_000)?)?),
        ("digitsum", 1 | 2) => num(nt::digit_sum(
            int(0)?,
            if a.len() == 2 { radix(1)? } else { 10 },
        )?),
        ("sumoftwosquares", 1) => {
            nt::sum_of_two_squares(int(0)?).map_or_else(Expr::undefined, |(x, y)| list(vec![x, y]))
        }
        ("factorial2", 1) => num(nt::double_factorial(
            a[0].as_i64().filter(|n| *n <= 100_000)?,
        )?),
        ("gcd" | "lcm", n) if n >= 1 => {
            let rs: Vec<&Rat> = a.iter().map(Expr::as_rat).collect::<Option<_>>()?;
            let (mut top, mut low) = (rs[0].numer().clone().abs(), rs[0].denom().clone());
            for r in &rs[1..] {
                if name == "gcd" {
                    top = top.gcd(r.numer());
                    low = low.lcm(r.denom());
                } else {
                    top = top.lcm(r.numer());
                    low = low.gcd(r.denom());
                }
            }
            Expr::rational(Rat::from((top, low)))
        }
        ("carmichael", 1) => num(nt::carmichael_lambda(nonzero(0)?)?),
        ("liouville", 1) => small(if nt::big_omega(nonzero(0)?)? % 2 == 0 {
            1
        } else {
            -1
        }),
        ("mangoldt" | "isprimepower", 1) => {
            let n = int(0)?;
            let base = nt::is_perfect_power(n).map_or_else(|| n.clone(), |(b, _)| b);
            let hit = *n > 1 && nt::is_prime(&base);
            match (name, hit) {
                ("isprimepower", _) => Expr::Bool(hit),
                (_, true) => func("ln", vec![num(base)]),
                _ => Expr::zero(),
            }
        }
        ("iscomposite", 1) => {
            let n = a[0].as_num()?.as_int();
            Expr::Bool(n.is_some_and(|n| *n > 1 && !nt::is_prime(n)))
        }
        ("isperfect", 1) => {
            let n = int(0)?;
            Expr::Bool(n.is_positive() && nt::divisor_sigma(1, n)? == Int::from(n << 1u32))
        }
        ("iscoprime", n) if n >= 2 => {
            let ns: Vec<&Int> = a.iter().map(Expr::as_int).collect::<Option<_>>()?;
            Expr::Bool((0..n).all(|i| (0..i).all(|j| ns[i].clone().gcd(ns[j]) == 1)))
        }
        ("primes", 1 | 2) => {
            let (lo, hi) = if a.len() == 1 {
                (Int::from(2), whole(0)?)
            } else {
                (a[0].as_rat()?.clone().ceil().numer().clone(), whole(1)?)
            };
            list(nt::primes_between(&lo, &hi, 100_000)?)
        }
        ("primefactors", 1) => list(
            nt::factor(nonzero(0)?)?
                .into_iter()
                .map(|(p, _)| p)
                .collect(),
        ),
        ("ifactors", 1) => Expr::list(
            nt::factor(nonzero(0)?)?
                .into_iter()
                .map(|(p, e)| Expr::list(vec![num(p), Expr::int(i64::from(e))]))
                .collect(),
        ),
        ("primorial", 1) => num(Int::from(Int::primorial(index(0, 10_000_000)?))),
        ("legendre", 2) => {
            let p = int(1)?;
            if *p > 2 && nt::is_prime(p) {
                nt::jacobi(int(0)?, p).map_or_else(Expr::undefined, small)
            } else {
                Expr::undefined()
            }
        }
        ("farey", 1) => Expr::list(
            nt::farey(index(0, 300).filter(|n| *n >= 1)?)
                .into_iter()
                .map(Expr::rational)
                .collect(),
        ),
        ("squaresr", 2) => num(nt::squares_r(index(0, 10_000)?, int(1)?)?),
        ("partitionsq", 1) => num(nt::partitions_distinct(index(0, 10_000)?)),
        ("eulerian", 2) => num(nt::eulerian(index(0, 3000)?, index(1, 3000)?)),
        ("superfactorial" | "hyperfactorial", 1) => {
            let n = index(0, 2000)?;
            let mut product = Int::from(1);
            let mut fact = Int::from(1);
            for k in 1..=n {
                if name == "superfactorial" {
                    fact *= k;
                    product *= &fact;
                } else {
                    product *= Int::from(k).pow(k);
                }
            }
            num(product)
        }
        ("pell", 1 | 2) => {
            let d = int(0)?;
            let Some((x1, y1)) = nt::pell(d) else {
                return Some(Expr::undefined());
            };
            let n = if a.len() == 2 {
                index(1, 100_000).filter(|n| *n >= 1)?
            } else {
                1
            };
            let (mut x, mut y) = (x1.clone(), y1.clone());
            for _ in 1..n {
                let next = Int::from(&x1 * &x) + Int::from(&y1 * &y) * d;
                y = Int::from(&x1 * &y) + Int::from(&y1 * &x);
                x = next;
            }
            list(vec![x, y])
        }
        ("egyptian", 1) => Expr::list(
            nt::egyptian(a[0].as_rat()?, 64)?
                .into_iter()
                .map(Expr::rational)
                .collect(),
        ),
        ("primitiveroots", 1) => list(nt::all_primitive_roots(int(0)?, 100_000)?),
        ("sqrtmodall", 2) => list(nt::sqrt_mod_all(int(0)?, int(1)?, 100_000)?),
        ("pochhammer" | "fallingfactorial", 2) => {
            let step = if name == "pochhammer" { 1 } else { -1 };
            let shifted = |k: i64| add(vec![a[0].clone(), Expr::int(step * k)]);
            match a[1].as_i64() {
                Some(n) if (0..=1000).contains(&n) => mul((0..n).map(shifted).collect()),
                Some(n) if (-1000..0).contains(&n) => {
                    powi(mul((1..=-n).map(|k| shifted(-k)).collect()), -1)
                }
                Some(_) => return None,
                None if name == "pochhammer" => div(
                    func("gamma", vec![add(vec![a[0].clone(), a[1].clone()])]),
                    func("gamma", vec![a[0].clone()]),
                ),
                None => div(
                    func("gamma", vec![add(vec![a[0].clone(), Expr::one()])]),
                    func(
                        "gamma",
                        vec![add(vec![a[0].clone(), neg(a[1].clone()), Expr::one()])],
                    ),
                ),
            }
        }
        ("base", 2) => {
            let b = radix(1)?;
            let mut n = int(0)?.clone().abs();
            let mut digits = Vec::new();
            while !n.is_zero() {
                digits.push(Expr::integer(Int::from(n.mod_u(b))));
                n /= b;
            }
            if digits.is_empty() {
                digits.push(Expr::zero());
            }
            digits.reverse();
            Expr::list(digits)
        }
        ("frombase", 2) => {
            let b = radix(1)?;
            let mut n = Int::new();
            for d in a[0].as_list()? {
                n = n * b + d.as_int().filter(|d| !d.is_negative() && **d < b)?;
            }
            num(n)
        }
        _ => return None,
    })
}

/// The number 0, exact or as a decimal.
fn nothing(e: &Expr) -> bool {
    e.is_zero() || matches!(e, Expr::Float(f) if f.v.is_zero())
}

fn rewrite(name: &str, a: &[Expr]) -> Option<Expr> {
    let f = |n: &str, x: Expr| func(n, vec![x]);
    let x = || a[0].clone();
    Some(match (name, a.len()) {
        ("sqrt", 1) => sqrt(x()),
        ("cbrt", 1) => pow(x(), Expr::rat(1, 3)),
        ("root", 2) => pow(x(), div(Expr::one(), a[1].clone())),
        ("exp", 1) => pow(Expr::e(), x()),
        ("log", 1) => f("ln", x()),
        ("log", 2) => div(f("ln", x()), f("ln", a[1].clone())),
        ("log2", 1) => div(f("ln", x()), f("ln", Expr::int(2))),
        ("log10", 1) => div(f("ln", x()), f("ln", Expr::int(10))),
        ("sec", 1) => powi(f("cos", x()), -1),
        ("csc", 1) => powi(f("sin", x()), -1),
        ("cot", 1) => powi(f("tan", x()), -1),
        ("sech", 1) => powi(f("cosh", x()), -1),
        ("csch", 1) => powi(f("sinh", x()), -1),
        ("coth", 1) => powi(f("tanh", x()), -1),
        // at 0 the reciprocal is no number: the values that are usual there
        ("acot", 1) if nothing(&a[0]) => div(Expr::pi(), Expr::int(2)),
        ("acoth", 1) if nothing(&a[0]) => mul(vec![Expr::i(), div(Expr::pi(), Expr::int(2))]),
        ("asech", 1) if nothing(&a[0]) => Expr::inf(),
        ("asec" | "acsc" | "acsch", 1) if nothing(&a[0]) => Expr::sym(crate::expr::ZOO),
        ("asec", 1) => f("acos", powi(x(), -1)),
        ("acsc", 1) => f("asin", powi(x(), -1)),
        ("acot", 1) => f("atan", powi(x(), -1)),
        ("asech", 1) => f("acosh", powi(x(), -1)),
        ("acsch", 1) => f("asinh", powi(x(), -1)),
        ("acoth", 1) => f("atanh", powi(x(), -1)),
        // far to the right 1 - erf cancels to nothing: erfc stays a function there
        ("erfc", 1)
            if a[0].is_const() && numeric::real_value(&a[0], 20).is_some_and(|v| v > 3u32) =>
        {
            return None;
        }
        ("erfc", 1) => sub(Expr::one(), f("erf", x())),
        ("erfi", 1) => mul(vec![neg(Expr::i()), f("erf", mul(vec![Expr::i(), x()]))]),
        ("beta", 2) => div(
            mul(vec![f("gamma", x()), f("gamma", a[1].clone())]),
            f("gamma", add(vec![x(), a[1].clone()])),
        ),
        ("lgamma", 1) if a[0].as_int().is_some_and(|n| n.is_positive()) => f("ln", f("gamma", x())),
        _ => return None,
    })
}

fn float_eval(name: &str, args: &[Expr]) -> Option<Expr> {
    let like = args.iter().find_map(|a| match a {
        Expr::Float(f) => Some(f.clone()),
        _ => None,
    })?;
    if !args.iter().all(Expr::is_const) {
        return None;
    }
    let v = numeric::evalf(&Expr::call(name, args.to_vec()), like.v.prec().0).ok()?;
    Some(Expr::float(Flt {
        v,
        digits: like.digits,
    }))
}

/// Printed with a leading minus sign.
pub fn is_neg_form(e: &Expr) -> bool {
    match e {
        Expr::Num(n) => n.is_neg_form(),
        Expr::Float(f) => {
            let (re, im) = (f.v.real(), f.v.imag());
            if re.is_zero() {
                im.is_sign_negative() && !im.is_zero()
            } else {
                re.is_sign_negative()
            }
        }
        Expr::Mul(a) => a[0].is_number() && is_neg_form(&a[0]),
        Expr::Add(a) => a.iter().find(|t| !t.is_number()).is_some_and(is_neg_form),
        _ => false,
    }
}

/// `x` with `e = i*x` when the numeric coefficient of `e` is purely imaginary.
fn imaginary_part_of(e: &Expr) -> Option<Expr> {
    let (c, rest) = coeff_and_rest(e);
    c.is_imag().then(|| mul(vec![Expr::rational(c.im), rest]))
}

/// `r` when `e = r*pi` with a rational `r`.
pub fn pi_multiple(e: &Expr) -> Option<Rat> {
    if e.is_zero() {
        return Some(Rat::new());
    }
    let (c, rest) = coeff_and_rest(e);
    (rest.is_sym("pi") && c.is_real()).then_some(c.re)
}

fn root(n: i64) -> Expr {
    sqrt(Expr::int(n))
}

fn sin_table() -> Vec<(Rat, Expr)> {
    let q = |p: i64, d: i64| Rat::from((p, d));
    vec![
        (q(0, 1), Expr::zero()),
        (q(1, 2), Expr::one()),
        (q(1, 6), Expr::half()),
        (q(1, 4), root(2) / 2),
        (q(1, 3), root(3) / 2),
        (q(1, 12), (root(6) - root(2)) / 4),
        (q(5, 12), (root(6) + root(2)) / 4),
        (q(1, 10), (root(5) - 1) / 4),
        (q(3, 10), (root(5) + 1) / 4),
        (q(1, 5), sqrt(Expr::int(10) - root(5) * 2) / 4),
        (q(2, 5), sqrt(Expr::int(10) + root(5) * 2) / 4),
        (q(1, 8), sqrt(Expr::int(2) - root(2)) / 2),
        (q(3, 8), sqrt(Expr::int(2) + root(2)) / 2),
    ]
}

thread_local! {
    static SIN: Vec<(Rat, Expr)> = sin_table();
    static ASIN: HashMap<Expr, Rat> = sin_table().into_iter().map(|(r, v)| (v, r)).collect();
    static ATAN: HashMap<Expr, Rat> = {
        let q = |p: i64, d: i64| Rat::from((p, d));
        [
            (Expr::one(), q(1, 4)),
            (root(3), q(1, 3)),
            (root(3) / 3, q(1, 6)),
            (Expr::int(2) - root(3), q(1, 12)),
            (Expr::int(2) + root(3), q(5, 12)),
            (root(2) - 1, q(1, 8)),
            (root(2) + 1, q(3, 8)),
        ]
        .into_iter()
        .collect()
    };
}

/// sin(r*pi): sign, reduced `t` in [0, 1/2] and the exact value when it is tabulated.
fn sin_pi(r: &Rat) -> (i32, Rat, Option<Expr>) {
    let two = Rat::from(2);
    let mut t = r - (r / &two).complete().floor() * &two;
    let mut s = 1;
    if t >= 1 {
        t -= 1;
        s = -1;
    }
    if t > 0.5f64 {
        t = Rat::from(1) - t;
    }
    let hit = SIN.with(|tab| tab.iter().find(|(q, _)| *q == t).map(|(_, v)| v.clone()));
    (s, t, hit)
}

fn signed(s: i32, e: Expr) -> Expr {
    if s < 0 { neg(e) } else { e }
}

fn sin_of_pi(r: &Rat) -> Expr {
    let (s, t, hit) = sin_pi(r);
    signed(
        s,
        hit.unwrap_or_else(|| Expr::call("sin", vec![mul(vec![Expr::rational(t), Expr::pi()])])),
    )
}

fn cos_of_pi(r: &Rat) -> Expr {
    let (s, t, hit) = sin_pi(&(Rat::from((1, 2)) - r));
    match hit {
        Some(v) => signed(s, v),
        None => signed(
            s,
            Expr::call(
                "cos",
                vec![mul(vec![Expr::rational(Rat::from((1, 2)) - t), Expr::pi()])],
            ),
        ),
    }
}

/// `t/pi` when `pi` is a factor of the term.
fn pi_coefficient(t: &Expr) -> Option<Expr> {
    match t {
        Expr::Sym(_) if t.is_sym("pi") => Some(Expr::one()),
        Expr::Mul(a) if a.iter().filter(|f| f.is_sym("pi")).count() == 1 => {
            Some(mul(a.iter().filter(|f| !f.is_sym("pi")).cloned().collect()))
        }
        _ => None,
    }
}

/// `arg = rest + shift*pi + m*pi`: rational shift, parity sign `(-1)^m` of the integer multiples.
fn split_pi(arg: &Expr) -> (Expr, Rat, Expr, bool) {
    let terms: Vec<Expr> = match arg {
        Expr::Add(a) => a.to_vec(),
        _ => vec![arg.clone()],
    };
    let (mut rest, mut shift, mut parity, mut found) = (Vec::new(), Rat::new(), Expr::one(), false);
    for t in terms {
        if let Some(q) = pi_coefficient(&t) {
            if let Some(r) = q.as_rat() {
                shift += r;
                found = true;
                continue;
            }
            if assume::is_integer(&q) {
                let (c, m) = coeff_and_rest(&q);
                if c.as_int().is_some_and(|k| k.is_odd()) {
                    parity = mul(vec![parity, pow(Expr::minus_one(), m)]);
                }
                found = true;
                continue;
            }
        }
        rest.push(t);
    }
    (add(rest), shift, parity, found)
}

fn trig(name: &str, x: &Expr) -> Option<Expr> {
    let f = |n: &str, a: Expr| func(n, vec![a]);
    if x.is_zero() {
        return Some(if name == "cos" {
            Expr::one()
        } else {
            Expr::zero()
        });
    }
    if x.is_infinite() {
        return Some(if x.is_zoo() {
            Expr::undefined()
        } else {
            Expr::call(name, vec![x.clone()])
        });
    }
    if let Some(y) = imaginary_part_of(x) {
        return Some(match name {
            "sin" => mul(vec![Expr::i(), f("sinh", y)]),
            "cos" => f("cosh", y),
            _ => mul(vec![Expr::i(), f("tanh", y)]),
        });
    }
    if is_neg_form(x) {
        let pos = f(name, neg(x.clone()));
        return Some(if name == "cos" { pos } else { neg(pos) });
    }
    if name == "cos"
        && let Some(u) = x.fn_arg("abs")
        && assume::is_real(u)
    {
        return Some(f("cos", u.clone()));
    }
    let (rest, shift, parity, found) = split_pi(x);
    if found {
        if rest.is_zero() {
            let (s, c) = (sin_of_pi(&shift), cos_of_pi(&shift));
            return Some(match name {
                "sin" => mul(vec![parity, s]),
                "cos" => mul(vec![parity, c]),
                _ if c.is_zero() => Expr::zoo(),
                _ if s.has_fn("sin") || c.has_fn("cos") => {
                    let half = Rat::from((1, 2));
                    let t = &shift - (&shift + &half).complete().floor();
                    if t == shift {
                        return None;
                    }
                    f("tan", mul(vec![Expr::rational(t), Expr::pi()]))
                }
                _ => div(s, c),
            });
        }
        let two = Rat::from(2);
        let t = &shift - (&shift / &two).complete().floor() * &two;
        let quarter = if t.is_zero() {
            Some(0)
        } else if t == 0.5f64 {
            Some(1)
        } else if t == 1 {
            Some(2)
        } else if t == 1.5f64 {
            Some(3)
        } else {
            None
        };
        if let Some(k) = quarter {
            let (s, c) = (f("sin", rest.clone()), f("cos", rest.clone()));
            let r = match (name, k) {
                ("sin", 0) => s,
                ("sin", 1) => c,
                ("sin", 2) => neg(s),
                ("sin", _) => neg(c),
                ("cos", 0) => c,
                ("cos", 1) => neg(s),
                ("cos", 2) => neg(c),
                ("cos", _) => s,
                (_, 0 | 2) => return Some(f("tan", rest)),
                _ => return Some(neg(powi(f("tan", rest), -1))),
            };
            return Some(mul(vec![parity, r]));
        }
        let reduced = if t > 1 { t - &two } else { t };
        if reduced != shift || !parity.is_one() {
            let inner = f(
                name,
                add(vec![rest, mul(vec![Expr::rational(reduced), Expr::pi()])]),
            );
            return Some(if name == "tan" {
                inner
            } else {
                mul(vec![parity, inner])
            });
        }
    }
    if let Expr::Fn(inner, a) = x
        && a.len() == 1
    {
        let u = a[0].clone();
        let c = |u: &Expr| sqrt(sub(Expr::one(), powi(u.clone(), 2)));
        let h = |u: &Expr| sqrt(add(vec![Expr::one(), powi(u.clone(), 2)]));
        return match (name, &**inner) {
            ("sin", "asin") | ("cos", "acos") | ("tan", "atan") => Some(u),
            ("sin", "acos") | ("cos", "asin") => Some(c(&u)),
            ("sin", "atan") => Some(div(u.clone(), h(&u))),
            ("cos", "atan") => Some(powi(h(&u), -1)),
            ("tan", "asin") => Some(div(u.clone(), c(&u))),
            ("tan", "acos") => Some(div(c(&u), u)),
            _ => None,
        };
    }
    None
}

fn hyperbolic(name: &str, x: &Expr) -> Option<Expr> {
    let f = |n: &str, a: Expr| func(n, vec![a]);
    if x.is_zero() {
        return Some(if name == "cosh" {
            Expr::one()
        } else {
            Expr::zero()
        });
    }
    if x.is_inf() || x.is_neg_inf() {
        return Some(match name {
            "cosh" => Expr::inf(),
            "sinh" => x.clone(),
            _ => Expr::int(if x.is_inf() { 1 } else { -1 }),
        });
    }
    if let Some(y) = imaginary_part_of(x) {
        return Some(match name {
            "sinh" => mul(vec![Expr::i(), f("sin", y)]),
            "cosh" => f("cos", y),
            _ => mul(vec![Expr::i(), f("tan", y)]),
        });
    }
    if is_neg_form(x) {
        let pos = f(name, neg(x.clone()));
        return Some(if name == "cosh" { pos } else { neg(pos) });
    }
    if name == "cosh"
        && let Some(u) = x.fn_arg("abs")
        && assume::is_real(u)
    {
        return Some(f("cosh", u.clone()));
    }
    // of c ln(u): the powers u^c and u^(-c)
    let (c, rest) = coeff_and_rest(x);
    if let Some(u) = rest.fn_arg("ln")
        && c.im.is_zero()
    {
        let up = pow(u.clone(), Expr::num(c.clone()));
        let down = pow(u.clone(), Expr::num(c.neg()));
        return Some(match name {
            "sinh" => div(sub(up, down), Expr::int(2)),
            "cosh" => div(add(vec![up, down]), Expr::int(2)),
            _ => {
                let square = pow(u.clone(), Expr::num(c.add(&c)));
                div(
                    sub(square.clone(), Expr::one()),
                    add(vec![square, Expr::one()]),
                )
            }
        });
    }
    if let Expr::Fn(inner, a) = x
        && a.len() == 1
    {
        let u = a[0].clone();
        return match (name, &**inner) {
            ("sinh", "asinh") | ("cosh", "acosh") | ("tanh", "atanh") => Some(u),
            ("cosh", "asinh") => Some(sqrt(add(vec![powi(u, 2), Expr::one()]))),
            ("sinh", "acosh") => Some(mul(vec![sqrt(u.clone() - 1), sqrt(u + 1)])),
            _ => None,
        };
    }
    None
}

fn inverse(name: &str, x: &Expr) -> Option<Expr> {
    let f = |n: &str, a: Expr| func(n, vec![a]);
    let pi_times = |r: Rat| mul(vec![Expr::rational(r), Expr::pi()]);
    // asin(sin(c)) for a real constant c: c moved into the principal range
    if let Expr::Fn(inner, a) = x
        && matches!(
            (name, &**inner),
            ("asin", "sin") | ("acos", "cos") | ("atan", "tan")
        )
        && let Some(v) = a.first().and_then(|u| numeric::real_value(u, 20))
    {
        let u = a[0].clone();
        let period = if name == "acos" { 2.0 } else { 1.0 };
        let turns = v.to_f64() / (std::f64::consts::PI * period);
        let k = turns.round();
        if (turns - k).abs() < 0.499 && k.abs() < 1e15 {
            let moved = sub(u, mul(vec![Expr::int((k * period) as i64), Expr::pi()]));
            let flip = match name {
                "asin" => (k as i64) % 2 != 0,
                "acos" => turns < k,
                _ => false,
            };
            return Some(if flip { neg(moved) } else { moved });
        }
    }
    if x.is_zero() {
        return Some(match name {
            "acos" => Expr::pi() / 2,
            "acosh" => mul(vec![Expr::i(), Expr::pi()]) / 2,
            _ => Expr::zero(),
        });
    }
    if name == "acos" {
        if x.is_one() {
            return Some(Expr::zero());
        }
        let s = f("asin", x.clone());
        return (!s.has_fn("asin")).then(|| sub(Expr::pi() / 2, s));
    }
    if name == "acosh" {
        return x
            .is_one()
            .then(Expr::zero)
            .or_else(|| x.is_inf().then(Expr::inf));
    }
    if name == "atan" && (x.is_inf() || x.is_neg_inf()) {
        return Some(signed(if x.is_inf() { 1 } else { -1 }, Expr::pi() / 2));
    }
    if name == "asinh" && (x.is_inf() || x.is_neg_inf()) {
        return Some(x.clone());
    }
    if name == "atanh" && (x.is_one() || x.is_minus_one()) {
        return Some(if x.is_one() {
            Expr::inf()
        } else {
            Expr::neg_inf()
        });
    }
    if let Some(y) = imaginary_part_of(x) {
        let partner = match name {
            "asin" => "asinh",
            "atan" => "atanh",
            "asinh" => "asin",
            _ => "atan",
        };
        return Some(mul(vec![Expr::i(), f(partner, y)]));
    }
    if is_neg_form(x) {
        return Some(neg(f(name, neg(x.clone()))));
    }
    // asin(sin(r pi)), asin(cos(r pi)), atan(tan(r pi)) for a rational r
    if let Expr::Fn(inner, a) = x
        && a.len() == 1
        && let Some(r) = pi_multiple(&a[0])
    {
        let half = Rat::from((1, 2));
        let wrap = |t: Rat, period: u32| -> Rat {
            let p = Rat::from(period);
            let mut t = &t - (&t / &p).complete().floor() * &p;
            if t > (&p / 2u32).complete() {
                t -= &p;
            }
            t
        };
        match (name, &**inner) {
            ("asin", "sin" | "cos") => {
                let s = wrap(
                    if &**inner == "cos" {
                        half.clone() - &r
                    } else {
                        r
                    },
                    2,
                );
                let folded = if s > half {
                    Rat::from(1) - s
                } else if s < (-&half).complete() {
                    Rat::from(-1) - s
                } else {
                    s
                };
                return Some(pi_times(folded));
            }
            ("atan", "tan") => return Some(pi_times(wrap(r, 1))),
            _ => {}
        }
    }
    match name {
        "asin" => ASIN.with(|t| t.get(x).cloned()).map(pi_times),
        "atan" => ATAN.with(|t| t.get(x).cloned()).map(pi_times),
        _ => None,
    }
}

fn ln_rational(r: &Rat) -> Expr {
    let mut terms = Vec::new();
    for (n, s) in [(r.numer(), 1i64), (r.denom(), -1)] {
        for (p, k) in crate::canon::factor_int(n).iter() {
            terms.push(mul(vec![
                Expr::int(s * i64::from(*k)),
                Expr::call("ln", vec![Expr::integer(p.clone())]),
            ]));
        }
    }
    add(terms)
}

fn ln(x: &Expr) -> Option<Expr> {
    let i_pi = || mul(vec![Expr::i(), Expr::pi()]);
    match x {
        Expr::Num(n) => {
            if n.is_zero() {
                return Some(Expr::neg_inf());
            }
            if n.is_real() {
                return Some(if n.is_positive() {
                    ln_rational(&n.re)
                } else {
                    add(vec![ln_rational(&(-&n.re).complete()), i_pi()])
                });
            }
            if n.re.is_zero() {
                let mag = n.im.clone().abs();
                return Some(add(vec![
                    ln_rational(&mag),
                    signed(if n.im.is_positive() { 1 } else { -1 }, i_pi() / 2),
                ]));
            }
            let angle = arg(x)?;
            (!angle.has_fn("arg")).then(|| {
                add(vec![
                    ln_rational(&n.norm()) / 2,
                    mul(vec![Expr::i(), angle]),
                ])
            })
        }
        Expr::Sym(s) => match &**s {
            "e" => Some(Expr::one()),
            "inf" => Some(Expr::inf()),
            _ => None,
        },
        Expr::Pow(b, e) => {
            let pos_base = b.is_sym("e")
                || b.is_sym("pi")
                || b.as_num().is_some_and(Num::is_positive)
                || assume::is_positive(b);
            let real_exp = e.as_rat().is_some() || (pos_base && assume::is_real(e));
            if pos_base && real_exp {
                return Some(mul(vec![(**e).clone(), func("ln", vec![(**b).clone()])]));
            }
            if b.is_sym("e")
                && let Some(v) = numeric::real_value(&func("im", vec![(**e).clone()]), 20)
                && let Some(pi) = numeric::real_value(&Expr::pi(), 20)
                && v.clone().abs() < pi
            {
                return Some((**e).clone());
            }
            None
        }
        Expr::Mul(a) if a.len() == 2 && a[0].is_minus_one() && a[1].is_inf() => Some(Expr::inf()),
        // the logarithm of a product of positive factors is the sum of theirs
        Expr::Mul(a) if a.iter().all(|f| !f.is_infinite() && assume::is_positive(f)) => {
            Some(add(a.iter().map(|f| func("ln", vec![f.clone()])).collect()))
        }
        _ => None,
    }
}

fn abs(x: &Expr) -> Option<Expr> {
    match x {
        Expr::Num(n) => Some(if n.is_real() {
            Expr::rational(n.re.clone().abs())
        } else {
            sqrt(Expr::rational(n.norm()))
        }),
        Expr::Mul(a) if a[0].as_num().is_some() => {
            let (c, rest) = coeff_and_rest(x);
            Some(mul(vec![
                func("abs", vec![Expr::num(c)]),
                func("abs", vec![rest]),
            ]))
        }
        Expr::Pow(b, e) if e.as_int().is_some() => {
            Some(pow(func("abs", vec![(**b).clone()]), (**e).clone()))
        }
        Expr::Pow(b, e) if b.is_sym("e") && assume::is_real(e) => Some(x.clone()),
        Expr::Fn(n, _) if &**n == "abs" => Some(x.clone()),
        Expr::Fn(n, a) if &**n == "conj" => Some(func("abs", vec![a[0].clone()])),
        _ if x.is_infinite() => Some(Expr::inf()),
        _ => match assume::sign(x).or_else(|| if x.is_const() { numeric::sign(x) } else { None }) {
            Some(s) if s >= 0 => Some(x.clone()),
            Some(_) => Some(neg(x.clone())),
            None if assume::is_nonnegative(x) => Some(x.clone()),
            None => {
                let (re, im) = complex::rect(x).filter(|_| x.is_const())?;
                // (a real value of an unknown sign stays as it is: its root of a square
                // would be its absolute value again)
                if im.is_zero() {
                    return None;
                }
                Some(sqrt(add(vec![powi(re, 2), powi(im, 2)])))
            }
        },
    }
}

fn sign(x: &Expr) -> Option<Expr> {
    if let Some(n) = x.as_num() {
        if n.is_zero() {
            return Some(Expr::zero());
        }
        return Some(if n.is_real() {
            Expr::int(n.re.cmp0() as i64)
        } else {
            div(x.clone(), func("abs", vec![x.clone()]))
        });
    }
    if x.is_inf() || x.is_neg_inf() {
        return Some(Expr::int(if x.is_inf() { 1 } else { -1 }));
    }
    match assume::sign(x).or_else(|| if x.is_const() { numeric::sign(x) } else { None }) {
        Some(s) => Some(Expr::int(i64::from(s))),
        None if is_neg_form(x) => Some(neg(func("sign", vec![neg(x.clone())]))),
        None => None,
    }
}

fn re_im(x: &Expr, want_re: bool) -> Option<Expr> {
    let name = if want_re { "re" } else { "im" };
    if assume::is_real(x) {
        return Some(if want_re { x.clone() } else { Expr::zero() });
    }
    match x {
        Expr::Num(n) => Some(Expr::rational(if want_re {
            n.re.clone()
        } else {
            n.im.clone()
        })),
        Expr::Add(a) => Some(add(a.iter().map(|t| func(name, vec![t.clone()])).collect())),
        Expr::Mul(a) if a[0].as_num().is_some() => {
            let (c, rest) = coeff_and_rest(x);
            let (re, im) = (func("re", vec![rest.clone()]), func("im", vec![rest]));
            let (a, b) = (Expr::rational(c.re), Expr::rational(c.im));
            Some(if want_re {
                sub(mul(vec![a, re]), mul(vec![b, im]))
            } else {
                add(vec![mul(vec![a, im]), mul(vec![b, re])])
            })
        }
        _ => complex::rect(x).map(|(re, im)| if want_re { re } else { im }),
    }
}

fn conj(x: &Expr) -> Option<Expr> {
    if assume::is_real(x) {
        return Some(x.clone());
    }
    let c = |t: &Expr| func("conj", vec![t.clone()]);
    match x {
        Expr::Num(n) => Some(Expr::num(n.conj())),
        Expr::Add(a) => Some(add(a.iter().map(c).collect())),
        Expr::Mul(a) => Some(mul(a.iter().map(c).collect())),
        Expr::Pow(b, e) if e.as_int().is_some() => Some(pow(c(b), (**e).clone())),
        Expr::Pow(b, e) if b.is_sym("e") => Some(pow(Expr::e(), c(e))),
        Expr::Fn(n, a) if &**n == "conj" => Some(a[0].clone()),
        Expr::Fn(n, a) if matches!(&**n, "sin" | "cos" | "tan" | "sinh" | "cosh" | "tanh") => {
            Some(func(n, vec![c(&a[0])]))
        }
        _ => complex::rect(x).map(|(re, im)| sub(re, mul(vec![Expr::i(), im]))),
    }
}

fn arg(x: &Expr) -> Option<Expr> {
    if let Some(s) = assume::sign(x).or_else(|| if x.is_const() { numeric::sign(x) } else { None })
    {
        return Some(match s {
            1 => Expr::zero(),
            -1 => Expr::pi(),
            _ => Expr::undefined(),
        });
    }
    let (re, im) = complex::rect(x).filter(|_| x.is_const())?;
    let (sr, si) = (numeric::sign(&re)?, numeric::sign(&im)?);
    if sr == 0 {
        return Some(signed(si, Expr::pi() / 2));
    }
    if si != 0
        && let Some(angle) = polar_angle(&re, &im)
    {
        return Some(angle);
    }
    let base = func("atan", vec![div(im, re)]);
    Some(match (sr, si) {
        (1, _) => base,
        (_, s) if s >= 0 => add(vec![base, Expr::pi()]),
        _ => sub(base, Expr::pi()),
    })
}

/// The angle of r*cos(a) + i*r*sin(a), where a is t, -t, pi/2 - t or t - pi/2 for a real number
/// t that stands in the parts: a + n*pi in (-pi, pi].
fn polar_angle(re: &Expr, im: &Expr) -> Option<Expr> {
    let mut angles: Vec<Expr> = Vec::new();
    for part in [re, im] {
        part.walk(&mut |e| {
            if let Expr::Fn(n, a) = e
                && matches!(&**n, "sin" | "cos")
                && a.len() == 1
                && a[0].is_const()
                && !angles.contains(&a[0])
            {
                angles.push(a[0].clone());
            }
        });
    }
    let digits = numeric::digits();
    for t in angles {
        if !assume::is_real(&t) {
            continue;
        }
        let (s, c) = (func("sin", vec![t.clone()]), func("cos", vec![t.clone()]));
        let quarter = Expr::pi() / 2;
        // (the angle with its sine and cosine)
        let forms = [
            (t.clone(), s.clone(), c.clone()),
            (neg(t.clone()), neg(s.clone()), c.clone()),
            (sub(quarter.clone(), t.clone()), c.clone(), s.clone()),
            (sub(t.clone(), quarter), neg(c), s),
        ];
        for (angle, sine, cosine) in forms {
            let cross = sub(mul(vec![re.clone(), sine]), mul(vec![im.clone(), cosine]));
            if !cross.is_zero() {
                continue;
            }
            // the angle itself tells which a + n*pi it is
            let part = |e: &Expr| {
                numeric::evalf_digits(e, digits)
                    .ok()
                    .map(|z| z.real().clone())
            };
            let (x, y, at) = (part(re)?, part(im)?, part(&angle)?);
            let pi = rug::Float::with_val(x.prec(), rug::float::Constant::Pi);
            let turns = ((y.atan2(&x) - at) / pi).round().to_integer()?;
            return Some(add(vec![
                angle,
                mul(vec![Expr::integer(turns), Expr::pi()]),
            ]));
        }
    }
    None
}

fn atan2(y: &Expr, x: &Expr) -> Option<Expr> {
    if !(y.is_const() && x.is_const()) {
        return None;
    }
    if y.is_zero() && x.is_zero() {
        return Some(Expr::undefined());
    }
    arg(&add(vec![x.clone(), mul(vec![Expr::i(), y.clone()])]))
}

fn round_rat(name: &str, r: &Rat) -> Rat {
    match name {
        "floor" => r.clone().floor(),
        "ceil" => r.clone().ceil(),
        "round" => r.clone().round(),
        "trunc" => r.clone().trunc(),
        _ => r - r.clone().trunc(),
    }
}

fn rounding(name: &str, x: &Expr) -> Option<Expr> {
    if let Some(n) = x.as_num() {
        return Some(Expr::num(Num::new(
            round_rat(name, &n.re),
            round_rat(name, &n.im),
        )));
    }
    if assume::is_integer(x) {
        return Some(if name == "frac" {
            Expr::zero()
        } else {
            x.clone()
        });
    }
    if !x.is_const() || x.is_infinite() {
        return None;
    }
    let (re, im) = complex::rect(x)?;
    let part = |e: &Expr| -> Option<Rat> {
        if let Some(r) = e.as_rat() {
            return Some(round_rat(name, r));
        }
        let v = numeric::real_value(e, numeric::digits().max(32) + 8)?;
        let nearest = v.clone().round();
        if (v.clone() - &nearest).abs() < 1e-20 {
            return None;
        }
        let whole = Rat::from(
            match name {
                "floor" => v.floor(),
                "ceil" => v.ceil(),
                "round" => nearest,
                _ => v.trunc(),
            }
            .to_integer()?,
        );
        Some(whole)
    };
    let (a, b) = (part(&re)?, part(&im)?);
    let whole = Expr::num(Num::new(a, b));
    Some(if name == "frac" {
        sub(x.clone(), whole)
    } else {
        whole
    })
}

fn factorial(x: &Expr) -> Option<Expr> {
    if let Some(n) = x.as_int() {
        return Some(match n.to_u32() {
            Some(k) if k <= 200_000 => Expr::integer(Int::from(Int::factorial(k))),
            None if n.is_negative() => Expr::zoo(),
            _ => return None,
        });
    }
    if x.as_rat().is_some_and(|r| *r.denom() == 2) {
        return gamma(&add(vec![x.clone(), Expr::one()]));
    }
    x.is_inf().then(Expr::inf)
}

fn gamma(x: &Expr) -> Option<Expr> {
    if let Some(n) = x.as_int() {
        return Some(match n.to_u32() {
            Some(k) if (1..=200_001).contains(&k) => {
                Expr::integer(Int::from(Int::factorial(k - 1)))
            }
            Some(0) | None if !n.is_positive() => Expr::zoo(),
            _ => return None,
        });
    }
    if let Some(r) = x.as_rat()
        && *r.denom() == 2
    {
        let n = (r - Rat::from((1, 2))).numer().to_i64()?;
        let m = u32::try_from(n.unsigned_abs())
            .ok()
            .filter(|m| *m <= 5000)?;
        let ratio = Rat::from((
            Int::from(Int::factorial(2 * m)),
            Int::from(Int::factorial(m)) * (Int::from(4).pow(m)),
        ));
        let c = if n >= 0 {
            ratio
        } else {
            ratio.recip() * (if m % 2 == 0 { 1 } else { -1 })
        };
        return Some(mul(vec![Expr::rational(c), sqrt(Expr::pi())]));
    }
    x.is_inf().then(Expr::inf)
}

fn binomial(n: &Expr, k: &Expr) -> Option<Expr> {
    if let Some(kk) = k.as_int() {
        if kk.is_negative() {
            return assume::is_integer(n).then(Expr::zero);
        }
        if kk.is_zero() {
            return Some(Expr::one());
        }
        if *kk == 1 {
            return Some(n.clone());
        }
        if let (Some(nn), Some(k32)) = (n.as_int(), kk.to_u32()) {
            return Some(Expr::integer(nn.clone().binomial(k32)));
        }
        if let (Some(r), Some(k32)) = (n.as_rat(), kk.to_u32().filter(|k| *k <= 1000)) {
            let mut acc = Rat::from(1);
            for j in 0..k32 {
                acc *= (r - Rat::from(j)) / Rat::from(j + 1);
            }
            return Some(Expr::rational(acc));
        }
    }
    (n == k).then(Expr::one)
}

fn min_max(name: &str, args: &[Expr]) -> Option<Expr> {
    if args.is_empty() {
        return None;
    }
    if args.len() == 1 {
        return Some(args[0].clone());
    }
    let mut best = args[0].clone();
    for a in &args[1..] {
        if *a == best {
            continue;
        }
        let s = numeric::sign(&sub(a.clone(), best.clone()))?;
        if (name == "min" && s < 0) || (name == "max" && s > 0) {
            best = a.clone();
        }
    }
    Some(best)
}

fn modulo(a: &Expr, m: &Expr) -> Option<Expr> {
    if let (Expr::Pow(b, e), Some(m)) = (a, m.as_int())
        && let (Some(b), Some(e)) = (b.as_int(), e.as_int())
        && !m.is_zero()
        && e.is_positive()
    {
        let r = crate::ntheory::pow_mod(b, e, m)?;
        return Some(Expr::integer(if m.is_negative() && !r.is_zero() {
            r + m
        } else {
            r
        }));
    }
    let (x, y) = (a.as_rat()?, m.as_rat()?);
    if y.is_zero() {
        return Some(Expr::undefined());
    }
    Some(Expr::rational(x - (x / y).complete().floor() * y))
}

fn table(x: &Expr, entries: &[(Expr, Expr)]) -> Option<Expr> {
    entries.iter().find(|(k, _)| k == x).map(|(_, v)| v.clone())
}

fn odd_special(name: &str, x: &Expr, entries: &[(Expr, Expr)]) -> Option<Expr> {
    if is_neg_form(x) {
        return Some(neg(func(name, vec![neg(x.clone())])));
    }
    table(x, entries)
}

fn zeta(s: &Expr) -> Option<Expr> {
    if s.is_inf() {
        return Some(Expr::one());
    }
    let n = s.as_i64()?;
    if n == 1 {
        return Some(Expr::zoo());
    }
    if n == 0 {
        return Some(Expr::rat(-1, 2));
    }
    if !(-2000..=2000).contains(&n) {
        return None;
    }
    if n < 0 {
        let m = (1 - n) as u32;
        return Some(Expr::rational(-ntheory::bernoulli(m) / Rat::from(m)));
    }
    if n % 2 == 1 {
        return None;
    }
    let m = n as u32;
    let c = ntheory::bernoulli(m).abs() * Rat::from(Int::from(1) << (m - 1))
        / Rat::from(Int::from(Int::factorial(m)));
    Some(mul(vec![Expr::rational(c), powi(Expr::pi(), n)]))
}

fn lambertw(x: &Expr) -> Option<Expr> {
    if x.is_zero() {
        return Some(Expr::zero());
    }
    if x.is_sym("e") {
        return Some(Expr::one());
    }
    if *x == neg(powi(Expr::e(), -1)) {
        return Some(Expr::minus_one());
    }
    if x.is_inf() {
        return Some(Expr::inf());
    }
    // W(a*ln(a)) = ln(a) for a >= 1/e, W(a*e^a) = a for a >= -1.
    let (c, rest) = coeff_and_rest(x);
    if let Some(u) = rest.fn_arg("ln")
        && u.as_num() == Some(&c)
        && c.is_real()
    {
        return Some(rest);
    }
    if let Expr::Pow(b, e) = &rest
        && b.is_sym("e")
        && **e == Expr::num(c.clone())
        && c.as_rat().is_some_and(|r| *r >= -1)
    {
        return Some(Expr::num(c));
    }
    None
}

fn digamma(x: &Expr) -> Option<Expr> {
    let gamma = || Expr::sym("eulergamma");
    if let Some(r) = x.as_rat()
        && *r.denom() == 2
    {
        // psi(n + 1/2) = -gamma - 2 ln 2 + 2 sum_{k=1}^{n} 1/(2k - 1)
        let n = (r - Rat::from((1, 2)))
            .numer()
            .to_i64()
            .filter(|n| (0..=100_000).contains(n))?;
        let mut odd = Rat::new();
        for k in 1..=n {
            odd += Rat::from((2, 2 * k - 1));
        }
        return Some(add(vec![
            Expr::rational(odd),
            neg(gamma()),
            mul(vec![Expr::int(-2), func("ln", vec![Expr::int(2)])]),
        ]));
    }
    if let Some(r) = x.as_rat()
        && let Some(q) = r
            .denom()
            .to_i64()
            .filter(|q| [3, 4, 5, 6, 8, 10, 12].contains(q))
    {
        // Gauss: psi(p/q) = -gamma - ln(2q) - (pi/2) cot(pi p/q) + 2 sum_{n < q/2} cos(2 pi n p/q) ln sin(pi n/q)
        let whole = r.clone().floor();
        let m = whole.numer().to_i64().filter(|m| m.abs() <= 1000)?;
        let p = (r.clone() - whole).numer().to_i64()?;
        let turn = |a: i64, b: i64| mul(vec![Expr::rat(a, b), Expr::pi()]);
        let mut terms = vec![
            neg(gamma()),
            neg(func("ln", vec![Expr::int(2 * q)])),
            mul(vec![
                Expr::rat(-1, 2),
                Expr::pi(),
                div(func("cos", vec![turn(p, q)]), func("sin", vec![turn(p, q)])),
            ]),
        ];
        for n in 1..=(q - 1) / 2 {
            terms.push(mul(vec![
                Expr::int(2),
                func("cos", vec![turn(2 * n * p, q)]),
                func("ln", vec![func("sin", vec![turn(n, q)])]),
            ]));
        }
        // psi(x + 1) = psi(x) + 1/x moves the argument into (0, 1)
        let base = Rat::from((p, q));
        for j in 0..m.max(0) {
            terms.push(Expr::rational((base.clone() + Rat::from(j)).recip()));
        }
        for j in 1..=(-m).max(0) {
            terms.push(Expr::rational(-(base.clone() - Rat::from(j)).recip()));
        }
        return Some(add(terms));
    }
    let n = x.as_i64()?;
    if n <= 0 {
        return Some(Expr::zoo());
    }
    if n > 100_000 {
        return None;
    }
    Some(sub(
        Expr::rational(ntheory::harmonic(n as u32 - 1)),
        gamma(),
    ))
}

/// `psi^(m)(x)` at whole and half-whole `x`.
fn polygamma(m: &Expr, x: &Expr) -> Option<Expr> {
    let m = m.as_i64().filter(|m| (0..=200).contains(m))?;
    if m == 0 {
        return Some(func("digamma", vec![x.clone()]));
    }
    let r = x.as_rat()?;
    let half = *r.denom() == 2;
    if !half && !r.is_integer() {
        return None;
    }
    let n = if half {
        (r - Rat::from((1, 2))).numer().to_i64()?
    } else {
        r.numer().to_i64()?
    };
    if !half && n <= 0 {
        return Some(Expr::zoo());
    }
    if !(0..=100_000).contains(&n) {
        return None;
    }
    let s = m + 1;
    let sign = if m % 2 == 1 { 1 } else { -1 };
    let factorial = Expr::integer(Int::from(Int::factorial(m as u32)));
    let zeta = func("zeta", vec![Expr::int(s)]);
    // psi^(m)(1) = (-1)^(m+1) m! zeta(m+1); psi^(m)(1/2) = (-1)^(m+1) m! (2^(m+1) - 1) zeta(m+1)
    let (base, mut partial) = if half {
        (
            mul(vec![Expr::integer((Int::from(1) << (s as u32)) - 1), zeta]),
            Rat::new(),
        )
    } else {
        (zeta, Rat::new())
    };
    let start: Rat = if half {
        Rat::from((1, 2))
    } else {
        Rat::from(1)
    };
    let steps = if half { n } else { n - 1 };
    for k in 0..steps {
        partial += (start.clone() + Rat::from(k)).recip().pow(s as u32);
    }
    Some(mul(vec![
        Expr::int(sign),
        factorial,
        sub(base, Expr::rational(partial)),
    ]))
}

/// `(-1)^r` in rectangular form when its sine and cosine are tabulated.
pub fn root_of_unity(r: &Rat) -> Option<Expr> {
    let (c, s) = (cos_of_pi(r), sin_of_pi(r));
    (!c.has_fn("cos") && !s.has_fn("sin")).then(|| add(vec![c, mul(vec![Expr::i(), s])]))
}

pub fn cos_pi(r: &Rat) -> Expr {
    cos_of_pi(r)
}
pub fn sin_pi_value(r: &Rat) -> Expr {
    sin_of_pi(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_absolute_value_of_what_has_no_sign_stays() {
        // the real part of something that is no number is real, and its sign is not known:
        // it must not go round between the absolute value and the root of the square
        let real = func("re", vec![Expr::Bool(true)]);
        let abs = func("abs", vec![real.clone()]);
        assert_eq!(abs, Expr::call("abs", vec![real.clone()]));
        assert_eq!(func("sqrt", vec![pow(real.clone(), Expr::int(2))]), abs);
        // what has a sign loses it
        assert_eq!(func("abs", vec![Expr::int(-3)]), Expr::int(3));
        assert_eq!(
            func(
                "abs",
                vec![sub(func("sqrt", vec![Expr::int(2)]), Expr::int(2))]
            ),
            sub(Expr::int(2), func("sqrt", vec![Expr::int(2)]))
        );
    }
}
