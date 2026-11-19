//! Lattice reduction (LLL with whole numbers only) and what it finds: whole-number relations
//! between numbers, minimal polynomials, closed forms.

use crate::canon::{add, div, mul, neg, pow, powi};
use crate::expr::Expr;
use crate::funcs::func;
use crate::numeric;
use rug::ops::{DivRounding, Pow};
use rug::{Complex, Float, Integer};

type Res = Result<Expr, String>;

fn dot(x: &[Integer], y: &[Integer]) -> Integer {
    let mut total = Integer::new();
    for (a, b) in x.iter().zip(y) {
        total += Integer::from(a * b);
    }
    total
}

/// `round(a/b)` for a positive `b`.
fn nearest(a: &Integer, b: &Integer) -> Integer {
    (Integer::from(a * 2u32) + b).div_floor(&Integer::from(b * 2u32))
}

/// A reduced basis of the lattice of the rows (Lenstra, Lenstra, Lovasz; Cohen's version in
/// whole numbers). `None` when the rows are dependent.
pub fn lll(mut b: Vec<Vec<Integer>>) -> Option<Vec<Vec<Integer>>> {
    let n = b.len();
    if n == 0 {
        return Some(b);
    }
    // d[i] is the Gram determinant of the first i rows, lam[i][j] the scaled coefficients
    let mut d = vec![Integer::from(1); n + 1];
    let mut lam = vec![vec![Integer::new(); n + 1]; n + 1];
    d[1] = dot(&b[0], &b[0]);
    if d[1] == 0 {
        return None;
    }
    if n == 1 {
        return Some(b);
    }
    fn red(b: &mut [Vec<Integer>], d: &[Integer], lam: &mut [Vec<Integer>], k: usize, l: usize) {
        if Integer::from(&lam[k][l] * 2u32).abs() <= d[l] {
            return;
        }
        let q = nearest(&lam[k][l], &d[l]);
        let lower = b[l - 1].clone();
        for (x, y) in b[k - 1].iter_mut().zip(&lower) {
            *x -= Integer::from(&q * y);
        }
        lam[k][l] -= Integer::from(&q * &d[l]);
        let (upper, lower) = lam.split_at_mut(k);
        for (target, source) in lower[0].iter_mut().zip(&upper[l]).take(l).skip(1) {
            *target -= Integer::from(&q * source);
        }
    }
    let (mut k, mut kmax) = (2usize, 1usize);
    while k <= n {
        if k > kmax {
            kmax = k;
            for j in 1..=k {
                let mut u = dot(&b[k - 1], &b[j - 1]);
                for i in 1..j {
                    u = (Integer::from(&d[i] * &u) - Integer::from(&lam[k][i] * &lam[j][i]))
                        .div_exact(&d[i - 1]);
                }
                if j < k {
                    lam[k][j] = u;
                } else {
                    if u == 0 {
                        return None;
                    }
                    d[k] = u;
                }
            }
        }
        loop {
            red(&mut b, &d, &mut lam, k, k - 1);
            // d_k d_(k-2) < 3/4 d_(k-1)^2 - lam_(k,k-1)^2: the rows k - 1 and k change places
            let left = (Integer::from(&d[k] * &d[k - 2])
                + Integer::from(lam[k][k - 1].square_ref()))
                * 4u32;
            let right = Integer::from(d[k - 1].square_ref()) * 3u32;
            if left < right {
                b.swap(k - 1, k - 2);
                let (upper, lower) = lam.split_at_mut(k);
                for (x, y) in lower[0]
                    .iter_mut()
                    .zip(upper[k - 1].iter_mut())
                    .take(k - 1)
                    .skip(1)
                {
                    std::mem::swap(x, y);
                }
                let l = lam[k][k - 1].clone();
                let big = (Integer::from(&d[k - 2] * &d[k]) + Integer::from(l.square_ref()))
                    .div_exact(&d[k - 1]);
                for row in lam.iter_mut().take(kmax + 1).skip(k + 1) {
                    let t = row[k].clone();
                    row[k] = (Integer::from(&d[k] * &row[k - 1]) - Integer::from(&l * &t))
                        .div_exact(&d[k - 1]);
                    row[k - 1] =
                        (Integer::from(&big * &t) + Integer::from(&l * &row[k])).div_exact(&d[k]);
                }
                d[k - 1] = big;
                k = (k - 1).max(2);
            } else {
                for l in (1..k - 1).rev() {
                    red(&mut b, &d, &mut lam, k, l);
                }
                k += 1;
                break;
            }
        }
    }
    Some(b)
}

