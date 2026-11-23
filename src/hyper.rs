//! Hypergeometric series, summed with the precision raised until the sum is accurate, and the
//! functions built on them: 0F1, 1F1, U, 2F1, pFq, incomplete gamma and beta functions.

use crate::special::{Out, expo, gamma_any, rgamma, size};
use rug::{Complex, Float};

const MAX_TERMS: u32 = 3_000_000;

/// The most terms of a series; `terms = N` sets another count.
fn max_terms() -> u32 {
    u32::try_from(crate::settings::terms(u64::from(MAX_TERMS))).unwrap_or(u32::MAX)
}

/// The whole number `z` is, if it is one.
pub(crate) fn is_int(z: &Complex) -> Option<i64> {
    if !z.imag().is_zero() || !z.real().is_integer() {
        return None;
    }
    z.real().to_integer()?.to_i64()
}

pub(crate) fn nonpositive_int(z: &Complex) -> bool {
    is_int(z).is_some_and(|n| n <= 0)
}

/// Runs `f` with more and more bits until its value keeps `p` of them; `f` returns the value
/// and the number of bits lost to cancellation.
pub(crate) fn adaptive(p: u32, f: &mut dyn FnMut(u32) -> Option<Option<(Complex, i64)>>) -> Out {
    let mut wp = p + 40;
    for round in 0..7 {
        let Some((v, lost)) = f(wp)? else {
            return Some(None);
        };
        let enough = lost + 24 <= i64::from(wp - p);
        // a value that is all cancellation is zero as far as the terms can tell
        let hopeless = v.is_zero() || lost > 2 * i64::from(p) + 2_000_000;
        if enough || round == 6 || (hopeless && round > 0) {
            return Some(Some(Complex::with_val(p, v)));
        }
        let more = if v.is_zero() { 2 * i64::from(wp) } else { lost };
        wp = p + 64 + more.clamp(0, 4_000_000) as u32;
    }
    None
}

/// One pass over `sum_k prod (a_i)_k / prod (b_j)_k z^k/k!` with `wp` bits: the sum and the bits
/// lost to cancellation. Inner `None`: a pole (a lower parameter is a whole number that is not
/// positive); outer `None`: the series does not come to an end.
fn series_once(
    a: &[Complex],
    b: &[Complex],
    z: &Complex,
    wp: u32,
) -> Option<Option<(Complex, i64)>> {
    let zz = Complex::with_val(wp, z);
    let big = a.iter().chain(b).map(size).fold(0.0f64, f64::max);
    let mut term = Complex::with_val(wp, 1);
    let mut sum = term.clone();
    let mut top = 1i64;
    let mut small = 0;
    for k in 0..max_terms() {
        let mut num = zz.clone();
        for x in a {
            num *= Complex::with_val(wp, x + k);
        }
        if num.is_zero() {
            return Some(Some((sum.clone(), top - expo(&sum))));
        }
        let mut den = Complex::with_val(wp, k + 1);
        for y in b {
            let f = Complex::with_val(wp, y + k);
            if f.is_zero() {
                return Some(None);
            }
            den *= f;
        }
        let ratio = num / den;
        term *= &ratio;
        sum += &term;
        let e = expo(&term);
        top = top.max(e);
        if f64::from(k) > big && e < expo(&sum) - i64::from(wp) - 10 && size(&ratio) < 0.98 {
            small += 1;
            if small >= 3 {
                return Some(Some((sum.clone(), top - expo(&sum))));
            }
        } else {
            small = 0;
        }
    }
    None
}

/// The generalized hypergeometric series where it converges: any `z` for at most as many upper
/// as lower parameters, `|z| < 1` for one upper parameter more.
pub(crate) fn pfq(a: &[Complex], b: &[Complex], z: &Complex, p: u32) -> Out {
    if z.is_zero() {
        return Some(Some(Complex::with_val(p, 1)));
    }
    let terminating = a.iter().any(nonpositive_int);
    if !terminating {
        if a.len() > b.len() + 1 {
            return None;
        }
        if a.len() == b.len() + 1 && size(z) > 0.97 {
            return match size(z) > 1.03 {
                true => pfq_far(a, b, z, p),
                false => None,
            };
        }
    }
    adaptive(p, &mut |wp| series_once(a, b, z, wp))
}

/// One upper parameter more than lower ones, beyond the unit circle: the function is
/// `prod gamma(b) / prod gamma(a) * G(-z | 1 - a; 0, 1 - b)`, a sum of series in `1/z`. On the
/// cut from 1 on, the value is the one that `2F1` takes there.
fn pfq_far(a: &[Complex], b: &[Complex], z: &Complex, p: u32) -> Out {
    let wp = p + 32;
    let from_one = |x: &Complex| Complex::with_val(wp, 1u32 - Complex::with_val(wp, x));
    let upper: Vec<Complex> = a.iter().map(from_one).collect();
    let lower: Vec<Complex> = b.iter().map(from_one).collect();
    let mut minus = Complex::with_val(wp, -z);
    if z.imag().is_zero() {
        minus = Complex::with_val(wp, (minus.real(), 0));
    }
    let Some(g) = meijer_g(&upper, &[], &[Complex::new(wp)], &lower, &minus, wp)? else {
        return Some(None);
    };
    let mut scale = Complex::with_val(wp, 1);
    for y in b {
        let Some(top) = gamma_any(y, wp)? else {
            return Some(None);
        };
        scale *= top;
    }
    for x in a {
        scale *= rgamma(x, wp)?;
    }
    Some(Some(Complex::with_val(p, g * scale)))
}

