//! Sparse multivariate polynomials over the rationals.

use crate::num::{Int, Rat};
use rug::Complete;
use std::collections::BTreeMap;

pub type Mono = Vec<u32>;

/// Monomials are compared lexicographically, variable 0 first: the last term is the leading one.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Poly {
    pub n: usize,
    pub t: BTreeMap<Mono, Rat>,
}

/// The coefficients by the power of `v`, modulo the prime `p`, with the values of `point` for
/// the other variables; `None` when a denominator or the leading coefficient vanishes there.
fn image(f: &Poly, v: usize, point: &[u64], p: u64) -> Option<Vec<u64>> {
    let residue = |n: &Int| n.mod_u(p as u32) as u64;
    let mut out = vec![0u64; f.deg(v) as usize + 1];
    for (m, c) in &f.t {
        let den = residue(c.denom());
        if den == 0 {
            return None;
        }
        let mut term = residue(c.numer()) * power_mod(den, p - 2, p) % p;
        for (k, e) in m.iter().enumerate() {
            if k != v && *e > 0 {
                term = term * power_mod(point[k] % p, u64::from(*e), p) % p;
            }
        }
        let slot = &mut out[m[v] as usize];
        *slot = (*slot + term) % p;
    }
    (out.last() != Some(&0)).then_some(out)
}

fn power_mod(mut base: u64, mut e: u64, p: u64) -> u64 {
    let mut out = 1;
    while e > 0 {
        if e & 1 == 1 {
            out = out * base % p;
        }
        base = base * base % p;
        e >>= 1;
    }
    out
}

/// The degree of the greatest common divisor of two polynomials modulo the prime `p`.
fn gcd_degree(mut a: Vec<u64>, mut b: Vec<u64>, p: u64) -> usize {
    let trim = |x: &mut Vec<u64>| {
        while x.last() == Some(&0) {
            x.pop();
        }
    };
    trim(&mut a);
    trim(&mut b);
    while !b.is_empty() {
        let inv = power_mod(*b.last().unwrap(), p - 2, p);
        while a.len() >= b.len() {
            let q = a.last().unwrap() * inv % p;
            let k = a.len() - b.len();
            for (i, c) in b.iter().enumerate() {
                a[k + i] = (a[k + i] + p - q * c % p) % p;
            }
            trim(&mut a);
        }
        std::mem::swap(&mut a, &mut b);
    }
    a.len().saturating_sub(1)
}

/// Whether images of `a` and `b` modulo a prime, with whole numbers for the variables other
/// than `v`, share no factor: then `a` and `b` share no factor that has `v` in it.
fn coprime_image(a: &Poly, b: &Poly, v: usize) -> bool {
    const PRIMES: [u64; 3] = [2_147_483_647, 2_147_483_629, 2_147_483_587];
    PRIMES.iter().enumerate().any(|(round, p)| {
        let point: Vec<u64> = (0..a.n as u64)
            .map(|k| 1_000_003 + 7919 * k + 104_729 * round as u64)
            .collect();
        match (image(a, v, &point, *p), image(b, v, &point, *p)) {
            (Some(x), Some(y)) => gcd_degree(x, y, *p) == 0,
            _ => false,
        }
    })
}

