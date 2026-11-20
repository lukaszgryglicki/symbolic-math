//! Pairs of matrices and forms of one matrix: balancing, the Schur form and the singular
//! values of a pair, Schur forms in another order, and forms with real numbers only.

use crate::arrays::{tidy, truthy};
use crate::canon::{add, div, mul, neg, pow, sub};
use crate::decomp::{
    CMat, c_adjoint, c_conj, c_mul, c_unit, from_complex, from_complex_list, precision, schur,
    size_of, svd, to_complex,
};
use crate::expr::Expr;
use crate::funcs::func;
use crate::matrix::{self, Rows};
use crate::numeric;
use crate::radix::text_of;
use crate::simplify::is_zero;
use rug::{Complex, Float};
use std::cmp::Ordering;

type Res = Result<Expr, String>;

pub(crate) fn times(a: &CMat, b: &CMat) -> CMat {
    let prec = precision();
    let columns = b.first().map_or(0, Vec::len);
    a.iter()
        .map(|row| {
            (0..columns)
                .map(|j| {
                    let mut s = Complex::new(prec);
                    for (x, other) in row.iter().zip(b) {
                        s += c_mul(x, &other[j]);
                    }
                    s
                })
                .collect()
        })
        .collect()
}

fn largest(m: &CMat) -> Float {
    let mut top = Float::new(precision());
    for z in m.iter().flatten() {
        top = top.max(&size_of(z));
    }
    top
}

/// What counts as nothing beside numbers of this size.
fn tiny(size: &Float) -> Float {
    let prec = precision();
    Float::with_val(prec, size * (Float::with_val(prec, 1) >> (prec / 2)))
}

/// The X with `A*X = B` by elimination; none when A has no inverse within the digits.
pub(crate) fn solved(a: &CMat, b: &CMat) -> Option<CMat> {
    let n = a.len();
    let small = tiny(&largest(a));
    let mut m: CMat = a
        .iter()
        .zip(b)
        .map(|(r, s)| r.iter().chain(s).cloned().collect())
        .collect();
    for col in 0..n {
        let best = (col..n).max_by(|i, j| {
            size_of(&m[*i][col])
                .partial_cmp(&size_of(&m[*j][col]))
                .unwrap_or(Ordering::Equal)
        })?;
        if size_of(&m[best][col]) <= small {
            return None;
        }
        m.swap(col, best);
        let lead = m[col][col].clone();
        for z in m[col].iter_mut() {
            *z /= &lead;
        }
        let pivot = m[col].clone();
        for (i, row) in m.iter_mut().enumerate() {
            let factor = row[col].clone();
            if i == col || factor.is_zero() {
                continue;
            }
            for (z, p) in row.iter_mut().zip(&pivot) {
                *z -= c_mul(&factor, p);
            }
        }
    }
    Some(m.into_iter().map(|r| r[n..].to_vec()).collect())
}

/// `A = Q*R` with a unitary Q and an R that is zero below its diagonal (Householder).
pub(crate) fn qr(a: &CMat) -> (CMat, CMat) {
    let prec = precision();
    let (m, n) = (a.len(), a[0].len());
    let mut r = a.clone();
    let mut q = c_unit(m, prec);
    for k in 0..n.min(m - 1) {
        let mut norm = Float::new(prec);
        for row in &r[k..] {
            norm += Float::with_val(prec, row[k].norm_ref());
        }
        let norm = norm.sqrt();
        if norm.is_zero() {
            continue;
        }
        let head = r[k][k].clone();
        let size = size_of(&head);
        let phase = match size.is_zero() {
            true => Complex::with_val(prec, 1),
            false => Complex::with_val(prec, &head / &size),
        };
        let alpha = -Complex::with_val(prec, &phase * &norm);
        let mut v: Vec<Complex> = r[k..].iter().map(|row| row[k].clone()).collect();
        v[0] -= &alpha;
        let mut length = Float::new(prec);
        for z in &v {
            length += Float::with_val(prec, z.norm_ref());
        }
        let length = length.sqrt();
        if length.is_zero() {
            continue;
        }
        for z in v.iter_mut() {
            *z /= &length;
        }
        for j in k..n {
            let mut s = Complex::new(prec);
            for (z, row) in v.iter().zip(&r[k..]) {
                s += c_mul(&c_conj(z), &row[j]);
            }
            s *= 2u32;
            for (z, row) in v.iter().zip(r[k..].iter_mut()) {
                row[j] -= c_mul(z, &s);
            }
        }
        for row in q.iter_mut() {
            let mut s = Complex::new(prec);
            for (z, x) in v.iter().zip(&row[k..]) {
                s += c_mul(x, z);
            }
            s *= 2u32;
            for (z, x) in v.iter().zip(row[k..].iter_mut()) {
                *x -= c_mul(&s, &c_conj(z));
            }
        }
        for row in r[k + 1..].iter_mut() {
            row[k] = Complex::new(prec);
        }
    }
    (q, r)
}

