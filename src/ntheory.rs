//! Exact number theory and combinatorics.

use rug::integer::IsPrime;
use rug::ops::{Pow, RemRounding};
use rug::{Assign, Complete, Integer, Rational};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::mem;
use std::sync::LazyLock;

pub const PRIMES_UP_TO_MAX: u64 = 100_000_000;
pub const PRIME_PI_MAX: u64 = 100_000_000_000_000;
pub const NTH_PRIME_MAX: u64 = 1_000_000_000_000;

const MAX_DIVISORS: u64 = 100_000;
const RHO_STEPS: u64 = 1 << 19;
const PM1_B1: u64 = 50_000;
const PM1_B2_RATIO: u64 = 50;
const ECM_D: u64 = 210;
const ECM_B2_RATIO: u64 = 50;
const ECM_CURVE_COST: u64 = 30;
const ECM_LEVELS: [(u64, u32); 5] = [
    (2_000, 200),
    (10_000, 150),
    (50_000, 400),
    (250_000, 1_000),
    (1_000_000, u32::MAX),
];
const ECM_WORK: u64 = 80_000_000;
const MAX_EFFORT: u32 = 6;
const ZIGZAG_CACHE: usize = 8_192;
const PARTITION_CACHE: usize = 250_000;
const DLOG_MAX_ORDER: u64 = 1 << 44;
const DLOG_TABLE_BITS: u64 = 1 << 25;

const WITNESSES: [(u64, u64); 12] = [
    (2, 2_047),
    (3, 1_373_653),
    (5, 25_326_001),
    (7, 3_215_031_751),
    (11, 2_152_302_898_747),
    (13, 3_474_749_660_383),
    (17, 341_550_071_728_321),
    (19, 341_550_071_728_321),
    (23, 3_825_123_056_546_413_051),
    (29, 3_825_123_056_546_413_051),
    (31, 3_825_123_056_546_413_051),
    (37, u64::MAX),
];

static SMALL_PRIMES: LazyLock<Vec<u32>> = LazyLock::new(|| {
    sieve_range(2, 65_535)
        .into_iter()
        .map(|p| p as u32)
        .collect()
});

fn mul_mod(a: u64, b: u64, m: u64) -> u64 {
    (u128::from(a) * u128::from(b) % u128::from(m)) as u64
}

fn pow_mod_u64(mut b: u64, mut e: u64, m: u64) -> u64 {
    let mut r = 1 % m;
    b %= m;
    while e > 0 {
        if e & 1 == 1 {
            r = mul_mod(r, b, m);
        }
        b = mul_mod(b, b, m);
        e >>= 1;
    }
    r
}

fn is_prime_u64(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    for (p, _) in WITNESSES {
        if n.is_multiple_of(p) {
            return n == p;
        }
    }
    let s = (n - 1).trailing_zeros();
    let d = (n - 1) >> s;
    for (a, limit) in WITNESSES {
        let mut x = pow_mod_u64(a, d, n);
        if x != 1 && x != n - 1 {
            let passes = (1..s).any(|_| {
                x = mul_mod(x, x, n);
                x == n - 1
            });
            if !passes {
                return false;
            }
        }
        if n < limit {
            return true;
        }
    }
    true
}

pub fn is_prime(n: &Integer) -> bool {
    match n.to_u64() {
        Some(x) => is_prime_u64(x),
        None => n.is_positive() && n.is_probably_prime(64) != IsPrime::No,
    }
}

pub fn next_prime(n: &Integer) -> Integer {
    if *n < 2 {
        return Integer::from(2);
    }
    let mut p = n.clone();
    loop {
        p.next_prime_mut();
        if is_prime(&p) {
            return p;
        }
    }
}

pub fn prev_prime(n: &Integer) -> Option<Integer> {
    if *n <= 2 {
        return None;
    }
    let mut p = n.clone();
    loop {
        p.prev_prime_mut();
        if is_prime(&p) {
            return Some(p);
        }
    }
}

fn sieve_range(lo: u64, hi: u64) -> Vec<u64> {
    const SPAN: u64 = 1 << 18;
    let mut out = Vec::new();
    if hi < 2 || lo > hi {
        return out;
    }
    if lo <= 2 {
        out.push(2);
    }
    let root = hi.isqrt();
    let base = if root < 3 {
        Vec::new()
    } else {
        sieve_range(3, root)
    };
    let mut flags = vec![true; (SPAN / 2) as usize];
    let mut start = lo.max(3) | 1;
    while start <= hi {
        let end = hi.min(start + SPAN - 2);
        let len = ((end - start) / 2 + 1) as usize;
        flags[..len].fill(true);
        for &p in base.iter().take_while(|&&p| p * p <= end) {
            let mut m = (p * p).max(start.div_ceil(p) * p);
            if m.is_multiple_of(2) {
                m += p;
            }
            for i in (((m - start) / 2) as usize..len).step_by(p as usize) {
                flags[i] = false;
            }
        }
        out.extend((0..len).filter(|&i| flags[i]).map(|i| start + 2 * i as u64));
        start = end + 2;
    }
    out
}

/// Panics if `n > PRIMES_UP_TO_MAX`.
pub fn primes_up_to(n: u64) -> Vec<u64> {
    assert!(
        n <= PRIMES_UP_TO_MAX,
        "primes_up_to: n exceeds PRIMES_UP_TO_MAX"
    );
    sieve_range(2, n)
}

fn prime_count(n: u64) -> u64 {
    if n < 2 {
        return 0;
    }
    let v = n.isqrt() as usize;
    let half = |x: u64| ((x - 1) >> 1) as usize;
    let mut s = v.div_ceil(2);
    let mut smalls: Vec<u32> = (0..s as u32).collect();
    let mut roughs: Vec<u32> = (0..s as u32).map(|i| 2 * i + 1).collect();
    let mut larges: Vec<u64> = roughs
        .iter()
        .map(|&r| (n / u64::from(r) - 1) >> 1)
        .collect();
    let mut skip = vec![false; v + 1];
    let mut pc = 0usize;
    for p in (3..=v).step_by(2) {
        if skip[p] {
            continue;
        }
        let q = p * p;
        if q > v {
            break;
        }
        skip[p] = true;
        for i in (q..=v).step_by(2 * p) {
            skip[i] = true;
        }
        let mut ns = 0;
        for k in 0..s {
            let i = roughs[k] as usize;
            if skip[i] {
                continue;
            }
            let d = i as u64 * p as u64;
            let sub = if d <= v as u64 {
                larges[smalls[(d >> 1) as usize] as usize - pc]
            } else {
                u64::from(smalls[half(n / d)])
            };
            larges[ns] = larges[k] + pc as u64 - sub;
            roughs[ns] = i as u32;
            ns += 1;
        }
        s = ns;
        let mut i = half(v as u64);
        let mut j = ((v / p) - 1) | 1;
        while j >= p {
            let c = smalls[j >> 1] - pc as u32;
            let e = (j * p) >> 1;
            while i >= e {
                smalls[i] -= c;
                i -= 1;
            }
            j -= 2;
        }
        pc += 1;
    }
    let (s, pc) = (s as i64, pc as i64);
    let mut total = larges[0] as i64 + (s + 2 * (pc - 1)) * (s - 1) / 2;
    total -= larges[1..s as usize].iter().sum::<u64>() as i64;
    for l in 1..s {
        let q = u64::from(roughs[l as usize]);
        let m = n / q;
        let e = i64::from(smalls[half(m / q)]) - pc;
        if e <= l {
            break;
        }
        let t: u64 = (l + 1..=e)
            .map(|k| u64::from(smalls[half(m / u64::from(roughs[k as usize]))]))
            .sum();
        total += t as i64 - (e - l) * (pc + l - 1);
    }
    total as u64 + 1
}

pub fn prime_pi(n: &Integer) -> Option<Integer> {
    if !n.is_positive() {
        return Some(Integer::new());
    }
    let x = n.to_u64().filter(|&x| x <= PRIME_PI_MAX)?;
    Some(Integer::from(prime_count(x)))
}

fn li_inverse(n: f64) -> f64 {
    let li = |x: f64| {
        let l = x.ln();
        let (mut sum, mut term) = (0.0, 1.0);
        for k in 1..400 {
            term *= l / f64::from(k);
            sum += term / f64::from(k);
        }
        0.577_215_664_901_532_9 + l.ln() + sum
    };
    let mut x = n * n.ln();
    for _ in 0..40 {
        let dx = (n - li(x)) * x.ln();
        x += dx;
        if dx.abs() < 0.5 {
            break;
        }
    }
    x
}

fn nth_prime_near(n: u64, x0: u64) -> u64 {
    let count = prime_count(x0);
    let window = |k: usize, x: u64| (k as u64).max(64) * u64::from(65 - x.leading_zeros());
    if count < n {
        let (mut need, mut lo) = ((n - count) as usize, x0 + 1);
        loop {
            let hi = lo + window(need, lo);
            let ps = sieve_range(lo, hi);
            if ps.len() >= need {
                return ps[need - 1];
            }
            need -= ps.len();
            lo = hi + 1;
        }
    }
    let (mut need, mut hi) = ((count - n + 1) as usize, x0);
    loop {
        let lo = hi.saturating_sub(window(need, hi));
        let ps = sieve_range(lo, hi);
        if ps.len() >= need {
            return ps[ps.len() - need];
        }
        need -= ps.len();
        hi = lo - 1;
    }
}

pub fn nth_prime(n: u64) -> Option<Integer> {
    if n == 0 || n > NTH_PRIME_MAX {
        return None;
    }
    let x0 = if n < 1000 {
        0
    } else {
        li_inverse(n as f64) as u64
    };
    Some(Integer::from(nth_prime_near(n, x0)))
}

fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn rho_u64(n: u64) -> u64 {
    let root = n.isqrt();
    if root * root == n {
        return root;
    }
    let mut c = 0;
    loop {
        c += 1;
        let step = |x: u64| ((u128::from(x) * u128::from(x) + c) % u128::from(n)) as u64;
        let (mut x, mut y, mut ys, mut q, mut g, mut r) = (2, 2, 2, 1, 1, 1u64);
        while g == 1 {
            x = y;
            for _ in 0..r {
                y = step(y);
            }
            let mut k = 0;
            while k < r && g == 1 {
                ys = y;
                for _ in 0..(r - k).min(128) {
                    y = step(y);
                    q = mul_mod(q, x.abs_diff(y), n);
                }
                g = gcd_u64(q, n);
                k += 128;
            }
            r *= 2;
        }
        if g == n {
            g = 1;
            while g == 1 {
                ys = step(ys);
                g = gcd_u64(x.abs_diff(ys), n);
            }
        }
        if g != n {
            return g;
        }
    }
}

fn factor_u64(n: u64, out: &mut Vec<u64>) {
    if is_prime_u64(n) {
        out.push(n);
    } else {
        let d = rho_u64(n);
        factor_u64(d, out);
        factor_u64(n / d, out);
    }
}

fn rho_brent(n: &Integer, mut budget: u64) -> Option<Integer> {
    let (mut x, mut y, mut ys) = (Integer::new(), Integer::new(), Integer::new());
    let (mut q, mut g, mut t) = (Integer::new(), Integer::new(), Integer::new());
    for c in 1u32.. {
        let step = |y: &mut Integer| {
            y.square_mut();
            *y += c;
            *y %= n;
        };
        y.assign(2);
        q.assign(1);
        g.assign(1);
        let (mut r, mut steps) = (1u64, 0u64);
        while g == 1 && steps < budget {
            x.assign(&y);
            for _ in 0..r {
                step(&mut y);
            }
            let mut k = 0;
            while k < r && g == 1 {
                ys.assign(&y);
                for _ in 0..(r - k).min(128) {
                    step(&mut y);
                    t.assign(&x - &y);
                    q *= &t;
                    q %= n;
                }
                g.assign(q.gcd_ref(n));
                k += 128;
            }
            steps += 2 * r;
            r *= 2;
        }
        if g == *n {
            g.assign(1);
            while g == 1 {
                step(&mut ys);
                t.assign(&x - &ys);
                g.assign(t.gcd_ref(n));
            }
        }
        if g != *n {
            return (g != 1).then_some(g);
        }
        budget = budget.saturating_sub(steps);
    }
    None
}

fn prime_power_below(q: u64, bound: u64) -> u64 {
    let mut qk = q;
    while qk <= bound / q {
        qk *= q;
    }
    qk
}

fn pollard_pm1(n: &Integer, b1: u64) -> Option<Integer> {
    let primes = sieve_range(2, PM1_B2_RATIO * b1);
    let split = primes.partition_point(|&q| q <= b1);
    let advance = |a: &mut Integer, q: u64| {
        a.pow_mod_mut(&Integer::from(prime_power_below(q, b1)), n)
            .ok()
    };
    let mut a = Integer::from(3);
    let mut g = Integer::new();
    for chunk in primes[..split].chunks(32) {
        let saved = a.clone();
        chunk.iter().try_for_each(|&q| advance(&mut a, q))?;
        g.assign((&a - 1u32).complete().gcd_ref(n));
        if g == *n {
            a = saved;
            for &q in chunk {
                advance(&mut a, q)?;
                g.assign((&a - 1u32).complete().gcd_ref(n));
                if g != 1 {
                    break;
                }
            }
        }
        if g != 1 {
            return (g != *n).then_some(g);
        }
    }
    let tail = &primes[split..];
    let first = *tail.first()?;
    let max_gap = tail.windows(2).map(|w| w[1] - w[0]).max().unwrap_or(2);
    let a2 = a.clone().square() % n;
    let mut steps = vec![a2.clone()];
    for i in 1..(max_gap / 2) as usize {
        let next = (&steps[i - 1] * &a2).complete() % n;
        steps.push(next);
    }
    let mut t = a.pow_mod(&Integer::from(first), n).ok()?;
    let mut acc = (&t - 1u32).complete();
    for w in tail.windows(2) {
        t *= &steps[((w[1] - w[0]) / 2 - 1) as usize];
        t %= n;
        acc *= (&t - 1u32).complete();
        acc %= n;
    }
    let g = acc.gcd(n);
    (g != 1 && g != *n).then_some(g)
}

type Point = (Integer, Integer);

struct Curve<'a> {
    n: &'a Integer,
    a24: Integer,
    t: [Integer; 3],
}

impl Curve<'_> {
    fn double(&mut self, p: &mut Point) {
        let [a, b, c] = &mut self.t;
        a.assign(&p.0 + &p.1);
        a.square_mut();
        *a %= self.n;
        b.assign(&p.0 - &p.1);
        b.square_mut();
        *b %= self.n;
        p.0.assign(&*a * &*b);
        p.0 %= self.n;
        *a -= &*b;
        c.assign(&*a * &self.a24);
        *c %= self.n;
        *c += &*b;
        p.1.assign(&*a * &*c);
        p.1 %= self.n;
    }

    fn add(&mut self, p: &Point, q: &Point, diff: &Point, out: &mut Point) {
        let [a, b, c] = &mut self.t;
        a.assign(&p.0 - &p.1);
        b.assign(&q.0 + &q.1);
        *a *= &*b;
        b.assign(&p.0 + &p.1);
        c.assign(&q.0 - &q.1);
        *b *= &*c;
        c.assign(&*a + &*b);
        *c %= self.n;
        c.square_mut();
        *c %= self.n;
        out.0.assign(&*c * &diff.1);
        out.0 %= self.n;
        *a -= &*b;
        *a %= self.n;
        a.square_mut();
        *a %= self.n;
        out.1.assign(&*a * &diff.0);
        out.1 %= self.n;
    }

    fn mul(&mut self, k: u64, p: &Point) -> Point {
        let (mut r0, mut r1, mut sum) = (p.clone(), p.clone(), Point::default());
        self.double(&mut r1);
        for i in (0..63 - k.leading_zeros()).rev() {
            self.add(&r0, &r1, p, &mut sum);
            if k >> i & 1 == 1 {
                mem::swap(&mut r0, &mut sum);
                self.double(&mut r1);
            } else {
                mem::swap(&mut r1, &mut sum);
                self.double(&mut r0);
            }
        }
        r0
    }
}

fn ecm_curve(n: &Integer, sigma: u32, b1: u64, primes: &[u64], flags: &[bool]) -> Option<Integer> {
    let u = Integer::from(sigma).square() - 5u32;
    let v = Integer::from(4 * sigma);
    let den = Integer::from((&u).pow(3)) * &v * 16u32;
    let Some(inv) = den.invert_ref(n) else {
        let g = den.gcd(n);
        return (g != *n).then_some(g);
    };
    let a24 = Integer::from(&v - &u).pow(3) * (3u32 * u.clone() + &v) % n * Integer::from(inv) % n;
    let mut curve = Curve {
        n,
        a24,
        t: Default::default(),
    };
    let mut q: Point = (u.pow(3) % n, v.pow(3) % n);
    for &p in primes {
        q = curve.mul(prime_power_below(p, b1), &q);
    }
    let g = q.1.gcd_ref(n).complete();
    if g != 1 {
        return (g != *n).then_some(g);
    }
    let mut q2 = q.clone();
    curve.double(&mut q2);
    let mut odd = vec![q.clone(), Point::default()];
    curve.add(&q, &q2, &q, &mut odd[1]);
    for i in 2..(ECM_D / 4) as usize {
        let mut next = Point::default();
        curve.add(&odd[i - 1], &q2, &odd[i - 2], &mut next);
        odd.push(next);
    }
    let giant = curve.mul(ECM_D, &q);
    let first = b1 / ECM_D;
    let (mut prev, mut cur) = (curve.mul(first - 1, &giant), curve.mul(first, &giant));
    let (mut acc, mut t1, mut t2, mut next) = (
        Integer::from(1),
        Integer::new(),
        Integer::new(),
        Point::default(),
    );
    let offsets: Vec<usize> = (1..ECM_D / 2)
        .filter(|&j| gcd_u64(j, ECM_D) == 1)
        .map(|j| j as usize)
        .collect();
    for m in first..=(flags.len() as u64 / ECM_D - 1) {
        let r = (m * ECM_D) as usize;
        for &j in offsets.iter().filter(|&&j| flags[r - j] || flags[r + j]) {
            t1.assign(&cur.0 * &odd[j / 2].1);
            t2.assign(&cur.1 * &odd[j / 2].0);
            t1 -= &t2;
            acc *= &t1;
            acc %= n;
        }
        curve.add(&cur, &giant, &prev, &mut next);
        mem::swap(&mut prev, &mut cur);
        mem::swap(&mut cur, &mut next);
    }
    let g = acc.gcd(n);
    (g != 1 && g != *n).then_some(g)
}

fn ecm(n: &Integer, mut work: u64) -> Option<Integer> {
    let mut sigma = 6;
    for (b1, curves) in ECM_LEVELS {
        let count = (work / (ECM_CURVE_COST * b1)).min(curves.into());
        if count == 0 {
            break;
        }
        work -= count * ECM_CURVE_COST * b1;
        let limit = (ECM_B2_RATIO * b1).div_ceil(ECM_D) * ECM_D + ECM_D;
        let mut primes = sieve_range(2, limit);
        let mut flags = vec![false; limit as usize + 1];
        primes.iter().for_each(|&p| flags[p as usize] = true);
        primes.truncate(primes.partition_point(|&p| p <= b1));
        for _ in 0..count {
            if let Some(g) = ecm_curve(n, sigma, b1, &primes, &flags) {
                return Some(g);
            }
            sigma += 1;
        }
    }
    None
}

fn find_factor(n: &Integer, effort: u32) -> Option<Integer> {
    let limbs = u64::from(n.significant_bits().div_ceil(64));
    let scale = |base: u64| base * 80 / (38 * limbs + limbs * limbs);
    let rho = rho_brent(n, scale(RHO_STEPS));
    if rho.is_some() || effort < 2 {
        return rho;
    }
    let work = scale(ECM_WORK) << (2 * (effort.min(MAX_EFFORT) - 2));
    pollard_pm1(n, scale(PM1_B1).max(1000)).or_else(|| ecm(n, work))
}

