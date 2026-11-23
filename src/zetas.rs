//! The Hurwitz zeta function, polylogarithms, Fresnel integrals, the inverse error function and
//! the step functions.

use crate::hyper::{adaptive, is_int};
use crate::ntheory::bernoulli;
use crate::special::{Out, erf_any, expo, gamma_any, pi, real_arg, rgamma, size, zeta_any};
use rug::{Complex, Float};

fn ln_c(z: &Complex, wp: u32) -> Complex {
    crate::numeric::unsigned_zero(Complex::with_val(wp, z)).ln()
}

/// `sum_{k >= 0} (a + k)^(-s)` continued in `s`, by Euler-Maclaurin summation.
pub(crate) fn hurwitz(s: &Complex, a: &Complex, p: u32) -> Out {
    let (ms, ma) = (size(s), size(a));
    if !ms.is_finite() || !ma.is_finite() || ms > 1e6 || ma > 1e9 {
        return None;
    }
    if Complex::with_val(p, s - 1u32).is_zero() {
        return Some(None);
    }
    if is_int(a).is_some_and(|n| n <= 0) {
        return Some(None);
    }
    let n = (0.2 * f64::from(p) + ms + 20.0) as u32;
    // the powers of the first terms outgrow the result when re s is negative
    let growth = (-s.real().to_f64()).max(0.0) * (f64::from(n) + ma + 2.0).log2();
    if growth > 4e6 {
        return None;
    }
    let wp = p + 64 + growth as u32 + 2 * (ms.max(2.0).log2().min(1e6) as u32);
    let neg_s = Complex::with_val(wp, -s);
    let mut sum = Complex::new(wp);
    let mut base = Complex::with_val(wp, a);
    // terms left of the imaginary axis are taken off first
    let mut guard = 0;
    while base.real().is_sign_negative() || base.real().is_zero() {
        sum += (Complex::with_val(wp, &neg_s * ln_c(&base, wp))).exp();
        base += 1u32;
        guard += 1;
        if guard > 2_000_000 {
            return None;
        }
    }
    for k in 0..n {
        let x = Complex::with_val(wp, &base + k);
        sum += Complex::with_val(wp, &neg_s * ln_c(&x, wp)).exp();
    }
    let w = Complex::with_val(wp, &base + n);
    let ln_w = ln_c(&w, wp);
    let w_s = Complex::with_val(wp, &neg_s * &ln_w).exp();
    let s1 = Complex::with_val(wp, s - 1u32);
    sum += Complex::with_val(wp, &w_s * &w) / &s1;
    sum += Complex::with_val(wp, &w_s / 2u32);
    let w2 = Complex::with_val(wp, w.square_ref());
    let mut q = Complex::with_val(wp, s * &w_s) / &w / 2u32;
    let mut last = i64::MAX;
    for k in 1..=(2 * n + 64) {
        let b = Float::with_val(wp, &bernoulli(2 * k));
        let term = Complex::with_val(wp, &q * &b);
        let e = expo(&term);
        if e > last && f64::from(k) > ms {
            break;
        }
        sum += &term;
        if e < expo(&sum) - i64::from(wp) {
            break;
        }
        last = e;
        let k2 = 2 * u64::from(k);
        q = q * Complex::with_val(wp, s + (k2 - 1)) * Complex::with_val(wp, s + k2)
            / ((k2 + 1) * (k2 + 2))
            / &w2;
    }
    Some(Some(Complex::with_val(p, sum)))
}

fn polylog_series(s: &Complex, z: &Complex, wp: u32) -> Option<(Complex, i64)> {
    let neg_s = Complex::with_val(wp, -s);
    let zz = Complex::with_val(wp, z);
    let mut power = zz.clone();
    let mut sum = zz.clone();
    let mut top = expo(&sum);
    for k in 2..4_000_000u32 {
        power *= &zz;
        let weight = Complex::with_val(wp, &neg_s * Float::with_val(wp, k).ln()).exp();
        let term = Complex::with_val(wp, &power * &weight);
        sum += &term;
        let e = expo(&term);
        top = top.max(e);
        if e < expo(&sum) - i64::from(wp) - 8 && f64::from(k) > size(s) {
            return Some((sum.clone(), top - expo(&sum)));
        }
    }
    None
}

