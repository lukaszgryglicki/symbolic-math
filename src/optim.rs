//! Optimisation with exact numbers: linear programs by the simplex method on fractions, the
//! same with whole unknowns by branching, and least squares with unknowns that are not negative.

use crate::arrays::{mldivide, order, tidy};
use crate::canon::{add, mul, sub};
use crate::expr::Expr;
use crate::matrix::{self, Rows};
use crate::num::Rat;
use std::cmp::Ordering;

type Res = Result<Expr, String>;

#[derive(Clone)]
struct Row {
    coeffs: Vec<Rat>,
    equal: bool,
    rhs: Rat,
}

/// `minimize cost*x` with `rows` and `low[i] <= x[i] <= high[i]` (`None`: no bound).
#[derive(Clone)]
struct Program {
    cost: Vec<Rat>,
    rows: Vec<Row>,
    low: Vec<Option<Rat>>,
    high: Vec<Option<Rat>>,
}

enum Outcome {
    Best(Vec<Rat>),
    Empty,
    Unbounded,
}

fn number(e: &Expr, name: &str) -> Result<Rat, String> {
    e.as_rat()
        .cloned()
        .ok_or_else(|| format!("{name} needs whole numbers and fractions, {e} is none"))
}

fn numbers(e: &Expr, name: &str) -> Result<Vec<Rat>, String> {
    match e.as_list() {
        Some(items) => items.iter().map(|x| number(x, name)).collect(),
        None => Ok(vec![number(e, name)?]),
    }
}

fn bounds(e: Option<&Expr>, n: usize, name: &str) -> Result<Vec<Option<Rat>>, String> {
    let Some(e) = e else {
        return Ok(vec![None; n]);
    };
    let one = |x: &Expr| -> Result<Option<Rat>, String> {
        if x.is_inf() || x.is_neg_inf() {
            Ok(None)
        } else {
            number(x, name).map(Some)
        }
    };
    match e.as_list() {
        Some([]) => Ok(vec![None; n]),
        Some(items) if items.len() == n => items.iter().map(one).collect(),
        Some(items) => Err(format!("{name}: {} bounds for {n} unknowns", items.len())),
        None => Ok(vec![one(e)?; n]),
    }
}

fn constraints(
    m: Option<&Expr>,
    b: Option<&Expr>,
    n: usize,
    equal: bool,
    name: &str,
) -> Result<Vec<Row>, String> {
    let (Some(m), Some(b)) = (m, b) else {
        return Ok(Vec::new());
    };
    if m.as_list().is_some_and(<[Expr]>::is_empty) {
        return Ok(Vec::new());
    }
    let rows: Rows = match matrix::rows(m) {
        Some(r) => r,
        None => vec![
            m.as_list()
                .map(<[Expr]>::to_vec)
                .unwrap_or_else(|| vec![m.clone()]),
        ],
    };
    let rhs = numbers(b, name)?;
    if rows.len() != rhs.len() || rows.iter().any(|r| r.len() != n) {
        return Err(format!(
            "{name}: the matrix of the conditions has {} row(s) of {} for {} right side(s) and {n} unknown(s)",
            rows.len(),
            rows[0].len(),
            rhs.len()
        ));
    }
    rows.iter()
        .zip(rhs)
        .map(|(r, rhs)| {
            Ok(Row {
                coeffs: r
                    .iter()
                    .map(|x| number(x, name))
                    .collect::<Result<_, _>>()?,
                equal,
                rhs,
            })
        })
        .collect()
}

fn zero() -> Rat {
    Rat::new()
}

fn pivot(t: &mut [Vec<Rat>], basis: &mut [usize], row: usize, col: usize) {
    let p = t[row][col].clone();
    for x in t[row].iter_mut() {
        *x /= &p;
    }
    let lead = t[row].clone();
    for (i, other) in t.iter_mut().enumerate() {
        if i == row || other[col].cmp0() == Ordering::Equal {
            continue;
        }
        let f = other[col].clone();
        for (x, y) in other.iter_mut().zip(&lead) {
            *x -= Rat::from(&f * y);
        }
    }
    basis[row] = col;
}

