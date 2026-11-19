//! Canonical ("automatic") simplification: every expression is built through
//! `add`, `mul`, `pow` and `func`, which keep one normal form.

use crate::expr::Expr;
use crate::funcs::func;
use crate::num::{Flt, Int, Num, Rat, gaussian_sqrt, int_root};
use crate::numeric;
use rug::ops::Pow as _;
use rug::{Complete, Complex};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Clone)]
enum Coef {
    Exact(Num),
    Approx(Flt),
}

impl Coef {
    fn one() -> Coef {
        Coef::Exact(Num::one())
    }
    fn zero() -> Coef {
        Coef::Exact(Num::zero())
    }
    fn is_zero(&self) -> bool {
        match self {
            Coef::Exact(n) => n.is_zero(),
            Coef::Approx(f) => f.v.is_zero(),
        }
    }
    fn is_one(&self) -> bool {
        matches!(self, Coef::Exact(n) if n.is_one())
    }
    fn approx(&self, like: &Flt) -> Flt {
        match self {
            Coef::Exact(n) => Flt {
                v: n.to_complex(like.v.prec().0),
                digits: like.digits,
            },
            Coef::Approx(f) => f.clone(),
        }
    }
    fn combine(&mut self, o: &Coef, product: bool) {
        let like = match (&*self, o) {
            (Coef::Exact(a), Coef::Exact(b)) => {
                *self = Coef::Exact(if product { a.mul(b) } else { a.add(b) });
                return;
            }
            (Coef::Approx(f), _) | (_, Coef::Approx(f)) => f.clone(),
        };
        let (a, b) = (self.approx(&like), o.approx(&like));
        let prec = a.v.prec().0.min(b.v.prec().0);
        let v = if product {
            Complex::with_val(prec, &a.v * &b.v)
        } else {
            Complex::with_val(prec, &a.v + &b.v)
        };
        *self = Coef::Approx(Flt {
            v,
            digits: a.digits.min(b.digits),
        });
    }
    fn add(&mut self, o: &Coef) {
        self.combine(o, false);
    }
    fn mul(&mut self, o: &Coef) {
        self.combine(o, true);
    }
    fn into_expr(self) -> Expr {
        match self {
            Coef::Exact(n) => Expr::num(n),
            Coef::Approx(f) => Expr::float(f),
        }
    }
}

fn rest_of(a: &[Expr]) -> Expr {
    if a.len() == 2 {
        a[1].clone()
    } else {
        Expr::Mul(a[1..].into())
    }
}

fn split_coef(t: &Expr) -> (Coef, Option<Expr>) {
    match t {
        Expr::Num(n) => (Coef::Exact((**n).clone()), None),
        Expr::Float(f) => (Coef::Approx((**f).clone()), None),
        Expr::Mul(a) => match &a[0] {
            Expr::Num(n) => (Coef::Exact((**n).clone()), Some(rest_of(a))),
            Expr::Float(f) => (Coef::Approx((**f).clone()), Some(rest_of(a))),
            _ => (Coef::one(), Some(t.clone())),
        },
        _ => (Coef::one(), Some(t.clone())),
    }
}

/// `term = coefficient * rest` with an exact coefficient (`rest` is 1 for a number).
pub fn coeff_and_rest(t: &Expr) -> (Num, Expr) {
    match t {
        Expr::Num(n) => ((**n).clone(), Expr::one()),
        Expr::Mul(a) => match &a[0] {
            Expr::Num(n) => ((**n).clone(), rest_of(a)),
            _ => (Num::one(), t.clone()),
        },
        _ => (Num::one(), t.clone()),
    }
}

fn with_coef(c: Coef, rest: Expr) -> Expr {
    if c.is_one() {
        return rest;
    }
    let mut v = vec![c.into_expr()];
    match rest {
        Expr::Mul(a) => v.extend(a.iter().cloned()),
        other => v.push(other),
    }
    Expr::Mul(v.into())
}

fn flatten_add(t: Expr, out: &mut Vec<Expr>) {
    match t {
        Expr::Add(a) => out.extend(a.iter().cloned()),
        other => out.push(other),
    }
}

