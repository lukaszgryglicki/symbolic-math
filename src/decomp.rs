//! Matrices taken apart and measured: norms, condition, LU, Cholesky, singular values, Schur,
//! Hessenberg, Jordan, Smith and Hermite forms, bases of the spaces of a matrix.

use crate::arrays::{Arr, arr, ctranspose, grid, order, tidy, whole, word};
use crate::canon::{add, div, mul, neg, pow, sub};
use crate::expr::Expr;
use crate::funcs::func;
use crate::matrix::{self, Rows};
use crate::numeric;
use crate::simplify::{is_zero, simplify};
use rug::{Complex, Float, Integer};
use std::cmp::Ordering;

type Res = Result<Expr, String>;
pub(crate) type CMat = Vec<Vec<Complex>>;

fn real_size(e: &Expr) -> f64 {
    numeric::real_value(&func("abs", vec![e.clone()]), 20)
        .map(|f| f.to_f64())
        .unwrap_or(f64::NAN)
}

/// The largest of real values, exactly.
fn largest(items: Vec<Expr>) -> Expr {
    let mut best: Option<Expr> = None;
    for x in items {
        best = Some(match best {
            Some(b) if order(&x, &b).is_ok_and(|o| o != Ordering::Greater) => b,
            _ => x,
        });
    }
    best.unwrap_or_else(Expr::zero)
}

fn abs_sum(items: &[Expr]) -> Expr {
    tidy(add(items
        .iter()
        .map(|x| func("abs", vec![x.clone()]))
        .collect()))
}

fn product(a: &Rows, b: &Rows) -> Result<Rows, String> {
    Ok(matrix::matmul(a, b)?
        .into_iter()
        .map(|r| r.into_iter().map(tidy).collect())
        .collect())
}

/// The singular values, the largest first: the roots of the eigenvalues of A^H A.
pub fn singular_values(m: &Rows) -> Result<Vec<Expr>, String> {
    if m.iter().all(Vec::is_empty) {
        return Ok(Vec::new());
    }
    let h = ctranspose(m);
    let gram = if m.len() < m[0].len() {
        product(m, &h)?
    } else {
        product(&h, m)?
    };
    let z = Expr::sym("#z");
    let solved = crate::solve::solve(&matrix::charpoly(&gram, &z)?, "#z")?;
    crate::solve::release(&solved.params);
    let mut values: Vec<Expr> = Vec::new();
    for r in &solved.roots {
        let size = numeric::real_value(&func("re", vec![r.value.clone()]), 20)
            .map(|f| f.to_f64())
            .unwrap_or(0.0);
        let v = if size.abs() < 1e-15 && r.approx {
            Expr::zero()
        } else {
            simplify(&crate::canon::sqrt(func("re", vec![r.value.clone()])))
        };
        values.extend(std::iter::repeat_n(v, r.mult as usize));
    }
    values.sort_by(|x, y| real_size(y).total_cmp(&real_size(x)));
    Ok(values)
}

fn matrix_norm(m: &Rows, kind: Option<&Expr>) -> Res {
    let which = match kind {
        None => "2".to_string(),
        Some(k) if k.is_inf() => "inf".to_string(),
        Some(k) => word(k)
            .or_else(|| k.as_i64().map(|v| v.to_string()))
            .unwrap_or_default(),
    };
    Ok(match which.as_str() {
        "1" => largest(matrix::transpose(m).iter().map(|c| abs_sum(c)).collect()),
        "inf" => largest(m.iter().map(|r| abs_sum(r)).collect()),
        "fro" => tidy(crate::canon::sqrt(add(m
            .concat()
            .iter()
            .map(|x| pow(func("abs", vec![x.clone()]), Expr::int(2)))
            .collect()))),
        "2" => singular_values(m)?
            .into_iter()
            .next()
            .unwrap_or_else(Expr::zero),
        _ => {
            return Err(
                "the norm of a matrix is norm(A) (the largest singular value), norm(A, 1), norm(A, inf) or norm(A, \"fro\")"
                    .into(),
            );
        }
    })
}

fn square(m: &Rows, name: &str) -> Result<usize, String> {
    if m.len() == m[0].len() {
        Ok(m.len())
    } else {
        Err(format!("{name} needs a square matrix"))
    }
}

fn rank(m: &Rows) -> usize {
    matrix::eliminate(m.clone(), false).1
}

/// `P*A = L*U` with the largest entry of every column as the pivot.
fn lu(m: &Rows) -> Result<(Rows, Rows, Rows), String> {
    let (r, c) = (m.len(), m[0].len());
    let k = r.min(c);
    let mut a = m.clone();
    let mut perm: Vec<usize> = (0..r).collect();
    let mut l = vec![vec![Expr::zero(); k]; r];
    for j in 0..k {
        let numbers = (j..r).all(|i| a[i][j].is_const());
        let mut best: Option<(usize, f64)> = None;
        for (i, row) in a.iter().enumerate().skip(j) {
            if is_zero(&row[j]) {
                continue;
            }
            let size = if numbers { real_size(&row[j]) } else { 1.0 };
            if best.is_none_or(|(_, s)| size > s * (1.0 + 1e-12)) {
                best = Some((i, size));
            }
        }
        let Some((p, _)) = best else {
            continue;
        };
        a.swap(j, p);
        l.swap(j, p);
        perm.swap(j, p);
        for i in j + 1..r {
            let f = tidy(div(a[i][j].clone(), a[j][j].clone()));
            if !f.is_zero() {
                let pivot_row = a[j].clone();
                for (x, y) in a[i].iter_mut().zip(&pivot_row).skip(j) {
                    *x = tidy(sub(x.clone(), mul(vec![f.clone(), y.clone()])));
                }
            }
            a[i][j] = Expr::zero();
            l[i][j] = f;
        }
    }
    for (i, row) in l.iter_mut().enumerate().take(k) {
        row[i] = Expr::one();
    }
    a.truncate(k);
    let p: Rows = perm
        .iter()
        .map(|from| (0..r).map(|j| Expr::int(i64::from(j == *from))).collect())
        .collect();
    Ok((l, a, p))
}

fn conj(x: &Expr) -> Expr {
    if x.is_const() {
        func("conj", vec![x.clone()])
    } else {
        x.clone()
    }
}

