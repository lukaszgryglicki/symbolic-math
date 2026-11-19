//! Algebraic expansion of products and integer powers of sums.

use crate::canon::{add, mul, pow, rebuild};
use crate::expr::Expr;

fn mul_expand(a: &Expr, b: &Expr) -> Expr {
    match (a, b) {
        (Expr::Add(x), Expr::Add(y)) => {
            let mut terms = Vec::with_capacity(x.len() * y.len());
            for s in x.iter() {
                for t in y.iter() {
                    terms.push(mul(vec![s.clone(), t.clone()]));
                }
            }
            add(terms)
        }
        (Expr::Add(x), _) => add(x.iter().map(|s| mul(vec![s.clone(), b.clone()])).collect()),
        (_, Expr::Add(y)) => add(y.iter().map(|t| mul(vec![a.clone(), t.clone()])).collect()),
        _ => mul(vec![a.clone(), b.clone()]),
    }
}

fn pow_expand(b: &Expr, n: u32) -> Expr {
    let (mut base, mut acc, mut k) = (b.clone(), Expr::one(), n);
    while k > 0 {
        if k & 1 == 1 {
            acc = mul_expand(&acc, &base);
        }
        k >>= 1;
        if k > 0 {
            base = mul_expand(&base, &base);
        }
    }
    acc
}

pub fn expand(e: &Expr) -> Expr {
    match e {
        Expr::Add(a) => add(a.iter().map(expand).collect()),
        Expr::Mul(a) => a
            .iter()
            .map(expand)
            .fold(Expr::one(), |acc, p| mul_expand(&acc, &p)),
        Expr::Pow(b, x) => {
            let (b, x) = (expand(b), expand(x));
            match x.as_i64() {
                Some(n) if matches!(b, Expr::Add(_)) && (2..=4096).contains(&n) => {
                    pow_expand(&b, n as u32)
                }
                Some(n) if matches!(b, Expr::Add(_)) && (-4096..=-2).contains(&n) => {
                    pow(pow_expand(&b, n.unsigned_abs() as u32), Expr::minus_one())
                }
                _ => {
                    let p = pow(b, x);
                    if matches!(p, Expr::Mul(_)) {
                        expand_once(&p)
                    } else {
                        p
                    }
                }
            }
        }
        Expr::Fn(..) | Expr::List(_) | Expr::Rel(..) => rebuild(e, &mut |c| expand(c)),
        _ => e.clone(),
    }
}

fn expand_once(e: &Expr) -> Expr {
    match e {
        Expr::Mul(a) => a.iter().fold(Expr::one(), |acc, p| mul_expand(&acc, p)),
        _ => e.clone(),
    }
}

/// Expansion of the numerator only: `(a + b)/c` keeps one denominator.
pub fn expand_numerator(e: &Expr) -> Expr {
    let (num, den) = crate::canon::numer_denom(e);
    crate::canon::div(expand(&num), den)
}
