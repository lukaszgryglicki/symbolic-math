//! Elliptic integrals (Carlson's symmetric forms by duplication), Jacobi's theta and elliptic
//! functions.

use crate::hyper::{adaptive, hyp2f1};
use crate::special::{Out, expo, pi, size};
use rug::{Complex, Float};

fn root(z: &Complex, wp: u32) -> Complex {
    crate::numeric::unsigned_zero(Complex::with_val(wp, z)).sqrt()
}

/// `(r/k)^(-1/6)` times the largest distance from `a0`: the size at which the series is exact
/// to the relative error `r = 2^-wp`.
fn spread(a0: &Complex, points: &[&Complex], wp: u32, k: f64) -> Float {
    let mut far = Float::new(64);
    for x in points {
        let d = Float::with_val(64, Complex::with_val(wp, a0 - *x).abs_ref());
        if d > far {
            far = d;
        }
    }
    far * (k.ln() / 6.0 + f64::from(wp) * std::f64::consts::LN_2 / 6.0).exp()
}

/// Carlson's RF(x, y, z).
fn rf(x: &Complex, y: &Complex, z: &Complex, wp: u32) -> Option<Complex> {
    let (mut x, mut y, mut z) = (
        Complex::with_val(wp, x),
        Complex::with_val(wp, y),
        Complex::with_val(wp, z),
    );
    let a0 = Complex::with_val(wp, &x + &y) + &z;
    let a0 = a0 / 3u32;
    let q = spread(&a0, &[&x, &y, &z], wp, 1.0 / 3.0);
    let (x0, y0) = (x.clone(), y.clone());
    let mut a = a0.clone();
    let mut scale = Float::with_val(wp, 1);
    for _ in 0..10_000 {
        if Float::with_val(64, &q * &scale) < Float::with_val(64, a.abs_ref()) {
            let x1 = Complex::with_val(wp, &a0 - &x0) * &scale / &a;
            let y1 = Complex::with_val(wp, &a0 - &y0) * &scale / &a;
            let z1 = -Complex::with_val(wp, &x1 + &y1);
            let e2 = Complex::with_val(wp, &x1 * &y1) - Complex::with_val(wp, z1.square_ref());
            let e3 = Complex::with_val(wp, &x1 * &y1) * &z1;
            let series = Complex::with_val(wp, 1) - Complex::with_val(wp, &e2 / 10u32)
                + Complex::with_val(wp, &e3 / 14u32)
                + Complex::with_val(wp, e2.square_ref()) / 24u32
                - Complex::with_val(wp, &e2 * &e3) * 3u32 / 44u32;
            return Some(series / root(&a, wp));
        }
        let (sx, sy, sz) = (root(&x, wp), root(&y, wp), root(&z, wp));
        let lambda = Complex::with_val(wp, &sx * &sy)
            + Complex::with_val(wp, &sx * &sz)
            + Complex::with_val(wp, &sy * &sz);
        a = (a + &lambda) / 4u32;
        x = (x + &lambda) / 4u32;
        y = (y + &lambda) / 4u32;
        z = (z + &lambda) / 4u32;
        scale /= 4u32;
    }
    None
}

