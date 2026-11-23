//! Filters and spectra of signals: windows, analog prototypes and the filters made of
//! them, filters by windows, smoothing by polynomials, responses, spectra, peaks, linear
//! prediction and a few waveforms. Everything is worked out with the digits of the session.

use crate::arrays::{whole, word};
use crate::eval::Session;
use crate::expr::Expr;
use crate::numeric;
use rug::ops::Pow;
use rug::{Complex, Float};
use std::cmp::Ordering;

pub(crate) type Res = Result<Expr, String>;

pub(crate) fn bits() -> u32 {
    crate::mfun::bits()
}

#[cfg(test)]
fn fl(x: f64) -> Float {
    Float::with_val(bits(), x)
}

pub(crate) fn int(x: i64) -> Float {
    Float::with_val(bits(), x)
}

/// A decimal number of a formula, as it is written.
pub(crate) fn dec(text: &str) -> Float {
    Float::with_val(bits(), Float::parse(text).expect("a number"))
}

pub(crate) fn pi() -> Float {
    Float::with_val(bits(), rug::float::Constant::Pi)
}

pub(crate) fn cx(re: &Float) -> Complex {
    Complex::with_val(bits(), re)
}

pub(crate) fn unit(angle: &Float) -> Complex {
    let (sin, cos) = angle.clone().sin_cos(Float::new(bits()));
    Complex::with_val(bits(), (cos, sin))
}

pub(crate) fn items(e: &Expr) -> Vec<Expr> {
    match e.as_list() {
        Some(l) => l.to_vec(),
        None => vec![e.clone()],
    }
}

pub(crate) fn real(e: &Expr, who: &str) -> Result<Float, String> {
    crate::mfun::real(e, bits(), who)
}

pub(crate) fn reals(e: &Expr, who: &str) -> Result<Vec<Float>, String> {
    items(e).iter().map(|x| real(x, who)).collect()
}

pub(crate) fn complexes(e: &Expr, who: &str) -> Result<Vec<Complex>, String> {
    items(e)
        .iter()
        .map(|x| numeric::evalf(x, bits()).map_err(|_| format!("{who}: {x} is no number")))
        .collect()
}

/// A list of numbers as a value; what rounding left where a zero belongs is 0.
pub(crate) fn shown(v: &[Float]) -> Expr {
    let top = v
        .iter()
        .fold(Float::new(bits()), |m, x| m.max(&x.clone().abs()));
    let least = top * Float::with_val(bits(), Float::i_pow_u(10, numeric::digits() + 8)).recip();
    Expr::list(
        v.iter()
            .map(|x| match x.clone().abs() <= least {
                true => Expr::zero(),
                false => crate::mfun::value(x),
            })
            .collect(),
    )
}

pub(crate) fn shown_c(v: &[Complex]) -> Expr {
    Expr::list(crate::decomp::from_complex_list(v))
}

pub(crate) fn count(e: &Expr, what: &str, low: i64, high: i64) -> Result<usize, String> {
    Ok(whole(e, what, low, high)? as usize)
}

/// The transform of Fourier of any number of points.
pub(crate) fn fft(x: &[Complex], inverse: bool) -> Vec<Complex> {
    let n = x.len();
    let mut y = crate::signal::dft(x, inverse, bits());
    if inverse {
        for v in &mut y {
            *v /= n as u64;
        }
    }
    y
}

/// The modified function of Bessel of order 0, by its series.
fn bessel_i0(x: &Float) -> Float {
    let quarter = Float::with_val(bits(), x * x) / 4u32;
    let (mut term, mut sum) = (int(1), int(1));
    for m in 1..2000u32 {
        term = term * &quarter / m / m;
        sum += &term;
        if term.clone().abs() < Float::with_val(bits(), &sum >> (bits() + 4)) {
            break;
        }
    }
    sum
}

/// The windows by their names: `n` values that fall towards both ends.
pub(crate) fn window(name: &str, n: usize, shape: Option<&Float>) -> Result<Vec<Float>, String> {
    if n == 0 {
        return Ok(Vec::new());
    }
    if n == 1 {
        return Ok(vec![int(1)]);
    }
    let last = (n - 1) as u64;
    // the place k as a share of the way, and sums of cosines of its multiples
    let turn = |k: usize, times: u32| (pi() * 2u32 * times * k as u64 / last).cos();
    let cosines = |weights: &[&str]| -> Vec<Float> {
        (0..n)
            .map(|k| {
                let mut sum = dec(weights[0]);
                for (m, w) in weights.iter().enumerate().skip(1) {
                    let term = dec(w) * turn(k, m as u32);
                    if m % 2 == 1 {
                        sum -= term;
                    } else {
                        sum += term;
                    }
                }
                sum
            })
            .collect()
    };
    // the place from -1 at the first value to 1 at the last
    let across = |k: usize| int(2 * k as i64 - last as i64) / last;
    Ok(match name {
        "rectwin" => vec![int(1); n],
        "hann" => cosines(&["0.5", "0.5"]),
        "hanning" => (1..=n)
            .map(|k| (int(1) - (pi() * 2u32 * k as u64 / (n as u64 + 1)).cos()) / 2u32)
            .collect(),
        "hamming" => cosines(&["0.54", "0.46"]),
        "blackman" => cosines(&["0.42", "0.5", "0.08"]),
        "blackmanharris" => cosines(&["0.35875", "0.48829", "0.14128", "0.01168"]),
        "nuttallwin" => cosines(&["0.3635819", "0.4891775", "0.1365995", "0.0106411"]),
        "flattopwin" => cosines(&[
            "0.21557895",
            "0.41663158",
            "0.277263158",
            "0.083578947",
            "0.006947368",
        ]),
        "bartlett" => (0..n).map(|k| int(1) - across(k).abs()).collect(),
        "triang" => (1..=n as i64)
            .map(|k| match n % 2 {
                1 => int(1) - int((2 * k - n as i64 - 1).abs()) / (n as u64 + 1),
                _ => int(1) - int((2 * k - n as i64 - 1).abs()) / n as u64,
            })
            .collect(),
        "bohmanwin" => (0..n)
            .map(|k| {
                let x = across(k).abs();
                let angle = pi() * &x;
                (int(1) - &x) * angle.clone().cos() + angle.sin() / pi()
            })
            .collect(),
        "parzenwin" => (0..n)
            .map(|k| {
                // |k - (n - 1)/2| over n/2
                let x = int(2 * k as i64 - last as i64).abs() / n as u64;
                match int(2 * k as i64 - last as i64).abs() * 2u32 <= last {
                    true => int(1) - x.clone().pow(2u32) * 6u32 + x.pow(3u32) * 6u32,
                    false => (int(1) - x).pow(3u32) * 2u32,
                }
            })
            .collect(),
        "gausswin" => {
            let alpha = shape.cloned().unwrap_or_else(|| dec("2.5"));
            (0..n)
                .map(|k| (-(Float::with_val(bits(), &alpha * across(k))).pow(2u32) / 2u32).exp())
                .collect()
        }
        "tukeywin" => {
            let r = shape.cloned().unwrap_or_else(|| dec("0.5"));
            if r <= 0 {
                return window("rectwin", n, None);
            }
            if r >= 1 {
                return window("hann", n, None);
            }
            (0..n)
                .map(|k| {
                    let x = int(k as i64) / last;
                    let half = Float::with_val(bits(), &r / 2u32);
                    let edge = |from: Float| (int(1) + (pi() * 2u32 / &r * from).cos()) / 2u32;
                    if x < half {
                        edge(x - half)
                    } else if x <= Float::with_val(bits(), 1u32 - &half) {
                        int(1)
                    } else {
                        edge(x - 1u32 + half)
                    }
                })
                .collect()
        }
        "kaiser" => {
            let beta = shape.cloned().unwrap_or_else(|| dec("0.5"));
            let whole = bessel_i0(&beta);
            (0..n)
                .map(|k| {
                    let inside = (int(1) - across(k).pow(2u32)).max(&int(0)).sqrt();
                    bessel_i0(&(inside * &beta)) / &whole
                })
                .collect()
        }
        "barthannwin" => (0..n)
            .map(|k| {
                let x = across(k) / 2u32;
                dec("0.62") - dec("0.48") * x.clone().abs() + dec("0.38") * (pi() * 2u32 * x).cos()
            })
            .collect(),
        "chebwin" => {
            let ripple = shape.cloned().unwrap_or_else(|| int(100));
            let order = int(last as i64);
            let beta = (Float::with_val(bits(), int(10).pow(ripple.abs() / 20u32)).acosh()
                / &order)
                .cosh();
            let mut p: Vec<Complex> = (0..n)
                .map(|k| {
                    let x = Float::with_val(bits(), &beta * (pi() * k as u64 / n as u64).cos());
                    let value = if x > 1 {
                        (x.acosh() * &order).cosh()
                    } else if x < -1 {
                        let sign = if n % 2 == 1 { 1 } else { -1 };
                        ((-x).acosh() * &order).cosh() * sign
                    } else {
                        (x.acos() * &order).cos()
                    };
                    cx(&value)
                })
                .collect();
            let w: Vec<Float> = match n % 2 {
                1 => {
                    let t = fft(&p, false);
                    let half = n.div_ceil(2);
                    (1..half)
                        .rev()
                        .chain(0..half)
                        .map(|k| t[k].real().clone())
                        .collect()
                }
                _ => {
                    for (k, v) in p.iter_mut().enumerate() {
                        *v *= unit(&(pi() * k as u64 / n as u64));
                    }
                    let t = fft(&p, false);
                    let half = n / 2 + 1;
                    (1..half)
                        .rev()
                        .chain(1..half)
                        .map(|k| t[k].real().clone())
                        .collect()
                }
            };
            let top = w
                .iter()
                .fold(Float::with_val(bits(), f64::NEG_INFINITY), |m, v| m.max(v));
            w.into_iter().map(|v| v / &top).collect()
        }
        _ => return Err(format!("{name} is no window")),
    })
}

const WINDOWS: [&str; 16] = [
    "rectwin",
    "hann",
    "hanning",
    "hamming",
    "blackman",
    "blackmanharris",
    "nuttallwin",
    "flattopwin",
    "bartlett",
    "triang",
    "bohmanwin",
    "parzenwin",
    "gausswin",
    "tukeywin",
    "kaiser",
    "chebwin",
];

/// A window as the argument of a function gives it: a name with a number of values, or
/// its values.
pub(crate) fn window_of(
    e: Option<&Expr>,
    n: usize,
    usual: &str,
    who: &str,
) -> Result<Vec<Float>, String> {
    match e {
        None => window(usual, n, None),
        Some(e) => match word(e) {
            Some(name) if WINDOWS.contains(&name.as_str()) || name == "barthannwin" => {
                window(&name, n, None)
            }
            Some(name) => Err(format!("{who}: {name} is no window")),
            None => {
                let values = reals(e, who)?;
                match values.len() == n {
                    true => Ok(values),
                    false => Err(format!(
                        "{who}: the window has {} values, {n} are needed",
                        values.len()
                    )),
                }
            }
        },
    }
}

// ---- polynomials with real and complex coefficients, the highest power first ----

fn poly_of_roots(roots: &[Complex]) -> Vec<Complex> {
    let mut p = vec![Complex::with_val(bits(), 1)];
    for r in roots {
        let mut next = vec![Complex::new(bits()); p.len() + 1];
        for (k, c) in p.iter().enumerate() {
            next[k] += c;
            next[k + 1] -= Complex::with_val(bits(), c * r);
        }
        p = next;
    }
    p
}

fn real_parts(p: &[Complex]) -> Vec<Float> {
    p.iter().map(|c| c.real().clone()).collect()
}

pub(crate) fn poly_mul(a: &[Float], b: &[Float]) -> Vec<Float> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![Float::new(bits()); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j] += Float::with_val(bits(), x * y);
        }
    }
    out
}

pub(crate) fn poly_add(a: &[Float], b: &[Float]) -> Vec<Float> {
    let n = a.len().max(b.len());
    (0..n)
        .map(|k| {
            let at = |p: &[Float]| {
                (k + p.len())
                    .checked_sub(n)
                    .map_or_else(|| Float::new(bits()), |i| p[i].clone())
            };
            at(a) + at(b)
        })
        .collect()
}

fn poly_power(p: &[Float], k: usize) -> Vec<Float> {
    (0..k).fold(vec![int(1)], |acc, _| poly_mul(&acc, p))
}