/// The lower triangular `L` with `L*L^H = A`.
fn cholesky(a: &Rows) -> Result<Rows, String> {
    let n = square(a, "chol")?;
    let mut l = vec![vec![Expr::zero(); n]; n];
    for i in 0..n {
        for j in 0..=i {
            if !is_zero(&sub(a[i][j].clone(), conj(&a[j][i]))) {
                return Err("chol: the matrix is not its own conjugate transpose".into());
            }
            let sum = add((0..j)
                .map(|k| mul(vec![l[i][k].clone(), conj(&l[j][k])]))
                .collect());
            let rest = simplify(&sub(a[i][j].clone(), sum));
            l[i][j] = if i == j {
                if numeric::sign(&rest).is_some_and(|s| s <= 0) || rest.is_zero() {
                    return Err("chol: the matrix is not positive definite".into());
                }
                crate::canon::sqrt(rest)
            } else {
                simplify(&div(rest, l[j][j].clone()))
            };
        }
    }
    Ok(l)
}

/// The columns of a matrix that hold its pivots.
fn pivot_columns(m: &Rows) -> Vec<usize> {
    let (r, rank, _) = matrix::eliminate(m.clone(), true);
    (0..rank)
        .filter_map(|i| (0..m[0].len()).find(|&j| !r[i][j].is_zero()))
        .collect()
}

fn hermitian_dot(a: &[Expr], b: &[Expr]) -> Expr {
    add(a
        .iter()
        .zip(b)
        .map(|(x, y)| mul(vec![conj(x), y.clone()]))
        .collect())
}

/// Orthonormal vectors with the span of the given ones (Gram-Schmidt).
fn orthonormal(vectors: &Rows) -> Rows {
    let mut out: Rows = Vec::new();
    for v in vectors {
        let mut w = v.clone();
        for u in &out {
            let scale = hermitian_dot(u, v);
            for (x, y) in w.iter_mut().zip(u) {
                *x = sub(x.clone(), mul(vec![scale.clone(), y.clone()]));
            }
        }
        let w: Vec<Expr> = w.iter().map(simplify).collect();
        if w.iter().all(is_zero) {
            continue;
        }
        let length = simplify(&crate::canon::sqrt(hermitian_dot(&w, &w)));
        out.push(
            w.iter()
                .map(|x| simplify(&div(x.clone(), length.clone())))
                .collect(),
        );
    }
    out
}

/// An orthonormal basis of the span of the columns, as columns.
fn orth(m: &Rows) -> Rows {
    let cols = matrix::transpose(m);
    let basis: Rows = pivot_columns(m)
        .into_iter()
        .map(|j| cols[j].clone())
        .collect();
    matrix::transpose(&orthonormal(&basis))
}

/// The eigenvalues with how often each is one.
fn eigenvalues(m: &Rows) -> Result<Vec<(Expr, usize)>, String> {
    const VAR: &str = "#ev";
    let solved = crate::solve::solve(&matrix::charpoly(m, &Expr::sym(VAR))?, VAR)?;
    crate::solve::release(&solved.params);
    if solved.roots.iter().any(|r| r.approx) {
        return Err(
            "the eigenvalues are known as decimal numbers only, and this needs them exactly".into(),
        );
    }
    Ok(solved
        .roots
        .into_iter()
        .map(|r| (r.value, r.mult as usize))
        .collect())
}

fn shifted(m: &Rows, by: &Expr) -> Rows {
    m.iter()
        .enumerate()
        .map(|(i, r)| {
            r.iter()
                .enumerate()
                .map(|(j, x)| {
                    if i == j {
                        tidy(sub(x.clone(), by.clone()))
                    } else {
                        x.clone()
                    }
                })
                .collect()
        })
        .collect()
}

fn apply(m: &Rows, v: &[Expr]) -> Vec<Expr> {
    m.iter()
        .map(|r| {
            tidy(add(r
                .iter()
                .zip(v)
                .map(|(x, y)| mul(vec![x.clone(), y.clone()]))
                .collect()))
        })
        .collect()
}

/// `[S, J]` with `A*S = S*J`: the chains of generalized eigenvectors and the Jordan blocks.
fn jordan(a: &Rows) -> Result<(Rows, Rows), String> {
    let n = square(a, "jordan")?;
    let mut chains: Vec<(Expr, Vec<Vec<Expr>>)> = Vec::new();
    for (value, mult) in eigenvalues(a)? {
        let shift = shifted(a, &value);
        // (the kernels of the powers of A - value*I grow until they hold `mult` vectors)
        let mut kernels: Vec<Rows> = vec![Vec::new()];
        let mut power = matrix::identity(n);
        while kernels.last().map(Vec::len) != Some(mult) {
            power = product(&power, &shift)?;
            let kernel = matrix::nullspace(&power);
            if Some(kernel.len()) == kernels.last().map(Vec::len) {
                return Err(
                    "jordan: the kernels of an eigenvalue do not fill its multiplicity".into(),
                );
            }
            kernels.push(kernel);
        }
        let mut found: Vec<Vec<Vec<Expr>>> = Vec::new();
        for level in (1..kernels.len()).rev() {
            // what of this level is known: the kernel below and the vectors of longer chains
            let mut known: Rows = kernels[level - 1].clone();
            for chain in &found {
                known.push(chain[level - 1].clone());
            }
            for v in &kernels[level] {
                let mut trial = known.clone();
                trial.push(v.clone());
                if rank(&trial) > known.len() {
                    let mut chain = vec![v.clone()];
                    for _ in 1..level {
                        let next = apply(&shift, &chain[0]);
                        chain.insert(0, next);
                    }
                    known = trial;
                    found.push(chain);
                }
            }
        }
        for chain in found {
            chains.push((value.clone(), chain));
        }
    }
    let mut columns: Rows = Vec::new();
    let mut j = vec![vec![Expr::zero(); n]; n];
    for (value, chain) in chains {
        for (k, v) in chain.into_iter().enumerate() {
            let at = columns.len();
            j[at][at] = value.clone();
            if k > 0 {
                j[at - 1][at] = Expr::one();
            }
            columns.push(v);
        }
    }
    if columns.len() != n {
        return Err("jordan: the chains do not fill the space".into());
    }
    Ok((matrix::transpose(&columns), j))
}

