//! Ordinary differential equations in one unknown function.

use crate::canon::{add, div, inv, ln, map_bottom_up, mul, neg, pow, powi, sqrt, sub, subst_sym};
use crate::diff::{diff, diff_n};
use crate::expand::expand;
use crate::expr::Expr;
use crate::funcs::func;
use crate::integrate::{exp_to_trig, integrate};
use crate::num::Int;
use crate::numeric;
use crate::ratsimp;
use crate::simplify::{is_zero, simplify};
use crate::solve;

/// Name of the symbol standing for the `n`-th derivative of the unknown function.
pub fn marker(n: u32) -> String {
    format!("#d{n}")
}

pub enum Solution {
    Explicit(Expr),
    /// A relation between the function and the variable: `left = right`.
    Implicit(Expr, Expr),
}

const ROOT: &str = "#r";
const LOG_VAR: &str = "#u";
const RATIO: &str = "#v";

fn terms_of(e: &Expr) -> Vec<Expr> {
    match e {
        Expr::Add(a) => a.to_vec(),
        other => vec![other.clone()],
    }
}

fn factors_of(e: &Expr) -> Vec<Expr> {
    match e {
        Expr::Mul(a) => a.to_vec(),
        other => vec![other.clone()],
    }
}

/// Real and imaginary parts, taking the symbols for real.
fn parts(e: &Expr) -> (Expr, Expr) {
    if let Expr::Float(f) = e {
        let part =
            |x: &rug::Float| numeric::to_float_expr(rug::Complex::with_val(x.prec(), x), f.digits);
        return (part(f.v.real()), part(f.v.imag()));
    }
    if e.is_const()
        && let Some((re, im)) = crate::complex::rect(e)
    {
        return (simplify(&re), simplify(&im));
    }
    let (mut re, mut im) = (Vec::new(), Vec::new());
    for t in terms_of(&expand(e)) {
        let (c, rest) = crate::canon::coeff_and_rest(&t);
        re.push(mul(vec![Expr::rational(c.re), rest.clone()]));
        im.push(mul(vec![Expr::rational(c.im), rest]));
    }
    (add(re), add(im))
}

fn negative(e: &Expr) -> bool {
    match numeric::sign(e) {
        Some(s) => s < 0,
        None => crate::funcs::is_neg_form(e),
    }
}

fn factorial_ratio(top: usize, low: usize) -> Expr {
    Expr::integer(Int::from(Int::factorial(top as u32)) / Int::from(Int::factorial(low as u32)))
}

