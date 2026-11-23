//! Systems of equations: linear by elimination, polynomial by a Groebner basis, others by substitution.

use crate::canon::{add, mul, neg, sub, subst_all, subst_sym};
use crate::expr::Expr;
use crate::fit;
use crate::matrix;
use crate::numeric;
use crate::poly;
use crate::ratsimp;
use crate::simplify;
use crate::solve;
use rug::{Complex, Float};
use std::collections::BTreeSet;

pub type Assignment = Vec<(String, Expr)>;

#[derive(Default, Debug)]
pub struct SystemSolved {
    pub solutions: Vec<Assignment>,
    pub notes: Vec<String>,
    /// Some value is numeric.
    pub approx: bool,
    /// Parameters introduced by the solver (integers of periodic families).
    pub params: Vec<String>,
    /// The equations contradict each other: no search can find a solution.
    pub empty: bool,
    /// Every solution is listed: the system is linear, or it was solved one unknown at a time
    /// through equations of a single unknown.
    pub complete: bool,
}

fn uses_any(e: &Expr, vars: &[String]) -> bool {
    vars.iter().any(|v| e.has_sym(v))
}

/// Coefficient rows `[a_1 .. a_n | c]` with `eq = sum a_j v_j + c` when every equation is linear.
pub(crate) fn linear_rows(eqs: &[Expr], vars: &[String]) -> Option<matrix::Rows> {
    let mut rows = Vec::new();
    for eq in eqs {
        let mut row = Vec::new();
        let mut rest = eq.clone();
        for v in vars {
            let cs = ratsimp::coefficients(&rest, v)?;
            if cs.len() > 2 {
                return None;
            }
            let a = cs.get(1).cloned().unwrap_or_else(Expr::zero);
            if uses_any(&a, vars) {
                return None;
            }
            rest = cs.first().cloned().unwrap_or_else(Expr::zero);
            row.push(a);
        }
        if uses_any(&rest, vars) {
            return None;
        }
        row.push(rest);
        rows.push(row);
    }
    Some(rows)
}

pub(crate) fn solve_linear(rows: matrix::Rows, vars: &[String], out: &mut SystemSolved) {
    let n = vars.len();
    crate::trace::step(|| {
        format!(
            "the equations are linear in {}: the rows of coefficients with the right sides, {}",
            vars.join(", "),
            matrix::from_rows(
                rows.iter()
                    .map(|r| {
                        let mut row = r.clone();
                        if let Some(last) = row.last_mut() {
                            *last = neg(last.clone());
                        }
                        row
                    })
                    .collect()
            )
        )
    });
    let (m, rank, _) = matrix::eliminate(rows, true);
    crate::trace::step(|| {
        format!(
            "elimination (Gauss and Jordan) leaves {} of rank {rank}",
            matrix::from_rows(
                m.iter()
                    .map(|r| {
                        let mut row = r.clone();
                        if let Some(last) = row.last_mut() {
                            *last = neg(last.clone());
                        }
                        row
                    })
                    .collect()
            )
        )
    });
    out.empty = true;
    out.complete = true;
    for row in &m[rank.min(m.len())..] {
        if !simplify::is_zero(&row[n]) {
            return;
        }
    }
    let mut pivots = vec![None; n];
    for (i, row) in m.iter().enumerate().take(rank) {
        match (0..n).find(|j| !row[*j].is_zero()) {
            Some(j) => pivots[j] = Some(i),
            None => {
                if !simplify::is_zero(&row[n]) {
                    return;
                }
            }
        }
    }
    out.empty = false;
    let free: Vec<usize> = (0..n).filter(|j| pivots[*j].is_none()).collect();
    let mut solution = Vec::new();
    for (j, p) in pivots.iter().enumerate() {
        if let Some(i) = p {
            let mut terms = vec![neg(m[*i][n].clone())];
            for &f in &free {
                terms.push(neg(mul(vec![m[*i][f].clone(), Expr::sym(&vars[f])])));
            }
            solution.push((vars[j].clone(), simplify::simplify(&add(terms))));
        }
    }
    if !free.is_empty() {
        let names: Vec<&str> = free.iter().map(|f| vars[*f].as_str()).collect();
        out.notes
            .push(format!("free: {} (any value)", names.join(", ")));
    }
    out.solutions.push(solution);
}

struct Search {
    notes: Vec<String>,
    approx: bool,
    params: Vec<String>,
    budget: u32,
    /// A step may have lost solutions: it solved an equation of several unknowns for one of
    /// them, or its solver had a remark.
    partial: bool,
}

