//! Bessel, Hankel, Airy and Struve functions of complex order and argument.

use crate::hyper::{adaptive, hyp0f1, is_int, pfq};
use crate::numeric::SafeExp;
use crate::special::{Out, expo, pi, rgamma, size};
use rug::{Complex, Float};

fn ln_c(z: &Complex, wp: u32) -> Complex {
    crate::numeric::unsigned_zero(Complex::with_val(wp, z)).ln()
}

/// `sum_k s^k a_k(nu)/z^k` with `a_k = prod_{j<=k} (4 nu^2 - (2j-1)^2)/(8j)`, the series of
/// Hankel's expansions, up to its smallest term; `None` when that term is not below the
/// precision. `turn` is the power of `i` that `s` is.
fn hankel_sum(nu: &Complex, z: &Complex, turn: u8, wp: u32) -> Option<Complex> {
    let mu = Complex::with_val(wp, nu.square_ref()) * 4u32;
    let mut step = Complex::with_val(wp, z.recip_ref()) / 8u32;
    for _ in 0..turn {
        step = step.mul_i(false);
    }
    let mut term = Complex::with_val(wp, 1);
    let mut sum = term.clone();
    let mut last = i64::MAX;
    let order = size(nu);
    for k in 1..crate::settings::terms(4_000_000) {
        let odd = 2 * k - 1;
        let f = Complex::with_val(wp, &mu - odd * odd);
        if f.is_zero() {
            return Some(sum);
        }
        term = term * f * &step / k;
        let e = expo(&term);
        if e > last && (k as f64) > order {
            return None;
        }
        sum += &term;
        if e < expo(&sum) - i64::from(wp) - 4 {
            return Some(sum);
        }
        last = e;
    }
    None
}

/// Whether Hankel's expansions can reach `wp` bits at `z`.
fn far_enough(nu: &Complex, z: &Complex, wp: u32) -> bool {
    let r = size(z);
    r.is_finite() && r > 0.36 * f64::from(wp) + 4.0 && r > 2.0 * size(nu)
}

/// `e^(i m nu pi)` for the half turn `m = +-1`.
fn half_turn(nu: &Complex, m: i32, wp: u32) -> Complex {
    if let Some(n) = is_int(nu) {
        return Complex::with_val(wp, if n % 2 == 0 { 1 } else { -1 });
    }
    (Complex::with_val(wp, nu * &pi(wp)) * m)
        .mul_i(false)
        .cexp()
}