/// A particular solution of `sum a_k y^(k) = g` for `g` a sum of polynomials times exponentials.
fn undetermined(a: &[Expr], g: &Expr, var: &str) -> Option<Expr> {
    let x = Expr::sym(var);
    let n = a.len() - 1;
    let r = Expr::sym(ROOT);
    let mut dp = vec![add(a
        .iter()
        .enumerate()
        .map(|(k, c)| mul(vec![c.clone(), powi(r.clone(), k as i64)]))
        .collect())];
    for k in 1..=n {
        dp.push(diff(&dp[k - 1], ROOT));
    }
    let mut groups: Vec<(Expr, Vec<Expr>)> = Vec::new();
    for t in terms_of(&expand(&solve::rewrite_exp(g))) {
        let (mut lambda, mut deg, mut c) = (Expr::zero(), 0usize, Vec::new());
        for f in factors_of(&t) {
            let (b, k) = f.base_exp();
            if !f.has_sym(var) {
                c.push(f);
            } else if b == x {
                deg += usize::try_from(k.as_i64()?).ok()?;
            } else if b.is_sym("e") {
                let co = ratsimp::coefficients(&k, var).filter(|co| co.len() <= 2)?;
                lambda = add(vec![lambda, co.get(1).cloned().unwrap_or_else(Expr::zero)]);
                c.push(pow(Expr::e(), co[0].clone()));
            } else {
                return None;
            }
        }
        let slot = match groups
            .iter()
            .position(|(l, _)| is_zero(&sub(l.clone(), lambda.clone())))
        {
            Some(i) => i,
            None => {
                groups.push((lambda, Vec::new()));
                groups.len() - 1
            }
        };
        let poly = &mut groups[slot].1;
        if poly.len() <= deg {
            poly.resize(deg + 1, Expr::zero());
        }
        poly[deg] = add(vec![poly[deg].clone(), mul(c)]);
    }
    let mut total = Vec::new();
    for (lambda, pc) in groups {
        // sum_k p^(k)(lambda)/k! R^(k) = P for R = x^s Q
        let c: Vec<Expr> = (0..=n)
            .map(|k| {
                simplify(&div(
                    subst_sym(&dp[k], ROOT, &lambda),
                    factorial_ratio(k, 0),
                ))
            })
            .collect();
        let s = c.iter().position(|v| !is_zero(v))?;
        let d = pc.len() - 1;
        let mut q = vec![Expr::zero(); d + s + 1];
        for i in (0..=d).rev() {
            let mut rhs = pc[i].clone();
            for k in (s + 1)..=n {
                if i + k <= d + s {
                    rhs = sub(
                        rhs,
                        mul(vec![
                            c[k].clone(),
                            q[i + k].clone(),
                            factorial_ratio(i + k, i),
                        ]),
                    );
                }
            }
            q[i + s] = simplify(&div(
                rhs,
                mul(vec![c[s].clone(), factorial_ratio(i + s, i)]),
            ));
        }
        let poly = add(q
            .iter()
            .enumerate()
            .map(|(j, c)| mul(vec![c.clone(), powi(x.clone(), j as i64)]))
            .collect());
        total.push(mul(vec![
            poly,
            pow(Expr::e(), mul(vec![lambda, x.clone()])),
        ]));
    }
    Some(simplify(&exp_to_trig(&expand(&add(total)))))
}

/// A particular solution by solving `(D - r) v = u` for each root in turn.
fn operator_integrals(a: &[Expr], g: &Expr, roots: &[solve::Root], var: &str) -> Option<Expr> {
    let x = Expr::sym(var);
    let mut u = div(g.clone(), a.last()?.clone());
    for root in roots {
        for _ in 0..root.mult {
            let turn = mul(vec![root.value.clone(), x.clone()]);
            let inner = integrate(
                &simplify(&mul(vec![pow(Expr::e(), neg(turn.clone())), u])),
                var,
            )?;
            u = simplify(&mul(vec![pow(Expr::e(), turn), inner]));
        }
    }
    Some(simplify(&exp_to_trig(&u)))
}

/// Variation of parameters for an equation of order two.
fn variation(basis: &[Expr], a: &[Expr], g: &Expr, var: &str) -> Option<Expr> {
    let [y1, y2] = basis else { return None };
    let w = simplify(&sub(
        mul(vec![y1.clone(), diff(y2, var)]),
        mul(vec![y2.clone(), diff(y1, var)]),
    ));
    let rhs = div(g.clone(), a[2].clone());
    let u1 = integrate(
        &simplify(&neg(div(mul(vec![y2.clone(), rhs.clone()]), w.clone()))),
        var,
    )?;
    let u2 = integrate(&simplify(&div(mul(vec![y1.clone(), rhs]), w)), var)?;
    Some(simplify(&add(vec![
        mul(vec![u1, y1.clone()]),
        mul(vec![u2, y2.clone()]),
    ])))
}