/// The simplex steps for the costs `cost` over the first `width` columns, by Bland's rule
/// (the first column that helps comes in, the first row of the least ratio goes out).
/// `false`: the costs fall without end.
fn simplex(t: &mut [Vec<Rat>], basis: &mut [usize], cost: &[Rat], width: usize) -> bool {
    let last = t.first().map_or(0, |r| r.len() - 1);
    loop {
        let entering = (0..width).find(|j| {
            let mut d = cost[*j].clone();
            for (i, row) in t.iter().enumerate() {
                d -= Rat::from(&cost[basis[i]] * &row[*j]);
            }
            d.cmp0() == Ordering::Less
        });
        let Some(col) = entering else {
            return true;
        };
        let mut leaving: Option<(usize, Rat)> = None;
        for (i, row) in t.iter().enumerate() {
            if row[col].cmp0() != Ordering::Greater {
                continue;
            }
            let ratio = Rat::from(&row[last] / &row[col]);
            let better = match &leaving {
                None => true,
                Some((k, best)) => match ratio.cmp(best) {
                    Ordering::Less => true,
                    Ordering::Equal => basis[i] < basis[*k],
                    Ordering::Greater => false,
                },
            };
            if better {
                leaving = Some((i, ratio));
            }
        }
        match leaving {
            Some((row, _)) => pivot(t, basis, row, col),
            None => return false,
        }
    }
}

fn solve(p: &Program) -> Outcome {
    let n = p.cost.len();
    // every unknown as a constant and columns that are not negative
    let mut shift: Vec<Rat> = vec![zero(); n];
    let mut parts: Vec<Vec<(usize, i32)>> = vec![Vec::new(); n];
    let mut rows = p.rows.clone();
    let mut width = 0;
    let mut extra: Vec<(usize, Rat)> = Vec::new();
    for i in 0..n {
        match (&p.low[i], &p.high[i]) {
            (Some(lo), hi) => {
                shift[i] = lo.clone();
                parts[i].push((width, 1));
                if let Some(hi) = hi {
                    if hi < lo {
                        return Outcome::Empty;
                    }
                    extra.push((width, Rat::from(hi - lo)));
                }
                width += 1;
            }
            (None, Some(hi)) => {
                shift[i] = hi.clone();
                parts[i].push((width, -1));
                width += 1;
            }
            (None, None) => {
                parts[i].push((width, 1));
                parts[i].push((width + 1, -1));
                width += 2;
            }
        }
    }
    let spread = |coeffs: &[Rat]| -> (Vec<Rat>, Rat) {
        let mut out = vec![zero(); width];
        let mut moved = zero();
        for (i, c) in coeffs.iter().enumerate() {
            moved += Rat::from(c * &shift[i]);
            for (col, sign) in &parts[i] {
                if *sign > 0 {
                    out[*col] += c;
                } else {
                    out[*col] -= c;
                }
            }
        }
        (out, moved)
    };
    let mut table: Vec<(Vec<Rat>, bool, Rat)> = Vec::new();
    for r in rows.drain(..) {
        let (coeffs, moved) = spread(&r.coeffs);
        table.push((coeffs, r.equal, r.rhs - moved));
    }
    for (col, most) in extra {
        let mut coeffs = vec![zero(); width];
        coeffs[col] = Rat::from(1);
        table.push((coeffs, false, most));
    }
    let (cost, _) = spread(&p.cost);
    // slack columns, then the columns that only start the search
    let slacks = table.iter().filter(|r| !r.1).count();
    let total = width + slacks + table.len();
    let mut t: Vec<Vec<Rat>> = Vec::new();
    let mut basis: Vec<usize> = Vec::new();
    let mut slack = width;
    for (k, (coeffs, equal, rhs)) in table.iter().enumerate() {
        let mut row = vec![zero(); total + 1];
        row[..width].clone_from_slice(coeffs);
        row[total] = rhs.clone();
        let mut own = None;
        if !*equal {
            row[slack] = Rat::from(1);
            own = Some(slack);
            slack += 1;
        }
        if row[total].cmp0() == Ordering::Less {
            for x in row.iter_mut() {
                *x = Rat::from(-&*x);
            }
            own = None;
        }
        match own {
            Some(col) => basis.push(col),
            None => {
                row[width + slacks + k] = Rat::from(1);
                basis.push(width + slacks + k);
            }
        }
        t.push(row);
    }
    let real = width + slacks;
    let mut start = vec![zero(); total];
    for c in start.iter_mut().skip(real) {
        *c = Rat::from(1);
    }
    simplex(&mut t, &mut basis, &start, total);
    if t.iter()
        .zip(&basis)
        .any(|(row, b)| *b >= real && row[total].cmp0() != Ordering::Equal)
    {
        return Outcome::Empty;
    }
    // starting columns that stayed at zero leave, or their rows say nothing
    let mut k = 0;
    while k < t.len() {
        if basis[k] >= real {
            match (0..real).find(|j| t[k][*j].cmp0() != Ordering::Equal) {
                Some(col) => pivot(&mut t, &mut basis, k, col),
                None => {
                    t.remove(k);
                    basis.remove(k);
                    continue;
                }
            }
        }
        k += 1;
    }
    for row in t.iter_mut() {
        let rhs = row[total].clone();
        row.truncate(real);
        row.push(rhs);
    }
    let mut costs = cost;
    costs.resize(real, zero());
    if !simplex(&mut t, &mut basis, &costs, real) {
        return Outcome::Unbounded;
    }
    let mut y = vec![zero(); real];
    for (row, b) in t.iter().zip(&basis) {
        y[*b] = row[real].clone();
    }
    Outcome::Best(
        (0..n)
            .map(|i| {
                let mut x = shift[i].clone();
                for (col, sign) in &parts[i] {
                    if *sign > 0 {
                        x += &y[*col];
                    } else {
                        x -= &y[*col];
                    }
                }
                x
            })
            .collect(),
    )
}