type Factors = BTreeMap<Integer, u32>;

fn factor_parts(n: &Integer, effort: u32) -> (Factors, Vec<(Integer, u32)>) {
    let mut primes = Factors::new();
    let mut rest = Vec::new();
    let mut m = n.clone().abs();
    for &p in SMALL_PRIMES.iter() {
        if m < u64::from(p) * u64::from(p) {
            break;
        }
        if m.is_divisible_u(p) {
            let p = Integer::from(p);
            let e = m.remove_factor_mut(&p);
            primes.insert(p, e);
        }
    }
    let mut stack = vec![(m, 1u32)];
    while let Some((c, k)) = stack.pop() {
        if c == 1 {
            continue;
        }
        if is_prime(&c) {
            *primes.entry(c).or_insert(0) += k;
        } else if effort == 0 {
            rest.push((c, k));
        } else if let Some(x) = c.to_u64() {
            let mut found = Vec::new();
            factor_u64(x, &mut found);
            for p in found {
                *primes.entry(Integer::from(p)).or_insert(0) += k;
            }
        } else if let Some((b, e)) = is_perfect_power(&c) {
            stack.push((b, k * e));
        } else if let Some(d) = find_factor(&c, effort) {
            stack.push((c.div_exact_ref(&d).complete(), k));
            stack.push((d, k));
        } else {
            rest.push((c, k));
        }
    }
    (primes, rest)
}

pub fn factor_limited(n: &Integer, effort: u32) -> (Vec<(Integer, u32)>, Option<Integer>) {
    if n.is_zero() {
        return (Vec::new(), Some(Integer::new()));
    }
    let (primes, rest) = factor_parts(n, effort);
    let cofactor = rest
        .iter()
        .map(|(c, k)| c.pow(k).complete())
        .reduce(|a, b| a * b);
    (primes.into_iter().collect(), cofactor)
}

pub fn factor(n: &Integer) -> Option<Vec<(Integer, u32)>> {
    match factor_limited(n, 2) {
        (primes, None) => Some(primes),
        _ => None,
    }
}

pub fn is_perfect_power(n: &Integer) -> Option<(Integer, u32)> {
    let mut b = n.clone().abs();
    let mut k = 1u32;
    let (mut root, mut rem) = (Integer::new(), Integer::new());
    while b > 1 && b.is_perfect_power() {
        let v = if b.is_even() {
            b.find_one(0).unwrap_or(0)
        } else {
            let q = [3u32, 5, 7, 11, 13]
                .into_iter()
                .find(|&q| b.is_divisible_u(q));
            q.map_or(0, |q| b.remove_factor_ref(&Integer::from(q)).complete().1)
        };
        let exact = (2..b.significant_bits())
            .filter(|&p| v.is_multiple_of(p) && is_prime_u64(p.into()))
            .find(|&p| {
                (&mut root, &mut rem).assign(b.root_rem_ref(p));
                rem.is_zero()
            });
        let Some(p) = exact else { break };
        mem::swap(&mut b, &mut root);
        k *= p;
    }
    if n.is_negative() {
        while k.is_multiple_of(2) {
            b.square_mut();
            k /= 2;
        }
        b = -b;
    }
    (k > 1).then_some((b, k))
}

pub fn divisors(n: &Integer) -> Option<Vec<Integer>> {
    let f = factor(n)?;
    let mut count = 1u64;
    for (_, e) in &f {
        count *= u64::from(*e) + 1;
        if count > MAX_DIVISORS {
            return None;
        }
    }
    let mut divs = vec![Integer::from(1)];
    for (p, e) in &f {
        for i in 0..divs.len() * *e as usize {
            let d = (&divs[i] * p).complete();
            divs.push(d);
        }
    }
    divs.sort();
    Some(divs)
}

pub fn divisor_sigma(k: u32, n: &Integer) -> Option<Integer> {
    let mut sigma = Integer::from(1);
    for (p, e) in factor(n)? {
        sigma *= if k == 0 {
            Integer::from(e) + 1u32
        } else {
            let top = Integer::from((&p).pow(k.checked_mul(e.checked_add(1)?)?)) - 1u32;
            top.div_exact(&(p.pow(k) - 1u32))
        };
    }
    Some(sigma)
}

pub fn totient(n: &Integer) -> Option<Integer> {
    let f = factor(n)?;
    Some(
        f.into_iter()
            .map(|(p, e)| Integer::from(&p - 1u32) * p.pow(e - 1))
            .product(),
    )
}

pub fn mobius(n: &Integer) -> Option<i32> {
    if n.is_zero() {
        return None;
    }
    for effort in 0..=2 {
        let (primes, rest) = factor_parts(n, effort);
        let mut exponents = primes.values().chain(rest.iter().map(|(_, k)| k));
        if exponents.any(|&e| e > 1) {
            return Some(0);
        }
        if rest.is_empty() {
            return Some(if primes.len().is_multiple_of(2) {
                1
            } else {
                -1
            });
        }
    }
    None
}

pub fn omega(n: &Integer) -> Option<u32> {
    Some(factor(n)?.len() as u32)
}

pub fn big_omega(n: &Integer) -> Option<u32> {
    Some(factor(n)?.iter().map(|(_, e)| e).sum())
}

pub fn radical(n: &Integer) -> Option<Integer> {
    Some(factor(n)?.into_iter().map(|(p, _)| p).product())
}

pub fn is_squarefree(n: &Integer) -> Option<bool> {
    if n.is_zero() {
        Some(false)
    } else {
        mobius(n).map(|m| m != 0)
    }
}

pub fn extended_gcd(a: &Integer, b: &Integer) -> (Integer, Integer, Integer) {
    a.extended_gcd_ref(b).complete()
}

pub fn mod_inverse(a: &Integer, m: &Integer) -> Option<Integer> {
    a.invert_ref(m).map(Integer::from)
}

pub fn pow_mod(base: &Integer, exp: &Integer, m: &Integer) -> Option<Integer> {
    base.pow_mod_ref(exp, m).map(Integer::from)
}

pub fn jacobi(a: &Integer, n: &Integer) -> Option<i32> {
    (n.is_odd() && n.is_positive()).then(|| a.jacobi(n))
}

pub fn kronecker(a: &Integer, n: &Integer) -> i32 {
    a.kronecker(n)
}

fn sqrt_mod_prime(a: &Integer, p: &Integer) -> Option<Integer> {
    let a = a.rem_euc(p).complete();
    if *p == 2 || a.is_zero() {
        return Some(a);
    }
    if a.jacobi(p) != 1 {
        return None;
    }
    let pm1 = Integer::from(p - 1u32);
    let s = pm1.find_one(0)?;
    let q = Integer::from(&pm1 >> s);
    let z = (2u32..).map(Integer::from).find(|z| z.jacobi(p) == -1)?;
    let mut c = z.pow_mod(&q, p).ok()?;
    let mut r = a
        .pow_mod_ref(&(Integer::from(&q + 1u32) >> 1u32), p)
        .map(Integer::from)?;
    let mut t = a.pow_mod_ref(&q, p).map(Integer::from)?;
    let mut m = s;
    while t != 1 {
        let mut i = 0;
        let mut t2 = t.clone();
        while t2 != 1 {
            t2 = t2.square() % p;
            i += 1;
        }
        for _ in 0..m - i - 1 {
            c = c.square() % p;
        }
        r = r * &c % p;
        c = c.square() % p;
        t = t * &c % p;
        m = i;
    }
    let other = Integer::from(p - &r);
    let r = r.min(other);
    (Integer::from(r.square_ref()) - a)
        .is_divisible(p)
        .then_some(r)
}

/// The roots of `y^2 = u (mod p^k)` for `p` not dividing `u`.
fn unit_sqrts(u: &Integer, p: &Integer, k: u32) -> Vec<Integer> {
    let pk = Integer::from(p.pow(k));
    if *p == 2 {
        return match k {
            1 => vec![Integer::from(1)],
            2 if u.mod_u(4) == 1 => vec![Integer::from(1), Integer::from(3)],
            _ if k >= 3 && u.mod_u(8) == 1 => {
                let mut y = Integer::from(1);
                for j in 3..k {
                    let off = (Integer::from(y.square_ref()) - u).keep_bits(j + 1);
                    if !off.is_zero() {
                        y += Integer::from(1) << (j - 1);
                    }
                }
                let half = Integer::from(&pk >> 1u32);
                let mut all = vec![
                    y.clone(),
                    Integer::from(&pk - &y),
                    Integer::from(&y + &half) % &pk,
                    (Integer::from(&pk - &y) + &half) % &pk,
                ];
                all.sort();
                all
            }
            _ => Vec::new(),
        };
    }
    let Some(mut r) = sqrt_mod_prime(u, p) else {
        return Vec::new();
    };
    let mut modulus = p.clone();
    for _ in 1..k {
        modulus *= p;
        let Ok(inv) = Integer::from(&r << 1u32).invert(&modulus) else {
            return Vec::new();
        };
        r = (&r - (Integer::from(r.square_ref()) - u) * inv).rem_euc(&modulus);
    }
    let mut all = vec![Integer::from(&pk - &r), r];
    all.sort();
    all
}

/// The roots of `x^2 = a (mod p^e)` in `[0, p^e)`; `None` when there are more than `limit`.
fn prime_power_sqrts(a: &Integer, p: &Integer, e: u32, limit: usize) -> Option<Vec<Integer>> {
    let pe = Integer::from(p.pow(e));
    let a = a.rem_euc(&pe).complete();
    let (base, step, count) = if a.is_zero() {
        let half = e.div_ceil(2);
        (
            vec![Integer::new()],
            Integer::from(p.pow(half)),
            Integer::from(p.pow(e - half)),
        )
    } else {
        let mut u = a.clone();
        let v = u.remove_factor_mut(p);
        if v % 2 == 1 {
            return Some(Vec::new());
        }
        let scale = Integer::from(p.pow(v / 2));
        (
            unit_sqrts(&u, p, e - v)
                .into_iter()
                .map(|y| y * &scale)
                .collect(),
            Integer::from(p.pow(e - v / 2)),
            scale,
        )
    };
    let count = count.to_usize().filter(|c| c * base.len() <= limit)?;
    let (step, pe) = (&step, &pe);
    let mut all: Vec<Integer> = base
        .iter()
        .flat_map(|x| (0..count as u64).map(move |t| (Integer::from(step * t) + x) % pe))
        .collect();
    all.sort();
    all.dedup();
    Some(all)
}

/// All `x` in `[0, |m|)` with `x^2 = a (mod m)`, in increasing order.
/// `None` when `m` is zero or cannot be factored, or when there are more than `limit` roots.
pub fn sqrt_mod_all(a: &Integer, m: &Integer, limit: usize) -> Option<Vec<Integer>> {
    if m.is_zero() {
        return None;
    }
    let mut roots = vec![(Integer::new(), Integer::from(1))];
    for (p, e) in factor(m)? {
        let local = prime_power_sqrts(a, &p, e, limit)?;
        if local.len() * roots.len() > limit {
            return None;
        }
        let pe = Integer::from((&p).pow(e));
        roots = roots
            .iter()
            .flat_map(|x| {
                local
                    .iter()
                    .filter_map(|r| crt(&[x.clone(), (r.clone(), pe.clone())]))
                    .collect::<Vec<_>>()
            })
            .collect();
        if roots.is_empty() {
            break;
        }
    }
    let mut all: Vec<Integer> = roots.into_iter().map(|(x, _)| x).collect();
    all.sort();
    Some(all)
}

/// The smallest `x >= 0` with `x^2 = a (mod m)`.
pub fn sqrt_mod(a: &Integer, m: &Integer) -> Option<Integer> {
    if is_prime(m) {
        return sqrt_mod_prime(a, m);
    }
    sqrt_mod_all(a, m, 1 << 16)?.into_iter().next()
}

pub fn crt(residues: &[(Integer, Integer)]) -> Option<(Integer, Integer)> {
    let (mut x, mut m) = (Integer::new(), Integer::from(1));
    for (r, mi) in residues {
        if mi.is_zero() {
            return None;
        }
        let (g, s, _) = extended_gcd(&m, mi);
        let diff = Integer::from(r - &x);
        if !diff.is_divisible(&g) {
            return None;
        }
        let step = mi.clone().abs().div_exact(&g);
        let k = (diff.div_exact(&g) * s).rem_euc(&step);
        x += k * &m;
        m *= step;
    }
    Some((x, m))
}

pub fn multiplicative_order(a: &Integer, n: &Integer) -> Option<Integer> {
    if n.is_zero() || a.gcd_ref(n).complete() != 1 {
        return None;
    }
    let mut order = Integer::from(1);
    for (p, e) in factor(n)? {
        let pm1 = Integer::from(&p - 1u32);
        let pe = Integer::from((&p).pow(e));
        let mut ord = Integer::from(&pe / &p) * &pm1;
        let mut qs: Vec<Integer> = factor(&pm1)?.into_iter().map(|(q, _)| q).collect();
        if e > 1 {
            qs.push(p);
        }
        for q in qs {
            while ord.is_divisible(&q) {
                let t = Integer::from(&ord / &q);
                if a.pow_mod_ref(&t, &pe).map(Integer::from)? != 1 {
                    break;
                }
                ord = t;
            }
        }
        order.lcm_mut(&ord);
    }
    Some(order)
}

pub fn primitive_root(n: &Integer) -> Option<Integer> {
    let n = n.clone().abs();
    if n.is_zero() {
        return None;
    }
    if n <= 2 || n == 4 {
        return Some(n - 1u32);
    }
    let odd = if n.is_even() {
        Integer::from(&n >> 1u32)
    } else {
        n.clone()
    };
    let p = if is_prime(&odd) {
        odd.clone()
    } else {
        is_perfect_power(&odd)?.0
    };
    if p.is_even() || !is_prime(&p) {
        return None;
    }
    let pm1 = Integer::from(&p - 1u32);
    let phi = Integer::from(&odd / &p) * &pm1;
    let mut qs: Vec<Integer> = factor(&pm1)?.into_iter().map(|(q, _)| q).collect();
    if odd != p {
        qs.push(p);
    }
    let exps: Vec<Integer> = qs.iter().map(|q| Integer::from(&phi / q)).collect();
    (2u32..).map(Integer::from).find(|g| {
        g.gcd_ref(&n).complete() == 1
            && exps
                .iter()
                .all(|e| g.pow_mod_ref(e, &n).map(Integer::from) != Some(Integer::from(1)))
    })
}

fn bsgs<T: Clone + Eq + Hash>(
    g: &T,
    h: &T,
    one: T,
    bound: u64,
    max_baby: u64,
    mul: impl Fn(&T, &T) -> T,
) -> Option<u64> {
    let baby = (bound.isqrt() + 1).min(max_baby);
    let mut table = HashMap::with_capacity(baby as usize);
    let (mut cur, mut step) = (h.clone(), one.clone());
    for j in 0..baby {
        let next = mul(&cur, g);
        table.insert(mem::replace(&mut cur, next), j);
        step = mul(&step, g);
    }
    let mut cur = one;
    (1..=bound.div_ceil(baby)).find_map(|i| {
        cur = mul(&cur, &step);
        table.get(&cur).map(|j| i * baby - j)
    })
}

/// The exponent in `[0, order)` with `g^x = h (mod m)` for `gcd(g, m) = 1`, by Pohlig-Hellman.
/// `Err(true)`: there is none, `Err(false)`: the search gave up.
fn dlog_coprime(g: &Integer, h: &Integer, m: &Integer) -> Result<Integer, bool> {
    let power = |b: &Integer, e: &Integer| b.pow_mod_ref(e, m).map(Integer::from).ok_or(false);
    let order = multiplicative_order(g, m).ok_or(false)?;
    let max_baby = match m.to_u64() {
        Some(_) => 1 << 20,
        None => (DLOG_TABLE_BITS / u64::from(m.significant_bits())).clamp(16, 1 << 18),
    };
    let mut parts = Vec::new();
    for (q, e) in factor(&order).ok_or(false)? {
        let small = q.to_u64().filter(|&q| q <= DLOG_MAX_ORDER).ok_or(false)?;
        let qe = Integer::from((&q).pow(e));
        let cofactor = Integer::from(&order / &qe);
        let (gi, hi) = (power(g, &cofactor)?, power(h, &cofactor)?);
        let gi_inv = Integer::from(gi.invert_ref(m).ok_or(false)?);
        let gamma = power(&gi, &Integer::from(&qe / &q))?;
        let (mut x, mut scale) = (Integer::new(), Integer::from(1));
        for k in 0..e {
            let rest = Integer::from(&hi * &power(&gi_inv, &x)?) % m;
            let target = power(&rest, &Integer::from((&q).pow(e - 1 - k)))?;
            let digit = if target == 1 {
                0
            } else {
                bsgs(
                    &gamma,
                    &target,
                    Integer::from(1) % m,
                    small,
                    max_baby,
                    |a, b| Integer::from(a * b) % m,
                )
                .ok_or(true)?
            };
            x += Integer::from(&scale * digit);
            scale *= &q;
        }
        parts.push((x, qe));
    }
    let (x, _) = crt(&parts).ok_or(false)?;
    if power(g, &x)? == *h {
        Ok(x)
    } else {
        Err(true)
    }
}

/// The smallest `x >= 0` with `g^x = h (mod m)`.
/// `Err(true)`: no such exponent exists, `Err(false)`: the search gave up.
pub fn discrete_log_search(g: &Integer, h: &Integer, m: &Integer) -> Result<Integer, bool> {
    let mut m = m.clone().abs();
    if m.is_zero() {
        return Err(true);
    }
    let (g, mut h) = (g.rem_euc(&m).complete(), h.rem_euc(&m).complete());
    let mut c = Integer::from(1) % &m;
    let mut k = 0u32;
    loop {
        if c == h {
            return Ok(Integer::from(k));
        }
        let d = g.gcd_ref(&m).complete();
        if d == 1 {
            break;
        }
        if !h.is_divisible(&d) {
            return Err(true);
        }
        m.div_exact_mut(&d);
        h.div_exact_mut(&d);
        c = c * Integer::from(&g / &d) % &m;
        k += 1;
    }
    let g = g % &m;
    let target = h * Integer::from(c.invert_ref(&m).ok_or(false)?) % &m;
    Ok(dlog_coprime(&g, &target, &m)? + k)
}

pub fn discrete_log(g: &Integer, h: &Integer, m: &Integer) -> Option<Integer> {
    discrete_log_search(g, h, m).ok()
}

thread_local! {
    static ZIGZAG: RefCell<(Vec<Integer>, Vec<Integer>)> =
        RefCell::new((vec![Integer::from(1)], vec![Integer::from(1)]));
    static PARTITIONS: RefCell<Vec<Integer>> = RefCell::new(vec![Integer::from(1)]);
}

fn zigzag(n: u32, cache: usize) -> Integer {
    ZIGZAG.with_borrow_mut(|(values, row)| {
        while values.len() <= n as usize {
            row.reverse();
            row.insert(0, Integer::new());
            for k in 1..row.len() {
                let (done, todo) = row.split_at_mut(k);
                todo[0] += &done[k - 1];
            }
            values.push(row[row.len() - 1].clone());
        }
        let value = values[n as usize].clone();
        if values.len() > cache {
            values.truncate(1);
            row.truncate(1);
            row[0].assign(1);
        }
        value
    })
}

/// Panics if `|n| > u32::MAX`.
pub fn fibonacci(n: i64) -> Integer {
    assert!(
        n.unsigned_abs() <= u64::from(u32::MAX),
        "fibonacci: index out of range"
    );
    let f = Integer::fibonacci(n.unsigned_abs() as u32).complete();
    if n < 0 && n % 2 == 0 { -f } else { f }
}

