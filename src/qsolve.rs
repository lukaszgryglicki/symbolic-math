//! Equations whose unknowns are quaternions.
//!
//! An unknown `x` is `x0 + x1 i + x2 j + x3 k` with four real components. The methods, in this
//! order: the components as a linear system; `G(x) = c` with a `G` that keeps every complex plane
//! (solved in the complex numbers and carried to the plane of `c`); a polynomial with its
//! coefficients on one side (by its companion polynomial); the components as a system.

use crate::canon::{add, div, mul, neg, pow, sqrt, sub, subst_all, subst_sym};
use crate::expr::Expr;
use crate::funcs::func;
use crate::quat::{self, Parts};
use crate::{assume, complex, fit, numeric, ratsimp, simplify, solve, system};
use rug::ops::Pow;
use rug::{Complex, Float};

pub struct Unknown {
    pub name: String,
    /// The names of the four components; `None` for an unknown that is a real number.
    pub parts: Option<[String; 4]>,
}

pub struct Solution {
    /// One value for each unknown.
    pub values: Vec<Expr>,
    pub approx: bool,
    pub mult: u32,
    /// What the components that are left in the values must satisfy.
    pub conditions: Vec<Expr>,
    /// The whole numbers that a family leaves out.
    pub skip: Vec<i64>,
}

impl Solution {
    fn new(values: Vec<Expr>, approx: bool, mult: u32) -> Solution {
        Solution {
            values,
            approx,
            mult,
            conditions: Vec::new(),
            skip: Vec::new(),
        }
    }
}

#[derive(Default)]
pub struct Solved {
    pub solutions: Vec<Solution>,
    /// Every value of the unknowns is a solution.
    pub all: bool,
    /// Whole-number parameters of families.
    pub params: Vec<String>,
    /// Real numbers that stay free.
    pub free: Vec<String>,
    /// The solutions name any unit pure quaternion.
    pub sphere: bool,
    pub notes: Vec<String>,
}

const SEARCHED: &str = "found by a search over the real components; other solutions may exist";

impl Unknown {
    fn names(&self) -> Vec<String> {
        match &self.parts {
            Some(parts) => parts.to_vec(),
            None => vec![self.name.clone()],
        }
    }

    /// The value of the unknown where the components have the given values.
    fn value(&self, assignment: &system::Assignment) -> Expr {
        let part = |name: &String| {
            assignment
                .iter()
                .find(|(n, _)| n == name)
                .map_or_else(|| Expr::sym(name), |(_, v)| v.clone())
        };
        match &self.parts {
            Some(parts) => quat::tidy(parts.each_ref().map(part)),
            None => part(&self.name),
        }
    }
}

/// A component as a plain sum: without the forms that simplification keeps from multiplying out.
fn plain(e: &Expr) -> Expr {
    let open = crate::canon::map_bottom_up(e, &mut |x| match x.fn_arg("hold") {
        Some(inner) => crate::canon::map_bottom_up(inner, &mut |y| y),
        None => x,
    });
    crate::expand::expand(&open)
}

fn all_names(unknowns: &[Unknown]) -> Vec<String> {
    unknowns.iter().flat_map(Unknown::names).collect()
}

fn solution(unknowns: &[Unknown], assignment: &system::Assignment, approx: bool) -> Solution {
    let values = unknowns.iter().map(|u| u.value(assignment)).collect();
    Solution::new(values, approx, 1)
}

/// A numeric value as an expression; an exact zero stays one.
fn float_expr(z: Complex) -> Expr {
    if z.is_zero() {
        Expr::zero()
    } else {
        numeric::to_float_expr(z, numeric::digits())
    }
}

/// Real and imaginary part of a root of the complex solver; the flag tells a numeric value.
fn split(value: &Expr, approx: bool) -> (Expr, Expr, bool) {
    let digits = numeric::digits();
    let floats = |v: &Complex| {
        let part = |x: &Float| float_expr(Complex::with_val(x.prec(), (x, 0)));
        (part(v.real()), part(v.imag()), true)
    };
    if let Expr::Float(f) = value {
        return floats(&f.v);
    }
    if let Some((re, im)) = complex::rect(value) {
        let (re, im) = (simplify::simplify(&re), simplify::simplify(&im));
        if !re.has_fn("arg") && !im.has_fn("arg") {
            return (re, im, approx);
        }
    }
    if value.is_const()
        && let Ok(v) = numeric::evalf_digits(value, digits)
    {
        return floats(&v);
    }
    (
        func("re", vec![value.clone()]),
        func("im", vec![value.clone()]),
        approx,
    )
}

fn vanishes(e: &Expr) -> bool {
    match e {
        Expr::Float(f) => f.v.is_zero(),
        _ if e.is_const() => numeric::is_zero_const(e).unwrap_or(false),
        _ => simplify::is_zero(e),
    }
}

/// Whether the parts are all zero; numeric parts count as zero below half of the digits.
fn negligible(parts: &Parts, approx: bool) -> bool {
    if !approx {
        return parts.iter().all(vanishes);
    }
    let limit = Float::with_val(64, 10).pow(-i32::try_from(numeric::digits() / 2).unwrap_or(16));
    parts.iter().all(|p| {
        numeric::evalf_digits(p, 20).is_ok_and(|v| Float::with_val(64, v.abs_ref()) < limit)
    })
}

