//! Iterates of a function for any number of steps: whole, negative, fractional or complex.
//! `t = 0` is the identity, `t = 1` the function, `t = -1` its inverse, and
//! `f[s](f[t](x)) = f[s + t](x)`.

use crate::canon::{add, div, mul, neg, pow, sub, subst_sym};
use crate::expr::Expr;
use crate::funcs::func;
use crate::numeric::{self, Env};
use crate::simplify::{is_zero, simplify};
use crate::{diff, ratsimp, solve};
use rug::float::{Constant, Special};
use rug::{Complex, Float};
use std::cell::Cell;
use std::rc::Rc;

fn sym(name: &str) -> Expr {
    Expr::sym(name)
}

/// A branch of the inverse of the map `x -> f`: the one that undoes `f` near a positive number.
pub fn inverse_map(f: &Expr, x: &str) -> Option<Expr> {
    // the inverse of an inverse function is the function, whatever the range of its values
    if let Expr::Fn(name, args) = f
        && args.len() == 1
        && args[0].is_sym(x)
    {
        let back = match &**name {
            "asin" => "sin",
            "acos" => "cos",
            "atan" => "tan",
            "asinh" => "sinh",
            "acosh" => "cosh",
            "atanh" => "tanh",
            "erf" => "erfinv",
            "erfc" => "erfcinv",
            "erfinv" => "erf",
            "erfcinv" => "erfc",
            _ => "",
        };
        if !back.is_empty() {
            return Some(func(back, vec![sym(x)]));
        }
    }
    let solved = solve::solve(&sub(f.clone(), sym("#y")), x).ok()?;
    solve::release(&solved.params);
    let at = |e: &Expr, name: &str, v: &Complex| {
        let mut env = Env::new();
        env.insert(Rc::from(name), v.clone());
        numeric::evalf_env(e, 128, &env).ok()
    };
    let branches: Vec<Expr> = solved
        .roots
        .iter()
        .filter(|r| !r.approx)
        .map(|r| {
            solved
                .params
                .iter()
                .fold(r.value.clone(), |v, p| subst_sym(&v, p, &Expr::zero()))
        })
        .collect();
    for sample in [0.7f64, 0.2, 1.3, 2.9] {
        let x0 = Complex::with_val(128, (sample, 0));
        for branch in &branches {
            let back = at(f, x, &x0).and_then(|y| at(branch, "#y", &y));
            if let Some(b) = back
                && Float::with_val(64, Complex::with_val(128, &b - &x0).abs_ref()) < 1e-20
            {
                return Some(subst_sym(branch, "#y", &sym(x)));
            }
        }
    }
    branches.first().map(|b| subst_sym(b, "#y", &sym(x)))
}

/// The coefficients of the Chebyshev polynomial `T_n`.
fn chebyshev(n: usize) -> Vec<i64> {
    let (mut before, mut last) = (vec![1i64], vec![0i64, 1]);
    if n == 0 {
        return before;
    }
    for _ in 1..n {
        let mut next = vec![0i64; last.len() + 1];
        for (k, c) in last.iter().enumerate() {
            next[k + 1] += 2 * c;
        }
        for (k, c) in before.iter().enumerate() {
            next[k] -= c;
        }
        before = last;
        last = next;
    }
    last
}

/// A polynomial of the degree n >= 3 that a shift of the variable turns into a power `a y^n`
/// or into a Chebyshev polynomial `T_n(s y)/s`: these are `z -> z^n` and `u -> n u` in the right
/// variable.
fn by_degree(f: &Expr, c: &[Expr], x: &str, t: &Expr) -> Option<Expr> {
    let n = c.len() - 1;
    let shift = simplify(&neg(div(
        c[n - 1].clone(),
        mul(vec![Expr::int(n as i64), c[n].clone()]),
    )));
    let y = sym("#iy");
    let moved = sub(
        subst_sym(f, x, &add(vec![y.clone(), shift.clone()])),
        shift.clone(),
    );
    let g: Vec<Expr> = ratsimp::coefficients(&simplify(&moved), "#iy")?
        .iter()
        .map(simplify)
        .collect();
    if g.len() != n + 1 {
        return None;
    }
    let steps = pow(Expr::int(n as i64), t.clone());
    let place = sub(sym(x), shift.clone());
    let back = |w: Expr| add(vec![w, shift.clone()]);
    if g[..n].iter().all(is_zero) {
        // a y^n: z = s y with s^(n-1) = a
        let s = simplify(&pow(g[n].clone(), Expr::rat(1, n as i64 - 1)));
        return Some(back(div(pow(mul(vec![s.clone(), place]), steps), s)));
    }
    if is_zero(&g[n - 2]) {
        return None;
    }
    // T_n(s y)/s = cosh(n acosh(s y))/s, or for an odd n sinh(n asinh(r y))/r, which has the
    // same terms without their signs: s^2 and r^2 from the two highest terms
    let pattern = chebyshev(n);
    let fits = |s: &Expr, signs: bool| {
        (0..=n).all(|k| {
            let c = if signs { pattern[k] } else { pattern[k].abs() };
            let want = mul(vec![Expr::int(c), pow(s.clone(), Expr::int(k as i64 - 1))]);
            is_zero(&simplify(&sub(g[k].clone(), want)))
        })
    };
    let r2 = simplify(&div(
        mul(vec![Expr::int(n as i64), g[n].clone()]),
        mul(vec![Expr::int(4), g[n - 2].clone()]),
    ));
    if n % 2 == 1 {
        let r = simplify(&crate::canon::sqrt(r2.clone()));
        if fits(&r, false) {
            let angle = func("asinh", vec![mul(vec![r.clone(), place])]);
            return Some(back(div(func("sinh", vec![mul(vec![steps, angle])]), r)));
        }
    }
    let root = crate::canon::sqrt(neg(r2));
    for s in [simplify(&root), simplify(&neg(root.clone()))] {
        if fits(&s, true) {
            let angle = func("acosh", vec![mul(vec![s.clone(), place])]);
            return Some(back(div(func("cosh", vec![mul(vec![steps, angle])]), s)));
        }
    }
    None
}

/// The iterate in closed form for the kinds of maps that have one.
fn by_kind(f: &Expr, x: &str, t: &Expr) -> Option<Expr> {
    let xs = sym(x);
    let free = |e: &Expr| !e.has_sym(x);
    // a x + b and a x^2 + b x + c
    if let Some(c) = ratsimp::coefficients(f, x) {
        match &c[..] {
            [_] => return t.is_zero().then_some(xs),
            [b, a] => {
                if a.is_one() {
                    return Some(add(vec![xs, mul(vec![t.clone(), b.clone()])]));
                }
                let at = pow(a.clone(), t.clone());
                return Some(add(vec![
                    mul(vec![at.clone(), xs]),
                    div(
                        mul(vec![b.clone(), sub(at, Expr::one())]),
                        sub(a.clone(), Expr::one()),
                    ),
                ]));
            }
            [c0, b, a] => {
                // conjugate to z^2 + k by z = a x + b/2
                let half = div(b.clone(), Expr::int(2));
                let k = simplify(&add(vec![
                    mul(vec![a.clone(), c0.clone()]),
                    half.clone(),
                    neg(pow(half.clone(), Expr::int(2))),
                ]));
                let z = add(vec![mul(vec![a.clone(), xs]), half.clone()]);
                let doubling = pow(Expr::int(2), t.clone());
                let back = |w: Expr| div(sub(w, half.clone()), a.clone());
                if k.is_zero() {
                    return Some(back(pow(z, doubling)));
                }
                if k == Expr::int(-2) {
                    let angle = func("acosh", vec![div(z, Expr::int(2))]);
                    return Some(back(mul(vec![
                        Expr::int(2),
                        func("cosh", vec![mul(vec![doubling, angle])]),
                    ])));
                }
                return None;
            }
            _ if c.len() <= 13 => {
                if let Some(closed) = by_degree(f, &c, x, t) {
                    return Some(closed);
                }
            }
            _ => {}
        }
    }
    // c x^p
    let p = simplify(&div(mul(vec![xs.clone(), diff::diff(f, x)]), f.clone()));
    if free(&p) && !p.is_zero() {
        let c = simplify(&div(f.clone(), pow(xs.clone(), p.clone())));
        if free(&c) {
            let pt = pow(p.clone(), t.clone());
            let scale = pow(c, div(sub(pt.clone(), Expr::one()), sub(p, Expr::one())));
            return Some(mul(vec![scale, pow(xs, pt)]));
        }
    }
    // (a x + b)/(c x + d): the power of the matrix
    let (top, low) = ratsimp::fraction(&ratsimp::together(f));
    if let (Some(n), Some(d)) = (
        ratsimp::coefficients(&top, x),
        ratsimp::coefficients(&low, x),
    ) && n.len() <= 2
        && d.len() == 2
    {
        let (a, b) = (n.get(1).cloned().unwrap_or_else(Expr::zero), n[0].clone());
        let (c, d) = (d[1].clone(), d[0].clone());
        let trace = add(vec![a.clone(), d.clone()]);
        let det = sub(
            mul(vec![a.clone(), d.clone()]),
            mul(vec![b.clone(), c.clone()]),
        );
        let root = crate::canon::sqrt(simplify(&sub(
            pow(trace.clone(), Expr::int(2)),
            mul(vec![Expr::int(4), det]),
        )));
        let m1 = simplify(&div(add(vec![trace.clone(), root.clone()]), Expr::int(2)));
        let m2 = simplify(&div(sub(trace, root.clone()), Expr::int(2)));
        let (p1, p2) = (pow(m1.clone(), t.clone()), pow(m2.clone(), t.clone()));
        // M^t = u M + v I
        let (u, v) = if is_zero(&root) {
            let u = mul(vec![
                t.clone(),
                pow(m1.clone(), sub(t.clone(), Expr::one())),
            ]);
            (u.clone(), sub(p1, mul(vec![u, m1])))
        } else {
            let gap = sub(m1.clone(), m2.clone());
            (
                div(sub(p1.clone(), p2.clone()), gap.clone()),
                div(sub(mul(vec![m1, p2]), mul(vec![m2, p1])), gap),
            )
        };
        let entry = |m: &Expr, diagonal: bool| {
            let scaled = mul(vec![u.clone(), m.clone()]);
            if diagonal {
                add(vec![scaled, v.clone()])
            } else {
                scaled
            }
        };
        return Some(div(
            add(vec![
                mul(vec![entry(&a, true), xs.clone()]),
                entry(&b, false),
            ]),
            add(vec![mul(vec![entry(&c, false), xs]), entry(&d, true)]),
        ));
    }
    // C B^x: tetration to the base B^C in the variable x/C
    let rate = simplify(&div(diff::diff(f, x), f.clone()));
    if free(&rate) && !rate.is_zero() {
        // f'/f is a constant: f is its value at 0 times e^(rate x)
        let c = simplify(&subst_sym(f, x, &Expr::zero()));
        if free(&c) && !c.is_zero() {
            let base = simplify(&pow(Expr::e(), mul(vec![rate, c.clone()])));
            let height = add(vec![
                func("slog", vec![base.clone(), div(xs, c.clone())]),
                t.clone(),
            ]);
            return Some(mul(vec![c, func("tet", vec![base, height])]));
        }
    }
    None
}

/// The `t`-th iterate of the map `x -> f` as an expression in `x`, when it has a closed form.
pub fn symbolic(f: &Expr, x: &str, t: &Expr) -> Option<Expr> {
    let f = simplify(f);
    let xs = sym(x);
    if t.is_zero() {
        return Some(xs);
    }
    if t.is_one() {
        return Some(f);
    }
    // tetration is left as it is: its values come from the external program
    let tidy = |e: Expr| if e.has_fn("tet") { e } else { simplify(&e) };
    let mut inverse = None;
    let mut closed = by_kind(&f, x, t).map(tidy);
    if closed.is_none() {
        inverse = inverse_map(&f, x);
        if let Some(g) = &inverse {
            closed = by_kind(&simplify(g), x, &neg(t.clone())).map(tidy);
        }
    }
    // a whole number of steps: the map or its inverse again and again, tidied up as long as
    // that takes no time
    let composed = t.as_i64().filter(|n| n.abs() <= 64).and_then(|n| {
        let step = if n > 0 {
            f.clone()
        } else {
            simplify(&inverse.clone().or_else(|| inverse_map(&f, x))?)
        };
        crate::budget::with_limit(std::time::Duration::from_millis(1500), || {
            let mut out = xs.clone();
            for _ in 0..n.unsigned_abs() {
                out = simplify(&subst_sym(&step, x, &out));
                if out.size() > 3000 {
                    return None;
                }
            }
            Some(out)
        })
    });
    match (closed, composed) {
        (Some(a), Some(b)) => Some(if b.to_string().len() <= a.to_string().len() {
            b
        } else {
            a
        }),
        (a, b) => a.or(b),
    }
}

/// The map, or the formula for its inverse, applied a whole count of times to a number: as it
/// is while the numbers stay short, and then with the digits asked for. Nothing when there is
/// no formula for the inverse, or the count is out of reach.
pub fn whole_steps(
    f: &Expr,
    x: &str,
    n: i64,
    point: &Expr,
    digits: u32,
) -> Option<Result<Expr, String>> {
    let count = n.unsigned_abs();
    if count > 1_000_000 {
        return None;
    }
    // steps back with a closed form are its branch, as the steps that are not whole are
    if n < 0 && by_kind(&simplify(f), x, &Expr::int(n)).is_some() {
        return None;
    }
    let step = if n >= 0 {
        simplify(f)
    } else {
        simplify(&inverse_map(f, x)?)
    };
    let mut exact = point.clone();
    let mut done = 0;
    // (numbers with a decimal point are not exact: their steps are counted with bits to spare)
    let inexact = |e: &Expr| e.any(&|part| matches!(part, Expr::Float(_)));
    let limit = if inexact(point) || inexact(&step) {
        0
    } else {
        count
    };
    crate::budget::with_limit(std::time::Duration::from_millis(500), || {
        while done < limit {
            let next = simplify(&subst_sym(&step, x, &exact));
            if next.size() > 60 || next.to_string().len() > 240 || crate::budget::expired() {
                break;
            }
            exact = next;
            done += 1;
        }
    });
    if done == count {
        return Some(Ok(exact));
    }
    // the rest with numbers, and with the bits that the steps lose: twice, with more bits the
    // second time, until the two agree to the digits asked for
    let too_large =
        || "the iterate is too large for a number, or the map has no value on the way".to_string();
    let run = |prec: u32| -> Option<Complex> {
        let map = Map::new(&step, x, prec);
        let mut z = numeric::evalf(&exact, prec).ok()?;
        for _ in done..count {
            z = map.value(&z)?;
        }
        Some(z)
    };
    let wanted = numeric::bits_for(digits);
    let mut prec = wanted + 64;
    let Some(mut rough) = run(prec) else {
        return Some(Err(too_large()));
    };
    loop {
        prec = prec + prec / 2 + 64;
        let Some(fine) = run(prec) else {
            return Some(Err(too_large()));
        };
        let gap = Complex::with_val(prec, &fine - &rough);
        let agree = exponent(&gap)
            .zip(exponent(&fine))
            .is_none_or(|(g, size)| g < size - wanted as i32 - 8);
        if agree || prec > 2_000_000 {
            return Some(if agree {
                Ok(numeric::to_float_expr(
                    Complex::with_val(wanted + 32, fine),
                    digits,
                ))
            } else {
                Err("the steps lose more digits than there are to work with".into())
            });
        }
        rough = fine;
    }
}

/// A map of the complex plane with its derivative.
trait Flow {
    fn prec(&self) -> u32;
    fn value(&self, z: &Complex) -> Option<Complex>;
    fn slope(&self, z: &Complex) -> Option<Complex>;
    /// The same map with more bits, when it can be had.
    fn sharper(&self, _prec: u32) -> Option<Box<dyn Flow + '_>> {
        None
    }
    /// Whether this is the inverse of the map that is iterated.
    fn is_inverse(&self) -> bool {
        false
    }
    /// A step back by the branch of the inverse that keeps the fixed point `p`, where the map
    /// has the slope `lambda`: the point that the map takes to `y`, followed from the fixed
    /// point as `y` comes from there on a line. Nothing when the line runs into a point that
    /// the map takes with the slope 0, or the branch ends. `hint` is a number next to the
    /// point, when one is known.
    fn unstep(
        &self,
        _y: &Complex,
        _p: &Complex,
        _lambda: &Complex,
        _hint: Option<&Complex>,
    ) -> Option<Complex> {
        None
    }
    /// The logarithm of the slope at a fixed point that the powers of the slope are taken
    /// with: the principal one of the slope of the map that is iterated.
    fn log_slope(&self, lambda: &Complex) -> Complex {
        let prec = self.prec();
        let clean = |z: Complex| numeric::unsigned_zero(z);
        if self.is_inverse() {
            -clean(Complex::with_val(prec, lambda.recip_ref())).ln()
        } else {
            clean(lambda.clone()).ln()
        }
    }
}