pub fn add(terms: Vec<Expr>) -> Expr {
    if terms.iter().any(crate::quat::is_quat) {
        return crate::quat::sum(&terms);
    }
    let mut flat = Vec::with_capacity(terms.len());
    for t in terms {
        flatten_add(t, &mut flat);
    }
    match flat.len() {
        0 => return Expr::zero(),
        1 => return flat.pop().unwrap(),
        _ => {}
    }
    let work = flat;
    if work.iter().any(Expr::is_infinite) {
        return add_infinite(work);
    }
    let mut constant = Coef::zero();
    let mut map: BTreeMap<Expr, Coef> = BTreeMap::new();
    for t in &work {
        let (c, rest) = split_coef(t);
        match rest {
            None => constant.add(&c),
            Some(r) => match map.get_mut(&r) {
                Some(x) => x.add(&c),
                None => {
                    map.insert(r, c);
                }
            },
        }
    }
    if let Coef::Approx(f) = constant.clone() {
        let keys: Vec<Expr> = map.keys().filter(|k| k.is_const()).cloned().collect();
        for k in keys {
            if let Ok(v) = numeric::evalf(&k, f.v.prec().0) {
                let mut c = map.remove(&k).unwrap();
                c.mul(&Coef::Approx(Flt {
                    v,
                    digits: f.digits,
                }));
                constant.add(&c);
            }
        }
    }
    let mut out = Vec::with_capacity(map.len() + 1);
    if !constant.is_zero() {
        out.push(constant.into_expr());
    }
    for (rest, c) in map {
        if !c.is_zero() {
            out.push(with_coef(c, rest));
        }
    }
    match out.len() {
        0 => Expr::zero(),
        1 => out.pop().unwrap(),
        _ => Expr::Add(out.into()),
    }
}

/// Direction of a directed infinity `c*inf`.
fn inf_direction(t: &Expr) -> Option<Num> {
    match t {
        Expr::Sym(_) if t.is_inf() => Some(Num::one()),
        Expr::Mul(a) if a.len() == 2 && a[1].is_inf() => a[0].as_num().cloned(),
        _ => None,
    }
}

fn directed_inf(d: &Num) -> Expr {
    if d.is_real() {
        return if d.is_positive() {
            Expr::inf()
        } else {
            Expr::neg_inf()
        };
    }
    let scale = if d.re.clone().abs() > d.im.clone().abs() {
        d.re.clone().abs()
    } else {
        d.im.clone().abs()
    };
    let unit = Num::new((&d.re / &scale).complete(), (&d.im / &scale).complete());
    Expr::Mul(vec![Expr::num(unit), Expr::inf()].into())
}

fn add_infinite(terms: Vec<Expr>) -> Expr {
    let inf_terms: Vec<&Expr> = terms.iter().filter(|t| t.is_infinite()).collect();
    if inf_terms.iter().any(|t| t.any(&|e| e.is_undefined())) {
        return Expr::undefined();
    }
    if inf_terms.iter().any(|t| t.any(&|e| e.is_zoo())) {
        return if inf_terms.len() > 1 {
            Expr::undefined()
        } else {
            Expr::zoo()
        };
    }
    let dirs: Vec<Option<Num>> = inf_terms.iter().map(|t| inf_direction(t)).collect();
    if dirs.iter().any(Option::is_none) {
        return if inf_terms.len() == 1 {
            inf_terms[0].clone()
        } else {
            Expr::undefined()
        };
    }
    let first = dirs[0].clone().unwrap();
    for d in dirs.iter().flatten() {
        match d.div(&first) {
            Some(q) if q.is_positive() => {}
            _ => return Expr::undefined(),
        }
    }
    directed_inf(&first)
}