/// Renames constants: a lone multiple `q C` becomes `C`, `e^(C + u)` becomes `C e^u`.
fn absorb(e: &Expr, constants: &[String]) -> Expr {
    let mut out = e.clone();
    for c in constants {
        let cs = Expr::sym(c);
        let lone = |out: &Expr| {
            let mut count = 0;
            out.walk(&mut |x| count += usize::from(*x == cs));
            if count != 1 {
                return out.clone();
            }
            map_bottom_up(out, &mut |x| match &x {
                Expr::Mul(f)
                    if f.len() == 2 && f.contains(&cs) && f.iter().any(Expr::is_number) =>
                {
                    cs.clone()
                }
                _ => x,
            })
        };
        out = lone(&out);
        out = map_bottom_up(&out, &mut |x| {
            let Expr::Pow(b, p) = &x else { return x };
            if !b.is_sym("e") {
                return x;
            }
            match &**p {
                Expr::Add(terms) if terms.contains(&cs) => {
                    let rest: Vec<Expr> = terms.iter().filter(|t| **t != cs).cloned().collect();
                    mul(vec![cs.clone(), pow(Expr::e(), add(rest))])
                }
                other if *other == cs => cs.clone(),
                _ => x,
            }
        });
        out = lone(&out);
    }
    out
}

/// Strips invertible layers off `left = right`: terms and factors without `y`, inverse functions.
fn peel(mut left: Expr, mut right: Expr, y: &str) -> (Expr, Expr) {
    for _ in 0..8 {
        let split = |items: &[Expr]| -> (Vec<Expr>, Vec<Expr>) {
            items.iter().cloned().partition(|t| t.has_sym(y))
        };
        match &left {
            Expr::Add(terms) => {
                let (with, without) = split(terms);
                if with.is_empty() || without.is_empty() {
                    break;
                }
                right = sub(right, add(without));
                left = add(with);
            }
            Expr::Mul(factors) => {
                let (with, without) = split(factors);
                if with.is_empty() || without.is_empty() {
                    break;
                }
                right = div(right, mul(without));
                left = mul(with);
            }
            Expr::Fn(name, args) if args.len() == 1 => {
                let inverse = match &**name {
                    "atan" => "tan",
                    "asin" => "sin",
                    "acos" => "cos",
                    "atanh" => "tanh",
                    "asinh" => "sinh",
                    "acosh" => "cosh",
                    "ln" => "exp",
                    _ => break,
                };
                right = func(inverse, vec![right]);
                left = args[0].clone();
            }
            Expr::Pow(b, u) if b.is_sym("e") => {
                right = ln(right);
                left = (**u).clone();
            }
            Expr::Pow(u, k) if k.as_i64().is_some_and(|k| k % 2 != 0) => {
                right = pow(right, inv((**k).clone()));
                left = (**u).clone();
            }
            _ => break,
        }
    }
    (left, right)
}

struct Ode {
    y: String,
    x: String,
    constants: Vec<String>,
    prefix: &'static str,
}

impl Ode {
    fn constant(&mut self) -> Expr {
        let name = format!("{}{}", self.prefix, self.constants.len() + 1);
        self.constants.push(name.clone());
        Expr::sym(&name)
    }

    fn xs(&self) -> Expr {
        Expr::sym(&self.x)
    }

    fn ys(&self) -> Expr {
        Expr::sym(&self.y)
    }