/// Without the zeros that stand before the first coefficient; what rounding left of a
/// zero counts as one.
pub(crate) fn trimmed(p: Vec<Float>) -> Vec<Float> {
    let top = p
        .iter()
        .fold(Float::new(bits()), |m, v| m.max(&v.clone().abs()));
    let tiny = Float::with_val(bits(), &top >> (bits() - 24));
    let first = p
        .iter()
        .position(|c| c.clone().abs() > tiny)
        .unwrap_or(p.len().saturating_sub(1));
    p[first..].to_vec()
}

/// A polynomial at a complex place.
pub(crate) fn poly_at(p: &[Float], x: &Complex) -> Complex {
    p.iter().fold(Complex::new(bits()), |s, c| s * x + c)
}

/// `b(s)/a(s)` with `top(s)/bottom(s)` in the place of s, and 1 as the first coefficient
/// of the new denominator.
fn substituted(
    b: &[Float],
    a: &[Float],
    top: &[Float],
    bottom: &[Float],
) -> (Vec<Float>, Vec<Float>) {
    let degree = b.len().max(a.len()) - 1;
    let turned = |p: &[Float]| {
        let own = p.len() - 1;
        let mut total = vec![Float::new(bits())];
        for (i, c) in p.iter().enumerate() {
            let power = own - i;
            let term = poly_mul(&poly_power(top, power), &poly_power(bottom, degree - power));
            total = poly_add(
                &total,
                &term
                    .iter()
                    .map(|t| Float::with_val(bits(), t * c))
                    .collect::<Vec<_>>(),
            );
        }
        total
    };
    let (num, den) = (turned(b), trimmed(turned(a)));
    let lead = den[0].clone();
    let scale = |p: Vec<Float>| p.into_iter().map(|c| c / &lead).collect::<Vec<_>>();
    (trimmed(scale(num)), scale(den))
}

// ---- filters as zeros, poles and a gain ----

pub(crate) struct Zpk {
    z: Vec<Complex>,
    p: Vec<Complex>,
    k: Float,
}

impl Zpk {
    fn shown(&self) -> Expr {
        Expr::list(vec![
            shown_c(&self.z),
            shown_c(&self.p),
            crate::mfun::value(&self.k),
        ])
    }

    /// The coefficients of numerator and denominator.
    fn tf(&self) -> (Vec<Float>, Vec<Float>) {
        let b: Vec<Float> = real_parts(&poly_of_roots(&self.z))
            .into_iter()
            .map(|c| c * &self.k)
            .collect();
        (b, real_parts(&poly_of_roots(&self.p)))
    }

    fn degree(&self) -> usize {
        self.p.len() - self.z.len()
    }

    /// The real part of prod(-z)/prod(-p).
    fn ratio(&self) -> Float {
        let product = |v: &[Complex]| {
            v.iter().fold(Complex::with_val(bits(), 1), |s, x| {
                s * Complex::with_val(bits(), -x)
            })
        };
        Complex::with_val(bits(), product(&self.z) / product(&self.p))
            .real()
            .clone()
    }

    fn lowpass(mut self, wo: &Float) -> Zpk {
        let degree = self.degree() as u32;
        for x in self.z.iter_mut().chain(self.p.iter_mut()) {
            *x *= wo;
        }
        self.k *= Float::with_val(bits(), wo.pow(degree));
        self
    }

    fn highpass(self, wo: &Float) -> Zpk {
        let degree = self.degree();
        let k = Float::with_val(bits(), &self.k * self.ratio());
        let turn = |v: &[Complex]| {
            v.iter()
                .map(|x| Complex::with_val(bits(), wo / x))
                .collect::<Vec<_>>()
        };
        let mut z = turn(&self.z);
        z.extend(vec![Complex::new(bits()); degree]);
        Zpk {
            z,
            p: turn(&self.p),
            k,
        }
    }

    fn bandpass(self, wo: &Float, bw: &Float) -> Zpk {
        let degree = self.degree();
        let split = |v: &[Complex]| {
            let mut out = Vec::new();
            for x in v {
                let half = Complex::with_val(bits(), x * bw) / 2u32;
                let root = Complex::with_val(
                    bits(),
                    half.clone().square() - Float::with_val(bits(), wo * wo),
                )
                .sqrt();
                out.push(Complex::with_val(bits(), &half + &root));
                out.push(Complex::with_val(bits(), &half - &root));
            }
            out
        };
        let mut z = split(&self.z);
        z.extend(vec![Complex::new(bits()); degree]);
        Zpk {
            z,
            p: split(&self.p),
            k: self.k * Float::with_val(bits(), bw.pow(degree as u32)),
        }
    }

    fn bandstop(self, wo: &Float, bw: &Float) -> Zpk {
        let degree = self.degree();
        let k = Float::with_val(bits(), &self.k * self.ratio());
        let split = |v: &[Complex]| {
            let mut out = Vec::new();
            for x in v {
                let half = Complex::with_val(bits(), Float::with_val(bits(), bw / 2u32) / x);
                let root = Complex::with_val(
                    bits(),
                    half.clone().square() - Float::with_val(bits(), wo * wo),
                )
                .sqrt();
                out.push(Complex::with_val(bits(), &half + &root));
                out.push(Complex::with_val(bits(), &half - &root));
            }
            out
        };
        let mut z = split(&self.z);
        for _ in 0..degree {
            z.push(Complex::with_val(bits(), (0, wo)));
            z.push(Complex::with_val(bits(), (0, -wo.clone())));
        }
        Zpk {
            z,
            p: split(&self.p),
            k,
        }
    }

    /// The sampled filter by z = (2 fs + s)/(2 fs - s).
    fn bilinear(self, fs: &Float) -> Zpk {
        let degree = self.degree();
        let two = Float::with_val(bits(), fs * 2u32);
        let product = |v: &[Complex]| {
            v.iter().fold(Complex::with_val(bits(), 1), |s, x| {
                s * Complex::with_val(bits(), &two - x)
            })
        };
        let gain = Complex::with_val(bits(), product(&self.z) / product(&self.p))
            .real()
            .clone();
        let turn = |v: &[Complex]| {
            v.iter()
                .map(|x| {
                    Complex::with_val(
                        bits(),
                        Complex::with_val(bits(), &two + x) / Complex::with_val(bits(), &two - x),
                    )
                })
                .collect::<Vec<_>>()
        };
        let mut z = turn(&self.z);
        z.extend(vec![Complex::with_val(bits(), -1); degree]);
        Zpk {
            z,
            p: turn(&self.p),
            k: self.k * gain,
        }
    }
}

fn product_real(v: &[Complex]) -> Float {
    v.iter()
        .fold(Complex::with_val(bits(), 1), |s, x| {
            s * Complex::with_val(bits(), -x)
        })
        .real()
        .clone()
}

fn buttap(n: usize) -> Zpk {
    let p = (1..=n)
        .map(|k| unit(&(pi() * (2 * k + n - 1) as u64 / (2 * n) as u64)))
        .collect();
    Zpk {
        z: Vec::new(),
        p,
        k: int(1),
    }
}

pub(crate) fn cheb1ap(n: usize, rp: &Float) -> Zpk {
    let eps = (int(10).pow(Float::with_val(bits(), rp / 10u32)) - 1u32).sqrt();
    let mu = eps.clone().recip().asinh() / n as u64;
    let p: Vec<Complex> = (1..=n)
        .map(|k| {
            let theta = pi() * (2 * k - 1) as u64 / (2 * n) as u64;
            let (sin, cos) = theta.sin_cos(Float::new(bits()));
            Complex::with_val(bits(), (-mu.clone().sinh() * sin, mu.clone().cosh() * cos))
        })
        .collect();
    let mut k = product_real(&p);
    if n.is_multiple_of(2) {
        k /= (int(1) + eps.square()).sqrt();
    }
    Zpk {
        z: Vec::new(),
        p,
        k,
    }
}

fn cheb2ap(n: usize, rs: &Float) -> Zpk {
    let delta = (int(10).pow(Float::with_val(bits(), rs / 10u32)) - 1u32)
        .sqrt()
        .recip();
    let mu = delta.recip().asinh() / n as u64;
    let (mut z, mut p) = (Vec::new(), Vec::new());
    for k in 1..=n {
        let theta = pi() * (2 * k - 1) as u64 / (2 * n) as u64;
        let (sin, cos) = theta.sin_cos(Float::new(bits()));
        // (the middle one of an odd order has its zero at infinity)
        if 2 * k != n + 1 {
            z.push(Complex::with_val(bits(), (0, cos.clone().recip())));
        }
        let pole = Complex::with_val(bits(), (-mu.clone().sinh() * sin, mu.clone().cosh() * cos));
        p.push(pole.recip());
    }
    let k = Complex::with_val(bits(), cx(&product_real(&p)) / cx(&product_real(&z)))
        .real()
        .clone();
    Zpk { z, p, k }
}

/// The moduli of Landen's way down from k, until nothing is left.
fn landen(k: &Float) -> Vec<Float> {
    let mut v = Vec::new();
    let mut k = k.clone();
    while v.len() < 40 && k > Float::with_val(bits(), 1) >> bits() {
        k = (k.clone() / (int(1) + (int(1) - k.clone().square()).sqrt())).square();
        v.push(k.clone());
    }
    v
}

/// The quarter period K(k).
fn quarter_period(k: &Float) -> Float {
    landen(k).iter().fold(pi() / 2u32, |s, v| s * (int(1) + v))
}

/// cd(u*K, k) and sn(u*K, k) for a complex u.
fn cd_sn(u: &Complex, k: &Float, sine: bool) -> Complex {
    let v = landen(k);
    let angle = Complex::with_val(bits(), u * pi()) / 2u32;
    let mut w = if sine { angle.sin() } else { angle.cos() };
    for step in v.iter().rev() {
        let top = Complex::with_val(bits(), &w * (int(1) + step));
        w = top / (Complex::with_val(bits(), w.clone().square() * step) + 1u32);
    }
    w
}

/// The u with sn(u*K, k) = w, for a complex w.
fn asn(w: &Complex, k: &Float) -> Complex {
    let mut moduli = vec![k.clone()];
    moduli.extend(landen(k));
    let mut w = w.clone();
    for pair in moduli.windows(2) {
        let root = (Complex::with_val(bits(), 1)
            - Complex::with_val(bits(), w.clone().square() * pair[0].clone().square()))
        .sqrt();
        w = Complex::with_val(bits(), w / (root + 1u32)) * 2u32 / (int(1) + &pair[1]);
    }
    // u = 1 - (2/pi)*acos(w)
    Complex::with_val(bits(), 1) - Complex::with_val(bits(), crate::numeric::acos(&w) * 2u32) / pi()
}

/// The modulus k whose ratio of quarter periods is n times that of k1.
fn degree_modulus(n: usize, k1: &Float) -> Float {
    let other = (int(1) - k1.clone().square()).sqrt();
    let q1 = (-pi() * quarter_period(&other) / quarter_period(k1)).exp();
    let q = q1.pow(int(1) / n as u64);
    let (mut top, mut bottom) = (Float::new(bits()), int(1));
    for m in 0..60u32 {
        top += Float::with_val(bits(), q.clone().pow(m * (m + 1)));
        if m > 0 {
            bottom += Float::with_val(bits(), q.clone().pow(m * m)) * 2u32;
        }
    }
    q.sqrt() * 4u32 * (top / bottom).square()
}

fn ellipap(n: usize, rp: &Float, rs: &Float) -> Zpk {
    let eps_p = (int(10).pow(Float::with_val(bits(), rp / 10u32)) - 1u32).sqrt();
    let eps_s = (int(10).pow(Float::with_val(bits(), rs / 10u32)) - 1u32).sqrt();
    let k1 = Float::with_val(bits(), &eps_p / &eps_s);
    let k = degree_modulus(n, &k1);
    let i = Complex::with_val(bits(), (0, 1));
    // v0 = -i*asn(i/eps_p, k1)/n
    let v0 = Complex::with_val(
        bits(),
        -i.clone() * asn(&Complex::with_val(bits(), &i / &eps_p), &k1),
    ) / n as u64;
    let (mut z, mut p) = (Vec::new(), Vec::new());
    for index in 1..=n / 2 {
        let u = int((2 * index - 1) as i64) / n as u64;
        let zeta = cd_sn(&cx(&u), &k, false);
        let zero = Complex::with_val(bits(), &i / Complex::with_val(bits(), zeta * &k));
        z.push(zero.clone());
        z.push(zero.conj());
        let pole = Complex::with_val(
            bits(),
            &i * cd_sn(
                &Complex::with_val(bits(), cx(&u) - Complex::with_val(bits(), &i * &v0)),
                &k,
                false,
            ),
        );
        p.push(pole.clone());
        p.push(pole.conj());
    }
    if n % 2 == 1 {
        let pole = Complex::with_val(
            bits(),
            &i * cd_sn(&Complex::with_val(bits(), &i * &v0), &k, true),
        );
        p.push(cx(pole.real()));
    }
    let mut gain = Complex::with_val(bits(), cx(&product_real(&p)) / cx(&product_real(&z)))
        .real()
        .clone();
    if n.is_multiple_of(2) {
        gain /= (int(1) + eps_p.square()).sqrt();
    }
    Zpk { z, p, k: gain }
}

