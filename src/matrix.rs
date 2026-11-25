//! Lists as vectors and matrices.

use crate::canon;
use crate::expr::Expr;

pub type Rows = Vec<Vec<Expr>>;

/// Rows of a rectangular list of lists.
pub fn rows(e: &Expr) -> Option<Rows> {
    let outer = e.as_list()?;
    let first = outer.first()?.as_list()?.len();
    let mut out = Vec::with_capacity(outer.len());
    for r in outer {
        let r = r.as_list()?;
        if r.len() != first || first == 0 {
            return None;
        }
        out.push(r.to_vec());
    }
    Some(out)
}

pub fn from_rows(m: Rows) -> Expr {
    Expr::list(m.into_iter().map(Expr::list).collect())
}

fn vector(e: &Expr) -> Option<Vec<Expr>> {
    let v = e.as_list()?;
    v.iter().all(|x| x.as_list().is_none()).then(|| v.to_vec())
}

fn map(e: &Expr, f: &dyn Fn(&Expr) -> Expr) -> Expr {
    match e {
        Expr::List(a) => Expr::list(a.iter().map(|x| map(x, f)).collect()),
        other => f(other),
    }
}

pub(crate) fn zip(a: &Expr, b: &Expr, f: &dyn Fn(&Expr, &Expr) -> Expr) -> Result<Expr, String> {
    match (a, b) {
        (Expr::List(x), Expr::List(y)) => {
            if x.len() != y.len() {
                return Err(format!(
                    "lists of different sizes: {} and {}",
                    x.len(),
                    y.len()
                ));
            }
            Ok(Expr::list(
                x.iter()
                    .zip(y.iter())
                    .map(|(p, q)| zip(p, q, f))
                    .collect::<Result<_, _>>()?,
            ))
        }
        (Expr::List(_), s) => Ok(map(a, &|x| f(x, s))),
        (s, Expr::List(_)) => Ok(map(b, &|x| f(s, x))),
        _ => Ok(f(a, b)),
    }
}

pub fn add(args: Vec<Expr>) -> Result<Expr, String> {
    if !args.iter().any(|a| matches!(a, Expr::List(_))) {
        return Ok(canon::add(args));
    }
    let mut it = args.into_iter();
    let mut acc = it.next().unwrap();
    for x in it {
        acc = zip(&acc, &x, &|p, q| canon::add(vec![p.clone(), q.clone()]))?;
    }
    Ok(acc)
}

fn dot(a: &[Expr], b: &[Expr]) -> Expr {
    canon::add(
        a.iter()
            .zip(b)
            .map(|(x, y)| canon::mul(vec![x.clone(), y.clone()]))
            .collect(),
    )
}

/// A row inside a row makes no matrix to calculate with: its entries are numbers and
/// expressions.
fn scalars(m: &Rows, who: &str) -> Result<(), String> {
    match m.iter().flatten().any(|x| matches!(x, Expr::List(_))) {
        true => Err(format!(
            "{who} needs a matrix of numbers and expressions: an entry is a list itself"
        )),
        false => Ok(()),
    }
}

pub fn matmul(a: &Rows, b: &Rows) -> Result<Rows, String> {
    scalars(a, "a product of matrices")?;
    scalars(b, "a product of matrices")?;
    if a.is_empty() || b.is_empty() {
        return Err("a matrix without entries cannot be multiplied".to_string());
    }
    if a[0].len() != b.len() {
        return Err(format!(
            "cannot multiply a {}x{} matrix by a {}x{} matrix",
            a.len(),
            a[0].len(),
            b.len(),
            b[0].len()
        ));
    }
    let cols: Vec<Vec<Expr>> = (0..b[0].len())
        .map(|j| b.iter().map(|r| r[j].clone()).collect())
        .collect();
    Ok(a.iter()
        .map(|r| cols.iter().map(|c| dot(r, c)).collect())
        .collect())
}