fn same_number(a: &Expr, b: &Expr) -> bool {
    if a == b {
        return true;
    }
    if !a.is_const() || !b.is_const() {
        return false;
    }
    let digits = numeric::digits();
    match (
        numeric::evalf_digits(a, digits),
        numeric::evalf_digits(b, digits),
    ) {
        (Ok(x), Ok(y)) => {
            let gap = Float::with_val(64, Complex::with_val(x.prec(), &x - &y).abs_ref());
            let scale = Float::with_val(64, x.abs_ref()) + 1u32;
            gap < scale >> (numeric::bits_for(digits) - 16)
        }
        _ => false,
    }
}

fn same_parts(a: &Parts, b: &Parts) -> bool {
    a.iter().zip(b).all(|(x, y)| same_number(x, y))
}

/// Whether the quaternion with these parts solves the equations of the components.
fn fits(comps: &[Expr], names: &[String; 4], parts: &Parts, approx: bool) -> bool {
    let pairs: Vec<(String, Expr)> = names.iter().cloned().zip(parts.iter().cloned()).collect();
    satisfied(comps, &pairs, approx)
}

/// Whether the equations hold with these values of the components.
fn satisfied(comps: &[Expr], pairs: &[(String, Expr)], approx: bool) -> bool {
    let approx = approx || pairs.iter().any(|(_, v)| v.has_float());
    let bits = numeric::bits_for(numeric::digits());
    comps.iter().all(|eq| {
        let e = subst_all(eq, pairs);
        if e.is_zero() {
            return true;
        }
        if e.any(&|x| x.is_infinite()) {
            return false;
        }
        if !approx {
            return match e.is_const() {
                true => numeric::is_zero_const(&e).unwrap_or(false),
                false => numeric::is_zero_identity(&e) == Some(true),
            };
        }
        // what is left of a numeric value is rounding noise
        let symbols: Vec<std::rc::Rc<str>> = e.free_syms().into_iter().collect();
        (0..3).all(|trial| {
            let env = numeric::sample_env(&symbols, trial, bits + 64);
            numeric::relative_size(&e, bits + 64, &env).is_some_and(|s| s < 28 - i64::from(bits))
        })
    })
}

/// Three unit vectors with rational parts: the points where a sphere of solutions is checked.
fn directions() -> [[Expr; 3]; 3] {
    let r = Expr::rat;
    [
        [r(1, 1), r(0, 1), r(0, 1)],
        [r(2, 3), r(-1, 3), r(2, 3)],
        [r(-2, 7), r(3, 7), r(6, 7)],
    ]
}

fn on_sphere(comps: &[Expr], names: &[String; 4], re: &Expr, radius: &Expr, approx: bool) -> bool {
    directions().iter().all(|d| {
        let part = |k: usize| simplify::simplify(&mul(vec![radius.clone(), d[k].clone()]));
        fits(
            comps,
            names,
            &[re.clone(), part(0), part(1), part(2)],
            approx,
        )
    })
}

/// `re + radius*u` for the unit pure quaternion named `sphere`.
fn sphere_value(re: &Expr, radius: &Expr, sphere: &str) -> Expr {
    add(vec![
        re.clone(),
        mul(vec![radius.clone(), Expr::sym(sphere)]),
    ])
}

/// The radius of a sphere with a sign that does not matter made positive.
fn radius_of(im: Expr) -> Expr {
    let negative = match numeric::sign(&im) {
        Some(s) => s < 0,
        None => crate::funcs::is_neg_form(&im),
    };
    if negative {
        simplify::simplify(&neg(im))
    } else {
        im
    }
}

/// The spheres found so far, each by its centre and radius. Two families of them are the same
/// when one part is the same number and the other one holds the same numbers (a radius up to
/// its sign), or when they differ in the name of their whole number only.
struct Spheres {
    seen: Vec<(Expr, Expr)>,
    params: Vec<String>,
}

impl Spheres {
    fn is_new(&mut self, re: &Expr, radius: &Expr) -> bool {
        let plain = |e: &Expr| {
            let used: Vec<&String> = self.params.iter().filter(|p| e.has_sym(p)).collect();
            match used.as_slice() {
                [one] => simplify::simplify(&subst_sym(e, one, &Expr::sym("#n"))),
                _ => e.clone(),
            }
        };
        let family = |a: &Expr, b: &Expr| solve::same_family(a, b, &self.params);
        let same = |(a, b): &(Expr, Expr)| {
            let centres = same_number(a, re) || same_number(&plain(a), &plain(re));
            let radii = same_number(b, radius) || same_number(&plain(b), &plain(radius));
            (centres && radii)
                || (same_number(a, re) && (family(b, radius) || family(b, &neg(radius.clone()))))
                || (same_number(b, radius) && family(a, re))
        };
        if self.seen.iter().any(same) {
            return false;
        }
        self.seen.push((re.clone(), radius.clone()));
        true
    }
}

fn linear(rows: crate::matrix::Rows, names: &[String], unknowns: &[Unknown]) -> Solved {
    let mut found = system::SystemSolved::default();
    system::solve_linear(rows, names, &mut found);
    let Some(assignment) = found.solutions.pop() else {
        return Solved::default();
    };
    let free: Vec<String> = names
        .iter()
        .filter(|n| !assignment.iter().any(|(a, _)| a == *n))
        .cloned()
        .collect();
    Solved {
        solutions: vec![solution(unknowns, &assignment, false)],
        free,
        ..Solved::default()
    }
}