fn modulus(x: &Expr, name: &str) -> Result<Float, String> {
    numeric::evalf(x, 128)
        .map(|z| Float::with_val(128, z.abs_ref()))
        .map_err(|_| format!("{name} needs a matrix of numbers, {x} is none"))
}

/// `[T, B]` with `B = T^-1*A*T`: rows and columns exchanged where that sets an eigenvalue
/// apart, then scaled by powers of two until every row is about as large as its column
/// (the steps of LAPACK's xGEBAL).
fn balance(a: &Rows) -> Result<(Rows, Rows), String> {
    let n = a.len();
    let mut mag: Vec<Vec<Float>> = a
        .iter()
        .map(|r| r.iter().map(|x| modulus(x, "balance")).collect())
        .collect::<Result<_, _>>()?;
    let exchange = |mag: &mut Vec<Vec<Float>>, i: usize, j: usize| {
        if i != j {
            for row in mag.iter_mut() {
                row.swap(i, j);
            }
            mag.swap(i, j);
        }
    };
    let mut swapped: Vec<usize> = (0..n).collect();
    let (mut low, mut high) = (0, n - 1);
    let mut alone = false;
    let mut again = true;
    while again && !alone {
        again = false;
        // (the rows of one pass are those that were left when it began, as in LAPACK)
        let last = high;
        for i in (0..=last).rev() {
            if (0..=high).all(|j| i == j || mag[i][j].is_zero()) {
                swapped[high] = i;
                exchange(&mut mag, i, high);
                again = true;
                if high == 0 {
                    alone = true;
                    break;
                }
                high -= 1;
            }
        }
    }
    again = !alone;
    while again {
        again = false;
        let first = low;
        for j in first..=high {
            if (low..=high).all(|i| i == j || mag[i][j].is_zero()) {
                swapped[low] = j;
                exchange(&mut mag, j, low);
                again = true;
                low += 1;
            }
        }
    }
    let mut power = vec![0i32; n];
    again = !alone;
    while again {
        again = false;
        for i in low..=high {
            let norm = |of: &dyn Fn(usize) -> Float| {
                (low..=high)
                    .fold(Float::new(128), |s, k| s + of(k).square())
                    .sqrt()
            };
            let mut c = norm(&|k| mag[k][i].clone());
            let mut r = norm(&|k| mag[i][k].clone());
            if c.is_zero() || r.is_zero() {
                continue;
            }
            let whole = Float::with_val(128, &c + &r);
            let mut f = 0i32;
            while c < Float::with_val(128, &r / 2u32) {
                f += 1;
                c *= 2u32;
                r /= 2u32;
            }
            while Float::with_val(128, &c / 2u32) >= r {
                f -= 1;
                c /= 2u32;
                r *= 2u32;
            }
            if Float::with_val(128, &c + &r) >= whole * 0.95f64 {
                continue;
            }
            power[i] += f;
            again = true;
            for cell in mag[i].iter_mut() {
                *cell >>= f;
            }
            for row in mag.iter_mut() {
                row[i] <<= f;
            }
        }
    }
    let mut t: Rows = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| match i == j {
                    true => pow(Expr::int(2), Expr::int(i64::from(power[i]))),
                    false => Expr::zero(),
                })
                .collect()
        })
        .collect();
    for i in (0..low).rev().chain(high + 1..n) {
        t.swap(i, swapped[i]);
    }
    let back = matrix::inverse(&t)?;
    let b = matrix::matmul(&matrix::matmul(&back, a)?, &t)?;
    let tidied = |m: Rows| -> Rows {
        m.into_iter()
            .map(|r| r.into_iter().map(tidy).collect())
            .collect()
    };
    Ok((tidied(t), tidied(b)))
}