fn mul2(a: &Expr, b: &Expr) -> Result<Expr, String> {
    match (rows(a), rows(b), vector(a), vector(b)) {
        (Some(x), Some(y), _, _) => Ok(from_rows(matmul(&x, &y)?)),
        (Some(x), _, _, Some(v)) => {
            if x[0].len() != v.len() {
                return Err("matrix and vector sizes do not match".into());
            }
            Ok(Expr::list(x.iter().map(|r| dot(r, &v)).collect()))
        }
        (_, Some(y), Some(v), _) => {
            if y.len() != v.len() {
                return Err("vector and matrix sizes do not match".into());
            }
            Ok(Expr::list(
                (0..y[0].len())
                    .map(|j| dot(&v, &y.iter().map(|r| r[j].clone()).collect::<Vec<_>>()))
                    .collect(),
            ))
        }
        (_, _, Some(v), Some(w)) => {
            if v.len() != w.len() {
                return Err("vectors of different sizes".into());
            }
            Ok(dot(&v, &w))
        }
        _ => zip(a, b, &|p, q| canon::mul(vec![p.clone(), q.clone()])),
    }
}

pub fn mul(args: Vec<Expr>) -> Result<Expr, String> {
    if !args.iter().any(|a| matches!(a, Expr::List(_))) {
        return Ok(canon::mul(args));
    }
    let mut it = args.into_iter();
    let mut acc = it.next().unwrap();
    for x in it {
        acc = mul2(&acc, &x)?;
    }
    Ok(acc)
}

pub fn identity(n: usize) -> Rows {
    (0..n)
        .map(|i| (0..n).map(|j| Expr::int(i64::from(i == j))).collect())
        .collect()
}

pub fn pow(b: Expr, e: Expr) -> Result<Expr, String> {
    match (&b, &e) {
        (Expr::List(_), _) => {
            if let (Some(m), Some(n)) = (rows(&b), e.as_i64())
                && m.len() == m[0].len()
            {
                let base = if n < 0 { inverse(&m)? } else { m };
                let (mut sq, mut acc, mut k) = (base, identity(0), n.unsigned_abs());
                let mut started = false;
                while k > 0 {
                    if k & 1 == 1 {
                        acc = if started {
                            matmul(&acc, &sq)?
                        } else {
                            sq.clone()
                        };
                        started = true;
                    }
                    k >>= 1;
                    if k > 0 {
                        sq = matmul(&sq, &sq)?;
                    }
                }
                return Ok(from_rows(if started { acc } else { identity(sq.len()) }));
            }
            if rows(&b).is_some_and(|m| m.len() == m[0].len() && m.len() > 1) && e.is_number() {
                return Err("a matrix power needs a whole exponent".into());
            }
            Ok(map(&b, &|x| canon::pow(x.clone(), e.clone())))
        }
        (_, Expr::List(_)) => Ok(map(&e, &|x| canon::pow(b.clone(), x.clone()))),
        _ => Ok(canon::pow(b, e)),
    }
}

/// The size of an entry that is a number, as far as the choice of a pivot needs it.
pub(crate) fn size_of_number(e: &Expr) -> Option<rug::Float> {
    let value = match e {
        Expr::Float(f) => rug::Complex::with_val(64, &f.v),
        Expr::Num(n) => n.to_complex(64),
        _ => return None,
    };
    Some(rug::Float::with_val(64, value.abs_ref()))
}

/// The row from `rank` on whose entry in the column `col` is the pivot: the first that is
/// not zero. Among rounded numbers the largest one: a small one in front (10^-40 next to 1)
/// would take the digits of every row that it is taken from.
fn pivot_row(m: &Rows, rank: usize, col: usize, zero: &dyn Fn(&Expr) -> bool) -> Option<usize> {
    let found: Vec<usize> = (rank..m.len()).filter(|&r| !zero(&m[r][col])).collect();
    let first = *found.first()?;
    if !found.iter().any(|&r| matches!(m[r][col], Expr::Float(_))) {
        return Some(first);
    }
    let Some(sizes) = found
        .iter()
        .map(|&r| size_of_number(&m[r][col]))
        .collect::<Option<Vec<_>>>()
    else {
        return Some(first);
    };
    let mut best = 0;
    for (k, size) in sizes.iter().enumerate() {
        if *size > sizes[best] {
            best = k;
        }
    }
    Some(found[best])
}

/// The bits that a difference of rounded numbers may lose before it is rounding only: all
/// but these of the bits of its numbers.
const LEFT_OF_A_DIFFERENCE: i64 = 24;

