//! Systems of differential equations: linear ones with constant coefficients by the matrix
//! exponential, any system by numbers (extrapolation of the midpoint rule).

use crate::canon::{add, mul, neg, pow, sub, subst, subst_sym};
use crate::diff::diff;
use crate::eval::{Session, map_raw};
use crate::expr::{Expr, Rel};
use crate::matrix;
use crate::numeric::{self, Env};
use crate::simplify::simplify;
use rug::{Complex, Float};
use std::rc::Rc;

type Lines = Result<(Expr, Vec<String>), String>;

/// The symbol for the derivative of the given order of an unknown.
fn state_name(u: &str, order: u32) -> String {
    if order == 0 {
        u.to_string()
    } else {
        format!("#d{order}_{u}")
    }
}

/// `u'`, `u''`, `diff(u, x)`, `u(x)` become symbols; `u(c)` and `u'(c)` become
/// `#at(u, order, c)`.
fn convert(e: &Expr, unknowns: &[String], x: &str) -> Expr {
    map_raw(e, &mut |node| {
        let (mut order, mut base) = (0u32, node);
        while let Expr::Fn(n, a) = base
            && &**n == "derivative'"
        {
            order += 1;
            base = &a[0];
        }
        if order == 0
            && let Expr::Fn(n, a) = node
            && &**n == "diff"
            && (a.len() == 2 || a.len() == 3)
            && a[1].is_sym(x)
        {
            order = a
                .get(2)
                .map_or(Some(1), |k| k.as_i64().and_then(|k| u32::try_from(k).ok()))?;
            base = &a[0];
        }
        let known = |name: &str| unknowns.iter().any(|u| u == name);
        match base {
            Expr::Sym(s) if known(s) && order > 0 => Some(Expr::sym(&state_name(s, order))),
            Expr::Fn(n, a) if known(n) && a.len() == 1 => Some(if a[0].is_sym(x) {
                Expr::sym(&state_name(n, order))
            } else {
                Expr::call(
                    "#at",
                    vec![Expr::sym(n), Expr::int(i64::from(order)), a[0].clone()],
                )
            }),
            _ => None,
        }
    })
}

/// A condition `u^(order)(point) = value`.
struct Condition {
    unknown: String,
    order: u32,
    point: Expr,
    value: Expr,
}

fn names_of(e: Option<&Expr>, usage: &str) -> Result<Vec<String>, String> {
    match e {
        Some(Expr::Sym(s)) => Ok(vec![s.to_string()]),
        Some(Expr::List(items)) if !items.is_empty() => items
            .iter()
            .map(|i| {
                i.sym_name()
                    .map(str::to_string)
                    .ok_or_else(|| usage.to_string())
            })
            .collect(),
        _ => Err(usage.to_string()),
    }
}

/// The right sides of a system at a point: `None` where they have no value.
type Field<'a> = &'a mut dyn FnMut(&Float, &[Complex]) -> Option<Vec<Complex>>;

/// One explicit step of the midpoint rule with `n` substeps over `h` (Gragg's method).
fn midpoint(
    f: Field,
    t: &Float,
    y: &[Complex],
    h: &Float,
    n: u32,
    prec: u32,
) -> Option<Vec<Complex>> {
    let step = Float::with_val(prec, h / n);
    let mut before: Vec<Complex> = y.to_vec();
    let slope = f(t, y)?;
    let mut now: Vec<Complex> = y
        .iter()
        .zip(&slope)
        .map(|(a, b)| Complex::with_val(prec, a + Complex::with_val(prec, b * &step)))
        .collect();
    let two = Float::with_val(prec, &step * 2u32);
    for m in 1..n {
        let at = Float::with_val(prec, t + Float::with_val(prec, &step * m));
        let slope = f(&at, &now)?;
        let next: Vec<Complex> = before
            .iter()
            .zip(&slope)
            .map(|(a, b)| Complex::with_val(prec, a + Complex::with_val(prec, b * &two)))
            .collect();
        before = now;
        now = next;
    }
    let end = Float::with_val(prec, t + h);
    let slope = f(&end, &now)?;
    Some(
        now.iter()
            .zip(&before)
            .zip(&slope)
            .map(|((a, b), c)| {
                (Complex::with_val(prec, a + b) + Complex::with_val(prec, c * &step)) / 2u32
            })
            .collect(),
    )
}

