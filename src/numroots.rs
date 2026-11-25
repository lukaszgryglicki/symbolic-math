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

/// The size of a number as a float of 64 bits.
fn modulus(z: &Complex) -> Float {
    Float::with_val(64, z.abs_ref())
}

/// The value of `f` at `x` and its first derivative there, by differences over `h` to both
/// sides, at the precision of `x`; with `second` the second derivative too.
fn slopes(
    f: &mut dyn FnMut(&Complex) -> Option<Complex>,
    x: &Complex,
    h: &Float,
) -> Option<(Complex, Complex, Complex)> {
    let prec = x.prec().0;
    let step = Float::with_val(prec, h);
    let here = f(x)?;
    let above = f(&Complex::with_val(prec, x + &step))?;
    let below = f(&Complex::with_val(prec, x - &step))?;
    let first = Complex::with_val(prec, &above - &below) / Float::with_val(prec, &step * 2u32);
    let second = (Complex::with_val(prec, &above + &below) - Complex::with_val(prec, &here * 2u32))
        / Float::with_val(prec, step.square_ref());
    let whole = |v: &Complex| v.real().is_finite() && v.imag().is_finite();
    (whole(&here) && whole(&first) && whole(&second)).then_some((here, first, second))
}

/// The most bits that a root is followed with: more than the digits that were asked for take,
/// by as many as a root next to 0 or two roots next to each other use up.
fn most_bits(target: u32) -> u32 {
    2 * target + 4096
}

/// Whether 0 itself is a root: the function is exactly 0 there with many more bits than the
/// search has (the root of e^x = 1 + 10^-200 is next to 0 and is not 0). `None`: the function
/// has no value at 0.
fn zero_root(f: &mut dyn FnMut(&Complex) -> Option<Complex>, target: u32) -> Option<bool> {
    f(&Complex::new(most_bits(target))).map(|v| v.is_zero())
}

/// The slope of `f` at a root that was found to `digits` digits: by differences over a step
/// far below those digits, with as many more bits.
pub fn slope_at(
    f: &mut dyn FnMut(&Complex) -> Option<Complex>,
    root: &Complex,
    digits: u32,
) -> Option<Complex> {
    let target = bits_for(digits) + 48;
    let low = exponent(root)
        .saturating_neg()
        .clamp(0, i64::from(most_bits(target) - target)) as u32;
    let size = match modulus(root) {
        size if size.is_zero() => Float::with_val(64, 1),
        size => size,
    };
    let wide = Complex::with_val(target + low + target / 2 + 32, root);
    let h = Float::with_val(64, &size >> (target / 2 + 8));
    slopes(f, &wide, &h).map(|(_, slope, _)| slope)
}