/// Carlson's RJ(x, y, z, p); RD(x, y, z) is RJ(x, y, z, z).
fn rj(x: &Complex, y: &Complex, z: &Complex, p: &Complex, wp: u32) -> Option<Complex> {
    let (mut x, mut y, mut z, mut p) = (
        Complex::with_val(wp, x),
        Complex::with_val(wp, y),
        Complex::with_val(wp, z),
        Complex::with_val(wp, p),
    );
    let a0 = (Complex::with_val(wp, &x + &y) + &z + Complex::with_val(wp, &p * 2u32)) / 5u32;
    let delta = Complex::with_val(wp, &p - &x)
        * Complex::with_val(wp, &p - &y)
        * Complex::with_val(wp, &p - &z);
    let q = spread(&a0, &[&x, &y, &z, &p], wp, 0.25);
    let (x0, y0, z0) = (x.clone(), y.clone(), z.clone());
    let mut a = a0.clone();
    let mut scale = Float::with_val(wp, 1);
    let mut sum = Complex::new(wp);
    let one = Complex::with_val(wp, 1);
    for _ in 0..10_000 {
        if Float::with_val(64, &q * &scale) < Float::with_val(64, a.abs_ref()) {
            let x1 = Complex::with_val(wp, &a0 - &x0) * &scale / &a;
            let y1 = Complex::with_val(wp, &a0 - &y0) * &scale / &a;
            let z1 = Complex::with_val(wp, &a0 - &z0) * &scale / &a;
            let p1 = -(Complex::with_val(wp, &x1 + &y1) + &z1) / 2u32;
            let xyz = Complex::with_val(wp, &x1 * &y1) * &z1;
            let p2 = Complex::with_val(wp, p1.square_ref());
            let e2 = Complex::with_val(wp, &x1 * &y1)
                + Complex::with_val(wp, &x1 * &z1)
                + Complex::with_val(wp, &y1 * &z1)
                - Complex::with_val(wp, &p2 * 3u32);
            let e3 = Complex::with_val(wp, &xyz + Complex::with_val(wp, &e2 * &p1) * 2u32)
                + Complex::with_val(wp, &p2 * &p1) * 4u32;
            let e4 = (Complex::with_val(wp, &xyz * 2u32)
                + Complex::with_val(wp, &e2 * &p1)
                + Complex::with_val(wp, &p2 * &p1) * 3u32)
                * &p1;
            let e5 = Complex::with_val(wp, &xyz * &p2);
            let series = Complex::with_val(wp, 1) - Complex::with_val(wp, &e2 * 3u32) / 14u32
                + Complex::with_val(wp, &e3 / 6u32)
                + Complex::with_val(wp, e2.square_ref()) * 9u32 / 88u32
                - Complex::with_val(wp, &e4 * 3u32) / 22u32
                - Complex::with_val(wp, &e2 * &e3) * 9u32 / 52u32
                + Complex::with_val(wp, &e5 * 3u32) / 26u32;
            let a32 = Complex::with_val(wp, &a * root(&a, wp));
            return Some(series * &scale / a32 + sum * 6u32);
        }
        let (sx, sy, sz, sp) = (root(&x, wp), root(&y, wp), root(&z, wp), root(&p, wp));
        let lambda = Complex::with_val(wp, &sx * &sy)
            + Complex::with_val(wp, &sx * &sz)
            + Complex::with_val(wp, &sy * &sz);
        let d = Complex::with_val(wp, &sp + &sx)
            * Complex::with_val(wp, &sp + &sy)
            * Complex::with_val(wp, &sp + &sz);
        // e = 4^(-3m) delta / d^2
        let cube = Float::with_val(wp, &scale * &scale) * &scale;
        let e = Complex::with_val(wp, &delta * &cube) / Complex::with_val(wp, d.square_ref());
        let shifted = Complex::with_val(wp, &one + &e);
        let rc = rf(&one, &shifted, &shifted, wp)?;
        sum += rc * &scale / d;
        a = (a + &lambda) / 4u32;
        x = (x + &lambda) / 4u32;
        y = (y + &lambda) / 4u32;
        z = (z + &lambda) / 4u32;
        p = (p + &lambda) / 4u32;
        scale /= 4u32;
    }
    None
}

fn half_pi(wp: u32) -> Complex {
    Complex::with_val(wp, pi(wp) / 2u32)
}

/// K(m) and E(m) by the arithmetic-geometric mean of 1 and `sqrt(1 - m)`; on the cut from 1 on
/// the values are those below it.
fn complete_ke(m: &Complex, wp: u32) -> Option<(Complex, Complex)> {
    let w = wp + 32;
    let one = Complex::with_val(w, 1);
    let mut a = one.clone();
    let mut b = root(&Complex::with_val(w, &one - m), w);
    if b.is_zero() {
        return None;
    }
    // 1 - sum 2^(n-1) c_n^2 with c_0^2 = m
    let mut rest = Complex::with_val(w, &one - Complex::with_val(w, m / 2u32));
    let mut weight = Float::with_val(w, 1);
    for _ in 0..400 {
        let c = Complex::with_val(w, &a - &b) / 2u32;
        let mean = Complex::with_val(w, &a + &b) / 2u32;
        let mut geo = root(&Complex::with_val(w, &a * &b), w);
        if size(&Complex::with_val(w, &mean - &geo)) > size(&Complex::with_val(w, &mean + &geo)) {
            geo = -geo;
        }
        rest -= Complex::with_val(w, c.square_ref()) * &weight;
        weight *= 2u32;
        (a, b) = (mean, geo);
        if c.is_zero() || expo(&c) < expo(&a) - i64::from(w) / 2 - 2 {
            // the next c is of the order of c^2: nothing more to add
            break;
        }
    }
    let k = half_pi(w) / &a;
    let e = Complex::with_val(w, &k * &rest);
    Some((k, e))
}

/// The complete integral K(m), with the parameter `m = k^2`.
pub(crate) fn complete_k(m: &Complex, p: u32) -> Out {
    match complete_ke(m, p) {
        Some((k, _)) => Some(Some(Complex::with_val(p, k))),
        None => Some(None),
    }
}

