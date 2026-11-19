//! Factorization of polynomials over the integers (Zassenhaus) and real root counting.

use crate::num::{Int, Rat};
use crate::poly::{Mono, Poly};
use rug::Complete;
use rug::ops::Pow as _;

type Zp = Vec<Int>;
type Mp = Vec<u64>;

fn trim<T: PartialEq + Default>(a: &mut Vec<T>) {
    let zero = T::default();
    while a.last() == Some(&zero) {
        a.pop();
    }
}

fn inv_mod(a: u64, p: u64) -> u64 {
    let (mut r, mut b, mut e) = (1u64, a % p, p - 2);
    while e > 0 {
        if e & 1 == 1 {
            r = r * b % p;
        }
        b = b * b % p;
        e >>= 1;
    }
    r
}

fn mp_from(z: &[Int], p: u64) -> Mp {
    let mut out: Mp = z.iter().map(|c| u64::from(c.mod_u(p as u32))).collect();
    trim(&mut out);
    out
}

fn mp_add(a: &Mp, b: &Mp, p: u64) -> Mp {
    let mut out: Mp = (0..a.len().max(b.len()))
        .map(|i| (a.get(i).unwrap_or(&0) + b.get(i).unwrap_or(&0)) % p)
        .collect();
    trim(&mut out);
    out
}

fn mp_sub(a: &Mp, b: &Mp, p: u64) -> Mp {
    let mut out: Mp = (0..a.len().max(b.len()))
        .map(|i| (a.get(i).unwrap_or(&0) + p - b.get(i).unwrap_or(&0)) % p)
        .collect();
    trim(&mut out);
    out
}

fn mp_mul(a: &Mp, b: &Mp, p: u64) -> Mp {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0u64; a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j] = (out[i + j] + x * y) % p;
        }
    }
    trim(&mut out);
    out
}

fn mp_divrem(a: &Mp, b: &Mp, p: u64) -> (Mp, Mp) {
    let mut r = a.clone();
    if r.len() < b.len() {
        return (Vec::new(), r);
    }
    let inv = inv_mod(*b.last().unwrap(), p);
    let mut q = vec![0u64; r.len() - b.len() + 1];
    while r.len() >= b.len() {
        let shift = r.len() - b.len();
        let c = r.last().unwrap() * inv % p;
        q[shift] = c;
        for (i, y) in b.iter().enumerate() {
            r[shift + i] = (r[shift + i] + p - c * y % p) % p;
        }
        trim(&mut r);
        if r.is_empty() {
            break;
        }
    }
    trim(&mut q);
    (q, r)
}

fn mp_monic(a: &Mp, p: u64) -> Mp {
    match a.last() {
        Some(&l) if l != 1 => {
            let inv = inv_mod(l, p);
            a.iter().map(|c| c * inv % p).collect()
        }
        _ => a.clone(),
    }
}

fn mp_gcd(a: &Mp, b: &Mp, p: u64) -> Mp {
    let (mut a, mut b) = (a.clone(), b.clone());
    while !b.is_empty() {
        let r = mp_divrem(&a, &b, p).1;
        a = b;
        b = r;
    }
    mp_monic(&a, p)
}

/// `(s, t)` with `s a + t b = 1` for coprime `a`, `b`.
fn mp_bezout(a: &Mp, b: &Mp, p: u64) -> (Mp, Mp) {
    let (mut r0, mut r1) = (a.clone(), b.clone());
    let (mut s0, mut s1, mut t0, mut t1): (Mp, Mp, Mp, Mp) = (vec![1], vec![], vec![], vec![1]);
    while !r1.is_empty() {
        let (q, r) = mp_divrem(&r0, &r1, p);
        let s = mp_sub(&s0, &mp_mul(&q, &s1, p), p);
        let t = mp_sub(&t0, &mp_mul(&q, &t1, p), p);
        (r0, r1, s0, s1, t0, t1) = (r1, r, s1, s, t1, t);
    }
    let inv = inv_mod(r0[0], p);
    (
        s0.iter().map(|c| c * inv % p).collect(),
        t0.iter().map(|c| c * inv % p).collect(),
    )
}

