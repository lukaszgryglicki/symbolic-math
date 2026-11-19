//! Numeric integration by the tanh-sinh rule.

use crate::numeric::bits_for;
use rug::float::Constant;
use rug::{Complex, Float};

/// Integral of `f` over `[a, b]`; integrable singularities at the ends are allowed.
pub fn quad(
    f: &mut dyn FnMut(&Float) -> Option<Complex>,
    a: &Float,
    b: &Float,
    digits: u32,
) -> Option<Complex> {
    let target = i64::from(bits_for(digits));
    let prec = bits_for(digits) + 48;
    let half = Float::with_val(prec, b - a) / 2u32;
    let mid = Float::with_val(prec, a + b) / 2u32;
    if half.is_zero() {
        return Some(Complex::new(prec));
    }
    let half_pi = Float::with_val(prec, Constant::Pi) / 2u32;
    // node at parameter t: offset from the nearer end and weight
    let node = |t: &Float| -> (Float, Float) {
        let s = Float::with_val(prec, &half_pi * t.clone().sinh());
        let e = Float::with_val(prec, &s * 2u32).exp();
        let y = Float::with_val(prec, 2) / (e + 1u32);
        let cosh = s.cosh();
        let w = Float::with_val(prec, &half_pi * t.clone().cosh()) / cosh.square();
        (y, w)
    };
    let mut sum = f(&mid)? * &half_pi;
    let mut eval = |offset: &Float, right: bool| -> Option<Option<Complex>> {
        let x = if right {
            Float::with_val(prec, b - Float::with_val(prec, &half * offset))
        } else {
            Float::with_val(prec, a + Float::with_val(prec, &half * offset))
        };
        if x == *a || x == *b {
            return Some(None);
        }
        Some(f(&x))
    };
    let exponent = |z: &Complex| -> i64 {
        let part = |x: &Float| x.get_exp().map_or(-(1 << 40), i64::from);
        part(z.real()).max(part(z.imag()))
    };
    let mut step = Float::with_val(prec, 1);
    let mut previous: Option<Complex> = None;
    for level in 0..14 {
        let mut k = 1u32;
        let mut quiet = 0;
        loop {
            if level == 0 || k % 2 == 1 {
                let t = Float::with_val(prec, &step * k);
                let (y, w) = node(&t);
                if w.is_zero() || y.is_zero() {
                    break;
                }
                let mut contribution = Complex::new(prec);
                let mut any = false;
                for right in [true, false] {
                    match eval(&y, right)? {
                        Some(v) => {
                            contribution += v;
                            any = true;
                        }
                        None => {
                            if y.get_exp().map_or(0, i64::from) > -16 {
                                return None;
                            }
                        }
                    }
                }
                let term = contribution * &w;
                if !any || term.is_zero() || exponent(&term) < exponent(&sum) - target - 16 {
                    quiet += 1;
                    if quiet >= 3 {
                        break;
                    }
                } else {
                    quiet = 0;
                }
                sum += term;
            }
            k += 1;
            if k > 200_000 {
                return None;
            }
        }
        let estimate = Complex::with_val(prec, &sum * &step) * &half;
        if let Some(old) = &previous {
            let diff = Complex::with_val(prec, &estimate - old);
            if diff.is_zero() || exponent(&diff) < exponent(&estimate) - target {
                return Some(estimate);
            }
        }
        previous = Some(estimate);
        step /= 2u32;
    }
    None
}