    /// General solution of `sum a_k y^(k) = g` with constant `a_k`, in the variable `var`.
    fn constant_coefficients(&mut self, a: &[Expr], g: &Expr, var: &str) -> Result<Expr, String> {
        let t = Expr::sym(var);
        let r = Expr::sym(ROOT);
        let n = a.len() - 1;
        // parameters count as positive while the roots are found: the solution is the same formula for the others
        let mut params: Vec<String> = Vec::new();
        for c in a {
            for s in c.free_syms() {
                if crate::assume::get(&s) == crate::assume::Facts::default()
                    && !params.iter().any(|p| **p == *s)
                {
                    params.push(s.to_string());
                }
            }
        }
        for p in &params {
            crate::assume::set(p, crate::assume::Facts::positive());
        }
        let p = add(a
            .iter()
            .enumerate()
            .map(|(k, c)| mul(vec![c.clone(), powi(r.clone(), k as i64)]))
            .collect());
        let solved = solve::solve(&p, ROOT);
        let rooted = solved.map(|s| {
            solve::release(&s.params);
            let split: Vec<(Expr, Expr)> = s
                .roots
                .iter()
                .map(|root| parts(&map_bottom_up(&root.value, &mut |x| x)))
                .collect();
            (s, split)
        });
        for p in &params {
            crate::assume::clear(p);
        }
        let (solved, split) = rooted?;
        if solved.roots.iter().map(|r| r.mult as usize).sum::<usize>() != n {
            return Err("the characteristic equation was not solved completely".to_string());
        }
        let real = a.iter().all(|c| is_zero(&parts(c).1));
        let mut basis = Vec::new();
        for (root, (re, im)) in solved.roots.iter().zip(split) {
            let paired = real && !is_zero(&im);
            if paired && negative(&im) {
                continue;
            }
            for j in 0..root.mult {
                let power = powi(t.clone(), i64::from(j));
                if paired {
                    let growth = pow(Expr::e(), mul(vec![re.clone(), t.clone()]));
                    let angle = mul(vec![im.clone(), t.clone()]);
                    basis.push(mul(vec![
                        power.clone(),
                        growth.clone(),
                        func("cos", vec![angle.clone()]),
                    ]));
                    basis.push(mul(vec![power, growth, func("sin", vec![angle])]));
                } else {
                    basis.push(mul(vec![
                        power,
                        pow(
                            Expr::e(),
                            mul(vec![
                                add(vec![re.clone(), mul(vec![Expr::i(), im.clone()])]),
                                t.clone(),
                            ]),
                        ),
                    ]));
                }
            }
        }
        if basis.len() != n {
            return Err("the characteristic roots do not pair up".to_string());
        }
        let mut total: Vec<Expr> = basis
            .iter()
            .map(|b| mul(vec![self.constant(), b.clone()]))
            .collect();
        if !is_zero(g) {
            let particular = undetermined(a, g, var)
                .or_else(|| variation(&basis, a, g, var))
                .or_else(|| operator_integrals(a, g, &solved.roots, var))
                .ok_or_else(|| format!("no particular solution found for the right side {g}"))?;
            total.push(particular);
        }
        Ok(add(total))
    }

    /// `a1 y' + a0 y = g` by an integrating factor.
    fn first_order_linear(&mut self, a0: &Expr, a1: &Expr, g: &Expr) -> Result<Expr, String> {
        let p = simplify(&div(a0.clone(), a1.clone()));
        let q = simplify(&div(g.clone(), a1.clone()));
        let fail = |what: &Expr| format!("cannot integrate {what}");
        let factor = simplify(&pow(
            Expr::e(),
            integrate(&p, &self.x).ok_or_else(|| fail(&p))?,
        ));
        let c = self.constant();
        if q.is_zero() {
            return Ok(simplify(&div(c, factor)));
        }
        let inside = simplify(&mul(vec![factor.clone(), q]));
        let inner = integrate(&inside, &self.x).ok_or_else(|| fail(&inside))?;
        Ok(simplify(&div(add(vec![inner, c]), factor)))
    }

    /// `sum c_k x^k y^(k) = g`: constant coefficients in `ln x`.
    fn cauchy_euler(&mut self, a: &[Expr], g: &Expr) -> Option<Result<Expr, String>> {
        let x = self.xs();
        let cs: Vec<Expr> = a
            .iter()
            .enumerate()
            .map(|(k, ak)| simplify(&div(ak.clone(), powi(x.clone(), k as i64))))
            .collect();
        if !cs.iter().all(|c| c.free_of(&self.x)) {
            return None;
        }
        let d = Expr::sym(ROOT);
        let falling = |k: usize| {
            mul((0..k)
                .map(|j| sub(d.clone(), Expr::int(j as i64)))
                .collect())
        };
        let poly = expand(&add(cs
            .iter()
            .enumerate()
            .map(|(k, c)| mul(vec![c.clone(), falling(k)]))
            .collect()));
        let mut b = ratsimp::coefficients(&poly, ROOT)?;
        b.resize(a.len(), Expr::zero());
        let gt = simplify(&subst_sym(g, &self.x, &pow(Expr::e(), Expr::sym(LOG_VAR))));
        Some(
            self.constant_coefficients(&b, &gt, LOG_VAR)
                .map(|s| simplify(&subst_sym(&s, LOG_VAR, &ln(x)))),
        )
    }

