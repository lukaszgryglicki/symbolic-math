//! Functional equations for one unknown function of one argument:
//! `f(x) + 2 f(1/x) = 3x`, `f(x + y) = f(x) + f(y)`, `f(2x) = 4 f(x)`, ...

use crate::canon::{add, div, map_bottom_up, mul, neg, pow, powi, sub, subst_sym};
use crate::expand::expand;
use crate::expr::Expr;
use crate::funcs::func;
use crate::num::Rat;
use crate::numeric::{self, Env};
use crate::simplify::{is_zero, simplify};
use crate::{ratsimp, recur, solve, system};
use rug::{Complex, Float};
use std::rc::Rc;

pub struct Found {
    /// The argument the solutions are written in.
    pub var: String,
    pub solutions: Vec<Expr>,
    pub notes: Vec<String>,
}

fn sym(name: &str) -> Expr {
    Expr::sym(name)
}

/// The distinct arguments the unknown function is called with.
fn calls(eq: &Expr, f: &str) -> Vec<Expr> {
    let mut out: Vec<Expr> = Vec::new();
    eq.walk(&mut |e| {
        if let Expr::Fn(n, a) = e
            && **n == *f
            && a.len() == 1
            && !out.contains(&a[0])
        {
            out.push(a[0].clone());
        }
    });
    out
}

/// Every call `f(arg)` replaced; `None` when a call cannot be.
fn replace(eq: &Expr, f: &str, with: &mut dyn FnMut(&Expr) -> Option<Expr>) -> Option<Expr> {
    let mut failed = false;
    let out = map_bottom_up(eq, &mut |e| match &e {
        Expr::Fn(n, a) if **n == *f && a.len() == 1 => match with(&a[0]) {
            Some(v) => v,
            None => {
                failed = true;
                e
            }
        },
        _ => e,
    });
    (!failed).then_some(out)
}

/// The equation with `f(arg)` set to `candidate` at `x = arg`.
fn plug(eq: &Expr, f: &str, x: &str, candidate: &Expr) -> Expr {
    replace(eq, f, &mut |arg| Some(subst_sym(candidate, x, arg))).unwrap_or_else(|| eq.clone())
}

/// Whether the candidate satisfies the equation at positive real sample points, whatever its
/// constants are.
fn holds(eq: &Expr, f: &str, x: &str, candidate: &Expr) -> bool {
    let residual = plug(eq, f, x, candidate);
    if residual.has_fn(f) || residual.is_undefined() {
        return false;
    }
    if residual.is_zero() {
        return true;
    }
    let names: Vec<Rc<str>> = residual.free_syms().into_iter().collect();
    // logarithms and powers live on the positive numbers; everything else must hold for both signs
    let mut positive_only = false;
    residual.walk(&mut |e| match e {
        Expr::Fn(n, _) if &**n == "ln" => positive_only = true,
        Expr::Pow(b, p) if p.as_int().is_none() && !b.is_number() && !b.is_sym("e") => {
            positive_only = true
        }
        _ => {}
    });
    let mut passed = 0;
    for trial in 0..8u32 {
        let mut env = Env::new();
        for (k, name) in names.iter().enumerate() {
            let seed = f64::from(trial * 7 + 3) * 0.6180339887 + (k as f64 + 1.0) * 0.4142135623;
            let mut value = 0.35 + (seed - seed.floor()) * 1.9;
            if !positive_only && (trial as usize + k) % 2 == 1 {
                value = -value;
            }
            env.insert(name.clone(), Complex::with_val(160, (value, 0)));
        }
        let Ok(v) = numeric::evalf_env(&residual, 160, &env) else {
            continue;
        };
        // the size of the terms tells how small a zero has to be
        let mut scale = Float::with_val(64, 1);
        if let Expr::Add(terms) = &residual {
            for t in terms.iter() {
                if let Ok(part) = numeric::evalf_env(t, 160, &env) {
                    scale = scale.max(&Float::with_val(64, part.abs_ref()));
                }
            }
        }
        if Float::with_val(64, v.abs_ref()) > scale * 1e-30 {
            return false;
        }
        passed += 1;
        if passed == 4 {
            return true;
        }
    }
    passed >= 2
}

struct Constants {
    taken: Vec<String>,
    next: usize,
}

impl Constants {
    fn fresh(&mut self) -> Expr {
        loop {
            self.next += 1;
            let name = format!("C{}", self.next);
            if !self.taken.contains(&name) {
                return sym(&name);
            }
        }
    }
}

/// Internal names left in a candidate become constants `C1`, `C2`, ...
fn name_constants(e: &Expr, constants: &mut Constants) -> Expr {
    let mut out = e.clone();
    for s in e.free_syms() {
        if s.starts_with("#q") {
            out = subst_sym(&out, &s, &constants.fresh());
        }
    }
    out
}

/// Solves `u(x) = t` for `x` when there is one solution.
fn invert(u: &Expr, x: &str) -> Option<Expr> {
    let solved = solve::solve(&sub(u.clone(), sym("#t")), x).ok()?;
    solve::release(&solved.params);
    match &solved.roots[..] {
        [one] if !one.approx && solved.params.iter().all(|p| !one.value.has_sym(p)) => {
            Some(one.value.clone())
        }
        _ => None,
    }
}

