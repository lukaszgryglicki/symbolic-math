//! Antiderivatives by rules (table, exponential polynomials, rational functions, substitution,
//! parts), checked by differentiation; definite integrals with a numeric cross-check.

use crate::canon::{add, div, map_bottom_up, mul, neg, pow, powi, sqrt, sub, subst};
use crate::complex;
use crate::diff::diff;
use crate::expand::expand;
use crate::expr::{Expr, Rel};
use crate::funcs::func;
use crate::limit::{self, Dir};
use crate::num::Rat;
use crate::numeric::{self, Env};
use crate::poly::Poly;
use crate::quad;
use crate::ratsimp;
use crate::simplify::{self, simplify};
use crate::solve;
use crate::trace;
use crate::zfactor;
use rug::{Complete, Complex, Float};
use std::rc::Rc;

fn f1(name: &str, u: &Expr) -> Expr {
    func(name, vec![u.clone()])
}

fn rat_expr(r: &Rat) -> Expr {
    Expr::rational(r.clone())
}

struct Cx {
    var: String,
    x: Expr,
    depth: u32,
}

impl Cx {
    fn has(&self, e: &Expr) -> bool {
        e.has_sym(&self.var)
    }

    /// `(a, b)` for `u = a x + b`.
    fn linear(&self, u: &Expr) -> Option<(Expr, Expr)> {
        let cs = ratsimp::coefficients(u, &self.var)?;
        (cs.len() == 2).then(|| (cs[1].clone(), cs[0].clone()))
    }

    fn inner(&self, var: &str) -> Cx {
        Cx {
            var: var.to_string(),
            x: Expr::sym(var),
            depth: self.depth + 1,
        }
    }

    fn go(&mut self, f: &Expr) -> Option<Expr> {
        if self.depth > 14 || crate::budget::expired() {
            return None;
        }
        if !self.has(f) {
            return Some(mul(vec![f.clone(), self.x.clone()]));
        }
        self.depth += 1;
        // (the steps of a way that finds no integral are not told)
        let r = trace::trial(|| self.rules(f));
        self.depth -= 1;
        r
    }

    /// The variable as the steps write it: `t`, `u`, `w` for the ones of substitutions.
    fn shown(&self, e: &Expr) -> String {
        shown(e)
    }

    fn rules(&mut self, f: &Expr) -> Option<Expr> {
        let var = self.shown(&self.x);
        let of = |method: &str| format!("integral of {} d{var}: {method}", shown(f));
        // a way that was begun and led nowhere takes its steps back
        let begun = trace::mark();
        if let Expr::Add(terms) = f {
            trace::step(|| of("term by term"));
            let nest = trace::nest();
            let parts: Option<Vec<Expr>> = terms.iter().map(|t| self.go(t)).collect();
            drop(nest);
            if let Some(parts) = parts {
                return Some(add(parts));
            }
            trace::forget(begun);
        }
        if let Expr::Mul(factors) = f {
            let (with, without): (Vec<Expr>, Vec<Expr>) =
                factors.iter().cloned().partition(|t| self.has(t));
            if !without.is_empty() {
                let front = mul(without);
                trace::step(|| {
                    of(&format!(
                        "the factor {} without {var} stands in front",
                        shown(&front)
                    ))
                });
                let _nest = trace::nest();
                return Some(mul(vec![front, self.go(&mul(with))?]));
            }
        }
        let told = |method: &str, r: &Expr| {
            trace::step(|| format!("integral of {} d{var} = {} ({method})", shown(f), shown(r)));
        };
        if let Some(r) = self.table(f) {
            told("a known integral", &r);
            return Some(r);
        }
        if let Some(r) = self.exp_poly(f) {
            told(
                "powers of the variable times exponentials, sines or cosines: known integrals",
                &r,
            );
            return Some(r);
        }
        if let Some(r) = self.rational(f) {
            told(
                "a ratio of polynomials: divided, then split into partial fractions",
                &r,
            );
            return Some(r);
        }
        if let Some(r) = self.by_parts(f) {
            return Some(r);
        }
        trace::forget(begun);
        if let Some(r) = self.substitution(f) {
            return Some(r);
        }
        trace::forget(begun);
        if let Some(r) = self.half_angle(f) {
            return Some(r);
        }
        trace::forget(begun);
        let ex = expand(f);
        if ex != *f && matches!(ex, Expr::Add(_)) {
            trace::step(|| of(&format!("multiplied out, it is {}", shown(&ex))));
            let _nest = trace::nest();
            return self.go(&ex);
        }
        None
    }

    /// `u` for `e = base^p` with `base = c u` and `u = a x + b` given: whether `e` is that power.
    fn power_of(&self, e: &Expr, base: &Expr) -> Option<Expr> {
        if e == base {
            return Some(Expr::one());
        }
        match e {
            Expr::Pow(b, p) if **b == *base && !self.has(p) => Some((**p).clone()),
            _ => None,
        }
    }