fn complete_e(m: &Complex, p: u32) -> Out {
    if Complex::with_val(p, m - 1u32).is_zero() {
        return Some(Some(Complex::with_val(p, 1)));
    }
    if size(m) < 1e-3 {
        // the terms of the mean cancel for a small parameter: the series is exact there
        let half = Complex::with_val(p + 16, 0.5);
        let one = Complex::with_val(p + 16, 1);
        let v = hyp2f1(&Complex::with_val(p + 16, -0.5), &half, &one, m, p + 16)?;
        return Some(v.map(|v| Complex::with_val(p, v * half_pi(p + 16))));
    }
    adaptive(p, &mut |wp| {
        let Some((k, e)) = complete_ke(m, wp) else {
            return Some(None);
        };
        // E is K times a difference: the bits the difference lost are missing
        let lost = expo(&k) - expo(&e);
        Some(Some((e, lost.max(0))))
    })
}

fn complete_pi(n: &Complex, m: &Complex, wp: u32) -> Out {
    let one = Complex::with_val(wp, 1);
    let one_n = Complex::with_val(wp, &one - n);
    if one_n.is_zero() {
        return Some(None);
    }
    let one_m = Complex::with_val(wp, &one - m);
    if one_m.is_zero() {
        return Some(None);
    }
    let zero = Complex::new(wp);
    let f = rf(&zero, &one_m, &one, wp)?;
    let j = rj(&zero, &one_m, &one, &one_n, wp)?;
    Some(Some(f + j * n / 3u32))
}

/// The incomplete integrals F (kind 0), E (1) and Pi (2) of the amplitude `phi`.
fn incomplete(kind: u8, n: Option<&Complex>, phi: &Complex, m: &Complex, p: u32) -> Out {
    adaptive(p, &mut |wp| {
        let w = wp + 24;
        // whole periods: phi = k pi + rest, with the rest between -pi/2 and pi/2
        let turns = Float::with_val(w, phi.real() / pi(w)).round();
        let rest = Complex::with_val(w, phi) - Complex::with_val(w, &turns * pi(w));
        let (s, c) = (
            Complex::with_val(w, rest.sin_ref()),
            Complex::with_val(w, rest.cos_ref()),
        );
        let one = Complex::with_val(w, 1);
        let s2 = Complex::with_val(w, s.square_ref());
        let x = Complex::with_val(w, c.square_ref());
        let y = Complex::with_val(w, &one - Complex::with_val(w, m * &s2));
        let mut value = rf(&x, &y, &one, w)? * &s;
        let mut top = expo(&value);
        if kind == 1 {
            let d = rj(&x, &y, &one, &one, w)? * Complex::with_val(w, &s2 * &s) * m / 3u32;
            top = top.max(expo(&d));
            value -= d;
        }
        if kind == 2 {
            let n = n.expect("the characteristic");
            let pole = Complex::with_val(w, &one - Complex::with_val(w, n * &s2));
            if pole.is_zero() {
                return Some(None);
            }
            let j = rj(&x, &y, &one, &pole, w)? * Complex::with_val(w, &s2 * &s) * n / 3u32;
            top = top.max(expo(&j));
            value += j;
        }
        if !turns.is_zero() {
            let Some(whole) = (match kind {
                0 => complete_k(m, w)?,
                1 => complete_e(m, w)?,
                _ => complete_pi(n.expect("the characteristic"), m, w)?,
            }) else {
                return Some(None);
            };
            let add = whole * Complex::with_val(w, &turns * 2u32);
            top = top.max(expo(&add));
            value += add;
        }
        let lost = top - expo(&value);
        Some(Some((value, lost.max(0))))
    })
}

/// Jacobi's theta functions 1 to 4 of the argument `z` and the nome `q`.
pub(crate) fn theta(kind: u8, z: &Complex, q: &Complex, p: u32) -> Out {
    if size(q) >= 1.0 {
        return None;
    }
    if q.is_zero() {
        return Some(Some(Complex::with_val(p, if kind >= 3 { 1 } else { 0 })));
    }
    adaptive(p, &mut |wp| {
        let w = wp + 16;
        let qq = Complex::with_val(w, q);
        let q2 = Complex::with_val(w, qq.square_ref());
        let e1 = Complex::with_val(w, z).mul_i(false).exp();
        let f1 = Complex::with_val(w, e1.recip_ref());
        let (e2, f2) = (
            Complex::with_val(w, e1.square_ref()),
            Complex::with_val(w, f1.square_ref()),
        );
        let odd = kind <= 2;
        let alternate = kind == 1 || kind == 4;
        // q^(n(n+1)) for the odd kinds, q^(n^2) for the even ones
        let mut power = Complex::with_val(w, 1);
        let mut step = if odd { q2.clone() } else { qq.clone() };
        let (mut e, mut f) = if odd {
            (e1.clone(), f1.clone())
        } else {
            (Complex::with_val(w, 1), Complex::with_val(w, 1))
        };
        let mut sum = Complex::new(w);
        let mut top = i64::MIN;
        let mut small = 0;
        for n in 0..2_000_000u32 {
            let wave = if kind == 1 {
                Complex::with_val(w, &e - &f)
            } else {
                Complex::with_val(w, &e + &f)
            };
            let mut term = Complex::with_val(w, &power * &wave);
            if alternate && n % 2 == 1 {
                term = -term;
            }
            if !odd && n == 0 {
                term = Complex::with_val(w, 1);
            }
            sum += &term;
            top = top.max(expo(&term));
            if !sum.is_zero() && expo(&term) < expo(&sum) - i64::from(w) - 8 {
                small += 1;
                if small >= 3 {
                    break;
                }
            } else {
                small = 0;
            }
            power *= &step;
            step *= &q2;
            e *= &e2;
            f *= &f2;
        }
        let lost = if sum.is_zero() {
            i64::from(w)
        } else {
            top - expo(&sum)
        };
        let value = match kind {
            1 => {
                let quarter = (crate::numeric::unsigned_zero(qq).ln() / 4u32).exp();
                (sum * quarter).mul_i(true)
            }
            2 => {
                let quarter = (crate::numeric::unsigned_zero(qq).ln() / 4u32).exp();
                sum * quarter
            }
            _ => sum,
        };
        Some(Some((value, lost.max(0))))
    })
}

