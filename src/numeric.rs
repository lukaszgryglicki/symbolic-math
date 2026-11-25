//! Arbitrary-precision complex evaluation of expressions.

use crate::assume;
use crate::canon::{pow, rebuild};
use crate::constants;
use crate::expr::{Expr, Rel};
use crate::num::Flt;
use crate::special;
use crate::tetration;
use rug::float::{Constant, Special};
use rug::ops::Pow as _;
use rug::{Assign, Complex, Float, Integer};
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub enum NumErr {
    Free(String),
    Unsupported(String),
    Undefined(String),
    External(String),
}

impl fmt::Display for NumErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NumErr::Free(s) => write!(f, "no numeric value: {s} is a free variable"),
            NumErr::Unsupported(s) => write!(f, "no numeric evaluation for {s}"),
            NumErr::Undefined(s) => write!(f, "undefined value: {s}"),
            NumErr::External(s) => write!(f, "{s}"),
        }
    }
}

thread_local! {
    static DIGITS: Cell<u32> = const { Cell::new(32) };
}

/// Significant decimal digits of the session.
pub fn digits() -> u32 {
    DIGITS.with(Cell::get)
}
pub fn set_digits(d: u32) {
    DIGITS.with(|c| c.set(d.clamp(1, 1_000_000)));
}
pub fn bits_for(digits: u32) -> u32 {
    (f64::from(digits) * std::f64::consts::LOG2_10).ceil() as u32 + 4
}
pub fn digits_for(bits: u32) -> u32 {
    (f64::from(bits) / std::f64::consts::LOG2_10).floor() as u32
}

pub type Env = HashMap<Rc<str>, Complex>;

pub struct Ctx<'a> {
    pub prec: u32,
    pub env: Option<&'a Env>,
    /// Largest binary exponent among the terms of the sums met so far.
    pub max_exp: i64,
}

pub fn evalf(e: &Expr, prec: u32) -> Result<Complex, NumErr> {
    eval(
        e,
        &mut Ctx {
            prec,
            env: None,
            max_exp: i64::MIN,
        },
    )
}

pub fn evalf_env(e: &Expr, prec: u32, env: &Env) -> Result<Complex, NumErr> {
    eval(
        e,
        &mut Ctx {
            prec,
            env: Some(env),
            max_exp: i64::MIN,
        },
    )
}

/// Negative zero parts become positive: a point on a branch cut is on its upper side.
pub fn unsigned_zero(mut z: Complex) -> Complex {
    if z.real().is_zero() {
        z.mut_real().assign(Special::Zero);
    }
    if z.imag().is_zero() {
        z.mut_imag().assign(Special::Zero);
    }
    z
}

fn magnitude(z: &Complex) -> i64 {
    let part = |x: &Float| x.get_exp().map_or(i64::MIN, i64::from);
    part(z.real()).max(part(z.imag()))
}

fn finite(z: Complex, what: &dyn Fn() -> String) -> Result<Complex, NumErr> {
    if z.real().is_finite() && z.imag().is_finite() {
        Ok(unsigned_zero(z))
    } else {
        Err(NumErr::Undefined(what()))
    }
}

/// The logarithm of a factor of a product, up to whole turns of its angle: of a power the
/// exponent times that of the base, of the gamma function its own logarithm, of anything
/// else the logarithm of its value. `None`: it has none (a factor that is 0, or no number).
fn logarithm_of(e: &Expr, cx: &mut Ctx) -> Option<Complex> {
    match e {
        Expr::Mul(factors) => {
            let mut sum = Complex::new(cx.prec);
            for f in factors.iter() {
                sum += logarithm_of(f, cx)?;
            }
            Some(sum)
        }
        Expr::Pow(b, x) if !b.is_sym("e") => Some(logarithm_of(b, cx)? * eval(x, cx).ok()?),
        Expr::Pow(_, x) => eval(x, cx).ok(),
        Expr::Fn(name, args) if matches!(&**name, "gamma" | "factorial") && args.len() == 1 => {
            let mut at = eval(&args[0], cx).ok()?;
            if &**name == "factorial" {
                at += 1u32;
            }
            special::log_gamma(&at, cx.prec)?
        }
        // (n over k: n!/(k! (n - k)!))
        Expr::Fn(name, args) if &**name == "binomial" && args.len() == 2 => {
            let (n, k) = (eval(&args[0], cx).ok()?, eval(&args[1], cx).ok()?);
            let of = |z: Complex| special::log_gamma(&(z + 1u32), cx.prec);
            let rest = Complex::with_val(cx.prec, &n - &k);
            Some(of(n)?? - of(k)?? - of(rest)??)
        }
        _ => {
            let v = eval(e, cx).ok()?;
            (!v.is_zero()).then(|| cln(v))
        }
    }
}

/// A product by the sum of the logarithms of its factors, with more bits than the product
/// is asked for (the logarithms are large where the factors are, and their sum is not).
/// `None`: not this way either.
fn product_by_logarithms(factors: &[Expr], cx: &mut Ctx) -> Option<Complex> {
    let mut wide = Ctx {
        prec: cx.prec + 128,
        env: cx.env,
        max_exp: i64::MIN,
    };
    let (value, beyond) = underflowed(|| {
        let mut sum = Complex::new(wide.prec);
        for f in factors {
            sum += logarithm_of(f, &mut wide)?;
        }
        Some(Complex::with_val(cx.prec, cexp(sum)))
    });
    value.filter(|v| !beyond && !v.is_zero() && v.real().is_finite() && v.imag().is_finite())
}

/// The size of an exponent beyond which `e` to it is no number that is kept: the exponents of
/// two that the numbers have go to 2^30, and e^745000000 is 2^1074807806.
const OUT_OF_RANGE: f64 = 7.45e8;

/// `e^z` as a method of a complex number: by `cexp`, not by the library alone.
pub trait SafeExp {
    fn cexp(self) -> Complex;
}

impl SafeExp for Complex {
    fn cexp(self) -> Complex {
        cexp(self)
    }
}