impl Poly {
    pub fn zero(n: usize) -> Poly {
        Poly {
            n,
            t: BTreeMap::new(),
        }
    }
    pub fn constant(n: usize, c: Rat) -> Poly {
        let mut p = Poly::zero(n);
        p.put(vec![0; n], c);
        p
    }
    pub fn int(n: usize, k: i64) -> Poly {
        Poly::constant(n, Rat::from(k))
    }
    pub fn one(n: usize) -> Poly {
        Poly::int(n, 1)
    }
    pub fn var(n: usize, i: usize) -> Poly {
        let mut m = vec![0; n];
        m[i] = 1;
        Poly::term(n, m, Rat::from(1))
    }
    pub fn term(n: usize, m: Mono, c: Rat) -> Poly {
        let mut p = Poly::zero(n);
        p.put(m, c);
        p
    }
    fn put(&mut self, m: Mono, c: Rat) {
        if c.is_zero() {
            return;
        }
        match self.t.get_mut(&m) {
            Some(x) => {
                *x += c;
                if x.is_zero() {
                    self.t.remove(&m);
                }
            }
            None => {
                self.t.insert(m, c);
            }
        }
    }
    pub fn is_zero(&self) -> bool {
        self.t.is_empty()
    }
    pub fn is_const(&self) -> bool {
        self.t.keys().all(|m| m.iter().all(|e| *e == 0))
    }
    pub fn as_const(&self) -> Option<Rat> {
        match self.t.len() {
            0 => Some(Rat::new()),
            1 if self.is_const() => self.t.values().next().cloned(),
            _ => None,
        }
    }
    pub fn is_one(&self) -> bool {
        self.as_const().is_some_and(|c| c == 1)
    }
    pub fn add(&self, o: &Poly) -> Poly {
        let mut r = self.clone();
        for (m, c) in &o.t {
            r.put(m.clone(), c.clone());
        }
        r
    }
    pub fn neg(&self) -> Poly {
        Poly {
            n: self.n,
            t: self
                .t
                .iter()
                .map(|(m, c)| (m.clone(), (-c).complete()))
                .collect(),
        }
    }
    pub fn sub(&self, o: &Poly) -> Poly {
        let mut r = self.clone();
        for (m, c) in &o.t {
            r.put(m.clone(), (-c).complete());
        }
        r
    }
    pub fn scale(&self, k: &Rat) -> Poly {
        if k.is_zero() {
            return Poly::zero(self.n);
        }
        Poly {
            n: self.n,
            t: self
                .t
                .iter()
                .map(|(m, c)| (m.clone(), (c * k).complete()))
                .collect(),
        }
    }
    pub fn mul(&self, o: &Poly) -> Poly {
        let mut r = Poly::zero(self.n);
        for (m1, c1) in &self.t {
            for (m2, c2) in &o.t {
                let m: Mono = m1.iter().zip(m2).map(|(a, b)| a + b).collect();
                r.put(m, (c1 * c2).complete());
            }
        }
        r
    }
    pub fn pow(&self, mut k: u32) -> Poly {
        let (mut base, mut acc) = (self.clone(), Poly::one(self.n));
        while k > 0 {
            if k & 1 == 1 {
                acc = acc.mul(&base);
            }
            k >>= 1;
            if k > 0 {
                base = base.mul(&base);
            }
        }
        acc
    }
    pub fn deg(&self, v: usize) -> u32 {
        self.t.keys().map(|m| m[v]).max().unwrap_or(0)
    }
    pub fn total_deg(&self) -> u32 {
        self.t.keys().map(|m| m.iter().sum()).max().unwrap_or(0)
    }
    pub fn uses(&self, v: usize) -> bool {
        self.t.keys().any(|m| m[v] > 0)
    }
    pub fn main_var(&self) -> Option<usize> {
        (0..self.n).find(|v| self.uses(*v))
    }
    /// Coefficients by degree in `v`; they do not contain `v`.
    pub fn coeffs(&self, v: usize) -> Vec<Poly> {
        if self.is_zero() {
            return Vec::new();
        }
        let mut out = vec![Poly::zero(self.n); self.deg(v) as usize + 1];
        for (m, c) in &self.t {
            let mut k = m.clone();
            let d = std::mem::take(&mut k[v]);
            out[d as usize].t.insert(k, c.clone());
        }
        out
    }
    pub fn from_coeffs(n: usize, v: usize, cs: &[Poly]) -> Poly {
        let mut r = Poly::zero(n);
        for (d, c) in cs.iter().enumerate() {
            for (m, k) in &c.t {
                let mut m = m.clone();
                m[v] += d as u32;
                r.put(m, k.clone());
            }
        }
        r
    }
    pub fn lc(&self, v: usize) -> Poly {
        self.coeffs(v).pop().unwrap_or_else(|| Poly::zero(self.n))
    }
    fn shift(&self, v: usize, d: u32) -> Poly {
        Poly {
            n: self.n,
            t: self
                .t
                .iter()
                .map(|(m, c)| {
                    let mut m = m.clone();
                    m[v] += d;
                    (m, c.clone())
                })
                .collect(),
        }
    }
    pub fn deriv(&self, v: usize) -> Poly {
        let mut r = Poly::zero(self.n);
        for (m, c) in &self.t {
            if m[v] > 0 {
                let mut k = m.clone();
                k[v] -= 1;
                r.put(k, (c * m[v]).complete());
            }
        }
        r
    }
    /// Substitutes a number for `v`.
    pub fn eval(&self, v: usize, x: &Rat) -> Poly {
        let mut r = Poly::zero(self.n);
        for (m, c) in &self.t {
            let mut k = m.clone();
            let d = std::mem::take(&mut k[v]);
            r.put(k, c * rug::ops::Pow::pow(x.clone(), d));
        }
        r
    }
    pub fn lead(&self) -> Option<(&Mono, &Rat)> {
        self.t.iter().next_back()
    }
    /// `self = content * primitive` with coprime integer coefficients and a positive leading one.
    pub fn primitive(&self) -> (Rat, Poly) {
        let Some((_, lead)) = self.lead() else {
            return (Rat::new(), self.clone());
        };
        let (mut g, mut l) = (Int::new(), Int::from(1));
        for c in self.t.values() {
            g = g.gcd(c.numer());
            l = l.lcm(c.denom());
        }
        let mut content = Rat::from((g, l));
        if lead.is_negative() {
            content = -content;
        }
        let inv = content.clone().recip();
        (content, self.scale(&inv))
    }
    pub fn normalized(&self) -> Poly {
        self.primitive().1
    }
    /// Exact quotient, `None` when `d` does not divide `self`.
    pub fn div_exact(&self, d: &Poly) -> Option<Poly> {
        let (dm, dc) = d.lead()?;
        if let Some(c) = d.as_const() {
            return Some(self.scale(&c.recip()));
        }
        let mut r = self.clone();
        let mut q = Poly::zero(self.n);
        while let Some((rm, rc)) = r.lead() {
            if !rm.iter().zip(dm).all(|(a, b)| a >= b) {
                return None;
            }
            let m: Mono = rm.iter().zip(dm).map(|(a, b)| a - b).collect();
            let c = (rc / dc).complete();
            let t = Poly::term(self.n, m, c);
            r = r.sub(&t.mul(d));
            q = q.add(&t);
        }
        Some(q)
    }
    /// Pseudo-remainder with respect to `v`.
    pub fn prem(&self, d: &Poly, v: usize) -> Poly {
        let dd = d.deg(v);
        if dd == 0 {
            return Poly::zero(self.n);
        }
        let dl = d.lc(v);
        let mut r = self.clone();
        while !r.is_zero() && r.deg(v) >= dd {
            let (rd, rl) = (r.deg(v), r.lc(v));
            r = r.mul(&dl).sub(&rl.mul(d).shift(v, rd - dd));
        }
        r
    }
    /// Gcd of the coefficients of `self` seen as a polynomial in `v`.
    pub fn content_in(&self, v: usize) -> Poly {
        let mut cs = self.coeffs(v);
        cs.sort_by_key(|c| c.t.len());
        let mut g = Poly::zero(self.n);
        for c in cs.iter().filter(|c| !c.is_zero()) {
            g = g.gcd(c);
            if g.is_one() {
                break;
            }
        }
        g
    }
    fn pp_in(&self, v: usize) -> Poly {
        match self.div_exact(&self.content_in(v)) {
            Some(p) => p.normalized(),
            None => self.normalized(),
        }
    }
    pub fn gcd(&self, o: &Poly) -> Poly {
        if self.is_zero() {
            return o.normalized();
        }
        if o.is_zero() {
            return self.normalized();
        }
        // out of time the trivial divisor is still a common one
        if self.is_const() || o.is_const() || crate::budget::expired() {
            return Poly::one(self.n);
        }
        let v = self.main_var().unwrap().min(o.main_var().unwrap());
        if !self.uses(v) {
            return self.gcd(&o.content_in(v));
        }
        if !o.uses(v) {
            return self.content_in(v).gcd(o);
        }
        let (ca, cb) = (self.content_in(v), o.content_in(v));
        let (mut a, mut b) = (
            self.div_exact(&ca).unwrap().normalized(),
            o.div_exact(&cb).unwrap().normalized(),
        );
        if a.deg(v) < b.deg(v) {
            std::mem::swap(&mut a, &mut b);
        }
        // most pairs share no factor, and an image modulo a prime shows that at once
        if b.deg(v) > 1 && coprime_image(&a, &b, v) {
            return ca.gcd(&cb).normalized();
        }
        while !b.is_zero() {
            if crate::budget::expired() {
                return Poly::one(self.n);
            }
            let r = a.prem(&b, v);
            a = b;
            b = if r.is_zero() { r } else { r.pp_in(v) };
        }
        let g = if a.uses(v) { a } else { Poly::one(self.n) };
        ca.gcd(&cb).mul(&g).normalized()
    }
    pub fn lcm(&self, o: &Poly) -> Poly {
        if self.is_zero() || o.is_zero() {
            return Poly::zero(self.n);
        }
        self.mul(o).div_exact(&self.gcd(o)).unwrap().normalized()
    }
    /// Square-free factors with multiplicities (Yun); constants are dropped.
    pub fn square_free(&self) -> Vec<(Poly, u32)> {
        let Some(v) = self.main_var() else {
            return Vec::new();
        };
        let c = self.content_in(v);
        let p = self.div_exact(&c).unwrap().normalized();
        let mut out = c.square_free();
        let dp = p.deriv(v);
        let g = p.gcd(&dp);
        let mut w = p.div_exact(&g).unwrap();
        let mut y = dp.div_exact(&g).unwrap();
        let mut i = 1;
        loop {
            let z = y.sub(&w.deriv(v));
            if z.is_zero() {
                if !w.is_const() {
                    out.push((w.normalized(), i));
                }
                break;
            }
            let h = w.gcd(&z);
            if !h.is_const() {
                out.push((h.clone(), i));
            }
            w = w.div_exact(&h).unwrap();
            y = z.div_exact(&h).unwrap();
            i += 1;
        }
        out
    }
    /// Dense coefficients when `v` is the only variable.
    pub fn to_dense(&self, v: usize) -> Option<Vec<Rat>> {
        if self
            .t
            .keys()
            .any(|m| m.iter().enumerate().any(|(i, e)| i != v && *e > 0))
        {
            return None;
        }
        let mut out = vec![
            Rat::new();
            if self.is_zero() {
                0
            } else {
                self.deg(v) as usize + 1
            }
        ];
        for (m, c) in &self.t {
            out[m[v] as usize] = c.clone();
        }
        Some(out)
    }
    pub fn from_dense(n: usize, v: usize, cs: &[Rat]) -> Poly {
        let mut r = Poly::zero(n);
        for (d, c) in cs.iter().enumerate() {
            let mut m = vec![0; n];
            m[v] = d as u32;
            r.put(m, c.clone());
        }
        r
    }
    /// Resultant with respect to `v` by the pseudo-remainder sequence.
    pub fn resultant(&self, o: &Poly, v: usize) -> Poly {
        let n = self.n;
        if self.is_zero() || o.is_zero() {
            return Poly::zero(n);
        }
        let (mut a, mut b) = (self.clone(), o.clone());
        let mut sign = 1;
        if a.deg(v) < b.deg(v) {
            if a.deg(v) % 2 == 1 && b.deg(v) % 2 == 1 {
                sign = -sign;
            }
            std::mem::swap(&mut a, &mut b);
        }
        // res(a, b) = (-1)^(da db) lc(b)^(da - dr) res(b, r) / lc(b)^((da-db+1) db) for r = prem(a, b)
        let (mut num, mut den) = (Poly::one(n), Poly::one(n));
        loop {
            let (da, db) = (a.deg(v), b.deg(v));
            if db == 0 {
                let r = if b.is_zero() && da > 0 {
                    Poly::zero(n)
                } else {
                    b.pow(da)
                };
                let total = num.mul(&r);
                let total = if sign < 0 { total.neg() } else { total };
                return total.div_exact(&den).unwrap_or(total);
            }
            let lb = b.lc(v);
            let mut r = a.clone();
            let mut steps = 0;
            while !r.is_zero() && r.uses(v) && r.deg(v) >= db {
                let (rd, rl) = (r.deg(v), r.lc(v));
                r = r.mul(&lb).sub(&rl.mul(&b).shift(v, rd - db));
                steps += 1;
            }
            if r.is_zero() {
                return Poly::zero(n);
            }
            let dr = r.deg(v);
            if da % 2 == 1 && db % 2 == 1 {
                sign = -sign;
            }
            num = num.mul(&lb.pow(da - dr));
            den = den.mul(&lb.pow(steps * db));
            a = b;
            b = r;
        }
    }
}