/// One way for polynomial equations between real numbers to hold: the values of some of them
/// and what is left for the others.
#[derive(Clone)]
struct Case {
    fixed: system::Assignment,
    left: Vec<Expr>,
    approx: bool,
}

/// No real numbers satisfy `e = 0`: every term is a product of even powers, the coefficients
/// have one sign and one term is a number.
fn definite(e: &Expr) -> bool {
    let e = crate::expand::expand(e);
    let terms: Vec<Expr> = match &e {
        Expr::Add(a) => a.to_vec(),
        other => vec![other.clone()],
    };
    let (mut constant, mut positive, mut negative) = (false, false, false);
    for t in &terms {
        let (c, rest) = crate::canon::coeff_and_rest(t);
        let factors: Vec<Expr> = match &rest {
            Expr::Mul(a) => a.to_vec(),
            other => vec![other.clone()],
        };
        let even = |f: &Expr| {
            let (base, k) = f.base_exp();
            f.is_one()
                || (matches!(base, Expr::Sym(_))
                    && k.as_int().is_some_and(|k| k.is_even() && k.is_positive()))
        };
        if !c.is_real() || c.is_zero() || !factors.iter().all(even) {
            return false;
        }
        constant |= rest.is_one();
        positive |= c.is_positive();
        negative |= c.is_negative();
    }
    constant && positive != negative
}

/// A real root of an equation of one unknown.
fn real_root(root: &solve::Root) -> Option<Expr> {
    if !root.value.is_const() {
        return None;
    }
    let value = numeric::evalf_digits(&root.value, numeric::digits()).ok()?;
    is_real_value(&value).then(|| match &root.value {
        Expr::Float(_) => float_expr(Complex::with_val(value.prec(), value.real())),
        exact => exact.clone(),
    })
}

/// Splits polynomial equations between real numbers into cases: the real roots of an equation
/// of one unknown are put into the others, an equation that gives one unknown by the others is
/// used up, a product gives a case for each factor, and a case without real numbers is dropped.
fn reduce(
    eqs: Vec<Expr>,
    fixed: system::Assignment,
    approx: bool,
    names: &[String],
    out: &mut Vec<Case>,
    budget: &mut u32,
    ground: bool,
) {
    let eqs: Vec<Expr> = eqs
        .iter()
        .map(crate::expand::expand)
        .filter(|e| !vanishes(e))
        .collect();
    let unknowns_of =
        |e: &Expr| -> Vec<String> { names.iter().filter(|n| e.has_sym(n)).cloned().collect() };
    if eqs.iter().any(|e| unknowns_of(e).is_empty() || definite(e)) {
        return;
    }
    if eqs.is_empty() || *budget == 0 {
        out.push(Case {
            fixed,
            left: eqs,
            approx,
        });
        return;
    }
    *budget -= 1;
    // the value of one unknown goes into the other equations and into the values so far
    let with = |fixed: &system::Assignment, at: usize, var: &String, value: &Expr| {
        let rest: Vec<Expr> = eqs
            .iter()
            .enumerate()
            .filter(|(k, _)| *k != at)
            .map(|(_, e)| subst_sym(e, var, value))
            .collect();
        let mut fixed: system::Assignment = fixed
            .iter()
            .map(|(n, v)| (n.clone(), subst_sym(v, var, value)))
            .collect();
        fixed.push((var.clone(), value.clone()));
        (rest, fixed)
    };
    let single = eqs
        .iter()
        .enumerate()
        .find_map(|(k, e)| match unknowns_of(e).as_slice() {
            [one] => Some((k, one.clone())),
            _ => None,
        });
    if let Some((at, var)) = single
        && let Ok(solved) = solve::solve(&eqs[at], &var)
    {
        solve::release(&solved.params);
        if solved.params.is_empty() && !solved.all {
            for root in &solved.roots {
                if let Some(value) = real_root(root) {
                    let (rest, fixed) = with(&fixed, at, &var, &value);
                    reduce(
                        rest,
                        fixed,
                        approx || root.approx,
                        names,
                        out,
                        budget,
                        false,
                    );
                }
            }
            return;
        }
    }
    let linear = eqs.iter().enumerate().find_map(|(k, e)| {
        unknowns_of(e).into_iter().find_map(|var| {
            match ratsimp::coefficients(e, &var)?.as_slice() {
                [rest, slope] if unknowns_of(slope).is_empty() => {
                    let value = simplify::simplify(&neg(div(rest.clone(), slope.clone())));
                    Some((k, var, value))
                }
                _ => None,
            }
        })
    });
    if let Some((at, var, value)) = linear {
        let (rest, fixed) = with(&fixed, at, &var, &value);
        reduce(rest, fixed, approx, names, out, budget, false);
        return;
    }
    let product = eqs.iter().enumerate().find_map(|(k, e)| {
        let parts = system::factors(e, names);
        (parts.len() > 1).then_some((k, parts))
    });
    if let Some((at, parts)) = product {
        for part in parts {
            let mut next = eqs.clone();
            next[at] = part;
            reduce(next, fixed.clone(), approx, names, out, budget, false);
        }
        return;
    }
    // nothing more to split: a Groebner basis tells what the equations come to
    let refs: Vec<&Expr> = eqs.iter().collect();
    if !ground
        && let Some((gens, polys)) = ratsimp::common(&refs)
        && let Some(basis) = crate::poly::groebner(&polys, 2000)
    {
        if basis.len() > 1 || !basis[0].is_const() {
            let next = basis.iter().map(|p| gens.expr(p)).collect();
            reduce(next, fixed, approx, names, out, budget, true);
        }
        return;
    }
    // the values as they are where the conditions hold
    let fixed = fixed
        .into_iter()
        .map(|(name, value)| {
            let short = remainder(&value, &eqs).filter(|r| r.size() < value.size());
            (name, short.unwrap_or(value))
        })
        .collect();
    out.push(Case {
        fixed,
        left: eqs,
        approx,
    });
}