    /// Integrals that are special functions, and integrals of special functions.
    fn special_table(&self, f: &Expr) -> Option<Expr> {
        let x = &self.x;
        let one = Expr::one;
        let call2 = |name: &str, a: Expr, b: &Expr| func(name, vec![a, b.clone()]);
        match f {
            Expr::Fn(name, args) if args.len() == 1 => {
                let u = &args[0];
                if matches!(&**name, "sin" | "cos")
                    && let Some(cs) = ratsimp::coefficients(u, &self.var)
                    && cs.len() == 3
                {
                    // a (x + b/(2a))^2 + c - b^2/(4a): Fresnel integrals
                    let (c, b, a) = (&cs[0], &cs[1], &cs[2]);
                    let t = add(vec![
                        x.clone(),
                        div(b.clone(), mul(vec![Expr::int(2), a.clone()])),
                    ]);
                    let phase = simplify(&sub(
                        c.clone(),
                        div(powi(b.clone(), 2), mul(vec![Expr::int(4), a.clone()])),
                    ));
                    let k = sqrt(div(mul(vec![Expr::int(2), a.clone()]), Expr::pi()));
                    let inner = mul(vec![k.clone(), t]);
                    let s = div(f1("fresnels", &inner), k.clone());
                    let c = div(f1("fresnelc", &inner), k);
                    let (sp, cp) = (f1("sin", &phase), f1("cos", &phase));
                    return Some(if &**name == "sin" {
                        add(vec![mul(vec![s, cp]), mul(vec![c, sp])])
                    } else {
                        sub(mul(vec![c, cp]), mul(vec![s, sp]))
                    });
                }
                let (a, _) = self.linear(u)?;
                let half_pi_square = mul(vec![Expr::pi(), powi(u.clone(), 2), Expr::half()]);
                let r = match &**name {
                    "fresnels" => add(vec![
                        mul(vec![u.clone(), f.clone()]),
                        div(f1("cos", &half_pi_square), Expr::pi()),
                    ]),
                    "fresnelc" => sub(
                        mul(vec![u.clone(), f.clone()]),
                        div(f1("sin", &half_pi_square), Expr::pi()),
                    ),
                    "heaviside" | "unitstep" => mul(vec![u.clone(), f.clone()]),
                    "diracdelta" => f1("heaviside", u),
                    "sinc" => f1("si", u),
                    "erfinv" => neg(div(
                        pow(Expr::e(), neg(powi(f.clone(), 2))),
                        sqrt(Expr::pi()),
                    )),
                    "li2" => add(vec![
                        mul(vec![u.clone(), f.clone()]),
                        neg(u.clone()),
                        mul(vec![
                            sub(u.clone(), one()),
                            f1("ln", &sub(one(), u.clone())),
                        ]),
                    ]),
                    "elliptick" => mul(vec![
                        Expr::int(2),
                        sub(
                            f1("elliptice", u),
                            mul(vec![sub(one(), u.clone()), f.clone()]),
                        ),
                    ]),
                    "elliptice" => mul(vec![
                        Expr::rat(2, 3),
                        sub(
                            mul(vec![add(vec![one(), u.clone()]), f.clone()]),
                            mul(vec![sub(one(), u.clone()), f1("elliptick", u)]),
                        ),
                    ]),
                    _ => return None,
                };
                Some(div(r, a))
            }
            Expr::Fn(name, args) if args.len() == 2 && !self.has(&args[0]) => {
                let (first, u) = (&args[0], &args[1]);
                let (a, _) = self.linear(u)?;
                let lower = |n: &str| call2(n, Expr::zero(), u);
                let next = add(vec![first.clone(), one()]);
                let r = match &**name {
                    "besselj" | "bessely" | "besselk" if first.is_one() => neg(lower(name)),
                    "besseli" if first.is_one() => lower(name),
                    "uppergamma" | "lowergamma" => {
                        sub(mul(vec![u.clone(), f.clone()]), call2(name, next, u))
                    }
                    "expint" => neg(call2(name, next, u)),
                    _ => return None,
                };
                Some(div(r, a))
            }
            Expr::Pow(base, e)
                if e.as_rat()
                    .is_some_and(|r| *r == Rat::from((1, 2)) || *r == Rat::from((-1, 2))) =>
            {
                // (A + B sin(u)^2)^(+-1/2): elliptic integrals
                let mut angle: Option<Expr> = None;
                let mut clash = false;
                let s = Expr::sym("#s");
                let replaced = map_bottom_up(base, &mut |node| {
                    if let Expr::Pow(b, p) = &node
                        && p.as_i64() == Some(2)
                        && let Expr::Fn(n, a) = &**b
                        && a.len() == 1
                        && matches!(&**n, "sin" | "cos")
                    {
                        match &angle {
                            Some(u) if *u != a[0] => clash = true,
                            Some(_) => {}
                            None => angle = Some(a[0].clone()),
                        }
                        return if &**n == "sin" {
                            s.clone()
                        } else {
                            sub(Expr::one(), s.clone())
                        };
                    }
                    node
                });
                let u = angle.filter(|_| !clash)?;
                let (a, _) = self.linear(&u)?;
                if self.has(&replaced) {
                    return None;
                }
                let cs = ratsimp::coefficients(&replaced, "#s").filter(|c| c.len() == 2)?;
                let m = simplify(&neg(div(cs[1].clone(), cs[0].clone())));
                let positive = e.as_rat().is_some_and(|r| r.is_positive());
                let name = if positive { "elliptice" } else { "ellipticf" };
                Some(div(
                    mul(vec![pow(cs[0].clone(), (**e).clone()), call2(name, u, &m)]),
                    a,
                ))
            }
            Expr::Mul(factors) => {
                // h(x) diracdelta(a x + b) picks the value of h at the zero
                if let Some(at) = factors.iter().position(
                    |t| matches!(t, Expr::Fn(n, a) if &**n == "diracdelta" && a.len() == 1),
                ) {
                    let u = &factors[at].children()[0];
                    let (a, b) = self.linear(u)?;
                    let zero = simplify(&neg(div(b, a.clone())));
                    let rest = mul(factors
                        .iter()
                        .enumerate()
                        .filter(|(k, _)| *k != at)
                        .map(|(_, t)| t.clone())
                        .collect());
                    let value = simplify(&subst(&rest, x, &zero));
                    if value.is_infinite() || value.is_undefined() {
                        return None;
                    }
                    return Some(div(mul(vec![value, f1("heaviside", u)]), a));
                }
                if factors.len() != 2 {
                    return None;
                }
                for (i, j) in [(0, 1), (1, 0)] {
                    let (p, g) = (&factors[i], &factors[j]);
                    // x^s e^(-k x): an incomplete gamma function
                    if let Some(s) = self.power_of(p, x)
                        && s.as_int().is_none()
                        && let Expr::Pow(b, w) = g
                        && b.is_sym("e")
                        && let Some((c, shift)) = self.linear(w)
                        && shift.is_zero()
                        && numeric::sign(&c) == Some(-1)
                    {
                        let k = neg(c);
                        let s1 = add(vec![s, one()]);
                        return Some(neg(mul(vec![
                            pow(k.clone(), neg(s1.clone())),
                            call2("uppergamma", s1, &mul(vec![k, x.clone()])),
                        ])));
                    }
                    let Expr::Fn(name, args) = g else {
                        continue;
                    };
                    let dilog = [Expr::int(2), args[0].clone()];
                    let (name, args): (&str, &[Expr]) = if &**name == "li2" && args.len() == 1 {
                        ("polylog", &dilog)
                    } else {
                        (&**name, &**args)
                    };
                    if args.len() != 2 || self.has(&args[0]) {
                        continue;
                    }
                    let (nu, u) = (&args[0], &args[1]);
                    let Some((a, shift)) = self.linear(u) else {
                        continue;
                    };
                    if !shift.is_zero() {
                        continue;
                    }
                    let Some(s) = self.power_of(p, x) else {
                        continue;
                    };
                    if name == "polylog" && s.is_minus_one() {
                        return Some(call2(name, add(vec![nu.clone(), one()]), u));
                    }
                    if !matches!(name, "besselj" | "bessely" | "besseli" | "besselk") {
                        continue;
                    }
                    // x^(nu+1) Z_nu(a x) and x^(1-nu) Z_nu(a x)
                    let up = simplify(&sub(s.clone(), add(vec![nu.clone(), one()]))).is_zero();
                    let down = simplify(&sub(s.clone(), sub(one(), nu.clone()))).is_zero();
                    let (order, sign) = if up {
                        (
                            add(vec![nu.clone(), one()]),
                            if name == "besselk" { -1 } else { 1 },
                        )
                    } else if down {
                        (
                            sub(nu.clone(), one()),
                            if name == "besseli" { 1 } else { -1 },
                        )
                    } else {
                        continue;
                    };
                    return Some(div(
                        mul(vec![Expr::int(sign), p.clone(), call2(name, order, u)]),
                        a,
                    ));
                }
                None
            }
            _ => None,
        }
    }