fn mp_deriv(a: &Mp, p: u64) -> Mp {
    let mut out: Mp = a
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, c)| c * (i as u64 % p) % p)
        .collect();
    trim(&mut out);
    out
}

fn mp_powmod(base: &Mp, e: &Int, f: &Mp, p: u64) -> Mp {
    let mut acc: Mp = vec![1];
    let mut b = mp_divrem(base, f, p).1;
    for i in 0..e.significant_bits() {
        if e.get_bit(i) {
            acc = mp_divrem(&mp_mul(&acc, &b, p), f, p).1;
        }
        b = mp_divrem(&mp_mul(&b, &b, p), f, p).1;
    }
    acc
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn equal_degree(g: &Mp, d: usize, p: u64, rng: &mut u64, out: &mut Vec<Mp>) {
    if g.len() - 1 == d {
        out.push(g.clone());
        return;
    }
    let e = (Int::from(p).pow(d as u32) - 1u32) / 2u32;
    loop {
        let mut a: Mp = (0..g.len() - 1).map(|_| next_random(rng) % p).collect();
        trim(&mut a);
        if a.len() < 2 {
            continue;
        }
        let b = mp_sub(&mp_powmod(&a, &e, g, p), &vec![1], p);
        let h = mp_gcd(&b, g, p);
        if h.len() > 1 && h.len() < g.len() {
            equal_degree(&h, d, p, rng, out);
            equal_degree(&mp_divrem(g, &h, p).0, d, p, rng, out);
            return;
        }
    }
}

/// Irreducible monic factors of a monic square-free polynomial modulo an odd prime.
fn factor_mod(f: &Mp, p: u64) -> Vec<Mp> {
    let mut out = Vec::new();
    let mut f = f.clone();
    let x: Mp = vec![0, 1];
    let mut h = x.clone();
    let mut d = 0;
    let mut rng = 0x2545_F491_4F6C_DD1Du64;
    while f.len() > 2 * (d + 1) {
        d += 1;
        h = mp_powmod(&h, &Int::from(p), &f, p);
        let g = mp_gcd(&mp_sub(&h, &x, p), &f, p);
        if g.len() > 1 {
            equal_degree(&g, d, p, &mut rng, &mut out);
            f = mp_divrem(&f, &g, p).0;
            if f.len() > 1 {
                h = mp_divrem(&h, &f, p).1;
            }
        }
    }
    if f.len() > 1 {
        out.push(f);
    }
    out
}

fn zp_mul(a: &[Int], b: &[Int]) -> Zp {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![Int::new(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j] += (x * y).complete();
        }
    }
    out
}

fn zp_content(a: &[Int]) -> Int {
    a.iter().fold(Int::new(), |g, c| g.gcd(c))
}

fn zp_primitive(a: &[Int]) -> Zp {
    let mut g = zp_content(a);
    if a.last().is_some_and(|l| l.is_negative()) {
        g = -g;
    }
    if g.is_zero() {
        return a.to_vec();
    }
    a.iter().map(|c| (c / &g).complete()).collect()
}

fn zp_div_exact(a: &[Int], b: &[Int]) -> Option<Zp> {
    if b.is_empty() || a.len() < b.len() {
        return None;
    }
    let mut r = a.to_vec();
    let lead = b.last().unwrap();
    let mut q = vec![Int::new(); a.len() - b.len() + 1];
    for shift in (0..q.len()).rev() {
        let top = &r[shift + b.len() - 1];
        if !top.is_divisible(lead) {
            return None;
        }
        let c = (top / lead).complete();
        for (i, y) in b.iter().enumerate() {
            r[shift + i] -= (&c * y).complete();
        }
        q[shift] = c;
    }
    r.iter().all(|c| c.is_zero()).then_some(q)
}

