//! Equations in one unknown: exact roots where a method applies, verified numerically,
//! parametrized families for periodic functions, numeric roots otherwise.

use crate::assume::{self, Facts};
use crate::canon::{add, div, map_bottom_up, minus_one_pow, mul, neg, pow, sqrt, sub, subst};
use crate::complex;
use crate::expr::Expr;
use crate::funcs::{func, root_of_unity};
use crate::num::{Int, Rat, rat};
use crate::numeric::{self, Env};
use crate::numroots;
use crate::poly::Poly;
use crate::ratsimp::{self, Gens};
use crate::simplify::{self, simplify};
use crate::zfactor;
use rug::{Complete, Complex};
use std::collections::BTreeSet;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct Root {
    pub value: Expr,
    pub mult: u32,
    /// A numeric value, not an exact form.
    pub approx: bool,
    /// The whole numbers for which a family gives no solution: the equation has no value
    /// there and does not tend to zero.
    pub skip: Vec<i64>,
}

#[derive(Debug, Default)]
pub struct Solved {
    pub roots: Vec<Root>,
    /// Integer parameters of the families among the roots.
    pub params: Vec<String>,
    /// Every value of the unknown is a solution.
    pub all: bool,
    pub notes: Vec<String>,
}

fn exact(value: Expr) -> Root {
    Root {
        value,
        mult: 1,
        approx: false,
        skip: Vec::new(),
    }
}

fn i_times(e: Expr) -> Expr {
    mul(vec![Expr::i(), e])
}

fn two_pi() -> Expr {
    mul(vec![Expr::int(2), Expr::pi()])
}

fn ugly(e: &Expr) -> bool {
    e.any(&|x| match x {
        Expr::Pow(b, p) => {
            b.as_num().is_some_and(|n| n.is_minus_one() || !n.is_real()) && p.as_int().is_none()
        }
        _ => false,
    })
}

/// Simplest exact form of a root; constants prefer `a + b*i`.
pub fn tidy(e: &Expr) -> Expr {
    let s = simplify(e);
    if s.any(&|x| matches!(x, Expr::Sym(n) if assume::is_integer_sym(n))) {
        return crate::expand::expand(&s);
    }
    if s.is_const()
        && !s.has_float()
        && !s.is_infinite()
        && !assume::is_real(&s)
        && let Some(r) = complex::rectform(&s)
    {
        let r = simplify(&r);
        if !r.has_fn("arg") && ((ugly(&s) && !ugly(&r)) || simplify::cost(&r) < simplify::cost(&s))
        {
            return r;
        }
    }
    s
}

fn kth_roots(y: &Expr, k: u32) -> Vec<Expr> {
    let principal = pow(y.clone(), Expr::rat(1, i64::from(k)));
    (0..k)
        .map(|j| {
            let r = Rat::from((2 * j, k));
            let w = root_of_unity(&r).unwrap_or_else(|| minus_one_pow(&r));
            tidy(&mul(vec![principal.clone(), w]))
        })
        .collect()
}

fn occurrences(e: &Expr, name: &str) -> usize {
    let mut n = 0;
    e.walk(&mut |x| {
        if x.is_sym(name) {
            n += 1;
        }
    });
    n
}

/// Trigonometric and hyperbolic functions written with exponentials.
pub fn rewrite_exp(e: &Expr) -> Expr {
    map_bottom_up(e, &mut |x| {
        let Expr::Fn(name, a) = &x else { return x };
        if a.len() != 1 {
            return x;
        }
        let circular = matches!(&**name, "sin" | "cos" | "tan");
        if !circular && !matches!(&**name, "sinh" | "cosh" | "tanh") {
            return x;
        }
        let arg = if circular {
            i_times(a[0].clone())
        } else {
            a[0].clone()
        };
        let (p, m) = (pow(Expr::e(), arg.clone()), pow(Expr::e(), neg(arg)));
        let (s, c) = (sub(p.clone(), m.clone()), add(vec![p, m]));
        match &**name {
            "sin" => div(s, i_times(Expr::int(2))),
            "cos" | "cosh" => div(c, Expr::int(2)),
            "tan" => div(s, i_times(c)),
            "sinh" => div(s, Expr::int(2)),
            _ => div(s, c),
        }
    })
}

struct Solver {
    var: String,
    x: Expr,
    params: Vec<String>,
    used: BTreeSet<String>,
    notes: Vec<String>,
    depth: u32,
}

type Roots = Result<Vec<Root>, String>;

impl Solver {
    fn param(&mut self) -> Expr {
        // (j is read as i, and k is a unit among the quaternions)
        let units = assume::quaternion_mode();
        let name = ["k", "n", "m", "l", "p", "q"]
            .iter()
            .filter(|s| !units || **s != "k")
            .map(|s| s.to_string())
            .chain((1..).map(|i| format!("n{i}")))
            .find(|s| !self.used.contains(s))
            .unwrap();
        self.used.insert(name.clone());
        self.params.push(name.clone());
        assume::set(&name, Facts::integer());
        Expr::sym(&name)
    }

    fn note(&mut self, text: &str) {
        if !self.notes.iter().any(|n| n == text) {
            self.notes.push(text.to_string());
        }
    }

    fn solve_expr(&mut self, f: &Expr) -> Roots {
        self.depth += 1;
        let r = if self.depth > 24 {
            Err("the equation is nested too deeply".to_string())
        } else {
            self.solve_inner(f)
        };
        self.depth -= 1;
        r
    }

    fn solve_inner(&mut self, f: &Expr) -> Roots {
        if !f.has_sym(&self.var) {
            return Ok(Vec::new());
        }
        if let Some(cs) = ratsimp::coefficients(f, &self.var) {
            return self.solve_poly(cs);
        }
        let scaled = |roots: Vec<Root>, e: &Expr| -> Vec<Root> {
            let k = e.as_i64().and_then(|k| u32::try_from(k).ok()).unwrap_or(1);
            roots
                .into_iter()
                .map(|r| Root {
                    mult: r.mult * k,
                    ..r
                })
                .collect()
        };
        match f {
            Expr::Mul(a) => {
                let mut out = Vec::new();
                let var = self.var.clone();
                for fac in a.iter().filter(|fac| fac.has_sym(&var)) {
                    match fac {
                        Expr::Pow(b, e) if e.free_of(&var) => match numeric::sign(e) {
                            Some(s) if s < 0 => {}
                            Some(_) => out.extend(scaled(self.solve_expr(b)?, e)),
                            None => out.extend(self.solve_expr(fac)?),
                        },
                        _ => out.extend(self.solve_expr(fac)?),
                    }
                }
                return Ok(together(out, &self.params));
            }
            Expr::Pow(b, e) if e.free_of(&self.var) && numeric::sign(e) == Some(1) => {
                return Ok(scaled(self.solve_expr(b)?, e));
            }
            _ => {}
        }
        if occurrences(f, &self.var) == 1 {
            return self.invert(f, &Expr::zero());
        }
        if let Expr::Add(terms) = f {
            let (with, without): (Vec<Expr>, Vec<Expr>) =
                terms.iter().cloned().partition(|t| t.has_sym(&self.var));
            if with.len() == 1 && !without.is_empty() {
                return self.invert(&with[0], &neg(add(without)));
            }
        }
        // a sum with a common factor: x*sin(x) - sin(x) vanishes where a factor does
        if f.size() <= 300 {
            let product = match ratsimp::factor(f) {
                Expr::Fn(n, a) if &*n == "hold" && a.len() == 1 => a[0].clone(),
                other => other,
            };
            if let Expr::Mul(parts) = &product
                && parts.iter().filter(|p| p.has_sym(&self.var)).count() >= 2
                && let Ok(found) = self.solve_expr(&product)
            {
                return Ok(found);
            }
        }
        self.by_kernels(f)
    }

