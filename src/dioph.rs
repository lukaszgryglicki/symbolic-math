//! Equations in whole numbers: congruences (prime powers, Hensel's lifting, the Chinese
//! remainder theorem) and Diophantine equations (linear, bilinear, binary quadratic forms of
//! the common kinds, Pell's equation, Pythagorean triples, search in a box).

use crate::canon::{add, coeff_and_rest, div, mul, neg, pow, sqrt};
use crate::expand::expand;
use crate::expr::Expr;
use crate::ntheory;
use rug::Integer;
use rug::ops::{DivRounding, Pow, RemRounding};

type Res = Result<Expr, String>;

/// Most solutions or steps a command goes through.
const LIMIT: usize = 200_000;

/// A polynomial with whole coefficients: exponents of the variables and the coefficient.
type Poly = Vec<(Vec<u32>, Integer)>;

/// `f` as a polynomial in `vars` with whole coefficients (denominators cleared); the common
/// denominator is returned too.
fn polynomial(f: &Expr, vars: &[String]) -> Option<(Poly, Integer)> {
    let ex = expand(f);
    let terms: Vec<Expr> = match &ex {
        Expr::Add(t) => t.to_vec(),
        other => vec![other.clone()],
    };
    let mut raw = Vec::new();
    let mut lcm = Integer::from(1);
    for term in &terms {
        let (c, rest) = coeff_and_rest(term);
        if !c.im.is_zero() {
            return None;
        }
        let factors: Vec<Expr> = match &rest {
            Expr::Mul(fs) => fs.to_vec(),
            other if other.is_one() => Vec::new(),
            other => vec![other.clone()],
        };
        let mut exps = vec![0u32; vars.len()];
        for fac in factors {
            let (base, e) = fac.base_exp();
            let at = vars.iter().position(|v| base.is_sym(v))?;
            exps[at] += u32::try_from(e.as_i64().filter(|k| (1..=10_000).contains(k))?).ok()?;
        }
        lcm = lcm.lcm(c.re.denom());
        raw.push((exps, c.re.clone()));
    }
    let mut poly: Poly = Vec::new();
    for (exps, c) in raw {
        let whole = Integer::from(c.numer() * &lcm).div_exact(c.denom());
        match poly.iter_mut().find(|(e, _)| *e == exps) {
            Some((_, total)) => *total += whole,
            None => poly.push((exps, whole)),
        }
    }
    poly.retain(|(_, c)| *c != 0);
    Some((poly, lcm))
}

fn horner(coeffs: &[Integer], x: &Integer, m: &Integer) -> Integer {
    let mut acc = Integer::new();
    for c in coeffs.iter().rev() {
        acc = (acc * x + c).rem_euc(m);
    }
    acc
}

/// Polynomials modulo a prime, lowest power first, without leading zeros.
fn trimmed(mut a: Vec<Integer>) -> Vec<Integer> {
    while a.last().is_some_and(|c| *c == 0) {
        a.pop();
    }
    a
}

fn poly_rem(a: &[Integer], b: &[Integer], p: &Integer) -> Vec<Integer> {
    let mut r = a.to_vec();
    let Some(lead) = b.last().and_then(|c| ntheory::mod_inverse(c, p)) else {
        return r;
    };
    while r.len() >= b.len() {
        let k = r.len() - b.len();
        let q = Integer::from(r.last().unwrap() * &lead).rem_euc(p);
        for (i, c) in b.iter().enumerate() {
            r[k + i] = (&r[k + i] - Integer::from(&q * c)).rem_euc(p);
        }
        r = trimmed(r);
    }
    r
}

fn poly_mul_rem(a: &[Integer], b: &[Integer], f: &[Integer], p: &Integer) -> Vec<Integer> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![Integer::new(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j] += Integer::from(x * y);
        }
    }
    for c in &mut out {
        *c = std::mem::take(c).rem_euc(p);
    }
    poly_rem(&trimmed(out), f, p)
}

fn poly_pow_rem(base: &[Integer], e: &Integer, f: &[Integer], p: &Integer) -> Vec<Integer> {
    let mut result = poly_rem(&[Integer::from(1)], f, p);
    let base = poly_rem(base, f, p);
    for bit in (0..e.significant_bits()).rev() {
        result = poly_mul_rem(&result, &result, f, p);
        if e.get_bit(bit) {
            result = poly_mul_rem(&result, &base, f, p);
        }
    }
    result
}

