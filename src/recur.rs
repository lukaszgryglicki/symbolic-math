//! Linear recurrences with constant coefficients.

use crate::canon::{add, mul, pow, powi, sub, subst_sym};
use crate::expand::expand;
use crate::expr::Expr;
use crate::ratsimp;
use crate::simplify::{is_zero, simplify};
use crate::solve;
use crate::system;

/// Name of the symbol standing for the term `a(n + k)`.
pub fn marker(k: i64) -> String {
    format!("#s{k}")
}

const ROOT: &str = "#r";

fn terms_of(e: &Expr) -> Vec<Expr> {
    match e {
        Expr::Add(a) => a.to_vec(),
        other => vec![other.clone()],
    }
}

/// A particular solution for a right side `sum P(n) lambda^n`; `c[j]` multiplies `a(n + j)`.
fn particular(c: &[Expr], g: &Expr, n: &str, roots: &[solve::Root]) -> Option<Expr> {
    let ns = Expr::sym(n);
    let mut groups: Vec<(Expr, Vec<Expr>)> = Vec::new();
    for t in terms_of(&expand(g)) {
        let factors: Vec<Expr> = match &t {
            Expr::Mul(a) => a.to_vec(),
            other => vec![other.clone()],
        };
        let (mut lambda, mut deg, mut coef) = (Vec::new(), 0usize, Vec::new());
        for f in factors {
            let (b, k) = f.base_exp();
            if !f.has_sym(n) {
                coef.push(f);
            } else if b == ns {
                deg += usize::try_from(k.as_i64()?).ok()?;
            } else if !b.has_sym(n) {
                let co = ratsimp::coefficients(&k, n).filter(|co| co.len() == 2)?;
                lambda.push(pow(b.clone(), co[1].clone()));
                coef.push(pow(b, co[0].clone()));
            } else {
                return None;
            }
        }
        let lambda = simplify(&mul(lambda));
        let slot = match groups
            .iter()
            .position(|(l, _)| is_zero(&sub(l.clone(), lambda.clone())))
        {
            Some(i) => i,
            None => {
                groups.push((lambda, Vec::new()));
                groups.len() - 1
            }
        };
        let poly = &mut groups[slot].1;
        if poly.len() <= deg {
            poly.resize(deg + 1, Expr::zero());
        }
        poly[deg] = add(vec![poly[deg].clone(), mul(coef)]);
    }
    let mut total = Vec::new();
    for (lambda, pc) in groups {
        let s = roots
            .iter()
            .find(|r| is_zero(&sub(r.value.clone(), lambda.clone())))
            .map_or(0, |r| r.mult as usize);
        let names: Vec<String> = (0..pc.len()).map(|i| format!("#q{i}")).collect();
        let trial = |at: Expr| {
            let q = add(names
                .iter()
                .enumerate()
                .map(|(i, name)| mul(vec![Expr::sym(name), powi(at.clone(), i as i64)]))
                .collect());
            mul(vec![powi(at, s as i64), q])
        };
        // sum_j c_j trial(n + j) lambda^j = P(n)
        let left = add(c
            .iter()
            .enumerate()
            .map(|(j, cj)| {
                mul(vec![
                    cj.clone(),
                    trial(add(vec![ns.clone(), Expr::int(j as i64)])),
                    powi(lambda.clone(), j as i64),
                ])
            })
            .collect());
        let right = add(pc
            .iter()
            .enumerate()
            .map(|(i, p)| mul(vec![p.clone(), powi(ns.clone(), i as i64)]))
            .collect());
        let eqs: Vec<Expr> = ratsimp::coefficients(&sub(left, right), n)?
            .into_iter()
            .filter(|e| !e.is_zero())
            .collect();
        let solved = system::solve_system(&eqs, &names).ok()?;
        let values = solved.solutions.first()?;
        let fitted = names.iter().fold(trial(ns.clone()), |acc, name| {
            let value = values
                .iter()
                .find(|(v, _)| v == name)
                .map_or_else(Expr::zero, |(_, e)| e.clone());
            subst_sym(&acc, name, &value)
        });
        total.push(mul(vec![fitted, pow(lambda, ns.clone())]));
    }
    Some(simplify(&add(total)))
}