/// `Li_s(e^mu)` for `|mu| < 2 pi`: `gamma(1 - s) (-mu)^(s-1) + sum zeta(s - k) mu^k/k!`.
fn polylog_near_one(s: &Complex, mu: &Complex, wp: u32) -> Option<Option<(Complex, i64)>> {
    let whole = is_int(s).filter(|n| *n >= 1);
    let minus_mu = Complex::with_val(wp, -mu);
    let mut sum = Complex::new(wp);
    let mut top = i64::MIN;
    match whole {
        Some(n) => {
            // mu^(n-1)/(n-1)! (H_(n-1) - ln(-mu))
            let n = u32::try_from(n).ok().filter(|n| *n <= 100_000)?;
            let mut harmonic = Float::new(wp);
            let mut fact = Complex::with_val(wp, 1);
            for k in 1..n {
                harmonic += Float::with_val(wp, k).recip();
                fact = fact * mu / k;
            }
            let lead = fact * (Complex::with_val(wp, harmonic) - ln_c(&minus_mu, wp));
            top = top.max(expo(&lead));
            sum += lead;
        }
        None => {
            let one_s = Complex::with_val(wp, 1u32 - Complex::with_val(wp, s));
            let Some(g) = gamma_any(&one_s, wp)? else {
                return Some(None);
            };
            let lead = g * (Complex::with_val(wp, s - 1u32) * ln_c(&minus_mu, wp)).exp();
            top = top.max(expo(&lead));
            sum += lead;
        }
    }
    let mut power = Complex::with_val(wp, 1);
    let mut small = 0;
    for k in 0..u32::try_from(crate::settings::terms(200_000)).unwrap_or(u32::MAX) {
        if k > 0 {
            power = power * mu / k;
        }
        if whole.is_some_and(|n| i64::from(k) == n - 1) {
            continue;
        }
        let at = Complex::with_val(wp, s - k);
        let Some(zeta) = zeta_any(&at, wp)? else {
            return Some(None);
        };
        let term = Complex::with_val(wp, &power * &zeta);
        sum += &term;
        if !term.is_zero() {
            top = top.max(expo(&term));
        }
        if term.is_zero() || expo(&term) < expo(&sum) - i64::from(wp) - 8 {
            small += 1;
            if small >= 6 && f64::from(k) > size(s) {
                let lost = top - expo(&sum);
                return Some(Some((sum, lost.max(0))));
            }
        } else {
            small = 0;
        }
    }
    None
}

/// The polylogarithm `Li_s(z)`, continuous from below on its cut from 1 to infinity.
pub(crate) fn polylog(s: &Complex, z: &Complex, p: u32) -> Out {
    if z.is_zero() {
        return Some(Some(Complex::new(p)));
    }
    if s.is_zero() {
        let one_z = Complex::with_val(p + 16, 1u32 - Complex::with_val(p + 16, z));
        if one_z.is_zero() {
            return Some(None);
        }
        return Some(Some(Complex::with_val(p, z / one_z)));
    }
    if Complex::with_val(p, z - 1u32).is_zero() {
        return if *s.real() > 1u32 {
            zeta_any(s, p)
        } else {
            Some(None)
        };
    }
    let r = size(z);
    if r <= 0.7 {
        return adaptive(p, &mut |wp| polylog_series(s, z, wp).map(Some));
    }
    let mu = ln_c(z, p + 64);
    if size(&mu) <= 4.0 {
        return adaptive(p, &mut |wp| polylog_near_one(s, &ln_c(z, wp + 16), wp + 16));
    }
    // Li_s(z) = (2 pi i)^s/gamma(s) zeta(1 - s, 1/2 + ln(-z)/(2 pi i)) - e^(i pi s) Li_s(1/z)
    adaptive(p, &mut |wp| {
        let w = wp + 32;
        let ss = Complex::with_val(w, s);
        let inverse = Complex::with_val(w, z.recip_ref());
        let (inner, _) = polylog_series(&ss, &inverse, w)?;
        let turn = Complex::with_val(w, &ss * &pi(w)).mul_i(false).exp();
        let t2 = inner * turn;
        let r = rgamma(&ss, w)?;
        let t1 = if r.is_zero() {
            Complex::new(w)
        } else {
            let two_pi_i = Complex::with_val(w, (0, pi(w) * 2u32));
            let l = ln_c(&Complex::with_val(w, -z), w);
            let a = Complex::with_val(w, &l / &two_pi_i) + Float::with_val(w, 0.5);
            let one_s = Complex::with_val(w, 1u32 - Complex::with_val(w, &ss));
            let Some(h) = hurwitz(&one_s, &a, w)? else {
                return Some(None);
            };
            (Complex::with_val(w, &ss * ln_c(&two_pi_i, w))).exp() * r * h
        };
        let value = Complex::with_val(w, &t1 - &t2);
        let lost = expo(&t1).max(expo(&t2)) - expo(&value);
        Some(Some((value, lost.max(0))))
    })
}

