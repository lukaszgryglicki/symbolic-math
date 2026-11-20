//! Special functions for the numeric evaluator, for real and complex arguments.

use crate::ntheory::bernoulli;
use rug::float::Constant;
use rug::{Complex, Float, Integer};
use std::f64::consts::{LN_2, LOG2_E};

/// Outer `None`: the function (or this argument) is not supported; inner `None`: pole.
pub(crate) type Out = Option<Option<Complex>>;

pub(crate) fn real_arg(z: &Complex) -> Option<Float> {
    z.imag().is_zero().then(|| z.real().clone())
}

fn real(p: u32, x: Float) -> Option<Complex> {
    x.is_finite().then(|| Complex::with_val(p, x))
}

pub(crate) fn expo(z: &Complex) -> i64 {
    let e = |x: &Float| x.get_exp().map_or(-(1i64 << 40), i64::from);
    e(z.real()).max(e(z.imag()))
}

pub(crate) fn size(z: &Complex) -> f64 {
    Float::with_val(64, z.abs_ref()).to_f64()
}

pub(crate) fn pi(p: u32) -> Float {
    Float::with_val(p, Constant::Pi)
}

pub(crate) fn euler(p: u32) -> Float {
    Float::with_val(p, Constant::Euler)
}

pub(crate) fn is_pole(z: &Complex) -> bool {
    z.imag().is_zero()
        && z.real().is_integer()
        && (z.real().is_zero() || z.real().is_sign_negative())
}

/// How far to move `z` to the right before an asymptotic series reaches `wp` bits.
fn shift(z: &Complex, wp: u32, extra: u32) -> Option<u32> {
    let r = 0.12 * f64::from(wp) + 8.0 + f64::from(extra);
    let x = z.real().to_f64();
    if x >= 0.0 && size(z) >= r {
        return Some(0);
    }
    let n = (r - x).ceil();
    (n <= 2e6).then_some(n.max(0.0) as u32)
}

/// `first * sum_k B_2k c(k) w^(-2(k-1))`: the tail of an asymptotic series, up to its smallest term.
fn tail(w: &Complex, wp: u32, first: Complex, coef: &mut dyn FnMut(u32) -> Float) -> Complex {
    let step = Complex::with_val(wp, w.square_ref()).recip();
    let mut power = first;
    let mut sum = Complex::new(wp);
    let mut last = i64::MAX;
    for k in 1..=wp {
        let b = Float::with_val(wp, &bernoulli(2 * k)) * coef(k);
        let term = Complex::with_val(wp, &power * &b);
        let e = expo(&term);
        if e > last {
            break;
        }
        sum += &term;
        if e < expo(&sum) - i64::from(wp) {
            break;
        }
        last = e;
        power *= &step;
    }
    sum
}

/// Stirling's series for `ln gamma(w)`, for `w` far from the origin.
fn stirling(w: &Complex, wp: u32) -> Complex {
    let ln_w = Complex::with_val(wp, w.ln_ref());
    let half = Float::with_val(wp, 0.5);
    let mut s = Complex::with_val(wp, w - &half) * ln_w - w + (pi(wp) * 2u32).ln() / 2u32;
    s += tail(w, wp, Complex::with_val(wp, w.recip_ref()), &mut |k| {
        let k = u64::from(k);
        Float::with_val(wp, 1) / (2 * k * (2 * k - 1))
    });
    s
}

fn gamma_bits(z: &Complex, p: u32) -> u32 {
    // (a size beyond every number cannot ask for bits without end)
    p + 64 + 2 * (size(z).max(2.0).log2().min(1e6) as u32)
}

fn gamma(z: &Complex, p: u32) -> Out {
    if is_pole(z) {
        return Some(None);
    }
    let wp = gamma_bits(z, p);
    let n = shift(z, wp, 0)?;
    let w = Complex::with_val(wp, z + n);
    let mut prod = Complex::with_val(wp, 1);
    for j in 0..n {
        prod *= Complex::with_val(wp, z + j);
    }
    Some(Some(Complex::with_val(p, stirling(&w, wp).exp() / prod)))
}

