//! Algebra commands: collected and trigonometric forms, coefficients, closed forms of numbers.

use crate::canon::{add, coeff_and_rest, div, map_bottom_up, mul, neg, pow, sqrt, sub};
use crate::expand::expand;
use crate::expr::Expr;
use crate::funcs::func;
use crate::ntheory;
use crate::num::{Int, Rat};
use crate::numeric;
use crate::ratsimp;
use rug::ops::Pow;
use rug::{Complete, Float};

fn terms_of(e: &Expr) -> Vec<Expr> {
    match e {
        Expr::Add(a) => a.to_vec(),
        other => vec![other.clone()],
    }
}

/// Terms of the expanded expression grouped by the power of `var`.
pub fn collect(e: &Expr, var: &str) -> Expr {
    let x = Expr::sym(var);
    let mut groups: Vec<(Expr, Vec<Expr>)> = Vec::new();
    for t in terms_of(&expand(e)) {
        let factors: Vec<Expr> = match &t {
            Expr::Mul(a) => a.to_vec(),
            other => vec![other.clone()],
        };
        let (mut power, mut rest) = (Vec::new(), Vec::new());
        for f in factors {
            let (b, k) = f.base_exp();
            if b == x && !k.has_sym(var) {
                power.push(k);
            } else {
                rest.push(f);
            }
        }
        let (key, c) = (add(power), mul(rest));
        match groups.iter_mut().find(|g| g.0 == key) {
            Some(g) => g.1.push(c),
            None => groups.push((key, vec![c])),
        }
    }
    add(groups
        .into_iter()
        .map(|(k, cs)| mul(vec![add(cs), pow(x.clone(), k)]))
        .collect())
}

/// Coefficient of `x^n`; the target is `x` with `n`, or the power itself.
pub fn coeff(e: &Expr, target: &Expr, n: Option<&Expr>) -> Option<Expr> {
    let (var, k) = match (target, n) {
        (Expr::Sym(s), Some(n)) => (s.to_string(), n.as_i64()?),
        (Expr::Sym(s), None) => (s.to_string(), 1),
        (Expr::Pow(b, k), None) => (b.sym_name()?.to_string(), k.as_i64()?),
        _ => return None,
    };
    let c = ratsimp::coefficients(e, &var)?;
    Some(
        usize::try_from(k)
            .ok()
            .and_then(|k| c.get(k).cloned())
            .unwrap_or_else(Expr::zero),
    )
}

/// Coefficients in `var` without the vanishing leading ones.
fn trimmed(e: &Expr, var: &str) -> Option<Vec<Expr>> {
    let mut c = ratsimp::coefficients(e, var)?;
    while c.last().is_some_and(crate::simplify::is_zero) {
        c.pop();
    }
    Some(c)
}

pub fn degree(e: &Expr, var: &str) -> Option<Expr> {
    let c = trimmed(e, var)?;
    Some(if c.is_empty() {
        Expr::neg_inf()
    } else {
        Expr::int(c.len() as i64 - 1)
    })
}

pub fn discriminant(e: &Expr, var: &str) -> Option<Expr> {
    let c = trimmed(e, var)?;
    let n = c.len().checked_sub(1).filter(|n| *n >= 1)?;
    if n == 1 {
        return Some(Expr::one());
    }
    let r = ratsimp::resultant(e, &crate::diff::diff(e, var), var)?;
    let sign = if (n * (n - 1) / 2) % 2 == 1 { -1 } else { 1 };
    Some(crate::simplify::normal(&div(
        mul(vec![Expr::int(sign), r]),
        c[n].clone(),
    )))
}