/// Meijer's G function `G^{m,n}_{p,q}(z | a_1..a_n; a_n+1..a_p | b_1..b_m; b_m+1..b_q)` by
/// Slater's sums of hypergeometric series: over the first b's where the series in z converge
/// (p < q, or p = q inside the unit circle), over the first a's else. Parameters that lie a
/// whole number apart make the single terms infinite; they are nudged a little apart and the
/// precision is raised by what the terms then cancel.
pub(crate) fn meijer_g(
    an: &[Complex],
    ap: &[Complex],
    bm: &[Complex],
    bq: &[Complex],
    z: &Complex,
    p: u32,
) -> Out {
    let (n, m) = (an.len(), bm.len());
    let (count_a, count_b) = (n + ap.len(), m + bq.len());
    if z.is_zero() || !z.real().is_finite() || !z.imag().is_finite() {
        return None;
    }
    let first = match count_a.cmp(&count_b) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => {
            if (size(z) - 1.0).abs() < 0.03 {
                return None;
            }
            size(z) < 1.0
        }
    };
    let all_a: Vec<&Complex> = an.iter().chain(ap).collect();
    let all_b: Vec<&Complex> = bm.iter().chain(bq).collect();
    let (summed, group) = if first { (m, &all_b) } else { (n, &all_a) };
    let whole_apart = |x: &Complex, y: &Complex| {
        let d = Complex::with_val(p + 64, x - y);
        let nearest = Float::with_val(p + 64, d.real().round_ref());
        let off = Complex::with_val(p + 64, d - nearest);
        expo(&off) < -i64::from(p / 2)
    };
    let nudge =
        (0..summed).any(|h| (0..group.len()).any(|j| j != h && whole_apart(group[j], group[h])));
    let step = Float::with_val(p + 64, 1) >> (p + 16);
    let value = adaptive(p, &mut |wp| {
        let at = |x: &Complex, k: usize, moved: bool| -> Complex {
            match moved {
                true => Complex::with_val(wp, x + Float::with_val(wp, &step * (k as u32 + 1))),
                false => Complex::with_val(wp, x),
            }
        };
        let a: Vec<Complex> = all_a
            .iter()
            .enumerate()
            .map(|(k, x)| at(x, k, nudge && !first))
            .collect();
        let b: Vec<Complex> = all_b
            .iter()
            .enumerate()
            .map(|(k, x)| at(x, k, nudge && first))
            .collect();
        // the sum runs over `own`, the other parameters are `other`; with the a's summed
        // every parameter is mirrored: a -> 1 - a and b -> 1 - b turn the one sum into the other
        let (own, other, own_first, other_first) = match first {
            true => (&b, &a, m, n),
            false => (&a, &b, n, m),
        };
        let signed = |x: Complex| if first { x } else { -x };
        let mut sum = Complex::new(wp);
        let mut top = -(1i64 << 40);
        for h in 0..own_first {
            let mut term = Complex::with_val(wp, 1);
            for (j, x) in own.iter().enumerate().filter(|(j, _)| *j != h) {
                // gamma(b_j - b_h) above for the first ones, 1/gamma(1 + b_h - b_j) for the rest
                let d = signed(Complex::with_val(wp, x - &own[h]));
                if j < own_first {
                    let Some(g) = gamma_any(&d, wp)? else {
                        return Some(None);
                    };
                    term *= g;
                } else {
                    term *= rgamma(&Complex::with_val(wp, 1u32 - d), wp)?;
                }
            }
            for (j, x) in other.iter().enumerate() {
                // gamma(1 + b_h - a_j) above for the first ones, 1/gamma(a_j - b_h) for the rest
                let d = signed(Complex::with_val(wp, x - &own[h]));
                if j < other_first {
                    let Some(g) = gamma_any(&Complex::with_val(wp, 1u32 - d), wp)? else {
                        return Some(None);
                    };
                    term *= g;
                } else {
                    term *= rgamma(&d, wp)?;
                }
            }
            let upper: Vec<Complex> = other
                .iter()
                .map(|x| Complex::with_val(wp, 1u32 - signed(Complex::with_val(wp, x - &own[h]))))
                .collect();
            let lower: Vec<Complex> = own
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != h)
                .map(|(_, x)| {
                    Complex::with_val(wp, 1u32 - signed(Complex::with_val(wp, x - &own[h])))
                })
                .collect();
            let flips = (other.len() + own_first + other_first) % 2 == 1;
            let mut arg = match first {
                true => Complex::with_val(wp, z),
                false => Complex::with_val(wp, z.recip_ref()),
            };
            if flips {
                arg = -arg;
            }
            let Some(series) = pfq(&upper, &lower, &arg, wp)? else {
                return Some(None);
            };
            let power = match first {
                true => own[h].clone(),
                false => Complex::with_val(wp, &own[h] - 1u32),
            };
            let log = Complex::with_val(wp, z.ln_ref());
            term *= Complex::with_val(wp, (log * power).exp_ref());
            term *= series;
            top = top.max(expo(&term));
            sum += term;
        }
        // (terms that cancel to nothing at all have lost every bit there was)
        let lost = match (sum.is_zero(), top > -(1i64 << 39)) {
            (true, true) => i64::from(wp),
            (true, false) => 0,
            (false, _) => top - expo(&sum),
        };
        Some(Some((sum, lost)))
    })?;
    match value {
        // (parameters that had to be nudged and still give something huge: a pole)
        Some(v) if nudge && expo(&v) > i64::from(p / 2) => Some(None),
        other => Some(other),
    }
}