    fn table(&self, f: &Expr) -> Option<Expr> {
        let x = &self.x;
        if f == x {
            return Some(div(powi(x.clone(), 2), Expr::int(2)));
        }
        if let Some(r) = self.special_table(f) {
            return Some(r);
        }
        match f {
            Expr::Pow(b, e) if !self.has(e) => {
                if let Some((a, _)) = self.linear(b) {
                    if e.is_minus_one() {
                        return Some(div(f1("ln", b), a));
                    }
                    let n1 = add(vec![(**e).clone(), Expr::one()]);
                    return Some(div(pow((**b).clone(), n1.clone()), mul(vec![a, n1])));
                }
                if let Expr::Fn(name, args) = &**b
                    && args.len() == 1
                    && let Some((a, _)) = self.linear(&args[0])
                    && let Some(k) = e.as_i64()
                {
                    let u = &args[0];
                    let r = match (&**name, k) {
                        ("cos", -2) => f1("tan", u),
                        ("sin", -2) => neg(div(f1("cos", u), f1("sin", u))),
                        ("cos", -1) => f1("ln", &add(vec![f1("tan", u), powi(f1("cos", u), -1)])),
                        ("sin", -1) => f1("ln", &f1("tan", &div(u.clone(), Expr::int(2)))),
                        ("tan", -1) => f1("ln", &f1("sin", u)),
                        ("tan", 2) => sub(f1("tan", u), u.clone()),
                        ("cosh", -2) => f1("tanh", u),
                        ("sinh", -2) => neg(div(f1("cosh", u), f1("sinh", u))),
                        ("tanh", -1) => f1("ln", &f1("sinh", u)),
                        ("ln", -1) => f1("li", u),
                        _ => return None,
                    };
                    return Some(div(r, a));
                }
                let half = e
                    .as_rat()
                    .filter(|r| **r == Rat::from((1, 2)) || **r == Rat::from((-1, 2)))?;
                self.sqrt_quadratic(b, half.is_positive())
            }
            Expr::Pow(b, e) if !self.has(b) => {
                if let Some((a, _)) = self.linear(e) {
                    let log = if b.is_sym("e") {
                        Expr::one()
                    } else {
                        f1("ln", b)
                    };
                    return Some(div(f.clone(), mul(vec![a, log])));
                }
                if b.is_sym("e")
                    && let Some(cs) = ratsimp::coefficients(e, &self.var)
                    && cs.len() == 3
                {
                    let (c, b1, a) = (&cs[0], &cs[1], &cs[2]);
                    let k = sqrt(neg(a.clone()));
                    let shift = div(b1.clone(), mul(vec![Expr::int(2), a.clone()]));
                    let constant = pow(
                        Expr::e(),
                        sub(
                            c.clone(),
                            div(powi(b1.clone(), 2), mul(vec![Expr::int(4), a.clone()])),
                        ),
                    );
                    let erf = f1("erf", &mul(vec![k.clone(), add(vec![x.clone(), shift])]));
                    return Some(mul(vec![
                        sqrt(Expr::pi()),
                        constant,
                        erf,
                        powi(mul(vec![Expr::int(2), k]), -1),
                    ]));
                }
                None
            }
            Expr::Fn(name, args) if args.len() == 1 => {
                let u = &args[0];
                let (a, _) = self.linear(u)?;
                let one = Expr::one;
                let u2 = powi(u.clone(), 2);
                let r = match &**name {
                    "sin" => neg(f1("cos", u)),
                    "cos" => f1("sin", u),
                    "tan" => neg(f1("ln", &f1("cos", u))),
                    "sinh" => f1("cosh", u),
                    "cosh" => f1("sinh", u),
                    "tanh" => f1("ln", &f1("cosh", u)),
                    "ln" => sub(mul(vec![u.clone(), f.clone()]), u.clone()),
                    "asin" => add(vec![mul(vec![u.clone(), f.clone()]), sqrt(sub(one(), u2))]),
                    "acos" => sub(mul(vec![u.clone(), f.clone()]), sqrt(sub(one(), u2))),
                    "atan" => sub(
                        mul(vec![u.clone(), f.clone()]),
                        div(f1("ln", &add(vec![one(), u2])), Expr::int(2)),
                    ),
                    "asinh" => sub(mul(vec![u.clone(), f.clone()]), sqrt(add(vec![u2, one()]))),
                    "acosh" => sub(
                        mul(vec![u.clone(), f.clone()]),
                        mul(vec![
                            sqrt(sub(u.clone(), one())),
                            sqrt(add(vec![u.clone(), one()])),
                        ]),
                    ),
                    "atanh" => add(vec![
                        mul(vec![u.clone(), f.clone()]),
                        div(f1("ln", &sub(one(), u2)), Expr::int(2)),
                    ]),
                    "erf" => add(vec![
                        mul(vec![u.clone(), f.clone()]),
                        div(pow(Expr::e(), neg(u2)), sqrt(Expr::pi())),
                    ]),
                    "ei" => sub(mul(vec![u.clone(), f.clone()]), pow(Expr::e(), u.clone())),
                    "si" => add(vec![mul(vec![u.clone(), f.clone()]), f1("cos", u)]),
                    "ci" => sub(mul(vec![u.clone(), f.clone()]), f1("sin", u)),
                    "shi" => sub(mul(vec![u.clone(), f.clone()]), f1("cosh", u)),
                    "chi" => sub(mul(vec![u.clone(), f.clone()]), f1("sinh", u)),
                    "lambertw" => mul(vec![
                        u.clone(),
                        add(vec![f.clone(), Expr::minus_one(), powi(f.clone(), -1)]),
                    ]),
                    "sign" => f1("abs", u),
                    _ => return None,
                };
                Some(div(r, a))
            }
            Expr::Mul(factors) if factors.len() == 2 => {
                // g(c w)/w for linear w: sine, cosine and exponential integrals
                for (i, j) in [(0, 1), (1, 0)] {
                    let Expr::Pow(w, e) = &factors[i] else {
                        continue;
                    };
                    if !e.is_minus_one() {
                        continue;
                    }
                    let Some((a, b)) = self.linear(w) else {
                        continue;
                    };
                    let (name, v) = match &factors[j] {
                        Expr::Fn(n, args) if args.len() == 1 => (n.to_string(), args[0].clone()),
                        Expr::Pow(b, p) if b.is_sym("e") => ("exp".to_string(), (**p).clone()),
                        _ => continue,
                    };
                    // ln(c0 + c1 w)/w with c0 > 0: the dilogarithm
                    if name == "ln"
                        && let Some((q, p)) = self.linear(&v)
                    {
                        let c1 = simplify(&div(q.clone(), a.clone()));
                        let c0 = simplify(&sub(p, mul(vec![c1.clone(), b.clone()])));
                        if numeric::sign(&c0) != Some(1) {
                            continue;
                        }
                        let inside = neg(mul(vec![div(c1, c0.clone()), (**w).clone()]));
                        return Some(div(
                            sub(
                                mul(vec![f1("ln", &c0), f1("ln", w)]),
                                f1("li2", &simplify(&inside)),
                            ),
                            a,
                        ));
                    }
                    let ratio = simplify(&div(v.clone(), (**w).clone()));
                    if self.has(&ratio) || ratio.is_zero() {
                        continue;
                    }
                    let special = match name.as_str() {
                        "sin" => "si",
                        "cos" => "ci",
                        "sinh" => "shi",
                        "cosh" => "chi",
                        "exp" => "ei",
                        _ => continue,
                    };
                    return Some(div(f1(special, &v), a));
                }
                None
            }
            _ => None,
        }
    }

    /// Integral of `q^(1/2)` or `q^(-1/2)` for a quadratic `q`.
    fn sqrt_quadratic(&self, q: &Expr, positive_power: bool) -> Option<Expr> {
        let cs = ratsimp::coefficients(q, &self.var)?;
        if cs.len() != 3 {
            return None;
        }
        let (c, b, a) = (&cs[0], &cs[1], &cs[2]);
        let x = &self.x;
        let line = add(vec![
            mul(vec![Expr::int(2), a.clone(), x.clone()]),
            b.clone(),
        ]);
        let disc = simplify(&sub(
            mul(vec![Expr::int(4), a.clone(), c.clone()]),
            powi(b.clone(), 2),
        ));
        let root = sqrt(q.clone());
        let base = match numeric::sign(a) {
            Some(s) if s < 0 => neg(div(
                f1("asin", &div(line.clone(), sqrt(neg(disc.clone())))),
                sqrt(neg(a.clone())),
            )),
            Some(_) if numeric::sign(&disc) == Some(1) => div(
                f1("asinh", &div(line.clone(), sqrt(disc.clone()))),
                sqrt(a.clone()),
            ),
            _ => div(
                f1(
                    "ln",
                    &add(vec![
                        mul(vec![Expr::int(2), sqrt(a.clone()), root.clone()]),
                        line.clone(),
                    ]),
                ),
                sqrt(a.clone()),
            ),
        };
        if !positive_power {
            return Some(base);
        }
        Some(add(vec![
            div(mul(vec![line, root]), mul(vec![Expr::int(4), a.clone()])),
            mul(vec![div(disc, mul(vec![Expr::int(8), a.clone()])), base]),
        ]))
    }