/// The filter of a prototype for the wanted band: `edges` in rad/s for a filter of
/// running time, as shares of half the sampling rate for a sampled one.
pub(crate) fn designed(
    proto: Zpk,
    edges: &[Float],
    kind: &str,
    analog: bool,
    who: &str,
) -> Result<(Vec<Float>, Vec<Float>), String> {
    let mut warped = Vec::new();
    for w in edges {
        if *w <= 0 || (!analog && *w >= 1) {
            return Err(match analog {
                true => format!("{who}: a frequency of the band is above zero"),
                false => format!(
                    "{who}: a frequency of a sampled filter lies between 0 and 1, where 1 is half the sampling rate; \"s\" as the last argument makes a filter of running time"
                ),
            });
        }
        warped.push(match analog {
            true => w.clone(),
            false => (pi() * w / 2u32).tan() * 4u32,
        });
    }
    let made = match (kind, warped.as_slice()) {
        ("low", [w]) => proto.lowpass(w),
        ("high", [w]) => proto.highpass(w),
        ("bandpass" | "stop", [low, high]) if low < high => {
            let (wo, bw) = (
                Float::with_val(bits(), low * high).sqrt(),
                Float::with_val(bits(), high - low),
            );
            match kind {
                "bandpass" => proto.bandpass(&wo, &bw),
                _ => proto.bandstop(&wo, &bw),
            }
        }
        ("low" | "high", _) => {
            return Err(format!("{who}: a lowpass or a highpass has one frequency"));
        }
        _ => {
            return Err(format!(
                "{who}: a bandpass or a bandstop has two frequencies [low, high], the lower first"
            ));
        }
    };
    Ok(match analog {
        true => made.tf(),
        false => made.bilinear(&int(2)).tf(),
    })
}

/// The words after the numbers of a design: the kind of the band, and "s" for a filter
/// of running time.
fn design_words(words: &[Expr], edges: usize, who: &str) -> Result<(String, bool), String> {
    let (mut kind, mut analog) = (None, false);
    for w in words {
        match word(w).as_deref() {
            Some("s") => analog = true,
            Some("z") => analog = false,
            Some("low" | "lowpass") => kind = Some("low"),
            Some("high" | "highpass") => kind = Some("high"),
            Some("bandpass" | "pass") => kind = Some("bandpass"),
            Some("stop" | "bandstop") => kind = Some("stop"),
            _ => {
                return Err(format!(
                    "{who}: {w} is none of \"low\", \"high\", \"bandpass\", \"stop\" and \"s\""
                ));
            }
        }
    }
    let kind = kind.unwrap_or(if edges == 2 { "bandpass" } else { "low" });
    Ok((kind.to_string(), analog))
}

// ---- filtering ----

/// The filter a[0]*y[n] = b[0]*x[n] + b[1]*x[n-1] + ... - a[1]*y[n-1] - ..., from the
/// given state (the transposed second direct form).
pub(crate) fn lfilter(
    b: &[Float],
    a: &[Float],
    x: &[Float],
    state: Option<&[Float]>,
) -> Vec<Float> {
    let n = b.len().max(a.len());
    let at = |p: &[Float], k: usize| {
        p.get(k).map_or_else(
            || Float::new(bits()),
            |c| Float::with_val(bits(), c / &a[0]),
        )
    };
    let (b, a): (Vec<Float>, Vec<Float>) = (
        (0..n).map(|k| at(b, k)).collect(),
        (0..n).map(|k| at(a, k)).collect(),
    );
    let mut z: Vec<Float> = match state {
        Some(s) => s.to_vec(),
        None => vec![Float::new(bits()); n.saturating_sub(1)],
    };
    z.push(Float::new(bits()));
    let mut out = Vec::with_capacity(x.len());
    for v in x {
        let y = Float::with_val(bits(), &b[0] * v) + &z[0];
        for k in 1..n {
            z[k - 1] =
                Float::with_val(bits(), &b[k] * v) + &z[k] - Float::with_val(bits(), &a[k] * &y);
        }
        out.push(y);
    }
    out
}

/// The state in which the filter rests on an input that stays 1.
pub(crate) fn resting(b: &[Float], a: &[Float]) -> Vec<Float> {
    let n = b.len().max(a.len());
    let at = |p: &[Float], k: usize| {
        p.get(k).map_or_else(
            || Float::new(bits()),
            |c| Float::with_val(bits(), c / &a[0]),
        )
    };
    let (b, a): (Vec<Float>, Vec<Float>) = (
        (0..n).map(|k| at(b, k)).collect(),
        (0..n).map(|k| at(a, k)).collect(),
    );
    if n < 2 {
        return Vec::new();
    }
    let rest: Vec<Float> = (1..n)
        .map(|k| Float::with_val(bits(), &b[k] - Float::with_val(bits(), &a[k] * &b[0])))
        .collect();
    let total_a = a.iter().fold(Float::new(bits()), |s, c| s + c);
    let first = rest.iter().fold(Float::new(bits()), |s, c| s + c) / total_a;
    let mut state = vec![first.clone()];
    let (mut asum, mut csum) = (int(1), Float::new(bits()));
    for k in 1..n - 1 {
        asum += &a[k];
        csum += &rest[k - 1];
        state.push(Float::with_val(bits(), &asum * &first) - &csum);
    }
    state
}

/// Forwards and backwards through the filter: no shift of the phase.
pub(crate) fn filtfilt(b: &[Float], a: &[Float], x: &[Float]) -> Result<Vec<Float>, String> {
    let edge = 3 * (b.len().max(a.len()) - 1);
    if x.len() <= edge {
        return Err(format!(
            "filtfilt: the data must have more than {edge} values, three times the order of the filter"
        ));
    }
    let n = x.len();
    let mut ext: Vec<Float> = (1..=edge)
        .rev()
        .map(|k| Float::with_val(bits(), &x[0] * 2u32) - &x[k])
        .collect();
    ext.extend(x.iter().cloned());
    ext.extend((1..=edge).map(|k| Float::with_val(bits(), &x[n - 1] * 2u32) - &x[n - 1 - k]));
    let rest = resting(b, a);
    let from = |first: &Float| {
        rest.iter()
            .map(|z| Float::with_val(bits(), z * first))
            .collect::<Vec<_>>()
    };
    let mut y = lfilter(b, a, &ext, Some(&from(&ext[0])));
    y.reverse();
    let mut y = lfilter(b, a, &y, Some(&from(&y[0])));
    y.reverse();
    Ok(y[edge..edge + n].to_vec())
}

/// B(e^(-iw))/A(e^(-iw)) at a frequency in radians a sample.
pub(crate) fn response_at(b: &[Float], a: &[Float], w: &Float) -> Complex {
    let back = unit(&Float::with_val(bits(), -w));
    let sum = |p: &[Float]| {
        p.iter()
            .rev()
            .fold(Complex::new(bits()), |s, c| s * &back + c)
    };
    sum(b) / sum(a)
}

/// The frequencies of a response: n of them from 0 up to pi, or the ones given.
pub(crate) fn frequencies(e: Option<&Expr>, who: &str) -> Result<Vec<Float>, String> {
    match e {
        Some(e) if e.as_list().is_some() => reals(e, who),
        Some(e) => {
            let n = count(e, "the number of frequencies", 1, 1 << 16)?;
            Ok((0..n).map(|k| pi() * k as u64 / n as u64).collect())
        }
        None => Ok((0..512u64).map(|k| pi() * k / 512u64).collect()),
    }
}

// ---- spectra ----

/// The transforms of the windowed pieces of a signal, each of `nfft` points.
fn pieces(x: &[Float], win: &[Float], overlap: usize, nfft: usize) -> Vec<Vec<Complex>> {
    let (length, step) = (win.len(), win.len() - overlap);
    let mut out = Vec::new();
    let mut start = 0;
    while start + length <= x.len() {
        let mut piece: Vec<Complex> = (0..length)
            .map(|k| cx(&Float::with_val(bits(), &x[start + k] * &win[k])))
            .collect();
        piece.resize(nfft.max(length), Complex::new(bits()));
        out.push(fft(&piece, false));
        start += step;
    }
    out
}

/// One side of a spectrum of nfft points: the values up to half the sampling rate, the
/// inner ones doubled, over `scale`.
fn one_side(values: Vec<Complex>, nfft: usize, scale: &Float) -> Vec<Complex> {
    let half = nfft / 2 + 1;
    values
        .into_iter()
        .take(half)
        .enumerate()
        .map(|(k, v)| {
            let inner = k > 0 && (nfft % 2 == 1 || k < nfft / 2);
            let v = v / scale;
            if inner { v * 2u32 } else { v }
        })
        .collect()
}

/// The options of a spectrum after the data: window, overlap, points, sampling rate.
struct Spectrum {
    win: Vec<Float>,
    overlap: usize,
    nfft: usize,
    rate: Option<Float>,
}

impl Spectrum {
    /// As pwelch reads them: pieces of an eighth of the data with a Hamming window, half
    /// of each shared with the next, unless said otherwise.
    fn welch(a: &[Expr], n: usize, who: &str) -> Result<Spectrum, String> {
        let given = |k: usize| {
            a.get(k)
                .filter(|e| !matches!(e.as_list(), Some(l) if l.is_empty()))
        };
        let win = match given(0) {
            Some(e) if e.as_list().is_none() && word(e).is_none() => window(
                "hamming",
                count(e, "the length of a piece", 1, n as i64)?,
                None,
            )?,
            Some(e) if word(e).is_some() => window_of(Some(e), (2 * n / 9).max(1), "hamming", who)?,
            Some(e) => reals(e, who)?,
            None => window("hamming", (2 * n / 9).max(1), None)?,
        };
        if win.is_empty() || win.len() > n {
            return Err(format!(
                "{who}: a piece has 1 to {n} values, as many as the data at most"
            ));
        }
        let overlap = match given(1) {
            Some(e) => count(e, "the overlap", 0, win.len() as i64 - 1)?,
            None => win.len() / 2,
        };
        let nfft = match given(2) {
            Some(e) => count(e, "the number of points", 1, 1 << 20)?,
            None => win.len().next_power_of_two().max(256),
        };
        let rate = match given(3) {
            Some(e) => Some(real(e, who)?),
            None => None,
        };
        Ok(Spectrum {
            win,
            overlap,
            nfft,
            rate,
        })
    }

    /// What the squared sizes are divided by: the rate (2 pi without one) times the sum
    /// of the squares of the window.
    fn scale(&self) -> Float {
        let power = self.win.iter().fold(Float::new(bits()), |s, w| {
            s + Float::with_val(bits(), w * w)
        });
        match &self.rate {
            Some(rate) => power * rate,
            None => power * pi() * 2u32,
        }
    }

    fn places(&self) -> Vec<Float> {
        let whole = match &self.rate {
            Some(rate) => rate.clone(),
            None => pi() * 2u32,
        };
        (0..=self.nfft / 2)
            .map(|k| Float::with_val(bits(), &whole * k as u64) / self.nfft as u64)
            .collect()
    }

    /// The mean over the pieces of conj(x)*y, one side.
    fn cross(&self, x: &[Float], y: &[Float]) -> Vec<Complex> {
        let nfft = self.nfft.max(self.win.len());
        let (px, py) = (
            pieces(x, &self.win, self.overlap, nfft),
            pieces(y, &self.win, self.overlap, nfft),
        );
        let mut sum = vec![Complex::new(bits()); nfft];
        for (u, v) in px.iter().zip(&py) {
            for k in 0..nfft {
                sum[k] +=
                    Complex::with_val(bits(), Complex::with_val(bits(), u[k].conj_ref()) * &v[k]);
            }
        }
        let scale = Float::with_val(bits(), self.scale() * px.len() as u64);
        one_side(sum, nfft, &scale)
    }
}

// ---- prediction ----

