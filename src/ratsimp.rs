//! Rational functions over "kernels": symbols, radicals, exponentials and function values are the
//! variables of multivariate polynomials. Gives the normal form `P/Q`, cancellation and factoring.

use crate::canon::{add, coeff_and_rest, div, mul, pow, rebuild};
use crate::expand::expand;
use crate::expr::Expr;
use crate::num::{Int, Num, Rat};
use crate::poly::Poly;
use crate::zfactor;
use rug::Complete;
use std::collections::BTreeMap;

type Frac = (Poly, Poly);
type Key = (Expr, Expr);

/// Exponent as a sum of `coefficient * rest` parts with rational coefficients.
fn exponent_parts(x: &Expr) -> Vec<(Rat, Expr)> {
    let mut out = Vec::new();
    let mut one = |t: &Expr| {
        let (c, rest) = coeff_and_rest(t);
        if !c.re.is_zero() {
            out.push((c.re, rest.clone()));
        }
        if !c.im.is_zero() {
            out.push((c.im, mul(vec![Expr::i(), rest])));
        }
    };
    match x {
        Expr::Add(terms) => terms.iter().for_each(&mut one),
        _ => one(x),
    }
    out
}

/// A whole exponent too large to multiply out: such a power is a kernel of its own.
fn huge_power(x: &Expr) -> bool {
    x.as_int().is_some_and(|k| k.significant_bits() > 10)
}

fn rat_gcd(a: &Rat, b: &Rat) -> Rat {
    if a.is_zero() {
        return b.clone().abs();
    }
    if b.is_zero() {
        return a.clone().abs();
    }
    let n = (a.numer() * b.denom())
        .complete()
        .gcd(&(b.numer() * a.denom()).complete());
    Rat::from((n, (a.denom() * b.denom()).complete()))
}

pub struct Gens {
    /// (base, rest) -> the common rational step of its exponents.
    steps: BTreeMap<Key, Rat>,
    keys: Vec<Key>,
    kernels: Vec<Expr>,
    /// kernel^q = base^p for kernels that are roots.
    relations: Vec<Option<(u32, u32, Expr)>>,
}

impl Gens {
    fn note(&mut self, base: &Expr, rest: &Expr, c: &Rat) {
        let key = (base.clone(), rest.clone());
        let g = match self.steps.get(&key) {
            Some(old) => rat_gcd(old, c),
            None => c.clone().abs(),
        };
        self.steps.insert(key, g);
    }

    fn scan(&mut self, e: &Expr) -> Option<()> {
        match e {
            Expr::Num(n) => {
                if !n.is_real() {
                    self.note(&Expr::i(), &Expr::one(), &Rat::from(1));
                }
            }
            Expr::Float(_) | Expr::Sym(_) | Expr::Fn(..) => {
                self.note(e, &Expr::one(), &Rat::from(1))
            }
            Expr::Add(a) | Expr::Mul(a) => {
                for x in a.iter() {
                    self.scan(x)?;
                }
            }
            Expr::Pow(_, x) if huge_power(x) => self.note(e, &Expr::one(), &Rat::from(1)),
            Expr::Pow(b, x) => {
                for (c, rest) in exponent_parts(x) {
                    if rest.is_one() {
                        self.scan(b)?;
                        if !c.is_integer() {
                            self.note(b, &rest, &c);
                        }
                    } else {
                        self.note(b, &rest, &c);
                    }
                }
            }
            _ => return None,
        }
        Some(())
    }

    pub fn of(exprs: &[&Expr]) -> Option<Gens> {
        let mut g = Gens {
            steps: BTreeMap::new(),
            keys: Vec::new(),
            kernels: Vec::new(),
            relations: Vec::new(),
        };
        for e in exprs {
            g.scan(e)?;
        }
        for (key, step) in &g.steps {
            let (base, rest) = key;
            let unit = *base == Expr::i() && rest.is_one();
            let kernel = if unit {
                Expr::i()
            } else {
                pow(
                    base.clone(),
                    mul(vec![Expr::rational(step.clone()), rest.clone()]),
                )
            };
            let relation = if unit {
                Some((2, 1, Expr::minus_one()))
            } else if rest.is_one() && !step.is_integer() {
                match (step.denom().to_u32(), step.numer().to_u32()) {
                    (Some(q), Some(p)) => Some((q, p, base.clone())),
                    _ => None,
                }
            } else {
                None
            };
            g.keys.push(key.clone());
            g.kernels.push(kernel);
            g.relations.push(relation);
        }
        Some(g)
    }