/// Panics if `|n| > u32::MAX`.
pub fn lucas(n: i64) -> Integer {
    assert!(
        n.unsigned_abs() <= u64::from(u32::MAX),
        "lucas: index out of range"
    );
    let l = Integer::lucas(n.unsigned_abs() as u32).complete();
    if n < 0 && n % 2 != 0 { -l } else { l }
}

pub fn factorial(n: u32) -> Integer {
    Integer::factorial(n).complete()
}

pub fn double_factorial(n: i64) -> Option<Integer> {
    match n {
        -1 => Some(Integer::from(1)),
        _ => u32::try_from(n)
            .ok()
            .map(|n| Integer::factorial_2(n).complete()),
    }
}

pub fn binomial(n: &Integer, k: &Integer) -> Option<Integer> {
    if k.is_negative() {
        return Some(Integer::new());
    }
    let (top, negate) = if n.is_negative() {
        (Integer::from(k - n) - 1u32, k.is_odd())
    } else {
        (n.clone(), false)
    };
    if *k > top {
        return Some(Integer::new());
    }
    let rest = Integer::from(&top - k);
    let b = top.binomial(k.min(&rest).to_u32()?);
    Some(if negate { -b } else { b })
}

pub fn multinomial(ks: &[u32]) -> Integer {
    let mut total = Integer::new();
    ks.iter()
        .map(|&k| {
            total += k;
            total.clone().binomial(k)
        })
        .product()
}

pub fn bernoulli(n: u32) -> Rational {
    match n {
        0 => Rational::from(1),
        1 => Rational::from((-1, 2)),
        _ if n % 2 == 1 => Rational::new(),
        _ => {
            let pow = Integer::from(1) << n;
            let b = Rational::from((
                zigzag(n - 1, ZIGZAG_CACHE) * n,
                Integer::from(&pow - 1u32) * pow,
            ));
            if n.is_multiple_of(4) { -b } else { b }
        }
    }
}

pub fn euler_number(n: u32) -> Integer {
    match n % 4 {
        0 => zigzag(n, ZIGZAG_CACHE),
        2 => -zigzag(n, ZIGZAG_CACHE),
        _ => Integer::new(),
    }
}

pub fn catalan_number(n: u32) -> Integer {
    Integer::from(2 * u64::from(n)).binomial(n) / (u64::from(n) + 1)
}

pub fn bell_number(n: u32) -> Integer {
    let mut row = vec![Integer::from(1)];
    for _ in 1..n {
        row.push(row[0].clone());
        for i in (0..row.len() - 1).rev() {
            let (low, high) = row.split_at_mut(i + 1);
            low[i] += &high[0];
        }
    }
    row.swap_remove(0)
}

pub fn stirling1(n: u32, k: u32) -> Integer {
    if k > n {
        return Integer::new();
    }
    let (n, k) = (n as usize, k as usize);
    let mut row = vec![Integer::new(); k + 1];
    row[0] = Integer::from(1);
    for i in 1..=n {
        for j in (k.saturating_sub(n - i).max(1)..=i.min(k)).rev() {
            let (low, high) = row.split_at_mut(j);
            high[0] *= i as u64 - 1;
            high[0] += &low[j - 1];
        }
        row[0] = Integer::new();
    }
    let s = row.swap_remove(k);
    if (n - k) % 2 == 1 { -s } else { s }
}

pub fn stirling2(n: u32, k: u32) -> Integer {
    if k > n {
        return Integer::new();
    }
    let (mut sum, mut c) = (Integer::new(), Integer::from(1));
    for j in 0..=k {
        let term = Integer::from(j).pow(n) * &c;
        if (k - j).is_multiple_of(2) {
            sum += term;
        } else {
            sum -= term;
        }
        if j < k {
            c *= k - j;
            c.div_exact_u_mut(j + 1);
        }
    }
    sum.div_exact(&factorial(k))
}

fn partitions_cached(n: u32, cache: usize) -> Integer {
    PARTITIONS.with_borrow_mut(|p| {
        for m in p.len()..=n as usize {
            let mut sum = Integer::new();
            for k in 1.. {
                let first = k * (3 * k - 1) / 2;
                if first > m {
                    break;
                }
                let second = first + k;
                if k % 2 == 1 {
                    sum += &p[m - first];
                    if second <= m {
                        sum += &p[m - second];
                    }
                } else {
                    sum -= &p[m - first];
                    if second <= m {
                        sum -= &p[m - second];
                    }
                }
            }
            p.push(sum);
        }
        let value = p[n as usize].clone();
        p.truncate(cache.max(1));
        value
    })
}

pub fn partitions(n: u32) -> Integer {
    partitions_cached(n, PARTITION_CACHE)
}

pub fn harmonic(n: u32) -> Rational {
    fn split(a: u64, b: u64) -> (Integer, Integer) {
        if b - a == 1 {
            return (Integer::from(1), Integer::from(a));
        }
        let mid = a.midpoint(b);
        let ((p1, q1), (p2, q2)) = (split(a, mid), split(mid, b));
        (p1 * &q2 + p2 * &q1, q1 * q2)
    }
    if n == 0 {
        Rational::new()
    } else {
        Rational::from(split(1, u64::from(n) + 1))
    }
}

pub fn derangements(n: u32) -> Integer {
    let mut d = Integer::from(1);
    for i in 1..=n {
        d *= i;
        if i.is_multiple_of(2) {
            d += 1u32;
        } else {
            d -= 1u32;
        }
    }
    d
}

pub fn continued_fraction(r: &Rational) -> Vec<Integer> {
    let (mut n, mut d) = (r.numer().clone(), r.denom().clone());
    let mut terms = Vec::new();
    while !d.is_zero() {
        let (q, rem) = n.div_rem_floor_ref(&d).complete();
        terms.push(q);
        n = mem::replace(&mut d, rem);
    }
    terms
}

fn convergent_pairs(terms: &[Integer]) -> impl Iterator<Item = (Integer, Integer)> {
    let mut prev = [
        Integer::from(1),
        Integer::new(),
        Integer::new(),
        Integer::from(1),
    ];
    terms.iter().map(move |a| {
        let p = Integer::from(a * &prev[0]) + &prev[2];
        let q = Integer::from(a * &prev[1]) + &prev[3];
        let [p1, q1, ..] = mem::take(&mut prev);
        prev = [p.clone(), q.clone(), p1, q1];
        (p, q)
    })
}

pub fn from_continued_fraction(terms: &[Integer]) -> Option<Rational> {
    let (p, q) = convergent_pairs(terms).last()?;
    (!q.is_zero()).then(|| Rational::from((p, q)))
}

pub fn convergents(terms: &[Integer]) -> Vec<Rational> {
    let all: Option<Vec<Rational>> = convergent_pairs(terms)
        .map(|(p, q)| (!q.is_zero()).then(|| Rational::from((p, q))))
        .collect();
    all.unwrap_or_default()
}

/// Panics if `max_den < 1`.
pub fn best_rational(x: &Rational, max_den: &Integer) -> Rational {
    assert!(*max_den >= 1, "best_rational: max_den must be positive");
    if x.denom() <= max_den {
        return x.clone();
    }
    let (mut p0, mut q0, mut p1, mut q1) = (
        Integer::new(),
        Integer::from(1),
        Integer::from(1),
        Integer::new(),
    );
    let (mut n, mut d) = (x.numer().clone(), x.denom().clone());
    loop {
        let (a, rem) = n.div_rem_floor_ref(&d).complete();
        let q2 = Integer::from(&a * &q1) + &q0;
        if q2 > *max_den {
            break;
        }
        let p2 = a * &p1 + &p0;
        (p0, q0, p1, q1) = (p1, q1, p2, q2);
        n = mem::replace(&mut d, rem);
    }
    let k = Integer::from(max_den - &q0) / &q1;
    let semi = Rational::from((p0 + Integer::from(&k * &p1), q0 + k * &q1));
    let conv = Rational::from((p1, q1));
    if Rational::from(&conv - x).abs() <= Rational::from(&semi - x).abs() {
        conv
    } else {
        semi
    }
}

pub fn isqrt(n: &Integer) -> Option<Integer> {
    (!n.is_negative()).then(|| n.sqrt_ref().complete())
}

pub fn iroot(n: &Integer, k: u32) -> Option<Integer> {
    (k > 0 && (k % 2 == 1 || !n.is_negative())).then(|| n.root_ref(k).complete())
}

pub fn is_square(n: &Integer) -> bool {
    n.is_perfect_square()
}

pub fn to_base(n: &Integer, base: u32) -> Option<String> {
    (2..=36)
        .contains(&base)
        .then(|| n.to_string_radix(base as i32))
}

pub fn from_base(s: &str, base: u32) -> Option<Integer> {
    let digits = s.strip_prefix(['-', '+']).unwrap_or(s);
    let valid =
        (2..=36).contains(&base) && !digits.is_empty() && digits.chars().all(|c| c.is_digit(base));
    if valid {
        Integer::from_str_radix(s, base as i32).ok()
    } else {
        None
    }
}

pub fn digit_sum(n: &Integer, base: u32) -> Option<Integer> {
    if base < 2 {
        return None;
    }
    if base <= 36 {
        let digits = n.to_string_radix(base as i32);
        return Some(
            digits
                .chars()
                .filter_map(|c| c.to_digit(base))
                .map(u64::from)
                .sum::<u64>()
                .into(),
        );
    }
    let (mut m, mut sum) = (n.clone().abs(), Integer::new());
    while !m.is_zero() {
        sum += m.mod_u(base);
        m /= base;
    }
    Some(sum)
}

pub fn sum_of_two_squares(n: &Integer) -> Option<(Integer, Integer)> {
    if n.is_negative() {
        return None;
    }
    if n.is_zero() {
        return Some((Integer::new(), Integer::new()));
    }
    let (mut re, mut im) = (Integer::from(1), Integer::new());
    for (p, e) in factor(n)? {
        let (x, y) = if p == 2 {
            (Integer::from(1), Integer::from(1))
        } else if p.mod_u(4) == 3 {
            if e % 2 == 1 {
                return None;
            }
            let scale = p.pow(e / 2);
            re *= &scale;
            im *= scale;
            continue;
        } else {
            let (mut a, mut b) = (p.clone(), sqrt_mod_prime(&Integer::from(-1), &p)?);
            while Integer::from(b.square_ref()) > p {
                let rem = Integer::from(&a % &b);
                a = mem::replace(&mut b, rem);
            }
            (a % &b, b)
        };
        for _ in 0..e {
            let next_re = Integer::from(&re * &x) - Integer::from(&im * &y);
            im = im * &x + re * &y;
            re = next_re;
        }
    }
    let (a, b) = (re.abs(), im.abs());
    Some(if a <= b { (a, b) } else { (b, a) })
}

/// Carmichael's function: the exponent of the group of units modulo `n`.
pub fn carmichael_lambda(n: &Integer) -> Option<Integer> {
    let mut lambda = Integer::from(1);
    for (p, e) in factor(n)? {
        let part = if p == 2 {
            Integer::from(1) << if e < 3 { e - 1 } else { e - 2 }
        } else {
            Integer::from((&p).pow(e - 1)) * (p - 1u32)
        };
        lambda.lcm_mut(&part);
    }
    Some(lambda)
}

/// The primes in `[lo, hi]`; `None` when there are more than `limit`.
pub fn primes_between(lo: &Integer, hi: &Integer, limit: usize) -> Option<Vec<Integer>> {
    let first = lo.clone().max(Integer::from(2));
    if let (Some(a), Some(b)) = (first.to_u64(), hi.to_u64())
        && b <= PRIME_PI_MAX
        && b.saturating_sub(a) <= 50_000_000
    {
        let found = sieve_range(a, b);
        return (found.len() <= limit).then(|| found.into_iter().map(Integer::from).collect());
    }
    let mut out = Vec::new();
    let mut p = next_prime(&(first - 1u32));
    while p <= *hi {
        if out.len() == limit {
            return None;
        }
        let next = next_prime(&p);
        out.push(mem::replace(&mut p, next));
    }
    Some(out)
}

/// The Farey sequence of order `n`: the reduced fractions in `[0, 1]` with denominators up to `n`.
pub fn farey(n: u32) -> Vec<Rational> {
    let n = i64::from(n.max(1));
    let (mut a, mut b, mut c, mut d) = (0, 1, 1, n);
    let mut out = vec![Rational::new()];
    while c <= n {
        let k = (n + b) / d;
        (a, b, c, d) = (c, d, k * c - a, k * d - b);
        out.push(Rational::from((a, b)));
    }
    out
}

/// The number of ways to write `n` as an ordered sum of `d` squares of integers.
pub fn squares_r(d: u32, n: &Integer) -> Option<Integer> {
    if n.is_negative() {
        return Some(Integer::new());
    }
    if n.is_zero() {
        return Some(Integer::from(1));
    }
    match d {
        0 => Some(Integer::new()),
        1 => Some(Integer::from(if is_square(n) { 2 } else { 0 })),
        2 => {
            let mut r = Integer::from(4);
            for (p, e) in factor(n)? {
                match p.mod_u(4) {
                    1 => r *= e + 1,
                    3 if e % 2 == 1 => return Some(Integer::new()),
                    _ => {}
                }
            }
            Some(r)
        }
        4 => {
            let odd = Integer::from(n >> n.find_one(0)?);
            Some(divisor_sigma(1, &odd)? * if n.is_odd() { 8u32 } else { 24u32 })
        }
        8 => {
            let odd = Integer::from(n >> n.find_one(0)?);
            let all = divisor_sigma(3, n)?;
            Some(if n.is_odd() {
                all * 16u32
            } else {
                (all - divisor_sigma(3, &odd)? * 2u32) * 16u32
            })
        }
        _ => {
            let n = n
                .to_usize()
                .filter(|&n| (n as f64).powf(1.5) * f64::from(d) <= 4e8 && d <= 10_000)?;
            let mut series = vec![Integer::new(); n + 1];
            series[0] = Integer::from(1);
            for _ in 0..d {
                for m in (1..=n).rev() {
                    let mut extra = Integer::new();
                    let mut k = 1;
                    while k * k <= m {
                        extra += &series[m - k * k];
                        k += 1;
                    }
                    series[m] += extra << 1u32;
                }
            }
            series.pop()
        }
    }
}

/// The number of partitions of `n` into distinct parts.
pub fn partitions_distinct(n: u32) -> Integer {
    let n = n as usize;
    let mut ways = vec![Integer::new(); n + 1];
    ways[0] = Integer::from(1);
    for part in 1..=n {
        for total in (part..=n).rev() {
            let smaller = ways[total - part].clone();
            ways[total] += smaller;
        }
    }
    ways.swap_remove(n)
}

/// The Eulerian number: permutations of `n` elements with `k` ascents.
pub fn eulerian(n: u32, k: u32) -> Integer {
    if n == 0 {
        return Integer::from(u32::from(k == 0));
    }
    if k >= n {
        return Integer::new();
    }
    let mut sum = Integer::new();
    for j in 0..=k + 1 {
        let term = Integer::from(n + 1).binomial(j) * Integer::from(k + 1 - j).pow(n);
        if j % 2 == 0 {
            sum += term;
        } else {
            sum -= term;
        }
    }
    sum
}

/// The smallest solution `(x, y)` with `y > 0` of `x^2 - d y^2 = 1`.
pub fn pell(d: &Integer) -> Option<(Integer, Integer)> {
    if !d.is_positive() || is_square(d) {
        return None;
    }
    let a0 = d.sqrt_ref().complete();
    let (mut m, mut den, mut a) = (Integer::new(), Integer::from(1), a0.clone());
    let (mut p_prev, mut p) = (Integer::from(1), a0.clone());
    let (mut q_prev, mut q) = (Integer::new(), Integer::from(1));
    for _ in 0..2_000_000 {
        if Integer::from(p.square_ref()) - Integer::from(q.square_ref()) * d == 1 {
            return Some((p, q));
        }
        m = Integer::from(&den * &a) - m;
        den = (d - Integer::from(m.square_ref())) / den;
        a = Integer::from(&a0 + &m) / &den;
        let next_p = Integer::from(&a * &p) + &p_prev;
        p_prev = mem::replace(&mut p, next_p);
        let next_q = Integer::from(&a * &q) + &q_prev;
        q_prev = mem::replace(&mut q, next_q);
    }
    None
}

/// The greedy expansion of `x` into a whole part and distinct unit fractions.
pub fn egyptian(x: &Rational, max_terms: usize) -> Option<Vec<Rational>> {
    if x.is_negative() {
        return None;
    }
    let whole = x.clone().floor();
    let mut rest = Rational::from(x - &whole);
    let mut out = Vec::new();
    if !whole.is_zero() {
        out.push(whole);
    }
    while !rest.is_zero() {
        if out.len() == max_terms {
            return None;
        }
        let (den, _) = rest.denom().div_rem_ceil_ref(rest.numer()).complete();
        let unit = Rational::from((Integer::from(1), den));
        rest -= &unit;
        out.push(unit);
    }
    Some(out)
}

