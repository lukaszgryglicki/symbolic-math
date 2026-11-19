//! Integral transforms (Laplace, Fourier, Z) with their inverses, residues, Pade approximants
//! and Fourier series.

use crate::assume::{self, Facts};
use crate::canon::{add, div, map_bottom_up, mul, neg, pow, powi, sqrt, sub, subst};
use crate::diff::{diff, diff_n};
use crate::expand::expand;
use crate::expr::Expr;
use crate::funcs::func;
use crate::integrate;
use crate::numeric;
use crate::ratsimp;
use crate::series;
use crate::simplify::simplify;
use rug::Integer;

type Res = Result<Expr, String>;

const S: &str = "laplace_s";
const U: &str = "transform_u";

fn f1(name: &str, x: Expr) -> Expr {
    func(name, vec![x])
}

fn int(k: i64) -> Expr {
    Expr::int(k)
}

fn factorial(n: i64) -> Expr {
    Expr::integer(Integer::from(Integer::factorial(n as u32)))
}

fn e_to(x: Expr) -> Expr {
    pow(Expr::e(), x)
}

/// `(a, b)` for `u = a t + b`.
fn linear(u: &Expr, t: &str) -> Option<(Expr, Expr)> {
    let cs = ratsimp::coefficients(u, t)?;
    match cs.as_slice() {
        [b] => Some((Expr::zero(), b.clone())),
        [b, a] => Some((a.clone(), b.clone())),
        _ => None,
    }
}

fn factors_of(e: &Expr) -> Vec<Expr> {
    match e {
        Expr::Mul(fs) => fs.to_vec(),
        other => vec![other.clone()],
    }
}

/// Runs `f` with the internal names taken as positive (transform variables) or real.
fn with_facts<T>(names: &[(&str, Facts)], f: impl FnOnce() -> T) -> T {
    for (n, facts) in names {
        assume::set(n, *facts);
    }
    let out = f();
    for (n, _) in names {
        assume::clear(n);
    }
    out
}

fn has_fn_of(e: &Expr, names: &[&str], var: &str) -> bool {
    e.any(&|x| matches!(x, Expr::Fn(n, a) if names.contains(&&**n) && a.iter().any(|y| y.has_sym(var))))
}

/// The Laplace transform: the integral of `f(t) e^(-s t)` from 0 to infinity.
pub fn laplace(f: &Expr, t: &str, s: &Expr) -> Option<Expr> {
    let r = with_facts(&[(S, Facts::positive())], || {
        laplace_inner(&simplify(f), t, &Expr::sym(S), 0)
    })?;
    Some(simplify(&ratsimp::together(&subst(&r, &Expr::sym(S), s))))
}

fn laplace_inner(f: &Expr, t: &str, s: &Expr, depth: u32) -> Option<Expr> {
    if depth > 12 {
        return None;
    }
    if !f.has_sym(t) {
        return Some(div(f.clone(), s.clone()));
    }
    if let Expr::Add(terms) = f {
        let parts: Option<Vec<Expr>> = terms
            .iter()
            .map(|x| laplace_inner(x, t, s, depth + 1))
            .collect();
        return parts.map(add);
    }
    let tsym = Expr::sym(t);
    let (mut constant, mut rate, mut level, mut rest) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut power = 0i64;
    for fac in factors_of(f) {
        if !fac.has_sym(t) {
            constant.push(fac);
            continue;
        }
        match &fac {
            Expr::Pow(b, u) if b.is_sym("e") && linear(u, t).is_some() => {
                let (a, b0) = linear(u, t).expect("checked");
                rate.push(a);
                level.push(b0);
            }
            Expr::Sym(_) => power += 1,
            Expr::Pow(b, n) if b.is_sym(t) && n.as_i64().is_some_and(|n| n > 0) => {
                power += n.as_i64().expect("checked")
            }
            _ => rest.push(fac),
        }
    }
    let c = mul(vec![mul(constant), e_to(add(level))]);
    let a = simplify(&add(rate));
    let g = mul(rest);
    if !a.is_zero() {
        // e^(a t) g(t): the transform of g at s - a
        let inner = simplify(&mul(vec![powi(tsym, power), g]));
        let v = laplace_inner(&inner, t, &sub(s.clone(), a), depth + 1)?;
        return Some(mul(vec![c, v]));
    }
    if g.is_one() {
        return Some(mul(vec![
            c,
            factorial(power),
            powi(s.clone(), -(power + 1)),
        ]));
    }
    if power > 0 {
        // t^n g(t): (-1)^n times the n-th derivative of the transform of g
        let u = Expr::sym(U);
        let base = laplace_inner(&g, t, &u, depth + 1)?;
        let d = diff_n(&base, U, power as u32);
        let sign = if power % 2 == 0 { 1 } else { -1 };
        return Some(mul(vec![c, int(sign), subst(&d, &u, s)]));
    }
    laplace_table(&g, t, s, depth).map(|v| mul(vec![c, v]))
}