/// Lifts `f = g h (mod p)` to a factorization modulo `p^k` (all monic).
fn hensel(f: &[Int], g0: &Mp, h0: &Mp, p: u64, k: u32) -> (Zp, Zp) {
    let (s, t) = mp_bezout(g0, h0, p);
    let lift = |m: &Mp| -> Zp { m.iter().map(|c| Int::from(*c)).collect() };
    let (mut g, mut h) = (lift(g0), lift(h0));
    let mut pj = Int::from(p);
    for _ in 1..k {
        let gh = zp_mul(&g, &h);
        let e: Vec<Int> = (0..f.len())
            .map(|i| ((&f[i] - gh.get(i).unwrap_or(&Int::new())).complete()).div_exact(&pj))
            .collect();
        let e = mp_from(&e, p);
        let (q, dg) = mp_divrem(&mp_mul(&t, &e, p), g0, p);
        let dh = mp_add(&mp_mul(&s, &e, p), &mp_mul(&q, h0, p), p);
        for (i, c) in dg.iter().enumerate() {
            g[i] += (&pj * *c).complete();
        }
        for (i, c) in dh.iter().enumerate() {
            h[i] += (&pj * *c).complete();
        }
        pj *= p;
    }
    (g, h)
}

const PRIMES: &[u64] = &[
    13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97, 101, 103, 107,
    109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181, 191, 193, 197, 199, 211,
    223, 227, 229, 233, 239, 241, 251,
];

const MAX_MODULAR_FACTORS: usize = 22;

/// Irreducible factors of a square-free primitive polynomial with a positive leading coefficient.
/// The flag is false when the search was given up (too many modular factors).
fn factor_square_free(f: &Zp) -> (Vec<Zp>, bool) {
    let n = f.len() - 1;
    if n <= 1 {
        return (vec![f.clone()], true);
    }
    // out of time the polynomial is left whole
    if crate::budget::expired() {
        return (vec![f.clone()], false);
    }
    let lc = f.last().unwrap().clone();
    let mut best: Option<(u64, Vec<Mp>)> = None;
    let mut tried = 0;
    for &p in PRIMES {
        if lc.is_divisible_u(p as u32) {
            continue;
        }
        let fp = mp_from(f, p);
        if mp_gcd(&fp, &mp_deriv(&fp, p), p).len() > 1 {
            continue;
        }
        let parts = factor_mod(&mp_monic(&fp, p), p);
        if parts.len() == 1 {
            return (vec![f.clone()], true);
        }
        if best.as_ref().is_none_or(|(_, b)| parts.len() < b.len()) {
            best = Some((p, parts));
        }
        tried += 1;
        if tried >= 7 {
            break;
        }
    }
    let Some((p, mods)) = best else {
        return (vec![f.clone()], false);
    };
    if mods.len() > MAX_MODULAR_FACTORS {
        return (vec![f.clone()], false);
    }
    let norm = f
        .iter()
        .fold(Int::new(), |s, c| s + (c * c).complete())
        .sqrt()
        + 1u32;
    let bound = (Int::from(1) << (n as u32 + 2)) * norm * lc.clone().abs();
    let mut k = 1;
    let mut m = Int::from(p);
    while m <= bound {
        m *= p;
        k += 1;
    }
    let lc_inv = lc
        .clone()
        .invert(&m)
        .expect("the prime does not divide the leading coefficient");
    let monic: Zp = f
        .iter()
        .map(|c| (c * &lc_inv).complete().modulo(&m))
        .collect();
    let mut lifted: Vec<Zp> = Vec::new();
    let mut rest = monic;
    for i in 0..mods.len() - 1 {
        let h0 = mods[i + 1..]
            .iter()
            .fold(vec![1u64], |acc, g| mp_mul(&acc, g, p));
        let (g, h) = hensel(&rest, &mods[i], &h0, p, k);
        lifted.push(g);
        rest = h.iter().map(|c| c.clone().modulo(&m)).collect();
    }
    lifted.push(rest);
    let half = (&m / 2u32).complete();
    let mut remaining = f.clone();
    let mut out = Vec::new();
    let mut size = 1;
    while 2 * size <= lifted.len() {
        let mut idx: Vec<usize> = (0..size).collect();
        let mut found = false;
        loop {
            let mut cand: Zp = vec![remaining.last().unwrap().clone()];
            for &i in &idx {
                cand = zp_mul(&cand, &lifted[i])
                    .into_iter()
                    .map(|c| c.modulo(&m))
                    .collect();
            }
            let cand: Zp = cand
                .into_iter()
                .map(|c| if c > half { c - &m } else { c })
                .collect();
            let cand = zp_primitive(&cand);
            if let Some(q) = zp_div_exact(&remaining, &cand) {
                out.push(cand);
                remaining = zp_primitive(&q);
                for &i in idx.iter().rev() {
                    lifted.remove(i);
                }
                found = true;
                break;
            }
            // next combination
            let mut i = size;
            while i > 0 && idx[i - 1] == lifted.len() - size + i - 1 {
                i -= 1;
            }
            if i == 0 {
                break;
            }
            idx[i - 1] += 1;
            for j in i..size {
                idx[j] = idx[j - 1] + 1;
            }
        }
        if !found {
            size += 1;
        }
    }
    if remaining.len() > 1 {
        out.push(remaining);
    }
    (out, true)
}