fn integers(m: &Rows, name: &str) -> Result<Vec<Vec<Integer>>, String> {
    m.iter()
        .map(|r| {
            r.iter()
                .map(|x| {
                    x.as_int().cloned().ok_or_else(|| {
                        format!("{name} needs a matrix of whole numbers, {x} is none")
                    })
                })
                .collect()
        })
        .collect()
}

fn from_integers(m: Vec<Vec<Integer>>) -> Rows {
    m.into_iter()
        .map(|r| r.into_iter().map(Expr::integer).collect())
        .collect()
}

fn unit(n: usize) -> Vec<Vec<Integer>> {
    (0..n)
        .map(|i| (0..n).map(|j| Integer::from(i == j)).collect())
        .collect()
}

fn quotient(a: &Integer, b: &Integer) -> Integer {
    <(Integer, Integer)>::from(a.div_rem_floor_ref(b)).0
}

/// `row[to] -= q * row[from]`.
fn row_op(m: &mut [Vec<Integer>], to: usize, from: usize, q: &Integer) {
    let source = m[from].clone();
    for (x, y) in m[to].iter_mut().zip(&source) {
        *x -= Integer::from(q * y);
    }
}

fn col_op(m: &mut [Vec<Integer>], to: usize, from: usize, q: &Integer) {
    for row in m.iter_mut() {
        let y = row[from].clone();
        row[to] -= Integer::from(q * &y);
    }
}

fn col_swap(m: &mut [Vec<Integer>], a: usize, b: usize) {
    for row in m.iter_mut() {
        row.swap(a, b);
    }
}

type Whole = Vec<Vec<Integer>>;

/// `U*A*V = S`: the Smith normal form, every entry of its diagonal dividing the next.
fn smith(a: Whole) -> (Whole, Whole, Whole) {
    let (r, c) = (a.len(), a[0].len());
    let (mut s, mut u, mut v) = (a, unit(r), unit(c));
    for t in 0..r.min(c) {
        loop {
            let mut least: Option<(usize, usize)> = None;
            for i in t..r {
                for j in t..c {
                    if !s[i][j].is_zero()
                        && least.is_none_or(|(p, q)| s[i][j].cmp_abs(&s[p][q]) == Ordering::Less)
                    {
                        least = Some((i, j));
                    }
                }
            }
            let Some((p, q)) = least else {
                return (u, v, s);
            };
            s.swap(t, p);
            u.swap(t, p);
            col_swap(&mut s, t, q);
            col_swap(&mut v, t, q);
            let mut clean = true;
            for i in t + 1..r {
                let quot = quotient(&s[i][t], &s[t][t]);
                if !quot.is_zero() {
                    row_op(&mut s, i, t, &quot);
                    row_op(&mut u, i, t, &quot);
                }
                clean &= s[i][t].is_zero();
            }
            for j in t + 1..c {
                let quot = quotient(&s[t][j], &s[t][t]);
                if !quot.is_zero() {
                    col_op(&mut s, j, t, &quot);
                    col_op(&mut v, j, t, &quot);
                }
                clean &= s[t][j].is_zero();
            }
            if !clean {
                continue;
            }
            let odd = (t + 1..r)
                .flat_map(|i| (t + 1..c).map(move |j| (i, j)))
                .find(|(i, j)| !s[*i][*j].is_divisible(&s[t][t]));
            match odd {
                Some((i, _)) => {
                    let minus = Integer::from(-1);
                    row_op(&mut s, t, i, &minus);
                    row_op(&mut u, t, i, &minus);
                }
                None => break,
            }
        }
        if s[t][t] < 0 {
            for x in s[t].iter_mut() {
                *x = Integer::from(-&*x);
            }
            for x in u[t].iter_mut() {
                *x = Integer::from(-&*x);
            }
        }
    }
    (u, v, s)
}

/// `U*A = H`: the Hermite normal form by rows: upper triangular, the pivots positive, the
/// entries above a pivot from 0 to under it.
fn hermite(a: Vec<Vec<Integer>>) -> (Vec<Vec<Integer>>, Vec<Vec<Integer>>) {
    let (r, c) = (a.len(), a[0].len());
    let (mut h, mut u) = (a, unit(r));
    let mut row = 0;
    for col in 0..c {
        if row == r {
            break;
        }
        loop {
            let mut least: Option<usize> = None;
            for i in row..r {
                if !h[i][col].is_zero()
                    && least.is_none_or(|p| h[i][col].cmp_abs(&h[p][col]) == Ordering::Less)
                {
                    least = Some(i);
                }
            }
            let Some(p) = least else {
                break;
            };
            h.swap(row, p);
            u.swap(row, p);
            let mut clean = true;
            for i in row + 1..r {
                let quot = quotient(&h[i][col], &h[row][col]);
                if !quot.is_zero() {
                    row_op(&mut h, i, row, &quot);
                    row_op(&mut u, i, row, &quot);
                }
                clean &= h[i][col].is_zero();
            }
            if clean {
                break;
            }
        }
        if h[row][col].is_zero() {
            continue;
        }
        if h[row][col] < 0 {
            for x in h[row].iter_mut() {
                *x = Integer::from(-&*x);
            }
            for x in u[row].iter_mut() {
                *x = Integer::from(-&*x);
            }
        }
        for i in 0..row {
            let quot = quotient(&h[i][col], &h[row][col]);
            if !quot.is_zero() {
                row_op(&mut h, i, row, &quot);
                row_op(&mut u, i, row, &quot);
            }
        }
        row += 1;
    }
    (u, h)
}

// ---- matrices of decimal numbers ----

pub(crate) fn precision() -> u32 {
    numeric::bits_for(numeric::digits()) + 64
}

pub(crate) fn to_complex(m: &Rows) -> Result<CMat, String> {
    let prec = precision();
    m.iter()
        .map(|r| {
            r.iter()
                .map(|x| numeric::evalf(x, prec).map_err(|e| e.to_string()))
                .collect()
        })
        .collect()
}

pub(crate) fn size_of(z: &Complex) -> Float {
    Float::with_val(z.prec().0, z.abs_ref())
}