/// A linear equation whose arguments form a finite orbit: the copies of the equation at the
/// points of the orbit are solved for the values of the function there.
fn orbit(eq: &Expr, f: &str, x: &str, open: &mut bool) -> Option<Expr> {
    let mut eq = eq.clone();
    let mut args = calls(&eq, f);
    if args.is_empty() {
        return None;
    }
    if !args.iter().any(|a| a.is_sym(x)) {
        // the first argument becomes the variable
        let back = invert(&args[0], x)?;
        eq = subst_sym(&subst_sym(&eq, x, &back), "#t", &sym(x));
        eq = replace(&eq, f, &mut |arg| Some(Expr::call(f, vec![simplify(arg)])))?;
        args = calls(&eq, f);
        if !args.iter().any(|a| a.is_sym(x)) {
            return None;
        }
    }
    let same = |a: &Expr, b: &Expr| a == b || is_zero(&sub(a.clone(), b.clone()));
    let mut points = vec![sym(x)];
    let mut k = 0;
    while k < points.len() {
        for m in &args {
            let v = simplify(&subst_sym(m, x, &points[k]));
            if !points.iter().any(|p| same(p, &v)) {
                points.push(v);
                if points.len() > 12 {
                    return None;
                }
            }
        }
        k += 1;
    }
    let names: Vec<String> = (0..points.len()).map(|k| format!("#F{k}")).collect();
    let mut eqs = Vec::new();
    for p in &points {
        let at = subst_sym(&eq, x, p);
        eqs.push(replace(&at, f, &mut |arg| {
            let arg = simplify(arg);
            let k = points.iter().position(|q| same(q, &arg))?;
            Some(sym(&names[k]))
        })?);
    }
    // linear in the values
    for e in &eqs {
        for n in &names {
            if names.iter().any(|m| crate::diff::diff(e, n).has_sym(m)) {
                return None;
            }
        }
    }
    let solved = system::solve_system(&eqs, &names).ok()?;
    let value = solved
        .solutions
        .first()?
        .iter()
        .find(|(n, _)| *n == names[0])?
        .1
        .clone();
    if names.iter().any(|n| value.has_sym(n)) {
        // the copies of the equation do not fix the values on the orbit
        *open = true;
        return None;
    }
    Some(simplify(&value))
}

fn rational(e: &Expr) -> Option<Rat> {
    simplify(e).as_rat().cloned()
}

/// `m` with `q = k^m` for whole `m`.
fn whole_log(q: &Rat, k: &Rat) -> Option<i64> {
    let ratio = (q.to_f64().ln() / k.to_f64().ln()).round();
    if !(-64.0..=64.0).contains(&ratio) {
        return None;
    }
    let m = ratio as i64;
    let power = if m >= 0 {
        Rat::from(rug::ops::Pow::pow(k, m as u32))
    } else {
        Rat::from(rug::ops::Pow::pow(k.clone().recip(), (-m) as u32))
    };
    (power == *q).then_some(m)
}

/// Whole exponents `m_j` and a base `k` with `q_j = k^(m_j)` for all the positive numbers `q`.
fn common_base(q: &[Rat]) -> Option<(Rat, Vec<i64>)> {
    let mut bases: Vec<Rat> = Vec::new();
    for x in q.iter().filter(|x| **x != 1) {
        let x = if *x < 1 { x.clone().recip() } else { x.clone() };
        bases.push(x.clone());
        for root in 2..=6u32 {
            let (n, d) = (x.numer().clone().root(root), x.denom().clone().root(root));
            let r = Rat::from((n, d));
            if Rat::from(rug::ops::Pow::pow(&r, root)) == x {
                bases.push(r);
            }
        }
    }
    bases.sort();
    bases.into_iter().find_map(|k| {
        Some((
            k.clone(),
            q.iter().map(|x| whole_log(x, &k)).collect::<Option<_>>()?,
        ))
    })
}

/// `b^(A ln(x) + B)` written as `x^(A ln(b)) b^B`.
fn tidy_powers(e: &Expr, x: &str) -> Expr {
    map_bottom_up(e, &mut |node| {
        if let Expr::Pow(b, exponent) = &node
            && b.as_rat().is_some_and(|r| *r > 0)
            && exponent.has_sym(x)
        {
            // the outermost logarithm with the variable inside
            let mut log = None;
            fn outer(e: &Expr, x: &str, found: &mut Option<Expr>) {
                match e {
                    Expr::Fn(n, a) if &**n == "ln" && a[0].has_sym(x) => {
                        found.get_or_insert_with(|| e.clone());
                    }
                    Expr::Add(a) | Expr::Mul(a) => a.iter().for_each(|t| outer(t, x, found)),
                    Expr::Pow(b, p) => {
                        outer(b, x, found);
                        outer(p, x, found);
                    }
                    _ => {}
                }
            }
            outer(exponent, x, &mut log);
            if let Some(log) = log
                && let Some(inner) = log.fn_arg("ln")
            {
                let lifted = crate::canon::subst(exponent, &log, &sym("#l"));
                if !lifted.has_sym(x)
                    && let Some(c) = ratsimp::coefficients(&lifted, "#l")
                    && c.len() == 2
                {
                    let slope = simplify(&mul(vec![c[1].clone(), func("ln", vec![(**b).clone()])]));
                    return mul(vec![
                        pow(inner.clone(), slope),
                        pow((**b).clone(), c[0].clone()),
                    ]);
                }
            }
        }
        node
    })
}