    pub fn len(&self) -> usize {
        self.kernels.len()
    }
    pub fn is_empty(&self) -> bool {
        self.kernels.is_empty()
    }
    pub fn kernels(&self) -> &[Expr] {
        &self.kernels
    }
    pub fn index_of(&self, kernel: &Expr) -> Option<usize> {
        self.kernels.iter().position(|k| k == kernel)
    }
    /// `(index, q, p, base)` for the kernels with `kernel^q = base^p`.
    pub fn radicals(&self) -> Vec<(usize, u32, u32, Expr)> {
        self.relations
            .iter()
            .enumerate()
            .filter_map(|(i, r)| r.as_ref().map(|(q, p, b)| (i, *q, *p, b.clone())))
            .collect()
    }
    /// Polynomial of an expression that has no denominator in these generators.
    pub fn poly(&self, e: &Expr) -> Option<Poly> {
        let (num, den) = self.frac(e)?;
        Some(num.scale(&den.as_const()?.recip()))
    }
    /// Numerator and denominator without the reduction of roots.
    pub fn raw_fraction(&self, e: &Expr) -> Option<(Poly, Poly)> {
        self.frac(e)
    }

    fn power(&self, base: &Expr, rest: &Expr, c: &Rat) -> Option<Frac> {
        let n = self.len();
        let key = (base.clone(), rest.clone());
        let idx = self.keys.iter().position(|k| *k == key)?;
        let k = (c / &self.steps[&key]).complete();
        let e = k.numer().to_i64().filter(|_| k.is_integer())?;
        // (e^t beside e^(t/1000000000) would be a power of a thousand million, and e^1000.5
        // beside e^(-7) one of thousands of a root: left as they are)
        if e.unsigned_abs() > 128 {
            return None;
        }
        let p = Poly::var(n, idx).pow(u32::try_from(e.unsigned_abs()).ok()?);
        Some(if e >= 0 {
            (p, Poly::one(n))
        } else {
            (Poly::one(n), p)
        })
    }

    fn frac(&self, e: &Expr) -> Option<Frac> {
        let n = self.len();
        match e {
            Expr::Num(x) => {
                let l = x.re.denom().clone().lcm(x.im.denom());
                let scaled = x.mul(&Num::from_int(l.clone()));
                let mut p = Poly::constant(n, scaled.re);
                if !scaled.im.is_zero() {
                    let (i, _) = self.power(&Expr::i(), &Expr::one(), &Rat::from(1))?;
                    p = p.add(&i.scale(&scaled.im));
                }
                Some((p, Poly::constant(n, Rat::from(l))))
            }
            Expr::Float(_) | Expr::Sym(_) | Expr::Fn(..) => {
                self.power(e, &Expr::one(), &Rat::from(1))
            }
            Expr::Add(a) => {
                let mut acc = (Poly::zero(n), Poly::one(n));
                for t in a.iter() {
                    let (tn, td) = self.frac(t)?;
                    let g = acc.1.gcd(&td);
                    let (da, db) = (acc.1.div_exact(&g)?, td.div_exact(&g)?);
                    acc = (acc.0.mul(&db).add(&tn.mul(&da)), acc.1.mul(&db));
                }
                Some(acc)
            }
            Expr::Mul(a) => {
                let mut acc = (Poly::one(n), Poly::one(n));
                for t in a.iter() {
                    let (tn, td) = self.frac(t)?;
                    acc = (acc.0.mul(&tn), acc.1.mul(&td));
                }
                Some(acc)
            }
            Expr::Pow(_, x) if huge_power(x) => self.power(e, &Expr::one(), &Rat::from(1)),
            Expr::Pow(b, x) => {
                let mut acc = (Poly::one(n), Poly::one(n));
                for (c, rest) in exponent_parts(x) {
                    let part = if rest.is_one() && c.is_integer() {
                        let (bn, bd) = self.frac(b)?;
                        let k = c.numer().to_i64()?;
                        // (a whole part of a sum in the exponent that is too large to multiply out: left as it is)
                        let m = u32::try_from(k.unsigned_abs())
                            .ok()
                            .filter(|m| *m <= 4096)?;
                        if k >= 0 {
                            (bn.pow(m), bd.pow(m))
                        } else {
                            (bd.pow(m), bn.pow(m))
                        }
                    } else {
                        self.power(b, &rest, &c)?
                    };
                    acc = (acc.0.mul(&part.0), acc.1.mul(&part.1));
                }
                Some(acc)
            }
            _ => None,
        }
    }