    /// Explicit solutions of `left(y) = right` when it can be solved for `y`, else the relation.
    /// `y'' = (p x + q) y` (Airy functions) and `x^2 y'' + x y' + (c x^2 - n^2) y = 0`
    /// (Bessel functions).
    fn airy_bessel(&mut self, a: &[Expr], g: &Expr) -> Option<Expr> {
        if a.len() != 3 || !g.is_zero() {
            return None;
        }
        let x = self.xs();
        let first = simplify(&div(a[1].clone(), a[2].clone()));
        let last = simplify(&div(a[0].clone(), a[2].clone()));
        if first.is_zero() {
            // y'' + last y = 0 with last = -(p x + q)
            let cs = ratsimp::coefficients(&last, &self.x).filter(|c| c.len() == 2)?;
            let (p, q) = (neg(cs[1].clone()), neg(cs[0].clone()));
            let u = mul(vec![
                pow(p.clone(), Expr::rat(-2, 3)),
                add(vec![mul(vec![p, x]), q]),
            ]);
            let (c1, c2) = (self.constant(), self.constant());
            return Some(add(vec![
                mul(vec![c1, func("airyai", vec![u.clone()])]),
                mul(vec![c2, func("airybi", vec![u])]),
            ]));
        }
        // y'' + y'/x + (c - n^2/x^2) y = 0
        if !is_zero(&sub(first, powi(x.clone(), -1))) {
            return None;
        }
        let scaled = simplify(&mul(vec![last, powi(x.clone(), 2)]));
        let cs = ratsimp::coefficients(&scaled, &self.x).filter(|c| c.len() == 3)?;
        if !cs[1].is_zero() || cs[2].is_zero() {
            return None;
        }
        let order = simplify(&sqrt(neg(cs[0].clone())));
        let c = cs[2].clone();
        let modified = numeric::sign(&c) == Some(-1);
        let rate = simplify(&sqrt(if modified { neg(c) } else { c }));
        let arg = mul(vec![rate, x]);
        let names = if modified {
            ("besseli", "besselk")
        } else {
            ("besselj", "bessely")
        };
        let (c1, c2) = (self.constant(), self.constant());
        Some(add(vec![
            mul(vec![c1, func(names.0, vec![order.clone(), arg.clone()])]),
            mul(vec![c2, func(names.1, vec![order, arg])]),
        ]))
    }

    fn resolve(&mut self, left: Expr, right: Expr) -> Vec<Solution> {
        let mut out = Vec::new();
        let (peeled, value) = peel(left.clone(), right.clone(), &self.y);
        if peeled.is_sym(&self.y) {
            return vec![Solution::Explicit(absorb(
                &simplify(&value),
                &self.constants,
            ))];
        }
        if let Ok(s) = solve::solve(&sub(peeled, value), &self.y) {
            solve::release(&s.params);
            if !s.all && s.notes.is_empty() {
                for r in s
                    .roots
                    .iter()
                    .filter(|r| !r.approx && !s.params.iter().any(|p| r.value.has_sym(p)))
                {
                    out.push(Solution::Explicit(absorb(
                        &simplify(&r.value),
                        &self.constants,
                    )));
                }
            }
        }
        if out.is_empty() {
            out.push(Solution::Implicit(left, right));
        }
        out
    }