/// The map `x -> f` of an expression.
struct Map<'a> {
    f: &'a Expr,
    df: Expr,
    /// The second derivative, when it is asked for.
    ddf: std::cell::OnceCell<Expr>,
    key: Rc<str>,
    prec: u32,
}

impl<'a> Map<'a> {
    fn new(f: &'a Expr, x: &str, prec: u32) -> Self {
        Map {
            f,
            df: diff::diff(f, x),
            ddf: std::cell::OnceCell::new(),
            key: Rc::from(x),
            prec,
        }
    }
    /// The same map with another count of bits.
    fn with_prec(&self, prec: u32) -> Map<'a> {
        Map {
            f: self.f,
            df: self.df.clone(),
            ddf: self.ddf.clone(),
            key: self.key.clone(),
            prec,
        }
    }
    fn at(&self, e: &Expr, z: &Complex) -> Option<Complex> {
        self.at_bits(e, z, self.prec)
    }
    fn at_bits(&self, e: &Expr, z: &Complex, bits: u32) -> Option<Complex> {
        let mut env = Env::new();
        env.insert(self.key.clone(), z.clone());
        numeric::evalf_env(e, bits, &env)
            .ok()
            .filter(|v| v.real().is_finite() && v.imag().is_finite())
    }
    /// `u` with `f(u) = y`, from the guess: Newton's method, each step with the bits that it
    /// can make good.
    fn solve(&self, y: &Complex, guess: &Complex) -> Option<Complex> {
        let full = self.prec;
        let tiny = -(full as i32) + 24;
        let far = 1e30 * (1.0 + abs(guess) + abs(y));
        let unit = exponent(y).unwrap_or(0).max(0);
        let mut u = guess.clone();
        let mut work = 64u32.min(full);
        for _ in 0..200 {
            if reaches(abs(&u), far) {
                return None;
            }
            let here = Complex::with_val(work, &u);
            let gap = Complex::with_val(work, self.at_bits(self.f, &here, work)? - y);
            if exponent(&gap).is_none_or(|e| e - unit < tiny) {
                if work >= full {
                    return Some(u);
                }
                work = full;
                continue;
            }
            let d = self.at_bits(&self.df, &here, work)?;
            if d.is_zero() {
                return None;
            }
            let step = gap / d;
            u = Complex::with_val(full, &u - &step);
            // the bits that were right before the step: twice as many after it, and no more
            // than the step was done with
            let size = exponent(&step).map_or(i32::MIN, |e| e - exponent(&u).unwrap_or(0).max(0));
            if work >= full && size < tiny / 2 - 16 {
                return Some(u);
            }
            let right = (-size).clamp(0, full as i32) as u32;
            let after = (2 * right).min(work.saturating_sub(16));
            work = (2 * after + 32).clamp(64, full);
        }
        None
    }
    /// A step back: the solution of `f(u) = y` next to the point `z`, where `f(z) = image`.
    /// The guess is the root of the parabola that the map is at `z`: the one that goes to `z`
    /// with `y - image`, and at a point where the slope is 0 the one with the principal root.
    /// Nothing when the solution is not where the parabola has it: the step is too long to
    /// tell the branches of the inverse apart.
    fn step_back(&self, y: &Complex, z: &Complex, image: &Complex) -> Option<Complex> {
        let prec = self.prec;
        let rise = Complex::with_val(prec, y - image);
        if rise.is_zero() {
            return Some(z.clone());
        }
        let slope = self.slope(z)?;
        let bend = self
            .at(self.ddf.get_or_init(|| diff::diff(&self.df, &self.key)), z)
            .unwrap_or_else(|| Complex::new(prec));
        // slope d + bend d^2/2 = rise
        let inside = Complex::with_val(prec, slope.square_ref())
            + Complex::with_val(prec, &bend * &rise) * 2u32;
        let mut root = numeric::unsigned_zero(inside).sqrt();
        if !slope.is_zero()
            && Complex::with_val(prec, &root * Complex::with_val(prec, slope.conj_ref()))
                .real()
                .is_sign_negative()
        {
            root = -root;
        }
        let low = Complex::with_val(prec, &slope + &root);
        let guess = if low.is_zero() {
            // the map is flat to a higher order here, c d^m: the principal root again
            let radius = Float::with_val(prec, 1e-3);
            let flat = taylor(self, z, &radius, 12).and_then(|a| {
                let r = 1e-3f64.log2();
                let size = |k: usize, c: &Complex| {
                    exponent(c).map(|e| f64::from(e) + (k as f64 + 1.0) * r)
                };
                let top = a
                    .iter()
                    .enumerate()
                    .filter_map(|(k, c)| size(k, c))
                    .fold(f64::NEG_INFINITY, f64::max);
                a.iter()
                    .enumerate()
                    .find(|(k, c)| size(*k, c).is_some_and(|s| s > top - f64::from(prec) + 48.0))
                    .map(|(k, c)| (k as u32 + 1, c.clone()))
            });
            match flat {
                Some((m, c)) if m >= 2 => {
                    let ratio = numeric::unsigned_zero(Complex::with_val(prec, &rise / &c));
                    let root = (ratio.ln() / m).exp();
                    Complex::with_val(prec, &root + z)
                }
                _ => z.clone(),
            }
        } else {
            Complex::with_val(prec, &rise / &low) * 2u32 + z
        };
        let u = self.solve(y, &guess)?;
        let moved = abs(&Complex::with_val(prec, &guess - z));
        let off = abs(&Complex::with_val(prec, &u - &guess));
        (off <= 0.5 * moved + 1e-30 * (1.0 + abs(z))).then_some(u)
    }
}

impl Flow for Map<'_> {
    fn prec(&self) -> u32 {
        self.prec
    }
    fn value(&self, z: &Complex) -> Option<Complex> {
        self.at(self.f, z)
    }
    fn slope(&self, z: &Complex) -> Option<Complex> {
        self.at(&self.df, z)
    }
    fn sharper(&self, prec: u32) -> Option<Box<dyn Flow + '_>> {
        Some(Box::new(self.with_prec(prec)))
    }
    fn unstep(
        &self,
        y: &Complex,
        p: &Complex,
        lambda: &Complex,
        hint: Option<&Complex>,
    ) -> Option<Complex> {
        let prec = self.prec;
        let line = Complex::with_val(prec, y - p);
        if line.is_zero() {
            return Some(p.clone());
        }
        let mut u = p.clone();
        let mut slope = lambda.clone();
        let (mut s, mut step) = (0.0f64, 1.0f64);
        loop {
            let next = (s + step).min(1.0);
            let target = if next >= 1.0 {
                y.clone()
            } else {
                Complex::with_val(prec, &line * next) + p
            };
            // where the line of the map has it
            let ask = Complex::with_val(prec, &line * (next - s)) / &slope;
            let guess = Complex::with_val(prec, &u + &ask);
            // the hint is for the whole way in one step
            let from = hint.filter(|_| s == 0.0 && next >= 1.0).unwrap_or(&guess);
            let found = self
                .solve(&target, from)
                .filter(|r| abs(&Complex::with_val(prec, r - &guess)) <= 0.5 * abs(&ask));
            match found {
                Some(r) => {
                    if next >= 1.0 {
                        return Some(r);
                    }
                    slope = self.slope(&r).filter(|d| !d.is_zero())?;
                    (u, s) = (r, next);
                    step *= 2.0;
                }
                None => {
                    step /= 2.0;
                    if step < 1e-9 {
                        return None;
                    }
                }
            }
        }
    }
}

/// A branch of the inverse of a map by Newton's method: from the value of a formula for the
/// inverse, which may have lost digits, or without one from the line through a fixed point
/// `q` where the map has the slope `lambda`.
struct Local<'a> {
    of: Map<'a>,
    branch: Option<Map<'a>>,
    at: Option<(Complex, Complex)>,
}

impl Flow for Local<'_> {
    fn prec(&self) -> u32 {
        self.of.prec
    }
    fn value(&self, z: &Complex) -> Option<Complex> {
        match (&self.branch, &self.at) {
            (Some(formula), _) => self.of.solve(z, &formula.value(z)?),
            (None, Some((q, lambda))) => self.of.unstep(z, q, lambda, None),
            (None, None) => None,
        }
    }
    fn slope(&self, z: &Complex) -> Option<Complex> {
        let d = self.of.slope(&self.value(z)?)?;
        (!d.is_zero()).then(|| numeric::unsigned_zero(d.recip()))
    }
    fn is_inverse(&self) -> bool {
        true
    }
    fn sharper(&self, prec: u32) -> Option<Box<dyn Flow + '_>> {
        Some(Box::new(Local {
            of: self.of.with_prec(prec),
            branch: self.branch.as_ref().map(|b| b.with_prec(prec)),
            at: self.at.clone(),
        }))
    }
}

fn abs(z: &Complex) -> f64 {
    Float::with_val(64, z.abs_ref()).to_f64()
}

/// Whether `x < bound` fails: also for an `x` that is no number.
fn reaches(x: f64, bound: f64) -> bool {
    x.is_nan() || x >= bound
}

/// A number without the noise of the last bits in a part that is 0 next to the other, or next
/// to 1 for a number that is smaller.
fn settled(z: Complex) -> Complex {
    let prec = z.prec().0;
    let floor = exponent(&z).unwrap_or(0).max(0) - prec as i32 + 48;
    let (mut re, mut im) = z.into_real_imag();
    for part in [&mut re, &mut im] {
        if part.get_exp().is_some_and(|e| e < floor) {
            *part = Float::new(prec);
        }
    }
    Complex::with_val(prec, (re, im))
}

/// The binary exponent of the size of a number: none for 0.
fn exponent(z: &Complex) -> Option<i32> {
    Float::with_val(64, z.abs_ref()).get_exp()
}

/// Whether a number is 0 to the given count of decimal digits.
fn negligible(z: &Complex, digits: f64) -> bool {
    exponent(z).is_none_or(|e| f64::from(e) < -digits * std::f64::consts::LOG2_10)
}

/// The first row of `C^t` for the Carleman matrix `C` of a series `a_1 h + a_2 h^2 + ...`:
/// the coefficients of the `t`-th iterate. `log` is the logarithm of `a_1` that its powers are
/// taken with.
fn iterate_series(a: &[Complex], t: &Complex, log: &Complex, prec: u32) -> Option<Vec<Complex>> {
    let n = a.len();
    let zero = Complex::new(prec);
    // rows: the powers of the series
    let mut rows: Vec<Vec<Complex>> = vec![a.to_vec()];
    for j in 1..n {
        let mut next = vec![zero.clone(); n];
        for (p, x) in rows[j - 1].iter().enumerate() {
            if x.is_zero() {
                continue;
            }
            for (q, y) in a.iter().enumerate() {
                if p + q + 1 < n {
                    next[p + q + 1] += Complex::with_val(prec, x * y);
                }
            }
        }
        rows.push(next);
    }
    let lambda = a[0].clone();
    let one = Complex::with_val(prec, 1);
    if abs(&Complex::with_val(prec, &lambda - &one)) < 1e-25 {
        // unipotent: the binomial series of (I + U)^t
        let mut row = vec![zero.clone(); n];
        row[0] = one.clone();
        let mut out = row.clone();
        let mut weight = one.clone();
        for m in 1..n {
            let mut next = vec![zero.clone(); n];
            for (k, v) in row.iter().enumerate() {
                if v.is_zero() {
                    continue;
                }
                for (c, x) in rows[k].iter().enumerate().skip(k + 1) {
                    next[c] += Complex::with_val(prec, v * x);
                }
            }
            row = next;
            weight = weight * Complex::with_val(prec, t - (m as u32 - 1)) / m as u32;
            for (o, v) in out.iter_mut().zip(&row) {
                *o += Complex::with_val(prec, &weight * v);
            }
        }
        return Some(out);
    }
    // distinct diagonal lambda^j: Parlett's recurrence for the function z^t
    let diagonal: Vec<Complex> = (1..=n)
        .map(|j| (Complex::with_val(prec, t * log) * j as u32).exp())
        .collect();
    let mut f: Vec<Vec<Complex>> = vec![vec![zero.clone(); n]; n];
    for j in 0..n {
        f[j][j] = diagonal[j].clone();
        for i in (0..j).rev() {
            let gap = Complex::with_val(prec, &rows[i][i] - &rows[j][j]);
            let size = exponent(&rows[i][i]).max(exponent(&rows[j][j]));
            if exponent(&gap).is_none_or(|e| e < size.unwrap_or(0) - 100) {
                return None;
            }
            let mut sum = Complex::with_val(
                prec,
                &rows[i][j] * Complex::with_val(prec, &f[i][i] - &f[j][j]),
            );
            for k in i + 1..j {
                sum += Complex::with_val(prec, &f[i][k] * &rows[k][j]);
                sum -= Complex::with_val(prec, &rows[i][k] * &f[k][j]);
            }
            f[i][j] = sum / gap;
        }
    }
    Some(f.swap_remove(0))
}

/// Taylor coefficients `a_1..a_n` of the map at `p`, from its values on a circle: the mean of
/// `f(z) (z - p)^-k` over as many points as it takes for twice as many to give the same. Few
/// points mix the high terms of the series into the low ones.
fn taylor(map: &dyn Flow, p: &Complex, radius: &Float, n: usize) -> Option<Vec<Complex>> {
    let prec = map.prec();
    let two_pi = Float::with_val(prec, Constant::Pi) * 2u32;
    // the value at the point m of `count` on the circle, with its direction
    let sample = |m: usize, count: usize| -> Option<(Complex, Complex)> {
        let angle = Float::with_val(prec, &two_pi * m as u32) / count as u32;
        let unit = Complex::with_val(prec, (angle.clone().cos(), angle.sin()));
        let z = Complex::with_val(prec, &unit * radius) + p;
        Some((Complex::with_val(prec, map.value(&z)? - p), unit))
    };
    // the terms a_k radius^k from the points
    let terms = |points: &[(Complex, Complex)]| -> Vec<Complex> {
        let count = points.len();
        (1..=n)
            .map(|k| {
                let mut sum = Complex::new(prec);
                for (m, (v, _)) in points.iter().enumerate() {
                    // unit^(-k m)
                    let turn = &points[(count - (k * m) % count) % count].1;
                    sum += Complex::with_val(prec, v * turn);
                }
                sum / count as u32
            })
            .collect()
    };
    let mut count = (2 * n + 8).next_power_of_two();
    let mut points = Vec::with_capacity(count);
    for m in 0..count {
        points.push(sample(m, count)?);
    }
    let mut last = terms(&points);
    loop {
        // the points between the points
        let mut more = Vec::with_capacity(2 * count);
        for (m, point) in points.into_iter().enumerate() {
            more.push(point);
            more.push(sample(2 * m + 1, 2 * count)?);
        }
        points = more;
        count *= 2;
        let next = terms(&points);
        let top = next.iter().filter_map(exponent).max().unwrap_or(0);
        let same = last.iter().zip(&next).all(|(a, b)| {
            exponent(&Complex::with_val(prec, a - b)).is_none_or(|e| e < top - prec as i32 + 48)
        });
        last = next;
        if same {
            break;
        }
        if count > 16384 {
            return None;
        }
    }
    let mut scale = Float::with_val(prec, 1);
    Some(
        last.into_iter()
            .map(|c| {
                scale *= radius;
                c / &scale
            })
            .collect(),
    )
}

/// The value of the series `a_1 h + a_2 h^2 + ...`.
fn series_value(a: &[Complex], h: &Complex, prec: u32) -> Complex {
    let mut sum = Complex::new(prec);
    for c in a.iter().rev() {
        sum += c;
        sum *= h;
    }
    sum
}

