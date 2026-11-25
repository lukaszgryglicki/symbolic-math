//! Sums and products: closed forms where a rule applies, numeric values of infinite sums otherwise.

use crate::canon::{add, div, mul, neg, pow, powi, sub, subst, subst_sym};
use crate::diff::diff;
use crate::expand::expand;
use crate::expr::Expr;
use crate::funcs::func;
use crate::ntheory;
use crate::num::{Int, Rat};
use crate::numeric::{self, Env};
use crate::poly::Poly;
use crate::ratsimp;
use crate::simplify::simplify;
use crate::zfactor;
use rug::{Complete, Complex};
use std::rc::Rc;

const DIRECT_LIMIT: i64 = 200_000;

/// As many terms of a sum or factors of a product with whole ends are taken one by one and
/// exactly; `terms = N` sets a smaller count. (A larger one is for the sums that are added as
/// numbers: a million terms sin(k) written out one by one are no answer.)
fn direct_limit() -> i64 {
    i64::try_from(crate::settings::terms(DIRECT_LIMIT as u64))
        .unwrap_or(i64::MAX)
        .min(DIRECT_LIMIT)
}

/// Bernoulli polynomial `B_n(x)`.
fn bernoulli_poly(n: u32, x: &Expr) -> Expr {
    let mut terms = Vec::new();
    for j in 0..=n {
        let c = Rat::from(Int::from(n).binomial(j)) * ntheory::bernoulli(j);
        if !c.is_zero() {
            terms.push(mul(vec![
                Expr::rational(c),
                powi(x.clone(), i64::from(n - j)),
            ]));
        }
    }
    add(terms)
}

/// `sum_{k=a}^{b} r^k` for a symbol-free-of-k `r`; an infinite upper end needs `|r| < 1`.
fn geometric(r: &Expr, a: &Expr, b: &Expr) -> Expr {
    let low = pow(r.clone(), a.clone());
    if b.is_inf() {
        return div(low, sub(Expr::one(), r.clone()));
    }
    div(
        sub(pow(r.clone(), add(vec![b.clone(), Expr::one()])), low),
        sub(r.clone(), Expr::one()),
    )
}

struct Summer<'a> {
    k: &'a str,
    a: &'a Expr,
    b: &'a Expr,
}

