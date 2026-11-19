//! Integrals over several variables and over regions; areas, volumes and lengths.

use crate::canon::{add, div, mul, neg, pow, powi, sqrt, sub, subst_sym};
use crate::expr::Expr;
use crate::funcs::func;
use crate::limit::{self, Dir};
use crate::numeric::{self, Env};
use crate::simplify::simplify;
use crate::{assume, complex, integrate, matrix, quad};
use rug::{Complex, Float};
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct Range {
    pub var: String,
    pub lo: Expr,
    pub hi: Expr,
}

impl Range {
    pub fn new(var: &str, lo: Expr, hi: Expr) -> Range {
        Range {
            var: var.to_string(),
            lo,
            hi,
        }
    }
}

/// The variables of an integral over a region.
pub enum Vars {
    /// One complex variable `z = x + i y` over a region of the plane.
    Complex(String),
    Real(Vec<String>),
}

type Value = Result<(Expr, bool), String>;

/// `|a b| = |a| |b|` and `|a^k| = |a|^k`: every absolute value around as little as possible.
fn spread_abs(f: &Expr) -> Expr {
    if !f.has_fn("abs") {
        return f.clone();
    }
    crate::canon::map_bottom_up(f, &mut |e| match e.fn_arg("abs") {
        Some(Expr::Mul(parts)) => mul(parts
            .iter()
            .map(|p| spread_abs(&func("abs", vec![p.clone()])))
            .collect()),
        Some(Expr::Pow(b, k)) if assume::is_real(k) => {
            pow(spread_abs(&func("abs", vec![(**b).clone()])), (**k).clone())
        }
        _ => e,
    })
}

/// The integral over one range: `(value, approximate)`.
fn single(f: &Expr, r: &Range) -> Value {
    let f = &spread_abs(f);
    let closed = r.lo.is_const() && r.hi.is_const() && f.free_syms().iter().all(|s| **s == *r.var);
    if closed {
        return integrate::definite(f, &r.var, &r.lo, &r.hi);
    }
    // corners that move with the parameters: only an absolute value of something linear
    let mut corner = None;
    let mut unclear = false;
    f.walk(&mut |e| {
        if let Expr::Fn(n, a) = e
            && a.iter().any(|x| x.has_sym(&r.var))
        {
            match &**n {
                "abs" => {
                    corner.get_or_insert_with(|| a[0].clone());
                }
                "sign" | "floor" | "ceil" | "round" | "trunc" | "frac" | "min" | "max"
                | "heaviside" | "mod" => unclear = true,
                _ => {}
            }
        }
    });
    if unclear {
        return Err("corners depend on parameters".into());
    }
    if let Some(g) = corner {
        let cs = crate::ratsimp::coefficients(&g, &r.var).filter(|c| c.len() == 2);
        let (b, a) = match cs.as_deref() {
            Some([b, a]) => (b, a),
            _ => return Err("corners depend on parameters".into()),
        };
        let s = numeric::sign(a)
            .filter(|s| *s != 0)
            .ok_or("corners depend on parameters")?;
        // the range is cut where the linear argument changes its sign
        let cut = func(
            "min",
            vec![
                func("max", vec![neg(div(b.clone(), a.clone())), r.lo.clone()]),
                r.hi.clone(),
            ],
        );
        let side = |sign: i32| {
            let plain = mul(vec![Expr::int(i64::from(sign)), g.clone()]);
            crate::canon::map_bottom_up(f, &mut |e| match e.fn_arg("abs") {
                Some(inner) if *inner == g => plain.clone(),
                _ => e,
            })
        };
        let (low, _) = single(&side(-s), &Range::new(&r.var, r.lo.clone(), cut.clone()))?;
        let (high, _) = single(&side(s), &Range::new(&r.var, cut, r.hi.clone()))?;
        return Ok((add(vec![low, high]), false));
    }
    let anti = integrate::integrate(f, &r.var).ok_or("no antiderivative")?;
    let at = |p: &Expr| {
        limit::limit(&anti, &r.var, p, Dir::Both)
            .map(|v| v.0)
            .unwrap_or_else(|_| subst_sym(&anti, &r.var, p))
    };
    let value = simplify(&sub(at(&r.hi), at(&r.lo)));
    if value.is_undefined() || value.is_infinite() {
        return Err("the antiderivative has no value at an end".into());
    }
    Ok((value, false))
}

/// Time for the search of a closed form before the numeric value is taken.
const SEARCH: std::time::Duration = std::time::Duration::from_secs(4);

/// Integrand evaluations allowed for one numeric attempt.
const BUDGET: u64 = 500_000;

/// Time allowed for one numeric attempt.
const ATTEMPT: std::time::Duration = std::time::Duration::from_secs(4);

#[allow(clippy::excessive_precision)]
const KRONROD: [(f64, f64, f64); 8] = [
    (
        0.991455371120812639206854697526329,
        0.022935322010529224963732008058970,
        0.0,
    ),
    (
        0.949107912342758524526189684047851,
        0.063092092629978553290700663189204,
        0.129484966168869693270611432679082,
    ),
    (
        0.864864423359769072789712788640926,
        0.104790010322250183839876322541518,
        0.0,
    ),
    (
        0.741531185599394439863864773280788,
        0.140653259715525918745189590510238,
        0.279705391489276667901467771423780,
    ),
    (
        0.586087235467691130294144838258730,
        0.169004726639267902826583426598550,
        0.0,
    ),
    (
        0.405845151377397166906606412076961,
        0.190350578064785409913256402421014,
        0.381830050505118944950369775488975,
    ),
    (
        0.207784955007898467600689403773245,
        0.204432940075298892414161999234649,
        0.0,
    ),
    (
        0.0,
        0.209482141084727828012999174891714,
        0.417959183673469387755102040816327,
    ),
];

/// Nodes in `[-1, 1]` with the weights of the 15 point Kronrod rule and of the 7 point Gauss rule.
fn kronrod_nodes() -> Vec<(f64, f64, f64)> {
    let mut nodes = Vec::new();
    for &(x, wk, wg) in &KRONROD {
        nodes.push((x, wk, wg));
        if x != 0.0 {
            nodes.push((-x, wk, wg));
        }
    }
    nodes
}

