//! Quaternions: `quat(a, b, c, d)` stands for `a + b i + c j + d k` with real components.
//! Other values join in as scalars: complex numbers as `a + b i`, everything else as real.

use crate::canon::{add, div, mul, neg, pow, sqrt};
use crate::expr::Expr;
use crate::funcs::func;
use crate::{assume, complex, numeric, simplify};

pub type Parts = [Expr; 4];

/// Functions with real Taylor coefficients: `f(a + n m) = re f(a + i m) + n im f(a + i m)`.
pub(crate) const ANALYTIC: &[&str] = &[
    "exp", "ln", "sin", "cos", "tan", "sinh", "cosh", "tanh", "asin", "acos", "atan", "asinh",
    "acosh", "atanh",
];

pub fn is_quat(e: &Expr) -> bool {
    matches!(e, Expr::Fn(n, a) if &**n == "quat" && a.len() == 4)
}

pub fn parts(e: &Expr) -> Option<Parts> {
    match e.fn_args("quat") {
        Some([a, b, c, d]) => Some([a.clone(), b.clone(), c.clone(), d.clone()]),
        _ => None,
    }
}

/// Components of any value.
pub fn lift(e: &Expr) -> Parts {
    if let Some(p) = parts(e) {
        return p;
    }
    let (re, im) = match e {
        Expr::Num(n) => (Expr::rational(n.re.clone()), Expr::rational(n.im.clone())),
        _ if assume::is_real(e) => (e.clone(), Expr::zero()),
        _ => complex::rect(e).unwrap_or_else(|| (e.clone(), Expr::zero())),
    };
    [re, im, Expr::zero(), Expr::zero()]
}

/// The value with these components; a complex number when the `j` and `k` parts vanish. The
/// components are multiplied out (their simple forms are found once, when a result is shown).
pub fn make(p: Parts) -> Expr {
    assemble(p.map(|c| {
        if c.is_number() {
            c
        } else {
            crate::expand::expand(&c)
        }
    }))
}

/// The value with these components as they are.
fn assemble(p: Parts) -> Expr {
    if p[2].is_zero() && p[3].is_zero() {
        let [re, im, ..] = p;
        return add(vec![re, mul(vec![Expr::i(), im])]);
    }
    Expr::call("quat", p.to_vec())
}

/// A quaternion with every component in its simple form.
pub fn simple(e: &Expr) -> Option<Expr> {
    Some(tidy(parts(e)?))
}

/// The value with these components, each in its simple form.
pub fn tidy(p: Parts) -> Expr {
    assemble(p.map(|c| {
        if c.is_number() {
            c
        } else {
            simplify::simplify(&c)
        }
    }))
}

pub fn sum(args: &[Expr]) -> Expr {
    let mut acc = lift(&Expr::zero());
    for a in args {
        let p = lift(a);
        for (total, part) in acc.iter_mut().zip(p) {
            *total = add(vec![total.clone(), part]);
        }
    }
    make(acc)
}

fn hamilton(x: &Parts, y: &Parts) -> Parts {
    let m = |i: usize, j: usize| mul(vec![x[i].clone(), y[j].clone()]);
    [
        add(vec![m(0, 0), neg(m(1, 1)), neg(m(2, 2)), neg(m(3, 3))]),
        add(vec![m(0, 1), m(1, 0), m(2, 3), neg(m(3, 2))]),
        add(vec![m(0, 2), neg(m(1, 3)), m(2, 0), m(3, 1)]),
        add(vec![m(0, 3), m(1, 2), neg(m(2, 1)), m(3, 0)]),
    ]
}

/// The product of the values in the given order.
pub fn product(args: &[Expr]) -> Expr {
    let mut acc = lift(&Expr::one());
    for a in args {
        acc = hamilton(&acc, &lift(a));
    }
    make(acc)
}

fn norm_squared(p: &Parts) -> Expr {
    add(p.iter().map(|c| pow(c.clone(), Expr::int(2))).collect())
}

fn conjugate(p: &Parts) -> Parts {
    [
        p[0].clone(),
        neg(p[1].clone()),
        neg(p[2].clone()),
        neg(p[3].clone()),
    ]
}

fn scale(p: &Parts, s: &Expr) -> Parts {
    p.clone().map(|c| mul(vec![c, s.clone()]))
}

fn inverse(p: &Parts) -> Parts {
    scale(&conjugate(p), &pow(norm_squared(p), Expr::minus_one()))
}

fn analytic(p: &Parts, f: &dyn Fn(Expr) -> Expr) -> Option<Expr> {
    let m = simplify::simplify(&sqrt(add(p[1..]
        .iter()
        .map(|c| pow(c.clone(), Expr::int(2)))
        .collect())));
    if m.is_zero() {
        return Some(f(p[0].clone()));
    }
    let value = f(add(vec![p[0].clone(), mul(vec![Expr::i(), m.clone()])]));
    let (u, w) = match complex::rect(&value) {
        Some(split) => split,
        None => {
            if !value.is_const() {
                return None;
            }
            let digits = numeric::digits();
            let v = numeric::evalf_digits(&value, digits).ok()?;
            let part = |x: &rug::Float| {
                numeric::to_float_expr(rug::Complex::with_val(x.prec(), (x, 0)), digits)
            };
            (part(v.real()), part(v.imag()))
        }
    };
    let s = div(w, m);
    Some(make([
        u,
        mul(vec![s.clone(), p[1].clone()]),
        mul(vec![s.clone(), p[2].clone()]),
        mul(vec![s, p[3].clone()]),
    ]))
}