impl Summer<'_> {
    fn has(&self, e: &Expr) -> bool {
        e.has_sym(self.k)
    }

    fn term(&self, t: &Expr) -> Option<Expr> {
        let kx = Expr::sym(self.k);
        if !self.has(t) {
            if self.b.is_inf() {
                return None;
            }
            return Some(mul(vec![
                t.clone(),
                add(vec![self.b.clone(), neg(self.a.clone()), Expr::one()]),
            ]));
        }
        let factors: Vec<Expr> = match t {
            Expr::Mul(f) => f.to_vec(),
            other => vec![other.clone()],
        };
        let (with, without): (Vec<Expr>, Vec<Expr>) =
            factors.into_iter().partition(|f| self.has(f));
        let coef = mul(without);
        // k^p r^k, possibly over k!
        // the harmonic series and its slower relatives diverge
        if with.len() == 1
            && self.b.is_inf()
            && let Expr::Pow(b, e) = &with[0]
            && **b == kx
            && e.as_rat().is_some_and(|s| *s >= -1 && *s < 0)
            && self.a.as_i64().is_some_and(|a| a >= 1)
            && let Some(sign) = crate::numeric::sign(&coef).filter(|s| *s != 0)
        {
            return Some(if sign > 0 {
                Expr::inf()
            } else {
                Expr::neg_inf()
            });
        }
        // sum 1/k^s = zeta(s) for a real s > 1
        if with.len() == 1
            && self.b.is_inf()
            && let Expr::Pow(b, e) = &with[0]
            && **b == kx
            && let Some(s) = e.as_rat().filter(|s| **s < -1)
            && let Some(start) = self.a.as_i64().filter(|a| (1..=1000).contains(a))
        {
            let head = add((1..start)
                .map(|i| pow(Expr::int(i), (**e).clone()))
                .collect());
            return Some(mul(vec![
                coef,
                sub(func("zeta", vec![Expr::rational((-s).complete())]), head),
            ]));
        }
        let (mut p, mut ratio, mut over_factorial) = (0i64, Vec::new(), false);
        for f in &with {
            let (b, e) = f.base_exp();
            if b == kx && e.as_i64().is_some_and(|n| n >= 0) {
                p += e.as_i64()?;
            } else if !self.has(&b) {
                let cs = ratsimp::coefficients(&e, self.k)?;
                if cs.len() != 2 {
                    return None;
                }
                ratio.push(pow(b.clone(), cs[1].clone()));
                if !cs[0].is_zero() {
                    return self.term(&mul(vec![
                        coef.clone(),
                        pow(b, cs[0].clone()),
                        mul(with.iter().filter(|g| *g != f).cloned().collect()),
                        pow(ratio.pop()?, kx.clone()),
                    ]));
                }
            } else if b == func("factorial", vec![kx.clone()]) && e.is_minus_one() {
                over_factorial = true;
            } else {
                return None;
            }
        }
        if over_factorial {
            // sum_{k>=0} k^p r^k/k! = (r d/dr)^p e^r
            if !self.b.is_inf()
                || self
                    .a
                    .as_i64()
                    .is_none_or(|a| !(0..=DIRECT_LIMIT).contains(&a))
            {
                return None;
            }
            let r = Expr::sym("#r");
            let mut closed = pow(Expr::e(), r.clone());
            for _ in 0..p {
                closed = mul(vec![r.clone(), diff(&closed, "#r")]);
            }
            let ratio = if ratio.is_empty() {
                Expr::one()
            } else {
                mul(ratio)
            };
            let mut total = subst(&closed, &r, &ratio);
            for i in 0..self.a.as_i64()? {
                total = sub(total, subst_sym(&mul(with.clone()), self.k, &Expr::int(i)));
            }
            return Some(mul(vec![coef, total]));
        }
        if ratio.is_empty() {
            if self.b.is_inf() {
                return None;
            }
            // (the formula of Faulhaber for the power p has p + 2 terms with the numbers of
            // Bernoulli up to that one: beyond the thousandth it is no closed form to read)
            let p = u32::try_from(p).ok().filter(|p| *p <= 1000)?;
            let hi = bernoulli_poly(p + 1, &add(vec![self.b.clone(), Expr::one()]));
            let lo = bernoulli_poly(p + 1, self.a);
            return Some(mul(vec![
                coef,
                div(sub(hi, lo), Expr::int(i64::from(p) + 1)),
            ]));
        }
        let ratio = simplify(&mul(ratio));
        if self.b.is_inf()
            && let Ok(v) = numeric::evalf(&func("abs", vec![ratio.clone()]), 64)
            && *v.real() >= 1
        {
            return None;
        }
        let r = Expr::sym("#r");
        let mut closed = geometric(&r, self.a, self.b);
        for _ in 0..p {
            closed = mul(vec![r.clone(), diff(&closed, "#r")]);
        }
        Some(mul(vec![coef, subst(&closed, &r, &ratio)]))
    }

    /// Sum of a rational function whose denominator splits into rational linear factors, to
    /// infinity or to a symbolic end.
    fn rational(&self, f: &Expr) -> Option<Expr> {
        let finite = !self.b.is_inf();
        if finite && self.b.is_const() {
            return None;
        }
        let start = self.a.as_rat()?.clone();
        let (num, den) = ratsimp::fraction(f);
        let dense = |e: &Expr| -> Option<Vec<Rat>> {
            ratsimp::coefficients(e, self.k)?
                .iter()
                .map(|c| c.as_rat().cloned())
                .collect()
        };
        let (p, q) = (dense(&num)?, dense(&den)?);
        if q.len() < p.len() + if finite { 1 } else { 2 } {
            return None;
        }
        let (_, factors, _) = zfactor::factor(&Poly::from_dense(1, 0, &q));
        let mut roots: Vec<(Rat, u32)> = Vec::new();
        for (g, m) in &factors {
            let d = g.to_dense(0)?;
            if d.len() != 2 {
                return None;
            }
            roots.push(((-&d[0]).complete() / &d[1], *m));
        }
        // leading coefficient of the denominator as a product of (k - root)
        let lead = q.last()?.clone();
        let kx = Expr::sym(self.k);
        let mut total = Vec::new();
        let mut residue_sum = Rat::new();
        for (i, (root, m)) in roots.iter().enumerate() {
            // Laurent coefficients at the root: expand N(k)/prod_{others}(k - r)^m around k = root
            let mut series = vec![Rat::new(); *m as usize];
            // work with exact Taylor coefficients by repeated symbolic differentiation
            let others = mul(roots
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, (r, mm))| {
                    powi(sub(kx.clone(), Expr::rational(r.clone())), -i64::from(*mm))
                })
                .collect());
            let numer_expr = add(p
                .iter()
                .enumerate()
                .map(|(d, c)| mul(vec![Expr::rational(c.clone()), powi(kx.clone(), d as i64)]))
                .collect());
            let mut g = mul(vec![
                numer_expr,
                others,
                Expr::rational(lead.clone().recip()),
            ]);
            let mut factorial = Rat::from(1);
            for (d, slot) in series.iter_mut().enumerate() {
                let v = simplify(&subst_sym(&g, self.k, &Expr::rational(root.clone())));
                *slot = v.as_rat()?.clone() / &factorial;
                factorial *= Rat::from(d as u32 + 1);
                g = diff(&g, self.k);
            }
            // term c/(k - root)^j with c = series[m - j]
            let shift = Expr::rational((&start - root).complete());
            if shift.as_rat().is_some_and(|s| s.is_integer() && *s <= 0) {
                return None;
            }
            // sum_{k >= a} 1/(k - r)^j = (-1)^j psi^(j-1)(a - r)/(j - 1)!, and the same from n + 1 on is taken off
            let beyond = add(vec![
                self.b.clone(),
                Expr::one(),
                Expr::rational((-root).complete()),
            ]);
            for j in 1..=*m {
                let c = series[(*m - j) as usize].clone();
                if c.is_zero() {
                    continue;
                }
                let order = Expr::int(i64::from(j) - 1);
                let psi = |at: &Expr| func("polygamma", vec![order.clone(), at.clone()]);
                let sign = if j % 2 == 0 { 1 } else { -1 };
                let fact = Rat::from(Int::from(Int::factorial(j - 1)));
                if j == 1 {
                    residue_sum += &c;
                }
                let tail = if finite { psi(&beyond) } else { Expr::zero() };
                total.push(mul(vec![
                    Expr::rational(c * sign / fact),
                    sub(psi(&shift), tail),
                ]));
            }
        }
        if finite {
            // psi(u + c) = psi(u) + 1/u + ... lets telescoping sums collapse
            let plain = simplify(&add(total));
            let moved = simplify(&whole_shifts(&plain));
            return Some(if moved.has_fn("digamma") || moved.has_fn("polygamma") {
                plain
            } else {
                moved
            });
        }
        if !residue_sum.is_zero() {
            return None;
        }
        let r = simplify(&add(total));
        (!r.has_fn("digamma") && !r.has_fn("polygamma")).then_some(r)
    }
}