/// The remainder of the polynomial `e` by the polynomials `basis`.
fn remainder(e: &Expr, basis: &[Expr]) -> Option<Expr> {
    if basis.is_empty() || e.is_number() {
        return None;
    }
    let mut all: Vec<&Expr> = vec![e];
    all.extend(basis.iter());
    let (gens, polys) = ratsimp::common(&all)?;
    Some(gens.expr(&crate::poly::reduce(&polys[0], &polys[1..])))
}

/// Whether `e` vanishes where the polynomials `basis` do: its remainder by them is zero.
fn vanishes_with(e: &Expr, basis: &[Expr]) -> bool {
    if vanishes(e) {
        return true;
    }
    if basis.is_empty() {
        return false;
    }
    let mut all: Vec<&Expr> = vec![e];
    all.extend(basis.iter());
    match ratsimp::common(&all) {
        Some((_, polys)) => crate::poly::reduce(&polys[0], &polys[1..]).is_zero(),
        None => false,
    }
}

/// Whether every point of the case `p` is one of the case `q`.
fn contained(p: &Case, q: &Case) -> bool {
    let at_p = |e: &Expr| crate::expand::expand(&subst_all(e, &p.fixed));
    let fixed = q.fixed.iter().all(|(name, value)| {
        let mine = p
            .fixed
            .iter()
            .find(|(n, _)| n == name)
            .map_or_else(|| Expr::sym(name), |(_, v)| v.clone());
        vanishes_with(&at_p(&sub(mine, value.clone())), &p.left)
    });
    fixed && q.left.iter().all(|c| vanishes_with(&at_p(c), &p.left))
}

/// The real solutions of equations that are polynomials in the unknowns, or quotients of them,
/// as cases; `None` for other equations.
fn real_cases(comps: &[Expr], names: &[String]) -> Option<Vec<Case>> {
    let (tops, bottoms): (Vec<Expr>, Vec<Expr>) = comps.iter().map(ratsimp::fraction).unzip();
    let refs: Vec<&Expr> = tops.iter().chain(&bottoms).collect();
    let (gens, _) = ratsimp::common(&refs)?;
    let is_unknown = |k: &Expr| k.sym_name().is_some_and(|s| names.iter().any(|n| n == s));
    if !gens.kernels().iter().all(is_unknown) {
        return None;
    }
    let mut cases = Vec::new();
    let mut budget = 300;
    // (the bases of these equations may take longer than any patience: after 20 seconds
    // the cases that were found until then are what is told)
    crate::budget::with_limit(std::time::Duration::from_secs(20), || {
        reduce(
            tops,
            Vec::new(),
            false,
            names,
            &mut cases,
            &mut budget,
            false,
        )
    });
    // a denominator must not vanish
    cases.retain(|case| {
        bottoms.iter().all(|d| {
            let there = crate::expand::expand(&subst_all(d, &case.fixed));
            !vanishes_with(&there, &case.left)
        })
    });
    let mut keep = vec![true; cases.len()];
    for a in 0..cases.len() {
        for b in 0..cases.len() {
            if a != b
                && keep[b]
                && contained(&cases[a], &cases[b])
                && (b < a || !contained(&cases[b], &cases[a]))
            {
                keep[a] = false;
                break;
            }
        }
    }
    let mut kept = keep.iter();
    cases.retain(|_| *kept.next().expect("one flag for each"));
    Some(cases)
}

/// A numeric point as exact numbers when every part is a fraction with a small denominator
/// and the equations hold exactly there.
fn snapped(comps: &[Expr], names: &[String], point: &[Complex]) -> Option<system::Assignment> {
    let bits = numeric::bits_for(numeric::digits());
    let mut exact = Vec::new();
    for (name, z) in names.iter().zip(point) {
        let x = z.real().to_rational()?;
        let near = crate::ntheory::best_rational(&x, &rug::Integer::from(1000));
        let gap = Float::with_val(64, &rug::Rational::from(&x - &near)).abs();
        if gap > Float::with_val(64, 1) >> (bits - 16) {
            return None;
        }
        exact.push((name.clone(), Expr::rational(near)));
    }
    satisfied(comps, &exact, false).then_some(exact)
}