/// `x - t` in an elimination. Where both are numbers and the difference is a rounded one, a
/// part of it that is smaller than the parts it came from by nearly all their bits is what
/// the rounding left of a zero: it is 0. (Taken for a number, it would be the pivot of a
/// matrix that has none there, and every entry after it would be made of rounding.)
fn difference(x: &Expr, t: &Expr) -> Expr {
    let v = crate::simplify::normal(&canon::sub(x.clone(), t.clone()));
    let Expr::Float(f) = &v else {
        return v;
    };
    let prec = f.v.prec().0;
    let number = |e: &Expr| match e {
        Expr::Float(g) => Some(rug::Complex::with_val(prec, &g.v)),
        Expr::Num(n) => Some(n.to_complex(prec)),
        _ => None,
    };
    let (Some(a), Some(b)) = (number(x), number(t)) else {
        return v;
    };
    let rounding = |d: &rug::Float, a: &rug::Float, b: &rug::Float| -> bool {
        let Some(top) = a.get_exp().max(b.get_exp()) else {
            return false;
        };
        d.get_exp().is_some_and(|e| {
            i64::from(e) < i64::from(top) - (i64::from(prec) - LEFT_OF_A_DIFFERENCE)
        })
    };
    let (re, im) = (
        rounding(f.v.real(), a.real(), b.real()),
        rounding(f.v.imag(), a.imag(), b.imag()),
    );
    if !re && !im {
        return v;
    }
    let part = |d: &rug::Float, gone: bool| match gone {
        true => rug::Float::new(prec),
        false => d.clone(),
    };
    let left = rug::Complex::with_val(prec, (part(f.v.real(), re), part(f.v.imag(), im)));
    if left.is_zero() {
        return Expr::zero();
    }
    Expr::float(crate::num::Flt {
        v: left,
        digits: f.digits,
    })
}

/// Gauss-Jordan elimination with exact zero tests; returns the reduced matrix, its rank and
/// the product of the pivots with the sign of the row exchanges.
pub fn eliminate(m: Rows, full: bool) -> (Rows, usize, Expr) {
    eliminate_with(m, full, &crate::simplify::is_zero)
}

/// The same with a caller's zero test (a tolerance for approximate entries).
pub fn eliminate_with(
    mut m: Rows,
    full: bool,
    zero: &dyn Fn(&Expr) -> bool,
) -> (Rows, usize, Expr) {
    if m.is_empty() {
        return (m, 0, Expr::one());
    }
    let (nr, nc) = (m.len(), m[0].len());
    let (mut rank, mut det) = (0, Expr::one());
    for col in 0..nc {
        if rank == nr {
            break;
        }
        let Some(p) = pivot_row(&m, rank, col, zero) else {
            for row in m.iter_mut().skip(rank) {
                row[col] = Expr::zero();
            }
            continue;
        };
        if p != rank {
            m.swap(p, rank);
            det = canon::neg(det);
        }
        let pivot = m[rank][col].clone();
        det = canon::mul(vec![det, pivot.clone()]);
        for x in m[rank].iter_mut() {
            *x = crate::simplify::normal(&canon::div(x.clone(), pivot.clone()));
        }
        // (1 and 0 as they are meant: a rounded number over itself may be a hair off 1, and
        // what it leaves in its column would be taken for an entry later)
        m[rank][col] = Expr::one();
        for r in 0..nr {
            if r == rank || (!full && r < rank) {
                continue;
            }
            let factor = m[r][col].clone();
            if factor.is_zero() {
                continue;
            }
            if zero(&factor) {
                m[r][col] = Expr::zero();
                continue;
            }
            let pivot_row = m[rank].clone();
            for (x, p) in m[r].iter_mut().zip(&pivot_row) {
                *x = difference(x, &canon::mul(vec![factor.clone(), p.clone()]));
            }
            m[r][col] = Expr::zero();
        }
        rank += 1;
    }
    (m, rank, det)
}