/// The recursion of Levinson and Durbin on autocorrelations r[0..=p]: the coefficients
/// of the predictor (the first is 1), the power of the error, the reflections.
pub(crate) fn levinson(
    r: &[Float],
    order: usize,
) -> Result<(Vec<Float>, Float, Vec<Float>), String> {
    if r.len() <= order || r[0].is_zero() {
        return Err(
            "levinson: the autocorrelations r(0), ..., r(p) are needed, and r(0) is not zero"
                .into(),
        );
    }
    let mut a = vec![int(1)];
    let mut error = r[0].clone();
    let mut reflections = Vec::new();
    for m in 1..=order {
        let mut acc = r[m].clone();
        for k in 1..m {
            acc += Float::with_val(bits(), &a[k] * &r[m - k]);
        }
        let k = -acc / &error;
        let mut next = a.clone();
        next.push(Float::new(bits()));
        for j in 1..=m {
            let other = if m - j < a.len() {
                a[m - j].clone()
            } else {
                Float::new(bits())
            };
            next[j] = if j < a.len() {
                a[j].clone()
            } else {
                Float::new(bits())
            } + Float::with_val(bits(), &k * other);
        }
        a = next;
        error *= int(1) - Float::with_val(bits(), &k * &k);
        reflections.push(k);
    }
    Ok((a, error, reflections))
}

/// The autocorrelations of a signal at the shifts 0..=p, over its length.
fn autocorrelations(x: &[Float], order: usize) -> Vec<Float> {
    (0..=order)
        .map(|shift| {
            let mut sum = Float::new(bits());
            for k in shift..x.len() {
                sum += Float::with_val(bits(), &x[k] * &x[k - shift]);
            }
            sum / x.len() as u64
        })
        .collect()
}

/// Burg's way to the predictor: forward and backward errors made least at every order.
fn burg(x: &[Float], order: usize) -> (Vec<Float>, Float, Vec<Float>) {
    let n = x.len();
    let (mut forward, mut backward) = (x.to_vec(), x.to_vec());
    let mut a = vec![int(1)];
    let mut error = x.iter().fold(Float::new(bits()), |s, v| {
        s + Float::with_val(bits(), v * v)
    }) / n as u64;
    let mut reflections = Vec::new();
    for m in 0..order {
        let (mut top, mut bottom) = (Float::new(bits()), Float::new(bits()));
        for k in m + 1..n {
            top += Float::with_val(bits(), &forward[k] * &backward[k - 1]);
            bottom += Float::with_val(bits(), &forward[k] * &forward[k])
                + Float::with_val(bits(), &backward[k - 1] * &backward[k - 1]);
        }
        let k = -top * 2u32 / bottom;
        let old = (forward.clone(), backward.clone());
        for j in m + 1..n {
            forward[j] = Float::with_val(
                bits(),
                &old.0[j] + Float::with_val(bits(), &k * &old.1[j - 1]),
            );
            backward[j] = Float::with_val(
                bits(),
                &old.1[j - 1] + Float::with_val(bits(), &k * &old.0[j]),
            );
        }
        let mut next = a.clone();
        next.push(Float::new(bits()));
        for j in 1..=m + 1 {
            let other = a[m + 1 - j].clone();
            next[j] = if j <= m {
                a[j].clone()
            } else {
                Float::new(bits())
            } + Float::with_val(bits(), &k * other);
        }
        a = next;
        error *= int(1) - Float::with_val(bits(), &k * &k);
        reflections.push(k);
    }
    (a, error, reflections)
}

// ---- peaks ----

/// The places of the values that are above both neighbours; of a flat top its first.
fn local_peaks(x: &[Float]) -> Vec<usize> {
    let mut found = Vec::new();
    let mut k = 1;
    while k + 1 < x.len() {
        if x[k] > x[k - 1] {
            let mut end = k;
            while end + 1 < x.len() && x[end + 1] == x[k] {
                end += 1;
            }
            if end + 1 < x.len() && x[end + 1] < x[k] {
                found.push(k);
            }
            k = end + 1;
        } else {
            k += 1;
        }
    }
    found
}

/// How far a peak stands above the higher of the lowest points on both ways to a higher
/// peak (or to the end).
fn prominence(x: &[Float], at: usize) -> Float {
    let side = |range: &mut dyn Iterator<Item = usize>| {
        let mut least = x[at].clone();
        for k in range {
            if x[k] > x[at] {
                break;
            }
            if x[k] < least {
                least = x[k].clone();
            }
        }
        least
    };
    let (left, right) = (side(&mut (0..at).rev()), side(&mut (at + 1..x.len())));
    Float::with_val(bits(), &x[at] - left.max(&right))
}

/// The numbers of a list every one through a function, or one number through it.
fn each(e: &Expr, who: &str, f: &dyn Fn(&Float) -> Float) -> Res {
    let values: Vec<Float> = reals(e, who)?.iter().map(f).collect();
    Ok(match e.as_list() {
        Some(_) => shown(&values),
        None => crate::mfun::value(&values[0]),
    })
}

/// x modulo m, from 0 up to m.
fn modulo(x: &Float, m: &Float) -> Float {
    let turns = Float::with_val(bits(), x / m).floor();
    Float::with_val(bits(), x - turns * m)
}

pub(crate) fn matrix_of(rows: &[Vec<Float>]) -> Expr {
    Expr::list(rows.iter().map(|r| shown(r)).collect())
}

pub(crate) fn rows_of(e: &Expr, who: &str) -> Result<Vec<Vec<Float>>, String> {
    let rows = e
        .as_list()
        .ok_or_else(|| format!("{who}: a matrix with a row for every section"))?;
    match rows.first().is_some_and(|r| r.as_list().is_some()) {
        true => rows.iter().map(|r| reals(r, who)).collect(),
        false => Ok(vec![reals(e, who)?]),
    }
}

/// The arguments with a filter that came as one value taken apart: the pair `[b, a]` or the three
/// `[zeros, poles, gain]`, as the functions that make filters give them. An error when there are
/// too few arguments for the function without such a value.
pub(crate) fn apart(name: &str, a: &[Expr]) -> Option<Result<Vec<Expr>, String>> {
    let flat = |e: &Expr| {
        e.as_list()
            .is_some_and(|l| !l.is_empty() && l.iter().all(|x| x.as_list().is_none()))
    };
    let row = |e: &Expr| {
        e.as_list()
            .is_some_and(|l| l.iter().all(|x| x.as_list().is_none()))
    };
    let (least, three) = match name {
        "freqz" | "grpdelay" | "impz" | "stepz" | "isminphase" | "ismaxphase" | "islinphase"
        | "zplane" | "tf" => (1, false),
        "tf2sos" | "tf2zp" | "tf2ss" | "select" => (2, false),
        "lfilter" | "filtfilt" | "freqs" | "lp2lp" | "lp2hp" | "bilinear" => (3, false),
        "lp2bp" | "lp2bs" => (4, false),
        "zpk" => (1, true),
        "zp2tf" | "zp2ss" | "zp2sos" => (3, true),
        _ => return None,
    };
    let parts = a.first().and_then(Expr::as_list).filter(|l| {
        if three {
            l.len() == 3 && row(&l[0]) && row(&l[1]) && l[2].as_list().is_none()
        } else {
            l.len() == 2
                && l.iter().all(flat)
                && (name != "select" || (a.len() == 2 && flat(&a[1])))
        }
    });
    match parts {
        Some(l) => Some(Ok(l.iter().chain(&a[1..]).cloned().collect())),
        None if a.len() < least => Some(Err(format!(
            "{name} takes {least} argument(s), {} given",
            a.len()
        ))),
        None => None,
    }
}

impl Session {
    /// The zeros of a polynomial with real coefficients, as numbers.
    fn signal_roots(&mut self, p: &[Float], who: &str) -> Result<Vec<Complex>, String> {
        let p = trimmed(p.to_vec());
        if p.len() < 2 {
            return Ok(Vec::new());
        }
        let found = self.call("roots", &[shown(&p)])?;
        complexes(&found, who)
    }

    /// The prototype of Bessel: the zeros of the reverse polynomial of Bessel, scaled so
    /// that the product of the poles is 1 (the phase is halfway at the frequency 1).
    fn besselap(&mut self, n: usize) -> Result<Zpk, String> {
        use rug::Integer;
        let factorial = |m: usize| (1..=m).fold(Integer::from(1), |p, k| p * k as u64);
        // the coefficient of s^k is (2n - k)!/(2^(n - k) k! (n - k)!)
        let coefficient = |k: usize| {
            factorial(2 * n - k)
                / (Integer::from(1) << (n - k) as u32)
                / factorial(k)
                / factorial(n - k)
        };
        let coefficients: Vec<Expr> = (0..=n)
            .rev()
            .map(|k| Expr::integer(coefficient(k)))
            .collect();
        let found = self.call("roots", &[Expr::list(coefficients)])?;
        let scale = Float::with_val(bits(), &coefficient(0)).pow(-(int(1) / n as u64));
        let mut p = complexes(&found, "besselap")?;
        for x in &mut p {
            *x *= &scale;
        }
        Ok(Zpk {
            z: Vec::new(),
            p,
            k: int(1),
        })
    }

    /// Sections of second order for zeros, poles and a gain: every pair of poles with
    /// the zeros nearest to it, the poles nearest to the unit circle last.
    fn sections(&mut self, z: &[Complex], p: &[Complex], k: &Float) -> Vec<Vec<Float>> {
        // pairs: a number with its conjugate, or two real ones, or one real one alone
        let paired = |v: &[Complex]| -> Vec<Vec<Complex>> {
            let tiny = Float::with_val(bits(), 1) >> (bits() / 2);
            let (mut complex, mut real): (Vec<Complex>, Vec<Complex>) = (Vec::new(), Vec::new());
            for x in v {
                if x.imag().clone().abs()
                    <= Float::with_val(bits(), &tiny * (int(1) + x.clone().abs().real()))
                {
                    real.push(cx(x.real()));
                } else if *x.imag() > 0 {
                    complex.push(x.clone());
                }
            }
            let mut pairs: Vec<Vec<Complex>> = complex
                .into_iter()
                .map(|x| vec![x.clone(), x.conj()])
                .collect();
            real.sort_by(|a, b| {
                a.real()
                    .clone()
                    .abs()
                    .partial_cmp(&b.real().clone().abs())
                    .unwrap_or(Ordering::Equal)
            });
            for two in real.chunks(2) {
                pairs.push(two.to_vec());
            }
            pairs
        };
        let far = |pair: &Vec<Complex>| {
            let size = pair
                .iter()
                .fold(Float::new(bits()), |m, x| m.max(x.clone().abs().real()));
            (int(1) - size).abs()
        };
        let mut poles = paired(p);
        let mut zeros = paired(z);
        // the poles nearest to the unit circle choose their zeros first
        poles.sort_by(|a, b| far(a).partial_cmp(&far(b)).unwrap_or(Ordering::Equal));
        let mut out = Vec::new();
        for pair in &poles {
            let nearest = (0..zeros.len()).min_by(|i, j| {
                let distance = |k: &usize| {
                    Complex::with_val(bits(), &zeros[*k][0] - &pair[0])
                        .abs()
                        .real()
                        .clone()
                };
                distance(i)
                    .partial_cmp(&distance(j))
                    .unwrap_or(Ordering::Equal)
            });
            let own = nearest.map(|k| zeros.remove(k)).unwrap_or_default();
            out.push((own, pair.clone()));
        }
        for own in zeros {
            out.push((own, Vec::new()));
        }
        out.reverse();
        let padded = |v: &[Complex]| {
            let mut c = real_parts(&poly_of_roots(v));
            while c.len() < 3 {
                c.push(Float::new(bits()));
            }
            c
        };
        let mut rows: Vec<Vec<Float>> = out
            .iter()
            .map(|(zs, ps)| {
                let mut row = padded(zs);
                row.extend(padded(ps));
                row
            })
            .collect();
        if rows.is_empty() {
            rows.push(vec![int(1), int(0), int(0), int(1), int(0), int(0)]);
        }
        let _ = k;
        rows
    }

