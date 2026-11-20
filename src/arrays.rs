//! Arrays the way MATLAB has them: a flat list is a row, a list of rows is a matrix. Building,
//! turning, cutting and asking about them, the operators that work entry by entry, and the
//! matrices with a name.

use crate::canon::{add, div, mul, neg, pow, sub};
use crate::expr::{Expr, Rel};
use crate::funcs::func;
use crate::matrix::{self, Rows};
use crate::numeric;
use crate::simplify::simplify;
use rug::Integer;
use std::cmp::Ordering;

type Res = Result<Expr, String>;

/// Most entries of an array that a command builds.
pub const LIMIT: usize = 1_000_000;

pub enum Arr {
    Scalar(Expr),
    Vector(Vec<Expr>),
    Matrix(Rows),
}

pub fn arr(e: &Expr) -> Arr {
    match e {
        Expr::List(items) => match matrix::rows(e) {
            Some(m) => Arr::Matrix(m),
            None => Arr::Vector(items.to_vec()),
        },
        other => Arr::Scalar(other.clone()),
    }
}

/// The rows of anything, and whether it was written as rows: a number is one row with one
/// entry, a flat list is one row.
pub fn grid(e: &Expr) -> (Rows, bool) {
    match arr(e) {
        Arr::Scalar(x) => (vec![vec![x]], false),
        Arr::Vector(v) => (vec![v], false),
        Arr::Matrix(m) => (m, true),
    }
}

/// Rows as a value: a single row that did not come from rows is a flat list.
pub fn ungrid(m: Rows, nested: bool) -> Expr {
    if m.len() == 1 && !nested {
        Expr::list(m.into_iter().next().unwrap_or_default())
    } else {
        matrix::from_rows(m)
    }
}

/// A word given as text or as a bare name, in small letters.
pub fn word(e: &Expr) -> Option<String> {
    crate::radix::text_of(e)
        .or_else(|| e.sym_name())
        .map(str::to_ascii_lowercase)
}

pub fn whole(e: &Expr, what: &str, lo: i64, hi: i64) -> Result<i64, String> {
    e.as_i64()
        .filter(|k| (lo..=hi).contains(k))
        .ok_or_else(|| format!("{what} must be a whole number from {lo} to {hi}, not {e}"))
}

/// The order of two real numbers.
pub fn order(a: &Expr, b: &Expr) -> Result<Ordering, String> {
    if let (Some(x), Some(y)) = (a.as_rat(), b.as_rat()) {
        return Ok(x.cmp(y));
    }
    if a == b {
        return Ok(Ordering::Equal);
    }
    let end = |e: &Expr| i32::from(e.is_inf()) - i32::from(e.is_neg_inf());
    if end(a) != 0 || end(b) != 0 {
        return Ok(end(a).cmp(&end(b)));
    }
    match numeric::sign(&sub(a.clone(), b.clone())) {
        Some(s) => Ok(s.cmp(&0)),
        None => Err(format!(
            "{a} and {b} cannot be put in order: real numbers are needed"
        )),
    }
}

/// The order of two entries of any kind: numbers by value, the rest as the canonical form
/// has them.
pub fn order_any(a: &Expr, b: &Expr) -> Ordering {
    order(a, b).unwrap_or_else(|_| a.cmp(b))
}

/// Equal as values: the same form, or constants with the difference zero.
pub fn same(a: &Expr, b: &Expr) -> bool {
    if a == b {
        return true;
    }
    match (a, b) {
        (Expr::List(x), Expr::List(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| same(p, q))
        }
        (Expr::List(_), _) | (_, Expr::List(_)) => false,
        _ => {
            a.is_const()
                && b.is_const()
                && numeric::is_zero_const(&sub(a.clone(), b.clone())) == Some(true)
        }
    }
}

/// Simplified unless it is a plain number.
pub fn tidy(e: Expr) -> Expr {
    if e.is_number() { e } else { simplify(&e) }
}

/// A condition written for whole lists, as the list of its conditions.
pub fn threaded(e: &Expr) -> Expr {
    match e {
        Expr::Rel(op, l, r) if l.as_list().is_some() || r.as_list().is_some() => {
            matrix::zip(l, r, &|x, y| Expr::rel(*op, x.clone(), y.clone()))
                .unwrap_or_else(|_| e.clone())
        }
        Expr::Fn(name, args) if matches!(&**name, "and" | "or" | "xor" | "not") => {
            let parts: Vec<Expr> = args.iter().map(threaded).collect();
            if !parts.iter().any(|p| p.as_list().is_some()) {
                return e.clone();
            }
            let mut out = parts[0].clone();
            if parts.len() == 1 {
                return map(&out, &|x| func(name, vec![x.clone()]));
            }
            for p in &parts[1..] {
                out = match matrix::zip(&out, p, &|x, y| func(name, vec![x.clone(), y.clone()])) {
                    Ok(v) => v,
                    Err(_) => return e.clone(),
                };
            }
            out
        }
        _ => e.clone(),
    }
}

pub fn map(e: &Expr, f: &dyn Fn(&Expr) -> Expr) -> Expr {
    match e {
        Expr::List(a) => Expr::list(a.iter().map(|x| map(x, f)).collect()),
        other => f(other),
    }
}

/// The same with a function that may fail.
pub fn map_result(e: &Expr, f: &dyn Fn(&Expr) -> Res) -> Res {
    match e {
        Expr::List(a) => Ok(Expr::list(
            a.iter()
                .map(|x| map_result(x, f))
                .collect::<Result<_, _>>()?,
        )),
        other => f(other),
    }
}

/// Whether an entry counts as true: a condition that holds, a number other than zero.
pub fn truthy(x: &Expr) -> Result<bool, String> {
    if let Expr::Bool(b) = x {
        return Ok(*b);
    }
    if let Some(t) = crate::logic::truth(x) {
        return Ok(t);
    }
    if matches!(x, Expr::Rel(Rel::Eq | Rel::Ne, ..)) {
        if let Expr::Rel(op, l, r) = x
            && l.is_const()
            && r.is_const()
        {
            return Ok(same(l, r) == (*op == Rel::Eq));
        }
        return Err(format!("{x} is neither true nor false"));
    }
    if x.is_zero() {
        return Ok(false);
    }
    if x.is_const() && !matches!(x, Expr::Rel(..) | Expr::List(_)) {
        return numeric::is_zero_const(x)
            .map(|z| !z)
            .ok_or_else(|| format!("whether {x} is zero cannot be told"));
    }
    Err(format!("{x} is neither true nor false"))
}