/// A turn of two neighbours: columns `k` and `k + 1` of every row become
/// `[a*x + b*y, -a*conj(y) + b*conj(x)]`.
fn turn_columns(m: &mut CMat, k: usize, x: &Complex, y: &Complex) {
    for row in m.iter_mut() {
        let (a, b) = (row[k].clone(), row[k + 1].clone());
        row[k] = c_mul(&a, x) + c_mul(&b, y);
        row[k + 1] = c_mul(&b, &c_conj(x)) - c_mul(&a, &c_conj(y));
    }
}

/// The same from the left: rows `k` and `k + 1` become
/// `[conj(x)*a + conj(y)*b, -y*a + x*b]`.
fn turn_rows(m: &mut CMat, k: usize, x: &Complex, y: &Complex) {
    let (upper, lower) = (m[k].clone(), m[k + 1].clone());
    for (j, (a, b)) in upper.iter().zip(&lower).enumerate() {
        m[k][j] = c_mul(&c_conj(x), a) + c_mul(&c_conj(y), b);
        m[k + 1][j] = c_mul(x, b) - c_mul(y, a);
    }
}

/// A vector of two numbers with the length 1; none for the vector 0.
fn unit_pair(x: Complex, y: Complex) -> Option<(Complex, Complex)> {
    let prec = precision();
    let length = (Float::with_val(prec, x.norm_ref()) + Float::with_val(prec, y.norm_ref())).sqrt();
    (!length.is_zero()).then(|| (x / &length, y / length))
}

/// A Schur form with the neighbours `k` and `k + 1` of its diagonal exchanged.
fn exchange_one(t: &mut CMat, u: &mut CMat, k: usize) {
    let prec = precision();
    let apart = Complex::with_val(prec, &t[k + 1][k + 1] - &t[k][k]);
    let scale = size_of(&t[k][k]) + size_of(&t[k + 1][k + 1]);
    if size_of(&apart) <= tiny(&scale) {
        return;
    }
    let Some((x, y)) = unit_pair(t[k][k + 1].clone(), apart) else {
        return;
    };
    turn_rows(t, k, &x, &y);
    turn_columns(t, k, &x, &y);
    turn_columns(u, k, &x, &y);
    t[k + 1][k] = Complex::new(prec);
}

/// The Schur form of a pair with the neighbours `k` and `k + 1` exchanged.
fn exchange_pair(aa: &mut CMat, bb: &mut CMat, q: &mut CMat, z: &mut CMat, k: usize) {
    let prec = precision();
    let h1 = c_mul(&bb[k + 1][k + 1], &aa[k][k]) - c_mul(&aa[k + 1][k + 1], &bb[k][k]);
    let h2 = c_mul(&bb[k + 1][k + 1], &aa[k][k + 1]) - c_mul(&aa[k + 1][k + 1], &bb[k][k + 1]);
    let scale = largest(aa) * largest(bb);
    if size_of(&h1) <= tiny(&scale) {
        return;
    }
    let Some((x, y)) = unit_pair(h2, -h1) else {
        return;
    };
    for m in [&mut *aa, &mut *bb, &mut *z] {
        turn_columns(m, k, &x, &y);
    }
    // the turn from the left that clears what the first turn put below the diagonal
    let from = match size_of(&aa[k][k]) + size_of(&aa[k + 1][k])
        >= size_of(&bb[k][k]) + size_of(&bb[k + 1][k])
    {
        true => &*aa,
        false => &*bb,
    };
    let Some((x, y)) = unit_pair(from[k][k].clone(), from[k + 1][k].clone()) else {
        return;
    };
    for m in [&mut *aa, &mut *bb, &mut *q] {
        turn_rows(m, k, &x, &y);
    }
    aa[k + 1][k] = Complex::new(prec);
    bb[k + 1][k] = Complex::new(prec);
}