fn value(cost: &[Rat], x: &[Rat]) -> Rat {
    let mut v = zero();
    for (c, x) in cost.iter().zip(x) {
        v += Rat::from(c * x);
    }
    v
}

/// The best point with whole numbers at the places `whole`: the program is split at a
/// fraction into the part below and the part above it.
fn branch(
    p: &Program,
    whole: &[usize],
    best: &mut Option<(Rat, Vec<Rat>)>,
    work: &mut u32,
) -> Result<(), String> {
    *work += 1;
    if *work > 20_000 {
        return Err("intlinprog: too many cases".into());
    }
    let x = match solve(p) {
        Outcome::Best(x) => x,
        Outcome::Empty => return Ok(()),
        Outcome::Unbounded => return Err("intlinprog: the costs fall without end".into()),
    };
    let here = value(&p.cost, &x);
    if best.as_ref().is_some_and(|(v, _)| here >= *v) {
        return Ok(());
    }
    let Some(at) = whole.iter().copied().find(|i| !x[*i].is_integer()) else {
        *best = Some((here, x));
        return Ok(());
    };
    let floor = Rat::from(x[at].clone().floor());
    let mut below = p.clone();
    below.high[at] = Some(match &p.high[at] {
        Some(h) if *h < floor => h.clone(),
        _ => floor.clone(),
    });
    branch(&below, whole, best, work)?;
    let ceil = floor + Rat::from(1);
    let mut above = p.clone();
    above.low[at] = Some(match &p.low[at] {
        Some(l) if *l > ceil => l.clone(),
        _ => ceil,
    });
    branch(&above, whole, best, work)
}

fn program(a: &[Expr], name: &str) -> Result<Program, String> {
    let cost = numbers(&a[0], name)?;
    let n = cost.len();
    let mut rows = constraints(a.get(1), a.get(2), n, false, name)?;
    rows.extend(constraints(a.get(3), a.get(4), n, true, name)?);
    Ok(Program {
        cost,
        rows,
        low: bounds(a.get(5), n, name)?,
        high: bounds(a.get(6), n, name)?,
    })
}

fn answer(x: Vec<Rat>) -> Expr {
    Expr::list(x.into_iter().map(Expr::rational).collect())
}