/// `A(n) a(n + 1) + B(n) a(n) = 0` with polynomials `A` and `B`: a product of gamma functions.
fn gamma_product(eq: &Expr, n: &str, shifts: &[i64]) -> Option<Expr> {
    if shifts != [0, 1] {
        return None;
    }
    let (m0, m1) = (recur::marker(0), recur::marker(1));
    let (b, a) = (crate::diff::diff(eq, &m0), crate::diff::diff(eq, &m1));
    let rest = subst_sym(&subst_sym(eq, &m0, &Expr::zero()), &m1, &Expr::zero());
    if [&a, &b].iter().any(|c| c.has_sym(&m0) || c.has_sym(&m1)) || !is_zero(&rest) {
        return None;
    }
    // a(n + 1) = ratio(n) a(n)
    let (top, low) = ratsimp::fraction(&ratsimp::together(&neg(div(b, a))));
    let ns = sym(n);
    let mut value = vec![sym("C1")];
    let mut lead = Vec::new();
    for (poly, sign) in [(&top, 1i64), (&low, -1)] {
        let c = ratsimp::coefficients(poly, n)?;
        lead.push(powi(c[c.len() - 1].clone(), sign));
        if c.len() == 1 {
            continue;
        }
        let solved = solve::solve(poly, n).ok()?;
        solve::release(&solved.params);
        if solved.roots.iter().any(|r| r.approx)
            || solved.roots.iter().map(|r| r.mult as usize).sum::<usize>() != c.len() - 1
        {
            return None;
        }
        for r in &solved.roots {
            let g = func("gamma", vec![sub(ns.clone(), r.value.clone())]);
            value.push(powi(g, sign * i64::from(r.mult)));
        }
    }
    let scale = simplify(&mul(lead));
    if !scale.is_one() {
        value.push(pow(scale, ns));
    }
    Some(mul(value))
}

/// Arguments `x + c`, `k x` or `x^p`: a linear recurrence with constant coefficients in disguise.
fn recurrence(eq: &Expr, f: &str, x: &str, notes: &mut Vec<String>) -> Option<Expr> {
    let args = calls(eq, f);
    if args.len() < 2 {
        return None;
    }
    let xs = sym(x);
    let n = "#n";
    let ns = sym(n);
    // (the whole shifts, x in terms of n, n in terms of x, what the constants may be)
    // shifts in units of a common constant (1 when they are rational numbers)
    let offsets: Vec<Expr> = args
        .iter()
        .map(|a| simplify(&sub(a.clone(), xs.clone())))
        .collect();
    let unit = offsets
        .iter()
        .find(|d| d.is_const() && !d.is_zero() && d.as_rat().is_none())
        .cloned()
        .unwrap_or_else(Expr::one);
    let (shifts, forward, back, period): (Vec<i64>, Expr, Expr, String) = if let Some(d) = offsets
        .iter()
        .map(|d| rational(&div(d.clone(), unit.clone())))
        .collect::<Option<Vec<Rat>>>()
    {
        let low = d.iter().min()?.clone();
        let offs: Vec<Rat> = d.iter().map(|r| Rat::from(r - &low)).collect();
        let den = offs
            .iter()
            .fold(rug::Integer::from(1), |acc, r| acc.lcm(r.denom()));
        let num = offs.iter().fold(rug::Integer::new(), |acc, r| {
            acc.gcd(&Rat::from(r * &den).numer().clone())
        });
        let h = Rat::from((num, den));
        if h == 0 {
            return None;
        }
        let shifts = offs
            .iter()
            .map(|r| Rat::from(r / &h).numer().to_i64())
            .collect::<Option<Vec<i64>>>()?;
        let step = mul(vec![Expr::rational(h), unit.clone()]);
        let start = mul(vec![Expr::rational(low), unit]);
        (
            shifts,
            sub(mul(vec![step.clone(), ns.clone()]), start.clone()),
            div(add(vec![xs.clone(), start]), step.clone()),
            format!("of period {step}"),
        )
    } else if let Some(q) = args
        .iter()
        .map(|a| rational(&div(a.clone(), xs.clone())).filter(|r| *r > 0))
        .collect::<Option<Vec<Rat>>>()
    {
        let (k, shifts) = common_base(&q)?;
        let base = Expr::rational(k.clone());
        (
            shifts,
            pow(base.clone(), ns.clone()),
            div(func("ln", vec![xs.clone()]), func("ln", vec![base])),
            format!("unchanged when {x} is multiplied by {k}"),
        )
    } else {
        let p = args
            .iter()
            .map(|a| match a {
                Expr::Sym(s) if **s == *x => Some(Rat::from(1)),
                Expr::Pow(b, e) if b.is_sym(x) => e.as_rat().filter(|r| **r > 0).cloned(),
                _ => None,
            })
            .collect::<Option<Vec<Rat>>>()?;
        let (k, shifts) = common_base(&p)?;
        let base = Expr::rational(k.clone());
        (
            shifts,
            pow(Expr::e(), pow(base.clone(), ns.clone())),
            div(
                func("ln", vec![func("ln", vec![xs.clone()])]),
                func("ln", vec![base]),
            ),
            format!("unchanged when {x} is raised to the power {k}"),
        )
    };
    let low = *shifts.iter().min()?;
    let shifts: Vec<i64> = shifts.iter().map(|s| s - low).collect();
    let mut k = 0;
    let marked = replace(eq, f, &mut |arg| {
        let index = args.iter().position(|a| a == arg)?;
        k += 1;
        Some(sym(&recur::marker(shifts[index])))
    })?;
    let _ = k;
    let in_n = simplify(&subst_sym(&marked, x, &forward));
    let mut distinct = shifts.clone();
    distinct.sort_unstable();
    distinct.dedup();
    let sequence = match recur::rsolve(&in_n, n, &distinct, &[]) {
        Ok(found) => found,
        Err(_) => gamma_product(&in_n, n, &distinct)?,
    };
    // a(n) is the function at the point of index n + low
    let value = subst_sym(&sequence, n, &sub(back, Expr::int(low)));
    let value = simplify(&tidy_powers(&simplify(&value), x));
    if value.free_syms().iter().any(|s| s.starts_with('C')) {
        notes.push(format!("the constants may also be functions {period}"));
    }
    Some(value)
}

