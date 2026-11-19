//! Truncated power series (Taylor, Laurent, Puiseux) with symbolic coefficients.

use crate::canon::{add, div, mul, pow, sub, subst};
use crate::diff::diff;
use crate::expr::Expr;
use crate::funcs::func;
use crate::num::Rat;
use crate::simplify;

/// Name of the symbol that stands for the logarithm of the expansion variable in coefficients.
pub const LOG: &str = "#L";

/// `sum c[k] t^((lo + k)/den)`; the terms from exponent `(lo + c.len())/den` on are unknown.
#[derive(Clone, Debug)]
pub struct Series {
    pub lo: i64,
    pub den: i64,
    pub c: Vec<Expr>,
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}

fn norm(e: Expr) -> Expr {
    if e.is_number() {
        e
    } else {
        simplify::normal(&e)
    }
}

impl Series {
    fn constant(c: Expr, n: usize) -> Series {
        let mut v = vec![Expr::zero(); n];
        if n > 0 {
            v[0] = c;
        }
        Series {
            lo: 0,
            den: 1,
            c: v,
        }
    }
    fn end(&self) -> i64 {
        self.lo + self.c.len() as i64
    }
    /// The same series with exponents counted in units of `1/den`.
    fn rescale(&self, den: i64) -> Series {
        if den == self.den {
            return self.clone();
        }
        let k = den / self.den;
        let mut c = Vec::with_capacity(self.c.len() * k as usize);
        for x in &self.c {
            c.push(x.clone());
            for _ in 1..k {
                c.push(Expr::zero());
            }
        }
        Series {
            lo: self.lo * k,
            den,
            c,
        }
    }
    fn common(a: &Series, b: &Series) -> (Series, Series) {
        let den = a.den / gcd(a.den, b.den) * b.den;
        (a.rescale(den), b.rescale(den))
    }
    /// Drops leading zero coefficients.
    pub fn trim(mut self) -> Series {
        let zeros = self.c.iter().take_while(|x| simplify::is_zero(x)).count();
        self.c.drain(..zeros);
        self.lo += zeros as i64;
        self
    }
    fn plus(&self, o: &Series) -> Series {
        let (a, b) = Series::common(self, o);
        let lo = a.lo.min(b.lo);
        let end = a.end().min(b.end());
        let at = |s: &Series, e: i64| {
            if e >= s.lo && e < s.end() {
                s.c[(e - s.lo) as usize].clone()
            } else {
                Expr::zero()
            }
        };
        Series {
            lo,
            den: a.den,
            c: (lo..end.max(lo))
                .map(|e| norm(add(vec![at(&a, e), at(&b, e)])))
                .collect(),
        }
    }
    fn times(&self, o: &Series) -> Series {
        let (a, b) = Series::common(self, o);
        let n = a.c.len().min(b.c.len());
        let mut c = vec![Vec::new(); n];
        for i in 0..n {
            if a.c[i].is_zero() {
                continue;
            }
            for j in 0..n - i {
                if !b.c[j].is_zero() {
                    c[i + j].push(mul(vec![a.c[i].clone(), b.c[j].clone()]));
                }
            }
        }
        Series {
            lo: a.lo + b.lo,
            den: a.den,
            c: c.into_iter().map(|t| norm(add(t))).collect(),
        }
    }
    fn scale(&self, k: &Expr) -> Series {
        Series {
            lo: self.lo,
            den: self.den,
            c: self
                .c
                .iter()
                .map(|x| norm(mul(vec![x.clone(), k.clone()])))
                .collect(),
        }
    }
    /// `(leading coefficient, w)` with `self = c t^(lo/den) (1 + w)`.
    fn split(&self) -> Option<(Expr, Series)> {
        let lead = self.c.first()?.clone();
        let mut w: Vec<Expr> = self
            .c
            .iter()
            .map(|x| norm(div(x.clone(), lead.clone())))
            .collect();
        w[0] = Expr::zero();
        Some((
            lead,
            Series {
                lo: 0,
                den: self.den,
                c: w,
            },
        ))
    }
    /// `(1 + w)^e` for a `w` without constant term.
    fn binomial(w: &Series, e: &Expr) -> Series {
        let n = w.c.len();
        let mut total = Series::constant(Expr::one(), n);
        total.den = w.den;
        let mut term = total.clone();
        for k in 1..n {
            let factor = div(sub(e.clone(), Expr::int(k as i64 - 1)), Expr::int(k as i64));
            term = term.times(w).scale(&factor);
            if term.c.iter().all(|x| x.is_zero()) {
                break;
            }
            total = total.plus(&term);
        }
        total
    }
    fn power(&self, e: &Expr) -> Option<Series> {
        let s = self.clone().trim();
        let (lead, w) = s.split()?;
        let body = Series::binomial(&w, e);
        if let Some(r) = e.as_rat() {
            let (p, q) = (r.numer().to_i64()?, r.denom().to_i64()?);
            let mut out = body.rescale(body.den * q).scale(&pow(lead, e.clone()));
            out.lo = s.lo * p;
            // exponents (lo p)/(den q): the body keeps its spacing of q units
            return Some(out);
        }
        (s.lo == 0).then(|| body.scale(&pow(lead, e.clone())))
    }
    /// `f(self)` for a function analytic at the constant term, by its Taylor series there.
    fn compose(&self, f: &dyn Fn(&Expr) -> Expr) -> Option<Series> {
        if self.lo < 0 && !self.c.is_empty() {
            let s = self.clone().trim();
            if s.lo < 0 {
                return None;
            }
            return s.compose(f);
        }
        let n = self.c.len();
        let s0 = if self.lo == 0 && n > 0 {
            self.c[0].clone()
        } else {
            Expr::zero()
        };
        let mut u = self.clone();
        if self.lo == 0 && n > 0 {
            u.c[0] = Expr::zero();
        }
        let z = Expr::sym("#z");
        let mut derivative = f(&z);
        let mut total = Series {
            lo: 0,
            den: self.den,
            c: vec![Expr::zero(); (self.end().max(0)) as usize],
        };
        let mut upow = Series::constant(Expr::one(), total.c.len());
        upow.den = self.den;
        let mut factorial = Rat::from(1);
        let valuation = u.clone().trim().lo.max(1);
        for k in 0..=(total.c.len() as i64 / valuation) {
            let value = simplify::simplify(&subst(&derivative, &z, &s0));
            if value.any(&|x| x.is_infinite()) {
                return None;
            }
            total = total.plus(&upow.scale(&div(value, Expr::rational(factorial.clone()))));
            upow = upow.times(&u);
            factorial *= Rat::from(k + 1);
            derivative = diff(&derivative, "#z");
        }
        Some(total)
    }
    /// The term of the power 0.
    fn constant_term(&self) -> Expr {
        usize::try_from(-self.lo)
            .ok()
            .and_then(|k| self.c.get(k))
            .cloned()
            .unwrap_or_else(Expr::zero)
    }
    /// `sum c(k) self^k` for a series without a constant term.
    fn power_sum(&self, coefficient: &dyn Fn(usize) -> Expr) -> Option<Series> {
        let u = self.clone().trim();
        if u.lo < 1 && !u.c.is_empty() {
            return None;
        }
        let terms = self.end().max(0) as usize;
        let mut total = Series {
            lo: 0,
            den: self.den,
            c: vec![Expr::zero(); terms],
        };
        let mut upow = Series::constant(Expr::one(), terms);
        upow.den = self.den;
        let valuation = u.lo.max(1) as usize;
        for k in 0..=(terms / valuation) {
            total = total.plus(&upow.scale(&coefficient(k)));
            upow = upow.times(self);
        }
        Some(total)
    }
    pub fn to_expr(&self, t: &Expr) -> Expr {
        add(self
            .c
            .iter()
            .enumerate()
            .filter(|(_, c)| !c.is_zero())
            .map(|(k, c)| {
                mul(vec![
                    c.clone(),
                    pow(
                        t.clone(),
                        Expr::rational(Rat::from((self.lo + k as i64, self.den))),
                    ),
                ])
            })
            .collect())
    }
}