/// Adaptive cubature of a double integral with corners or kinks inside: `(value, error)`.
fn adaptive_double(f: &Expr, ranges: &[Range]) -> Option<((f64, f64), f64)> {
    let [outer, inner] = ranges else {
        return None;
    };
    const PREC: u32 = 64;
    let nodes = kronrod_nodes();
    let (ko, ki): (Rc<str>, Rc<str>) = (Rc::from(outer.var.as_str()), Rc::from(inner.var.as_str()));
    let real = |e: &Expr, env: &Env| {
        let v = numeric::evalf_env(e, PREC, env).ok()?;
        v.imag().is_zero().then(|| v.real().to_f64())
    };
    let (a, b) = (real(&outer.lo, &Env::new())?, real(&outer.hi, &Env::new())?);
    if !(a.is_finite() && b.is_finite()) {
        return None;
    }
    // the inner range is mapped to [0, 1]
    let cell = |u0: f64, u1: f64, s0: f64, s1: f64| -> Option<((f64, f64), f64)> {
        let (mut kr, mut ki_sum, mut gr, mut gi) = (0.0, 0.0, 0.0, 0.0);
        let mut env = Env::new();
        for &(x, wkx, wgx) in &nodes {
            let u = 0.5 * (u0 + u1) + 0.5 * (u1 - u0) * x;
            env.insert(ko.clone(), Complex::with_val(PREC, (u, 0)));
            env.remove(&ki);
            let (lo, hi) = (real(&inner.lo, &env)?, real(&inner.hi, &env)?);
            for &(y, wky, wgy) in &nodes {
                let s = 0.5 * (s0 + s1) + 0.5 * (s1 - s0) * y;
                env.insert(ki.clone(), Complex::with_val(PREC, (lo + s * (hi - lo), 0)));
                let v = numeric::evalf_env(f, PREC, &env).ok()?;
                let (re, im) = (v.real().to_f64() * (hi - lo), v.imag().to_f64() * (hi - lo));
                if !(re.is_finite() && im.is_finite()) {
                    return None;
                }
                kr += wkx * wky * re;
                ki_sum += wkx * wky * im;
                gr += wgx * wgy * re;
                gi += wgx * wgy * im;
            }
        }
        let scale = 0.25 * (u1 - u0) * (s1 - s0);
        let err = ((kr - gr).powi(2) + (ki_sum - gi).powi(2)).sqrt() * scale.abs();
        Some(((kr * scale, ki_sum * scale), err))
    };
    let mut cells = vec![(a, b, 0.0, 1.0, cell(a, b, 0.0, 1.0)?)];
    let per_cell = (nodes.len() * nodes.len()) as u64;
    let mut used = per_cell;
    let until = std::time::Instant::now() + ATTEMPT;
    loop {
        let total = cells
            .iter()
            .fold((0.0, 0.0), |t, c| (t.0 + c.4.0.0, t.1 + c.4.0.1));
        let err: f64 = cells.iter().map(|c| c.4.1).sum();
        let size = (total.0 * total.0 + total.1 * total.1).sqrt();
        if err <= 1e-13 * size.max(1e-300)
            || used + 4 * per_cell > BUDGET
            || std::time::Instant::now() > until
        {
            return Some((total, err));
        }
        let worst = cells
            .iter()
            .enumerate()
            .max_by(|x, y| x.1.4.1.total_cmp(&y.1.4.1))
            .map(|(k, _)| k)?;
        let (u0, u1, s0, s1, _) = cells.swap_remove(worst);
        let (um, sm) = (0.5 * (u0 + u1), 0.5 * (s0 + s1));
        for (p0, p1, q0, q1) in [
            (u0, um, s0, sm),
            (um, u1, s0, sm),
            (u0, um, sm, s1),
            (um, u1, sm, s1),
        ] {
            cells.push((p0, p1, q0, q1, cell(p0, p1, q0, q1)?));
        }
        used += 4 * per_cell;
    }
}

fn nested(
    f: &Expr,
    ranges: &[Range],
    env: &mut Env,
    digits: u32,
    left: &Cell<u64>,
    until: std::time::Instant,
) -> Option<Complex> {
    let (first, rest) = ranges.split_first()?;
    let prec = numeric::bits_for(digits) + 64;
    let end = |e: &Expr, env: &Env| {
        let v = numeric::evalf_env(e, prec, env).ok()?;
        v.imag().is_zero().then(|| v.real().clone())
    };
    let (lo, hi) = (end(&first.lo, env)?, end(&first.hi, env)?);
    let key: Rc<str> = Rc::from(first.var.as_str());
    let mut g = |x: &Float| -> Option<Complex> {
        env.insert(key.clone(), Complex::with_val(x.prec(), x));
        if rest.is_empty() {
            if left.get() == 0
                || (left.get().is_multiple_of(64) && std::time::Instant::now() > until)
            {
                return None;
            }
            left.set(left.get() - 1);
            numeric::evalf_env(f, x.prec(), env).ok()
        } else {
            nested(f, rest, env, digits, left, until)
        }
    };
    let value = if lo > hi {
        quad::quad(&mut g, &hi, &lo, digits).map(|v| -v)
    } else {
        quad::quad(&mut g, &lo, &hi, digits)
    };
    env.remove(&key);
    value
}

/// Numeric value of the iterated integral with about `digits` digits, at the points of `env` for
/// the other symbols.
fn numeric_value(f: &Expr, ranges: &[Range], env: &Env, digits: u32) -> Option<Complex> {
    let mut env = env.clone();
    let left = Cell::new(BUDGET);
    nested(
        f,
        ranges,
        &mut env,
        digits,
        &left,
        std::time::Instant::now() + ATTEMPT,
    )
}

fn free_parameters(f: &Expr, ranges: &[Range]) -> Vec<Rc<str>> {
    let mut all = f.free_syms();
    for r in ranges {
        all.extend(r.lo.free_syms());
        all.extend(r.hi.free_syms());
    }
    all.into_iter()
        .filter(|s| ranges.iter().all(|r| r.var != **s))
        .collect()
}

type Rule = Rc<Vec<(Float, Float)>>;