/// The coefficients of `e` as a polynomial in all the generators; `None` when it is not one.
fn coefficients_in(e: &Expr, gens: &[String]) -> Option<Vec<Expr>> {
    let Some((first, rest)) = gens.split_first() else {
        return Some(vec![e.clone()]);
    };
    if !e.has_sym(first) {
        return coefficients_in(e, rest);
    }
    let mut out = Vec::new();
    for c in ratsimp::coefficients(e, first)? {
        out.extend(coefficients_in(&c, rest)?);
    }
    Some(out)
}

/// `arg = sum p_i v_i + r` with numbers `p_i`.
fn linear_form(arg: &Expr, vars: &[String]) -> Option<(Vec<Rat>, Expr)> {
    let mut rest = expand(arg);
    let mut p = Vec::new();
    for v in vars {
        let c = ratsimp::coefficients(&rest, v)?;
        match c.len() {
            1 => p.push(Rat::new()),
            2 => {
                p.push(c[1].as_rat()?.clone());
                rest = c[0].clone();
            }
            _ => return None,
        }
    }
    (!vars.iter().any(|v| rest.has_sym(v))).then_some((p, rest))
}

/// `arg = m prod v_i^(p_i)` with numbers `p_i`.
fn monomial_form(arg: &Expr, vars: &[String]) -> Option<(Vec<Rat>, Expr)> {
    let factors: Vec<Expr> = match arg {
        Expr::Mul(a) => a.to_vec(),
        other => vec![other.clone()],
    };
    let mut p = vec![Rat::new(); vars.len()];
    let mut rest = Vec::new();
    for t in factors {
        let (base, exponent) = match &t {
            Expr::Pow(b, e) => ((**b).clone(), e.as_rat().cloned()),
            other => (other.clone(), Some(Rat::from(1))),
        };
        match vars.iter().position(|v| base.is_sym(v)) {
            Some(k) => p[k] += exponent?,
            None if vars.iter().any(|v| t.has_sym(v)) => return None,
            None => rest.push(t),
        }
    }
    Some((p, mul(rest)))
}

/// A family of candidates: the value at an argument in terms of unknown coefficients and of
/// generators that stand for independent functions of the variables.
struct Family {
    /// The unknown coefficients to solve for.
    unknowns: Vec<String>,
    /// Generators next to the variables themselves.
    gens: Vec<String>,
    /// The candidate as a function of `x` once the unknowns are known.
    shape: Expr,
}

fn fit(
    eq: &Expr,
    f: &str,
    vars: &[String],
    family: &Family,
    at: &mut dyn FnMut(&Expr) -> Option<Expr>,
) -> Vec<Expr> {
    crate::budget::with_limit(std::time::Duration::from_secs(3), || {
        fit_in_time(eq, f, vars, family, at)
    })
}

fn fit_in_time(
    eq: &Expr,
    f: &str,
    vars: &[String],
    family: &Family,
    at: &mut dyn FnMut(&Expr) -> Option<Expr>,
) -> Vec<Expr> {
    let Some(residual) = replace(eq, f, at) else {
        return Vec::new();
    };
    let top = ratsimp::fraction(&ratsimp::together(&residual)).0;
    if crate::budget::expired() {
        return Vec::new();
    }
    let mut gens = family.gens.clone();
    gens.extend(vars.iter().cloned());
    let Some(conditions) = coefficients_in(&expand(&top), &gens) else {
        return Vec::new();
    };
    let conditions: Vec<Expr> = conditions
        .iter()
        .map(simplify)
        .filter(|c| !c.is_zero())
        .collect();
    if conditions.iter().any(|c| gens.iter().any(|g| c.has_sym(g))) {
        return Vec::new();
    }
    let used: Vec<String> = family
        .unknowns
        .iter()
        .filter(|u| conditions.iter().any(|c| c.has_sym(u)))
        .cloned()
        .collect();
    if conditions.is_empty() {
        return vec![family.shape.clone()];
    }
    if used.is_empty() {
        return Vec::new();
    }
    let Ok(solved) = system::solve_system(&conditions, &used) else {
        return Vec::new();
    };
    solve::release(&solved.params);
    solved
        .solutions
        .iter()
        .map(|s| {
            let filled = s.iter().fold(family.shape.clone(), |e, (name, value)| {
                subst_sym(&e, name, value)
            });
            simplify(&filled)
        })
        .collect()
}