fn divides(a: &Mono, b: &Mono) -> bool {
    a.iter().zip(b).all(|(x, y)| x <= y)
}

/// Normal form of `p` modulo the basis (lexicographic order).
pub fn reduce(p: &Poly, basis: &[Poly]) -> Poly {
    let mut p = p.clone();
    let mut r = Poly::zero(p.n);
    'outer: while let Some((pm, pc)) = p.lead().map(|(m, c)| (m.clone(), c.clone())) {
        // (where the coefficients grow a reduction may take longer than a search has: at
        // the end of its time what is left stays as it is, which is in the same class)
        if crate::budget::expired() {
            r.t.extend(p.t);
            return r;
        }
        for g in basis {
            // (by a polynomial that is zero nothing is reduced)
            let Some((gm, gc)) = g.lead() else {
                continue;
            };
            if divides(gm, &pm) {
                let m: Mono = pm.iter().zip(gm).map(|(a, b)| a - b).collect();
                p = p.sub(&Poly::term(p.n, m, (&pc / gc).complete()).mul(g));
                continue 'outer;
            }
        }
        p.t.remove(&pm);
        r.t.insert(pm, pc);
    }
    r
}

fn s_poly(f: &Poly, g: &Poly) -> Poly {
    let ((fm, fc), (gm, gc)) = (f.lead().unwrap(), g.lead().unwrap());
    let l: Mono = fm.iter().zip(gm).map(|(a, b)| *a.max(b)).collect();
    let a = Poly::term(
        f.n,
        l.iter().zip(fm).map(|(x, y)| x - y).collect(),
        fc.clone().recip(),
    );
    let b = Poly::term(
        f.n,
        l.iter().zip(gm).map(|(x, y)| x - y).collect(),
        gc.clone().recip(),
    );
    a.mul(f).sub(&b.mul(g))
}