    fn solve_poly(&mut self, mut cs: Vec<Expr>) -> Roots {
        while cs.last().is_some_and(simplify::is_zero) {
            cs.pop();
        }
        let mut out = Vec::new();
        let low = cs.iter().take_while(|c| simplify::is_zero(c)).count();
        if low > 0 && low < cs.len() {
            out.push(Root {
                value: Expr::zero(),
                mult: low as u32,
                approx: false,
                skip: Vec::new(),
            });
            cs.drain(..low);
        }
        let n = cs.len().saturating_sub(1);
        if n == 0 {
            return Ok(out);
        }
        if cs.iter().all(|c| c.as_rat().is_some()) {
            let dense: Vec<Rat> = cs.iter().map(|c| c.as_rat().unwrap().clone()).collect();
            out.extend(self.rational_poly(&dense));
            return Ok(out);
        }
        // only powers of x^k
        let step = (1..=n).filter(|i| !cs[*i].is_zero()).fold(0, gcd_usize);
        if step > 1 {
            let reduced: Vec<Expr> = cs.iter().step_by(step).cloned().collect();
            for r in self.solve_poly(reduced)? {
                for v in kth_roots(&r.value, step as u32) {
                    out.push(Root {
                        value: v,
                        ..r.clone()
                    });
                }
            }
            return Ok(out);
        }
        match n {
            1 => out.push(exact(tidy(&neg(div(cs[0].clone(), cs[1].clone()))))),
            2 => out.extend(quadratic(&cs[2], &cs[1], &cs[0])),
            _ => {
                let x = self.x.clone();
                let poly = add(cs
                    .iter()
                    .enumerate()
                    .map(|(i, c)| mul(vec![c.clone(), pow(x.clone(), Expr::int(i as i64))]))
                    .collect());
                let factored = ratsimp::factor(&poly);
                let parts: Vec<Expr> = match &factored {
                    Expr::Mul(a) => a.iter().filter(|p| p.has_sym(&self.var)).cloned().collect(),
                    _ => Vec::new(),
                };
                if parts.len() >= 2 || parts.iter().any(|p| matches!(p, Expr::Pow(..))) {
                    for p in parts {
                        out.extend(self.solve_expr(&p)?);
                    }
                    out = together(out, &self.params);
                } else if poly.free_syms().len() == 1 {
                    match self.constant_poly(&cs) {
                        Some(roots) => out.extend(roots),
                        None => out.extend(self.numeric(&poly)?),
                    }
                } else {
                    return Err(format!(
                        "no general formula for a polynomial of degree {n} with symbolic coefficients"
                    ));
                }
            }
        }
        Ok(out)
    }

    /// Roots of a polynomial of degree 3 or more whose coefficients (ascending) are numbers that
    /// are not all rational. `None` when the numeric values do not settle.
    fn constant_poly(&mut self, cs: &[Expr]) -> Option<Vec<Root>> {
        let n = cs.len() - 1;
        let digits = numeric::digits();
        if cs.iter().all(|c| c.as_num().is_some()) {
            // the polynomial times its conjugate has rational coefficients and all of its roots
            let part = |k: usize| cs[k].as_num().expect("a number");
            let mut norm = vec![Rat::new(); 2 * n + 1];
            for j in 0..=n {
                for k in 0..=n {
                    let (a, b) = (part(j), part(k));
                    norm[j + k] += (&a.re * &b.re).complete() + (&a.im * &b.im).complete();
                }
            }
            let mut out = Vec::new();
            for mut r in self.rational_poly(&norm) {
                r.mult = multiplicity(cs, &r);
                if r.mult > 0 {
                    out.push(r);
                }
            }
            return Some(out);
        }
        let c_at = |prec: u32| -> Option<Vec<Complex>> {
            cs.iter().map(|c| numeric::evalf(c, prec).ok()).collect()
        };
        let roots = numroots::poly_roots_complex(n, &c_at, digits)?;
        // a root that counts twice does not settle: the search from starting points takes over
        let near = |a: &Complex, b: &Complex| {
            let gap = rug::Float::with_val(64, Complex::with_val(a.prec(), a - b).abs_ref());
            gap < (rug::Float::with_val(64, a.abs_ref()) + 1u32) >> 30u32
        };
        if roots
            .iter()
            .enumerate()
            .any(|(k, a)| roots[..k].iter().any(|b| near(a, b)))
        {
            return None;
        }
        Some(
            roots
                .into_iter()
                .map(|v| Root {
                    value: numeric::to_float_expr(v, digits),
                    mult: 1,
                    approx: true,
                    skip: Vec::new(),
                })
                .collect(),
        )
    }

    /// Roots of a polynomial with rational coefficients.
    fn rational_poly(&mut self, dense: &[Rat]) -> Vec<Root> {
        let (_, factors, _) = zfactor::factor(&Poly::from_dense(1, 0, dense));
        let mut out = Vec::new();
        for (f, mult) in factors {
            let c = f.to_dense(0).unwrap();
            for mut r in self.irreducible(&c) {
                r.mult *= mult;
                out.push(r);
            }
        }
        out
    }

    fn irreducible(&mut self, c: &[Rat]) -> Vec<Root> {
        let n = c.len() - 1;
        let e = |r: &Rat| Expr::rational(r.clone());
        if n == 1 {
            return vec![exact(Expr::rational((-&c[0]).complete() / &c[1]))];
        }
        if n == 2 {
            return quadratic(&e(&c[2]), &e(&c[1]), &e(&c[0]));
        }
        let step = (1..=n).filter(|i| !c[*i].is_zero()).fold(0, gcd_usize);
        if step > 1 {
            let reduced: Vec<Rat> = c.iter().step_by(step).cloned().collect();
            let mut out = Vec::new();
            for r in self.rational_poly(&reduced) {
                if r.approx {
                    return self.numeric_poly(c);
                }
                for v in kth_roots(&r.value, step as u32) {
                    out.push(Root {
                        value: v,
                        ..r.clone()
                    });
                }
            }
            return out;
        }
        if let Some(roots) = cyclotomic_roots(c) {
            return roots;
        }
        let exact_roots = match n {
            3 => Some(cubic(c)),
            4 => quartic(c),
            _ => None,
        };
        match exact_roots {
            Some(roots) if roots.iter().all(|r| simplify::cost(r) <= 160) => {
                roots.into_iter().map(exact).collect()
            }
            _ => self.numeric_poly(c),
        }
    }

    fn numeric_poly(&mut self, c: &[Rat]) -> Vec<Root> {
        let digits = numeric::digits();
        match numroots::poly_roots(c, digits) {
            Some(roots) => roots
                .into_iter()
                .map(|v| Root {
                    value: numeric::to_float_expr(v, digits),
                    mult: 1,
                    approx: true,
                    skip: Vec::new(),
                })
                .collect(),
            None => {
                self.note("the numeric root search of a polynomial did not converge");
                Vec::new()
            }
        }
    }