    /// Sums of `c x^n e^(l x + m)`, also after writing sines and cosines with exponentials.
    fn exp_poly(&mut self, f: &Expr) -> Option<Expr> {
        let var = self.var.clone();
        let is = |e: &Expr, names: &[&str]| matches!(e, Expr::Fn(n, a) if names.contains(&&**n) && a.iter().any(|x| x.has_sym(&var)));
        if f.any(&|e| is(e, &["tan", "tanh"])) {
            return None;
        }
        let trig = f.any(&|e| is(e, &["sin", "cos", "sinh", "cosh"]));
        let g = expand(&if trig {
            solve::rewrite_exp(f)
        } else {
            f.clone()
        });
        let terms: Vec<Expr> = match &g {
            Expr::Add(a) => a.to_vec(),
            other => vec![other.clone()],
        };
        let mut total = Vec::new();
        for t in terms {
            let factors: Vec<Expr> = match &t {
                Expr::Mul(a) => a.to_vec(),
                other => vec![other.clone()],
            };
            let (mut coef, mut n, mut exponent) = (Vec::new(), 0u32, Vec::new());
            for fac in factors {
                if !self.has(&fac) {
                    coef.push(fac);
                    continue;
                }
                let (b, e) = fac.base_exp();
                if b == self.x {
                    n += u32::try_from(e.as_i64().filter(|k| (0..=300).contains(k))?).ok()?;
                } else if !self.has(&b) {
                    exponent.push(if b.is_sym("e") {
                        e
                    } else {
                        mul(vec![e, f1("ln", &b)])
                    });
                } else {
                    return None;
                }
            }
            let l = add(exponent);
            let cs = if l.is_zero() {
                Vec::new()
            } else {
                ratsimp::coefficients(&l, &var)?
            };
            if cs.len() == 3 && n <= 60 {
                // x^n e^q for a quadratic q = a x^2 + b x + c: with I_n its integral,
                // I_n = (x^(n-1) e^q - (n - 1) I_(n-2) - b I_(n-1))/(2a) = A_n e^q + B_n I_0
                let (b, a2) = (cs[1].clone(), mul(vec![Expr::int(2), cs[2].clone()]));
                let whole = pow(Expr::e(), l.clone());
                let base = self.table(&whole)?;
                let (mut a_prev, mut b_prev) = (Expr::zero(), Expr::one());
                let (mut a_cur, mut b_cur) =
                    (powi(a2.clone(), -1), neg(div(b.clone(), a2.clone())));
                if n == 0 {
                    (a_cur, b_cur) = (Expr::zero(), Expr::one());
                }
                for k in 2..=n {
                    let step = Expr::int(i64::from(k) - 1);
                    let a_next = expand(&div(
                        sub(
                            powi(self.x.clone(), i64::from(k) - 1),
                            add(vec![
                                mul(vec![step.clone(), a_prev.clone()]),
                                mul(vec![b.clone(), a_cur.clone()]),
                            ]),
                        ),
                        a2.clone(),
                    ));
                    let b_next = simplify(&neg(div(
                        add(vec![
                            mul(vec![step, b_prev.clone()]),
                            mul(vec![b.clone(), b_cur.clone()]),
                        ]),
                        a2.clone(),
                    )));
                    (a_prev, b_prev) = (a_cur, b_cur);
                    (a_cur, b_cur) = (a_next, b_next);
                }
                // the common denominator of the polynomial goes in front of the whole product
                let (top, low) = match ratsimp::coefficients(&a_cur, &var) {
                    Some(cs) if cs.iter().all(|c| c.as_rat().is_some()) => {
                        let mut lcm = rug::Integer::from(1);
                        for c in &cs {
                            lcm = lcm.lcm(c.as_rat().expect("checked").denom());
                        }
                        let scale = Expr::integer(lcm);
                        (expand(&mul(vec![scale.clone(), a_cur.clone()])), scale)
                    }
                    _ => (a_cur.clone(), Expr::one()),
                };
                total.push(mul(vec![
                    mul(coef),
                    add(vec![
                        mul(vec![top, whole, powi(low, -1)]),
                        mul(vec![b_cur, base]),
                    ]),
                ]));
                continue;
            }
            if cs.len() > 2 {
                return None;
            }
            let coef = mul(coef);
            let lambda = cs.get(1).cloned().unwrap_or_else(Expr::zero);
            if lambda.is_zero() {
                let constant = pow(Expr::e(), cs.first().cloned().unwrap_or_else(Expr::zero));
                total.push(mul(vec![
                    coef,
                    constant,
                    div(
                        powi(self.x.clone(), i64::from(n) + 1),
                        Expr::int(i64::from(n) + 1),
                    ),
                ]));
                continue;
            }
            let mut sum = Vec::new();
            let mut falling = Rat::from(1);
            for k in 0..=n {
                let sign = if k % 2 == 0 { 1 } else { -1 };
                sum.push(mul(vec![
                    Expr::rational(falling.clone() * sign),
                    powi(self.x.clone(), i64::from(n - k)),
                    powi(lambda.clone(), -(i64::from(k) + 1)),
                ]));
                falling *= Rat::from(n - k);
            }
            total.push(mul(vec![coef, pow(Expr::e(), l), add(sum)]));
        }
        let r = add(total);
        Some(if trig {
            simplify(&expand(&exp_to_trig(&r)))
        } else {
            r
        })
    }

    /// Rational functions with rational coefficients by partial fractions.
    fn rational(&mut self, f: &Expr) -> Option<Expr> {
        let var = self.var.clone();
        let (num, den) = ratsimp::fraction(f);
        let dense = |e: &Expr| -> Option<Vec<Rat>> {
            ratsimp::coefficients(e, &var)?
                .iter()
                .map(|c| c.as_rat().cloned())
                .collect()
        };
        let (p, q) = (dense(&num)?, dense(&den)?);
        if q.len() < 2 {
            return None;
        }
        let x = self.x.clone();
        let expr_of = |c: &[Rat]| {
            add(c
                .iter()
                .enumerate()
                .map(|(i, k)| mul(vec![rat_expr(k), powi(x.clone(), i as i64)]))
                .collect())
        };
        let (quotient, parts) = partial_fractions(&p, &q)?;
        let mut out = vec![add(quotient
            .iter()
            .enumerate()
            .map(|(i, k)| {
                mul(vec![
                    rat_expr(&(k / Rat::from(i as u32 + 1))),
                    powi(x.clone(), i as i64 + 1),
                ])
            })
            .collect())];
        for (a, g, j) in &parts {
            out.push(self.partial_term(a, g, *j, &expr_of(g))?);
        }
        Some(add(out))
    }

    /// Integral of `A/q^j` for an irreducible `q` and `deg A < deg q`.
    fn partial_term(&mut self, a: &[Rat], q: &[Rat], j: u32, q_expr: &Expr) -> Option<Expr> {
        let x = self.x.clone();
        let power = |k: i64| powi(q_expr.clone(), k);
        let j1 = 1 - i64::from(j);
        match q.len() - 1 {
            1 => {
                let k = (&a[0] / &q[1]).complete();
                Some(if j == 1 {
                    mul(vec![rat_expr(&k), f1("ln", q_expr)])
                } else {
                    mul(vec![rat_expr(&(k / Rat::from(j1))), power(j1)])
                })
            }
            2 => {
                let (c0, b, a2) = (&q[0], &q[1], &q[2]);
                let a1 = a.get(1).cloned().unwrap_or_default();
                let kappa = &a1 / (Rat::from(2) * a2);
                let beta = &a[0] - (&kappa * b).complete();
                let mut out = Vec::new();
                if !kappa.is_zero() {
                    out.push(if j == 1 {
                        mul(vec![rat_expr(&kappa), f1("ln", q_expr)])
                    } else {
                        mul(vec![rat_expr(&(kappa / Rat::from(j1))), power(j1)])
                    });
                }
                if !beta.is_zero() {
                    let disc = (b * b).complete() - Rat::from(4) * a2 * c0;
                    let line = add(vec![
                        mul(vec![rat_expr(&(Rat::from(2) * a2)), x.clone()]),
                        rat_expr(b),
                    ]);
                    let minus_d = rat_expr(&(-&disc).complete());
                    let mut integral = if disc.is_negative() {
                        let s = sqrt(minus_d.clone());
                        mul(vec![
                            Expr::int(2),
                            powi(s.clone(), -1),
                            f1("atan", &div(line.clone(), s)),
                        ])
                    } else {
                        let s = sqrt(rat_expr(&disc));
                        div(
                            f1(
                                "ln",
                                &div(
                                    sub(line.clone(), s.clone()),
                                    add(vec![line.clone(), s.clone()]),
                                ),
                            ),
                            s,
                        )
                    };
                    for step in 2..=j {
                        let s1 = Expr::int(i64::from(step) - 1);
                        let first = div(
                            line.clone(),
                            mul(vec![
                                s1.clone(),
                                minus_d.clone(),
                                power(i64::from(step) - 1),
                            ]),
                        );
                        let factor = div(
                            mul(vec![
                                rat_expr(&(Rat::from(2) * a2)),
                                Expr::int(2 * i64::from(step) - 3),
                            ]),
                            mul(vec![s1, minus_d.clone()]),
                        );
                        integral = add(vec![first, mul(vec![factor, integral])]);
                    }
                    out.push(mul(vec![rat_expr(&beta), integral]));
                }
                Some(add(out))
            }
            degree => {
                if j != 1 {
                    return None;
                }
                let solved = solve::solve(q_expr, &self.var).ok()?;
                if solved.roots.len() != degree
                    || solved.roots.iter().any(|r| r.approx)
                    || !solved.params.is_empty()
                {
                    return None;
                }
                let a_expr = add(a
                    .iter()
                    .enumerate()
                    .map(|(i, k)| mul(vec![rat_expr(k), powi(x.clone(), i as i64)]))
                    .collect());
                let dq = diff(q_expr, &self.var);
                let mut out = Vec::new();
                for r in &solved.roots {
                    let at = |e: &Expr| subst(e, &x, &r.value);
                    let c = simplify(&div(at(&a_expr), at(&dq)));
                    let (re, im) = complex::rect(&r.value)?;
                    let (re, im) = (simplify(&re), simplify(&im));
                    match numeric::sign(&im)? {
                        0 => out.push(mul(vec![c, f1("ln", &sub(x.clone(), r.value.clone()))])),
                        1 => {
                            let (u, v) = complex::rect(&c)?;
                            let shifted = sub(x.clone(), re);
                            out.push(mul(vec![
                                simplify(&u),
                                f1(
                                    "ln",
                                    &add(vec![powi(shifted.clone(), 2), powi(im.clone(), 2)]),
                                ),
                            ]));
                            out.push(mul(vec![
                                Expr::int(-2),
                                simplify(&v),
                                f1("atan", &div(shifted, im)),
                            ]));
                        }
                        _ => {}
                    }
                }
                Some(add(out))
            }
        }
    }