/// The rate `k` of the exponentials `e^(k m v)` written out in the equation, and the equation
/// with them replaced by powers of the generators `#X0`, `#X1`, ... for `e^(k v)`.
fn explicit_rate(eq: &Expr, vars: &[String]) -> Option<(Expr, Expr)> {
    let written = solve::rewrite_exp(eq);
    let mut rates: Vec<Expr> = Vec::new();
    let mut plain = true;
    written.walk(&mut |e| {
        if let Expr::Pow(b, u) = e
            && b.is_sym("e")
            && vars.iter().any(|v| u.has_sym(v))
        {
            match linear_rates(u, vars) {
                Some((c, _)) => rates.extend(c.into_iter().filter(|c| !c.is_zero())),
                None => plain = false,
            }
        }
    });
    if !plain || rates.is_empty() {
        return None;
    }
    let size = |e: &Expr| numeric::evalf(e, 64).map(|v| Float::with_val(64, v.abs_ref()).to_f64());
    let mut k = rates[0].clone();
    for r in &rates {
        if size(r).ok()? < size(&k).ok()? {
            k = r.clone();
        }
    }
    // the rate without a leading minus sign: e^(-k v) is a power of the same generator
    if crate::funcs::is_neg_form(&k) {
        k = neg(k);
    }
    let mut whole = true;
    let replaced = map_bottom_up(&written, &mut |e| {
        if let Expr::Pow(b, u) = &e
            && b.is_sym("e")
            && vars.iter().any(|v| u.has_sym(v))
            && let Some((c, rest)) = linear_rates(u, vars)
        {
            let mut parts = vec![pow(Expr::e(), rest)];
            for (j, c) in c.iter().enumerate() {
                match simplify(&div(c.clone(), k.clone())).as_i64() {
                    Some(m) => parts.push(powi(sym(&format!("#X{j}")), m)),
                    None => whole = false,
                }
            }
            return mul(parts);
        }
        e
    });
    whole.then_some((k, replaced))
}

/// `u = sum c_i v_i + rest` with constants `c_i`.
fn linear_rates(u: &Expr, vars: &[String]) -> Option<(Vec<Expr>, Expr)> {
    let mut rest = expand(u);
    let mut c = Vec::new();
    for v in vars {
        let parts = ratsimp::coefficients(&rest, v)?;
        match parts.len() {
            1 => c.push(Expr::zero()),
            2 => {
                c.push(parts[1].clone());
                rest = parts[0].clone();
            }
            _ => return None,
        }
    }
    (!vars.iter().any(|v| rest.has_sym(v)) && c.iter().all(|k| k.is_const())).then_some((c, rest))
}