/// The entries as 1 for true and 0 for false, where a condition was written for lists.
pub fn counted(e: &Expr) -> Option<Expr> {
    let spread = threaded(e);
    let bools = |x: &Expr| matches!(x, Expr::Bool(_) | Expr::Rel(..));
    let mut any = false;
    spread.walk(&mut |x| any |= bools(x));
    if !any || spread.as_list().is_none() {
        return None;
    }
    let mut ok = true;
    let out = map(&spread, &|x| {
        if bools(x) || matches!(x, Expr::Fn(n, _) if matches!(&**n, "and" | "or" | "xor" | "not")) {
            match truthy(x) {
                Ok(t) => Expr::int(i64::from(t)),
                Err(_) => x.clone(),
            }
        } else {
            x.clone()
        }
    });
    out.walk(&mut |x| ok &= !bools(x));
    ok.then_some(out)
}

fn elementwise(name: &str, a: &Expr, b: &Expr) -> Res {
    let f = |x: &Expr, y: &Expr| match name {
        "times" => mul(vec![x.clone(), y.clone()]),
        "rdivide" => div(x.clone(), y.clone()),
        "ldivide" => div(y.clone(), x.clone()),
        _ => pow(x.clone(), y.clone()),
    };
    matrix::zip(a, b, &f)
}

pub fn ctranspose(m: &Rows) -> Rows {
    matrix::transpose(m)
        .into_iter()
        .map(|r| r.iter().map(|x| func("conj", vec![x.clone()])).collect())
        .collect()
}

fn product(a: &Rows, b: &Rows) -> Result<Rows, String> {
    Ok(matrix::matmul(a, b)?
        .into_iter()
        .map(|r| r.into_iter().map(tidy).collect())
        .collect())
}

/// `A\b`: the solution of `A*x = b`; with more equations than unknowns the one of least squares,
/// with fewer the shortest one.
pub fn mldivide(a: &Expr, b: &Expr) -> Res {
    let Some(m) = matrix::rows(a) else {
        if a.as_list().is_some() {
            return Err("A\\b: A must be a matrix or a number".into());
        }
        return matrix::mul(vec![crate::canon::inv(a.clone()), b.clone()]);
    };
    let (rhs, flat) = match arr(b) {
        Arr::Vector(v) => (v.into_iter().map(|x| vec![x]).collect::<Rows>(), true),
        Arr::Matrix(r) => (r, false),
        Arr::Scalar(_) => return Err("A\\b: b must be a vector or a matrix".into()),
    };
    if rhs.len() != m.len() {
        return Err(format!(
            "A\\b: A has {} row(s) and b has {}",
            m.len(),
            rhs.len()
        ));
    }
    let (r, c) = (m.len(), m[0].len());
    let rank = matrix::eliminate(m.clone(), false).1;
    let x = if r == c && rank == c {
        product(&matrix::inverse(&m)?, &rhs)?
    } else if rank == c {
        let h = ctranspose(&m);
        product(&matrix::inverse(&product(&h, &m)?)?, &product(&h, &rhs)?)?
    } else if rank == r {
        let h = ctranspose(&m);
        product(&h, &product(&matrix::inverse(&product(&m, &h)?)?, &rhs)?)?
    } else {
        return Err(format!(
            "A\\b: the rank of A is {rank}, less than its {r} row(s) and {c} column(s): pinv(A)*b gives the shortest solution of least squares"
        ));
    };
    Ok(if flat {
        Expr::list(x.into_iter().map(|mut row| row.remove(0)).collect())
    } else {
        matrix::from_rows(x)
    })
}

fn mrdivide(a: &Expr, b: &Expr) -> Res {
    let Some(m) = matrix::rows(b) else {
        if b.as_list().is_some() {
            return Err("A/B: B must be a matrix or a number".into());
        }
        return matrix::mul(vec![a.clone(), crate::canon::inv(b.clone())]);
    };
    let (left, nested) = grid(a);
    let turned = mldivide(
        &matrix::from_rows(matrix::transpose(&m)),
        &matrix::from_rows(matrix::transpose(&left)),
    )?;
    let back = matrix::rows(&turned).ok_or("A/B: the sizes do not fit")?;
    Ok(ungrid(matrix::transpose(&back), nested))
}

/// The sizes as MATLAB counts them: a number is 1 by 1, a flat list 1 by n.
pub fn size(e: &Expr) -> Vec<usize> {
    fn dims(e: &Expr) -> Option<Vec<usize>> {
        let items = e.as_list()?;
        let inner: Vec<Option<Vec<usize>>> = items.iter().map(dims).collect();
        let mut out = vec![items.len()];
        if let Some(Some(first)) = inner.first()
            && inner.iter().all(|d| d.as_ref() == Some(first))
        {
            out.extend(first.iter().copied());
        }
        Some(out)
    }
    match dims(e) {
        None => vec![1, 1],
        Some(d) if d.len() == 1 => vec![1, d[0]],
        Some(d) => d,
    }
}

fn leaves(e: &Expr, out: &mut Vec<Expr>) {
    match e {
        Expr::List(items) => items.iter().for_each(|x| leaves(x, out)),
        other => out.push(other.clone()),
    }
}

/// The entries of a matrix column after column, those of a list in their order.
pub fn column_major(e: &Expr) -> Vec<Expr> {
    match arr(e) {
        Arr::Scalar(x) => vec![x],
        Arr::Vector(v) => {
            let mut out = Vec::new();
            v.iter().for_each(|x| leaves(x, &mut out));
            out
        }
        Arr::Matrix(m) => matrix::transpose(&m).concat(),
    }
}

fn span_of(a: &[Expr]) -> Result<(usize, usize), String> {
    let n = |e: &Expr| whole(e, "the size", 0, LIMIT as i64).map(|k| k as usize);
    let (r, c) = match a {
        [one] => match one.as_list() {
            Some([r, c]) => (n(r)?, n(c)?),
            Some(_) => return Err("the sizes are [rows, columns]".into()),
            None => (n(one)?, n(one)?),
        },
        [r, c] => (n(r)?, n(c)?),
        _ => return Err("the sizes are rows, columns".into()),
    };
    if r.saturating_mul(c) > LIMIT {
        return Err(format!("an array of more than {LIMIT} entries"));
    }
    Ok((r, c))
}