    fn candidates(&self, f: &Expr) -> Vec<Expr> {
        let mut out: Vec<Expr> = Vec::new();
        let x = &self.x;
        let mut push = |e: &Expr| {
            if self.has(e) && e != x && !out.contains(e) && out.len() < 14 {
                out.push(e.clone());
            }
        };
        f.walk(&mut |e| match e {
            Expr::Fn(_, args) => {
                push(e);
                args.iter().for_each(&mut push);
            }
            Expr::Pow(b, p) => {
                if self.has(p) {
                    push(e);
                    push(p);
                } else {
                    push(b);
                    if p.as_int().is_none() || (**b == *x && p.as_i64().is_some_and(|k| k >= 2)) {
                        push(e);
                    }
                }
            }
            _ => {}
        });
        out
    }

    /// `x` in terms of `t` when `u(x) = t` has a simple inverse.
    fn invert(&self, u: &Expr, t: &Expr) -> Option<Expr> {
        let back = |inner: &Expr, value: Expr| -> Option<Expr> {
            let (a, b) = self.linear(inner)?;
            Some(div(sub(value, b), a))
        };
        if self.linear(u).is_some() {
            return back(u, t.clone());
        }
        match u {
            Expr::Pow(b, p) if !self.has(p) => back(b, pow(t.clone(), powi((**p).clone(), -1))),
            Expr::Pow(b, p) if b.is_sym("e") => back(p, f1("ln", t)),
            Expr::Fn(n, a) if &**n == "ln" && a.len() == 1 => {
                back(&a[0], pow(Expr::e(), t.clone()))
            }
            _ => None,
        }
    }

    fn substitution(&mut self, f: &Expr) -> Option<Expr> {
        let name = format!("#u{}", self.depth);
        let t = Expr::sym(&name);
        let begun = trace::mark();
        for u in self.candidates(f) {
            trace::forget(begun);
            let du = diff(&u, &self.var);
            if du.is_zero() {
                continue;
            }
            let g = simplify(&div(f.clone(), du.clone()));
            let mut g2 = subst(&g, &u, &t);
            if self.has(&g2) {
                // x = t^(1/k) would pick a branch
                if matches!(&u, Expr::Pow(b, p) if **b == self.x && p.as_int().is_some()) {
                    continue;
                }
                let Some(back) = self.invert(&u, &t) else {
                    continue;
                };
                g2 = simplify(&subst(&g2, &self.x, &back));
                if self.has(&g2) {
                    continue;
                }
            }
            if g2.size() > f.size() * 3 + 30 {
                continue;
            }
            trace::step(|| {
                format!(
                    "integral of {} d{}: with {} = {} (d{} = {} d{}) it is the integral of {} d{}",
                    shown(f),
                    shown(&self.x),
                    shown(&t),
                    shown(&u),
                    shown(&t),
                    shown(&du),
                    shown(&self.x),
                    shown(&g2),
                    shown(&t)
                )
            });
            let nest = trace::nest();
            let found = self.inner(&name).go(&g2);
            drop(nest);
            if let Some(r) = found {
                let back = subst(&r, &t, &u);
                trace::step(|| {
                    format!("with {} = {} again: {}", shown(&t), shown(&u), shown(&back))
                });
                return Some(back);
            }
        }
        trace::forget(begun);
        None
    }

    fn by_parts(&mut self, f: &Expr) -> Option<Expr> {
        let factors: Vec<Expr> = match f {
            Expr::Mul(a) => a.to_vec(),
            other => vec![other.clone()],
        };
        let log_like = |e: &Expr| {
            let (b, p) = e.base_exp();
            matches!(&b, Expr::Fn(n, _) if matches!(&**n, "ln" | "asin" | "acos" | "atan" | "asinh" | "acosh" | "atanh" | "erf" | "ei" | "si" | "ci" | "li"))
                && p.as_i64().is_some_and(|k| (1..=8).contains(&k))
        };
        let pos = factors.iter().position(|e| self.has(e) && log_like(e))?;
        let u = factors[pos].clone();
        let dv = mul(factors
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != pos)
            .map(|(_, e)| e.clone())
            .collect());
        let var = self.var.clone();
        trace::step(|| {
            format!(
                "integral of {} d{}: by parts with u = {}, dv = {} d{}: it is u v less the integral of v du",
                shown(f),
                shown(&self.x),
                shown(&u),
                shown(&dv),
                shown(&self.x)
            )
        });
        let _nest = trace::nest();
        let v = self.inner(&var).go(&dv)?;
        let rest = simplify(&mul(vec![v.clone(), diff(&u, &var)]));
        // the same integral again: I = u v - c I
        let ratio = simplify(&div(rest.clone(), f.clone()));
        if !self.has(&ratio) {
            let denominator = simplify(&add(vec![Expr::one(), ratio.clone()]));
            trace::step(|| {
                format!(
                    "v du is {} times what is integrated: the integral I is u v - ({}) I, which is solved for I",
                    shown(&ratio),
                    shown(&ratio)
                )
            });
            return (!denominator.is_zero()).then(|| div(mul(vec![u, v]), denominator));
        }
        if rest.size() > f.size() * 4 + 40 {
            return None;
        }
        let tail = self.inner(&var).go(&rest)?;
        Some(sub(mul(vec![u, v]), tail))
    }

    /// Rational functions of `sin x`, `cos x`, `tan x` through `t = tan(x/2)`.
    fn half_angle(&mut self, f: &Expr) -> Option<Expr> {
        let x = self.x.clone();
        let only_trig = !subst_all(f, &x, &["sin", "cos", "tan"]).has_sym(&self.var);
        if !only_trig || !(f.has_fn("sin") || f.has_fn("cos") || f.has_fn("tan")) {
            return None;
        }
        let name = format!("#w{}", self.depth);
        let t = Expr::sym(&name);
        let t2 = powi(t.clone(), 2);
        let one_plus = add(vec![Expr::one(), t2.clone()]);
        let g = map_bottom_up(f, &mut |e| match e
            .fn_arg("sin")
            .or(e.fn_arg("cos"))
            .or(e.fn_arg("tan"))
        {
            Some(a) if *a == x => match &e {
                Expr::Fn(n, _) if &**n == "sin" => {
                    div(mul(vec![Expr::int(2), t.clone()]), one_plus.clone())
                }
                Expr::Fn(n, _) if &**n == "cos" => {
                    div(sub(Expr::one(), t2.clone()), one_plus.clone())
                }
                _ => div(
                    mul(vec![Expr::int(2), t.clone()]),
                    sub(Expr::one(), t2.clone()),
                ),
            },
            _ => e,
        });
        let g = ratsimp::together(&mul(vec![g, Expr::int(2), powi(one_plus.clone(), -1)]));
        trace::step(|| {
            format!(
                "integral of {} d{}: with {t} = tan({}/2), where sin = 2{t}/(1 + {t}^2), cos = (1 - {t}^2)/(1 + {t}^2), d{} = 2 d{t}/(1 + {t}^2), it is the integral of {} d{t}",
                shown(f),
                shown(&x),
                shown(&x),
                shown(&x),
                shown(&g),
                t = shown(&t)
            )
        });
        let _nest = trace::nest();
        let r = self.inner(&name).go(&g)?;
        Some(subst(&r, &t, &f1("tan", &div(x, Expr::int(2)))))
    }
}

/// An expression as a step shows it: the variables of substitutions by plain letters.
fn shown(e: &Expr) -> String {
    let mut text = e.to_string();
    for (inner, letter) in [("#u", "t"), ("#w", "s")] {
        while let Some(at) = text.find(inner) {
            let digits = text[at + 2..]
                .chars()
                .take_while(char::is_ascii_digit)
                .count();
            let level = &text[at + 2..at + 2 + digits];
            let name = match level {
                "" | "0" | "1" => letter.to_string(),
                more => format!("{letter}{more}"),
            };
            text.replace_range(at..at + 2 + digits, &name);
        }
    }
    text
}

/// The expression with the named functions of exactly `x` replaced by constants.
fn subst_all(f: &Expr, x: &Expr, names: &[&str]) -> Expr {
    map_bottom_up(f, &mut |e| match &e {
        Expr::Fn(n, a) if names.contains(&&**n) && a.len() == 1 && a[0] == *x => Expr::one(),
        _ => e,
    })
}