fn poly_gcd(a: &[Integer], b: &[Integer], p: &Integer) -> Vec<Integer> {
    let (mut a, mut b) = (a.to_vec(), b.to_vec());
    while !b.is_empty() {
        let r = poly_rem(&a, &b, p);
        a = b;
        b = r;
    }
    a
}

/// The roots of a product of different linear factors modulo an odd prime: the factors with
/// `(x + a)^((p - 1)/2) = 1` are split off for one shift `a` after another.
fn split_roots(g: &[Integer], p: &Integer, shift: &mut u32, out: &mut Vec<Integer>) -> Option<()> {
    match g.len() {
        0 | 1 => return Some(()),
        2 => {
            let lead = ntheory::mod_inverse(&g[1], p)?;
            out.push((-Integer::from(&g[0] * &lead)).rem_euc(p));
            return Some(());
        }
        _ => {}
    }
    let half = Integer::from(p - 1u32) >> 1;
    for _ in 0..200 {
        *shift += 1;
        let mut power = poly_pow_rem(&[Integer::from(*shift), Integer::from(1)], &half, g, p);
        if power.is_empty() {
            power.push(Integer::new());
        }
        power[0] = Integer::from(&power[0] - 1u32).rem_euc(p);
        let h = poly_gcd(g, &trimmed(power), p);
        if h.len() > 1 && h.len() < g.len() {
            // g / h by the remainders of the long division
            let mut quotient = vec![Integer::new(); g.len() - h.len() + 1];
            let mut r = g.to_vec();
            let lead = ntheory::mod_inverse(h.last()?, p)?;
            while r.len() >= h.len() {
                let k = r.len() - h.len();
                let q = Integer::from(r.last()? * &lead).rem_euc(p);
                for (i, c) in h.iter().enumerate() {
                    r[k + i] = (&r[k + i] - Integer::from(&q * c)).rem_euc(p);
                }
                quotient[k] = q;
                r.pop();
            }
            split_roots(&h, p, shift, out)?;
            return split_roots(&trimmed(quotient), p, shift, out);
        }
    }
    None
}

/// The roots of a polynomial modulo a prime that is too large for a search.
fn roots_mod_prime(coeffs: &[Integer], p: &Integer) -> Result<Vec<Integer>, String> {
    let f = trimmed(coeffs.iter().map(|c| c.clone().rem_euc(p)).collect());
    if f.is_empty() {
        return Err(format!("more than {LIMIT} solutions"));
    }
    if f.len() == 1 {
        return Ok(Vec::new());
    }
    // gcd(f, x^p - x): the product of the linear factors of f
    let x = [Integer::new(), Integer::from(1)];
    let mut power = poly_pow_rem(&x, p, &f, p);
    power.resize(power.len().max(2), Integer::new());
    power[1] = Integer::from(&power[1] - 1u32).rem_euc(p);
    let g = poly_gcd(&f, &trimmed(power), p);
    let mut roots = Vec::new();
    split_roots(&g, p, &mut 0, &mut roots)
        .ok_or("the roots modulo a prime factor were not found")?;
    roots.sort();
    Ok(roots)
}