fn magnitude(z: &Complex) -> i64 {
    let part = |x: &Float| x.get_exp().map_or(i64::MIN, i64::from);
    part(z.real()).max(part(z.imag()))
}

/// The solution of `y' = f(t, y)` at `t1` from `y(t0)`: steps of the midpoint rule with
/// 2, 4, 6, ... substeps, extrapolated to step 0, with the step chosen by the error seen.
pub(crate) fn integrate(
    f: Field,
    t0: &Float,
    t1: &Float,
    y0: Vec<Complex>,
    bits: u32,
) -> Result<Vec<Complex>, String> {
    integrate_by(f, t0, t1, y0, bits, 16)
}

/// As `integrate`, the first step being that share of the way (1: the whole way at once,
/// for short ways between points that lie close).
pub(crate) fn integrate_by(
    f: Field,
    t0: &Float,
    t1: &Float,
    y0: Vec<Complex>,
    bits: u32,
    share: u32,
) -> Result<Vec<Complex>, String> {
    const COLUMNS: usize = 16;
    let prec = bits + 64;
    let mut t = Float::with_val(prec, t0);
    let mut y = y0;
    let span = Float::with_val(prec, t1 - t0);
    if span.is_zero() {
        return Ok(y);
    }
    let mut h = Float::with_val(prec, &span / share);
    let mut work = 0u64;
    loop {
        let left = Float::with_val(prec, t1 - &t);
        if left.is_zero() || (left.is_sign_negative() != span.is_sign_negative()) {
            return Ok(y);
        }
        let mut last = false;
        if Float::with_val(prec, h.abs_ref()) >= Float::with_val(prec, left.abs_ref()) {
            h = left;
            last = true;
        }
        // the table of extrapolation: row k holds the columns for 2(k + 1) substeps
        let mut table: Vec<Vec<Vec<Complex>>> = Vec::new();
        let mut accepted: Option<(usize, Vec<Complex>)> = None;
        for k in 0..COLUMNS {
            let n = 2 * (k as u32 + 1);
            work += u64::from(n) + 1;
            let Some(first) = midpoint(f, &t, &y, &h, n, prec) else {
                break;
            };
            let mut row = vec![first];
            for j in 0..k {
                // T(k, j+1) = T(k, j) + (T(k, j) - T(k-1, j))/((n_k/n_(k-j-1))^2 - 1)
                let ratio = Float::with_val(prec, n) / (2 * (k - j) as u32);
                let weight = Float::with_val(prec, ratio.square_ref()) - 1u32;
                let next: Vec<Complex> = row[j]
                    .iter()
                    .zip(&table[k - 1][j])
                    .map(|(a, b)| {
                        Complex::with_val(prec, a + Complex::with_val(prec, a - b) / &weight)
                    })
                    .collect();
                row.push(next);
            }
            if k >= 2 {
                let (best, lower) = (&row[k], &row[k - 1]);
                // each part to the bits asked for; parts far below the largest one to its size
                let floor = best
                    .iter()
                    .map(magnitude)
                    .max()
                    .unwrap_or(0)
                    .saturating_sub(30);
                let close = best.iter().zip(lower).all(|(a, b)| {
                    let d = Complex::with_val(prec, a - b);
                    d.is_zero()
                        || magnitude(&d) <= magnitude(a).max(floor).saturating_sub(i64::from(bits))
                });
                if close {
                    accepted = Some((k, best.clone()));
                    break;
                }
            }
            table.push(row);
        }
        if work > 4_000_000 {
            return Err("the numeric solution takes too many steps: the system is stiff or has a singular point in the way".into());
        }
        match accepted {
            Some((k, value)) => {
                t += &h;
                y = value;
                if last {
                    return Ok(y);
                }
                if k <= 7 {
                    h *= 2u32;
                } else if k >= 12 {
                    h /= 2u32;
                }
            }
            None => {
                h /= 4u32;
                let tiny = Float::with_val(prec, h.abs_ref())
                    < Float::with_val(prec, span.abs_ref()) >> 70u32;
                if tiny {
                    return Err(format!(
                        "the numeric solution does not get past {}: a singular point",
                        numeric::fmt_complex(&Complex::with_val(prec, &t), 12)
                    ));
                }
            }
        }
    }
}