pub fn det(m: &Rows) -> Result<Expr, String> {
    if m.iter().any(|row| row.len() != m.len()) {
        return Err("the determinant needs a square matrix".into());
    }
    scalars(m, "the determinant")?;
    let n = m.len();
    // (rounded numbers go by the elimination, which knows what the rounding left of a zero)
    let rounded = m.iter().flatten().any(|e| matches!(e, Expr::Float(_)));
    if n <= 3 && !rounded {
        let d = |i: usize, j: usize| m[i][j].clone();
        let e = match n {
            1 => d(0, 0),
            2 => d(0, 0) * d(1, 1) - d(0, 1) * d(1, 0),
            _ => {
                d(0, 0) * (d(1, 1) * d(2, 2) - d(1, 2) * d(2, 1))
                    - d(0, 1) * (d(1, 0) * d(2, 2) - d(1, 2) * d(2, 0))
                    + d(0, 2) * (d(1, 0) * d(2, 1) - d(1, 1) * d(2, 0))
            }
        };
        return Ok(crate::simplify::normal(&e));
    }
    let (_, rank, det) = eliminate(m.clone(), false);
    Ok(if rank < n {
        Expr::zero()
    } else {
        crate::simplify::normal(&det)
    })
}

pub fn inverse(m: &Rows) -> Result<Rows, String> {
    let n = m.len();
    if m.iter().any(|row| row.len() != n) {
        return Err("the inverse needs a square matrix".into());
    }
    scalars(m, "the inverse")?;
    let wide: Rows = m
        .iter()
        .zip(identity(n))
        .map(|(r, i)| r.iter().cloned().chain(i).collect())
        .collect();
    let (reduced, _, _) = eliminate(wide, true);
    for (i, row) in reduced.iter().enumerate() {
        if !(0..n).all(|j| {
            if i == j {
                row[j].is_one()
            } else {
                row[j].is_zero()
            }
        }) {
            return Err("the matrix is singular".into());
        }
    }
    Ok(reduced.into_iter().map(|r| r[n..].to_vec()).collect())
}

pub fn transpose(m: &Rows) -> Rows {
    // (no rows: no columns)
    (0..m.first().map_or(0, Vec::len))
        .map(|j| m.iter().map(|r| r[j].clone()).collect())
        .collect()
}

pub fn trace(m: &Rows) -> Result<Expr, String> {
    if m.len() != m[0].len() {
        return Err("the trace needs a square matrix".into());
    }
    Ok(canon::add((0..m.len()).map(|i| m[i][i].clone()).collect()))
}

/// Basis of the null space from the reduced row echelon form.
pub fn nullspace_with(m: &Rows, zero: &dyn Fn(&Expr) -> bool) -> Vec<Vec<Expr>> {
    let (r, rank, _) = eliminate_with(m.clone(), true, zero);
    let nc = m[0].len();
    let pivots: Vec<usize> = (0..rank)
        .filter_map(|i| (0..nc).find(|&j| !r[i][j].is_zero()))
        .collect();
    (0..nc)
        .filter(|j| !pivots.contains(j))
        .map(|free| {
            let mut v = vec![Expr::zero(); nc];
            v[free] = Expr::one();
            for (i, &p) in pivots.iter().enumerate() {
                v[p] = crate::simplify::normal(&canon::neg(r[i][free].clone()));
            }
            v
        })
        .collect()
}

pub fn nullspace(m: &Rows) -> Vec<Vec<Expr>> {
    nullspace_with(m, &crate::simplify::is_zero)
}

/// Coefficients of `det(x I - A)` from the highest power down (Faddeev-LeVerrier).
pub fn charpoly_coefficients(a: &Rows) -> Result<Vec<Expr>, String> {
    let n = a.len();
    if n != a[0].len() {
        return Err("the characteristic polynomial needs a square matrix".into());
    }
    let mut coeffs = vec![Expr::one()];
    let mut mk = identity(n);
    for k in 1..=n {
        let am: Rows = matmul(a, &mk)?
            .into_iter()
            .map(|r| r.iter().map(crate::simplify::normal).collect())
            .collect();
        let c = crate::simplify::normal(&canon::div(canon::neg(trace(&am)?), Expr::int(k as i64)));
        mk = am;
        for (i, row) in mk.iter_mut().enumerate() {
            row[i] = crate::simplify::normal(&canon::add(vec![row[i].clone(), c.clone()]));
        }
        coeffs.push(c);
    }
    Ok(coeffs)
}

