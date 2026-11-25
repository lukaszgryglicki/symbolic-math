//! Algebraic expansion of products and integer powers of sums.

use crate::canon::{add, mul, pow, rebuild};
use crate::expr::Expr;
use rug::Integer;

/// A sum that is put together term by term. Like terms are joined whenever the terms that wait
/// have doubled: the products of two long sums never stand side by side all at once (the
/// products of (x + y + z)^64 with itself are millions, its square has 8385 terms).
struct Sum {
    terms: Vec<Expr>,
    joined: usize,
}

impl Sum {
    fn new() -> Sum {
        Sum {
            terms: Vec::new(),
            joined: 0,
        }
    }

    fn push(&mut self, term: Expr) {
        match term {
            Expr::Add(parts) => self.terms.extend(parts.iter().cloned()),
            other => self.terms.push(other),
        }
        if self.terms.len() >= 2 * self.joined + 4096 {
            self.terms = match add(std::mem::take(&mut self.terms)) {
                Expr::Add(parts) => parts.to_vec(),
                other => vec![other],
            };
            self.joined = self.terms.len();
        }
    }

    fn done(self) -> Expr {
        add(self.terms)
    }
}

fn mul_expand(a: &Expr, b: &Expr) -> Expr {
    match (a, b) {
        (Expr::Add(x), Expr::Add(y)) => {
            let mut sum = Sum::new();
            for s in x.iter() {
                for t in y.iter() {
                    sum.push(mul(vec![s.clone(), t.clone()]));
                }
            }
            sum.done()
        }
        (Expr::Add(x), _) => add(x.iter().map(|s| mul(vec![s.clone(), b.clone()])).collect()),
        (_, Expr::Add(y)) => add(y.iter().map(|t| mul(vec![a.clone(), t.clone()])).collect()),
        _ => mul(vec![a.clone(), b.clone()]),
    }
}

/// The most products that the multinomial theorem is asked for: a sum of terms that do not
/// run into each other has as many terms when it is multiplied out, and more than these are
/// no answer that anybody reads.
const PRODUCTS: u32 = 20_000_000;

/// How a power of a sum of `m` terms is multiplied out.
enum Way {
    /// By the multinomial theorem: one product for each way to share the exponent out.
    Shared,
    /// By squaring: the terms run into each other (as those of x^2 + x + 1 do), and the powers
    /// on the way are much shorter than their products are many.
    Squared,
    /// Not at all: the result would have more terms than `PRODUCTS`.
    Left,
}

/// A term whose powers and products with other such terms are single terms again: numbers,
/// names, functions and their powers. (A root of a sum is none: its square is a sum.)
fn plain(term: &Expr) -> bool {
    let atom = |e: &Expr| {
        matches!(
            e,
            Expr::Num(_) | Expr::Float(_) | Expr::Sym(_) | Expr::Fn(..)
        )
    };
    let factor = |e: &Expr| match e {
        Expr::Pow(base, _) => atom(base),
        other => atom(other),
    };
    match term {
        Expr::Mul(factors) => factors.iter().all(factor),
        other => factor(other),
    }
}

fn way(terms: &[Expr], n: u32) -> Way {
    let m = terms.len();
    if m > 64 || !terms.iter().all(plain) {
        return Way::Squared;
    }
    if m == 2 {
        return Way::Shared;
    }
    // do the products of two terms run into each other?
    let mut pairs = Sum::new();
    for (at, s) in terms.iter().enumerate() {
        for t in &terms[at..] {
            pairs.push(mul(vec![s.clone(), t.clone()]));
        }
    }
    let distinct = match pairs.done() {
        Expr::Add(parts) => parts.len(),
        _ => 1,
    };
    if distinct < m * (m + 1) / 2 {
        return Way::Squared;
    }
    let products = Integer::from(n + m as u32 - 1).binomial(m as u32 - 1);
    if products > PRODUCTS {
        Way::Left
    } else {
        Way::Shared
    }
}

/// The powers of the terms of a sum, each made when it is asked for the first time.
struct Powers<'a> {
    terms: &'a [Expr],
    known: Vec<Vec<Option<Expr>>>,
}