/// Whether coefficients taken on a circle are those of a power series that holds in its disk:
/// the terms on the circle fall off, and the series gives the map inside.
fn holds(map: &dyn Flow, p: &Complex, a: &[Complex], radius: &Float) -> bool {
    let prec = map.prec();
    let r = radius.to_f64().log2();
    let on_circle =
        |k: usize, c: &Complex| exponent(c).map(|e| f64::from(e) + (k as f64 + 1.0) * r);
    let sizes: Vec<f64> = a
        .iter()
        .enumerate()
        .map(|(k, c)| on_circle(k, c).unwrap_or(f64::NEG_INFINITY))
        .collect();
    let top = sizes.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if sizes.len() >= 12 && sizes[sizes.len() - 3..].iter().any(|s| *s > top - 10.0) {
        return false;
    }
    for turn in [1.0f64, 3.0, 5.0] {
        let h = Complex::with_val(prec, (turn.cos(), turn.sin()))
            * Float::with_val(prec, radius / 4u32);
        let z = Complex::with_val(prec, p + &h);
        let Some(value) = map.value(&z) else {
            return false;
        };
        let gap = Complex::with_val(prec, value - p) - series_value(a, &h, prec);
        // the terms left out: a quarter of the radius to the power of their count
        let allowed = top - 2.0 * (a.len() as f64) + 12.0;
        if exponent(&gap).is_some_and(|e| f64::from(e) > allowed.max(top - f64::from(prec) + 40.0))
        {
            return false;
        }
    }
    true
}

/// The first term `a (z - p)^m` of the map at its fixed point `p`, when the map has a power
/// series there.
fn first_term(map: &dyn Flow, p: &Complex, terms: usize) -> Option<(u32, Complex)> {
    let prec = map.prec();
    let radius = Float::with_val(prec, 1e-3);
    let a = taylor(map, p, &radius, terms)?;
    if !holds(map, p, &a, &radius) {
        return None;
    }
    let r = 1e-3f64.log2();
    let on_circle =
        |k: usize, c: &Complex| exponent(c).map(|e| f64::from(e) + (k as f64 + 1.0) * r);
    let top = a
        .iter()
        .enumerate()
        .filter_map(|(k, c)| on_circle(k, c))
        .fold(f64::NEG_INFINITY, f64::max);
    let floor = top - f64::from(prec) + 48.0;
    a.iter()
        .enumerate()
        .find(|(k, c)| on_circle(*k, c).is_some_and(|s| s > floor))
        .map(|(k, c)| (k as u32 + 1, c.clone()))
}

/// The fixed point next to a number. A tangent one is a double root of `f(z) = z`, which
/// Newton's method finds to half of the digits: with twice the digits then.
fn polish(map: &dyn Flow, from: &Complex) -> Option<Complex> {
    let prec = map.prec();
    let mut p = newton_fixed(map, from)?;
    let slope = map.slope(&p)?;
    if abs(&Complex::with_val(prec, &slope - 1u32)) < 1e-6
        && let Some(sharp) = map.sharper(2 * prec + 32)
        && let Some(better) = newton_fixed(&*sharp, &Complex::with_val(sharp.prec(), &p))
    {
        p = settled(Complex::with_val(prec, better));
    }
    // a short number next to it that the map keeps is the fixed point itself, without the
    // noise that Newton's method leaves in a root of a higher order
    let short = |x: &Float| (Float::with_val(prec, x) << 40u32).round() >> 40u32;
    let snapped = Complex::with_val(prec, (short(p.real()), short(p.imag())));
    if abs(&Complex::with_val(prec, &snapped - &p)) < 1e-20 * (1.0 + abs(&p))
        && map.value(&snapped).is_some_and(|v| v == snapped)
    {
        return Some(snapped);
    }
    Some(p)
}

/// Newton's method for `f(z) = z`, with the double root of a tangent fixed point in mind.
fn newton_fixed(map: &dyn Flow, from: &Complex) -> Option<Complex> {
    let prec = map.prec();
    let mut p = from.clone();
    // the last step: one that is not small has not come to a point, as where the map only
    // comes close to the diagonal far away
    let mut last = 0.0f64;
    for _ in 0..300 {
        let gap = Complex::with_val(prec, map.value(&p)? - &p);
        let slope = Complex::with_val(prec, map.slope(&p)? - 1u32);
        if gap.is_zero() {
            // a point that a long step has come to and that the map keeps to all digits:
            // a fixed point when the map does not keep the points around it as well
            if last >= 1e-8 {
                let beside =
                    Complex::with_val(prec, &p + Float::with_val(prec, 1e-6 * (1.0 + abs(&p))));
                if map
                    .value(&beside)
                    .is_none_or(|v| Complex::with_val(prec, v - &beside).is_zero())
                {
                    return None;
                }
                last = 0.0;
            }
            break;
        }
        if slope.is_zero() {
            break;
        }
        let factor = if abs(&slope) < 1e-3 { 2u32 } else { 1 };
        let move_by = Complex::with_val(prec, &gap / &slope) * factor;
        p -= &move_by;
        if reaches(abs(&p), 1e60) {
            return None;
        }
        last = abs(&move_by) / (1.0 + abs(&p));
        if move_by.is_zero()
            || exponent(&move_by).unwrap_or(0) < exponent(&p).unwrap_or(0).max(0) - prec as i32 + 8
        {
            break;
        }
    }
    let left = abs(&Complex::with_val(prec, map.value(&p)? - &p));
    (left < 1e-30 * (1.0 + abs(&p)) && last < 1e-8).then(|| settled(p))
}

/// The fixed point the orbit of `start` under the map runs into (`known`, when it is given),
/// with the orbit up to a point close to it.
fn run_in(
    map: &dyn Flow,
    start: &Complex,
    near: f64,
    known: Option<&Complex>,
) -> Option<(Complex, Vec<Complex>)> {
    let prec = map.prec();
    let mut orbit = vec![start.clone()];
    let mut steps: Vec<f64> = Vec::new();
    let mut fixed: Option<Complex> = known.cloned();
    if let Some(p) = &fixed
        && abs(&Complex::with_val(prec, start - p)) <= near
    {
        return Some((p.clone(), orbit));
    }
    // when the next look for a fixed point is due: twice as late after each one in vain, and
    // after ten of them the orbit creeps on to no point
    let (mut look, mut looks) = (8usize, 0);
    for k in 0..400_000usize {
        let z = orbit.last()?.clone();
        let next = map.value(&z)?;
        if abs(&next) > 1e60 {
            return None;
        }
        let step = abs(&Complex::with_val(prec, &next - &z));
        steps.push(step);
        orbit.push(next.clone());
        if fixed.is_none() && step < 1e-3 * (1.0 + abs(&next)) && k >= look && step <= steps[k - 8]
        {
            match polish(map, &next) {
                Some(p) if abs(&Complex::with_val(prec, &p - &next)) < 0.5 => fixed = Some(p),
                _ => {
                    look = (2 * k).max(look + 8);
                    looks += 1;
                    if looks >= 10 {
                        return None;
                    }
                }
            }
        }
        if let Some(p) = &fixed
            && abs(&Complex::with_val(prec, &next - p)) <= near
        {
            return Some((p.clone(), orbit));
        }
        if k > 400 && step > 1e-2 && (fixed.is_none() || known.is_some()) {
            return None;
        }
        if known.is_some() && k > 100_000 {
            return None;
        }
    }
    None
}

const NO_POINT: &str = "the orbit runs into no fixed point: no regular iteration";
const PAST: &str = "Boettcher's coordinate has more than one value past it";
const BRANCH: &str = "the iterate would be past a point where the slope of the map is 0, as seen from the fixed point: the branch of the inverse of the map that keeps the fixed point does not go that far";
const EDGE: &str = "the iterate would be past the edge of the disk around the fixed point where the map is a rotation, or too close to it for the digits asked for: a count of steps that is not real moves the point across the circles of the rotation";

/// Where an iterate was built: the fixed point, the slope of the map there and, for the slope
/// 0, the degree of the first term of the map.
struct Site {
    point: Complex,
    slope: Complex,
    degree: u32,
    /// The point that the iterate was taken at in the place of the start, when the start is
    /// past a fold of the map: no number when it is not known.
    stand_in: Option<Complex>,
    /// Whether further iterates of the value go on with this one: for the inverse of a map,
    /// whether that inverse takes the value back the way that it came.
    carried: bool,
    /// Whether the orbit went far away from the fixed point before it came in.
    wandered: bool,
}

/// A number in few digits, for a note.
fn brief(z: &Complex) -> String {
    let size = exponent(z).unwrap_or(0);
    let part = |x: &Float| {
        if x.get_exp().is_none_or(|e| e < size - 66) {
            Float::new(64)
        } else {
            Float::with_val(64, x)
        }
    };
    let short = Complex::with_val(64, (part(z.real()), part(z.imag())));
    numeric::to_float_expr(short, 10).to_string()
}

/// What an orbit carries on of the noise in its start, which is known to `bits` bits: the
/// first point that is the fixed point `p` up to that noise, when the map takes the start to
/// the fixed point in so many steps, and the most bits that a point before it has lost of its
/// distance from the fixed point.
fn carried(map: &dyn Flow, p: &Complex, orbit: &[Complex], bits: u32) -> (Option<usize>, i32) {
    let prec = map.prec();
    let mut noise = exponent(&orbit[0]).map(|e| e - bits as i32);
    let mut lost = 0;
    for k in 1..orbit.len() {
        // with the slope 0 the noise comes in to the second order
        let slope = map.slope(&orbit[k - 1]).and_then(|d| exponent(&d));
        noise = noise
            .map(|n| slope.map_or(2 * n, |s| (n + s).max(2 * n)))
            .max(exponent(&orbit[k]).map(|e| e - prec as i32));
        let Some(e) = exponent(&Complex::with_val(prec, &orbit[k] - p)) else {
            return (Some(k), lost);
        };
        if let Some(n) = noise {
            if e <= n + 24 {
                return (Some(k), lost);
            }
            lost = lost.max(n + bits as i32 - e);
        }
    }
    (None, lost)
}

/// Whether the orbit of a point next to the fixed point `p` is drawn into it: an orbit that only
/// passes by is not. One that does not move in the digits at hand is as good as drawn in.
fn approaches(map: &dyn Flow, p: &Complex, from: &Complex) -> bool {
    let prec = map.prec();
    let far = |z: &Complex| Float::with_val(prec, Complex::with_val(prec, z - p).abs_ref());
    let first = far(from);
    if first.is_zero() {
        return true;
    }
    let mut z = from.clone();
    for _ in 0..16 {
        match map.value(&z) {
            Some(next) if far(&next) <= Float::with_val(prec, &first * 4u32) => z = next,
            _ => return false,
        }
    }
    far(&z) <= first
}

/// The value at `start` of the `t`-th iterate of a map, by regular iteration at the fixed point
/// its orbit runs into (`known`, when it is given): the iterate next to the fixed point, where
/// it is a power series, at a point of the orbit, and from there back by as many steps. For the
/// inverse of a map `of` a step back is that map. Else it is the branch of the inverse that
/// keeps the fixed point: the iterate is the one that goes on from the fixed point, whatever
/// the side of a fold of the map the point itself is on.
fn regular(
    map: &dyn Flow,
    of: Option<&dyn Flow>,
    t: &Complex,
    start: &Complex,
    digits: u32,
    known: Option<&Complex>,
    want: &Cell<u32>,
) -> Result<(Complex, Site), String> {
    let prec = map.prec();
    let no_point = || NO_POINT.to_string();
    // the bits to spare, and a word for more of them when so many are lost on the way
    let spare = prec as i32 - numeric::bits_for(digits) as i32 - 64;
    let more = |lost: i32| {
        if lost > spare {
            want.set(want.get().max((lost - spare) as u32 + 32));
        }
    };
    let (p, way) = run_in(map, start, 1e-2, known).ok_or_else(no_point)?;
    let lambda = settled(map.slope(&p).ok_or_else(no_point)?);
    let size = abs(&lambda);
    let from = abs(&Complex::with_val(prec, start - &p));
    let wandered = way.iter().any(|z| {
        let far = abs(&Complex::with_val(prec, z - &p));
        far.is_nan() || far > 4.0 * from
    });
    let site = |degree: u32, stand_in: Option<Complex>| Site {
        point: p.clone(),
        slope: lambda.clone(),
        degree,
        stand_in,
        carried: true,
        wandered,
    };
    if size < 1e-12
        && let Some((v, degree, stand_in)) = superattracting(map, &p, t, start, digits, want)?
    {
        return Ok((v, site(degree, stand_in)));
    }
    // next to the fixed point 0 a map may take its small values from numbers of the size 1
    if p.is_zero()
        && let Some(e) = exponent(start)
    {
        more(-e);
    }
    let tangent = abs(&Complex::with_val(prec, &lambda - 1u32)) < 1e-12;
    if !tangent && (size - 1.0).abs() < 1e-9 {
        return Err(
            "the slope at the fixed point has size 1 and is not 1: no regular iteration".into(),
        );
    }
    if size > 1.0 + 1e-9 || !approaches(map, &p, way.last().expect("orbit")) {
        return Err(no_point());
    }
    // an orbit that ends on the fixed point: the point is where the points next to it are in
    // the limit, at the fixed point
    if of.is_none() {
        let (landed, lost) = carried(map, &p, &way[..way.len().min(64)], prec);
        if landed.is_some() {
            return Ok((p.clone(), site(1, Some(p.clone()))));
        }
        if p.is_zero() {
            more(lost);
        }
    }
    // the whole steps: the map itself
    if of.is_none() && *t.real() >= 1 {
        let whole = t.real().to_f64().floor();
        if whole > 1e6 {
            return Err("too many steps".into());
        }
        let mut z = start.clone();
        let mut taken = 0u32;
        for _ in 0..whole as u32 {
            let next = map
                .value(&z)
                .ok_or("the map has no value on the orbit of the point")?;
            if next == z {
                break;
            }
            z = next;
            taken += 1;
        }
        let rest = Complex::with_val(prec, t - whole as u32);
        let (y, mut found) = regular(map, of, &rest, &z, digits, known, want)?;
        // what is said of the point after its whole steps, said of the point itself: how its
        // orbit went, and the point that the branch from the fixed point has in its place
        found.wandered = found.wandered || wandered;
        if taken <= 64 {
            let mut c = match &found.stand_in {
                Some(c) if c.real().is_nan() => None,
                Some(c) => Some(c.clone()),
                None => Some(z),
            };
            for _ in 0..taken {
                c = c.and_then(|c| map.unstep(&c, &p, &lambda, None));
            }
            found.stand_in = match c {
                None => Some(Complex::with_val(prec, Special::Nan)),
                Some(c) => {
                    let gap = Complex::with_val(prec, &c - start);
                    let same = exponent(&gap)
                        .is_none_or(|e| e < exponent(start).unwrap_or(0).max(0) - prec as i32 + 64);
                    (!same).then_some(c)
                }
            };
        }
        return Ok((y, found));
    }
    let log = map.log_slope(&lambda);
    // how much the iterate stretches near the fixed point
    let stretch = (Complex::with_val(64, t * &log))
        .real()
        .to_f64()
        .exp()
        .max(1.0);
    if reaches(stretch.log2(), 1e5) {
        return Err("the count of steps is too large".into());
    }
    let ratio: f64 = if tangent { 24.0 } else { 256.0 };
    let terms = (f64::from(digits) * std::f64::consts::LOG2_10 / ratio.log2()).ceil() as usize + 6;
    let mut near = if tangent {
        // in the unit of length of the map z + a z^(q+1): |a|^(-1/q)
        let unit = taylor(map, &p, &Float::with_val(prec, 1e-3), 12)
            .and_then(|a| {
                let top = a.iter().skip(1).map(abs).fold(0.0f64, f64::max);
                a.iter()
                    .enumerate()
                    .skip(1)
                    .find(|(_, c)| abs(c) > 1e-9 * top)
                    .map(|(k, c)| abs(c).powf(-1.0 / k as f64))
            })
            .unwrap_or(1.0);
        1.2e-2 * unit.min(1.0) * (32.0 / f64::from(digits)).min(1.0)
    } else {
        1e-4 / stretch
    };
    let mut orbit = way;
    for _ in 0..12 {
        // the orbit up to its first point that is as near
        let far = |z: &Complex| abs(&Complex::with_val(prec, z - &p));
        match orbit.iter().position(|z| far(z) <= near) {
            Some(at) => orbit.truncate(at + 1),
            None => {
                for _ in 0..4_000_000 {
                    let last = orbit.last().expect("orbit");
                    if far(last) <= near {
                        break;
                    }
                    let next = map.value(last).ok_or_else(no_point)?;
                    orbit.push(next);
                }
                if far(orbit.last().expect("orbit")) > near {
                    return Err(no_point());
                }
            }
        }
        let h = Complex::with_val(prec, orbit.last().expect("orbit") - &p);
        if h.is_zero() {
            // the fixed point stays; a point whose orbit ends on it is where the points next
            // to it are in the limit: at the fixed point
            let stand_in = (orbit.len() > 1 && of.is_none()).then(|| p.clone());
            return Ok((p.clone(), site(1, stand_in)));
        }
        // the iterate is taken that close to the fixed point, where a point has fewer digits,
        // and it comes back from there
        let size = exponent(&p)
            .max(exponent(&Complex::with_val(prec, start - &p)))
            .unwrap_or(0)
            .max(0);
        more(stretch.log2().ceil() as i32 + size - exponent(&h).unwrap_or(0));
        // a disk that the point and its iterate are well inside of, and no speck
        let reach = (abs(&h) * stretch).max(near * 1e-3);
        let radius = Float::with_val(prec, reach * ratio);
        let Some(mut a) = taylor(map, &p, &radius, terms).filter(|a| holds(map, &p, a, &radius))
        else {
            near /= 8.0;
            continue;
        };
        a[0] = lambda.clone();
        let b = iterate_series(&a, t, &log, prec)
            .ok_or("the powers of the slope at the fixed point come too close to each other")?;
        // The series of an iterate holds in a smaller disk than that of the map when a point
        // with the slope 0 is next to the fixed point, and at a slope 1 it only falls off up
        // to a term: its last terms at the point tell, and closer to the fixed point they
        // are smaller.
        let lead = exponent(&Complex::with_val(prec, &b[0] * &h)).unwrap_or(0);
        let mut power = h.clone();
        let mut tail: Option<i32> = None;
        for (k, c) in b.iter().enumerate() {
            if k + 4 >= b.len() {
                tail = tail.max(exponent(&Complex::with_val(prec, c * &power)));
            }
            power *= &h;
        }
        if tail.is_some_and(|e| e > lead - numeric::bits_for(digits) as i32 - 16) {
            near /= if tangent { 2.0 } else { 8.0 };
            continue;
        }
        let mut y = Complex::with_val(prec, series_value(&b, &h, prec) + &p);
        let steps = orbit.len() - 1;
        let mut stand_in = None;
        let mut carried = true;
        match of {
            Some(of) => {
                // on the way back the inverse must take each point to the one before it:
                // else the value is on another branch than its own iterates would be
                let floor = exponent(&p).unwrap_or(i32::MIN);
                let same = |a: &Complex, b: &Complex| {
                    exponent(&Complex::with_val(prec, a - b)).is_none_or(|e| {
                        e < exponent(b).unwrap_or(e).max(floor) - numeric::bits_for(digits) as i32
                            + 20
                    })
                };
                for _ in 0..steps {
                    let next = of.value(&y).ok_or(
                        "the map has no value on the way back from its fixed point: the iterate may be too large for a number",
                    )?;
                    carried = carried && map.value(&next).is_some_and(|back| same(&back, &y));
                    y = next;
                }
            }
            None => {
                // at a tangent point a step back moves a point by little, and by nearly as
                // much as the step before it: a guess from the last two
                let mut last: Option<(Complex, Complex)> = None;
                let mut before: Option<(Complex, Complex)> = None;
                for _ in 0..steps {
                    let hint = match (&last, &before) {
                        (Some((y1, u1)), Some((y2, u2))) if tangent => {
                            let moved = Complex::with_val(prec, u1 - y1);
                            let earlier = Complex::with_val(prec, u2 - y2);
                            let apart = Complex::with_val(prec, y1 - y2);
                            (!apart.is_zero()).then(|| {
                                let trend = Complex::with_val(prec, &moved - &earlier) / &apart;
                                Complex::with_val(prec, &y + &moved)
                                    + trend * Complex::with_val(prec, &y - y1)
                            })
                        }
                        _ => None,
                    };
                    let u = map.unstep(&y, &p, &lambda, hint.as_ref()).ok_or(BRANCH)?;
                    before = last.take();
                    last = Some((y, u.clone()));
                    y = u;
                }
                // the point that the branch has in the place of the start: itself, unless a
                // fold of the map lies between
                let from = steps.min(64);
                let mut c = Some(orbit[from].clone());
                for _ in 0..from {
                    c = c.and_then(|c| map.unstep(&c, &p, &lambda, None));
                }
                let same = c.as_ref().is_some_and(|c| {
                    let gap = Complex::with_val(prec, c - start);
                    exponent(&gap)
                        .is_none_or(|e| e < exponent(start).unwrap_or(0).max(0) - prec as i32 + 64)
                });
                if !same {
                    stand_in = Some(c.unwrap_or_else(|| Complex::with_val(prec, Special::Nan)));
                }
            }
        }
        // nothing is left of a value that is 0 to the digits of a fixed point elsewhere
        let floor = exponent(&p).unwrap_or(0) - numeric::bits_for(digits) as i32 - 24;
        if !p.is_zero() && exponent(&y).is_some_and(|e| e < floor) {
            y = Complex::new(prec);
        }
        let mut found = site(1, stand_in);
        found.carried = carried;
        return Ok((y, found));
    }
    Err(
        "the map has no power series in a disk around its fixed point that the orbit comes into"
            .into(),
    )
}

