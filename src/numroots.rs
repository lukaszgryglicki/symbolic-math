//! Numeric roots: all roots of a polynomial and roots of general expressions.

use crate::numeric::bits_for;
use crate::zfactor::real_root_count;
use rug::float::Constant;
use rug::{Complex, Float};

fn horner(c: &[Complex], z: &Complex, prec: u32) -> (Complex, Complex) {
    let mut p = Complex::new(prec);
    let mut d = Complex::new(prec);
    for a in c.iter().rev() {
        d = d * z + &p;
        p = p * z + a;
    }
    (p, d)
}

fn exponent(z: &Complex) -> i64 {
    let part = |x: &Float| x.get_exp().map_or(i64::MIN, i64::from);
    part(z.real()).max(part(z.imag()))
}

/// The roots of a square-free polynomial of degree `n` by Aberth's method; `c_at` gives the
/// coefficients (ascending) at a precision.
fn aberth(
    n: usize,
    c_at: &dyn Fn(u32) -> Option<Vec<Complex>>,
    digits: u32,
) -> Option<(Vec<Complex>, u32)> {
    let target = bits_for(digits) + 64;
    let mut prec = 128u32.max(64 + 8 * n as u32).min(target.max(128));
    let c = c_at(prec)?;
    let lead = Float::with_val(prec, c[n].abs_ref());
    let mut radius = Float::with_val(prec, 0);
    for a in &c[..n] {
        let r = Float::with_val(prec, a.abs_ref()) / &lead;
        if r > radius {
            radius = r;
        }
    }
    let radius = (radius + 1u32) / 2u32;
    let two_pi = Float::with_val(prec, Constant::Pi) * 2u32;
    let mut z: Vec<Complex> = (0..n)
        .map(|k| {
            let angle = Float::with_val(prec, &two_pi * (k as f64 + 0.37)) / (n as u32);
            let (s, co) = angle.sin_cos(Float::new(prec));
            Complex::with_val(prec, (co * &radius, s * &radius))
        })
        .collect();
    let mut c = c;
    let mut budget = 400 + 60 * n;
    // the smallest of the largest steps of a round so far, and the rounds since it
    let (mut best, mut stale) = (i64::MAX, 0u32);
    loop {
        let mut moved = false;
        let mut largest = i64::MIN;
        for k in 0..n {
            let (p, d) = horner(&c, &z[k], prec);
            if p.is_zero() {
                continue;
            }
            let newton = if d.is_zero() {
                Complex::with_val(prec, (1e-3, 1e-3))
            } else {
                p / d
            };
            let mut sum = Complex::new(prec);
            for (j, zj) in z.iter().enumerate() {
                if j != k {
                    let diff = Complex::with_val(prec, &z[k] - zj);
                    if !diff.is_zero() {
                        sum += diff.recip();
                    }
                }
            }
            let denom = Complex::with_val(prec, 1) - Complex::with_val(prec, &newton * &sum);
            let step = if denom.is_zero() {
                newton
            } else {
                newton / denom
            };
            let size = exponent(&step) - exponent(&z[k]).max(-i64::from(prec));
            largest = largest.max(size);
            if size > 12 - i64::from(prec) {
                moved = true;
            }
            z[k] -= step;
        }
        // roots that lie close together settle only as far as the coefficients tell them
        // apart: there the steps stop getting smaller, and that is as near as they come
        if moved {
            if largest < best {
                (best, stale) = (largest, 0);
            } else {
                stale += 1;
            }
            if stale >= 16 && best < -i64::from(prec) / 3 {
                moved = false;
            }
        }
        if !moved {
            if prec >= target {
                break;
            }
            (best, stale) = (i64::MAX, 0);
            prec = (prec * 2).min(target);
            c = c_at(prec)?;
            for zk in z.iter_mut() {
                *zk = Complex::with_val(prec, &*zk);
            }
        }
        budget -= 1;
        if budget == 0 {
            return None;
        }
    }
    Some((z, prec))
}

/// A real part that is rounding noise next to the imaginary part is zero; with `both` the same
/// holds for the imaginary part.
fn settle(z: &mut Complex, digits: u32, both: bool) {
    let gap = i64::from(bits_for(digits)) + 40;
    let part = |x: &Float| x.get_exp().map_or(i64::MIN, i64::from);
    let (re, im) = (part(z.real()), part(z.imag()));
    if re == i64::MIN || im == i64::MIN {
        return;
    }
    if re < im - gap {
        *z = Complex::with_val(z.prec(), (0, z.imag()));
    } else if both && im < re - gap {
        *z = Complex::with_val(z.prec(), z.real());
    }
}