impl Powers<'_> {
    fn of(&mut self, term: usize, k: u32) -> Expr {
        let row = &mut self.known[term];
        if row.len() <= k as usize {
            row.resize(k as usize + 1, None);
        }
        if let Some(power) = &row[k as usize] {
            return power.clone();
        }
        let power = pow(self.terms[term].clone(), Expr::int(i64::from(k)));
        self.known[term][k as usize] = Some(power.clone());
        power
    }

    /// Every product of powers of the terms from `start` on whose exponents add up to `left`,
    /// times `partial` and the count `ways` of the orders in which its factors can stand.
    fn share(&mut self, start: usize, left: u32, ways: &Integer, partial: &Expr, sum: &mut Sum) {
        if left == 0 {
            sum.push(mul_expand(&Expr::integer(ways.clone()), partial));
            return;
        }
        let last = self.terms.len() - 1;
        for term in start..=last {
            // (the last term takes all that is left)
            let least = if term == last { left } else { 1 };
            let mut choose = Integer::from(left).binomial(least);
            for k in least..=left {
                let power = self.of(term, k);
                let product = mul_expand(partial, &power);
                self.share(
                    term + 1,
                    left - k,
                    &Integer::from(ways * &choose),
                    &product,
                    sum,
                );
                // C(left, k + 1) = C(left, k) (left - k)/(k + 1)
                choose *= left - k;
                choose /= k + 1;
            }
        }
    }
}

fn pow_expand(b: &Expr, n: u32) -> Expr {
    let Expr::Add(terms) = b else {
        return pow(b.clone(), Expr::int(i64::from(n)));
    };
    match way(terms, n) {
        Way::Left => pow(b.clone(), Expr::int(i64::from(n))),
        Way::Shared => {
            let mut powers = Powers {
                terms,
                known: vec![Vec::new(); terms.len()],
            };
            let mut sum = Sum::new();
            powers.share(0, n, &Integer::from(1), &Expr::one(), &mut sum);
            sum.done()
        }
        Way::Squared => {
            let (mut base, mut acc, mut k) = (b.clone(), Expr::one(), n);
            while k > 0 {
                if k & 1 == 1 {
                    acc = mul_expand(&acc, &base);
                }
                k >>= 1;
                if k > 0 {
                    base = mul_expand(&base, &base);
                }
            }
            acc
        }
    }
}

pub fn expand(e: &Expr) -> Expr {
    match e {
        Expr::Add(a) => add(a.iter().map(expand).collect()),
        Expr::Mul(a) => a
            .iter()
            .map(expand)
            .fold(Expr::one(), |acc, p| mul_expand(&acc, &p)),
        Expr::Pow(b, x) => {
            let (b, x) = (expand(b), expand(x));
            match x.as_i64() {
                Some(n) if matches!(b, Expr::Add(_)) && (2..=4096).contains(&n) => {
                    pow_expand(&b, n as u32)
                }
                Some(n) if matches!(b, Expr::Add(_)) && (-4096..=-2).contains(&n) => {
                    pow(pow_expand(&b, n.unsigned_abs() as u32), Expr::minus_one())
                }
                _ => {
                    let p = pow(b, x);
                    if matches!(p, Expr::Mul(_)) {
                        expand_once(&p)
                    } else {
                        p
                    }
                }
            }
        }
        Expr::Fn(..) | Expr::List(_) | Expr::Rel(..) => rebuild(e, &mut |c| expand(c)),
        _ => e.clone(),
    }
}

fn expand_once(e: &Expr) -> Expr {
    match e {
        Expr::Mul(a) => a.iter().fold(Expr::one(), |acc, p| mul_expand(&acc, p)),
        _ => e.clone(),
    }
}

/// Expansion of the numerator only: `(a + b)/c` keeps one denominator.
pub fn expand_numerator(e: &Expr) -> Expr {
    let (num, den) = crate::canon::numer_denom(e);
    crate::canon::div(expand(&num), den)
}