/// `psi^(m)(u + c)` with a whole `c` in terms of `psi^(m)(u)`.
fn whole_shifts(e: &Expr) -> Expr {
    crate::canon::map_bottom_up(e, &mut |x| {
        let (m, arg) = match (x.fn_args("digamma"), x.fn_args("polygamma")) {
            (Some([arg]), _) => (0u32, arg.clone()),
            (_, Some([m, arg])) => match m.as_i64().and_then(|m| u32::try_from(m).ok()) {
                Some(m) => (m, arg.clone()),
                None => return x,
            },
            _ => return x,
        };
        let terms: Vec<Expr> = match &arg {
            Expr::Add(t) => t.to_vec(),
            other => vec![other.clone()],
        };
        let c = terms
            .iter()
            .find_map(|t| t.as_rat())
            .and_then(|r| r.clone().floor().numer().to_i64())
            .filter(|c| *c != 0 && c.unsigned_abs() <= 64);
        let (Some(c), false) = (c, arg.is_const()) else {
            return x;
        };
        let u = sub(arg, Expr::int(c));
        // psi^(m)(u + 1) = psi^(m)(u) + (-1)^m m!/u^(m+1)
        let step = |at: Expr| {
            let sign = if m % 2 == 0 { 1 } else { -1 };
            mul(vec![
                Expr::integer(Int::from(Int::factorial(m)) * sign),
                powi(at, -i64::from(m) - 1),
            ])
        };
        let mut out = vec![func("polygamma", vec![Expr::int(i64::from(m)), u.clone()])];
        for j in 0..c.max(0) {
            out.push(step(add(vec![u.clone(), Expr::int(j)])));
        }
        for j in 1..=(-c).max(0) {
            out.push(neg(step(sub(u.clone(), Expr::int(j)))));
        }
        add(out)
    })
}

/// `sum_{k=0}^{n} binomial(n, k) p^k q^(n-k) = (p + q)^n`.
fn binomial_theorem(f: &Expr, k: &str, a: &Expr, b: &Expr) -> Option<Expr> {
    if !a.is_zero() || b.is_infinite() {
        return None;
    }
    let kx = Expr::sym(k);
    let factors: Vec<Expr> = match f {
        Expr::Mul(t) => t.to_vec(),
        other => vec![other.clone()],
    };
    let at = factors.iter().position(|x| {
        x.fn_args("binomial")
            .is_some_and(|args| args[0] == *b && args[1] == kx)
    })?;
    let (mut p, mut coef) = (Vec::new(), Vec::new());
    for (i, x) in factors.iter().enumerate() {
        if i == at {
            continue;
        }
        if !x.has_sym(k) {
            coef.push(x.clone());
            continue;
        }
        let (base, e) = x.base_exp();
        if base.has_sym(k) {
            return None;
        }
        let c = ratsimp::coefficients(&e, k).filter(|c| c.len() == 2)?;
        coef.push(pow(base.clone(), c[0].clone()));
        p.push(pow(base, c[1].clone()));
    }
    Some(simplify(&mul(vec![
        mul(coef),
        pow(add(vec![Expr::one(), mul(p)]), b.clone()),
    ])))
}

fn numeric_term(f: &Expr, k: &str, n: &Int, prec: u32) -> Option<Complex> {
    let mut env = Env::new();
    env.insert(Rc::from(k), Complex::with_val(prec, n));
    numeric::evalf_env(f, prec, &env).ok()
}

/// ln|z|; -inf for 0.
fn ln_size(z: &Complex) -> f64 {
    let norm = rug::Float::with_val(64, z.norm_ref());
    if norm.is_zero() {
        f64::NEG_INFINITY
    } else {
        norm.ln().to_f64() / 2.0
    }
}

/// How a series shrinks far out, when it does so like `r^n/n^p`: the rate `-ln r` of a term
/// (above 0: the terms end up as small as wanted) and the terms that `bits` bits need. The
/// largest of 200 terms in a row is measured, so that terms that wave do not mislead, and the
/// law must hold again much farther out, where a power of the index no longer counts.
fn geometric_rate(
    term: &dyn Fn(&Int) -> Option<Complex>,
    a: &Int,
    bits: u32,
) -> Option<(f64, u64)> {
    const ROW: u32 = 200;
    let first = a.to_f64();
    let largest = |from: u32| -> Option<f64> {
        let mut most = f64::NEG_INFINITY;
        for n in from..from + ROW {
            most = most.max(ln_size(&term(&Int::from(a + n))?));
        }
        Some(most)
    };
    // the fall of ln|term| over a row from the index n on, and ln((n + row)/n)
    let fall = |n: u32| -> Option<(f64, f64)> {
        let index = first + f64::from(n);
        let fall = largest(n)? - largest(n + ROW)?;
        (index > 0.0 && fall.is_finite())
            .then(|| (fall, crate::detmath::ln((index + f64::from(ROW)) / index)))
    };
    let ((l1, a1), (l2, a2)) = (fall(5_000)?, fall(50_000)?);
    // fall = p ln((n + row)/n) + row rate
    let p = (l1 - l2) / (a1 - a2);
    let rate = (l2 - p * a2) / f64::from(ROW);
    crate::trace::detail(|| {
        format!(
            "the size of the terms far out, as r^n/n^p: r = {:.9}, p = {p:.4} (from the terms near the 5000th and the 50000th)",
            crate::detmath::exp(-rate)
        )
    });
    if !(rate.is_finite() && rate > 1e-13 && p.abs() < 1e6) {
        return None;
    }
    let fits = |n: u32| {
        fall(n).is_some_and(|(l, a)| {
            (l - (p * a + f64::from(ROW) * rate)).abs() <= 0.05 * l.abs() + 1e-12
        })
    };
    // far out the ratio must do most of the shrinking: the terms 1/n, or ln(1 + 1/n), look
    // like a geometric series with a ratio next to 1 when only two places are measured
    let rules = fall(5_000_000).is_some_and(|(l, _)| f64::from(ROW) * rate >= 0.5 * l);
    if !(fits(20_000) && fits(5_000_000) && rules) {
        return None;
    }
    let needed = (f64::from(bits) + 64.0) * std::f64::consts::LN_2 / rate + p.max(0.0) * 64.0;
    Some((rate, needed.min(1e18) as u64))
}