/// The matrix as a value with the digits of the session; what is zero within them is 0.
pub(crate) fn from_complex(m: &CMat) -> Expr {
    let digits = numeric::digits();
    let prec = precision();
    let mut top = Float::new(prec);
    for z in m.iter().flatten() {
        let s = size_of(z);
        if s > top {
            top = s;
        }
    }
    let tiny = top * Float::with_val(prec, Float::i_pow_u(10, digits + 2)).recip();
    matrix::from_rows(
        m.iter()
            .map(|r| {
                r.iter()
                    .map(|z| {
                        let (mut re, mut im) = (z.real().clone(), z.imag().clone());
                        if Float::with_val(prec, re.abs_ref()) <= tiny {
                            re = Float::new(prec);
                        }
                        if Float::with_val(prec, im.abs_ref()) <= tiny {
                            im = Float::new(prec);
                        }
                        if re.is_zero() && im.is_zero() {
                            Expr::zero()
                        } else {
                            numeric::to_float_expr(Complex::with_val(prec, (re, im)), digits)
                        }
                    })
                    .collect()
            })
            .collect(),
    )
}

/// A list of decimal numbers as a value, as `from_complex` gives a matrix.
pub(crate) fn from_complex_list(v: &[Complex]) -> Vec<Expr> {
    match from_complex(&vec![v.to_vec()])
        .as_list()
        .and_then(<[Expr]>::first)
    {
        Some(Expr::List(row)) => row.to_vec(),
        _ => Vec::new(),
    }
}

pub(crate) fn c_unit(n: usize, prec: u32) -> CMat {
    (0..n)
        .map(|i| {
            (0..n)
                .map(|j| Complex::with_val(prec, u32::from(i == j)))
                .collect()
        })
        .collect()
}

pub(crate) fn c_conj(z: &Complex) -> Complex {
    Complex::with_val(z.prec(), z.conj_ref())
}

pub(crate) fn c_mul(a: &Complex, b: &Complex) -> Complex {
    Complex::with_val(a.prec(), a * b)
}

pub(crate) fn c_adjoint(m: &CMat) -> CMat {
    (0..m[0].len())
        .map(|j| m.iter().map(|r| c_conj(&r[j])).collect())
        .collect()
}

/// `A = P*H*P^H` with `H` zero below its first subdiagonal (Householder).
#[allow(clippy::needless_range_loop)]
fn hessenberg(a: &CMat) -> (CMat, CMat) {
    let n = a.len();
    let prec = precision();
    let mut h = a.clone();
    let mut p = c_unit(n, prec);
    for k in 0..n.saturating_sub(2) {
        let mut v: Vec<Complex> = (k + 1..n).map(|i| h[i][k].clone()).collect();
        let mut norm = Float::new(prec);
        for z in &v {
            norm += Float::with_val(prec, z.norm_ref());
        }
        let norm = norm.sqrt();
        if norm.is_zero() {
            continue;
        }
        let first = size_of(&v[0]);
        let phase = if first.is_zero() {
            Complex::with_val(prec, 1)
        } else {
            Complex::with_val(prec, &v[0] / &first)
        };
        v[0] += Complex::with_val(prec, &phase * &norm);
        let mut length = Float::new(prec);
        for z in &v {
            length += Float::with_val(prec, z.norm_ref());
        }
        if length.is_zero() {
            continue;
        }
        let twice = Float::with_val(prec, 2) / length;
        for j in 0..n {
            let mut s = Complex::new(prec);
            for (i, z) in v.iter().enumerate() {
                s += c_mul(&c_conj(z), &h[k + 1 + i][j]);
            }
            s *= &twice;
            for (i, z) in v.iter().enumerate() {
                h[k + 1 + i][j] -= c_mul(z, &s);
            }
        }
        for m in [&mut h, &mut p] {
            for row in m.iter_mut() {
                let mut s = Complex::new(prec);
                for (j, z) in v.iter().enumerate() {
                    s += c_mul(&row[k + 1 + j], z);
                }
                s *= &twice;
                for (j, z) in v.iter().enumerate() {
                    row[k + 1 + j] -= c_mul(&s, &c_conj(z));
                }
            }
        }
        for row in h.iter_mut().skip(k + 2) {
            row[k] = Complex::new(prec);
        }
    }
    (h, p)
}

/// `A = Q*T*Q^H` with `T` upper triangular (the QR steps with Wilkinson's shift).
#[allow(clippy::needless_range_loop)]
pub(crate) fn schur(a: &CMat) -> Result<(CMat, CMat), String> {
    let n = a.len();
    let prec = precision();
    let (mut t, mut q) = hessenberg(a);
    if n < 2 {
        return Ok((q, t));
    }
    let eps = Float::with_val(prec, 1) >> (prec - 8);
    let mut hi = n - 1;
    let mut rounds = 0;
    while hi > 0 {
        let mut lo = 0;
        for l in (1..=hi).rev() {
            let mut scale = size_of(&t[l - 1][l - 1]) + size_of(&t[l][l]);
            if scale.is_zero() {
                scale = Float::with_val(prec, 1);
            }
            if size_of(&t[l][l - 1]) <= scale * &eps {
                t[l][l - 1] = Complex::new(prec);
                lo = l;
                break;
            }
        }
        if lo == hi {
            hi -= 1;
            rounds = 0;
            continue;
        }
        rounds += 1;
        if rounds > 500 {
            return Err("schur: the steps do not settle".into());
        }
        let mu = if rounds % 11 == 10 {
            let mut s = size_of(&t[hi][hi - 1]);
            if hi >= 2 {
                s += size_of(&t[hi - 1][hi - 2]);
            }
            Complex::with_val(prec, s)
        } else {
            let (a, b, c, d) = (
                &t[hi - 1][hi - 1],
                &t[hi - 1][hi],
                &t[hi][hi - 1],
                &t[hi][hi],
            );
            let half = Complex::with_val(prec, a - d) / 2u32;
            let root = (Complex::with_val(prec, &half * &half) + c_mul(b, c)).sqrt();
            let mid = Complex::with_val(prec, a + d) / 2u32;
            let (one, two) = (
                Complex::with_val(prec, &mid + &root),
                Complex::with_val(prec, &mid - &root),
            );
            let near = |z: &Complex| size_of(&Complex::with_val(prec, z - d));
            if near(&one) <= near(&two) { one } else { two }
        };
        for i in lo..=hi {
            t[i][i] -= &mu;
        }
        let mut turns: Vec<Option<(Complex, Complex)>> = Vec::new();
        for k in lo..hi {
            let (a, b) = (t[k][k].clone(), t[k + 1][k].clone());
            let r =
                (Float::with_val(prec, a.norm_ref()) + Float::with_val(prec, b.norm_ref())).sqrt();
            if r.is_zero() {
                turns.push(None);
                continue;
            }
            let (c, s) = (
                Complex::with_val(prec, &a / &r),
                Complex::with_val(prec, &b / &r),
            );
            let (cc, sc) = (c_conj(&c), c_conj(&s));
            for j in k..n {
                let (x, y) = (t[k][j].clone(), t[k + 1][j].clone());
                t[k][j] = c_mul(&cc, &x) + c_mul(&sc, &y);
                t[k + 1][j] = c_mul(&c, &y) - c_mul(&s, &x);
            }
            turns.push(Some((c, s)));
        }
        for (idx, turn) in turns.iter().enumerate() {
            let Some((c, s)) = turn else {
                continue;
            };
            let k = lo + idx;
            let (cc, sc) = (c_conj(c), c_conj(s));
            for row in t.iter_mut().take(k + 2) {
                let (x, y) = (row[k].clone(), row[k + 1].clone());
                row[k] = c_mul(c, &x) + c_mul(s, &y);
                row[k + 1] = c_mul(&cc, &y) - c_mul(&sc, &x);
            }
            for row in q.iter_mut() {
                let (x, y) = (row[k].clone(), row[k + 1].clone());
                row[k] = c_mul(c, &x) + c_mul(s, &y);
                row[k + 1] = c_mul(&cc, &y) - c_mul(&sc, &x);
            }
        }
        for i in lo..=hi {
            t[i][i] += &mu;
        }
    }
    for (i, row) in t.iter_mut().enumerate() {
        for z in row.iter_mut().take(i) {
            *z = Complex::new(prec);
        }
    }
    Ok((q, t))
}