/// Whole numbers `a` with `sum a_i x_i` near zero, from numbers known to `digits` digits: the
/// candidate with the least coefficients, not yet checked.
fn candidate(values: &[Complex], digits: u32) -> Option<Vec<Integer>> {
    let n = values.len();
    let scale = Float::with_val(values[0].prec().0, Integer::from(10).pow(digits));
    let complex = values.iter().any(|v| !v.imag().is_zero());
    let mut rows = Vec::with_capacity(n);
    for (i, v) in values.iter().enumerate() {
        let mut row = vec![Integer::new(); n];
        row[i] = Integer::from(1);
        row.push(
            Float::with_val(scale.prec(), v.real() * &scale)
                .round()
                .to_integer()?,
        );
        if complex {
            row.push(
                Float::with_val(scale.prec(), v.imag() * &scale)
                    .round()
                    .to_integer()?,
            );
        }
        rows.push(row);
    }
    let reduced = lll(rows)?;
    let mut best = reduced.into_iter().next()?;
    best.truncate(n);
    if best.iter().all(|a| *a == 0) {
        return None;
    }
    // lowest terms, the first coefficient that is not zero positive
    let mut g = Integer::new();
    for a in &best {
        g = g.gcd(a);
    }
    let lead_negative = best.iter().find(|a| **a != 0).is_some_and(|a| *a < 0);
    for a in &mut best {
        *a = Integer::from(&*a / &g);
        if lead_negative {
            *a = Integer::from(-&*a);
        }
    }
    Some(best)
}

fn residual(values: &[Complex], a: &[Integer]) -> Float {
    let prec = values[0].prec().0;
    let mut total = Complex::new(prec);
    for (v, c) in values.iter().zip(a) {
        total += Complex::with_val(prec, v * c);
    }
    Float::with_val(prec, total.abs_ref())
}

fn largest(a: &[Integer]) -> Integer {
    a.iter().map(|x| x.clone().abs()).max().unwrap_or_default()
}

/// A whole-number relation between numbers given by `at(digits)`, found with `digits` digits
/// and confirmed with twice as many.
fn relation_checked(at: &dyn Fn(u32) -> Option<Vec<Complex>>, digits: u32) -> Option<Vec<Integer>> {
    let low = at(digits + 10)?;
    let a = candidate(&low, digits)?;
    let high = at(2 * digits + 20)?;
    let size = Float::with_val(64, &largest(&a))
        * high
            .iter()
            .map(|v| Float::with_val(64, v.abs_ref()))
            .fold(Float::with_val(64, 1), |m, v| m.max(&v));
    let bound = size * Float::with_val(64, 10).pow(-(2 * digits as i32 - 8));
    (residual(&high, &a) <= bound).then_some(a)
}

fn values_of(exprs: &[Expr], digits: u32) -> Option<Vec<Complex>> {
    exprs
        .iter()
        .map(|e| numeric::evalf_digits(e, digits).ok())
        .collect()
}

/// `findrelation([x1, ..., xn])`: whole numbers, not all zero, with `sum a_i x_i = 0`.
pub fn find_relation(exprs: &[Expr]) -> Res {
    if exprs.len() < 2 || exprs.iter().any(|e| !e.is_const()) {
        return Err("findrelation([x1, x2, ...]) needs two or more numbers".into());
    }
    let base = numeric::digits();
    for digits in [base, 2 * base, 4 * base] {
        if let Some(a) = relation_checked(&|d| values_of(exprs, d), digits) {
            return Ok(Expr::list(a.into_iter().map(Expr::integer).collect()));
        }
    }
    Err(format!(
        "no whole-number relation found between these numbers with {} digits",
        4 * base
    ))
}

/// The polynomial of least degree with whole coefficients that has the number as a root.
pub fn minimal_polynomial(e: &Expr, var: &str, max_degree: usize) -> Res {
    if !e.is_const() {
        return Err("minpoly needs a number".into());
    }
    let x = Expr::sym(var);
    if let Some(n) = e.as_num()
        && n.im.is_zero()
    {
        let (p, q) = (n.re.numer().clone(), n.re.denom().clone());
        return Ok(add(vec![
            mul(vec![Expr::integer(q), x]),
            neg(Expr::integer(p)),
        ]));
    }
    for degree in 1..=max_degree {
        let digits = numeric::digits().max(14 * (degree as u32 + 1) + 20);
        let powers = |d: u32| -> Option<Vec<Complex>> {
            let v = numeric::evalf_digits(e, d + 10).ok()?;
            let prec = v.prec().0;
            let mut out = vec![Complex::with_val(prec, 1)];
            for _ in 0..degree {
                let next = Complex::with_val(prec, out.last()? * &v);
                out.push(next);
            }
            Some(out)
        };
        if let Some(mut a) = relation_checked(&powers, digits) {
            if a.last().is_some_and(|c| *c < 0) {
                for c in &mut a {
                    *c = Integer::from(-&*c);
                }
            }
            if a.last().is_some_and(|c| *c == 0) {
                continue;
            }
            return Ok(add(a
                .into_iter()
                .enumerate()
                .map(|(k, c)| mul(vec![Expr::integer(c), powi(x.clone(), k as i64)]))
                .collect()));
        }
    }
    Err(format!(
        "no polynomial of degree {max_degree} or less with whole coefficients has this root"
    ))
}