/// J (0), Y (1), H1 (2), H2 (3), I (4), K (5) far from the origin, by Hankel's expansions: the
/// value and the bits lost to cancellation.
fn far(kind: u8, nu: &Complex, z: &Complex, wp: u32) -> Option<(Complex, i64)> {
    if !far_enough(nu, z, wp) {
        return None;
    }
    let w = wp + 16;
    let upper = !z.imag().is_sign_negative() || z.imag().is_zero();
    if z.real().is_sign_negative() && !z.real().is_zero() {
        // the left half-plane: half a turn from the right one
        let direct = (kind == 2 && upper) || (kind == 3 && !upper);
        if !direct {
            let m: i32 = if upper { 1 } else { -1 };
            let back = Complex::with_val(w, -z);
            let turned = half_turn(nu, m, w);
            return match kind {
                0 | 4 => {
                    let (v, lost) = far(kind, nu, &back, wp)?;
                    Some((v * turned, lost))
                }
                1 | 5 => {
                    // Y_nu(z) = e^(-m nu pi i) Y_nu(w) + 2i m cos(nu pi) J_nu(w)
                    // K_nu(z) = e^(-m nu pi i) K_nu(w) - pi i m I_nu(w)
                    let (second, l2) = far(kind, nu, &back, wp)?;
                    let (first, l1) = far(kind - 1, nu, &back, wp)?;
                    let t1 = second * Complex::with_val(w, turned.recip_ref());
                    let t2 = if kind == 1 {
                        let cos = if is_int(nu).is_some() {
                            half_turn(nu, 1, w)
                        } else {
                            Complex::with_val(w, nu * &pi(w)).cos()
                        };
                        (first * cos * (2 * m)).mul_i(false)
                    } else {
                        (first * pi(w) * m).mul_i(true)
                    };
                    let sum = Complex::with_val(w, &t1 + &t2);
                    let lost = expo(&t1).max(expo(&t2)) - expo(&sum);
                    Some((sum, lost.max(0) + l1.max(l2)))
                }
                _ => {
                    let (j, l1) = far(0, nu, z, wp)?;
                    let (y, l2) = far(1, nu, z, wp)?;
                    let iy = y.mul_i(kind == 3);
                    let sum = Complex::with_val(w, &j + &iy);
                    let lost = expo(&j).max(expo(&iy)) - expo(&sum);
                    Some((sum, lost.max(0) + l1.max(l2)))
                }
            };
        }
    }
    let zz = Complex::with_val(w, z);
    let root = (ln_c(&zz, w) / -2i32).cexp();
    if kind >= 4 {
        // e^z/sqrt(2 pi z) sum (-1)^k a_k/z^k +- i e^(+-nu pi i) e^(-z)/sqrt(2 pi z) sum a_k/z^k
        let falling = hankel_sum(nu, &zz, 0, w)?;
        let small = Complex::with_val(w, -&zz).cexp() * &root;
        if kind == 5 {
            return Some((small * falling * (pi(w) / 2u32).sqrt(), 0));
        }
        let rising = hankel_sum(nu, &zz, 2, w)?;
        let front = (pi(w) * 2u32).sqrt().recip();
        let t1 = Complex::with_val(w, &zz).cexp() * &root * rising * &front;
        let m: i32 = if upper { 1 } else { -1 };
        let phase = (Complex::with_val(w, nu * &pi(w)) * m).mul_i(false).cexp();
        let t2 = (small * falling * front * phase).mul_i(!upper);
        let sum = Complex::with_val(w, &t1 + &t2);
        let lost = expo(&t1).max(expo(&t2)) - expo(&sum);
        return Some((sum, lost.max(0)));
    }
    // sqrt(2/(pi z)) e^(+-i omega) sum (+-i)^k a_k/z^k with omega = z - nu pi/2 - pi/4
    let front = root * (Float::with_val(w, 2) / pi(w)).sqrt();
    let omega = Complex::with_val(w, &zz - Complex::with_val(w, nu * &pi(w)) / 2u32) - pi(w) / 4u32;
    let h1 = |w: u32| -> Option<Complex> {
        Some(hankel_sum(nu, &zz, 1, w)? * Complex::with_val(w, &omega).mul_i(false).cexp() * &front)
    };
    let h2 = |w: u32| -> Option<Complex> {
        Some(hankel_sum(nu, &zz, 3, w)? * Complex::with_val(w, &omega).mul_i(true).cexp() * &front)
    };
    match kind {
        2 => Some((h1(w)?, 0)),
        3 => Some((h2(w)?, 0)),
        _ => {
            let (a, b) = (h1(w)?, h2(w)?);
            let sum = if kind == 0 {
                Complex::with_val(w, &a + &b) / 2u32
            } else {
                (Complex::with_val(w, &a - &b) / 2u32).mul_i(true)
            };
            let lost = expo(&a).max(expo(&b)) - expo(&sum) - 1;
            Some((sum, lost.max(0)))
        }
    }
}

/// The value of `far` to `p` bits, when the expansions can give it.
fn far_value(kind: u8, nu: &Complex, z: &Complex, p: u32) -> Option<Complex> {
    let mut wp = p + 24;
    for _ in 0..3 {
        let (v, lost) = far(kind, nu, z, wp)?;
        if lost + 16 <= i64::from(wp - p) {
            return Some(Complex::with_val(p, v));
        }
        if lost > 100_000 {
            return None;
        }
        wp = p + 40 + lost as u32;
    }
    None
}

/// `(z/2)^nu / gamma(nu + 1) 0F1(; nu + 1; -+z^2/4)`: J for `modified = false`, I otherwise.
fn bessel_ji(nu: &Complex, z: &Complex, modified: bool, p: u32) -> Out {
    if let Some(v) = far_value(if modified { 4 } else { 0 }, nu, z, p) {
        return Some(Some(v));
    }
    if let Some(n) = is_int(nu).filter(|n| *n < 0) {
        // J_(-n) = (-1)^n J_n and I_(-n) = I_n
        let v = bessel_ji(&Complex::with_val(p, n.unsigned_abs()), z, modified, p)?;
        let flip = !modified && n % 2 != 0;
        return Some(v.map(|v| if flip { -v } else { v }));
    }
    if z.is_zero() {
        return if nu.is_zero() {
            Some(Some(Complex::with_val(p, 1)))
        } else if nu.real().is_sign_positive() && !nu.real().is_zero() {
            Some(Some(Complex::new(p)))
        } else {
            Some(None)
        };
    }
    let wp = p + 32;
    let b = Complex::with_val(wp, nu + 1u32);
    let mut arg = Complex::with_val(wp, z.square_ref()) / 4u32;
    if !modified {
        arg = -arg;
    }
    let Some(f) = hyp0f1(&b, &arg, wp)? else {
        return Some(None);
    };
    let half = Complex::with_val(wp, z) / 2u32;
    let power = (Complex::with_val(wp, nu) * ln_c(&half, wp)).cexp();
    Some(Some(Complex::with_val(p, f * power * rgamma(&b, wp)?)))
}