fn laplace_table(g: &Expr, t: &str, s: &Expr, depth: u32) -> Option<Expr> {
    let tsym = Expr::sym(t);
    let s2 = || powi(s.clone(), 2);
    if let Expr::Pow(b, a) = g
        && b.is_sym(t)
        && !a.has_sym(t)
    {
        // t^a: gamma(a + 1)/s^(a + 1) for a > -1
        let a1 = add(vec![(**a).clone(), Expr::one()]);
        if a.is_const() && numeric::sign(&a1) != Some(1) {
            return None;
        }
        return Some(div(f1("gamma", a1.clone()), pow(s.clone(), a1)));
    }
    if let Expr::Fn(name, args) = g {
        let last = args.last()?;
        let line = linear(last, t);
        match (&**name, args.len(), line) {
            ("sin" | "cos", 1, Some((a, b))) => {
                let den = add(vec![s2(), powi(a.clone(), 2)]);
                let (sb, cb) = (f1("sin", b.clone()), f1("cos", b));
                let num = if &**name == "sin" {
                    add(vec![mul(vec![a, cb]), mul(vec![s.clone(), sb])])
                } else {
                    sub(mul(vec![s.clone(), cb]), mul(vec![a, sb]))
                };
                return Some(div(num, den));
            }
            ("ln", 1, Some((a, b))) if b.is_zero() => {
                return Some(div(
                    sub(
                        f1("ln", a),
                        add(vec![Expr::sym("eulergamma"), f1("ln", s.clone())]),
                    ),
                    s.clone(),
                ));
            }
            ("besselj" | "besseli", 2, Some((a, b)))
                if b.is_zero() && args[0].as_i64().is_some_and(|n| (0..=50).contains(&n)) =>
            {
                let n = args[0].as_i64().expect("checked");
                let a2 = powi(a.clone(), 2);
                let root = sqrt(if &**name == "besselj" {
                    add(vec![s2(), a2])
                } else {
                    sub(s2(), a2)
                });
                let top = if &**name == "besselj" {
                    sub(root.clone(), s.clone())
                } else {
                    sub(s.clone(), root.clone())
                };
                return Some(div(powi(top, n), mul(vec![powi(a, n), root])));
            }
            ("erf", 1, Some((a, b))) if b.is_zero() => {
                let half = div(s.clone(), mul(vec![int(2), a]));
                return Some(div(
                    mul(vec![
                        e_to(powi(half.clone(), 2)),
                        sub(Expr::one(), f1("erf", half)),
                    ]),
                    s.clone(),
                ));
            }
            ("sinc", 1, Some((a, b))) if b.is_zero() => {
                return Some(div(f1("atan", div(a.clone(), s.clone())), a));
            }
            ("heaviside" | "unitstep", 1, Some((a, b))) if a.is_one() => {
                // step at t = -b, which must not be negative
                if numeric::sign(&b).is_some_and(|x| x > 0) {
                    return None;
                }
                return Some(div(e_to(mul(vec![b, s.clone()])), s.clone()));
            }
            ("diracdelta", 1, Some((a, b))) if a.is_one() => {
                if numeric::sign(&b).is_some_and(|x| x > 0) {
                    return None;
                }
                return Some(e_to(mul(vec![b, s.clone()])));
            }
            _ => {}
        }
    }
    if let Expr::Mul(fs) = g {
        // a step or an impulse at t = c >= 0 times h(t)
        for (k, fac) in fs.iter().enumerate() {
            let Expr::Fn(name, args) = fac else { continue };
            if !matches!(&**name, "heaviside" | "unitstep" | "diracdelta") || args.len() != 1 {
                continue;
            }
            let Some((a, b)) = linear(&args[0], t) else {
                continue;
            };
            if !a.is_one() || numeric::sign(&b).is_some_and(|x| x > 0) {
                continue;
            }
            let at = neg(b.clone());
            let h = mul(fs
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != k)
                .map(|(_, x)| x.clone())
                .collect());
            let delay = e_to(mul(vec![b, s.clone()]));
            if &**name == "diracdelta" {
                return Some(mul(vec![simplify(&subst(&h, &tsym, &at)), delay]));
            }
            let moved = simplify(&subst(&h, &tsym, &add(vec![tsym.clone(), at])));
            return Some(mul(vec![delay, laplace_inner(&moved, t, s, depth + 1)?]));
        }
    }
    // g(t)/t: the integral of the transform of g from s to infinity
    if let Expr::Mul(fs) = g
        && let Some(k) = fs
            .iter()
            .position(|x| matches!(x, Expr::Pow(b, n) if b.is_sym(t) && n.is_minus_one()))
    {
        let h = mul(fs
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != k)
            .map(|(_, x)| x.clone())
            .collect());
        let u = Expr::sym(U);
        let inner = laplace_inner(&h, t, &u, depth + 1)?;
        if let Some(v) = with_facts(&[(U, Facts::positive())], || {
            integrate::definite_with_parameters(&simplify(&inner), U, s, &Expr::inf())
        }) {
            return Some(v);
        }
    }
    // hyperbolic and circular functions through exponentials
    if has_fn_of(g, &["sin", "cos", "sinh", "cosh"], t) {
        let reduced = crate::algebra::trigreduce(g);
        let ex = expand(&crate::solve::rewrite_exp(&reduced));
        if ex != *g && !has_fn_of(&ex, &["sin", "cos", "sinh", "cosh"], t) {
            let v = laplace_inner(&ex, t, s, depth + 1)?;
            return Some(ratsimp::together(&v));
        }
    }
    // the integral itself
    let integrand = simplify(&mul(vec![g.clone(), e_to(neg(mul(vec![s.clone(), tsym])))]));
    integrate::definite_with_parameters(&integrand, t, &Expr::zero(), &Expr::inf())
}