/// `e^z`. The library works with as many more bits as a part of `z` is smaller than 1, twice
/// that for the imaginary part, so that a part like 1e-90000 makes it take forever: a part
/// that is far below the bits at hand takes the first term of its series, which is all that
/// these bits can hold.
pub fn cexp(z: Complex) -> Complex {
    let prec = z.prec().0;
    // a size beyond the exponents of the numbers that are kept: 0, or larger than every
    // number, as the library would say. It is not asked: where the size comes near the end
    // of the widest range of exponents that it has (e^z for z = (2^63 - 1) ln((1 + i)/2) is
    // 2^(-2^62) in size), it does not end
    if z.real().is_finite() && z.imag().is_finite() {
        let size = z.real().to_f64();
        if size < -OUT_OF_RANGE {
            // SAFETY: sets a flag of MPFR, which belongs to this thread.
            unsafe { mpfr_set_underflow() };
            return Complex::new(prec);
        }
        if size > OUT_OF_RANGE {
            // SAFETY: as above.
            unsafe { mpfr_set_overflow() };
            let (sin, cos) = Float::with_val(prec, z.imag()).sin_cos(Float::new(prec));
            let endless = |part: Float| match part.cmp0() {
                Some(std::cmp::Ordering::Less) => Float::with_val(prec, Special::NegInfinity),
                Some(std::cmp::Ordering::Greater) => Float::with_val(prec, Special::Infinity),
                _ => Float::new(prec),
            };
            return Complex::with_val(prec, (endless(cos), endless(sin)));
        }
    }
    let far_below = |x: &Float| x.get_exp().is_some_and(|e| e < -2 * (prec as i32) - 64);
    let (low_re, low_im) = (far_below(z.real()), far_below(z.imag()));
    if !low_re && !low_im {
        return z.exp();
    }
    let (x, y) = z.into_real_imag();
    if low_im {
        // cos(y) is 1 and sin(y) is y
        let size = if low_re {
            Float::with_val(prec, 1)
        } else {
            x.exp()
        };
        let turn = Float::with_val(prec, &size * &y);
        Complex::with_val(prec, (size, turn))
    } else {
        // e^x is 1
        let (sin, cos) = y.sin_cos(Float::new(prec));
        Complex::with_val(prec, (cos, sin))
    }
}

/// `a/b`. Where the parts of one of the numbers are further apart than `gap`: times the
/// conjugate of `b`, over the square of its size. (The library rounds each part of a
/// quotient as if it stood alone, with as many bits as the parts of the numbers are apart:
/// where a function has values as 2^(10^9) + i, that takes all the memory.)
pub fn cdiv(a: &Complex, b: &Complex) -> Complex {
    let prec = a.prec().0.max(b.prec().0);
    let apart = |z: &Complex| match (z.real().get_exp(), z.imag().get_exp()) {
        (Some(re), Some(im)) => (i64::from(re) - i64::from(im)).abs() > gap(prec),
        _ => false,
    };
    if !apart(a) && !apart(b) {
        return Complex::with_val(prec, a / b);
    }
    let turned = Complex::with_val(prec, b.conj_ref());
    let norm = Float::with_val(prec, b.norm_ref());
    Complex::with_val(prec, a * &turned) / norm
}

/// `ln(z)`. Where one part of `z` is smaller than the other by more than all the bits at hand
/// (a root that is followed towards the real line has such an imaginary part), the library
/// works with as many bits as the two are apart: there the size of the larger part is the
/// size of `z`, as far as these bits tell, and the angle is that of the two parts.
pub fn cln(z: Complex) -> Complex {
    let prec = z.prec().0;
    let (Some(re), Some(im)) = (z.real().get_exp(), z.imag().get_exp()) else {
        return z.ln();
    };
    if (i64::from(re) - i64::from(im)).abs() <= 2 * i64::from(prec) + 64 {
        return z.ln();
    }
    let larger = if re > im { z.real() } else { z.imag() };
    let size = Float::with_val(prec, larger.abs_ref()).ln();
    let angle = Float::with_val(prec, z.imag().atan2_ref(z.real()));
    Complex::with_val(prec, (size, angle))
}

pub fn cpow(x: &Complex, y: &Complex) -> Option<Complex> {
    let prec = x.prec().0.max(y.prec().0);
    if x.is_zero() {
        return y
            .real()
            .is_sign_positive()
            .then(|| Complex::new(prec))
            .filter(|_| !y.real().is_zero());
    }
    let x = unsigned_zero(x.clone());
    // a whole power: by squaring, so that a real base keeps a real value
    if y.imag().is_zero()
        && y.real().is_integer()
        && let Some(k) = y.real().to_integer().and_then(|k| k.to_i32())
        && k.unsigned_abs() <= 1 << 16
    {
        let r = int_pow(&x, k, prec);
        return (r.real().is_finite() && r.imag().is_finite()).then(|| unsigned_zero(r));
    }
    // a positive number to a real power: exact where the power is
    if x.imag().is_zero() && y.imag().is_zero() && x.real().is_sign_positive() {
        let r = Float::with_val(prec, x.real().pow(y.real()));
        return r.is_finite().then(|| Complex::with_val(prec, (r, 0)));
    }
    // exp(y ln x): the angle needs as many more bits as the exponent has, and an exponent
    // beyond every precision gives no number
    let rough = cln(Complex::with_val(64, &x)) * Complex::with_val(64, y);
    let extra = Float::with_val(64, rough.abs_ref())
        .get_exp()
        .unwrap_or(0)
        .max(0) as u32;
    if extra > 4 * prec + 1024 {
        return None;
    }
    let work = prec + extra + 32;
    let exponent = cln(Complex::with_val(work, &x)) * Complex::with_val(work, y);
    // (a size beyond the exponents of the numbers that are kept: larger than every number, or
    // 0 as the power of a positive number is then. MPC would look for the digits of the
    // other part, with as many bits as the two are apart.)
    let size = exponent.real().to_f64();
    if size > OUT_OF_RANGE {
        // SAFETY: sets a flag of MPFR, which belongs to this thread.
        unsafe { mpfr_set_overflow() };
        return None;
    }
    let r = Complex::with_val(prec, cexp(exponent));
    (r.real().is_finite() && r.imag().is_finite()).then(|| unsigned_zero(r))
}

/// A whole power by squaring: the general power takes a logarithm and an exponential.
pub fn int_pow(base: &Complex, k: i32, prec: u32) -> Complex {
    let count = k.unsigned_abs();
    let work = prec + 2 * (32 - count.leading_zeros()) + 8;
    let mut result = Complex::with_val(work, 1);
    let mut square = Complex::with_val(work, base);
    let mut left = count;
    while left > 0 {
        if left & 1 == 1 {
            result *= &square;
        }
        left >>= 1;
        if left > 0 {
            square.square_mut();
        }
    }
    if k < 0 {
        // (the conjugate by the square of the size, not the correctly rounded reciprocal: of a
        // number with one part smaller than the other by more than any precision, as
        // 2^10000000 + i, MPC rounds it only with as many bits as the parts are apart)
        let norm = Float::with_val(work, result.norm_ref());
        if norm.is_finite() && !norm.is_zero() {
            result.conj_mut();
            result /= norm;
        } else {
            result.recip_mut();
        }
    }
    Complex::with_val(prec, result)
}