/// Y for `modified = false`: `(J_nu cos(nu pi) - J_(-nu)) / sin(nu pi)`; K otherwise:
/// `pi/2 (I_(-nu) - I_nu) / sin(nu pi)`. A whole order is taken a hair off, with twice the bits.
fn bessel_yk(nu: &Complex, z: &Complex, modified: bool, p: u32) -> Out {
    if z.is_zero() {
        return Some(None);
    }
    if let Some(v) = far_value(if modified { 5 } else { 1 }, nu, z, p) {
        return Some(Some(v));
    }
    adaptive(p, &mut |wp| {
        let whole = is_int(nu).is_some();
        let (w2, v) = if whole {
            let w2 = 2 * wp + 64;
            let hair = Float::with_val(w2, 1) >> (wp + 8);
            (w2, Complex::with_val(w2, nu) + hair)
        } else {
            (wp + 16, Complex::with_val(wp + 16, nu))
        };
        let minus = Complex::with_val(w2, -&v);
        let Some(plus_value) = bessel_ji(&v, z, modified, w2)? else {
            return Some(None);
        };
        let Some(minus_value) = bessel_ji(&minus, z, modified, w2)? else {
            return Some(None);
        };
        let angle = Complex::with_val(w2, &v * &pi(w2));
        let sin = Complex::with_val(w2, angle.sin_ref());
        let (t1, t2, scale) = if modified {
            (
                minus_value,
                plus_value,
                Complex::with_val(w2, pi(w2) / 2u32),
            )
        } else {
            (
                plus_value * Complex::with_val(w2, angle.cos_ref()),
                minus_value,
                Complex::with_val(w2, 1),
            )
        };
        let diff = Complex::with_val(w2, &t1 - &t2);
        let mut lost = expo(&t1).max(expo(&t2)) - expo(&diff);
        if whole {
            lost -= i64::from(wp);
        }
        Some(Some((diff * scale / sin, lost.max(0))))
    })
}

/// `J + iY` for kind 1, `J - iY` for kind 2.
fn hankel(kind: u8, nu: &Complex, z: &Complex, p: u32) -> Out {
    if let Some(v) = far_value(kind + 1, nu, z, p) {
        return Some(Some(v));
    }
    adaptive(p, &mut |wp| {
        let w = wp + 16;
        let Some(j) = bessel_ji(nu, z, false, w)? else {
            return Some(None);
        };
        let Some(y) = bessel_yk(nu, z, false, w)? else {
            return Some(None);
        };
        let iy = y.mul_i(kind == 2);
        let sum = Complex::with_val(w, &j + &iy);
        let lost = expo(&j).max(expo(&iy)) - expo(&sum);
        Some(Some((sum, lost.max(0))))
    })
}

/// `sqrt(pi/(2z))` times the Bessel function of order `n + 1/2`.
fn spherical(second: bool, n: &Complex, z: &Complex, p: u32) -> Out {
    if z.is_zero() {
        return if second {
            Some(None)
        } else if n.is_zero() {
            Some(Some(Complex::with_val(p, 1)))
        } else if n.real().is_sign_positive() {
            Some(Some(Complex::new(p)))
        } else {
            Some(None)
        };
    }
    let wp = p + 16;
    let order = Complex::with_val(wp, n) + Float::with_val(wp, 0.5);
    let value = if second {
        bessel_yk(&order, z, false, wp)?
    } else {
        bessel_ji(&order, z, false, wp)?
    };
    let Some(value) = value else {
        return Some(None);
    };
    // z^(-1/2) on the branch of the power in the Bessel function: whole orders give entire functions
    let factor = (ln_c(z, wp) / -2i32).cexp() * (pi(wp) / 2u32).sqrt();
    Some(Some(Complex::with_val(p, value * factor)))
}