pub fn mul(factors: Vec<Expr>) -> Expr {
    if factors.iter().any(crate::quat::is_quat) {
        return crate::quat::product(&factors);
    }
    let mut coef = Coef::one();
    let mut pows: BTreeMap<Expr, Vec<Expr>> = BTreeMap::new();
    let mut stack = factors;
    for _ in 0..64 {
        while let Some(f) = stack.pop() {
            match f {
                Expr::Mul(a) => stack.extend(a.iter().cloned()),
                Expr::Num(n) => coef.mul(&Coef::Exact((*n).clone())),
                Expr::Float(x) => coef.mul(&Coef::Approx((*x).clone())),
                Expr::Pow(b, e) => pows.entry((*b).clone()).or_default().push((*e).clone()),
                other => pows.entry(other).or_default().push(Expr::one()),
            }
        }
        let mut merged = BTreeMap::new();
        for (b, exps) in std::mem::take(&mut pows) {
            if exps.len() == 1 {
                merged.insert(b, exps);
            } else {
                stack.push(pow(b, add(exps)));
            }
        }
        pows = merged;
        if stack.is_empty() {
            let rads: Vec<Expr> = pows
                .iter()
                .filter(|(b, e)| is_radical(b, &e[0]))
                .map(|(b, _)| b.clone())
                .collect();
            if rads.len() < 2 {
                break;
            }
            let mut primes = BTreeMap::new();
            for k in rads {
                let e = pows.remove(&k).unwrap();
                prime_powers(&mut primes, k.as_int().unwrap(), e[0].as_rat().unwrap());
            }
            let (c, parts) = radicals_from_primes(primes);
            coef.mul(&Coef::Exact(Num::real(c)));
            for (m, f) in parts {
                pows.insert(Expr::integer(m), vec![Expr::rational(f)]);
            }
            break;
        }
    }
    if let Coef::Approx(f) = coef.clone() {
        let keys: Vec<Expr> = pows
            .keys()
            .filter(|k| k.is_const() && !k.is_infinite())
            .cloned()
            .collect();
        for k in keys {
            let e = pows[&k][0].clone();
            if !e.is_const() {
                continue;
            }
            let factor = if e.is_one() {
                k.clone()
            } else {
                Expr::Pow(Rc::new(k.clone()), Rc::new(e))
            };
            if let Ok(v) = numeric::evalf(&factor, f.v.prec().0) {
                pows.remove(&k);
                coef.mul(&Coef::Approx(Flt {
                    v,
                    digits: f.digits,
                }));
            }
        }
    }
    if pows.keys().any(Expr::is_infinite) {
        return mul_infinite(coef, pows);
    }
    if coef.is_zero() {
        return Expr::zero();
    }
    let mut out: Vec<Expr> = pows
        .into_iter()
        .map(|(b, mut e)| {
            let e = e.pop().unwrap();
            if e.is_one() {
                b
            } else {
                Expr::Pow(Rc::new(b), Rc::new(e))
            }
        })
        .collect();
    out.sort();
    if coef.is_one() {
        return match out.len() {
            0 => Expr::one(),
            1 => out.pop().unwrap(),
            _ => Expr::Mul(out.into()),
        };
    }
    if out.is_empty() {
        return coef.into_expr();
    }
    if out.len() == 1
        && let Expr::Add(terms) = &out[0]
    {
        let c = coef.into_expr();
        return add(terms
            .iter()
            .map(|t| mul(vec![c.clone(), t.clone()]))
            .collect());
    }
    let mut v = Vec::with_capacity(out.len() + 1);
    v.push(coef.into_expr());
    v.extend(out);
    Expr::Mul(v.into())
}

fn mul_infinite(coef: Coef, pows: BTreeMap<Expr, Vec<Expr>>) -> Expr {
    if pows.keys().any(Expr::is_undefined) {
        return Expr::undefined();
    }
    if coef.is_zero() {
        return Expr::undefined();
    }
    if pows.keys().any(Expr::is_zoo) {
        return Expr::zoo();
    }
    let mut dir = match &coef {
        Coef::Exact(n) => n.clone(),
        Coef::Approx(f) => {
            let part = |x: &rug::Float| Rat::from(x.cmp0().map_or(0, |o| o as i32));
            Num::new(part(f.v.real()), part(f.v.imag()))
        }
    };
    let mut kept = Vec::new();
    for (b, e) in pows {
        if b.is_inf() {
            continue;
        }
        let factor = if e[0].is_one() {
            b
        } else {
            Expr::Pow(Rc::new(b), Rc::new(e[0].clone()))
        };
        match numeric::sign(&factor) {
            Some(s) if s > 0 => {}
            Some(s) if s < 0 => dir = dir.neg(),
            Some(_) => return Expr::undefined(),
            None => kept.push(factor),
        }
    }
    if kept.is_empty() {
        return directed_inf(&dir);
    }
    kept.sort();
    let mut v = Vec::new();
    if !dir.is_one() {
        v.push(Expr::num(dir));
    }
    v.extend(kept);
    v.push(Expr::inf());
    Expr::Mul(v.into())
}

fn is_radical(b: &Expr, e: &Expr) -> bool {
    b.as_int().is_some_and(|n| *n >= 2) && e.as_rat().is_some_and(|r| !r.is_integer())
}

/// Primes with their exponents.
pub type PrimePowers = Rc<Vec<(Int, u32)>>;

thread_local! {
    static SMALL_PRIMES: Vec<u32> = {
        let n = 1usize << 16;
        let mut sieve = vec![true; n];
        let mut primes = Vec::new();
        for i in 2..n {
            if sieve[i] {
                primes.push(i as u32);
                let mut j = i * i;
                while j < n {
                    sieve[j] = false;
                    j += i;
                }
            }
        }
        primes
    };
    static FACTOR_CACHE: RefCell<BTreeMap<Int, PrimePowers>> = const { RefCell::new(BTreeMap::new()) };
}