/// A pole with the coefficients `c_1, c_2, ...` of its principal part.
pub type Pole = (Expr, Vec<Expr>);

/// The poles of a rational function with the coefficients of their principal parts
/// `c_1/(x - p) + c_2/(x - p)^2 + ...`, and the polynomial part.
pub fn principal_parts(r: &Expr, var: &str) -> Option<(Expr, Vec<Pole>)> {
    let (num, den) = ratsimp::fraction(&ratsimp::together(r));
    let lower = ratsimp::coefficients(&den, var)?;
    let upper = ratsimp::coefficients(&num, var)?;
    if lower.len() <= 1 {
        return Some((r.clone(), Vec::new()));
    }
    let (quotient, rem) = if upper.len() >= lower.len() {
        ratsimp::div_rem(&num, &den, var)?
    } else {
        (Expr::zero(), num.clone())
    };
    let solved = crate::solve::solve(&den, var).ok()?;
    crate::solve::release(&solved.params);
    if solved.all || !solved.params.is_empty() {
        return None;
    }
    let count: u32 = solved.roots.iter().map(|r| r.mult).sum();
    if count as usize != lower.len() - 1 {
        return None;
    }
    let x = Expr::sym(var);
    let slope = diff(&den, var);
    let mut parts = Vec::new();
    for root in &solved.roots {
        let p = &root.value;
        if root.mult == 1 {
            let c = simplify(&div(subst(&rem, &x, p), subst(&slope, &x, p)));
            parts.push((p.clone(), vec![c]));
            continue;
        }
        let shifted = subst(
            &div(rem.clone(), den.clone()),
            &x,
            &add(vec![p.clone(), Expr::sym(U)]),
        );
        let s = series::expand(&shifted, U, root.mult as usize + 2)?.trim();
        if s.den != 1 {
            return None;
        }
        let mut cs = Vec::new();
        for j in 1..=i64::from(root.mult) {
            let at = -j - s.lo;
            cs.push(match usize::try_from(at).ok().and_then(|k| s.c.get(k)) {
                Some(c) => simplify(c),
                None => Expr::zero(),
            });
        }
        parts.push((p.clone(), cs));
    }
    Some((quotient, parts))
}

/// Complex exponentials written with sines and cosines, and the sum tidied.
pub(crate) fn real_form(e: &Expr) -> Expr {
    let spread = map_bottom_up(e, &mut |x| match &x {
        Expr::Pow(b, p) if b.is_sym("e") => pow(Expr::e(), expand(p)),
        _ => x,
    });
    let trig = integrate::exp_to_trig(&spread);
    let out = simplify(&expand(&trig));
    if crate::simplify::cost(&out) <= crate::simplify::cost(e) + 40 || out.is_const() {
        out
    } else {
        simplify(e)
    }
}

fn is_rational_in(e: &Expr, var: &str) -> bool {
    let (num, den) = ratsimp::fraction(&ratsimp::together(e));
    ratsimp::coefficients(&num, var).is_some() && ratsimp::coefficients(&den, var).is_some()
}