/// Ai or its derivative far from the origin in the sector `|arg z| <= 2 pi/3`:
/// `e^(-zeta)/(2 sqrt(pi) z^(1/4)) sum (-1)^k u_k/zeta^k` with `zeta = 2/3 z^(3/2)`.
fn airy_direct(derivative: bool, z: &Complex, w: u32) -> Option<Complex> {
    let l = ln_c(z, w);
    let zeta = Complex::with_val(w, &l * Float::with_val(w, 1.5)).cexp() * 2u32 / 3u32;
    let step = -Complex::with_val(w, zeta.recip_ref());
    // u_k = u_(k-1) (6k-5)(6k-3)(6k-1)/(216 k (2k-1)), v_k = -(6k+1)/(6k-1) u_k
    let mut u = Complex::with_val(w, 1);
    let mut sum = u.clone();
    let mut last = i64::MAX;
    let mut done = false;
    for k in 1..crate::settings::terms(4_000_000) {
        u = u * ((6 * k - 5) * (6 * k - 3) * (6 * k - 1)) / (216 * k * (2 * k - 1)) * &step;
        let term = if derivative {
            -Complex::with_val(w, &u * (6 * k + 1)) / (6 * k - 1)
        } else {
            u.clone()
        };
        let e = expo(&term);
        if e > last {
            break;
        }
        sum += &term;
        if e < expo(&sum) - i64::from(w) - 4 {
            done = true;
            break;
        }
        last = e;
    }
    if !done {
        return None;
    }
    let quarter = Complex::with_val(w, &l / 4u32).cexp();
    let front = (-zeta).cexp() / (pi(w).sqrt() * 2u32);
    Some(if derivative {
        -(sum * front * quarter)
    } else {
        sum * front / quarter
    })
}

/// Ai or its derivative far from the origin: directly, or near the negative axis by
/// `Ai(z) = -e^(-2 pi i/3) Ai(z e^(-2 pi i/3)) - e^(2 pi i/3) Ai(z e^(2 pi i/3))`.
fn airy_far(derivative: bool, z: &Complex, w: u32) -> Option<(Complex, i64)> {
    let (re, im) = (z.real().to_f64(), z.imag().to_f64());
    let wide = im.abs() > -re * 3f64.sqrt() || re >= 0.0;
    if wide {
        return Some((airy_direct(derivative, z, w)?, 0));
    }
    let turn = Complex::with_val(
        w,
        (
            Float::with_val(w, -0.5),
            Float::with_val(w, 3).sqrt() / 2u32,
        ),
    );
    let back = Complex::with_val(w, turn.conj_ref());
    let a1 = airy_direct(derivative, &Complex::with_val(w, z * &back), w)?;
    let a2 = airy_direct(derivative, &Complex::with_val(w, z * &turn), w)?;
    let (f1, f2) = if derivative {
        (
            Complex::with_val(w, back.square_ref()),
            Complex::with_val(w, turn.square_ref()),
        )
    } else {
        (back, turn)
    };
    let (t1, t2) = (a1 * f1, a2 * f2);
    let sum = -Complex::with_val(w, &t1 + &t2);
    let lost = expo(&t1).max(expo(&t2)) - expo(&sum);
    Some((sum, lost.max(0)))
}

/// Bi or its derivative far from the origin:
/// `Bi(z) = e^(i pi/6) Ai(z e^(2 pi i/3)) + e^(-i pi/6) Ai(z e^(-2 pi i/3))`.
fn airy_far_bi(derivative: bool, z: &Complex, w: u32) -> Option<(Complex, i64)> {
    let turn = Complex::with_val(
        w,
        (
            Float::with_val(w, -0.5),
            Float::with_val(w, 3).sqrt() / 2u32,
        ),
    );
    let back = Complex::with_val(w, turn.conj_ref());
    let (a1, l1) = airy_far(derivative, &Complex::with_val(w, z * &turn), w)?;
    let (a2, l2) = airy_far(derivative, &Complex::with_val(w, z * &back), w)?;
    // e^(i pi/6) = sqrt(3)/2 + i/2; with the derivative the factors turn on by e^(+-2 pi i/3)
    let sixth = Complex::with_val(
        w,
        (Float::with_val(w, 3).sqrt() / 2u32, Float::with_val(w, 0.5)),
    );
    let (f1, f2) = if derivative {
        (
            Complex::with_val(w, &sixth * &turn),
            Complex::with_val(w, sixth.conj_ref()) * &back,
        )
    } else {
        (sixth.clone(), Complex::with_val(w, sixth.conj_ref()))
    };
    let (t1, t2) = (a1 * f1, a2 * f2);
    let sum = Complex::with_val(w, &t1 + &t2);
    let lost = expo(&t1).max(expo(&t2)) - expo(&sum);
    Some((sum, lost.max(0) + l1.max(l2)))
}