/// Prime factorization used by the canonical forms; a part that cannot be split
/// cheaply is returned as one factor.
pub fn factor_int(n: &Int) -> PrimePowers {
    if let Some(hit) = FACTOR_CACHE.with(|c| c.borrow().get(n).cloned()) {
        return hit;
    }
    let mut out: Vec<(Int, u32)> = Vec::new();
    let mut m = n.clone().abs();
    if m > 1 {
        SMALL_PRIMES.with(|primes| {
            for &p in primes {
                if m == 1
                    || m.significant_bits() <= 32
                        && u64::from(p) * u64::from(p) > m.to_u64().unwrap_or(u64::MAX)
                {
                    break;
                }
                if m.is_divisible_u(p) {
                    let (rest, k) = m.clone().remove_factor(&Int::from(p));
                    m = rest;
                    out.push((Int::from(p), k));
                }
            }
        });
        if m > 1 {
            let mut k = 1;
            if m.is_perfect_power() {
                for e in (2..=m.significant_bits()).rev() {
                    if let Some((root, true)) = int_root(&m, e) {
                        m = root;
                        k = e;
                        break;
                    }
                }
            }
            out.push((m, k));
        }
    }
    let out = Rc::new(out);
    FACTOR_CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() > 4096 {
            c.clear();
        }
        c.insert(n.clone(), out.clone());
    });
    out
}

fn prime_powers(primes: &mut BTreeMap<Int, Rat>, n: &Int, e: &Rat) {
    for (p, k) in factor_int(n).iter() {
        *primes.entry(p.clone()).or_default() += (e * *k).complete();
    }
}

fn radicals_from_primes(primes: BTreeMap<Int, Rat>) -> (Rat, Vec<(Int, Rat)>) {
    let mut coef = Rat::from(1);
    let mut groups: BTreeMap<Rat, Int> = BTreeMap::new();
    for (p, e) in primes {
        let whole = e.clone().floor();
        if let Some(k) = whole.numer().to_i32()
            && k != 0
        {
            coef *= Rat::from(p.clone()).pow(k);
        }
        let frac = e - whole;
        if !frac.is_zero() {
            *groups.entry(frac).or_insert_with(|| Int::from(1)) *= &p;
        }
    }
    (coef, groups.into_iter().map(|(f, m)| (m, f)).collect())
}

fn radical_expr(coef: Rat, parts: Vec<(Int, Rat)>) -> Expr {
    let mut v: Vec<Expr> = parts
        .into_iter()
        .map(|(m, f)| Expr::Pow(Rc::new(Expr::integer(m)), Rc::new(Expr::rational(f))))
        .collect();
    v.sort();
    if coef != 1 {
        v.insert(0, Expr::rational(coef));
    }
    match v.len() {
        0 => Expr::one(),
        1 => v.pop().unwrap(),
        _ => Expr::Mul(v.into()),
    }
}

/// `b^r` for a positive rational `b` and a rational `r`.
fn rational_radical(b: &Rat, r: &Rat) -> Expr {
    let mut primes = BTreeMap::new();
    prime_powers(&mut primes, b.numer(), r);
    prime_powers(&mut primes, b.denom(), &(-r).complete());
    let (coef, parts) = radicals_from_primes(primes);
    radical_expr(coef, parts)
}

/// Principal value of `(-1)^r`.
pub fn minus_one_pow(r: &Rat) -> Expr {
    if crate::assume::real_mode() && r.denom().is_odd() {
        // the real odd root
        return Expr::int(if r.numer().is_odd() { -1 } else { 1 });
    }
    let two = Rat::from(2);
    let mut t = r - (r / &two).complete().floor() * &two;
    if t > 1 {
        t -= &two;
    }
    if t.is_zero() {
        return Expr::one();
    }
    if t == 1 {
        return Expr::minus_one();
    }
    if *t.denom() == 2 {
        return Expr::num(Num::new(
            Rat::new(),
            Rat::from(if t.is_positive() { 1 } else { -1 }),
        ));
    }
    let atom = |t: Rat| Expr::Pow(Rc::new(Expr::minus_one()), Rc::new(Expr::rational(t)));
    if t.is_positive() {
        atom(t)
    } else {
        Expr::Mul(vec![Expr::minus_one(), atom(t + 1u32)].into())
    }
}

const MAX_POWER_BITS: u64 = 1 << 23;