/// Series of `e` in `var` around 0 with about `n` terms; logarithms of `var` appear as `#L`.
pub fn expand(e: &Expr, var: &str, n: usize) -> Option<Series> {
    if !e.has_sym(var) {
        return Some(Series::constant(e.clone(), n));
    }
    match e {
        Expr::Sym(_) => {
            let mut c = vec![Expr::zero(); n];
            c[0] = Expr::one();
            Some(Series { lo: 1, den: 1, c })
        }
        Expr::Add(a) => {
            let mut total: Option<Series> = None;
            for t in a.iter() {
                let s = expand(t, var, n)?;
                total = Some(match total {
                    Some(acc) => acc.plus(&s),
                    None => s,
                });
            }
            total
        }
        Expr::Mul(a) => {
            let mut total: Option<Series> = None;
            for t in a.iter() {
                let s = expand(t, var, n)?.trim();
                total = Some(match total {
                    Some(acc) => acc.times(&s),
                    None => s,
                });
            }
            total
        }
        Expr::Pow(b, x) => {
            if !x.has_sym(var) {
                let base = expand(b, var, n)?;
                if let Some(k) = x.as_i64().filter(|k| *k >= 0 && *k <= 64) {
                    let mut out = Series::constant(Expr::one(), n);
                    let base = base.trim();
                    for _ in 0..k {
                        out = out.times(&base);
                    }
                    return Some(out);
                }
                return base.power(x);
            }
            if b.is_sym("e") {
                return exponential(&expand(x, var, n)?);
            }
            let log = logarithm(&expand(b, var, n)?)?;
            exponential(&expand(x, var, n)?.trim().times(&log))
        }
        Expr::Fn(name, args)
            if args.len() == 1 && !matches!(&**name, "elliptick" | "elliptice") =>
        {
            let inner = expand(&args[0], var, n)?;
            if &**name == "ln" {
                return logarithm(&inner);
            }
            if &**name == "gamma"
                && let Some(pole) = inner
                    .clone()
                    .trim()
                    .constant_term()
                    .as_i64()
                    .filter(|k| (-200..=0).contains(k))
            {
                // at a pole: gamma(u) = gamma(u - pole + 1)/(u (u + 1) ... (u - pole))
                let u = &args[0];
                let low = mul((0..=-pole)
                    .map(|j| add(vec![u.clone(), Expr::int(j)]))
                    .collect());
                let top = func("gamma", vec![add(vec![u.clone(), Expr::int(1 - pole)])]);
                return expand(&div(top, low), var, n);
            }
            let name = name.clone();
            inner.compose(&|z| func(&name, vec![z.clone()]))
        }
        Expr::Fn(name, args) if &**name == "sinc" && args.len() == 1 => expand(
            &div(func("sin", vec![args[0].clone()]), args[0].clone()),
            var,
            n,
        ),
        Expr::Fn(name, args) => {
            // one argument holds the variable: the series in it
            let mut holders = (0..args.len()).filter(|k| args[*k].has_sym(var));
            let (Some(at), None) = (holders.next(), holders.next()) else {
                return None;
            };
            let inner = expand(&args[at], var, n)?;
            if let Some(coefficient) = maclaurin(name, args, at)
                && let Some(s) = inner.power_sum(&coefficient)
            {
                return Some(s);
            }
            inner.compose(&|z| {
                let mut a = args.to_vec();
                a[at] = z.clone();
                func(name, a)
            })
        }
        _ => None,
    }
}