/// `latticereduce(m)`: a reduced basis of the lattice of the rows of a matrix of whole or
/// rational numbers.
pub fn reduce(m: &Expr) -> Res {
    let usage = "latticereduce needs a matrix of whole or rational numbers with independent rows";
    let rows = crate::matrix::rows(m).ok_or(usage)?;
    let mut lcm = Integer::from(1);
    for c in rows.iter().flatten() {
        lcm = lcm.lcm(c.as_rat().ok_or(usage)?.denom());
    }
    let whole: Vec<Vec<Integer>> = rows
        .iter()
        .map(|r| {
            r.iter()
                .map(|c| {
                    let q = c.as_rat().expect("checked");
                    Integer::from(q.numer() * &lcm).div_exact(q.denom())
                })
                .collect()
        })
        .collect();
    let reduced = lll(whole).ok_or(usage)?;
    Ok(Expr::list(
        reduced
            .into_iter()
            .map(|r| {
                Expr::list(
                    r.into_iter()
                        .map(|c| div(Expr::integer(c), Expr::integer(lcm.clone())))
                        .collect(),
                )
            })
            .collect(),
    ))
}

fn small(a: &[Integer], digits: u32, n: usize) -> bool {
    // coefficients of a chance relation are about 10^(digits/n)
    let bound = Integer::from(10).pow((f64::from(digits) / n as f64 * 0.55) as u32);
    largest(a) <= bound
}

/// A root of a polynomial of degree 2 to 4 with small whole coefficients that agrees with `x`.
pub fn algebraic_guess(x: &Float, digits: u32, decimals: u32) -> Option<(Expr, usize)> {
    let prec = x.prec();
    let v = Complex::with_val(prec, x);
    let reliable = digits.saturating_sub(3);
    for degree in 2..=4usize {
        let mut powers = vec![Complex::with_val(prec, 1)];
        for _ in 0..degree {
            let next = Complex::with_val(prec, powers.last()? * &v);
            powers.push(next);
        }
        let Some(a) = candidate(&powers, decimals) else {
            continue;
        };
        if !small(&a, reliable, degree + 1) || a.last().is_some_and(|c| *c == 0) {
            continue;
        }
        let bound = Float::with_val(64, &largest(&a))
            * (Float::with_val(64, x.abs_ref()) + 1u32).pow(degree as u32)
            * Float::with_val(64, 10).pow(-(decimals as i32 - 3));
        if residual(&powers, &a) > bound {
            continue;
        }
        let t = Expr::sym("#r");
        let poly = add(a
            .iter()
            .enumerate()
            .map(|(k, c)| mul(vec![Expr::integer(c.clone()), powi(t.clone(), k as i64)]))
            .collect());
        let Ok(solved) = crate::solve::solve(&poly, "#r") else {
            continue;
        };
        crate::solve::release(&solved.params);
        let mut best: Option<(Float, Expr)> = None;
        for r in solved.roots.iter().filter(|r| !r.approx) {
            let Ok(value) = numeric::evalf_digits(&r.value, digits) else {
                continue;
            };
            let gap = Float::with_val(64, Complex::with_val(prec, &value - x).abs_ref());
            if best.as_ref().is_none_or(|(g, _)| gap < *g) {
                best = Some((gap, r.value.clone()));
            }
        }
        if let Some((_, root)) = best {
            // the digits of the coefficients are what the guess costs
            let heavy = a
                .iter()
                .filter(|c| **c != 0 && Integer::from(c.abs_ref()) != 1)
                .map(|c| Integer::from(c.abs_ref()).to_string().len())
                .sum::<usize>()
                + degree;
            return Some((crate::simplify::simplify(&root), heavy));
        }
    }
    None
}