/// The principal branch of `ln gamma(z)`.
fn lgamma(z: &Complex, p: u32) -> Out {
    if is_pole(z) {
        return Some(None);
    }
    let wp = gamma_bits(z, p);
    let n = shift(z, wp, 0)?;
    let w = Complex::with_val(wp, z + n);
    let mut s = stirling(&w, wp);
    for j in 0..n {
        s -= Complex::with_val(wp, z + j).ln();
    }
    Some(Some(Complex::with_val(p, s)))
}

fn digamma(z: &Complex, p: u32) -> Out {
    if is_pole(z) {
        return Some(None);
    }
    let wp = p + 64;
    let n = shift(z, wp, 0)?;
    let w = Complex::with_val(wp, z + n);
    let inv = Complex::with_val(wp, w.recip_ref());
    let mut s = Complex::with_val(wp, w.ln_ref()) - Complex::with_val(wp, &inv / 2u32);
    s -= tail(&w, wp, Complex::with_val(wp, inv.square_ref()), &mut |k| {
        Float::with_val(wp, 1) / (2 * u64::from(k))
    });
    for j in 0..n {
        s -= Complex::with_val(wp, z + j).recip();
    }
    Some(Some(Complex::with_val(p, s)))
}

/// The `m`-th derivative of the digamma function.
fn polygamma(m: u32, z: &Complex, p: u32) -> Out {
    if m == 0 {
        return digamma(z, p);
    }
    if is_pole(z) {
        return Some(None);
    }
    let wp = p + 64 + 4 * m;
    let n = shift(z, wp, m)?;
    let w = Complex::with_val(wp, z + n);
    let inv = Complex::with_val(wp, w.recip_ref());
    let inv2 = Complex::with_val(wp, inv.square_ref());
    let fact = |k: u32| Float::with_val(wp, Integer::from(Integer::factorial(k)));
    let w_m = crate::numeric::int_pow(&inv, m as i32, wp);
    // (m-1)!/w^m + m!/(2 w^(m+1)) + sum B_2k (2k+m-1)!/(2k)! w^(-2k-m)
    let mut s = Complex::with_val(wp, &w_m * &fact(m - 1))
        + Complex::with_val(wp, &w_m * &inv) * fact(m) / 2u32;
    let mut c = fact(m + 1) / 2u32;
    s += tail(&w, wp, Complex::with_val(wp, &w_m * &inv2), &mut |k| {
        let out = c.clone();
        let k = u64::from(k);
        let m = u64::from(m);
        c = c.clone() * ((2 * k + m) * (2 * k + m + 1)) / ((2 * k + 1) * (2 * k + 2));
        out
    });
    let mut lower = Complex::new(wp);
    for j in 0..n {
        lower += crate::numeric::int_pow(&Complex::with_val(wp, z + j).recip(), m as i32 + 1, wp);
    }
    s += lower * fact(m);
    Some(Some(Complex::with_val(p, if m % 2 == 1 { s } else { -s })))
}