/// Candidates from families with coefficients to solve for, and shapes to be checked as they are.
fn families(eq: &Expr, f: &str, vars: &[String]) -> (Vec<Expr>, Vec<Expr>) {
    let x = sym(&vars[0]);
    let q = |k: usize| sym(&format!("#q{k}"));
    let mut out = Vec::new();
    let whole = |p: &Rat| p.is_integer().then(|| p.numer().to_i64()).flatten();
    // polynomials
    for degree in [4usize, 1] {
        let unknowns: Vec<String> = (0..=degree).map(|k| format!("#q{k}")).collect();
        let shape_at = |arg: &Expr| {
            add((0..=degree)
                .map(|k| mul(vec![q(k), powi(arg.clone(), k as i64)]))
                .collect())
        };
        let family = Family {
            unknowns,
            gens: Vec::new(),
            shape: shape_at(&x),
        };
        let found = fit(eq, f, vars, &family, &mut |arg| Some(shape_at(arg)));
        if !found.is_empty() {
            out.extend(found);
            break;
        }
    }
    // (A + G x) e^(c x) + B e^(-c x) + D with X_i for e^(c v_i); c is the rate of the
    // exponentials of the equation when it has some
    let gens: Vec<String> = (0..vars.len()).map(|k| format!("#X{k}")).collect();
    let (c, with_gens) = match explicit_rate(eq, vars) {
        Some((k, replaced)) => (k, replaced),
        None => (sym("#q9"), eq.clone()),
    };
    let exp_shape = |arg: &Expr, e: Expr| {
        add(vec![
            mul(vec![
                add(vec![q(0), mul(vec![q(3), arg.clone()])]),
                e.clone(),
            ]),
            div(q(1), e),
            q(2),
        ])
    };
    let family = Family {
        unknowns: vec!["#q0".into(), "#q1".into(), "#q2".into(), "#q3".into()],
        gens: gens.clone(),
        shape: exp_shape(&x, pow(Expr::e(), mul(vec![c.clone(), x.clone()]))),
    };
    let mut exps: Vec<Expr> = Vec::new();
    for s in fit(&with_gens, f, vars, &family, &mut |arg| {
        let (p, rest) = linear_form(arg, vars)?;
        let mut parts = vec![pow(Expr::e(), mul(vec![c.clone(), rest]))];
        for (g, p) in gens.iter().zip(&p) {
            parts.push(powi(sym(g), whole(p)?));
        }
        Some(exp_shape(arg, mul(parts)))
    }) {
        // c and -c give the same family: the shorter way to write it is kept
        let mirrored = simplify(&subst_sym(&s, "#q9", &neg(sym("#q9"))));
        let s = if mirrored.to_string().len() < s.to_string().len() {
            mirrored
        } else {
            s
        };
        if !exps.iter().any(|old| is_zero(&sub(old.clone(), s.clone()))) {
            exps.push(s);
        }
    }
    out.extend(exps);
    // A e^(c x^2) + D with G_i for e^(c v_i^2)
    let gens: Vec<String> = (0..vars.len()).map(|k| format!("#G{k}")).collect();
    let squares: Vec<String> = (0..vars.len()).map(|k| format!("#S{k}")).collect();
    let c2 = sym("#q9");
    let gauss_shape = |e: Expr| add(vec![mul(vec![q(0), e]), q(2)]);
    let family = Family {
        unknowns: vec!["#q0".into(), "#q2".into()],
        gens: gens.clone(),
        shape: gauss_shape(pow(Expr::e(), mul(vec![c2.clone(), powi(x.clone(), 2)]))),
    };
    out.extend(fit(eq, f, vars, &family, &mut |arg| {
        let mut sq = expand(&powi(arg.clone(), 2));
        for (v, s) in vars.iter().zip(&squares) {
            sq = subst_sym(&sq, v, &crate::canon::sqrt(sym(s)));
        }
        let (p, rest) = linear_form(&simplify(&sq), &squares)?;
        let mut parts = vec![pow(Expr::e(), mul(vec![c2.clone(), rest]))];
        for (g, p) in gens.iter().zip(&p) {
            parts.push(powi(sym(g), whole(p)?));
        }
        Some(gauss_shape(mul(parts)))
    }));
    // (a ln(x) + b) x^k for k = 0, 1 with L_i for ln(v_i)
    let gens: Vec<String> = (0..vars.len()).map(|k| format!("#L{k}")).collect();
    for k in [0i64, 1] {
        let log_shape = |arg: &Expr, l: Expr| {
            mul(vec![
                add(vec![mul(vec![q(0), l]), q(1)]),
                powi(arg.clone(), k),
            ])
        };
        let family = Family {
            unknowns: vec!["#q0".into(), "#q1".into()],
            gens: gens.clone(),
            shape: log_shape(&x, func("ln", vec![x.clone()])),
        };
        out.extend(fit(eq, f, vars, &family, &mut |arg| {
            let (p, rest) = monomial_form(arg, vars)?;
            let mut terms = vec![func("ln", vec![rest])];
            for (g, p) in gens.iter().zip(&p) {
                terms.push(mul(vec![Expr::rational(p.clone()), sym(g)]));
            }
            Some(log_shape(arg, add(terms)))
        }));
    }
    // A x^c + D with P_i for v_i^c
    let gens: Vec<String> = (0..vars.len()).map(|k| format!("#P{k}")).collect();
    let c = sym("#q9");
    let pow_shape = |p: Expr| add(vec![mul(vec![q(0), p]), q(2)]);
    let family = Family {
        unknowns: vec!["#q0".into(), "#q2".into()],
        gens: gens.clone(),
        shape: pow_shape(pow(x.clone(), c.clone())),
    };
    out.extend(fit(eq, f, vars, &family, &mut |arg| {
        let (p, rest) = monomial_form(arg, vars)?;
        let mut parts = vec![pow(rest, c.clone())];
        for (g, p) in gens.iter().zip(&p) {
            parts.push(powi(sym(g), whole(p)?));
        }
        Some(pow_shape(mul(parts)))
    }));
    // shapes with one constant, checked as they are
    let k = q(0);
    let kx = mul(vec![k.clone(), x.clone()]);
    let mut guesses = Vec::new();
    for name in ["tan", "tanh", "cot", "coth", "sin", "sinh", "atan", "atanh"] {
        guesses.push(func(name, vec![kx.clone()]));
    }
    guesses.push(div(k.clone(), x.clone()));
    guesses.push(div(x.clone(), add(vec![Expr::one(), kx.clone()])));
    guesses.push(div(k.clone(), add(vec![Expr::one(), neg(x.clone())])));
    guesses.push(func("abs", vec![x.clone()]));
    guesses.push(mul(vec![k, func("abs", vec![x])]));
    (out, guesses)
}