    /// Applies `kernel^q = base^p` to the powers of the root kernels.
    fn reduce(&self, p: &Poly) -> Poly {
        let n = self.len();
        let mut p = p.clone();
        for _ in 0..4 {
            let mut changed = false;
            for (i, rel) in self.relations.iter().enumerate() {
                let Some((q, pw, base)) = rel else { continue };
                if p.deg(i) < *q {
                    continue;
                }
                let Some((bn, bd)) = self.frac(base) else {
                    continue;
                };
                if !bd.is_const() || bn.uses(i) {
                    continue;
                }
                // (the power of the base that the highest power of the root comes to: where
                // it has millions of bits, as 3^(10^9) for sqrt(3)^(2 10^9 + 1), the powers
                // of the root stay as they are)
                let bits =
                    bn.t.values()
                        .map(|c| {
                            c.numer()
                                .significant_bits()
                                .max(c.denom().significant_bits())
                        })
                        .fold(1u32, u32::max);
                let work = u64::from(p.deg(i) / *q)
                    .saturating_mul(u64::from(*pw))
                    .saturating_mul(u64::from(bits))
                    .saturating_mul(bn.t.len() as u64);
                if work > 1 << 23 {
                    continue;
                }
                let b = bn.scale(&bd.as_const().unwrap().recip()).pow(*pw);
                let mut acc = Poly::zero(n);
                for (d, c) in p.coeffs(i).iter().enumerate() {
                    if c.is_zero() {
                        continue;
                    }
                    let d = d as u32;
                    acc = acc.add(&c.mul(&Poly::var(n, i).pow(d % q)).mul(&b.pow(d / q)));
                }
                p = acc;
                changed = true;
            }
            if !changed {
                break;
            }
        }
        p
    }

    /// Normal form: reduced, the denominator free of `i` and of square roots where possible, gcd cancelled.
    fn normalize(&self, f: Frac) -> Option<Frac> {
        let n = self.len();
        let (mut num, mut den) = (self.reduce(&f.0), self.reduce(&f.1));
        if den.is_zero() {
            return None;
        }
        for (i, rel) in self.relations.iter().enumerate() {
            if !matches!(rel, Some((2, _, _))) || !den.uses(i) {
                continue;
            }
            let conj = Poly::from_coeffs(
                n,
                i,
                &den.coeffs(i)
                    .iter()
                    .enumerate()
                    .map(|(d, c)| if d % 2 == 1 { c.neg() } else { c.clone() })
                    .collect::<Vec<_>>(),
            );
            let nd = self.reduce(&den.mul(&conj));
            if nd.uses(i) || nd.is_zero() {
                continue;
            }
            num = self.reduce(&num.mul(&conj));
            den = nd;
        }
        let g = num.gcd(&den);
        if !g.is_const() {
            num = num.div_exact(&g)?;
            den = den.div_exact(&g)?;
        }
        let (c, den) = den.primitive();
        Some((num.scale(&c.recip()), den))
    }

    pub fn convert(&self, e: &Expr) -> Option<Frac> {
        self.normalize(self.frac(e)?)
    }

    pub fn expr(&self, p: &Poly) -> Expr {
        let mut terms = Vec::with_capacity(p.t.len());
        for (m, c) in &p.t {
            let mut factors = vec![Expr::rational(c.clone())];
            for (i, e) in m.iter().enumerate() {
                if *e > 0 {
                    factors.push(pow(self.kernels[i].clone(), Expr::int(i64::from(*e))));
                }
            }
            terms.push(mul(factors));
        }
        add(terms)
    }
}