thread_local! {
    static RULES: std::cell::RefCell<std::collections::HashMap<(u32, u32), Rule>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Nodes and weights of the Gauss-Legendre rule of order `n` on `[-1, 1]`.
fn gauss_rule(n: u32, prec: u32) -> Rule {
    if let Some(rule) = RULES.with_borrow(|r| r.get(&(n, prec)).cloned()) {
        return rule;
    }
    let pi = Float::with_val(prec, rug::float::Constant::Pi);
    let mut rule = Vec::with_capacity(n as usize);
    for i in 1..=n.div_ceil(2) {
        let mut x = (Float::with_val(prec, 4 * i - 1) / (4 * n + 2) * &pi).cos();
        let mut slope = Float::new(prec);
        for _ in 0..100 {
            // Legendre polynomial and its derivative at x
            let (mut p0, mut p1) = (Float::with_val(prec, 1), x.clone());
            for k in 2..=n {
                let next = (Float::with_val(prec, &x * &p1) * (2 * k - 1)
                    - Float::with_val(prec, &p0 * (k - 1)))
                    / k;
                p0 = std::mem::replace(&mut p1, next);
            }
            slope = (Float::with_val(prec, &x * &p1) - &p0) * n
                / (Float::with_val(prec, x.square_ref()) - 1u32);
            let step = Float::with_val(prec, &p1 / &slope);
            x -= &step;
            if step.is_zero() || step.get_exp().unwrap_or(0) < -(prec as i32) + 4 {
                break;
            }
        }
        let w = Float::with_val(prec, 2)
            / ((Float::with_val(prec, 1) - Float::with_val(prec, x.square_ref())) * slope.square());
        if 2 * i - 1 == n {
            rule.push((Float::new(prec), w));
        } else {
            rule.push((x.clone(), w.clone()));
            rule.push((-x, w));
        }
    }
    let rule = Rc::new(rule);
    RULES.with_borrow_mut(|r| r.insert((n, prec), rule.clone()));
    rule
}

fn gauss_nested(
    f: &Expr,
    ranges: &[Range],
    env: &mut Env,
    rule: &Rule,
    prec: u32,
) -> Option<Complex> {
    let (first, rest) = ranges.split_first()?;
    let end = |e: &Expr, env: &Env| {
        let v = numeric::evalf_env(e, prec, env).ok()?;
        (v.imag().is_zero() && v.real().is_finite()).then(|| v.real().clone())
    };
    let (lo, hi) = (end(&first.lo, env)?, end(&first.hi, env)?);
    let half = Float::with_val(prec, &hi - &lo) / 2u32;
    let mid = Float::with_val(prec, &hi + &lo) / 2u32;
    let key: Rc<str> = Rc::from(first.var.as_str());
    let mut sum = Complex::new(prec);
    for (x, w) in rule.iter() {
        let u = Float::with_val(prec, &half * x) + &mid;
        env.insert(key.clone(), Complex::with_val(prec, u));
        let v = if rest.is_empty() {
            numeric::evalf_env(f, prec, env).ok()?
        } else {
            gauss_nested(f, rest, env, rule, prec)?
        };
        sum += v * w;
    }
    env.remove(&key);
    Some(sum * half)
}

/// Tensor Gauss-Legendre rules of growing order: the value and the digits two orders agree to.
fn gauss_adaptive(f: &Expr, ranges: &[Range], digits: u32) -> Option<(Complex, u32)> {
    let prec = numeric::bits_for(digits) + 32;
    let most = (BUDGET as f64).powf(1.0 / ranges.len() as f64) as u32;
    let mut previous: Option<Complex> = None;
    let mut best: Option<(Complex, u32)> = None;
    let mut order = 6;
    let until = std::time::Instant::now() + ATTEMPT;
    while order <= most.max(6) {
        let rule = gauss_rule(order, prec);
        let value = gauss_nested(f, ranges, &mut Env::new(), &rule, prec)?;
        if let Some(old) = &previous {
            let diff = Float::with_val(64, Complex::with_val(prec, &value - old).abs_ref());
            let size = Float::with_val(64, value.abs_ref());
            let agreed = if diff.is_zero() {
                digits
            } else if size.is_zero() {
                0
            } else {
                let ratio = Float::with_val(64, &size / &diff).log10();
                ratio.to_f64().clamp(0.0, f64::from(digits)) as u32
            };
            // a smooth integrand gains digits fast: a slow gain means corners or singular points
            let slow = order >= 48 && best.as_ref().is_some_and(|(_, d)| agreed < d + 3);
            best = Some((value.clone(), agreed));
            if agreed >= digits || slow || std::time::Instant::now() > until {
                break;
            }
        }
        previous = Some(value);
        order *= 2;
    }
    best
}

/// A range that reaches infinity is mapped to a finite one.
fn finite_ranges(f: &Expr, ranges: &[Range]) -> (Expr, Vec<Range>) {
    let mut f = f.clone();
    let mut out: Vec<Range> = ranges.to_vec();
    for k in 0..out.len() {
        let (lo, hi) = (out[k].lo.clone(), out[k].hi.clone());
        if !lo.is_infinite() && !hi.is_infinite() {
            continue;
        }
        let s = sym(&out[k].var);
        let one_minus = sub(Expr::one(), s.clone());
        // x = g(s) and dx = g'(s) ds
        let (g, weight, a, b) = if lo.is_infinite() && hi.is_infinite() {
            let d = sub(Expr::one(), square(&s));
            (
                div(s.clone(), d.clone()),
                div(add(vec![Expr::one(), square(&s)]), square(&d)),
                Expr::minus_one(),
                Expr::one(),
            )
        } else if hi.is_infinite() {
            (
                add(vec![lo, div(s.clone(), one_minus.clone())]),
                powi(one_minus, -2),
                Expr::zero(),
                Expr::one(),
            )
        } else {
            (
                sub(hi, div(s.clone(), one_minus.clone())),
                powi(one_minus, -2),
                Expr::zero(),
                Expr::one(),
            )
        };
        let name = out[k].var.clone();
        f = mul(vec![subst_sym(&f, &name, &g), weight]);
        for inner in out.iter_mut().skip(k + 1) {
            inner.lo = subst_sym(&inner.lo, &name, &g);
            inner.hi = subst_sym(&inner.hi, &name, &g);
        }
        out[k] = Range::new(&name, a, b);
    }
    (f, out)
}

fn float_value(v: Complex, digits: u32) -> (Expr, bool) {
    let prec = numeric::bits_for(digits) + 32;
    // noise far below the value is no part of it
    let size = Float::with_val(64, v.abs_ref());
    let tiny = |x: &Float| {
        !x.is_zero() && Float::with_val(64, x.abs_ref()) * 10f64.powi(digits as i32) < size
    };
    let (re, im) = v.into_real_imag();
    let re = if tiny(&re) { Float::new(prec) } else { re };
    let im = if tiny(&im) { Float::new(prec) } else { im };
    (
        numeric::to_float_expr(Complex::with_val(prec, (re, im)), digits),
        true,
    )
}

/// The numeric iterated integral with as many of the digits of the session as it can give.
pub fn numeric_iterated(f: &Expr, ranges: &[Range]) -> Value {
    if !free_parameters(f, ranges).is_empty() {
        return Err(
            "no closed form found; with parameters there is no numeric value either".into(),
        );
    }
    let (f, ranges) = finite_ranges(f, ranges);
    let (f, ranges) = (&f, &ranges[..]);
    let full = numeric::digits();
    let smooth = gauss_adaptive(f, ranges, full);
    if let Some((v, digits)) = &smooth
        && *digits >= full
    {
        return Ok(float_value(v.clone(), full));
    }
    if ranges.len() <= 2
        && let Some(v) = numeric_value(f, ranges, &Env::new(), full)
    {
        return Ok(float_value(v, full));
    }
    // corners or kinks inside: fewer digits by adaptive cubature
    let mut rough = smooth.filter(|(_, d)| *d >= 4);
    if let Some(((re, im), err)) = adaptive_double(f, ranges) {
        let size = (re * re + im * im).sqrt();
        let digits = if size == 0.0 || err == 0.0 {
            13
        } else {
            ((size / err).log10().floor() as i64).clamp(0, 13) as u32
        };
        if digits >= 3 && rough.as_ref().is_none_or(|(_, d)| digits > *d) {
            let chop = |x: f64| if x.abs() <= err { 0.0 } else { x };
            rough = Some((Complex::with_val(64, (chop(re), chop(im))), digits));
        }
    }
    match rough {
        Some((v, digits)) => Ok(float_value(v, digits)),
        None => Err("the numeric integral did not converge".into()),
    }
}

/// Real sample values for the outer variables (inside their ranges) and the parameters.
fn sample(f: &Expr, outer: &[Range], inner: &Range, at: f64) -> Option<Vec<(String, Expr)>> {
    let mut all = vec![inner.clone()];
    all.extend(outer.iter().cloned());
    let mut env = Env::new();
    let mut out = Vec::new();
    let mut put = |env: &mut Env, name: &str, x: Float| {
        env.insert(Rc::from(name), Complex::with_val(128, &x));
        out.push((
            name.to_string(),
            numeric::to_float_expr(Complex::with_val(128, x), 30),
        ));
    };
    for (k, p) in free_parameters(f, &all).iter().enumerate() {
        put(
            &mut env,
            p,
            Float::with_val(128, 0.6180339887 + 0.2360679775 * k as f64),
        );
    }
    for r in outer {
        let end = |e: &Expr, env: &Env| -> Option<Option<Float>> {
            if e.is_infinite() {
                return Some(None);
            }
            let v = numeric::evalf_env(e, 128, env).ok()?;
            v.imag().is_zero().then(|| Some(v.real().clone()))
        };
        let x = match (end(&r.lo, &env)?, end(&r.hi, &env)?) {
            (Some(lo), Some(hi)) => Float::with_val(128, &hi - &lo) * at + lo,
            (Some(lo), None) => lo + at,
            (None, Some(hi)) => hi - at,
            (None, None) => Float::with_val(128, at - 0.5),
        };
        put(&mut env, &r.var, x);
    }
    Some(out)
}

/// Whether the closed form of an inner integral agrees with its numeric value at sample points
/// of the outer variables; `None` when no numeric value can be had.
fn step_confirmed(f: &Expr, r: &Range, value: &Expr, outer: &[Range]) -> Option<bool> {
    let mut seen = None;
    for at in [0.3819660113, 0.7236067977] {
        let point = sample(f, outer, r, at)?;
        let fix = |e: &Expr| {
            point
                .iter()
                .fold(e.clone(), |e, (name, x)| subst_sym(&e, name, x))
        };
        let (g, lo, hi) = (fix(f), fix(&r.lo), fix(&r.hi));
        let rough = integrate::numeric_integral(&g, &r.var, &lo, &hi, 10)?;
        let exact = numeric::evalf(&fix(value), 128).ok()?;
        let diff = Float::with_val(64, Complex::with_val(128, &rough - &exact).abs_ref());
        let scale = Float::with_val(64, exact.abs_ref()) + 1u32;
        if diff > scale * 1e-7 {
            return Some(false);
        }
        seen = Some(true);
    }
    seen
}

fn symbolic(f: &Expr, ranges: &[Range]) -> Value {
    let Some((last, outer)) = ranges.split_last() else {
        return Ok((f.clone(), false));
    };
    let (inner, approx) = single(f, last)?;
    if crate::budget::expired() {
        return Err("no closed form found in time".into());
    }
    if outer.is_empty() {
        return Ok((inner, approx));
    }
    if approx {
        return Err("the inner integral has no closed form".into());
    }
    if step_confirmed(f, last, &inner, outer) == Some(false) {
        return Err("the closed form of the inner integral fails the numeric check".into());
    }
    symbolic(&inner, outer)
}

/// The iterated integral: the first range is the outermost, inner bounds may use the outer
/// variables. `(value, approximate)`.
pub fn iterated(f: &Expr, ranges: &[Range]) -> Value {
    if ranges.len() == 1 {
        let r = &ranges[0];
        if r.lo.is_const() && r.hi.is_const() && f.free_syms().iter().all(|s| **s == *r.var) {
            return integrate::definite(f, &r.var, &r.lo, &r.hi);
        }
    }
    match crate::budget::with_limit(SEARCH, || symbolic(f, ranges)) {
        Ok(done) if !done.1 || ranges.len() == 1 => Ok(done),
        _ => numeric_iterated(f, ranges),
    }
}

/// Like `iterated`, trying the other order of two independent ranges for a closed form.
fn iterated_any_order(f: &Expr, ranges: &[Range]) -> Value {
    let first = iterated(f, ranges);
    if matches!(first, Ok((_, false))) || ranges.len() != 2 {
        return first;
    }
    let independent = ranges.iter().all(|r| {
        ranges
            .iter()
            .all(|o| !r.lo.has_sym(&o.var) && !r.hi.has_sym(&o.var))
    });
    if independent {
        let swapped = [ranges[1].clone(), ranges[0].clone()];
        if let Ok((value, false)) = crate::budget::with_limit(SEARCH, || symbolic(f, &swapped)) {
            return Ok((value, false));
        }
    }
    first
}

/// Fresh variable names; the facts set for them last as long as this value.
struct Names {
    taken: Vec<String>,
    positive: Vec<String>,
}

impl Names {
    fn fresh(&mut self, wish: &str) -> String {
        let mut name = wish.to_string();
        let mut k = 0;
        while self.taken.contains(&name) {
            k += 1;
            name = format!("{wish}{k}");
        }
        self.taken.push(name.clone());
        name
    }

    /// A fresh name for a radius: a positive number.
    fn radius(&mut self, wish: &str) -> String {
        let name = self.fresh(wish);
        assume::set(&name, assume::Facts::positive());
        self.positive.push(name.clone());
        name
    }
}

impl Drop for Names {
    fn drop(&mut self) {
        for p in &self.positive {
            assume::clear(p);
        }
    }
}

/// A part of a region: the box of parameters, the coordinates as functions of them and the
/// volume factor (negative for a part that is taken away).
struct Piece {
    ranges: Vec<Range>,
    coords: Vec<Expr>,
    jac: Expr,
    /// Polar coordinates about the origin: the names of the angle and the radius, and the whole
    /// turns to take from the angle for the principal argument.
    polar: Option<(String, String, i64)>,
}

/// A point: a list of coordinates or a complex number.
fn point(e: &Expr) -> Result<Vec<Expr>, String> {
    match e {
        Expr::List(items) if !items.is_empty() => Ok(items.to_vec()),
        Expr::List(_) => Err("a point needs coordinates".into()),
        _ => complex::rect(e)
            .map(|(re, im)| vec![re, im])
            .ok_or_else(|| format!("{e} is not a point")),
    }
}

fn plane_point(e: &Expr) -> Result<[Expr; 2], String> {
    <[Expr; 2]>::try_from(point(e)?).map_err(|_| format!("{e} is not a point of the plane"))
}

fn sym(name: &str) -> Expr {
    Expr::sym(name)
}

fn two_pi() -> Expr {
    mul(vec![Expr::int(2), Expr::pi()])
}

fn magnitude(e: Expr) -> Expr {
    match numeric::sign(&e) {
        Some(s) if s < 0 => neg(e),
        Some(_) => e,
        None => func("abs", vec![e]),
    }
}

/// Polar coordinates about `c`; about the origin the range of the angle is cut where the
/// principal argument jumps.
fn polar(c: &[Expr; 2], r0: Expr, r1: Expr, t0: Expr, t1: Expr, names: &mut Names) -> Vec<Piece> {
    let (t, r) = (names.fresh("t"), names.radius("r"));
    let at = |trig: &str| mul(vec![sym(&r), func(trig, vec![sym(&t)])]);
    let coords = vec![
        add(vec![c[0].clone(), at("cos")]),
        add(vec![c[1].clone(), at("sin")]),
    ];
    let piece = |lo: Expr, hi: Expr, turns: Option<i64>| Piece {
        coords: coords.clone(),
        jac: sym(&r),
        ranges: vec![
            Range::new(&t, lo, hi),
            Range::new(&r, r0.clone(), r1.clone()),
        ],
        polar: turns.map(|m| (t.clone(), r.clone(), m)),
    };
    let ends = (numeric::real_value(&t0, 20), numeric::real_value(&t1, 20));
    let (Some(lo), Some(hi)) = ends else {
        return vec![piece(t0, t1, None)];
    };
    if !(c[0].is_zero() && c[1].is_zero()) || lo >= hi {
        return vec![piece(t0, t1, None)];
    }
    let pi = Float::with_val(lo.prec(), rug::float::Constant::Pi);
    // cuts at the odd multiples of pi inside the range
    let odd = |x: &Float, up: bool| {
        let q = (Float::with_val(x.prec(), x / &pi) - 1u32) / 2u32;
        let k = if up {
            q.floor() + 1u32
        } else {
            q.ceil() - 1u32
        };
        k.to_integer().and_then(|k| k.to_i64())
    };
    let (Some(first), Some(last)) = (odd(&lo, true), odd(&hi, false)) else {
        return vec![piece(t0, t1, None)];
    };
    if last - first > 64 {
        return vec![piece(t0, t1, None)];
    }
    let mut cuts = vec![t0.clone()];
    cuts.extend((first..=last).map(|k| mul(vec![Expr::int(2 * k + 1), Expr::pi()])));
    cuts.push(t1.clone());
    cuts.windows(2)
        .enumerate()
        .map(|(k, w)| piece(w[0].clone(), w[1].clone(), Some(first + k as i64)))
        .collect()
}

fn box_piece(p: &[Expr], q: &[Expr], names: &mut Names) -> Result<Piece, String> {
    if p.len() != q.len() {
        return Err("the corners have different numbers of coordinates".into());
    }
    let wishes = ["x", "y", "z", "w"];
    let mut ranges = Vec::new();
    for (k, (a, b)) in p.iter().zip(q).enumerate() {
        let v = names.fresh(wishes.get(k).copied().unwrap_or("x"));
        let swapped = numeric::sign(&sub(b.clone(), a.clone())) == Some(-1);
        let (lo, hi) = if swapped { (b, a) } else { (a, b) };
        ranges.push(Range::new(&v, lo.clone(), hi.clone()));
    }
    Ok(Piece {
        coords: ranges.iter().map(|r| sym(&r.var)).collect(),
        jac: Expr::one(),
        ranges,
        polar: None,
    })
}

/// The ball of radii `r0..r1` around `c`, stretched by `axes` along the coordinates.
fn ball(c: &[Expr], r0: Expr, r1: Expr, axes: Option<&[Expr]>, names: &mut Names) -> Piece {
    let n = c.len();
    let r = names.radius("r");
    let angles: Vec<String> = (1..n).map(|_| names.fresh("t")).collect();
    let stretch = |k: usize| axes.map_or_else(Expr::one, |a| a[k].clone());
    let mut prefix = sym(&r);
    let mut coords = Vec::new();
    let mut jac = vec![powi(sym(&r), n as i64 - 1)];
    for (k, a) in angles.iter().enumerate() {
        coords.push(add(vec![
            c[k].clone(),
            mul(vec![stretch(k), prefix.clone(), func("cos", vec![sym(a)])]),
        ]));
        let s = func("sin", vec![sym(a)]);
        jac.push(powi(s.clone(), (n - 2 - k) as i64));
        prefix = mul(vec![prefix, s]);
    }
    coords.push(add(vec![
        c[n - 1].clone(),
        mul(vec![stretch(n - 1), prefix]),
    ]));
    jac.extend((0..n).map(stretch));
    let mut ranges: Vec<Range> = angles
        .iter()
        .enumerate()
        .rev()
        .map(|(k, a)| {
            let top = if k + 2 == n { two_pi() } else { Expr::pi() };
            Range::new(a, Expr::zero(), top)
        })
        .collect();
    ranges.push(Range::new(&r, r0, r1));
    Piece {
        ranges,
        coords,
        jac: mul(jac),
        polar: None,
    }
}

fn edge_matrix(points: &[Vec<Expr>]) -> matrix::Rows {
    points[1..]
        .iter()
        .map(|p| {
            p.iter()
                .zip(&points[0])
                .map(|(a, b)| sub(a.clone(), b.clone()))
                .collect()
        })
        .collect()
}

/// The simplex with these corners; `factor` multiplies the volume factor.
fn simplex(points: &[Vec<Expr>], factor: Option<Expr>, names: &mut Names) -> Result<Piece, String> {
    let n = points.len() - 1;
    if n == 0 || points.iter().any(|p| p.len() != n) {
        return Err(format!(
            "a simplex of {} corners needs points with {n} coordinates",
            n + 1
        ));
    }
    let det = matrix::det(&edge_matrix(points))?;
    let vars: Vec<String> = (0..n).map(|_| names.fresh("u")).collect();
    let mut ranges = Vec::new();
    let mut used = Vec::new();
    for v in &vars {
        ranges.push(Range::new(
            v,
            Expr::zero(),
            sub(Expr::one(), add(used.clone())),
        ));
        used.push(sym(v));
    }
    let coords = (0..n)
        .map(|j| {
            let mut terms = vec![points[0][j].clone()];
            for (i, v) in vars.iter().enumerate() {
                terms.push(mul(vec![
                    sym(v),
                    sub(points[i + 1][j].clone(), points[0][j].clone()),
                ]));
            }
            add(terms)
        })
        .collect();
    Ok(Piece {
        ranges,
        coords,
        jac: match factor {
            Some(f) => mul(vec![det, f]),
            None => magnitude(det),
        },
        polar: None,
    })
}

fn corners(args: &[Expr]) -> Result<Vec<Vec<Expr>>, String> {
    let items: &[Expr] = match args {
        [Expr::List(inner)]
            if inner.iter().all(|p| matches!(p, Expr::List(_))) || inner.len() > 2 =>
        {
            inner
        }
        _ => args,
    };
    items.iter().map(point).collect()
}

/// Twice the signed area of a polygon and of the triangles of its fan.
fn fan(points: &[Vec<Expr>]) -> Result<(Expr, Vec<Expr>), String> {
    if points.len() < 3 || points.iter().any(|p| p.len() != 2) {
        return Err("a polygon needs three or more points of the plane".into());
    }
    let mut dets = Vec::new();
    for k in 1..points.len() - 1 {
        dets.push(matrix::det(&edge_matrix(&[
            points[0].clone(),
            points[k].clone(),
            points[k + 1].clone(),
        ]))?);
    }
    Ok((simplify(&add(dets.clone())), dets))
}

fn region_parts(region: &Expr) -> Result<(&str, &[Expr]), String> {
    match region {
        Expr::Fn(n, a) => Ok((&**n, &a[..])),
        _ => Err(format!(
            "{region} is not a region (rectangle, disk, annulus, sector, ellipse, triangle, polygon, box, ball, shell, ellipsoid, simplex, cylinder, cone)"
        )),
    }
}

fn pieces(region: &Expr, names: &mut Names) -> Result<Vec<Piece>, String> {
    let (name, a) = region_parts(region)?;
    let origin = [Expr::zero(), Expr::zero()];
    let zero = Expr::zero;
    Ok(match (name, a.len()) {
        ("rectangle", 2) => vec![box_piece(
            &plane_point(&a[0])?,
            &plane_point(&a[1])?,
            names,
        )?],
        ("rectangle", 4) => vec![box_piece(
            &[a[0].clone(), a[2].clone()],
            &[a[1].clone(), a[3].clone()],
            names,
        )?],
        ("box", 2) => vec![box_piece(&point(&a[0])?, &point(&a[1])?, names)?],
        ("disk", 1) => polar(
            &origin,
            zero(),
            a[0].clone(),
            neg(Expr::pi()),
            Expr::pi(),
            names,
        ),
        ("disk", 2) => polar(
            &plane_point(&a[0])?,
            zero(),
            a[1].clone(),
            neg(Expr::pi()),
            Expr::pi(),
            names,
        ),
        ("annulus", 3) => polar(
            &plane_point(&a[0])?,
            a[1].clone(),
            a[2].clone(),
            neg(Expr::pi()),
            Expr::pi(),
            names,
        ),
        ("sector", 4) => polar(
            &plane_point(&a[0])?,
            zero(),
            a[1].clone(),
            a[2].clone(),
            a[3].clone(),
            names,
        ),
        ("sector", 5) => polar(
            &plane_point(&a[0])?,
            a[1].clone(),
            a[2].clone(),
            a[3].clone(),
            a[4].clone(),
            names,
        ),
        ("ellipse", 3) => vec![ball(
            &plane_point(&a[0])?,
            zero(),
            Expr::one(),
            Some(&[a[1].clone(), a[2].clone()]),
            names,
        )],
        ("triangle", 3) | ("tetrahedron", 4) | ("simplex", _) => {
            vec![simplex(&corners(a)?, None, names)?]
        }
        ("polygon", _) => {
            let points = corners(a)?;
            let (total, dets) = fan(&points)?;
            let turn = match numeric::sign(&total) {
                Some(s) if s != 0 => Expr::int(i64::from(s)),
                Some(_) => return Err("the polygon has no area".into()),
                None => func("sign", vec![total]),
            };
            let mut out = Vec::new();
            for (k, det) in dets.iter().enumerate() {
                if det.is_zero() {
                    continue;
                }
                let corners = [
                    points[0].clone(),
                    points[k + 1].clone(),
                    points[k + 2].clone(),
                ];
                out.push(simplex(&corners, Some(turn.clone()), names)?);
            }
            out
        }
        ("ball", 2) | ("shell", 3) | ("ellipsoid", 2) => {
            let c = point(&a[0])?;
            if c.len() < 2 {
                return Err("the center needs two or more coordinates".into());
            }
            match name {
                "ball" => vec![ball(&c, zero(), a[1].clone(), None, names)],
                "shell" => vec![ball(&c, a[1].clone(), a[2].clone(), None, names)],
                _ => {
                    let axes = a[1].as_list().filter(|x| x.len() == c.len()).ok_or(
                        "ellipsoid(center, [a, b, ...]) needs one half-axis for each coordinate",
                    )?;
                    vec![ball(&c, zero(), Expr::one(), Some(axes), names)]
                }
            }
        }
        ("cylinder" | "cone", 3) => {
            let c = <[Expr; 3]>::try_from(point(&a[0])?)
                .map_err(|_| format!("{name}(center, radius, height) needs a center [x, y, z]"))?;
            let (t, s, r) = (names.fresh("t"), names.fresh("s"), names.radius("r"));
            let top = if name == "cone" {
                mul(vec![
                    a[1].clone(),
                    sub(Expr::one(), div(sym(&s), a[2].clone())),
                ])
            } else {
                a[1].clone()
            };
            let at = |trig: &str| mul(vec![sym(&r), func(trig, vec![sym(&t)])]);
            vec![Piece {
                coords: vec![
                    add(vec![c[0].clone(), at("cos")]),
                    add(vec![c[1].clone(), at("sin")]),
                    add(vec![c[2].clone(), sym(&s)]),
                ],
                jac: sym(&r),
                ranges: vec![
                    Range::new(&t, zero(), two_pi()),
                    Range::new(&s, zero(), a[2].clone()),
                    Range::new(&r, zero(), top),
                ],
                polar: None,
            }]
        }
        _ => {
            return Err(format!(
                "{region} is not a region: see --functions for rectangle, disk, annulus, sector, ellipse, triangle, polygon, box, ball, shell, ellipsoid, simplex, cylinder and cone"
            ));
        }
    })
}

pub fn is_region(e: &Expr) -> bool {
    matches!(e, Expr::Fn(n, _) if matches!(&**n,
        "rectangle" | "box" | "disk" | "annulus" | "sector" | "ellipse" | "triangle"
            | "tetrahedron" | "simplex" | "polygon" | "ball" | "shell" | "ellipsoid"
            | "cylinder" | "cone"))
}

fn taken_names(parts: &[&Expr], vars: &[String]) -> Names {
    let mut taken: Vec<String> = vars.to_vec();
    for e in parts {
        taken.extend(e.free_syms().iter().map(|s| s.to_string()));
    }
    Names {
        taken,
        positive: Vec::new(),
    }
}

/// The integral of `f` over the region: `(value, approximate)`. A complex integrand gives the
/// integral of its real part plus `i` times that of its imaginary part.
pub fn over_region(f: &Expr, vars: &Vars, region: &Expr) -> Value {
    assume::with_real_symbols(|| {
        let listed: Vec<String> = match vars {
            Vars::Complex(z) => vec![z.clone()],
            Vars::Real(v) => v.clone(),
        };
        let mut names = taken_names(&[f, region], &listed);
        let parts = pieces(region, &mut names)?;
        let (mut total, mut approx) = (Vec::new(), false);
        for p in &parts {
            let n = p.coords.len();
            let g = match vars {
                Vars::Complex(z) if n == 2 => {
                    // under a name that is not taken as real while the expression is rebuilt
                    let f = &subst_sym(f, z, &sym("#z"));
                    let z = "#z";
                    // about the origin |z| and arg(z) are the radius and the angle themselves
                    let direct = match &p.polar {
                        Some((t, r, turns)) => {
                            crate::canon::map_bottom_up(f, &mut |node| match node
                                .fn_arg("abs")
                                .or(node.fn_arg("arg"))
                            {
                                Some(inner) if inner.is_sym(z) => {
                                    if node.fn_arg("abs").is_some() {
                                        sym(r)
                                    } else {
                                        sub(sym(t), mul(vec![Expr::int(2 * turns), Expr::pi()]))
                                    }
                                }
                                _ => node,
                            })
                        }
                        None => f.clone(),
                    };
                    subst_sym(
                        &direct,
                        z,
                        &add(vec![
                            p.coords[0].clone(),
                            mul(vec![Expr::i(), p.coords[1].clone()]),
                        ]),
                    )
                }
                Vars::Complex(z) => {
                    return Err(format!(
                        "{z} is one variable but the region has {n} dimensions: give [x, y, ...]"
                    ));
                }
                Vars::Real(v) if v.len() == n => {
                    // through placeholders, so that x -> y, y -> x style maps stay apart
                    let holders: Vec<String> = (0..n).map(|k| format!("#v{k}")).collect();
                    let mut g = f.clone();
                    for (x, h) in v.iter().zip(&holders) {
                        g = subst_sym(&g, x, &sym(h));
                    }
                    for (h, c) in holders.iter().zip(&p.coords) {
                        g = subst_sym(&g, h, c);
                    }
                    g
                }
                Vars::Real(v) => {
                    return Err(format!(
                        "{} variable(s) for a region of {n} dimensions",
                        v.len()
                    ));
                }
            };
            // split into real and imaginary part while sums of squares are still to be seen
            let written = complex::expand_parts(&g);
            let (re, im) = complex::rect(&written).unwrap_or((written, Expr::zero()));
            let scaled = |part: Expr| simplify(&mul(vec![part, p.jac.clone()]));
            let (re, im) = (scaled(re), scaled(im));
            let (v, a) = iterated_any_order(&re, &p.ranges)?;
            approx |= a;
            total.push(v);
            if !im.is_zero() {
                let (w, a) = iterated_any_order(&im, &p.ranges)?;
                approx |= a;
                total.push(mul(vec![Expr::i(), w]));
            }
        }
        let value = simplify(&add(total));
        Ok((
            if approx {
                numeric::n(&value, numeric::digits())
            } else {
                value
            },
            approx,
        ))
    })
}

fn gamma_half(n: usize) -> Expr {
    func("gamma", vec![Expr::rat(n as i64 + 2, 2)])
}

/// Volume of the unit ball of `n` dimensions.
pub fn unit_ball(n: usize) -> Expr {
    div(pow(Expr::pi(), Expr::rat(n as i64, 2)), gamma_half(n))
}

fn square(e: &Expr) -> Expr {
    powi(e.clone(), 2)
}

fn distance(p: &[Expr], q: &[Expr]) -> Expr {
    sqrt(add(p
        .iter()
        .zip(q)
        .map(|(a, b)| square(&sub(a.clone(), b.clone())))
        .collect()))
}

/// Area of a plane region, volume of a region of three or more dimensions.
pub fn measure(region: &Expr) -> Result<Expr, String> {
    let (name, a) = region_parts(region)?;
    let closed = assume::with_real_symbols(|| -> Result<Option<Expr>, String> {
        Ok(Some(match (name, a.len()) {
            ("rectangle", 2 | 4) | ("box", 2) => {
                let (p, q) = match a.len() {
                    4 => (
                        vec![a[0].clone(), a[2].clone()],
                        vec![a[1].clone(), a[3].clone()],
                    ),
                    _ => (point(&a[0])?, point(&a[1])?),
                };
                if p.len() != q.len() {
                    return Err("the corners have different numbers of coordinates".into());
                }
                mul(p
                    .iter()
                    .zip(&q)
                    .map(|(x, y)| magnitude(sub(y.clone(), x.clone())))
                    .collect())
            }
            ("disk", 1) => mul(vec![Expr::pi(), square(&a[0])]),
            ("disk", 2) => mul(vec![Expr::pi(), square(&a[1])]),
            ("annulus", 3) => mul(vec![Expr::pi(), sub(square(&a[2]), square(&a[1]))]),
            ("sector", 4) => div(
                mul(vec![sub(a[3].clone(), a[2].clone()), square(&a[1])]),
                Expr::int(2),
            ),
            ("sector", 5) => div(
                mul(vec![
                    sub(a[4].clone(), a[3].clone()),
                    sub(square(&a[2]), square(&a[1])),
                ]),
                Expr::int(2),
            ),
            ("ellipse", 3) => mul(vec![Expr::pi(), a[1].clone(), a[2].clone()]),
            ("triangle", 3) | ("tetrahedron", 4) | ("simplex", _) => {
                let points = corners(a)?;
                let n = points.len() - 1;
                let size = points[0].len();
                if n == 0 || size < n || points.iter().any(|p| p.len() != size) {
                    return Ok(None);
                }
                let fact = Expr::integer(crate::ntheory::factorial(n as u32));
                let edges = edge_matrix(&points);
                if size == n {
                    div(magnitude(matrix::det(&edges)?), fact)
                } else {
                    // in a space of more dimensions: by the Gram determinant of the edges
                    let gram = matrix::matmul(&edges, &matrix::transpose(&edges))?;
                    div(sqrt(matrix::det(&gram)?), fact)
                }
            }
            ("polygon", _) => div(magnitude(fan(&corners(a)?)?.0), Expr::int(2)),
            ("ball", 2) => mul(vec![
                unit_ball(point(&a[0])?.len()),
                pow(a[1].clone(), Expr::int(point(&a[0])?.len() as i64)),
            ]),
            ("shell", 3) => {
                let n = point(&a[0])?.len();
                let p = |r: &Expr| pow(r.clone(), Expr::int(n as i64));
                mul(vec![unit_ball(n), sub(p(&a[2]), p(&a[1]))])
            }
            ("ellipsoid", 2) => {
                let axes = a[1].as_list().ok_or("ellipsoid(center, [a, b, ...])")?;
                mul(vec![unit_ball(axes.len()), mul(axes.to_vec())])
            }
            ("cylinder", 3) => mul(vec![Expr::pi(), square(&a[1]), a[2].clone()]),
            ("cone", 3) => div(
                mul(vec![Expr::pi(), square(&a[1]), a[2].clone()]),
                Expr::int(3),
            ),
            _ => return Ok(None),
        }))
    })?;
    match closed {
        Some(v) => Ok(simplify(&v)),
        None => {
            let dims = assume::with_real_symbols(|| {
                pieces(region, &mut taken_names(&[region], &[])).map(|p| p[0].coords.len())
            })?;
            let vars: Vec<String> = (0..dims).map(|k| format!("#m{k}")).collect();
            over_region(&Expr::one(), &Vars::Real(vars), region).map(|v| v.0)
        }
    }
}

/// Length of the boundary of a plane region, area of the surface of a solid.
pub fn boundary(region: &Expr) -> Result<Expr, String> {
    let (name, a) = region_parts(region)?;
    let two = |e: Expr| mul(vec![Expr::int(2), e]);
    let value = assume::with_real_symbols(|| -> Result<Expr, String> {
        Ok(match (name, a.len()) {
            ("rectangle", 2 | 4) | ("box", 2) => {
                let (p, q) = match a.len() {
                    4 => (
                        vec![a[0].clone(), a[2].clone()],
                        vec![a[1].clone(), a[3].clone()],
                    ),
                    _ => (point(&a[0])?, point(&a[1])?),
                };
                if p.len() != q.len() {
                    return Err("the corners have different numbers of coordinates".into());
                }
                let sides: Vec<Expr> = p
                    .iter()
                    .zip(&q)
                    .map(|(x, y)| magnitude(sub(y.clone(), x.clone())))
                    .collect();
                // every face: the product of all sides but one, twice
                two(add((0..sides.len())
                    .map(|skip| {
                        mul(sides
                            .iter()
                            .enumerate()
                            .filter(|(k, _)| *k != skip)
                            .map(|(_, s)| s.clone())
                            .collect())
                    })
                    .collect()))
            }
            ("disk", 1) => mul(vec![two_pi(), a[0].clone()]),
            ("disk", 2) => mul(vec![two_pi(), a[1].clone()]),
            ("annulus", 3) => mul(vec![two_pi(), add(vec![a[1].clone(), a[2].clone()])]),
            ("sector", 4) => add(vec![
                mul(vec![sub(a[3].clone(), a[2].clone()), a[1].clone()]),
                two(a[1].clone()),
            ]),
            ("sector", 5) => add(vec![
                mul(vec![
                    sub(a[4].clone(), a[3].clone()),
                    add(vec![a[1].clone(), a[2].clone()]),
                ]),
                two(sub(a[2].clone(), a[1].clone())),
            ]),
            ("ellipse", 3) => {
                let t = "#t";
                let speed = sqrt(add(vec![
                    mul(vec![square(&a[1]), square(&func("sin", vec![sym(t)]))]),
                    mul(vec![square(&a[2]), square(&func("cos", vec![sym(t)]))]),
                ]));
                let quarter = Range::new(t, Expr::zero(), div(Expr::pi(), Expr::int(2)));
                mul(vec![Expr::int(4), iterated(&speed, &[quarter])?.0])
            }
            ("triangle", 3) | ("polygon", _) => {
                let points = corners(a)?;
                if points.iter().any(|p| p.len() != 2) {
                    return Err("the corners must be points of the plane".into());
                }
                add((0..points.len())
                    .map(|k| distance(&points[k], &points[(k + 1) % points.len()]))
                    .collect())
            }
            ("tetrahedron", 4) | ("simplex", _) => {
                let points = corners(a)?;
                let n = points.len() - 1;
                if n < 2 || points.iter().any(|p| p.len() != n) {
                    return Err("a simplex of n + 1 corners needs points with n coordinates".into());
                }
                // every face by its Gram determinant
                let fact = Expr::integer(crate::ntheory::factorial(n as u32 - 1));
                add((0..=n)
                    .map(|skip| {
                        let face: Vec<Vec<Expr>> = points
                            .iter()
                            .enumerate()
                            .filter(|(k, _)| *k != skip)
                            .map(|(_, p)| p.clone())
                            .collect();
                        let edges = edge_matrix(&face);
                        let gram = matrix::matmul(&edges, &matrix::transpose(&edges))?;
                        Ok(div(sqrt(matrix::det(&gram)?), fact.clone()))
                    })
                    .collect::<Result<Vec<Expr>, String>>()?)
            }
            ("ball", 2) => {
                let n = point(&a[0])?.len();
                mul(vec![
                    Expr::int(n as i64),
                    unit_ball(n),
                    pow(a[1].clone(), Expr::int(n as i64 - 1)),
                ])
            }
            ("cylinder", 3) => mul(vec![
                two_pi(),
                a[1].clone(),
                add(vec![a[1].clone(), a[2].clone()]),
            ]),
            ("cone", 3) => mul(vec![
                Expr::pi(),
                a[1].clone(),
                add(vec![
                    a[1].clone(),
                    sqrt(add(vec![square(&a[1]), square(&a[2])])),
                ]),
            ]),
            _ => return Err(format!("no boundary measure is known for {region}")),
        })
    })?;
    Ok(simplify(&value))
}

/// The center of mass of a region of constant density.
pub fn centroid(region: &Expr) -> Result<Expr, String> {
    let dims = assume::with_real_symbols(|| {
        pieces(region, &mut taken_names(&[region], &[])).map(|p| p[0].coords.len())
    })?;
    let vars: Vec<String> = (0..dims).map(|k| format!("#m{k}")).collect();
    let size = measure(region)?;
    let mut out = Vec::new();
    for v in &vars {
        let (moment, _) = over_region(&sym(v), &Vars::Real(vars.clone()), region)?;
        out.push(simplify(&div(moment, size.clone())));
    }
    Ok(Expr::list(out))
}

/// The real crossing points of `f` with zero, in increasing order.
fn crossings(f: &Expr, var: &str) -> Result<Vec<Expr>, String> {
    let solved = crate::solve::solve(f, var)?;
    crate::solve::release(&solved.params);
    if solved.all || solved.roots.iter().any(|r| !r.value.is_const()) {
        return Err("the curves cross without end: give the interval".into());
    }
    let digits = numeric::digits();
    let mut real: Vec<(Float, Expr)> = solved
        .roots
        .iter()
        .filter_map(|r| Some((numeric::real_value(&r.value, digits)?, r.value.clone())))
        .collect();
    real.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    real.dedup_by(|a, b| a.0 == b.0);
    Ok(real.into_iter().map(|(_, e)| e).collect())
}

/// The area between the curve `f` and the axis (or the curve `g`), over the interval or between
/// the outermost crossing points.
pub fn area_between(f: &Expr, g: Option<&Expr>, var: &str, ends: Option<(&Expr, &Expr)>) -> Value {
    assume::with_real_symbols(|| area_gap(f, g, var, ends))
}

fn area_gap(f: &Expr, g: Option<&Expr>, var: &str, ends: Option<(&Expr, &Expr)>) -> Value {
    let gap = match g {
        Some(g) => simplify(&sub(f.clone(), g.clone())),
        None => f.clone(),
    };
    let (lo, hi) = match ends {
        Some((a, b)) => (a.clone(), b.clone()),
        None => {
            let points = crossings(&gap, var)?;
            if points.len() < 2 {
                return Err(format!(
                    "{} crossing point(s) found: give the interval, area(f, {var}, a, b)",
                    points.len()
                ));
            }
            (points[0].clone(), points[points.len() - 1].clone())
        }
    };
    iterated(&func("abs", vec![gap]), &[Range::new(var, lo, hi)])
}

/// Length of the curve `y = f(x)` or of the parametric curve `[x(t), y(t), ...]`.
pub fn arc_length(curve: &Expr, var: &str, lo: &Expr, hi: &Expr) -> Value {
    assume::with_real_symbols(|| {
        let d = |e: &Expr| crate::diff::diff(e, var);
        let speed = match curve {
            Expr::List(parts) => sqrt(add(parts.iter().map(|p| square(&d(p))).collect())),
            f => sqrt(add(vec![Expr::one(), square(&d(f))])),
        };
        iterated(
            &simplify(&speed),
            &[Range::new(var, lo.clone(), hi.clone())],
        )
    })
}

/// Area of the surface `z = f` over a region.
pub fn surface_area(f: &Expr, vars: &Vars, region: &Expr) -> Value {
    let height = assume::with_real_symbols(|| match vars {
        Vars::Complex(z) => Ok(complex::expand_parts(&subst_sym(
            f,
            z,
            &add(vec![sym("x#"), mul(vec![Expr::i(), sym("y#")])]),
        ))),
        Vars::Real(v) if v.len() == 2 => Ok(subst_sym(
            &subst_sym(f, &v[0], &sym("x#")),
            &v[1],
            &sym("y#"),
        )),
        Vars::Real(_) => Err("a surface needs two variables".to_string()),
    })?;
    let slope = |v: &str| square(&crate::diff::diff(&height, v));
    let element = sqrt(add(vec![Expr::one(), slope("x#"), slope("y#")]));
    over_region(
        &element,
        &Vars::Real(vec!["x#".to_string(), "y#".to_string()]),
        region,
    )
}