    pub(crate) fn sigdesign(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        let n = a.len();
        // names that other values have too
        match name {
            "vartestn" if !(n == 1 && a[0].as_i64().is_some()) => return None,
            "hilbert" if a[0].as_list().is_none() => return None,
            _ => {}
        }
        Some((|| -> Res {
            let who = name;
            let number = |k: usize| real(&a[k], who);
            let list = |k: usize| reals(&a[k], who);
            let some = |v: Vec<Float>| match v.is_empty() {
                true => Err(format!("{who} needs values")),
                false => Ok(v),
            };
            let pair = |b: Vec<Float>, a: Vec<Float>| Expr::list(vec![shown(&b), shown(&a)]);
            Ok(match (name, n) {
                // ---- windows
                ("vartestn", 1) => shown(&window("bartlett", count(&a[0], "the number of values", 0, 1 << 20)?, None)?),
                (
                    "rectwin" | "hann" | "hanning" | "hamming" | "blackman" | "blackmanharris" | "nuttallwin" | "flattopwin"
                    | "triang" | "bohmanwin" | "parzenwin" | "gausswin" | "tukeywin" | "kaiser" | "chebwin" | "barthannwin",
                    1 | 2,
                ) => {
                    let size = count(&a[0], "the number of values", 0, 1 << 20)?;
                    match a.get(1).map(|e| (word(e), e)) {
                        None => shown(&window(name, size, None)?),
                        Some((Some(flag), _)) if flag == "periodic" => {
                            let mut w = window(name, size + 1, None)?;
                            w.pop();
                            shown(&w)
                        }
                        Some((Some(flag), _)) if flag == "symmetric" => shown(&window(name, size, None)?),
                        Some((None, shape)) if matches!(name, "gausswin" | "tukeywin" | "kaiser" | "chebwin") => {
                            shown(&window(name, size, Some(&real(shape, who)?))?)
                        }
                        Some(_) => return Err(format!("{name}(n), {name}(n, \"periodic\"); gausswin, tukeywin, kaiser and chebwin take their shape as a second number")),
                    }
                }
                // ---- prototypes and filters
                ("buttap", 1) => buttap(count(&a[0], "the order", 1, 500)?).shown(),
                ("cheb1ap", 2) => cheb1ap(count(&a[0], "the order", 1, 500)?, &number(1)?).shown(),
                ("cheb2ap", 2) => cheb2ap(count(&a[0], "the order", 1, 500)?, &number(1)?).shown(),
                ("ellipap", 3) => {
                    let (rp, rs) = (number(1)?, number(2)?);
                    if rp <= 0 || rs <= rp {
                        return Err("ellipap(n, Rp, Rs): the ripple Rp of the passband is above 0 and below the depth Rs of the stopband, both in dB".into());
                    }
                    ellipap(count(&a[0], "the order", 1, 200)?, &rp, &rs).shown()
                }
                ("besselap", 1) => self.besselap(count(&a[0], "the order", 1, 40)?)?.shown(),
                ("butter" | "besself", 2..=4) | ("cheby1" | "cheby2", 3..=5) | ("ellip", 4..=6) => {
                    let order = count(&a[0], "the order", 1, 200)?;
                    let at = match name {
                        "butter" | "besself" => 1,
                        "ellip" => 3,
                        _ => 2,
                    };
                    let edges = list(at)?;
                    let (kind, analog) = design_words(&a[at + 1..], edges.len(), who)?;
                    let proto = match name {
                        "butter" => buttap(order),
                        "cheby1" => cheb1ap(order, &number(1)?),
                        "cheby2" => cheb2ap(order, &number(1)?),
                        "ellip" => {
                            let (rp, rs) = (number(1)?, number(2)?);
                            if rp <= 0 || rs <= rp {
                                return Err("ellip(n, Rp, Rs, Wp): the ripple Rp is above 0 and below the depth Rs, both in dB".into());
                            }
                            ellipap(order, &rp, &rs)
                        }
                        _ => self.besselap(order)?,
                    };
                    // (the frequency of a filter of Chebyshev's second kind is where its stopband begins)
                    let proto = match name {
                        "cheby2" => proto,
                        _ => proto,
                    };
                    let (b, den) = designed(proto, &edges, &kind, analog || name == "besself", who)?;
                    pair(b, den)
                }
                ("buttord" | "cheb1ord" | "cheb2ord" | "ellipord", 4 | 5) => {
                    let analog = match a.get(4).map(word) {
                        None => false,
                        Some(Some(w)) if w == "s" => true,
                        Some(_) => return Err(format!("{name}(Wp, Ws, Rp, Rs[, \"s\"])")),
                    };
                    let (wp, ws, rp, rs) = (list(0)?, list(1)?, number(2)?, number(3)?);
                    if wp.len() != ws.len() || wp.is_empty() || wp.len() > 2 || rp <= 0 || rs <= rp {
                        return Err(format!("{name}(Wp, Ws, Rp, Rs): the edges of the passband and of the stopband, one of each or two; Rp above 0 and below Rs, in dB"));
                    }
                    let warp = |w: &Float| match analog {
                        true => w.clone(),
                        false => (pi() * w / 2u32).tan(),
                    };
                    let (passb, stopb): (Vec<Float>, Vec<Float>) = (wp.iter().map(warp).collect(), ws.iter().map(warp).collect());
                    // how far the stopband is from the passband, for a lowpass of edge 1
                    let (nat, kind) = match (passb.as_slice(), stopb.as_slice()) {
                        ([p], [s]) if p < s => (Float::with_val(bits(), s / p), "low"),
                        ([p], [s]) => (Float::with_val(bits(), p / s), "high"),
                        ([p1, p2], [s1, s2]) if s1 < p1 && p2 < s2 => {
                            let far = |s: &Float| {
                                (Float::with_val(bits(), s * s) - Float::with_val(bits(), p1 * p2)) / (Float::with_val(bits(), s * (Float::with_val(bits(), p1 - p2))))
                            };
                            (far(s1).abs().min(&far(s2).abs()), "bandpass")
                        }
                        _ => return Err(format!("{name}: a bandstop is not worked out here; a bandpass has its stopband edges outside the passband")),
                    };
                    let gstop = int(10).pow(Float::with_val(bits(), &rs / 10u32)) - 1u32;
                    let gpass = int(10).pow(Float::with_val(bits(), &rp / 10u32)) - 1u32;
                    let order = match name {
                        "buttord" => (Float::with_val(bits(), &gstop / &gpass).log10() / (nat.clone().log10() * 2u32)).ceil(),
                        "ellipord" => {
                            let (k, k1) = (nat.clone().recip(), Float::with_val(bits(), &gpass / &gstop).sqrt());
                            let other = |k: &Float| (int(1) - k.clone().square()).sqrt();
                            (quarter_period(&k) * quarter_period(&other(&k1)) / (quarter_period(&other(&k)) * quarter_period(&k1))).ceil()
                        }
                        _ => (Float::with_val(bits(), &gstop / &gpass).sqrt().acosh() / nat.clone().acosh()).ceil(),
                    };
                    let order = order.max(&int(1));
                    let back = |w: Float| match analog {
                        true => w,
                        false => w.atan() * 2u32 / pi(),
                    };
                    let natural: Vec<Float> = match name {
                        // the frequency where the gain is 3 dB down, so that the stopband is met exactly
                        "buttord" => {
                            let w0 = Float::with_val(bits(), &nat / gstop.clone().pow(int(1) / (order.clone() * 2u32)));
                            match (kind, passb.as_slice()) {
                                ("low", [p]) => vec![Float::with_val(bits(), &w0 * p)],
                                ("high", [p]) => vec![Float::with_val(bits(), p / &w0)],
                                (_, [p1, p2]) => {
                                    let width = Float::with_val(bits(), p2 - p1);
                                    let edge = |w: Float| {
                                        (-w.clone() * &width / 2u32 + (w.square() / 4u32 * width.clone().square() + Float::with_val(bits(), p1 * p2)).sqrt()).abs()
                                    };
                                    let mut both = vec![edge(-w0.clone()), edge(w0)];
                                    both.sort_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal));
                                    both
                                }
                                _ => passb.clone(),
                            }
                        }
                        // where the stopband of the second kind begins, so that the passband is met exactly
                        "cheb2ord" => {
                            let shift = (Float::with_val(bits(), &gstop / &gpass).sqrt().acosh() / &order).cosh().recip();
                            match (kind, passb.as_slice()) {
                                ("low", [p]) => vec![Float::with_val(bits(), p / &shift)],
                                ("high", [p]) => vec![Float::with_val(bits(), p * &shift)],
                                (_, [p1, p2]) => {
                                    let width = Float::with_val(bits(), p1 - p2);
                                    let root = (Float::with_val(bits(), width.clone().square()) / (shift.clone().square() * 4u32) + Float::with_val(bits(), p1 * p2)).sqrt();
                                    let half = Float::with_val(bits(), &width / (shift.clone() * 2u32));
                                    let low = Float::with_val(bits(), &half + &root);
                                    let high = Float::with_val(bits(), p1 * p2) / &low;
                                    vec![low, high]
                                }
                                _ => passb.clone(),
                            }
                        }
                        _ => passb.clone(),
                    };
                    let shown_edges: Vec<Float> = natural.into_iter().map(back).collect();
                    let edges = match shown_edges.len() {
                        1 => crate::mfun::value(&shown_edges[0]),
                        _ => shown(&shown_edges),
                    };
                    Expr::list(vec![crate::arrays::tidy(crate::mfun::value(&order)), edges])
                }
                ("lp2lp" | "lp2hp", 3) | ("lp2bp" | "lp2bs", 4) | ("bilinear", 3) => {
                    let (b, den, wo) = (trimmed(list(0)?), trimmed(list(1)?), number(2)?);
                    let (top, bottom): (Vec<Float>, Vec<Float>) = match name {
                        "lp2lp" => (vec![int(1), int(0)], vec![wo]),
                        "lp2hp" => (vec![wo], vec![int(1), int(0)]),
                        "bilinear" => (vec![Float::with_val(bits(), &wo * 2u32), Float::with_val(bits(), &wo * -2i32)], vec![int(1), int(1)]),
                        _ => {
                            let bw = number(3)?;
                            let (square, line) = (vec![int(1), int(0), Float::with_val(bits(), &wo * &wo)], vec![bw, int(0)]);
                            match name {
                                "lp2bp" => (square, line),
                                _ => (line, square),
                            }
                        }
                    };
                    let (b, den) = substituted(&b, &den, &top, &bottom);
                    pair(b, den)
                }
                ("iirnotch" | "iirpeak", 2) => {
                    let (w0, bw) = (number(0)?, number(1)?);
                    if w0 <= 0 || w0 >= 1 || bw <= 0 || bw >= 1 {
                        return Err(format!("{name}(w0, bw): the frequency and the width lie between 0 and 1, where 1 is half the sampling rate"));
                    }
                    let beta = (pi() * &bw / 2u32).tan();
                    let gain = (int(1) + &beta).recip();
                    let middle = Float::with_val(bits(), (pi() * &w0).cos() * -2i32);
                    let den = vec![int(1), Float::with_val(bits(), &gain * &middle), Float::with_val(bits(), &gain * 2u32) - 1u32];
                    let b: Vec<Float> = match name {
                        "iirnotch" => vec![gain.clone(), Float::with_val(bits(), &gain * &middle), gain],
                        _ => {
                            let rest = Float::with_val(bits(), 1u32 - &gain);
                            vec![rest.clone(), int(0), -rest]
                        }
                    };
                    pair(b, den)
                }
                ("fir1", 2..=5) => {
                    let mut order = count(&a[0], "the order", 1, 100_000)?;
                    let edges = list(1)?;
                    if edges.is_empty() || edges.len() > 2 || edges.iter().any(|w| *w <= 0 || *w >= 1) || (edges.len() == 2 && edges[0] >= edges[1]) {
                        return Err("fir1(n, Wn): one frequency or two rising ones, between 0 and 1, where 1 is half the sampling rate".into());
                    }
                    let (mut kind, mut scale, mut shape) = (None, true, None);
                    for e in &a[2..] {
                        match word(e).as_deref() {
                            Some("low" | "high" | "bandpass" | "stop") => kind = word(e),
                            Some("scale") => scale = true,
                            Some("noscale") => scale = false,
                            _ => shape = Some(e),
                        }
                    }
                    let kind = kind.unwrap_or_else(|| if edges.len() == 2 { "bandpass".into() } else { "low".into() });
                    // (a highpass and a bandstop have a gain at half the sampling rate: an even order)
                    if matches!(kind.as_str(), "high" | "stop") && order % 2 == 1 {
                        order += 1;
                    }
                    let bands: Vec<(Float, Float)> = match (kind.as_str(), edges.as_slice()) {
                        ("low", [w]) => vec![(int(0), w.clone())],
                        ("high", [w]) => vec![(w.clone(), int(1))],
                        ("bandpass", [low, high]) => vec![(low.clone(), high.clone())],
                        ("stop", [low, high]) => vec![(int(0), low.clone()), (high.clone(), int(1))],
                        _ => return Err("fir1: a lowpass or a highpass has one frequency, a bandpass or a bandstop two".into()),
                    };
                    let win = window_of(shape, order + 1, "hamming", who)?;
                    let middle = int(order as i64) / 2u32;
                    let sinc = |edge: &Float, m: &Float| match m.is_zero() {
                        true => edge.clone(),
                        false => (pi() * edge * m).sin() / (pi() * m),
                    };
                    let mut h: Vec<Float> = (0..=order)
                        .map(|k| {
                            let m = int(k as i64) - &middle;
                            let ideal = bands.iter().fold(Float::new(bits()), |s, (low, high)| s + sinc(high, &m) - sinc(low, &m));
                            ideal * &win[k]
                        })
                        .collect();
                    if scale {
                        let (low, high) = &bands[0];
                        let at = if low.is_zero() {
                            int(0)
                        } else if *high == 1 {
                            int(1)
                        } else {
                            Float::with_val(bits(), low + high) / 2u32
                        };
                        let gain = response_at(&h, &[int(1)], &(pi() * at)).abs().real().clone();
                        for v in &mut h {
                            *v /= &gain;
                        }
                    }
                    shown(&h)
                }
                ("kaiserord", 3 | 4) => {
                    let (mut f, amps, dev) = (list(0)?, list(1)?, list(2)?);
                    if let Some(rate) = a.get(3) {
                        let half = real(rate, who)? / 2u32;
                        for v in &mut f {
                            *v /= &half;
                        }
                    }
                    if f.len() < 2 || f.len() % 2 == 1 || amps.len() != f.len() / 2 + 1 || dev.len() != amps.len() {
                        return Err("kaiserord([band edges], [amplitudes], [deviations][, fs]): two edges between neighbouring bands, an amplitude and a deviation for every band".into());
                    }
                    let least = dev.iter().fold(Float::with_val(bits(), f64::INFINITY), |m, d| m.min(d));
                    let depth = -least.log10() * 20u32;
                    let beta = if depth > 50 {
                        dec("0.1102") * (depth.clone() - dec("8.7"))
                    } else if depth >= 21 {
                        dec("0.5842") * (depth.clone() - 21u32).pow(dec("0.4")) + dec("0.07886") * (depth.clone() - 21u32)
                    } else {
                        int(0)
                    };
                    let width = f.chunks(2).map(|e| Float::with_val(bits(), &e[1] - &e[0])).fold(Float::with_val(bits(), f64::INFINITY), |m, d| m.min(&d));
                    let dw = pi() * width;
                    let mut order = match depth > 21 {
                        true => ((depth - dec("7.95")) / (dec("2.285") * &dw)).ceil(),
                        false => (dec("5.79") / &dw).ceil(),
                    };
                    let cuts: Vec<Float> = f.chunks(2).map(|e| Float::with_val(bits(), &e[0] + &e[1]) / 2u32).collect();
                    let kind = match (amps[0] > amps[1], cuts.len()) {
                        (true, 1) => "low",
                        (false, 1) => "high",
                        (false, 2) => "bandpass",
                        (true, 2) => "stop",
                        (true, _) => "DC-1",
                        (false, _) => "DC-0",
                    };
                    // (a last band that passes needs an even order)
                    if amps[amps.len() - 1] > amps[amps.len() - 2] && Float::with_val(bits(), &order % 2u32) == 1 {
                        order += 1u32;
                    }
                    let edges = match cuts.len() {
                        1 => crate::mfun::value(&cuts[0]),
                        _ => shown(&cuts),
                    };
                    Expr::list(vec![crate::arrays::tidy(crate::mfun::value(&order)), edges, crate::mfun::value(&beta), crate::texts::text(kind)])
                }
                ("sgolay", 2) | ("sgolayfilt", 3) => {
                    let at = usize::from(name == "sgolayfilt");
                    let order = count(&a[at], "the order of the polynomial", 0, 60)?;
                    let frame = count(&a[at + 1], "the length of the frame", 1, 1001)?;
                    if frame % 2 == 0 || order >= frame {
                        return Err(format!("{name}: the frame has an odd length that is above the order of the polynomial"));
                    }
                    let half = (frame / 2) as i64;
                    let design: crate::matrix::Rows = (-half..=half)
                        .map(|x| (0..=order as u32).map(|k| Expr::integer(rug::Integer::from(x).pow(k))).collect())
                        .collect();
                    let turned = crate::matrix::transpose(&design);
                    let inverse = crate::matrix::inverse(&crate::matrix::matmul(&turned, &design)?)?;
                    let project = crate::matrix::matmul(&crate::matrix::matmul(&design, &inverse)?, &turned)?;
                    if name == "sgolay" {
                        return Ok(crate::matrix::from_rows(project));
                    }
                    let x = items(&a[0]);
                    if x.len() < frame {
                        return Err("sgolayfilt: the data has fewer values than the frame".into());
                    }
                    let dot = |row: &[Expr], from: usize| {
                        crate::arrays::tidy(crate::canon::add(row.iter().enumerate().map(|(j, c)| crate::canon::mul(vec![c.clone(), x[from + j].clone()])).collect()))
                    };
                    let (h, last) = (half as usize, x.len());
                    let smoothed: Vec<Expr> = (0..last)
                        .map(|i| {
                            if i < h {
                                dot(&project[i], 0)
                            } else if i + h >= last {
                                dot(&project[frame - (last - i)], last - frame)
                            } else {
                                dot(&project[h], i - h)
                            }
                        })
                        .collect();
                    Expr::list(smoothed)
                }
                // ---- responses
                ("freqz", 1..=3) => {
                    let (b, den) = (list(0)?, if n > 1 { list(1)? } else { vec![int(1)] });
                    let ws = frequencies(a.get(2), who)?;
                    let h: Vec<Complex> = ws.iter().map(|w| response_at(&b, &den, w)).collect();
                    Expr::list(vec![shown_c(&h), shown(&ws)])
                }
                ("freqs", 3) => {
                    let (b, den) = (list(0)?, list(1)?);
                    let h: Vec<Complex> = list(2)?
                        .iter()
                        .map(|w| {
                            let s = Complex::with_val(bits(), (0, w));
                            poly_at(&b, &s) / poly_at(&den, &s)
                        })
                        .collect();
                    match a[2].as_list() {
                        Some(_) => shown_c(&h),
                        None => shown_c(&h).as_list().map(|l| l[0].clone()).unwrap_or_else(Expr::undefined),
                    }
                }
                ("grpdelay", 1..=3) => {
                    let (b, den) = (list(0)?, if n > 1 { list(1)? } else { vec![int(1)] });
                    let ws = frequencies(a.get(2), who)?;
                    let mut reversed = den.clone();
                    reversed.reverse();
                    let c = poly_mul(&b, &reversed);
                    let ramp: Vec<Float> = c.iter().enumerate().map(|(k, v)| Float::with_val(bits(), v * k as u64)).collect();
                    let delays: Vec<Float> = ws
                        .iter()
                        .map(|w| {
                            let back = unit(&Float::with_val(bits(), -w));
                            let sum = |p: &[Float]| p.iter().rev().fold(Complex::new(bits()), |s, v| s * &back + v);
                            Complex::with_val(bits(), sum(&ramp) / sum(&c)).real().clone() - (den.len() as u64 - 1)
                        })
                        .collect();
                    Expr::list(vec![shown(&delays), shown(&ws)])
                }
                ("impz" | "stepz", 1..=3) => {
                    let (b, den) = (some(list(0)?)?, if n > 1 { some(list(1)?)? } else { vec![int(1)] });
                    let samples = match a.get(2) {
                        Some(e) => count(e, "the number of samples", 1, 1 << 20)?,
                        None if den.len() == 1 => b.len(),
                        None => 20,
                    };
                    let mut x = vec![int(if name == "stepz" { 1 } else { 0 }); samples];
                    x[0] = int(1);
                    shown(&lfilter(&b, &den, &x, None))
                }
                ("filtfilt", 3) => shown(&filtfilt(&list(0)?, &list(1)?, &list(2)?)?),
                ("sosfilt", 2) => {
                    let mut x = list(1)?;
                    for row in rows_of(&a[0], who)? {
                        if row.len() != 6 {
                            return Err("sosfilt(sections, x): every section is [b0, b1, b2, a0, a1, a2]".into());
                        }
                        x = lfilter(&row[..3], &row[3..], &x, None);
                    }
                    shown(&x)
                }
                ("sos2tf", 1 | 2) => {
                    let (mut b, mut den) = (vec![int(1)], vec![int(1)]);
                    for row in rows_of(&a[0], who)? {
                        if row.len() != 6 {
                            return Err("sos2tf(sections): every section is [b0, b1, b2, a0, a1, a2]".into());
                        }
                        b = poly_mul(&b, &row[..3]);
                        den = poly_mul(&den, &row[3..]);
                    }
                    if let Some(g) = a.get(1) {
                        let gain = real(g, who)?;
                        for v in &mut b {
                            *v *= &gain;
                        }
                    }
                    // (zeros at the end are powers of 1/z that both share)
                    while b.len() > 1 && den.len() > 1 && b[b.len() - 1].is_zero() && den[den.len() - 1].is_zero() {
                        b.pop();
                        den.pop();
                    }
                    pair(b, den)
                }
                ("tf2sos", 2) | ("zp2sos", 3) => {
                    let (z, p, k) = match name {
                        "tf2sos" => {
                            let (b, den) = (trimmed(list(0)?), trimmed(list(1)?));
                            let k = Float::with_val(bits(), &b[0] / &den[0]);
                            (self.signal_roots(&b, who)?, self.signal_roots(&den, who)?, k)
                        }
                        _ => (complexes(&a[0], who)?, complexes(&a[1], who)?, number(2)?),
                    };
                    let rows = self.sections(&z, &p, &k);
                    Expr::list(vec![matrix_of(&rows), crate::mfun::value(&k)])
                }
                ("isminphase" | "ismaxphase", 1 | 2) => {
                    let (b, den) = (list(0)?, if n > 1 { list(1)? } else { vec![int(1)] });
                    let (zeros, poles) = (self.signal_roots(&b, who)?, self.signal_roots(&den, who)?);
                    let inside = |v: &[Complex]| v.iter().all(|x| *x.clone().abs().real() < 1);
                    // a zero on the unit circle counts for both; one of m zeros that lie together
                    // is known m times less exactly
                    let digits = f64::from(numeric::digits());
                    let slack = |z: &Complex| {
                        let near = zeros.iter().filter(|o| *Complex::with_val(bits(), *o - z).abs().real() < 0.05).count();
                        Float::with_val(bits(), 10).pow(-2.0 * digits / (3.0 * near as f64))
                    };
                    let within = zeros.iter().all(|z| *z.clone().abs().real() <= Float::with_val(bits(), 1) + slack(z));
                    let without = zeros.iter().all(|z| *z.clone().abs().real() >= Float::with_val(bits(), 1) - slack(z));
                    Expr::Bool(match name {
                        "isminphase" => within && inside(&poles),
                        _ => without && inside(&poles),
                    })
                }
                ("islinphase", 1 | 2) | ("firtype", 1) => {
                    let b = trimmed(list(0)?);
                    let fir = n == 1 || trimmed(list(1)?).len() == 1;
                    let tiny = Float::with_val(bits(), 1) >> (bits() / 2);
                    let mirrored = |sign: i32| (0..b.len()).all(|k| Float::with_val(bits(), &b[k] - Float::with_val(bits(), &b[b.len() - 1 - k] * sign)).abs() <= tiny);
                    let (even, odd) = (mirrored(1), mirrored(-1));
                    match name {
                        "islinphase" => Expr::Bool(fir && (even || odd)),
                        _ => match (even, odd, b.len() % 2) {
                            (true, _, 1) => Expr::int(1),
                            (true, _, _) => Expr::int(2),
                            (_, true, 1) => Expr::int(3),
                            (_, true, _) => Expr::int(4),
                            _ => return Err("firtype: the filter has no linear phase (its coefficients are no mirror of themselves)".into()),
                        },
                    }
                }
                ("polystab", 1) => {
                    let p = some(trimmed(list(0)?))?;
                    let mut roots = self.signal_roots(&p, who)?;
                    for r in &mut roots {
                        if *r.clone().abs().real() > 1 {
                            *r = Complex::with_val(bits(), r.clone().conj().recip());
                        }
                    }
                    shown(&real_parts(&poly_of_roots(&roots)).into_iter().map(|c| c * &p[0]).collect::<Vec<_>>())
                }
                // ---- spectra
                ("periodogram", 1..=4) => {
                    let x = some(list(0)?)?;
                    let given = |k: usize| a.get(k).filter(|e| !matches!(e.as_list(), Some(l) if l.is_empty()));
                    let spectrum = Spectrum {
                        win: window_of(given(1), x.len(), "rectwin", who)?,
                        overlap: 0,
                        nfft: match given(2) {
                            Some(e) => count(e, "the number of points", 1, 1 << 20)?,
                            None => x.len().next_power_of_two().max(256),
                        },
                        rate: match given(3) {
                            Some(e) => Some(real(e, who)?),
                            None => None,
                        },
                    };
                    let power: Vec<Float> = spectrum.cross(&x, &x).iter().map(|v| v.real().clone()).collect();
                    Expr::list(vec![shown(&power), shown(&spectrum.places())])
                }
                ("pwelch", 1..=5) => {
                    let x = some(list(0)?)?;
                    let spectrum = Spectrum::welch(&a[1..], x.len(), who)?;
                    let power: Vec<Float> = spectrum.cross(&x, &x).iter().map(|v| v.real().clone()).collect();
                    Expr::list(vec![shown(&power), shown(&spectrum.places())])
                }
                ("cpsd" | "mscohere" | "tfestimate", 2..=6) => {
                    let (x, y) = (some(list(0)?)?, some(list(1)?)?);
                    if x.len() != y.len() {
                        return Err(format!("{name}: both signals have one length"));
                    }
                    let spectrum = Spectrum::welch(&a[2..], x.len(), who)?;
                    let pxy = spectrum.cross(&x, &y);
                    let values = match name {
                        "cpsd" => shown_c(&pxy),
                        _ => {
                            let (pxx, pyy) = (spectrum.cross(&x, &x), spectrum.cross(&y, &y));
                            match name {
                                "mscohere" => shown(
                                    &(0..pxy.len())
                                        .map(|k| Float::with_val(bits(), pxy[k].norm_ref()) / Float::with_val(bits(), pxx[k].real() * pyy[k].real()))
                                        .collect::<Vec<_>>(),
                                ),
                                // the transfer from x to y: Pxy/Pxx
                                _ => shown_c(&(0..pxy.len()).map(|k| Complex::with_val(bits(), &pxy[k] / pxx[k].real())).collect::<Vec<_>>()),
                            }
                        }
                    };
                    Expr::list(vec![values, shown(&spectrum.places())])
                }
                ("spectrogram", 1..=4) => {
                    let x = some(list(0)?)?;
                    let spectrum = Spectrum::welch(&a[1..], x.len(), who)?;
                    let nfft = spectrum.nfft.max(spectrum.win.len());
                    let columns = pieces(&x, &spectrum.win, spectrum.overlap, nfft);
                    Expr::list((0..=nfft / 2).map(|k| shown_c(&columns.iter().map(|c| c[k].clone()).collect::<Vec<_>>())).collect())
                }
                ("bandpower" | "rssq" | "peak2rms", 1) => {
                    let x = some(list(0)?)?;
                    if x.is_empty() {
                        return Err(format!("{name} needs values"));
                    }
                    let squares = x.iter().fold(Float::new(bits()), |s, v| s + Float::with_val(bits(), v * v));
                    let mean = Float::with_val(bits(), &squares / x.len() as u64);
                    crate::mfun::value(&match name {
                        "bandpower" => mean,
                        "rssq" => squares.sqrt(),
                        _ => x.iter().fold(Float::new(bits()), |m, v| m.max(&v.clone().abs())) / mean.sqrt(),
                    })
                }
                ("enbw", 1 | 2) => {
                    let w = list(0)?;
                    let (sum, squares) = w.iter().fold((Float::new(bits()), Float::new(bits())), |(s, q), v| (s + v, q + Float::with_val(bits(), v * v)));
                    let bins = squares * w.len() as u64 / sum.square();
                    crate::mfun::value(&match a.get(1) {
                        Some(rate) => bins * real(rate, who)? / w.len() as u64,
                        None => bins,
                    })
                }
                ("mag2db" | "db2mag" | "pow2db" | "db2pow" | "db", 1) => each(&a[0], who, &|x| match name {
                    "mag2db" | "db" => x.clone().abs().log10() * 20u32,
                    "pow2db" => x.clone().log10() * 10u32,
                    "db2mag" => int(10).pow(Float::with_val(bits(), x / 20u32)),
                    _ => int(10).pow(Float::with_val(bits(), x / 10u32)),
                })?,
                ("rceps", 1) => {
                    let x: Vec<Complex> = list(0)?.iter().map(cx).collect();
                    let logs: Vec<Complex> = fft(&x, false).iter().map(|v| cx(&v.clone().abs().real().clone().ln())).collect();
                    shown(&real_parts(&fft(&logs, true)))
                }
                ("goertzel", 2) => {
                    let x = some(list(0)?)?;
                    let mut out = Vec::new();
                    for e in items(&a[1]) {
                        let bin = count(&e, "the place of a frequency", 1, x.len() as i64)? - 1;
                        let step = unit(&(pi() * -2i32 * bin as u64 / x.len() as u64));
                        out.push(x.iter().rev().fold(Complex::new(bits()), |s, v| s * &step + v));
                    }
                    match a[1].as_list() {
                        Some(_) => shown_c(&out),
                        None => shown_c(&out).as_list().map(|l| l[0].clone()).unwrap_or_else(Expr::undefined),
                    }
                }
                ("czt", 1..=4) => {
                    let x = complexes(&a[0], who)?;
                    let m = match a.get(1) {
                        Some(e) => count(e, "the number of points", 1, 1 << 16)?,
                        None => x.len(),
                    };
                    let w = match a.get(2) {
                        Some(e) => numeric::evalf(e, bits()).map_err(|_| "czt: the ratio w is a number".to_string())?,
                        None => unit(&(pi() * -2i32 / m as u64)),
                    };
                    let start = match a.get(3) {
                        Some(e) => numeric::evalf(e, bits()).map_err(|_| "czt: the start a is a number".to_string())?,
                        None => Complex::with_val(bits(), 1),
                    };
                    let mut out = Vec::new();
                    let mut place = start.recip();
                    for _ in 0..m {
                        out.push(x.iter().rev().fold(Complex::new(bits()), |s, v| s * &place + v));
                        place *= &w;
                    }
                    shown_c(&out)
                }
                ("dftmtx", 1) => {
                    let size = count(&a[0], "the size", 1, 256)?;
                    let mut rows = Vec::new();
                    for k in 0..size {
                        let row: Vec<Expr> = (0..size).map(|j| Expr::int(i64::from(j == k))).collect();
                        rows.push(self.call("fft", &[Expr::list(row)])?);
                    }
                    Expr::list(rows)
                }
                ("cconv", 2 | 3) => {
                    let (x, y) = (some(list(0)?)?, some(list(1)?)?);
                    let full = poly_mul(&x, &y);
                    let size = match a.get(2) {
                        Some(e) => count(e, "the length", 1, 1 << 20)?,
                        None => full.len(),
                    };
                    let mut out = vec![Float::new(bits()); size];
                    for (k, v) in full.iter().enumerate() {
                        out[k % size] += v;
                    }
                    shown(&out)
                }
                ("hilbert", 1) | ("envelope", 1) => {
                    let x = some(list(0)?)?;
                    let size = x.len();
                    if size == 0 {
                        return Err(format!("{name} needs values"));
                    }
                    let mean = match name {
                        "envelope" => x.iter().fold(Float::new(bits()), |s, v| s + v) / size as u64,
                        _ => Float::new(bits()),
                    };
                    let mut t = fft(&x.iter().map(|v| cx(&Float::with_val(bits(), v - &mean))).collect::<Vec<_>>(), false);
                    for (k, v) in t.iter_mut().enumerate() {
                        // the frequencies above zero twice, the ones below not at all
                        if k == 0 || (size % 2 == 0 && k == size / 2) {
                        } else if k < size.div_ceil(2) {
                            *v *= 2u32;
                        } else {
                            *v = Complex::new(bits());
                        }
                    }
                    let analytic = fft(&t, true);
                    match name {
                        "hilbert" => shown_c(&analytic),
                        _ => {
                            let sizes: Vec<Float> = analytic.iter().map(|v| v.clone().abs().real().clone()).collect();
                            Expr::list(vec![
                                shown(&sizes.iter().map(|s| Float::with_val(bits(), &mean + s)).collect::<Vec<_>>()),
                                shown(&sizes.iter().map(|s| Float::with_val(bits(), &mean - s)).collect::<Vec<_>>()),
                            ])
                        }
                    }
                }
                // ---- peaks, medians, rates
                ("findpeaks", _) if n >= 1 && n % 2 == 1 => {
                    let x = list(0)?;
                    let mut found = local_peaks(&x);
                    let (mut distance, mut most, mut order) = (None, None, "none".to_string());
                    for option in a[1..].chunks(2) {
                        let key = word(&option[0]).unwrap_or_default().to_lowercase();
                        match key.as_str() {
                            "minpeakheight" => {
                                let least = real(&option[1], who)?;
                                found.retain(|k| x[*k] >= least);
                            }
                            "threshold" => {
                                let least = real(&option[1], who)?;
                                found.retain(|k| Float::with_val(bits(), &x[*k] - &x[*k - 1]).min(&Float::with_val(bits(), &x[*k] - &x[*k + 1])) >= least);
                            }
                            "minpeakprominence" => {
                                let least = real(&option[1], who)?;
                                found.retain(|k| prominence(&x, *k) >= least);
                            }
                            "minpeakdistance" => distance = Some(real(&option[1], who)?),
                            "npeaks" => most = Some(count(&option[1], "the number of peaks", 1, 1 << 30)?),
                            "sortstr" => order = word(&option[1]).unwrap_or_default().to_lowercase(),
                            _ => return Err(format!("findpeaks: {} is none of \"MinPeakHeight\", \"Threshold\", \"MinPeakProminence\", \"MinPeakDistance\", \"NPeaks\", \"SortStr\"", option[0])),
                        }
                    }
                    if let Some(least) = distance {
                        // the highest peaks first: each takes away the ones too near to it
                        let mut by_height = found.clone();
                        by_height.sort_by(|i, j| x[*j].partial_cmp(&x[*i]).unwrap_or(Ordering::Equal).then(i.cmp(j)));
                        let mut kept: Vec<usize> = Vec::new();
                        for k in by_height {
                            if kept.iter().all(|other| int((k as i64 - *other as i64).abs()) > least) {
                                kept.push(k);
                            }
                        }
                        kept.sort_unstable();
                        found = kept;
                    }
                    match order.as_str() {
                        "descend" => found.sort_by(|i, j| x[*j].partial_cmp(&x[*i]).unwrap_or(Ordering::Equal).then(i.cmp(j))),
                        "ascend" => found.sort_by(|i, j| x[*i].partial_cmp(&x[*j]).unwrap_or(Ordering::Equal).then(i.cmp(j))),
                        "none" => {}
                        _ => return Err("findpeaks: \"SortStr\" is \"ascend\", \"descend\" or \"none\"".into()),
                    }
                    if let Some(most) = most {
                        found.truncate(most);
                    }
                    let values = items(&a[0]);
                    Expr::list(vec![
                        Expr::list(found.iter().map(|k| values[*k].clone()).collect()),
                        Expr::list(found.iter().map(|k| Expr::int(*k as i64 + 1)).collect()),
                    ])
                }
                ("medfilt1", 1 | 2) => {
                    let x = list(0)?;
                    let size = match a.get(1) {
                        Some(e) => count(e, "the number of values of a median", 1, 1 << 20)?,
                        None => 3,
                    };
                    let before = if size % 2 == 1 { (size - 1) / 2 } else { size / 2 };
                    let values = items(&a[0]);
                    let mut out = Vec::new();
                    for k in 0..x.len() {
                        let mut part: Vec<(Float, Expr)> = (0..size)
                            .map(|j| match (k + j).checked_sub(before).filter(|at| *at < x.len()) {
                                Some(at) => (x[at].clone(), values[at].clone()),
                                None => (Float::new(bits()), Expr::zero()),
                            })
                            .collect();
                        part.sort_by(|p, q| p.0.partial_cmp(&q.0).unwrap_or(Ordering::Equal));
                        out.push(match size % 2 {
                            1 => part[size / 2].1.clone(),
                            _ => crate::arrays::tidy(crate::canon::div(
                                crate::canon::add(vec![part[size / 2 - 1].1.clone(), part[size / 2].1.clone()]),
                                Expr::int(2),
                            )),
                        });
                    }
                    Expr::list(out)
                }
                ("upfirdn", 2..=4) => {
                    let (x, h) = (list(0)?, list(1)?);
                    let up = if n > 2 { count(&a[2], "the factor up", 1, 1 << 16)? } else { 1 };
                    let down = if n > 3 { count(&a[3], "the factor down", 1, 1 << 16)? } else { 1 };
                    if x.is_empty() || h.is_empty() {
                        return Err("upfirdn(x, h[, p[, q]]): values and a filter".into());
                    }
                    let mut wide = vec![Float::new(bits()); (x.len() - 1) * up + 1];
                    for (k, v) in x.iter().enumerate() {
                        wide[k * up] = v.clone();
                    }
                    let full = poly_mul(&wide, &h);
                    shown(&full.into_iter().step_by(down).collect::<Vec<_>>())
                }
                ("decimate", 2 | 3) => {
                    let x = list(0)?;
                    let factor = count(&a[1], "the factor", 1, 1 << 16)?;
                    let order = if n > 2 { count(&a[2], "the order of the filter", 1, 13)? } else { 8 };
                    if factor == 1 {
                        return Ok(a[0].clone());
                    }
                    let edge = dec("0.8") / factor as u64;
                    let (b, den) = designed(cheb1ap(order, &dec("0.05")), &[edge], "low", false, who)?;
                    let smooth = filtfilt(&b, &den, &x)?;
                    let size = x.len();
                    let first = factor - (factor * size.div_ceil(factor) - size);
                    shown(&smooth.into_iter().skip(first - 1).step_by(factor).collect::<Vec<_>>())
                }
                ("buffer", 2 | 3) => {
                    let x = items(&a[0]);
                    let frame = count(&a[1], "the length of a frame", 1, 1 << 20)?;
                    let shared = if n > 2 { count(&a[2], "the overlap", 0, frame as i64 - 1)? } else { 0 };
                    let step = frame - shared;
                    // the first frame begins with zeros where it would share with one before it
                    let mut padded = vec![Expr::zero(); shared];
                    padded.extend(x);
                    let frames = (padded.len() - shared).div_ceil(step).max(1);
                    let at = |k: usize| padded.get(k).cloned().unwrap_or_else(Expr::zero);
                    Expr::list((0..frame).map(|row| Expr::list((0..frames).map(|column| at(column * step + row)).collect())).collect())
                }
                ("xcov", 1 | 2) => {
                    let centred = |e: &Expr| -> Res {
                        let v = reals(e, who)?;
                        let mean = v.iter().fold(Float::new(bits()), |s, x| s + x) / v.len().max(1) as u64;
                        Ok(shown(&v.iter().map(|x| Float::with_val(bits(), x - &mean)).collect::<Vec<_>>()))
                    };
                    let args: Vec<Expr> = a.iter().map(centred).collect::<Result<_, _>>()?;
                    self.call("xcorr", &args)?
                }
                // ---- prediction
                ("levinson", 1 | 2) => {
                    let r = list(0)?;
                    let order = if n > 1 { count(&a[1], "the order", 1, r.len() as i64 - 1)? } else { r.len().saturating_sub(1) };
                    let (coefficients, error, reflections) = levinson(&r, order)?;
                    Expr::list(vec![shown(&coefficients), crate::mfun::value(&error), shown(&reflections)])
                }
                ("aryule" | "arburg" | "lpc", 2) => {
                    let x = list(0)?;
                    let order = count(&a[1], "the order", 1, x.len() as i64 - 1)?;
                    let (coefficients, error, reflections) = match name {
                        "arburg" => burg(&x, order),
                        _ => levinson(&autocorrelations(&x, order), order)?,
                    };
                    match name {
                        "lpc" => Expr::list(vec![shown(&coefficients), crate::mfun::value(&error)]),
                        _ => Expr::list(vec![shown(&coefficients), crate::mfun::value(&error), shown(&reflections)]),
                    }
                }
                ("poly2rc", 1) => {
                    // the recursion of Levinson backwards
                    let p = list(0)?;
                    let mut current: Vec<Float> = p.iter().map(|c| Float::with_val(bits(), c / &p[0])).collect();
                    let mut reflections = Vec::new();
                    while current.len() > 1 {
                        let m = current.len() - 1;
                        let k = current[m].clone();
                        let rest = int(1) - Float::with_val(bits(), &k * &k);
                        if rest.is_zero() {
                            return Err("poly2rc: a reflection of size 1: the polynomial has a zero on the unit circle".into());
                        }
                        let lower: Vec<Float> = (0..m)
                            .map(|j| (Float::with_val(bits(), &current[j] - Float::with_val(bits(), &k * &current[m - j]))) / &rest)
                            .collect();
                        reflections.push(k);
                        current = lower;
                    }
                    reflections.reverse();
                    shown(&reflections)
                }
                ("rc2poly", 1) => {
                    let mut p = vec![int(1)];
                    for k in list(0)? {
                        let m = p.len();
                        let mut next = p.clone();
                        next.push(Float::new(bits()));
                        for j in 1..=m {
                            let other = p[m - j].clone();
                            next[j] = if j < m { p[j].clone() } else { Float::new(bits()) } + Float::with_val(bits(), &k * other);
                        }
                        p = next;
                    }
                    shown(&p)
                }
                // ---- waveforms
                ("sawtooth", 1 | 2) => {
                    let width = if n > 1 { number(1)? } else { int(1) };
                    if !(0..=1).contains(&width) {
                        return Err("sawtooth(t, width): the place of the top is from 0 to 1".into());
                    }
                    each(&a[0], who, &|t| {
                        let x = modulo(t, &(pi() * 2u32)) / (pi() * 2u32);
                        if x < width {
                            x * 2u32 / &width - 1u32
                        } else {
                            Float::with_val(bits(), 1u32 - &x) * 2u32 / Float::with_val(bits(), 1u32 - &width) - 1u32
                        }
                    })?
                }
                ("rectpuls", 1 | 2) => {
                    let width = if n > 1 { number(1)? } else { int(1) };
                    let half = Float::with_val(bits(), &width / 2u32);
                    each(&a[0], who, &|t| int(i64::from(*t >= Float::with_val(bits(), -&half) && *t < half)))?
                }
                ("tripuls", 1..=3) => {
                    let width = if n > 1 { number(1)? } else { int(1) };
                    let skew = if n > 2 { number(2)? } else { int(0) };
                    let half = Float::with_val(bits(), &width / 2u32);
                    let top = Float::with_val(bits(), &skew * &half);
                    each(&a[0], who, &|t| {
                        if *t <= Float::with_val(bits(), -&half) || *t >= half {
                            int(0)
                        } else if *t <= top {
                            (Float::with_val(bits(), t + &half)) / (Float::with_val(bits(), &top + &half))
                        } else {
                            (Float::with_val(bits(), &half - t)) / (Float::with_val(bits(), &half - &top))
                        }
                    })?
                }
                ("gauspuls", 3) => {
                    // a cosine under a bell whose spectrum is 6 dB down at the edges of the band
                    let (fc, bw) = (number(1)?, number(2)?);
                    let reference = int(10).pow(dec("-0.3"));
                    let variance = -(Float::with_val(bits(), &bw * &fc).square()) / (reference.ln() * 8u32);
                    let time = (variance * pi().square() * 4u32).recip();
                    each(&a[0], who, &|t| (-Float::with_val(bits(), t * t) / (time.clone() * 2u32)).exp() * (pi() * 2u32 * &fc * t).cos())?
                }
                ("chirp", 4 | 5) => {
                    let (f0, t1, f1) = (number(1)?, number(2)?, number(3)?);
                    let method = a.get(4).and_then(word).unwrap_or_else(|| "linear".into());
                    let phase = |t: &Float| -> Result<Float, String> {
                        Ok(match method.as_str() {
                            "linear" => {
                                let rate = Float::with_val(bits(), &f1 - &f0) / &t1;
                                pi() * 2u32 * (Float::with_val(bits(), &f0 * t) + rate * t * t / 2u32)
                            }
                            "quadratic" => {
                                let rate = Float::with_val(bits(), &f1 - &f0) / Float::with_val(bits(), &t1 * &t1);
                                pi() * 2u32 * (Float::with_val(bits(), &f0 * t) + rate * t * t * t / 3u32)
                            }
                            "logarithmic" => {
                                if f0 <= 0 || f1 <= 0 {
                                    return Err("chirp: a logarithmic sweep has frequencies above zero".into());
                                }
                                let ratio = Float::with_val(bits(), &f1 / &f0).pow(Float::with_val(bits(), t1.clone().recip()));
                                pi() * 2u32 * &f0 * (ratio.clone().pow(t) - 1u32) / ratio.ln()
                            }
                            _ => return Err("chirp: the sweep is \"linear\", \"quadratic\" or \"logarithmic\"".into()),
                        })
                    };
                    let values: Vec<Float> = list(0)?.iter().map(|t| phase(t).map(Float::cos)).collect::<Result<_, _>>()?;
                    match a[0].as_list() {
                        Some(_) => shown(&values),
                        None => crate::mfun::value(&values[0]),
                    }
                }
                ("diric", 2) => {
                    let size = count(&a[1], "the number n", 1, 1 << 30)? as u64;
                    each(&a[0], who, &|x| {
                        let half = Float::with_val(bits(), x / 2u32);
                        let bottom = half.clone().sin() * size;
                        match bottom.clone().abs() < Float::with_val(bits(), 1) >> (bits() / 2) {
                            // (at the multiples of 2 pi the value is 1 or -1)
                            true => {
                                let turns = Float::with_val(bits(), x / (pi() * 2u32)).round();
                                let odd = Float::with_val(bits(), &turns * (size - 1)) % 2u32 != 0;
                                int(if odd { -1 } else { 1 })
                            }
                            false => (half * size).sin() / bottom,
                        }
                    })?
                }
                _ => return Err(String::new()),
            })
        })())
        .filter(|r| !matches!(r, Err(e) if e.is_empty()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: &Float, b: f64) -> bool {
        (a.to_f64() - b).abs() < 1e-12
    }

    #[test]
    fn windows_are_what_their_formulas_say() {
        let hann = window("hann", 5, None).unwrap();
        for (value, want) in hann.iter().zip([0.0, 0.5, 1.0, 0.5, 0.0]) {
            assert!(near(value, want));
        }
        let triangle = window("triang", 4, None).unwrap();
        for (value, want) in triangle.iter().zip([0.25, 0.75, 0.75, 0.25]) {
            assert!(near(value, want));
        }
        // a Kaiser window of shape 0 is flat; every window is its own mirror
        assert!(
            window("kaiser", 6, Some(&int(0)))
                .unwrap()
                .iter()
                .all(|v| near(v, 1.0))
        );
        for name in WINDOWS {
            let w = window(name, 9, None).unwrap();
            for k in 0..9 {
                assert!((w[k].to_f64() - w[8 - k].to_f64()).abs() < 1e-12, "{name}");
            }
        }
    }

    #[test]
    fn a_butterworth_lowpass_is_3_db_down_at_its_edge() {
        let (b, a) = designed(buttap(4), &[fl(0.3)], "low", false, "butter").unwrap();
        let gain = response_at(&b, &a, &(pi() * fl(0.3))).abs().real().to_f64();
        assert!((gain - 0.5f64.sqrt()).abs() < 1e-12, "{gain}");
        let rest = response_at(&b, &a, &int(0)).abs().real().to_f64();
        assert!((rest - 1.0).abs() < 1e-12);
        // an elliptic prototype has the ripple of its passband at the edge
        let proto = ellipap(5, &int(1), &int(40));
        let (b, a) = proto.tf();
        let edge = Complex::with_val(bits(), (0, 1));
        let gain = Complex::with_val(bits(), poly_at(&b, &edge) / poly_at(&a, &edge))
            .abs()
            .real()
            .to_f64();
        assert!((20.0 * gain.log10() + 1.0).abs() < 1e-9, "{gain}");
    }

    #[test]
    fn filtering_both_ways_keeps_a_line_and_levinson_finds_the_predictor() {
        let (b, a) = designed(buttap(2), &[fl(0.2)], "low", false, "butter").unwrap();
        // what does not change comes out as it is; a line does away from its ends, where
        // the filter starts from rest at a level
        let level: Vec<Float> = (0..40).map(|_| int(7)).collect();
        for got in filtfilt(&b, &a, &level).unwrap() {
            assert!((got.to_f64() - 7.0).abs() < 1e-12);
        }
        let line: Vec<Float> = (0..40).map(|k| int(2 * k + 1)).collect();
        let smooth = filtfilt(&b, &a, &line).unwrap();
        for (got, want) in smooth.iter().zip(&line).skip(15).take(10) {
            assert!((got.to_f64() - want.to_f64()).abs() < 1e-3, "{got} {want}");
        }
        assert!((smooth[0].to_f64() - 1.0).abs() > 0.01);
        // r(k) = 0.5^k is the autocorrelation of x[n] = 0.5 x[n-1] + noise
        let r: Vec<Float> = (0..4).map(|k| fl(0.5f64.powi(k))).collect();
        let (coefficients, error, _) = levinson(&r, 3).unwrap();
        assert!(near(&coefficients[1], -0.5) && near(&coefficients[2], 0.0) && near(&error, 0.75));
        assert_eq!(
            local_peaks(&[int(0), int(2), int(1), int(3), int(3), int(1)]),
            [1, 3]
        );
    }
}