/// Simplifies inside function arguments and exponents first.
fn inner_normal(e: &Expr) -> Expr {
    match e {
        Expr::Fn(..) => rebuild(e, &mut |c| together(c)),
        Expr::Pow(b, x) if !x.is_number() => pow(inner_normal(b), together(x)),
        Expr::Add(_) | Expr::Mul(_) | Expr::Pow(..) => rebuild(e, &mut |c| inner_normal(c)),
        _ => e.clone(),
    }
}

fn convert(e: &Expr) -> Option<(Gens, Frac)> {
    if e.any(&|x| x.is_infinite() || matches!(x, Expr::List(_) | Expr::Rel(..) | Expr::Bool(_))) {
        return None;
    }
    let e = inner_normal(e);
    let g = Gens::of(&[&e])?;
    if g.len() > 12 {
        return None;
    }
    let f = g.convert(&e)?;
    Some((g, f))
}

/// One fraction `P/Q` with expanded, coprime numerator and denominator.
pub fn together(e: &Expr) -> Expr {
    match e {
        Expr::List(_) | Expr::Rel(..) => return rebuild(e, &mut |c| together(c)),
        Expr::Num(_) | Expr::Sym(_) | Expr::Float(_) | Expr::Bool(_) => return e.clone(),
        _ => {}
    }
    match convert(e) {
        Some((g, (num, den))) => div(g.expr(&num), g.expr(&den)),
        None => e.clone(),
    }
}

/// Uses `sin^2 + cos^2 = 1` to lower the powers of sines or of cosines, whichever gives the shorter result.
pub fn trigsimp(e: &Expr) -> Expr {
    if !["sin", "cos", "sinh", "cosh"].iter().any(|n| e.has_fn(n)) {
        return e.clone();
    }
    let with_tan = crate::canon::map_bottom_up(e, &mut |x| {
        for (t, s, c) in [("tan", "sin", "cos"), ("tanh", "sinh", "cosh")] {
            if let Some(u) = x.fn_arg(t) {
                return div(
                    crate::funcs::func(s, vec![u.clone()]),
                    crate::funcs::func(c, vec![u.clone()]),
                );
            }
        }
        x
    });
    let mut best = e.clone();
    for source in [e, &with_tan] {
        if source
            .any(&|x| x.is_infinite() || matches!(x, Expr::List(_) | Expr::Rel(..) | Expr::Bool(_)))
        {
            continue;
        }
        let src = inner_normal(source);
        if src.to_string().len() < best.to_string().len() {
            best = src.clone();
        }
        // (argument, hyperbolic)
        let mut args: Vec<(Expr, bool)> = Vec::new();
        src.walk(&mut |x| {
            for (name, hyper) in [
                ("sin", false),
                ("cos", false),
                ("sinh", true),
                ("cosh", true),
            ] {
                if let Some(u) = x.fn_arg(name)
                    && !args.contains(&(u.clone(), hyper))
                {
                    args.push((u.clone(), hyper));
                }
            }
        });
        let names = |hyper: bool| {
            if hyper {
                ("sinh", "cosh")
            } else {
                ("sin", "cos")
            }
        };
        let extras: Vec<Expr> = args
            .iter()
            .flat_map(|(u, hyper)| {
                let (s, c) = names(*hyper);
                [
                    crate::funcs::func(s, vec![u.clone()]),
                    crate::funcs::func(c, vec![u.clone()]),
                ]
            })
            .collect();
        let mut all: Vec<&Expr> = vec![&src];
        all.extend(extras.iter());
        let Some(g) = Gens::of(&all) else { continue };
        if g.len() > 12 {
            continue;
        }
        let Some((num, den)) = g.convert(&src) else {
            continue;
        };
        let n = g.len();
        let pairs: Vec<(usize, usize, bool)> = (0..n)
            .filter_map(|i| {
                let kernel = &g.kernels()[i];
                let (u, hyper) = match (kernel.fn_arg("sin"), kernel.fn_arg("sinh")) {
                    (Some(u), _) => (u, false),
                    (_, Some(u)) => (u, true),
                    _ => return None,
                };
                let j = g.index_of(&crate::funcs::func(names(hyper).1, vec![u.clone()]))?;
                Some((i, j, hyper))
            })
            .collect();
        if pairs.is_empty() {
            continue;
        }
        for lower_sin in [true, false] {
            let (mut a, mut b) = (num.clone(), den.clone());
            for &(s, c, hyper) in &pairs {
                let (hi, lo) = if lower_sin { (s, c) } else { (c, s) };
                // sin^2 = 1 - cos^2, cos^2 = 1 - sin^2; sinh^2 = cosh^2 - 1, cosh^2 = 1 + sinh^2
                let square = Poly::var(n, lo).pow(2);
                let one_minus = match (hyper, lower_sin) {
                    (false, _) => Poly::one(n).sub(&square),
                    (true, true) => square.sub(&Poly::one(n)),
                    (true, false) => Poly::one(n).add(&square),
                };
                let lower = |p: &Poly| {
                    let mut acc = Poly::zero(n);
                    for (d, coef) in p.coeffs(hi).iter().enumerate() {
                        let d = d as u32;
                        acc = acc.add(
                            &coef
                                .mul(&Poly::var(n, hi).pow(d % 2))
                                .mul(&one_minus.pow(d / 2)),
                        );
                    }
                    acc
                };
                a = lower(&a);
                b = lower(&b);
            }
            if b.is_zero() {
                continue;
            }
            let Some((a, b)) = g.normalize((a, b)) else {
                continue;
            };
            let candidate = div(g.expr(&a), g.expr(&b));
            if candidate.to_string().len() < best.to_string().len() {
                best = candidate;
            }
        }
    }
    best
}