/// The nome `exp(-pi K(1 - m)/K(m))`.
fn nome(m: &Complex, wp: u32) -> Out {
    if m.is_zero() {
        return Some(Some(Complex::new(wp)));
    }
    let one_m = Complex::with_val(wp, 1u32 - Complex::with_val(wp, m));
    if one_m.is_zero() {
        return Some(Some(Complex::with_val(wp, 1)));
    }
    let (Some(k), Some(k1)) = (complete_k(m, wp)?, complete_k(&one_m, wp)?) else {
        return Some(None);
    };
    Some(Some((-(k1 / k) * pi(wp)).exp()))
}

/// Jacobi's sn (kind 0), cn (1) and dn (2) of `u` with the parameter `m`.
fn jacobi(kind: u8, u: &Complex, m: &Complex, p: u32) -> Out {
    let wp = p + 32;
    let uu = Complex::with_val(wp, u);
    if m.is_zero() {
        return Some(Some(Complex::with_val(
            p,
            match kind {
                0 => uu.sin(),
                1 => uu.cos(),
                _ => Complex::with_val(wp, 1),
            },
        )));
    }
    if Complex::with_val(wp, m - 1u32).is_zero() {
        return Some(Some(Complex::with_val(
            p,
            match kind {
                0 => uu.tanh(),
                _ => uu.cosh().recip(),
            },
        )));
    }
    let q = nome(m, wp)??;
    if size(&q) >= 0.999_999 {
        return None;
    }
    let zero = Complex::new(wp);
    let t2 = theta(2, &zero, &q, wp)??;
    let t3 = theta(3, &zero, &q, wp)??;
    let t4 = theta(4, &zero, &q, wp)??;
    let zeta = uu / Complex::with_val(wp, t3.square_ref());
    let low = theta(4, &zeta, &q, wp)??;
    if low.is_zero() {
        return Some(None);
    }
    let value = match kind {
        0 => theta(1, &zeta, &q, wp)?? * t3 / (t2 * low),
        1 => theta(2, &zeta, &q, wp)?? * t4 / (t2 * low),
        _ => theta(3, &zeta, &q, wp)?? * t4 / (t3 * low),
    };
    Some(Some(Complex::with_val(p, value)))
}

pub fn call(name: &str, a: &[Complex], p: u32) -> Out {
    let wp = p + 16;
    match (name, a.len()) {
        ("elliptick", 1) => complete_k(&a[0], p),
        ("elliptice", 1) => complete_e(&a[0], p),
        ("elliptice", 2) => incomplete(1, None, &a[0], &a[1], p),
        ("ellipticf", 2) => incomplete(0, None, &a[0], &a[1], p),
        ("ellipticpi", 2) => Some(complete_pi(&a[0], &a[1], wp)?.map(|v| Complex::with_val(p, v))),
        ("ellipticpi", 3) => incomplete(2, Some(&a[0]), &a[1], &a[2], p),
        ("ellipticnome", 1) => Some(nome(&a[0], wp)?.map(|v| Complex::with_val(p, v))),
        ("jacobisn", 2) => jacobi(0, &a[0], &a[1], p),
        ("jacobicn", 2) => jacobi(1, &a[0], &a[1], p),
        ("jacobidn", 2) => jacobi(2, &a[0], &a[1], p),
        ("elliptictheta", 3) => {
            let kind = crate::hyper::is_int(&a[0]).filter(|k| (1..=4).contains(k))?;
            theta(kind as u8, &a[1], &a[2], p)
        }
        _ => crate::zetas::call(name, a, p),
    }
}