impl Session {
    /// The residuals of the equations and the conditions among the items.
    fn ode_items(
        &mut self,
        first: &Expr,
        unknowns: &[String],
        x: &str,
    ) -> Result<(Vec<Expr>, Vec<Condition>), String> {
        let items: Vec<&Expr> = match first {
            Expr::List(v) => v.iter().collect(),
            other => vec![other],
        };
        let (mut equations, mut conditions) = (Vec::new(), Vec::new());
        for item in items {
            let value = self.eval(&convert(item, unknowns, x))?;
            let (left, right) = match &value {
                Expr::Rel(Rel::Eq, l, r) => ((**l).clone(), (**r).clone()),
                other => (other.clone(), Expr::zero()),
            };
            match left.fn_args("#at") {
                Some([u, order, point]) if !right.has_fn("#at") => conditions.push(Condition {
                    unknown: u.sym_name().unwrap_or_default().to_string(),
                    order: order.as_i64().unwrap_or(0) as u32,
                    point: point.clone(),
                    value: right,
                }),
                _ => {
                    let f = sub(left, right);
                    if f.has_fn("#at") {
                        return Err("conditions must have the form y(a) = b or y'(a) = b".into());
                    }
                    equations.push(f);
                }
            }
        }
        Ok((equations, conditions))
    }