pub fn charpoly(a: &Rows, x: &Expr) -> Result<Expr, String> {
    let c = charpoly_coefficients(a)?;
    let n = c.len() - 1;
    Ok(canon::add(
        c.into_iter()
            .enumerate()
            .map(|(k, c)| canon::mul(vec![c, canon::powi(x.clone(), (n - k) as i64)]))
            .collect(),
    ))
}

pub fn cross(a: &[Expr], b: &[Expr]) -> Result<Expr, String> {
    if a.len() != 3 || b.len() != 3 {
        return Err("the vector product needs two vectors of three components".into());
    }
    let d = |i: usize, j: usize| crate::simplify::normal(&(&(&a[i] * &b[j]) - &(&a[j] * &b[i])));
    Ok(Expr::list(vec![d(1, 2), d(2, 0), d(0, 1)]))
}

pub fn dot_product(a: &[Expr], b: &[Expr]) -> Result<Expr, String> {
    if a.len() != b.len() {
        return Err("the scalar product needs vectors of the same size".into());
    }
    Ok(dot(a, b))
}

/// Solution of `A x = b`; free unknowns appear as `t1, t2, ...`.
pub fn linsolve(a: &Rows, b: &[Expr]) -> Result<Expr, String> {
    if a.len() != b.len() {
        return Err("linsolve(A, b): b needs one entry for each row of A".into());
    }
    let nc = a[0].len();
    let wide: Rows = a
        .iter()
        .zip(b)
        .map(|(r, y)| {
            r.iter()
                .cloned()
                .chain(std::iter::once(y.clone()))
                .collect()
        })
        .collect();
    let (r, rank, _) = eliminate(wide, true);
    let mut pivots = Vec::new();
    for row in r.iter().take(rank) {
        match (0..nc).find(|&j| !row[j].is_zero()) {
            Some(j) => pivots.push(j),
            None => return Err("the system has no solution".into()),
        }
    }
    let mut x = vec![Expr::zero(); nc];
    let mut free = 0;
    for (j, slot) in x.iter_mut().enumerate() {
        if !pivots.contains(&j) {
            free += 1;
            *slot = Expr::sym(&format!("t{free}"));
        }
    }
    for (i, &p) in pivots.iter().enumerate() {
        let mut terms = vec![r[i][nc].clone()];
        for j in (0..nc).filter(|j| !pivots.contains(j)) {
            terms.push(canon::neg(canon::mul(vec![r[i][j].clone(), x[j].clone()])));
        }
        x[p] = crate::simplify::normal(&canon::add(terms));
    }
    Ok(Expr::list(x))
}

/// `f(A)` for a function of one variable: the polynomial that agrees with `f` at the
/// eigenvalues (with derivatives at repeated ones), taken at the matrix.
pub fn function_of(a: &Rows, f: &Expr, var: &str) -> Result<Rows, String> {
    let n = a.len();
    if n == 0 || a.iter().any(|r| r.len() != n) {
        return Err("a function of a matrix needs a square matrix".into());
    }
    let z = Expr::sym("#z");
    let solved = crate::solve::solve(&charpoly(a, &z)?, "#z")?;
    crate::solve::release(&solved.params);
    if solved.roots.iter().any(|r| r.approx)
        || solved.roots.iter().map(|r| r.mult as usize).sum::<usize>() != n
    {
        return Err("the eigenvalues were not found in exact form".into());
    }
    // conditions on the coefficients c_0..c_(n-1) of the polynomial
    let (mut rows, mut right) = (Vec::new(), Vec::new());
    for root in &solved.roots {
        let mut derived = f.clone();
        for j in 0..root.mult as usize {
            let value = crate::canon::subst_sym(&derived, var, &root.value);
            if value.is_undefined() || value.is_infinite() {
                return Err(format!(
                    "the function has no value at the eigenvalue {}",
                    root.value
                ));
            }
            right.push(value);
            rows.push(
                (0..n)
                    .map(|k| {
                        if k < j {
                            Expr::zero()
                        } else {
                            // (z^k)^(j) = k!/(k - j)! z^(k - j)
                            let falling: i64 = ((k - j + 1)..=k).map(|x| x as i64).product();
                            if k == j {
                                Expr::int(falling)
                            } else {
                                canon::mul(vec![
                                    Expr::int(falling),
                                    canon::powi(root.value.clone(), (k - j) as i64),
                                ])
                            }
                        }
                    })
                    .collect::<Vec<Expr>>(),
            );
            derived = crate::diff::diff(&derived, var);
        }
    }
    let c = linsolve(&rows, &right)?;
    let c = c
        .as_list()
        .ok_or("the function of the matrix was not found")?
        .to_vec();
    let mut power = identity(n);
    let mut total = vec![vec![Expr::zero(); n]; n];
    for ck in &c {
        for i in 0..n {
            for j in 0..n {
                total[i][j] = canon::add(vec![
                    total[i][j].clone(),
                    canon::mul(vec![ck.clone(), power[i][j].clone()]),
                ]);
            }
        }
        power = matmul(&power, a)?;
    }
    // e^(i t) reads better as cos(t) + i sin(t) when that is shorter
    let tidy = |e: &Expr| {
        let plain = crate::simplify::simplify(e);
        let circular = crate::simplify::simplify(&crate::expand::expand(
            &crate::integrate::exp_to_trig(&plain),
        ));
        if circular.to_string().len() < plain.to_string().len() {
            circular
        } else {
            plain
        }
    };
    Ok(total
        .into_iter()
        .map(|r| r.iter().map(tidy).collect())
        .collect())
}