    /// `y' = b(x) y + c(x) y^n`.
    fn bernoulli(&mut self, slope: &Expr) -> Option<Vec<Expr>> {
        let ys = self.ys();
        let (mut b, mut c, mut n) = (Vec::new(), Vec::new(), None::<Expr>);
        for t in terms_of(&expand(slope)) {
            let (mut power, mut rest) = (Vec::new(), Vec::new());
            for f in factors_of(&t) {
                let (base, k) = f.base_exp();
                if base == ys && k.is_number() {
                    power.push(k);
                } else if f.has_sym(&self.y) {
                    return None;
                } else {
                    rest.push(f);
                }
            }
            let k = add(power);
            if k.is_one() {
                b.push(mul(rest));
            } else if k.is_zero() || n.as_ref().is_some_and(|m| *m != k) {
                return None;
            } else {
                n = Some(k);
                c.push(mul(rest));
            }
        }
        let m = sub(Expr::one(), n?);
        // v = y^(1 - n): v' = m (b v + c)
        let p = integrate(&simplify(&mul(vec![m.clone(), add(b)])), &self.x)?;
        let factor = pow(Expr::e(), neg(p));
        let inner = integrate(
            &simplify(&mul(vec![factor.clone(), m.clone(), add(c)])),
            &self.x,
        )?;
        let v = simplify(&div(add(vec![inner, self.constant()]), factor));
        let root = pow(v, inv(m.clone()));
        Some(if m.as_i64().is_some_and(|m| m % 2 == 0) {
            vec![neg(root.clone()), root]
        } else {
            vec![root]
        })
    }

    /// `y' = A(x) B(y)`.
    fn separable(&mut self, slope: &Expr) -> Option<Vec<Solution>> {
        // by the factors, with e^(u + v) taken apart
        let apart = map_bottom_up(slope, &mut |e| match &e {
            Expr::Pow(b, p) if b.is_sym("e") && matches!(**p, Expr::Add(_)) => Expr::Mul(
                terms_of(p)
                    .into_iter()
                    .map(|t| pow(Expr::e(), t))
                    .collect::<Vec<_>>()
                    .into(),
            ),
            _ => e,
        });
        let mut flat = Vec::new();
        for f in factors_of(&apart) {
            flat.extend(factors_of(&f));
        }
        if flat
            .iter()
            .all(|f| !(f.has_sym(&self.x) && f.has_sym(&self.y)))
        {
            let (of_y, of_x): (Vec<Expr>, Vec<Expr>) =
                flat.into_iter().partition(|f| f.has_sym(&self.y));
            let left = integrate(&simplify(&inv(mul(of_y))), &self.y)?;
            let right = add(vec![integrate(&mul(of_x), &self.x)?, self.constant()]);
            return Some(self.resolve(left, right));
        }
        for (x0, y0) in [(1, 1), (2, 3), (1, 2), (3, 1)] {
            let (x0, y0) = (Expr::int(x0), Expr::int(y0));
            let scale = simplify(&subst_sym(&subst_sym(slope, &self.x, &x0), &self.y, &y0));
            if scale.is_zero()
                || !numeric::evalf(&scale, 64)
                    .is_ok_and(|v| v.real().is_finite() && v.imag().is_finite())
            {
                continue;
            }
            let a = simplify(&div(subst_sym(slope, &self.y, &y0), scale));
            let b = simplify(&subst_sym(slope, &self.x, &x0));
            if !is_zero(&sub(slope.clone(), mul(vec![a.clone(), b.clone()]))) {
                return None;
            }
            let left = integrate(&simplify(&inv(b)), &self.y)?;
            let right = add(vec![integrate(&a, &self.x)?, self.constant()]);
            return Some(self.resolve(left, right));
        }
        None
    }