pub(crate) fn hyp0f1(b: &Complex, z: &Complex, p: u32) -> Out {
    pfq(&[], std::slice::from_ref(b), z, p)
}

/// Kummer's function M(a, b, z).
pub(crate) fn hyp1f1(a: &Complex, b: &Complex, z: &Complex, p: u32) -> Out {
    if nonpositive_int(b) && !is_int(a).is_some_and(|n| n <= 0 && n > is_int(b).unwrap_or(0)) {
        return Some(None);
    }
    if z.real().is_sign_negative() && size(z) > 8.0 && !nonpositive_int(a) {
        // e^z M(b - a, b, -z): the terms of this series do not cancel
        let wp = p + 16;
        let ba = Complex::with_val(wp, b - a);
        let minus = Complex::with_val(wp, -z);
        let m = pfq(&[ba], std::slice::from_ref(b), &minus, wp)?;
        return Some(m.map(|m| Complex::with_val(p, m * Complex::with_val(wp, z.exp_ref()))));
    }
    pfq(std::slice::from_ref(a), std::slice::from_ref(b), z, p)
}

/// One Taylor step of the solution of the hypergeometric equation from `z0` by `h`: the value
/// and the derivative at `z0 + h`.
fn taylor_step(
    abc: (&Complex, &Complex, &Complex),
    z0: &Complex,
    h: &Complex,
    start: (Complex, Complex),
    wp: u32,
) -> Option<(Complex, Complex)> {
    let (a, b, c) = abc;
    let one = Complex::with_val(wp, 1);
    let q = Complex::with_val(wp, &one - z0) * z0;
    let c1 = Complex::with_val(wp, &one - Complex::with_val(wp, z0 * 2u32));
    let ab1 = Complex::with_val(wp, a + b) + 1u32;
    let c2 = Complex::with_val(wp, c - Complex::with_val(wp, &ab1 * z0));
    let (mut u0, mut u1) = start;
    let mut hn = Complex::with_val(wp, h);
    let mut sum = Complex::with_val(wp, &u0 + Complex::with_val(wp, &u1 * h));
    let mut dsum = u1.clone();
    let mut small = 0;
    for n in 0..max_terms() {
        let lower = Complex::with_val(wp, a + n) * Complex::with_val(wp, b + n) * &u0;
        let upper = (Complex::with_val(wp, &c1 * n) + &c2) * &u1 * (n + 1);
        let next = (lower - upper) / (Complex::with_val(wp, &q * (n + 2)) * (n + 1));
        // the term u_{n+2} h^(n+2) and its derivative (n + 2) u_{n+2} h^(n+1)
        let dterm = Complex::with_val(wp, &next * &hn) * (n + 2);
        hn *= h;
        let term = Complex::with_val(wp, &next * &hn);
        sum += &term;
        dsum += &dterm;
        let tiny = (term.is_zero() || expo(&term) < expo(&sum) - i64::from(wp) - 8)
            && (dterm.is_zero() || expo(&dterm) < expo(&dsum) - i64::from(wp) - 8);
        if tiny && n > 8 {
            small += 1;
            if small >= 4 {
                return Some((sum, dsum));
            }
        } else {
            small = 0;
        }
        u0 = u1;
        u1 = next;
    }
    None
}