/// `G(x) = c` where `G` maps every complex plane of the quaternions into itself as it maps the
/// complex numbers: the solutions are the complex ones, turned into the plane of `c`, or whole
/// spheres when `c` is real.
fn planes(
    g: &Expr,
    c: &Expr,
    comps: &[Expr; 4],
    unknown: &Unknown,
    sphere: &str,
) -> Option<Solved> {
    let names = unknown.parts.as_ref()?;
    let var = std::slice::from_ref(&unknown.name);
    // (a function that is not analytic has curves of complex solutions)
    let analytic = |e: &Expr| !matches!(e, Expr::Fn(n, _) if !quat::ANALYTIC.contains(&&**n));
    if !g.has_sym(&unknown.name) || c.has_sym(&unknown.name) || g.any(&|e| !analytic(e)) {
        return None;
    }
    let c = quat::lift(c);
    let x: Vec<Expr> = names.iter().map(|n| Expr::sym(n)).collect();
    let square = |e: &Expr| pow(e.clone(), Expr::int(2));
    let m = sqrt(add(x[1..].iter().map(square).collect()));
    let z = add(vec![x[0].clone(), mul(vec![Expr::i(), m.clone()])]);
    let at_z = assume::with_complex(var, || subst_sym(g, &unknown.name, &z));
    let (u, v) = (func("re", vec![at_z.clone()]), func("im", vec![at_z]));
    for (k, have) in comps.iter().enumerate() {
        let want = if k == 0 {
            sub(u.clone(), c[0].clone())
        } else {
            sub(
                div(mul(vec![v.clone(), x[k].clone()]), m.clone()),
                c[k].clone(),
            )
        };
        if numeric::is_zero_identity(&sub(have.clone(), want)) != Some(true) {
            return None;
        }
    }
    let length = simplify::simplify(&sqrt(add(c[1..].iter().map(square).collect())));
    let real_c = vanishes(&length);
    let target = sub(
        g.clone(),
        add(vec![c[0].clone(), mul(vec![Expr::i(), length.clone()])]),
    );
    let solved = assume::with_complex(var, || solve::solve(&target, &unknown.name)).ok()?;
    if solved.all {
        solve::release(&solved.params);
        return None;
    }
    let equations: Vec<Expr> = comps.iter().filter(|e| !e.is_zero()).cloned().collect();
    let mut out = Solved {
        notes: solved.notes.clone(),
        ..Solved::default()
    };
    let mut spheres = Spheres {
        seen: Vec::new(),
        params: solved.params.clone(),
    };
    for root in &solved.roots {
        let (re, im, approx) = split(&root.value, root.approx);
        if !real_c {
            let part = |k: usize| {
                simplify::simplify(&div(mul(vec![im.clone(), c[k].clone()]), length.clone()))
            };
            let (parts, approx) = readable([re, part(1), part(2), part(3)], approx);
            if fits(&equations, names, &parts, approx) {
                out.solutions.push(Solution {
                    skip: root.skip.clone(),
                    ..Solution::new(vec![quat::tidy(parts)], approx, 1)
                });
            }
        } else if vanishes(&im) {
            // a real root: the complex solver has checked it, and the parts of a function of a
            // quaternion are 0/0 on the real line
            out.solutions.push(Solution {
                skip: root.skip.clone(),
                ..Solution::new(vec![re], approx, root.mult)
            });
        } else {
            let radius = radius_of(im);
            if spheres.is_new(&re, &radius) && on_sphere(&equations, names, &re, &radius, approx) {
                let zero = Expr::zero;
                let ([re, radius, ..], approx) = readable([re, radius, zero(), zero()], approx);
                out.sphere = true;
                out.solutions.push(Solution {
                    skip: root.skip.clone(),
                    ..Solution::new(vec![sphere_value(&re, &radius, sphere)], approx, 1)
                });
            }
        }
    }
    out.params = solved
        .params
        .iter()
        .filter(|p| out.solutions.iter().any(|s| s.values[0].has_sym(p)))
        .cloned()
        .collect();
    for p in solved.params.iter().filter(|p| !out.params.contains(p)) {
        assume::clear(p);
    }
    Some(out)
}

/// The coefficients `a_0 .. a_n` when the equation is `sum a_k x^k = 0` (or `sum x^k a_k = 0`:
/// the flag) with constant quaternions `a_k` and `n >= 2`.
fn one_sided(comps: &[Expr; 4], names: &[String; 4]) -> Option<(Vec<Parts>, bool)> {
    let mut on_line: Vec<Vec<Expr>> = Vec::new();
    for c in comps {
        let mut e = c.clone();
        for n in &names[1..] {
            e = subst_sym(&e, n, &Expr::zero());
        }
        on_line.push(ratsimp::coefficients(&e, &names[0])?);
    }
    let degree = on_line.iter().map(Vec::len).max()?.checked_sub(1)?;
    if !(2..=24).contains(&degree) {
        return None;
    }
    let coeffs: Vec<Parts> = (0..=degree)
        .map(|k| std::array::from_fn(|s| on_line[s].get(k).cloned().unwrap_or_else(Expr::zero)))
        .collect();
    if !coeffs.iter().flatten().all(Expr::is_const) {
        return None;
    }
    let x = Expr::call("quat", names.iter().map(|n| Expr::sym(n)).collect());
    let built = |right: bool| -> Parts {
        let terms = coeffs
            .iter()
            .enumerate()
            .map(|(k, a)| {
                let (a, p) = (quat::make(a.clone()), pow(x.clone(), Expr::int(k as i64)));
                mul(if right { vec![p, a] } else { vec![a, p] })
            })
            .collect();
        quat::lift(&add(terms))
    };
    let agrees = |candidate: &Parts| {
        comps
            .iter()
            .zip(candidate)
            .all(|(have, want)| simplify::is_zero(&sub(have.clone(), want.clone())))
    };
    if agrees(&built(false)) {
        Some((coeffs, false))
    } else if agrees(&built(true)) {
        Some((coeffs, true))
    } else {
        None
    }
}