/// Euler-Maclaurin summation, with the functional equation on the left.
fn zeta(s: &Complex, p: u32) -> Out {
    let a = size(s);
    if !a.is_finite() || a > 1e6 {
        return None;
    }
    let wp = p + 64 + 2 * (a.max(2.0).log2() as u32);
    let s1 = Complex::with_val(wp, s - 1u32);
    if s1.is_zero() {
        return Some(None);
    }
    if s.real().is_sign_negative() && !s.real().is_zero() {
        let t = Complex::with_val(wp, -&s1);
        let (Some(zt), Some(g)) = (zeta(&t, wp)?, gamma(&t, wp)?) else {
            return Some(None);
        };
        let sin = (Complex::with_val(wp, s) * pi(wp) / 2u32).sin();
        let two = (Complex::with_val(wp, s) * Float::with_val(wp, 2).ln()).exp();
        let pis = (s1 * pi(wp).ln()).exp();
        return Some(Some(Complex::with_val(p, two * pis * sin * g * zt)));
    }
    let n = (0.2 * f64::from(wp) + a + 10.0) as u32;
    let neg_s = Complex::with_val(wp, -s);
    let mut sum = Complex::new(wp);
    for k in 1..n {
        let ln_k = Float::with_val(wp, k).ln();
        sum += Complex::with_val(wp, &neg_s * &ln_k).exp();
    }
    let ln_n = Float::with_val(wp, n).ln();
    let n_s = Complex::with_val(wp, &neg_s * &ln_n).exp();
    sum += Complex::with_val(wp, &n_s * n) / &s1;
    sum += Complex::with_val(wp, &n_s / 2u32);
    let mut q = Complex::with_val(wp, s * &n_s) / n / 2u32;
    let n2 = u64::from(n) * u64::from(n);
    let mut last = i64::MAX;
    for k in 1..=(2 * n) {
        let b = Float::with_val(wp, &bernoulli(2 * k));
        let term = Complex::with_val(wp, &q * &b);
        let e = expo(&term);
        if e > last {
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
            / n2;
    }
    Some(Some(Complex::with_val(p, sum)))
}

fn erf(z: &Complex, p: u32) -> Option<Complex> {
    let a2 = size(z).powi(2);
    if !a2.is_finite() || a2 > 1e8 {
        return None;
    }
    let base = p + 48;
    if a2 > LN_2 * f64::from(base) + 16.0 {
        // erfc(w) ~ e^(-w^2)/(w sqrt(pi)) sum (-1)^k (2k-1)!!/(2 w^2)^k for re w >= 0
        let flip = z.real().is_sign_negative();
        let w = if flip {
            Complex::with_val(base, -z)
        } else {
            Complex::with_val(base, z)
        };
        let w2 = Complex::with_val(base, w.square_ref());
        let step = Complex::with_val(base, &w2 * 2u32).recip();
        let (mut t, mut sum, mut last, mut done) = (
            Complex::with_val(base, 1),
            Complex::with_val(base, 1),
            i64::MAX,
            false,
        );
        for k in 0..(4 * base) {
            t = -t * (2 * k + 1) * &step;
            let e = expo(&t);
            if e > last {
                break;
            }
            sum += &t;
            if e < -i64::from(base) {
                done = true;
                break;
            }
            last = e;
        }
        if done {
            let c = (-w2).exp() * sum / (w * pi(base).sqrt());
            let r = Complex::with_val(base, 1) - c;
            return Some(Complex::with_val(p, if flip { -r } else { r }));
        }
    }
    let wp = base + (LOG2_E * a2) as u32 + 16;
    let z2 = -Complex::with_val(wp, z.square_ref());
    let mut t = Complex::with_val(wp, z);
    let mut sum = t.clone();
    for n in 1..u32::MAX {
        t = t * &z2 / n;
        let term = Complex::with_val(wp, &t / (2 * n + 1));
        sum += &term;
        if f64::from(n) > a2 && expo(&term) < expo(&sum) - i64::from(wp) {
            break;
        }
    }
    Some(Complex::with_val(p, sum * 2u32 / pi(wp).sqrt()))
}

/// `(ln z - ln(1/z))/2`: the logarithm, real on the negative axis.
fn log_mean(z: &Complex, wp: u32) -> Complex {
    if z.imag().is_zero() {
        return Complex::with_val(wp, Float::with_val(wp, z.real().abs_ref()).ln());
    }
    Complex::with_val(wp, z.ln_ref())
}

/// Asymptotic series of Ei for large `|z|`.
fn ei_far(z: &Complex, wp: u32) -> Option<Complex> {
    let inv = Complex::with_val(wp, z.recip_ref());
    let (mut t, mut sum, mut last, mut done) = (
        Complex::with_val(wp, 1),
        Complex::with_val(wp, 1),
        i64::MAX,
        false,
    );
    for k in 1..(8 * wp) {
        t = t * &inv * k;
        let e = expo(&t);
        if e > last {
            break;
        }
        sum += &t;
        if e < -i64::from(wp) {
            done = true;
            break;
        }
        last = e;
    }
    if !done {
        return None;
    }
    let mut v = Complex::with_val(wp, z.exp_ref()) * sum * inv;
    if !z.imag().is_zero() {
        let turn = Complex::with_val(wp, (0, pi(wp)));
        if z.imag().is_sign_positive() {
            v += turn;
        } else {
            v -= turn;
        }
    }
    Some(v)
}

/// `sum_{k>=1} z^k/(k k!)`, the entire part of the exponential integral.
fn ein(z: &Complex, wp: u32) -> Complex {
    let a = size(z);
    if a > LN_2 * f64::from(wp) + 20.0
        && let Some(v) = ei_far(z, wp)
    {
        return v - euler(wp) - log_mean(z, wp);
    }
    let xp = wp + (2.8854 * a) as u32 + 32;
    let zz = Complex::with_val(xp, z);
    let mut t = zz.clone();
    let mut sum = zz.clone();
    for k in 2..u32::MAX {
        t = t * &zz / k;
        let term = Complex::with_val(xp, &t / k);
        sum += &term;
        if f64::from(k) > a && expo(&term) < expo(&sum) - i64::from(xp) {
            break;
        }
    }
    Complex::with_val(wp, sum)
}

fn ei(z: &Complex, p: u32) -> Out {
    if z.is_zero() {
        return Some(None);
    }
    let a = size(z);
    if !a.is_finite() || a > 1e6 {
        return None;
    }
    let wp = p + 48;
    if a > LN_2 * f64::from(wp) + 20.0
        && let Some(v) = ei_far(z, wp)
    {
        return Some(Some(Complex::with_val(p, v)));
    }
    let xp = wp + (2.8854 * a) as u32;
    Some(Some(Complex::with_val(
        p,
        ein(z, xp) + euler(xp) + log_mean(z, xp),
    )))
}

/// Si, Ci, Shi and Chi from the entire part of Ei.
fn trig_integral(name: &str, z: &Complex, p: u32) -> Out {
    let a = size(z);
    if !a.is_finite() || a > 1e6 {
        return None;
    }
    let wp = p + 48;
    let odd = matches!(name, "si" | "shi");
    if z.is_zero() {
        return Some(odd.then(|| Complex::new(p)));
    }
    let rotate = matches!(name, "si" | "ci");
    let u = if rotate {
        Complex::with_val(wp, z).mul_i(false)
    } else {
        Complex::with_val(wp, z)
    };
    let (plus, minus) = (ein(&u, wp), ein(&Complex::with_val(wp, -&u), wp));
    let v = match name {
        "si" => (plus - minus).mul_i(true) / 2u32,
        "shi" => (plus - minus) / 2u32,
        _ => (plus + minus) / 2u32 + euler(wp) + Complex::with_val(wp, z.ln_ref()),
    };
    Some(Some(Complex::with_val(p, v)))
}

fn li2(z: &Complex, wp: u32) -> Complex {
    if z.is_zero() {
        return Complex::new(wp);
    }
    let pi2_6 = pi(wp).square() / 6u32;
    let rest = Complex::with_val(wp, 1) - z;
    if rest.is_zero() {
        return Complex::with_val(wp, pi2_6);
    }
    let a = size(z);
    if a > 1.0 {
        let l = crate::numeric::unsigned_zero(Complex::with_val(wp, -z)).ln();
        return -li2(&Complex::with_val(wp, z.recip_ref()), wp) - l.square() / 2u32 - pi2_6;
    }
    if a < 0.01 {
        let mut t = Complex::with_val(wp, z);
        let mut sum = t.clone();
        for k in 2..u32::MAX {
            t *= z;
            let term = Complex::with_val(wp, &t / (u64::from(k) * u64::from(k)));
            sum += &term;
            if expo(&term) < expo(&sum) - i64::from(wp) {
                break;
            }
        }
        return sum;
    }
    if *z.real() > 0.5f64 {
        let logs = Complex::with_val(wp, z.ln_ref()) * Complex::with_val(wp, rest.ln_ref());
        return -li2(&rest, wp) - logs + pi2_6;
    }
    // sum B_n u^(n+1)/(n+1)! with u = -ln(1 - z)
    let u = -rest.ln();
    let mut t = u.clone();
    let mut sum = u.clone();
    t = t * &u / 2u32;
    sum -= Complex::with_val(wp, &t / 2u32);
    for n in 2..u32::MAX {
        t = t * &u / (n + 1);
        if n % 2 == 1 {
            continue;
        }
        let b = Float::with_val(wp, &bernoulli(n));
        let term = Complex::with_val(wp, &t * &b);
        sum += &term;
        if expo(&term) < expo(&sum) - i64::from(wp) {
            break;
        }
    }
    sum
}

/// Arithmetic-geometric mean, taking the root nearer to the arithmetic mean.
fn agm(x: &Complex, y: &Complex, p: u32) -> Complex {
    let wp = p + 32;
    let (mut a, mut b) = (Complex::with_val(wp, x), Complex::with_val(wp, y));
    if a.is_zero() || b.is_zero() || Complex::with_val(wp, &a + &b).is_zero() {
        return Complex::new(p);
    }
    for _ in 0..200 {
        let mean = Complex::with_val(wp, &a + &b) / 2u32;
        let mut root = Complex::with_val(wp, &a * &b).sqrt();
        if size(&Complex::with_val(wp, &mean - &root)) > size(&Complex::with_val(wp, &mean + &root))
        {
            root = -root;
        }
        let gap = Complex::with_val(wp, &mean - &root);
        (a, b) = (mean, root);
        if gap.is_zero() || expo(&gap) < expo(&a) - i64::from(wp) + 4 {
            break;
        }
    }
    Complex::with_val(p, a)
}

pub(crate) fn gamma_any(z: &Complex, p: u32) -> Out {
    match real_arg(z) {
        Some(x) => Some(real(p, x.gamma())),
        None => gamma(z, p),
    }
}

/// `1/gamma(z)`, zero at the poles of gamma.
pub(crate) fn rgamma(z: &Complex, p: u32) -> Option<Complex> {
    if is_pole(z) {
        return Some(Complex::new(p));
    }
    Some(match gamma_any(z, p)? {
        Some(g) => g.recip(),
        None => Complex::new(p),
    })
}

pub(crate) fn zeta_any(z: &Complex, p: u32) -> Out {
    match real_arg(z) {
        Some(x) => Some(real(p, x.zeta())),
        None => zeta(z, p),
    }
}

pub(crate) fn erf_any(z: &Complex, p: u32) -> Option<Complex> {
    match real_arg(z) {
        Some(x) => real(p, x.erf()),
        None => erf(z, p),
    }
}

pub fn call(name: &str, a: &[Complex], p: u32) -> Out {
    let z = a.first()?;
    let x = real_arg(z);
    match (name, a.len()) {
        ("gamma", 1) => gamma_any(z, p),
        ("factorial", 1) => gamma_any(&Complex::with_val(p + 8, z + 1u32), p),
        ("lgamma", 1) => match x {
            Some(x) => Some(real(p, x.ln_abs_gamma().0)),
            None => lgamma(z, p),
        },
        ("digamma", 1) => match x {
            Some(x) => Some(real(p, x.digamma())),
            None => digamma(z, p),
        },
        ("polygamma", 2) => {
            let m = x
                .filter(Float::is_integer)?
                .to_integer()?
                .to_u32()
                .filter(|m| *m <= 10_000)?;
            polygamma(m, &a[1], p)
        }
        ("harmonic", 1) => {
            let v = call("digamma", &[Complex::with_val(p + 8, z + 1u32)], p)?;
            Some(v.map(|v| v + euler(p)))
        }
        ("zeta", 1) => match x {
            Some(x) => Some(real(p, x.zeta())),
            None => zeta(z, p),
        },
        ("erf", 1) => match x {
            Some(x) => Some(real(p, x.erf())),
            None => Some(Some(erf(z, p)?)),
        },
        ("erfc", 1) => match x {
            Some(x) => Some(real(p, x.erfc())),
            None => Some(Some(Complex::with_val(p, 1) - erf(z, p)?)),
        },
        ("ei", 1) => match x {
            Some(x) if !x.is_zero() => Some(real(p, x.eint())),
            _ => ei(z, p),
        },
        ("li", 1) => {
            if z.is_zero() {
                return Some(Some(Complex::new(p)));
            }
            let l = Complex::with_val(p + 16, z.ln_ref());
            if l.is_zero() {
                return Some(None);
            }
            call("ei", &[l], p)
        }
        ("li2", 1) => match x {
            Some(x) if x <= 1u32 => Some(real(p, x.li2())),
            _ => Some(Some(Complex::with_val(p, li2(z, p + 48)))),
        },
        ("si" | "ci" | "shi" | "chi", 1) => trig_integral(name, z, p),
        ("agm", 2) => Some(Some(agm(z, &a[1], p))),
        ("binomial", 2) => {
            let wp = p + 32;
            let (n, k) = (Complex::with_val(wp, z), Complex::with_val(wp, &a[1]));
            let top = gamma_any(&Complex::with_val(wp, &n + 1u32), wp)?;
            let low1 = gamma_any(&Complex::with_val(wp, &k + 1u32), wp)?;
            let low2 = gamma_any(&(Complex::with_val(wp, &n - &k) + 1u32), wp)?;
            Some(match (top, low1, low2) {
                (Some(t), Some(l1), Some(l2)) => Some(Complex::with_val(p, t / (l1 * l2))),
                (Some(_), _, _) => Some(Complex::new(p)),
                _ => None,
            })
        }
        ("factorial2", 1) => {
            // 2^(z/2) (pi/2)^((cos(pi z) - 1)/4) gamma(z/2 + 1)
            let wp = p + 32;
            let zz = Complex::with_val(wp, z);
            let Some(g) = gamma_any(&(Complex::with_val(wp, &zz / 2u32) + 1u32), wp)? else {
                return Some(None);
            };
            let two = (Complex::with_val(wp, &zz / 2u32) * Float::with_val(wp, 2).ln()).exp();
            let turn = (Complex::with_val(wp, &zz * &pi(wp)).cos() - 1u32) / 4u32;
            let half_pi = (turn * (pi(wp) / 2u32).ln()).exp();
            Some(Some(Complex::with_val(p, two * half_pi * g)))
        }
        ("lambertw", 1) => Some(lambert_w(0, z)),
        ("lambertw", 2) => {
            let k = x?.to_integer()?.to_i64()?;
            Some(lambert_w(k, &a[1]))
        }
        _ => crate::hyper::call(name, a, p),
    }
}

/// Branch `k` of the Lambert W function.
pub fn lambert_w(k: i64, z: &Complex) -> Option<Complex> {
    let p = z.prec().0.max(z.prec().1);
    if z.is_zero() {
        return (k == 0).then(|| Complex::new(p));
    }
    let two_pi_k = Float::with_val(64, Constant::Pi) * 2u32 * k;
    let start = |prec: u32| -> Complex {
        let zz = Complex::with_val(prec, z);
        let e = Float::with_val(prec, 1).exp();
        let near = Complex::with_val(prec, &zz * &e) + 1u32;
        let near_branch_point = Float::with_val(prec, near.abs_ref()) < 0.3;
        // (left of the branch point on the real axis the value is not real: the start must
        // leave the axis, which the steps from a real start never do)
        let left_of_it = k == 0
            && z.imag().is_zero()
            && near.real().is_sign_negative()
            && Float::with_val(prec, zz.abs_ref()) < 1.5;
        if (near_branch_point || left_of_it)
            && (k == 0
                || (k == -1 && !z.imag().is_sign_positive())
                || (k == 1 && z.imag().is_sign_negative()))
        {
            // w = -1 + q - q^2/3 + 11 q^3/72 with q = +-sqrt(2(ez + 1))
            let mut q = (near * 2u32).sqrt();
            if k != 0 {
                q = -q;
            }
            let q2 = q.clone().square();
            return Complex::with_val(prec, -1) + &q - Complex::with_val(prec, &q2 / 3u32)
                + q2 * &q * 11u32 / 72u32;
        }
        if k == 0 && Float::with_val(prec, zz.abs_ref()) < 1.5 {
            // (1 + z)-Pade style guess, good inside the unit disk
            let denom = Complex::with_val(prec, &zz + 1u32);
            return if Float::with_val(prec, denom.abs_ref()) < 0.2 {
                zz
            } else {
                zz.clone() / denom
            };
        }
        let l1 = zz.ln() + Complex::with_val(prec, (0, &two_pi_k));
        let l2 = l1.clone().ln();
        l1.clone() - &l2 + l2 / l1
    };
    let mut prec = 64u32.min(p + 32);
    let mut w = start(prec);
    let target = p + 32;
    let mut stalled = 0;
    for _ in 0..400 {
        let zz = Complex::with_val(prec, z);
        let ew = w.clone().exp();
        let f = Complex::with_val(prec, &w * &ew) - &zz;
        let w1 = Complex::with_val(prec, &w + 1u32);
        if w1.is_zero() {
            w += Complex::with_val(prec, (1e-10, 1e-10));
            continue;
        }
        let denom = Complex::with_val(prec, &ew * &w1)
            - Complex::with_val(prec, &w + 2u32) * &f / (Complex::with_val(prec, &w1 * 2u32));
        if denom.is_zero() {
            return None;
        }
        let step = f / denom;
        let next = Complex::with_val(prec, &w - &step);
        let small = {
            let s = Float::with_val(prec, step.abs_ref());
            let m = Float::with_val(prec, next.abs_ref());
            s.is_zero()
                || s.get_exp().unwrap_or(0) < m.get_exp().unwrap_or(0) - (prec as i32) / 3 - 2
        };
        w = next;
        if small {
            if prec >= target {
                stalled += 1;
                if stalled >= 2 {
                    return Some(Complex::with_val(p, w));
                }
            } else {
                prec = (prec * 3).min(target);
                w = Complex::with_val(prec, w);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lambert_w_left_of_its_branch_point() {
        // w*e^w = z on the principal branch, also where w is not real although z is
        let check = |z: f64, re: f64, im: f64| {
            let w = lambert_w(0, &Complex::with_val(128, (z, 0))).expect("a value");
            assert!((w.real().to_f64() - re).abs() < 1e-12, "W({z}) = {w}");
            assert!((w.imag().to_f64() - im).abs() < 1e-12, "W({z}) = {w}");
            let back = Complex::with_val(128, &w * w.clone().exp());
            assert!((back.real().to_f64() - z).abs() < 1e-25 && back.imag().to_f64().abs() < 1e-25);
        };
        check(-1.0, -0.318_131_505_204_764_1, 1.337_235_701_430_689_4);
        check(-0.4, -0.944_089_738_264_935_7, 0.407_267_964_032_857_7);
        check(-100.0, 3.205_380_786_307_449_4, 2.482_590_531_815_924);
        check(-0.2, -0.259_171_101_819_073_7, 0.0);
        check(2.0, 0.852_605_502_013_725_5, 0.0);
    }
}