/// `x` as a combination with small whole numbers of 1 and well-known constants, or as a product
/// of powers of small primes, pi and e.
pub fn constant_guess(x: &Float, digits: u32, decimals: u32) -> Vec<Expr> {
    let prec = x.prec();
    let reliable = digits.saturating_sub(3);
    let f1 = |name: &str, a: Expr| func(name, vec![a]);
    let pi = Expr::pi();
    let single: Vec<Expr> = vec![
        pi.clone(),
        powi(pi.clone(), 2),
        Expr::e(),
        f1("ln", Expr::int(2)),
        f1("ln", Expr::int(3)),
        f1("ln", Expr::int(5)),
        Expr::sym("eulergamma"),
        Expr::sym("catalan"),
        f1("zeta", Expr::int(3)),
        crate::canon::sqrt(pi.clone()),
        powi(pi.clone(), -1),
        powi(pi.clone(), 3),
        f1("ln", pi.clone()),
        powi(Expr::e(), 2),
        powi(Expr::e(), -1),
    ];
    let single: Vec<Expr> = single
        .into_iter()
        .chain([
            f1("lambertw", Expr::int(1)),
            f1("ln", Expr::int(10)),
            f1("gamma", Expr::rat(1, 4)),
            f1("gamma", Expr::rat(1, 3)),
            crate::canon::sqrt(Expr::e()),
        ])
        .collect();
    let pairs: Vec<Vec<Expr>> = vec![
        vec![pi.clone(), f1("ln", Expr::int(2))],
        vec![pi.clone(), Expr::e()],
        vec![pi.clone(), crate::canon::sqrt(Expr::int(3))],
        vec![pi.clone(), crate::canon::sqrt(Expr::int(2))],
        vec![f1("ln", Expr::int(2)), f1("ln", Expr::int(3))],
        vec![powi(pi.clone(), 2), powi(f1("ln", Expr::int(2)), 2)],
        vec![pi.clone(), powi(pi.clone(), 2)],
        vec![Expr::e(), powi(Expr::e(), 2)],
    ];
    let value = |e: &Expr| {
        numeric::evalf(e, prec + 32)
            .ok()
            .map(|v| Complex::with_val(prec, v))
    };
    let mut found = Vec::new();
    let mut try_set = |set: &[Expr]| {
        let mut values = vec![Complex::with_val(prec, x), Complex::with_val(prec, 1)];
        for k in set {
            match value(k) {
                Some(v) => values.push(v),
                None => return,
            }
        }
        let Some(a) = candidate(&values, decimals) else {
            return;
        };
        if a[0] == 0 || !small(&a, reliable, values.len()) || a[2..].iter().all(|c| *c == 0) {
            return;
        }
        // a0 x + a1 + sum a_k K_k = 0
        let mut terms = vec![Expr::integer(a[1].clone())];
        for (c, k) in a[2..].iter().zip(set) {
            terms.push(mul(vec![Expr::integer(c.clone()), k.clone()]));
        }
        found.push(crate::simplify::simplify(&neg(div(
            add(terms),
            Expr::integer(a[0].clone()),
        ))));
    };
    for k in &single {
        try_set(std::slice::from_ref(k));
    }
    for set in &pairs {
        try_set(set);
    }
    // ln|x| = a + b pi: e to a combination of 1 and pi
    if !x.is_zero() {
        let values = vec![
            Complex::with_val(prec, Float::with_val(prec, x.abs_ref()).ln()),
            Complex::with_val(prec, 1),
            Complex::with_val(prec, rug::float::Constant::Pi),
        ];
        if let Some(a) = candidate(&values, reliable.min(decimals + 3))
            && a[0] != 0
            && a[2] != 0
            && small(&a, reliable, 3)
        {
            let exponent = neg(div(
                add(vec![
                    Expr::integer(a[1].clone()),
                    mul(vec![Expr::integer(a[2].clone()), pi.clone()]),
                ]),
                Expr::integer(a[0].clone()),
            ));
            let value = pow(Expr::e(), exponent);
            found.push(if x.is_sign_negative() {
                neg(value)
            } else {
                value
            });
        }
    }
    // ln|x| as a combination of logarithms: a product of powers
    if !x.is_zero() {
        let bases = [
            Expr::int(2),
            Expr::int(3),
            Expr::int(5),
            Expr::int(7),
            pi.clone(),
            Expr::e(),
        ];
        let mut values = vec![Complex::with_val(
            prec,
            Float::with_val(prec, x.abs_ref()).ln(),
        )];
        for b in &bases {
            if let Some(v) = value(&f1("ln", b.clone())) {
                values.push(v);
            }
        }
        if values.len() == bases.len() + 1
            && let Some(a) = candidate(&values, reliable.min(decimals + 3))
            && a[0] != 0
            && small(&a, reliable, values.len())
        {
            let product = mul(bases
                .iter()
                .zip(&a[1..])
                .filter(|(_, c)| **c != 0)
                .map(|(b, c)| {
                    pow(
                        b.clone(),
                        neg(div(Expr::integer(c.clone()), Expr::integer(a[0].clone()))),
                    )
                })
                .collect());
            found.push(if x.is_sign_negative() {
                neg(product)
            } else {
                product
            });
        }
    }
    found
}