/// Orthogonal vectors with the same spans as the given ones (not normalized).
pub fn gram_schmidt(vectors: &Rows) -> Result<Rows, String> {
    let mut out: Rows = Vec::new();
    for v in vectors {
        let mut w = v.clone();
        for u in &out {
            if u.len() != v.len() {
                return Err("the vectors have different lengths".into());
            }
            let scale = canon::div(dot_product(v, u)?, dot_product(u, u)?);
            for (x, y) in w.iter_mut().zip(u) {
                *x = canon::sub(x.clone(), canon::mul(vec![scale.clone(), y.clone()]));
            }
        }
        let w: Vec<Expr> = w.iter().map(crate::simplify::simplify).collect();
        if w.iter().any(|x| !x.is_zero()) {
            out.push(w);
        }
    }
    Ok(out)
}

/// The lower triangular `L` with `L L^T = A` for a symmetric positive definite matrix.
pub fn cholesky(a: &Rows) -> Result<Rows, String> {
    let n = a.len();
    if a.iter().any(|r| r.len() != n) {
        return Err("cholesky needs a square matrix".into());
    }
    let mut l = vec![vec![Expr::zero(); n]; n];
    for i in 0..n {
        for j in 0..=i {
            let sum = canon::add(
                (0..j)
                    .map(|k| canon::mul(vec![l[i][k].clone(), l[j][k].clone()]))
                    .collect(),
            );
            let rest = crate::simplify::simplify(&canon::sub(a[i][j].clone(), sum));
            l[i][j] = if i == j {
                if crate::numeric::sign(&rest).is_some_and(|s| s <= 0) {
                    return Err("the matrix is not positive definite".into());
                }
                canon::sqrt(rest)
            } else {
                crate::simplify::simplify(&canon::div(rest, l[j][j].clone()))
            };
        }
    }
    Ok(l)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rounded number of `prec` bits.
    fn rounded(text: &str, prec: u32) -> Expr {
        let value = rug::Float::with_val(prec, rug::Float::parse(text).expect("a number"));
        crate::numeric::to_float_expr(rug::Complex::with_val(prec, value), 32)
    }

    fn matrix_of(rows: &[&[&str]], prec: u32) -> Rows {
        rows.iter()
            .map(|row| row.iter().map(|x| rounded(x, prec)).collect())
            .collect()
    }

    #[test]
    fn the_column_of_a_pivot_is_1_and_0_as_they_are_meant() {
        // the equation of Lyapunov of a balanced system: with some numbers of bits a pivot
        // over itself is a hair off 1, and what it left in its column was taken for an entry
        let m: &[&[&str]] = &[
            &[
                "-0.8171793746730032",
                "0.9701425001453319",
                "0.9701425001453319",
                "0",
            ],
            &["-0.9701425001453319", "-3", "0", "0.9701425001453319"],
            &["-0.9701425001453319", "0", "-3", "0.9701425001453319"],
            &[
                "0",
                "-0.9701425001453319",
                "-0.9701425001453319",
                "-5.182820625326997",
            ],
        ];
        for prec in (64..400).step_by(3) {
            let (reduced, rank, _) = eliminate(matrix_of(m, prec), true);
            assert_eq!(rank, 4, "{prec} bits");
            for (i, row) in reduced.iter().enumerate() {
                for (j, x) in row.iter().enumerate() {
                    assert!(
                        if i == j { x.is_one() } else { x.is_zero() },
                        "{prec} bits: {x} at {i}, {j}"
                    );
                }
            }
            let right: Vec<Expr> = ["1", "2", "3", "4"]
                .iter()
                .map(|x| rounded(x, prec))
                .collect();
            let solved = linsolve(&matrix_of(m, prec), &right).expect("one solution");
            assert!(!solved.to_string().contains('t'), "{prec} bits: {solved}");
        }
    }

    #[test]
    fn among_rounded_numbers_the_largest_is_the_pivot() {
        // (the first one that was not 0 was taken: 10^-40 in front took the digits of every
        // row, the inverse was "singular" and the solution was 0.333984375)
        let m = matrix_of(&[&["1e-40", "1"], &["3", "1"]], 150);
        let inv = inverse(&m).expect("an inverse");
        let wanted = [
            [
                "-0.33333333333333333333333333333333",
                "0.33333333333333333333333333333333",
            ],
            ["1", "-3.3333333333333333333333333333333e-41"],
        ];
        for (row, texts) in inv.iter().zip(wanted) {
            for (x, text) in row.iter().zip(texts) {
                assert_eq!(x.to_string(), text);
            }
        }
        let solved = linsolve(&m, &[Expr::int(1), Expr::int(2)]).expect("a solution");
        assert_eq!(
            solved.to_string(),
            "[0.33333333333333333333333333333333, 1]"
        );
        // exact numbers keep the first pivot that is not 0, as they did
        let exact = vec![
            vec![Expr::rational(rug::Rational::from((1, 1000))), Expr::int(1)],
            vec![Expr::int(3), Expr::int(1)],
        ];
        let (_, rank, det) = eliminate(exact, false);
        assert_eq!((rank, det.to_string().as_str()), (2, "-2999/1000"));
    }

    #[test]
    fn what_the_rounding_leaves_of_a_zero_is_no_pivot() {
        for prec in [100u32, 150, 173, 200, 256] {
            let third = |k: u32| {
                let value = rug::Float::with_val(prec, k) / 3u32;
                crate::numeric::to_float_expr(rug::Complex::with_val(prec, value), 32)
            };
            // rows 1 and 2 are the same but for a factor: no inverse, determinant 0, rank 2
            let m = vec![
                vec![third(3), third(6), third(9)],
                vec![third(1), third(2), third(3)],
                vec![third(15), third(18), third(24)],
            ];
            assert!(inverse(&m).is_err(), "{prec} bits");
            assert!(det(&m).expect("a determinant").is_zero(), "{prec} bits");
            assert_eq!(eliminate(m, false).1, 2, "{prec} bits");
        }
        // a small number that stands in the matrix is one
        let m = matrix_of(&[&["1", "0"], &["0", "1e-60"]], 150);
        assert_eq!(eliminate(m.clone(), false).1, 2);
        assert_eq!(inverse(&m).expect("an inverse")[1][1].to_string(), "1e60");
    }

    #[test]
    fn a_matrix_without_entries_is_no_trap() {
        let (reduced, rank, det) = eliminate(Vec::new(), true);
        assert!(reduced.is_empty());
        assert_eq!((rank, det), (0, Expr::one()));
        assert!(matmul(&Vec::new(), &Vec::new()).is_err());
        let one = vec![vec![Expr::int(2)]];
        assert!(matmul(&one, &Vec::new()).is_err());
        assert_eq!(
            matmul(&one, &one).expect("a product"),
            vec![vec![Expr::int(4)]]
        );
    }
}