fn to_zp(c: &[Rat]) -> Zp {
    c.iter().map(|r| r.numer().clone()).collect()
}

/// Factors of a square-free primitive polynomial in one or several variables.
fn factor_piece(s: &Poly, complete: &mut bool) -> Vec<Poly> {
    let used: Vec<usize> = (0..s.n).filter(|v| s.uses(*v)).collect();
    if used.len() == 1 {
        let v = used[0];
        let dense = to_zp(&s.to_dense(v).unwrap());
        let (parts, ok) = factor_square_free(&dense);
        *complete &= ok;
        return parts
            .iter()
            .map(|z| {
                Poly::from_dense(
                    s.n,
                    v,
                    &z.iter().map(|c| Rat::from(c.clone())).collect::<Vec<_>>(),
                )
            })
            .collect();
    }
    // linear in a variable: its content in that variable times an irreducible cofactor
    if let Some(&v) = used.iter().find(|v| s.deg(**v) == 1) {
        let content = s.content_in(v);
        if content.is_const() {
            return vec![s.clone()];
        }
        if let Some(cofactor) = s.div_exact(&content) {
            let mut parts = factor_piece(&content, complete);
            parts.push(cofactor);
            let product = parts.iter().fold(Poly::one(s.n), |acc, f| acc.mul(f));
            if product == *s {
                return parts;
            }
            if product.neg() == *s {
                parts[0] = parts[0].neg();
                return parts;
            }
        }
        *complete = false;
        return vec![s.clone()];
    }
    // Kronecker substitution: variable used[i] becomes t^(d^i).
    let d = u64::from(used.iter().map(|v| s.deg(*v)).max().unwrap_or(0)) + 1;
    let top = d.checked_pow(used.len() as u32).filter(|t| *t <= 512);
    let Some(top) = top else {
        *complete = false;
        return vec![s.clone()];
    };
    let mut image = vec![Int::new(); top as usize];
    for (m, c) in &s.t {
        let e: u64 = used
            .iter()
            .enumerate()
            .map(|(i, v)| u64::from(m[*v]) * d.pow(i as u32))
            .sum();
        image[e as usize] += c.numer();
    }
    trim(&mut image);
    let image_poly = Poly::from_dense(
        1,
        0,
        &image
            .iter()
            .map(|c| Rat::from(c.clone()))
            .collect::<Vec<_>>(),
    );
    let mut parts: Vec<Zp> = Vec::new();
    for (sq, mult) in image_poly.square_free() {
        let (fs, ok) = factor_square_free(&to_zp(&sq.to_dense(0).unwrap()));
        *complete &= ok;
        for f in fs {
            for _ in 0..mult {
                parts.push(f.clone());
            }
        }
    }
    if parts.len() > 16 {
        *complete = false;
        return vec![s.clone()];
    }
    let decode = |z: &Zp| -> Poly {
        let mut p = Poly::zero(s.n);
        for (e, c) in z.iter().enumerate().filter(|(_, c)| !c.is_zero()) {
            let mut m: Mono = vec![0; s.n];
            let mut e = e as u64;
            for v in &used {
                m[*v] = (e % d) as u32;
                e /= d;
            }
            p = p.add(&Poly::term(s.n, m, Rat::from(c.clone())));
        }
        p
    };
    let mut remaining = s.clone();
    let mut out = Vec::new();
    let mut size = 1;
    while 2 * size <= parts.len() {
        let mut found = false;
        let total = parts.len();
        let mut idx: Vec<usize> = (0..size).collect();
        loop {
            let cand = idx
                .iter()
                .fold(vec![Int::from(1)], |acc, i| zp_mul(&acc, &parts[*i]));
            let cand = decode(&cand).normalized();
            if !cand.is_const()
                && let Some(q) = remaining.div_exact(&cand)
            {
                out.push(cand);
                remaining = q.normalized();
                for &i in idx.iter().rev() {
                    parts.remove(i);
                }
                found = true;
                break;
            }
            let mut i = size;
            while i > 0 && idx[i - 1] == total - size + i - 1 {
                i -= 1;
            }
            if i == 0 {
                break;
            }
            idx[i - 1] += 1;
            for j in i..size {
                idx[j] = idx[j - 1] + 1;
            }
        }
        if !found {
            size += 1;
        }
    }
    if !remaining.is_const() {
        out.push(remaining);
    }
    out
}