fn num_pow(x: &Num, y: &Num) -> Expr {
    let raw = || Expr::Pow(Rc::new(Expr::num(x.clone())), Rc::new(Expr::num(y.clone())));
    if x.is_zero() {
        return match y.re.cmp0() {
            std::cmp::Ordering::Greater => Expr::zero(),
            std::cmp::Ordering::Less => Expr::zoo(),
            std::cmp::Ordering::Equal => Expr::undefined(),
        };
    }
    if x.is_one() {
        return Expr::one();
    }
    if let Some(n) = y.as_int() {
        return match n.to_i64() {
            Some(k) if u64::from(x.bits()).saturating_mul(k.unsigned_abs()) <= MAX_POWER_BITS => {
                x.powi(k).map_or_else(Expr::zoo, Expr::num)
            }
            _ if x.is_minus_one() => Expr::int(if n.is_even() { 1 } else { -1 }),
            _ => raw(),
        };
    }
    if !y.is_real() {
        let unit = |half: Rat| {
            let z = Num::new((&y.re * &half).complete(), (&y.im * &half).complete());
            mul(vec![
                minus_one_pow(&z.re),
                pow(Expr::e(), mul(vec![Expr::rational(-z.im), Expr::pi()])),
            ])
        };
        if x.is_minus_one() {
            return unit(Rat::from(1));
        }
        if x.is_imag() && x.im.clone().abs() == 1 {
            return unit(Rat::from((if x.im.is_positive() { 1 } else { -1 }, 2)));
        }
        return raw();
    }
    let r = &y.re;
    if x.is_real() {
        if x.re.is_positive() {
            return rational_radical(&x.re, r);
        }
        return mul(vec![
            minus_one_pow(r),
            rational_radical(&(-&x.re).complete(), r),
        ]);
    }
    if x.re.is_zero() {
        let half = (r / 2u32).complete();
        return if x.im.is_positive() {
            mul(vec![minus_one_pow(&half), rational_radical(&x.im, r)])
        } else {
            mul(vec![
                minus_one_pow(&(-half)),
                rational_radical(&(-&x.im).complete(), r),
            ])
        };
    }
    if *r.denom() == 2
        && let Some(s) = gaussian_sqrt(x)
        && let Some(k) = r.numer().to_i64()
        && let Some(v) = s.powi(k)
    {
        return Expr::num(v);
    }
    raw()
}

fn float_pow(b: &Expr, e: &Expr) -> Option<Expr> {
    let like = match (b, e) {
        (Expr::Float(f), _) | (_, Expr::Float(f)) => f.clone(),
        _ => return None,
    };
    if !b.is_const() || !e.is_const() {
        return None;
    }
    let prec = like.v.prec().0;
    let (x, y) = (numeric::evalf(b, prec).ok()?, numeric::evalf(e, prec).ok()?);
    let v = numeric::cpow(&x, &y)?;
    Some(Expr::float(Flt {
        v,
        digits: like.digits,
    }))
}

fn int_perfect_power(n: &Int) -> Option<(Int, u32)> {
    if *n < 4 || !n.is_perfect_power() {
        return None;
    }
    (2..=n.significant_bits())
        .rev()
        .find_map(|k| match int_root(n, k) {
            Some((root, true)) => Some((root, k)),
            _ => None,
        })
}