fn real_only(name: &str, z: &Complex) -> Result<Float, NumErr> {
    if z.imag().is_zero() {
        Ok(z.real().clone())
    } else {
        Err(NumErr::Unsupported(format!("{name} of a complex number")))
    }
}

fn per_part(z: &Complex, f: fn(Float) -> Float) -> Complex {
    let p = z.prec().0;
    Complex::with_val(p, (f(z.real().clone()), f(z.imag().clone())))
}

fn round_away(x: Float) -> Float {
    x.round()
}

fn eval(e: &Expr, cx: &mut Ctx) -> Result<Complex, NumErr> {
    let v = eval_node(e, cx)?;
    cx.max_exp = cx.max_exp.max(magnitude(&v));
    Ok(v)
}

fn eval_node(e: &Expr, cx: &mut Ctx) -> Result<Complex, NumErr> {
    let p = cx.prec;
    match e {
        Expr::Num(n) => Ok(n.to_complex(p)),
        Expr::Float(f) => Ok(Complex::with_val(p, &f.v)),
        Expr::Sym(s) => {
            if let Some(v) = cx.env.and_then(|env| env.get(s)) {
                return Ok(Complex::with_val(p, v));
            }
            let real = match &**s {
                "pi" => Float::with_val(p, Constant::Pi),
                "e" => Float::with_val(p, 1).exp(),
                "inf" | "ComplexInfinity" | "undefined" => {
                    return Err(NumErr::Undefined(s.to_string()));
                }
                name => match constants::value(name, p) {
                    Some(v) => v.map_err(NumErr::External)?,
                    None => return Err(NumErr::Free(s.to_string())),
                },
            };
            Ok(Complex::with_val(p, real))
        }
        Expr::Add(terms) => {
            let mut sum = Complex::new(p);
            for t in terms.iter() {
                sum += eval(t, cx)?;
            }
            Ok(sum)
        }
        Expr::Mul(factors) => {
            let (direct, beyond) = underflowed(|| -> Result<Complex, NumErr> {
                let mut prod = Complex::with_val(p, 1);
                for f in factors.iter() {
                    prod *= eval(f, cx)?;
                }
                finite(prod, &|| e.to_string())
            });
            // factors beyond the exponents of the numbers that are kept, whose product is a
            // number like any other (the density of a distribution with large parameters is
            // a power with 10^9 digits over another one): by the sum of their logarithms
            let lost = match &direct {
                Ok(v) => v.is_zero(),
                Err(_) => true,
            };
            if beyond
                && lost
                && let Some(v) = product_by_logarithms(factors, cx)
            {
                return finite(v, &|| e.to_string());
            }
            direct
        }
        Expr::Pow(b, x) => {
            if b.is_sym("e") {
                return finite(cexp(eval(x, cx)?), &|| e.to_string());
            }
            let base = eval(b, cx)?;
            if let Some(k) = x.as_i64().and_then(|k| i32::try_from(k).ok()) {
                if base.is_zero() && k < 0 {
                    return Err(NumErr::Undefined(e.to_string()));
                }
                return finite(int_pow(&base, k, p), &|| e.to_string());
            }
            if x.as_rat().is_some_and(|r| *r == crate::num::rat(1, 2)) {
                return Ok(unsigned_zero(unsigned_zero(base).sqrt()));
            }
            if crate::assume::real_mode()
                && let Some(r) = x.as_rat()
                && r.denom().is_odd()
                && base.imag().is_zero()
                && base.real().is_sign_negative()
            {
                // the real odd root of a negative number
                let size = Float::with_val(p, -base.real()).pow(Float::with_val(p, r));
                let value = if r.numer().is_odd() { -size } else { size };
                return finite(Complex::with_val(p, (value, 0)), &|| e.to_string());
            }
            let y = eval(x, cx)?;
            cpow(&base, &y).ok_or_else(|| NumErr::Undefined(e.to_string()))
        }
        Expr::Fn(name, args) if &**name == "piecewise" => {
            let (pairs, otherwise) = crate::logic::pieces(args);
            for (cond, value) in pairs {
                if holds(cond, cx)? {
                    return eval(value, cx);
                }
            }
            match otherwise {
                Some(value) => eval(value, cx),
                None => Err(NumErr::Undefined(e.to_string())),
            }
        }
        Expr::Fn(name, args)
            if &**name == "hypergeometric"
                && args.len() == 3
                && matches!((&args[0], &args[1]), (Expr::List(_), Expr::List(_))) =>
        {
            let mut lists = Vec::new();
            for list in &args[..2] {
                let mut values = Vec::new();
                for x in list.as_list().unwrap_or_default() {
                    values.push(unsigned_zero(eval(x, cx)?));
                }
                lists.push(values);
            }
            let z = unsigned_zero(eval(&args[2], cx)?);
            match crate::hyper::pfq_any(&lists[0], &lists[1], &z, p) {
                Some(Some(v)) => finite(v, &|| e.to_string()),
                Some(None) => Err(NumErr::Undefined(e.to_string())),
                None => Err(NumErr::Unsupported(format!(
                    "{e}: the series does not converge here"
                ))),
            }
        }
        Expr::Fn(name, args)
            if &**name == "meijerg"
                && args.len() == 5
                && args[..4].iter().all(|l| l.as_list().is_some()) =>
        {
            let mut lists = Vec::new();
            for list in &args[..4] {
                let mut values = Vec::new();
                for x in list.as_list().unwrap_or_default() {
                    values.push(unsigned_zero(eval(x, cx)?));
                }
                lists.push(values);
            }
            let z = unsigned_zero(eval(&args[4], cx)?);
            let all_real = lists.iter().flatten().all(|x| x.imag().is_zero())
                && z.imag().is_zero()
                && *z.real() > 0u32;
            match crate::hyper::meijer_g(&lists[0], &lists[1], &lists[2], &lists[3], &z, p) {
                // (real parameters at a positive place give a real value unless the place is
                // beyond the unit circle of a function with a cut there)
                Some(Some(v))
                    if all_real
                        && v.imag().get_exp().is_none_or(|e| {
                            i64::from(e) < crate::special::expo(&v) - i64::from(p) + 16
                        }) =>
                {
                    finite(Complex::with_val(p, (v.real(), 0)), &|| e.to_string())
                }
                Some(Some(v)) => finite(v, &|| e.to_string()),
                Some(None) => Err(NumErr::Undefined(e.to_string())),
                None => Err(NumErr::Unsupported(format!(
                    "{e}: the series of Meijer's G function do not converge here"
                ))),
            }
        }
        // a product that is kept as it is written, and a sum in the order of its terms
        Expr::Fn(name, args) if &**name == "hold" && args.len() == 1 => eval(&args[0], cx),
        Expr::Fn(name, args) if &**name == "based" && args.len() == 2 => eval(&args[0], cx),
        Expr::Fn(name, args) if &**name == "ordered" => {
            let mut sum = Complex::new(p);
            for t in args.iter() {
                sum += eval(t, cx)?;
            }
            Ok(sum)
        }
        Expr::Fn(name, args) => {
            if let Some(r) = tetration::numeric(name, args, cx) {
                return r;
            }
            let mut vals = Vec::with_capacity(args.len());
            for a in args.iter() {
                vals.push(unsigned_zero(eval(a, cx)?));
            }
            let r = call(name, &vals, p)
                .ok_or_else(|| NumErr::Unsupported(format!("{name}/{}", vals.len())))?;
            match r {
                Some(v) => finite(v, &|| e.to_string()),
                None => Err(NumErr::Undefined(e.to_string())),
            }
        }
        _ => Err(NumErr::Unsupported(e.to_string())),
    }
}