/// The polynomial part and the terms `A/g^j` of a rational function.
type Fractions = (Vec<Rat>, Vec<(Vec<Rat>, Vec<Rat>, u32)>);

/// Partial fractions of `p/q` over the rationals.
fn partial_fractions(p: &[Rat], q: &[Rat]) -> Option<Fractions> {
    let (quotient, rest) = dense_div_rem(p, q);
    if rest.is_empty() {
        return Some((quotient, Vec::new()));
    }
    let (content, factors, complete) = zfactor::factor(&Poly::from_dense(1, 0, q));
    if !complete {
        return None;
    }
    let parts: Vec<(Vec<Rat>, u32)> = factors
        .iter()
        .map(|(g, m)| (g.to_dense(0).unwrap(), *m))
        .collect();
    let n = q.len() - 1;
    // columns: x^k * Q/(c q_i^j)
    let mut columns: Vec<Vec<Rat>> = Vec::new();
    let mut owner: Vec<(usize, u32, usize)> = Vec::new();
    for (i, (g, m)) in parts.iter().enumerate() {
        for j in 1..=*m {
            let mut cofactor = vec![Rat::from(1)];
            for (i2, (g2, m2)) in parts.iter().enumerate() {
                let times = if i2 == i { m2 - j } else { *m2 };
                for _ in 0..times {
                    cofactor = dense_mul(&cofactor, g2);
                }
            }
            for k in 0..g.len() - 1 {
                let mut col = vec![Rat::new(); k];
                col.extend(cofactor.iter().cloned());
                col.resize(n, Rat::new());
                columns.push(col);
                owner.push((i, j, k));
            }
        }
    }
    let mut rhs: Vec<Rat> = rest.iter().map(|c| (c / &content).complete()).collect();
    rhs.resize(n, Rat::new());
    let solution = solve_rational_system(&columns, &rhs)?;
    let mut out = Vec::new();
    for (i, (g, m)) in parts.iter().enumerate() {
        for j in 1..=*m {
            let a: Vec<Rat> = (0..g.len() - 1)
                .map(|k| solution[owner.iter().position(|o| *o == (i, j, k)).unwrap()].clone())
                .collect();
            if !a.iter().all(|c| c.is_zero()) {
                out.push((a, g.clone(), j));
            }
        }
    }
    Some((quotient, out))
}

/// Partial fractions in `var`, for rational coefficients.
pub fn apart(f: &Expr, var: &str) -> Option<Expr> {
    let (num, den) = ratsimp::fraction(f);
    let dense = |e: &Expr| -> Option<Vec<Rat>> {
        ratsimp::coefficients(e, var)?
            .iter()
            .map(|c| c.as_rat().cloned())
            .collect()
    };
    let (p, q) = (dense(&num)?, dense(&den)?);
    let x = Expr::sym(var);
    let expr_of = |c: &[Rat]| {
        add(c
            .iter()
            .enumerate()
            .map(|(i, k)| mul(vec![rat_expr(k), powi(x.clone(), i as i64)]))
            .collect())
    };
    let (quotient, parts) = partial_fractions(&p, &q)?;
    let mut terms = vec![expr_of(&quotient)];
    for (a, g, j) in &parts {
        terms.push(div(expr_of(a), powi(expr_of(g), i64::from(*j))));
    }
    Some(add(terms))
}

/// `e^(a + i b)` as `e^a (cos b + i sin b)`.
pub fn exp_to_trig(e: &Expr) -> Expr {
    map_bottom_up(e, &mut |x| {
        let Expr::Pow(b, p) = &x else { return x };
        if !b.is_sym("e") {
            return x;
        }
        let terms: Vec<Expr> = match &**p {
            Expr::Add(a) => a.to_vec(),
            other => vec![other.clone()],
        };
        let (mut re, mut im) = (Vec::new(), Vec::new());
        for t in terms {
            let (c, rest) = crate::canon::coeff_and_rest(&t);
            if !c.re.is_zero() {
                re.push(mul(vec![Expr::rational(c.re), rest.clone()]));
            }
            if !c.im.is_zero() {
                im.push(mul(vec![Expr::rational(c.im), rest]));
            }
        }
        if im.is_empty() {
            return x;
        }
        let angle = add(im);
        mul(vec![
            pow(Expr::e(), add(re)),
            add(vec![
                f1("cos", &angle),
                mul(vec![Expr::i(), f1("sin", &angle)]),
            ]),
        ])
    })
}

fn dense_mul(a: &[Rat], b: &[Rat]) -> Vec<Rat> {
    let mut out = vec![Rat::new(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j] += (x * y).complete();
        }
    }
    out
}

fn dense_div_rem(a: &[Rat], b: &[Rat]) -> (Vec<Rat>, Vec<Rat>) {
    let mut r = a.to_vec();
    let mut q = vec![Rat::new(); a.len().saturating_sub(b.len()) + 1];
    while r.len() >= b.len() {
        let shift = r.len() - b.len();
        let c = (r.last().unwrap() / b.last().unwrap()).complete();
        for (i, y) in b.iter().enumerate() {
            r[shift + i] -= (&c * y).complete();
        }
        q[shift] = c;
        r.pop();
    }
    while r.last().is_some_and(|c| c.is_zero()) {
        r.pop();
    }
    if a.len() < b.len() {
        q.clear();
    }
    (q, r)
}

/// Solves `sum x_k column_k = rhs` over the rationals (square system).
fn solve_rational_system(columns: &[Vec<Rat>], rhs: &[Rat]) -> Option<Vec<Rat>> {
    let n = rhs.len();
    if columns.len() != n {
        return None;
    }
    let mut m: Vec<Vec<Rat>> = (0..n)
        .map(|r| {
            columns
                .iter()
                .map(|c| c[r].clone())
                .chain(std::iter::once(rhs[r].clone()))
                .collect()
        })
        .collect();
    for col in 0..n {
        let p = (col..n).find(|r| !m[*r][col].is_zero())?;
        m.swap(p, col);
        let pivot = m[col][col].clone();
        for x in m[col].iter_mut() {
            *x /= &pivot;
        }
        for r in 0..n {
            if r != col && !m[r][col].is_zero() {
                let factor = m[r][col].clone();
                let pivot_row = m[col].clone();
                for (x, p) in m[r].iter_mut().zip(&pivot_row).skip(col) {
                    *x -= (&factor * p).complete();
                }
            }
        }
    }
    Some(m.into_iter().map(|row| row[n].clone()).collect())
}

/// Checks `F' = f` at real sample points (and as an identity).
fn verify(antiderivative: &Expr, f: &Expr, var: &str) -> bool {
    let d = sub(diff(antiderivative, var), f.clone());
    if simplify::is_zero(&d) {
        return true;
    }
    let others: Vec<Rc<str>> = d.free_syms().into_iter().filter(|s| &**s != var).collect();
    let mut passed = 0;
    for trial in 0..10u32 {
        let mut env = Env::new();
        for (k, s) in others.iter().enumerate() {
            env.insert(
                s.clone(),
                Complex::with_val(512, 0.61 + 0.37 * k as f64 + 0.11 * f64::from(trial)),
            );
        }
        env.insert(
            Rc::from(var),
            Complex::with_val(512, 0.37 + 0.283 * f64::from(trial)),
        );
        match numeric::is_zero_at(&d, &env) {
            Some(true) => passed += 1,
            Some(false) => return false,
            None => {}
        }
    }
    passed >= 3
}

/// Antiderivative of `f` with respect to `var`, or `None` when no rule gives a checked result.
/// `f` with its outermost piecewise part brought to the top: `c1, f1, c2, f2, ..., otherwise`.
fn lift_piecewise(f: &Expr) -> Option<Vec<Expr>> {
    let mut found: Option<Expr> = None;
    f.walk(&mut |e| {
        if found.is_none() && e.fn_args("piecewise").is_some() {
            found = Some(e.clone());
        }
    });
    let p = found?;
    let args = p.fn_args("piecewise")?;
    Some(
        args.iter()
            .enumerate()
            .map(|(k, x)| {
                if k % 2 == 1 || k + 1 == args.len() {
                    subst(f, &p, x)
                } else {
                    x.clone()
                }
            })
            .collect(),
    )
}