/// The value at `start` of the `t`-th iterate at a fixed point `p` with the slope 0, where the
/// map is `p + a (z - p)^m + ...` with `m >= 2`: Boettcher's coordinate `phi` turns the map into
/// `w -> w^m`, and the iterate is `phi^-1(phi(z)^(m^t))`, followed from `t = 0`, where it is
/// the point itself. Nothing when the slope is small and not 0.
fn superattracting(
    map: &dyn Flow,
    fixed: &Complex,
    t: &Complex,
    start: &Complex,
    digits: u32,
    want: &Cell<u32>,
) -> Result<Option<(Complex, u32, Option<Complex>)>, String> {
    let low = map.prec();
    let fail = || "the map has no power series at its fixed point with the slope 0".to_string();
    let too_far = || {
        "Boettcher's coordinate at the fixed point with the slope 0 has no value at the point: it is too far, or the coordinate jumps on the line between them"
            .to_string()
    };
    let (m, _) = first_term(map, fixed, 24).ok_or_else(fail)?;
    if m < 2 {
        return Ok(None);
    }
    // the fixed point itself when it is a short number
    let mut p = fixed.clone();
    let short = |x: &Float| (Float::with_val(low, x) << 40u32).round() >> 40u32;
    let snapped = Complex::with_val(low, (short(p.real()), short(p.imag())));
    if map.value(&snapped).is_some_and(|v| v == snapped) {
        p = snapped;
    }
    // f(p + w) - p loses digits for small w: more of them
    let sharper = map.sharper((m + 1) * low + 64);
    let map: &dyn Flow = sharper.as_deref().unwrap_or(map);
    let prec = map.prec();
    if prec > low {
        p = Complex::with_val(prec, p);
        for _ in 0..100 {
            let gap = Complex::with_val(prec, map.value(&p).ok_or_else(fail)? - &p);
            if gap.is_zero() {
                break;
            }
            let slope = Complex::with_val(prec, map.slope(&p).ok_or_else(fail)? - 1u32);
            let step = Complex::with_val(prec, &gap / &slope);
            p -= &step;
            if step.is_zero()
                || exponent(&step).unwrap_or(0) < exponent(&p).unwrap_or(0) - prec as i32 + 4
            {
                break;
            }
        }
    }
    let start = Complex::with_val(prec, start);
    if Complex::with_val(prec, &start - &p).is_zero() {
        return Ok(Some((Complex::with_val(low, p), m, None)));
    }
    // next to the fixed point 0 a map may take its small values from numbers of the size 1
    if p.is_zero()
        && let Some(e) = exponent(&start)
        && -e > low as i32 - 64
    {
        want.set(want.get().max((-e - low as i32 + 64) as u32 + 32));
    }
    let terms = 24usize.max((low / 20 + 4) as usize);
    let (again, lead) = first_term(map, &p, terms).ok_or_else(fail)?;
    if again != m {
        return Err(fail());
    }
    // ln c with c^(m-1) = a: phi(z) = c (z - p) (1 + ...)
    let ln_c = Complex::with_val(prec, lead.ln_ref()) / (m - 1);
    let done = -(low as i32) + 16;
    let two_pi = Float::with_val(prec, Constant::Pi) * 2u32;
    // the logarithm of the coordinate next to the fixed point, from a copy of the map with any
    // count of bits: ln(c w_0) + the sum of ln(w_(k+1)/(a w_k^m))/m^(k+1) along the orbit
    // w_k = f^k(z) - p, up to terms of the size 2^done
    let direct_with = |flow: &dyn Flow, z: &Complex, done: i32| -> Option<Complex> {
        let bits = flow.prec();
        let (p, lead) = (Complex::with_val(bits, &p), Complex::with_val(bits, &lead));
        let mut point = Complex::with_val(bits, z);
        let mut w = Complex::with_val(bits, &point - &p);
        if w.is_zero() {
            return None;
        }
        let mut total = Complex::with_val(bits, w.ln_ref()) + Complex::with_val(bits, &ln_c);
        let mut weight = Float::with_val(bits, 1);
        // the size of the last term over that of its point: the terms fall off like the points
        let mut rate: Option<i32> = None;
        for _ in 0..200 {
            // a term that is too small to count is not taken: its point is known to too
            // few digits
            if let (Some(r), Some(e)) = (rate, exponent(&w))
                && r + e < done - 8
            {
                return Some(total);
            }
            point = flow.value(&point)?;
            let next = Complex::with_val(bits, &point - &p);
            if next.is_zero() {
                return None;
            }
            let mut low_term = lead.clone();
            for _ in 0..m {
                low_term *= &w;
            }
            let ratio = Complex::with_val(bits, &next / &low_term);
            let gap = Complex::with_val(bits, &ratio - 1u32);
            if reaches(abs(&gap), 0.9) {
                // no clear branch of the logarithm: too far from the fixed point
                return None;
            }
            weight /= m;
            total += Complex::with_val(bits, ratio.ln_ref()) * &weight;
            let Some(size) = exponent(&gap) else {
                return Some(total);
            };
            rate = Some(size - exponent(&w).unwrap_or(0));
            w = next;
        }
        None
    };
    // further away, through the orbit: ln phi(f^k(z)) with the first k that works
    // the values of the logarithm of the coordinate after k steps are 2 pi/m^k apart: no more
    // of them than can be told apart
    let most = (44.0 / f64::from(m).log2()).floor() as u32;
    let pushed_with = |flow: &dyn Flow, z: &Complex, done: i32| -> Option<(Complex, u32)> {
        let mut point = Complex::with_val(flow.prec(), z);
        for k in 0..=most {
            if let Some(l) = direct_with(flow, &point, done) {
                return Some((l, k));
            }
            point = flow.value(&point)?;
            if reaches(abs(&point), 1e100) {
                return None;
            }
        }
        None
    };
    // a copy of the map with few bits, for the choice among the values
    let rough = map.sharper(192);
    let rough: &dyn Flow = rough.as_deref().unwrap_or(map);
    // The logarithm of the coordinate of a point: from ln phi(f^k(z)) it is known up to
    // 2 pi i j/m^k. The right one is ln(c (z - p)) + h(z) for the function h that is 0 at the
    // fixed point and has no jump on the line from there to the point.
    let coordinate = |z: &Complex| -> Option<Complex> {
        let (l, k) = pushed_with(map, z, done)?;
        if k == 0 {
            return Some(l);
        }
        let line = Complex::with_val(prec, z - &p);
        let ln_line = Complex::with_val(prec, line.ln_ref());
        let pi = std::f64::consts::PI;
        let parts = |z: &Complex| (z.real().to_f64(), z.imag().to_f64());
        let (line_re, line_im) = parts(&ln_line);
        let (c_re, c_im) = parts(&ln_c);
        // h on the line, with few digits: its values at a point differ in the angle only, and
        // so it is the angle that may not move by much in a step, nor by more than a part
        // of that at the speed of the step before
        let (mut s, mut step) = (0.0f64, 1.0 / 16.0);
        let (mut h_re, mut h_im) = (0.0f64, 0.0f64);
        let mut speed = 0.0f64;
        let mut count = 0;
        while s < 1.0 {
            count += 1;
            if count > 20_000 {
                return None;
            }
            let next = (s + step).min(1.0);
            let at = Complex::with_val(192, &line * next) + Complex::with_val(192, &p);
            let (l, k) = pushed_with(rough, &at, -48)?;
            let scale = f64::from(m).powi(k as i32);
            let spacing = 2.0 * pi / scale;
            let (l_re, l_im) = parts(&l);
            let re = l_re / scale - line_re - next.ln() - c_re;
            let first = l_im / scale - line_im - c_im;
            let im = first + ((h_im - first) / spacing).round() * spacing;
            let turn = (im - h_im).abs();
            if turn > (spacing / 4.0).min(0.4)
                || 2.0 * speed * (next - s) > spacing / 8.0
                || (re - h_re).abs() > 1.0
            {
                if step < 1e-9 {
                    return None;
                }
                step /= 2.0;
                continue;
            }
            speed = turn / (next - s);
            (h_re, h_im, s) = (re, im, next);
            step *= 2.0;
        }
        // with all digits: the value next to it
        let scale = Float::with_val(prec, rug::ops::Pow::pow(Float::with_val(prec, m), k));
        let value = Complex::with_val(prec, &l / &scale);
        let spacing = Float::with_val(prec, &two_pi / &scale);
        let first = value.imag().to_f64() - line_im - c_im;
        let turns = ((h_im - first) / spacing.to_f64()).round();
        Some(value + Complex::with_val(prec, (0, spacing * turns)))
    };
    let degree = Float::with_val(prec, m);
    let ln_m = degree.ln();
    // An orbit that ends on the fixed point before it is next to it: the point is where the
    // points next to it are in the limit, at the fixed point.
    let pushed = pushed_with(map, &start, done);
    let mut orbit = vec![start.clone()];
    for _ in 0..pushed.as_ref().map_or(most, |(_, k)| *k) {
        match map.value(orbit.last().expect("orbit")) {
            Some(next) => orbit.push(next),
            None => break,
        }
    }
    if carried(map, &p, &orbit, low).0.is_some() {
        let at = Complex::with_val(low, &p);
        return Ok(Some((at.clone(), m, Some(at))));
    }
    // a point that the coordinate cannot be followed to has the value of the point that the
    // map takes to the same points and that is on the way from the fixed point: the
    // logarithm of the coordinate further on the orbit, over the power of the degree
    let base = coordinate(&start)
        .or_else(|| {
            let (l, k) = pushed?;
            let scale = Float::with_val(prec, rug::ops::Pow::pow(Float::with_val(prec, m), k));
            Some(l / scale)
        })
        .ok_or_else(too_far)?;
    // The point with a given logarithm of the coordinate, as v = ln(u - p): followed from next
    // to the fixed point, where the coordinate is c (u - p), out along the ray of the
    // coordinate. This is the inverse of the coordinate that goes on from the fixed point.
    let reach = |goal: &Complex, digits: u32| -> Result<Complex, String> {
        let lost = || {
            format!(
                "the iterate would be past a point where the slope of the map is 0, as seen from the fixed point: {PAST}"
            )
        };
        let deep = -12.0 * std::f64::consts::LN_2;
        let depth = goal.real().to_f64();
        if !depth.is_finite() {
            return Err("the count of steps is too large".into());
        }
        // goal + q, from the coordinate of the size 2^-12 to q = 0
        let mut q = (deep - depth).min(0.0);
        let mut step = 1.0f64;
        let mut v = Complex::with_val(prec, goal - &ln_c) + Float::with_val(prec, q);
        // how much v moves with the coordinate: 1 next to the fixed point
        let mut rate = Complex::with_val(prec, 1);
        let mut first = true;
        loop {
            let next = if first { q } else { (q + step).min(0.0) };
            let last = next >= 0.0;
            let target = Complex::with_val(prec, goal + Float::with_val(prec, next));
            let bits = if last {
                numeric::bits_for(digits) as i32 + 24
            } else {
                40
            };
            let residual = |v: &Complex| -> Option<Complex> {
                let u = Complex::with_val(prec, v.exp_ref()) + &p;
                let mut d = Complex::with_val(prec, coordinate(&u)? - &target);
                // the imaginary part of a logarithm: up to turns
                let turns = Float::with_val(prec, d.imag() / &two_pi).round();
                d -= Complex::with_val(prec, (0, turns * &two_pi));
                Some(d)
            };
            let ask = Complex::with_val(prec, &rate * (next - q));
            let guess = Complex::with_val(prec, &v + &ask);
            // the secant method in v, where a step is a relative step of u - p: the point,
            // and how much v moves with the coordinate there
            let solve = || -> Option<(Complex, Complex)> {
                let mut x0 = guess.clone();
                let mut f0 = residual(&x0)?;
                let mut x1 = Complex::with_val(prec, &x0 - Complex::with_val(prec, &f0 * &rate));
                let mut f1 = residual(&x1)?;
                let mut slope = rate.clone();
                for _ in 0..80 {
                    if exponent(&f1).is_none_or(|e| e < -bits) {
                        return Some((x1, slope));
                    }
                    let df = Complex::with_val(prec, &f1 - &f0);
                    if df.is_zero() {
                        return None;
                    }
                    slope = Complex::with_val(prec, &x1 - &x0) / df;
                    let x2 = Complex::with_val(prec, &x1 - Complex::with_val(prec, &f1 * &slope));
                    if reaches(abs(&Complex::with_val(prec, &x2 - &guess)), 8.0) {
                        return None;
                    }
                    x0 = x1;
                    f0 = f1;
                    x1 = x2;
                    f1 = residual(&x1)?;
                }
                None
            };
            let solved = solve();
            // not far from where the last step points to: the same branch
            let near = solved.as_ref().is_some_and(|(r, _)| {
                first || abs(&Complex::with_val(prec, r - &guess)) <= 0.5 * abs(&ask)
            });
            let sane = |slope: &Complex| abs(slope) > 1e-6 && abs(slope) < 1e12;
            match solved {
                Some((r, slope)) if near => {
                    if sane(&slope) {
                        rate = slope;
                    }
                    (v, q, first) = (r, next, false);
                    if last {
                        return Ok(v);
                    }
                    step = (step * 2.0).min(4.0);
                }
                other => {
                    if first {
                        return Err(lost());
                    }
                    step /= 2.0;
                    if step < 1e-9 {
                        return Err(lost());
                    }
                    // a short step that went further than asked: v moves more than was thought
                    if let Some((r, _)) = other
                        && step < 0.1
                    {
                        let slope =
                            Complex::with_val(prec, &r - &v) / Float::with_val(prec, next - q);
                        if sane(&slope) {
                            rate = slope;
                        }
                    }
                }
            }
        }
    };
    let goal = Complex::with_val(prec, &base * Complex::with_val(prec, t * &ln_m).exp());
    if !goal.real().is_sign_negative() || goal.real().is_zero() {
        return Err("the iterate leaves the basin of the fixed point with the slope 0: Boettcher's coordinate would have the size 1 or more".into());
    }
    let v = reach(&goal, digits)?;
    // the point that this inverse of the coordinate has in the place of the start: itself,
    // unless a fold of the map lies between
    let u0 = Complex::with_val(prec, reach(&base, 16)?.exp_ref()) + &p;
    let gap = abs(&Complex::with_val(prec, &u0 - &start));
    let stand_in = (gap > 1e-9 * abs(&Complex::with_val(prec, &start - &p)))
        .then(|| Complex::with_val(low, u0));
    Ok(Some((Complex::with_val(low, v.exp() + p), m, stand_in)))
}