/// The rank of every eigenvalue in the new order: the greater, the sooner.
fn ranks(select: &Expr, values: &[(Complex, Complex)], name: &str) -> Result<Vec<i64>, String> {
    let usage = || {
        format!(
            "{name}: the eigenvalues that come first are chosen by a list of {} truths or \
             numbers, or by \"lhp\", \"rhp\", \"udi\", \"udo\"",
            values.len()
        )
    };
    if let Some(word) = text_of(select) {
        let side = word.to_lowercase();
        if !matches!(side.as_str(), "lhp" | "rhp" | "udi" | "udo") {
            return Err(usage());
        }
        let prec = precision();
        return Ok(values
            .iter()
            .map(|(top, under)| {
                // (an eigenvalue without an end is outside the circle, and in no half)
                let chosen = match under.is_zero() {
                    true => side == "udo",
                    false => {
                        let value = Complex::with_val(prec, top / under);
                        let size = size_of(&value);
                        match side.as_str() {
                            "lhp" => value.real().is_sign_negative() && !value.real().is_zero(),
                            "rhp" => value.real().is_sign_positive() && !value.real().is_zero(),
                            "udi" => size < 1u32,
                            _ => size > 1u32,
                        }
                    }
                };
                i64::from(chosen)
            })
            .collect());
    }
    let items = select
        .as_list()
        .filter(|l| l.len() == values.len())
        .ok_or_else(usage)?;
    items
        .iter()
        .map(|x| match x.as_i64() {
            Some(k) => Ok(k),
            None => truthy(x).map(i64::from).map_err(|_| usage()),
        })
        .collect()
}

/// The exchanges of neighbours that bring the greatest ranks to the front and leave
/// equal ranks as they stand.
fn in_order(mut keys: Vec<i64>, exchange: &mut dyn FnMut(usize)) {
    for end in 1..keys.len() {
        let mut k = end;
        while k > 0 && keys[k - 1] < keys[k] {
            exchange(k - 1);
            keys.swap(k - 1, k);
            k -= 1;
        }
    }
}

fn square(m: &Rows, name: &str) -> Result<usize, String> {
    match !m.is_empty() && m.iter().all(|r| r.len() == m.len()) {
        true => Ok(m.len()),
        false => Err(format!("{name} needs square matrices")),
    }
}

fn triangular(m: &CMat, name: &str) -> Result<(), String> {
    let small = tiny(&largest(m));
    let below = m
        .iter()
        .enumerate()
        .any(|(i, row)| row[..i].iter().any(|z| size_of(z) > small));
    match below {
        true => Err(format!(
            "{name} needs a form that is zero below its diagonal, as schurdecomposition and qz give"
        )),
        false => Ok(()),
    }
}

/// `[AA, BB, Q, Z]` with `Q*A*Z = AA` and `Q*B*Z = BB` zero below their diagonals.
fn qz(a: &CMat, b: &CMat) -> Result<[CMat; 4], String> {
    let prec = precision();
    let n = a.len();
    // A + mu*B has an inverse for all but n numbers mu, unless no number of the pair is one
    let shifts = [0i32, 1, -1, 2, -2, 3, -3, 5, -5, 7, -7];
    let (mu, shifted, quotient) = shifts
        .iter()
        .find_map(|mu| {
            let shifted: CMat = a
                .iter()
                .zip(b)
                .map(|(r, s)| {
                    r.iter()
                        .zip(s)
                        .map(|(x, y)| Complex::with_val(prec, x + Complex::with_val(prec, y * *mu)))
                        .collect()
                })
                .collect();
            let quotient = solved(&shifted, b)?;
            Some((*mu, shifted, quotient))
        })
        .ok_or("qz: det(A - x*B) is zero for every x, the pair has no eigenvalues of its own")?;
    let (z, t) = schur(&quotient)?;
    let (unitary, r) = qr(&times(&shifted, &z));
    let mut q = c_adjoint(&unitary);
    let mut bb = times(&r, &t);
    // AA = R*(1 - mu*T)
    let rest: CMat = t
        .iter()
        .enumerate()
        .map(|(i, row)| {
            row.iter()
                .enumerate()
                .map(|(j, x)| {
                    Complex::with_val(prec, u32::from(i == j)) - Complex::with_val(prec, x * mu)
                })
                .collect()
        })
        .collect();
    let mut aa = times(&r, &rest);
    // the diagonal of BB without a sign or an imaginary part
    for i in 0..n {
        let size = size_of(&bb[i][i]);
        if size.is_zero() {
            continue;
        }
        let phase = c_conj(&Complex::with_val(prec, &bb[i][i] / &size));
        for m in [&mut aa, &mut bb, &mut q] {
            for x in m[i].iter_mut() {
                *x *= &phase;
            }
        }
        bb[i][i] = Complex::with_val(prec, size);
    }
    for i in 0..n {
        for j in 0..i {
            aa[i][j] = Complex::new(prec);
            bb[i][j] = Complex::new(prec);
        }
    }
    Ok([aa, bb, q, z])
}