/// Pieces `x < b1, F1, x < b2, F2, ..., F` with rising numbers `b`: constants are added so that the
/// pieces meet at the breakpoints.
fn join_pieces(pieces: &mut [Expr], var: &str) {
    let mut breaks: Vec<Expr> = Vec::new();
    for pair in pieces.as_chunks::<2>().0 {
        match &pair[0] {
            Expr::Rel(Rel::Lt | Rel::Le, l, r) if l.is_sym(var) && r.is_const() => {
                breaks.push((**r).clone())
            }
            _ => return,
        }
    }
    let rising = breaks.windows(2).all(|w| {
        numeric::sign(&crate::canon::sub(w[1].clone(), w[0].clone())).is_some_and(|s| s > 0)
    });
    if !rising {
        return;
    }
    let x = Expr::sym(var);
    for (k, b) in breaks.iter().enumerate() {
        let next = 2 * k + 3;
        let next = if next < pieces.len() {
            next
        } else if pieces.len() % 2 == 1 {
            pieces.len() - 1
        } else {
            return;
        };
        let gap = simplify(&crate::canon::sub(
            subst(&pieces[2 * k + 1], &x, b),
            subst(&pieces[next], &x, b),
        ));
        if gap.is_infinite() || gap.is_undefined() || !gap.is_const() {
            return;
        }
        pieces[next] = simplify(&add(vec![pieces[next].clone(), gap]));
    }
}

pub fn integrate(f: &Expr, var: &str) -> Option<Expr> {
    if let Some(lifted) = lift_piecewise(f) {
        let mut out = Vec::with_capacity(lifted.len());
        for (k, x) in lifted.iter().enumerate() {
            out.push(if k % 2 == 1 || k + 1 == lifted.len() {
                integrate(x, var)?
            } else {
                x.clone()
            });
        }
        join_pieces(&mut out, var);
        return Some(Expr::call("piecewise", out));
    }
    let mut cx = Cx {
        var: var.to_string(),
        x: Expr::sym(var),
        depth: 0,
    };
    let raw = trace::whole(|| cx.go(f));
    if raw.is_none() {
        trace::step(|| format!("no rule gives the integral of {f} d{var}"));
    }
    let raw = raw?;
    if raw.any(&|e| matches!(e, Expr::Sym(s) if s.starts_with('#'))) {
        return None;
    }
    // as an antiderivative ln(e^u) is u: they differ by a constant between the branch jumps
    let raw = map_bottom_up(&raw, &mut |e| match e.fn_arg("ln") {
        Some(Expr::Pow(b, u)) if b.is_sym("e") => (**u).clone(),
        _ => e,
    });
    let result = simplify(&raw);
    if verify(&result, f, var) {
        trace::step(|| format!("its derivative is {f} again (checked): the integral is {result}"));
        return Some(result);
    }
    let holds = verify(&raw, f, var);
    trace::step(|| {
        if holds {
            format!("its derivative is {f} again (checked): the integral is {raw}")
        } else {
            format!("the derivative of {raw} is not {f} again: the integral is not given")
        }
    });
    holds.then_some(raw)
}

fn real_point(e: &Expr, prec: u32) -> Option<Float> {
    let v = numeric::evalf(e, prec).ok()?;
    v.imag().is_zero().then(|| v.real().clone())
}

/// Numeric integral over a real interval, possibly infinite.
pub fn numeric_integral(f: &Expr, var: &str, a: &Expr, b: &Expr, digits: u32) -> Option<Complex> {
    if !f.free_syms().iter().all(|s| &**s == var) {
        return None;
    }
    let prec = numeric::bits_for(digits) + 64;
    let key: Rc<str> = Rc::from(var);
    let mut value = |x: &Float| -> Option<Complex> {
        let mut env = Env::new();
        env.insert(key.clone(), Complex::with_val(x.prec(), x));
        numeric::evalf_env(f, x.prec(), &env).ok()
    };
    let (lo_inf, hi_inf) = (a.is_neg_inf(), b.is_inf());
    if a.is_inf() || b.is_neg_inf() {
        return numeric_integral(f, var, b, a, digits).map(|v| -v);
    }
    let zero = Float::new(prec);
    let one = Float::with_val(prec, 1);
    match (lo_inf, hi_inf) {
        (false, false) => {
            let (lo, hi) = (real_point(a, prec)?, real_point(b, prec)?);
            if lo > hi {
                return quad::quad(&mut value, &hi, &lo, digits).map(|v| -v);
            }
            quad::quad(&mut value, &lo, &hi, digits)
        }
        (false, true) | (true, false) => {
            let end = real_point(if hi_inf { a } else { b }, prec)?;
            let mut g = |t: &Float| -> Option<Complex> {
                let s = Float::with_val(t.prec(), 1u32 - t);
                if s.is_zero() {
                    return None;
                }
                let shift = Float::with_val(t.prec(), t / &s);
                let x = if hi_inf {
                    Float::with_val(t.prec(), &end + &shift)
                } else {
                    Float::with_val(t.prec(), &end - &shift)
                };
                Some(value(&x)? / s.square())
            };
            quad::quad(&mut g, &zero, &one, digits)
        }
        (true, true) => {
            let mut g = |t: &Float| -> Option<Complex> {
                let s = Float::with_val(t.prec(), 1u32 - t.clone().square());
                if s.is_zero() {
                    return None;
                }
                let x = Float::with_val(t.prec(), t / &s);
                let weight = Float::with_val(t.prec(), t.clone().square() + 1u32) / s.square();
                Some(value(&x)? * weight)
            };
            quad::quad(&mut g, &Float::with_val(prec, -1), &one, digits)
        }
    }
}

/// Real zeros of the denominator strictly inside the interval; `None` when they cannot be listed.
fn singular_points(f: &Expr, var: &str, a: &Expr, b: &Expr) -> Option<Vec<Expr>> {
    let (_, den) = ratsimp::fraction(f);
    // exponentials never vanish
    let den = match &den {
        Expr::Mul(factors) => mul(factors
            .iter()
            .filter(|x| !matches!(x, Expr::Pow(b, _) if b.is_sym("e") || b.as_num().is_some_and(|n| n.is_positive())))
            .cloned()
            .collect()),
        Expr::Pow(b, _) if b.is_sym("e") || b.as_num().is_some_and(|n| n.is_positive()) => Expr::one(),
        _ => den,
    };
    if !den.has_sym(var) {
        return Some(Vec::new());
    }
    // (where the denominator vanishes: the steps of that equation are not told)
    let solved = {
        let _quiet = trace::mute();
        solve::solve(&den, var).ok()?
    };
    if !solved.params.is_empty() || solved.all {
        return None;
    }
    let inside = |r: &Expr| -> Option<bool> {
        let below = if a.is_neg_inf() {
            1
        } else {
            numeric::sign(&sub(r.clone(), a.clone()))?
        };
        let above = if b.is_inf() {
            1
        } else {
            numeric::sign(&sub(b.clone(), r.clone()))?
        };
        Some(below > 0 && above > 0)
    };
    let mut points: Vec<(Float, Expr)> = Vec::new();
    for r in &solved.roots {
        let v = numeric::evalf(&r.value, 128).ok()?;
        if !v.imag().is_zero() {
            continue;
        }
        if inside(&r.value)? {
            points.push((v.real().clone(), r.value.clone()));
        }
    }
    points.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal));
    Some(points.into_iter().map(|(_, e)| e).collect())
}

/// Samples of the integrand on a finite interval: no failure and no isolated spike (a pole between
/// two samples shows as one); also whether all samples are real and of one sign.
struct Scan {
    clean: bool,
    real: bool,
    sign: i32,
}

fn scan(f: &Expr, var: &str, a: &Expr, b: &Expr) -> Scan {
    let (Some(lo), Some(hi)) = (real_point(a, 128), real_point(b, 128)) else {
        return Scan {
            clean: true,
            real: false,
            sign: 0,
        };
    };
    let key: Rc<str> = Rc::from(var);
    let mut values = Vec::new();
    for i in 1..128u32 {
        let x = Float::with_val(128, &lo + Float::with_val(128, &hi - &lo) * i / 128u32);
        let mut env = Env::new();
        env.insert(key.clone(), Complex::with_val(128, x));
        match numeric::evalf_env(f, 128, &env) {
            Ok(v) => values.push(v),
            Err(_) => {
                return Scan {
                    clean: false,
                    real: false,
                    sign: 0,
                };
            }
        }
    }
    let size: Vec<f64> = values
        .iter()
        .map(|v| Float::with_val(64, v.abs_ref()).to_f64())
        .collect();
    let spike =
        (1..size.len() - 1).any(|i| size[i] > 1e6 * (size[i - 1].max(size[i + 1]) + 1e-300));
    let real = values.iter().all(|v| v.imag().is_zero());
    let positive = values
        .iter()
        .all(|v| !v.real().is_sign_negative() || v.real().is_zero());
    let negative = values
        .iter()
        .all(|v| v.real().is_sign_negative() || v.real().is_zero());
    Scan {
        clean: !spike,
        real,
        sign: if real && positive {
            1
        } else if real && negative {
            -1
        } else {
            0
        },
    }
}