/// ln(1 + e) without the loss of a small `e`.
fn ln_1p(e: Complex, prec: u32) -> Complex {
    let small = rug::Float::with_val(64, e.norm_ref()) < 0.25f64;
    if !small {
        return Complex::with_val(prec, e + 1u32).ln();
    }
    // ln|1 + e| = ln(1 + 2 re + |e|^2)/2, and the angle of 1 + e
    let twice =
        rug::Float::with_val(prec, e.real() * 2u32) + rug::Float::with_val(prec, e.norm_ref());
    let re = twice.ln_1p() / 2u32;
    let im =
        rug::Float::with_val(prec, e.imag()).atan2(&rug::Float::with_val(prec, e.real() + 1u32));
    Complex::with_val(prec, (re, im))
}

/// Numeric value of an infinite sum: term by term where the terms shrink like those of a
/// geometric series, Richardson extrapolation where they shrink like a power of the index.
/// `term_at` gives the term of an index at a precision. The error tells what is missing when
/// that is known (more terms than `terms` allows).
fn numeric_sum(term_at: &dyn Fn(&Int, u32) -> Option<Complex>, a: &Int) -> Result<Complex, String> {
    let unknown = String::new;
    let digits = numeric::digits();
    let target = numeric::bits_for(digits);
    let prec = target * 2 + 128;
    let size = |z: &Complex| -> i64 {
        z.real()
            .get_exp()
            .map_or(i64::MIN, i64::from)
            .max(z.imag().get_exp().map_or(i64::MIN, i64::from))
    };
    let term = |n: &Int| term_at(n, prec);
    let most = crate::settings::terms(2_000_000);
    // the terms one by one until 64 in a row no longer change the digits (`spare` bits beyond
    // them): terms that vanish now and then, as with a cosine of the index, are no end
    let direct = |spare: i64| -> Option<Complex> {
        let below = i64::from(target) + spare;
        let mut sum = Complex::new(prec);
        let mut n = a.clone();
        let mut quiet = 0u32;
        for _ in 0..most {
            let t = term(&n)?;
            if t.is_zero() || size(&t) < size(&sum).saturating_sub(below) {
                quiet += 1;
            } else {
                quiet = 0;
            }
            sum += t;
            n += 1u32;
            if quiet >= 64 {
                crate::trace::step(|| {
                    format!(
                        "{} terms were added: the last 64 of them no longer change the digits",
                        Int::from(&n - a)
                    )
                });
                return Some(sum);
            }
        }
        None
    };
    let allowed = || match crate::settings::terms_set() {
        Some(n) => format!("terms = {n} allows {n} of them"),
        None => format!("{most} are allowed: terms = N allows more"),
    };
    let (t1, t2) = (
        term(&Int::from(a + 200u32)).ok_or_else(unknown)?,
        term(&Int::from(a + 240u32)).ok_or_else(unknown)?,
    );
    if t2.is_zero() || size(&t2) < size(&t1).saturating_sub(6) {
        // geometric or faster
        crate::trace::step(|| {
            "the terms shrink at least as fast as those of a geometric series: they are added one by one".to_string()
        });
        return direct(24).ok_or_else(|| {
            format!(
                "after {most} terms the next ones still change the digits ({})",
                allowed()
            )
        });
    }
    let mut missing = unknown();
    let ended = [5_000u32, 50_000, 5_000_000]
        .iter()
        .any(|n| term(&Int::from(a + *n)).is_some_and(|t| t.is_zero()));
    if ended {
        // (terms that are nothing at this precision: as good as an end)
        if let Some(v) = direct(24) {
            return Ok(v);
        }
    } else if let Some((rate, needed)) = geometric_rate(&|n| term_at(n, 64), a, target) {
        crate::trace::step(|| {
            format!(
                "the terms shrink like those of a geometric series with the ratio {:.6}: about {needed} terms give {digits} digits",
                crate::detmath::exp(-rate)
            )
        });
        if needed <= most {
            // (what is left after a term is that term over 1 - ratio)
            let spare = 28 + crate::detmath::log2(1.0 / rate).max(0.0).ceil() as i64;
            if let Some(v) = direct(spare) {
                return Ok(v);
            }
        } else {
            missing = format!("about {needed} terms give {digits} digits, {}", allowed());
        }
    }
    // algebraic decay: extrapolate the partial sums S(N) at N = 16, 32, 48, ... in 1/N to 1/N = 0;
    // terms of alternating sign are taken in pairs
    let alternating = {
        let (u, v) = (
            term(&Int::from(a + 300u32)).ok_or_else(unknown)?,
            term(&Int::from(a + 301u32)).ok_or_else(unknown)?,
        );
        u.real().is_sign_negative() != v.real().is_sign_negative()
    };
    crate::trace::step(|| {
        format!(
            "the terms shrink like a power of the index{}: the sums of the first 16, 32, 48, ... terms are extrapolated to all of them (Richardson)",
            if alternating {
                ", with alternating signs (taken in pairs)"
            } else {
                ""
            }
        )
    });
    let work = target * 5 + 512;
    let wide = |n: &Int| term_at(n, work);
    let mut partial = Complex::new(work);
    let mut n = a.clone();
    let mut diagonal: Vec<Complex> = Vec::new();
    let mut previous_row: Vec<Complex> = Vec::new();
    let mut agreed = 0;
    // (sixteen terms a row, or thirty-two: `terms = N` ends the rows sooner)
    let usual = 6 * digits as usize + 40;
    let rows =
        usual.min(crate::settings::terms_set().map_or(usize::MAX, |t| (t / 16).max(8) as usize));
    for j in 0..rows {
        for _ in 0..16 {
            partial += wide(&n).ok_or_else(unknown)?;
            n += 1u32;
            if alternating {
                partial += wide(&n).ok_or_else(unknown)?;
                n += 1u32;
            }
        }
        let mut row = vec![partial.clone()];
        for i in 1..=j {
            let delta = Complex::with_val(work, &row[i - 1] - &previous_row[i - 1]);
            row.push(Complex::with_val(
                work,
                &row[i - 1] + delta * ((j - i + 1) as u32) / (i as u32),
            ));
        }
        diagonal.push(row[j].clone());
        previous_row = row;
        if j >= 4 {
            let d = Complex::with_val(work, &diagonal[j] - &diagonal[j - 1]);
            if d.is_zero() || size(&d) < size(&diagonal[j]).saturating_sub(i64::from(target) + 10) {
                agreed += 1;
                if agreed >= 3 {
                    crate::trace::step(|| {
                        format!(
                            "{} terms were used: three extrapolations in a row agree to {digits} digits",
                            Int::from(&n - a)
                        )
                    });
                    return Ok(Complex::with_val(prec, &diagonal[j]));
                }
            } else {
                agreed = 0;
            }
        }
    }
    if rows < usual && missing.is_empty() {
        missing = format!(
            "the extrapolation from the first {} terms did not settle, and terms = {} allows no more",
            Int::from(&n - a),
            crate::settings::terms_set().unwrap_or(0)
        );
    }
    Err(missing)
}