/// 2F1 away from the unit disk: from a point inside it along a straight line to `z`, in Taylor
/// steps of half the distance to the singular points 0 and 1. A point on the cut from 1 to
/// infinity is reached from below.
fn hyp2f1_far(a: &Complex, b: &Complex, c: &Complex, z: &Complex, wp: u32) -> Out {
    let below = z.imag().is_sign_negative() && !z.imag().is_zero()
        || (z.imag().is_zero() && *z.real() > 1u32);
    let root3 = Float::with_val(wp, 3).sqrt() / 4u32;
    let mut z0 = Complex::with_val(
        wp,
        (
            Float::with_val(wp, 0.25),
            if below { -root3 } else { root3 },
        ),
    );
    let Some(mut w) = pfq(&[a.clone(), b.clone()], std::slice::from_ref(c), &z0, wp)? else {
        return Some(None);
    };
    let (a1, b1, c1) = (
        Complex::with_val(wp, a + 1u32),
        Complex::with_val(wp, b + 1u32),
        Complex::with_val(wp, c + 1u32),
    );
    let Some(d) = pfq(&[a1, b1], &[c1], &z0, wp)? else {
        return Some(None);
    };
    let mut dw = d * Complex::with_val(wp, a * b) / c;
    for _ in 0..100_000 {
        let rest = Complex::with_val(wp, z - &z0);
        let near_one = Complex::with_val(wp, 1u32 - Complex::with_val(wp, &z0));
        let room =
            Float::with_val(64, z0.abs_ref()).min(&Float::with_val(64, near_one.abs_ref())) / 2u32;
        let dist = Float::with_val(64, rest.abs_ref());
        let last = dist <= room;
        let h = if last {
            rest
        } else {
            rest * Float::with_val(wp, &room / &dist)
        };
        (w, dw) = taylor_step((a, b, c), &z0, &h, (w, dw), wp)?;
        if last {
            return Some(Some(w));
        }
        z0 += h;
    }
    None
}

/// Gauss's function far left on the real axis, by its two series in 1/z:
/// gamma(c) gamma(b - a)/(gamma(b) gamma(c - a)) (-z)^(-a) 2F1(a, a - c + 1; a - b + 1; 1/z) and
/// the same with a and b the other way round. `None`: not this way.
fn hyp2f1_inverse(a: &Complex, b: &Complex, c: &Complex, z: &Complex, p: u32) -> Option<Complex> {
    // (a - b a whole number: both parts have poles that cancel each other; b moved a hair
    // stands for the limit, with twice the bits)
    let whole = is_int(&Complex::with_val(p + 32, a - b)).is_some();
    let wp = if whole { 2 * p + 96 } else { p + 64 };
    let (a, c) = (Complex::with_val(wp, a), Complex::with_val(wp, c));
    let b = match whole {
        true => Complex::with_val(wp, b) + (Float::with_val(wp, 1) >> (p + 24)),
        false => Complex::with_val(wp, b),
    };
    let inv = Complex::with_val(wp, z.recip_ref());
    let log = Complex::with_val(wp, Float::with_val(wp, -z.real())).ln();
    let gc = gamma_any(&c, wp)??;
    let part = |a: &Complex, b: &Complex| -> Option<Complex> {
        let low = rgamma(b, wp)? * rgamma(&Complex::with_val(wp, &c - a), wp)?;
        if low.is_zero() {
            return Some(Complex::new(wp));
        }
        let top = gamma_any(&Complex::with_val(wp, b - a), wp)??;
        let series = pfq(
            &[a.clone(), Complex::with_val(wp, a - &c) + 1u32],
            &[Complex::with_val(wp, a - b) + 1u32],
            &inv,
            wp,
        )??;
        let power = (Complex::with_val(wp, -a) * &log).exp();
        Some(Complex::with_val(wp, &gc * top) * low * power * series)
    };
    let (first, second) = (part(&a, &b)?, part(&b, &a)?);
    let sum = Complex::with_val(wp, &first + &second);
    // (parts that cancel each other further than the spare bits reach: not this way)
    let lost = expo(&first).max(expo(&second)) - expo(&sum);
    if sum.is_zero() || lost + 24 > i64::from(wp - p) {
        return None;
    }
    Some(Complex::with_val(p, sum))
}

