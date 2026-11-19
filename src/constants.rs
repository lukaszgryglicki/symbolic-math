//! Named real constants without a closed form, to any precision.
//!
//! The series and iterations are those of the `tet` program (claude-tetration, `src/constants.rs`),
//! so that a name means the same number in both programs.

use rug::float::Constant;
use rug::{Float, Integer};
use std::cell::RefCell;
use std::collections::HashMap;

type R = Result<Float, String>;

thread_local! {
    static CACHE: RefCell<HashMap<&'static str, Float>> = RefCell::new(HashMap::new());
}

type Compute = fn(u32) -> R;

const TABLE: &[(&str, Compute)] = &[
    ("eulergamma", |p| Ok(Float::with_val(p, Constant::Euler))),
    ("catalan", |p| Ok(Float::with_val(p, Constant::Catalan))),
    ("dottie", dottie),
    ("laplace", laplace),
    ("soldner", soldner),
    ("khinchin", khinchin),
    ("erdosborwein", erdos_borwein),
    ("cahen", cahen),
    ("recfib", reciprocal_fibonacci),
    ("niven", niven),
    ("mertens", mertens),
    ("twinprime", twin_prime),
];

/// The constant `name` rounded to `prec` bits; `None` when the name is not one of these constants.
pub fn value(name: &str, prec: u32) -> Option<R> {
    let (key, f) = TABLE.iter().find(|(n, _)| *n == name)?;
    if let Some(hit) = CACHE.with(|c| c.borrow().get(key).filter(|v| v.prec() >= prec).cloned()) {
        return Some(Ok(Float::with_val(prec, hit)));
    }
    Some(f(prec).map(|v| {
        let v = Float::with_val(prec, v);
        CACHE.with(|c| c.borrow_mut().insert(key, v.clone()));
        v
    }))
}

fn one(prec: u32) -> Float {
    Float::with_val(prec, 1)
}

fn tiny(prec: u32, bits: u32) -> Float {
    one(prec) >> bits
}

fn count_bits(count: u32) -> u32 {
    u32::BITS - count.leading_zeros()
}

fn newton(
    work: u32,
    start: Float,
    step: impl Fn(&Float) -> R,
    residual: impl Fn(&Float) -> R,
) -> R {
    let small = tiny(work, work - 8);
    let mut x = start;
    for _ in 0..256 {
        let change = step(&x)?;
        if !change.is_finite() {
            return Err("Newton's iteration left the real numbers".into());
        }
        let settled = change.clone().abs() <= Float::with_val(work, &small * x.clone().abs());
        x -= change;
        if settled {
            if residual(&x)?.abs() > tiny(work, work - 16) {
                return Err("Newton's iteration missed its root".into());
            }
            return Ok(x);
        }
    }
    Err("Newton's iteration did not settle".into())
}

fn dottie(prec: u32) -> R {
    let work = prec + 64;
    newton(
        work,
        Float::with_val(work, 0.75),
        |x| {
            let (sin, cos) = x.clone().sin_cos(Float::new(work));
            Ok(Float::with_val(work, x - cos) / (sin + 1u32))
        },
        |x| Ok(Float::with_val(work, x - x.clone().cos())),
    )
}

fn laplace(prec: u32) -> R {
    let work = prec + 64;
    let value = |x: &Float| {
        let s = (x.clone().square() + 1u32).sqrt();
        let defect = x.clone().ln() + &s - Float::with_val(work, &s + 1u32).ln();
        (defect, s)
    };
    newton(
        work,
        Float::with_val(work, 2) / 3u32,
        |x| {
            let (defect, s) = value(x);
            let slope = x.clone().recip() + Float::with_val(work, x / (s + 1u32));
            Ok(defect / slope)
        },
        |x| Ok(value(x).0),
    )
}

/// `Ei(x)` for `0 < x < 1` by `gamma + ln x + sum x^k/(k k!)`.
fn ei_small(x: &Float, work: u32) -> R {
    let small = tiny(work, work + 4);
    let mut sum = Float::with_val(work, Constant::Euler) + x.clone().ln();
    let mut power = one(work);
    let mut factorial = Integer::from(1);
    for k in 1u32.. {
        power *= x;
        factorial *= k;
        let term = Float::with_val(work, &power / (Float::with_val(work, &factorial) * k));
        sum += &term;
        if term < small {
            break;
        }
    }
    Ok(sum)
}