/// The function whose Laplace transform is `f(s)`.
pub fn inverse_laplace(f: &Expr, s: &str, t: &Expr) -> Option<Expr> {
    let f = simplify(f);
    if !f.has_sym(s) {
        return Some(mul(vec![f, f1("diracdelta", t.clone())]));
    }
    if is_rational_in(&f, s) {
        let (quotient, parts) = principal_parts(&f, s)?;
        let mut terms = Vec::new();
        if !quotient.is_zero() {
            if quotient.has_sym(s) {
                return None;
            }
            terms.push(mul(vec![quotient, f1("diracdelta", t.clone())]));
        }
        for (p, cs) in parts {
            for (j, c) in cs.iter().enumerate() {
                terms.push(mul(vec![
                    c.clone(),
                    div(powi(t.clone(), j as i64), factorial(j as i64)),
                    e_to(mul(vec![p.clone(), t.clone()])),
                ]));
            }
        }
        return Some(real_form(&add(terms)));
    }
    if let Expr::Add(terms) = &f {
        let parts: Option<Vec<Expr>> = terms.iter().map(|x| inverse_laplace(x, s, t)).collect();
        return parts.map(|p| simplify(&add(p)));
    }
    let (mut constant, mut rest) = (Vec::new(), Vec::new());
    let mut delay = Vec::new();
    for fac in factors_of(&f) {
        if !fac.has_sym(s) {
            constant.push(fac);
            continue;
        }
        match &fac {
            Expr::Pow(b, u)
                if b.is_sym("e") && linear(u, s).is_some_and(|(_, b0)| b0.is_zero()) =>
            {
                delay.push(neg(linear(u, s).expect("checked").0));
            }
            _ => rest.push(fac),
        }
    }
    let c = mul(constant);
    if !delay.is_empty() {
        // e^(-a s) F(s): f(t - a) from t = a on
        let a = simplify(&add(delay));
        if numeric::sign(&a).is_some_and(|x| x < 0) {
            return None;
        }
        let moved = sub(t.clone(), a);
        let inner = inverse_laplace(&mul(rest), s, &moved)?;
        return Some(mul(vec![c, f1("heaviside", moved), inner]));
    }
    let g = mul(rest);
    let value = match &g {
        // (s + c)^(-a): t^(a-1) e^(-c t)/gamma(a)
        Expr::Pow(b, a) if !a.has_sym(s) && linear(b, s).is_some_and(|(k, _)| k.is_one()) => {
            let (_, shift) = linear(b, s)?;
            let a = neg((**a).clone());
            if a.is_const() && numeric::sign(&a) != Some(1) {
                return None;
            }
            div(
                mul(vec![
                    pow(t.clone(), sub(a.clone(), Expr::one())),
                    e_to(neg(mul(vec![shift, t.clone()]))),
                ]),
                f1("gamma", a),
            )
        }
        // ((s + c)^2 + w^2)^(-1/2): e^(-c t) J0(w t)
        Expr::Pow(q, h) if **h == Expr::rat(-1, 2) => {
            let cs = ratsimp::coefficients(q, s).filter(|c| c.len() == 3 && c[2].is_one())?;
            let shift = div(cs[1].clone(), int(2));
            let w2 = simplify(&sub(cs[0].clone(), powi(shift.clone(), 2)));
            let positive = numeric::sign(&w2).is_none_or(|x| x > 0);
            let w = sqrt(if positive { w2.clone() } else { neg(w2) });
            mul(vec![
                e_to(neg(mul(vec![shift, t.clone()]))),
                func(
                    if positive { "besselj" } else { "besseli" },
                    vec![Expr::zero(), mul(vec![w, t.clone()])],
                ),
            ])
        }
        Expr::Fn(name, args) if &**name == "atan" && args.len() == 1 => {
            // atan(a/s): sin(a t)/t
            let a = simplify(&mul(vec![args[0].clone(), Expr::sym(s)]));
            if a.has_sym(s) {
                return None;
            }
            div(f1("sin", mul(vec![a, t.clone()])), t.clone())
        }
        _ => return None,
    };
    Some(mul(vec![c, value]))
}

/// `abs(x)`, `sign(x)` and the step functions of `x` itself on the half-line of the given sign.
fn on_side(f: &Expr, x: &str, positive: bool) -> Expr {
    map_bottom_up(f, &mut |e| match &e {
        Expr::Fn(n, a) if a.len() == 1 && a[0].is_sym(x) => match &**n {
            "abs" if positive => a[0].clone(),
            "abs" => neg(a[0].clone()),
            "sign" => int(if positive { 1 } else { -1 }),
            "heaviside" | "unitstep" => int(i64::from(positive)),
            _ => e,
        },
        _ => e,
    })
}

/// The Fourier transform: the integral of `f(x) e^(-i k x)` over the real line.
pub fn fourier(f: &Expr, x: &str, k: &Expr) -> Option<Expr> {
    const K: &str = "fourier_k";
    let r = with_facts(&[(K, Facts::real()), (U, Facts::real())], || {
        fourier_inner(&simplify(f), x, &Expr::sym(K), 0)
    })?;
    Some(simplify(&subst(&r, &Expr::sym(K), k)))
}

