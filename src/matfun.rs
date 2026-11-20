//! Functions of matrices of decimal numbers: the exponential by halving and squaring, the
//! square root by the iteration of Denman and Beavers, the logarithm by square roots down
//! to a matrix near the unit matrix. None of them needs the eigenvalues.

use crate::decomp::{self, CMat, c_unit, precision, size_of};
use crate::expr::Expr;
use crate::matrix::Rows;
use crate::pencils::{solved, times};
use rug::{Complex, Float};

type Res = Result<Expr, String>;

/// The largest sum of the sizes of a column.
fn norm(a: &CMat) -> Float {
    let prec = precision();
    let columns = a.first().map_or(0, Vec::len);
    let mut most = Float::new(prec);
    for j in 0..columns {
        let mut sum = Float::new(prec);
        for row in a {
            sum += size_of(&row[j]);
        }
        if sum > most {
            most = sum;
        }
    }
    most
}

/// `a + sign*b`.
fn sum(a: &CMat, b: &CMat, sign: i32) -> CMat {
    let prec = precision();
    a.iter()
        .zip(b)
        .map(|(r, s)| {
            r.iter()
                .zip(s)
                .map(|(x, y)| match sign < 0 {
                    true => Complex::with_val(prec, x - y),
                    false => Complex::with_val(prec, x + y),
                })
                .collect()
        })
        .collect()
}

/// Every entry times 2^by.
fn shifted(a: &CMat, by: i32) -> CMat {
    a.iter()
        .map(|r| r.iter().map(|x| x.clone() << by).collect())
        .collect()
}

fn over(a: &CMat, by: u32) -> CMat {
    let prec = precision();
    a.iter()
        .map(|r| r.iter().map(|x| Complex::with_val(prec, x / by)).collect())
        .collect()
}

/// 2^(-bits) of the working precision.
fn small(bits: u32) -> Float {
    Float::with_val(precision(), 1) >> bits
}

/// e^A: A is halved until it is small, the series is summed, the sum is squared as often.
pub(crate) fn exp(a: &CMat) -> CMat {
    let prec = precision();
    let n = a.len();
    let size = norm(a);
    let halvings = match size.is_zero() || !size.is_finite() {
        true => 0,
        false => (size.get_exp().unwrap_or(0) + 8).clamp(0, 4096),
    };
    let part = shifted(a, -halvings);
    let mut total = c_unit(n, prec);
    let mut term = c_unit(n, prec);
    for k in 1..=400u32 {
        term = over(&times(&term, &part), k);
        total = sum(&total, &term, 1);
        if norm(&term) <= small(prec + 8) {
            break;
        }
    }
    for _ in 0..halvings {
        total = times(&total, &total);
    }
    total
}

/// The square root whose eigenvalues lie right of the imaginary axis; none for a matrix
/// with an eigenvalue that is zero or below zero.
pub(crate) fn sqrt(a: &CMat) -> Option<CMat> {
    let prec = precision();
    let n = a.len();
    let unit = c_unit(n, prec);
    let (mut y, mut z) = (a.clone(), unit.clone());
    let mut last = Float::with_val(prec, f64::INFINITY);
    for round in 0..200 {
        let (y_inverse, z_inverse) = (solved(&y, &unit)?, solved(&z, &unit)?);
        let next = shifted(&sum(&y, &z_inverse, 1), -1);
        z = shifted(&sum(&z, &y_inverse, 1), -1);
        let change = norm(&sum(&next, &y, -1));
        y = next;
        if !change.is_finite() {
            return None;
        }
        // (the steps end at the root, or where rounding lets them get no smaller)
        if change <= norm(&y) * small(prec - 24) || (round > 8 && change >= last) {
            let rest = norm(&sum(&times(&y, &y), a, -1));
            if rest <= norm(a) * small(prec - 60) {
                return Some(y);
            }
            if round > 60 {
                return None;
            }
        }
        last = change;
    }
    None
}

/// The logarithm whose eigenvalues have imaginary parts between -pi and pi; none for a
/// matrix with an eigenvalue that is zero or below zero.
pub(crate) fn log(a: &CMat) -> Option<CMat> {
    let prec = precision();
    let n = a.len();
    let unit = c_unit(n, prec);
    let mut x = a.clone();
    let mut roots = 0;
    while norm(&sum(&x, &unit, -1)) > small(6) {
        if roots >= 160 {
            return None;
        }
        x = sqrt(&x)?;
        roots += 1;
    }
    // ln(1 + E) = E - E^2/2 + E^3/3 - ...
    let e = sum(&x, &unit, -1);
    let mut power = e.clone();
    let mut total = e.clone();
    for k in 2..=600u32 {
        power = times(&power, &e);
        let term = over(&power, k);
        total = sum(&total, &term, if k % 2 == 0 { -1 } else { 1 });
        if norm(&term) <= small(prec + 8) {
            break;
        }
    }
    Some(shifted(&total, roots))
}