/// The value at `start` of the `t`-th iterate at a fixed point where the slope has size 1 and
/// is no root of unity: the map is a rotation in the right coordinate, and the series of the
/// iterate holds in a disk around the point.
fn rotation(
    map: &dyn Flow,
    t: &Complex,
    start: &Complex,
    digits: u32,
    known: Option<&Complex>,
) -> Result<(Complex, Site), String> {
    let prec = map.prec();
    let none = || "no fixed point with a slope of size 1 next to the point".to_string();
    let p = match known {
        Some(p) => p.clone(),
        None => polish(map, start).ok_or_else(none)?,
    };
    let lambda = settled(map.slope(&p).ok_or_else(none)?);
    if (abs(&lambda) - 1.0).abs() > 1e-9 {
        return Err(none());
    }
    let mut power = lambda.clone();
    for q in 1..=120u32 {
        if abs(&Complex::with_val(prec, &power - 1u32)) < 1e-7 {
            return Err(format!(
                "the slope at the fixed point is a root of 1 (its power {q} is 1): the map is no rotation there"
            ));
        }
        power *= &lambda;
    }
    let h = Complex::with_val(prec, start - &p);
    let site = || Site {
        point: p.clone(),
        slope: lambda.clone(),
        degree: 1,
        stand_in: None,
        carried: true,
        wandered: false,
    };
    if h.is_zero() {
        return Ok((p.clone(), site()));
    }
    // (more digits take more terms, and a point next to the edge of the disk more than there
    // is time for)
    let most = if digits > 64 { 384 } else { 192 };
    let by_series = |t: &Complex| -> Option<Complex> {
        for terms in [48usize, 96, 192, 384] {
            if terms > most {
                break;
            }
            let radius = Float::with_val(prec, abs(&h) * 3.0);
            let Some(mut a) = taylor(map, &p, &radius, terms) else {
                continue;
            };
            a[0] = lambda.clone();
            let Some(b) = iterate_series(&a, t, &map.log_slope(&lambda), prec) else {
                continue;
            };
            let mut y = p.clone();
            let mut power = h.clone();
            let mut tail = Complex::new(prec);
            for (k, c) in b.iter().enumerate() {
                let term = Complex::with_val(prec, c * &power);
                if k + 8 >= terms && abs(&term) > abs(&tail) {
                    tail = term.clone();
                }
                y += term;
                power *= &h;
            }
            if negligible(&tail, f64::from(digits) + 3.0) {
                return Some(y);
            }
        }
        None
    };
    if let Some(y) = by_series(t) {
        return Ok((y, site()));
    }
    // a point of the disk whose iterate is not in it: the real part of the count of steps
    // keeps the point on its circle
    let around = Complex::with_val(prec, (t.real(), 0));
    if !t.imag().is_zero() && by_series(&around).is_some() {
        return Err(EDGE.into());
    }
    Err("the point is not close enough to the fixed point of the rotation: the series of the iterate does not settle".into())
}

/// The fixed points of the map where it does not contract, the ones next to the point first.
fn repelling(map: &Map, f: &Expr, x: &str, start: &Complex) -> Vec<(Complex, Complex)> {
    let prec = map.prec;
    let mut seeds = vec![
        start.clone(),
        Complex::new(prec),
        Complex::with_val(prec, start / 2u32),
        Complex::with_val(prec, start * 2u32),
        Complex::with_val(prec, -start),
    ];
    if let Ok(found) = solve::solve(&sub(f.clone(), sym(x)), x) {
        solve::release(&found.params);
        for root in &found.roots {
            let value = found
                .params
                .iter()
                .fold(root.value.clone(), |v, p| subst_sym(&v, p, &Expr::zero()));
            if let Ok(z) = numeric::evalf(&value, prec) {
                seeds.push(z);
            }
        }
    }
    let far = |z: &Complex| abs(&Complex::with_val(prec, z - start));
    let mut out: Vec<(Complex, Complex)> = Vec::new();
    for seed in seeds {
        let Some(q) = polish(map, &seed) else {
            continue;
        };
        let Some(lambda) = map.slope(&q).map(settled) else {
            continue;
        };
        if abs(&lambda) < 1.0 - 1e-9
            || out
                .iter()
                .any(|(o, _)| abs(&Complex::with_val(prec, o - &q)) < 1e-20 * (1.0 + abs(&q)))
        {
            continue;
        }
        out.push((q, lambda));
    }
    // for a real point the real ones first
    let off = |q: &Complex| {
        start.imag().is_zero() && abs(&Complex::with_val(prec, (0, q.imag()))) > 1e-30
    };
    out.sort_by(|a, b| {
        off(&a.0)
            .cmp(&off(&b.0))
            .then(far(&a.0).total_cmp(&far(&b.0)))
    });
    out.truncate(4);
    out
}

/// The value at the number `x0` of the `t`-th iterate of the map `x -> f`: at the fixed point
/// `at` (a number next to it, or `inf`) when it is given, else at the one the orbit of the point
/// runs into, forward or backward.
pub fn value(
    f: &Expr,
    x: &str,
    t: &Expr,
    x0: &Expr,
    at: Option<&Expr>,
    digits: u32,
) -> Result<(Complex, String), String> {
    let mut prec = numeric::bits_for(digits) + 128;
    // again with more bits when the way to the fixed point has lost too many
    for _ in 0..4 {
        let want = Cell::new(0);
        let found = value_with(f, x, t, x0, at, digits, prec, &want);
        match want.get() {
            0 => return found,
            more if more > 400_000 => {
                return Err("the iterate takes more digits than there are to work with: the point is too close to a fixed point, or the count of steps is too large".into());
            }
            more => prec += more,
        }
    }
    value_with(f, x, t, x0, at, digits, prec, &Cell::new(0))
}