struct Pair {
    u: CMat,
    v: CMat,
    x: CMat,
    c: Vec<Float>,
    s: Vec<Float>,
    /// How many columns the pair has in common after its rows are made independent.
    width: usize,
}

/// `A = U*C*X^H` and `B = V*S*X^H` with unitary U and V and `C^H*C + S^H*S = 1`: the
/// numbers of C rise along its diagonal, those of S fall.
fn gsvd(a: &CMat, b: &CMat) -> Result<Pair, String> {
    let prec = precision();
    let (m, n, p) = (a.len(), b.len(), a[0].len());
    let width = p.min(m + n);
    let stacked: CMat = a.iter().chain(b).cloned().collect();
    let (q, r) = qr(&stacked);
    let part = |from: usize, to: usize| -> CMat {
        q[from..to]
            .iter()
            .map(|row| row[..width].to_vec())
            .collect()
    };
    let (q1, q2) = (part(0, m), part(m, m + n));
    let (u_first, values, w_first) = svd(&q1)?;
    // the singular values of the upper part, the least first; the columns of W beyond
    // them belong to the value 0
    let turned = |mat: &CMat, count: usize| -> CMat {
        mat.iter()
            .map(|row| {
                let mut row = row.clone();
                row[..count].reverse();
                row
            })
            .collect()
    };
    let w = turned(&w_first, width);
    let u = turned(&u_first, m.min(width));
    let mut c = vec![Float::new(prec); width];
    for (k, value) in values.iter().enumerate() {
        c[width - 1 - k] = value.clone();
    }
    let y = times(&q2, &w);
    let lengths: Vec<Float> = (0..width)
        .map(|j| {
            y.iter()
                .fold(Float::new(prec), |s, row| {
                    s + Float::with_val(prec, row[j].norm_ref())
                })
                .sqrt()
        })
        .collect();
    let small = Float::with_val(prec, 1) >> (prec / 2);
    let mut columns: Vec<Vec<Complex>> = Vec::new();
    let mut s = vec![Float::new(prec); width];
    for j in 0..width.min(n) {
        if lengths[j] <= small {
            break;
        }
        s[j] = lengths[j].clone();
        columns.push(
            y.iter()
                .map(|row| Complex::with_val(prec, &row[j] / &lengths[j]))
                .collect(),
        );
    }
    for value in c
        .iter_mut()
        .zip(&s)
        .filter(|(_, s)| s.is_zero())
        .map(|(c, _)| c)
    {
        *value = Float::with_val(prec, 1);
    }
    // the columns of V that are left: any that stand at right angles to those there
    let mut next = 0;
    while columns.len() < n && next < n {
        let mut cand: Vec<Complex> = (0..n)
            .map(|i| Complex::with_val(prec, u32::from(i == next)))
            .collect();
        next += 1;
        for _ in 0..2 {
            for have in &columns {
                let mut dot = Complex::new(prec);
                for (h, x) in have.iter().zip(&cand) {
                    dot += c_mul(&c_conj(h), x);
                }
                for (h, x) in have.iter().zip(cand.iter_mut()) {
                    *x -= c_mul(&dot, h);
                }
            }
        }
        let length = cand
            .iter()
            .fold(Float::new(prec), |s, z| {
                s + Float::with_val(prec, z.norm_ref())
            })
            .sqrt();
        if length > 1e-6 {
            columns.push(
                cand.iter()
                    .map(|z| Complex::with_val(prec, z / &length))
                    .collect(),
            );
        }
    }
    let v: CMat = (0..n)
        .map(|i| columns.iter().map(|col| col[i].clone()).collect())
        .collect();
    // X = R^H*W for the rows of R that count
    let upper: CMat = r[..width].to_vec();
    let x = times(&c_adjoint(&upper), &w);
    Ok(Pair {
        u,
        v,
        x,
        c,
        s,
        width,
    })
}