fn two_pi() -> Expr {
    mul(vec![int(2), Expr::pi()])
}

fn fourier_inner(f: &Expr, x: &str, k: &Expr, depth: u32) -> Option<Expr> {
    if depth > 12 {
        return None;
    }
    if !f.has_sym(x) {
        return Some(mul(vec![f.clone(), two_pi(), f1("diracdelta", k.clone())]));
    }
    if let Expr::Add(terms) = f {
        let parts: Option<Vec<Expr>> = terms
            .iter()
            .map(|t| fourier_inner(t, x, k, depth + 1))
            .collect();
        return parts.map(add);
    }
    let xsym = Expr::sym(x);
    // 1/(x^2 + a^2): pi e^(-a |k|)/a
    if let Expr::Pow(q, n) = f
        && n.is_minus_one()
        && let Some(cs) = ratsimp::coefficients(q, x)
        && cs.len() == 3
        && cs[1].is_zero()
        && cs[2].is_one()
        && numeric::sign(&cs[0]).is_none_or(|s| s > 0)
    {
        let a = sqrt(cs[0].clone());
        return Some(div(
            mul(vec![
                Expr::pi(),
                e_to(neg(mul(vec![a.clone(), f1("abs", k.clone())]))),
            ]),
            a,
        ));
    }
    if let Some(v) = fourier_rational(f, x, k) {
        return Some(v);
    }
    let (mut constant, mut rest, mut turn) = (Vec::new(), Vec::new(), Vec::new());
    let mut power = 0i64;
    for fac in factors_of(f) {
        if !fac.has_sym(x) {
            constant.push(fac);
            continue;
        }
        match &fac {
            Expr::Sym(_) => power += 1,
            Expr::Pow(b, n) if b.is_sym(x) && n.as_i64().is_some_and(|n| n > 0) => {
                power += n.as_i64().expect("checked")
            }
            // e^(i a x + b) with a real a: a shift of k
            Expr::Pow(b, u)
                if b.is_sym("e")
                    && linear(u, x).is_some_and(|(a, _)| {
                        let w = simplify(&div(a, Expr::i()));
                        w.is_const() && numeric::real_value(&w, 20).is_some()
                            || (!w.is_const()
                                && !w.any(&|y| *y == Expr::i())
                                && assume::is_real(&w))
                    }) =>
            {
                let (a, b0) = linear(u, x).expect("checked");
                turn.push(simplify(&div(a, Expr::i())));
                constant.push(e_to(b0));
            }
            _ => rest.push(fac),
        }
    }
    let c = mul(constant);
    let g = mul(rest);
    if !turn.is_empty() {
        let inner = simplify(&mul(vec![powi(xsym, power), g]));
        let v = fourier_inner(&inner, x, &sub(k.clone(), add(turn)), depth + 1)?;
        return Some(mul(vec![c, v]));
    }
    if power > 0 {
        // x^n g(x): i^n times the n-th derivative of the transform of g
        let u = Expr::sym(U);
        let base = fourier_inner(&g, x, &u, depth + 1)?;
        let d = if base.has_fn("diracdelta") && g.is_one() {
            // derivatives of the delta function stay as they are
            (0..power).fold(base, |acc, _| diff(&acc, U))
        } else {
            diff_n(&base, U, power as u32)
        };
        return Some(mul(vec![c, pow(Expr::i(), int(power)), subst(&d, &u, k)]));
    }
    fourier_table(&g, x, k, depth).map(|v| mul(vec![c, v]))
}

/// The Fourier transform of a rational function without real poles: each pole `p` gives
/// `2 pi i e^(-i k p)` on the side of `k` where the contour closes around it.
fn fourier_rational(g: &Expr, x: &str, k: &Expr) -> Option<Expr> {
    if !is_rational_in(g, x) {
        return None;
    }
    let (quotient, parts) = principal_parts(g, x)?;
    if !quotient.is_zero() || parts.is_empty() {
        return None;
    }
    let mut terms = Vec::new();
    for (p, cs) in &parts {
        let above = match numeric::evalf_digits(p, 20) {
            Ok(v) if !v.imag().is_zero() => !v.imag().is_sign_negative(),
            _ => return None,
        };
        let wave = e_to(neg(mul(vec![Expr::i(), k.clone(), p.clone()])));
        let side = if above {
            mul(vec![two_pi(), Expr::i(), f1("heaviside", neg(k.clone()))])
        } else {
            neg(mul(vec![two_pi(), Expr::i(), f1("heaviside", k.clone())]))
        };
        for (j, c) in cs.iter().enumerate() {
            let j = j as i64;
            terms.push(mul(vec![
                c.clone(),
                div(powi(neg(mul(vec![Expr::i(), k.clone()])), j), factorial(j)),
                side.clone(),
                wave.clone(),
            ]));
        }
    }
    Some(real_form(&add(terms)))
}