/// The most bits of an exact sum or product of many fractions: beyond them (about 78000
/// digits) the fraction is not written out, and the terms are taken as numbers.
const EXACT_BITS: usize = 1 << 18;

/// The terms (or the factors) from `lo` to `hi` taken together exactly; `None` when they are
/// numbers whose exact sum or product grows beyond `EXACT_BITS`.
fn taken_exactly(f: &Expr, k: &str, lo: i64, hi: i64, times: bool) -> Option<Expr> {
    let join = |parts: Vec<Expr>| simplify(&if times { mul(parts) } else { add(parts) });
    let at = |i: i64| subst_sym(f, k, &Expr::int(i));
    let numbers = hi - lo >= 2048
        && simplify(&at(lo)).as_rat().is_some()
        && simplify(&at(hi)).as_rat().is_some();
    if !numbers {
        return Some(join((lo..=hi).map(at).collect()));
    }
    let mut total = if times { Expr::one() } else { Expr::zero() };
    let mut from = lo;
    loop {
        let to = from.saturating_add(1023).min(hi);
        let mut parts: Vec<Expr> = (from..=to).map(at).collect();
        parts.push(total);
        total = join(parts);
        let Some(q) = total.as_rat() else {
            // not numbers after all (the terms at both ends of the sum of sqrt(k) to 90000
            // are whole): what was added so far would be gone through again with every
            // thousand terms more, so all of them are joined at once
            return Some(join((lo..=hi).map(at).collect()));
        };
        let bits = q.numer().significant_bits() as usize + q.denom().significant_bits() as usize;
        if bits > EXACT_BITS {
            return None;
        }
        if to == hi {
            break;
        }
        from = to + 1;
    }
    Some(total)
}

/// The sum (or the product) of `f` for the whole numbers from `lo` to `hi` by numbers: for
/// more terms than are taken exactly, when the term has no other name.
fn numeric_finite(f: &Expr, k: &str, lo: i64, hi: i64, times: bool) -> Option<Complex> {
    let count = u64::try_from(hi.checked_sub(lo)?).ok()?.checked_add(1)?;
    if count > crate::settings::terms(2_000_000) || !f.free_syms().iter().all(|s| &**s == k) {
        return None;
    }
    let prec = numeric::bits_for(numeric::digits()) * 2 + 128;
    let mut total = Complex::with_val(prec, u32::from(times));
    for n in lo..=hi {
        let t = numeric_term(f, k, &Int::from(n), prec)?;
        if !(t.real().is_finite() && t.imag().is_finite()) {
            return None;
        }
        if times {
            total *= t;
        } else {
            total += t;
        }
    }
    crate::trace::step(|| {
        format!(
            "the {count} {} are taken one by one as numbers of {prec} bits",
            if times { "factors" } else { "terms" }
        )
    });
    Some(total)
}