/// `A = U*S*V^H`: the singular values, the largest first, with unitary U and V (one-sided
/// Jacobi turns).
pub(crate) fn svd(a: &CMat) -> Result<(CMat, Vec<Float>, CMat), String> {
    let (m, n) = (a.len(), a[0].len());
    if m < n {
        let (v, s, u) = svd(&c_adjoint(a))?;
        return Ok((u, s, v));
    }
    let prec = precision();
    let mut w = a.clone();
    let mut v = c_unit(n, prec);
    let eps = Float::with_val(prec, 1) >> (prec - 8);
    let mut settled = false;
    for _ in 0..300 {
        let mut turned = false;
        for p in 0..n {
            for q in p + 1..n {
                let (mut alpha, mut beta) = (Float::new(prec), Float::new(prec));
                let mut gamma = Complex::new(prec);
                for row in &w {
                    alpha += Float::with_val(prec, row[p].norm_ref());
                    beta += Float::with_val(prec, row[q].norm_ref());
                    gamma += c_mul(&c_conj(&row[p]), &row[q]);
                }
                let g = size_of(&gamma);
                if g.is_zero() || g <= Float::with_val(prec, &alpha * &beta).sqrt() * &eps {
                    continue;
                }
                turned = true;
                let back = c_conj(&Complex::with_val(prec, &gamma / &g));
                let zeta = Float::with_val(prec, &beta - &alpha) / (Float::with_val(prec, 2) * &g);
                let root = (Float::with_val(prec, &zeta * &zeta) + 1u32).sqrt();
                let mut tan = (Float::with_val(prec, zeta.abs_ref()) + root).recip();
                if zeta.is_sign_negative() {
                    tan = -tan;
                }
                let cos = (Float::with_val(prec, &tan * &tan) + 1u32).sqrt().recip();
                let sin = Float::with_val(prec, &cos * &tan);
                for mat in [&mut w, &mut v] {
                    for row in mat.iter_mut() {
                        let x = row[p].clone();
                        let y = c_mul(&back, &row[q]);
                        row[p] =
                            Complex::with_val(prec, &x * &cos) - Complex::with_val(prec, &y * &sin);
                        row[q] =
                            Complex::with_val(prec, &x * &sin) + Complex::with_val(prec, &y * &cos);
                    }
                }
            }
        }
        if !turned {
            settled = true;
            break;
        }
    }
    if !settled {
        return Err("svd: the turns do not settle".into());
    }
    let mut lengths: Vec<Float> = (0..n)
        .map(|j| {
            let mut s = Float::new(prec);
            for row in &w {
                s += Float::with_val(prec, row[j].norm_ref());
            }
            s.sqrt()
        })
        .collect();
    let mut places: Vec<usize> = (0..n).collect();
    places.sort_by(|i, j| {
        lengths[*j]
            .partial_cmp(&lengths[*i])
            .unwrap_or(Ordering::Equal)
    });
    let top = lengths[places[0]].clone();
    let tiny = Float::with_val(prec, &top * &eps) * 1024u32;
    let mut u: Vec<Vec<Complex>> = Vec::new();
    for j in &places {
        if lengths[*j] > tiny {
            u.push(
                w.iter()
                    .map(|row| Complex::with_val(prec, &row[*j] / &lengths[*j]))
                    .collect(),
            );
        } else {
            lengths[*j] = Float::new(prec);
        }
    }
    // (the columns of U that no singular value gives: unit vectors made orthogonal to the rest)
    let mut next = 0;
    while u.len() < m && next < m {
        let mut cand: Vec<Complex> = (0..m)
            .map(|i| Complex::with_val(prec, u32::from(i == next)))
            .collect();
        next += 1;
        for _ in 0..2 {
            for have in &u {
                let mut s = Complex::new(prec);
                for (x, y) in have.iter().zip(&cand) {
                    s += c_mul(&c_conj(x), y);
                }
                for (x, y) in have.iter().zip(cand.iter_mut()) {
                    *y -= c_mul(&s, x);
                }
            }
        }
        let mut length = Float::new(prec);
        for z in &cand {
            length += Float::with_val(prec, z.norm_ref());
        }
        let length = length.sqrt();
        if length > Float::with_val(prec, 1e-6) {
            u.push(
                cand.iter()
                    .map(|z| Complex::with_val(prec, z / &length))
                    .collect(),
            );
        }
    }
    let u_mat: CMat = (0..m)
        .map(|i| u.iter().map(|c| c[i].clone()).collect())
        .collect();
    let v_mat: CMat = v
        .iter()
        .map(|row| places.iter().map(|j| row[*j].clone()).collect())
        .collect();
    let values: Vec<Float> = places.iter().map(|j| lengths[*j].clone()).collect();
    Ok((u_mat, values, v_mat))
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    let mat = |k: usize| -> Result<Rows, String> {
        matrix::rows(&a[k]).ok_or_else(|| format!("{name} needs a matrix"))
    };
    Ok(Some(match (name, n) {
        ("norm", 1 | 2) if matrix::rows(&a[0]).is_some() => matrix_norm(&mat(0)?, a.get(1))?,
        ("singularvalues", 1) => Expr::list(singular_values(&grid(&a[0]).0)?),
        ("cond", 1 | 2) => {
            let m = mat(0)?;
            match a.get(1) {
                None => {
                    let s = singular_values(&m)?;
                    match (s.first(), s.last()) {
                        (Some(top), Some(low)) if !is_zero(low) => {
                            tidy(div(top.clone(), low.clone()))
                        }
                        _ => Expr::inf(),
                    }
                }
                Some(kind) => {
                    square(&m, name)?;
                    if rank(&m) < m.len() {
                        Expr::inf()
                    } else {
                        tidy(mul(vec![
                            matrix_norm(&m, Some(kind))?,
                            matrix_norm(&matrix::inverse(&m)?, Some(kind))?,
                        ]))
                    }
                }
            }
        }
        ("rcond", 1) => {
            let m = mat(0)?;
            square(&m, name)?;
            if rank(&m) < m.len() {
                Expr::zero()
            } else {
                let one = Expr::one();
                tidy(div(
                    Expr::one(),
                    mul(vec![
                        matrix_norm(&m, Some(&one))?,
                        matrix_norm(&matrix::inverse(&m)?, Some(&one))?,
                    ]),
                ))
            }
        }
        ("vecnorm", 1..=3) => {
            let p = a.get(1).cloned().unwrap_or_else(|| Expr::int(2));
            let length = |v: &[Expr]| -> Res {
                if p.is_inf() {
                    return Ok(largest(
                        v.iter().map(|x| func("abs", vec![x.clone()])).collect(),
                    ));
                }
                Ok(tidy(pow(
                    add(v
                        .iter()
                        .map(|x| pow(func("abs", vec![x.clone()]), p.clone()))
                        .collect()),
                    div(Expr::one(), p.clone()),
                )))
            };
            let across = match a.get(2).map(|d| whole(d, "the dimension", 1, 2)) {
                None => false,
                Some(d) => d? == 2,
            };
            match arr(&a[0]) {
                Arr::Scalar(x) => func("abs", vec![x]),
                Arr::Vector(v) => length(&v)?,
                Arr::Matrix(m) => {
                    let lists = if across { m } else { matrix::transpose(&m) };
                    Expr::list(lists.iter().map(|v| length(v)).collect::<Result<_, _>>()?)
                }
            }
        }
        ("pinv", 1) => {
            // A = C*F with the pivot columns C and the rows F of the reduced form that are
            // not zero: the pseudoinverse is F^H (F F^H)^-1 (C^H C)^-1 C^H
            let m = mat(0)?;
            let (reduced, rank, _) = matrix::eliminate(m.clone(), true);
            if rank == 0 {
                return Ok(Some(matrix::from_rows(vec![
                    vec![Expr::zero(); m.len()];
                    m[0].len()
                ])));
            }
            let pivots = pivot_columns(&m);
            let f: Rows = reduced[..rank].to_vec();
            let c: Rows = m
                .iter()
                .map(|row| pivots.iter().map(|&j| row[j].clone()).collect())
                .collect();
            let (ch, fh) = (ctranspose(&c), ctranspose(&f));
            let left = product(&fh, &matrix::inverse(&product(&f, &fh)?)?)?;
            let right = product(&matrix::inverse(&product(&ch, &c)?)?, &ch)?;
            simplify(&matrix::from_rows(product(&left, &right)?))
        }
        ("orth", 1) => {
            let basis = orth(&mat(0)?);
            if basis.is_empty() {
                Expr::list(Vec::new())
            } else {
                matrix::from_rows(basis)
            }
        }
        ("colspace", 1) => {
            let m = mat(0)?;
            let (r, rank, _) = matrix::eliminate(matrix::transpose(&m), true);
            if rank == 0 {
                Expr::list(Vec::new())
            } else {
                matrix::from_rows(matrix::transpose(&r[..rank].to_vec()))
            }
        }
        ("lu", 1) => {
            let (l, u, p) = lu(&mat(0)?)?;
            Expr::list(vec![
                matrix::from_rows(l),
                matrix::from_rows(u),
                matrix::from_rows(p),
            ])
        }
        ("chol", 1 | 2) => {
            let l = cholesky(&mat(0)?)?;
            match a.get(1).map(word) {
                None => matrix::from_rows(ctranspose(&l)),
                Some(Some(w)) if w == "upper" => matrix::from_rows(ctranspose(&l)),
                Some(Some(w)) if w == "lower" => matrix::from_rows(l),
                Some(_) => return Err("chol(A), chol(A, \"lower\") or chol(A, \"upper\")".into()),
            }
        }
        ("jordan" | "jordandecomposition", 1) => {
            let (s, j) = jordan(&mat(0)?)?;
            if name == "jordan" {
                matrix::from_rows(j)
            } else {
                Expr::list(vec![matrix::from_rows(s), matrix::from_rows(j)])
            }
        }
        ("diagonalize", 1) => {
            let (s, j) = jordan(&mat(0)?)?;
            if j.iter()
                .enumerate()
                .any(|(i, r)| r.iter().enumerate().any(|(k, x)| i != k && !x.is_zero()))
            {
                return Err("diagonalize: the matrix has too few eigenvectors; jordandecomposition gives its Jordan form".into());
            }
            Expr::list(vec![matrix::from_rows(s), matrix::from_rows(j)])
        }
        ("smithform" | "smithdecomposition", 1) => {
            let (u, v, s) = smith(integers(&mat(0)?, name)?);
            if name == "smithform" {
                matrix::from_rows(from_integers(s))
            } else {
                Expr::list(vec![
                    matrix::from_rows(from_integers(u)),
                    matrix::from_rows(from_integers(v)),
                    matrix::from_rows(from_integers(s)),
                ])
            }
        }
        ("hermiteform" | "hermitedecomposition", 1) => {
            let (u, h) = hermite(integers(&mat(0)?, name)?);
            if name == "hermiteform" {
                matrix::from_rows(from_integers(h))
            } else {
                Expr::list(vec![
                    matrix::from_rows(from_integers(u)),
                    matrix::from_rows(from_integers(h)),
                ])
            }
        }
        ("schur" | "schurdecomposition" | "hess" | "hessenbergdecomposition", 1) => {
            let m = mat(0)?;
            square(&m, name)?;
            let c = to_complex(&m)?;
            let (q, t) = if name.starts_with("schur") {
                schur(&c)?
            } else {
                let (h, p) = hessenberg(&c);
                (p, h)
            };
            if matches!(name, "schur" | "hess") {
                from_complex(&t)
            } else {
                Expr::list(vec![from_complex(&q), from_complex(&t)])
            }
        }
        ("singularvaluedecomposition", 1) => {
            let m = mat(0)?;
            let (u, s, v) = svd(&to_complex(&m)?)?;
            let prec = precision();
            let sigma: CMat = (0..m.len())
                .map(|i| {
                    (0..m[0].len())
                        .map(|j| {
                            if i == j {
                                Complex::with_val(prec, &s[i])
                            } else {
                                Complex::new(prec)
                            }
                        })
                        .collect()
                })
                .collect();
            Expr::list(vec![
                from_complex(&u),
                from_complex(&sigma),
                from_complex(&v),
            ])
        }
        ("sylvester", 3) => {
            let (ma, mb, mc) = (mat(0)?, mat(1)?, mat(2)?);
            let (r, c) = (square(&ma, name)?, square(&mb, name)?);
            if mc.len() != r || mc[0].len() != c {
                return Err(format!(
                    "sylvester(A, B, C): C must have {r} row(s) and {c} column(s)"
                ));
            }
            // (X is taken column after column)
            let size = r * c;
            let mut system = vec![vec![Expr::zero(); size]; size];
            for (j, i) in (0..c).flat_map(|j| (0..r).map(move |i| (j, i))) {
                let row = j * r + i;
                for (k, entry) in ma[i].iter().enumerate() {
                    let at = j * r + k;
                    system[row][at] = add(vec![system[row][at].clone(), entry.clone()]);
                }
                for (k, line) in mb.iter().enumerate() {
                    let at = k * r + i;
                    system[row][at] = add(vec![system[row][at].clone(), line[j].clone()]);
                }
            }
            if rank(&system) < size {
                return Err(
                    "sylvester: A and -B share an eigenvalue, so there is not one solution".into(),
                );
            }
            let rhs: Rows = (0..size).map(|k| vec![mc[k % r][k / r].clone()]).collect();
            let x = product(&matrix::inverse(&system)?, &rhs)?;
            matrix::from_rows(
                (0..r)
                    .map(|i| (0..c).map(|j| x[j * r + i][0].clone()).collect())
                    .collect(),
            )
        }
        ("subspace", 2) => {
            let (mut qa, mut qb) = (orth(&grid_columns(&a[0])), orth(&grid_columns(&a[1])));
            if qa.is_empty() || qb.is_empty() {
                return Err("subspace needs two matrices that are not zero".into());
            }
            if qa[0].len() < qb[0].len() {
                std::mem::swap(&mut qa, &mut qb);
            }
            let inside = product(&qa, &product(&ctranspose(&qa), &qb)?)?;
            let rest: Rows = qb
                .iter()
                .zip(&inside)
                .map(|(r, s)| {
                    r.iter()
                        .zip(s)
                        .map(|(x, y)| tidy(sub(x.clone(), y.clone())))
                        .collect()
                })
                .collect();
            let size = matrix_norm(&rest, None)?;
            let size = if order(&size, &Expr::one()).is_ok_and(|o| o == Ordering::Greater) {
                Expr::one()
            } else {
                size
            };
            tidy(func("asin", vec![size]))
        }
        ("planerot", 1) => {
            let v = a[0]
                .as_list()
                .filter(|v| v.len() == 2)
                .ok_or("planerot([x, y])")?;
            let r = simplify(&crate::canon::sqrt(add(vec![
                pow(v[0].clone(), Expr::int(2)),
                pow(v[1].clone(), Expr::int(2)),
            ])));
            if is_zero(&r) {
                Expr::list(vec![
                    matrix::from_rows(matrix::identity(2)),
                    Expr::list(vec![Expr::zero(), Expr::zero()]),
                ])
            } else {
                let (c, s) = (
                    tidy(div(v[0].clone(), r.clone())),
                    tidy(div(v[1].clone(), r.clone())),
                );
                Expr::list(vec![
                    matrix::from_rows(vec![vec![c.clone(), s.clone()], vec![neg(s), c]]),
                    Expr::list(vec![r, Expr::zero()]),
                ])
            }
        }
        ("minpoly", 1 | 2) if matrix::rows(&a[0]).is_some() => {
            let m = mat(0)?;
            let size = square(&m, name)?;
            let mut powers: Rows = vec![matrix::identity(size).concat()];
            let mut power = matrix::identity(size);
            let mut found = None;
            for _ in 0..size {
                power = product(&power, &m)?;
                powers.push(power.concat());
                let kernel = matrix::nullspace(&matrix::transpose(&powers));
                if let Some(v) = kernel.first() {
                    let lead = v[v.len() - 1].clone();
                    found = Some(
                        v.iter()
                            .rev()
                            .map(|x| tidy(div(x.clone(), lead.clone())))
                            .collect::<Vec<Expr>>(),
                    );
                    break;
                }
            }
            let c = found.ok_or("minpoly: no polynomial found")?;
            match a.get(1) {
                None => Expr::list(c),
                Some(x) => {
                    let top = c.len() - 1;
                    add(c
                        .into_iter()
                        .enumerate()
                        .map(|(k, v)| mul(vec![v, pow(x.clone(), Expr::int((top - k) as i64))]))
                        .collect())
                }
            }
        }
        ("eigs" | "svds", 1 | 2) => {
            let m = mat(0)?;
            let most = if n == 2 {
                whole(&a[1], "the number of values", 0, 1_000_000)? as usize
            } else {
                6
            };
            let mut values: Vec<Expr> = if name == "svds" {
                singular_values(&m)?
            } else {
                square(&m, name)?;
                const VAR: &str = "#ev";
                let solved = crate::solve::solve(&matrix::charpoly(&m, &Expr::sym(VAR))?, VAR)?;
                crate::solve::release(&solved.params);
                solved
                    .roots
                    .iter()
                    .flat_map(|r| std::iter::repeat_n(r.value.clone(), r.mult as usize))
                    .collect()
            };
            values.sort_by(|x, y| real_size(y).total_cmp(&real_size(x)));
            values.truncate(most);
            Expr::list(values)
        }
        ("polyeig", 2..) => {
            // the numbers x with det(A0 + x*A1 + ... + x^p*Ap) = 0
            const VAR: &str = "#ev";
            let size = square(&mat(0)?, name)?;
            let mut pencil = vec![vec![Expr::zero(); size]; size];
            for k in 0..n {
                let m = mat(k)?;
                if square(&m, name)? != size {
                    return Err("polyeig(A0, A1, ...): matrices of one size".into());
                }
                let power = if k == 0 {
                    Expr::one()
                } else {
                    pow(Expr::sym(VAR), Expr::int(k as i64))
                };
                for (row, line) in pencil.iter_mut().zip(&m) {
                    for (entry, x) in row.iter_mut().zip(line) {
                        *entry = add(vec![entry.clone(), mul(vec![power.clone(), x.clone()])]);
                    }
                }
            }
            let p = matrix::det(&pencil)?;
            if p.free_of(VAR) {
                return Err("polyeig: the determinant does not depend on x".into());
            }
            let solved = crate::solve::solve(&p, VAR)?;
            crate::solve::release(&solved.params);
            Expr::list(
                solved
                    .roots
                    .iter()
                    .flat_map(|r| std::iter::repeat_n(r.value.clone(), r.mult as usize))
                    .collect(),
            )
        }
        ("pca", 1) => {
            // the directions of the largest variances: the eigenvectors of the covariances
            let data = mat(0)?;
            let (count, size) = (data.len(), data[0].len());
            if count < 2 {
                return Err(
                    "pca needs a matrix with two or more rows, one for every observation".into(),
                );
            }
            let means: Vec<Expr> = (0..size)
                .map(|j| {
                    tidy(div(
                        add(data.iter().map(|r| r[j].clone()).collect()),
                        Expr::int(count as i64),
                    ))
                })
                .collect();
            let centred: Rows = data
                .iter()
                .map(|r| {
                    r.iter()
                        .zip(&means)
                        .map(|(x, m)| tidy(sub(x.clone(), m.clone())))
                        .collect()
                })
                .collect();
            let covariances: Rows = product(&ctranspose(&centred), &centred)?
                .into_iter()
                .map(|r| {
                    r.into_iter()
                        .map(|x| tidy(div(x, Expr::int(count as i64 - 1))))
                        .collect()
                })
                .collect();
            let (q, t) = schur(&to_complex(&covariances)?)?;
            let prec = precision();
            let mut by_size: Vec<usize> = (0..size).collect();
            by_size.sort_by(|i, j| {
                t[*j][*j]
                    .real()
                    .partial_cmp(t[*i][*i].real())
                    .unwrap_or(Ordering::Equal)
            });
            // every direction with its largest part positive
            let directions: CMat = by_size
                .iter()
                .map(|k| {
                    let column: Vec<Complex> = q.iter().map(|r| r[*k].clone()).collect();
                    let largest = column
                        .iter()
                        .max_by(|x, y| {
                            size_of(x)
                                .partial_cmp(&size_of(y))
                                .unwrap_or(Ordering::Equal)
                        })
                        .cloned()
                        .unwrap_or_else(|| Complex::with_val(prec, 1));
                    if largest.real().is_sign_negative() {
                        column.iter().map(|z| Complex::with_val(prec, -z)).collect()
                    } else {
                        column
                    }
                })
                .collect();
            let coefficients = c_adjoint(&directions)
                .into_iter()
                .map(|r| r.iter().map(c_conj).collect())
                .collect::<CMat>();
            let scores = product(
                &centred,
                &matrix::rows(&from_complex(&coefficients)).unwrap_or_default(),
            )?;
            let variances: Vec<Complex> = by_size.iter().map(|k| t[*k][*k].clone()).collect();
            Expr::list(vec![
                from_complex(&coefficients),
                matrix::from_rows(scores),
                Expr::list(from_complex_list(&variances)),
            ])
        }
        ("eigenvalues", 2) => {
            let (ma, mb) = (mat(0)?, mat(1)?);
            let size = square(&ma, "eig")?;
            if square(&mb, "eig")? != size {
                return Err("eig(A, B): two matrices of one size".into());
            }
            const VAR: &str = "#ev";
            let pencil: Rows = ma
                .iter()
                .zip(&mb)
                .map(|(r, s)| {
                    r.iter()
                        .zip(s)
                        .map(|(x, y)| sub(x.clone(), mul(vec![Expr::sym(VAR), y.clone()])))
                        .collect()
                })
                .collect();
            let p = matrix::det(&pencil)?;
            if p.free_of(VAR) {
                return Err("eig(A, B): det(A - x*B) does not depend on x".into());
            }
            let solved = crate::solve::solve(&p, VAR)?;
            crate::solve::release(&solved.params);
            Expr::list(
                solved
                    .roots
                    .iter()
                    .flat_map(|r| std::iter::repeat_n(r.value.clone(), r.mult as usize))
                    .collect(),
            )
        }
        _ => return Ok(None),
    }))
}

/// The matrix whose columns span a space: a flat list is one column.
fn grid_columns(e: &Expr) -> Rows {
    match arr(e) {
        Arr::Scalar(x) => vec![vec![x]],
        Arr::Vector(v) => v.into_iter().map(|x| vec![x]).collect(),
        Arr::Matrix(m) => m,
    }
}

pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    inner(name, a).transpose()
}