/// Sine and cosine (or their hyperbolic twins) of a sum or a whole multiple, multiplied out.
fn sin_cos(arg: &Expr, hyper: bool) -> (Expr, Expr) {
    let (s, c) = if hyper {
        ("sinh", "cosh")
    } else {
        ("sin", "cos")
    };
    let join = |(s1, c1): (Expr, Expr), (s2, c2): (Expr, Expr)| {
        let cross = &s1 * &s2;
        (
            &(&s1 * &c2) + &(&c1 * &s2),
            if hyper {
                &(&c1 * &c2) + &cross
            } else {
                &(&c1 * &c2) - &cross
            },
        )
    };
    let terms = terms_of(arg);
    if terms.len() > 1 {
        return join(
            sin_cos(&terms[0], hyper),
            sin_cos(&add(terms[1..].to_vec()), hyper),
        );
    }
    let (k, rest) = coeff_and_rest(arg);
    if let Some(n) = k.as_i64().filter(|n| (2..=64).contains(&n.abs()))
        && !rest.is_one()
    {
        let (ss, cc) = join(
            sin_cos(&rest, hyper),
            sin_cos(&mul(vec![Expr::int(n.abs() - 1), rest.clone()]), hyper),
        );
        return if n < 0 { (neg(ss), cc) } else { (ss, cc) };
    }
    (func(s, vec![arg.clone()]), func(c, vec![arg.clone()]))
}

pub fn trigexpand(e: &Expr) -> Expr {
    let out = map_bottom_up(e, &mut |x| {
        let Expr::Fn(n, a) = &x else { return x };
        if a.len() != 1 {
            return x;
        }
        let hyper = matches!(&**n, "sinh" | "cosh" | "tanh");
        if !hyper && !matches!(&**n, "sin" | "cos" | "tan") {
            return x;
        }
        let (s, c) = sin_cos(&a[0], hyper);
        match &**n {
            "sin" | "sinh" => expand(&s),
            "cos" | "cosh" => expand(&c),
            _ => div(expand(&s), expand(&c)),
        }
    });
    expand(&out)
}

/// Products and powers of sines and cosines as sums of multiple angles.
pub fn trigreduce(e: &Expr) -> Expr {
    let i = Expr::i();
    let as_exp = map_bottom_up(e, &mut |x| match &x {
        Expr::Fn(n, a) if a.len() == 1 && matches!(&**n, "sin" | "cos") => {
            let turn = mul(vec![i.clone(), a[0].clone()]);
            let (p, m) = (pow(Expr::e(), turn.clone()), pow(Expr::e(), neg(turn)));
            if &**n == "sin" {
                div(sub(p, m), mul(vec![Expr::int(2), i.clone()]))
            } else {
                div(add(vec![p, m]), Expr::int(2))
            }
        }
        _ => x,
    });
    expand(&crate::integrate::exp_to_trig(&expand(&as_exp)))
}

/// The first continued fraction convergent within `tol` of `x`.
fn simplest_within(x: &Rat, tol: &Rat) -> Rat {
    let (mut p0, mut q0, mut p1, mut q1) = (Int::from(0), Int::from(1), Int::from(1), Int::from(0));
    let mut rest = x.clone();
    loop {
        let whole = rest.clone().floor();
        let a = whole.numer().clone();
        let (p2, q2) = ((&a * &p1).complete() + &p0, (&a * &q1).complete() + &q0);
        let c = Rat::from((p2.clone(), q2.clone()));
        if (c.clone() - x).abs() <= *tol {
            return c;
        }
        (p0, q0, p1, q1) = (p1, q1, p2, q2);
        let frac = rest - whole;
        if frac.is_zero() {
            return x.clone();
        }
        rest = frac.recip();
    }
}

fn power_of_ten(n: u32) -> Int {
    Int::from(Int::u_pow_u(10, n))
}

/// A simple fraction near the number: within the tolerance, or to the working digits.
pub fn rationalize(e: &Expr, tol: Option<&Expr>) -> Result<Expr, String> {
    let digits = numeric::digits();
    let tolerance = match tol {
        Some(t) => Some(
            numeric::real_value(t, 20)
                .and_then(|f| f.to_rational())
                .filter(|r| *r > 0)
                .ok_or("rationalize(x, tolerance): the tolerance must be a positive number")?,
        ),
        None => None,
    };
    let near = |exact: Rat| -> Expr {
        if exact.is_zero() {
            return Expr::zero();
        }
        let tol = tolerance
            .clone()
            .unwrap_or_else(|| exact.clone().abs() / Rat::from(power_of_ten(digits)));
        Expr::rational(simplest_within(&exact, &tol))
    };
    if let Some(n) = e.as_num() {
        if tolerance.is_none() {
            return Ok(e.clone());
        }
        return Ok(add(vec![
            near(n.re.clone()),
            mul(vec![Expr::i(), near(n.im.clone())]),
        ]));
    }
    let v =
        numeric::evalf_digits(e, digits + 8).map_err(|_| format!("{e} has no numeric value"))?;
    let part = |x: &Float| {
        x.to_rational()
            .map(near)
            .ok_or_else(|| format!("{e} is not finite"))
    };
    Ok(add(vec![
        part(v.real())?,
        mul(vec![Expr::i(), part(v.imag())?]),
    ]))
}