/// Least squares `min |C*x - d|` with no entry of `x` negative (Lawson and Hanson).
fn nonnegative(c: &Rows, d: &[Expr]) -> Res {
    let n = c[0].len();
    let columns = matrix::transpose(c);
    let mut inside: Vec<usize> = Vec::new();
    let mut x = vec![Expr::zero(); n];
    let residual = |x: &[Expr]| -> Vec<Expr> {
        c.iter()
            .zip(d)
            .map(|(row, target)| {
                let got = add(row
                    .iter()
                    .zip(x)
                    .map(|(p, q)| mul(vec![p.clone(), q.clone()]))
                    .collect());
                tidy(sub(target.clone(), got))
            })
            .collect()
    };
    let positive = |e: &Expr| order(e, &Expr::zero()).map(|o| o == Ordering::Greater);
    for _ in 0..10 * n + 10 {
        let r = residual(&x);
        // the unknown outside that would help most
        let mut best: Option<(usize, Expr)> = None;
        for (j, col) in columns.iter().enumerate() {
            if inside.contains(&j) {
                continue;
            }
            let w = tidy(add(col
                .iter()
                .zip(&r)
                .map(|(p, q)| mul(vec![p.clone(), q.clone()]))
                .collect()));
            if positive(&w)?
                && best
                    .as_ref()
                    .is_none_or(|(_, b)| order(&w, b).is_ok_and(|o| o == Ordering::Greater))
            {
                best = Some((j, w));
            }
        }
        let Some((j, _)) = best else {
            return Ok(Expr::list(x));
        };
        inside.push(j);
        loop {
            let part: Rows = c
                .iter()
                .map(|row| inside.iter().map(|k| row[*k].clone()).collect())
                .collect();
            let fit = mldivide(&matrix::from_rows(part), &Expr::list(d.to_vec()))?;
            let z = fit.as_list().ok_or("lsqnonneg: no fit")?.to_vec();
            if z.iter().all(|v| positive(v).unwrap_or(false)) {
                x = vec![Expr::zero(); n];
                for (k, v) in inside.iter().zip(z) {
                    x[*k] = v;
                }
                break;
            }
            // back along the line to the first unknown that reaches zero
            let mut step: Option<Expr> = None;
            for (k, v) in inside.iter().zip(&z) {
                if !positive(v)? {
                    let ratio = tidy(crate::canon::div(
                        x[*k].clone(),
                        sub(x[*k].clone(), v.clone()),
                    ));
                    if step
                        .as_ref()
                        .is_none_or(|s| order(&ratio, s).is_ok_and(|o| o == Ordering::Less))
                    {
                        step = Some(ratio);
                    }
                }
            }
            let step = step.ok_or("lsqnonneg: no step")?;
            for (k, v) in inside.iter().zip(&z) {
                x[*k] = tidy(add(vec![
                    x[*k].clone(),
                    mul(vec![step.clone(), sub(v.clone(), x[*k].clone())]),
                ]));
            }
            inside.retain(|k| positive(&x[*k]).unwrap_or(false));
            for (k, v) in x.iter_mut().enumerate() {
                if !inside.contains(&k) {
                    *v = Expr::zero();
                }
            }
            if inside.is_empty() {
                break;
            }
        }
    }
    Err("lsqnonneg: the steps do not settle".into())
}

/// A solution of a system of linear equations with fractions (the free unknowns zero), none
/// when the equations contradict each other.
fn linear(mut m: Vec<Vec<Rat>>, mut b: Vec<Rat>) -> Option<Vec<Rat>> {
    let (rows, columns) = (m.len(), m.first().map_or(0, Vec::len));
    let mut pivots = Vec::new();
    let mut row = 0;
    for col in 0..columns {
        let Some(found) = (row..rows).find(|r| !m[*r][col].is_zero()) else {
            continue;
        };
        m.swap(row, found);
        b.swap(row, found);
        let lead = m[row][col].clone();
        for x in m[row].iter_mut() {
            *x /= &lead;
        }
        b[row] /= &lead;
        for r in 0..rows {
            if r == row || m[r][col].is_zero() {
                continue;
            }
            let factor = m[r][col].clone();
            let (pivot_row, pivot_b) = (m[row].clone(), b[row].clone());
            for (x, p) in m[r].iter_mut().zip(&pivot_row) {
                *x -= Rat::from(&factor * p);
            }
            b[r] -= factor * pivot_b;
        }
        pivots.push(col);
        row += 1;
        if row == rows {
            break;
        }
    }
    if b[row..].iter().any(|x| !x.is_zero()) {
        return None;
    }
    let mut x = vec![zero(); columns];
    for (r, col) in pivots.iter().enumerate() {
        x[*col] = b[r].clone();
    }
    Some(x)
}