/// The first number of the two that `eig` gives for a block of two real numbers a side.
fn first_of_block(a: &Expr, b: &Expr, c: &Expr, d: &Expr) -> (Expr, Expr) {
    let half = sub(a.clone(), d.clone());
    let p = div(half, Expr::int(2));
    let inside = tidy(add(vec![
        pow(p.clone(), Expr::int(2)),
        mul(vec![b.clone(), c.clone()]),
    ]));
    let root = func("sqrt", vec![inside.clone()]);
    let mean = div(add(vec![a.clone(), d.clone()]), Expr::int(2));
    let down =
        numeric::sign(&inside).is_some_and(|s| s >= 0) && numeric::sign(&p).is_some_and(|s| s < 0);
    let (first, second) = match down {
        true => (sub(mean.clone(), root.clone()), add(vec![mean, root])),
        false => (add(vec![mean.clone(), root.clone()]), sub(mean, root)),
    };
    (tidy(first), tidy(second))
}

fn values_of(m: &Rows, name: &str) -> Result<Vec<Expr>, String> {
    let n = square(m, name)?;
    let nothing = |x: &Expr| x.is_zero() || is_zero(x);
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        if m[i][..i.saturating_sub(1)].iter().any(|x| !nothing(x)) {
            return Err(format!(
                "{name} needs a Schur form: zero below the diagonal, or with blocks of two"
            ));
        }
        if i + 1 < n && !nothing(&m[i + 1][i]) {
            let (first, second) =
                first_of_block(&m[i][i], &m[i][i + 1], &m[i + 1][i], &m[i + 1][i + 1]);
            out.extend([first, second]);
            i += 2;
        } else {
            out.push(m[i][i].clone());
            i += 1;
        }
    }
    Ok(out)
}

/// The complex Schur form from a real one with blocks of two (the steps of MATLAB).
fn complex_form(u: &mut CMat, t: &mut CMat) {
    let prec = precision();
    let n = t.len();
    for m in (1..n).rev() {
        if !t[m][m - 1].is_zero() {
            let (a, b, c, d) = (
                t[m - 1][m - 1].clone(),
                t[m - 1][m].clone(),
                t[m][m - 1].clone(),
                t[m][m].clone(),
            );
            let p = Complex::with_val(prec, &a - &d) / 2u32;
            let inside = Complex::with_val(prec, p.square_ref()) + c_mul(&b, &c);
            let real = inside.imag().is_zero() && !inside.real().is_sign_negative();
            let mut root = inside.sqrt();
            if real && p.real().is_sign_negative() && p.imag().is_zero() {
                root = -root;
            }
            let mu = Complex::with_val(prec, &a + &d) / 2u32 + root - &d;
            if let Some((x, y)) = unit_pair(mu, c) {
                // G = [conj(x), y; -y, x] from the left, its adjoint from the right
                let (upper, lower) = (t[m - 1].clone(), t[m].clone());
                for (j, (p, q)) in upper.iter().zip(&lower).enumerate().skip(m - 1) {
                    t[m - 1][j] = c_mul(&c_conj(&x), p) + c_mul(&y, q);
                    t[m][j] = c_mul(&x, q) - c_mul(&y, p);
                }
                let last = m;
                for (rows, to) in [(&mut *t, last + 1), (&mut *u, n)] {
                    for row in rows[..to].iter_mut() {
                        let (p, q) = (row[m - 1].clone(), row[m].clone());
                        row[m - 1] = c_mul(&p, &x) + c_mul(&q, &c_conj(&y));
                        row[m] = c_mul(&q, &c_conj(&x)) - c_mul(&p, &y);
                    }
                }
            }
        }
        t[m][m - 1] = Complex::new(prec);
    }
}