fn conjugate(p: &Parts) -> Parts {
    [
        p[0].clone(),
        neg(p[1].clone()),
        neg(p[2].clone()),
        neg(p[3].clone()),
    ]
}

/// Numeric parts where the exact ones are too long to read.
fn readable(parts: Parts, approx: bool) -> (Parts, bool) {
    let length: usize = parts.iter().map(|p| p.to_string().len()).sum();
    if approx || length <= 240 {
        return (parts, approx);
    }
    let digits = numeric::digits();
    (parts.map(|p| numeric::n(&p, digits)), true)
}

/// The zeros of a polynomial with its coefficients on one side. The companion polynomial
/// `sum conj(a_j) a_k t^(j+k)` is real; each of its real zeros is a zero, and each pair
/// `p +- q i` of the others stands for the quaternions with real part `p` and a pure part of
/// length `q`, where the polynomial is `A x + B`: one zero `-A^-1 B`, or all of them when
/// `A = B = 0`.
fn companion(
    coeffs: Vec<Parts>,
    right: bool,
    comps: &[Expr; 4],
    unknown: &Unknown,
    sphere: &str,
) -> Option<Solved> {
    let names = unknown.parts.as_ref()?;
    let var = std::slice::from_ref(&unknown.name);
    let coeffs: Vec<Parts> = if right {
        coeffs.iter().map(conjugate).collect()
    } else {
        coeffs
    };
    let n = coeffs.len() - 1;
    let t = Expr::sym(&unknown.name);
    let mut terms = Vec::new();
    for m in 0..=2 * n {
        let mut inner = Vec::new();
        for j in m.saturating_sub(n)..=m.min(n) {
            for (a, b) in coeffs[j].iter().zip(&coeffs[m - j]) {
                inner.push(mul(vec![a.clone(), b.clone()]));
            }
        }
        terms.push(mul(vec![
            simplify::simplify(&add(inner)),
            pow(t.clone(), Expr::int(m as i64)),
        ]));
    }
    let solved = assume::with_complex(var, || solve::solve(&add(terms), &unknown.name)).ok()?;
    solve::release(&solved.params);
    if solved.all || !solved.params.is_empty() {
        return None;
    }
    let equations: Vec<Expr> = comps.iter().filter(|e| !e.is_zero()).cloned().collect();
    let mut out = Solved::default();
    let mut classes = Spheres {
        seen: Vec::new(),
        params: Vec::new(),
    };
    let mut points: Vec<Parts> = Vec::new();
    for root in &solved.roots {
        // (a long exact root gives a zero that is too long to read: its digits serve)
        let long = !root.approx && root.value.to_string().len() > 80;
        let (re, im, approx) = if long {
            split(&numeric::n(&root.value, numeric::digits()), true)
        } else {
            split(&root.value, root.approx)
        };
        let zero = Expr::zero;
        let (parts, approx) = if vanishes(&im) {
            ([re, zero(), zero(), zero()], approx)
        } else {
            let radius = radius_of(im);
            if !classes.is_new(&re, &radius) {
                continue;
            }
            let trace = mul(vec![Expr::int(2), re.clone()]);
            let norm = simplify::simplify(&add(vec![
                pow(re.clone(), Expr::int(2)),
                pow(radius.clone(), Expr::int(2)),
            ]));
            // x^k = p x + q on this class, from x^2 = trace x - norm
            let (mut p, mut q) = (Expr::zero(), Expr::one());
            let (mut a, mut b): (Parts, Parts) = (
                std::array::from_fn(|_| Expr::zero()),
                std::array::from_fn(|_| Expr::zero()),
            );
            for coefficient in &coeffs {
                for s in 0..4 {
                    a[s] = add(vec![
                        a[s].clone(),
                        mul(vec![coefficient[s].clone(), p.clone()]),
                    ]);
                    b[s] = add(vec![
                        b[s].clone(),
                        mul(vec![coefficient[s].clone(), q.clone()]),
                    ]);
                }
                let next = simplify::simplify(&add(vec![mul(vec![trace.clone(), p.clone()]), q]));
                q = simplify::simplify(&neg(mul(vec![norm.clone(), p])));
                p = next;
            }
            let a = a.map(|e| simplify::simplify(&e));
            let b = b.map(|e| simplify::simplify(&e));
            if negligible(&a, approx) {
                if negligible(&b, approx) && on_sphere(&equations, names, &re, &radius, approx) {
                    out.sphere = true;
                    out.solutions.push(Solution::new(
                        vec![sphere_value(&re, &radius, sphere)],
                        approx,
                        1,
                    ));
                }
                continue;
            }
            let value = mul(vec![
                neg(pow(quat::make(a), Expr::minus_one())),
                quat::make(b),
            ]);
            let parts = quat::lift(&value).map(|e| simplify::simplify(&e));
            readable(parts, approx)
        };
        let parts = if right { conjugate(&parts) } else { parts };
        if points.iter().any(|known| same_parts(known, &parts))
            || !fits(&equations, names, &parts, approx)
        {
            continue;
        }
        points.push(parts.clone());
        out.solutions
            .push(Solution::new(vec![quat::tidy(parts)], approx, 1));
    }
    Some(out)
}

/// Whether a numeric value is real: no imaginary part above the rounding noise.
fn is_real_value(z: &Complex) -> bool {
    let noise =
        (Float::with_val(64, z.abs_ref()) + 1u32) >> (numeric::bits_for(numeric::digits()) - 8);
    Float::with_val(64, z.imag().abs_ref()) < noise
}

