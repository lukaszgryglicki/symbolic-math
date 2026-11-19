//! Real and imaginary parts of expressions whose symbols are real or constant.

use crate::assume;
use crate::canon::{add, div, mul, neg, pow, powi, sqrt, sub};
use crate::expand::expand;
use crate::expr::Expr;
use crate::funcs::{cos_pi, func, sin_pi_value};
use crate::numeric;

type Parts = (Expr, Expr);

fn times(a: &Parts, b: &Parts) -> Parts {
    (
        expand(&sub(
            mul(vec![a.0.clone(), b.0.clone()]),
            mul(vec![a.1.clone(), b.1.clone()]),
        )),
        expand(&add(vec![
            mul(vec![a.0.clone(), b.1.clone()]),
            mul(vec![a.1.clone(), b.0.clone()]),
        ])),
    )
}

fn norm(a: &Parts) -> Expr {
    add(vec![powi(a.0.clone(), 2), powi(a.1.clone(), 2)])
}

fn quotient(a: &Parts, b: &Parts) -> Parts {
    let n = norm(b);
    let p = times(a, &(b.0.clone(), neg(b.1.clone())));
    (div(p.0, n.clone()), div(p.1, n))
}

fn f1(name: &str, x: &Expr) -> Expr {
    func(name, vec![x.clone()])
}

/// `e = re + i*im` with real `re` and `im`.
pub fn rect(e: &Expr) -> Option<Parts> {
    if assume::is_real(e) {
        return Some((e.clone(), Expr::zero()));
    }
    match e {
        Expr::Num(n) => Some((Expr::rational(n.re.clone()), Expr::rational(n.im.clone()))),
        Expr::Float(f) => {
            let part = |x: &rug::Float| {
                if x.is_zero() {
                    Expr::zero()
                } else {
                    let value = rug::Complex::with_val(x.prec(), (x, 0));
                    crate::numeric::to_float_expr(value, f.digits)
                }
            };
            Some((part(f.v.real()), part(f.v.imag())))
        }
        Expr::Add(a) => {
            let (mut re, mut im) = (Vec::new(), Vec::new());
            for t in a.iter() {
                let (r, i) = rect(t)?;
                re.push(r);
                im.push(i);
            }
            Some((add(re), add(im)))
        }
        Expr::Mul(a) => {
            let mut acc = (Expr::one(), Expr::zero());
            for f in a.iter() {
                acc = times(&acc, &rect(f)?);
            }
            Some(acc)
        }
        Expr::Pow(b, x) => rect_pow(b, x),
        Expr::Fn(n, a) if a.len() == 1 => rect_fn(n, &a[0]),
        _ => None,
    }
}

fn polar(modulus: Expr, angle: Expr) -> Parts {
    (
        mul(vec![modulus.clone(), f1("cos", &angle)]),
        mul(vec![modulus, f1("sin", &angle)]),
    )
}

fn rect_pow(b: &Expr, x: &Expr) -> Option<Parts> {
    if b.is_sym("e") {
        let (u, v) = rect(x)?;
        return Some(polar(pow(Expr::e(), u), v));
    }
    if assume::is_positive(b) {
        let (u, v) = rect(x)?;
        return Some(polar(pow(b.clone(), u), mul(vec![v, f1("ln", b)])));
    }
    if let Some(n) = x.as_i64().filter(|n| n.unsigned_abs() <= 64) {
        let base = rect(b)?;
        let (mut sq, mut acc, mut k) =
            (base.clone(), (Expr::one(), Expr::zero()), n.unsigned_abs());
        while k > 0 {
            if k & 1 == 1 {
                acc = times(&acc, &sq);
            }
            k >>= 1;
            if k > 0 {
                sq = times(&sq, &sq);
            }
        }
        return Some(if n < 0 {
            quotient(&(Expr::one(), Expr::zero()), &acc)
        } else {
            acc
        });
    }
    if let Some(r) = x.as_rat()
        && assume::is_negative(b)
    {
        let m = pow(neg(b.clone()), x.clone());
        return Some((
            mul(vec![m.clone(), cos_pi(r)]),
            mul(vec![m, sin_pi_value(r)]),
        ));
    }
    let (u, v) = rect(b)?;
    let modulus = sqrt(norm(&(u.clone(), v.clone())));
    if x.as_rat().is_some_and(|r| *r == crate::num::rat(1, 2))
        && let Some(s) = numeric::sign(&v).filter(|s| *s != 0)
    {
        let half = |t: Expr| sqrt(div(t, Expr::int(2)));
        let im = half(sub(modulus.clone(), u.clone()));
        return Some((
            half(add(vec![modulus, u])),
            if s < 0 { neg(im) } else { im },
        ));
    }
    let angle = angle(b, &u, &v);
    if assume::is_real(x) {
        return Some(polar(pow(modulus, x.clone()), mul(vec![x.clone(), angle])));
    }
    // z^w = e^(w ln z)
    let (p, q) = rect(x)?;
    let log = (f1("ln", &modulus), angle);
    let (re, im) = times(&(p, q), &log);
    Some(polar(pow(Expr::e(), re), im))
}