/// Content and irreducible primitive factors with multiplicities; the flag tells whether every
/// factor is known to be irreducible.
pub fn factor(p: &Poly) -> (Rat, Vec<(Poly, u32)>, bool) {
    let (content, pp) = p.primitive();
    let mut complete = true;
    let mut out: Vec<(Poly, u32)> = Vec::new();
    for (s, mult) in pp.square_free() {
        for f in factor_piece(&s, &mut complete) {
            match out.iter_mut().find(|(g, _)| *g == f) {
                Some((_, m)) => *m += mult,
                None => out.push((f, mult)),
            }
        }
    }
    out.sort_by(|a, b| {
        (a.0.total_deg(), a.0.t.len())
            .cmp(&(b.0.total_deg(), b.0.t.len()))
            .then_with(|| {
                let (x, y): (Vec<_>, Vec<_>) = (a.0.t.iter().collect(), b.0.t.iter().collect());
                x.cmp(&y)
            })
    });
    (content, out, complete)
}

fn dense_rem(a: &[Rat], b: &[Rat]) -> Vec<Rat> {
    let mut r = a.to_vec();
    while r.len() >= b.len() && !r.is_empty() {
        let c = (r.last().unwrap() / b.last().unwrap()).complete();
        let shift = r.len() - b.len();
        for (i, y) in b.iter().enumerate() {
            r[shift + i] -= (&c * y).complete();
        }
        r.pop();
        trim(&mut r);
    }
    r
}

/// Number of distinct real roots of a polynomial with rational coefficients (Sturm).
pub fn real_root_count(c: &[Rat]) -> usize {
    let mut p0: Vec<Rat> = c.to_vec();
    trim(&mut p0);
    if p0.len() < 2 {
        return 0;
    }
    let mut p1: Vec<Rat> = p0
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, c)| (c * i as u32).complete())
        .collect();
    let mut chain = vec![p0.clone()];
    while !p1.is_empty() {
        chain.push(p1.clone());
        let r: Vec<Rat> = dense_rem(&p0, &p1).into_iter().map(|c| -c).collect();
        p0 = p1;
        p1 = r;
    }
    let changes = |at_plus: bool| {
        let signs: Vec<i32> = chain
            .iter()
            .map(|p| {
                let s = p.last().unwrap().cmp0() as i32;
                if at_plus || (p.len() - 1) % 2 == 0 {
                    s
                } else {
                    -s
                }
            })
            .collect();
        signs.windows(2).filter(|w| w[0] != w[1]).count()
    };
    changes(false) - changes(true)
}