/// Ai (0), Bi (1) and their derivatives (2, 3): far from the origin by the expansions in
/// `z^(-3/2)`, else from the two power series at the origin.
fn airy(kind: u8, z: &Complex, p: u32) -> Out {
    adaptive(p, &mut |wp| {
        let w = wp + 16;
        if size(z) > 6.0 {
            let far = if kind.is_multiple_of(2) {
                airy_far(kind >= 2, z, w)
            } else {
                airy_far_bi(kind >= 2, z, w)
            };
            if let Some(found) = far {
                return Some(Some(found));
            }
        }
        let third = |k: u32| Complex::with_val(w, Float::with_val(w, k) / 3u32);
        let zz = Complex::with_val(w, z);
        let z3 = Complex::with_val(w, zz.clone() * &zz * &zz) / 9u32;
        // Ai(0) = 3^(-2/3)/gamma(2/3), -Ai'(0) = 3^(-1/3)/gamma(1/3)
        let three = Float::with_val(w, 3);
        let c1 =
            rgamma(&third(2), w)? * (-(Float::with_val(w, three.ln_ref()) * 2u32 / 3u32)).exp();
        let c2 = rgamma(&third(1), w)? * (-(three.ln() / 3u32)).exp();
        let (f, g) = if kind < 2 {
            (
                hyp0f1(&third(2), &z3, w)??,
                hyp0f1(&third(4), &z3, w)?? * &zz,
            )
        } else {
            (
                hyp0f1(&third(5), &z3, w)?? * Complex::with_val(w, zz.square_ref()) / 2u32,
                hyp0f1(&third(1), &z3, w)??,
            )
        };
        let (t1, t2) = (c1 * f, c2 * g);
        let value = if kind.is_multiple_of(2) {
            Complex::with_val(w, &t1 - &t2)
        } else {
            Complex::with_val(w, &t1 + &t2) * Float::with_val(w, 3).sqrt()
        };
        let lost = expo(&t1).max(expo(&t2)) - expo(&value);
        Some(Some((value, lost.max(0))))
    })
}

/// Struve's H (`modified = false`) and L.
fn struve(nu: &Complex, z: &Complex, modified: bool, p: u32) -> Out {
    let wp = p + 32;
    let half = Float::with_val(wp, 0.5);
    let b2 = Complex::with_val(wp, nu) + Float::with_val(wp, 1.5);
    if let Some(m) = is_int(&b2).filter(|m| *m <= 0) {
        // H_(-n-1/2) = (-1)^n J_(n+1/2), L_(-n-1/2) = I_(n+1/2)
        let n = 1 - m;
        let order = Complex::with_val(wp, n) + half;
        let v = bessel_ji(&order, z, modified, p)?;
        let flip = !modified && n % 2 != 0;
        return Some(v.map(|v| if flip { -v } else { v }));
    }
    if z.is_zero() {
        return if *nu.real() > -1 {
            Some(Some(Complex::new(p)))
        } else {
            Some(None)
        };
    }
    let mut arg = Complex::with_val(wp, z.square_ref()) / 4u32;
    if !modified {
        arg = -arg;
    }
    let b1 = Complex::with_val(wp, Float::with_val(wp, 1.5));
    let one = Complex::with_val(wp, 1);
    let Some(f) = pfq(&[one], &[b1.clone(), b2.clone()], &arg, wp)? else {
        return Some(None);
    };
    let zh = Complex::with_val(wp, z) / 2u32;
    let power = ((Complex::with_val(wp, nu) + 1u32) * ln_c(&zh, wp)).cexp();
    Some(Some(Complex::with_val(
        p,
        f * power * rgamma(&b1, wp)? * rgamma(&b2, wp)?,
    )))
}

pub fn call(name: &str, a: &[Complex], p: u32) -> Out {
    match (name, a.len()) {
        ("besselj", 2) => bessel_ji(&a[0], &a[1], false, p),
        ("besseli", 2) => bessel_ji(&a[0], &a[1], true, p),
        ("bessely", 2) => bessel_yk(&a[0], &a[1], false, p),
        ("besselk", 2) => bessel_yk(&a[0], &a[1], true, p),
        ("hankel1", 2) => hankel(1, &a[0], &a[1], p),
        ("hankel2", 2) => hankel(2, &a[0], &a[1], p),
        ("sphericalbesselj", 2) => spherical(false, &a[0], &a[1], p),
        ("sphericalbessely", 2) => spherical(true, &a[0], &a[1], p),
        ("airyai", 1) => airy(0, &a[0], p),
        ("airybi", 1) => airy(1, &a[0], p),
        ("airyaiprime", 1) => airy(2, &a[0], p),
        ("airybiprime", 1) => airy(3, &a[0], p),
        ("struveh", 2) => struve(&a[0], &a[1], false, p),
        ("struvel", 2) => struve(&a[0], &a[1], true, p),
        _ => crate::elliptic::call(name, a, p),
    }
}