/// The residues `x` modulo `m` with `f(x) = 0 (mod m)`.
pub fn mod_solve(f: &Expr, var: &str, m: &Integer) -> Result<Vec<Integer>, String> {
    if *m < 1 {
        return Err("the modulus must be a positive whole number".into());
    }
    let (poly, lcm) = polynomial(f, &[var.to_string()]).ok_or_else(|| {
        format!("modsolve needs a polynomial in {var} with rational coefficients")
    })?;
    if Integer::from(lcm.gcd_ref(m)) != 1 {
        return Err("a denominator of the equation has a factor in common with the modulus".into());
    }
    let degree = poly.iter().map(|(e, _)| e[0]).max().unwrap_or(0) as usize;
    let mut coeffs = vec![Integer::new(); degree + 1];
    for (e, c) in &poly {
        coeffs[e[0] as usize] = c.clone();
    }
    let slope: Vec<Integer> = coeffs
        .iter()
        .enumerate()
        .skip(1)
        .map(|(k, c)| Integer::from(c * k as u32))
        .collect();
    if *m == 1 {
        return Ok(vec![Integer::new()]);
    }
    let factors = ntheory::factor(m).ok_or("the modulus could not be factored")?;
    let mut parts: Vec<(Vec<Integer>, Integer)> = Vec::new();
    for (p, k) in &factors {
        let small = p.to_u64().filter(|p| *p <= 100_000);
        let mut roots: Vec<Integer> = match small {
            Some(small) => (0..small)
                .map(Integer::from)
                .filter(|r| horner(&coeffs, r, p) == 0)
                .collect(),
            None => roots_mod_prime(&coeffs, p)?,
        };
        let mut modulus = p.clone();
        for _ in 1..*k {
            let next = Integer::from(&modulus * p);
            let mut lifted = Vec::new();
            for r in &roots {
                let value = horner(&coeffs, r, &next);
                let d = horner(&slope, r, p);
                if d != 0 {
                    // one lift: r - f(r)/f'(r)
                    let inverse = ntheory::mod_inverse(&d, p).ok_or("no inverse")?;
                    let t = (-value.div_exact(&modulus) * inverse).rem_euc(p);
                    lifted.push(r + t * &modulus);
                } else if value == 0 {
                    let small = small.ok_or_else(|| format!("more than {LIMIT} solutions"))?;
                    for t in 0..small {
                        lifted.push(r + Integer::from(t) * &modulus);
                        if lifted.len() > LIMIT {
                            return Err(format!("more than {LIMIT} solutions"));
                        }
                    }
                }
            }
            roots = lifted;
            modulus = next;
        }
        parts.push((roots, modulus));
    }
    // every choice of one root for each prime power
    let mut all: Vec<(Integer, Integer)> = vec![(Integer::new(), Integer::from(1))];
    for (roots, modulus) in parts {
        let mut next = Vec::new();
        for (r0, m0) in &all {
            for r in &roots {
                let pair = ntheory::crt(&[(r0.clone(), m0.clone()), (r.clone(), modulus.clone())])
                    .ok_or("the remainders do not combine")?;
                next.push(pair);
                if next.len() > LIMIT {
                    return Err(format!("more than {LIMIT} solutions"));
                }
            }
        }
        all = next;
    }
    let mut out: Vec<Integer> = all.into_iter().map(|(r, _)| r).collect();
    out.sort();
    Ok(out)
}

fn int(n: &Integer) -> Expr {
    Expr::integer(n.clone())
}

fn rule(v: &str, e: Expr) -> Expr {
    Expr::call("rule", vec![Expr::sym(v), e])
}

/// Names for the free whole numbers that are not variables of the equation.
fn parameters(count: usize, vars: &[String], base: &str) -> Vec<String> {
    let taken = |n: &str| vars.iter().any(|v| v == n);
    if count == 1 && !taken(base) {
        return vec![base.to_string()];
    }
    (1..=count)
        .map(|k| {
            let mut name = format!("{base}{k}");
            while taken(&name) {
                name.push('_');
            }
            name
        })
        .collect()
}