/// A whole power, by squaring.
fn power(a: &CMat, exponent: i64) -> Option<CMat> {
    let prec = precision();
    let unit = c_unit(a.len(), prec);
    let mut base = match exponent < 0 {
        true => solved(a, &unit)?,
        false => a.clone(),
    };
    let mut left = exponent.unsigned_abs();
    let mut out = unit;
    while left > 0 {
        if left & 1 == 1 {
            out = times(&out, &base);
        }
        left >>= 1;
        if left > 0 {
            base = times(&base, &base);
        }
    }
    Some(out)
}

/// `matrixexp`, `matrixsqrt`, `matrixlog` and `matrixpower` of a square matrix of numbers
/// of which one or more are decimal; `more` is the factor of the exponential or the
/// power. None where the way of the eigenvalues has to go: other matrices, and those that
/// have no root or logarithm of this kind.
pub(crate) fn numeric(name: &str, m: &Rows, more: Option<&Expr>) -> Option<Res> {
    let n = m.len();
    let numbers = |e: &Expr| e.is_const() && !e.is_undefined() && !e.any(&|x| x.is_infinite());
    if n == 0
        || m.iter().any(|r| r.len() != n)
        || !m.iter().flatten().all(numbers)
        || !m.iter().flatten().any(Expr::has_float)
        || more.is_some_and(|e| !numbers(e))
    {
        return None;
    }
    let a = decomp::to_complex(m).ok()?;
    let prec = precision();
    let factor = |e: &Expr| crate::numeric::evalf(e, prec).ok();
    let times_number = |a: &CMat, by: &Complex| -> CMat {
        a.iter()
            .map(|r| r.iter().map(|x| Complex::with_val(prec, x * by)).collect())
            .collect()
    };
    let found = match (name, more) {
        ("matrixexp", None) => exp(&a),
        ("matrixexp", Some(t)) => exp(&times_number(&a, &factor(t)?)),
        ("matrixsqrt", None) => sqrt(&a)?,
        ("matrixlog", None) => log(&a)?,
        ("matrixpower", Some(p)) => match p.as_i64() {
            Some(whole) => power(&a, whole)?,
            None => exp(&times_number(&log(&a)?, &factor(p)?)),
        },
        _ => return None,
    };
    if found
        .iter()
        .flatten()
        .any(|z| !z.real().is_finite() || !z.imag().is_finite())
    {
        return None;
    }
    Some(Ok(decomp::from_complex(&found)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(rows: &[&[f64]]) -> CMat {
        let prec = precision();
        rows.iter()
            .map(|r| r.iter().map(|x| Complex::with_val(prec, *x)).collect())
            .collect()
    }

    fn near(a: &CMat, b: &CMat) -> bool {
        norm(&sum(a, b, -1)) <= small(100)
    }

    #[test]
    fn the_logarithm_undoes_the_exponential() {
        let a = matrix(&[&[0.5, 1.25, 0.0], &[-0.75, 0.25, 2.0], &[0.0, 0.5, -1.5]]);
        assert!(near(&log(&exp(&a)).unwrap(), &a));
        // a rotation by a quarter of a turn and back
        let turn = matrix(&[&[0.0, -1.0], &[1.0, 0.0]]);
        let quarter = log(&turn).unwrap();
        assert!(near(&exp(&quarter), &turn));
    }

    #[test]
    fn a_root_squared_is_the_matrix() {
        let a = matrix(&[&[4.0, 1.0], &[2.0, 3.0]]);
        let root = sqrt(&a).unwrap();
        assert!(near(&times(&root, &root), &a));
        // an eigenvalue below zero: no root of this kind
        assert!(sqrt(&matrix(&[&[-4.0, 0.0], &[0.0, 1.0]])).is_none());
        assert!(log(&matrix(&[&[0.0, 0.0], &[0.0, 1.0]])).is_none());
    }

    #[test]
    fn powers_by_squaring() {
        let a = matrix(&[&[1.0, 1.0], &[0.0, 1.0]]);
        assert!(near(
            &power(&a, 5).unwrap(),
            &matrix(&[&[1.0, 5.0], &[0.0, 1.0]])
        ));
        assert!(near(
            &power(&a, -2).unwrap(),
            &matrix(&[&[1.0, -2.0], &[0.0, 1.0]])
        ));
        // a defective matrix has its exponential and its logarithm too
        let e = exp(&matrix(&[&[0.0, 1.0], &[0.0, 0.0]]));
        assert!(near(&e, &a));
        assert!(near(
            &log(&a).unwrap(),
            &matrix(&[&[0.0, 1.0], &[0.0, 0.0]])
        ));
    }
}