    /// `y' = phi(y/x)`.
    fn homogeneous(&mut self, slope: &Expr) -> Option<Vec<Solution>> {
        let v = Expr::sym(RATIO);
        let phi = simplify(&subst_sym(slope, &self.y, &mul(vec![v.clone(), self.xs()])));
        if phi.has_sym(&self.x) {
            return None;
        }
        let rest = simplify(&sub(phi, v));
        if rest.is_zero() {
            return Some(vec![Solution::Explicit(mul(vec![
                self.constant(),
                self.xs(),
            ]))]);
        }
        let h = integrate(&simplify(&inv(rest)), RATIO)?;
        let left = simplify(&subst_sym(&h, RATIO, &div(self.ys(), self.xs())));
        let right = add(vec![ln(self.xs()), self.constant()]);
        Some(self.resolve(left, right))
    }

    /// `M + N y' = 0` with `dM/dy = dN/dx`.
    fn exact(&mut self, f: &Expr) -> Option<Vec<Solution>> {
        let d1 = marker(1);
        let n = simplify(&diff(f, &d1));
        let m = simplify(&subst_sym(f, &d1, &Expr::zero()));
        if n.has_sym(&d1) || !is_zero(&sub(diff(&m, &self.y), diff(&n, &self.x))) {
            return None;
        }
        let first = integrate(&m, &self.x)?;
        let second = integrate(&simplify(&sub(n, diff(&first, &self.y))), &self.y)?;
        let c = self.constant();
        Some(self.resolve(simplify(&add(vec![first, second])), c))
    }

    fn first_order(&mut self, f: &Expr) -> Result<Vec<Solution>, String> {
        let d1 = marker(1);
        let solved = solve::solve(f, &d1)?;
        solve::release(&solved.params);
        let slopes: Vec<Expr> = solved
            .roots
            .iter()
            .filter(|r| !r.approx && !solved.params.iter().any(|p| r.value.has_sym(p)))
            .map(|r| r.value.clone())
            .collect();
        if slopes.is_empty() {
            return Err(format!("the equation cannot be solved for {}'", self.y));
        }
        let mut out = Vec::new();
        for slope in slopes {
            if !slope.has_sym(&self.y) {
                let anti = integrate(&slope, &self.x)
                    .ok_or_else(|| format!("cannot integrate {slope}"))?;
                out.push(Solution::Explicit(add(vec![anti, self.constant()])));
            } else if let Some(s) = self.bernoulli(&slope) {
                out.extend(s.into_iter().map(Solution::Explicit));
            } else if let Some(s) = self.separable(&slope) {
                out.extend(s);
            } else if let Some(s) = self.homogeneous(&slope) {
                out.extend(s);
            } else if let Some(s) = self.exact(f) {
                out.extend(s);
            } else {
                return Err(format!("no method for {}' = {slope}", self.y));
            }
        }
        Ok(out)
    }
}