/// Whether a symmetric matrix has no negative eigenvalue: no negative pivot, and nothing in
/// the row of a pivot that is zero.
fn positive_semidefinite(mut h: Vec<Vec<Rat>>) -> bool {
    let n = h.len();
    for k in 0..n {
        match h[k][k].cmp0() {
            Ordering::Less => return false,
            Ordering::Equal => {
                if h[k][k..].iter().any(|x| !x.is_zero()) {
                    return false;
                }
            }
            Ordering::Greater => {
                for i in k + 1..n {
                    let factor = Rat::from(&h[i][k] / &h[k][k]);
                    let pivot_row = h[k].clone();
                    for (x, p) in h[i].iter_mut().zip(&pivot_row) {
                        *x -= Rat::from(&factor * p);
                    }
                }
            }
        }
    }
    true
}

/// The least of `x'*H*x/2 + f*x` with the conditions of a linear program, for an `H` without
/// negative eigenvalues: the first choice of conditions that hold with equality whose
/// stationary point meets all conditions with multipliers that are not negative.
fn quadratic(h: &Rows, p: &Program, name: &str) -> Res {
    let n = p.cost.len();
    if h.len() != n || h.iter().any(|r| r.len() != n) {
        return Err(format!(
            "{name}: H has {n} rows and columns, as many as there are unknowns"
        ));
    }
    let entries: Vec<Vec<Rat>> = h
        .iter()
        .map(|r| r.iter().map(|x| number(x, name)).collect::<Result<_, _>>())
        .collect::<Result<_, _>>()?;
    let h: Vec<Vec<Rat>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| Rat::from(&entries[i][j] + &entries[j][i]) / 2)
                .collect()
        })
        .collect();
    if !positive_semidefinite(h.clone()) {
        return Err(format!(
            "{name}: H has a negative eigenvalue, so the problem is not convex and has no least value that this search finds"
        ));
    }
    let equal: Vec<&Row> = p.rows.iter().filter(|r| r.equal).collect();
    let mut limits: Vec<Row> = p.rows.iter().filter(|r| !r.equal).cloned().collect();
    for k in 0..n {
        let unit = |sign: i32| -> Vec<Rat> {
            (0..n)
                .map(|j| Rat::from(if j == k { sign } else { 0 }))
                .collect()
        };
        if let Some(low) = &p.low[k] {
            limits.push(Row {
                coeffs: unit(-1),
                equal: false,
                rhs: Rat::from(-low),
            });
        }
        if let Some(high) = &p.high[k] {
            limits.push(Row {
                coeffs: unit(1),
                equal: false,
                rhs: high.clone(),
            });
        }
    }
    if limits.len() > 16 {
        return Err(format!(
            "{name}: {} conditions with <= are more than the 16 that the exact search takes",
            limits.len()
        ));
    }
    let dot = |c: &[Rat], x: &[Rat]| -> Rat {
        c.iter()
            .zip(x)
            .fold(zero(), |s, (p, q)| s + Rat::from(p * q))
    };
    for chosen in 0u32..1 << limits.len() {
        let tight: Vec<&Row> = limits
            .iter()
            .enumerate()
            .filter(|(k, _)| chosen >> k & 1 == 1)
            .map(|(_, r)| r)
            .collect();
        let held: Vec<&Row> = equal.iter().copied().chain(tight.iter().copied()).collect();
        let size = n + held.len();
        let mut system = vec![vec![zero(); size]; size];
        let mut right = vec![zero(); size];
        for i in 0..n {
            system[i][..n].clone_from_slice(&h[i]);
            right[i] = Rat::from(-&p.cost[i]);
            for (k, row) in held.iter().enumerate() {
                system[i][n + k] = row.coeffs[i].clone();
                system[n + k][i] = row.coeffs[i].clone();
            }
        }
        for (k, row) in held.iter().enumerate() {
            right[n + k] = row.rhs.clone();
        }
        let Some(found) = linear(system, right) else {
            continue;
        };
        let (x, multipliers) = found.split_at(n);
        let fits = limits.iter().all(|r| dot(&r.coeffs, x) <= r.rhs)
            && multipliers[equal.len()..]
                .iter()
                .all(|m| m.cmp0() != Ordering::Less);
        if fits {
            return Ok(answer(x.to_vec()));
        }
    }
    Err(format!(
        "{name}: no point meets all conditions, or the value falls without end"
    ))
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    Ok(Some(match (name, n) {
        ("quadprog", 2 | 4 | 6 | 7 | 8) => {
            let h = matrix::rows(&a[0])
                .ok_or("quadprog(H, f, A, b, Aeq, beq, lb, ub): H is a matrix")?;
            quadratic(&h, &program(&a[1..], name)?, name)?
        }
        ("lsqlin", 2 | 4 | 6 | 7 | 8) => {
            // the squares of C*x - d are x'*(C'*C)*x - 2*(C'*d)*x and a number
            let c =
                matrix::rows(&a[0]).ok_or("lsqlin(C, d, A, b, Aeq, beq, lb, ub): C is a matrix")?;
            let d = a[1]
                .as_list()
                .filter(|d| d.len() == c.len())
                .ok_or("lsqlin(C, d): d has an entry for every row of C")?;
            let ct = matrix::transpose(&c);
            let h: Rows = matrix::matmul(&ct, &c)?
                .into_iter()
                .map(|r| {
                    r.into_iter()
                        .map(|x| tidy(mul(vec![Expr::int(2), x])))
                        .collect()
                })
                .collect();
            let f: Vec<Expr> = ct
                .iter()
                .map(|column| {
                    tidy(mul(vec![
                        Expr::int(-2),
                        add(column
                            .iter()
                            .zip(d)
                            .map(|(p, q)| mul(vec![p.clone(), q.clone()]))
                            .collect()),
                    ]))
                })
                .collect();
            let mut rest = vec![Expr::list(f)];
            rest.extend(a[2..].iter().cloned());
            quadratic(&h, &program(&rest, name)?, name)?
        }
        ("linprog", 3 | 5 | 6 | 7) => match solve(&program(a, name)?) {
            Outcome::Best(x) => answer(x),
            Outcome::Empty => return Err("linprog: no point meets all conditions".into()),
            Outcome::Unbounded => return Err("linprog: the costs fall without end".into()),
        },
        ("intlinprog", 4 | 6 | 7 | 8) => {
            let mut rest = vec![a[0].clone()];
            rest.extend(a[2..].iter().cloned());
            let p = program(&rest, name)?;
            let count = p.cost.len() as i64;
            let whole: Vec<usize> = match a[1].as_list() {
                Some(items) => items.to_vec(),
                None => vec![a[1].clone()],
            }
            .iter()
            .map(|k| {
                k.as_i64()
                    .filter(|k| (1..=count).contains(k))
                    .map(|k| k as usize - 1)
                    .ok_or_else(|| format!("intlinprog: {k} is no number of an unknown"))
            })
            .collect::<Result<_, _>>()?;
            let (mut best, mut work) = (None, 0);
            branch(&p, &whole, &mut best, &mut work)?;
            match best {
                Some((_, x)) => answer(x),
                None => return Err("intlinprog: no point meets all conditions".into()),
            }
        }
        ("lsqnonneg", 2) => {
            let c = matrix::rows(&a[0]).ok_or("lsqnonneg(C, d): C is a matrix")?;
            let d = a[1]
                .as_list()
                .filter(|d| d.len() == c.len())
                .ok_or("lsqnonneg(C, d): d has an entry for every row of C")?;
            nonnegative(&c, d)?
        }
        _ => return Ok(None),
    }))
}

pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    inner(name, a).transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rug::Integer;

    fn r(p: i64, q: i64) -> Rat {
        Rat::from((Integer::from(p), Integer::from(q)))
    }

    #[test]
    fn a_small_program_reaches_its_corner() {
        // max x + 2y with x + y <= 4, x + 3y <= 6, x, y >= 0: the corner (3, 1)
        let p = Program {
            cost: vec![r(-1, 1), r(-2, 1)],
            rows: vec![
                Row {
                    coeffs: vec![r(1, 1), r(1, 1)],
                    equal: false,
                    rhs: r(4, 1),
                },
                Row {
                    coeffs: vec![r(1, 1), r(3, 1)],
                    equal: false,
                    rhs: r(6, 1),
                },
            ],
            low: vec![Some(zero()), Some(zero())],
            high: vec![None, None],
        };
        match solve(&p) {
            Outcome::Best(x) => assert_eq!(x, vec![r(3, 1), r(1, 1)]),
            _ => panic!("no corner"),
        }
    }
}