/// Gauss's hypergeometric function, on its principal branch (cut from 1 to infinity).
pub(crate) fn hyp2f1(a: &Complex, b: &Complex, c: &Complex, z: &Complex, p: u32) -> Out {
    if z.is_zero() {
        return Some(Some(Complex::with_val(p, 1)));
    }
    let ends = |x: &Complex| is_int(x).filter(|n| *n <= 0);
    let last = match (ends(a), ends(b)) {
        (Some(m), Some(n)) => Some(m.max(n)),
        (m, n) => m.or(n),
    };
    if let Some(n) = is_int(c).filter(|n| *n <= 0) {
        // a pole, unless the series ends before the zero of the lower parameter
        if !last.is_some_and(|m| m > n) {
            return Some(None);
        }
    }
    if last.is_some() {
        return pfq(&[a.clone(), b.clone()], std::slice::from_ref(c), z, p);
    }
    let wp = p + 32;
    let one_z = Complex::with_val(wp, 1u32 - Complex::with_val(wp, z));
    if one_z.is_zero() {
        // Gauss: gamma(c) gamma(c - a - b) / (gamma(c - a) gamma(c - b))
        let s = Complex::with_val(wp, c - a) - b;
        if !s.real().is_sign_positive() || s.real().is_zero() {
            return Some(None);
        }
        let (Some(gc), Some(gs)) = (gamma_any(c, wp)?, gamma_any(&s, wp)?) else {
            return Some(None);
        };
        let low =
            rgamma(&Complex::with_val(wp, c - a), wp)? * rgamma(&Complex::with_val(wp, c - b), wp)?;
        return Some(Some(Complex::with_val(p, gc * gs * low)));
    }
    if size(z) <= 0.75 {
        return pfq(&[a.clone(), b.clone()], std::slice::from_ref(c), z, p);
    }
    let w = Complex::with_val(wp, -z) / &one_z;
    if size(&w) <= 0.75 {
        // (1 - z)^(-a) 2F1(a, c - b; c; z/(z - 1))
        let cb = Complex::with_val(wp, c - b);
        let Some(f) = pfq(&[a.clone(), cb], std::slice::from_ref(c), &w, wp)? else {
            return Some(None);
        };
        let factor = (Complex::with_val(wp, -a) * crate::numeric::unsigned_zero(one_z).ln()).exp();
        return Some(Some(Complex::with_val(p, f * factor)));
    }
    if z.imag().is_zero()
        && *z.real() < -1.35
        && let Some(v) = hyp2f1_inverse(a, b, c, z, p)
    {
        return Some(Some(v));
    }
    let far = size(z).max(2.0).log2().min(1e6) as u32;
    let near = (-size(&one_z).min(1.0).log2()).clamp(0.0, 1e6) as u32;
    let wp = p + 64 + 8 * (far + near);
    hyp2f1_far(a, b, c, z, wp).map(|v| v.map(|v| Complex::with_val(p, v)))
}

/// `z^(-a) sum (a)_k (a - b + 1)_k / k! (-1/z)^k`, when the terms get small enough before they grow.
fn u_asymptotic(a: &Complex, b: &Complex, z: &Complex, wp: u32) -> Option<Complex> {
    let ab = Complex::with_val(wp, a - b) + 1u32;
    let step = -Complex::with_val(wp, z.recip_ref());
    let mut term = Complex::with_val(wp, 1);
    let mut sum = term.clone();
    let mut last = i64::MAX;
    let mut done = false;
    for k in 0..max_terms() {
        let f = Complex::with_val(wp, a + k) * Complex::with_val(wp, &ab + k);
        if f.is_zero() {
            done = true;
            break;
        }
        term = term * f * &step / (k + 1);
        let e = expo(&term);
        if e > last && f64::from(k) > size(a) + size(&ab) {
            break;
        }
        sum += &term;
        if e < expo(&sum) - i64::from(wp) - 4 {
            done = true;
            break;
        }
        last = e;
    }
    if !done {
        return None;
    }
    let power = (Complex::with_val(wp, -a)
        * crate::numeric::unsigned_zero(Complex::with_val(wp, z)).ln())
    .exp();
    Some(sum * power)
}

/// Tricomi's function U(a, b, z), on its principal branch (cut along the negative real axis).
pub(crate) fn hyperu(a: &Complex, b: &Complex, z: &Complex, p: u32) -> Out {
    if z.is_zero() {
        // gamma(1 - b) / gamma(a - b + 1) for re b < 1
        if *b.real() >= 1u32 {
            return Some(None);
        }
        let wp = p + 32;
        let Some(g) = gamma_any(&Complex::with_val(wp, 1u32 - Complex::with_val(wp, b)), wp)?
        else {
            return Some(None);
        };
        let r = rgamma(&(Complex::with_val(wp, a - b) + 1u32), wp)?;
        return Some(Some(Complex::with_val(p, g * r)));
    }
    adaptive(p, &mut |wp| {
        if let Some(v) = u_asymptotic(a, b, z, wp + 8) {
            return Some(Some((v, 0)));
        }
        // gamma(1 - b)/gamma(a - b + 1) M(a, b, z) + gamma(b - 1)/gamma(a) z^(1 - b) M(a - b + 1, 2 - b, z)
        let whole = is_int(b).is_some();
        let (w2, bb) = if whole {
            // the limit of the formula: b a hair off the whole number, with twice the bits
            let w2 = 2 * wp + 64;
            let hair = Float::with_val(w2, 1) >> (wp + 8);
            (w2, Complex::with_val(w2, b) + hair)
        } else {
            (wp + 16, Complex::with_val(wp + 16, b))
        };
        let one = Complex::with_val(w2, 1);
        let (aa, zz) = (Complex::with_val(w2, a), Complex::with_val(w2, z));
        let ab1 = Complex::with_val(w2, &aa - &bb) + 1u32;
        let Some(g1) = gamma_any(&Complex::with_val(w2, &one - &bb), w2)? else {
            return Some(None);
        };
        let Some(g2) = gamma_any(&Complex::with_val(w2, &bb - &one), w2)? else {
            return Some(None);
        };
        let Some(m1) = hyp1f1(&aa, &bb, &zz, w2)? else {
            return Some(None);
        };
        let two_b = Complex::with_val(w2, 2u32 - Complex::with_val(w2, &bb));
        let Some(m2) = hyp1f1(&ab1, &two_b, &zz, w2)? else {
            return Some(None);
        };
        let power = (Complex::with_val(w2, &one - &bb)
            * crate::numeric::unsigned_zero(zz.clone()).ln())
        .exp();
        let t1 = g1 * rgamma(&ab1, w2)? * m1;
        let t2 = g2 * rgamma(&aa, w2)? * power * m2;
        let sum = Complex::with_val(w2, &t1 + &t2);
        let mut lost = expo(&t1).max(expo(&t2)) - expo(&sum);
        if whole {
            lost -= i64::from(wp);
        }
        Some(Some((sum, lost.max(0))))
    })
}

