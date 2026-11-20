//! Simplification beyond the canonical form: several rewrites are tried and the smallest result wins.

use crate::expand::expand;
use crate::expr::Expr;
use crate::numeric;
use crate::ratsimp;

/// Size of the printed form, the measure of simplicity.
pub fn cost(e: &Expr) -> usize {
    e.to_string().len()
}

/// Whether the expression is identically zero.
pub fn is_zero(e: &Expr) -> bool {
    if e.is_zero() {
        return true;
    }
    if e.is_number() || e.is_infinite() {
        return false;
    }
    // (a tower of powers is not multiplied out: its values tell)
    if !towering(e) && ratsimp::is_zero(e) == Some(true) {
        return true;
    }
    // tetration of a variable is not tried at sample points: every value is a run of the program
    if !e.is_const() && crate::tetration::uses_tet(e) {
        return false;
    }
    // only a sum can vanish without one of its parts doing so
    matches!(e, Expr::Add(_)) && numeric::is_zero_identity(e) == Some(true)
}

/// A normal form that recognises zero.
pub fn normal(e: &Expr) -> Expr {
    if e.is_number() {
        return e.clone();
    }
    if is_zero(e) {
        return Expr::zero();
    }
    if towering(e) {
        return e.clone();
    }
    let t = ratsimp::together(e);
    if cost(&t) <= cost(e) { t } else { e.clone() }
}

fn better(best: &mut Expr, best_cost: &mut usize, candidate: Expr) {
    let c = cost(&candidate);
    if c < *best_cost {
        *best = candidate;
        *best_cost = c;
    }
}

/// The expression with the arguments of its functions and the bases of its roots simplified.
fn simplify_parts(e: &Expr) -> Expr {
    match e {
        Expr::Add(a) => crate::canon::add(a.iter().map(simplify_parts).collect()),
        Expr::Mul(a) => crate::canon::mul(a.iter().map(simplify_parts).collect()),
        Expr::Pow(b, x) if x.as_int().is_some() => {
            crate::canon::pow(simplify_parts(b), (**x).clone())
        }
        Expr::Pow(b, x) if !b.is_number() || !x.is_number() => {
            crate::canon::pow(simplify(b), simplify(x))
        }
        Expr::Fn(n, a)
            if !matches!(&**n, "hold" | "ordered" | "quat" | "based" | "string")
                && !a.is_empty()
                && e.size() <= 400 =>
        {
            crate::funcs::func(n, a.iter().map(simplify).collect())
        }
        _ => e.clone(),
    }
}

pub fn simplify(e: &Expr) -> Expr {
    match e {
        Expr::Num(_) | Expr::Float(_) | Expr::Sym(_) | Expr::Bool(_) => return e.clone(),
        Expr::List(_) | Expr::Rel(..) => return crate::canon::rebuild(e, &mut |c| simplify(c)),
        Expr::Fn(n, _) if matches!(&**n, "hold" | "based" | "string") => return e.clone(),
        Expr::Fn(n, _) if &**n == "quat" => {
            return crate::quat::simple(e).unwrap_or_else(|| e.clone());
        }
        _ => {}
    }
    if e.is_infinite() || crate::budget::expired() {
        return e.clone();
    }
    if is_zero(e) {
        return Expr::zero();
    }
    let parts = simplify_parts(e);
    let parts = if parts.has_fn("gamma") || parts.has_fn("factorial") {
        crate::canon::map_bottom_up(&parts, &mut gamma_ratios)
    } else {
        parts
    };
    let e = &parts;
    if e.is_number() || matches!(e, Expr::Sym(_)) {
        return parts;
    }
    let mut best = e.clone();
    let mut best_cost = cost(e);
    // a tower of powers has a degree, and a tower of functions a count of forms, that grows
    // with each floor as the floors below taken together: such a tower is left standing
    let (degree, depth) = (degree_bound(e), kernel_depth(e));
    let terms = dense_terms(degree, kernel_count(e));
    if !towering(e) {
        better(&mut best, &mut best_cost, expand(e));
    }
    if e.size() <= 400 && degree <= 128 && depth <= 8 {
        let t = ratsimp::together(e);
        better(&mut best, &mut best_cost, t.clone());
        if e.size() <= 150 {
            // in several variables the factors of a high degree take long to find
            if terms <= 150 {
                better(&mut best, &mut best_cost, ratsimp::factor(&t));
            }
            better(&mut best, &mut best_cost, ratsimp::trigsimp(e));
        }
    }
    best
}