fn fourier_table(g: &Expr, x: &str, k: &Expr, depth: u32) -> Option<Expr> {
    let xsym = Expr::sym(x);
    if let Expr::Fn(name, args) = g
        && args.len() == 1
        && let Some((a, b)) = linear(&args[0], x)
    {
        match &**name {
            // delta(a x + b): e^(i k b/a)/|a|
            "diracdelta" => {
                return Some(div(
                    e_to(mul(vec![Expr::i(), k.clone(), div(b, a.clone())])),
                    f1("abs", a),
                ));
            }
            "sign" if b.is_zero() && numeric::sign(&a) == Some(1) => {
                return Some(div(int(2), mul(vec![Expr::i(), k.clone()])));
            }
            "heaviside" | "unitstep" if numeric::sign(&a) == Some(1) => {
                // the step at x = -b/a: pi delta(k) + e^(i k b/a)/(i k)
                return Some(add(vec![
                    mul(vec![Expr::pi(), f1("diracdelta", k.clone())]),
                    div(
                        e_to(mul(vec![Expr::i(), k.clone(), div(b, a)])),
                        mul(vec![Expr::i(), k.clone()]),
                    ),
                ]));
            }
            _ => {}
        }
    }
    if has_fn_of(g, &["sin", "cos"], x) {
        let reduced = crate::algebra::trigreduce(g);
        let ex = expand(&crate::solve::rewrite_exp(&reduced));
        if ex != *g && !has_fn_of(&ex, &["sin", "cos"], x) {
            return fourier_inner(&ex, x, k, depth + 1);
        }
    }
    // the two half-lines apart, with abs(x), sign(x) and the step function resolved
    let wave = e_to(neg(mul(vec![Expr::i(), k.clone(), xsym])));
    let piece = |positive: bool| -> Option<Expr> {
        let h = simplify(&mul(vec![on_side(g, x, positive), wave.clone()]));
        if h.is_zero() {
            return Some(Expr::zero());
        }
        let (lo, hi) = if positive {
            (Expr::zero(), Expr::inf())
        } else {
            (Expr::neg_inf(), Expr::zero())
        };
        integrate::definite_with_parameters(&h, x, &lo, &hi)
    };
    let total = add(vec![piece(true)?, piece(false)?]);
    Some(simplify(&ratsimp::together(&total)))
}

/// The partial sum of the Fourier series of `f` on the interval from `a` to `b`.
pub fn fourier_series(f: &Expr, x: &str, n: i64, a: &Expr, b: &Expr) -> Res {
    let length = simplify(&sub(b.clone(), a.clone()));
    let omega = div(two_pi(), length.clone());
    let xsym = Expr::sym(x);
    let coefficient = |wave: Expr| -> Res {
        let g = simplify(&mul(vec![f.clone(), wave]));
        let (v, _) = integrate::definite(&g, x, a, b)?;
        Ok(simplify(&mul(vec![div(int(2), length.clone()), v])))
    };
    let mut terms = vec![div(coefficient(Expr::one())?, int(2))];
    for k in 1..=n {
        let angle = mul(vec![int(k), omega.clone(), xsym.clone()]);
        let (c, s) = (
            coefficient(f1("cos", angle.clone()))?,
            coefficient(f1("sin", angle.clone()))?,
        );
        terms.push(mul(vec![c, f1("cos", angle.clone())]));
        terms.push(mul(vec![s, f1("sin", angle)]));
    }
    Ok(Expr::call(
        "ordered",
        terms.into_iter().filter(|t| !t.is_zero()).collect(),
    ))
}

/// The Z transform: the sum of `f(n) z^(-n)` over `n = 0, 1, 2, ...`.
pub fn z_transform(f: &Expr, n: &str, z: &Expr) -> Option<Expr> {
    const Z: &str = "transform_z";
    let r = z_inner(&expand(&simplify(f)), n, &Expr::sym(Z), 0)?;
    Some(simplify(&ratsimp::together(&subst(&r, &Expr::sym(Z), z))))
}