/// Whether some real point satisfies equations that are not more than the unknowns.
fn has_point(comps: &[Expr], names: &[String]) -> bool {
    let digits = numeric::digits().min(40);
    let (m, n) = (comps.len(), names.len());
    let mut dice = system::Dice(0x2545_F491_4F6C_DD1D);
    for attempt in 0..40 {
        let active: Vec<String> = (0..m).map(|i| names[(attempt + i) % n].clone()).collect();
        let mut eqs = comps.to_vec();
        for name in names.iter().filter(|name| !active.contains(name)) {
            let fixed = Expr::rat((dice.roll() * 1000.0) as i64, 1000);
            eqs = eqs.iter().map(|e| subst_sym(e, name, &fixed)).collect();
        }
        let start: Vec<Complex> = active
            .iter()
            .map(|_| Complex::with_val(64, (dice.roll(), 0.0)))
            .collect();
        if fit::newton_system(&eqs, &active, &start, digits)
            .is_some_and(|point| point.iter().all(is_real_value))
        {
            return true;
        }
    }
    false
}

/// The components as a system of real equations.
fn general(
    comps: &[Expr],
    names: &[String],
    unknowns: &[Unknown],
    sphere: &str,
) -> Result<Solved, String> {
    let digits = numeric::digits();
    let only_unknowns = comps.iter().all(|c| {
        c.free_syms()
            .iter()
            .all(|s| names.iter().any(|n| **s == **n))
    });
    let square = comps.len() == names.len() && only_unknowns;
    let limit = std::time::Duration::from_secs(20);
    let found = crate::budget::with_limit(limit, || system::solve_system(comps, names));
    let mut out = Solved::default();
    let mut unclear = false;
    let mut known: Vec<Vec<Complex>> = Vec::new();
    let mut families = false;
    let mut complete = false;
    match found {
        Ok(found) => {
            complete = found.complete;
            for s in &found.solutions {
                let full: Vec<Expr> = names
                    .iter()
                    .map(|n| {
                        s.iter()
                            .find(|(name, _)| name == n)
                            .map_or_else(|| Expr::sym(n), |(_, v)| v.clone())
                    })
                    .collect();
                if !satisfied(comps, s, found.approx) {
                    continue;
                }
                if full.iter().all(Expr::is_const) {
                    let values: Option<Vec<Complex>> = full
                        .iter()
                        .map(|v| numeric::evalf_digits(v, digits).ok())
                        .collect();
                    match values {
                        Some(point) if point.iter().all(is_real_value) => {
                            // numbers where the exact form is too long to read
                            let length: usize = full.iter().map(|v| v.to_string().len()).sum();
                            let long = length > 240 && !found.approx;
                            let shown: system::Assignment = if long {
                                let rounded = point.iter().map(|z| float_expr(z.clone()));
                                names.iter().cloned().zip(rounded).collect()
                            } else {
                                s.clone()
                            };
                            known.push(point);
                            out.solutions
                                .push(solution(unknowns, &shown, found.approx || long));
                        }
                        Some(_) => {}
                        None => unclear = true,
                    }
                } else if full.iter().all(assume::is_real) {
                    families = true;
                    for n in names {
                        if full.iter().any(|v| v.has_sym(n)) && !out.free.contains(n) {
                            out.free.push(n.clone());
                        }
                    }
                    out.solutions.push(solution(unknowns, s, found.approx));
                } else {
                    unclear = true;
                }
            }
            out.params = found
                .params
                .iter()
                .filter(|p| {
                    out.solutions
                        .iter()
                        .any(|s| s.values.iter().any(|v| v.has_sym(p)))
                })
                .cloned()
                .collect();
            for p in found.params.iter().filter(|p| !out.params.contains(p)) {
                assume::clear(p);
            }
        }
        // (no values: the cases of polynomial equations tell what there is)
        Err(_) if real_cases(comps, names).is_some() => unclear = true,
        Err(e) if !square => return Err(e),
        Err(_) => {}
    }
    if unclear {
        solve::release(&out.params);
        return Ok(described(comps, names, unknowns, sphere));
    }
    if complete {
        return Ok(out);
    }
    if square && !families {
        for point in system::numeric_search(comps, names, true, &mut known) {
            let exact = snapped(comps, names, &point);
            let approx = exact.is_none();
            let assignment = exact.unwrap_or_else(|| {
                let values = point.into_iter().map(float_expr);
                names.iter().cloned().zip(values).collect()
            });
            out.solutions.push(solution(unknowns, &assignment, approx));
        }
    }
    out.notes.push(SEARCHED.to_string());
    Ok(out)
}

/// A case that is a sphere about a real number, `x0 = a` and `x1^2 + x2^2 + x3^2 = r^2`, as
/// `a + r*u` with the unit pure quaternion named `sphere`.
fn sphere_of(case: &Case, unknowns: &[Unknown], sphere: &str) -> Option<Expr> {
    let [unknown] = unknowns else {
        return None;
    };
    let names = unknown.parts.as_ref()?;
    let ([(name, centre)], [condition]) = (case.fixed.as_slice(), case.left.as_slice()) else {
        return None;
    };
    if *name != names[0] || !centre.is_const() {
        return None;
    }
    let squares = add(names[1..]
        .iter()
        .map(|n| pow(Expr::sym(n), Expr::int(2)))
        .collect());
    // the condition is c*(x1^2 + x2^2 + x3^2 - r^2)
    let scale = ratsimp::coefficients(condition, &names[1])?.get(2)?.clone();
    let rest = simplify::simplify(&sub(div(condition.clone(), scale), squares));
    let square = neg(rest);
    if !square.is_const() || numeric::sign(&square) != Some(1) {
        return None;
    }
    Some(sphere_value(
        centre,
        &simplify::simplify(&sqrt(square)),
        sphere,
    ))
}