/// Whether `special` is a member of the family `general` with one constant.
fn member(special: &Expr, general: &Expr, x: &str) -> bool {
    let constants: Vec<Rc<str>> = general
        .free_syms()
        .into_iter()
        .filter(|s| **s != *x && !special.has_sym(s))
        .collect();
    let [c] = &constants[..] else {
        return false;
    };
    for point in [2i64, 3] {
        let at = |e: &Expr| subst_sym(e, x, &Expr::int(point));
        let Ok(solved) = solve::solve(&sub(at(general), at(special)), c) else {
            continue;
        };
        solve::release(&solved.params);
        if solved.all {
            return true;
        }
        for r in &solved.roots {
            if r.approx {
                continue;
            }
            // of a periodic family of values the member with all parameters zero
            let value = solved
                .params
                .iter()
                .fold(r.value.clone(), |v, p| subst_sym(&v, p, &Expr::zero()));
            if is_zero(&sub(subst_sym(general, c, &value), special.clone())) {
                return true;
            }
        }
    }
    false
}

/// `a e^u + a e^-u` and `a e^u - a e^-u` read as hyperbolic functions.
fn hyperbolic_form(e: &Expr, x: &str) -> Expr {
    let Expr::Add(terms) = e else {
        return e.clone();
    };
    let [t1, t2] = &terms[..] else {
        return e.clone();
    };
    let split = |t: &Expr| -> Option<(Expr, Expr)> {
        let mut exponent = None;
        t.walk(&mut |n| {
            if let Expr::Pow(b, u) = n
                && b.is_sym("e")
                && u.has_sym(x)
            {
                exponent.get_or_insert_with(|| (**u).clone());
            }
        });
        let u = exponent?;
        let coefficient = simplify(&div(t.clone(), pow(Expr::e(), u.clone())));
        (!coefficient.has_sym(x)).then_some((coefficient, u))
    };
    let (Some((a, u)), Some((b, v))) = (split(t1), split(t2)) else {
        return e.clone();
    };
    if !is_zero(&add(vec![u.clone(), v])) {
        return e.clone();
    }
    // the argument without a leading minus sign
    let (u, a, b) = if crate::funcs::is_neg_form(&u) {
        (neg(u), b, a)
    } else {
        (u, a, b)
    };
    let double = |c: &Expr| mul(vec![Expr::int(2), c.clone()]);
    if is_zero(&sub(a.clone(), b.clone())) {
        mul(vec![double(&a), func("cosh", vec![u])])
    } else if is_zero(&add(vec![a.clone(), b])) {
        mul(vec![double(&a), func("sinh", vec![u])])
    } else {
        e.clone()
    }
}

fn is_constant_name(s: &str) -> bool {
    s.starts_with("#q")
        || (s.len() > 1 && s.starts_with('C') && s[1..].chars().all(|c| c.is_ascii_digit()))
}

/// The constants of a solution in the order they are met.
fn constants_of(e: &Expr) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    e.walk(&mut |n| {
        if let Expr::Sym(s) = n
            && is_constant_name(s)
            && !out.iter().any(|o| **o == **s)
        {
            out.push(s.to_string());
        }
    });
    out
}

/// The solution with its constants renamed `prefix1`, `prefix2`, ... from `first` on.
fn renamed(e: &Expr, prefix: &str, first: usize) -> Expr {
    let names = constants_of(e);
    let mut out = e.clone();
    for (k, n) in names.iter().enumerate() {
        out = subst_sym(&out, n, &sym(&format!("#r{k}")));
    }
    for k in 0..names.len() {
        out = subst_sym(
            &out,
            &format!("#r{k}"),
            &sym(&format!("{prefix}{}", first + k)),
        );
    }
    out
}

/// A shorter way to write a solution: trigonometric functions for `e^(i u)`, numbers that
/// multiply a constant taken into it, `C1 cos(u) + C2 sin(u)` for two combinations of constants.
fn tidy(e: &Expr, x: &str) -> Expr {
    let mut best = hyperbolic_form(e, x);
    let circular = simplify(&expand(&crate::integrate::exp_to_trig(&best)));
    if circular.to_string().len() < best.to_string().len() {
        best = circular.clone();
    }
    // a cos(u) + b sin(u) with independent combinations a, b of two constants
    let mut angle = None;
    circular.walk(&mut |n| {
        if let Some(u) = n.fn_arg("cos").or(n.fn_arg("sin"))
            && u.has_sym(x)
        {
            angle.get_or_insert_with(|| u.clone());
        }
    });
    let names = constants_of(&circular);
    if let (Some(u), [c1, c2]) = (angle, &names[..]) {
        let (cos, sin) = (func("cos", vec![u.clone()]), func("sin", vec![u]));
        let lifted = crate::canon::subst(
            &crate::canon::subst(&circular, &cos, &sym("#c")),
            &sin,
            &sym("#s"),
        );
        let a = crate::diff::diff(&lifted, "#c");
        let b = crate::diff::diff(&lifted, "#s");
        let rest = subst_sym(
            &subst_sym(&lifted, "#c", &Expr::zero()),
            "#s",
            &Expr::zero(),
        );
        let d = |e: &Expr, c: &str| crate::diff::diff(e, c);
        let det = sub(
            mul(vec![d(&a, c1), d(&b, c2)]),
            mul(vec![d(&a, c2), d(&b, c1)]),
        );
        let plain = [&a, &b]
            .iter()
            .all(|e| !e.has_sym(x) && !e.has_sym("#c") && !e.has_sym("#s"));
        if plain
            && !rest.has_sym("#c")
            && !rest.has_sym("#s")
            && !is_zero(&det)
            && constants_of(&rest).is_empty()
        {
            best = add(vec![mul(vec![sym(c1), cos]), mul(vec![sym(c2), sin]), rest]);
        }
    }
    // a number times a constant is a constant
    for c in constants_of(&best) {
        let slope = crate::diff::diff(&best, &c);
        if slope.has_sym(&c) {
            continue;
        }
        let (factor, _) = crate::canon::coeff_and_rest(&slope);
        if !factor.is_one() && !factor.is_zero() {
            best = simplify(&subst_sym(&best, &c, &div(sym(&c), Expr::num(factor))));
        }
    }
    best
}