/// The argument of `z = u + i v`.
fn angle(z: &Expr, u: &Expr, v: &Expr) -> Expr {
    let exact = func("arg", vec![z.clone()]);
    if exact.has_fn("arg") {
        func("atan2", vec![v.clone(), u.clone()])
    } else {
        exact
    }
}

fn rect_fn(name: &str, z: &Expr) -> Option<Parts> {
    let (u, v) = rect(z)?;
    let (su, cu) = (f1("sin", &u), f1("cos", &u));
    let (sv, cv) = (f1("sin", &v), f1("cos", &v));
    let (shu, chu) = (f1("sinh", &u), f1("cosh", &u));
    let (shv, chv) = (f1("sinh", &v), f1("cosh", &v));
    let sin = (
        mul(vec![su.clone(), chv.clone()]),
        mul(vec![cu.clone(), shv.clone()]),
    );
    let cos = (mul(vec![cu, chv]), neg(mul(vec![su, shv])));
    let sinh = (
        mul(vec![shu.clone(), cv.clone()]),
        mul(vec![chu.clone(), sv.clone()]),
    );
    let cosh = (mul(vec![chu, cv]), mul(vec![shu, sv]));
    Some(match name {
        "sin" => sin,
        "cos" => cos,
        "tan" => quotient(&sin, &cos),
        "sinh" => sinh,
        "cosh" => cosh,
        "tanh" => quotient(&sinh, &cosh),
        "conj" => (u, neg(v)),
        "re" => (u, Expr::zero()),
        "im" => (v, Expr::zero()),
        "abs" => (sqrt(norm(&(u, v))), Expr::zero()),
        "arg" => (angle(z, &u, &v), Expr::zero()),
        "ln" => {
            let angle = angle(z, &u, &v);
            (div(f1("ln", &norm(&(u, v))), Expr::int(2)), angle)
        }
        "asin" | "acos" | "atan" | "asinh" | "acosh" | "atanh"
            if !v.is_zero()
                || (z.is_const()
                    && numeric::evalf(&f1(name, z), 128).is_ok_and(|w| !w.imag().is_zero())) =>
        {
            let one = Expr::one;
            let iz = mul(vec![Expr::i(), z.clone()]);
            let ln = |t: Expr| f1("ln", &t);
            let asin = || {
                mul(vec![
                    neg(Expr::i()),
                    ln(add(vec![iz.clone(), sqrt(sub(one(), powi(z.clone(), 2)))])),
                ])
            };
            let by_logs = match name {
                "asin" => asin(),
                "acos" => sub(div(Expr::pi(), Expr::int(2)), asin()),
                "atan" => mul(vec![
                    div(Expr::i(), Expr::int(2)),
                    sub(ln(sub(one(), iz.clone())), ln(add(vec![one(), iz.clone()]))),
                ]),
                "asinh" => ln(add(vec![
                    z.clone(),
                    sqrt(add(vec![powi(z.clone(), 2), one()])),
                ])),
                "acosh" => ln(add(vec![
                    z.clone(),
                    mul(vec![
                        sqrt(add(vec![z.clone(), one()])),
                        sqrt(sub(z.clone(), one())),
                    ]),
                ])),
                _ => div(
                    sub(ln(add(vec![one(), z.clone()])), ln(sub(one(), z.clone()))),
                    Expr::int(2),
                ),
            };
            return rect(&by_logs);
        }
        _ => return None,
    })
}

/// `re`, `im`, `abs`, `arg` and `conj` written out for real symbols.
pub fn expand_parts(e: &Expr) -> Expr {
    crate::canon::map_bottom_up(e, &mut |node| {
        if let Expr::Fn(n, a) = &node
            && a.len() == 1
            && matches!(&**n, "re" | "im" | "abs" | "arg" | "conj")
            && let Some((u, v)) = rect(&a[0])
        {
            return match &**n {
                "re" => u,
                "im" => v,
                "abs" if v.is_zero() => node,
                "abs" => sqrt(norm(&(u, v))),
                "arg" if v.is_zero() => node,
                "arg" => angle(&a[0], &u, &v),
                _ => add(vec![u, mul(vec![neg(Expr::i()), v])]),
            };
        }
        node
    })
}

/// The expression split as `re + i*im` with every symbol taken as real.
pub fn complex_expand(e: &Expr) -> (Expr, Option<Parts>) {
    assume::with_real_symbols(|| {
        let written = expand_parts(e);
        let parts = rect(&written);
        (written, parts)
    })
}

/// Rectangular form `re + i*im` of a constant when it has one.
pub fn rectform(e: &Expr) -> Option<Expr> {
    let (re, im) = rect(e)?;
    Some(add(vec![re, mul(vec![Expr::i(), im])]))
}