fn soldner(prec: u32) -> R {
    let work = prec + 64;
    let x = newton(
        work,
        Float::with_val(work, 0.375),
        |x| Ok(ei_small(x, work)? * x / x.clone().exp()),
        |x| ei_small(x, work),
    )?;
    Ok(x.exp())
}

fn erdos_borwein(prec: u32) -> R {
    let terms = prec + 72;
    let work = prec + 64 + count_bits(terms);
    let mut sum = Float::new(work);
    let mut power = Integer::from(1);
    for _ in 0..terms {
        power <<= 1u32;
        sum += Float::with_val(work, &Integer::from(&power - 1u32)).recip();
    }
    Ok(sum)
}

fn cahen(prec: u32) -> R {
    let work = prec + 64;
    let small = tiny(work, work + 4);
    let mut sum = Float::new(work);
    let mut s = Integer::from(2);
    let mut positive = true;
    loop {
        let term = Float::with_val(work, &Integer::from(&s - 1u32)).recip();
        if positive {
            sum += &term;
        } else {
            sum -= &term;
        }
        if term < small {
            return Ok(sum);
        }
        positive = !positive;
        s = s.clone() * &s - &s + 1u32;
    }
}

fn reciprocal_fibonacci(prec: u32) -> R {
    let terms = (prec + 80) * 2;
    let work = prec + 64 + count_bits(terms);
    let small = tiny(work, work + 8);
    let mut sum = Float::new(work);
    let (mut current, mut next) = (Integer::from(1), Integer::from(1));
    loop {
        let term = Float::with_val(work, &current).recip();
        sum += &term;
        if term < small {
            return Ok(sum);
        }
        let following = current + &next;
        current = std::mem::replace(&mut next, following);
    }
}

/// `zeta(n) - 1` with `bits` correct bits after its leading one.
fn zeta_excess(n: u32, bits: u32) -> Float {
    Float::with_val(bits + n + 16, Float::zeta_u(n)) - 1u32
}

fn khinchin(prec: u32) -> R {
    let terms = (prec + 72) / 2 + 2;
    let work = prec + 64 + count_bits(terms);
    let mut sum = Float::new(work);
    let mut alternating = Float::new(work);
    for n in 1..=terms {
        if n > 1 {
            alternating -= Float::with_val(work, 2 * n - 2).recip();
        }
        alternating += Float::with_val(work, 2 * n - 1).recip();
        sum += Float::with_val(work, zeta_excess(2 * n, work) * &alternating) / n;
    }
    Ok((sum / Float::with_val(work, Constant::Log2)).exp())
}

fn niven(prec: u32) -> R {
    let terms = prec + 72;
    let work = prec + 64 + count_bits(terms);
    let mut sum = one(work);
    for k in 2..=terms + 1 {
        let excess = zeta_excess(k, work);
        sum += Float::with_val(work, &excess / Float::with_val(work, &excess + 1u32));
    }
    Ok(sum)
}

fn moebius(mut n: u32) -> i32 {
    let (mut sign, mut p) = (1, 2);
    while p * p <= n {
        if n.is_multiple_of(p) {
            n /= p;
            if n.is_multiple_of(p) {
                return 0;
            }
            sign = -sign;
        }
        p += 1;
    }
    if n > 1 { -sign } else { sign }
}

fn mertens(prec: u32) -> R {
    let terms = prec + 72;
    let work = prec + 64 + count_bits(terms);
    let mut sum = Float::with_val(work, Constant::Euler);
    for n in 2..=terms + 1 {
        let sign = moebius(n);
        if sign != 0 {
            sum += Float::with_val(work, zeta_excess(n, work).ln_1p()) * sign / n;
        }
    }
    Ok(sum)
}

fn twin_prime(prec: u32) -> R {
    let terms = (prec + 72) * 12 / 7 + 2;
    let work = prec + 64 + count_bits(terms);
    let mut sum = Float::new(work);
    for n in 2..=terms {
        let mut weight = Integer::new();
        for d in (1..=n).filter(|d| n.is_multiple_of(*d)) {
            weight += (Integer::from(1) << (n / d)) * moebius(d);
        }
        let weight = weight.div_exact(&Integer::from(n));
        let zeta = zeta_excess(n, work + n);
        let p = zeta.prec();
        let factor = Float::with_val(p, &zeta - (Float::with_val(p, &zeta + 1u32) >> n));
        sum += Float::with_val(work, factor.ln_1p()) * weight;
    }
    Ok((-sum).exp())
}