fn z_inner(f: &Expr, n: &str, z: &Expr, depth: u32) -> Option<Expr> {
    if depth > 12 {
        return None;
    }
    if let Expr::Add(terms) = f {
        let parts: Option<Vec<Expr>> = terms.iter().map(|t| z_inner(t, n, z, depth + 1)).collect();
        return parts.map(add);
    }
    let unit = div(z.clone(), sub(z.clone(), Expr::one()));
    if !f.has_sym(n) {
        return Some(mul(vec![f.clone(), unit]));
    }
    let (mut constant, mut rest, mut scale) = (Vec::new(), Vec::new(), Vec::new());
    let mut power = 0i64;
    for fac in factors_of(f) {
        if !fac.has_sym(n) {
            constant.push(fac);
            continue;
        }
        match &fac {
            Expr::Sym(_) => power += 1,
            Expr::Pow(b, k) if b.is_sym(n) && k.as_i64().is_some_and(|k| k > 0) => {
                power += k.as_i64().expect("checked")
            }
            // b^(c n + d): the base b^c scales z
            Expr::Pow(b, u) if !b.has_sym(n) && linear(u, n).is_some() => {
                let (c, d) = linear(u, n).expect("checked");
                scale.push(pow((**b).clone(), c));
                constant.push(pow((**b).clone(), d));
            }
            _ => rest.push(fac),
        }
    }
    let c = mul(constant);
    let g = mul(rest);
    if !scale.is_empty() {
        let inner = simplify(&mul(vec![powi(Expr::sym(n), power), g]));
        let v = z_inner(&inner, n, &div(z.clone(), mul(scale)), depth + 1)?;
        return Some(mul(vec![c, v]));
    }
    if power > 0 {
        // n^k g(n): the operator -z d/dz applied k times
        let u = Expr::sym(U);
        let mut value = z_inner(&g, n, &u, depth + 1)?;
        for _ in 0..power {
            value = simplify(&neg(mul(vec![u.clone(), diff(&value, U)])));
        }
        return Some(mul(vec![c, subst(&value, &u, z)]));
    }
    if g.is_one() {
        return Some(mul(vec![c, unit]));
    }
    let value = match &g {
        // 1/n!: e^(1/z)
        Expr::Pow(b, k)
            if k.is_minus_one()
                && matches!(&**b, Expr::Fn(name, a) if &**name == "factorial" && a[0].is_sym(n)) =>
        {
            e_to(powi(z.clone(), -1))
        }
        Expr::Fn(name, a) if &**name == "kroneckerdelta" && a.len() == 2 => {
            let at = if a[0].is_sym(n) { &a[1] } else { &a[0] };
            if at.has_sym(n) {
                return None;
            }
            pow(z.clone(), neg(at.clone()))
        }
        // sin(w n + p) and cos(w n + p) over z^2 - 2 z cos(w) + 1
        Expr::Fn(name, a)
            if matches!(&**name, "sin" | "cos") && a.len() == 1 && linear(&a[0], n).is_some() =>
        {
            let (w, phase) = linear(&a[0], n).expect("checked");
            let (sw, cw) = (f1("sin", w.clone()), f1("cos", w));
            let den = add(vec![
                powi(z.clone(), 2),
                neg(mul(vec![int(2), z.clone(), cw.clone()])),
                Expr::one(),
            ]);
            let sine = div(mul(vec![z.clone(), sw]), den.clone());
            let cosine = div(mul(vec![z.clone(), sub(z.clone(), cw)]), den);
            let (sp, cp) = (f1("sin", phase.clone()), f1("cos", phase));
            if &**name == "sin" {
                add(vec![mul(vec![sine, cp]), mul(vec![cosine, sp])])
            } else {
                sub(mul(vec![cosine, cp]), mul(vec![sine, sp]))
            }
        }
        _ if has_fn_of(&g, &["sin", "cos", "sinh", "cosh"], n) => {
            let ex = expand(&crate::solve::rewrite_exp(&crate::algebra::trigreduce(&g)));
            if ex == g || has_fn_of(&ex, &["sin", "cos", "sinh", "cosh"], n) {
                return None;
            }
            let v = z_inner(&ex, n, z, depth + 1)?;
            real_form(&ratsimp::together(&v))
        }
        _ => return None,
    };
    Some(mul(vec![c, value]))
}

/// The sequence whose Z transform is `f(z)`.
pub fn inverse_z(f: &Expr, z: &str, n: &Expr) -> Option<Expr> {
    let g = simplify(&div(f.clone(), Expr::sym(z)));
    if !is_rational_in(&g, z) {
        return None;
    }
    let (quotient, parts) = principal_parts(&g, z)?;
    if !quotient.is_zero() {
        return None;
    }
    let mut terms = Vec::new();
    for (p, cs) in parts {
        for (j, c) in cs.iter().enumerate() {
            let j = j as i64;
            if p.is_zero() {
                terms.push(mul(vec![
                    c.clone(),
                    func("kroneckerdelta", vec![n.clone(), int(j)]),
                ]));
                continue;
            }
            // c/(z - p)^(j+1) of F/z: c binomial(n, j) p^(n - j)
            let falling = mul((0..j).map(|i| sub(n.clone(), int(i))).collect());
            terms.push(mul(vec![
                c.clone(),
                div(falling, factorial(j)),
                pow(p.clone(), sub(n.clone(), int(j))),
            ]));
        }
    }
    Some(simplify(&add(terms)))
}