/// Continued fraction terms of a real number; an irrational one gives the terms that two precisions agree on.
pub fn cfrac(e: &Expr, terms: usize) -> Result<Vec<Int>, String> {
    if let Some(r) = e.as_rat() {
        return Ok(ntheory::continued_fraction(r)
            .into_iter()
            .take(terms)
            .collect());
    }
    let fail = || format!("cfrac needs a real number, got {e}");
    let mut bits = 128 + 8 * terms as u32;
    let mut best = Vec::new();
    for _ in 0..6 {
        let at = |p: u32| -> Option<Vec<Int>> {
            let v = numeric::evalf(e, p).ok()?;
            v.imag()
                .is_zero()
                .then(|| v.real().to_rational())
                .flatten()
                .map(|r| ntheory::continued_fraction(&r))
        };
        let (a, b) = (at(bits).ok_or_else(fail)?, at(bits + 64).ok_or_else(fail)?);
        let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
        best = a[..common.min(terms)].to_vec();
        if common >= terms {
            break;
        }
        bits *= 2;
    }
    Ok(best)
}

/// A closed form whose value agrees with the real number `x`: to nearly all of `digits` digits,
/// or, for a number written with decimals, to less than `unit`, one unit of its last place.
fn identify_real(x: &Float, digits: u32, unit: Option<&Float>) -> Option<Expr> {
    let prec = x.prec();
    let slack = Float::with_val(prec, 10).pow(-(digits.saturating_sub(4) as i32));
    let max_den = power_of_ten((digits / 4).max(4));
    let near = |c: &Float, t: &Float| match unit {
        // the same share of the value as 5/4 of a unit of the last place is of x: the last
        // digit may be off by one, as in the shortest decimal form of a machine number
        Some(u) => {
            Float::with_val(prec, c - t).abs() * Float::with_val(prec, x.abs_ref()) * 4u32
                < Float::with_val(prec, u * Float::with_val(prec, t.abs_ref())) * 5u32
        }
        None => Float::with_val(prec, c - t).abs() <= Float::with_val(prec, t.abs_ref()) * &slack,
    };
    let close = |cand: &Expr| -> bool {
        numeric::evalf_digits(cand, digits.max(numeric::digits()))
            .is_ok_and(|c| c.imag().is_zero() && near(&Float::with_val(prec, c.real()), x))
    };
    let simple = |t: &Float| -> Option<Rat> {
        let c = ntheory::best_rational(&t.to_rational()?, &max_den);
        near(&Float::with_val(prec, &c), t).then_some(c)
    };
    let value = |e: &Expr| {
        numeric::evalf(e, prec + 32)
            .ok()
            .map(|v| Float::with_val(prec, v.real()))
    };
    let num = |r: Rat| Expr::rational(r);
    // a form says something only when it is much shorter than the digits it explains
    let weight = |e: &Expr| -> usize {
        let mut total = 0;
        e.walk(&mut |node| {
            total += match node {
                Expr::Num(n) => {
                    let count = |v: &Int| {
                        if *v == 1 || *v == 0 {
                            0
                        } else {
                            v.clone().abs().to_string().len()
                        }
                    };
                    count(n.re.numer())
                        + count(n.re.denom())
                        + count(n.im.numer())
                        + count(n.im.denom())
                }
                Expr::Sym(_) | Expr::Fn(..) => 2,
                _ => 0,
            };
        });
        total
    };
    let limit = ((digits as usize * 3) / 5).max(2);
    let accept = |cand: Expr| (weight(&cand) <= limit && close(&cand)).then_some(cand);
    // the first form that fits among the well-known shapes
    let classic = || -> Option<Expr> {
        // fractions with small denominators first: 0.33 is 1/3 before it is 33/100
        for exponent in 1..=4u32 {
            if let Some(q) = x.to_rational() {
                let c = ntheory::best_rational(&q, &power_of_ten(exponent));
                if near(&Float::with_val(prec, &c), x) && weight(&num(c.clone())) <= limit {
                    return Some(num(c));
                }
            }
        }
        if let Some(r) = simple(x)
            && weight(&num(r.clone())) <= limit
        {
            return Some(num(r));
        }
        let f1 = |name: &str, a: Expr| func(name, vec![a]);
        let pi = Expr::pi();
        let constants: Vec<Expr> = vec![
            pi.clone(),
            pow(pi.clone(), Expr::int(2)),
            Expr::e(),
            f1("ln", Expr::int(2)),
            f1("ln", Expr::int(3)),
            f1("ln", Expr::int(10)),
            sqrt(pi.clone()),
            pow(pi.clone(), Expr::int(-1)),
            pow(Expr::e(), Expr::int(-1)),
            Expr::sym("eulergamma"),
            Expr::sym("catalan"),
            f1("zeta", Expr::int(3)),
            pow(pi.clone(), Expr::int(3)),
            pow(pi.clone(), Expr::int(4)),
            pow(Expr::e(), Expr::int(2)),
        ];
        for k in &constants {
            if let Some(kv) = value(k)
                && let Some(r) = simple(&Float::with_val(prec, x / &kv))
                && let Some(hit) = accept(mul(vec![num(r), k.clone()]))
            {
                return Some(hit);
            }
        }
        let sign = if x.is_sign_negative() {
            Expr::minus_one()
        } else {
            Expr::one()
        };
        let xf = x.to_f64();
        for (k, root) in [
            (2u32, Expr::half()),
            (3, Expr::rat(1, 3)),
            (4, Expr::rat(1, 4)),
        ] {
            if let Some(r) = simple(&Float::with_val(prec, x.abs_ref()).pow(k))
                && let Some(hit) = accept(mul(vec![sign.clone(), pow(num(r), root)]))
            {
                return Some(hit);
            }
        }
        if xf.abs() < 100.0
            && let Some(r) = simple(&x.clone().exp())
            && let Some(hit) = accept(f1("ln", num(r)))
        {
            return Some(hit);
        }
        if let Some(r) = simple(&Float::with_val(prec, x.abs_ref()).ln())
            && let Some(hit) = accept(mul(vec![sign.clone(), pow(Expr::e(), num(r))]))
        {
            return Some(hit);
        }
        if xf.abs() < 1.6 {
            for (forward, back) in [
                (x.clone().tan(), "atan"),
                (x.clone().sin(), "asin"),
                (x.clone().cos(), "acos"),
            ] {
                if let Some(r) = simple(&forward)
                    && let Some(hit) = accept(f1(back, num(r)))
                {
                    return Some(hit);
                }
            }
        }
        if xf.abs() < 100.0 && xf.abs() > 1e-3 {
            // a x^2 + b x = c with small whole numbers
            for a in 1..=200i64 {
                for b in -2000..=2000i64 {
                    let v = a as f64 * xf * xf + b as f64 * xf;
                    let c = v.round();
                    if (v - c).abs() > 1e-8 {
                        continue;
                    }
                    let disc = b * b + 4 * a * c as i64;
                    if disc < 0 {
                        continue;
                    }
                    for s in [1, -1] {
                        let cand = div(
                            add(vec![
                                Expr::int(-b),
                                mul(vec![Expr::int(s), sqrt(Expr::int(disc))]),
                            ]),
                            Expr::int(2 * a),
                        );
                        if let Some(hit) = accept(cand) {
                            return Some(hit);
                        }
                    }
                }
            }
            // r1 + r2 K with small fractions
            let mut bases = constants[..6].to_vec();
            bases.extend([2, 3, 5, 7].map(|n| sqrt(Expr::int(n))));
            bases.push(Expr::sym("eulergamma"));
            const L: f64 = 27720.0;
            for k in &bases {
                let Some(kf) = value(k).map(|v| v.to_f64()) else {
                    continue;
                };
                for q in 1..=12i64 {
                    for p in (-12 * q)..=(12 * q) {
                        if p == 0 || Int::from(p).gcd(&Int::from(q)) != 1 {
                            continue;
                        }
                        let w = (xf - p as f64 / q as f64 * kf) * L;
                        if (w - w.round()).abs() > 1e-7 || w.abs() > 1e12 {
                            continue;
                        }
                        let cand = add(vec![
                            Expr::rat(w.round() as i64, L as i64),
                            mul(vec![Expr::rat(p, q), k.clone()]),
                        ]);
                        if let Some(hit) = accept(cand) {
                            return Some(hit);
                        }
                    }
                }
            }
        }
        None
    };
    // every form that fits, with its weight: the lightest one wins
    let mut found: Vec<(usize, Expr)> = Vec::new();
    if let Some(hit) = classic() {
        found.push((weight(&hit), hit));
    }
    // lattice reduction: roots of polynomials with small whole coefficients, combinations of
    // constants, products of powers
    if digits >= 6 && found.iter().all(|(w, _)| *w > 3) {
        // the decimals that are known: the scale of the lattice
        let size = Float::with_val(64, x.abs_ref()).log10().floor().to_f64() as i64;
        let decimals = match unit {
            Some(u) => (-Float::with_val(64, u).log10().to_f64()).round() as i64 - 1,
            None => i64::from(digits) - 4 - size - 1,
        };
        if decimals >= 4 {
            let decimals = decimals as u32;
            // a root of a polynomial weighs what the polynomial does
            if let Some((hit, heavy)) = crate::lattice::algebraic_guess(x, digits, decimals)
                && heavy <= limit
                && close(&hit)
            {
                found.push((heavy, hit));
            }
            for hit in crate::lattice::constant_guess(x, digits, decimals) {
                if let Some(hit) = accept(hit) {
                    found.push((weight(&hit), hit));
                }
            }
        }
    }
    found.sort_by_key(|(w, c)| (*w, crate::simplify::cost(c)));
    found.into_iter().next().map(|(_, c)| c)
}