pub fn pow(b: Expr, e: Expr) -> Expr {
    if b.is_undefined() || e.is_undefined() {
        return Expr::undefined();
    }
    if (crate::quat::is_quat(&b) || crate::quat::is_quat(&e))
        && let Some(v) = crate::quat::power(&b, &e)
    {
        return v;
    }
    if let Expr::Num(n) = &e {
        if n.is_zero() {
            return if b.is_zero() || b.is_infinite() {
                Expr::undefined()
            } else {
                Expr::one()
            };
        }
        if n.is_one() {
            return b;
        }
    }
    if b.is_infinite() || e.is_infinite() {
        return pow_infinite(b, e);
    }
    match (&b, &e) {
        (Expr::Num(x), Expr::Num(y)) => return num_pow(x, y),
        (Expr::Float(_), _) | (_, Expr::Float(_)) => {
            if let Some(r) = float_pow(&b, &e) {
                return r;
            }
        }
        _ => {}
    }
    if b.is_zero() && !e.is_number() && crate::assume::is_positive(&e) {
        return Expr::zero();
    }
    if b.is_minus_one() && !e.is_number() && crate::assume::is_integer(&e) {
        // only the parity of a whole exponent matters
        let terms: Vec<Expr> = match &e {
            Expr::Add(a) => a.to_vec(),
            other => vec![other.clone()],
        };
        let mut odd = Vec::new();
        let mut flip = false;
        for t in terms {
            let (c, rest) = coeff_and_rest(&t);
            match c.as_int() {
                Some(k) if k.is_even() => {}
                Some(_) if rest.is_one() => flip = !flip,
                Some(_) => odd.push(rest),
                None => odd.push(t),
            }
        }
        let reduced = add(odd);
        let sign = if flip { Expr::minus_one() } else { Expr::one() };
        if reduced != e {
            return if reduced.is_zero() {
                sign
            } else {
                mul(vec![sign, pow(Expr::minus_one(), reduced)])
            };
        }
    }
    if let Expr::Num(x) = &b {
        if x.is_one() {
            return Expr::one();
        }
        if x.is_positive() {
            if let Some(n) = x.as_int() {
                if let Some((root, k)) = int_perfect_power(n) {
                    return pow(Expr::integer(root), mul(vec![Expr::int(i64::from(k)), e]));
                }
            } else if *x.re.numer() == 1 {
                return pow(
                    Expr::integer(x.re.denom().clone()),
                    mul(vec![Expr::minus_one(), e]),
                );
            }
        }
    }
    // |x|^(2k) = x^(2k) for real x
    if let Some(inner) = b.fn_arg("abs")
        && e.as_int().is_some_and(|k| k.is_even())
        && crate::assume::is_real(inner)
    {
        return pow(inner.clone(), e);
    }
    match &b {
        Expr::Pow(b2, e2) => {
            if e.as_int().is_some() || e2.as_rat().is_some_and(|r| *r > -1 && *r <= 1) {
                return pow((**b2).clone(), mul(vec![(**e2).clone(), e]));
            }
            // with real odd roots (x^m)^(p/q) = x^(m p/q) for odd q
            if crate::assume::real_mode()
                && e2.as_int().is_some()
                && e.as_rat().is_some_and(|r| r.denom().is_odd())
                && crate::assume::is_real(b2)
            {
                return pow((**b2).clone(), mul(vec![(**e2).clone(), e]));
            }
            // (p^a)^e = p^(a e) for p > 0 and real a; (x^(2m))^r = |x|^(2m r) for real x
            if crate::assume::is_positive(b2) && crate::assume::is_real(e2) {
                return pow((**b2).clone(), mul(vec![(**e2).clone(), e]));
            }
            if e2.as_int().is_some_and(|k| k.is_even())
                && e.as_rat().is_some()
                && crate::assume::is_real(b2)
            {
                return pow(
                    crate::funcs::func("abs", vec![(**b2).clone()]),
                    mul(vec![(**e2).clone(), e]),
                );
            }
        }
        Expr::Mul(args) => {
            if e.as_int().is_some() {
                return mul(args.iter().map(|a| pow(a.clone(), e.clone())).collect());
            }
            if crate::assume::real_mode()
                && args[0].is_minus_one()
                && let Some(r) = e.as_rat()
                && r.denom().is_odd()
            {
                return mul(vec![minus_one_pow(r), pow(rest_of(args), e)]);
            }
            if let Expr::Num(c) = &args[0]
                && c.is_real()
                && !c.is_minus_one()
            {
                let rest = rest_of(args);
                return if c.is_positive() {
                    mul(vec![pow(Expr::num((**c).clone()), e.clone()), pow(rest, e)])
                } else {
                    mul(vec![
                        pow(Expr::num(c.neg()), e.clone()),
                        pow(mul(vec![Expr::minus_one(), rest]), e),
                    ])
                };
            }
            // positive factors come out of any power
            let (positive, other): (Vec<Expr>, Vec<Expr>) = args
                .iter()
                .cloned()
                .partition(|a| !a.is_number() && crate::assume::is_positive(a));
            if !positive.is_empty() && (!other.is_empty() || positive.len() > 1) {
                let mut parts: Vec<Expr> =
                    positive.into_iter().map(|p| pow(p, e.clone())).collect();
                if !other.is_empty() {
                    parts.push(pow(mul(other), e));
                }
                return mul(parts);
            }
        }
        Expr::Sym(s) if &**s == "e" => {
            if let Some(r) = exp_rules(&e) {
                return r;
            }
        }
        _ => {}
    }
    Expr::Pow(Rc::new(b), Rc::new(e))
}

/// `q` when `x = q*i*pi` with a rational `q`.
pub fn i_pi_multiple(x: &Expr) -> Option<Rat> {
    match x {
        Expr::Mul(f) if f.len() == 2 && f[1].is_sym("pi") => f[0]
            .as_num()
            .filter(|n| n.re.is_zero())
            .map(|n| n.im.clone()),
        _ => None,
    }
}