/// No less than the degree of the expression as a polynomial in its symbols and functions,
/// were its whole powers multiplied out.
fn degree_bound(e: &Expr) -> u64 {
    match e {
        Expr::Sym(_) | Expr::Fn(..) => 1,
        Expr::Add(a) => a.iter().map(degree_bound).max().unwrap_or(0),
        Expr::Mul(a) => a.iter().map(degree_bound).fold(0, u64::saturating_add),
        Expr::Pow(b, x) => match x.as_i64() {
            Some(n) => degree_bound(b).saturating_mul(n.unsigned_abs()),
            None => 1,
        },
        Expr::List(a) => a.iter().map(degree_bound).max().unwrap_or(0),
        _ => 0,
    }
}

/// How many different symbols, functions and powers that are not whole the expression is a
/// polynomial in; counting stops at 8.
fn kernel_count(e: &Expr) -> usize {
    fn collect<'a>(e: &'a Expr, seen: &mut std::collections::HashSet<&'a Expr>) {
        if seen.len() >= 8 {
            return;
        }
        match e {
            Expr::Sym(_) | Expr::Fn(..) => {
                seen.insert(e);
            }
            Expr::Add(a) | Expr::Mul(a) | Expr::List(a) => a.iter().for_each(|x| collect(x, seen)),
            Expr::Pow(b, x) if x.as_i64().is_some() => collect(b, seen),
            Expr::Pow(..) => {
                seen.insert(e);
            }
            _ => {}
        }
    }
    let mut seen = std::collections::HashSet::new();
    collect(e, &mut seen);
    seen.len()
}

/// Quotients of gamma functions (and factorials) whose arguments differ by a whole number:
/// `gamma(x + 2)/gamma(x)` is `x (x + 1)`.
fn gamma_ratios(e: Expr) -> Expr {
    use crate::canon::{add, mul, pow, sub};
    let Expr::Mul(factors) = &e else {
        return e;
    };
    // (the argument as one of gamma, the power, the place among the factors)
    let mut found: Vec<(Expr, i64, usize)> = Vec::new();
    for (at, f) in factors.iter().enumerate() {
        let (base, power) = match f {
            Expr::Pow(b, p) => match p.as_i64() {
                Some(p) => (&**b, p),
                None => continue,
            },
            other => (other, 1),
        };
        if let Some(u) = base.fn_arg("gamma") {
            found.push((u.clone(), power, at));
        } else if let Some(u) = base.fn_arg("factorial") {
            found.push((add(vec![u.clone(), Expr::one()]), power, at));
        }
    }
    if found.len() < 2 {
        return e;
    }
    let mut extra = Vec::new();
    let mut changed = false;
    // gamma(u) gamma(m - u) for a whole m: the reflection pi/sin(pi u), with the factors
    // between 1 - u and m - u
    for i in 0..found.len() {
        for j in i + 1..found.len() {
            while found[i].1 != 0 && found[j].1 != 0 && (found[i].1 > 0) == (found[j].1 > 0) {
                let m = match add(vec![found[i].0.clone(), found[j].0.clone()]).as_i64() {
                    Some(m) if m.abs() <= 64 => m,
                    _ => break,
                };
                let u = found[i].0.clone();
                if u.as_rat().is_some_and(|r| r.is_integer()) {
                    break;
                }
                let sign = if found[i].1 > 0 { 1 } else { -1 };
                // sin(pi (n + w)) = (-1)^n sin(pi w) for the whole part n of u
                let number = match &u {
                    Expr::Add(terms) => terms.iter().find_map(Expr::as_num),
                    other => other.as_num(),
                };
                let nearest = number
                    .map(|n| (n.re.clone() + crate::num::rat(1, 2)).floor())
                    .and_then(|r| r.numer().to_i64());
                let whole = nearest.filter(|n| n.abs() <= 64).unwrap_or(0);
                let w = sub(u.clone(), Expr::int(whole));
                // (and sin(pi (1/2 + z)) = cos(pi z))
                let half = number.is_some_and(|n| {
                    (n.re.clone() - rug::Rational::from(whole)) == crate::num::rat(-1, 2)
                });
                let sine = if half {
                    let z = add(vec![w, Expr::half()]);
                    mul(vec![
                        Expr::minus_one(),
                        crate::funcs::func("cos", vec![mul(vec![Expr::pi(), z])]),
                    ])
                } else {
                    crate::funcs::func("sin", vec![mul(vec![Expr::pi(), w])])
                };
                let mut run = vec![
                    Expr::int(if whole % 2 == 0 { 1 } else { -1 }),
                    Expr::pi(),
                    pow(sine, Expr::minus_one()),
                ];
                // gamma(m - u) = gamma(1 - u) (1 - u) ... (m - 1 - u)
                for t in 1..m {
                    run.push(sub(Expr::int(t), u.clone()));
                }
                for t in m..1 {
                    run.push(pow(sub(Expr::int(t), u.clone()), Expr::minus_one()));
                }
                extra.push(pow(mul(run), Expr::int(sign)));
                found[i].1 -= sign;
                found[j].1 -= sign;
                changed = true;
            }
        }
    }
    for i in 0..found.len() {
        for j in 0..found.len() {
            while found[i].1 > 0 && found[j].1 < 0 {
                // gamma(u)/gamma(v) with u = v + d
                let d = match crate::canon::sub(found[i].0.clone(), found[j].0.clone()).as_i64() {
                    Some(d) if d.abs() <= 64 => d,
                    _ => break,
                };
                let v = &found[j].0;
                let run: Vec<Expr> = if d >= 0 {
                    (0..d).map(|t| add(vec![v.clone(), Expr::int(t)])).collect()
                } else {
                    (1..=-d)
                        .map(|t| pow(sub(v.clone(), Expr::int(t)), Expr::minus_one()))
                        .collect()
                };
                extra.extend(run);
                found[i].1 -= 1;
                found[j].1 += 1;
                changed = true;
            }
        }
    }
    if !changed {
        return e;
    }
    let mut out: Vec<Expr> = factors
        .iter()
        .enumerate()
        .filter(|(at, _)| !found.iter().any(|f| f.2 == *at))
        .map(|(_, f)| f.clone())
        .collect();
    for (_, power, at) in &found {
        let base = match &factors[*at] {
            Expr::Pow(b, _) => (**b).clone(),
            other => other.clone(),
        };
        out.push(pow(base, Expr::int(*power)));
    }
    out.extend(extra);
    mul(out)
}