/// The inverse of erf on the real numbers between -1 and 1.
fn erfinv(x: &Float, p: u32) -> Option<Option<Float>> {
    if x.is_zero() {
        return Some(Some(Float::new(p)));
    }
    let size = Float::with_val(x.prec(), x.abs_ref());
    if size == 1u32 {
        return Some(None);
    }
    if size > 1u32 {
        return None;
    }
    // Winitzki's approximation as the start of Newton's method
    let wide = x.prec().max(p) + 64;
    let one = Float::with_val(wide, 1);
    let l = (Float::with_val(wide, &one - x) * Float::with_val(wide, &one + x)).ln();
    let (lf, a) = (Float::with_val(64, &l), 0.147f64);
    let c = Float::with_val(64, 2.0 / (std::f64::consts::PI * a)) + Float::with_val(64, &lf / 2u32);
    let inner = (Float::with_val(64, c.square_ref()) - Float::with_val(64, &lf / a)).sqrt() - &c;
    let mut y = inner.sqrt();
    if x.is_sign_negative() {
        y = -y;
    }
    let target = p + 32;
    let mut prec = 64u32;
    let half_root_pi = |w: u32| pi(w).sqrt() / 2u32;
    // the slope e^(y^2) magnifies the rounding of erf(y) - x: that many bits more are carried
    let more =
        (Float::with_val(64, y.square_ref()).to_f64() * std::f64::consts::LOG2_E) as u32 + 16;
    for _ in 0..400 {
        let w = prec + more;
        let yy = Float::with_val(w, &y);
        let err = Float::with_val(w, yy.erf_ref()) - x;
        let slope = Float::with_val(w, yy.square_ref()).exp() * half_root_pi(w);
        let step = err * slope;
        let next = Float::with_val(w, &yy - &step);
        let settled = step.is_zero()
            || step.get_exp().unwrap_or(i32::MIN)
                < next.get_exp().unwrap_or(0) - (prec as i32) / 2 - 2;
        y = next;
        if settled {
            if prec >= target {
                // one more step at full precision squares the error away
                let yy = Float::with_val(w, &y);
                let err = Float::with_val(w, yy.erf_ref()) - x;
                let slope = Float::with_val(w, yy.square_ref()).exp() * half_root_pi(w);
                return Some(Some(Float::with_val(p, yy - err * slope)));
            }
            prec = (prec * 2).min(target);
        }
    }
    None
}

/// Fresnel's S (`sine = true`) and C: the integrals of `sin(pi t^2/2)` and `cos(pi t^2/2)`.
fn fresnel(sine: bool, z: &Complex, p: u32) -> Out {
    if z.is_zero() {
        return Some(Some(Complex::new(p)));
    }
    adaptive(p, &mut |wp| {
        let w = wp + 16;
        let half_root = Complex::with_val(w, pi(w).sqrt() / 2u32) * z;
        let plus = Complex::with_val(w, &half_root * Complex::with_val(w, (1, 1)));
        let minus = Complex::with_val(w, &half_root * Complex::with_val(w, (1, -1)));
        let (ep, em) = (erf_any(&plus, w)?, erf_any(&minus, w)?);
        let em_i = em.mul_i(false);
        let (inner, factor) = if sine {
            (
                Complex::with_val(w, &ep - &em_i),
                Complex::with_val(w, (1, 1)),
            )
        } else {
            (
                Complex::with_val(w, &ep + &em_i),
                Complex::with_val(w, (1, -1)),
            )
        };
        let lost = expo(&ep).max(expo(&em_i)) - expo(&inner);
        Some(Some((inner * factor / 4u32, lost.max(0))))
    })
}