fn exp_term(t: &Expr) -> Option<Expr> {
    if let Some(u) = t.fn_arg("ln") {
        return Some(u.clone());
    }
    if let Some(q) = i_pi_multiple(t) {
        return Some(minus_one_pow(&q));
    }
    if let Expr::Mul(f) = t
        && let Some(pos) = f.iter().position(|x| x.fn_arg("ln").is_some())
    {
        let u = f[pos].fn_arg("ln").unwrap().clone();
        let rest: Vec<Expr> = f
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != pos)
            .map(|(_, x)| x.clone())
            .collect();
        return Some(pow(u, mul(rest)));
    }
    None
}

fn exp_rules(x: &Expr) -> Option<Expr> {
    if let Expr::Add(terms) = x {
        let (mut factors, mut rest) = (Vec::new(), Vec::new());
        for t in terms.iter() {
            match exp_term(t) {
                Some(f) => factors.push(f),
                None => rest.push(t.clone()),
            }
        }
        if factors.is_empty() {
            return None;
        }
        factors.push(pow(Expr::e(), add(rest)));
        return Some(mul(factors));
    }
    exp_term(x)
}

fn pow_infinite(b: Expr, e: Expr) -> Expr {
    let raw = |b: Expr, e: Expr| Expr::Pow(Rc::new(b), Rc::new(e));
    if b.is_zoo() || e.is_zoo() {
        return match e.as_rat() {
            Some(r) if r.is_positive() => Expr::zoo(),
            Some(_) => Expr::zero(),
            None => Expr::undefined(),
        };
    }
    if let Some(r) = e.as_rat() {
        if b.is_inf() {
            return if r.is_positive() {
                Expr::inf()
            } else {
                Expr::zero()
            };
        }
        if b.is_neg_inf() {
            return if r.is_negative() {
                Expr::zero()
            } else {
                mul(vec![minus_one_pow(r), Expr::inf()])
            };
        }
        return raw(b, e);
    }
    let up = if e.is_inf() {
        true
    } else if e.is_neg_inf() {
        false
    } else if b.is_inf() {
        return match numeric::sign(&e) {
            Some(s) if s > 0 => Expr::inf(),
            Some(s) if s < 0 => Expr::zero(),
            _ => raw(b, e),
        };
    } else {
        return raw(b, e);
    };
    if b.is_inf() {
        return if up { Expr::inf() } else { Expr::zero() };
    }
    if b.is_infinite() {
        return if up { Expr::zoo() } else { Expr::zero() };
    }
    let (above, below) = (
        numeric::sign(&add(vec![b.clone(), Expr::minus_one()])),
        numeric::sign(&add(vec![b.clone(), Expr::one()])),
    );
    match (above, below) {
        (Some(s), _) if s > 0 => {
            if up {
                Expr::inf()
            } else {
                Expr::zero()
            }
        }
        (Some(s), Some(t)) if s < 0 && t > 0 => match (up, numeric::sign(&b)) {
            (true, _) => Expr::zero(),
            (false, Some(p)) if p > 0 => Expr::inf(),
            (false, Some(0)) => Expr::zoo(),
            _ => Expr::undefined(),
        },
        (Some(_), Some(_)) => Expr::undefined(),
        _ => raw(b, e),
    }
}

pub fn neg(e: Expr) -> Expr {
    mul(vec![Expr::minus_one(), e])
}
pub fn sub(a: Expr, b: Expr) -> Expr {
    add(vec![a, neg(b)])
}
pub fn inv(e: Expr) -> Expr {
    pow(e, Expr::minus_one())
}
pub fn div(a: Expr, b: Expr) -> Expr {
    mul(vec![a, inv(b)])
}
pub fn sqrt(e: Expr) -> Expr {
    pow(e, Expr::half())
}
pub fn square(e: Expr) -> Expr {
    pow(e, Expr::int(2))
}
pub fn powi(e: Expr, n: i64) -> Expr {
    pow(e, Expr::int(n))
}
pub fn exp(e: Expr) -> Expr {
    pow(Expr::e(), e)
}
pub fn ln(e: Expr) -> Expr {
    func("ln", vec![e])
}
pub fn f1(name: &str, a: Expr) -> Expr {
    func(name, vec![a])
}