fn sorted(z: Vec<Complex>, digits: u32) -> Vec<Complex> {
    let out_prec = bits_for(digits) + 32;
    let mut out: Vec<Complex> = z
        .into_iter()
        .map(|v| Complex::with_val(out_prec, v))
        .collect();
    out.sort_by(|a, b| {
        let key = |v: &Complex| !v.imag().is_zero();
        key(a)
            .cmp(&key(b))
            .then_with(|| {
                a.real()
                    .partial_cmp(b.real())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                a.imag()
                    .partial_cmp(b.imag())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    out
}

/// All roots of a square-free polynomial of degree `n` with complex coefficients, which `c_at`
/// gives (ascending) at a precision. Sorted like the roots of `poly_roots`.
pub fn poly_roots_complex(
    n: usize,
    c_at: &dyn Fn(u32) -> Option<Vec<Complex>>,
    digits: u32,
) -> Option<Vec<Complex>> {
    if n == 0 {
        return Some(Vec::new());
    }
    let (mut z, _) = aberth(n, c_at, digits)?;
    z.iter_mut().for_each(|v| settle(v, digits, true));
    Some(sorted(z, digits))
}

/// All complex roots of a square-free polynomial (ascending coefficients), each to `digits` digits.
/// The real roots are exactly real. Sorted: real roots ascending, then the others.
pub fn poly_roots(coeffs: &[rug::Rational], digits: u32) -> Option<Vec<Complex>> {
    let n = coeffs.len().checked_sub(1)?;
    if n == 0 {
        return Some(Vec::new());
    }
    let c_at = |prec: u32| -> Option<Vec<Complex>> {
        Some(coeffs.iter().map(|r| Complex::with_val(prec, r)).collect())
    };
    let (mut z, prec) = aberth(n, &c_at, digits)?;
    // real roots of a real polynomial are exactly real
    let real_count = real_root_count(coeffs);
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|a, b| {
        z[*a]
            .imag()
            .clone()
            .abs()
            .partial_cmp(&z[*b].imag().clone().abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for &k in order.iter().take(real_count) {
        z[k] = Complex::with_val(prec, z[k].real());
    }
    z.iter_mut().for_each(|v| settle(v, digits, false));
    Some(sorted(z, digits))
}

/// Root of `f` near `start` by the secant method in the complex plane, to `digits` digits.
pub fn find_root(
    f: &mut dyn FnMut(&Complex) -> Option<Complex>,
    start: &Complex,
    digits: u32,
) -> Option<Complex> {
    let target = bits_for(digits) + 48;
    let mut prec = 96u32.min(target);
    let mut x0 = Complex::with_val(prec, start);
    let mut x1 = Complex::with_val(prec, &x0 * 1.0001f64) + Complex::with_val(prec, (1e-4, 1e-5));
    let mut f0 = f(&x0)?;
    let mut f1 = f(&x1)?;
    let mut settled = 0;
    for _ in 0..400 {
        if f1.is_zero() {
            if prec >= target {
                return Some(x1);
            }
            f0 = Complex::with_val(prec, (1e-30, 0));
        }
        let df = Complex::with_val(prec, &f1 - &f0);
        if df.is_zero() {
            return None;
        }
        let step = Complex::with_val(prec, &f1 * Complex::with_val(prec, &x1 - &x0)) / df;
        let x2 = Complex::with_val(prec, &x1 - &step);
        if !(x2.real().is_finite() && x2.imag().is_finite()) || exponent(&x2) > 4000 {
            return None;
        }
        let small = step.is_zero()
            || exponent(&step) < exponent(&x2).max(-i64::from(prec)) - i64::from(prec) + 16;
        x0 = x1;
        f0 = f1;
        x1 = x2;
        f1 = f(&x1)?;
        if small {
            if prec >= target {
                settled += 1;
                if settled >= 2 {
                    return Some(x1);
                }
            } else {
                let old = prec;
                prec = (prec * 2).min(target);
                x1 = Complex::with_val(prec, x1);
                let nudge = Float::with_val(prec, 1) >> (old / 2);
                x0 = Complex::with_val(prec, &x1 * (nudge + 1u32));
                if x0 == x1 {
                    x0 += Complex::with_val(prec, (Float::with_val(prec, 1) >> (old / 2), 0));
                }
                f0 = f(&x0)?;
                f1 = f(&x1)?;
            }
        }
    }
    None
}