    /// `dsolve([x' = ..., y' = ...], [x, y], t)`: a linear system with constant coefficients.
    pub(crate) fn dsolve_system(&mut self, raw: &[Expr], decimal: bool) -> Lines {
        let usage = "dsolve([x' = a*x + b*y, y' = c*x + d*y], [x, y], t), with conditions like x(0) = 1 in the list";
        let unknowns = names_of(raw.get(1), usage)?;
        let var = match raw.get(2) {
            Some(Expr::Sym(s)) => s.to_string(),
            None if !unknowns.iter().any(|u| u == "t") => "t".to_string(),
            _ => return Err(usage.into()),
        };
        let (equations, conditions) = self.ode_items(raw.first().ok_or(usage)?, &unknowns, &var)?;
        let n = unknowns.len();
        if equations.len() != n {
            return Err(format!(
                "{} equation(s) for {n} unknown function(s)",
                equations.len()
            ));
        }
        if equations.iter().any(|f| {
            f.any(&|e| matches!(e, Expr::Sym(s) if s.starts_with("#d") && !s.starts_with("#d1_")))
        }) {
            return Err("write the system with first derivatives only (a higher derivative as a further unknown), or use ndsolve".into());
        }
        let marks: Vec<String> = unknowns.iter().map(|u| state_name(u, 1)).collect();
        let free_of = |e: &Expr, names: &[String]| names.iter().all(|m| !e.has_sym(m));
        // M X' = rhs
        let m: matrix::Rows = equations
            .iter()
            .map(|f| marks.iter().map(|d| simplify(&diff(f, d))).collect())
            .collect();
        let rhs: Vec<Expr> = equations
            .iter()
            .map(|f| {
                simplify(&neg(marks
                    .iter()
                    .fold(f.clone(), |acc, d| subst_sym(&acc, d, &Expr::zero()))))
            })
            .collect();
        if m.iter().flatten().any(|c| !free_of(c, &marks)) {
            return Err("the system is not linear in the derivatives".into());
        }
        let slopes = matrix::linsolve(&m, &rhs)
            .map_err(|_| "the equations do not give every derivative".to_string())?;
        let slopes = slopes
            .as_list()
            .filter(|l| l.len() == n)
            .ok_or("the equations do not give every derivative")?
            .to_vec();
        // X' = A X + B(t)
        let a: matrix::Rows = slopes
            .iter()
            .map(|f| unknowns.iter().map(|u| simplify(&diff(f, u))).collect())
            .collect();
        let b: Vec<Expr> = slopes
            .iter()
            .map(|f| {
                simplify(
                    &unknowns
                        .iter()
                        .fold(f.clone(), |acc, u| subst_sym(&acc, u, &Expr::zero())),
                )
            })
            .collect();
        let constant = a
            .iter()
            .flatten()
            .all(|c| free_of(c, &unknowns) && !c.has_sym(&var));
        let rebuilt_ok = slopes.iter().zip(&a).zip(&b).all(|((f, row), b0)| {
            let again = add(row
                .iter()
                .zip(&unknowns)
                .map(|(c, u)| mul(vec![c.clone(), Expr::sym(u)]))
                .chain(std::iter::once(b0.clone()))
                .collect());
            crate::simplify::is_zero(&sub(f.clone(), again))
        });
        if !constant || !rebuilt_ok || b.iter().any(|x| !free_of(x, &unknowns)) {
            return Err("only linear systems with constant coefficients are solved in closed form: ndsolve gives numbers for the others".into());
        }
        let t = Expr::sym(&var);
        let lam = Expr::sym("#lam");
        let mut e_at = matrix::function_of(&a, &pow(Expr::e(), mul(vec![t.clone(), lam])), "#lam")
            .map_err(|e| format!("{e}: ndsolve gives numbers for this system"))?;
        if a.iter()
            .flatten()
            .all(|c| c.is_const() && numeric::real_value(c, 20).is_some())
        {
            // a real matrix has a real exponential: complex pairs of eigenvalues give sines and cosines
            for entry in e_at.iter_mut().flatten() {
                if let Some((re, _)) =
                    crate::assume::with_real_symbols(|| crate::complex::rect(entry))
                {
                    *entry = crate::assume::with_real_symbols(|| {
                        simplify(&crate::algebra::trigreduce(&simplify(&re)))
                    });
                }
            }
        }
        // the particular part: the integral of e^(-A t) B(t)
        let mut inner: Vec<Expr> = Vec::new();
        let forced = b.iter().any(|x| !x.is_zero());
        if forced {
            let back: matrix::Rows = e_at
                .iter()
                .map(|row| row.iter().map(|c| subst(c, &t, &neg(t.clone()))).collect())
                .collect();
            for row in &back {
                let f = simplify(&add(row
                    .iter()
                    .zip(&b)
                    .map(|(c, b0)| mul(vec![c.clone(), b0.clone()]))
                    .collect()));
                inner.push(crate::integrate::integrate(&f, &var).ok_or_else(|| {
                    format!(
                        "no antiderivative found for {f}: ndsolve gives numbers for this system"
                    )
                })?);
            }
        }
        let prefix = ["C", "K", "A"]
            .into_iter()
            .find(|p| {
                !equations
                    .iter()
                    .any(|f| f.free_syms().iter().any(|s| s.starts_with(p)))
            })
            .unwrap_or("C_");
        let constants: Vec<String> = (1..=n).map(|k| format!("{prefix}{k}")).collect();
        let build = |cs: &[Expr]| -> Vec<Expr> {
            e_at.iter()
                .map(|row| {
                    add(row
                        .iter()
                        .enumerate()
                        .map(|(j, c)| {
                            let inside = match inner.get(j) {
                                Some(p) => add(vec![cs[j].clone(), p.clone()]),
                                None => cs[j].clone(),
                            };
                            mul(vec![c.clone(), inside])
                        })
                        .collect())
                })
                .collect()
        };
        let symbols: Vec<Expr> = constants.iter().map(|c| Expr::sym(c)).collect();
        let mut solution = build(&symbols);
        if !conditions.is_empty() {
            if conditions.len() != n || conditions.iter().any(|c| c.order != 0) {
                return Err(format!(
                    "{n} conditions like x(0) = 1 are needed, one for each unknown function"
                ));
            }
            // linear equations for the constants
            let mut rows: matrix::Rows = Vec::new();
            let mut right = Vec::new();
            for c in &conditions {
                let k = unknowns
                    .iter()
                    .position(|u| *u == c.unknown)
                    .ok_or("a condition names a function that is not an unknown")?;
                let at = simplify(&subst(&solution[k], &t, &c.point));
                rows.push(
                    constants
                        .iter()
                        .map(|name| simplify(&diff(&at, name)))
                        .collect(),
                );
                let rest = constants
                    .iter()
                    .fold(at, |acc, name| subst_sym(&acc, name, &Expr::zero()));
                right.push(simplify(&sub(c.value.clone(), rest)));
            }
            let found = matrix::linsolve(&rows, &right)
                .map_err(|_| "the conditions do not fix the constants".to_string())?;
            let values = found
                .as_list()
                .filter(|l| l.len() == n)
                .ok_or("the conditions do not fix the constants")?
                .to_vec();
            solution = build(&values);
        }
        let digits = numeric::digits();
        let (mut lines, mut items) = (Vec::new(), Vec::new());
        for (u, e) in unknowns.iter().zip(&solution) {
            let e = crate::transform::real_form(e);
            let shown = if decimal {
                numeric::n(&e, digits)
            } else {
                e.clone()
            };
            lines.push(format!("{u} = {shown}"));
            items.push(Expr::eq(Expr::sym(u), e));
        }
        Ok((Expr::list(items), lines))
    }