/// Rebuilds a node from transformed children through the canonical constructors.
pub fn rebuild(e: &Expr, f: &mut dyn FnMut(&Expr) -> Expr) -> Expr {
    match e {
        Expr::Add(a) => add(a.iter().map(&mut *f).collect()),
        Expr::Mul(a) => mul(a.iter().map(&mut *f).collect()),
        Expr::Pow(b, x) => {
            let nb = f(b);
            pow(nb, f(x))
        }
        Expr::Fn(n, a) => func(n, a.iter().map(&mut *f).collect()),
        Expr::List(a) => Expr::list(a.iter().map(&mut *f).collect()),
        Expr::Rel(op, a, b) => {
            let na = f(a);
            Expr::rel(*op, na, f(b))
        }
        _ => e.clone(),
    }
}

pub fn subst(e: &Expr, from: &Expr, to: &Expr) -> Expr {
    if e == from {
        return to.clone();
    }
    match e {
        Expr::Num(_) | Expr::Float(_) | Expr::Sym(_) | Expr::Bool(_) => e.clone(),
        _ if !e.has(from) => e.clone(),
        _ => rebuild(e, &mut |c| subst(c, from, to)),
    }
}

pub fn subst_sym(e: &Expr, name: &str, to: &Expr) -> Expr {
    subst(e, &Expr::sym(name), to)
}

/// Replaces several symbols at once.
pub fn subst_all(e: &Expr, pairs: &[(String, Expr)]) -> Expr {
    match e {
        Expr::Sym(s) => pairs
            .iter()
            .find(|(name, _)| **name == **s)
            .map_or_else(|| e.clone(), |(_, v)| v.clone()),
        Expr::Num(_) | Expr::Float(_) | Expr::Bool(_) => e.clone(),
        _ => rebuild(e, &mut |c| subst_all(c, pairs)),
    }
}

/// Applies `f` bottom-up to every node.
pub fn map_bottom_up(e: &Expr, f: &mut dyn FnMut(Expr) -> Expr) -> Expr {
    let r = rebuild(e, &mut |c| map_bottom_up(c, f));
    f(r)
}

macro_rules! binop {
    ($tr:ident, $m:ident, $f:expr) => {
        impl std::ops::$tr<Expr> for Expr {
            type Output = Expr;
            fn $m(self, o: Expr) -> Expr {
                $f(self, o)
            }
        }
        impl std::ops::$tr<&Expr> for Expr {
            type Output = Expr;
            fn $m(self, o: &Expr) -> Expr {
                $f(self, o.clone())
            }
        }
        impl std::ops::$tr<Expr> for &Expr {
            type Output = Expr;
            fn $m(self, o: Expr) -> Expr {
                $f(self.clone(), o)
            }
        }
        impl std::ops::$tr<&Expr> for &Expr {
            type Output = Expr;
            fn $m(self, o: &Expr) -> Expr {
                $f(self.clone(), o.clone())
            }
        }
        impl std::ops::$tr<i64> for Expr {
            type Output = Expr;
            fn $m(self, o: i64) -> Expr {
                $f(self, Expr::int(o))
            }
        }
        impl std::ops::$tr<i64> for &Expr {
            type Output = Expr;
            fn $m(self, o: i64) -> Expr {
                $f(self.clone(), Expr::int(o))
            }
        }
    };
}
binop!(Add, add, |a, b| add(vec![a, b]));
binop!(Sub, sub, sub);
binop!(Mul, mul, |a, b| mul(vec![a, b]));
binop!(Div, div, div);

impl std::ops::Neg for Expr {
    type Output = Expr;
    fn neg(self) -> Expr {
        neg(self)
    }
}
impl std::ops::Neg for &Expr {
    type Output = Expr;
    fn neg(self) -> Expr {
        neg(self.clone())
    }
}

/// Numerator and denominator of a product (negative powers and rational coefficients go below).
pub fn numer_denom(e: &Expr) -> (Expr, Expr) {
    match e {
        Expr::Num(n) => {
            let d = n.re.denom().clone().lcm(n.im.denom());
            let scaled = n.mul(&Num::from_int(d.clone()));
            (Expr::num(scaled), Expr::integer(d))
        }
        Expr::Pow(b, x) => {
            let (c, _) = coeff_and_rest(x);
            if c.is_real() && c.is_negative() {
                (Expr::one(), pow((**b).clone(), neg((**x).clone())))
            } else {
                (e.clone(), Expr::one())
            }
        }
        Expr::Mul(a) => {
            let (mut n, mut d) = (Vec::new(), Vec::new());
            for f in a.iter() {
                let (x, y) = numer_denom(f);
                n.push(x);
                d.push(y);
            }
            (mul(n), mul(d))
        }
        _ => (e.clone(), Expr::one()),
    }
}