/// `b^e` when one of them is a quaternion.
pub fn power(b: &Expr, e: &Expr) -> Option<Expr> {
    match (parts(b), parts(e)) {
        (Some(p), None) => {
            // (with names among its parts a power is a polynomial in them of that degree, with
            // thousands of terms from the 32nd on: beyond the 48th it stays as it is written)
            let most = match p.iter().all(Expr::is_const) {
                true => 100_000,
                false => 48,
            };
            if let Some(n) = e.as_i64().filter(|n| n.unsigned_abs() <= most) {
                let mut square = if n < 0 { inverse(&p) } else { p };
                let mut result = lift(&Expr::one());
                let mut k = n.unsigned_abs();
                while k > 0 {
                    if k & 1 == 1 {
                        result = hamilton(&result, &square).map(|c| crate::expand::expand(&c));
                    }
                    k >>= 1;
                    if k > 0 {
                        square = hamilton(&square, &square).map(|c| crate::expand::expand(&c));
                    }
                }
                return Some(make(result));
            }
            assume::is_real(e)
                .then(|| analytic(&p, &|z| pow(z, e.clone())))
                .flatten()
        }
        (None, Some(p)) if b.is_sym("e") || assume::is_positive(b) => {
            analytic(&p, &|z| pow(b.clone(), z))
        }
        _ => None,
    }
}

/// A function of a quaternion (the first argument).
pub fn apply(name: &str, args: &[Expr]) -> Option<Expr> {
    let p = parts(args.first()?)?;
    if args.len() != 1 {
        return None;
    }
    let length = |p: &[Expr]| {
        simplify::simplify(&sqrt(add(p
            .iter()
            .map(|c| pow(c.clone(), Expr::int(2)))
            .collect())))
    };
    Some(match name {
        "conj" => make(conjugate(&p)),
        "abs" | "norm" => length(&p),
        "re" => p[0].clone(),
        "im" => Expr::list(p[1..].to_vec()),
        "arg" => func("atan2", vec![length(&p[1..]), p[0].clone()]),
        "sign" | "normalize" => make(scale(&p, &pow(length(&p), Expr::minus_one()))),
        "inv" => make(inverse(&p)),
        _ if ANALYTIC.contains(&name) => analytic(&p, &|z| func(name, vec![z]))?,
        _ => return None,
    })
}

/// `v` turned by the rotation of the quaternion `q`: the vector part of `q v q^-1`.
pub fn rotate(v: &[Expr], q: &Expr) -> Option<Expr> {
    let [x, y, z] = v else {
        return None;
    };
    let q = lift(q);
    let turned = hamilton(
        &hamilton(&q, &[Expr::zero(), x.clone(), y.clone(), z.clone()]),
        &inverse(&q),
    );
    Some(Expr::list(
        turned[1..].iter().map(simplify::simplify).collect(),
    ))
}

/// The unit quaternion of the rotation by `angle` about `axis`.
pub fn from_axis_angle(axis: &[Expr], angle: &Expr) -> Option<Expr> {
    let [x, y, z] = axis else {
        return None;
    };
    let length = sqrt(add(axis
        .iter()
        .map(|c| pow(c.clone(), Expr::int(2)))
        .collect()));
    if length.is_zero() {
        return None;
    }
    let half = div(angle.clone(), Expr::int(2));
    let s = div(func("sin", vec![half.clone()]), length);
    Some(make([
        func("cos", vec![half]),
        mul(vec![s.clone(), x.clone()]),
        mul(vec![s.clone(), y.clone()]),
        mul(vec![s, z.clone()]),
    ]))
}

/// The rotation matrix of a quaternion (normalized first).
pub fn rotation_matrix(q: &Expr) -> Expr {
    let p = lift(q);
    let n = norm_squared(&p);
    let [a, b, c, d] = p;
    let m = |x: &Expr, y: &Expr| mul(vec![x.clone(), y.clone()]);
    let two = |e: Expr| mul(vec![Expr::int(2), e]);
    let entry = |e: Expr| simplify::simplify(&div(e, n.clone()));
    let sq = |x: &Expr| pow(x.clone(), Expr::int(2));
    let rows = [
        [
            add(vec![sq(&a), sq(&b), neg(sq(&c)), neg(sq(&d))]),
            two(add(vec![m(&b, &c), neg(m(&a, &d))])),
            two(add(vec![m(&b, &d), m(&a, &c)])),
        ],
        [
            two(add(vec![m(&b, &c), m(&a, &d)])),
            add(vec![sq(&a), neg(sq(&b)), sq(&c), neg(sq(&d))]),
            two(add(vec![m(&c, &d), neg(m(&a, &b))])),
        ],
        [
            two(add(vec![m(&b, &d), neg(m(&a, &c))])),
            two(add(vec![m(&c, &d), m(&a, &b)])),
            add(vec![sq(&a), neg(sq(&b)), neg(sq(&c)), sq(&d)]),
        ],
    ];
    Expr::list(
        rows.into_iter()
            .map(|r| Expr::list(r.into_iter().map(entry).collect()))
            .collect(),
    )
}