/// Lerch's transcendent `sum_{k >= 0} z^k / (k + a)^s`: the series inside the unit circle,
/// Hurwitz's zeta function at `z = 1` and `z = -1`.
pub(crate) fn lerch(z: &Complex, s: &Complex, a: &Complex, p: u32) -> Out {
    if is_int(a).is_some_and(|n| n <= 0) {
        return Some(None);
    }
    let wp = p + 32;
    let one = Complex::with_val(wp, 1);
    if Complex::with_val(wp, z - &one).is_zero() {
        return hurwitz(s, a, p);
    }
    if Complex::with_val(wp, z + &one).is_zero() {
        // 2^(-s) (zeta(s, a/2) - zeta(s, (a + 1)/2))
        let half = Complex::with_val(wp, a / 2u32);
        let next = Complex::with_val(wp, &half + Float::with_val(wp, 0.5));
        let (Some(low), Some(high)) = (hurwitz(s, &half, wp)?, hurwitz(s, &next, wp)?) else {
            return Some(None);
        };
        let scale = (-Complex::with_val(wp, s * Float::with_val(wp, 2).ln())).exp();
        return Some(Some(Complex::with_val(p, (low - high) * scale)));
    }
    if size(z) > 0.9995 {
        return None;
    }
    let whole = is_int(s).and_then(|n| i32::try_from(n).ok());
    adaptive(p, &mut |wp| {
        let zz = Complex::with_val(wp, z);
        let mut power = Complex::with_val(wp, 1);
        let mut sum = Complex::new(wp);
        let mut top = -(1i64 << 40);
        for k in 0..u32::try_from(crate::settings::terms(4_000_000)).unwrap_or(u32::MAX) {
            let base = Complex::with_val(wp, a + k);
            // (a whole power of a real number is real: no logarithm turns it about)
            let fall = match whole {
                Some(n) => Complex::with_val(wp, rug::ops::Pow::pow(&base, -n)),
                None => (-Complex::with_val(wp, s * ln_c(&base, wp))).exp(),
            };
            let term = Complex::with_val(wp, &power * fall);
            top = top.max(expo(&term));
            sum += &term;
            power *= &zz;
            // (the terms fall like z^k once k is past the size of a and s)
            if f64::from(k) > 2.0 * (size(a) + size(s)) + 8.0
                && expo(&term) < expo(&sum) - i64::from(wp) - 8
            {
                return Some(Some((sum.clone(), (top - expo(&sum)).max(0))));
            }
            if power.is_zero() {
                return Some(Some((sum.clone(), (top - expo(&sum)).max(0))));
            }
        }
        None
    })
}

pub fn call(name: &str, a: &[Complex], p: u32) -> Out {
    let z = a.first()?;
    let x = real_arg(z);
    let real = |v: Option<Option<Float>>| -> Out { Some(v?.map(|v| Complex::with_val(p, v))) };
    match (name, a.len()) {
        ("zeta", 2) => hurwitz(&a[0], &a[1], p),
        ("polylog", 2) => polylog(&a[0], &a[1], p),
        ("lerchphi", 3) => lerch(&a[0], &a[1], &a[2], p),
        ("dirichleteta", 1) => {
            // (1 - 2^(1-s)) zeta(s), ln 2 at s = 1
            let wp = p + 32;
            let s1 = Complex::with_val(wp, 1u32 - Complex::with_val(wp, z));
            let ln2 = Float::with_val(wp, 2).ln();
            if s1.is_zero() {
                return Some(Some(Complex::with_val(p, ln2)));
            }
            let Some(zeta) = zeta_any(z, wp)? else {
                return Some(None);
            };
            let factor = Complex::with_val(wp, 1u32 - Complex::with_val(wp, &s1 * &ln2).exp());
            Some(Some(Complex::with_val(p, zeta * factor)))
        }
        ("erfinv", 1) => real(erfinv(&x?, p)),
        ("erfcinv", 1) => {
            let x = x?;
            let extra = (-x.get_exp().unwrap_or(0)).max(0) as u32;
            let inner = Float::with_val(p + 64 + extra, 1u32 - &x);
            real(erfinv(&inner, p))
        }
        ("fresnels", 1) => fresnel(true, z, p),
        ("fresnelc", 1) => fresnel(false, z, p),
        ("sinc", 1) => {
            if z.is_zero() {
                return Some(Some(Complex::with_val(p, 1)));
            }
            Some(Some(Complex::with_val(
                p,
                Complex::with_val(p + 8, z.sin_ref()) / z,
            )))
        }
        ("heaviside", 1) => {
            let x = x?;
            Some(Some(Complex::with_val(
                p,
                if x.is_zero() {
                    0.5
                } else if x.is_sign_negative() {
                    0.0
                } else {
                    1.0
                },
            )))
        }
        ("unitstep", 1) => {
            let x = x?;
            Some(Some(Complex::with_val(
                p,
                u32::from(x.is_zero() || !x.is_sign_negative()),
            )))
        }
        ("diracdelta", 1) => {
            let x = x?;
            if x.is_zero() {
                Some(None)
            } else {
                Some(Some(Complex::new(p)))
            }
        }
        _ => crate::orthopoly::numeric(name, a, p),
    }
}