    /// `ndsolve([equations and conditions], y or [x, y], [t, t0, t1])`: the values at `t1`;
    /// with `[t, t0, t1, n]` the values at `n` equal steps.
    pub(crate) fn ndsolve(&mut self, raw: &[Expr], _decimal: bool) -> Lines {
        let usage = "ndsolve([y'' + y = 0, y(0) = 0, y'(0) = 1], y, [x, 0, 10]) or ndsolve([...], [x, y], [t, 0, 1, 10]) for 10 steps";
        let unknowns = names_of(raw.get(1), usage)?;
        let spec = match raw.get(2) {
            Some(Expr::List(items)) if (3..=4).contains(&items.len()) => items,
            _ => return Err(usage.into()),
        };
        let var = spec[0].sym_name().ok_or(usage)?.to_string();
        let ends = self.eval_all(&spec[1..])?;
        let digits = numeric::digits();
        let bits = numeric::bits_for(digits) + 24;
        let prec = bits + 64;
        let real = |e: &Expr| -> Result<Float, String> {
            numeric::real_value(e, digits + 5)
                .map(|v| Float::with_val(prec, v))
                .ok_or_else(|| format!("{e} is not a real number"))
        };
        let (t0, t1) = (real(&ends[0])?, real(&ends[1])?);
        let steps = match ends.get(2) {
            Some(k) => k
                .as_i64()
                .filter(|k| (1..=100_000).contains(k))
                .ok_or("the number of steps must be a whole number from 1 to 100000")?
                as u32,
            None => 1,
        };
        let (equations, conditions) = self.ode_items(raw.first().ok_or(usage)?, &unknowns, &var)?;
        if equations.len() != unknowns.len() {
            return Err(format!(
                "{} equation(s) for {} unknown function(s)",
                equations.len(),
                unknowns.len()
            ));
        }
        // the order of each unknown, and the equations solved for the highest derivatives
        let mut orders = Vec::new();
        for u in &unknowns {
            let order = (1..=20u32)
                .rev()
                .find(|k| equations.iter().any(|f| f.has_sym(&state_name(u, *k))))
                .ok_or_else(|| format!("the equations have no derivative of {u}"))?;
            orders.push(order);
        }
        let tops: Vec<String> = unknowns
            .iter()
            .zip(&orders)
            .map(|(u, k)| state_name(u, *k))
            .collect();
        let m: matrix::Rows = equations
            .iter()
            .map(|f| tops.iter().map(|d| simplify(&diff(f, d))).collect())
            .collect();
        if m.iter()
            .flatten()
            .any(|c| tops.iter().any(|d| c.has_sym(d)))
        {
            return Err("the equations are not linear in the highest derivatives".into());
        }
        let rhs: Vec<Expr> = equations
            .iter()
            .map(|f| {
                neg(tops
                    .iter()
                    .fold(f.clone(), |acc, d| subst_sym(&acc, d, &Expr::zero())))
            })
            .collect();
        let slopes = matrix::linsolve(&m, &rhs)
            .map_err(|_| "the equations do not give the highest derivatives".to_string())?;
        let slopes = slopes
            .as_list()
            .filter(|l| l.len() == unknowns.len())
            .ok_or("the equations do not give the highest derivatives")?
            .to_vec();
        // the state: every unknown with its derivatives below the highest
        let mut state: Vec<String> = Vec::new();
        let mut rates: Vec<Expr> = Vec::new();
        for ((u, order), slope) in unknowns.iter().zip(&orders).zip(&slopes) {
            for k in 0..*order {
                state.push(state_name(u, k));
                rates.push(if k + 1 == *order {
                    slope.clone()
                } else {
                    Expr::sym(&state_name(u, k + 1))
                });
            }
        }
        let allowed = |e: &Expr| {
            e.free_syms()
                .iter()
                .all(|s| **s == *var || state.iter().any(|n| **s == **n))
        };
        if let Some(bad) = rates.iter().find(|r| !allowed(r)) {
            return Err(format!(
                "the equations must hold numbers only besides the unknown functions and {var}: {bad}"
            ));
        }
        let mut start: Vec<Complex> = Vec::new();
        for ((u, order), _) in unknowns.iter().zip(&orders).zip(&slopes) {
            for k in 0..*order {
                let c = conditions
                    .iter()
                    .find(|c| c.unknown == *u && c.order == k)
                    .ok_or_else(|| {
                        let primes = "'".repeat(k as usize);
                        format!("the start value {u}{primes}({}) is missing", ends[0])
                    })?;
                let at = real(&c.point)?;
                if Float::with_val(prec, &at - &t0).abs() > Float::with_val(prec, 1) >> (bits - 8) {
                    return Err(format!(
                        "the conditions must be given at the start {} of the range",
                        ends[0]
                    ));
                }
                let v = numeric::evalf_digits(&c.value, digits + 8)
                    .map_err(|_| format!("the start value {} is not a number", c.value))?;
                start.push(Complex::with_val(prec, v));
            }
        }
        let keys: Vec<Rc<str>> = state.iter().map(|n| Rc::from(n.as_str())).collect();
        let time: Rc<str> = Rc::from(var.as_str());
        let mut f = |t: &Float, y: &[Complex]| -> Option<Vec<Complex>> {
            let mut env = Env::new();
            env.insert(time.clone(), Complex::with_val(prec, t));
            for (k, v) in keys.iter().zip(y) {
                env.insert(k.clone(), v.clone());
            }
            rates
                .iter()
                .map(|r| numeric::evalf_env(r, prec, &env).ok())
                .collect()
        };
        // where each unknown sits in the state
        let mut places = Vec::new();
        let mut at = 0usize;
        for order in &orders {
            places.push(at);
            at += *order as usize;
        }
        // the size of the solution so far: what is far below it is rounding noise
        let mut scale = start.iter().map(magnitude).max().unwrap_or(0).max(0);
        let float = |z: &Complex, scale: i64| {
            let floor = scale - i64::from(bits) + 24;
            let part = |x: &Float| {
                if x.get_exp().map_or(i64::MIN, i64::from) < floor {
                    Float::new(prec)
                } else {
                    x.clone()
                }
            };
            numeric::to_float_expr(
                Complex::with_val(prec, (part(z.real()), part(z.imag()))),
                digits,
            )
        };
        let mut y = start;
        let mut t = t0.clone();
        let span = Float::with_val(prec, &t1 - &t0);
        let mut rows = Vec::new();
        for k in 1..=steps {
            let next = if k == steps {
                t1.clone()
            } else {
                Float::with_val(prec, &t0 + Float::with_val(prec, &span * k) / steps)
            };
            y = integrate(&mut f, &t, &next, y, bits)?;
            t = next;
            scale = scale.max(y.iter().map(magnitude).max().unwrap_or(0));
            let mut row = vec![float(&Complex::with_val(prec, &t), i64::MIN / 2)];
            row.extend(places.iter().map(|p| float(&y[*p], scale)));
            rows.push(Expr::list(row));
        }
        if steps > 1 {
            let value = Expr::list(rows);
            let line = value.to_string();
            return Ok((value, vec![line]));
        }
        let (mut lines, mut items) = (Vec::new(), Vec::new());
        for (u, p) in unknowns.iter().zip(&places) {
            let v = float(&y[*p], scale);
            lines.push(format!("{u}({}) = {v}", ends[1]));
            items.push(v);
        }
        let value = if items.len() == 1 {
            items.pop().expect("one value")
        } else {
            Expr::list(items)
        };
        Ok((value, lines))
    }
}