/// The coefficient of `(x - x0)^n` in the series of `f` at `x0`.
pub fn series_coefficient(f: &Expr, x: &str, x0: &Expr, n: i64) -> Res {
    let shifted = if x0.is_inf() {
        subst(f, &Expr::sym(x), &powi(Expr::sym(U), -1))
    } else {
        subst(f, &Expr::sym(x), &add(vec![x0.clone(), Expr::sym(U)]))
    };
    let mut terms = (n.max(0) as usize) + 4;
    for _ in 0..6 {
        let s = series::expand(&shifted, U, terms)
            .ok_or_else(|| format!("no series found for {f} at {x} = {x0}"))?
            .trim();
        if s.den != 1 {
            return Err(format!(
                "{f} has a branch point at {x} = {x0}: its series has fractional powers"
            ));
        }
        if s.c.is_empty() || n < s.lo {
            if s.c.is_empty() && terms < 200 {
                terms *= 2;
                continue;
            }
            return Ok(Expr::zero());
        }
        if let Some(c) = usize::try_from(n - s.lo).ok().and_then(|k| s.c.get(k)) {
            if c.has_sym(series::LOG) {
                return Err(format!("{f} has a logarithm at {x} = {x0}"));
            }
            return Ok(simplify(c));
        }
        terms = terms * 2 + (n - s.lo) as usize;
    }
    Err(format!("no series found for {f} at {x} = {x0}"))
}

/// The residue of `f` at `x0`: the coefficient of `1/(x - x0)` in its Laurent series.
pub fn residue(f: &Expr, x: &str, x0: &Expr) -> Res {
    if x0.is_infinite() {
        // minus the residue of f(1/w)/w^2 at w = 0
        let w = Expr::sym("#w");
        let g = div(subst(f, &Expr::sym(x), &powi(w.clone(), -1)), powi(w, 2));
        return residue(&simplify(&g), "#w", &Expr::zero()).map(|r| simplify(&neg(r)));
    }
    series_coefficient(f, x, x0, -1)
}

/// The Pade approximant of `f` at `x0` with a numerator of degree `m` and a denominator of
/// degree `n`.
pub fn pade(f: &Expr, x: &str, x0: &Expr, m: usize, n: usize) -> Res {
    let shifted = subst(f, &Expr::sym(x), &add(vec![x0.clone(), Expr::sym(U)]));
    let s = series::expand(&shifted, U, m + n + 3)
        .ok_or_else(|| format!("no series found for {f} at {x} = {x0}"))?;
    if s.den != 1 || s.lo < 0 && s.c.iter().take((-s.lo) as usize).any(|c| !c.is_zero()) {
        return Err(format!("{f} has no Taylor series at {x} = {x0}"));
    }
    let c = |k: i64| -> Expr {
        usize::try_from(k - s.lo)
            .ok()
            .and_then(|i| s.c.get(i))
            .map(simplify)
            .unwrap_or_else(Expr::zero)
    };
    if (m + n) as i64 >= s.lo + s.c.len() as i64 {
        return Err("the series is too short for this approximant".into());
    }
    // sum_{j=1..n} b_j c_(m+k-j) = -c_(m+k) for k = 1..n
    let mut b = vec![Expr::one()];
    if n > 0 {
        let rows: Vec<Vec<Expr>> = (1..=n)
            .map(|k| (1..=n).map(|j| c((m + k) as i64 - j as i64)).collect())
            .collect();
        let rhs: Vec<Expr> = (1..=n).map(|k| neg(c((m + k) as i64))).collect();
        let solved = crate::matrix::linsolve(&rows, &rhs)
            .map_err(|_| "there is no Pade approximant of this order".to_string())?;
        let items = solved
            .as_list()
            .filter(|l| {
                l.len() == n && l.iter().all(Expr::is_const) == rhs.iter().all(Expr::is_const)
            })
            .ok_or("there is no Pade approximant of this order")?;
        b.extend(items.iter().cloned());
    }
    let step = sub(Expr::sym(x), x0.clone());
    let top: Vec<Expr> = (0..=m)
        .map(|i| {
            let a = add((0..=i.min(n))
                .map(|j| mul(vec![b[j].clone(), c(i as i64 - j as i64)]))
                .collect());
            mul(vec![simplify(&a), powi(step.clone(), i as i64)])
        })
        .collect();
    let low: Vec<Expr> = (0..=n)
        .map(|j| mul(vec![b[j].clone(), powi(step.clone(), j as i64)]))
        .collect();
    let (num, den) = ratsimp::fraction(&ratsimp::together(&div(add(top), add(low))));
    Ok(div(num, den))
}