/// `a1 x1 + ... + an xn = c`: every solution with `n - 1` free whole numbers.
fn linear(a: &[Integer], c: &Integer, vars: &[String], notes: &mut Vec<String>) -> Res {
    let n = a.len();
    // g[k] = gcd(a[k], ..., a[n-1])
    let mut g = vec![Integer::new(); n + 1];
    for k in (0..n).rev() {
        g[k] = Integer::from(a[k].gcd_ref(&g[k + 1]));
    }
    if g[0] == 0 {
        return Err("the equation has no variable".into());
    }
    if !c.is_divisible(&g[0]) {
        notes.push(format!(
            "no solution: the right side is not a multiple of {}, the greatest common divisor of the coefficients",
            g[0]
        ));
        return Ok(Expr::list(Vec::new()));
    }
    let names = parameters(n - 1, vars, "t");
    // y is the value the rest of the sum must take, over g[k]: a[k] x_k + g[k+1] y_next = g[k] y
    let mut y = int(&Integer::from(c / &g[0]));
    let mut out = Vec::new();
    for k in 0..n {
        if k + 1 == n {
            // a[k] x = g[k] y with g[k] = |a[k]|
            let sign = if a[k] < 0 { -1 } else { 1 };
            out.push(rule(
                &vars[k],
                crate::simplify::simplify(&mul(vec![Expr::int(sign), y.clone()])),
            ));
            break;
        }
        if g[k + 1] == 0 {
            // the later coefficients are all zero: x_k is fixed, the others are free
            let sign = if a[k] < 0 { -1 } else { 1 };
            out.push(rule(
                &vars[k],
                crate::simplify::simplify(&mul(vec![Expr::int(sign), y.clone()])),
            ));
            for (j, v) in vars.iter().enumerate().skip(k + 1) {
                out.push(rule(v, Expr::sym(&names[j - 1])));
            }
            break;
        }
        let (gcd, u, v) = ntheory::extended_gcd(&a[k], &g[k + 1]);
        debug_assert_eq!(gcd, g[k]);
        let t = Expr::sym(&names[k]);
        // x_k = u y + (g[k+1]/g[k]) t, y_next = v y - (a[k]/g[k]) t
        let x = add(vec![
            mul(vec![int(&u), y.clone()]),
            mul(vec![int(&Integer::from(&g[k + 1] / &g[k])), t.clone()]),
        ]);
        let next = add(vec![
            mul(vec![int(&v), y.clone()]),
            neg(mul(vec![int(&Integer::from(&a[k] / &g[k])), t])),
        ]);
        out.push(rule(
            &vars[k],
            crate::simplify::simplify(&crate::expand::expand(&x)),
        ));
        y = crate::expand::expand(&next);
    }
    if n > 1 {
        notes.push(format!(
            "{} {} any whole number{}",
            names.join(", "),
            if n > 2 { "are" } else { "is" },
            if n > 2 { "s" } else { "" }
        ));
    }
    Ok(Expr::list(out))
}

fn pair(x: &Integer, y: &Integer) -> Expr {
    Expr::list(vec![int(x), int(y)])
}

/// `x^2 - d y^2 = n` for a `d > 0` that is not a square: the classes of solutions.
fn pell(d: &Integer, n: &Integer, vars: &[String], notes: &mut Vec<String>) -> Res {
    if *n == 0 {
        return Ok(Expr::list(vec![pair(&Integer::new(), &Integer::new())]));
    }
    let (x1, y1) = ntheory::pell(d).ok_or("no fundamental solution found")?;
    // v up to y1 sqrt(|n| / (2 (x1 +- 1)))
    let low = if *n > 0 {
        Integer::from(&x1 + 1u32) * 2u32
    } else {
        Integer::from(&x1 - 1u32) * 2u32
    };
    let bound = (Integer::from(y1.square_ref()) * Integer::from(n.abs_ref()))
        .div_floor(&low)
        .sqrt();
    let steps = bound
        .to_usize()
        .filter(|b| *b <= 20_000_000)
        .ok_or("the search for the first solutions is too long for this equation")?;
    let mut bases: Vec<(Integer, Integer)> = Vec::new();
    for v in 0..=steps {
        let v = Integer::from(v);
        let u2 = n + d * Integer::from(v.square_ref());
        if u2 < 0 || !ntheory::is_square(&u2) {
            continue;
        }
        let u = u2.sqrt();
        bases.push((u.clone(), v.clone()));
        if u != 0 && v != 0 {
            bases.push((Integer::from(-&u), v));
        }
    }
    if bases.is_empty() {
        notes.push("no solution".to_string());
        return Ok(Expr::list(Vec::new()));
    }
    let k = parameters(1, vars, "n").remove(0);
    let root = sqrt(int(d));
    let alpha = add(vec![int(&x1), mul(vec![int(&y1), root.clone()])]);
    let beta = add(vec![int(&x1), neg(mul(vec![int(&y1), root.clone()]))]);
    let (an, bn) = (pow(alpha, Expr::sym(&k)), pow(beta, Expr::sym(&k)));
    let mut families = Vec::new();
    for (u, v) in &bases {
        let plus = add(vec![int(u), mul(vec![int(v), root.clone()])]);
        let minus = add(vec![int(u), neg(mul(vec![int(v), root.clone()]))]);
        let x = div(
            add(vec![
                mul(vec![plus.clone(), an.clone()]),
                mul(vec![minus.clone(), bn.clone()]),
            ]),
            Expr::int(2),
        );
        let y = div(
            add(vec![
                mul(vec![plus, an.clone()]),
                neg(mul(vec![minus, bn.clone()])),
            ]),
            mul(vec![Expr::int(2), root.clone()]),
        );
        families.push(Expr::list(vec![rule(&vars[0], x), rule(&vars[1], y)]));
    }
    notes.push(format!(
        "{k} is any whole number; with every solution both signs changed is one too; the least solution of {}^2 - {d} {}^2 = 1 is ({x1}, {y1})",
        vars[0], vars[1]
    ));
    Ok(if families.len() == 1 {
        families.remove(0)
    } else {
        Expr::list(families)
    })
}