/// Solves `f = 0`, where `f` holds `y`, `x` and the derivative markers; returns the solutions and their constants.
pub fn dsolve(
    f: &Expr,
    y: &str,
    x: &str,
    notes: &mut Vec<String>,
) -> Result<(Vec<Solution>, Vec<String>), String> {
    let order = (1..=20u32)
        .rev()
        .find(|n| f.has_sym(&marker(*n)))
        .ok_or_else(|| format!("the equation has no derivative of {y}"))?;
    let prefix = ["C", "K", "A"]
        .into_iter()
        .find(|p| !f.free_syms().iter().any(|s| s.starts_with(p)))
        .unwrap_or("C_");
    let mut ode = Ode {
        y: y.to_string(),
        x: x.to_string(),
        constants: Vec::new(),
        prefix,
    };
    let names: Vec<String> = std::iter::once(y.to_string())
        .chain((1..=order).map(marker))
        .collect();
    let free = |e: &Expr| names.iter().all(|n| !e.has_sym(n));
    // linear: f = sum a_k y^(k) - g
    let a: Vec<Expr> = names.iter().map(|n| simplify(&diff(f, n))).collect();
    let g = simplify(&neg(names
        .iter()
        .fold(f.clone(), |acc, n| subst_sym(&acc, n, &Expr::zero()))));
    let linear = a.iter().all(free) && free(&g) && {
        let rebuilt = add(a
            .iter()
            .zip(&names)
            .map(|(c, n)| mul(vec![c.clone(), Expr::sym(n)]))
            .collect());
        is_zero(&sub(sub(f.clone(), rebuilt), neg(g.clone())))
    };
    let solutions = if linear && a.iter().all(|c| c.free_of(x)) {
        vec![Solution::Explicit(ode.constant_coefficients(&a, &g, x)?)]
    } else if linear && order == 1 {
        vec![Solution::Explicit(
            ode.first_order_linear(&a[0], &a[1], &g)?,
        )]
    } else if linear {
        match ode
            .cauchy_euler(&a, &g)
            .or_else(|| ode.airy_bessel(&a, &g).map(Ok))
        {
            Some(s) => vec![Solution::Explicit(s?)],
            None => return Err("linear equations with variable coefficients are solved for order 1 and for the Cauchy-Euler form only".to_string()),
        }
    } else if order == 1 {
        ode.first_order(f)?
    } else {
        return Err("no method for this differential equation".to_string());
    };
    for s in &solutions {
        let Solution::Explicit(e) = s else { continue };
        let mut residual = subst_sym(f, y, e);
        for k in 1..=order {
            residual = subst_sym(&residual, &marker(k), &diff_n(e, x, k));
        }
        match check(&simplify(&residual), x) {
            (0, 0) => notes.push(format!("{y} = {e} was not verified numerically")),
            (0, _) => {
                return Err(format!(
                    "the candidate {y} = {e} does not satisfy the equation"
                ));
            }
            (_, 0) => {}
            _ => notes.push(format!(
                "{y} = {e} satisfies the equation on a part of the real line only"
            )),
        }
    }
    Ok((solutions, ode.constants))
}

/// How many real sample points make the residual vanish, and how many do not.
fn check(residual: &Expr, x: &str) -> (u32, u32) {
    let names: Vec<std::rc::Rc<str>> = residual.free_syms().into_iter().collect();
    let (mut pass, mut fail) = (0, 0);
    for (i, x0) in [-2.3, -1.1, -0.4, 0.3, 0.9, 1.7, 2.6, 3.8]
        .iter()
        .enumerate()
    {
        let mut env = numeric::Env::new();
        for (j, name) in names.iter().enumerate() {
            let other = (0.37 + 0.29 * j as f64) * if (i + j) % 2 == 0 { 1.0 } else { -1.0 };
            env.insert(
                name.clone(),
                rug::Complex::with_val(192, if **name == *x { *x0 } else { other }),
            );
        }
        if let Ok(v) = numeric::evalf_env(residual, 192, &env) {
            if rug::Float::with_val(64, v.abs_ref()) < 1e-30 {
                pass += 1;
            } else {
                fail += 1;
            }
        }
    }
    (pass, fail)
}

/// Fixes constants from conditions `y^(d)(p) = v`.
pub fn apply_conditions(
    sol: &Expr,
    x: &str,
    constants: &[String],
    conditions: &[(u32, Expr, Expr)],
) -> Result<Expr, String> {
    let used: Vec<String> = constants
        .iter()
        .filter(|c| sol.has_sym(c))
        .cloned()
        .collect();
    if conditions.is_empty() || used.is_empty() {
        return Ok(sol.clone());
    }
    let eqs: Vec<Expr> = conditions
        .iter()
        .map(|(d, p, v)| simplify(&sub(subst_sym(&diff_n(sol, x, *d), x, p), v.clone())))
        .collect();
    let vars: Vec<String> = used.into_iter().take(eqs.len()).collect();
    let solved = crate::system::solve_system(&eqs, &vars)?;
    let first = solved
        .solutions
        .first()
        .ok_or("the conditions cannot be met")?;
    Ok(simplify(
        &first.iter().fold(sol.clone(), |acc, (name, value)| {
            subst_sym(&acc, name, value)
        }),
    ))
}