/// A root that the function has several times over, about `spread` away from `x`: there the
/// secant method only creeps, and the digits of the function end long before those of the
/// root. Newton's method with the count of times (which the first two derivatives tell), at
/// as many times the precision; to `target` bits, where the search itself has `searched`.
fn repeated(
    f: &mut dyn FnMut(&Complex) -> Option<Complex>,
    x: &Complex,
    spread: &Float,
    target: u32,
    searched: u32,
) -> Option<Complex> {
    if spread.is_zero() || !spread.is_finite() {
        return None;
    }
    let bits = i64::from(target);
    let more = searched.saturating_sub(target);
    // how often: f f''/f'^2 is (m - 1)/m next to a root that counts m times
    let first = Complex::with_val(2 * target + 64 + more, x);
    let (value, slope, bend) = slopes(f, &first, &Float::with_val(64, spread >> 16u32))?;
    if value.is_zero() {
        return Some(Complex::with_val(target, x));
    }
    if slope.is_zero() {
        return None;
    }
    let share = crate::numeric::cdiv(
        &Complex::with_val(64, &value * &bend),
        &Complex::with_val(64, slope.square_ref()),
    );
    let times = crate::numeric::cdiv(&Complex::with_val(64, 1), &Complex::with_val(64, 1 - share));
    let count = times.real().to_f64().round();
    if !(1.0..=12.0).contains(&count)
        || (times.real().to_f64() - count).abs() > 0.25
        || times.imag().to_f64().abs() > 0.25
    {
        return None;
    }
    let count = count as u32;
    let prec = (count + 1) * target + 64 + more;
    let mut at = Complex::with_val(prec, x);
    let mut far = Float::with_val(64, spread);
    let mut before: Option<Float> = None;
    let mut slow = 0;
    let reach = Float::with_val(64, spread * 64u32);
    // (the steps may run out next to two roots that are not one: then this is no root)
    let mut arrived = false;
    for _ in 0..80 {
        let here = f(&at)?;
        if here.is_zero() {
            arrived = true;
            break;
        }
        // how far the root is now: the function shrank as the `count`-th power of that
        let size = modulus(&here);
        if let Some(old) = &before {
            let ratio = Float::with_val(64, &size / old).to_f64();
            let shrunk = crate::detmath::pow(ratio, 1.0 / f64::from(count));
            if !shrunk.is_finite() {
                return None;
            }
            // (creeping, or running away: not such a root)
            slow = if shrunk > 0.5 { slow + 1 } else { 0 };
            if slow >= 3 {
                return None;
            }
            far *= shrunk.max(1e-300);
        }
        before = Some(size);
        // (as near as the digits ask for: the differences below would only be rounding)
        let near = !at.is_zero()
            && far
                .get_exp()
                .is_none_or(|e| i64::from(e) < exponent(&at) - (bits + 8));
        if far.is_zero() || near {
            arrived = true;
            break;
        }
        let h = Float::with_val(64, &far >> 16u32);
        let (_, slope, _) = slopes(f, &at, &h)?;
        if slope.is_zero() {
            return None;
        }
        let step = crate::numeric::cdiv(&Complex::with_val(prec, &here * count), &slope);
        at -= &step;
        if !(at.real().is_finite() && at.imag().is_finite())
            || modulus(&Complex::with_val(64, &at - x)) > reach
        {
            return None;
        }
        if exponent(&at) < 16 - 2 * bits {
            // 0 itself, or nothing: the function only fades away towards 0
            return zero_root(f, target)?.then(|| Complex::new(target));
        }
        if step.is_zero() || exponent(&step) < exponent(&at).saturating_sub(bits + 8) {
            arrived = true;
            break;
        }
    }
    arrived.then(|| Complex::with_val(target, at))
}

/// A root as the secant method gave it, looked at once more with half as many bits again: a
/// root that counts several times was left where the digits of the function ended, with half
/// the digits of the root (or a third of them) still wrong. Such a root is found again by
/// `repeated`. `None`: the point is no root at all. The secant method also stands still
/// where it came back from a point at which the function is huge (as next to the circle
/// where x^1000 begins to count), and a step of Newton's method from there is long.
fn checked(
    f: &mut dyn FnMut(&Complex) -> Option<Complex>,
    root: Complex,
    target: u32,
    searched: u32,
) -> Option<Complex> {
    // (a root next to 0 was followed with more bits than it is told with)
    let root = Complex::with_val(target, root);
    if root.is_zero() {
        return Some(root);
    }
    let size = modulus(&root);
    let wide = Complex::with_val(searched.max(target) + target / 2 + 32, &root);
    let h = Float::with_val(64, &size >> (target / 2 + 8));
    let Some((value, slope, _)) = slopes(f, &wide, &h) else {
        return Some(root);
    };
    if value.is_zero() || slope.is_zero() {
        return Some(root);
    }
    // what one step of Newton's method would still change
    let change = modulus(&crate::numeric::cdiv(
        &Complex::with_val(64, &value),
        &Complex::with_val(64, &slope),
    ));
    if !change.is_finite() || change <= Float::with_val(64, &size >> (target - 40)) {
        return Some(root);
    }
    match repeated(
        f,
        &root,
        &Float::with_val(64, &change * 4u32),
        target,
        searched,
    ) {
        Some(better) => Some(better),
        None if change > Float::with_val(64, &size >> 8u32) => None,
        None => Some(root),
    }
}