fn cat(parts: &[Expr], across: bool, name: &str) -> Res {
    let mut out: Rows = Vec::new();
    let mut nested = false;
    for p in parts {
        if p.as_list().is_some_and(<[Expr]>::is_empty) {
            continue;
        }
        let (g, n) = grid(p);
        nested |= n;
        if out.is_empty() {
            out = g;
        } else if across {
            if g.len() != out.len() {
                return Err(format!(
                    "{name}: {} row(s) next to {} row(s)",
                    out.len(),
                    g.len()
                ));
            }
            for (row, more) in out.iter_mut().zip(g) {
                row.extend(more);
            }
        } else {
            if g[0].len() != out[0].len() {
                return Err(format!(
                    "{name}: {} column(s) above {} column(s)",
                    out[0].len(),
                    g[0].len()
                ));
            }
            out.extend(g);
        }
    }
    if out.is_empty() {
        return Ok(Expr::list(Vec::new()));
    }
    Ok(ungrid(out, nested))
}

fn tile(e: &Expr, r: usize, c: usize) -> Res {
    let (g, nested) = grid(e);
    if g.len()
        .saturating_mul(r)
        .saturating_mul(g[0].len())
        .saturating_mul(c)
        > LIMIT
    {
        return Err(format!("an array of more than {LIMIT} entries"));
    }
    let mut out: Rows = Vec::new();
    for _ in 0..r {
        for row in &g {
            let mut wide = Vec::with_capacity(row.len() * c);
            for _ in 0..c {
                wide.extend(row.iter().cloned());
            }
            out.push(wide);
        }
    }
    if out.is_empty() || out[0].is_empty() {
        return Ok(Expr::list(Vec::new()));
    }
    Ok(ungrid(out, nested))
}

fn rot90(m: &Rows, k: i64) -> Rows {
    let mut out = m.clone();
    for _ in 0..k.rem_euclid(4) {
        let (r, c) = (out.len(), out[0].len());
        out = (0..c)
            .map(|i| (0..r).map(|j| out[j][c - 1 - i].clone()).collect())
            .collect();
    }
    out
}

fn shifted<T: Clone>(v: &[T], k: i64) -> Vec<T> {
    let n = v.len() as i64;
    if n == 0 {
        return Vec::new();
    }
    (0..n)
        .map(|i| v[(i - k).rem_euclid(n) as usize].clone())
        .collect()
}

fn magic(n: usize) -> Rows {
    let cell = |v: usize| Expr::int(v as i64);
    if n == 2 {
        // (no square of this size is magic: MATLAB gives this one)
        return vec![vec![cell(4), cell(3)], vec![cell(1), cell(2)]];
    }
    if n % 2 == 1 {
        return (1..=n)
            .map(|i| {
                (1..=n)
                    .map(|j| {
                        let a = (i + j + n - n.div_ceil(2) - 1) % n;
                        let b = (i + 2 * j + n - 2) % n;
                        cell(n * a + b + 1)
                    })
                    .collect()
            })
            .collect();
    }
    if n.is_multiple_of(4) {
        let band = |k: usize| (k % 4) / 2;
        return (1..=n)
            .map(|i| {
                (1..=n)
                    .map(|j| {
                        let v = (i - 1) * n + j;
                        cell(if band(i) == band(j) { n * n + 1 - v } else { v })
                    })
                    .collect()
            })
            .collect();
    }
    let p = n / 2;
    let small: Vec<Vec<usize>> = magic(p)
        .iter()
        .map(|r| r.iter().map(|x| x.as_i64().unwrap_or(0) as usize).collect())
        .collect();
    let mut m = vec![vec![0usize; n]; n];
    for i in 0..p {
        for j in 0..p {
            m[i][j] = small[i][j];
            m[i][j + p] = small[i][j] + 2 * p * p;
            m[i + p][j] = small[i][j] + 3 * p * p;
            m[i + p][j + p] = small[i][j] + p * p;
        }
    }
    if n > 2 {
        let k = (n - 2) / 4;
        let mut cols: Vec<usize> = (0..k).collect();
        cols.extend(n - k + 1..n);
        for j in cols {
            for i in 0..p {
                let (top, low) = (m[i][j], m[i + p][j]);
                m[i][j] = low;
                m[i + p][j] = top;
            }
        }
        for j in [0, k] {
            let (top, low) = (m[k][j], m[k + p][j]);
            m[k][j] = low;
            m[k + p][j] = top;
        }
    }
    m.into_iter()
        .map(|r| r.into_iter().map(cell).collect())
        .collect()
}

fn binomial(n: u32, k: u32) -> Integer {
    Integer::from(n).binomial(k)
}

fn pascal(n: usize, kind: i64) -> Rows {
    let cell = |v: Integer| Expr::integer(v);
    if kind == 0 {
        return (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| cell(binomial((i + j) as u32, i as u32)))
                    .collect()
            })
            .collect();
    }
    let lower: Rows = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    let v = if j <= i {
                        binomial(i as u32, j as u32)
                    } else {
                        Integer::new()
                    };
                    cell(if j % 2 == 1 { -v } else { v })
                })
                .collect()
        })
        .collect();
    if kind == 1 {
        return lower;
    }
    let turned = rot90(&lower, 3);
    if n.is_multiple_of(2) {
        turned
            .into_iter()
            .map(|r| r.into_iter().map(neg).collect())
            .collect()
    } else {
        turned
    }
}

const ROSSER: [[i64; 8]; 8] = [
    [611, 196, -192, 407, -8, -52, -49, 29],
    [196, 899, 113, -192, -71, -43, -8, -44],
    [-192, 113, 899, 196, 61, 49, 8, 52],
    [407, -192, 196, 611, 8, 44, 59, -23],
    [-8, -71, 61, 8, 411, -599, 208, 208],
    [-52, -43, 49, 44, -599, 411, 208, 208],
    [-49, -8, 8, 59, 208, 208, 99, -911],
    [29, -44, 52, -23, 208, 208, -911, 99],
];

/// The Hadamard matrix of Sylvester for a power of two; for 12 and 20 times one the matrix of
/// Paley.
fn hadamard(n: usize) -> Option<Vec<Vec<i64>>> {
    if n == 1 {
        return Some(vec![vec![1]]);
    }
    if n.is_multiple_of(2)
        && let Some(h) = hadamard(n / 2)
    {
        let mut out = Vec::with_capacity(n);
        for sign in [1, -1] {
            for row in &h {
                let mut wide = row.clone();
                wide.extend(row.iter().map(|x| sign * x));
                out.push(wide);
            }
        }
        return Some(out);
    }
    let q = n.checked_sub(1).filter(|q| matches!(q, 11 | 19))?;
    let residue = |x: usize| (1..q).any(|y| (y * y) % q == x % q);
    let mut out = vec![vec![1i64; n]; n];
    for (i, row) in out.iter_mut().enumerate().skip(1) {
        for (j, entry) in row.iter_mut().enumerate().skip(1) {
            *entry = if i != j && residue((j + q - i) % q) {
                1
            } else {
                -1
            };
        }
    }
    Some(out)
}