/// Definite integral: `(value, approximate)`.
/// A definite integral with symbols besides the variable: the antiderivative between the ends,
/// taken by limits.
pub fn definite_with_parameters(f: &Expr, var: &str, a: &Expr, b: &Expr) -> Option<Expr> {
    let anti = integrate(f, var)?;
    let at = |p: &Expr, side: Dir| {
        let dir = if p.is_infinite() { Dir::Both } else { side };
        limit::limit(&anti, var, p, dir)
            .ok()
            .filter(|r| !r.1)
            .map(|r| r.0)
    };
    let value = simplify(&sub(at(b, Dir::Left)?, at(a, Dir::Right)?));
    (!value.is_undefined() && !value.has_sym(var)).then_some(value)
}

/// The integral of terms `h(x) diracdelta(g(x))`: `h` over `|g'|` at the simple zeros of `g`
/// inside the interval; a zero at an end counts half.
fn delta_integral(f: &Expr, var: &str, a: &Expr, b: &Expr) -> Option<Result<(Expr, bool), String>> {
    let ex = expand(f);
    let terms: Vec<Expr> = match &ex {
        Expr::Add(t) => t.to_vec(),
        other => vec![other.clone()],
    };
    let x = Expr::sym(var);
    let bound = |e: &Expr| -> Option<Option<Float>> {
        if e.is_infinite() {
            Some(None)
        } else {
            numeric::real_value(e, 30).map(Some)
        }
    };
    let (lo, hi) = (bound(a)?, bound(b)?);
    let mut total = Vec::new();
    let mut approx = false;
    for t in terms {
        if !t.has_fn("diracdelta") {
            match definite(&t, var, a, b) {
                Ok((v, ap)) => {
                    total.push(v);
                    approx |= ap;
                }
                Err(e) => return Some(Err(e)),
            }
            continue;
        }
        let factors: Vec<Expr> = match &t {
            Expr::Mul(fs) => fs.to_vec(),
            other => vec![other.clone()],
        };
        let at = factors.iter().position(
            |q| matches!(q, Expr::Fn(n, args) if &**n == "diracdelta" && args.len() == 1),
        )?;
        let g = factors[at].children()[0].clone();
        let rest = mul(factors
            .iter()
            .enumerate()
            .filter(|(k, _)| *k != at)
            .map(|(_, q)| q.clone())
            .collect());
        if rest.has_fn("diracdelta") {
            return None;
        }
        let solved = solve::solve(&g, var).ok()?;
        solve::release(&solved.params);
        if solved.all || !solved.params.is_empty() {
            return None;
        }
        let slope = diff(&g, var);
        for r in &solved.roots {
            let Some(v) = numeric::real_value(&r.value, 30) else {
                continue;
            };
            let at_end = |end: &Expr| simplify(&sub(r.value.clone(), end.clone())).is_zero();
            let weight = if at_end(a) || at_end(b) {
                Expr::half()
            } else if lo.as_ref().is_none_or(|l| v > *l) && hi.as_ref().is_none_or(|h| v < *h) {
                Expr::one()
            } else {
                continue;
            };
            if r.mult != 1 {
                return None;
            }
            let steep = func("abs", vec![subst(&slope, &x, &r.value)]);
            total.push(div(mul(vec![weight, subst(&rest, &x, &r.value)]), steep));
        }
    }
    Some(Ok((simplify(&add(total)), approx)))
}

pub fn definite(f: &Expr, var: &str, a: &Expr, b: &Expr) -> Result<(Expr, bool), String> {
    if a == b {
        return Ok((Expr::zero(), false));
    }
    let digits = numeric::digits();
    // from the larger end to the smaller one: minus the integral the other way
    if a.is_inf()
        || b.is_neg_inf()
        || (!a.is_infinite()
            && !b.is_infinite()
            && numeric::sign(&sub(a.clone(), b.clone())) == Some(1))
    {
        let (value, approx) = definite(f, var, b, a)?;
        return Ok((simplify(&neg(value)), approx));
    }
    if f.has_fn("diracdelta")
        && let Some(r) = delta_integral(f, var, a, b)
    {
        return r;
    }
    // corners of abs, sign, min, max and steps of floor and its kin: piece by piece
    if let Some(points) = crate::reals::corners(f, var, a, b) {
        let mut cuts = vec![a.clone()];
        cuts.extend(points);
        cuts.push(b.clone());
        let pieces: Option<Vec<Expr>> = cuts
            .windows(2)
            .map(|w| crate::reals::smooth_between(f, var, &w[0], &w[1]))
            .collect();
        if let Some(pieces) = pieces.filter(|p| p.len() > 1 || p[0] != *f) {
            let (mut total, mut approx) = (Vec::new(), false);
            for (piece, w) in pieces.iter().zip(cuts.windows(2)) {
                let (v, a) = definite(piece, var, &w[0], &w[1])?;
                approx |= a;
                total.push(v);
            }
            let value = simplify(&add(total));
            return Ok((
                if approx {
                    numeric::n(&value, digits)
                } else {
                    value
                },
                approx,
            ));
        }
    }
    let diverges = || format!("the integral of {f} from {a} to {b} diverges");
    let no_value =
        || format!("no value found for the integral of {f} from {a} to {b} (it may diverge)");
    let singular = singular_points(f, var, a, b);
    let mut cuts = vec![a.clone()];
    cuts.extend(singular.clone().unwrap_or_default());
    cuts.push(b.clone());
    if let (Some(anti), Some(points)) = (integrate(f, var), &singular) {
        let mut pieces = Vec::new();
        trace::step(|| {
            format!(
                "from {a} to {b}: the value of {anti} at the upper end less its value at the lower end{}",
                if points.is_empty() {
                    String::new()
                } else {
                    format!(
                        ", piece by piece between the points where the integrand has no value ({})",
                        points
                            .iter()
                            .map(|p| p.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            )
        });
        for w in cuts.windows(2) {
            // (the values at the ends are limits: their own steps are not told)
            let _quiet = trace::mute();
            match (
                limit::limit(&anti, var, &w[1], Dir::Left),
                limit::limit(&anti, var, &w[0], Dir::Right),
            ) {
                (Ok((hi, false)), Ok((lo, false))) => pieces.push(sub(hi, lo)),
                _ => {
                    pieces.clear();
                    break;
                }
            }
        }
        if !pieces.is_empty() {
            let value = simplify(&add(pieces));
            if value.is_undefined() {
                return Err(diverges());
            }
            if value.is_infinite() || !value.is_const() || !points.is_empty() {
                return Ok((value, false));
            }
            let seen = scan(f, var, a, b);
            let singular_message = format!("the integrand {f} is singular between {a} and {b}");
            match (
                numeric_integral(f, var, a, b, 12),
                numeric::evalf(&value, 128),
            ) {
                (Some(q), Ok(v)) if seen.clean => {
                    let d = Complex::with_val(128, &q - &v);
                    let scale = Float::with_val(64, v.abs_ref()) + 1u32;
                    if Float::with_val(64, d.abs_ref()) <= scale * 1e-8 {
                        return Ok((value, false));
                    }
                }
                (None, Ok(v)) if seen.clean => {
                    let consistent = (!seen.real || v.imag().is_zero())
                        && (seen.sign == 0 || v.real().cmp0().map(|o| o as i32) == Some(seen.sign));
                    if consistent {
                        return Ok((value, false));
                    }
                    return Err(singular_message);
                }
                _ => return Err(singular_message),
            }
        }
    }
    if !scan(f, var, a, b).clean {
        return Err(format!("the integrand {f} is singular between {a} and {b}"));
    }
    trace::step(|| {
        format!(
            "no antiderivative in closed form: the integral from {a} to {b} is found as a number, by the tanh-sinh rule (the step is halved until the sum settles to {digits} digits)"
        )
    });
    let mut total = Complex::new(numeric::bits_for(digits) + 32);
    for w in cuts.windows(2) {
        total += numeric_integral(f, var, &w[0], &w[1], digits).ok_or_else(no_value)?;
    }
    Ok((numeric::to_float_expr(total, digits), true))
}