/// Solves the equation `eq = 0` for the function `f`.
pub fn funsolve(eq: &Expr, f: &str) -> Result<Found, String> {
    let args = calls(eq, f);
    if args.is_empty() {
        return Err(format!("the equation has no call of {f}"));
    }
    let mut vars: Vec<String> = Vec::new();
    for a in &args {
        for s in a.free_syms() {
            if !vars.contains(&s.to_string()) {
                vars.push(s.to_string());
            }
        }
    }
    vars.sort();
    if vars.is_empty() {
        return Err(format!("{f} is called with constants only"));
    }
    let x = vars[0].clone();
    let mut notes = Vec::new();
    let mut constants = Constants {
        taken: eq.free_syms().iter().map(|s| s.to_string()).collect(),
        next: 0,
    };
    let mut found: Vec<Expr> = Vec::new();
    let keep = |candidate: Expr, found: &mut Vec<Expr>, constants: &mut Constants| {
        if candidate.is_undefined() || candidate.is_infinite() || !holds(eq, f, &x, &candidate) {
            return false;
        }
        let named = tidy(&simplify(&name_constants(&candidate, constants)), &x);
        let plain = renamed(&named, "#k", 1);
        if !found
            .iter()
            .any(|old| is_zero(&sub(renamed(old, "#k", 1), plain.clone())))
        {
            found.push(named);
        }
        true
    };
    let mut open = false;
    if vars.len() == 1 {
        if let Some(c) = orbit(eq, f, &x, &mut open) {
            keep(c, &mut found, &mut constants);
        }
        if found.is_empty()
            && let Some(c) = recurrence(eq, f, &x, &mut notes)
            && !keep(c, &mut found, &mut constants)
        {
            notes.clear();
        }
    }
    if found.is_empty() {
        let (solved, guesses) = families(eq, f, &vars);
        for c in solved {
            keep(c, &mut found, &mut constants);
        }
        // a family of exponentials holds the circular and hyperbolic functions already
        let wide = found.iter().any(|s| {
            s.free_syms().len() > 2 && (s.has_fn("sinh") || s.has_fn("cosh") || s.has_sym("e"))
        });
        for c in guesses {
            let name = match &c {
                Expr::Fn(n, _) => n.to_string(),
                _ => String::new(),
            };
            if wide && matches!(name.as_str(), "sin" | "sinh") {
                continue;
            }
            keep(c, &mut found, &mut constants);
        }
    }
    if found.is_empty() && vars.len() == 2 {
        // one variable set to a value leaves an equation in the other
        let (x1, y1) = (sym(&vars[0]), sym(&vars[1]));
        for value in [x1.clone(), Expr::zero(), Expr::one(), neg(x1)] {
            let reduced = simplify(&subst_sym(eq, &vars[1], &value));
            let reduced = replace(&reduced, f, &mut |arg| {
                Some(Expr::call(f, vec![simplify(arg)]))
            })
            .unwrap_or(reduced);
            if reduced.has_sym(&vars[1]) || !reduced.has_fn(f) {
                continue;
            }
            let mut quiet = Vec::new();
            let guess = orbit(&reduced, f, &x, &mut false)
                .or_else(|| recurrence(&reduced, f, &x, &mut quiet));
            if let Some(c) = guess {
                keep(c, &mut found, &mut constants);
            }
            if !found.is_empty() {
                break;
            }
        }
        let _ = y1;
    }
    // members of a wider family are not listed again
    let mut kept: Vec<Expr> = Vec::new();
    for (k, s) in found.iter().enumerate() {
        let covered = found.iter().enumerate().any(|(j, wider)| {
            j != k && wider.free_syms().len() > s.free_syms().len() && member(s, wider, &x)
        });
        if !covered {
            kept.push(s.clone());
        }
    }
    if kept.is_empty() {
        return Err(format!(
            "no solution found for {f}: tried a finite orbit of the arguments, a recurrence, and polynomials, exponentials, logarithms and powers"
        ));
    }
    if open {
        kept.truncate(3);
        notes.push(format!(
            "the equation does not fix {f}: these are some of its solutions"
        ));
    }
    // the constants are numbered through
    let mut first = 1;
    let solutions = kept
        .iter()
        .map(|s| {
            let out = renamed(s, "C", first);
            first += constants_of(s).len();
            out
        })
        .collect();
    Ok(Found {
        var: x,
        solutions,
        notes,
    })
}