fn band(m: &Rows) -> (usize, usize) {
    let (mut lower, mut upper) = (0, 0);
    for (i, row) in m.iter().enumerate() {
        for (j, x) in row.iter().enumerate() {
            if !x.is_zero() && !crate::simplify::is_zero(x) {
                if i > j {
                    lower = lower.max(i - j);
                } else {
                    upper = upper.max(j - i);
                }
            }
        }
    }
    (lower, upper)
}

fn mask(e: &Expr) -> Result<Vec<Vec<bool>>, String> {
    let (g, _) = grid(&threaded(e));
    g.iter().map(|r| r.iter().map(truthy).collect()).collect()
}

fn sort_rows(m: &Rows, keys: &[i64]) -> Rows {
    let mut out = m.clone();
    out.sort_by(|x, y| {
        for k in keys {
            let c = (k.unsigned_abs() as usize) - 1;
            let o = order_any(&x[c], &y[c]);
            let o = if *k < 0 { o.reverse() } else { o };
            if o != Ordering::Equal {
                return o;
            }
        }
        Ordering::Equal
    });
    out
}

fn unique_sorted(items: &[Expr]) -> Vec<Expr> {
    let mut out: Vec<Expr> = Vec::new();
    for x in crate::lists::sorted(items) {
        if !out.last().is_some_and(|l| same(l, &x)) {
            out.push(x);
        }
    }
    out
}

fn sorted_dir(items: &[Expr], descend: bool) -> Vec<Expr> {
    let mut out = crate::lists::sorted(items);
    if descend {
        out.reverse();
    }
    out
}