/// Whether a condition holds for the values of the variables.
fn holds(cond: &Expr, cx: &mut Ctx) -> Result<bool, NumErr> {
    match cond {
        Expr::Bool(b) => Ok(*b),
        Expr::Rel(op, l, r) => {
            let (x, y) = (eval(l, cx)?, eval(r, cx)?);
            let d = Complex::with_val(cx.prec, &x - &y);
            let scale = magnitude(&x).max(magnitude(&y));
            let tiny = |v: &Float| {
                v.is_zero()
                    || v.get_exp().map_or(i64::MIN, i64::from) < scale - i64::from(cx.prec) + 16
            };
            let equal = tiny(d.real()) && tiny(d.imag());
            Ok(match op {
                Rel::Eq => equal,
                Rel::Ne => !equal,
                _ => {
                    if !tiny(d.imag()) {
                        return Err(NumErr::Unsupported(format!(
                            "{cond}: complex numbers have no order"
                        )));
                    }
                    let s = if equal {
                        0
                    } else {
                        d.real().cmp0().map_or(0, |o| o as i32)
                    };
                    match op {
                        Rel::Lt => s < 0,
                        Rel::Le => s <= 0,
                        Rel::Gt => s > 0,
                        _ => s >= 0,
                    }
                }
            })
        }
        Expr::Fn(name, a) => match (&**name, a.len()) {
            ("and", _) => {
                for x in a.iter() {
                    if !holds(x, cx)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            ("or", _) => {
                for x in a.iter() {
                    if holds(x, cx)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ("not", 1) => Ok(!holds(&a[0], cx)?),
            ("xor", _) => {
                let mut acc = false;
                for x in a.iter() {
                    acc ^= holds(x, cx)?;
                }
                Ok(acc)
            }
            ("implies", 2) => Ok(!holds(&a[0], cx)? || holds(&a[1], cx)?),
            _ => Err(NumErr::Unsupported(cond.to_string())),
        },
        _ => Err(NumErr::Unsupported(cond.to_string())),
    }
}

/// The value on a branch cut by the usual convention (continuous counter-clockwise around the
/// branch point), where the library takes the side of a positive zero.
fn on_cut(name: &str, z: &Complex, value: Complex) -> Complex {
    match name {
        "asin" | "acos" | "atanh" if z.imag().is_zero() && *z.real() > 1 => value.conj(),
        "asinh" | "atan" if z.real().is_zero() && *z.imag() < -1 => {
            let (re, im) = value.into_real_imag();
            Complex::with_val(re.prec(), (-re, im))
        }
        _ => value,
    }
}

/// `a^2 - b^2 = 1` to the last bit. There `1 - z^2` (z = a + b i) has no real part at all, and
/// the inverse sine of the library MPC (1.4.1) asks for more and more precision without an
/// end, until the memory of the machine is used up.
fn on_hyperbola(a: &Float, b: &Float) -> bool {
    let (Some(ea), Some(eb)) = (a.get_exp(), b.get_exp()) else {
        return false;
    };
    // (numbers that far apart, or that large, cannot differ by exactly 1 in their squares)
    let p = i64::from(a.prec().max(b.prec()));
    if i64::from(ea) < 1 || i64::from(ea) > p / 2 + 2 || i64::from(eb) < -p || eb > ea {
        return false;
    }
    match (a.to_rational(), b.to_rational()) {
        (Some(a), Some(b)) => {
            let difference =
                rug::Rational::from(a.square_ref()) - rug::Rational::from(b.square_ref());
            difference == 1
        }
        _ => false,
    }
}

/// The precision at which the logarithm formulas of the inverse functions keep `p` bits at `z`.
fn wide(z: &Complex, p: u32) -> u32 {
    let size = magnitude(z).clamp(0, 1 << 24) as u32;
    p + 2 * size + 64
}

/// The inverse sine. On the hyperbola where the library does not end: -i ln(i z + sqrt(1 - z^2)).
pub fn asin(z: &Complex) -> Complex {
    if !on_hyperbola(z.real(), z.imag()) {
        return z.clone().asin();
    }
    let p = z.prec().0.max(z.prec().1);
    let q = wide(z, p);
    let zz = Complex::with_val(q, z);
    let root = (Complex::with_val(q, 1) - zz.clone().square()).sqrt();
    Complex::with_val(p, (zz.mul_i(false) + root).ln().mul_i(true))
}

/// The inverse cosine: pi/2 - asin(z) where the library does not end.
pub fn acos(z: &Complex) -> Complex {
    if !on_hyperbola(z.real(), z.imag()) {
        return z.clone().acos();
    }
    let p = z.prec().0.max(z.prec().1);
    let q = wide(z, p);
    let half_pi = Float::with_val(q, Constant::Pi) / 2u32;
    Complex::with_val(p, half_pi - asin(&Complex::with_val(q, z)))
}

/// The inverse hyperbolic sine (the library takes it from the inverse sine of i z):
/// ln(z + sqrt(z^2 + 1)) where that does not end.
pub fn asinh(z: &Complex) -> Complex {
    if !on_hyperbola(z.imag(), z.real()) {
        return z.clone().asinh();
    }
    let p = z.prec().0.max(z.prec().1);
    let q = wide(z, p);
    let zz = Complex::with_val(q, z);
    let root = (zz.clone().square() + 1u32).sqrt();
    Complex::with_val(p, (zz + root).ln())
}

/// The inverse hyperbolic cosine (the library takes it from the inverse cosine):
/// ln(z + sqrt(z + 1) sqrt(z - 1)) where that does not end.
pub fn acosh(z: &Complex) -> Complex {
    if !on_hyperbola(z.real(), z.imag()) {
        return z.clone().acosh();
    }
    let p = z.prec().0.max(z.prec().1);
    let q = wide(z, p);
    let zz = Complex::with_val(q, z);
    let root = Complex::with_val(q, &zz + 1u32).sqrt() * Complex::with_val(q, &zz - 1u32).sqrt();
    Complex::with_val(p, (zz + root).ln())
}

/// How far apart the parts of a number may be, in bits, for the functions of the library:
/// 2^16 (a part 10^19000 times smaller than the other still has its digits), or twice the
/// bits of the number where that is more.
fn gap(prec: u32) -> i64 {
    (2 * i64::from(prec) + 64).max(1 << 16)
}

/// A number with one part smaller than the other by more than `gap`, with that part raised
/// to twice the bits of the number below the other (and on its side of 0). The library
/// works with as many bits as the parts of a number are apart, millions for
/// sin(1 + i/10^(10^8)); to the bits at hand the function has the same value here. `None`:
/// the number is none of these.
fn levelled(z: &Complex) -> Option<Complex> {
    let prec = z.prec().0;
    let (re, im) = (
        i64::from(z.real().get_exp()?),
        i64::from(z.imag().get_exp()?),
    );
    if (re - im).abs() <= gap(prec) {
        return None;
    }
    let below = 2 * i64::from(prec) + 64;
    let raised = |small: &Float, large: i64| {
        let size = Float::with_val(prec, 0.5) << i32::try_from(large - below).unwrap_or(i32::MIN);
        if small.is_sign_negative() {
            -size
        } else {
            size
        }
    };
    Some(match re > im {
        true => Complex::with_val(prec, (z.real(), raised(z.imag(), re))),
        false => Complex::with_val(prec, (raised(z.real(), im), z.imag())),
    })
}

/// The value of a function at a number that was `levelled`: a part of it that is smaller
/// than the other by more than the bits at hand comes from the part that was raised, and is
/// 0 as far as these bits tell.
fn settled_parts(v: Complex) -> Complex {
    let prec = v.prec().0;
    let (Some(re), Some(im)) = (v.real().get_exp(), v.imag().get_exp()) else {
        return v;
    };
    let apart = i64::from(re) - i64::from(im);
    if apart.abs() <= i64::from(prec) {
        return v;
    }
    let (re, im) = v.into_real_imag();
    match apart > 0 {
        true => Complex::with_val(prec, (re, 0)),
        false => Complex::with_val(prec, (0, im)),
    }
}

/// Outer `None`: unknown function; inner `None`: pole or undefined point.
fn call(name: &str, a: &[Complex], p: u32) -> Option<Option<Complex>> {
    let real = |x: Float| Complex::with_val(p, x);
    let z = a.first()?;
    if a.len() == 1
        && matches!(
            name,
            "sin"
                | "cos"
                | "tan"
                | "asin"
                | "acos"
                | "atan"
                | "sinh"
                | "cosh"
                | "tanh"
                | "asinh"
                | "acosh"
                | "atanh"
        )
        && let Some(near) = levelled(z)
    {
        return call(name, &[near], p).map(|v| v.map(settled_parts));
    }
    let one = match (name, a.len()) {
        ("sin", 1) => z.clone().sin(),
        ("cos", 1) => z.clone().cos(),
        ("tan", 1) => z.clone().tan(),
        ("asin", 1) => on_cut(name, z, asin(z)),
        ("acos", 1) => on_cut(name, z, acos(z)),
        ("atan", 1) => on_cut(name, z, z.clone().atan()),
        ("sinh", 1) => z.clone().sinh(),
        ("cosh", 1) => z.clone().cosh(),
        ("tanh", 1) => z.clone().tanh(),
        ("asinh", 1) => on_cut(name, z, asinh(z)),
        ("acosh", 1) => acosh(z),
        ("atanh", 1) => on_cut(name, z, z.clone().atanh()),
        ("ln", 1) => {
            if z.is_zero() {
                return Some(None);
            }
            cln(z.clone())
        }
        ("abs", 1) => z.clone().abs(),
        ("arg", 1) => z.clone().arg(),
        ("re", 1) => real(z.real().clone()),
        ("im", 1) => real(z.imag().clone()),
        ("conj", 1) => z.clone().conj(),
        ("sign", 1) => {
            if z.is_zero() {
                return Some(Some(Complex::new(p)));
            }
            let m = Float::with_val(p, z.abs_ref());
            z.clone() / m
        }
        ("floor", 1) => per_part(z, Float::floor),
        ("ceil", 1) => per_part(z, Float::ceil),
        ("round", 1) => per_part(z, round_away),
        ("trunc", 1) => per_part(z, Float::trunc),
        ("frac", 1) => per_part(z, Float::fract),
        ("atan2", 2) => {
            let (y, x) = (real_only(name, z).ok()?, real_only(name, &a[1]).ok()?);
            if y.is_zero() && x.is_zero() {
                return Some(None);
            }
            real(y.atan2(&x))
        }
        ("min", _) | ("max", _) => {
            let mut best = real_only(name, z).ok()?;
            for v in &a[1..] {
                let x = real_only(name, v).ok()?;
                if (name == "min") == (x < best) {
                    best = x;
                }
            }
            real(best)
        }
        ("mod", 2) => {
            let (x, m) = (real_only(name, z).ok()?, real_only(name, &a[1]).ok()?);
            if m.is_zero() {
                return Some(None);
            }
            let q = Float::with_val(p, &x / &m).floor();
            real(x - q * m)
        }
        _ => return special::call(name, a, p),
    };
    Some(Some(one))
}

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Stable,
    Noise,
    Unsettled,
}

fn judge(hi: &Float, lo: &Float, scale: i64, lo_prec: u32, need: u32) -> Part {
    if hi.is_zero() && lo.is_zero() {
        return Part::Stable;
    }
    if hi.is_zero() != lo.is_zero() {
        return Part::Noise;
    }
    // (no numbers: the same infinity twice has settled, anything else has not)
    if !hi.is_finite() || !lo.is_finite() {
        return match hi == lo {
            true => Part::Stable,
            false => Part::Unsettled,
        };
    }
    let diff = Float::with_val(hi.prec(), hi - lo);
    let d = diff.get_exp().map_or(i64::MIN, i64::from);
    let h = hi.get_exp().map_or(i64::MIN, i64::from);
    if d <= h.saturating_sub(i64::from(need)) {
        Part::Stable
    } else if h.max(lo.get_exp().map_or(i64::MIN, i64::from)) <= scale - i64::from(lo_prec) + 12 {
        Part::Noise
    } else {
        Part::Unsettled
    }
}

const ROUNDS: u32 = 5;

/// Value correct to `digits` significant digits in each part: the expression is evaluated at two
/// precisions, doubled until they agree; a part that stays rounding noise to the end is an exact zero.
pub fn evalf_digits_env(e: &Expr, digits: u32, env: Option<&Env>) -> Result<Complex, NumErr> {
    let need = bits_for(digits);
    if let Some(r) = tetration::direct(e, digits, env) {
        return r.map(|v| Complex::with_val(need + 32, v));
    }
    let mut p = need + 32;
    let rounds = if tetration::uses_tet(e) { 2 } else { ROUNDS };
    for round in 1..=rounds {
        let mut cx = Ctx {
            prec: p + 64,
            env,
            max_exp: i64::MIN,
        };
        let hi = eval(e, &mut cx)?;
        let lo = eval(
            e,
            &mut Ctx {
                prec: p,
                env,
                max_exp: i64::MIN,
            },
        )?;
        let parts = [
            judge(hi.real(), lo.real(), cx.max_exp, p, need + 2),
            judge(hi.imag(), lo.imag(), cx.max_exp, p, need + 2),
        ];
        let settled = parts.iter().all(|s| *s == Part::Stable);
        crate::trace::detail(|| {
            format!(
                "the value of {e} with {p} and with {} bits: the digits {}",
                p + 64,
                if settled {
                    "agree"
                } else {
                    "do not agree yet: more bits are taken"
                }
            )
        });
        if settled || (round == rounds && !parts.contains(&Part::Unsettled)) {
            let mut out = Complex::with_val(need + 32, &hi);
            if parts[0] == Part::Noise {
                out.mut_real().assign(Special::Zero);
            }
            if parts[1] == Part::Noise {
                out.mut_imag().assign(Special::Zero);
            }
            return Ok(out);
        }
        p = p.saturating_mul(2);
    }
    Err(NumErr::External(format!(
        "the value of {e} did not settle to {digits} digits"
    )))
}

/// Zero test by two precisions: rounding noise shrinks when the precision grows, a value does not.
fn zero_test(e: &Expr, env: Option<&Env>) -> Result<bool, NumErr> {
    if tetration::uses_tet(e) {
        // one run of tet per value: zero when nothing is left of the digits asked for
        let prec = bits_for(digits()) + 24;
        let mut cx = Ctx {
            prec,
            env,
            max_exp: i64::MIN,
        };
        let v = eval(e, &mut cx)?;
        return Ok(v.is_zero() || magnitude(&v) <= cx.max_exp - i64::from(bits_for(digits())) + 12);
    }
    let mut prec = (bits_for(digits()) * 2).max(320);
    for _ in 0..3 {
        let lo = eval(
            e,
            &mut Ctx {
                prec,
                env,
                max_exp: i64::MIN,
            },
        )?;
        if lo.is_zero() {
            return Ok(true);
        }
        let hi = eval(
            e,
            &mut Ctx {
                prec: prec * 2,
                env,
                max_exp: i64::MIN,
            },
        )?;
        if hi.is_zero() {
            return Ok(true);
        }
        let (ml, mh) = (magnitude(&lo), magnitude(&hi));
        if mh <= ml - i64::from(prec / 2) {
            return Ok(true);
        }
        let diff = Complex::with_val(prec * 2, &hi - &lo);
        if magnitude(&diff) <= mh - 24 {
            return Ok(false);
        }
        prec *= 2;
    }
    Ok(false)
}

pub fn evalf_digits(e: &Expr, digits: u32) -> Result<Complex, NumErr> {
    evalf_digits_env(e, digits, None)
}

/// The value of a function of approximate numbers with `prec` bits, as `evalf` gives it, when
/// the same value with more bits has the same `digits` digits. Where the two differ, the
/// bits were not enough for the way the function is calculated (a series whose terms cancel,
/// a recurrence that loses its digits): then the value that settles with more of them, or
/// none where none does.
pub fn evalf_checked(e: &Expr, prec: u32, digits: u32) -> Option<Complex> {
    let mut cx = Ctx {
        prec: prec + 64,
        env: None,
        max_exp: i64::MIN,
    };
    let lo = evalf(e, prec).ok()?;
    // (what is no number is given as it always was: there is nothing to compare)
    if !(lo.real().is_finite() && lo.imag().is_finite()) {
        return Some(lo);
    }
    let Ok(hi) = eval(e, &mut cx) else {
        return Some(lo);
    };
    let need = bits_for(digits).min(prec.saturating_sub(8));
    let parts = [
        judge(hi.real(), lo.real(), cx.max_exp, prec, need),
        judge(hi.imag(), lo.imag(), cx.max_exp, prec, need),
    ];
    match parts.contains(&Part::Unsettled) {
        false => Some(lo),
        true => evalf_digits(e, digits)
            .ok()
            .map(|v| Complex::with_val(prec, v)),
    }
}

/// Zero test of `e` at the point `env`.
pub fn is_zero_at(e: &Expr, env: &Env) -> Option<bool> {
    zero_test(e, Some(env)).ok()
}

/// log2 of |value| relative to the largest intermediate value at the point `env`.
pub fn relative_size(e: &Expr, prec: u32, env: &Env) -> Option<i64> {
    let mut cx = Ctx {
        prec,
        env: Some(env),
        max_exp: i64::MIN,
    };
    let v = eval(e, &mut cx).ok()?;
    if v.is_zero() {
        return Some(i64::MIN / 2);
    }
    Some(magnitude(&v) - cx.max_exp.max(0))
}

/// Sign of a real constant expression; `None` when it is not real or has no value.
pub fn sign(e: &Expr) -> Option<i32> {
    if let Some(r) = e.as_rat() {
        return Some(r.cmp0() as i32);
    }
    if !e.is_const() {
        return assume::sign(e);
    }
    let v = evalf_digits(e, 20).ok()?;
    if !v.imag().is_zero() {
        return None;
    }
    v.real().cmp0().map(|o| o as i32)
}

/// Whether a constant expression is zero (to the working precision).
pub fn is_zero_const(e: &Expr) -> Option<bool> {
    if let Some(n) = e.as_num() {
        return Some(n.is_zero());
    }
    if !e.is_const() {
        return None;
    }
    zero_test(e, None).ok()
}

/// Real value of a real constant expression.
pub fn real_value(e: &Expr, digits: u32) -> Option<Float> {
    if !e.is_const() {
        return None;
    }
    let v = evalf_digits(e, digits).ok()?;
    v.imag().is_zero().then(|| v.real().clone())
}

pub fn to_float_expr(v: Complex, digits: u32) -> Expr {
    Expr::float(Flt { v, digits })
}

unsafe extern "C" {
    fn mpfr_clear_underflow();
    fn mpfr_clear_overflow();
    fn mpfr_set_underflow();
    fn mpfr_set_overflow();
    fn mpfr_underflow_p() -> std::ffi::c_int;
    fn mpfr_overflow_p() -> std::ffi::c_int;
}

/// What `f` gives, and whether a number became smaller or larger on the way than the
/// exponents of the numbers that are kept allow (MPFR makes it 0 or infinite then, and says
/// so by its flags): 0.99 to the power 2^63 is no 0, and neither is 1 over the gamma function
/// of 10^10.
pub fn underflowed<T>(f: impl FnOnce() -> T) -> (T, bool) {
    // SAFETY: these functions only read and write the flags of MPFR, which belong to this
    // thread; they take no arguments and have no other effects.
    let before = unsafe { (mpfr_underflow_p() != 0, mpfr_overflow_p() != 0) };
    unsafe {
        mpfr_clear_underflow();
        mpfr_clear_overflow();
    }
    let out = f();
    let during = unsafe { mpfr_underflow_p() != 0 || mpfr_overflow_p() != 0 };
    // (what was raised before stays raised: one of these may run inside another)
    unsafe {
        if before.0 {
            mpfr_set_underflow();
        }
        if before.1 {
            mpfr_set_overflow();
        }
    }
    (out, during)
}

/// Numeric form: constants become approximate numbers, variables stay.
pub fn n(e: &Expr, digits: u32) -> Expr {
    match e {
        Expr::Num(x) if x.is_int() => e.clone(),
        Expr::Float(f) => {
            if f.digits <= digits {
                e.clone()
            } else {
                to_float_expr(Complex::with_val(bits_for(digits) + 32, &f.v), digits)
            }
        }
        Expr::Bool(_) => e.clone(),
        Expr::Fn(name, args) if matches!(&**name, "ordered" | "hold") => {
            let plain = match (&**name, &args[..]) {
                ("hold", [inner]) => crate::canon::map_bottom_up(inner, &mut |x| x),
                _ => crate::canon::add(args.to_vec()),
            };
            n(&plain, digits)
        }
        Expr::List(_) | Expr::Rel(..) => rebuild(e, &mut |c| n(c, digits)),
        _ if e.is_const() && !e.is_infinite() => {
            match underflowed(|| evalf_digits(e, digits)) {
                // (a 0 that is none: the number is too small for the exponents that are
                // kept, and it stays as it is written, as one that is too large does)
                (Ok(v), under) if !(under && v.is_zero()) => to_float_expr(v, digits),
                _ => rebuild(e, &mut |c| n(c, digits)),
            }
        }
        Expr::Pow(b, x) if x.as_rat().is_some() => pow(n(b, digits), (**x).clone()),
        Expr::Pow(b, x) if b.is_sym("e") => pow(Expr::e(), n(x, digits)),
        Expr::Add(terms) => {
            // the numbers of a sum are taken together: each one by itself has its digits, and
            // their sum would have none where they cancel (ln(101) + ... - 50 ln(10) is 10^-50)
            let (numbers, others): (Vec<Expr>, Vec<Expr>) = terms
                .iter()
                .cloned()
                .partition(|t| t.is_const() && !t.is_infinite());
            if numbers.len() < 2 {
                return rebuild(e, &mut |c| n(c, digits));
            }
            let together = crate::canon::add(numbers);
            let value = match evalf_digits(&together, digits) {
                Ok(v) => to_float_expr(v, digits),
                Err(_) => rebuild(&together, &mut |c| n(c, digits)),
            };
            let mut parts: Vec<Expr> = others.iter().map(|c| n(c, digits)).collect();
            parts.push(value);
            crate::canon::add(parts)
        }
        _ => rebuild(e, &mut |c| n(c, digits)),
    }
}

pub fn fmt_real(x: &Float, digits: u32) -> String {
    if x.is_zero() {
        return "0".into();
    }
    if x.is_nan() {
        return "undefined".into();
    }
    if x.is_infinite() {
        return if x.is_sign_negative() {
            "-inf".into()
        } else {
            "inf".into()
        };
    }
    let (neg, s, exp) = x.to_sign_string_exp(10, Some(digits.max(1) as usize));
    let point = i64::from(exp.unwrap_or(0));
    let s = s.trim_end_matches('0');
    let s = if s.is_empty() { "0" } else { s };
    let len = s.len() as i64;
    let body = if point > i64::from(digits) || point < -5 {
        let frac = if len > 1 {
            format!(".{}", &s[1..])
        } else {
            String::new()
        };
        format!("{}{}e{}", &s[..1], frac, point - 1)
    } else if point <= 0 {
        format!("0.{}{}", "0".repeat((-point) as usize), s)
    } else if point >= len {
        format!("{}{}", s, "0".repeat((point - len) as usize))
    } else {
        format!("{}.{}", &s[..point as usize], &s[point as usize..])
    };
    if neg { format!("-{body}") } else { body }
}

pub fn fmt_complex(z: &Complex, digits: u32) -> String {
    let (re, im) = (z.real(), z.imag());
    if im.is_zero() {
        return fmt_real(re, digits);
    }
    let mag = fmt_real(&im.clone().abs(), digits);
    let unit = if mag == "1" {
        "i".to_string()
    } else {
        format!("{mag}i")
    };
    let neg = im.is_sign_negative();
    if re.is_zero() {
        return if neg { format!("-{unit}") } else { unit };
    }
    format!(
        "{} {} {}",
        fmt_real(re, digits),
        if neg { '-' } else { '+' },
        unit
    )
}

/// Deterministic sample points for numeric identity checks.
pub fn sample_env(vars: &[Rc<str>], trial: u32, prec: u32) -> Env {
    let mut env = Env::new();
    for v in vars {
        let mut h = 0x9E37_79B9_7F4A_7C15u64.wrapping_mul(u64::from(trial) + 1);
        for b in v.bytes() {
            h = (h ^ u64::from(b)).wrapping_mul(0x100_0000_01B3);
            h ^= h >> 29;
        }
        let mut next = || {
            h ^= h << 13;
            h ^= h >> 7;
            h ^= h << 17;
            (h % 2_000_003) as u32
        };
        let unit = |n: u32| Float::with_val(prec, n) / 2_000_003u32;
        let value = if assume::is_integer_sym(v) {
            Complex::with_val(prec, Integer::from(next() % 7) - 3)
        } else if assume::is_positive_sym(v) {
            Complex::with_val(prec, unit(next()) * 2u32 + 0.25f64)
        } else if assume::is_real_sym(v) {
            Complex::with_val(prec, unit(next()) * 4u32 - 2u32)
        } else {
            Complex::with_val(
                prec,
                (unit(next()) * 4u32 - 2u32, unit(next()) * 4u32 - 2u32),
            )
        };
        env.insert(v.clone(), value);
    }
    env
}

/// The functions that are made of pieces: one of them may be zero on a whole stretch of
/// its values without being zero everywhere.
/// Functions that take one of several values (a logarithm, an inverse sine): an identity
/// with them is tried at numbers of many sizes.
const CUTS: &[&str] = &[
    "ln", "asin", "acos", "atan", "asinh", "acosh", "atanh", "lambertw",
];

const PIECES: &[&str] = &[
    "abs",
    "sign",
    "sgn",
    "re",
    "im",
    "real",
    "imag",
    "conj",
    "arg",
    "angle",
    "floor",
    "ceil",
    "round",
    "fix",
    "trunc",
    "frac",
    "mod",
    "rem",
    "max",
    "min",
    "heaviside",
    "piecewise",
    "dirac",
    "boxcar",
    "rectpuls",
    "tripuls",
    "sawtooth",
    "square",
    "gcd",
    "lcm",
    "hardlim",
    "hardlims",
    "unitstep",
    "kroneckerdelta",
];

/// Probabilistic test of `e == 0` as an identity in its variables.
pub fn is_zero_identity(e: &Expr) -> Option<bool> {
    if e.is_zero() {
        return Some(true);
    }
    let vars: Vec<Rc<str>> = e.free_syms().into_iter().collect();
    if vars.is_empty() {
        return is_zero_const(e);
    }
    let pieces = e.any(&|x| match x {
        Expr::Fn(n, _) => PIECES.contains(&&**n),
        Expr::Rel(..) | Expr::Bool(_) => true,
        _ => false,
    });
    // functions with several values, of which one is taken: what holds for the numbers next
    // to 0 need not hold for all of them (asin(sin(x)) is x between -pi/2 and pi/2 only,
    // ln(e^x) is x where the imaginary part of x is between -pi and pi)
    let cuts = !pieces
        && e.any(&|x| match x {
            Expr::Fn(n, _) => CUTS.contains(&&**n),
            Expr::Pow(base, power) => !base.is_const() && power.as_int().is_none(),
            _ => false,
        });
    if pieces || cuts {
        // values of many sizes, real ones of both signs among them: every one must agree
        const SIZES: [f64; 8] = [1.0, 0.001, 30.0, 0.25, 1000.0, 0.03, 4.0, 0.5];
        let (trials, enough) = if pieces { (48u32, 24) } else { (16, 8) };
        let mut hits = 0;
        for trial in 0..trials {
            let mut env = sample_env(&vars, trial, 512);
            for (name, value) in env.iter_mut() {
                if assume::is_integer_sym(name) {
                    continue;
                }
                *value *= SIZES[(trial % 8) as usize];
                match trial % 3 {
                    1 => *value = Complex::with_val(512, value.real()),
                    2 if !assume::is_positive_sym(name) => {
                        *value =
                            Complex::with_val(512, -Float::with_val(512, value.real().abs_ref()))
                    }
                    _ => {}
                }
            }
            match zero_test(e, Some(&env)) {
                Ok(true) => hits += 1,
                Ok(false) => return Some(false),
                Err(_) => continue,
            }
        }
        return match hits >= enough {
            true => Some(true),
            false => None,
        };
    }
    let mut hits = 0;
    for trial in 0..12 {
        let env = sample_env(&vars, trial, 512);
        match zero_test(e, Some(&env)) {
            Ok(true) => hits += 1,
            Ok(false) => return Some(false),
            Err(_) => continue,
        }
        if hits >= 3 {
            return Some(true);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quotient_is_the_one_of_the_library_where_the_parts_are_near() {
        for prec in [64u32, 96, 170, 300] {
            let a = Complex::with_val(prec, (1.25, -3.5));
            let b = Complex::with_val(prec, (0.013, 1e-5));
            assert_eq!(
                cdiv(&a, &b),
                Complex::with_val(prec, &a / &b),
                "{prec} bits"
            );
            let zero = Complex::new(prec);
            assert!(cdiv(&zero, &b).is_zero());
        }
    }

    #[test]
    fn a_quotient_of_parts_far_apart_takes_no_more_bits_than_it_has() {
        // 1/(2^100000 + i): the library would work with 100000 bits for each part
        let prec = 170;
        let huge = Float::with_val(prec, 1) << 100_000u32;
        let b = Complex::with_val(prec, (huge.clone(), 1));
        let q = cdiv(&Complex::with_val(prec, 1), &b);
        assert_eq!(*q.real(), Float::with_val(prec, huge.recip_ref()));
        assert_eq!(q.imag().get_exp(), Some(-199_999));
        assert!(q.imag().is_sign_negative());
    }

    #[test]
    fn the_exponential_function_ends_beyond_the_exponents_that_are_kept() {
        let prec = 128;
        let (small, flagged) = underflowed(|| cexp(Complex::with_val(prec, (-1e9, 1))));
        assert!(small.is_zero() && flagged);
        let (large, flagged) = underflowed(|| cexp(Complex::with_val(prec, (1e9, 0))));
        assert!(large.real().is_infinite() && flagged);
        // (the size that made the library go on for ever: next to the end of its exponents)
        let near_the_end = Float::with_val(prec, 0.5).ln() * Float::with_val(prec, i64::MAX);
        let (gone, flagged) = underflowed(|| cexp(Complex::with_val(prec, (near_the_end, 2))));
        assert!(gone.is_zero() && flagged);
        let (usual, flagged) = underflowed(|| cexp(Complex::with_val(prec, (1, 0))));
        assert!(!flagged && (usual.real().to_f64() - std::f64::consts::E).abs() < 1e-15);
    }

    #[test]
    fn a_tiny_part_of_an_argument_does_not_take_the_memory() {
        // sin(1 + i/2^(10^7)): the library works with as many bits as the parts are apart
        let prec = 170;
        let tiny = Float::with_val(prec, 1) >> 10_000_000u32;
        let z = Complex::with_val(prec, (1, tiny));
        let got = call("sin", &[z], prec).flatten().expect("a value");
        assert_eq!(*got.real(), Float::with_val(prec, 1).sin());
        assert!(got.imag().is_zero());
        // (a part that is small, not out of reach, keeps its digits)
        let small = Float::with_val(prec, 1) >> 1000u32;
        let z = Complex::with_val(prec, (1, small.clone()));
        let got = call("sin", &[z], prec).flatten().expect("a value");
        let wanted = Float::with_val(prec, 1).cos() * small;
        assert_eq!(got.imag().get_exp(), wanted.get_exp());
    }
}