/// `a x^2 + b x y + c y^2 + d x + e y + f = 0` in two unknowns.
fn quadratic(q: &[Integer; 6], vars: &[String], notes: &mut Vec<String>) -> Option<Res> {
    let [a, b, c, d, e, f] = q;
    let zero = Integer::new();
    if *a == 0 && *c == 0 && *b != 0 {
        // (b x + e)(b y + d) = d e - b f
        let n = Integer::from(d * e) - Integer::from(b * f);
        if n == 0 {
            let mut lines = Vec::new();
            let t = parameters(1, vars, "t").remove(0);
            if e.is_divisible(b) {
                lines.push(Expr::list(vec![
                    rule(&vars[0], int(&Integer::from(-e).div_exact(b))),
                    rule(&vars[1], Expr::sym(&t)),
                ]));
            }
            if d.is_divisible(b) {
                lines.push(Expr::list(vec![
                    rule(&vars[0], Expr::sym(&t)),
                    rule(&vars[1], int(&Integer::from(-d).div_exact(b))),
                ]));
            }
            notes.push(format!("{t} is any whole number"));
            return Some(Ok(Expr::list(lines)));
        }
        let divisors = match ntheory::divisors(&Integer::from(n.abs_ref())) {
            Some(d) => d,
            None => return Some(Err("the number could not be factored".into())),
        };
        let mut out = Vec::new();
        for u in &divisors {
            for sign in [1, -1] {
                let u = Integer::from(u * sign);
                let w = Integer::from(&n / &u);
                let (top_x, top_y) = (Integer::from(&u - e), Integer::from(&w - d));
                if top_x.is_divisible(b) && top_y.is_divisible(b) {
                    out.push((top_x.div_exact(b), top_y.div_exact(b)));
                }
            }
        }
        out.sort();
        return Some(Ok(Expr::list(
            out.iter().map(|(x, y)| pair(x, y)).collect(),
        )));
    }
    let disc = Integer::from(b.square_ref()) - Integer::from(a * c) * 4u32;
    if disc < 0 {
        // an ellipse: x between the roots of (b x + e)^2 - 4 c (a x^2 + d x + f) = 0
        let qa = disc.clone();
        let qb = (Integer::from(b * e) - Integer::from(c * d) * 2u32) * 2u32;
        let qc = Integer::from(e.square_ref()) - Integer::from(c * f) * 4u32;
        let inner = Integer::from(qb.square_ref()) - Integer::from(&qa * &qc) * 4u32;
        if inner < 0 {
            return Some(Ok(Expr::list(Vec::new())));
        }
        let root = inner.sqrt() + 1u32;
        let two_a = Integer::from(&qa * 2u32);
        let minus_b = Integer::from(-&qb);
        let ends = [
            Integer::from(&minus_b - &root).div_floor(&two_a),
            Integer::from(&minus_b + &root).div_floor(&two_a),
        ];
        let (lo, hi) = (
            Integer::from(ends.iter().min().expect("two ends")) - 1u32,
            Integer::from(ends.iter().max().expect("two ends")) + 1u32,
        );
        if Integer::from(&hi - &lo) > 20_000_000 {
            return Some(Err("the search is too long for this equation".into()));
        }
        let mut out = Vec::new();
        let mut x = lo;
        while x <= hi {
            // c y^2 + (b x + e) y + (a x^2 + d x + f) = 0
            let lin = Integer::from(b * &x) + e;
            let rest = a * Integer::from(x.square_ref()) + Integer::from(d * &x) + f;
            let dy = Integer::from(lin.square_ref()) - Integer::from(c * &rest) * 4u32;
            if dy >= 0 && ntheory::is_square(&dy) {
                let s = dy.sqrt();
                let two_c = Integer::from(c * 2u32);
                for sign in [1, -1] {
                    let top = Integer::from(-&lin) + Integer::from(&s * sign);
                    if top.is_divisible(&two_c) {
                        let y = top.div_exact(&two_c);
                        if !out.contains(&(x.clone(), y.clone())) {
                            out.push((x.clone(), y));
                        }
                    }
                }
            }
            x += 1u32;
        }
        out.sort();
        return Some(Ok(Expr::list(
            out.iter().map(|(x, y)| pair(x, y)).collect(),
        )));
    }
    if *b == zero && *d == zero && *e == zero && *a != zero && *c != zero {
        // a x^2 + c y^2 = -f with a and c of different signs
        let (lead, other, swap) = if a.clone().abs() == 1 {
            (a, c, false)
        } else if c.clone().abs() == 1 {
            (c, a, true)
        } else {
            return None;
        };
        // lead u^2 + other v^2 = -f  ->  u^2 - D v^2 = N
        let (dd, n) = if *lead == 1 {
            (Integer::from(-other), Integer::from(-f))
        } else {
            (other.clone(), f.clone())
        };
        let names: Vec<String> = if swap {
            vec![vars[1].clone(), vars[0].clone()]
        } else {
            vars.to_vec()
        };
        let order = |u: &Integer, v: &Integer| if swap { pair(v, u) } else { pair(u, v) };
        if dd >= 0 && ntheory::is_square(&dd) {
            // (u - s v)(u + s v) = N
            let s = dd.clone().sqrt();
            if n == 0 {
                let t = parameters(1, vars, "t").remove(0);
                notes.push(format!("{t} is any whole number"));
                let fam = |sign: i32| {
                    Expr::list(vec![
                        rule(
                            &names[0],
                            mul(vec![int(&Integer::from(&s * sign)), Expr::sym(&t)]),
                        ),
                        rule(&names[1], Expr::sym(&t)),
                    ])
                };
                return Some(Ok(Expr::list(vec![fam(1), fam(-1)])));
            }
            let divisors = match ntheory::divisors(&Integer::from(n.abs_ref())) {
                Some(d) => d,
                None => return Some(Err("the number could not be factored".into())),
            };
            let mut out = Vec::new();
            let two_s = Integer::from(&s * 2u32);
            for p in &divisors {
                for sign in [1, -1] {
                    let p = Integer::from(p * sign);
                    let q = Integer::from(&n / &p);
                    // u - s v = p, u + s v = q
                    let (su, dv) = (Integer::from(&p + &q), Integer::from(&q - &p));
                    if su.is_even() && dv.is_divisible(&two_s) {
                        let found = (su / 2u32, dv.div_exact(&two_s));
                        if !out.contains(&found) {
                            out.push(found);
                        }
                    }
                }
            }
            out.sort();
            return Some(Ok(Expr::list(
                out.iter().map(|(u, v)| order(u, v)).collect(),
            )));
        }
        if dd > 0 {
            return Some(pell(&dd, &n, &names, notes));
        }
    }
    None
}