impl Search {
    /// Solves equations one unknown at a time.
    fn chain(&mut self, eqs: &[Expr], vars: &[String]) -> Result<Vec<Assignment>, String> {
        let eqs: Vec<Expr> = eqs
            .iter()
            .map(simplify::normal)
            .filter(|e| !e.is_zero())
            .collect();
        if eqs.iter().any(|e| !uses_any(e, vars)) {
            return Ok(Vec::new());
        }
        if eqs.is_empty() {
            return Ok(vec![Vec::new()]);
        }
        if vars.is_empty() {
            return Ok(Vec::new());
        }
        if self.budget == 0 {
            return Err("the system needs too many steps".into());
        }
        self.budget -= 1;
        // an equation with one unknown first, then the shortest ones
        let mut order: Vec<(usize, usize, usize)> = Vec::new();
        for (i, eq) in eqs.iter().enumerate() {
            let present: Vec<usize> = (0..vars.len()).filter(|j| eq.has_sym(&vars[*j])).collect();
            for &j in &present {
                order.push((present.len(), eq.size(), i * vars.len() + j));
            }
        }
        order.sort();
        let mut last_error = "no method for this system".to_string();
        let begun = crate::trace::mark();
        for (count, _, code) in order {
            // (the steps of an equation that is not used after all are taken back)
            crate::trace::forget(begun);
            let (i, j) = (code / vars.len(), code % vars.len());
            crate::trace::step(|| {
                format!(
                    "{} = 0 is solved for {}{}",
                    eqs[i],
                    vars[j],
                    if count == 1 { " (its only unknown)" } else { "" }
                )
            });
            let nest = crate::trace::nest();
            let solved = match solve::solve(&eqs[i], &vars[j]) {
                Ok(s) => s,
                Err(e) => {
                    last_error = e;
                    continue;
                }
            };
            drop(nest);
            if solved.all {
                continue;
            }
            if count > 1 && (solved.roots.iter().any(|r| r.approx) || !solved.params.is_empty()) {
                solve::release(&solved.params);
                continue;
            }
            self.approx |= solved.roots.iter().any(|r| r.approx);
            // with other unknowns in it an equation gives all the values of one unknown when
            // its degree is at most 2 and stays so whatever the others are
            let steady = count == 1
                || ratsimp::coefficients(&eqs[i], &vars[j]).is_some_and(|cs| {
                    cs.len() <= 3 && cs.last().is_some_and(|c| !uses_any(c, vars))
                });
            self.partial |= !steady || !solved.notes.is_empty() || !solved.params.is_empty();
            self.params.extend(solved.params.iter().cloned());
            for n in solved.notes {
                // (every equation of the chain had a count of roots of its own: the system
                // has one note)
                let n = match crate::settings::roots() {
                    Some(want) if n.starts_with("numeric search (roots = ") => format!(
                        "numeric search (roots = {want}): the solutions found are listed, there may be others"
                    ),
                    _ => n,
                };
                if !self.notes.contains(&n) {
                    self.notes.push(n);
                }
            }
            let rest_vars: Vec<String> = vars.iter().filter(|v| **v != vars[j]).cloned().collect();
            let mut out = Vec::new();
            for r in &solved.roots {
                let rest: Vec<Expr> = eqs
                    .iter()
                    .enumerate()
                    .filter(|(k, _)| *k != i)
                    .map(|(_, e)| subst_sym(e, &vars[j], &r.value))
                    .collect();
                if !rest.is_empty() {
                    crate::trace::step(|| {
                        format!(
                            "{} = {} is put into the other equations: {}",
                            vars[j],
                            r.value,
                            rest.iter()
                                .map(|e| format!("{} = 0", simplify::normal(e)))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    });
                }
                let _nest = crate::trace::nest();
                for mut tail in self.chain(&rest, &rest_vars)? {
                    let mut value = r.value.clone();
                    for (name, v) in &tail {
                        value = subst_sym(&value, name, v);
                    }
                    tail.push((
                        vars[j].clone(),
                        if r.approx { value } else { solve::tidy(&value) },
                    ));
                    out.push(tail);
                }
            }
            return Ok(out);
        }
        crate::trace::forget(begun);
        Err(last_error)
    }
}

fn check(eqs: &[Expr], solution: &Assignment, approx: bool) -> bool {
    eqs.iter().all(|eq| {
        // (all the values at once: one after the other 0/0 can pass as 0)
        let e = subst_all(eq, solution);
        if e.any(&|x| x.is_infinite()) {
            return false;
        }
        if approx || e.has_float() {
            let env = numeric::Env::new();
            let vars: Vec<std::rc::Rc<str>> = e.free_syms().into_iter().collect();
            let env = if vars.is_empty() {
                env
            } else {
                numeric::sample_env(&vars, 0, 256)
            };
            let bits = i64::from(numeric::bits_for(numeric::digits()));
            return numeric::relative_size(&e, numeric::bits_for(numeric::digits()) + 64, &env)
                .is_some_and(|s| s < -bits + 28);
        }
        simplify::is_zero(&e)
    })
}

/// The factors of `e` with an unknown, each once; `e` itself when it is no product of them.
pub(crate) fn factors(e: &Expr, vars: &[String]) -> Vec<Expr> {
    if e.size() > 400 {
        return vec![e.clone()];
    }
    let factored = ratsimp::factor(e);
    let parts: Vec<Expr> = match &factored {
        Expr::Mul(a) => a.to_vec(),
        Expr::Fn(n, a) if &**n == "hold" => match a.first() {
            Some(Expr::Mul(a)) => a.to_vec(),
            _ => return vec![e.clone()],
        },
        _ => return vec![e.clone()],
    };
    let mut out: Vec<Expr> = Vec::new();
    for p in parts.iter().filter(|p| uses_any(p, vars)) {
        let base = match p {
            Expr::Pow(b, k) => match k.as_int() {
                Some(k) if k.is_positive() => (**b).clone(),
                _ => return vec![e.clone()],
            },
            other => other.clone(),
        };
        if !out.contains(&base) {
            out.push(base);
        }
    }
    if out.is_empty() {
        return vec![e.clone()];
    }
    out
}

/// Whether every point of the solution `a` is one of `b`: `a` satisfies what defines `b`.
fn inside(a: &Assignment, b: &Assignment) -> bool {
    b.iter().all(|(name, value)| {
        let mine = a
            .iter()
            .find(|(n, _)| n == name)
            .map_or_else(|| Expr::sym(name), |(_, v)| v.clone());
        simplify::is_zero(&sub(mine, subst_all(value, a)))
    })
}

/// Numbers in [-2.5, 2.5], the same in every run.
pub(crate) struct Dice(pub u64);

impl Dice {
    pub(crate) fn roll(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64 * 5.0 - 2.5
    }
}

/// A numeric solution with the parts that are rounding noise set to zero.
fn settled(point: &[Complex], digits: u32) -> Vec<Complex> {
    let bits = numeric::bits_for(digits);
    point
        .iter()
        .map(|z| {
            let noise = (Float::with_val(64, z.abs_ref()) + 1u32) >> (bits - 8);
            let part = |x: &Float| {
                if Float::with_val(64, x.abs_ref()) < noise {
                    Float::new(x.prec())
                } else {
                    x.clone()
                }
            };
            Complex::with_val(z.prec(), (part(z.real()), part(z.imag())))
        })
        .collect()
}

fn near(a: &[Complex], b: &[Complex]) -> bool {
    a.iter().zip(b).all(|(x, y)| {
        let gap = Float::with_val(64, Complex::with_val(x.prec(), x - y).abs_ref());
        gap < (Float::with_val(64, x.abs_ref()) + 1u32) >> 40u32
    })
}

/// Every part of the equations has a value at the point: no division by zero and no logarithm
/// of zero, where an equation holds only as a limit.
pub(crate) fn defined_at(eqs: &[Expr], vars: &[String], point: &[Complex], digits: u32) -> bool {
    let bits = numeric::bits_for(digits);
    let env: numeric::Env = vars
        .iter()
        .map(|v| std::rc::Rc::from(v.as_str()))
        .zip(point.iter().cloned())
        .collect();
    let value = |e: &Expr| numeric::evalf_env(e, bits + 64, &env).ok();
    let finite = |z: &Complex| z.real().is_finite() && z.imag().is_finite();
    let mut fine = true;
    for f in eqs {
        f.walk(&mut |part| {
            if !fine {
                return;
            }
            fine = match part {
                Expr::Num(_) | Expr::Float(_) | Expr::Sym(_) => true,
                Expr::Pow(base, x) if x.as_rat().is_some_and(|r| r.is_negative()) => {
                    // (a search that ends next to such a point is short of it by the root of
                    // its precision)
                    value(base).is_some_and(|b| {
                        finite(&b)
                            && Float::with_val(64, b.abs_ref())
                                > Float::with_val(64, 1) >> (bits / 2 - 8)
                    })
                }
                other => value(other).is_some_and(|z| finite(&z)),
            };
        });
    }
    fine
}

/// Solutions of as many equations as unknowns by Newton's method from many starting points,
/// without the ones in `known` (which they join). With `real` only real solutions count. The
/// six nearest to the origin are given: a periodic system has no end of them. `roots = N` asks
/// for N of them, from as many more starting points.
pub fn numeric_search(
    eqs: &[Expr],
    vars: &[String],
    real: bool,
    known: &mut Vec<Vec<Complex>>,
) -> Vec<Vec<Complex>> {
    let digits = numeric::digits();
    let jac = fit::jacobian(eqs, vars);
    let mut dice = Dice(0x9E37_79B9_7F4A_7C15);
    let mut found = Vec::new();
    let started = std::time::Instant::now();
    let asked = crate::settings::roots().map(|n| n as usize);
    let most = asked.unwrap_or(6);
    let usual = if real { 60 } else { 120 };
    let rounds = usual * most.div_ceil(6).min(400);
    let seconds = 15 + asked.map_or(0, |n| (n as u64 / 2).min(285));
    crate::trace::step(|| {
        format!(
            "Newton's method for {} equations from {rounds} starting points of chance (a fixed row of them): the {most} solutions nearest to 0 are kept",
            eqs.len()
        )
    });
    for round in 0..rounds {
        if started.elapsed().as_secs() >= seconds {
            break;
        }
        // (the starting points of more rounds than usual lie farther out)
        let wide = (1 + round / usual) as f64;
        let start: Vec<Complex> = vars
            .iter()
            .map(|_| {
                let re = dice.roll() * wide;
                let im = if real || round % 2 == 0 {
                    0.0
                } else {
                    dice.roll() * wide
                };
                Complex::with_val(64, (re, im))
            })
            .collect();
        let Some(point) = fit::newton_from(eqs, &jac, vars, &start, digits) else {
            continue;
        };
        let point = settled(&point, digits);
        if (real && point.iter().any(|z| !z.imag().is_zero()))
            || known.iter().any(|k| near(k, &point))
            || !defined_at(eqs, vars, &point, digits)
        {
            continue;
        }
        known.push(point.clone());
        found.push(point);
    }
    let size = |p: &Vec<Complex>| {
        p.iter()
            .map(|z| Float::with_val(64, z.abs_ref()))
            .fold(Float::new(64), |m, x| m + x)
    };
    found.sort_by(|a, b| {
        size(a)
            .partial_cmp(&size(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    found.truncate(most);
    found
}

/// Solves `eqs = 0` for `vars`.
pub fn solve_system(given: &[Expr], vars: &[String]) -> Result<SystemSolved, String> {
    let mut out = SystemSolved::default();
    crate::trace::step(|| {
        format!(
            "a system of {} for {}: {}",
            crate::trace::count(given.len(), "equation"),
            vars.join(", "),
            given
                .iter()
                .map(|e| format!("{e} = 0"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    });
    let _nest = crate::trace::nest();
    let eqs: Vec<Expr> = given
        .iter()
        .map(|e| ratsimp::fraction(&simplify::normal(e)).0)
        .collect();
    // a product vanishes where a factor does: one system for each choice of factors
    let mut choices: Vec<Vec<Expr>> = Vec::new();
    let mut cases = 1usize;
    for e in &eqs {
        let mut parts = factors(e, vars);
        if cases * parts.len() > 48 {
            parts = vec![e.clone()];
        }
        cases *= parts.len();
        choices.push(parts);
    }
    out.empty = true;
    out.complete = true;
    let mut failure = None;
    if cases > 1 {
        crate::trace::step(|| {
            format!(
                "equations that are products hold where one of their factors is 0: {cases} systems, one for each choice of factors"
            )
        });
    }
    for case in 0..cases {
        let mut index = case;
        let branch: Vec<Expr> = choices
            .iter()
            .map(|parts| {
                let pick = parts[index % parts.len()].clone();
                index /= parts.len();
                pick
            })
            .collect();
        if cases > 1 {
            crate::trace::step(|| {
                format!(
                    "case {}: {}",
                    case + 1,
                    branch
                        .iter()
                        .map(|e| format!("{e} = 0"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            });
        }
        let _case = (cases > 1).then(crate::trace::nest);
        match solve_case(&branch, vars) {
            Ok(found) => {
                out.empty &= found.empty;
                out.complete &= found.complete;
                out.approx |= found.approx;
                for s in found.solutions {
                    if !out.solutions.contains(&s) {
                        out.solutions.push(s);
                    }
                }
                for n in found.notes {
                    if !n.starts_with("free:") && !out.notes.contains(&n) {
                        out.notes.push(n);
                    }
                }
                out.params.extend(found.params);
            }
            Err(e) => {
                out.empty = false;
                out.complete = false;
                failure.get_or_insert(e);
            }
        }
    }
    // a value that makes a denominator vanish is no solution
    let below = |e: &Expr| uses_any(&ratsimp::fraction(&simplify::normal(e)).1, vars);
    if given.iter().any(below) {
        let approx = out.approx;
        out.solutions.retain(|s| check(given, s, approx));
    }
    if let Some(e) = failure {
        if out.solutions.is_empty() {
            return Err(e);
        }
        out.notes
            .push(format!("a case of the system was left out: {e}"));
    }
    // a solution that lies in another one says nothing new
    let mut keep = vec![true; out.solutions.len()];
    for a in 0..out.solutions.len() {
        for b in 0..out.solutions.len() {
            if a != b
                && keep[b]
                && inside(&out.solutions[a], &out.solutions[b])
                && (b < a || !inside(&out.solutions[b], &out.solutions[a]))
            {
                keep[a] = false;
                break;
            }
        }
    }
    let mut kept = keep.iter();
    out.solutions
        .retain(|_| *kept.next().expect("one flag for each"));
    if cases > 1 {
        // the cases come in the order of the factors: numbers by size, the rest by their text
        let key = |s: &Assignment| -> Vec<(u8, f64, f64, String)> {
            s.iter()
                .map(|(_, v)| {
                    let value = v
                        .is_const()
                        .then(|| numeric::evalf_digits(v, 20).ok())
                        .flatten();
                    match value {
                        Some(z) => (0, z.real().to_f64(), z.imag().to_f64(), String::new()),
                        None => (1, 0.0, 0.0, v.to_string()),
                    }
                })
                .collect()
        };
        out.solutions.sort_by(|a, b| {
            key(a)
                .partial_cmp(&key(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    for s in &out.solutions {
        if s.len() < vars.len() {
            let missing: Vec<&str> = vars
                .iter()
                .filter(|v| !s.iter().any(|(n, _)| n == *v))
                .map(String::as_str)
                .collect();
            let note = format!("free: {} (any value)", missing.join(", "));
            if !out.notes.contains(&note) {
                out.notes.push(note);
            }
        }
    }
    Ok(out)
}

/// One case of a system: the equations as they are.
fn solve_case(eqs: &[Expr], vars: &[String]) -> Result<SystemSolved, String> {
    let mut out = SystemSolved::default();
    if let Some(rows) = linear_rows(eqs, vars) {
        solve_linear(rows, vars, &mut out);
        return Ok(out);
    }
    if eqs
        .iter()
        .any(|e| !uses_any(e, vars) && !simplify::is_zero(e))
    {
        out.empty = true;
        out.complete = true;
        return Ok(out);
    }
    let mut work = eqs.to_vec();
    let mut triangular = false;
    // polynomial systems: a lexicographic Groebner basis is triangular
    let refs: Vec<&Expr> = eqs.iter().collect();
    if let Some((gens, polys)) = ratsimp::common(&refs)
        && gens
            .kernels()
            .iter()
            .all(|k| k.sym_name().is_some_and(|s| vars.iter().any(|v| v == s)))
        && let Some(basis) = poly::groebner(&polys, 4000)
    {
        if basis.len() == 1 && basis[0].is_const() {
            out.empty = true;
            out.complete = true;
            return Ok(out);
        }
        work = basis.iter().map(|p| gens.expr(p)).collect();
        triangular = true;
        crate::trace::step(|| {
            format!(
                "polynomial equations: a Groebner basis (the unknowns removed one after the other) is {}",
                work.iter()
                    .map(|e| format!("{e} = 0"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        });
    }
    let mut search = Search {
        notes: Vec::new(),
        approx: false,
        params: Vec::new(),
        budget: 400,
        partial: false,
    };
    let found = crate::trace::whole(|| search.chain(&work, vars))?;
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for mut s in found {
        s.sort_by_key(|(name, _)| vars.iter().position(|v| v == name).unwrap_or(usize::MAX));
        if !check(eqs, &s, search.approx) {
            crate::trace::step(|| {
                format!(
                    "{} is put into the equations as they were given: they do not all hold, so it is left out",
                    s.iter()
                        .map(|(n, v)| format!("{n} = {v}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            });
            search.partial = true;
            continue;
        }
        let key: Vec<String> = s.iter().map(|(n, v)| format!("{n}={v}")).collect();
        if seen.insert(key.join(",")) {
            out.solutions.push(s);
        }
    }
    out.complete = triangular && !search.partial;
    out.notes = search.notes;
    out.approx = search.approx;
    out.params = search.params;
    Ok(out)
}