/// Numerator and denominator of the normal form.
pub fn fraction(e: &Expr) -> (Expr, Expr) {
    match convert(e) {
        Some((g, (num, den))) => (g.expr(&num), g.expr(&den)),
        None => crate::canon::numer_denom(e),
    }
}

/// Whether the normal form proves the expression zero.
pub fn is_zero(e: &Expr) -> Option<bool> {
    convert(e).map(|(_, (num, _))| num.is_zero())
}

fn factored(g: &Gens, p: &Poly) -> (Rat, Vec<Expr>, bool) {
    let (content, parts, complete) = zfactor::factor(p);
    let factors = parts
        .into_iter()
        .map(|(f, m)| {
            let base = g.expr(&f);
            if m == 1 {
                base
            } else {
                pow(base, Expr::int(i64::from(m)))
            }
        })
        .collect();
    (content, factors, complete)
}

/// Product of irreducible factors over the rationals (numerator and denominator).
pub fn factor(e: &Expr) -> Expr {
    factor_told(e).0
}

/// The same, and whether every factor is known to have no factors of its own: the search
/// for them is given up where a polynomial has too many factors modulo the primes that are
/// tried (as x^120 - 1 has), and it is left whole then.
pub fn factor_told(e: &Expr) -> (Expr, bool) {
    if let Some(n) = e.as_int() {
        return (factor_integer(n), true);
    }
    if let Expr::List(_) | Expr::Rel(..) = e {
        let mut complete = true;
        let parts = rebuild(e, &mut |c| {
            let (part, known) = factor_told(c);
            complete &= known;
            part
        });
        return (parts, complete);
    }
    let Some((g, (num, den))) = convert(e) else {
        return (e.clone(), true);
    };
    let (cn, mut top, above) = factored(&g, &num);
    let (cd, bottom, below) = factored(&g, &den);
    let complete = above && below;
    let content = cn / cd;
    top.extend(bottom.into_iter().map(|f| pow(f, Expr::minus_one())));
    if top.is_empty() {
        return (Expr::rational(content), complete);
    }
    if content == 1 {
        return (mul(top), complete);
    }
    if top.len() == 1 && matches!(top[0], Expr::Add(_)) {
        let held = Expr::call(
            "hold",
            vec![Expr::Mul(
                vec![Expr::rational(content), top.pop().unwrap()].into(),
            )],
        );
        return (held, complete);
    }
    top.insert(0, Expr::rational(content));
    (mul(top), complete)
}

