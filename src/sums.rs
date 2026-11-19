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
            let p = u32::try_from(p).ok()?;
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
            .filter(|c| *c != 0 && c.abs() <= 64);
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

/// Numeric value of an infinite sum: direct for geometric decay, Richardson extrapolation otherwise.
fn numeric_sum(f: &Expr, k: &str, a: &Int) -> Option<Complex> {
    if !f.free_syms().iter().all(|s| &**s == k) {
        return None;
    }
    let digits = numeric::digits();
    let target = numeric::bits_for(digits);
    let prec = target * 2 + 128;
    let size = |z: &Complex| -> i64 {
        z.real()
            .get_exp()
            .map_or(i64::MIN, i64::from)
            .max(z.imag().get_exp().map_or(i64::MIN, i64::from))
    };
    let term = |n: &Int| numeric_term(f, k, n, prec);
    let (t1, t2) = (term(&Int::from(a + 200u32))?, term(&Int::from(a + 240u32))?);
    if t2.is_zero() || size(&t2) < size(&t1) - 6 {
        // geometric or faster
        let mut sum = Complex::new(prec);
        let mut n = a.clone();
        for _ in 0..2_000_000u32 {
            let t = term(&n)?;
            let done = t.is_zero() || size(&t) < size(&sum) - i64::from(target) - 24;
            sum += t;
            n += 1u32;
            if done && Int::from(&n - a) > 8 {
                let next = term(&n)?;
                if next.is_zero() || size(&next) <= size(&sum) - i64::from(target) - 20 {
                    return Some(sum);
                }
            }
        }
        return None;
    }
    // algebraic decay: extrapolate the partial sums S(N) at N = 16, 32, 48, ... in 1/N to 1/N = 0;
    // terms of alternating sign are taken in pairs
    let alternating = {
        let (u, v) = (term(&Int::from(a + 300u32))?, term(&Int::from(a + 301u32))?);
        u.real().is_sign_negative() != v.real().is_sign_negative()
    };
    let work = target * 5 + 512;
    let wide = |n: &Int| numeric_term(f, k, n, work);
    let mut partial = Complex::new(work);
    let mut n = a.clone();
    let mut diagonal: Vec<Complex> = Vec::new();
    let mut previous_row: Vec<Complex> = Vec::new();
    let mut agreed = 0;
    for j in 0..(6 * digits as usize + 40) {
        for _ in 0..16 {
            partial += wide(&n)?;
            n += 1u32;
            if alternating {
                partial += wide(&n)?;
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
            if d.is_zero() || size(&d) < size(&diagonal[j]) - i64::from(target) - 10 {
                agreed += 1;
                if agreed >= 3 {
                    return Some(Complex::with_val(prec, &diagonal[j]));
                }
            } else {
                agreed = 0;
            }
        }
    }
    None
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
        if hi - lo <= DIRECT_LIMIT {
            let total = add((lo..=hi).map(|i| subst_sym(&f, k, &Expr::int(i))).collect());
            return Ok((simplify(&total), false));
        }
    }
    if a.is_neg_inf() {
        return Err("sum: the lower end must be finite".into());
    }
    if a.is_one() && !b.is_inf() && f == powi(Expr::sym(k), -1) {
        return Ok((func("harmonic", vec![b.clone()]), false));
    }
    if let Some(r) = binomial_theorem(&f, k, a, b) {
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
        return Ok((simplify(&add(parts)), false));
    }
    if let Some(r) = s.rational(&f) {
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
            let half = Expr::int((lo + 1) / 2);
            let inner = Summer { k, a: &half, b };
            if let Some(r) = inner.rational(&paired) {
                return Ok((simplify(&add(vec![first, r])), false));
            }
        }
        if let Some(v) = numeric_sum(&f, k, &Int::from(lo)) {
            let digits = numeric::digits();
            return Ok((
                numeric::to_float_expr(
                    Complex::with_val(numeric::bits_for(digits) + 32, v),
                    digits,
                ),
                true,
            ));
        }
        return Err(format!(
            "no value found for the sum of {f} (it may diverge)"
        ));
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
        if hi - lo <= DIRECT_LIMIT {
            let total = mul((lo..=hi).map(|i| subst_sym(&f, k, &Expr::int(i))).collect());
            return Ok((simplify(&total), false));
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
        let log = func("ln", vec![f.clone()]);
        if let Some(v) = numeric_sum(&log, k, &Int::from(lo)) {
            let digits = numeric::digits();
            let value = Complex::with_val(numeric::bits_for(digits) + 32, v.exp());
            return Ok((numeric::to_float_expr(value, digits), true));
        }
        return Err(format!(
            "no value found for the product of {f} (it may diverge)"
        ));
    }
    Ok((
        Expr::call("product", vec![f, kx, a.clone(), b.clone()]),
        false,
    ))
}