fn direction(e: &Expr) -> Option<bool> {
    match word(e)?.as_str() {
        "ascend" | "ascending" => Some(false),
        "descend" | "descending" => Some(true),
        _ => None,
    }
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    let int = |k: usize, what: &str, lo: i64, hi: i64| whole(&a[k], what, lo, hi);
    Ok(Some(match (name, n) {
        ("times" | "rdivide" | "ldivide" | "power", 2) => elementwise(name, &a[0], &a[1])?,
        ("plus", 2..) => matrix::add(a.to_vec())?,
        ("minus", 2) => matrix::add(vec![
            a[0].clone(),
            matrix::mul(vec![Expr::minus_one(), a[1].clone()])?,
        ])?,
        ("uminus", 1) => matrix::mul(vec![Expr::minus_one(), a[0].clone()])?,
        ("uplus", 1) => a[0].clone(),
        ("mtimes", 2..) => matrix::mul(a.to_vec())?,
        ("mpower", 2) => matrix::pow(a[0].clone(), a[1].clone())?,
        ("mldivide", 2) => mldivide(&a[0], &a[1])?,
        ("mrdivide", 2) => mrdivide(&a[0], &a[1])?,
        ("colon", 2) => Expr::list(crate::lists::span(&a[0], &a[1], &Expr::one())?),
        ("colon", 3) => Expr::list(crate::lists::span(&a[0], &a[2], &a[1])?),
        ("linspace", 2 | 3) => {
            let count = if n == 3 {
                int(2, "the number of points", 1, LIMIT as i64)?
            } else {
                100
            };
            let step = sub(a[1].clone(), a[0].clone());
            Expr::list(
                (0..count)
                    .map(|k| {
                        if k == count - 1 {
                            a[1].clone()
                        } else {
                            tidy(add(vec![
                                a[0].clone(),
                                mul(vec![Expr::rat(k, count - 1), step.clone()]),
                            ]))
                        }
                    })
                    .collect(),
            )
        }
        ("logspace", 2 | 3) => {
            let count = if n == 3 {
                int(2, "the number of points", 1, LIMIT as i64)?
            } else {
                50
            };
            let step = sub(a[1].clone(), a[0].clone());
            Expr::list(
                (0..count)
                    .map(|k| {
                        let at = if k == count - 1 {
                            a[1].clone()
                        } else {
                            add(vec![
                                a[0].clone(),
                                mul(vec![Expr::rat(k, (count - 1).max(1)), step.clone()]),
                            ])
                        };
                        tidy(pow(Expr::int(10), at))
                    })
                    .collect(),
            )
        }
        ("meshgrid" | "ndgrid", 1 | 2) => {
            let x = a[0]
                .as_list()
                .ok_or_else(|| format!("{name} needs lists"))?;
            let y = a[n - 1]
                .as_list()
                .ok_or_else(|| format!("{name} needs lists"))?;
            if x.len().saturating_mul(y.len()) > LIMIT {
                return Err(format!("an array of more than {LIMIT} entries"));
            }
            if name == "meshgrid" {
                Expr::list(vec![
                    matrix::from_rows(y.iter().map(|_| x.to_vec()).collect()),
                    matrix::from_rows(y.iter().map(|v| vec![v.clone(); x.len()]).collect()),
                ])
            } else {
                Expr::list(vec![
                    matrix::from_rows(x.iter().map(|v| vec![v.clone(); y.len()]).collect()),
                    matrix::from_rows(x.iter().map(|_| y.to_vec()).collect()),
                ])
            }
        }
        ("repmat", 2 | 3) => {
            let (r, c) = span_of(&a[1..])?;
            tile(&a[0], r, c)?
        }
        ("repelem", 2) => {
            let v = a[0]
                .as_list()
                .ok_or("repelem(list, n) repeats every entry n times")?;
            let counts: Vec<i64> = match a[1].as_list() {
                Some(c) if c.len() == v.len() => c
                    .iter()
                    .map(|k| whole(k, "the number of times", 0, LIMIT as i64))
                    .collect::<Result<_, _>>()?,
                Some(_) => return Err("repelem(list, counts): a count for every entry".into()),
                None => vec![int(1, "the number of times", 0, LIMIT as i64)?; v.len()],
            };
            if counts.iter().sum::<i64>() > LIMIT as i64 {
                return Err(format!("a list of more than {LIMIT} entries"));
            }
            Expr::list(
                v.iter()
                    .zip(counts)
                    .flat_map(|(x, k)| std::iter::repeat_n(x.clone(), k as usize))
                    .collect(),
            )
        }
        ("repelem", 3) => {
            let (g, nested) = grid(&a[0]);
            let r = int(1, "the number of times", 1, 10_000)? as usize;
            let c = int(2, "the number of times", 1, 10_000)? as usize;
            if g.len() * g[0].len() * r * c > LIMIT {
                return Err(format!("an array of more than {LIMIT} entries"));
            }
            let mut out: Rows = Vec::new();
            for row in &g {
                let wide: Vec<Expr> = row
                    .iter()
                    .flat_map(|x| std::iter::repeat_n(x.clone(), c))
                    .collect();
                out.extend(std::iter::repeat_n(wide, r));
            }
            ungrid(out, nested)
        }
        ("horzcat", 1..) => cat(a, true, name)?,
        ("vertcat", 1..) => cat(a, false, name)?,
        ("cat", 2..) => match int(0, "the dimension", 1, 2)? {
            1 => cat(&a[1..], false, name)?,
            _ => cat(&a[1..], true, name)?,
        },
        ("blkdiag", 1..) => {
            let blocks: Vec<Rows> = a.iter().map(|p| grid(p).0).collect();
            let width: usize = blocks.iter().map(|b| b[0].len()).sum();
            let mut out: Rows = Vec::new();
            let mut at = 0;
            for b in &blocks {
                for row in b {
                    let mut wide = vec![Expr::zero(); width];
                    wide[at..at + row.len()].clone_from_slice(row);
                    out.push(wide);
                }
                at += b[0].len();
            }
            matrix::from_rows(out)
        }
        ("identity", 2) => {
            let (r, c) = span_of(a)?;
            matrix::from_rows(
                (0..r)
                    .map(|i| (0..c).map(|j| Expr::int(i64::from(i == j))).collect())
                    .collect(),
            )
        }
        ("diag", 2) => {
            let k = int(1, "the number of the diagonal", -10_000, 10_000)?;
            let (up, down) = (k.max(0) as usize, (-k).max(0) as usize);
            match arr(&a[0]) {
                Arr::Matrix(m) if m.len() > 1 && m[0].len() > 1 => Expr::list(
                    (0..m.len().min(m[0].len()))
                        .filter(|i| i + down < m.len() && i + up < m[0].len())
                        .map(|i| m[i + down][i + up].clone())
                        .collect(),
                ),
                other => {
                    let v = match other {
                        Arr::Vector(v) => v,
                        Arr::Matrix(m) => m.concat(),
                        Arr::Scalar(x) => vec![x],
                    };
                    let size = v.len() + up + down;
                    let mut out = vec![vec![Expr::zero(); size]; size];
                    for (i, x) in v.into_iter().enumerate() {
                        out[i + down][i + up] = x;
                    }
                    matrix::from_rows(out)
                }
            }
        }
        ("flip", 1 | 2) => {
            let dim = if n == 2 {
                int(1, "the dimension", 1, 2)?
            } else {
                0
            };
            match arr(&a[0]) {
                Arr::Scalar(x) => x,
                Arr::Vector(v) if dim != 1 => Expr::list(v.into_iter().rev().collect()),
                Arr::Vector(v) => Expr::list(v),
                Arr::Matrix(m) if dim == 2 || (dim == 0 && m.len() == 1) => matrix::from_rows(
                    m.into_iter()
                        .map(|r| r.into_iter().rev().collect())
                        .collect(),
                ),
                Arr::Matrix(m) => matrix::from_rows(m.into_iter().rev().collect()),
            }
        }
        ("fliplr", 1) => match arr(&a[0]) {
            Arr::Scalar(x) => x,
            Arr::Vector(v) => Expr::list(v.into_iter().rev().collect()),
            Arr::Matrix(m) => matrix::from_rows(
                m.into_iter()
                    .map(|r| r.into_iter().rev().collect())
                    .collect(),
            ),
        },
        ("flipud", 1) => match arr(&a[0]) {
            Arr::Matrix(m) => matrix::from_rows(m.into_iter().rev().collect()),
            _ => a[0].clone(),
        },
        ("rot90", 1 | 2) => {
            let k = if n == 2 {
                int(1, "the number of quarter turns", -1_000_000, 1_000_000)?
            } else {
                1
            };
            let (g, nested) = grid(&a[0]);
            if g[0].is_empty() {
                return Ok(Some(a[0].clone()));
            }
            let out = rot90(&g, k);
            let rows = out.len();
            ungrid(out, nested || rows > 1)
        }
        ("circshift", 2 | 3) => {
            let steps = |e: &Expr| whole(e, "the shift", -(LIMIT as i64), LIMIT as i64);
            let dim = if n == 3 {
                int(2, "the dimension", 1, 2)?
            } else {
                0
            };
            match arr(&a[0]) {
                Arr::Scalar(x) => x,
                Arr::Vector(v) => match dim {
                    1 => Expr::list(v),
                    _ => Expr::list(shifted(&v, steps(&a[1])?)),
                },
                Arr::Matrix(m) => {
                    let (down, right) = match (a[1].as_list(), dim) {
                        (Some([r, c]), 0) => (steps(r)?, steps(c)?),
                        (Some(_), _) => return Err("circshift(matrix, [down, right])".into()),
                        (None, 2) => (0, steps(&a[1])?),
                        (None, 0) if m.len() == 1 => (0, steps(&a[1])?),
                        (None, _) => (steps(&a[1])?, 0),
                    };
                    matrix::from_rows(
                        shifted(&m, down)
                            .iter()
                            .map(|r| shifted(r, right))
                            .collect(),
                    )
                }
            }
        }
        ("tril" | "triu", 1 | 2) => {
            let k = if n == 2 {
                int(
                    1,
                    "the number of the diagonal",
                    -(LIMIT as i64),
                    LIMIT as i64,
                )?
            } else {
                0
            };
            let (g, nested) = grid(&a[0]);
            let keep = |i: usize, j: usize| {
                let d = j as i64 - i as i64;
                if name == "tril" { d <= k } else { d >= k }
            };
            ungrid(
                g.iter()
                    .enumerate()
                    .map(|(i, r)| {
                        r.iter()
                            .enumerate()
                            .map(|(j, x)| if keep(i, j) { x.clone() } else { Expr::zero() })
                            .collect()
                    })
                    .collect(),
                nested,
            )
        }
        ("sortrows", 1 | 2) => {
            let m = matrix::rows(&a[0]).ok_or("sortrows needs a matrix")?;
            let width = m[0].len() as i64;
            let col = |e: &Expr| {
                e.as_i64()
                    .filter(|k| *k != 0 && k.abs() <= width)
                    .ok_or_else(|| format!("sortrows: {e} is no column of the matrix (a column counted from 1, with a minus sign for the falling order)"))
            };
            let keys: Vec<i64> = match a.get(1) {
                None => (1..=width).collect(),
                Some(k) => match (k.as_list(), direction(k)) {
                    (_, Some(false)) => (1..=width).collect(),
                    (_, Some(true)) => (1..=width).map(|c| -c).collect(),
                    (Some(list), _) => list.iter().map(col).collect::<Result<_, _>>()?,
                    (None, _) => vec![col(k)?],
                },
            };
            matrix::from_rows(sort_rows(&m, &keys))
        }
        ("sort", 2 | 3) => {
            let mut dim = 0;
            let mut descend = false;
            for extra in &a[1..] {
                match (direction(extra), extra.as_i64()) {
                    (Some(d), _) => descend = d,
                    (None, Some(k @ 1..=2)) => dim = k,
                    _ => {
                        return Err(format!(
                            "sort(list, \"descend\"), sort(matrix, 1) down the columns, sort(matrix, 2) along the rows; {extra} is neither"
                        ));
                    }
                }
            }
            match arr(&a[0]) {
                Arr::Scalar(x) => x,
                Arr::Vector(v) => Expr::list(sorted_dir(&v, descend)),
                Arr::Matrix(m) => match dim {
                    2 => matrix::from_rows(m.iter().map(|r| sorted_dir(r, descend)).collect()),
                    1 => matrix::from_rows(matrix::transpose(
                        &matrix::transpose(&m)
                            .iter()
                            .map(|c| sorted_dir(c, descend))
                            .collect(),
                    )),
                    _ => {
                        let rows: Vec<Expr> = m.into_iter().map(Expr::list).collect();
                        Expr::list(sorted_dir(&rows, descend))
                    }
                },
            }
        }
        ("issorted", 1 | 2) => {
            let way = match a.get(1).map(|w| (direction(w), word(w))) {
                None | Some((Some(false), _)) => 1,
                Some((Some(true), _)) => -1,
                Some((_, Some(w))) if w == "either" => 0,
                Some(_) => {
                    return Err("issorted(list, \"ascend\" | \"descend\" | \"either\")".into());
                }
            };
            let run = |v: &[Expr], up: bool| {
                v.windows(2).all(|w| {
                    let o = order_any(&w[0], &w[1]);
                    if up {
                        o != Ordering::Greater
                    } else {
                        o != Ordering::Less
                    }
                })
            };
            let ok = |v: &[Expr]| match way {
                1 => run(v, true),
                -1 => run(v, false),
                _ => run(v, true) || run(v, false),
            };
            Expr::Bool(match arr(&a[0]) {
                Arr::Scalar(_) => true,
                Arr::Vector(v) => ok(&v),
                Arr::Matrix(m) if m.len() == 1 => ok(&m[0]),
                Arr::Matrix(m) => matrix::transpose(&m).iter().all(|c| ok(c)),
            })
        }
        ("unique", 2) => {
            let v = a[0].as_list().ok_or("unique needs a list")?;
            match word(&a[1]).as_deref() {
                Some("sorted") => Expr::list(unique_sorted(v)),
                Some("stable") => {
                    let mut out: Vec<Expr> = Vec::new();
                    for x in v {
                        if !out.iter().any(|y| same(x, y)) {
                            out.push(x.clone());
                        }
                    }
                    Expr::list(out)
                }
                _ => return Err("unique(list, \"sorted\") or unique(list, \"stable\")".into()),
            }
        }
        ("setxor", 2) => {
            let (x, y) = (
                a[0].as_list().ok_or("setxor needs two lists")?,
                a[1].as_list().ok_or("setxor needs two lists")?,
            );
            let mut out: Vec<Expr> = x
                .iter()
                .filter(|v| !y.iter().any(|w| same(v, w)))
                .cloned()
                .collect();
            out.extend(x_not_in(y, x));
            Expr::list(unique_sorted(&out))
        }
        ("ismember", 2) => {
            let set = a[1].as_list().ok_or("ismember(x, list)")?;
            let mut pool = Vec::new();
            set.iter().for_each(|x| leaves(x, &mut pool));
            map(&a[0], &|x| Expr::Bool(pool.iter().any(|y| same(x, y))))
        }
        ("isequal", 2..) => Expr::Bool(a.windows(2).all(|w| same(&w[0], &w[1]))),
        ("numel", 1) => Expr::int(size(&a[0]).iter().product::<usize>() as i64),
        ("size", 1) => Expr::list(
            size(&a[0])
                .into_iter()
                .map(|d| Expr::int(d as i64))
                .collect(),
        ),
        ("size", 2) => {
            let k = int(1, "the dimension", 1, 1000)? as usize;
            Expr::int(size(&a[0]).get(k - 1).copied().unwrap_or(1) as i64)
        }
        ("ndims", 1) => Expr::int(size(&a[0]).len() as i64),
        ("isempty", 1) => Expr::Bool(match &a[0] {
            Expr::List(items) => items.is_empty() || size(&a[0]).contains(&0),
            other => crate::radix::text_of(other).is_some_and(str::is_empty),
        }),
        ("isrow", 1) => Expr::Bool({
            let s = size(&a[0]);
            s.len() == 2 && s[0] == 1
        }),
        ("iscolumn", 1) => Expr::Bool({
            let s = size(&a[0]);
            s.len() == 2 && s[1] == 1
        }),
        ("isvector", 1) => Expr::Bool({
            let s = size(&a[0]);
            s.len() == 2 && (s[0] == 1 || s[1] == 1)
        }),
        ("isscalar", 1) => Expr::Bool(size(&a[0]).iter().all(|d| *d == 1)),
        ("ismatrix", 1) => Expr::Bool(size(&a[0]).len() == 2),
        ("isdiag" | "istril" | "istriu", 1) => {
            let (lower, upper) = band(&grid(&a[0]).0);
            Expr::Bool(match name {
                "isdiag" => lower == 0 && upper == 0,
                "istril" => upper == 0,
                _ => lower == 0,
            })
        }
        ("bandwidth", 1 | 2) => {
            let (lower, upper) = band(&grid(&a[0]).0);
            match a.get(1).map(word) {
                None => Expr::list(vec![Expr::int(lower as i64), Expr::int(upper as i64)]),
                Some(Some(w)) if w == "lower" => Expr::int(lower as i64),
                Some(Some(w)) if w == "upper" => Expr::int(upper as i64),
                Some(_) => return Err("bandwidth(matrix), bandwidth(matrix, \"lower\") or bandwidth(matrix, \"upper\")".into()),
            }
        }
        ("isbanded", 3) => {
            let (lower, upper) = band(&grid(&a[0]).0);
            Expr::Bool(
                lower as i64 <= int(1, "the lower width", 0, LIMIT as i64)?
                    && upper as i64 <= int(2, "the upper width", 0, LIMIT as i64)?,
            )
        }
        ("issymmetric" | "ishermitian", 1 | 2) => {
            let skew = match a.get(1).map(word) {
                None => false,
                Some(Some(w)) if w == "skew" => true,
                Some(Some(w)) if w == "nonskew" => false,
                Some(_) => return Err(format!("{name}(matrix) or {name}(matrix, \"skew\")")),
            };
            let (g, _) = grid(&a[0]);
            let turned = if name == "ishermitian" {
                ctranspose(&g)
            } else {
                matrix::transpose(&g)
            };
            Expr::Bool(
                g.len() == g[0].len()
                    && g.iter().zip(&turned).all(|(r, t)| {
                        r.iter().zip(t).all(|(x, y)| {
                            let d = if skew {
                                add(vec![x.clone(), y.clone()])
                            } else {
                                sub(x.clone(), y.clone())
                            };
                            crate::simplify::is_zero(&d)
                        })
                    }),
            )
        }
        ("nnz", 1) => Expr::int(mask(&a[0])?.concat().iter().filter(|t| **t).count() as i64),
        ("nonzeros", 1) => {
            let all = column_major(&a[0]);
            let mut out = Vec::new();
            for x in all {
                if truthy(&x)? {
                    out.push(x);
                }
            }
            Expr::list(out)
        }
        ("find", 1 | 2) => {
            let most = if n == 2 {
                int(1, "the number of entries", 0, LIMIT as i64)? as usize
            } else {
                usize::MAX
            };
            let m = mask(&a[0])?;
            let flat = m.len() == 1 && matrix::rows(&threaded(&a[0])).is_none();
            let mut out = Vec::new();
            if flat {
                for (j, t) in m[0].iter().enumerate() {
                    if *t {
                        out.push(Expr::int(j as i64 + 1));
                    }
                }
            } else {
                for j in 0..m[0].len() {
                    for (i, row) in m.iter().enumerate() {
                        if row[j] {
                            out.push(Expr::list(vec![
                                Expr::int(i as i64 + 1),
                                Expr::int(j as i64 + 1),
                            ]));
                        }
                    }
                }
            }
            out.truncate(most);
            Expr::list(out)
        }
        ("any" | "all", 1 | 2) => {
            let m = mask(&a[0])?;
            let fold = |v: &[bool]| {
                Expr::Bool(if name == "any" {
                    v.iter().any(|t| *t)
                } else {
                    v.iter().all(|t| *t)
                })
            };
            let nested = matrix::rows(&threaded(&a[0])).is_some();
            let dim = match a.get(1) {
                None => 0,
                Some(d) if word(d).as_deref() == Some("all") => 3,
                Some(d) => whole(d, "the dimension", 1, 2)?,
            };
            match dim {
                3 => fold(&m.concat()),
                0 if !nested || m.len() == 1 || m[0].len() == 1 => fold(&m.concat()),
                2 => Expr::list(m.iter().map(|r| fold(r)).collect()),
                1 if !nested => Expr::list(m[0].iter().map(|t| Expr::Bool(*t)).collect()),
                _ => Expr::list(
                    (0..m[0].len())
                        .map(|j| fold(&m.iter().map(|r| r[j]).collect::<Vec<bool>>()))
                        .collect(),
                ),
            }
        }
        ("ind2sub", 2) => {
            let dims = a[0]
                .as_list()
                .filter(|d| d.len() == 2)
                .ok_or("ind2sub([rows, columns], index)")?;
            let rows = whole(&dims[0], "the number of rows", 1, i64::MAX)?;
            let cols = whole(&dims[1], "the number of columns", 1, i64::MAX)?;
            let one = |e: &Expr| -> Res {
                let k = whole(e, "the index", 1, rows.saturating_mul(cols))? - 1;
                Ok(Expr::list(vec![
                    Expr::int(k % rows + 1),
                    Expr::int(k / rows + 1),
                ]))
            };
            match a[1].as_list() {
                Some(list) => Expr::list(list.iter().map(one).collect::<Result<_, _>>()?),
                None => one(&a[1])?,
            }
        }
        ("sub2ind", 3) => {
            let dims = a[0]
                .as_list()
                .filter(|d| d.len() == 2)
                .ok_or("sub2ind([rows, columns], row, column)")?;
            let rows = whole(&dims[0], "the number of rows", 1, i64::MAX)?;
            let cols = whole(&dims[1], "the number of columns", 1, i64::MAX)?;
            let one = |r: &Expr, c: &Expr| -> Res {
                let r = whole(r, "the row", 1, rows)?;
                let c = whole(c, "the column", 1, cols)?;
                Ok(Expr::int((c - 1) * rows + r))
            };
            match (a[1].as_list(), a[2].as_list()) {
                (Some(r), Some(c)) if r.len() == c.len() => Expr::list(
                    r.iter()
                        .zip(c)
                        .map(|(x, y)| one(x, y))
                        .collect::<Result<_, _>>()?,
                ),
                (None, None) => one(&a[1], &a[2])?,
                _ => {
                    return Err(
                        "sub2ind([rows, columns], row, column): as many rows as columns".into(),
                    );
                }
            }
        }
        ("magic", 1) => {
            let size = int(0, "the size of a magic square", 1, 1000)? as usize;
            matrix::from_rows(magic(size))
        }
        ("invhilb", 1) => {
            let size = int(0, "the size", 1, 500)? as u32;
            matrix::from_rows(
                (1..=size)
                    .map(|i| {
                        (1..=size)
                            .map(|j| {
                                let v = Integer::from(i + j - 1)
                                    * binomial(size + i - 1, size - j)
                                    * binomial(size + j - 1, size - i)
                                    * binomial(i + j - 2, i - 1).square();
                                Expr::integer(if (i + j) % 2 == 1 { -v } else { v })
                            })
                            .collect()
                    })
                    .collect(),
            )
        }
        ("pascal", 1 | 2) => {
            let size = int(0, "the size", 1, 500)? as usize;
            let kind = if n == 2 { int(1, "the kind", 0, 2)? } else { 0 };
            matrix::from_rows(pascal(size, kind))
        }
        ("rosser", 0) => matrix::from_rows(
            ROSSER
                .iter()
                .map(|r| r.iter().map(|x| Expr::int(*x)).collect())
                .collect(),
        ),
        ("wilkinson", 1) => {
            let size = int(0, "the size", 1, 1000)?;
            matrix::from_rows(
                (0..size)
                    .map(|i| {
                        (0..size)
                            .map(|j| {
                                if i == j {
                                    Expr::rat((size - 1 - 2 * i).abs(), 2)
                                } else {
                                    Expr::int(i64::from((i - j).abs() == 1))
                                }
                            })
                            .collect()
                    })
                    .collect(),
            )
        }
        ("toeplitz", 1 | 2) => {
            let col = a[0]
                .as_list()
                .filter(|c| !c.is_empty())
                .ok_or("toeplitz needs a list")?;
            let row: Vec<Expr> = match a.get(1) {
                Some(r) => r
                    .as_list()
                    .filter(|r| !r.is_empty())
                    .ok_or("toeplitz needs lists")?
                    .to_vec(),
                None => col.to_vec(),
            };
            let col: Vec<Expr> = if n == 1 {
                col.iter().map(|x| func("conj", vec![x.clone()])).collect()
            } else {
                col.to_vec()
            };
            matrix::from_rows(
                (0..col.len())
                    .map(|i| {
                        (0..row.len())
                            .map(|j| {
                                if i > j || (i == j && n == 2) {
                                    col[i - j].clone()
                                } else {
                                    row[j - i].clone()
                                }
                            })
                            .collect()
                    })
                    .collect(),
            )
        }
        ("hankel", 1 | 2) => {
            let col = a[0]
                .as_list()
                .filter(|c| !c.is_empty())
                .ok_or("hankel needs a list")?;
            let row: Vec<Expr> = match a.get(1) {
                Some(r) => r
                    .as_list()
                    .filter(|r| !r.is_empty())
                    .ok_or("hankel needs lists")?
                    .to_vec(),
                None => vec![Expr::zero(); col.len()],
            };
            matrix::from_rows(
                (0..col.len())
                    .map(|i| {
                        (0..row.len())
                            .map(|j| {
                                if i + j < col.len() {
                                    col[i + j].clone()
                                } else {
                                    row[i + j + 1 - col.len()].clone()
                                }
                            })
                            .collect()
                    })
                    .collect(),
            )
        }
        ("compan", 1) => {
            let p = a[0]
                .as_list()
                .filter(|p| p.len() >= 2)
                .ok_or("compan needs the coefficients of a polynomial, the highest first")?;
            if crate::simplify::is_zero(&p[0]) {
                return Err("compan: the first coefficient must not be zero".into());
            }
            let size = p.len() - 1;
            matrix::from_rows(
                (0..size)
                    .map(|i| {
                        (0..size)
                            .map(|j| {
                                if i == 0 {
                                    tidy(neg(div(p[j + 1].clone(), p[0].clone())))
                                } else {
                                    Expr::int(i64::from(i == j + 1))
                                }
                            })
                            .collect()
                    })
                    .collect(),
            )
        }
        ("hadamard", 1) => {
            let size = int(0, "the size", 1, 4096)? as usize;
            let h =
                hadamard(size).ok_or("hadamard(n): n is a power of two, or 12 or 20 times one")?;
            matrix::from_rows(
                h.into_iter()
                    .map(|r| r.into_iter().map(Expr::int).collect())
                    .collect(),
            )
        }
        ("kron", 2)
            if a.iter().any(|x| !matches!(arr(x), Arr::Matrix(_)))
                && a.iter().all(|x| x.as_list().is_none_or(|l| !l.is_empty())) =>
        {
            // (a flat list is a row here, a number a matrix of one entry)
            let ((p, nested_p), (q, nested_q)) = (grid(&a[0]), grid(&a[1]));
            let mut out: Rows = Vec::with_capacity(p.len() * q.len());
            for row in &p {
                for line in &q {
                    out.push(
                        row.iter()
                            .flat_map(|x| {
                                line.iter()
                                    .map(move |y| tidy(mul(vec![x.clone(), y.clone()])))
                            })
                            .collect(),
                    );
                }
            }
            ungrid(out, nested_p || nested_q)
        }
        ("vander", 1) => {
            let v = a[0].as_list().ok_or("vander needs a list")?;
            let size = v.len();
            matrix::from_rows(
                v.iter()
                    .map(|x| {
                        (0..size)
                            .map(|j| match size - 1 - j {
                                0 => Expr::one(),
                                e => pow(x.clone(), Expr::int(e as i64)),
                            })
                            .collect()
                    })
                    .collect(),
            )
        }
        ("transpose", 1) if matches!(arr(&a[0]), Arr::Vector(ref v) if !v.is_empty() && v.iter().all(|x| x.as_list().is_none())) => {
            Expr::list(
                a[0].as_list()
                    .unwrap_or_default()
                    .iter()
                    .map(|x| Expr::list(vec![x.clone()]))
                    .collect(),
            )
        }
        ("ctranspose", 1) => match arr(&a[0]) {
            Arr::Scalar(x) => func("conj", vec![x]),
            _ => {
                let (g, _) = grid(&a[0]);
                matrix::from_rows(ctranspose(&g))
            }
        },
        ("nchoosek", 2) => match a[0].as_list() {
            Some(_) => return crate::lists::pure("subsets", a).transpose(),
            None => func("binomial", a.to_vec()),
        },
        ("perms", 1) => {
            let all = crate::lists::pure("permutations", a).ok_or("perms needs a list")??;
            Expr::list(
                all.as_list()
                    .unwrap_or_default()
                    .iter()
                    .rev()
                    .cloned()
                    .collect(),
            )
        }
        _ => return Ok(None),
    }))
}

fn x_not_in(items: &[Expr], other: &[Expr]) -> Vec<Expr> {
    items
        .iter()
        .filter(|v| !other.iter().any(|w| same(v, w)))
        .cloned()
        .collect()
}

pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    inner(name, a).transpose()
}