/// The coefficients of the series at 0 of a function whose derivatives have no simple values
/// there: the hypergeometric functions and their kin.
fn maclaurin(name: &str, args: &[Expr], at: usize) -> Option<Box<dyn Fn(usize) -> Expr>> {
    let rising = |a: &Expr, k: usize| -> Expr {
        mul((0..k as i64)
            .map(|j| add(vec![a.clone(), Expr::int(j)]))
            .collect())
    };
    let factorial = |k: usize| Expr::integer(rug::Integer::from(rug::Integer::factorial(k as u32)));
    let hyper =
        move |upper: Vec<Expr>, lower: Vec<Expr>, scale: Expr| -> Box<dyn Fn(usize) -> Expr> {
            Box::new(move |k| {
                let top = mul(upper.iter().map(|a| rising(a, k)).collect());
                let low = mul(lower.iter().map(|b| rising(b, k)).collect());
                simplify::simplify(&mul(vec![
                    scale.clone(),
                    div(top, mul(vec![low, factorial(k)])),
                ]))
            })
        };
    let half_pi = div(Expr::pi(), Expr::int(2));
    let last = args.len() - 1;
    Some(match (name, args.len()) {
        ("elliptick", 1) => hyper(vec![Expr::half(), Expr::half()], vec![Expr::one()], half_pi),
        ("elliptice", 1) => hyper(
            vec![Expr::rat(-1, 2), Expr::half()],
            vec![Expr::one()],
            half_pi,
        ),
        ("hyp0f1", 2) if at == last => hyper(vec![], vec![args[0].clone()], Expr::one()),
        ("hyp1f1", 3) if at == last => {
            hyper(vec![args[0].clone()], vec![args[1].clone()], Expr::one())
        }
        ("hyp2f1", 4) if at == last => hyper(
            vec![args[0].clone(), args[1].clone()],
            vec![args[2].clone()],
            Expr::one(),
        ),
        ("hypergeometric", 3) if at == last => hyper(
            args[0].as_list()?.to_vec(),
            args[1].as_list()?.to_vec(),
            Expr::one(),
        ),
        ("polylog", 2) if at == last => {
            let s = args[0].clone();
            Box::new(move |k| {
                if k == 0 {
                    Expr::zero()
                } else {
                    pow(Expr::int(k as i64), crate::canon::neg(s.clone()))
                }
            })
        }
        _ => return None,
    })
}