/// The same with a given count of bits: `want` is how many more of them it takes.
#[allow(clippy::too_many_arguments)]
fn value_with(
    f: &Expr,
    x: &str,
    t: &Expr,
    x0: &Expr,
    at: Option<&Expr>,
    digits: u32,
    prec: u32,
    want: &Cell<u32>,
) -> Result<(Complex, String), String> {
    let number =
        |e: &Expr| numeric::evalf(e, prec).map_err(|_| format!("{e} has no numeric value"));
    let (tv, start) = (number(t)?, number(x0)?);
    let forward = Map::new(f, x, prec);
    // the fixed point asked for: nothing for infinity
    let anchor: Option<Option<(Complex, Complex, Complex)>> = match at {
        None => None,
        Some(e) if e.is_sym("inf") || e.is_sym("ComplexInfinity") => Some(None),
        Some(e) => {
            let none = || format!("the map has no fixed point next to {e}");
            let given = number(e)?;
            let q = polish(&forward, &given).ok_or_else(none)?;
            let lambda = settled(forward.slope(&q).ok_or_else(none)?);
            Some(Some((q, lambda, given)))
        }
    };
    // infinity as a fixed point of a quotient of polynomials: the map 1/f(1/w) at w = 0
    let (top, low) = ratsimp::fraction(&ratsimp::together(f));
    let quotient =
        ratsimp::coefficients(&top, x).is_some() && ratsimp::coefficients(&low, x).is_some();
    let turned = if quotient {
        simplify(&ratsimp::together(&pow(
            subst_sym(f, x, &pow(sym(x), Expr::minus_one())),
            Expr::minus_one(),
        )))
    } else {
        Expr::zero()
    };
    let around = Map::new(&turned, x, prec);
    let zero = Complex::new(prec);
    let at_infinity = quotient
        && around.value(&zero).is_some_and(|v| v.is_zero())
        && around.slope(&zero).is_some_and(|d| abs(&d) < 1.0 + 1e-9);
    let inverse = std::cell::OnceCell::new();
    let sources = std::cell::OnceCell::new();
    type Found = Result<(Complex, String), String>;
    // what was done, for a note
    let stand_in = |s: &Site, turn: bool| match &s.stand_in {
        None => String::new(),
        Some(c) => {
            let place = if c.real().is_nan() {
                "the point there".to_string()
            } else if turn {
                brief(&Complex::with_val(prec, c.recip_ref()))
            } else {
                brief(c)
            };
            format!(
                "; the point is not on the branch of the map that goes on from the fixed point: this is the iterate at {place}, which the map takes to the same points"
            )
        }
    };
    let inward_site = |s: Site| {
        let what = if s.degree >= 2 {
            format!(
                "Boettcher's coordinate at the fixed point {} (slope 0, degree {})",
                brief(&s.point),
                s.degree
            )
        } else if abs(&Complex::with_val(prec, &s.slope - 1u32)) < 1e-12 {
            format!(
                "regular iteration at the fixed point {} (slope 1)",
                brief(&s.point)
            )
        } else {
            format!(
                "regular iteration at the attracting fixed point {} (slope {})",
                brief(&s.point),
                brief(&s.slope)
            )
        };
        what + &stand_in(&s, false)
    };
    let outward_site = |s: Site| {
        let slope = Complex::with_val(prec, s.slope.recip_ref());
        if abs(&Complex::with_val(prec, &slope - 1u32)) < 1e-12 {
            format!(
                "regular iteration at the fixed point {} (slope 1)",
                brief(&s.point)
            )
        } else {
            format!(
                "regular iteration at the repelling fixed point {} (slope {})",
                brief(&s.point),
                brief(&slope)
            )
        }
    };
    let to_infinity = |tv: &Complex, start: &Complex| -> Found {
        if !at_infinity {
            return Err(NO_POINT.into());
        }
        // 0 has no place in 1/x: when the orbit goes through it, the iterate at the image of 0
        // first, and from there back along the orbit
        let mut path = vec![start.clone()];
        for _ in 0..8 {
            let last = path.last().expect("path");
            let Some(next) = forward.value(last) else {
                break;
            };
            let done = last.is_zero();
            path.push(next);
            if done {
                break;
            }
        }
        let through = path.len() >= 2 && path[path.len() - 2].is_zero();
        if !through {
            path.truncate(1);
        }
        let top = path.last().expect("path").clone();
        if top.is_zero() {
            return Err(NO_POINT.into());
        }
        // a negative number is on the upper side of the cut of its logarithm, and so its
        // reciprocal is on the lower side of its own: the zero of its imaginary part is -0
        let w0 = Complex::with_val(prec, numeric::unsigned_zero(top).recip_ref());
        let at_top = |s: &Complex| -> Result<(Complex, Site), String> {
            let (u, site) = regular(&around, None, s, &w0, digits, Some(&zero), want)?;
            if u.is_zero() {
                return Err("the iterate is infinite".into());
            }
            Ok((u.recip(), site))
        };
        let describe = |site: &Site| {
            let what = if site.degree >= 2 {
                format!(
                    "Boettcher's coordinate at infinity (degree {})",
                    site.degree
                )
            } else {
                format!(
                    "regular iteration at infinity (slope {} in 1/{x})",
                    brief(&site.slope)
                )
            };
            what + &stand_in(site, true)
        };
        let whole = at_top(tv)?;
        if path.len() == 1 {
            return Ok((whole.0, describe(&whole.1)));
        }
        // from 0 itself, when the slope is 0 there, the principal root; from a point before it
        // there is no such choice
        if path.len() > 2 && forward.slope(&zero).is_none_or(|d| d.is_zero()) {
            return Err(format!(
                "the orbit of the point goes through 0, where the slope of the map is 0: {PAST}"
            ));
        }
        // followed from 0 steps, where the iterate is the orbit
        let finest = 64u64;
        let easy = abs(tv) < 1.0 && !tv.real().is_sign_negative();
        let all = if easy {
            finest
        } else {
            (abs(tv) * 4.0).ceil().clamp(1.0, 1e4) as u64 * finest
        };
        let (mut part, mut piece) = (0u64, finest);
        let mut current = path.clone();
        let mut site = None;
        while part < all {
            let next_part = (part + piece).min(all);
            let share = Float::with_val(prec, next_part) / Float::with_val(prec, all);
            let s = Complex::with_val(prec, tv * &share);
            let step = at_top(&s).ok().and_then(|(value, place)| {
                let mut values = vec![value];
                for k in (0..path.len() - 1).rev() {
                    let y = values.last().expect("values");
                    values.push(forward.step_back(y, &current[k], &current[k + 1])?);
                }
                values.reverse();
                Some((values, place))
            });
            match step {
                Some((values, place)) => {
                    current = values;
                    site = Some(place);
                    part = next_part;
                    piece = (piece * 2).min(finest);
                }
                None => {
                    piece /= 2;
                    if piece == 0 {
                        return Err(
                            "the iterate cannot be followed from the point through 0 on its orbit"
                                .into(),
                        );
                    }
                }
            }
        }
        let site = site.ok_or(NO_POINT)?;
        Ok((current.swap_remove(0), describe(&site)))
    };
    let turning = |s: Site| {
        format!(
            "regular iteration at the fixed point {} (slope {} of size 1)",
            brief(&s.point),
            brief(&s.slope)
        )
    };
    // the inverse of the map as a map of its own: by its formula, or next to a fixed point
    let by_formula = || -> Option<Local> {
        let g = inverse.get_or_init(|| inverse_map(f, x)).as_ref()?;
        Some(Local {
            of: forward.with_prec(prec),
            branch: Some(Map::new(g, x, prec)),
            at: None,
        })
    };
    let next_to = |q: &Complex, lambda: &Complex| Local {
        of: forward.with_prec(prec),
        branch: None,
        at: Some((q.clone(), lambda.clone())),
    };
    let close = |a: &Complex, b: &Complex| {
        let gap = Complex::with_val(prec, a - b);
        negligible(&gap, f64::from(digits) - 6.0)
            || abs(&gap) <= 10f64.powi(-(digits as i32) + 6) * abs(b)
            || exponent(&gap)
                .zip(exponent(b))
                .is_some_and(|(g, size)| g < size - (numeric::bits_for(digits) as i32 - 20))
    };
    // Away from a fixed point: the inverse runs into it. The point stands for the image of
    // the point that this inverse has for it, which is the point itself up to a fold of the
    // map.
    // at home with an inverse: it takes the image of the point back to it, and then the
    // iterate goes along with the map
    let at_home = |local: &Local, start: &Complex| match forward.value(start) {
        Some(image) => local
            .value(&image)
            .is_some_and(|again| close(&again, start)),
        None => true,
    };
    let outward = |local: &Local,
                   tv: &Complex,
                   start: &Complex,
                   known: Option<&Complex>|
     -> Found {
        let home = at_home(local, start);
        let back = Complex::with_val(prec, -tv);
        let (v, s) = regular(local, Some(&forward), &back, start, digits, known, want)?;
        let carried = s.carried;
        let mut what = outward_site(s);
        let before = local.value(start);
        if !home && let Some(before) = &before {
            what = format!(
                "{what}; the point is past a fold of the map, as seen from the fixed point: it is taken as the image of {}",
                brief(before)
            );
        }
        if !carried {
            what = format!(
                "{what}; the value is past a fold of the map, as seen from the fixed point: its iterates do not carry this one on"
            );
        }
        Ok((v, what))
    };
    // A fixed point with the slope 1 has a side that the map runs into and a side that it
    // runs out of, and the orbit of a point may come in from the far side by a long way
    // round. A point that is off the branch on such a way in, and at home on the way out, is
    // on the way out.
    let way_out = |s: &Site, tv: &Complex, start: &Complex| -> Option<(Complex, String)> {
        let tangent = s.degree == 1 && abs(&Complex::with_val(prec, &s.slope - 1u32)) < 1e-12;
        if !tangent || !s.wandered || s.stand_in.is_none() {
            return None;
        }
        let from = |local: &Local| {
            at_home(local, start)
                .then(|| outward(local, tv, start, Some(&s.point)).ok())
                .flatten()
        };
        by_formula()
            .and_then(|local| from(&local))
            .or_else(|| from(&next_to(&s.point, &s.slope)))
    };
    let inward_at = |tv: &Complex, start: &Complex, known: Option<&Complex>| -> Found {
        // the bits that the way in asks for are not needed when the way out is taken
        let asked = want.get();
        let (v, s) = regular(&forward, None, tv, start, digits, known, want)?;
        let inward = want.replace(asked);
        match way_out(&s, tv, start) {
            Some(found) => Ok(found),
            None => {
                want.set(inward.max(asked));
                Ok((v, inward_site(s)))
            }
        }
    };
    // regular iteration at a given fixed point with the slope lambda: `given` is the number
    // that was written for the fixed point
    let at_point =
        |q: &Complex, lambda: &Complex, given: &Complex, tv: &Complex, start: &Complex| -> Found {
            let size = abs(lambda);
            let tangent = abs(&Complex::with_val(prec, lambda - 1u32)) < 1e-12;
            let inward = || inward_at(tv, start, Some(q));
            let inward_only = || -> Found {
                regular(&forward, None, tv, start, digits, Some(q), want)
                    .map(|(v, s)| (v, inward_site(s)))
            };
            let away = || -> Found {
                if let Some(local) = by_formula()
                    && let Ok(found) = outward(&local, tv, start, Some(q))
                {
                    return Ok(found);
                }
                outward(&next_to(q, lambda), tv, start, Some(q))
            };
            let missed = |e: String| {
                if e == NO_POINT {
                    "the orbit of the point does not run into this fixed point, forward or backward"
                        .to_string()
                } else {
                    e
                }
            };
            if tangent {
                // a number beside the fixed point tells the side: the one that the map takes
                // the number away from the fixed point on, or towards it
                let far = |z: &Complex| abs(&Complex::with_val(prec, z - q));
                let beside = far(given) > 1e-12 * (1.0 + abs(q));
                let leaves = beside && forward.value(given).is_some_and(|v| far(&v) > far(given));
                if leaves {
                    away().or_else(|_| inward()).map_err(missed)
                } else if beside {
                    inward_only().or_else(|_| away()).map_err(missed)
                } else {
                    inward().or_else(|_| away()).map_err(missed)
                }
            } else if size < 1.0 - 1e-9 {
                inward().map_err(missed)
            } else if size > 1.0 + 1e-9 {
                away().map_err(missed)
            } else {
                rotation(&forward, tv, start, digits, Some(q)).map(|(v, s)| (v, turning(s)))
            }
        };
    let missed = |r: &Found| matches!(r, Err(e) if e == NO_POINT);
    // the ways to an iterate, in their order: 0 is the fixed point asked for, or the one the
    // orbit runs into (infinity is one, and a point that the orbit goes around); 1 is the one
    // that the formula for the inverse runs into; then the fixed points where the map does not
    // contract, with the inverse next to each
    let way = |which: usize, tv: &Complex, start: &Complex| -> Found {
        match which {
            0 => {
                match &anchor {
                    Some(None) => {
                        return to_infinity(tv, start).map_err(|e| {
                            if e == NO_POINT {
                                "the orbit of the point does not run to infinity as a fixed point of a quotient of polynomials".to_string()
                            } else {
                                e
                            }
                        });
                    }
                    Some(Some((q, lambda, given))) => {
                        return at_point(q, lambda, given, tv, start);
                    }
                    None => {}
                }
                let first = inward_at(tv, start, None);
                if !missed(&first) {
                    return first;
                }
                let escaped = to_infinity(tv, start);
                if !missed(&escaped) {
                    return escaped;
                }
                rotation(&forward, tv, start, digits, None).map(|(v, s)| (v, turning(s)))
            }
            1 => outward(&by_formula().ok_or(NO_POINT)?, tv, start, None),
            _ => {
                let (q, lambda) = sources
                    .get_or_init(|| repelling(&forward, f, x, start))
                    .get(which - 2)
                    .ok_or(NO_POINT)?;
                outward(&next_to(q, lambda), tv, start, Some(q))
            }
        }
    };
    // what went wrong first, when it tells more than that there is no fixed point
    let mut failure: Option<String> = None;
    let ways = if anchor.is_some() { 1 } else { 6 };
    for which in 0..ways {
        let (found, how) = match way(which, &tv, &start) {
            Ok(found) => found,
            Err(e) => {
                // the orbit decides: a fixed point that it runs into is the one, whatever
                // comes of it; a point that it may go around is not
                let around = e.starts_with("the point is not close enough")
                    || e.starts_with("no fixed point with a slope of size 1")
                    || e.starts_with("the slope at the fixed point is a root of 1");
                if which == 0 && e != NO_POINT && !around {
                    // the orbit has chosen: another fixed point is for the one who asks
                    let elsewhere = e.contains("has no value at the point")
                        || e.contains(PAST)
                        || e.starts_with("the slope at the fixed point has size 1")
                        || e.starts_with("the map has no power series");
                    return Err(if anchor.is_none() && elsewhere {
                        format!(
                            "{e} (a number next to another fixed point of the map, as a fifth argument, gives the iterate at that one)"
                        )
                    } else {
                        e
                    });
                }
                // a value that is too large at the nearest fixed point is the answer: farther
                // fixed points have other iterates
                if e.ends_with("the iterate may be too large for a number") {
                    return Err(e);
                }
                if !e.starts_with("no fixed point with a slope of size 1")
                    && failure.as_ref().is_none_or(|old| old == NO_POINT)
                {
                    failure = Some(e);
                }
                continue;
            }
        };
        // a power in Boettcher's coordinate is a principal power, which does not commute with
        // the map where the angle of the coordinate is large: the value stands, with a word
        if how.starts_with("Boettcher") {
            let asked = want.get();
            let apart = match (forward.value(&start), forward.value(&found)) {
                (Some(image), Some(here)) => {
                    way(which, &tv, &image).is_ok_and(|(there, _)| !close(&there, &here))
                }
                _ => false,
            };
            want.set(asked);
            if apart {
                return Ok((
                    found,
                    format!(
                        "{how}; the power of the coordinate is a principal power, and at this point it does not commute with the map"
                    ),
                ));
            }
        }
        return Ok((found, how));
    }
    Err(failure.unwrap_or_else(|| NO_POINT.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BITS: u32 = 400;

    fn expr(src: &str) -> Expr {
        simplify(&crate::parser::parse(src).expect("parses").stmts[0])
    }

    fn number(src: &str) -> Complex {
        numeric::evalf(&expr(src), BITS).expect("a number")
    }

    fn run(f: &str, t: &str, x0: &str, at: Option<&str>) -> Result<(Complex, String), String> {
        let anchor = at.map(expr);
        value(&expr(f), "x", &expr(t), &expr(x0), anchor.as_ref(), 32)
    }

    /// The iterate at a point given as text.
    fn at(f: &str, t: &str, x0: &str) -> Complex {
        run(f, t, x0, None)
            .unwrap_or_else(|e| panic!("{f} [{t}] at {x0}: {e}"))
            .0
    }

    /// The iterate at a number, at the fixed point `p` when it is given.
    fn from(f: &str, t: &str, z: &Complex, p: Option<&str>) -> Complex {
        let anchor = p.map(expr);
        let point = numeric::to_float_expr(z.clone(), 60);
        value(&expr(f), "x", &expr(t), &point, anchor.as_ref(), 32)
            .unwrap_or_else(|e| panic!("{f} [{t}]: {e}"))
            .0
    }

    fn image(f: &str, z: &Complex) -> Complex {
        let f = expr(f);
        Map::new(&f, "x", BITS)
            .value(z)
            .expect("a value of the map")
    }

    fn gap(a: &Complex, b: &Complex) -> f64 {
        abs(&Complex::with_val(BITS, a - b)) / abs(b).max(1e-300)
    }

    fn assert_same(what: &str, got: &Complex, want: &Complex) {
        let far = gap(got, want);
        assert!(far < 1e-28, "{what}: {got} is not {want} (off by {far:e})");
    }

    fn real(x: f64) -> Complex {
        Complex::with_val(BITS, (x, 0))
    }

    /// A value without the wrapper that keeps a product as it is written.
    fn plain_value(e: &Expr) -> Option<Complex> {
        let plain = crate::canon::map_bottom_up(e, &mut |x| match &x {
            Expr::Fn(name, args) if &**name == "hold" && args.len() == 1 => args[0].clone(),
            _ => x,
        });
        numeric::evalf(&plain, BITS).ok()
    }

    fn ratio(n: i64, d: i64) -> Complex {
        Complex::with_val(
            BITS,
            (Float::with_val(BITS, n) / Float::with_val(BITS, d), 0),
        )
    }

    #[test]
    fn chebyshev_polynomials() {
        assert_eq!(chebyshev(0), [1]);
        assert_eq!(chebyshev(1), [0, 1]);
        assert_eq!(chebyshev(2), [-1, 0, 2]);
        assert_eq!(chebyshev(3), [0, -3, 0, 4]);
        assert_eq!(chebyshev(4), [1, 0, -8, 0, 8]);
        assert_eq!(chebyshev(5), [0, 5, 0, -20, 0, 16]);
        assert_eq!(chebyshev(7), [0, -7, 0, 56, 0, -112, 0, 64]);
        // T_n(1) = 1 and the leading term is 2^(n-1) x^n
        for n in 1..=12 {
            let c = chebyshev(n);
            assert_eq!(c.iter().sum::<i64>(), 1, "T_{n}(1)");
            assert_eq!(c[n], 1 << (n - 1), "T_{n}");
        }
    }

    #[test]
    fn inverse_is_the_branch_next_to_a_positive_number() {
        for (f, g) in [
            ("x^2", "sqrt(x)"),
            ("x^2-1", "sqrt(x+1)"),
            ("e^x", "ln(x)"),
            ("2x+1", "(x-1)/2"),
            ("x^3", "x^(1/3)"),
            ("1/(1+x)", "1/x-1"),
            ("sin(x)", "asin(x)"),
            ("x/2+x^2/4", "sqrt(4x+1)-1"),
        ] {
            let inverse =
                inverse_map(&expr(f), "x").unwrap_or_else(|| panic!("{f} has an inverse"));
            for x0 in [0.3f64, 0.7, 1.1] {
                let z = real(x0);
                let env = |v: &Complex| {
                    let mut env = Env::new();
                    env.insert(Rc::from("x"), v.clone());
                    env
                };
                let got = numeric::evalf_env(&inverse, BITS, &env(&z)).expect("a value");
                let want = numeric::evalf_env(&expr(g), BITS, &env(&z)).expect("a value");
                assert_same(&format!("the inverse of {f} at {x0}"), &got, &want);
            }
        }
        assert!(inverse_map(&expr("x+sin(x)"), "x").is_none());
    }

    /// The closed form at a number of steps and a point.
    fn closed(f: &str, t: &str, x0: &str) -> Complex {
        let g =
            symbolic(&expr(f), "x", &expr(t)).unwrap_or_else(|| panic!("{f} has a closed form"));
        plain_value(&simplify(&subst_sym(&g, "x", &expr(x0))))
            .unwrap_or_else(|| panic!("{f} [{t}] at {x0} has a value"))
    }

    #[test]
    fn closed_forms_are_the_map_and_its_inverse_at_whole_steps() {
        for f in [
            "2x+1",
            "x+3",
            "-x/3+1",
            "(1+i)*x+2",
            "x^2",
            "3x^2",
            "x^3/2",
            "sqrt(x)",
            "x^2+2x",
            "x^2-2x+2",
            "x^2-2",
            "2x^2-1",
            "4x*(1-x)",
            "x^3+3x",
            "4x^3-3x",
            "x^3+3x^2+3x",
            "x^5+5x^3+5x",
            "8x^4-8x^2+1",
            "16x^5-20x^3+5x",
            "x^4+4x^3+6x^2+4x",
            "1/(1+x)",
            "(2x+1)/(x+3)",
            "x/(2+x)",
            "x/(1-x)",
            "(3x-1)/(x+1)",
        ] {
            for x0 in ["13/10", "7/5+3i/10"] {
                let z = number(x0);
                let once = image(f, &z);
                let twice = image(f, &once);
                let name = |t: &str| format!("{f} [{t}] at {x0}");
                assert_same(&name("0"), &closed(f, "0", x0), &z);
                assert_same(&name("1"), &closed(f, "1", x0), &once);
                assert_same(&name("2"), &closed(f, "2", x0), &twice);
                // a symbolic count of steps, then the number
                let g = symbolic(&expr(f), "x", &expr("t")).expect("a closed form");
                let general = |t: &str| {
                    let e = simplify(&subst_sym(&subst_sym(&g, "t", &expr(t)), "x", &expr(x0)));
                    plain_value(&e).unwrap_or_else(|| panic!("{f} [{t}] at {x0}: {e}"))
                };
                assert_same(&name("t = 1"), &general("1"), &once);
                assert_same(&name("t = 2"), &general("2"), &twice);
                // halves, thirds and steps that are not real go together
                let half = general("1/2");
                assert_same(&name("1/2"), &closed(f, "1/2", x0), &half);
                let back = image(f, &closed(f, "-1", x0));
                assert_same(&name("-1"), &back, &z);
            }
        }
    }

    #[test]
    fn closed_forms_by_kind() {
        let shown = |f: &str, t: &str| symbolic(&expr(f), "x", &expr(t)).map(|g| g.to_string());
        assert_eq!(shown("2x+1", "t").as_deref(), Some("x*2^t + 2^t - 1"));
        assert_eq!(shown("x+3", "t").as_deref(), Some("3t + x"));
        assert_eq!(shown("x^2", "t").as_deref(), Some("x^(2^t)"));
        assert_eq!(
            shown("x^2+2x", "1/2").as_deref(),
            Some("(x + 1)^(sqrt(2)) - 1")
        );
        assert_eq!(
            shown("2x^2-1", "1/2").as_deref(),
            Some("cosh(sqrt(2)*acosh(x))")
        );
        assert_eq!(shown("x/(1+x)", "t").as_deref(), Some("x/(t*x + 1)"));
        assert_eq!(shown("e^x", "-1").as_deref(), Some("ln(x)"));
        assert_eq!(
            shown("e^x", "1/2").as_deref(),
            Some("e^^(slog(e, x) + 1/2)")
        );
        assert_eq!(shown("sin(x)", "2").as_deref(), Some("sin(sin(x))"));
        assert_eq!(shown("sin(x)", "-1").as_deref(), Some("asin(x)"));
        assert_eq!(
            shown("x^3+3x", "t").as_deref(),
            Some("2sinh(3^t*asinh(x/2))")
        );
        assert_eq!(shown("4x^3-3x", "t").as_deref(), Some("cosh(3^t*acosh(x))"));
        assert_eq!(
            shown("x^3+3x^2+3x", "t").as_deref(),
            Some("(x + 1)^(3^t) - 1")
        );
        // no closed form: a step that is not whole of a map of no known kind
        for f in [
            "sin(x)", "x^2+1", "x^3+x", "cos(x)", "x^2+x", "e^x-1", "x^4+x",
        ] {
            assert_eq!(shown(f, "1/2"), None, "{f}");
            assert_eq!(shown(f, "t"), None, "{f}");
        }
        assert_eq!(shown("x^2+1", "0").as_deref(), Some("x"));
        assert_eq!(shown("x^2+1", "2").as_deref(), Some("x^4 + 2x^2 + 2"));
    }

    #[test]
    fn series_of_an_iterate() {
        // e^x - 1: the half iterate is x + x^2/4 + x^3/48 + 0 x^4 + x^5/3840 - 7 x^6/92160 + ...
        let mut a = Vec::new();
        let mut factorial = Float::with_val(BITS, 1);
        for k in 1..=9u32 {
            factorial *= k;
            a.push(Complex::with_val(
                BITS,
                (Float::with_val(BITS, factorial.recip_ref()), 0),
            ));
        }
        let zero = Complex::new(BITS);
        let half = iterate_series(&a, &ratio(1, 2), &zero, BITS).expect("a series");
        let want = [
            (1, 1),
            (1, 4),
            (1, 48),
            (0, 1),
            (1, 3840),
            (-7, 92160),
            (1, 645120),
            (53, 3440640),
            (-281, 30965760),
        ];
        for (k, (n, d)) in want.into_iter().enumerate() {
            let far = abs(&Complex::with_val(BITS, &half[k] - ratio(n, d)));
            assert!(far < 1e-100, "x^{}: {} is not {n}/{d}", k + 1, half[k]);
        }
        // t = -1 is the inverse, ln(1 + x) = x - x^2/2 + x^3/3 - ...
        let inverse = iterate_series(&a, &real(-1.0), &zero, BITS).expect("a series");
        for (k, c) in inverse.iter().enumerate() {
            let sign = if k % 2 == 0 { 1 } else { -1 };
            let far = abs(&Complex::with_val(BITS, c - ratio(sign, k as i64 + 1)));
            assert!(far < 1e-100, "x^{}: {c}", k + 1);
        }
        // t = 2 is the series of e^(e^x - 1) - 1: 1, 1, 5/6, 5/8, ...
        let twice = iterate_series(&a, &real(2.0), &zero, BITS).expect("a series");
        for (k, (n, d)) in [(1, 1), (1, 1), (5, 6), (5, 8), (13, 30)]
            .into_iter()
            .enumerate()
        {
            let far = abs(&Complex::with_val(BITS, &twice[k] - ratio(n, d)));
            assert!(far < 1e-100, "x^{}: {} is not {n}/{d}", k + 1, twice[k]);
        }
        // a slope that is not 1: 2x + x^2 = (1 + x)^2 - 1, and its iterate is (1 + x)^(2^t) - 1
        let mut b = vec![Complex::new(BITS); 8];
        b[0] = real(2.0);
        b[1] = real(1.0);
        for t in [
            ratio(1, 2),
            ratio(-1, 3),
            Complex::with_val(BITS, (0.3, -0.7)),
        ] {
            let log = real(2.0).ln();
            let got = iterate_series(&b, &t, &log, BITS).expect("a series");
            let power = Complex::with_val(BITS, &t * &log).exp();
            // the binomial coefficients of the power 2^t
            let mut want = Complex::with_val(BITS, 1);
            for (k, c) in got.iter().enumerate() {
                want *= Complex::with_val(BITS, &power - k as u32);
                want /= k as u32 + 1;
                let far = abs(&Complex::with_val(BITS, c - &want));
                assert!(far < 1e-100, "x^{}: {c} is not {want}", k + 1);
            }
        }
        // powers of the slope that meet: no series
        let mut c = vec![Complex::new(BITS); 4];
        c[0] = real(-1.0);
        c[1] = real(1.0);
        assert!(
            iterate_series(&c, &ratio(1, 2), &Complex::with_val(BITS, (0, 3.1)), BITS).is_none()
        );
    }

    #[test]
    fn taylor_coefficients_from_a_circle() {
        let f = expr("sin(x)");
        let map = Map::new(&f, "x", BITS);
        let a =
            taylor(&map, &Complex::new(BITS), &Float::with_val(BITS, 0.5), 9).expect("a series");
        let mut factorial = Float::with_val(BITS, 1);
        for (k, c) in a.iter().enumerate() {
            factorial *= k as u32 + 1;
            let want = match k % 4 {
                0 => Float::with_val(BITS, factorial.recip_ref()),
                2 => -Float::with_val(BITS, factorial.recip_ref()),
                _ => Float::new(BITS),
            };
            let far = abs(&Complex::with_val(
                BITS,
                c - Complex::with_val(BITS, (want, 0)),
            ));
            assert!(far < 1e-100, "x^{}: {c}", k + 1);
        }
        assert!(
            holds(
                &map,
                &Complex::new(BITS),
                &a[..],
                &Float::with_val(BITS, 0.5)
            ) || a.len() < 12
        );
        // around another point: e^x at 1 is e + e h + e h^2/2 + ..., and the first term is e h
        let g = expr("e^x-e+1");
        let map = Map::new(&g, "x", BITS);
        let one = real(1.0);
        let b = taylor(&map, &one, &Float::with_val(BITS, 0.25), 6).expect("a series");
        let e = real(1.0).exp();
        let mut factorial = Float::with_val(BITS, 1);
        for (k, c) in b.iter().enumerate() {
            factorial *= k as u32 + 1;
            let want = Complex::with_val(BITS, &e / &factorial);
            assert!(gap(c, &want) < 1e-100, "h^{}: {c}", k + 1);
        }
        let (degree, lead) = first_term(&map, &one, 12).expect("a first term");
        assert_eq!(degree, 1);
        assert!(gap(&lead, &e) < 1e-100);
        // the first term of a map that is flat at its fixed point
        let flat = expr("x^3*(1+x)");
        let (degree, lead) =
            first_term(&Map::new(&flat, "x", BITS), &Complex::new(BITS), 12).expect("a term");
        assert_eq!(degree, 3);
        assert!(gap(&lead, &real(1.0)) < 1e-100);
        // no series around a point where the map has none
        let root = expr("sqrt(x)");
        let map = Map::new(&root, "x", BITS);
        let found = taylor(&map, &Complex::new(BITS), &Float::with_val(BITS, 0.5), 24);
        assert!(found.is_none_or(|a| !holds(
            &map,
            &Complex::new(BITS),
            &a,
            &Float::with_val(BITS, 0.5)
        )));
    }

    #[test]
    fn small_helpers() {
        // noise next to the other part goes, a small number of its own stays
        let noisy = Complex::with_val(BITS, (0.75, 1e-110));
        assert!(settled(noisy).imag().is_zero());
        let small = Complex::with_val(BITS, (0.0, 1e-90));
        assert!(!settled(small).imag().is_zero());
        assert!(settled(Complex::with_val(BITS, (0.0, 1e-110))).is_zero());
        let large = Complex::with_val(BITS, (1e40, 1e-70));
        assert!(settled(large).imag().is_zero());
        let both = Complex::with_val(BITS, (0.75, 1e-30));
        assert!(!settled(both).imag().is_zero());
        assert_eq!(exponent(&Complex::new(BITS)), None);
        assert_eq!(exponent(&real(1.0)), Some(1));
        assert_eq!(exponent(&real(0.25)), Some(-1));
        assert!(negligible(&real(1e-40), 39.0));
        assert!(!negligible(&real(1e-40), 41.0));
        assert!(negligible(&Complex::new(BITS), 1e9));
        assert_eq!(brief(&real(0.5)), "0.5");
        assert_eq!(brief(&Complex::with_val(BITS, (0.5, 1e-40))), "0.5");
        assert_eq!(brief(&Complex::with_val(BITS, (1e-60, 0))), "1e-60");
        assert_eq!(brief(&Complex::with_val(BITS, (-0.25, 2))), "-0.25 + 2i");
        // the logarithm of a slope is the principal one of the map that is iterated
        let f = expr("-x/2");
        let map = Map::new(&f, "x", BITS);
        let log = map.log_slope(&real(-0.5));
        assert!(
            (log.imag().to_f64() - std::f64::consts::PI).abs() < 0.01,
            "{log}"
        );
        let back = Local {
            of: Map::new(&f, "x", BITS),
            branch: None,
            at: Some((Complex::new(BITS), real(-0.5))),
        };
        // the inverse has the slope -2, and stands for the map with the slope -1/2
        let log = back.log_slope(&real(-2.0));
        assert!(
            gap(&log, &Complex::with_val(BITS, -map.log_slope(&real(-0.5)))) < 1e-100,
            "{log}"
        );
    }

    #[test]
    fn steps_back_keep_the_fixed_point() {
        let f = expr("x/2+x^2/4");
        let map = Map::new(&f, "x", BITS);
        let (zero, half) = (Complex::new(BITS), real(0.5));
        // the branch -1 + sqrt(1 + 4y), not -1 - sqrt(1 + 4y)
        for y in [0.3f64, -0.2, 2.5] {
            let u = map
                .unstep(&real(y), &zero, &half, None)
                .expect("a step back");
            let want = Complex::with_val(BITS, real(y) * 4u32 + 1u32).sqrt() - 1u32;
            assert!(gap(&u, &want) < 1e-100, "{y}: {u}");
        }
        // past the value at the point where the slope is 0 there is no such branch on the line
        assert!(map.unstep(&real(-0.3), &zero, &half, None).is_none());
        // off the line it goes on
        let y = Complex::with_val(BITS, (-0.3, 0.01));
        let u = map.unstep(&y, &zero, &half, None).expect("a step back");
        assert!(gap(&map.value(&u).expect("a value"), &y) < 1e-100);
        // Newton's method with a ladder of precisions
        let g = expr("e^x-1");
        let map = Map::new(&g, "x", BITS);
        let u = map.solve(&real(1.0), &real(0.5)).expect("a solution");
        assert!(gap(&u, &real(2.0).ln()) < 1e-110, "{u}");
        // a step back at a point where the slope is 0: the principal root
        let h = expr("x^2+1");
        let map = Map::new(&h, "x", BITS);
        let u = map
            .step_back(&real(1.25), &zero, &real(1.0))
            .expect("a step back");
        assert!(gap(&u, &real(0.5)) < 1e-100, "{u}");
        let u = map
            .step_back(&real(0.75), &zero, &real(1.0))
            .expect("a step back");
        assert!(gap(&u, &Complex::with_val(BITS, (0, 0.5))) < 1e-100, "{u}");
    }

    #[test]
    fn orbit_that_ends_on_the_fixed_point() {
        let f = expr("sin(x)/2");
        let map = Map::new(&f, "x", 234);
        let pi = Complex::with_val(234, (Float::with_val(234, Constant::Pi), 0));
        let zero = Complex::new(234);
        let next = map.value(&pi).expect("a value");
        assert!(!next.is_zero());
        // pi to all its bits lands on 0 up to the noise; a point next to it does not
        assert_eq!(carried(&map, &zero, &[pi.clone(), next], 234).0, Some(1));
        let beside = Complex::with_val(234, &pi - Float::with_val(234, 1e-40));
        let next = map.value(&beside).expect("a value");
        let (landed, lost) = carried(&map, &zero, &[beside, next], 234);
        assert_eq!(landed, None);
        assert!((120..=140).contains(&lost), "{lost}");
        let far = Complex::with_val(234, 1);
        let next = map.value(&far).expect("a value");
        let (landed, lost) = carried(&map, &zero, &[far, next], 234);
        assert!(landed.is_none() && lost <= 2, "{landed:?} {lost}");
        // and the iterate is the fixed point, with a word about it
        for (f, x0, p) in [
            ("sin(x)/2", "pi", 0.0),
            ("sin(x)", "pi", 0.0),
            ("x/2+x^2/4", "-2", 0.0),
            ("gamma(x)", "3", 1.0),
            ("x^2+x^3", "-1", 0.0),
            ("sin(x)^2", "pi", 0.0),
        ] {
            for t in ["1/2", "1/3", "-1/2", "3/10-7i/10"] {
                let (v, how) =
                    run(f, t, x0, None).unwrap_or_else(|e| panic!("{f} [{t}] at {x0}: {e}"));
                assert!(
                    abs(&Complex::with_val(BITS, &v - real(p))) < 1e-60,
                    "{f} [{t}] at {x0}: {v}"
                );
                assert!(
                    how.contains("the point is not on the branch"),
                    "{f} [{t}] at {x0}: {how}"
                );
            }
        }
    }

    /// Maps with a point to start at, for the laws that every way to an iterate keeps.
    const FLOWS: &[(&str, &str, &str)] = &[
        ("attracting", "cos(x)", "1"),
        ("attracting", "cos(x)", "1/5+3i/10"),
        ("attracting", "x/2+x^2/4", "3/10"),
        ("attracting", "sin(x)/2", "1"),
        ("attracting", "(x^2+1)/3", "1"),
        ("attracting", "(1/2+i/3)*x+x^2", "1/10"),
        ("attracting", "sqrt(2)^x", "1"),
        ("attracting", "-x/2+x^3", "3/10"),
        ("attracting", "9/10*atan(x)", "1"),
        ("slope 1", "e^x-1", "-1"),
        ("slope 1", "e^x-1", "1"),
        ("slope 1", "x+x^2", "-3/10"),
        ("slope 1", "ln(1+x)", "1"),
        ("slope 1", "x*e^x", "1/2"),
        ("slope 1", "x-x^3", "1/2"),
        ("slope 1", "x+1/x", "2"),
        ("Boettcher", "x^2+x^3", "3/10"),
        ("Boettcher", "sin(x)^2", "1/2"),
        ("Boettcher", "(x-1)^2*(x+1)+1", "13/10"),
        ("Boettcher", "x^3*(1+x)", "1/2"),
        ("Boettcher", "5x^4+x^6", "3/10"),
        ("Boettcher", "x^2+1", "1"),
        ("Boettcher", "x^2+1", "5/2+i"),
        ("Boettcher", "x^3+2x", "2"),
        ("Boettcher", "2x^2-3x+4", "3"),
        ("Boettcher", "(x^3+1)/(x+1)", "3"),
        ("repelling", "2sinh(x)", "1"),
        ("repelling", "2x+sin(x)", "1"),
        ("repelling", "x^2-1", "1/2"),
        ("repelling", "-2sinh(x)", "3/10"),
        ("of size 1", "e^(i)*x+x^2", "1/20"),
        ("of size 1", "e^(i*sqrt(2))*sin(x)", "2/25"),
        ("infinity", "2x+1", "3/10"),
        ("infinity", "x/3+2", "1+i"),
    ];

    #[test]
    fn each_way_tells_how() {
        for (kind, f, x0) in FLOWS {
            let (_, how) = run(f, "1/2", x0, None).unwrap_or_else(|e| panic!("{f} at {x0}: {e}"));
            let told = match *kind {
                "attracting" => how.starts_with("regular iteration at the attracting fixed point"),
                "slope 1" => how.contains("(slope 1)") || how.contains("(slope 1 in 1/x)"),
                "Boettcher" => how.starts_with("Boettcher's coordinate at"),
                "repelling" => how.starts_with("regular iteration at the repelling fixed point"),
                "of size 1" => how.contains("of size 1)"),
                _ => {
                    how.starts_with("regular iteration at") && how.contains("fixed point")
                        || how.starts_with("regular iteration at infinity")
                }
            };
            assert!(told, "{f} at {x0}: {how}");
        }
    }

    #[test]
    fn halves_and_thirds_make_the_map() {
        for (_, f, x0) in FLOWS {
            let start = number(x0);
            let want = image(f, &start);
            let half = from(f, "1/2", &start, None);
            assert_same(
                &format!("{f}: two halves at {x0}"),
                &from(f, "1/2", &half, None),
                &want,
            );
            let third = from(f, "1/3", &start, None);
            let two = from(f, "1/3", &third, None);
            assert_same(
                &format!("{f}: three thirds at {x0}"),
                &from(f, "1/3", &two, None),
                &want,
            );
            assert_same(
                &format!("{f}: two thirds at {x0}"),
                &two,
                &from(f, "2/3", &start, None),
            );
            // a step and a half is the map after the half, and the half after the map
            let more = from(f, "3/2", &start, None);
            assert_same(&format!("{f}: 3/2 at {x0}"), &more, &image(f, &half));
            assert_same(
                &format!("{f}: 1/2 after the map at {x0}"),
                &from(f, "1/2", &want, None),
                &more,
            );
        }
    }

    #[test]
    fn steps_add_up() {
        for (kind, f, x0) in FLOWS {
            let start = number(x0);
            for (s, t, sum) in [
                ("1/4", "1/2", "3/4"),
                ("3/10", "9/20", "3/4"),
                ("7/4", "3/4", "5/2"),
                ("1/7", "2/7", "3/7"),
                ("1/5+i/10", "1/5-i/10", "2/5"),
            ] {
                // a count of steps that is not real leaves the disk of a rotation
                if *kind == "of size 1" && s.contains('i') {
                    continue;
                }
                let first = from(f, t, &start, None);
                let both = from(f, s, &first, None);
                let name = format!("{f}: [{s}] after [{t}] at {x0}");
                assert_same(&name, &both, &from(f, sum, &start, None));
            }
        }
    }

    #[test]
    fn steps_back_undo_steps() {
        for (kind, f, x0) in FLOWS {
            let start = number(x0);
            for t in ["1/2", "1/3", "1/4+i/5"] {
                if *kind == "of size 1" && t.contains('i') {
                    continue;
                }
                let there = from(f, t, &start, None);
                let back = from(f, &format!("-({t})"), &there, None);
                assert_same(&format!("{f}: [-({t})] after [{t}] at {x0}"), &back, &start);
            }
            // nothing is the point itself
            assert_same(
                &format!("{f}: [0] at {x0}"),
                &from(f, "0", &start, None),
                &start,
            );
        }
    }

    #[test]
    fn ways_agree_with_closed_forms() {
        // the fixed point that the orbit runs into is the one of the closed form
        // (the closed form of 4x(1 - x) is the iterate at 0, and the formula for the inverse of
        // that map runs into 3/4)
        for (f, x0, p) in [
            ("2x+1", "3/10", None),
            ("2x+1", "-2+i", None),
            ("x/2+1", "3", None),
            ("-x/3+1", "2", None),
            ("(1+i)*x+2", "3/10", None),
            ("x^2", "3/10", None),
            ("x^2", "3", None),
            ("x^2", "1/2+i/3", None),
            ("3x^2", "1/10", None),
            ("3x^2", "2", None),
            ("x^3/2", "1/2", None),
            ("x^3/2", "3", None),
            ("x^2+2x", "-1/2", None),
            ("x^2-2x+2", "3/2", None),
            ("x^2-2", "3", None),
            ("x^2-2", "5/2+i", None),
            ("2x^2-1", "3", None),
            ("4x^3-3x", "2", None),
            ("x^3+3x", "2", None),
            ("x^3+3x", "1+i", None),
            ("x^3+3x^2+3x", "-1/2", None),
            ("x^5+5x^3+5x", "1", None),
            ("1/(1+x)", "1", None),
            ("(2x+1)/(x+3)", "1", None),
            ("x/(2+x)", "3", None),
            ("x/(1-x)", "-3", None),
            ("4x*(1-x)", "3/10", Some("0")),
            ("x^3+3x", "1/2", Some("0")),
        ] {
            for t in ["1/2", "1/3", "5/2", "-1/2", "3/10-7i/10"] {
                // the principal power of a negative number leaves the reals: no way back there
                let got = match run(f, t, x0, p) {
                    Ok((v, _)) => v,
                    Err(e) => {
                        assert!(
                            e.contains("past a point") || e.contains("leaves the basin"),
                            "{f} [{t}] at {x0}: {e}"
                        );
                        continue;
                    }
                };
                assert_same(&format!("{f} [{t}] at {x0}"), &got, &closed(f, t, x0));
            }
        }
    }

    #[test]
    fn known_values() {
        let near = |got: &Complex, want: &str| {
            let want = number(want);
            assert!(gap(got, &want) < 1e-30, "{got} is not {want}");
        };
        near(
            &at("sin(x)", "1/2", "1"),
            "0.90870842974336355496561124039192",
        );
        near(
            &at("e^x-1", "1/2", "1"),
            "1.2710274138899515214246518766723",
        );
        near(
            &at("x^2+1", "1/2", "1"),
            "1.4122853525689035276835226679979",
        );
        near(
            &at("cos(x)", "1/2", "1"),
            "0.75308905103837029464247966234978 + 0.21095439744221607041226043321636i",
        );
        // sin(x)^2 [1/2] at 1 is real, and the half iterate of x^2 at infinity is x^sqrt(2)
        let noise = |z: &Complex| abs(&Complex::with_val(BITS, (0, z.imag()))) < 1e-60;
        assert!(noise(&at("sin(x)^2", "1/2", "1")));
        near(&at("x^2", "1/2", "3"), "3^sqrt(2)");
        // a point at a fixed point stays
        near(
            &at(
                "cos(x)",
                "1/2",
                "0.73908513321516064165531208767387340401341175890076",
            ),
            "0.73908513321516064165531208767387340401341175890076",
        );
        assert!(at("x/2+x^2/4", "1/3", "0").is_zero());
        assert!(at("x^2+x^3", "1/3", "0").is_zero());
        near(&at("x^2-1", "1/2", "(1+sqrt(5))/2"), "(1+sqrt(5))/2");
    }

    #[test]
    fn fixed_point_that_is_asked_for() {
        // the same map at each of its fixed points
        let start = number("5");
        let low = from("sqrt(2)^x", "1/2", &number("1"), Some("2"));
        assert_same(
            "sqrt(2)^x at 2",
            &from("sqrt(2)^x", "1/2", &low, Some("2")),
            &image("sqrt(2)^x", &number("1")),
        );
        let high = from("sqrt(2)^x", "1/2", &start, Some("4"));
        assert_same(
            "sqrt(2)^x at 4",
            &from("sqrt(2)^x", "1/2", &high, Some("4")),
            &image("sqrt(2)^x", &start),
        );
        // between the two fixed points the two half iterates are close, and not the same
        let between = number("3");
        let (a, b) = (
            from("sqrt(2)^x", "1/2", &between, Some("2")),
            from("sqrt(2)^x", "1/2", &between, Some("4")),
        );
        let apart = gap(&a, &b);
        assert!(apart > 1e-27 && apart < 1e-20, "{a} and {b}: {apart:e}");
        // e^x at its fixed point in the upper half of the plane
        let p = "3/10+13i/10";
        let half = from("e^x", "1/2", &number("1/2"), Some(p));
        assert_same(
            "e^x at its fixed point",
            &from("e^x", "1/2", &half, Some(p)),
            &number("e^(1/2)"),
        );
        let (_, how) = run("e^x", "1/2", "1/2", Some(p)).expect("a value");
        assert!(how.contains("0.3181315052 + 1.337235701i"), "{how}");
        // a polynomial at infinity, at an attracting point and at a repelling one
        let (v, how) = run("x^2+1", "1/2", "1", Some("inf")).expect("a value");
        assert!(
            how.starts_with("Boettcher's coordinate at infinity"),
            "{how}"
        );
        assert_same("x^2+1 at infinity", &v, &at("x^2+1", "1/2", "1"));
        let (v, how) = run("x^2+1", "1/2", "1", Some("1/2+9i/10")).expect("a value");
        assert!(
            how.contains("repelling fixed point 0.5 + 0.8660254038i"),
            "{how}"
        );
        assert_same(
            "x^2+1 at a repelling point",
            &from("x^2+1", "1/2", &v, Some("1/2+9i/10")),
            &real(2.0),
        );
        // x^2 - 1 from the left of 0: real at one fixed point, with a word, and not at the other
        let (v, how) = run("x^2-1", "1/2", "-3/10", None).expect("a value");
        let noise = |z: &Complex| abs(&Complex::with_val(BITS, (0, z.imag()))) < 1e-60;
        assert!(
            noise(&v) && how.contains("1.618033989") && how.contains("past a fold"),
            "{v}: {how}"
        );
        let (w, how) = run("x^2-1", "1/2", "-3/10", Some("-3/5")).expect("a value");
        assert!(
            !noise(&w) && how.contains("-0.6180339887") && !how.contains("fold"),
            "{w}: {how}"
        );
        assert_same(
            "x^2-1 at -0.618",
            &from("x^2-1", "1/2", &w, Some("-3/5")),
            &number("-91/100"),
        );
        // a closed form is not used with a fixed point: 2x at 0 and at infinity is the same map
        let (v, how) = run("2x", "1/2", "3", Some("0")).expect("a value");
        assert!(how.contains("repelling fixed point 0 (slope 2)"), "{how}");
        assert_same("2x at 0", &v, &number("3*sqrt(2)"));
        let (v, how) = run("2x", "1/2", "3", Some("inf")).expect("a value");
        assert!(how.contains("at infinity (slope 0.5 in 1/x)"), "{how}");
        assert_same("2x at infinity", &v, &number("3*sqrt(2)"));
        // what is not there
        let e = run("e^x", "1/2", "1", Some("inf")).expect_err("no fixed point");
        assert!(e.contains("does not run to infinity"), "{e}");
        let e = run("x^2+1", "1/2", "1", Some("1/2-9i/10")).expect_err("the other side");
        assert!(
            e.contains("past a point") || e.contains("does not run into this fixed point"),
            "{e}"
        );
    }

    #[test]
    fn points_off_the_branch_and_what_is_said() {
        // the iterate at the point that the branch from the fixed point has for the same orbit
        let (v, how) = run("x/2+x^2/4", "1/2", "-17/10", None).expect("a value");
        assert!(how.contains("this is the iterate at -0.3"), "{how}");
        assert_same("x/2+x^2/4 at -1.7", &v, &at("x/2+x^2/4", "1/2", "-3/10"));
        let (v, how) = run("cos(x)", "1/3", "4", None).expect("a value");
        assert!(how.contains("this is the iterate at 0.8584073464"), "{how}");
        assert_same("cos at 4", &v, &at("cos(x)", "1/3", "2pi-4-pi+pi"));
        let (v, how) = run("x^2+x^3", "1/2", "-9/10", None).expect("a value");
        assert!(
            how.contains("this is the iterate at -0.3541381265"),
            "{how}"
        );
        // which is the other point that the map takes to 81/1000
        let cubic = expr("x^2+x^3");
        let other = Map::new(&cubic, "x", BITS)
            .solve(&number("81/1000"), &number("-177/500"))
            .expect("a solution");
        assert_same("x^2+x^3 at -0.9", &v, &from("x^2+x^3", "1/2", &other, None));
        // steps back that would go past a point where the slope is 0
        for (f, t, x0) in [
            ("x/2+x^2/4", "-1/2", "-1/2"),
            ("sin(x)/2", "-1/2", "1"),
            ("(x^2+1)/3", "-1/2", "0"),
            ("x^2+1", "-1/2", "1/10"),
        ] {
            let e = run(f, t, x0, None).expect_err("no such iterate");
            assert!(
                e.starts_with("the iterate would be past a point where the slope of the map is 0"),
                "{f}: {e}"
            );
        }
        // out of the basin, out of the disk, out of reach
        let e = run("x^2+1", "i", "-2", None).expect_err("no such iterate");
        assert!(e.starts_with("the iterate leaves the basin"), "{e}");
        let e = run("e^(pi*i*(sqrt(5)-1))*x+x^2", "i", "1/20", None).expect_err("no such iterate");
        assert!(
            e.starts_with("the iterate would be past the edge of the disk"),
            "{e}"
        );
        let e = run("x^2+x", "1/2", "1/100", None).expect_err("too far");
        assert!(
            e.contains("has no value at the point") && e.contains("fifth argument"),
            "{e}"
        );
        assert!(run("x^2+x", "1/2", "1/100", Some("0")).is_ok());
        let e = run("2sinh(x)", "161/4", "3/10", None).expect_err("too large");
        assert!(e.contains("too large"), "{e}");
        // a slope that is a root of 1 is no rotation, and the value is that of another fixed point
        let (_, how) = run("x^3-x", "1/2", "3/10", None).expect("a value");
        assert!(
            how.contains("repelling fixed point 1.414213562") && how.contains("past a fold"),
            "{how}"
        );
        let e = run("-x+x^2", "1/2", "1/10", Some("0")).expect_err("a root of 1");
        assert!(e.contains("root of 1"), "{e}");
        // a value whose iterates do not carry it on
        let (_, how) = run("x^2-1", "3/2", "1/2", None).expect("a value");
        assert!(
            how.contains("its iterates do not carry this one on"),
            "{how}"
        );
        // a principal power that does not go along with the map
        let (_, how) = run("x^2+1", "1/2", "-2", None).expect("a value");
        assert!(how.contains("does not commute with the map"), "{how}");
    }

    #[test]
    fn digits_that_the_way_takes() {
        // next to the fixed point 0 the iterate is z + t z^2/2 for e^z - 1
        for (x0, t) in [
            ("1/10^80", "1/2"),
            ("-1/10^80", "1/3"),
            ("1/10^40", "5/2"),
            ("i/10^45", "1/2"),
        ] {
            let (z, s) = (number(x0), number(t));
            let want = Complex::with_val(BITS, &z + Complex::with_val(BITS, &s * &z) * &z / 2u32);
            assert_same(&format!("e^x-1 [{t}] at {x0}"), &at("e^x-1", t, x0), &want);
        }
        // and lambda^t z for a map with the slope lambda
        for (f, x0, lead) in [
            ("x/2+x^2/4", "1/10^60", "sqrt(1/2)/10^60"),
            ("ln(1+x)/2", "-1/10^80", "-sqrt(1/2)/10^80"),
            ("(e^x-1)/3", "i/10^120", "sqrt(1/3)*i/10^120"),
            ("2sinh(x)", "1/10^50", "sqrt(2)/10^50"),
            ("2x+e^x-1", "-1/10^70", "-sqrt(3)/10^70"),
        ] {
            let got = at(f, "1/2", x0);
            assert_same(&format!("{f} at {x0}"), &got, &number(lead));
        }
        // a point next to one that the map takes to the fixed point
        let got = at("sin(x)/2", "1/2", "pi-1/10^60");
        assert_same("sin(x)/2 next to pi", &got, &number("sqrt(1/2)/10^60"));
        // many steps back from next to the fixed point, and forward again
        let start = number("1/10^50");
        let far = from("x/2+x^2/4", "-150", &start, None);
        assert!(gap(&far, &number("2^150/10^50")) < 1e-4, "{far}");
        assert_same(
            "150 steps back and forward",
            &from("x/2+x^2/4", "150", &far, None),
            &start,
        );
        let start = number("37/50");
        let far = from("cos(x)", "-61/2", &start, None);
        assert_same(
            "61/2 steps back and forward",
            &from("cos(x)", "61/2", &far, None),
            &start,
        );
        // more digits than 32 are the same number
        for (f, t, x0) in [
            ("cos(x)", "1/2", "1"),
            ("e^x-1", "1/2", "1"),
            ("x^2+1", "1/3", "1"),
            ("2sinh(x)", "1/2", "1"),
            ("x^2+x^3", "i/2", "3/10"),
            ("e^(i)*x+x^2", "1/2", "1/20"),
        ] {
            let short = at(f, t, x0);
            let long = value(&expr(f), "x", &expr(t), &expr(x0), None, 70)
                .expect("a value")
                .0;
            assert!(
                gap(&short, &long) < 1e-31,
                "{f} [{t}] at {x0}: {short} and {long}"
            );
        }
    }

    #[test]
    fn no_fixed_point_no_iterate() {
        // e^x + x has no fixed point at all
        let e = run("x+e^x", "1/2", "1", None).expect_err("no way");
        assert_eq!(e, NO_POINT);
        let e = run("e^x", "1/2", "1", Some("5")).expect_err("no fixed point there");
        assert!(e.contains("no fixed point next to 5"), "{e}");
    }
}