    /// Solves `lhs = rhs` for an `rhs` without the unknown by undoing the outermost operation.
    fn invert(&mut self, lhs: &Expr, rhs: &Expr) -> Roots {
        let var = self.var.clone();
        let has = |e: &Expr| e.has_sym(&var);
        if *lhs == self.x {
            return Ok(vec![exact(tidy(rhs))]);
        }
        match lhs {
            Expr::Add(terms) => {
                let (with, without): (Vec<Expr>, Vec<Expr>) =
                    terms.iter().cloned().partition(|t| has(t));
                if without.is_empty() || with.len() != 1 {
                    return self.by_kernels(&sub(lhs.clone(), rhs.clone()));
                }
                self.invert(&with[0], &sub(rhs.clone(), add(without)))
            }
            Expr::Mul(factors) => {
                let (with, without): (Vec<Expr>, Vec<Expr>) =
                    factors.iter().cloned().partition(|t| has(t));
                if without.is_empty() || with.len() != 1 {
                    return self.by_kernels(&sub(lhs.clone(), rhs.clone()));
                }
                self.invert(&with[0], &div(rhs.clone(), mul(without)))
            }
            Expr::Pow(b, e) => {
                if !has(e) {
                    if let Some(r) = e.as_rat() {
                        let (p, q) = (r.numer().clone(), r.denom().clone());
                        let target = if p.is_negative() {
                            div(Expr::one(), rhs.clone())
                        } else {
                            rhs.clone()
                        };
                        let target = pow(target, Expr::integer(q));
                        let k = p.abs().to_u32().ok_or("the exponent is too large")?;
                        if k > 360 {
                            return Err("the exponent is too large".into());
                        }
                        let mut out = Vec::new();
                        for root in kth_roots(&target, k) {
                            out.extend(self.invert(b, &root)?);
                        }
                        return Ok(out);
                    }
                    return self.invert(b, &pow(rhs.clone(), div(Expr::one(), (**e).clone())));
                }
                if !has(b) {
                    if simplify::is_zero(rhs) {
                        return Ok(Vec::new());
                    }
                    let k = self.param();
                    let log = add(vec![log_of(rhs), mul(vec![two_pi(), Expr::i(), k])]);
                    let target = if b.is_sym("e") {
                        log
                    } else {
                        div(log, log_of(b))
                    };
                    return self.invert(e, &target);
                }
                if **b == self.x && **e == self.x {
                    // x^x = c
                    return Ok(vec![exact(pow(
                        Expr::e(),
                        func("lambertw", vec![log_of(rhs)]),
                    ))]);
                }
                self.numeric(&sub(lhs.clone(), rhs.clone()))
            }
            Expr::Fn(name, args) if args.len() == 1 => {
                let g = &args[0];
                let c = rhs.clone();
                let f = |n: &str, a: Expr| func(n, vec![a]);
                let pi = Expr::pi();
                let mut targets: Vec<Expr> = Vec::new();
                let family = |solver: &mut Solver, base: Expr, period: Expr| {
                    let k = solver.param();
                    add(vec![base, mul(vec![period, k])])
                };
                match &**name {
                    "ln" => targets.push(pow(Expr::e(), c)),
                    "sin" => {
                        if simplify::is_zero(&c) {
                            targets.push(family(self, Expr::zero(), pi));
                        } else {
                            let a = f("asin", c.clone());
                            let both = !simplify::is_zero(&sub(pow(c, Expr::int(2)), Expr::one()));
                            targets.push(family(self, a.clone(), two_pi()));
                            if both {
                                targets.push(family(self, sub(pi, a), two_pi()));
                            }
                        }
                    }
                    "cos" => {
                        if simplify::is_zero(&c) {
                            targets.push(family(self, div(pi, Expr::int(2)), Expr::pi()));
                        } else {
                            let a = f("acos", c.clone());
                            let both = !simplify::is_zero(&sub(pow(c, Expr::int(2)), Expr::one()));
                            targets.push(family(self, a.clone(), two_pi()));
                            if both {
                                targets.push(family(self, neg(a), two_pi()));
                            }
                        }
                    }
                    "tan" => targets.push(family(self, f("atan", c), pi)),
                    "sinh" => {
                        let a = f("asinh", c);
                        targets.push(family(self, a.clone(), i_times(two_pi())));
                        targets.push(family(self, sub(i_times(pi), a), i_times(two_pi())));
                    }
                    "cosh" => {
                        let a = f("acosh", c);
                        targets.push(family(self, a.clone(), i_times(two_pi())));
                        targets.push(family(self, neg(a), i_times(two_pi())));
                    }
                    "tanh" => targets.push(family(self, f("atanh", c), i_times(pi))),
                    "asin" => targets.push(f("sin", c)),
                    "acos" => targets.push(f("cos", c)),
                    "atan" => targets.push(f("tan", c)),
                    "asinh" => targets.push(f("sinh", c)),
                    "acosh" => targets.push(f("cosh", c)),
                    "atanh" => targets.push(f("tanh", c)),
                    "conj" => targets.push(f("conj", c)),
                    "lambertw" => targets.push(mul(vec![c.clone(), pow(Expr::e(), c)])),
                    "abs" => {
                        self.note("abs: the real solutions are given");
                        match numeric::sign(&c) {
                            Some(s) if s < 0 => {}
                            Some(0) => targets.push(Expr::zero()),
                            _ => {
                                targets.push(c.clone());
                                targets.push(neg(c));
                            }
                        }
                    }
                    _ => return self.numeric(&sub(lhs.clone(), rhs.clone())),
                }
                let mut out = Vec::new();
                for t in targets {
                    out.extend(self.invert(g, &t)?);
                }
                Ok(out)
            }
            Expr::Fn(name, args) if args.len() == 2 => {
                let (a, b) = (&args[0], &args[1]);
                let c = rhs.clone();
                let two = |n: &str, p: Expr, q: Expr| func(n, vec![p, q]);
                match (&**name, has(a), has(b)) {
                    ("tet", false, true) => self.invert(b, &two("slog", a.clone(), c)),
                    ("tet", true, false) => self.invert(a, &two("ssrt", c, b.clone())),
                    ("slog", false, true) => self.invert(b, &two("tet", a.clone(), c)),
                    ("slog", true, false) => self.invert(a, &two("ssrt", b.clone(), c)),
                    ("ssrt", true, false) => self.invert(a, &two("tet", c, b.clone())),
                    ("ssrt", false, true) => self.invert(b, &two("slog", c, a.clone())),
                    ("lambertw", false, true) => {
                        self.invert(b, &mul(vec![c.clone(), pow(Expr::e(), c)]))
                    }
                    _ => self.numeric(&sub(lhs.clone(), rhs.clone())),
                }
            }
            _ => self.numeric(&sub(lhs.clone(), rhs.clone())),
        }
    }