pub fn factor_integer(n: &Int) -> Expr {
    let (mut parts, rest) = crate::ntheory::factor_limited(n, 2);
    parts.extend(rest.map(|c| (c, 1)));
    if n.clone().abs() <= 1 || (parts.len() == 1 && parts[0].1 == 1 && !n.is_negative()) {
        return Expr::integer(n.clone());
    }
    let mut factors: Vec<Expr> = Vec::new();
    if n.is_negative() {
        factors.push(Expr::minus_one());
    }
    for (p, k) in parts {
        let base = if crate::ntheory::is_prime(&p) {
            Expr::integer(p)
        } else {
            Expr::call("composite", vec![Expr::integer(p)])
        };
        factors.push(if k == 1 {
            base
        } else {
            Expr::Pow(base.into(), Expr::int(i64::from(k)).into())
        });
    }
    Expr::call("hold", vec![Expr::Mul(factors.into())])
}

/// Coefficients of `e` as a polynomial in `var` (ascending); `None` when it is not one.
pub fn coefficients(e: &Expr, var: &str) -> Option<Vec<Expr>> {
    let x = Expr::sym(var);
    let e = expand(e);
    let terms: Vec<Expr> = match &e {
        Expr::Add(a) => a.to_vec(),
        other => vec![other.clone()],
    };
    let mut out: Vec<Vec<Expr>> = Vec::new();
    for t in terms {
        let factors: Vec<Expr> = match &t {
            Expr::Mul(a) => a.to_vec(),
            other => vec![other.clone()],
        };
        let (mut deg, mut rest) = (0usize, Vec::new());
        for f in factors {
            let (b, k) = f.base_exp();
            if b == x {
                deg += usize::try_from(k.as_i64().filter(|k| (0..=100_000).contains(k))?).ok()?;
            } else if f.has_sym(var) {
                return None;
            } else {
                rest.push(f);
            }
        }
        if out.len() <= deg {
            out.resize(deg + 1, Vec::new());
        }
        out[deg].push(mul(rest));
    }
    Some(out.into_iter().map(add).collect())
}

/// Polynomials in several expressions over common kernels.
pub fn common(exprs: &[&Expr]) -> Option<(Gens, Vec<Poly>)> {
    let g = Gens::of(exprs)?;
    let mut polys = Vec::new();
    for e in exprs {
        let (num, den) = g.convert(e)?;
        polys.push(num.scale(&den.as_const()?.recip()));
    }
    Some((g, polys))
}

pub fn poly_gcd(a: &Expr, b: &Expr) -> Option<Expr> {
    let (g, p) = common(&[a, b])?;
    Some(g.expr(&p[0].gcd(&p[1])))
}

pub fn poly_lcm(a: &Expr, b: &Expr) -> Option<Expr> {
    let (g, p) = common(&[a, b])?;
    Some(g.expr(&p[0].lcm(&p[1])))
}

/// Quotient and remainder of polynomials in `var` with coefficients in the other kernels.
pub fn div_rem(a: &Expr, b: &Expr, var: &str) -> Option<(Expr, Expr)> {
    let x = Expr::sym(var);
    let (ca, cb) = (coefficients(a, var)?, coefficients(b, var)?);
    let cb: Vec<Expr> = {
        let mut v = cb;
        while v.last().is_some_and(crate::simplify::is_zero) {
            v.pop();
        }
        v
    };
    let lead = cb.last()?.clone();
    let mut r = ca;
    let mut q = vec![Expr::zero(); r.len().saturating_sub(cb.len()) + 1];
    while r.len() >= cb.len() {
        let shift = r.len() - cb.len();
        let c = together(&div(r.last().unwrap().clone(), lead.clone()));
        for (i, y) in cb.iter().enumerate() {
            r[shift + i] = together(&(&r[shift + i] - &(&c * y)));
        }
        q[shift] = c;
        r.pop();
        while r.last().is_some_and(|c| c.is_zero()) {
            r.pop();
        }
        if r.is_empty() {
            break;
        }
    }
    let build = |cs: &[Expr]| {
        add(cs
            .iter()
            .enumerate()
            .map(|(i, c)| mul(vec![c.clone(), pow(x.clone(), Expr::int(i as i64))]))
            .collect())
    };
    Some((build(&q), build(&r)))
}

pub fn resultant(a: &Expr, b: &Expr, var: &str) -> Option<Expr> {
    let (g, p) = common(&[a, b])?;
    let v = g.index_of(&Expr::sym(var))?;
    Some(g.expr(&p[0].resultant(&p[1], v)))
}