/// The solutions of equations that give no values for all the components: cases with the
/// components that are fixed and the conditions on the others.
fn described(comps: &[Expr], names: &[String], unknowns: &[Unknown], sphere: &str) -> Solved {
    let mut out = Solved::default();
    let cases = real_cases(comps, names).unwrap_or_else(|| {
        vec![Case {
            fixed: Vec::new(),
            left: comps.to_vec(),
            approx: false,
        }]
    });
    for case in cases {
        let open: Vec<String> = names
            .iter()
            .filter(|n| !case.fixed.iter().any(|(f, _)| f == *n))
            .cloned()
            .collect();
        let only_open = |c: &Expr| {
            c.free_syms()
                .iter()
                .all(|s| open.iter().any(|n| **s == **n))
        };
        if !case.left.is_empty()
            && case.left.len() <= open.len()
            && case.left.iter().all(only_open)
            && !has_point(&case.left, &open)
        {
            let shown: Vec<String> = case.left.iter().map(condition).collect();
            out.notes.push(format!(
                "a numeric search found no real numbers with {}: there may be none",
                shown.join(", ")
            ));
        }
        // numbers where the exact values are too long to read
        let length: usize = case.fixed.iter().map(|(_, v)| v.to_string().len()).sum();
        let long = length > 240 && !case.approx;
        let fixed: system::Assignment = if long {
            let rounded = |v: &Expr| numeric::n(v, numeric::digits());
            case.fixed
                .iter()
                .map(|(n, v)| (n.clone(), rounded(v)))
                .collect()
        } else {
            case.fixed.clone()
        };
        let mut found = solution(unknowns, &fixed, case.approx || long);
        if let Some(value) = sphere_of(&case, unknowns, sphere) {
            found.values = vec![value];
            out.sphere = true;
            out.solutions.push(found);
            continue;
        }
        for n in &open {
            let used =
                found.values.iter().any(|v| v.has_sym(n)) || case.left.iter().any(|c| c.has_sym(n));
            if used && !out.free.contains(n) {
                out.free.push(n.clone());
            }
        }
        found.conditions = case.left;
        out.solutions.push(found);
    }
    out
}

/// Solves the equations `comps = 0` between real components for the unknowns.
pub fn by_components(comps: &[Expr], unknowns: &[Unknown], sphere: &str) -> Result<Solved, String> {
    let names = all_names(unknowns);
    let comps: Vec<Expr> = comps.iter().map(plain).filter(|c| !vanishes(c)).collect();
    if comps.is_empty() {
        return Ok(Solved {
            all: true,
            ..Solved::default()
        });
    }
    if comps.iter().any(|c| !names.iter().any(|n| c.has_sym(n))) {
        return Ok(Solved::default());
    }
    if let Some(rows) = system::linear_rows(&comps, &names) {
        return Ok(linear(rows, &names, unknowns));
    }
    general(&comps, &names, unknowns, sphere)
}

/// Solves one equation, given by its four components, for one quaternion. `slice` is the
/// equation as `G(x) = c` with the terms that have the unknown on the left; `sphere` names any
/// unit pure quaternion in the answer.
pub fn single(
    comps: &[Expr; 4],
    unknown: &Unknown,
    slice: Option<&(Expr, Expr)>,
    sphere: &str,
) -> Result<Solved, String> {
    let names = unknown
        .parts
        .as_ref()
        .ok_or("the unknown is not a quaternion")?;
    let comps = &comps.each_ref().map(plain);
    let present: Vec<Expr> = comps.iter().filter(|c| !vanishes(c)).cloned().collect();
    let has_unknown = |c: &Expr| names.iter().any(|n| c.has_sym(n));
    let unknowns = std::slice::from_ref(unknown);
    if present.is_empty() || !present.iter().all(has_unknown) {
        return by_components(comps, unknowns, sphere);
    }
    if let Some(rows) = system::linear_rows(&present, names) {
        return Ok(linear(rows, names, unknowns));
    }
    if let Some((g, c)) = slice
        && let Some(solved) = planes(g, c, comps, unknown, sphere)
    {
        return Ok(solved);
    }
    if let Some((coeffs, right)) = one_sided(comps, names)
        && let Some(solved) = companion(coeffs, right, comps, unknown, sphere)
    {
        return Ok(solved);
    }
    general(&present, names, unknowns, sphere)
}

/// A condition `e = 0` written with its number on the right.
pub fn condition(e: &Expr) -> String {
    let terms: Vec<Expr> = match e {
        Expr::Add(a) => a.to_vec(),
        other => vec![other.clone()],
    };
    let (numbers, rest): (Vec<Expr>, Vec<Expr>) = terms.into_iter().partition(Expr::is_number);
    let (mut left, mut right) = (add(rest), neg(add(numbers)));
    if crate::funcs::is_neg_form(&left) {
        (left, right) = (neg(left), neg(right));
    }
    format!("{left} = {right}")
}