/// All primitive roots modulo `n` in increasing order; `None` when there are more than `limit`.
pub fn all_primitive_roots(n: &Integer, limit: usize) -> Option<Vec<Integer>> {
    let n = n.clone().abs();
    let Some(g) = primitive_root(&n) else {
        return (!n.is_zero()).then(Vec::new);
    };
    if n <= 2 {
        return Some(vec![g]);
    }
    let phi = totient(&n)?;
    if totient(&phi)? > limit {
        return None;
    }
    let phi = phi.to_u64()?;
    let mut all = Vec::new();
    let mut power = Integer::from(1);
    for k in 1..phi {
        power = power * &g % &n;
        if gcd_u64(k, phi) == 1 {
            all.push(power.clone());
        }
    }
    all.sort();
    Some(all)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    const FACTOR_TABLE: &str = "
        18446744073709551615 3^1 5^1 17^1 257^1 641^1 65537^1 6700417^1
        18446744073709551617 274177^1 67280421310721^1
        1000000000000000009 1000000000000000009^1
        340282366920938463463374607431768211455 3^1 5^1 17^1 257^1 641^1 65537^1 274177^1 670041\
        7^1 67280421310721^1
        340282366920938463463374607431768211457 59649589127497217^1 5704689200685129054721^1
        265252859812191058636308480000000 2^26 3^14 5^7 7^4 11^2 13^2 17^1 19^1 23^1 29^1
        1000000000000000000000000000001 61^1 101^1 3541^1 9901^1 27961^1 4188901^1 39526741^1
        100000000000000000001 73^1 137^1 1676321^1 5964848081^1
        1111111111111111111 1111111111111111111^1
        11111111111111111111111 11111111111111111111111^1
        9746347772161 7^1 11^1 13^1 17^1 19^1 31^1 37^1 41^1 641^1
        3825123056546413051 149491^1 747451^1 34233211^1
        318665857834031151167461 399165290221^1 798330580441^1
        618970019642690137449562111 618970019642690137449562111^1
        147573952589676412927 193707721^1 761838257287^1
        2535301200456458802993406410751 7432339208719^1 341117531003194129^1
        79228162514264337593543950337 641^1 6700417^1 18446744069414584321^1
        123456789012345678901234567890 2^1 3^3 5^1 7^1 13^1 31^1 37^1 211^1 241^1 2161^1 3607^1 \
        3803^1 2906161^1
        600851475143 71^1 839^1 1471^1 6857^1
        190513714613143829143824 2^4 3^5 7^2 1000003^3
        999999999950000000000429 999999999961^1 999999999989^1
        12345678901234567890123456789 3^3 7^1 13^1 31^1 37^1 211^1 241^1 2161^1 3607^1 3803^1 29\
        06161^1
        162259276829213363391578010288129 3^1 643^1 84115747449047881488635567801^1
        147808829414344714390263595577208591425 5^2 11^1 13^1 17^1 41^1 97^1 211^1 281^1 401^1 3\
        121^1 4621^1 1783433557073281^1
        10000000000000000000300000000000000000001 89^1 4178340421^1 13250071409^1 20294948530726\
        20581^1
    ";
    const SEMIPRIMES_15: &str = "
        838751130704297 329410655966303
        183813464482741 947319933382931
        353437507901953 455882627814259
        891839475179987 890035283249989
        344689291220977 358076009562199
        712118700672341 516633532368193
        334470881723819 500330789515451
        208622915988871 683531095338389
    ";
    const SEMIPRIMES_MIXED: &str = "
        526096899077 6640095957355092188247871
        25067741132701 7780695107640492924431349292065492346129
        8552992705923427 8767600403500163
        61570968198397907 19097191951168631
        6565287541 644231708267814161303794035785817031131602873568333878428079
        5855735721881 9295445914283
        29870807867 4822098614995704379399089845459
        566104738817159 324071108488284400783816973668028652307538574854439931186671085610868352\
        94396081
    ";
    const HARD_SEMIPRIME: (&str, &str) = (
        "875313513250145968038026669434706686666046213",
        "884602600630772878526113658324027178126255629",
    );
    const NEXT_PRIME_OFFSETS: [u32; 40] = [
        1, 1, 9, 7, 3, 3, 19, 7, 7, 19, 3, 39, 37, 31, 37, 61, 3, 3, 51, 39, 117, 9, 117, 7, 13,
        67, 103, 331, 319, 57, 33, 49, 61, 193, 69, 67, 43, 133, 3, 121,
    ];
    const PREV_PRIME_OFFSETS: [u32; 40] = [
        3, 3, 3, 27, 9, 17, 9, 11, 63, 33, 23, 11, 29, 27, 11, 63, 3, 11, 39, 11, 101, 27, 23, 257,
        123, 141, 99, 209, 27, 11, 27, 21, 9, 411, 23, 159, 81, 59, 57, 17,
    ];
    const PRIME_PI_TABLE: &[(u64, u64)] = &[
        (142, 34),
        (591, 107),
        (1920, 293),
        (3642, 509),
        (28994, 3153),
        (33345, 3570),
        (573033, 47063),
        (648647, 52722),
        (3430018, 245554),
        (6564316, 448875),
        (50563078, 3032882),
        (64284835, 3800933),
        (849865522, 43575442),
        (976888836, 49731416),
    ];
    const NTH_PRIME_TABLE: &[(u64, u64)] = &[
        (44, 193),
        (98, 521),
        (286, 1871),
        (457, 3229),
        (1421, 11839),
        (7885, 80599),
        (54261, 669377),
        (86021, 1104221),
        (693526, 10466669),
        (789472, 12023159),
    ];
    const ARITH_TABLE: &str = "
        1 1 1 1 1 1 0 0
        2 2 3 9 1 -1 1 1
        12 6 28 2044 4 0 2 3
        360 24 1170 55798470 96 0 3 6
        9973 2 9974 991921850318 9972 -1 1 1
        1000000000000 169 2499694822171 1152073732716798399304670671629509341 400000000000 0 2 2\
        4
        1099511627775 192 2220409884672 1390968480062546839764005389334065152 473702400000 0 7 8
        600851475143 16 610544148480 216921505300712022090801267027025920 591194251200 1 4 4
        901800900 729 3913638183 879272468031556107916427673 172972800 0 6 12
        999999000001 2 999999000002 999997000005999993000005999997000002 999999000000 -1 1 1
        1000000000000000009 2 1000000000000000010 1000000000000000027000000000000000243000000000\
        000000730 1000000000000000008 -1 1 1
        18446744073709551615 128 31421980989189888768 656299966396315658049897658316449695873961\
        5059396145582336 9208981628670443520 -1 7 7
        1000036000099 4 1000038000136 1000108004185068042414424061803006264 1000034000064 1 2 2
        6469693230 1024 25082265600 319927601860614968841569894400 1021870080 1 10 10
        2147483647 2 2147483648 9903520300447984150353281024 2147483646 -1 1 1
        720720 240 3249792 449671560269920128 138240 0 6 10
        1000000000000037 2 1000000000000038 1000000000000111000000000004107000000000050654 10000\
        00000000036 -1 1 1
        987654321987654321 576 2141890840189900800 100490058429257683593430794226289862178286042\
        4716851200 422272393825075200 0 8 10
    ";
    const KRONECKER_ROWS: [&str; 25] = [
        "0+000-0+000-0+000-0+000-0",
        "-0+-+++---+-0+-+++---+-0+",
        "0-0-0-000+0-0+0-000+0+0+0",
        "0+-0-+0--0--0++0++0-+0+-0",
        "0-0-0+0+0-0-0+0+0-0-0+0+0",
        "+-+--0++-+--0++-+--0++-+-",
        "0-000-0-000-0+000+0+000+0",
        "-+0-+-+0--+-0+-++0-+-+0-+",
        "0+0-0+0-0+0-0+0-0+0-0+0-0",
        "0+-0+-0+-0+-0+-0+-0+-0+-0",
        "0-0-0+0+0-0-0+0+0-0-0+0+0",
        "++---++--+--+++-++--+++--",
        "00000000000+0+00000000000",
        "+++++++++++++++++++++++++",
        "0-0+0+0-0-0+0+0-0-0+0+0-0",
        "0++0--0-+0-+0+-0+-0--0++0",
        "0+0+0+0+0+0+0+0+0+0+0+0+0",
        "-+0+--+0+--+0+--+0+--+0+-",
        "0-000-0+000+0+000+0-000-0",
        "+--++0+-++++0++++-+0++--+",
        "0-0+0+0-0-0+0+0-0-0+0+0-0",
        "0++0++0++0++0++0++0++0++0",
        "0-0+0-000+0+0+0+000-0+0-0",
        "-0-+-++++--+0+--++++-+-0-",
        "0+000-0-000+0+000-0-000+0",
    ];
    const KRONECKER_BIG: &str = "
        -452527365215874617881209760606 -433527842192923632201320512567 0
        -333617883898336988911790922828 729850818609557767292303020689 0
        80620652571402997403510760927 239124742733617911573737496663 0
        100677454249757519429836530876 563614317797129484686262509250 0
        -224780258352820739100257844218 716895964153287207148861425423 -1
        -760462058341105746246689372370 -393028889048614696304457684344 0
        789769665113052603953455893744 -744392628714960478924662718289 -1
        579097334718538414599853408824 380026654492240251395679510425 1
        261428663514797170183314192090 -586333578209041968990930032397 0
        530114085907851710566145922523 487376194347486274399096491823 -1
        835704315475559286106145234168 3293685372734302699685133101 -1
        -22881984139471800215715010008 -822637667232685695801813304806 0
    ";
    const PRIMITIVE_ROOTS: [u32; 199] = [
        1, 2, 3, 2, 5, 3, 0, 2, 3, 2, 0, 2, 3, 0, 0, 3, 5, 2, 0, 0, 7, 5, 0, 2, 7, 2, 0, 2, 0, 3,
        0, 0, 3, 0, 0, 2, 3, 0, 0, 6, 0, 3, 0, 0, 5, 5, 0, 3, 3, 0, 0, 2, 5, 0, 0, 0, 3, 2, 0, 2,
        3, 0, 0, 0, 0, 2, 0, 0, 0, 7, 0, 5, 5, 0, 0, 0, 0, 3, 0, 2, 7, 2, 0, 0, 3, 0, 0, 3, 0, 0,
        0, 0, 5, 0, 0, 5, 3, 0, 0, 2, 0, 5, 0, 0, 3, 2, 0, 6, 0, 0, 0, 3, 0, 0, 0, 0, 11, 0, 0, 2,
        7, 0, 0, 2, 0, 3, 0, 0, 0, 2, 0, 0, 7, 0, 0, 3, 0, 2, 0, 0, 7, 0, 0, 0, 5, 0, 0, 2, 0, 6,
        0, 0, 0, 0, 0, 5, 3, 0, 0, 0, 5, 2, 0, 0, 5, 5, 0, 2, 0, 0, 0, 2, 0, 0, 0, 0, 3, 2, 0, 2,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 19, 0, 5, 5, 0, 0, 2, 0, 3, 0,
    ];
    const PRIMITIVE_ROOTS_BIG: &str = "
        2305843009213693951:37 1000000007:5 998244353:3 618970019642690137449562111:3
        12157665459056928801:2 2682137239327929801614:3 1000000000000000009:7
        18446744069414584321:7
    ";
    const ORDER_TABLE: &str = "
        2 2305843009213693951 61
        10 1000000007 1000000006
        3 18446744073709551557 18446744073709551556
        7 1000000000039 500000000019
        2 205891132094649 137260754729766
        5 1099511627776 274877906944
        10 999999000001 36
        2 6685349671 27720
        123456789 1000000000000000009 333333333333333336
        2 4951760154835678088235319297 1891
    ";
    const DLOG_TABLE: &[(u64, u64, u64, u64)] = &[
        (5, 186153212, 1000000007, 660884444),
        (3, 850635776, 998244353, 493754269),
        (2, 458689132828, 1000000000039, 270916124873),
        (7, 1324678365, 2147483647, 820495932),
        (2, 5137535223, 10000000019, 1009629701),
    ];
    const SQRT_MOD_TABLE: &str = "
        92025546424015934464961069271975722225 170141183460469231731687303715884105727 784649073\
        08969450875873340614617370157
        9071298359447453074196815262880708590 170141183460469231731687303715884105727 -1
        143174977646413499152917710357926242927 170141183460469231731687303715884105727 11325649\
        895400189453362511308408481687
        21646663994515894585392180 618970019642690137449562111 -1
        386702092240616973408635273 618970019642690137449562111 -1
        5617234298764214455520936 618970019642690137449562111 146042824400825690984324603
        14360975465577961733 18446744069414584321 -1
        13286848605526297961 18446744069414584321 8215701126991008937
        13785268017400829303 18446744069414584321 699369374259109189
        1262863167 3221225473 48060449
        345413501 3221225473 202223507
        2416624865 3221225473 -1
        950199855941864181426575891013 1000000000000000000000000000057 4094912927096111807500448\
        50292
        281431053323628013644788489297 1000000000000000000000000000057 -1
        911652536280414162692565070556 1000000000000000000000000000057 2662379539871562332594354\
        50007
        2126388764717174293 2305843009213693951 1050081479205097290
        796261809311211500 2305843009213693951 -1
        1552117551101139244 2305843009213693951 569259154081864276
        9255 65537 22000
        23327 65537 -1
        9622 65537 27595
        749264962886364511 1000000000000000009 395527942058623574
        387555878993735709 1000000000000000009 -1
        93499834741215186 1000000000000000009 -1
    ";
    const BERNOULLI_TABLE: &str = "
        0:1 1:-1/2 2:1/6 3:0 4:-1/30 5:0 6:1/42 7:0 8:-1/30 9:0 10:5/66 11:0 12:-691/2730 13:0
        14:7/6 15:0 16:-3617/510 17:0 18:43867/798 19:0 20:-174611/330 21:0 22:854513/138 23:0
        24:-236364091/2730 25:0 26:8553103/6 27:0 28:-23749461029/870 29:0
        30:8615841276005/14322 60:-1215233140483755572040304994079820246041491/56786730
        100:-94598037819122125295227433069493721872702841533066936133385696204311395415197247711\
        /33330
        200:-49838404942833341476492863214039966210849588745720667496805582261726366962152368756\
        8865802302210999132601412697613279391058654527145340515840099290478026350382802884371712\
        359337984274122861159800280019110197888555893671151/1366530
    ";
    const BERNOULLI_2000: (&str, u64, usize) = ("2338224387510", 479916381, 4152);
    const EULER_TABLE: &str = "
        0:1 1:0 2:-1 3:0 4:5 5:0 6:-61 7:0 8:1385 9:0 10:-50521 11:0 12:2702765 13:0
        14:-199360981 15:0 16:19391512145 17:0 18:-2404879675441 19:0 20:370371188237525
        50:-6053285248188621896314383785111649088103498225146815121
        100:290352834666109749705460383476443587507755300664615894508049231914699764337062502388\
        9353447129967354174648294748510553528692457632980625125
    ";
    const BELL_TABLE: &str = "
        0:1 1:1 2:2 3:5 4:15 5:52 6:203 7:877 8:4140 9:21147 10:115975 11:678570 12:4213597
        50:185724268771078270438257767181908917499221852770
        100:475853912767648336587907688413872078263636696868256114666163346375591144978924426226\
        72724044217756306953557882560751
    ";
    const CATALAN_TABLE: &str = "
        0:1 1:1 2:2 3:5 4:14 5:42 6:132 7:429 8:1430 9:4862 10:16796 11:58786 12:208012
        50:1978261657756160653623774456
        100:896519947090131496687170070074100632420837521538745909320
    ";
    const PARTITION_TABLE: &str = "
        0:1 1:1 2:2 3:3 4:5 5:7 6:11 7:15 8:22 9:30 10:42 11:56 12:77 13:101 14:135 15:176
        16:231 17:297 18:385 19:490 20:627 100:190569292 1000:24061467864032622473692149727991
        10000:3616725132563629398882047189095369549501603033931565042208186860588795256875406642\
        0592310556052906916435144
        100000:274935105697756965126775163209863526881734293159800547582031259843021473281149641\
        7305505074166073662159015784477429624894049306307020046179276449303351011607934245719015\
        5718943509725312466108452006369558934464248716828789832182345009262853831404597021307130\
        6745106244192273112389997022844086093709355316296978515695698921961084801586005694210985\
        19
    ";
    const HARMONIC_TABLE: &str = "
        0:0 1:1 2:3/2 3:11/6 10:7381/2520 30:9304682830147/2329089562800
        100:14466636279520351160221518043104131447711/2788815009188499086581352357412492142272
    ";
    const DERANGEMENT_TABLE: &str = "
        0:1 1:0 2:1 3:2 4:9 5:44 6:265 7:1854 8:14833 9:133496 10:1334961 11:14684570
        12:176214841 30:97581073836835777732377428235481
        60:3061120089030075932414107959955432155359786926462466522532734900460828338404172761
    ";
    const STIRLING_TABLE: &str = "
        0:0:1:1 0:1:0:0 1:0:0:0 1:1:1:1 1:2:0:0 2:0:0:0 2:1:-1:1 2:2:1:1 2:3:0:0 3:0:0:0 3:1:2:1
        3:2:-3:3 3:3:1:1 3:4:0:0 4:0:0:0 4:1:-6:1 4:2:11:7 4:3:-6:6 4:4:1:1 4:5:0:0 5:0:0:0
        5:1:24:1 5:2:-50:15 5:3:35:25 5:4:-10:10 5:5:1:1 5:6:0:0 6:0:0:0 6:1:-120:1 6:2:274:31
        6:3:-225:90 6:4:85:65 6:5:-15:15 6:6:1:1 6:7:0:0 7:0:0:0 7:1:720:1 7:2:-1764:63
        7:3:1624:301 7:4:-735:350 7:5:175:140 7:6:-21:21 7:7:1:1 7:8:0:0 8:0:0:0 8:1:-5040:1
        8:2:13068:127 8:3:-13132:966 8:4:6769:1701 8:5:-1960:1050 8:6:322:266 8:7:-28:28 8:8:1:1
        8:9:0:0 20:7:-52260903362512720:11143554045652
        30:11:-39539238727270799376544542000:215047101560666876619690
        50:25:-506473132983732155508942707968971278382005365625:74538021532732000833796262348376\
        25465912500
        60:1:-138683118545689835737939019720389406345902876772687432540821294940160000000000000:\
        1
        60:59:-1770:1770
        100:40:258493109341474980430742712945413962507901239771399312103012483964090229983933164\
        1609285043108904560358508292946350594271338496:45563990592573583802064986297302310145538\
        2995667098634377209055877853954312432303165159638984538596909232698044
    ";
    const FIBONACCI_TABLE: &str = "
        -11:89:-199 -10:-55:123 -3:2:-4 -2:-1:3 -1:1:-1 0:0:2 1:1:1 2:1:3 3:2:4 10:55:123
        11:89:199 90:2880067194370816120:6440026026380244498
        300:222232244629420445529739893461909967206666939096499764990979600:49692640578374667639\
        3791436882468230898067489522034699520200002
    ";
    const BINOMIAL_GRID: [[i64; 11]; 14] = [
        [0, 0, 1, -6, 21, -56, 126, -252, 462, -792, 1287],
        [0, 0, 1, -5, 15, -35, 70, -126, 210, -330, 495],
        [0, 0, 1, -4, 10, -20, 35, -56, 84, -120, 165],
        [0, 0, 1, -3, 6, -10, 15, -21, 28, -36, 45],
        [0, 0, 1, -2, 3, -4, 5, -6, 7, -8, 9],
        [0, 0, 1, -1, 1, -1, 1, -1, 1, -1, 1],
        [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0],
        [0, 0, 1, 3, 3, 1, 0, 0, 0, 0, 0],
        [0, 0, 1, 4, 6, 4, 1, 0, 0, 0, 0],
        [0, 0, 1, 5, 10, 10, 5, 1, 0, 0, 0],
        [0, 0, 1, 6, 15, 20, 15, 6, 1, 0, 0],
        [0, 0, 1, 7, 21, 35, 35, 21, 7, 1, 0],
    ];
    const BINOMIAL_BIG: &str = "
        100000000000000000000 2 4999999999999999999950000000000000000000
        100000000000000000000 99999999999999999997 166666666666666666661666666666666666666700000\
        000000000000000
        -100000000000000000000 3 -166666666666666666671666666666666666666700000000000000000000
        -100000000000000000000 4 416666666666666666691666666666666666667125000000000000000002500\
        0000000000000000
        200 100 90548514656103281165404177077484163874504589675413336841320
        -200 77 -4870017945142407949409628738186241848162305455499043978943751770104000
        10000000000 5 833333332500000000291666666625000000002000000000
    ";
    const BEST_RATIONAL_TABLE: &str = "
        13593/5698 36008 13593/5698
        -1/1 9 -1/1
        -96501181911633/38398311040304 24 -58/23
        -147390439892548/92770317118111 1268763 -1669421/1050765
        -2388667047736/7856026528299 890 -45/148
        37/31 23658823030 37/31
        10211345026/4991061171 94244670 192774412/94223521
        -34/141 262545569 -34/141
        120172202968247458617/45704394525372667033 709 1667/634
        -145826/8406929 12973419935 -145826/8406929
        -3861601/2005347 7935 -8340/4331
        -5705/3379 15861570619 -5705/3379
        -224841441543480939151/90703248393408871681 222 -176/71
        246963619784151/6502395631657475 57451 2101/55318
        1483078147313986/877921115818701 8064546213 9719631559/5753621142
        1749193910/682765769 9134158708 1749193910/682765769
        533084678727/298153956650 9617619502 10575693013/5914979068
        -22915/6634432 3 0/1
        313/385 70627810817 313/385
        75745704159070342793/124730804676419315894092 38 0/1
        124/71 7 7/4
        -8/3 806171382 -8/3
        -2140470808826220453/774997438039124087 699 -58/21
        -2046042561467324293805/699430249989579820991 51225817216 -142439749294/48692373919
        -384109438759/190580297486 730 -1042/517
        513/392 92825 513/392
        22159317591105174893087/25305817813589160108378 9323 6127/6997
        238653457298770/80572877354047 444338 650206/219519
        -122928433903288/71643968702641 533 -477/278
        -1000373609/552579161 8513479 -7782085/4298612
        -12492995486289/6706765866059 90365 -95/51
        -286351857329/898172647947 3693 -754/2365
        154932/68717 2916425 154932/68717
        -22303771573954964126966/8153224996917543174233 3037250 -8282798/3027807
        -4582032213626623/1873532003058684 5 -12/5
        29736953367140356/19253268080722201 8 11/7
        -98166797669109935/51116059020479838 285 -169/88
        -6522398590309247/7474507058847711 65 -48/55
        -51760974687/31126962148 38282 -43099/25918
        -6726357537036/8044016924701 24481 -12716/15207
        1/2 1 0/1
        -1/2 1 -1/1
        3/2 1 1/1
        5/6 4 3/4
        314159265358979/100000000000000 10 22/7
        314159265358979/100000000000000 100 311/99
        314159265358979/100000000000000 1000 355/113
        314159265358979/100000000000000 1000000 3126535/995207
        -314159265358979/100000000000000 1000 -355/113
        7/10 3 2/3
        1/3 2 1/2
        2/5 3 1/3
        0/1 5 0/1
    ";
    const PERFECT_POWERS: &str = "
        -4096:-16:3 -3375:-15:3 -3125:-5:5 -2744:-14:3 -2197:-13:3 -2187:-3:7 -2048:-2:11
        -1728:-12:3 -1331:-11:3 -1024:-4:5 -1000:-10:3 -729:-9:3 -512:-2:9 -343:-7:3 -243:-3:5
        -216:-6:3 -128:-2:7 -125:-5:3 -64:-4:3 -32:-2:5 -27:-3:3 -8:-2:3 4:2:2 8:2:3 9:3:2
        16:2:4 25:5:2 27:3:3 32:2:5 36:6:2 49:7:2 64:2:6 81:3:4 100:10:2 121:11:2 125:5:3
        128:2:7 144:12:2 169:13:2 196:14:2 216:6:3 225:15:2 243:3:5 256:2:8 289:17:2 324:18:2
        343:7:3 361:19:2 400:20:2 441:21:2 484:22:2 512:2:9 529:23:2 576:24:2 625:5:4 676:26:2
        729:3:6 784:28:2 841:29:2 900:30:2 961:31:2 1000:10:3 1024:2:10 1089:33:2 1156:34:2
        1225:35:2 1296:6:4 1331:11:3 1369:37:2 1444:38:2 1521:39:2 1600:40:2 1681:41:2 1728:12:3
        1764:42:2 1849:43:2 1936:44:2 2025:45:2 2048:2:11 2116:46:2 2187:3:7 2197:13:3 2209:47:2
        2304:48:2 2401:7:4 2500:50:2 2601:51:2 2704:52:2 2744:14:3 2809:53:2 2916:54:2 3025:55:2
        3125:5:5 3136:56:2 3249:57:2 3364:58:2 3375:15:3 3481:59:2 3600:60:2 3721:61:2 3844:62:2
        3969:63:2 4096:2:12
    ";

    const PI_POWERS_OF_TEN: [u64; 15] = [
        0,
        4,
        25,
        168,
        1_229,
        9_592,
        78_498,
        664_579,
        5_761_455,
        50_847_534,
        455_052_511,
        4_118_054_813,
        37_607_912_018,
        346_065_536_839,
        3_204_941_750_802,
    ];
    const NTH_PRIME_POWERS_OF_TEN: [u64; 13] = [
        2,
        29,
        541,
        7_919,
        104_729,
        1_299_709,
        15_485_863,
        179_424_673,
        2_038_074_743,
        22_801_763_489,
        252_097_800_623,
        2_760_727_302_517,
        29_996_224_275_833,
    ];
    const CARMICHAEL: [u64; 12] = [
        561,
        1_105,
        1_729,
        2_465,
        2_821,
        6_601,
        8_911,
        41_041,
        825_265,
        321_197_185,
        5_394_826_801,
        9_746_347_772_161,
    ];

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }

        fn int(&mut self, bits: u32) -> Integer {
            let mut x = Integer::new();
            for _ in 0..bits.div_ceil(64) {
                x = (x << 64u32) + self.next();
            }
            x.keep_bits(bits)
        }

        fn signed(&mut self, bits: u32) -> Integer {
            let x = self.int(bits);
            if self.next() & 1 == 0 { x } else { -x }
        }

        fn varied(&mut self, min_bits: u32, spread: u64) -> Integer {
            let bits = min_bits + self.below(spread) as u32;
            self.int(bits)
        }

        fn signed_varied(&mut self, min_bits: u32, spread: u64) -> Integer {
            let bits = min_bits + self.below(spread) as u32;
            self.signed(bits)
        }
    }

    fn int(s: &str) -> Integer {
        s.parse().unwrap()
    }

    fn rat(s: &str) -> Rational {
        s.parse().unwrap()
    }

    fn big(x: i64) -> Integer {
        Integer::from(x)
    }

    fn pow(base: u32, exp: u32) -> Integer {
        Integer::from(base).pow(exp)
    }

    fn rows(table: &str) -> impl Iterator<Item = Vec<&str>> {
        table
            .lines()
            .map(|line| line.split_whitespace().collect::<Vec<_>>())
            .filter(|row| !row.is_empty())
    }

    fn records(table: &str) -> impl Iterator<Item = Vec<&str>> {
        table
            .split_whitespace()
            .map(|record| record.split(':').collect())
    }

    fn check_sequence<T>(table: &str, f: impl Fn(u32) -> T)
    where
        T: PartialEq + std::fmt::Debug + std::str::FromStr<Err: std::fmt::Debug>,
    {
        for record in records(table) {
            assert_eq!(
                f(record[0].parse().unwrap()),
                record[1].parse().unwrap(),
                "index {}",
                record[0]
            );
        }
    }

    fn semiprime(table: &str, index: usize) -> (Integer, Integer) {
        let row = rows(table).nth(index).unwrap();
        (int(row[0]), int(row[1]))
    }

    fn parse_factors(tokens: &[&str]) -> Vec<(Integer, u32)> {
        tokens
            .iter()
            .map(|t| {
                let (p, e) = t.split_once('^').unwrap();
                (int(p), e.parse().unwrap())
            })
            .collect()
    }

    fn product(f: &[(Integer, u32)]) -> Integer {
        f.iter().map(|(p, e)| Integer::from(p.pow(*e))).product()
    }

    fn check_factorization(n: &Integer, effort: u32) -> (Vec<(Integer, u32)>, Option<Integer>) {
        let (f, cofactor) = factor_limited(n, effort);
        assert!(
            f.windows(2).all(|w| w[0].0 < w[1].0),
            "ascending primes for {n}"
        );
        assert!(
            f.iter().all(|(p, e)| is_prime(p) && *e > 0),
            "prime powers for {n}"
        );
        let rest = cofactor.clone().unwrap_or_else(|| Integer::from(1));
        assert_eq!(product(&f) * &rest, n.clone().abs(), "product for {n}");
        if let Some(c) = &cofactor {
            assert!(*c > 1 && !is_prime(c), "composite cofactor for {n}");
        }
        (f, cofactor)
    }

    fn naive_factor(mut n: u64) -> Vec<(u64, u32)> {
        let mut f = Vec::new();
        let mut p = 2;
        while p * p <= n {
            let mut e = 0;
            while n.is_multiple_of(p) {
                n /= p;
                e += 1;
            }
            if e > 0 {
                f.push((p, e));
            }
            p += 1;
        }
        if n > 1 {
            f.push((n, 1));
        }
        f
    }

    fn lucy(n: u64) -> u64 {
        let r = n.isqrt();
        let mut vals: Vec<u64> = (1..=r).map(|i| n / i).collect();
        vals.extend((1..n / r).rev());
        let idx = |x: u64| {
            if x <= r {
                vals.len() - x as usize
            } else {
                (n / x - 1) as usize
            }
        };
        let mut s: Vec<u64> = vals.iter().map(|&x| x - 1).collect();
        for p in 2..=r {
            let below = s[idx(p - 1)];
            if s[idx(p)] == below {
                continue;
            }
            for (i, &x) in vals.iter().enumerate().take_while(|&(_, &x)| x >= p * p) {
                s[i] -= s[idx(x / p)] - below;
            }
        }
        s[0]
    }

    fn strong_probable_prime(n: u64, a: u64) -> bool {
        let (n, a) = (Integer::from(n), Integer::from(a));
        let nm1 = Integer::from(&n - 1u32);
        let s = nm1.find_one(0).unwrap();
        let mut x = a.pow_mod(&Integer::from(&nm1 >> s), &n).unwrap();
        if x == 1 || x == nm1 {
            return true;
        }
        (1..s).any(|_| {
            x = x.clone().square() % &n;
            x == nm1
        })
    }

    #[test]
    fn primality_small_and_edge_cases() {
        let primes = primes_up_to(2_000_000);
        let mut next = primes.iter().copied().peekable();
        for n in 0..=2_000_000u64 {
            let expected = next.next_if_eq(&n).is_some();
            assert_eq!(is_prime_u64(n), expected, "{n}");
        }
        for n in [-7, -2, -1, 0, 1, 4, 9, 91] {
            assert!(!is_prime(&big(n)), "{n}");
        }
        for n in [2, 3, 5, 7, 97, 7919, 2_147_483_647] {
            assert!(is_prime(&big(n)), "{n}");
        }
        assert!(!is_prime(&-int("170141183460469231731687303715884105727")));
    }

    #[test]
    fn primality_pseudoprimes() {
        for n in CARMICHAEL {
            assert!(!is_prime(&Integer::from(n)), "{n}");
            assert!(
                (2..40u64)
                    .filter(|a| gcd_u64(*a, n) == 1)
                    .all(|a| pow_mod_u64(a, n - 1, n) == 1)
            );
        }
        for (i, &(_, limit)) in WITNESSES[..11].iter().enumerate() {
            assert!(!is_prime_u64(limit), "{limit}");
            let (f, cofactor) = check_factorization(&Integer::from(limit), 1);
            assert!(cofactor.is_none() && f.iter().map(|(_, e)| e).sum::<u32>() > 1);
            for &(a, _) in &WITNESSES[..=i] {
                assert!(strong_probable_prime(limit, a), "{limit} base {a}");
            }
        }
        for s in [
            "318665857834031151167461",
            "3317044064679887385961981",
            "4759123141",
            "1122004669633",
            "18446744073709551617",
        ] {
            assert!(!is_prime(&int(s)), "{s}");
        }
        let k = (1_000_000_000u64..)
            .find(|&k| [6, 12, 18].iter().all(|c| is_prime_u64(c * k + 1)))
            .unwrap();
        let parts: Vec<Integer> = [6, 12, 18]
            .iter()
            .map(|c| Integer::from(c * k + 1))
            .collect();
        let chernick: Integer = parts.iter().product();
        assert!(chernick > Integer::from(1) << 64u32 && !is_prime(&chernick));
        let exp = Integer::from(&chernick - 1u32);
        assert!(
            (2u32..30)
                .all(|a| pow_mod(&Integer::from(a), &exp, &chernick) == Some(Integer::from(1)))
        );
        let expected: Vec<(Integer, u32)> = parts.into_iter().map(|p| (p, 1)).collect();
        assert_eq!(factor_limited(&chernick, 1), (expected, None));
    }

    #[test]
    fn primality_matches_gmp_below_two_to_64() {
        let mut rng = Rng(0x00C0_FFEE);
        for _ in 0..150_000 {
            let shift = rng.below(44);
            let n = (rng.next() >> shift) | 1;
            assert_eq!(
                is_prime_u64(n),
                Integer::from(n).is_probably_prime(25) != IsPrime::No,
                "{n}"
            );
        }
        for p in primes_up_to(300_000) {
            for (a, b) in [(2, 1), (3, 2), (4, 3), (7, 6)] {
                assert!(!is_prime_u64(p * (a * p - b)), "{p} {a} {b}");
            }
        }
    }

    #[test]
    fn primality_large() {
        for e in [61, 89, 107, 127, 521, 607] {
            assert!(is_prime(&((Integer::from(1) << e) - 1u32)), "2^{e}-1");
        }
        for e in [67, 101, 103, 109, 257, 523] {
            assert!(!is_prime(&((Integer::from(1) << e) - 1u32)), "2^{e}-1");
        }
        let two64 = Integer::from(1) << 64u32;
        assert!(
            is_prime(&Integer::from(&two64 - 59u32)) && is_prime(&Integer::from(&two64 + 13u32))
        );
        assert!((0u32..13).all(|d| !is_prime(&Integer::from(&two64 + d))));
        assert!((1u32..59).all(|d| !is_prime(&Integer::from(&two64 - d))));
        let m127 = (Integer::from(1) << 127u32) - 1u32;
        assert!(!is_prime(&m127.clone().square()));
        assert!(!is_prime(&(m127 * ((Integer::from(1) << 89u32) - 1u32))));
    }

    #[test]
    fn next_and_prev_prime() {
        for (n, expected) in [
            (-100, 2),
            (0, 2),
            (1, 2),
            (2, 3),
            (3, 5),
            (13, 17),
            (23, 29),
            (7918, 7919),
            (7919, 7927),
        ] {
            assert_eq!(next_prime(&big(n)), expected, "{n}");
        }
        for n in [-5, 0, 1, 2] {
            assert_eq!(prev_prime(&big(n)), None);
        }
        for (n, expected) in [(3, 2), (4, 3), (5, 3), (20, 19), (7920, 7919), (7919, 7907)] {
            assert_eq!(prev_prime(&big(n)), Some(big(expected)), "{n}");
        }
        for (k, (&up, &down)) in NEXT_PRIME_OFFSETS
            .iter()
            .zip(&PREV_PRIME_OFFSETS)
            .enumerate()
        {
            let n = pow(10, k as u32 + 1);
            assert_eq!(next_prime(&n), Integer::from(&n + up), "next 10^{}", k + 1);
            assert_eq!(
                prev_prime(&n),
                Some(Integer::from(&n - down)),
                "prev 10^{}",
                k + 1
            );
        }
        let two64 = Integer::from(1) << 64u32;
        assert_eq!(
            next_prime(&Integer::from(&two64 - 59u32)),
            Integer::from(&two64 + 13u32)
        );
        assert_eq!(
            prev_prime(&Integer::from(&two64 + 13u32)),
            Some(Integer::from(&two64 - 59u32))
        );
        let mut rng = Rng(11);
        for bits in [20, 40, 63, 64, 65, 100, 200, 400] {
            let n = rng.int(bits);
            let p = next_prime(&n);
            assert!(p > n && is_prime(&p));
            let mut between = Integer::from(&n + 1u32);
            while between < p {
                assert!(!is_prime(&between), "{between}");
                between += 1u32;
            }
            assert_eq!(prev_prime(&Integer::from(&p + 1u32)), Some(p.clone()));
            let q = prev_prime(&p).unwrap();
            assert!(q < p && is_prime(&q) && next_prime(&q) == p);
        }
        let primes = primes_up_to(3000);
        for w in primes.windows(2) {
            for n in w[0]..w[1] {
                assert_eq!(next_prime(&Integer::from(n)), w[1]);
                assert_eq!(prev_prime(&Integer::from(n + 1)), Some(Integer::from(w[0])));
            }
        }
    }

    #[test]
    fn sieve_and_primes_up_to() {
        assert!(primes_up_to(0).is_empty() && primes_up_to(1).is_empty());
        assert_eq!(primes_up_to(2), [2]);
        assert_eq!(primes_up_to(30), [2, 3, 5, 7, 11, 13, 17, 19, 23, 29]);
        assert_eq!(primes_up_to(31).len(), 11);
        for (k, &count) in PI_POWERS_OF_TEN[..8].iter().enumerate() {
            assert_eq!(primes_up_to(10u64.pow(k as u32)).len() as u64, count);
        }
        let all = primes_up_to(PRIMES_UP_TO_MAX);
        assert_eq!((all.len(), all[all.len() - 1]), (5_761_455, 99_999_989));
        assert!(all.windows(2).all(|w| w[0] < w[1]));
        let mut rng = Rng(5);
        for base in [
            0,
            1 << 18,
            (1 << 19) - 300,
            10u64.pow(9),
            10u64.pow(12),
            (1 << 44) - 77,
        ] {
            let lo = base + rng.below(200);
            let hi = lo + 300 + rng.below(700);
            let expected: Vec<u64> = (lo..=hi).filter(|&n| is_prime_u64(n)).collect();
            assert_eq!(sieve_range(lo, hi), expected, "{lo}..={hi}");
        }
        assert!(sieve_range(24, 28).is_empty() && sieve_range(10, 5).is_empty());
        assert_eq!(SMALL_PRIMES.len(), 6542);
    }

    #[test]
    #[should_panic(expected = "PRIMES_UP_TO_MAX")]
    fn primes_up_to_rejects_large_limit() {
        primes_up_to(PRIMES_UP_TO_MAX + 1);
    }

    #[test]
    fn prime_pi_values() {
        for n in [-10, -1, 0, 1] {
            assert_eq!(prime_pi(&big(n)), Some(Integer::new()));
        }
        let mut count = 0;
        for n in 0..=30_000u64 {
            count += u64::from(is_prime_u64(n));
            assert_eq!(prime_count(n), count, "{n}");
        }
        for (k, &expected) in PI_POWERS_OF_TEN.iter().enumerate() {
            assert_eq!(
                prime_pi(&pow(10, k as u32)),
                Some(Integer::from(expected)),
                "10^{k}"
            );
        }
        for &(x, expected) in PRIME_PI_TABLE {
            assert_eq!(prime_count(x), expected, "{x}");
        }
        let mut rng = Rng(7);
        for bits in [17, 20, 24, 27, 30, 31, 32, 33, 34] {
            for _ in 0..3 {
                let x = rng.below(1 << bits) + (1 << (bits - 1));
                assert_eq!(prime_count(x), lucy(x), "{x}");
            }
        }
        for x in [
            80u64,
            81,
            82,
            624,
            625,
            626,
            14_640,
            14_641,
            28_560,
            28_561,
            10u64.pow(8) - 1,
            99_980_001,
            99_980_000,
        ] {
            assert_eq!(prime_count(x), lucy(x), "{x}");
        }
        assert_eq!(prime_pi(&Integer::from(PRIME_PI_MAX + 1)), None);
        assert_eq!(prime_pi(&pow(10, 30)), None);
    }

    #[test]
    fn nth_prime_values() {
        assert_eq!(nth_prime(0), None);
        assert_eq!(nth_prime(NTH_PRIME_MAX + 1), None);
        let primes = primes_up_to(400_000);
        for (i, &p) in primes.iter().enumerate().take(3000) {
            assert_eq!(nth_prime(i as u64 + 1), Some(Integer::from(p)));
        }
        for n in (3000..primes.len()).step_by(97) {
            assert_eq!(nth_prime(n as u64 + 1), Some(Integer::from(primes[n])));
        }
        for (k, &expected) in NTH_PRIME_POWERS_OF_TEN.iter().enumerate() {
            assert_eq!(
                nth_prime(10u64.pow(k as u32)),
                Some(Integer::from(expected)),
                "10^{k}"
            );
        }
        for &(n, expected) in NTH_PRIME_TABLE {
            assert_eq!(nth_prime(n), Some(Integer::from(expected)), "{n}");
        }
        let mut rng = Rng(3);
        for bits in [12, 20, 28, 33, 36] {
            let n = rng.below(1 << bits) + 1;
            let p = nth_prime(n).unwrap();
            assert!(is_prime(&p));
            assert_eq!(prime_pi(&p), Some(Integer::from(n)));
            assert_eq!(
                prime_pi(&Integer::from(&p - 1u32)),
                Some(Integer::from(n - 1))
            );
        }
        let t = Instant::now();
        assert_eq!(nth_prime(9_999_999), Some(Integer::from(179_424_671)));
        assert!(t.elapsed().as_secs_f64() < 0.5);
        for n in [1usize, 2, 3, 25, 168, 1000, 9999, 33_860] {
            let p = primes[n - 1];
            for x0 in [
                0,
                1,
                2,
                p / 2,
                p - 2,
                p - 1,
                p,
                p + 1,
                p + 2,
                p + 1000,
                3 * p + 77,
                50 * p,
            ] {
                assert_eq!(nth_prime_near(n as u64, x0), p, "n = {n}, x0 = {x0}");
            }
        }
    }

    #[test]
    fn factor_small_numbers() {
        assert_eq!(factor(&big(0)), None);
        assert_eq!(
            factor_limited(&big(0), 2),
            (Vec::new(), Some(Integer::new()))
        );
        for n in [1, -1] {
            assert_eq!(factor(&big(n)), Some(Vec::new()));
        }
        assert_eq!(factor(&big(-12)), Some(vec![(big(2), 2), (big(3), 1)]));
        for n in 1..=6000u64 {
            let expected: Vec<(Integer, u32)> = naive_factor(n)
                .into_iter()
                .map(|(p, e)| (Integer::from(p), e))
                .collect();
            for effort in 0..3 {
                assert_eq!(
                    factor_limited(&Integer::from(n), effort),
                    (expected.clone(), None),
                    "{n}"
                );
            }
        }
    }

    #[test]
    fn factor_table() {
        for row in rows(FACTOR_TABLE) {
            let (n, expected) = (int(row[0]), parse_factors(&row[1..]));
            assert_eq!(factor(&-n.clone()), Some(expected.clone()), "-{n}");
            assert_eq!(check_factorization(&n, 2), (expected, None), "{n}");
        }
    }

    #[test]
    fn factor_every_u64_at_effort_one() {
        let mut cases = vec![
            u64::MAX,
            u64::MAX - 58,
            4_294_967_291 * 4_294_967_279,
            4_294_967_291 * 4_294_967_291,
            2_097_143u64.pow(3),
            65_537u64.pow(3) * 65_521,
            65_537 * 65_539 * 65_543,
            4_294_967_311 * 4_294_967_279,
            65_521u64.pow(4),
            2_147_483_647 * 4_294_967_311,
            1 << 63,
            3u64.pow(40),
            6_700_417 * 2_753_074_036_095,
            1_000_000_007 * 1_000_000_009,
            999_999_937 * 4_294_967_291,
        ];
        let mut rng = Rng(42);
        cases.extend((0..400).map(|_| rng.next()));
        cases.extend((0..200).map(|_| rng.next() >> rng.below(30)));
        for _ in 0..60 {
            let p = next_prime(&Integer::from(rng.below(1 << 32) | 1 << 31))
                .to_u64()
                .unwrap();
            let q = next_prime(&Integer::from(rng.below(1 << 31) | 1 << 30))
                .to_u64()
                .unwrap();
            cases.push(p * q);
        }
        for n in cases {
            let (f, cofactor) = check_factorization(&Integer::from(n), 1);
            assert_eq!(cofactor, None, "{n}");
            assert_eq!(factor(&Integer::from(n)), Some(f));
        }
    }

    #[test]
    fn factor_effort_levels() {
        let n = Integer::from(65_537u64 * 65_539);
        assert_eq!(factor_limited(&n, 0), (Vec::new(), Some(n.clone())));
        assert_eq!(
            factor_limited(&n, 1),
            (vec![(big(65_537), 1), (big(65_539), 1)], None)
        );
        let m61 = (Integer::from(1) << 61u32) - 1u32;
        let n = Integer::from(1024 * 243) * &m61;
        assert_eq!(
            factor_limited(&n, 0),
            (vec![(big(2), 10), (big(3), 5), (m61.clone(), 1)], None)
        );
        let hard = Integer::from(1024 * 243) * Integer::from(65_537u64 * 65_539) * &m61;
        let (f, cofactor) = factor_limited(&hard, 0);
        assert_eq!(
            (f, cofactor),
            (
                vec![(big(2), 10), (big(3), 5)],
                Some(Integer::from(65_537u64 * 65_539) * &m61)
            )
        );
        let (p, q) = semiprime(SEMIPRIMES_15, 0);
        let n = Integer::from(&p * &q);
        assert_eq!(factor_limited(&n, 0), (Vec::new(), Some(n.clone())));
        assert_eq!(factor_limited(&n, 1), (Vec::new(), Some(n.clone())));
        let cube = Integer::from((&p).pow(3));
        assert_eq!(factor_limited(&cube, 0), (Vec::new(), Some(cube.clone())));
        assert_eq!(factor_limited(&cube, 1), (vec![(p.clone(), 3)], None));
        let squares = n.clone().square();
        let mut expected = vec![(p.clone(), 2), (q.clone(), 2)];
        expected.sort();
        assert_eq!(
            factor_limited(&squares, 1),
            (Vec::new(), Some(squares.clone()))
        );
        assert_eq!(factor_limited(&squares, 2), (expected.clone(), None));
        assert_eq!(factor_limited(&squares, 7), (expected, None));
        let mixed = Integer::from(&m61 * &m61) * 49u32 * ((Integer::from(1) << 127u32) - 1u32);
        let m127 = (Integer::from(1) << 127u32) - 1u32;
        assert_eq!(factor(&mixed), Some(vec![(big(7), 2), (m61, 2), (m127, 1)]));
    }

    #[test]
    fn factor_semiprimes_within_contract() {
        for row in rows(SEMIPRIMES_15).chain(rows(SEMIPRIMES_MIXED)) {
            let (p, q) = (int(row[0]), int(row[1]));
            let mut expected = vec![(p.clone(), 1), (q.clone(), 1)];
            expected.sort();
            assert_eq!(factor(&Integer::from(&p * &q)), Some(expected), "{p} * {q}");
        }
        let mut rng = Rng(2026);
        for _ in 0..12 {
            let p = next_prime(&(pow(10, 14) + rng.below(9 * 10u64.pow(14))));
            let q = next_prime(&(pow(10, 14) + rng.below(9 * 10u64.pow(14))));
            let (f, cofactor) = check_factorization(&Integer::from(&p * &q), 2);
            assert_eq!((f.len(), cofactor), (2, None), "{p} * {q}");
        }
    }

    #[test]
    fn factor_gives_up_on_hard_composites() {
        let hard = int(HARD_SEMIPRIME.0) * int(HARD_SEMIPRIME.1);
        let n = Integer::from(&hard * 12u32);
        let small = vec![(big(2), 2), (big(3), 1)];
        assert_eq!(factor_limited(&n, 1), (small.clone(), Some(hard.clone())));
        let t = Instant::now();
        assert_eq!(factor_limited(&n, 2), (small, Some(hard.clone())));
        assert!(t.elapsed().as_secs_f64() < 20.0);
        assert_eq!(factor(&hard), None);
    }

    #[test]
    fn arithmetic_functions_reject_unfactorable_input() {
        let n = int(HARD_SEMIPRIME.0) * int(HARD_SEMIPRIME.1);
        let t = Instant::now();
        assert_eq!(mobius(&Integer::from(&n * 4u32)), Some(0));
        assert_eq!(is_squarefree(&Integer::from(n.square_ref())), Some(false));
        assert_eq!(mobius(&(Integer::from(n.square_ref()) * 5u32)), Some(0));
        assert!(t.elapsed().as_secs_f64() < 4.0);
        assert_eq!(mobius(&n), None);
        assert_eq!(divisors(&Integer::from(&n * 6u32)), None);
    }

    #[test]
    fn factoring_stages() {
        let m61 = (Integer::from(1) << 61u32) - 1u32;
        let mut r = next_prime(&pow(10, 25));
        while !is_prime(&(Integer::from(&r << 1u32) + 1u32)) {
            r = next_prime(&r);
        }
        let n = (Integer::from(&r << 1u32) + 1u32) * &m61;
        for b1 in [50_000, 1_400, 400] {
            assert_eq!(pollard_pm1(&n, b1), Some(m61.clone()), "{b1}");
        }
        assert!((1..=200).all(|k| pow_mod(&big(3), &big(k), &m61) != Some(big(1))));
        assert_eq!(pollard_pm1(&n, 2), None);

        let n = Integer::from(1_000_000_007u64) * next_prime(&pow(10, 20));
        assert_eq!(rho_brent(&n, 1 << 19), Some(big(1_000_000_007)));
        assert_eq!(rho_brent(&n, 16), None);
        assert_eq!(rho_brent(&n, 0), None);
        let small = primes_up_to(60);
        for (i, &p) in small.iter().enumerate().skip(1) {
            for &q in &small[i..] {
                let found = rho_brent(&Integer::from(p * q), 1 << 12);
                assert!(
                    found == Some(Integer::from(p)) || found == Some(Integer::from(q)),
                    "{p} * {q}"
                );
            }
        }

        let (p, q) = semiprime(SEMIPRIMES_15, 1);
        let n = Integer::from(&p * &q);
        let cost = ECM_CURVE_COST * ECM_LEVELS[0].0;
        assert_eq!(ecm(&n, cost - 1), None);
        let found: Vec<Option<Integer>> = (0..9).map(|j| ecm(&n, cost << j)).collect();
        let first = found.iter().position(Option::is_some).unwrap();
        assert!(found[first] == Some(p) || found[first] == Some(q));
        assert!(found[first..].iter().all(|f| *f == found[first]));
    }

    #[test]
    fn bounded_caches_stay_consistent() {
        let expected: Vec<Integer> = (0..120).map(|n| zigzag(n, usize::MAX)).collect();
        for (cap, n) in [(50, 119), (10, 60), (200, 119), (0, 77), (5, 5), (5, 6)] {
            assert_eq!(zigzag(n, cap), expected[n as usize], "{cap} {n}");
            assert_eq!(zigzag(n / 2, cap), expected[n as usize / 2], "{cap} {n}");
        }
        assert_eq!(bernoulli(30), rat("8615841276005/14322"));
        assert_eq!(euler_number(10), -50_521);
        let expected: Vec<Integer> = (0..400).map(|n| partitions_cached(n, usize::MAX)).collect();
        for (cap, n) in [(100, 399), (1, 250), (0, 399), (399, 398), (400, 399)] {
            assert_eq!(partitions_cached(n, cap), expected[n as usize], "{cap} {n}");
            assert_eq!(
                partitions_cached(n / 3, cap),
                expected[n as usize / 3],
                "{cap} {n}"
            );
        }
        assert_eq!(partitions(200), int("3972999029388"));
    }

    #[test]
    fn factor_random_products() {
        let mut rng = Rng(99);
        for bits in [70, 90, 110, 128, 160, 200, 300] {
            for _ in 0..4 {
                let n = rng.signed(bits);
                for effort in 0..2 {
                    check_factorization(&n, effort);
                }
            }
        }
        for _ in 0..20 {
            let primes: Vec<Integer> = (0..4).map(|_| next_prime(&rng.varied(20, 18))).collect();
            let n = primes.iter().product::<Integer>() * primes[0].clone().square();
            let (f, cofactor) = check_factorization(&n, 1);
            assert_eq!(cofactor, None);
            assert_eq!(f.iter().map(|(_, e)| e).sum::<u32>(), 6);
        }
    }

    #[test]
    fn perfect_powers() {
        let mut expected = records(PERFECT_POWERS).peekable();
        for n in -4100..=4100 {
            let want = expected
                .next_if(|r| r[0] == n.to_string())
                .map(|r| (int(r[1]), r[2].parse().unwrap()));
            assert_eq!(is_perfect_power(&Integer::from(n)), want, "{n}");
        }
        assert_eq!(expected.next(), None);
        assert_eq!(
            is_perfect_power(&(Integer::from(1) << 100u32)),
            Some((big(2), 100))
        );
        assert_eq!(
            is_perfect_power(&-(Integer::from(1) << 100u32)),
            Some((big(-16), 25))
        );
        assert_eq!(is_perfect_power(&pow(6, 12)), Some((big(6), 12)));
        assert_eq!(is_perfect_power(&-pow(6, 12)), Some((big(-1296), 3)));
        assert_eq!(is_perfect_power(&-pow(7, 64)), None);
        assert_eq!(
            is_perfect_power(&(Integer::from(1) << 100_003u32)),
            Some((big(2), 100_003))
        );
        assert_eq!(is_perfect_power(&pow(3, 20_011)), Some((big(3), 20_011)));
        let m127 = (Integer::from(1) << 127u32) - 1u32;
        for k in [2u32, 3, 5, 7, 12, 35] {
            let n = Integer::from((&m127).pow(k));
            assert_eq!(is_perfect_power(&n), Some((m127.clone(), k)));
            assert_eq!(is_perfect_power(&Integer::from(&n + 1u32)), None);
            let odd = k >> k.trailing_zeros();
            let want = (odd > 1).then(|| (-Integer::from((&m127).pow(k / odd)), odd));
            assert_eq!(is_perfect_power(&-n), want);
        }
        let mut rng = Rng(17);
        for _ in 0..200 {
            let (b, k) = (rng.varied(1, 40) + 2u32, 2 + rng.below(9) as u32);
            let (base, exp) = is_perfect_power(&Integer::from((&b).pow(k))).unwrap();
            assert!(
                exp % k == 0
                    && Integer::from((&base).pow(exp / k)) == b
                    && is_perfect_power(&base).is_none()
            );
        }
    }

    #[test]
    fn divisor_functions_against_brute_force() {
        for n in 1..=1500u64 {
            let divs: Vec<u64> = (1..=n).filter(|d| n % d == 0).collect();
            let big_n = Integer::from(n);
            let f = naive_factor(n);
            assert_eq!(divisors(&big_n).unwrap(), divs);
            assert_eq!(divisors(&-big_n.clone()).unwrap(), divs);
            for k in 0..4 {
                let sigma: Integer = divs.iter().map(|&d| Integer::from(d).pow(k)).sum();
                assert_eq!(divisor_sigma(k, &big_n), Some(sigma));
            }
            let phi = (1..=n).filter(|&k| gcd_u64(k, n) == 1).count();
            assert_eq!(totient(&big_n), Some(Integer::from(phi)));
            assert_eq!(totient(&-big_n.clone()), Some(Integer::from(phi)));
            let squarefree = f.iter().all(|&(_, e)| e == 1);
            let mu = if !squarefree {
                0
            } else if f.len().is_multiple_of(2) {
                1
            } else {
                -1
            };
            assert_eq!(mobius(&big_n), Some(mu));
            assert_eq!(is_squarefree(&big_n), Some(squarefree));
            assert_eq!(omega(&big_n), Some(f.len() as u32));
            assert_eq!(big_omega(&big_n), Some(f.iter().map(|&(_, e)| e).sum()));
            assert_eq!(
                radical(&big_n),
                Some(Integer::from(f.iter().map(|&(p, _)| p).product::<u64>()))
            );
            let mobius_sum: i32 = divs
                .iter()
                .map(|&d| mobius(&Integer::from(d)).unwrap())
                .sum();
            assert_eq!(mobius_sum, i32::from(n == 1));
            let totient_sum: Integer = divs
                .iter()
                .map(|&d| totient(&Integer::from(d)).unwrap())
                .sum();
            assert_eq!(totient_sum, n);
        }
    }

    #[test]
    fn arithmetic_function_table_and_edge_cases() {
        for row in rows(ARITH_TABLE) {
            let n = int(row[0]);
            let (small_omega, large_omega) = (row[6].parse().unwrap(), row[7].parse().unwrap());
            assert_eq!(divisor_sigma(0, &n), Some(int(row[1])), "{n}");
            assert_eq!(divisor_sigma(1, &n), Some(int(row[2])), "{n}");
            assert_eq!(divisor_sigma(3, &n), Some(int(row[3])), "{n}");
            assert_eq!(divisor_sigma(3, &-n.clone()), Some(int(row[3])), "{n}");
            assert_eq!(totient(&n), Some(int(row[4])), "{n}");
            assert_eq!(mobius(&n), Some(row[5].parse().unwrap()), "{n}");
            assert_eq!(mobius(&-n.clone()), Some(row[5].parse().unwrap()), "{n}");
            assert_eq!(omega(&n), Some(small_omega), "{n}");
            assert_eq!(big_omega(&n), Some(large_omega), "{n}");
            assert_eq!(is_squarefree(&n), Some(small_omega == large_omega), "{n}");
            assert_eq!(
                divisors(&n).map(|d| Integer::from(d.len())),
                Some(int(row[1])),
                "{n}"
            );
        }
        let zero = Integer::new();
        assert_eq!(divisors(&zero), None);
        assert_eq!(divisor_sigma(1, &zero), None);
        assert_eq!(totient(&zero), None);
        assert_eq!(mobius(&zero), None);
        assert_eq!(omega(&zero), None);
        assert_eq!(big_omega(&zero), None);
        assert_eq!(radical(&zero), None);
        assert_eq!(is_squarefree(&zero), Some(false));
        assert_eq!(divisors(&factorial(30)), None);
        assert_eq!(divisors(&Integer::from(Integer::primorial(60))), None);
        assert_eq!(
            divisors(&Integer::from(Integer::primorial(50))).map(|d| d.len()),
            Some(1 << 15)
        );
        let two_pow = Integer::from(1) << 99_999u32;
        assert_eq!(divisors(&two_pow).map(|d| d.len()), Some(100_000));
        assert_eq!(divisors(&Integer::from(&two_pow << 1u32)), None);
        assert_eq!(
            divisor_sigma(1, &two_pow),
            Some(Integer::from(&two_pow << 1u32) - 1u32)
        );
        assert_eq!(divisor_sigma(u32::MAX, &big(4)), None);
        for p in [
            6u64,
            28,
            496,
            8128,
            33_550_336,
            8_589_869_056,
            137_438_691_328,
        ] {
            assert_eq!(
                divisor_sigma(1, &Integer::from(p)),
                Some(Integer::from(2 * p))
            );
        }
        let m127 = (Integer::from(1) << 127u32) - 1u32;
        let perfect = Integer::from(&m127 << 126u32);
        assert_eq!(
            divisor_sigma(1, &perfect),
            Some(Integer::from(&perfect << 1u32))
        );
        assert_eq!(totient(&m127), Some(Integer::from(&m127 - 1u32)));
        assert_eq!(radical(&(perfect * 9u32)), Some(m127 * 6u32));
        let mut rng = Rng(8);
        for _ in 0..40 {
            let (a, b) = (rng.int(40) + 1u32, rng.int(40) + 1u32);
            if a.gcd_ref(&b).complete() != 1 {
                continue;
            }
            let ab = Integer::from(&a * &b);
            assert_eq!(
                totient(&ab).unwrap(),
                totient(&a).unwrap() * totient(&b).unwrap()
            );
            for k in 0..3 {
                assert_eq!(
                    divisor_sigma(k, &ab).unwrap(),
                    divisor_sigma(k, &a).unwrap() * divisor_sigma(k, &b).unwrap()
                );
            }
            assert_eq!(
                mobius(&ab).unwrap(),
                mobius(&a).unwrap() * mobius(&b).unwrap()
            );
            assert_eq!(omega(&ab).unwrap(), omega(&a).unwrap() + omega(&b).unwrap());
        }
    }

    #[test]
    fn extended_gcd_and_inverse() {
        let mut rng = Rng(1);
        let mut pairs: Vec<(Integer, Integer)> = (-12..=12)
            .flat_map(|a| (-12..=12).map(move |b| (big(a), big(b))))
            .collect();
        pairs.extend((0..200).map(|i| (rng.signed(1 + i), rng.signed(1 + i / 2))));
        for (a, b) in pairs {
            let (g, x, y) = extended_gcd(&a, &b);
            assert_eq!(g, a.clone().gcd(&b));
            assert!(
                g >= 0 && Integer::from(&a * &x) + Integer::from(&b * &y) == g,
                "{a} {b}"
            );
            match mod_inverse(&a, &b) {
                Some(inv) => {
                    assert!(g == 1 && inv >= 0 && inv < b.clone().abs());
                    assert!((Integer::from(&a * &inv) - 1u32).is_divisible(&b));
                }
                None => assert!(b.is_zero() || g != 1),
            }
        }
        assert_eq!(extended_gcd(&big(0), &big(0)), (big(0), big(0), big(0)));
        assert_eq!(mod_inverse(&big(3), &big(7)), Some(big(5)));
        assert_eq!(mod_inverse(&big(3), &big(-7)), Some(big(5)));
        assert_eq!(mod_inverse(&big(-3), &big(7)), Some(big(2)));
        assert_eq!(mod_inverse(&big(2), &big(4)), None);
        assert_eq!(mod_inverse(&big(5), &big(0)), None);
        assert_eq!(mod_inverse(&big(5), &big(1)), Some(big(0)));
        assert_eq!(mod_inverse(&big(0), &big(-1)), Some(big(0)));
        assert_eq!(mod_inverse(&big(0), &big(5)), None);
    }

    #[test]
    fn modular_powers() {
        for base in -9..=9i64 {
            for m in (-9..=9i64).filter(|&m| m != 0) {
                let modulus = m.abs();
                let mut expected = 1 % modulus;
                for exp in 0..8 {
                    assert_eq!(
                        pow_mod(&big(base), &big(exp), &big(m)),
                        Some(big(expected)),
                        "{base}^{exp} mod {m}"
                    );
                    let inverse = (0..modulus).find(|x| (x * expected - 1) % modulus == 0);
                    assert_eq!(
                        pow_mod(&big(base), &big(-exp), &big(m)),
                        inverse.map(big),
                        "{base}^-{exp} mod {m}"
                    );
                    expected = (expected * base).rem_euclid(modulus);
                }
            }
        }
        assert_eq!(pow_mod(&big(2), &big(10), &big(0)), None);
        assert_eq!(pow_mod(&big(2), &big(-1), &big(0)), None);
        assert_eq!(pow_mod(&big(0), &big(0), &big(7)), Some(big(1)));
        let m127 = (Integer::from(1) << 127u32) - 1u32;
        assert_eq!(
            pow_mod(&big(3), &Integer::from(&m127 - 1u32), &m127),
            Some(big(1))
        );
        assert_eq!(pow_mod(&big(2), &big(127), &m127), Some(big(1)));
        assert_eq!(
            pow_mod(&big(-2), &big(127), &m127),
            Some(Integer::from(&m127 - 1u32))
        );
        let inv = pow_mod(&big(10), &big(-3), &m127).unwrap();
        assert_eq!(Integer::from(&inv * 1000u32) % &m127, 1);
        let huge = pow(7, 1000);
        assert_eq!(
            pow_mod(&big(3), &huge, &big(1_000_000_007)),
            Some(big(3).pow_mod(&huge, &big(1_000_000_007)).unwrap())
        );
        assert_eq!(
            pow_mod(&big(3), &huge, &big(1_000_000_007)),
            pow_mod(&big(3), &(huge % 1_000_000_006u32), &big(1_000_000_007))
        );
    }

    #[test]
    fn jacobi_and_kronecker() {
        for (a, row) in (-12..=12).zip(KRONECKER_ROWS) {
            for (n, symbol) in (-12..=12).zip(row.chars()) {
                let expected = "-0+".find(symbol).unwrap() as i32 - 1;
                assert_eq!(kronecker(&big(a), &big(n)), expected, "({a}/{n})");
                let odd_positive = n > 0 && n % 2 == 1;
                assert_eq!(
                    jacobi(&big(a), &big(n)),
                    odd_positive.then_some(expected),
                    "({a}/{n})"
                );
            }
        }
        for row in rows(KRONECKER_BIG) {
            let (a, n, expected) = (int(row[0]), int(row[1]), row[2].parse().unwrap());
            assert_eq!(kronecker(&a, &n), expected, "({a}/{n})");
            assert_eq!(jacobi(&a, &n), (n.is_odd() && n > 0).then_some(expected));
        }
        for p in primes_up_to(120).into_iter().skip(1) {
            for a in 0..p {
                let residue = (1..p).any(|x| x * x % p == a);
                let expected = if a == 0 {
                    0
                } else if residue {
                    1
                } else {
                    -1
                };
                assert_eq!(jacobi(&Integer::from(a), &Integer::from(p)), Some(expected));
            }
        }
        let mut rng = Rng(21);
        for _ in 0..100 {
            let (a, b) = (rng.signed(90), rng.signed(90));
            let (m, n) = (
                rng.int(70) | Integer::from(1),
                rng.int(70) | Integer::from(1),
            );
            let mn = Integer::from(&m * &n);
            assert_eq!(
                jacobi(&a, &mn).unwrap(),
                jacobi(&a, &m).unwrap() * jacobi(&a, &n).unwrap()
            );
            assert_eq!(
                jacobi(&Integer::from(&a * &b), &n).unwrap(),
                jacobi(&a, &n).unwrap() * jacobi(&b, &n).unwrap()
            );
            assert_eq!(jacobi(&Integer::from(&a + &n), &n), jacobi(&a, &n));
        }
    }

    #[test]
    fn modular_square_roots() {
        for p in primes_up_to(300) {
            for a in -3..p as i64 + 3 {
                let expected = (0..p).find(|r| (r * r) as i64 % p as i64 == a.rem_euclid(p as i64));
                assert_eq!(
                    sqrt_mod(&big(a), &Integer::from(p)),
                    expected.map(Integer::from),
                    "sqrt({a}) mod {p}"
                );
            }
        }
        for row in rows(SQRT_MOD_TABLE) {
            let expected = (row[2] != "-1").then(|| int(row[2]));
            assert_eq!(
                sqrt_mod(&int(row[0]), &int(row[1])),
                expected,
                "sqrt({}) mod {}",
                row[0],
                row[1]
            );
        }
        for (n, root) in [(-7, 2), (-1, 0), (1, 0), (4, 0), (9, 2), (15, 2), (561, 2)] {
            assert_eq!(sqrt_mod(&big(4), &big(n)), Some(big(root)), "{n}");
        }
        assert_eq!(sqrt_mod(&big(4), &big(0)), None);
        for m in 1..=400i64 {
            for a in -2..m + 2 {
                let expected: Vec<Integer> = (0..m)
                    .filter(|x| (x * x - a).rem_euclid(m) == 0)
                    .map(big)
                    .collect();
                assert_eq!(
                    sqrt_mod_all(&big(a), &big(m), 1 << 16),
                    Some(expected.clone()),
                    "sqrt({a}) mod {m}"
                );
                assert_eq!(sqrt_mod(&big(a), &big(-m)), expected.first().cloned());
            }
        }
        let m = pow(3, 40) * pow(2, 70) * 1_000_003u32;
        let roots = sqrt_mod_all(&big(1_234_321), &m, 1 << 16).unwrap();
        assert_eq!(roots.len(), 16);
        assert!(roots.windows(2).all(|w| w[0] < w[1]));
        assert!(
            roots
                .iter()
                .all(|x| (x.clone().square() - 1_234_321u32).is_divisible(&m))
        );
        assert_eq!(roots[0], 1111);
        assert_eq!(sqrt_mod_all(&big(0), &pow(2, 80), 1 << 16), None);
        assert_eq!(
            sqrt_mod_all(&big(0), &pow(2, 20), 1 << 16).map(|v| v.len()),
            Some(1 << 10)
        );
        let mut rng = Rng(31);
        for p in [
            "18446744069414584321",
            "170141183460469231731687303715884105727",
            "4179340454199820289",
            "1000000000000000000000000000057",
        ] {
            let p = int(p);
            assert_eq!(sqrt_mod(&p, &p), Some(Integer::new()));
            for _ in 0..25 {
                let x = rng.int(130) % &p;
                let a = x.clone().square() % &p;
                let r = sqrt_mod(&a, &p).unwrap();
                assert!(r == x || r == Integer::from(&p - &x));
                assert!(Integer::from(&r << 1u32) <= p);
                let shifted = Integer::from(&a - &p) - &p;
                assert_eq!(sqrt_mod(&shifted, &p), Some(r));
            }
        }
    }

    #[test]
    fn chinese_remainders() {
        assert_eq!(crt(&[]), Some((big(0), big(1))));
        assert_eq!(
            crt(&[(big(2), big(3)), (big(3), big(5)), (big(2), big(7))]),
            Some((big(23), big(105)))
        );
        assert_eq!(
            crt(&[(big(1), big(4)), (big(3), big(6))]),
            Some((big(9), big(12)))
        );
        assert_eq!(crt(&[(big(1), big(4)), (big(2), big(6))]), None);
        assert_eq!(crt(&[(big(1), big(0))]), None);
        assert_eq!(crt(&[(big(-1), big(-5))]), Some((big(4), big(5))));
        assert_eq!(
            crt(&[(big(17), big(1)), (big(5), big(-7))]),
            Some((big(5), big(7)))
        );
        let mut rng = Rng(77);
        for round in 0..300 {
            let x = rng.signed(200);
            let moduli: Vec<Integer> = (0..1 + rng.below(6))
                .map(|_| rng.signed_varied(1, 70))
                .collect();
            if moduli.iter().any(Integer::is_zero) {
                continue;
            }
            let mut system: Vec<(Integer, Integer)> = moduli
                .iter()
                .map(|m| {
                    (
                        Integer::from(&x % m) + Integer::from(m * rng.below(5)),
                        m.clone(),
                    )
                })
                .collect();
            let lcm = moduli.iter().fold(Integer::from(1), |acc, m| acc.lcm(m));
            assert_eq!(
                crt(&system),
                Some((x.clone().rem_euc(&lcm), lcm.clone())),
                "round {round}"
            );
            if lcm > 2 && round % 3 == 0 {
                let m = moduli[0].clone();
                system.push((Integer::from(&x + 1u32), Integer::from(&m * 2u32)));
                assert_eq!(crt(&system).is_some(), m.abs() == 1 && lcm.is_odd());
            }
        }
    }

    #[test]
    fn multiplicative_orders() {
        for n in 1..=150i64 {
            for a in -3..=n + 2 {
                let expected = (gcd_u64(a.unsigned_abs(), n as u64) == 1).then(|| {
                    (1..=n)
                        .find(|&k| pow_mod(&big(a), &big(k), &big(n)) == Some(big(1 % n)))
                        .unwrap()
                });
                assert_eq!(
                    multiplicative_order(&big(a), &big(n)),
                    expected.map(big),
                    "ord({a}) mod {n}"
                );
                assert_eq!(multiplicative_order(&big(a), &big(-n)), expected.map(big));
            }
        }
        assert_eq!(multiplicative_order(&big(1), &big(0)), None);
        for row in rows(ORDER_TABLE) {
            assert_eq!(
                multiplicative_order(&int(row[0]), &int(row[1])),
                Some(int(row[2])),
                "{row:?}"
            );
        }
        let mut rng = Rng(13);
        for _ in 0..60 {
            let n = rng.varied(10, 50) + 2u32;
            let a = rng.int(64);
            let Some(order) = multiplicative_order(&a, &n) else {
                assert_ne!(a.gcd(&n), 1);
                continue;
            };
            assert_eq!(pow_mod(&a, &order, &n), Some(big(1)));
            assert!(totient(&n).unwrap().is_divisible(&order));
            for (q, _) in factor(&order).unwrap() {
                assert_ne!(pow_mod(&a, &Integer::from(&order / &q), &n), Some(big(1)));
            }
        }
    }

    #[test]
    fn primitive_roots() {
        assert_eq!(primitive_root(&big(0)), None);
        assert_eq!(primitive_root(&big(1)), Some(big(0)));
        for (n, &expected) in (2..).zip(&PRIMITIVE_ROOTS) {
            let want = (expected != 0).then(|| Integer::from(expected));
            assert_eq!(primitive_root(&big(n)), want, "{n}");
            assert_eq!(primitive_root(&big(-n)), want, "-{n}");
            if let Some(g) = want {
                assert_eq!(multiplicative_order(&g, &big(n)), totient(&big(n)));
                let smaller = (1..expected)
                    .all(|h| multiplicative_order(&big(h.into()), &big(n)) != totient(&big(n)));
                assert!(smaller, "{n}");
            }
        }
        for record in records(PRIMITIVE_ROOTS_BIG) {
            let (n, expected) = (int(record[0]), int(record[1]));
            assert_eq!(primitive_root(&n), Some(expected.clone()), "{n}");
            assert_eq!(multiplicative_order(&expected, &n), totient(&n));
        }
        for n in [
            "340282366920938463463374607431768211456",
            "1000000000000000000000000000000",
            "1000000016000000063",
        ] {
            assert_eq!(primitive_root(&int(n)), None, "{n}");
        }
    }

    #[test]
    fn discrete_logarithms() {
        for m in 1..=40i64 {
            for g in 0..m + 1 {
                let mut first = vec![None; m as usize];
                let mut power = 1 % m;
                for x in 0..2 * m + 8 {
                    first[power as usize].get_or_insert(x);
                    power = power * g % m;
                }
                for h in 0..m {
                    assert_eq!(
                        discrete_log(&big(g), &big(h), &big(m)),
                        first[h as usize].map(big),
                        "log_{g}({h}) mod {m}"
                    );
                }
                assert_eq!(
                    discrete_log(&big(g - m), &big(m + 1), &big(-m)),
                    first[(1 % m) as usize].map(big)
                );
            }
        }
        assert_eq!(discrete_log(&big(2), &big(1), &big(0)), None);
        for &(g, h, m, x) in DLOG_TABLE {
            assert_eq!(
                discrete_log(&Integer::from(g), &Integer::from(h), &Integer::from(m)),
                Some(Integer::from(x))
            );
        }
        let p = Integer::from(17_592_186_044_423u64);
        let x = Integer::from(17_592_186_040_001u64);
        let h = pow_mod(&big(5), &x, &p).unwrap();
        assert_eq!(discrete_log(&big(5), &h, &p), Some(x));
        assert_eq!(discrete_log(&big(25), &big(5), &p), None);
        let n = Integer::from(1_000_003u64 * 998_244_353) * 48u32;
        let h = pow_mod(&big(6), &big(123_456_789), &n).unwrap();
        let x = discrete_log(&big(6), &h, &n).unwrap();
        assert!(x <= 123_456_789 && pow_mod(&big(6), &x, &n) == Some(h));
        let m127 = (Integer::from(1) << 127u32) - 1u32;
        let x = Integer::from(54_321_012_345u64);
        let h = pow_mod(&big(3), &x, &m127).unwrap();
        assert_eq!(discrete_log(&big(3), &h, &m127), Some(x));
        assert_eq!(
            discrete_log(&big(2), &Integer::from(&m127 - 1u32), &m127),
            None
        );
        assert_eq!(
            discrete_log(&big(2), &(Integer::from(1) << 100u32), &m127),
            Some(big(100))
        );
        let big_power = pow_mod(&big(3), &(Integer::from(1) << 60u32), &m127).unwrap();
        let x = discrete_log(&big(3), &big_power, &m127).unwrap();
        assert_eq!(pow_mod(&big(3), &x, &m127), Some(big_power));
        assert!(x < multiplicative_order(&big(3), &m127).unwrap());
        assert_eq!(
            discrete_log_search(&big(2), &Integer::from(&m127 - 1u32), &m127),
            Err(true)
        );
        assert_eq!(discrete_log_search(&big(2), &big(1), &big(0)), Err(true));
        assert_eq!(discrete_log_search(&big(6), &big(5), &big(12)), Err(true));
        let p = next_prime(&pow(10, 40));
        let q = next_prime(&pow(10, 41));
        let n = Integer::from(&p * &q);
        let h = pow_mod(&big(7), &big(1_000_003), &n).unwrap();
        assert!(matches!(
            discrete_log_search(&big(7), &h, &n),
            Ok(_) | Err(false)
        ));
    }

    #[test]
    fn fibonacci_lucas_and_factorials() {
        for record in records(FIBONACCI_TABLE) {
            let n = record[0].parse().unwrap();
            assert_eq!(fibonacci(n), int(record[1]), "F({n})");
            assert_eq!(lucas(n), int(record[2]), "L({n})");
        }
        for n in -60..=60i64 {
            assert_eq!(fibonacci(n + 1), fibonacci(n) + fibonacci(n - 1));
            assert_eq!(lucas(n), fibonacci(n - 1) + fibonacci(n + 1));
            let cassini = fibonacci(n + 1) * fibonacci(n - 1) - fibonacci(n).square();
            assert_eq!(cassini, if n % 2 == 0 { 1 } else { -1 });
        }
        assert_eq!(fibonacci(-(1 << 20)), -fibonacci(1 << 20));
        assert_eq!(factorial(0), 1);
        assert_eq!(factorial(20), 2_432_902_008_176_640_000u64);
        assert_eq!(factorial(25), int("15511210043330985984000000"));
        for (n, expected) in [
            (-1, 1),
            (0, 1),
            (1, 1),
            (2, 2),
            (5, 15),
            (6, 48),
            (9, 945),
            (10, 3840),
        ] {
            assert_eq!(double_factorial(n), Some(big(expected)), "{n}!!");
        }
        assert_eq!(double_factorial(-2), None);
        assert_eq!(double_factorial(i64::MIN), None);
        assert_eq!(double_factorial(1 << 32), None);
        for n in 1..200u32 {
            assert_eq!(
                double_factorial(n.into()).unwrap() * double_factorial(i64::from(n) - 1).unwrap(),
                factorial(n)
            );
        }
    }

    #[test]
    #[should_panic(expected = "fibonacci")]
    fn fibonacci_rejects_huge_index() {
        fibonacci(1 << 32);
    }

    #[test]
    fn binomials_and_multinomials() {
        for (n, row) in (-6..).zip(BINOMIAL_GRID) {
            for (k, expected) in (-2..).zip(row) {
                assert_eq!(
                    binomial(&big(n), &big(k)),
                    Some(big(expected)),
                    "C({n}, {k})"
                );
            }
        }
        for row in rows(BINOMIAL_BIG) {
            assert_eq!(
                binomial(&int(row[0]), &int(row[1])),
                Some(int(row[2])),
                "C({}, {})",
                row[0],
                row[1]
            );
        }
        let huge = pow(10, 30);
        assert_eq!(binomial(&huge, &Integer::from(&huge >> 1u32)), None);
        assert_eq!(binomial(&-huge.clone(), &huge), None);
        assert_eq!(binomial(&huge, &Integer::from(&huge + 1u32)), Some(big(0)));
        assert_eq!(binomial(&huge, &huge), Some(big(1)));
        assert_eq!(binomial(&huge, &big(0)), Some(big(1)));
        assert_eq!(binomial(&big(-1), &huge), Some(big(1)));
        assert_eq!(
            binomial(&big(-1), &Integer::from(&huge + 1u32)),
            Some(big(-1))
        );
        assert_eq!(binomial(&big(-2), &huge), Some(Integer::from(&huge + 1u32)));
        for n in 0..60 {
            for k in 0..=n + 1 {
                let pascal =
                    binomial(&big(n), &big(k)).unwrap() + binomial(&big(n), &big(k + 1)).unwrap();
                assert_eq!(binomial(&big(n + 1), &big(k + 1)), Some(pascal));
            }
        }
        assert_eq!(multinomial(&[]), 1);
        assert_eq!(multinomial(&[0, 0]), 1);
        assert_eq!(multinomial(&[7]), 1);
        assert_eq!(multinomial(&[2, 3, 4]), 1260);
        assert_eq!(multinomial(&[1; 10]), factorial(10));
        assert_eq!(
            multinomial(&[30, 40, 50]),
            factorial(120) / (factorial(30) * factorial(40) * factorial(50))
        );
        assert_eq!(
            multinomial(&[u32::MAX, 1, 1]),
            (Integer::from(u32::MAX) + 2u32) * (Integer::from(u32::MAX) + 1u32)
        );
    }

    #[test]
    fn bernoulli_and_euler_numbers() {
        check_sequence(BERNOULLI_TABLE, bernoulli);
        for n in 1..=80u32 {
            let sum: Rational = (0..=n)
                .map(|k| bernoulli(k) * Integer::from(n + 1).binomial(k))
                .sum();
            assert_eq!(sum, 0, "{n}");
        }
        let t = Instant::now();
        let b = bernoulli(2000);
        assert!(t.elapsed().as_secs_f64() < 2.0);
        let (denominator, residue, digits) = BERNOULLI_2000;
        assert_eq!(*b.denom(), int(denominator));
        assert!(b.numer().is_negative());
        assert_eq!(b.numer().clone().abs() % 1_000_000_007u32, residue);
        assert_eq!(b.numer().clone().abs().to_string().len(), digits);
        for n in (2..=400u32).step_by(2) {
            let expected: Integer = primes_up_to(u64::from(n) + 1)
                .into_iter()
                .filter(|p| u64::from(n) % (p - 1) == 0)
                .product();
            assert_eq!(*bernoulli(n).denom(), expected, "{n}");
        }
        check_sequence(EULER_TABLE, euler_number);
        for n in 1..=40u32 {
            let sum: Integer = (0..=n)
                .map(|k| euler_number(2 * k) * Integer::from(2 * n).binomial(2 * k))
                .sum();
            assert_eq!(sum, 0, "{n}");
        }
    }

    #[test]
    fn catalan_bell_and_stirling_numbers() {
        check_sequence(CATALAN_TABLE, catalan_number);
        for n in 0..40u32 {
            let convolution: Integer = (0..=n)
                .map(|i| catalan_number(i) * catalan_number(n - i))
                .sum();
            assert_eq!(catalan_number(n + 1), convolution);
        }
        check_sequence(BELL_TABLE, bell_number);
        for record in records(STIRLING_TABLE) {
            let (n, k) = (record[0].parse().unwrap(), record[1].parse().unwrap());
            assert_eq!(stirling1(n, k), int(record[2]), "s({n}, {k})");
            assert_eq!(stirling2(n, k), int(record[3]), "S({n}, {k})");
        }
        for n in 0..45u32 {
            let second: Integer = (0..=n).map(|k| stirling2(n, k)).sum();
            assert_eq!(second, bell_number(n), "{n}");
            let unsigned: Integer = (0..=n).map(|k| stirling1(n, k).abs()).sum();
            assert_eq!(unsigned, factorial(n), "{n}");
            let signed: Integer = (0..=n).map(|k| stirling1(n, k)).sum();
            assert_eq!(signed, i32::from(n < 2), "{n}");
            for k in 1..=n {
                assert_eq!(
                    stirling2(n + 1, k),
                    stirling2(n, k) * k + stirling2(n, k - 1)
                );
                assert_eq!(
                    stirling1(n + 1, k),
                    stirling1(n, k - 1) - stirling1(n, k) * n
                );
            }
        }
        assert_eq!(stirling1(300, 299), -Integer::from(300 * 299 / 2));
        assert_eq!(stirling2(300, 299), 300 * 299 / 2);
        assert_eq!(stirling1(120, 1), -factorial(119));
        assert_eq!(stirling2(200, 2), (Integer::from(1) << 199u32) - 1u32);
    }

    #[test]
    fn partitions_harmonic_and_derangements() {
        check_sequence(PARTITION_TABLE, partitions);
        let mut ways = vec![Integer::new(); 301];
        ways[0] = Integer::from(1);
        for part in 1..=300 {
            for total in part..=300 {
                let smaller = ways[total - part].clone();
                ways[total] += smaller;
            }
        }
        for (n, expected) in ways.iter().enumerate() {
            assert_eq!(partitions(n as u32), *expected, "p({n})");
        }
        check_sequence(HARMONIC_TABLE, harmonic);
        for n in 1..300u32 {
            assert_eq!(harmonic(n) - harmonic(n - 1), Rational::from((1, n)));
        }
        let naive: Rational = (1..=3000u32).map(|k| Rational::from((1, k))).sum();
        assert_eq!(harmonic(3000), naive);
        check_sequence(DERANGEMENT_TABLE, derangements);
        for n in 2..200u32 {
            assert_eq!(
                derangements(n),
                (derangements(n - 1) + derangements(n - 2)) * (n - 1)
            );
        }
    }

    #[test]
    fn continued_fractions() {
        let ints = |v: &[i64]| v.iter().map(|&x| big(x)).collect::<Vec<_>>();
        assert_eq!(continued_fraction(&rat("415/93")), ints(&[4, 2, 6, 7]));
        assert_eq!(continued_fraction(&rat("-7/3")), ints(&[-3, 1, 2]));
        assert_eq!(continued_fraction(&rat("0")), ints(&[0]));
        assert_eq!(continued_fraction(&rat("-5")), ints(&[-5]));
        assert_eq!(continued_fraction(&rat("1/3")), ints(&[0, 3]));
        assert_eq!(continued_fraction(&rat("3/2")), ints(&[1, 2]));
        assert_eq!(
            convergents(&ints(&[4, 2, 6, 7])),
            [rat("4"), rat("9/2"), rat("58/13"), rat("415/93")]
        );
        assert_eq!(from_continued_fraction(&[]), None);
        assert!(convergents(&[]).is_empty());
        assert_eq!(from_continued_fraction(&ints(&[1, 0])), None);
        assert!(convergents(&ints(&[1, 0, 3])).is_empty());
        assert_eq!(from_continued_fraction(&ints(&[0, 0, 2])), Some(rat("2")));
        assert_eq!(from_continued_fraction(&ints(&[2, -3])), Some(rat("5/3")));
        assert_eq!(
            from_continued_fraction(&ints(&[3, 7, 15, 1])),
            Some(rat("355/113"))
        );
        let fib: Vec<Integer> = vec![big(1); 40];
        assert_eq!(
            from_continued_fraction(&fib),
            Some(Rational::from((fibonacci(41), fibonacci(40))))
        );
        let mut rng = Rng(55);
        for _ in 0..300 {
            let x = Rational::from((rng.signed_varied(1, 150), rng.varied(1, 150) + 1u32));
            let terms = continued_fraction(&x);
            assert!(terms[1..].iter().all(|a| *a >= 1));
            assert!(terms.len() == 1 || terms[terms.len() - 1] >= 2);
            assert_eq!(from_continued_fraction(&terms), Some(x.clone()));
            let conv = convergents(&terms);
            assert_eq!((conv.len(), &conv[conv.len() - 1]), (terms.len(), &x));
            for (i, w) in conv.windows(2).enumerate() {
                let cross = Integer::from(w[1].numer() * w[0].denom())
                    - Integer::from(w[0].numer() * w[1].denom());
                assert_eq!(cross, if i % 2 == 0 { 1 } else { -1 });
            }
        }
    }

    #[test]
    fn best_rational_approximations() {
        for row in rows(BEST_RATIONAL_TABLE) {
            assert_eq!(
                best_rational(&rat(row[0]), &int(row[1])),
                rat(row[2]),
                "{row:?}"
            );
        }
        for q in 1..=40 {
            for p in -45..=90 {
                let x = Rational::from((p, q));
                for max_den in 1..=12 {
                    let best = best_rational(&x, &big(max_den));
                    let distance = Rational::from(&best - &x).abs();
                    let optimum = (1..=max_den)
                        .map(|d| {
                            let numerator = Rational::from(&x * d).round().numer().clone();
                            (Rational::from((numerator, d)) - &x).abs()
                        })
                        .min()
                        .unwrap();
                    assert!(
                        *best.denom() <= max_den && distance == optimum,
                        "{x} with denominator <= {max_den}"
                    );
                }
            }
        }
        let x = rat("-123456789/1000");
        assert_eq!(best_rational(&x, &pow(10, 40)), x);
        assert_eq!(best_rational(&x, &big(1)), rat("-123457"));
    }

    #[test]
    #[should_panic(expected = "max_den")]
    fn best_rational_rejects_zero_denominator_bound() {
        best_rational(&rat("1/3"), &big(0));
    }

    #[test]
    fn integer_roots() {
        assert_eq!(isqrt(&big(-1)), None);
        for n in 0..2000i64 {
            let r = isqrt(&big(n)).unwrap().to_i64().unwrap();
            assert!(r * r <= n && (r + 1) * (r + 1) > n);
            assert_eq!(is_square(&big(n)), r * r == n);
            assert!(!is_square(&big(-n - 1)));
            for k in 1..7u32 {
                let root = iroot(&big(n), k).unwrap().to_i64().unwrap();
                assert!(root.pow(k) <= n && (root + 1).pow(k) > n, "{n} {k}");
                let negative = iroot(&big(-n), k);
                assert_eq!(
                    negative,
                    (k % 2 == 1)
                        .then(|| big(-root))
                        .or((n == 0).then(|| big(0))),
                    "-{n} {k}"
                );
            }
            assert_eq!(iroot(&big(n), 0), None);
        }
        let ten40 = pow(10, 40);
        assert_eq!(isqrt(&ten40), Some(pow(10, 20)));
        assert_eq!(
            isqrt(&Integer::from(&ten40 - 1u32)),
            Some(pow(10, 20) - 1u32)
        );
        assert_eq!(iroot(&Integer::from(&ten40 - 1u32), 40), Some(big(9)));
        assert_eq!(iroot(&-ten40.clone(), 5), Some(big(-100_000_000)));
        assert_eq!(
            iroot(&-Integer::from(&ten40 - 1u32), 5),
            Some(big(-99_999_999))
        );
        assert_eq!(iroot(&ten40, u32::MAX), Some(big(1)));
        assert!(
            is_square(&ten40) && !is_square(&Integer::from(&ten40 + 1u32)) && !is_square(&-ten40)
        );
    }

    #[test]
    fn radix_conversion_and_digit_sums() {
        assert_eq!(to_base(&big(255), 16).as_deref(), Some("ff"));
        assert_eq!(to_base(&big(-255), 2).as_deref(), Some("-11111111"));
        assert_eq!(to_base(&big(0), 7).as_deref(), Some("0"));
        assert_eq!(to_base(&big(1295), 36).as_deref(), Some("zz"));
        assert_eq!(to_base(&big(5), 1), None);
        assert_eq!(to_base(&big(5), 37), None);
        assert_eq!(from_base("zz", 36), Some(big(1295)));
        assert_eq!(from_base("ZZ", 36), Some(big(1295)));
        assert_eq!(from_base("-Ff", 16), Some(big(-255)));
        assert_eq!(from_base("+101", 2), Some(big(5)));
        assert_eq!(from_base("-0", 10), Some(big(0)));
        assert_eq!(from_base("000123", 10), Some(big(123)));
        for bad in [
            "", "-", "+", "12", "1_0", " 1", "1 ", "--1", "+-1", "1.0", "0x1", "1٣",
        ] {
            assert_eq!(from_base(bad, 2), None, "{bad:?}");
        }
        assert_eq!(from_base("10", 1), None);
        assert_eq!(from_base("10", 37), None);
        assert_eq!(from_base("g", 16), None);
        let mut rng = Rng(101);
        for base in 2..=36 {
            for _ in 0..12 {
                let n = rng.signed_varied(1, 300);
                let text = to_base(&n, base).unwrap();
                assert!(
                    text.chars()
                        .all(|c| c == '-' || c.is_ascii_digit() || c.is_ascii_lowercase())
                );
                assert_eq!(from_base(&text, base), Some(n.clone()));
                assert_eq!(from_base(&text.to_uppercase(), base), Some(n.clone()));
                let mut sum = Integer::new();
                let mut rest = n.clone().abs();
                while !rest.is_zero() {
                    sum += rest.mod_u(base);
                    rest /= base;
                }
                assert_eq!(digit_sum(&n, base), Some(sum));
            }
        }
        assert_eq!(digit_sum(&big(12_345), 10), Some(big(15)));
        assert_eq!(digit_sum(&big(-12_345), 10), Some(big(15)));
        assert_eq!(digit_sum(&big(255), 2), Some(big(8)));
        assert_eq!(digit_sum(&big(255), 16), Some(big(30)));
        assert_eq!(digit_sum(&big(12_345), 100), Some(big(69)));
        assert_eq!(digit_sum(&big(-12_345), 1000), Some(big(357)));
        assert_eq!(digit_sum(&big(12_345), u32::MAX), Some(big(12_345)));
        assert_eq!(digit_sum(&big(0), 10), Some(big(0)));
        assert_eq!(digit_sum(&big(7), 1), None);
        assert_eq!(digit_sum(&big(7), 0), None);
        assert_eq!(digit_sum(&(pow(10, 100) - 1u32), 10), Some(big(900)));
        assert_eq!(digit_sum(&(pow(1000, 100) - 1u32), 1000), Some(big(99_900)));
    }

    #[test]
    fn sums_of_two_squares() {
        assert_eq!(sum_of_two_squares(&big(-4)), None);
        assert_eq!(sum_of_two_squares(&big(0)), Some((big(0), big(0))));
        for n in 1..=5000i64 {
            let representable = (0..=n.isqrt()).any(|a| {
                let b = (n - a * a).isqrt();
                a * a + b * b == n
            });
            let result = sum_of_two_squares(&big(n));
            assert_eq!(result.is_some(), representable, "{n}");
            if let Some((a, b)) = result {
                assert!(a >= 0 && a <= b && a.square() + b.square() == n, "{n}");
            }
        }
        let m127 = (Integer::from(1) << 127u32) - 1u32;
        assert_eq!(sum_of_two_squares(&m127), None);
        assert_eq!(
            sum_of_two_squares(&m127.clone().square()),
            Some((big(0), m127.clone()))
        );
        let mut rng = Rng(404);
        let mut checked = 0;
        while checked < 40 {
            let p = next_prime(&rng.varied(40, 200));
            if p.mod_u(4) != 1 {
                continue;
            }
            checked += 1;
            let n = Integer::from(&p * 2u32) * 49u32 * 13u32.pow(3);
            let (a, b) = sum_of_two_squares(&n).unwrap();
            assert!(a <= b && a.square() + b.square() == n);
            assert_eq!(sum_of_two_squares(&(n * 7u32)), None);
        }
        let n = pow(5, 40) * pow(13, 21) * pow(3, 30) * (Integer::from(1) << 77u32);
        let (a, b) = sum_of_two_squares(&n).unwrap();
        assert!(a <= b && a.square() + b.square() == n);
    }

    #[test]
    fn later_additions() {
        for n in 1..=300u64 {
            let lambda = (1..=n)
                .find(|&k| {
                    (1..=n).all(|a| {
                        gcd_u64(a, n) != 1
                            || pow_mod(&Integer::from(a), &Integer::from(k), &Integer::from(n))
                                == Some(Integer::from(1 % n))
                    })
                })
                .unwrap();
            assert_eq!(
                carmichael_lambda(&Integer::from(n)),
                Some(Integer::from(lambda)),
                "{n}"
            );
            let roots: Vec<Integer> = (0..n)
                .filter(|&g| {
                    gcd_u64(g, n) == 1
                        && multiplicative_order(&Integer::from(g), &Integer::from(n))
                            == totient(&Integer::from(n))
                })
                .map(Integer::from)
                .collect();
            assert_eq!(
                all_primitive_roots(&Integer::from(n), 1000),
                Some(roots),
                "{n}"
            );
        }
        assert_eq!(carmichael_lambda(&big(0)), None);
        assert_eq!(all_primitive_roots(&big(0), 10), None);
        assert_eq!(all_primitive_roots(&big(1_000_003), 10), None);
        let listed = |a: i64, b: i64| primes_between(&big(a), &big(b), 1000).unwrap();
        assert_eq!(listed(-5, 30), [2, 3, 5, 7, 11, 13, 17, 19, 23, 29]);
        assert_eq!(listed(14, 16), Vec::<Integer>::new());
        assert_eq!(listed(17, 17), [17]);
        assert_eq!(primes_between(&big(1), &big(100_000), 1000), None);
        let far = pow(10, 30);
        assert_eq!(
            primes_between(&far, &Integer::from(&far + 200u32), 10),
            Some(vec![
                Integer::from(&far + 57u32),
                Integer::from(&far + 99u32)
            ])
        );
        assert_eq!(
            farey(5),
            [
                "0", "1/5", "1/4", "1/3", "2/5", "1/2", "3/5", "2/3", "3/4", "4/5", "1"
            ]
            .map(rat)
        );
        assert_eq!(farey(1), [rat("0"), rat("1")]);
        assert_eq!(farey(100).len(), 3045);
        for d in 0..=9u32 {
            let mut counts = vec![0u64; 41];
            counts[0] = 1;
            for _ in 0..d {
                let mut next = vec![0u64; 41];
                for (m, &c) in counts.iter().enumerate() {
                    for k in -6i64..=6 {
                        let t = m + (k * k) as usize;
                        if t <= 40 {
                            next[t] += c;
                        }
                    }
                }
                counts = next;
            }
            for (n, &c) in counts.iter().enumerate() {
                assert_eq!(
                    squares_r(d, &Integer::from(n)),
                    Some(Integer::from(c)),
                    "r_{d}({n})"
                );
            }
        }
        assert_eq!(squares_r(2, &big(-1)), Some(big(0)));
        assert_eq!(squares_r(24, &big(2)), Some(big(1104)));
        check_sequence(
            "0:1 1:1 2:1 3:2 4:2 5:3 6:4 7:5 8:6 9:8 10:10 20:64 50:3658 100:444793",
            partitions_distinct,
        );
        let rows = [
            vec![1],
            vec![1],
            vec![1, 1],
            vec![1, 4, 1],
            vec![1, 11, 11, 1],
            vec![1, 26, 66, 26, 1],
            vec![1, 57, 302, 302, 57, 1],
        ];
        for (n, row) in rows.iter().enumerate() {
            for k in 0..8 {
                let want = row.get(k).copied().unwrap_or(0);
                assert_eq!(eulerian(n as u32, k as u32), want, "A({n}, {k})");
            }
        }
        for n in 1..40u32 {
            let total: Integer = (0..n).map(|k| eulerian(n, k)).sum();
            assert_eq!(total, factorial(n));
        }
        for (d, x, y) in [
            (2, "3", "2"),
            (3, "2", "1"),
            (7, "8", "3"),
            (13, "649", "180"),
        ] {
            assert_eq!(pell(&big(d)), Some((int(x), int(y))));
        }
        assert_eq!(pell(&big(61)), Some((int("1766319049"), int("226153980"))));
        for d in 2..400 {
            match pell(&big(d)) {
                Some((x, y)) => assert!(y > 0 && x.square() - y.square() * big(d) == 1, "{d}"),
                None => assert!(is_square(&big(d))),
            }
        }
        assert_eq!(pell(&big(0)), None);
        assert_eq!(pell(&big(-3)), None);
        assert_eq!(
            egyptian(&rat("4/13"), 20),
            Some(["1/4", "1/18", "1/468"].map(rat).to_vec())
        );
        assert_eq!(
            egyptian(&rat("7/2"), 20),
            Some(["3", "1/2"].map(rat).to_vec())
        );
        assert_eq!(egyptian(&rat("0"), 20), Some(Vec::new()));
        assert_eq!(egyptian(&rat("-1/2"), 20), None);
        assert_eq!(egyptian(&rat("5/121"), 3), None);
        let mut rng = Rng(5);
        for _ in 0..50 {
            let x = Rational::from((rng.int(12), rng.int(12) + 1u32));
            if let Some(parts) = egyptian(&x, 12) {
                assert_eq!(parts.iter().sum::<Rational>(), x);
                assert!(parts.windows(2).all(|w| w[0] > w[1]));
            }
        }
    }
}