/// `(value, approximate)`.
pub fn sum(f: &Expr, k: &str, a: &Expr, b: &Expr) -> Result<(Expr, bool), String> {
    let known = crate::assume::get(k);
    crate::assume::set(k, crate::assume::Facts::integer());
    let result = sum_integer_index(f, k, a, b);
    crate::assume::clear(k);
    if known != crate::assume::Facts::default() {
        crate::assume::set(k, known);
    }
    result
}

fn sum_integer_index(f: &Expr, k: &str, a: &Expr, b: &Expr) -> Result<(Expr, bool), String> {
    let f = simplify(&crate::canon::rebuild(f, &mut |c| c.clone()));
    if let (Some(lo), Some(hi)) = (a.as_i64(), b.as_i64()) {
        if hi < lo {
            return Ok((Expr::zero(), false));
        }
        if hi
            .checked_sub(lo)
            .is_some_and(|span| span <= direct_limit())
        {
            if let Some(total) = taken_exactly(&f, k, lo, hi, false) {
                crate::trace::step(|| {
                    format!(
                        "the {} terms from {k} = {lo} to {hi} are added one by one",
                        hi - lo + 1
                    )
                });
                return Ok((total, false));
            }
            crate::trace::step(|| {
                format!(
                    "the exact sum of the {} terms would have more than {} digits: it is not written out",
                    hi - lo + 1,
                    EXACT_BITS * 3 / 10
                )
            });
        }
    }
    if a.is_neg_inf() {
        return Err("sum: the lower end must be finite".into());
    }
    if a.is_one() && !b.is_inf() && f == powi(Expr::sym(k), -1) {
        crate::trace::step(|| format!("1 + 1/2 + ... + 1/{b} is the harmonic number of {b}"));
        return Ok((func("harmonic", vec![b.clone()]), false));
    }
    if let Some(r) = binomial_theorem(&f, k, a, b) {
        crate::trace::step(|| format!("by the binomial theorem: {r}"));
        return Ok((r, false));
    }
    let s = Summer { k, a, b };
    let ex = expand(&f);
    let terms: Vec<Expr> = match &ex {
        Expr::Add(t) => t.to_vec(),
        other => vec![other.clone()],
    };
    let closed: Option<Vec<Expr>> = terms.iter().map(|t| s.term(t)).collect();
    if let Some(parts) = closed {
        crate::trace::step(|| {
            format!(
                "the sum of {f} for {k} from {a} to {b}: each term has a known sum (powers of {k} by the formulas of Faulhaber, r^{k} by the geometric series, 1/{k}^s by the zeta function, x^{k}/{k}! by the exponential): {}",
                terms
                    .iter()
                    .zip(&parts)
                    .map(|(t, p)| format!("{t} gives {p}"))
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        });
        return Ok((simplify(&add(parts)), false));
    }
    if let Some(r) = s.rational(&f) {
        crate::trace::step(|| {
            format!(
                "the sum of {f} for {k} from {a} to {b}: a ratio of polynomials is split into partial fractions, whose sums telescope or are values of the digamma function: {r}"
            )
        });
        return Ok((r, false));
    }
    // alternating rational terms: pair even and odd indices
    if b.is_inf()
        && let Some(lo) = a.as_i64()
    {
        let kx = Expr::sym(k);
        let even = subst(&f, &kx, &mul(vec![Expr::int(2), kx.clone()]));
        let odd = subst(
            &f,
            &kx,
            &add(vec![mul(vec![Expr::int(2), kx.clone()]), Expr::one()]),
        );
        let paired = simplify(&add(vec![even, odd]));
        if !paired.any(&|e| matches!(e, Expr::Pow(b, _) if b.is_minus_one())) && lo % 2 == 0 {
            let half = Expr::int(lo / 2);
            let inner = Summer { k, a: &half, b };
            if let Some(r) = inner.rational(&paired) {
                return Ok((r, false));
            }
        }
        if !paired.any(&|e| matches!(e, Expr::Pow(b, _) if b.is_minus_one())) && lo % 2 != 0 {
            let first = subst_sym(&f, k, &Expr::int(lo));
            let half = Expr::int(lo.div_euclid(2) + lo.rem_euclid(2));
            let inner = Summer { k, a: &half, b };
            if let Some(r) = inner.rational(&paired) {
                return Ok((simplify(&add(vec![first, r])), false));
            }
        }
        let only_index = f.free_syms().iter().all(|s| &**s == k);
        let found = if only_index {
            let (found, under) = numeric::underflowed(|| {
                numeric_sum(&|n, prec| numeric_term(&f, k, n, prec), &Int::from(lo))
            });
            match found {
                // (a sum that begins so far out that its terms are 0 as numbers)
                Ok(v) if under && v.is_zero() => {
                    Err("its terms are smaller than the smallest number that is kept".to_string())
                }
                other => other,
            }
        } else {
            Err(String::new())
        };
        return match found {
            Ok(v) => {
                let digits = numeric::digits();
                Ok((
                    numeric::to_float_expr(
                        Complex::with_val(numeric::bits_for(digits) + 32, v),
                        digits,
                    ),
                    true,
                ))
            }
            Err(missing) if missing.is_empty() => Err(format!(
                "no value found for the sum of {f} (it may diverge)"
            )),
            Err(missing) => Err(format!("no value found for the sum of {f}: {missing}")),
        };
    }
    if let (Some(lo), Some(hi)) = (a.as_i64(), b.as_i64())
        && let Some(v) = numeric_finite(&f, k, lo, hi, false)
    {
        let digits = numeric::digits();
        let value = Complex::with_val(numeric::bits_for(digits) + 32, v);
        return Ok((numeric::to_float_expr(value, digits), true));
    }
    Ok((
        Expr::call("sum", vec![f, Expr::sym(k), a.clone(), b.clone()]),
        false,
    ))
}

/// The parts of a product of powers and polynomials in the index.
#[derive(Default)]
struct Factors {
    /// The factors without the index.
    constants: Vec<Expr>,
    /// The roots of the polynomials, each with the power of its factor.
    roots: Vec<(Expr, i64)>,
    /// Powers of bases without the index: the base and the exponent, a polynomial in the index.
    powers: Vec<(Expr, Expr)>,
}

fn factors_of(f: &Expr, k: &str, power: i64, out: &mut Factors) -> Option<()> {
    if !f.has_sym(k) {
        out.constants.push(powi(f.clone(), power));
        return Some(());
    }
    match f {
        Expr::Mul(a) => a.iter().try_for_each(|x| factors_of(x, k, power, out)),
        Expr::Pow(g, c) if c.as_i64().is_some() => factors_of(g, k, power * c.as_i64()?, out),
        Expr::Pow(base, h) if !base.has_sym(k) => {
            ratsimp::coefficients(h, k)?;
            out.powers
                .push(((**base).clone(), mul(vec![Expr::int(power), (**h).clone()])));
            Some(())
        }
        _ => {
            let cs = ratsimp::coefficients(f, k)?;
            let degree = cs.len().checked_sub(1).filter(|d| *d >= 1)? as i64;
            let solved = crate::solve::solve(f, k).ok()?;
            crate::solve::release(&solved.params);
            if solved.all || !solved.params.is_empty() || solved.roots.iter().any(|r| r.approx) {
                return None;
            }
            if solved.roots.iter().map(|r| i64::from(r.mult)).sum::<i64>() != degree {
                return None;
            }
            for r in &solved.roots {
                out.roots.push((r.value.clone(), power * i64::from(r.mult)));
            }
            out.constants.push(powi(cs.last()?.clone(), power));
            Some(())
        }
    }
}

/// A product of powers and of polynomials in the index in closed form: every linear factor
/// `k - r` gives `gamma(b + 1 - r)/gamma(a - r)`, and without an upper end what is left of the
/// gamma functions when the product settles.
fn closed_product(f: &Expr, k: &str, a: &Expr, b: &Expr) -> Option<Expr> {
    let mut parts = Factors::default();
    factors_of(&ratsimp::factor(&ratsimp::together(f)), k, 1, &mut parts)?;
    let gamma = |e: Expr| func("gamma", vec![simplify(&e)]);
    // a factor that vanishes or has a pole at an index of the range
    let hits = |r: &Expr| {
        simplify(&sub(a.clone(), r.clone()))
            .as_int()
            .is_some_and(|d| *d <= 0)
    };
    if parts.roots.iter().any(|(r, _)| hits(r)) {
        return None;
    }
    let constant = simplify(&mul(parts.constants.clone()));
    if b.is_inf() {
        let count: i64 = parts.roots.iter().map(|(_, p)| p).sum();
        if !parts.powers.is_empty() || !constant.is_one() || count != 0 {
            return None;
        }
        let drift = simplify(&add(parts
            .roots
            .iter()
            .map(|(r, p)| mul(vec![Expr::int(*p), r.clone()]))
            .collect()));
        if !drift.is_zero() {
            // the factors go like b^(-drift)
            let size = numeric::evalf_digits(&drift, 20).ok()?;
            return (*size.real() > 0).then(Expr::zero);
        }
        return Some(simplify(&mul(parts
            .roots
            .iter()
            .map(|(r, p)| powi(gamma(sub(a.clone(), r.clone())), -p))
            .collect())));
    }
    let count = add(vec![b.clone(), neg(a.clone()), Expr::one()]);
    let mut out = vec![pow(constant, count)];
    for (base, h) in &parts.powers {
        let (total, approx) = sum(h, k, a, b).ok()?;
        if approx || total.has_fn("sum") {
            return None;
        }
        out.push(pow(base.clone(), total));
    }
    for (r, p) in &parts.roots {
        let upper = add(vec![b.clone(), Expr::one(), neg(r.clone())]);
        let ratio = if r.as_int().is_some() {
            // whole roots: factorials
            let fact = |e: Expr| func("factorial", vec![simplify(&sub(e, Expr::one()))]);
            div(fact(upper), fact(sub(a.clone(), r.clone())))
        } else {
            div(gamma(upper), gamma(sub(a.clone(), r.clone())))
        };
        out.push(powi(ratio, *p));
    }
    Some(simplify(&mul(out)))
}

/// `(value, approximate)`.
pub fn product(f: &Expr, k: &str, a: &Expr, b: &Expr) -> Result<(Expr, bool), String> {
    let f = simplify(f);
    let kx = Expr::sym(k);
    if let (Some(lo), Some(hi)) = (a.as_i64(), b.as_i64()) {
        if hi < lo {
            return Ok((Expr::one(), false));
        }
        if hi
            .checked_sub(lo)
            .is_some_and(|span| span <= direct_limit())
            && let Some(total) = taken_exactly(&f, k, lo, hi, true)
        {
            return Ok((total, false));
        }
    }
    if !f.has_sym(k) && !b.is_inf() {
        return Ok((
            pow(f, add(vec![b.clone(), neg(a.clone()), Expr::one()])),
            false,
        ));
    }
    if f == kx && !b.is_inf() {
        let fact = |e: Expr| func("factorial", vec![e]);
        return Ok((
            div(fact(b.clone()), fact(sub(a.clone(), Expr::one()))),
            false,
        ));
    }
    if let Some(value) = closed_product(&f, k, a, b) {
        return Ok((value, false));
    }
    if b.is_inf()
        && let Some(lo) = a.as_i64()
    {
        // the sum of the logarithms, each from the factor less 1: a factor near 1 keeps its digits
        crate::trace::step(|| {
            format!(
                "no closed form of the product is known: it is e to the power of the sum of the logarithms of its factors, ln({f})"
            )
        });
        let excess = simplify(&sub(f.clone(), Expr::one()));
        let found = if f.free_syms().iter().all(|s| &**s == k) {
            numeric_sum(
                &|n, prec| numeric_term(&excess, k, n, prec).map(|e| ln_1p(e, prec)),
                &Int::from(lo),
            )
        } else {
            Err(String::new())
        };
        return match found {
            Ok(v) => {
                let digits = numeric::digits();
                let value = Complex::with_val(numeric::bits_for(digits) + 32, numeric::cexp(v));
                Ok((numeric::to_float_expr(value, digits), true))
            }
            Err(missing) if missing.is_empty() => Err(format!(
                "no value found for the product of {f} (it may diverge)"
            )),
            Err(missing) => Err(format!(
                "no value found for the product of {f}: of the sum of its logarithms {missing}"
            )),
        };
    }
    if let (Some(lo), Some(hi)) = (a.as_i64(), b.as_i64())
        && let Some(v) = numeric_finite(&f, k, lo, hi, true)
    {
        let digits = numeric::digits();
        let value = Complex::with_val(numeric::bits_for(digits) + 32, v);
        return Ok((numeric::to_float_expr(value, digits), true));
    }
    Ok((
        Expr::call("product", vec![f, kx, a.clone(), b.clone()]),
        false,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn how_fast_the_terms_of_a_series_shrink() {
        let bits = numeric::bits_for(32);
        let term = |q: f64, p: i32| {
            move |n: &Int| -> Option<Complex> {
                let n = rug::Float::with_val(64, n);
                let power = rug::Float::with_val(64, q).ln() * &n;
                Some(Complex::with_val(64, power.exp() / n.pow(p)))
            }
        };
        use rug::ops::Pow;
        // 0.99^n/n^2: the ratio 0.99, about twelve thousand terms for 32 digits
        let (rate, needed) = geometric_rate(&term(0.99, 2), &Int::from(1), bits).expect("a rate");
        assert!((rate + 0.99f64.ln()).abs() < 1e-6, "{rate}");
        assert!((11_000..14_000).contains(&needed), "{needed}");
        let (rate, needed) = geometric_rate(&term(0.999, 0), &Int::from(1), bits).expect("a rate");
        assert!((rate + 0.999f64.ln()).abs() < 1e-7, "{rate}");
        assert!((110_000..130_000).contains(&needed), "{needed}");
        // powers of the index alone, and ln(1 + 1/n), are no geometric series
        assert!(geometric_rate(&term(1.0, 2), &Int::from(1), bits).is_none());
        let log = |n: &Int| -> Option<Complex> {
            let n = rug::Float::with_val(64, n);
            Some(Complex::with_val(64, n.recip().ln_1p()))
        };
        assert!(geometric_rate(&log, &Int::from(1), bits).is_none());
    }

    #[test]
    fn the_logarithm_of_a_factor_near_one_keeps_its_digits() {
        for (re, im) in [
            (1e-30, 0.0),
            (-1e-25, 3e-27),
            (0.2, -0.1),
            (3.0, 4.0),
            (-0.5, 0.0),
        ] {
            let e = Complex::with_val(400, (re, im));
            let exact = Complex::with_val(2000, &e + 1u32).ln();
            let got = ln_1p(e, 400);
            let gap = Complex::with_val(400, &got - &exact);
            let scale = rug::Float::with_val(64, exact.abs_ref());
            assert!(
                rug::Float::with_val(64, gap.abs_ref()) <= scale * 1e-100f64,
                "ln(1 + ({re} + {im} i))"
            );
        }
    }

    #[test]
    fn an_exact_sum_that_grows_beyond_use_is_not_written_out() {
        let k = Expr::sym("k");
        let term = crate::canon::powi(add(vec![crate::canon::powi(k, 2), Expr::one()]), -1);
        // 1/(k^2 + 1): three thousand terms are a fraction that can be read, a hundred thousand not
        let small = taken_exactly(&term, "k", 1, 3000, false).expect("an exact sum");
        assert!(small.as_rat().is_some());
        assert!(taken_exactly(&term, "k", 1, 100_000, false).is_none());
        // whole numbers stay small, and what is no number is not counted
        let square = crate::canon::powi(Expr::sym("k"), 2);
        assert_eq!(
            taken_exactly(&square, "k", 1, 200_000, false).and_then(|s| s.as_i64()),
            Some(2_666_686_666_700_000)
        );
        assert_eq!(
            taken_exactly(&square, "k", 1, 2000, false).and_then(|s| s.as_i64()),
            Some(2000 * 2001 * 4001 / 6)
        );
        assert_eq!(
            taken_exactly(&Expr::sym("k"), "k", 1, 10, true).and_then(|s| s.as_i64()),
            Some(3_628_800)
        );
    }
}