/// How many terms the expression has at most with its products and whole powers multiplied out
/// and nothing collected.
fn term_bound(e: &Expr) -> u64 {
    const CAP: u64 = 1 << 40;
    match e {
        Expr::Add(a) => a
            .iter()
            .map(term_bound)
            .fold(0, u64::saturating_add)
            .min(CAP),
        Expr::Mul(a) => a
            .iter()
            .map(term_bound)
            .fold(1, u64::saturating_mul)
            .min(CAP),
        Expr::Pow(b, x) => match (term_bound(b), x.as_i64().map(i64::unsigned_abs)) {
            (1, _) | (_, None) => 1,
            // the terms of a power: the ways to pick its factors with repeats
            (t, Some(n)) => {
                let mut ways = 1u64;
                for k in 1..=n.min(t - 1) {
                    ways = ways.saturating_mul(t + n - k) / k;
                    if ways > CAP {
                        return CAP;
                    }
                }
                ways
            }
        },
        _ => 1,
    }
}

/// Whether the expression multiplied out is too large to work with: a tower of powers, or a
/// high power in several variables.
fn towering(e: &Expr) -> bool {
    let degree = degree_bound(e);
    degree > 2048 || term_bound(e).min(dense_terms(degree, kernel_count(e))) > 2100
}

/// How many terms a polynomial of this degree in so many variables has at most.
fn dense_terms(degree: u64, variables: usize) -> u64 {
    let mut terms = 1u64;
    for k in 1..=variables as u64 {
        terms = terms.saturating_mul(degree + k) / k;
        if terms > 1 << 40 {
            break;
        }
    }
    terms
}

/// How deep functions and powers that are not whole stand in each other.
fn kernel_depth(e: &Expr) -> u32 {
    let deepest = |a: &[Expr]| a.iter().map(kernel_depth).max().unwrap_or(0);
    match e {
        Expr::Add(a) | Expr::Mul(a) | Expr::List(a) => deepest(a),
        Expr::Fn(_, a) => 1 + deepest(a),
        Expr::Pow(b, x) if x.as_i64().is_some() => kernel_depth(b),
        Expr::Pow(b, x) => 1 + kernel_depth(b).max(kernel_depth(x)),
        _ => 0,
    }
}