/// Root of `f` near `start` by the secant method in the complex plane, to `digits` digits. A
/// root that counts several times is found by Newton's method with that count (`repeated`).
///
/// The search begins with few bits and goes on with more. Where the digits of the function
/// end before those of the root do - next to 0 (e^x + sin(x) = 1 + 10^-20 has the root
/// 5 10^-21, and to 96 bits the left side is known to 10^-29 only), or where two roots lie
/// next to each other (the slope between them is tiny) - the steps stop getting shorter:
/// then the search goes on with as many more bits as are missing, up to `most_bits`.
pub fn find_root(
    f: &mut dyn FnMut(&Complex) -> Option<Complex>,
    start: &Complex,
    digits: u32,
) -> Option<Complex> {
    let target = bits_for(digits) + 48;
    let most = most_bits(target);
    let mut prec = 96u32.min(target);
    let mut x0 = Complex::with_val(prec, start);
    let mut x1 = Complex::with_val(prec, &x0 * 1.0001f64) + Complex::with_val(prec, (1e-4, 1e-5));
    let mut f0 = f(&x0)?;
    let mut f1 = f(&x1)?;
    let mut settled = 0;
    // the last step was small: the iteration has arrived, as far as this precision tells
    let mut arrived = false;
    // steps in a row that were each shorter than the one before, and the last one: the mark
    // of a root that counts several times, towards which the secant method only creeps
    let mut shrinking = 0u32;
    let mut last: Option<Float> = None;
    let mut tried = false;
    // steps in a row that were short next to the point (or to 1) and not short enough: the
    // mark of digits of the function that have ended (or of a root that counts several times)
    let mut weak = 0u32;
    // 0 itself was looked at
    let mut zero_seen = false;
    // the bits that a root as small as `x` takes when the function is not small next to it
    let need = |x: &Complex| -> u32 {
        let low = exponent(x)
            .saturating_neg()
            .clamp(0, i64::from(most - target));
        target + low as u32
    };
    for _ in 0..400 {
        if !zero_seen && exponent(&x1) < 16 - 2 * i64::from(target) {
            // the iteration goes towards 0
            zero_seen = true;
            match zero_root(f, target) {
                Some(true) => return Some(Complex::new(target)),
                // a root next to 0: it is followed, with more bits
                Some(false) => {}
                // (no value there: the function only fades away towards 0, as x ln(x) does)
                None => return None,
            }
        }
        let mut small = false;
        // the digits of the function have ended: with how many bits the search goes on
        let more: Option<u32> = if (prec >= target || zero_seen) && prec < need(&x1) {
            // next to 0 that is known before the steps show it: a function as e^x - 1 has
            // as many digits fewer as its argument is small. (And where the search has come
            // down to 0, which is no root: with the first few bits a number as 1 + 10^-90 in
            // the equation is 1, and the steps would go on towards 0 for ever.)
            Some(need(&x1).saturating_add(64))
        } else if f1.is_zero() {
            // exactly 0: at the root, as far as these bits tell
            if prec >= target {
                return checked(f, x1, target, prec);
            }
            Some(target)
        } else {
            let df = Complex::with_val(prec, &f1 - &f0);
            if df.is_zero() {
                // the same value at two points: a flat function, or two points that the
                // rounding no longer tells apart at the root
                let gap = Complex::with_val(prec, &x1 - &x0);
                if arrived {
                    small = true;
                    None
                } else if exponent(&gap) < exponent(&x1).max(0) - 24 {
                    Some(prec.saturating_add(64))
                } else {
                    return None;
                }
            } else {
                let step = crate::numeric::cdiv(
                    &Complex::with_val(prec, &f1 * Complex::with_val(prec, &x1 - &x0)),
                    &df,
                );
                let x2 = Complex::with_val(prec, &x1 - &step);
                if !(x2.real().is_finite() && x2.imag().is_finite()) || exponent(&x2) > 4000 {
                    return None;
                }
                // (next to the point itself, as far as the bits that are asked for go)
                let bits = i64::from(prec.min(target));
                let length = exponent(&step);
                small = step.is_zero() || length < exponent(&x2).saturating_sub(bits - 16);
                let size = modulus(&step);
                shrinking = match &last {
                    Some(old) if size < *old && !size.is_zero() => shrinking + 1,
                    _ => 0,
                };
                last = Some(size);
                x0 = x1;
                f0 = f1;
                x1 = x2;
                f1 = f(&x1)?;
                weak = match !small && length < exponent(&x1).max(0) - 24 {
                    true => weak + 1,
                    false => 0,
                };
                // the way to a root that counts once goes through such steps in two or three
                // of them. After four, and after every four more: the value once more with
                // more bits. Where the two differ in their first digits, these bits no longer
                // tell the function, and the search goes on with more of them
                let mut wanted = None;
                if weak >= 4 && weak.is_multiple_of(4) {
                    let again = f(&Complex::with_val(prec + 64, &x1))?;
                    let gap = Complex::with_val(prec + 64, &again - &f1);
                    if !gap.is_zero() && exponent(&gap) > exponent(&again).saturating_sub(8) {
                        wanted = Some(prec.saturating_add(64));
                    }
                }
                wanted
            }
        };
        if let Some(wanted) = more {
            if prec >= most {
                return None;
            }
            let old = prec;
            prec = wanted.max((2 * prec).min(target)).min(most);
            x1 = Complex::with_val(prec, x1);
            let nudge = Float::with_val(prec, 1) >> (old / 2);
            x0 = Complex::with_val(prec, &x1 * Float::with_val(prec, &nudge + 1u32));
            if x0 == x1 {
                x0 += Complex::with_val(prec, (nudge, 0));
            }
            f0 = f(&x0)?;
            f1 = f(&x1)?;
            (arrived, settled, shrinking, last, weak) = (false, 0, 0, None, 0);
            continue;
        }
        arrived = small;
        if !small && !tried && shrinking >= 32 {
            // (once: a search that wanders has such a row of steps by chance only)
            tried = true;
            let spread = last.as_ref().map(|size| Float::with_val(64, size * 8u32));
            if let Some(root) = spread.and_then(|spread| repeated(f, &x1, &spread, target, prec)) {
                return Some(root);
            }
        }
        if small {
            if prec >= target {
                settled += 1;
                if settled >= 2 {
                    return checked(f, x1, target, prec);
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
                (arrived, shrinking, last, weak) = (false, 0, None, 0);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use rug::ops::Pow;

    /// The root of cos(x) = x to more digits than are asked for.
    const DOTTIE: &str = "0.739085133215160641655312087673873404013411758900757464965680635773284654883547594599376106931766531849801246";

    fn off(root: &Complex, re: &str, im: f64) -> f64 {
        let want = Float::with_val(256, Float::parse(re).expect("a number"));
        let gap = Complex::with_val(256, root - Complex::with_val(256, (want, im)));
        Float::with_val(64, gap.abs_ref()).to_f64()
    }

    #[test]
    fn a_root_that_counts_several_times_has_all_its_digits() {
        // (the secant method alone leaves such a root with a half or a third of its digits)
        for times in 1..=6u32 {
            let mut f = |z: &Complex| {
                let p = z.prec().0;
                let base = Complex::with_val(p, z.cos_ref()) - z;
                Some(Complex::with_val(p, base.pow(times)))
            };
            for digits in [12, 32, 60] {
                let root = find_root(&mut f, &Complex::with_val(64, (0.7, 0.0)), digits)
                    .unwrap_or_else(|| panic!("no root for the power {times}"));
                let gap = off(&root, DOTTIE, 0.0);
                assert!(
                    gap < 10f64.powi(-(digits as i32) - 1),
                    "power {times}, {digits} digits: off by {gap:e}"
                );
            }
        }
        // a root in the plane, twice: the square of sin(z) - z
        let mut f = |z: &Complex| {
            let p = z.prec().0;
            Some((Complex::with_val(p, z.sin_ref()) - z).square())
        };
        let root = find_root(&mut f, &Complex::with_val(64, (7.5, 2.7)), 32).expect("a root");
        let value = Complex::with_val(400, root.sin_ref()) - &root;
        assert!(Float::with_val(64, value.abs_ref()) < 1e-40f64, "{root}");
    }

    /// The secant method comes back from a point where the function is huge and stands still
    /// there: that place is no root.
    #[test]
    fn a_standstill_away_from_a_root_is_no_root() {
        // (e^z - 1 - z - z^1000)/z^2: huge outside the circle of radius 1
        let mut f = |z: &Complex| {
            let p = z.prec().0;
            let power = Complex::with_val(p, z.pow(1000u32));
            let value = Complex::with_val(p, z.exp_ref()) - 1u32 - z - power;
            Some(value / Complex::with_val(p, z.square_ref()))
        };
        assert!(find_root(&mut f, &Complex::with_val(64, (0.875, -0.375)), 32).is_none());
        let root = find_root(&mut f, &Complex::with_val(64, (1.0, 0.0)), 32).expect("a root");
        assert!(off(&root, "0.9996683682940640163089920346389", 0.0) < 1e-30);
    }

    #[test]
    fn a_root_at_zero_is_zero_itself() {
        for power in [1u32, 2, 5, 7] {
            let mut f = |z: &Complex| Some(Complex::with_val(z.prec(), z.pow(power)));
            let root = find_root(&mut f, &Complex::with_val(64, (0.3, 0.0)), 32);
            assert!(root.is_some_and(|z| z.is_zero()), "x^{power}");
        }
        // sin(x)^2, tan(x) - x: not powers, and yet 0 exactly
        let mut f = |z: &Complex| Some(Complex::with_val(z.prec(), z.sin_ref()).square());
        assert!(
            find_root(&mut f, &Complex::with_val(64, (0.3, 0.0)), 32).is_some_and(|z| z.is_zero())
        );
        let mut f = |z: &Complex| Some(Complex::with_val(z.prec(), z.tan_ref()) - z);
        assert!(
            find_root(&mut f, &Complex::with_val(64, (0.2, 0.0)), 32).is_some_and(|z| z.is_zero())
        );
    }

    #[test]
    fn a_place_where_the_function_only_fades_away_is_no_root() {
        // x ln(x) - sin(x) goes to 0 at 0, where it has no value
        let mut f = |z: &Complex| {
            if z.is_zero() {
                return None;
            }
            let p = z.prec().0;
            let log = Complex::with_val(p, z.ln_ref());
            Some(Complex::with_val(p, z * log) - Complex::with_val(p, z.sin_ref()))
        };
        assert!(find_root(&mut f, &Complex::with_val(64, (0.1, 0.0)), 32).is_none());
        // ... and its root that is one is found from farther right
        let root = find_root(&mut f, &Complex::with_val(64, (1.5, 0.0)), 32).expect("a root");
        assert!(off(&root, "1.7526772813628442960253420704836", 0.0) < 1e-31);
    }

    #[test]
    fn a_root_that_counts_once_is_as_before() {
        let mut f = |z: &Complex| Some(Complex::with_val(z.prec(), z.cos_ref()) - z);
        let root = find_root(&mut f, &Complex::with_val(64, (1.0, 0.0)), 50).expect("a root");
        assert!(off(&root, DOTTIE, 0.0) < 1e-51);
        // a tiny root is no creeping towards 0, and it has its digits though the function loses
        // them all at the precision of the search: e^x = 1 + 10^-50 has the root 10^-50 - ...
        let mut f = |z: &Complex| {
            let p = z.prec().0;
            let small = Float::with_val(p, Float::parse("1e-50").expect("a number"));
            Some(Complex::with_val(p, z.exp_ref()) - Complex::with_val(p, (small + 1u32, 0)))
        };
        let root = find_root(&mut f, &Complex::with_val(64, (0.1, 0.0)), 32).expect("a root");
        let want = Float::with_val(600, Float::parse("1e-50").expect("a number")).ln_1p();
        let gap = Float::with_val(64, Float::with_val(600, root.real() - &want).abs() / &want);
        assert!(gap < 1e-33f64 && root.imag().is_zero(), "{root}");
    }

    /// The size of what is left of an equation at a root, as a float of 64 bits.
    fn left(value: Complex) -> Float {
        Float::with_val(64, value.abs_ref())
    }

    #[test]
    fn a_root_next_to_zero_has_all_its_digits() {
        // e^x + sin(x) = 1 + c has the root c/2 - c^2/16 + ...; next to it the left side has
        // as many digits fewer as c is small, and with few bits 1 + c is 1
        for (c, digits) in [
            ("1e-8", 32u32),
            ("1e-20", 32),
            ("1e-200", 32),
            ("1e-20", 60),
        ] {
            let mut f = |z: &Complex| {
                let p = z.prec().0;
                let right = Float::with_val(p, Float::parse(c).expect("a number")) + 1u32;
                Some(
                    Complex::with_val(p, z.exp_ref()) + Complex::with_val(p, z.sin_ref())
                        - Complex::with_val(p, (right, 0)),
                )
            };
            let root = find_root(&mut f, &Complex::with_val(64, (0.1, 0.0)), digits)
                .unwrap_or_else(|| panic!("no root for {c}"));
            assert!(!root.is_zero(), "{c}: 0 is no root");
            // what is left of the equation there, with all the bits that it takes: 2 (x - root)
            let wide = 4000;
            let x = Complex::with_val(wide, &root);
            let small = Float::with_val(wide, Float::parse(c).expect("a number"));
            let rest = Complex::with_val(wide, x.exp_ref()) + Complex::with_val(wide, x.sin_ref())
                - 1u32
                - &small;
            let off = left(rest) / Float::with_val(64, &small);
            assert!(
                off < Float::with_val(64, Float::i_pow_u(10, digits - 1)).recip(),
                "{c}, {digits} digits: {root} is off by {off} of itself"
            );
        }
        // a root that 0 is: no number next to it is taken for it
        let mut f = |z: &Complex| Some(Complex::with_val(z.prec(), z.exp_ref()) - 1u32);
        let root = find_root(&mut f, &Complex::with_val(64, (0.1, 0.0)), 32).expect("a root");
        assert!(root.is_zero(), "{root}");
    }

    #[test]
    fn two_roots_next_to_each_other_are_told_apart() {
        // (x - 1)^2 = c e^x: the roots 1 +- sqrt(c e) (1 + ...). The slope between them is as
        // small as they are near, and the digits of the function end before those of a root
        for (c, gap) in [
            ("1e-18", 1.648_721_272_059_269e-9),
            ("1e-30", 1.648_721_270_700_129_5e-15),
            ("1e-50", 1.648_721_270_700_128_2e-25),
        ] {
            let mut f = |z: &Complex| {
                let p = z.prec().0;
                let small = Float::with_val(p, Float::parse(c).expect("a number"));
                let square = Complex::with_val(p, z - 1u32).square();
                Some(square - Complex::with_val(p, z.exp_ref()) * small)
            };
            let root = find_root(&mut f, &Complex::with_val(64, (2.0, 0.0)), 32)
                .unwrap_or_else(|| panic!("no root for {c}"));
            let away = left(Complex::with_val(400, &root - 1u32)).to_f64();
            assert!((away / gap - 1.0).abs() < 1e-12, "{c}: {root}");
            // (what is left there, over the slope 2 sqrt(c e): how far the root is off)
            let wide = Complex::with_val(1000, &root);
            let rest = f(&wide).expect("a value");
            assert!(left(rest).to_f64() / (2.0 * gap) < 1e-33, "{c}: {root}");
        }
    }

    #[test]
    fn the_slope_at_a_root_is_told_with_its_digits() {
        // the derivative of cos(x) - x at its root: -sin(root) - 1
        let mut f = |z: &Complex| Some(Complex::with_val(z.prec(), z.cos_ref()) - z);
        let root = find_root(&mut f, &Complex::with_val(64, (1.0, 0.0)), 32).expect("a root");
        let slope = slope_at(&mut f, &root, 32).expect("a slope");
        let want = -Complex::with_val(400, root.sin_ref()) - 1u32;
        assert!(
            left(Complex::with_val(400, &slope - &want)) < 1e-30f64,
            "{slope}"
        );
    }
}