/// The upper incomplete gamma function: the integral of `t^(a-1) e^(-t)` from `z` to infinity.
pub(crate) fn upper_gamma(a: &Complex, z: &Complex, p: u32) -> Out {
    if z.is_zero() {
        return if a.real().is_sign_positive() && !a.real().is_zero() {
            gamma_any(a, p)
        } else {
            Some(None)
        };
    }
    let wp = p + 24;
    let one_a = Complex::with_val(wp, 1u32 - Complex::with_val(wp, a));
    let Some(u) = hyperu(&one_a, &one_a, z, wp)? else {
        return Some(None);
    };
    Some(Some(Complex::with_val(
        p,
        u * Complex::with_val(wp, -z).exp(),
    )))
}

/// The lower incomplete gamma function: the integral from 0 to `z`.
pub(crate) fn lower_gamma(a: &Complex, z: &Complex, p: u32) -> Out {
    if nonpositive_int(a) {
        return Some(None);
    }
    if z.is_zero() {
        return if a.real().is_sign_positive() && !a.real().is_zero() {
            Some(Some(Complex::new(p)))
        } else {
            Some(None)
        };
    }
    // z^a/a 1F1(a; a + 1; -z)
    let wp = p + 24;
    let a1 = Complex::with_val(wp, a + 1u32);
    let Some(m) = hyp1f1(a, &a1, &Complex::with_val(wp, -z), wp)? else {
        return Some(None);
    };
    let power = (Complex::with_val(wp, a)
        * crate::numeric::unsigned_zero(Complex::with_val(wp, z)).ln())
    .exp();
    Some(Some(Complex::with_val(p, m * power / a)))
}

/// The incomplete beta function: the integral of `t^(a-1) (1-t)^(b-1)` from 0 to `z`.
pub(crate) fn beta_inc(z: &Complex, a: &Complex, b: &Complex, p: u32) -> Out {
    if nonpositive_int(a) {
        return Some(None);
    }
    let wp = p + 24;
    if z.is_zero() {
        return if a.real().is_sign_positive() && !a.real().is_zero() {
            Some(Some(Complex::new(p)))
        } else {
            Some(None)
        };
    }
    // z^a/a 2F1(a, 1 - b; a + 1; z)
    let one_b = Complex::with_val(wp, 1u32 - Complex::with_val(wp, b));
    let a1 = Complex::with_val(wp, a + 1u32);
    let Some(f) = hyp2f1(a, &one_b, &a1, z, wp)? else {
        return Some(None);
    };
    let power = (Complex::with_val(wp, a)
        * crate::numeric::unsigned_zero(Complex::with_val(wp, z)).ln())
    .exp();
    Some(Some(Complex::with_val(p, f * power / a)))
}

/// The share of the whole beta function that the integral up to `x` has, for 0 < x < 1 and
/// large positive `a`, `b`: by the continued fraction, which loses nothing where the series
/// cancels itself away. `None` when it does not settle.
fn beta_share(x: &Float, a: &Float, b: &Float, p: u32) -> Option<Float> {
    let size = Float::with_val(64, a + b).to_f64().max(4.0).log2().ceil() as u32;
    let wp = p + 64 + 2 * size;
    let (x, a, b) = (
        Float::with_val(wp, x),
        Float::with_val(wp, a),
        Float::with_val(wp, b),
    );
    // (the fraction settles fast below the middle (a + 1)/(a + b + 2); above it the mirrored one does)
    let whole = Float::with_val(wp, &a + &b);
    let mirrored = Float::with_val(wp, &x * Float::with_val(wp, &whole + 2u32))
        > Float::with_val(wp, &a + 1u32);
    let (x, a, b) = match mirrored {
        true => (Float::with_val(wp, 1u32 - &x), b, a),
        false => (x, a, b),
    };
    let log_front = Float::with_val(wp, &a * Float::with_val(wp, x.ln_ref()))
        + Float::with_val(wp, &b * Float::with_val(wp, (-x.clone()).ln_1p()))
        + Float::with_val(wp, whole.ln_gamma_ref())
        - Float::with_val(wp, a.ln_gamma_ref())
        - Float::with_val(wp, b.ln_gamma_ref());
    let front = log_front.exp() / &a;
    // the method of Lentz
    let tiny = Float::with_val(wp, 1) >> (4 * wp);
    let kept = |v: Float| {
        if Float::with_val(wp, v.abs_ref()) < tiny {
            tiny.clone()
        } else {
            v
        }
    };
    let done = Float::with_val(wp, 1) >> (wp - 8);
    let mut c = Float::with_val(wp, 1);
    let mut d =
        kept(1u32 - Float::with_val(wp, &whole * &x) / Float::with_val(wp, &a + 1u32)).recip();
    let mut h = d.clone();
    for m in 1..=u32::try_from(crate::settings::terms(20_000_000)).unwrap_or(u32::MAX) {
        let twice = Float::with_val(wp, &a + 2 * m);
        let even =
            Float::with_val(wp, &b - m) * m * &x / (Float::with_val(wp, &twice - 1u32) * &twice);
        d = kept(1u32 + Float::with_val(wp, &even * &d)).recip();
        c = kept(1u32 + Float::with_val(wp, &even / &c));
        h *= Float::with_val(wp, &d * &c);
        let odd = -(Float::with_val(wp, &a + m) * Float::with_val(wp, &whole + m) * &x)
            / (Float::with_val(wp, &twice + 1u32) * &twice);
        d = kept(1u32 + Float::with_val(wp, &odd * &d)).recip();
        c = kept(1u32 + Float::with_val(wp, &odd / &c));
        let step = Float::with_val(wp, &d * &c);
        h *= &step;
        if Float::with_val(wp, &step - 1u32).abs() < done {
            let value = Float::with_val(wp, &front * &h);
            return Some(match mirrored {
                true => Float::with_val(p, 1u32 - value),
                false => Float::with_val(p, value),
            });
        }
    }
    None
}