/// Reduced Groebner basis for the lexicographic order (variable 0 largest); `None` past `limit` reductions.
pub fn groebner(polys: &[Poly], limit: usize) -> Option<Vec<Poly>> {
    let mut g: Vec<Poly> = polys
        .iter()
        .filter(|p| !p.is_zero())
        .map(Poly::normalized)
        .collect();
    let mut pairs: Vec<(usize, usize)> = (0..g.len())
        .flat_map(|i| (0..i).map(move |j| (j, i)))
        .collect();
    let mut steps = 0;
    while let Some((i, j)) = pairs.pop() {
        let (a, b) = (g[i].lead().unwrap().0, g[j].lead().unwrap().0);
        if a.iter().zip(b).all(|(x, y)| *x == 0 || *y == 0) {
            continue;
        }
        steps += 1;
        if steps > limit || crate::budget::expired() {
            return None;
        }
        let s = reduce(&s_poly(&g[i], &g[j]), &g);
        if !s.is_zero() {
            if s.is_const() {
                return Some(vec![Poly::one(s.n)]);
            }
            let k = g.len();
            pairs.extend((0..k).map(|x| (x, k)));
            g.push(s.normalized());
            pairs.sort_by_key(|(x, y)| {
                let (a, b) = (g[*x].lead().unwrap().0, g[*y].lead().unwrap().0);
                std::cmp::Reverse(a.iter().zip(b).map(|(p, q)| *p.max(q)).sum::<u32>())
            });
        }
    }
    // minimal, then reduced
    let mut keep: Vec<Poly> = Vec::new();
    for (i, p) in g.iter().enumerate() {
        let lm = p.lead().unwrap().0;
        let redundant = g.iter().enumerate().any(|(j, q)| {
            let qm = q.lead().unwrap().0;
            j != i && divides(qm, lm) && (qm != lm || j < i)
        });
        if !redundant {
            keep.push(p.clone());
        }
    }
    let mut out = Vec::new();
    for i in 0..keep.len() {
        let others: Vec<Poly> = keep
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, p)| p.clone())
            .collect();
        let r = reduce(&keep[i], &others);
        if !r.is_zero() {
            let lead = r.lead().unwrap().1.clone().recip();
            out.push(r.scale(&lead));
        }
    }
    out.sort_by(|a, b| b.lead().unwrap().0.cmp(a.lead().unwrap().0));
    Some(out)
}