/// The significant digits and the unit of the last place of a number written with decimals.
fn written(r: &Rat) -> Option<(u32, Rat)> {
    let mut scale = Int::from(1);
    for places in 0..=2000u32 {
        if scale.is_divisible(r.denom()) {
            if places == 0 {
                return None;
            }
            let whole = Int::from(r.numer() * &scale).div_exact(r.denom()).abs();
            let digits = whole.to_string().len() as u32;
            return Some((digits, Rat::from((Int::from(1), scale))));
        }
        scale *= 10u32;
    }
    None
}

/// A closed form guess for a number; the number itself when nothing fits. A number written
/// with decimals is matched to its last place, any other one to the working digits.
pub fn identify(e: &Expr) -> Expr {
    let working = numeric::digits();
    let Ok(v) = numeric::evalf_digits(e, working) else {
        return e.clone();
    };
    let literal = |r: &Rat| written(r).map(|(d, unit)| (d.min(working), unit));
    let (re_info, im_info) = match e.as_num() {
        Some(n) => (literal(&n.re), literal(&n.im)),
        None => (None, None),
    };
    let part = |x: &Float, info: Option<(u32, Rat)>| {
        if x.is_zero() {
            return Some(Expr::zero());
        }
        match info {
            Some((digits, unit)) => {
                identify_real(x, digits, Some(&Float::with_val(x.prec(), &unit)))
            }
            None => identify_real(x, working, None),
        }
    };
    match (part(v.real(), re_info), part(v.imag(), im_info)) {
        (Some(re), Some(im)) => add(vec![re, mul(vec![Expr::i(), im])]),
        _ => e.clone(),
    }
}