fn exponential(s: &Series) -> Option<Series> {
    s.compose(&|z| pow(Expr::e(), z.clone()))
}

fn logarithm(s: &Series) -> Option<Series> {
    let s = s.clone().trim();
    let (lead, w) = s.split()?;
    let n = w.c.len();
    let mut total = Series::constant(Expr::zero(), n);
    total.den = w.den;
    let mut term = Series::constant(Expr::one(), n);
    term.den = w.den;
    for k in 1..n {
        term = term.times(&w);
        if term.c.iter().all(|x| x.is_zero()) {
            break;
        }
        let sign = if k % 2 == 1 { 1 } else { -1 };
        total = total.plus(&term.scale(&Expr::rat(sign, k as i64)));
    }
    let mut constant = func("ln", vec![lead]);
    if s.lo != 0 {
        constant = add(vec![
            constant,
            mul(vec![
                Expr::rational(Rat::from((s.lo, s.den))),
                Expr::sym(LOG),
            ]),
        ]);
    }
    if total.c.is_empty() {
        return None;
    }
    total.c[0] = add(vec![total.c[0].clone(), constant]);
    Some(total)
}

/// `series(f, x, a, order)`: terms up to `(x - a)^order` and the order term, in ascending order.
pub fn series_expr(f: &Expr, var: &str, point: &Expr, order: i64) -> Option<Expr> {
    let x = Expr::sym(var);
    let t = Expr::sym("#t");
    let (shifted, back) = if point.is_inf() {
        (
            subst(f, &x, &pow(t.clone(), Expr::minus_one())),
            pow(x.clone(), Expr::minus_one()),
        )
    } else if point.is_neg_inf() {
        (
            subst(f, &x, &crate::canon::neg(pow(t.clone(), Expr::minus_one()))),
            crate::canon::neg(pow(x.clone(), Expr::minus_one())),
        )
    } else {
        (
            subst(f, &x, &add(vec![point.clone(), t.clone()])),
            sub(x.clone(), point.clone()),
        )
    };
    let mut n = (order.max(0) as usize) + 2;
    for _ in 0..6 {
        let s = expand(&shifted, "#t", n)?.trim();
        if s.c.is_empty() || s.end() <= order * s.den {
            n = n * 2 + 2;
            continue;
        }
        let log = func("ln", vec![back.clone()]);
        let mut terms: Vec<Expr> = Vec::new();
        for (k, c) in s.c.iter().enumerate() {
            let e = Rat::from((s.lo + k as i64, s.den));
            if e > order || c.is_zero() {
                continue;
            }
            let c = simplify::simplify(&subst(c, &Expr::sym(LOG), &log));
            terms.push(mul(vec![c, pow(back.clone(), Expr::rational(e))]));
        }
        terms.push(Expr::call(
            "O",
            vec![pow(back.clone(), Expr::int(order + 1))],
        ));
        return Some(Expr::call("ordered", terms));
    }
    None
}