/// Solves `f = 0`, where `f` holds the markers of `a(n + k)` for the given shifts; conditions are `a(index) = value`.
pub fn rsolve(
    f: &Expr,
    n: &str,
    shifts: &[i64],
    conditions: &[(Expr, Expr)],
) -> Result<Expr, String> {
    let (Some(&low), Some(&high)) = (shifts.iter().min(), shifts.iter().max()) else {
        return Err("the equation has no term of the sequence".to_string());
    };
    if low == high {
        return Err("the equation has one term of the sequence only".to_string());
    }
    let names: Vec<String> = (low..=high).map(marker).collect();
    let free = |e: &Expr| names.iter().all(|m| !e.has_sym(m));
    let c: Vec<Expr> = names
        .iter()
        .map(|m| simplify(&crate::diff::diff(f, m)))
        .collect();
    let g0 = simplify(&crate::canon::neg(
        names
            .iter()
            .fold(f.clone(), |acc, m| subst_sym(&acc, m, &Expr::zero())),
    ));
    let rebuilt = add(c
        .iter()
        .zip(&names)
        .map(|(k, m)| mul(vec![k.clone(), Expr::sym(m)]))
        .collect());
    if !c.iter().all(free)
        || !free(&g0)
        || !is_zero(&sub(sub(f.clone(), rebuilt), crate::canon::neg(g0.clone())))
    {
        return Err("only linear recurrences are supported".to_string());
    }
    if c.iter().any(|k| k.has_sym(n)) {
        return Err("only recurrences with constant coefficients are supported".to_string());
    }
    let ns = Expr::sym(n);
    // sum_j c_j a(n + j) = g(n) after moving the lowest shift to 0
    let g = simplify(&subst_sym(&g0, n, &sub(ns.clone(), Expr::int(low))));
    let r = Expr::sym(ROOT);
    let p = add(c
        .iter()
        .enumerate()
        .map(|(j, k)| mul(vec![k.clone(), powi(r.clone(), j as i64)]))
        .collect());
    let solved = solve::solve(&p, ROOT)?;
    solve::release(&solved.params);
    let order = c.len() - 1;
    if solved.roots.iter().map(|r| r.mult as usize).sum::<usize>() != order {
        return Err("the characteristic equation was not solved completely".to_string());
    }
    let mut constants = Vec::new();
    let mut total = Vec::new();
    for root in &solved.roots {
        for j in 0..root.mult {
            let name = format!("C{}", constants.len() + 1);
            total.push(mul(vec![
                Expr::sym(&name),
                powi(ns.clone(), i64::from(j)),
                pow(root.value.clone(), ns.clone()),
            ]));
            constants.push(name);
        }
    }
    if !is_zero(&g) {
        total.push(
            particular(&c, &g, n, &solved.roots)
                .ok_or_else(|| format!("no particular solution found for the right side {g}"))?,
        );
    }
    let mut sol = add(total);
    let at = |s: &Expr, m: &Expr| simplify(&subst_sym(s, n, m));
    for m in 0..4i64 {
        let left = add(c
            .iter()
            .enumerate()
            .map(|(j, k)| mul(vec![k.clone(), at(&sol, &Expr::int(m + j as i64))]))
            .collect());
        if !is_zero(&sub(left, at(&g, &Expr::int(m)))) {
            return Err("the candidate solution does not satisfy the recurrence".to_string());
        }
    }
    if !conditions.is_empty() {
        let eqs: Vec<Expr> = conditions
            .iter()
            .map(|(m, v)| sub(at(&sol, m), v.clone()))
            .collect();
        let vars: Vec<String> = constants.iter().take(eqs.len()).cloned().collect();
        let solved = system::solve_system(&eqs, &vars)?;
        let values = solved
            .solutions
            .first()
            .ok_or("the conditions cannot be met")?;
        sol = simplify(
            &values
                .iter()
                .fold(sol, |acc, (name, value)| subst_sym(&acc, name, value)),
        );
    }
    Ok(sol)
}