/// The real arguments where `beta_share` is the way: 0 < x < 1 and positive a, b, one of them large.
fn beta_large(z: &Complex, a: &Complex, b: &Complex) -> bool {
    [z, a, b].iter().all(|v| v.imag().is_zero())
        && *z.real() > 0u32
        && *z.real() < 1u32
        && *a.real() > 0u32
        && *b.real() > 0u32
        && (*a.real() > 200u32 || *b.real() > 200u32)
}

/// Real a > 0 and z beyond 1000 and a + 1.
fn far_right(a: &Complex, z: &Complex) -> bool {
    a.imag().is_zero()
        && z.imag().is_zero()
        && *a.real() > 0u32
        && *z.real() > 1000u32
        && *z.real() > Float::with_val(64, a.real() + 1u32)
}

fn beta_full(a: &Complex, b: &Complex, wp: u32) -> Out {
    let (Some(ga), Some(gb)) = (gamma_any(a, wp)?, gamma_any(b, wp)?) else {
        return Some(None);
    };
    let r = rgamma(&Complex::with_val(wp, a + b), wp)?;
    Some(Some(ga * gb * r))
}

/// Whether the function is real at these arguments: the rounding noise of an imaginary part
/// that is exactly zero is then dropped.
fn real_valued(name: &str, a: &[Complex]) -> bool {
    if a.iter().any(|z| !z.imag().is_zero()) {
        return false;
    }
    let x = |k: usize| a[k].real();
    let positive = |k: usize| !x(k).is_sign_negative() || x(k).is_zero();
    let whole = |k: usize| x(k).is_integer();
    let below_one = |k: usize| *x(k) < 1u32;
    let from_minus_one = |k: usize| *x(k) >= -1;
    match (name, a.len()) {
        ("besselj" | "besseli" | "sphericalbesselj" | "struveh" | "struvel", 2) => {
            whole(0) || positive(1)
        }
        ("bessely" | "besselk" | "sphericalbessely", 2) => positive(1),
        ("airyai" | "airybi" | "airyaiprime" | "airybiprime", 1) => true,
        ("hyp0f1", 2) | ("hyp1f1", 3) => true,
        ("hyperu", 3) => positive(2),
        ("hyp2f1", 4) => below_one(3),
        ("uppergamma" | "lowergamma" | "gammaq" | "gammap" | "expint", 2) => positive(1),
        ("betainc" | "betaregularized", 3) => positive(0) && *x(0) <= 1u32,
        ("fresnels" | "fresnelc" | "sinc" | "dirichleteta", 1) => true,
        ("polylog", 2) => *x(1) <= 1u32,
        ("zeta", 2) => positive(1),
        ("elliptick" | "elliptice" | "ellipticnome", 1) => below_one(0),
        ("ellipticf" | "elliptice", 2) => *x(1) <= 1u32,
        ("ellipticpi", 2) => below_one(0) && below_one(1),
        ("ellipticpi", 3) => below_one(0) && *x(2) <= 1u32,
        ("jacobisn" | "jacobicn" | "jacobidn", 2) => true,
        ("elliptictheta", 3) => positive(2) || *x(0) >= 3u32,
        ("legendrep" | "chebyshevt" | "chebyshevu", 2) => from_minus_one(1),
        ("hermiteh", 2) | ("laguerrel", 2 | 3) => true,
        ("gegenbauerc", 3) => from_minus_one(2),
        ("jacobip", 4) => from_minus_one(3),
        _ => false,
    }
}