/// Eigenvectors and a diagonal with pairs of conjugate numbers as real matrices: a pair
/// becomes the block `[re, im; -im, re]` and the real and imaginary part of its vector.
fn real_form(v: &Rows, d: &Rows) -> Result<(Rows, Rows), String> {
    let n = square(d, "cdf2rdf")?;
    if v.iter().any(|r| r.len() != n) {
        return Err("cdf2rdf(V, D) needs a column of V for every number of D".into());
    }
    let imaginary = |x: &Expr| {
        let part = func("im", vec![x.clone()]);
        !(part.is_zero() || is_zero(&part))
    };
    let (mut t, mut back) = (matrix::identity(n), matrix::identity(n));
    let mut i = 0;
    while i < n {
        if imaginary(&d[i][i]) {
            if i + 1 >= n || !imaginary(&d[i + 1][i + 1]) {
                return Err("cdf2rdf: every eigenvalue that is not real needs its conjugate as the next one".into());
            }
            let (one, half) = (Expr::one(), Expr::half());
            let turn = mul(vec![Expr::i(), Expr::half()]);
            (t[i][i], t[i][i + 1]) = (one.clone(), one);
            (t[i + 1][i], t[i + 1][i + 1]) = (Expr::i(), neg(Expr::i()));
            (back[i][i], back[i][i + 1]) = (half.clone(), neg(turn.clone()));
            (back[i + 1][i], back[i + 1][i + 1]) = (half, turn);
            i += 2;
        } else {
            i += 1;
        }
    }
    let tidied = |m: Rows| -> Rows {
        m.into_iter()
            .map(|r| {
                r.into_iter()
                    .map(|x| crate::simplify::simplify(&tidy(x)))
                    .collect()
            })
            .collect()
    };
    let vectors = matrix::matmul(v, &back)?;
    let blocks = matrix::matmul(&matrix::matmul(&t, d)?, &back)?;
    Ok((tidied(vectors), tidied(blocks)))
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    let mat = |k: usize| -> Result<Rows, String> {
        matrix::rows(&a[k])
            .filter(|m| !m.is_empty() && !m[0].is_empty())
            .ok_or_else(|| format!("{name} needs matrices"))
    };
    let numbers = |k: usize| -> Result<CMat, String> {
        to_complex(&mat(k)?).map_err(|_| format!("{name} needs matrices of numbers"))
    };
    let all = |parts: &[&CMat]| Expr::list(parts.iter().map(|m| from_complex(m)).collect());
    Ok(Some(match (name, n) {
        ("balance", 1) => {
            let m = mat(0)?;
            square(&m, name)?;
            let (t, b) = balance(&m)?;
            Expr::list(vec![matrix::from_rows(t), matrix::from_rows(b)])
        }
        ("qz", 2) => {
            let (ra, rb) = (mat(0)?, mat(1)?);
            if square(&ra, name)? != square(&rb, name)? {
                return Err("qz(A, B) needs two square matrices of one size".into());
            }
            let [aa, bb, q, z] = qz(&numbers(0)?, &numbers(1)?)?;
            all(&[&aa, &bb, &q, &z])
        }
        ("ordqz", 5) => {
            let (mut aa, mut bb, mut q, mut z) =
                (numbers(0)?, numbers(1)?, numbers(2)?, numbers(3)?);
            let size = aa.len();
            if [&aa, &bb, &q, &z]
                .iter()
                .any(|m| m.len() != size || m[0].len() != size)
            {
                return Err(
                    "ordqz(AA, BB, Q, Z, select) needs four square matrices of one size".into(),
                );
            }
            triangular(&aa, name)?;
            triangular(&bb, name)?;
            let values: Vec<(Complex, Complex)> = (0..size)
                .map(|i| (aa[i][i].clone(), bb[i][i].clone()))
                .collect();
            let keys = ranks(&a[4], &values, name)?;
            in_order(keys, &mut |k| {
                exchange_pair(&mut aa, &mut bb, &mut q, &mut z, k)
            });
            all(&[&aa, &bb, &q, &z])
        }
        ("ordschur", 3) => {
            let (mut u, mut t) = (numbers(0)?, numbers(1)?);
            let size = t.len();
            if [&u, &t]
                .iter()
                .any(|m| m.len() != size || m[0].len() != size)
            {
                return Err("ordschur(U, T, select) needs two square matrices of one size".into());
            }
            triangular(&t, name)?;
            let one = Complex::with_val(precision(), 1);
            let values: Vec<(Complex, Complex)> =
                (0..size).map(|i| (t[i][i].clone(), one.clone())).collect();
            let keys = ranks(&a[2], &values, name)?;
            in_order(keys, &mut |k| exchange_one(&mut t, &mut u, k));
            all(&[&u, &t])
        }
        ("ordeig", 1) => Expr::list(values_of(&mat(0)?, name)?),
        ("ordeig", 2) => {
            let (aa, bb) = (mat(0)?, mat(1)?);
            if square(&aa, name)? != square(&bb, name)? {
                return Err("ordeig(AA, BB) needs two square matrices of one size".into());
            }
            Expr::list(
                aa.iter()
                    .zip(&bb)
                    .enumerate()
                    .map(|(i, (ra, rb))| match rb[i].is_zero() || is_zero(&rb[i]) {
                        true => Expr::inf(),
                        false => tidy(div(ra[i].clone(), rb[i].clone())),
                    })
                    .collect(),
            )
        }
        ("rsf2csf", 2) => {
            let (mut u, mut t) = (numbers(0)?, numbers(1)?);
            let size = t.len();
            if [&u, &t]
                .iter()
                .any(|m| m.len() != size || m[0].len() != size)
            {
                return Err("rsf2csf(U, T) needs two square matrices of one size".into());
            }
            complex_form(&mut u, &mut t);
            all(&[&u, &t])
        }
        ("cdf2rdf", 2) => {
            let (v, d) = real_form(&mat(0)?, &mat(1)?)?;
            Expr::list(vec![matrix::from_rows(v), matrix::from_rows(d)])
        }
        ("gsvd" | "gsvddecomposition", 2) => {
            let (ra, rb) = (mat(0)?, mat(1)?);
            if ra[0].len() != rb[0].len() {
                return Err(format!(
                    "{name}(A, B) needs two matrices with as many columns"
                ));
            }
            let pair = gsvd(&numbers(0)?, &numbers(1)?)?;
            let prec = precision();
            if name == "gsvd" {
                let finite: Vec<Complex> = pair
                    .c
                    .iter()
                    .zip(&pair.s)
                    .filter(|(_, s)| !s.is_zero())
                    .map(|(c, s)| Complex::with_val(prec, c / s))
                    .collect();
                let mut out = from_complex_list(&finite);
                out.extend(pair.s.iter().filter(|s| s.is_zero()).map(|_| Expr::inf()));
                Expr::list(out)
            } else {
                // the numbers of C end at its last row and column, those of S begin at the first
                let shift = pair.width.saturating_sub(ra.len());
                let placed = |rows: usize, values: &[Float], shift: usize| -> CMat {
                    (0..rows)
                        .map(|i| {
                            (0..pair.width)
                                .map(|j| match j == i + shift {
                                    true => Complex::with_val(prec, &values[j]),
                                    false => Complex::new(prec),
                                })
                                .collect()
                        })
                        .collect()
                };
                let c = placed(ra.len(), &pair.c, shift);
                let s = placed(rb.len(), &pair.s, 0);
                all(&[&pair.u, &pair.v, &pair.x, &c, &s])
            }
        }
        _ => return Ok(None),
    }))
}

pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    inner(name, a).transpose()
}