    fn by_kernels(&mut self, f: &Expr) -> Roots {
        let var = self.var.clone();
        // a sin(u) + b cos(u) = 0
        if let Expr::Add(terms) = f
            && terms.len() == 2
        {
            let part = |t: &Expr, name: &str| -> Option<(Expr, Expr)> {
                let factors: Vec<Expr> = match t {
                    Expr::Mul(a) => a.to_vec(),
                    other => vec![other.clone()],
                };
                let pos = factors
                    .iter()
                    .position(|x| x.fn_arg(name).is_some_and(|u| u.has_sym(&var)))?;
                let rest: Vec<Expr> = factors
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != pos)
                    .map(|(_, x)| x.clone())
                    .collect();
                let coef = mul(rest);
                (!coef.has_sym(&var)).then(|| (coef, factors[pos].fn_arg(name).unwrap().clone()))
            };
            for (s, c) in [(0, 1), (1, 0)] {
                if let (Some((a, u)), Some((b, v))) =
                    (part(&terms[s], "sin"), part(&terms[c], "cos"))
                    && u == v
                {
                    return self.invert(&func("tan", vec![u]), &neg(div(b, a)));
                }
            }
        }
        let Some(gens) = Gens::of(&[f]) else {
            return self.numeric(f);
        };
        let xk: Vec<usize> = (0..gens.len())
            .filter(|i| gens.kernels()[*i].has_sym(&var))
            .collect();
        let kind = |k: &Expr| -> &'static str {
            match k {
                Expr::Fn(n, _)
                    if matches!(&**n, "sin" | "cos" | "tan" | "sinh" | "cosh" | "tanh") =>
                {
                    "trig"
                }
                Expr::Fn(n, _) if &**n == "ln" => "ln",
                Expr::Pow(_, e) if e.has_sym(&var) => "exp",
                Expr::Pow(..) => "root",
                Expr::Sym(_) => "x",
                _ => "other",
            }
        };
        let kinds: Vec<&str> = xk.iter().map(|i| kind(&gens.kernels()[*i])).collect();
        if xk.len() == 1 && kinds[0] != "x" {
            let idx = xk[0];
            let kernel = gens.kernels()[idx].clone();
            let Some((num, _)) = gens.convert(f) else {
                return self.numeric(f);
            };
            let cs: Vec<Expr> = num.coeffs(idx).iter().map(|c| gens.expr(c)).collect();
            let mut out = Vec::new();
            for r in self.solve_poly(cs)? {
                for mut s in self.invert(&kernel, &r.value)? {
                    s.approx |= r.approx;
                    out.push(s);
                }
            }
            return Ok(out);
        }
        if kinds.contains(&"trig") {
            let g = rewrite_exp(f);
            if g != *f {
                return self.solve_expr(&ratsimp::fraction(&g).0);
            }
        }
        if kinds.contains(&"ln")
            && let Expr::Add(terms) = f
        {
            let mut product = Vec::new();
            let mut rest = Vec::new();
            let mut ok = true;
            for t in terms.iter() {
                if !t.has_sym(&var) {
                    rest.push(t.clone());
                    continue;
                }
                let factors: Vec<Expr> = match t {
                    Expr::Mul(a) => a.to_vec(),
                    other => vec![other.clone()],
                };
                match factors.iter().position(|x| x.fn_arg("ln").is_some()) {
                    Some(pos) => {
                        let c = mul(factors
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| *i != pos)
                            .map(|(_, x)| x.clone())
                            .collect());
                        if c.has_sym(&var) {
                            ok = false;
                        }
                        product.push(pow(factors[pos].fn_arg("ln").unwrap().clone(), c));
                    }
                    None => ok = false,
                }
            }
            if ok && !product.is_empty() {
                let eq = sub(mul(product), pow(Expr::e(), neg(add(rest))));
                return self.solve_expr(&ratsimp::fraction(&eq).0);
            }
        }
        if kinds.contains(&"root") && kinds.iter().all(|k| matches!(*k, "root" | "x")) {
            let Some((mut p, _)) = gens.convert(f) else {
                return self.numeric(f);
            };
            let mut eliminated = false;
            for (idx, q, pw, base) in gens.radicals() {
                if !gens.kernels()[idx].has_sym(&var) || !p.uses(idx) {
                    continue;
                }
                let Some(b) = gens.poly(&base) else {
                    return self.numeric(f);
                };
                let rel = Poly::var(gens.len(), idx).pow(q).sub(&b.pow(pw));
                if rel.is_zero() {
                    continue;
                }
                p = p.resultant(&rel, idx);
                eliminated = true;
            }
            if eliminated && !p.is_zero() {
                return self.solve_expr(&gens.expr(&p));
            }
        }
        if let Some(r) = self.lambert(f, &gens, &xk, &kinds) {
            return r;
        }
        self.numeric(f)
    }

    /// `(a1 x + a0) E + B = 0` or `A E + (b1 x + b0) = 0` with `E = e^(l x + d)`.
    fn lambert(&mut self, f: &Expr, gens: &Gens, xk: &[usize], kinds: &[&str]) -> Option<Roots> {
        if xk.len() != 2 || !kinds.contains(&"x") {
            return None;
        }
        let var = self.var.clone();
        let (ix, ie) = if kinds[0] == "x" {
            (xk[0], xk[1])
        } else {
            (xk[1], xk[0])
        };
        let (num, _) = gens.convert(f)?;
        let w = |e: Expr| func("lambertw", vec![e]);
        if kinds.contains(&"ln") {
            // a x ln(x) + B = 0
            let kernel = &gens.kernels()[ie];
            if kernel.fn_arg("ln") != Some(&self.x) || num.deg(ie) != 1 || num.deg(ix) != 1 {
                return None;
            }
            let cs = num.coeffs(ie);
            let (b, a) = (
                gens.expr(&cs[0]),
                ratsimp::coefficients(&gens.expr(&cs[1]), &var)?,
            );
            if b.has_sym(&var) || a.len() != 2 || !a[0].is_zero() {
                return None;
            }
            return Some(Ok(vec![exact(pow(
                Expr::e(),
                w(neg(div(b, a[1].clone()))),
            ))]));
        }
        let Expr::Pow(base, expo) = &gens.kernels()[ie] else {
            return None;
        };
        if base.has_sym(&var) || num.deg(ie) != 1 {
            return None;
        }
        let line = ratsimp::coefficients(&mul(vec![(**expo).clone(), log_of(base)]), &var)?;
        if line.len() != 2 {
            return None;
        }
        let (d, l) = (line[0].clone(), line[1].clone());
        let cs = num.coeffs(ie);
        let (b, a) = (
            ratsimp::coefficients(&gens.expr(&cs[0]), &var)?,
            ratsimp::coefficients(&gens.expr(&cs[1]), &var)?,
        );
        let (a1, a0, rhs, l, d) = match (a.len(), b.len()) {
            (2, 1) => (a[1].clone(), a[0].clone(), neg(b[0].clone()), l, d),
            (1, 2) => (
                b[1].clone(),
                b[0].clone(),
                neg(a[0].clone()),
                neg(l),
                neg(d),
            ),
            _ => return None,
        };
        let shift = div(a0, a1.clone());
        let c = simplify(&mul(vec![
            rhs,
            div(l.clone(), a1),
            pow(Expr::e(), sub(mul(vec![l.clone(), shift.clone()]), d)),
        ]));
        let mut out = vec![exact(sub(div(w(c.clone()), l.clone()), shift.clone()))];
        if let Some(v) = numeric::real_value(&c, 20)
            && v.is_sign_negative()
            && v > -0.3678794411714423f64
        {
            out.push(exact(sub(
                div(func("lambertw", vec![Expr::minus_one(), c]), l),
                shift,
            )));
        }
        self.note("Lambert W: the real branches are given; lambertw(k, z) with other integers k gives complex solutions");
        Some(Ok(out))
    }

    /// Numeric search from many starting points.
    fn numeric(&mut self, f: &Expr) -> Roots {
        let vars: Vec<Rc<str>> = f.free_syms().into_iter().collect();
        if vars.len() != 1 || &*vars[0] != self.var.as_str() {
            return Err("no method for this equation".to_string());
        }
        let digits = numeric::digits();
        let key: Rc<str> = vars[0].clone();
        // simple numbers that solve the equation exactly; where one counts several times the
        // search looks at the equation without it (it would only creep towards such a root)
        let mut simple = self.simple_roots(f);
        let places: Vec<(Complex, u32)> = simple
            .iter()
            .filter_map(|(c, times)| Some((numeric::evalf(c, 1024).ok()?, *times)))
            .collect();
        let mut eval = |z: &Complex| -> Option<Complex> {
            // (far away nothing is looked for, and gamma or zeta cost much there)
            let size = rug::Float::with_val(64, z.abs_ref());
            if size.is_nan() || size >= 1e6f64 {
                return None;
            }
            let mut env = Env::new();
            env.insert(key.clone(), z.clone());
            let mut value = numeric::evalf_env(f, z.prec().0, &env).ok()?;
            for (at, times) in places.iter().filter(|(_, times)| *times > 1) {
                let gap = Complex::with_val(z.prec(), z - at);
                if gap.is_zero() {
                    return None;
                }
                for _ in 0..*times {
                    value /= &gap;
                }
            }
            Some(value)
        };
        let mut starts: Vec<(f64, f64)> = Vec::new();
        for r in [
            0.0, 0.5, -0.5, 1.0, -1.0, 2.0, -2.0, 3.0, -3.0, 5.0, -5.0, 10.0, -10.0, 30.0, -30.0,
            100.0, 0.1, -0.1, 1.5, 7.0,
        ] {
            starts.push((r, 0.0));
        }
        for (a, b) in [
            (0.0, 1.0),
            (0.0, -1.0),
            (1.0, 1.0),
            (1.0, -1.0),
            (-1.0, 1.0),
            (-1.0, -1.0),
            (2.0, 3.0),
            (2.0, -3.0),
            (-3.0, 2.0),
            (-3.0, -2.0),
            (0.3, 5.0),
            (0.3, -5.0),
            (4.0, 2.5),
            (4.0, -2.5),
            (-4.0, 2.5),
            (-4.0, -2.5),
            (8.0, 3.0),
            (8.0, -3.0),
            (-8.0, 3.0),
            (-8.0, -3.0),
            (14.0, 3.5),
            (14.0, -3.5),
            (-14.0, 3.5),
            (-14.0, -3.5),
        ] {
            starts.push((a, b));
        }
        let mut found: Vec<Complex> = Vec::new();
        for (at, (re, im)) in starts.into_iter().enumerate() {
            // (the far starting points only when the near ones found next to nothing)
            if at >= 32 && found.len() + simple.len() >= 2 {
                break;
            }
            let start = Complex::with_val(64, (re + 0.013, im));
            let Some(mut z) = numroots::find_root(&mut eval, &start, digits) else {
                continue;
            };
            let scale = rug::Float::with_val(64, z.abs_ref()) + 1u32;
            let noise = rug::Float::with_val(64, &scale >> (numeric::bits_for(digits) - 8));
            if rug::Float::with_val(64, z.imag().abs_ref()) < noise {
                z = Complex::with_val(z.prec(), z.real());
            } else if rug::Float::with_val(64, z.real().abs_ref()) < noise {
                z = Complex::with_val(z.prec(), (0, z.imag()));
            }
            let known = found
                .iter()
                .chain(places.iter().map(|(at, _)| at))
                .any(|w| {
                    let d = Complex::with_val(64, &z - w);
                    rug::Float::with_val(64, d.abs_ref())
                        < rug::Float::with_val(64, &scale >> 40u32)
                });
            if !known {
                found.push(z);
            }
        }
        found.sort_by(|a, b| {
            let key = |v: &Complex| !v.imag().is_zero();
            key(a)
                .cmp(&key(b))
                .then_with(|| {
                    a.real()
                        .partial_cmp(b.real())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| {
                    a.imag()
                        .partial_cmp(b.imag())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        // an equation that holds on a whole stretch of numbers, as abs(x)/x = 1 and
        // floor(x) = 2 do: the number next to a solution is one too
        let holds = |z: &Complex| {
            let mut env = Env::new();
            env.insert(key.clone(), z.clone());
            numeric::evalf_env(f, z.prec().0.max(128), &env).is_ok_and(|v| v.is_zero())
        };
        let beside = |z: &Complex| {
            let prec = z.prec().0.max(128);
            let step = (rug::Float::with_val(prec, z.abs_ref()) + 1u32) >> 20u32;
            let zero = rug::Float::new(prec);
            holds(&Complex::with_val(
                prec,
                z + Complex::with_val(prec, (&step, &zero)),
            )) || holds(&Complex::with_val(
                prec,
                z + Complex::with_val(prec, (&zero, &step)),
            ))
        };
        if found
            .iter()
            .chain(places.iter().map(|(at, _)| at))
            .any(beside)
        {
            // a few examples, as round as they can be
            for z in found.iter_mut() {
                let round = format!("{:.2}", z.real().to_f64());
                let Ok(short) = rug::Float::parse(&round) else {
                    continue;
                };
                let plain = Complex::with_val(z.prec(), rug::Float::with_val(z.prec().0, short));
                if holds(&plain) {
                    *z = plain;
                }
            }
            found.dedup();
            let short = |z: &Complex| numeric::to_float_expr(z.clone(), digits).to_string().len();
            found.sort_by_key(short);
            found.truncate(3);
            simple.truncate(3);
            self.note(
                "the equation holds on whole stretches of numbers: the ones listed are examples",
            );
        }
        self.note("numeric search: the roots found are listed, there may be others");
        if found.is_empty() && !simple.is_empty() {
            self.note("no exact method: simple numbers were tried and a numeric search was made, solutions may have been missed");
        }
        let exact_ones = simple.into_iter().map(|(value, mult)| Root {
            value,
            mult,
            approx: false,
            skip: Vec::new(),
        });
        let numeric_ones = found.into_iter().map(|v| Root {
            value: numeric::to_float_expr(v, digits),
            mult: 1,
            approx: true,
            skip: Vec::new(),
        });
        Ok(exact_ones.chain(numeric_ones).collect())
    }

    /// The simple numbers that solve `f = 0` exactly, each with the number of times it does.
    fn simple_roots(&self, f: &Expr) -> Vec<(Expr, u32)> {
        let pi = Expr::pi;
        let half_pi = || div(pi(), Expr::int(2));
        let mut candidates: Vec<Expr> = (0..=6)
            .flat_map(|n| [n, -n])
            .skip(1)
            .map(Expr::int)
            .collect();
        candidates.extend([
            Expr::rat(1, 2),
            Expr::rat(-1, 2),
            Expr::rat(3, 2),
            Expr::rat(-3, 2),
            pi(),
            neg(pi()),
            half_pi(),
            neg(half_pi()),
            Expr::i(),
            neg(Expr::i()),
        ]);
        let mut out = Vec::new();
        for c in candidates {
            let at = |g: &Expr| {
                let v = subst(g, &self.x, &c);
                (!(v.is_infinite() || v.any(&|x| x.is_infinite()))).then_some(v)
            };
            if !at(f).is_some_and(|v| simplify::is_zero(&v)) {
                continue;
            }
            let (mut times, mut d) = (1, f.clone());
            while times < 8 {
                d = crate::diff::diff(&d, &self.var);
                if !at(&d).is_some_and(|v| simplify::is_zero(&v)) {
                    break;
                }
                times += 1;
            }
            // (zero with every derivative: the equation holds on a whole stretch, as
            // sign(x) + sign(x - 2) = 0 does, and such a number is no root by itself)
            if times < 8 {
                out.push((c, times));
            }
        }
        out
    }
}

/// The roots of the factors of a product: a root of several factors counts as often as they
/// say together.
fn together(roots: Vec<Root>, params: &[String]) -> Vec<Root> {
    let same = |a: &Root, b: &Root| {
        if a.value == b.value {
            return true;
        }
        if !(a.value.is_const() && b.value.is_const()) {
            return same_family(&a.value, &b.value, params);
        }
        let env = Env::new();
        if let (Some(x), Some(y)) = (approx_value(&a.value, &env), approx_value(&b.value, &env)) {
            let gap = rug::Float::with_val(64, Complex::with_val(128, &x - &y).abs_ref());
            if gap > (rug::Float::with_val(64, x.abs_ref()) + 1u32) >> 60u32 {
                return false;
            }
        }
        same_root(a, b)
    };
    let mut out: Vec<Root> = Vec::new();
    for r in roots {
        match out.iter_mut().find(|o| same(o, &r)) {
            Some(o) => o.mult += r.mult,
            None => out.push(r),
        }
    }
    out
}

fn gcd_usize(a: usize, b: usize) -> usize {
    if b == 0 { a } else { gcd_usize(b, a % b) }
}

/// Natural logarithm of a constant, with the argument of a number of modulus one in closed form.
fn log_of(c: &Expr) -> Expr {
    let plain = func("ln", vec![c.clone()]);
    if !plain.has_fn("ln") || !c.is_const() || assume::is_positive(c) {
        return plain;
    }
    if let Some((re, im)) = complex::rect(c) {
        let modulus = add(vec![pow(re, Expr::int(2)), pow(im, Expr::int(2))]);
        let angle = func("arg", vec![c.clone()]);
        if !angle.has_fn("arg") {
            let m = simplify(&modulus);
            let radial = if simplify::is_zero(&sub(m.clone(), Expr::one())) {
                Expr::zero()
            } else {
                div(func("ln", vec![m]), Expr::int(2))
            };
            return add(vec![radial, i_times(angle)]);
        }
    }
    plain
}

fn quadratic(a: &Expr, b: &Expr, c: &Expr) -> Vec<Root> {
    let disc = simplify(&sub(
        pow(b.clone(), Expr::int(2)),
        mul(vec![Expr::int(4), a.clone(), c.clone()]),
    ));
    let two_a = mul(vec![Expr::int(2), a.clone()]);
    if simplify::is_zero(&disc) {
        return vec![Root {
            value: tidy(&neg(div(b.clone(), two_a))),
            mult: 2,
            approx: false,
            skip: Vec::new(),
        }];
    }
    let s = sqrt(disc);
    [neg(s.clone()), s]
        .into_iter()
        .map(|r| exact(tidy(&div(sub(r, b.clone()), two_a.clone()))))
        .collect()
}

fn real_cbrt(e: &Expr) -> Expr {
    match numeric::sign(e) {
        Some(s) if s < 0 => neg(pow(neg(e.clone()), Expr::rat(1, 3))),
        _ => pow(e.clone(), Expr::rat(1, 3)),
    }
}

/// Roots of an irreducible cubic with rational coefficients.
fn cubic(c: &[Rat]) -> Vec<Expr> {
    let (a, b, cc, d) = (&c[3], &c[2], &c[1], &c[0]);
    let a2 = (a * a).complete();
    let p = (Rat::from(3) * a * cc - (b * b).complete()) / (Rat::from(3) * &a2);
    let q = (Rat::from(2) * b * b * b - Rat::from(9) * a * b * cc + Rat::from(27) * &a2 * d)
        / (Rat::from(27) * &a2 * a);
    let shift = Expr::rational((-b).complete() / (Rat::from(3) * a));
    let e = |r: Rat| Expr::rational(r);
    let finish = |t: Expr| tidy(&add(vec![t, shift.clone()]));
    if p.is_zero() {
        return kth_roots(&e(-q), 3).into_iter().map(finish).collect();
    }
    let disc = -(Rat::from(4) * &p * &p * &p + Rat::from(27) * &q * &q);
    if disc.is_positive() {
        let m = mul(vec![Expr::int(2), sqrt(e((-&p).complete() / Rat::from(3)))]);
        let theta = func(
            "acos",
            vec![div(
                e(Rat::from(3) * &q),
                mul(vec![e(p.clone()), m.clone()]),
            )],
        );
        return (0..3)
            .map(|k| {
                let angle = div(
                    sub(theta.clone(), mul(vec![Expr::int(2 * k), Expr::pi()])),
                    Expr::int(3),
                );
                finish(mul(vec![m.clone(), func("cos", vec![angle])]))
            })
            .collect();
    }
    let inner = sqrt(e(
        (&q * &q).complete() / Rat::from(4) + (&p * &p).complete() * &p / Rat::from(27)
    ));
    let half_q = e((-&q).complete() / Rat::from(2));
    let (u, v) = (
        real_cbrt(&add(vec![half_q.clone(), inner.clone()])),
        real_cbrt(&sub(half_q, inner)),
    );
    let (sum, diff) = (add(vec![u.clone(), v.clone()]), sub(u, v));
    let imaginary = mul(vec![Expr::i(), sqrt(Expr::int(3)), diff, Expr::half()]);
    vec![
        finish(sum.clone()),
        finish(add(vec![
            mul(vec![Expr::rat(-1, 2), sum.clone()]),
            imaginary.clone(),
        ])),
        finish(sub(mul(vec![Expr::rat(-1, 2), sum]), imaginary)),
    ]
}

/// Roots of an irreducible quartic when its resolvent cubic has a rational root.
fn quartic(c: &[Rat]) -> Option<Vec<Expr>> {
    let a = &c[4];
    let (b, cc, d, e0) = (
        (&c[3] / a).complete(),
        (&c[2] / a).complete(),
        (&c[1] / a).complete(),
        (&c[0] / a).complete(),
    );
    let b2 = (&b * &b).complete();
    let p = &cc - Rat::from((3, 8)) * &b2;
    let q = &d - (&b * &cc).complete() / Rat::from(2) + (&b2 * &b).complete() / Rat::from(8);
    let r = &e0 - (&b * &d).complete() / Rat::from(4) + (&b2 * &cc).complete() / Rat::from(16)
        - Rat::from((3, 256)) * &b2 * &b2;
    let shift = Expr::rational((-&b).complete() / Rat::from(4));
    let e = |x: Rat| Expr::rational(x);
    let finish = |t: Expr| tidy(&add(vec![t, shift.clone()]));
    let resolvent = [
        -(&q * &q).complete(),
        (&p * &p).complete() - Rat::from(4) * &r,
        Rat::from(2) * &p,
        Rat::from(1),
    ];
    let (_, factors, _) = zfactor::factor(&Poly::from_dense(1, 0, &resolvent));
    let z0 = factors.iter().find_map(|(f, _)| {
        let dense = f.to_dense(0)?;
        (dense.len() == 2)
            .then(|| (-&dense[0]).complete() / &dense[1])
            .filter(|z| !z.is_zero())
    })?;
    let s = sqrt(e(z0.clone()));
    let half = |x: Expr| div(x, Expr::int(2));
    let base = e(&p + z0);
    let q_over_s = div(e(q), s.clone());
    let mut out = Vec::new();
    for (lin, constant) in [
        (s.clone(), half(sub(base.clone(), q_over_s.clone()))),
        (neg(s), half(add(vec![base, q_over_s]))),
    ] {
        out.extend(
            quadratic(&Expr::one(), &lin, &constant)
                .into_iter()
                .map(|root| finish(root.value)),
        );
    }
    Some(out)
}

/// Roots of a cyclotomic polynomial as roots of unity.
fn cyclotomic_roots(c: &[Rat]) -> Option<Vec<Root>> {
    let degree = c.len() - 1;
    if !(c[0] == 1 || c[0] == -1) || c[degree] != 1 {
        return None;
    }
    let f = Poly::from_dense(1, 0, c);
    for n in 1..=(6 * degree as u32 + 6) {
        let mut xn = vec![Rat::new(); n as usize + 1];
        xn[0] = Rat::from(-1);
        xn[n as usize] = Rat::from(1);
        if Poly::from_dense(1, 0, &xn).div_exact(&f).is_none() {
            continue;
        }
        let roots: Vec<Root> = (1..n)
            .filter(|k| Int::from(*k).gcd(&Int::from(n)) == 1)
            .map(|k| {
                let r = rat(2 * i64::from(k), i64::from(n));
                exact(root_of_unity(&r).map_or_else(|| minus_one_pow(&r), |w| tidy(&w)))
            })
            .collect();
        return (roots.len() == degree).then_some(roots);
    }
    None
}

fn approx_value(e: &Expr, env: &Env) -> Option<Complex> {
    numeric::evalf_env(e, 128, env).ok()
}

/// Checks a candidate root in the original equation (families for several values of the parameters).
/// How many times `root` is a root of the polynomial with the coefficients `cs` (ascending).
fn multiplicity(cs: &[Expr], root: &Root) -> u32 {
    let bits = numeric::bits_for(numeric::digits());
    let mut cs = cs.to_vec();
    let mut count = 0;
    while cs.len() > 1 {
        let value = add(cs
            .iter()
            .enumerate()
            .map(|(k, c)| {
                mul(vec![
                    c.clone(),
                    pow(root.value.clone(), Expr::int(k as i64)),
                ])
            })
            .collect());
        let zero = if root.approx || value.has_float() {
            numeric::relative_size(&value, bits + 64, &Env::new())
                .is_some_and(|size| size <= 24 - i64::from(bits))
        } else {
            numeric::is_zero_const(&value) == Some(true)
        };
        if !zero {
            break;
        }
        count += 1;
        cs = (1..cs.len())
            .map(|k| mul(vec![Expr::int(k as i64), cs[k].clone()]))
            .collect();
    }
    count
}

/// The size of the equation at a root against its largest term there. Decimal numbers in the
/// equation or in the root leave their rounding, so that nothing is exactly zero.
fn rounding(f: &Expr, var: &str, value: &Expr, env: &Env) -> Option<i64> {
    let prec = numeric::bits_for(numeric::digits()) + 64;
    let at = numeric::evalf_env(value, prec, env).ok()?;
    let mut point = env.clone();
    point.insert(Rc::from(var), at);
    numeric::relative_size(f, prec, &point)
}

fn verify(f: &Expr, var: &str, root: &Root, params: &[String]) -> bool {
    let g = subst(f, &Expr::sym(var), &root.value);
    // a numeric root where the equation has no value, or where a pole stands next to a zero
    // as in (x - pi/2)*tan(x) at pi/2: the equation does not get smaller towards it
    let unsound = || {
        root.approx
            && f.free_syms().iter().all(|s| &**s == var)
            && approx_value(&root.value, &Env::new()).is_some_and(|at| {
                let point = std::slice::from_ref(&at);
                !crate::system::defined_at(
                    std::slice::from_ref(f),
                    &[var.to_string()],
                    point,
                    numeric::digits(),
                ) || shrinks(f, var, &at) == Some(false)
            })
    };
    if g.is_zero() {
        return !unsound();
    }
    if g.is_infinite() || g.any(&|x| x.is_infinite()) {
        return false;
    }
    let others: Vec<Rc<str>> = g
        .free_syms()
        .into_iter()
        .filter(|s| !params.iter().any(|p| p.as_str() == &**s))
        .collect();
    let used: Vec<&String> = params.iter().filter(|p| g.has_sym(p)).collect();
    let trials: &[i64] = if used.is_empty() {
        &[0]
    } else {
        &[-2, -1, 0, 1, 3]
    };
    let mut checked = false;
    for (round, k) in trials.iter().enumerate() {
        let mut env = numeric::sample_env(&others, round as u32, 512);
        for (j, p) in used.iter().enumerate() {
            env.insert(Rc::from(p.as_str()), Complex::with_val(512, *k + j as i64));
        }
        if root.approx || g.has_float() {
            let bits = i64::from(numeric::bits_for(numeric::digits()));
            let prec = numeric::bits_for(numeric::digits()) + 64;
            let size = match root.approx {
                true => numeric::relative_size(&g, prec, &env),
                false => rounding(f, var, &root.value, &env).or_else(|| numeric::relative_size(&g, prec, &env)),
            };
            match size {
                Some(size) if size > -bits + 24 => return false,
                Some(_) => checked = true,
                None => return false,
            }
            continue;
        }
        match numeric::is_zero_at(&g, &env) {
            Some(true) => checked = true,
            Some(false) => return false,
            None => {
                if g.is_const() || used.is_empty() {
                    return false;
                }
            }
        }
    }
    checked && !unsound()
}

/// Whether `f` gets smaller towards the point, as it does at a root of any order: its sizes at
/// two distances from the point. `None` when it has no values there.
fn shrinks(f: &Expr, var: &str, at: &Complex) -> Option<bool> {
    let prec = numeric::bits_for(numeric::digits()) + 128;
    let unit = (rug::Float::with_val(prec, at.abs_ref()) + 1u32) >> 40u32;
    let size = |times: u32| {
        // (along the real line from a real point: floor and sign are not functions of a
        // complex number)
        let step = rug::Float::with_val(prec, &unit * times);
        let aside = if at.imag().is_zero() {
            rug::Float::new(prec)
        } else {
            rug::Float::with_val(prec, &step / 2u32)
        };
        let offset = Complex::with_val(prec, (&step, aside));
        let mut env = Env::new();
        env.insert(Rc::from(var), Complex::with_val(prec, at + offset));
        let value = numeric::evalf_env(f, prec, &env).ok()?;
        let size = rug::Float::with_val(64, value.abs_ref());
        size.is_finite().then_some(size)
    };
    let (near, far) = (size(1)?, size(2)?);
    Some(far.is_zero() || near < far * 0.99f64)
}

/// The numbers that a family leaves out, as in "0", "0 and 1", "-1, 0 and 1".
pub fn skipped(skip: &[i64]) -> String {
    let names: Vec<String> = skip.iter().map(i64::to_string).collect();
    match names.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [most @ .., last] => format!("{} and {last}", most.join(", ")),
    }
}

/// The whole numbers in -12..=12 for which a family of roots gives no solution of `f = 0`.
fn left_out(f: &Expr, var: &str, root: &Root, params: &[String]) -> Vec<i64> {
    let mut used = params.iter().filter(|p| root.value.has_sym(p));
    let (Some(p), None) = (used.next(), used.next()) else {
        return Vec::new();
    };
    if root.approx || f.free_syms().iter().any(|s| &**s != var) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for n in SCAN {
        let member = subst(&root.value, &Expr::sym(p), &Expr::int(n));
        if member.is_infinite() || member.any(&|x| x.is_infinite()) {
            // (no number: 1/(n*pi) for n = 0)
            out.push(n);
            continue;
        }
        let g = subst(f, &Expr::sym(var), &member);
        if g.is_zero() {
            continue;
        }
        if g.is_infinite() || g.any(&|x| x.is_infinite()) {
            out.push(n);
            continue;
        }
        if g.has_float()
            && let Some(size) = rounding(f, var, &member, &Env::new())
        {
            if size > 24 - i64::from(numeric::bits_for(numeric::digits())) {
                out.push(n);
            }
            continue;
        }
        match numeric::is_zero_const(&g) {
            Some(true) => {}
            Some(false) => out.push(n),
            None => return Vec::new(),
        }
    }
    out
}

/// The whole numbers at which the members of a family are tried.
const SCAN: std::ops::RangeInclusive<i64> = -12..=12;

/// The numbers `m*n + r` that are left when whole classes of numbers are left out: `m` with
/// the remainders `r` that stay.
fn classes(skip: &[i64]) -> Option<(i64, Vec<i64>)> {
    (2..=12).find_map(|m| {
        let class = |r: i64| SCAN.filter(move |n| n.rem_euclid(m) == r);
        let out = |r: i64| class(r).all(|n| skip.contains(&n));
        let stays = |r: i64| class(r).all(|n| !skip.contains(&n));
        let kept: Vec<i64> = (0..m).filter(|r| stays(*r)).collect();
        ((0..m).all(|r| out(r) || stays(r)) && !kept.is_empty() && kept.len() < m as usize)
            .then_some((m, kept))
    })
}

fn sort_key(r: &Root) -> (u8, Option<(bool, f64, f64)>, String) {
    let env = Env::new();
    let numeric_key = if r.value.is_const() {
        approx_value(&r.value, &env)
            .map(|v| (!v.imag().is_zero(), v.real().to_f64(), v.imag().to_f64()))
    } else {
        None
    };
    (
        u8::from(numeric_key.is_none()),
        numeric_key,
        r.value.to_string(),
    )
}

/// Solves `f = 0` for `var`.
pub fn solve(given: &Expr, var: &str) -> Result<Solved, String> {
    // (a factored form that simplification keeps from multiplying out is solved as what it says)
    let open = crate::canon::map_bottom_up(given, &mut |x| match x.fn_arg("hold") {
        Some(inner) => inner.clone(),
        None => x,
    });
    let given = &open;
    let f = simplify::normal(given);
    if !f.has_sym(var) {
        return Ok(Solved {
            all: simplify::is_zero(&f),
            ..Solved::default()
        });
    }
    let mut used: BTreeSet<String> = f.free_syms().iter().map(|s| s.to_string()).collect();
    used.insert(var.to_string());
    let mut solver = Solver {
        var: var.to_string(),
        x: Expr::sym(var),
        params: Vec::new(),
        used,
        notes: Vec::new(),
        depth: 0,
    };
    let (num, _) = ratsimp::fraction(&f);
    let target = if num.has_sym(var) {
        num.clone()
    } else {
        f.clone()
    };
    let result = solver.solve_expr(&target);
    let mut candidates = match result {
        Ok(c) => c,
        Err(e) => {
            for p in &solver.params {
                assume::clear(p);
            }
            return Err(e);
        }
    };
    // 1/tan(u) vanishes where cos(u) does, and 1/tanh(u) where cosh(u) does
    let mut turned: Vec<(Expr, u32)> = Vec::new();
    f.walk(&mut |e| {
        if let Expr::Pow(b, k) = e
            && k.as_rat().is_some_and(|r| r.is_negative())
            && let Expr::Fn(n, a) = &**b
            && matches!(&**n, "tan" | "tanh")
            && a.len() == 1
            && a[0].has_sym(var)
        {
            let name = if &**n == "tan" { "cos" } else { "cosh" };
            let zero = func(name, vec![a[0].clone()]);
            // (as often as the power says when nothing else of the equation has this function)
            let alone = !num.any(&|part| part == &**b);
            let times = k
                .as_i64()
                .filter(|_| alone)
                .map_or(1, |k| k.unsigned_abs() as u32);
            if !turned.iter().any(|(z, _)| *z == zero) {
                turned.push((zero, times));
            }
        }
    });
    // (the poles of gamma are without end to one side: no family of the kind that is printed)
    f.walk(&mut |e| {
        if let Expr::Pow(b, k) = e
            && k.as_rat().is_some_and(|r| r.is_negative())
            && let Expr::Fn(n, a) = &**b
            && matches!(&**n, "gamma" | "factorial" | "digamma" | "polygamma")
            && a.iter().any(|x| x.has_sym(var))
        {
            solver.note(&format!(
                "1/{b} is zero at the poles of {b}, a row of whole numbers without end: these solutions are not listed"
            ));
        }
    });
    let own = candidates.len();
    for (zero, times) in &turned {
        if let Ok(more) = solver.solve_expr(zero) {
            candidates.extend(more.into_iter().map(|mut r| {
                r.mult *= times;
                r
            }));
        }
    }
    let mut roots: Vec<Root> = Vec::new();
    for (at, mut r) in candidates.into_iter().enumerate() {
        if r.value.is_infinite() || r.value.any(&|x| x.is_infinite()) {
            continue;
        }
        if !r.approx {
            r.value = tidy(&r.value);
        }
        // (where tan is infinite the equation as it was given has its value: the common
        // denominator has none there)
        if !verify(&f, var, &r, &solver.params)
            && !(at >= own && verify(given, var, &r, &solver.params))
        {
            continue;
        }
        // two families that hold the same numbers are one
        let same = |o: &Root| same_root(o, &r) || same_family(&o.value, &r.value, &solver.params);
        match roots.iter_mut().find(|o| same(o)) {
            Some(o) => o.mult = o.mult.max(r.mult),
            None => roots.push(r),
        }
    }
    // a family whose numbers all stand in another one is not listed
    let all: Vec<Expr> = roots.iter().map(|r| r.value.clone()).collect();
    let mut at = 0;
    roots.retain(|r| {
        at += 1;
        !all.iter().enumerate().any(|(other, o)| {
            other != at - 1
                && within(&r.value, o, &solver.params)
                && !(within(o, &r.value, &solver.params) && other > at - 1)
        })
    });
    // the members of a family at which the equation, as it was given, has no value are left out
    let mut kept: Vec<Root> = Vec::new();
    for r in roots {
        restrict(r, given, var, &mut solver, false, &mut kept);
    }
    Ok(finish(kept, solver))
}

/// A root as a root of `f = 0` too. A family loses the whole numbers for which `f` has no
/// value or another one: as whole classes (every second number), one by one, or - when
/// `few` and most of the numbers are lost - all but the ones that are listed then.
fn restrict(
    mut r: Root,
    f: &Expr,
    var: &str,
    solver: &mut Solver,
    few: bool,
    kept: &mut Vec<Root>,
) {
    let mut skip = left_out(f, var, &r, &solver.params);
    skip.append(&mut r.skip);
    skip.sort_unstable();
    skip.dedup();
    r.skip = skip;
    if r.skip.len() == SCAN.count() {
        return;
    }
    let Some(own) = solver.params.iter().find(|p| r.value.has_sym(p)).cloned() else {
        kept.push(r);
        return;
    };
    let member = |n: Expr| tidy(&subst(&r.value, &Expr::sym(&own), &n));
    if let Some((m, stay)) = classes(&r.skip) {
        for (at, rest) in stay.iter().enumerate() {
            let n = if at == 0 {
                Expr::sym(&own)
            } else {
                solver.param()
            };
            let value = member(add(vec![mul(vec![Expr::int(m), n]), Expr::int(*rest)]));
            let same = |o: &Root| same_family(&o.value, &value, &solver.params);
            if !kept.iter().any(same) {
                kept.push(Root {
                    value,
                    skip: Vec::new(),
                    ..r.clone()
                });
            }
        }
    } else if few && r.skip.len() > SCAN.count() / 2 {
        for n in SCAN.filter(|n| !r.skip.contains(n)) {
            let one = Root {
                value: member(Expr::int(n)),
                skip: Vec::new(),
                ..r.clone()
            };
            if !kept.iter().any(|o| same_root(o, &one)) {
                kept.push(one);
            }
        }
    } else {
        kept.push(r);
    }
}

/// The roots in their order, without the numbers that a family holds already.
fn finish(mut roots: Vec<Root>, mut solver: Solver) -> Solved {
    let families: Vec<(Expr, Expr, Vec<i64>)> = roots
        .iter()
        .filter_map(|r| affine(&r.value, &solver.params).map(|(a, b)| (a, b, r.skip.clone())))
        .collect();
    roots.retain(|r| {
        r.approx
            || !r.value.is_const()
            || !families.iter().any(|(a, b, skip)| {
                simplify(&div(sub(r.value.clone(), a.clone()), b.clone()))
                    .as_i64()
                    .is_some_and(|n| !skip.contains(&n))
            })
    });
    roots.sort_by(|a, b| {
        sort_key(a)
            .partial_cmp(&sort_key(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let params: Vec<String> = solver
        .params
        .iter()
        .filter(|p| roots.iter().any(|r| r.value.has_sym(p)))
        .cloned()
        .collect();
    if roots.iter().any(|r| r.approx) {
        solver.notes.retain(|n| !n.starts_with("no exact method"));
    } else {
        solver.notes.retain(|n| !n.starts_with("numeric search"));
    }
    for p in solver.params.iter().filter(|p| !params.contains(p)) {
        assume::clear(p);
    }
    Solved {
        roots,
        params,
        all: false,
        notes: solver.notes,
    }
}

/// Solves several equations `eqs = 0` in the one unknown `var`: the roots of one of them that
/// are roots of the others too.
pub fn common(eqs: &[Expr], var: &str) -> Result<Solved, String> {
    let normal: Vec<Expr> = eqs.iter().map(simplify::normal).collect();
    let mut failure = "no method for this system".to_string();
    for (first, f) in normal.iter().enumerate() {
        if !f.has_sym(var) {
            continue;
        }
        let solved = match solve(f, var) {
            Ok(solved) if !solved.all => solved,
            Ok(_) => continue,
            Err(e) => {
                failure = e;
                continue;
            }
        };
        let mut used: BTreeSet<String> = normal
            .iter()
            .flat_map(|e| e.free_syms())
            .map(|s| s.to_string())
            .collect();
        used.insert(var.to_string());
        used.extend(solved.params.iter().cloned());
        let mut solver = Solver {
            var: var.to_string(),
            x: Expr::sym(var),
            params: solved.params.clone(),
            used,
            notes: solved.notes.clone(),
            depth: 0,
        };
        let mut roots = solved.roots;
        for (other, g) in normal.iter().enumerate() {
            if other == first {
                continue;
            }
            let mut kept = Vec::new();
            for r in roots {
                let whole = verify(g, var, &r, &solver.params);
                if r.value.is_const() {
                    if whole {
                        kept.push(r);
                    }
                } else if whole {
                    restrict(r, &eqs[other], var, &mut solver, false, &mut kept);
                } else if !left_out(&eqs[other], var, &r, &solver.params).is_empty() {
                    // not the whole family: classes of it, or a few of its numbers
                    restrict(r, &eqs[other], var, &mut solver, true, &mut kept);
                }
            }
            roots = kept;
        }
        return Ok(finish(roots, solver));
    }
    Err(failure)
}

fn same_root(a: &Root, b: &Root) -> bool {
    if a.value == b.value {
        return true;
    }
    if !a.value.is_const() || !b.value.is_const() {
        return false;
    }
    if a.approx || b.approx {
        let env = Env::new();
        return match (approx_value(&a.value, &env), approx_value(&b.value, &env)) {
            (Some(x), Some(y)) => {
                let d = Complex::with_val(128, &x - &y);
                let scale = rug::Float::with_val(64, x.abs_ref()) + 1u32;
                rug::Float::with_val(64, d.abs_ref()) < scale >> 60u32
            }
            _ => false,
        };
    }
    simplify::is_zero(&sub(a.value.clone(), b.value.clone()))
}

/// `a + b*n` for the whole number `n` of a family, which is the one of `params` in it.
fn affine(value: &Expr, params: &[String]) -> Option<(Expr, Expr)> {
    let mut used = params.iter().filter(|p| value.has_sym(p));
    let (Some(p), None) = (used.next(), used.next()) else {
        return None;
    };
    match ratsimp::coefficients(value, p)?.as_slice() {
        [a, b] => Some((a.clone(), b.clone())),
        _ => None,
    }
}

/// Whether two families `a + b n` hold the same numbers: the steps are equal up to their sign
/// and the starts differ by a whole number of steps.
pub(crate) fn same_family(x: &Expr, y: &Expr, params: &[String]) -> bool {
    let (Some((a, b)), Some((c, d))) = (affine(x, params), affine(y, params)) else {
        return false;
    };
    let ratio = simplify(&div(d, b.clone()));
    (ratio.is_one() || ratio.is_minus_one()) && simplify(&div(sub(c, a), b)).as_int().is_some()
}

/// Whether every number of the family `x` is one of the family `y`: the step of `x` is a whole
/// number of steps of `y`, and so is the way from one start to the other.
fn within(x: &Expr, y: &Expr, params: &[String]) -> bool {
    let (Some((a, b)), Some((c, d))) = (affine(x, params), affine(y, params)) else {
        return false;
    };
    let whole = |e: Expr| simplify(&e).as_int().is_some();
    whole(div(b, d.clone())) && whole(div(sub(a, c), d))
}

/// Releases the integer assumptions of the family parameters.
pub fn release(params: &[String]) {
    for p in params {
        assume::clear(p);
    }
}