pub(crate) fn drop_noise(v: Complex) -> Complex {
    let p = v.prec().0;
    Complex::with_val(p, (v.real(), 0))
}

pub fn call(name: &str, a: &[Complex], p: u32) -> Out {
    let v = dispatch(name, a, p)?;
    Some(v.map(|v| {
        if real_valued(name, a) {
            drop_noise(v)
        } else {
            v
        }
    }))
}

fn dispatch(name: &str, a: &[Complex], p: u32) -> Out {
    let wp = p + 16;
    let cut = |v: Option<Complex>| v.map(|v| Complex::with_val(p, v));
    match (name, a.len()) {
        ("hyp0f1", 2) => hyp0f1(&a[0], &a[1], p),
        ("hyp1f1", 3) => hyp1f1(&a[0], &a[1], &a[2], p),
        ("hyp2f1", 4) => hyp2f1(&a[0], &a[1], &a[2], &a[3], p),
        ("hyperu", 3) => hyperu(&a[0], &a[1], &a[2], p),
        ("uppergamma", 2) => upper_gamma(&a[0], &a[1], p),
        ("uppergamma", 3) => {
            let (Some(x), Some(y)) = (
                upper_gamma(&a[0], &a[1], wp)?,
                upper_gamma(&a[0], &a[2], wp)?,
            ) else {
                return Some(None);
            };
            Some(cut(Some(x - y)))
        }
        // (far right of a the series of the lower function cancels itself: the upper one is the way)
        ("lowergamma" | "gammap", 2) if far_right(&a[0], &a[1]) => {
            let Some(upper) = upper_gamma(&a[0], &a[1], wp)? else {
                return Some(None);
            };
            if name == "gammap" {
                return Some(cut(Some(1u32 - upper * rgamma(&a[0], wp)?)));
            }
            let Some(whole) = gamma_any(&a[0], wp)? else {
                return Some(None);
            };
            Some(cut(Some(whole - upper)))
        }
        ("lowergamma", 2) => lower_gamma(&a[0], &a[1], p),
        ("gammaq", 2) => {
            let Some(g) = upper_gamma(&a[0], &a[1], wp)? else {
                return Some(None);
            };
            Some(cut(Some(g * rgamma(&a[0], wp)?)))
        }
        ("gammap", 2) => {
            if nonpositive_int(&a[0]) {
                return Some(Some(Complex::with_val(p, 1)));
            }
            let Some(g) = lower_gamma(&a[0], &a[1], wp)? else {
                return Some(None);
            };
            Some(cut(Some(g * rgamma(&a[0], wp)?)))
        }
        ("expint", 2) => {
            // z^(n-1) gamma(1 - n, z)
            let (n, z) = (&a[0], &a[1]);
            if z.is_zero() {
                return if *n.real() > 1u32 {
                    Some(Some(Complex::with_val(
                        p,
                        (Complex::with_val(wp, n) - 1u32).recip(),
                    )))
                } else {
                    Some(None)
                };
            }
            let one_n = Complex::with_val(wp, 1u32 - Complex::with_val(wp, n));
            let Some(g) = upper_gamma(&one_n, z, wp)? else {
                return Some(None);
            };
            let power = (Complex::with_val(wp, n - 1u32)
                * crate::numeric::unsigned_zero(Complex::with_val(wp, z)).ln())
            .exp();
            Some(cut(Some(g * power)))
        }
        ("betainc" | "betaregularized", 3) if beta_large(&a[0], &a[1], &a[2]) => {
            let share = beta_share(a[0].real(), a[1].real(), a[2].real(), wp)?;
            if name == "betaregularized" {
                return Some(cut(Some(Complex::with_val(wp, share))));
            }
            let Some(full) = beta_full(&a[1], &a[2], wp)? else {
                return Some(None);
            };
            Some(cut(Some(full * share)))
        }
        ("betainc", 3) => beta_inc(&a[0], &a[1], &a[2], p),
        ("betaregularized", 3) => {
            let Some(x) = beta_inc(&a[0], &a[1], &a[2], wp)? else {
                return Some(None);
            };
            let Some(full) = beta_full(&a[1], &a[2], wp)? else {
                return Some(None);
            };
            Some(cut(Some(x / full)))
        }
        _ => crate::bessel::call(name, a, p),
    }
}

/// `hypergeometric([a...], [b...], z)` for the numeric evaluator.
pub(crate) fn pfq_any(a: &[Complex], b: &[Complex], z: &Complex, p: u32) -> Out {
    let v = match (a.len(), b.len()) {
        (2, 1) => hyp2f1(&a[0], &a[1], &b[0], z, p),
        (1, 1) => hyp1f1(&a[0], &b[0], z, p),
        _ => pfq(a, b, z, p),
    }?;
    let real = a.iter().chain(b).chain([z]).all(|x| x.imag().is_zero())
        && (a.len() <= b.len() || *z.real() < 1u32);
    Some(v.map(|v| if real { drop_noise(v) } else { v }))
}