/// The solutions with every unknown between `-bound` and `bound`.
fn search(poly: &Poly, vars: usize, bound: i64) -> Result<Vec<Vec<i64>>, String> {
    let side = 2 * bound as u128 + 1;
    let total = side
        .checked_pow(vars as u32)
        .filter(|t| *t <= 30_000_000)
        .ok_or("the box is too large: at most 30 million points are searched")?;
    let _ = total;
    let mut point = vec![-bound; vars];
    let mut out = Vec::new();
    loop {
        let mut sum = Integer::new();
        for (exps, c) in poly {
            let mut term = c.clone();
            for (x, e) in point.iter().zip(exps) {
                if *e > 0 {
                    term *= Integer::from(*x).pow(*e);
                }
            }
            sum += term;
        }
        if sum == 0 {
            out.push(point.clone());
            if out.len() > LIMIT {
                return Err(format!("more than {LIMIT} solutions"));
            }
        }
        let mut k = vars;
        loop {
            if k == 0 {
                return Ok(out);
            }
            k -= 1;
            if point[k] < bound {
                point[k] += 1;
                break;
            }
            point[k] = -bound;
        }
    }
}

/// `diophantine(equation, [x, y, ...])` and `diophantine(equation, [x, y, ...], bound)`.
pub fn diophantine(f: &Expr, vars: &[String], bound: Option<i64>, notes: &mut Vec<String>) -> Res {
    let (poly, _) = polynomial(f, vars).ok_or(
        "diophantine needs a polynomial equation with rational coefficients in the unknowns",
    )?;
    let n = vars.len();
    let coefficient = |exps: &[u32]| -> Integer {
        poly.iter()
            .find(|(e, _)| e == exps)
            .map_or_else(Integer::new, |(_, c)| c.clone())
    };
    let degree = poly
        .iter()
        .map(|(e, _)| e.iter().sum::<u32>())
        .max()
        .unwrap_or(0);
    if let Some(b) = bound {
        let found = search(&poly, n, b)?;
        notes.push(format!("the solutions with every unknown from -{b} to {b}"));
        return Ok(Expr::list(
            found
                .into_iter()
                .map(|p| {
                    if n == 1 {
                        Expr::int(p[0])
                    } else {
                        Expr::list(p.into_iter().map(Expr::int).collect())
                    }
                })
                .collect(),
        ));
    }
    if n == 1 && degree >= 1 {
        // one unknown: the whole roots of the polynomial
        let solved = crate::solve::solve(f, &vars[0])?;
        crate::solve::release(&solved.params);
        let mut roots: Vec<Integer> = solved
            .roots
            .iter()
            .filter_map(|r| r.value.as_int().cloned())
            .collect();
        roots.sort();
        roots.dedup();
        return Ok(Expr::list(roots.iter().map(int).collect()));
    }
    if degree <= 1 {
        let a: Vec<Integer> = (0..n)
            .map(|k| {
                let mut e = vec![0u32; n];
                e[k] = 1;
                coefficient(&e)
            })
            .collect();
        let c = -coefficient(&vec![0u32; n]);
        return linear(&a, &c, vars, notes);
    }
    if n == 2 && degree == 2 {
        let q = [
            coefficient(&[2, 0]),
            coefficient(&[1, 1]),
            coefficient(&[0, 2]),
            coefficient(&[1, 0]),
            coefficient(&[0, 1]),
            coefficient(&[0, 0]),
        ];
        if let Some(r) = quadratic(&q, vars, notes) {
            return r;
        }
    }
    if n == 3 && degree == 2 && poly.len() == 3 {
        // x^2 + y^2 = z^2 in any order of the unknowns
        let squares: Vec<(usize, Integer)> = (0..3)
            .filter_map(|k| {
                let mut e = vec![0u32; 3];
                e[k] = 2;
                let c = coefficient(&e);
                (c != 0).then_some((k, c))
            })
            .collect();
        if squares.len() == 3 {
            let total: Integer = squares.iter().map(|(_, c)| c.clone()).sum();
            let unit = squares.iter().all(|(_, c)| c.clone().abs() == 1);
            if unit && (total == 1 || total == -1) {
                let odd = squares
                    .iter()
                    .find(|(_, c)| Integer::from(c * &total) < 0)
                    .map(|(k, _)| *k)
                    .ok_or("the signs of the squares do not fit")?;
                let legs: Vec<usize> = (0..3).filter(|k| *k != odd).collect();
                let names = parameters(3, vars, "t");
                let (d, m, k) = (
                    Expr::sym(&names[0]),
                    Expr::sym(&names[1]),
                    Expr::sym(&names[2]),
                );
                let (m2, k2) = (pow(m.clone(), Expr::int(2)), pow(k.clone(), Expr::int(2)));
                let mut rules = vec![Expr::zero(); 3];
                rules[legs[0]] = rule(
                    &vars[legs[0]],
                    mul(vec![d.clone(), add(vec![m2.clone(), neg(k2.clone())])]),
                );
                rules[legs[1]] = rule(&vars[legs[1]], mul(vec![Expr::int(2), d.clone(), m, k]));
                rules[odd] = rule(&vars[odd], mul(vec![d, add(vec![m2, k2])]));
                notes.push(format!(
                    "{} are any whole numbers; the two legs may change places",
                    names.join(", ")
                ));
                return Ok(Expr::list(rules));
            }
        }
    }
    Err("this kind of Diophantine equation has no general method here: diophantine(equation, [x, y], 100) lists the solutions with every unknown from -100 to 100".to_string())
}
