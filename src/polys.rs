//! Polynomials as the lists of their coefficients, the highest first, the way MATLAB has them;
//! and curves through points: straight lines, splines and their kin.

use crate::arrays::{map, mldivide, order, same, tidy, whole, word};
use crate::canon::{add, div, mul, neg, pow, sub};
use crate::expr::{Expr, Rel};
use crate::funcs::func;
use crate::matrix::{self, Rows};
use crate::simplify::{is_zero, simplify};
use std::cmp::Ordering;

type Res = Result<Expr, String>;

fn coefficients<'a>(e: &'a Expr, name: &str) -> Result<&'a [Expr], String> {
    e.as_list()
        .filter(|l| l.iter().all(|x| x.as_list().is_none()))
        .ok_or_else(|| {
            format!("{name} needs the coefficients of a polynomial as a list, the highest first")
        })
}

/// Without the zeros in front.
fn trimmed(p: &[Expr]) -> Vec<Expr> {
    let start = p.iter().position(|x| !is_zero(x)).unwrap_or(p.len());
    p[start..].to_vec()
}

pub fn convolve(a: &[Expr], b: &[Expr]) -> Vec<Expr> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![Vec::new(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j].push(mul(vec![x.clone(), y.clone()]));
        }
    }
    out.into_iter().map(|t| tidy(add(t))).collect()
}

/// Quotient and remainder of `u / v`; the remainder is as long as `u`.
fn divide(u: &[Expr], v: &[Expr]) -> Result<(Vec<Expr>, Vec<Expr>), String> {
    let v = trimmed(v);
    if v.is_empty() {
        return Err("deconv: division by the zero polynomial".into());
    }
    if u.len() < v.len() {
        return Ok((vec![Expr::zero()], u.to_vec()));
    }
    let mut rest = u.to_vec();
    let mut quot = Vec::with_capacity(u.len() - v.len() + 1);
    for k in 0..=u.len() - v.len() {
        let q = tidy(div(rest[k].clone(), v[0].clone()));
        for (j, c) in v.iter().enumerate() {
            rest[k + j] = tidy(sub(rest[k + j].clone(), mul(vec![q.clone(), c.clone()])));
        }
        rest[k] = Expr::zero();
        quot.push(q);
    }
    Ok((quot, rest))
}

/// The value of the polynomial at `x`.
pub fn value(p: &[Expr], x: &Expr) -> Expr {
    let top = p.len().saturating_sub(1);
    tidy(add(p
        .iter()
        .enumerate()
        .map(|(k, c)| {
            if k == top {
                c.clone()
            } else {
                mul(vec![c.clone(), pow(x.clone(), Expr::int((top - k) as i64))])
            }
        })
        .collect()))
}

fn derivative(p: &[Expr]) -> Vec<Expr> {
    let top = p.len().saturating_sub(1);
    let out: Vec<Expr> = p
        .iter()
        .take(top)
        .enumerate()
        .map(|(k, c)| tidy(mul(vec![Expr::int((top - k) as i64), c.clone()])))
        .collect();
    if out.is_empty() {
        vec![Expr::zero()]
    } else {
        out
    }
}

/// The monic polynomial with the given roots.
fn from_roots(roots: &[Expr]) -> Vec<Expr> {
    let mut p = vec![Expr::one()];
    for r in roots {
        p = convolve(&p, &[Expr::one(), neg(r.clone())]);
    }
    p.into_iter()
        .map(|c| simplify(&crate::expand::expand(&c)))
        .collect()
}

/// The roots of a polynomial with the given coefficients, each as often as it is one.
fn roots_of(p: &[Expr]) -> Result<Vec<Expr>, String> {
    let p = trimmed(p);
    if p.len() < 2 {
        return Ok(Vec::new());
    }
    const VAR: &str = "#r";
    let solved = crate::solve::solve(&value(&p, &Expr::sym(VAR)), VAR)?;
    crate::solve::release(&solved.params);
    Ok(solved
        .roots
        .iter()
        .flat_map(|r| std::iter::repeat_n(r.value.clone(), r.mult as usize))
        .collect())
}

fn factorial(k: usize) -> Expr {
    Expr::integer(rug::Integer::from(rug::Integer::factorial(k as u32)))
}

/// `[r, p, k]` of `b(s)/a(s)`: the residues, their poles and the polynomial part.
fn residues(b: &[Expr], a: &[Expr]) -> Result<Expr, String> {
    let a = trimmed(a);
    if a.is_empty() {
        return Err("residue: division by the zero polynomial".into());
    }
    let (quot, rest) = divide(b, &a)?;
    let direct = trimmed(&quot);
    let poles = roots_of(&a)?;
    const VAR: &str = "#s";
    let s = Expr::sym(VAR);
    let top = value(&trimmed(&rest), &s);
    let mut groups: Vec<(Expr, usize)> = Vec::new();
    for p in &poles {
        match groups.iter_mut().find(|(q, _)| same(q, p)) {
            Some((_, m)) => *m += 1,
            None => groups.push((p.clone(), 1)),
        }
    }
    let (mut r_out, mut p_out) = (Vec::new(), Vec::new());
    for (pole, mult) in &groups {
        let others: Vec<Expr> = groups
            .iter()
            .filter(|(q, _)| !same(q, pole))
            .map(|(q, m)| pow(sub(s.clone(), q.clone()), Expr::int(*m as i64)))
            .collect();
        let mut g = div(top.clone(), mul(vec![a[0].clone(), mul(others)]));
        let mut ders = vec![g.clone()];
        for _ in 1..*mult {
            g = crate::diff::diff(&g, VAR);
            ders.push(g.clone());
        }
        for j in 1..=*mult {
            let at = crate::canon::subst_sym(&ders[mult - j], VAR, pole);
            r_out.push(simplify(&div(at, factorial(mult - j))));
            p_out.push(pole.clone());
        }
    }
    Ok(Expr::list(vec![
        Expr::list(r_out),
        Expr::list(p_out),
        Expr::list(direct),
    ]))
}

/// `[b, a]`: the fraction with the residues `r` at the poles `p` and the polynomial part `k`.
fn from_residues(r: &[Expr], p: &[Expr], k: &[Expr]) -> Result<Expr, String> {
    if r.len() != p.len() {
        return Err("residue(r, p, k): a pole for every residue".into());
    }
    let den = from_roots(p);
    let mut num = convolve(&trimmed(k), &den);
    let mut seen: Vec<(Expr, usize)> = Vec::new();
    for (res, pole) in r.iter().zip(p) {
        let power = match seen.iter_mut().find(|(q, _)| same(q, pole)) {
            Some((_, m)) => {
                *m += 1;
                *m
            }
            None => {
                seen.push((pole.clone(), 1));
                1
            }
        };
        let mut part = den.clone();
        for _ in 0..power {
            part = divide(&part, &[Expr::one(), neg(pole.clone())])?.0;
        }
        let part: Vec<Expr> = part
            .iter()
            .map(|c| mul(vec![res.clone(), c.clone()]))
            .collect();
        let width = num.len().max(part.len());
        let pad = |v: &[Expr]| {
            let mut out = vec![Expr::zero(); width - v.len()];
            out.extend(v.iter().cloned());
            out
        };
        num = pad(&num)
            .into_iter()
            .zip(pad(&part))
            .map(|(x, y)| simplify(&crate::expand::expand(&add(vec![x, y]))))
            .collect();
    }
    let num = trimmed(&num);
    Ok(Expr::list(vec![
        Expr::list(if num.is_empty() {
            vec![Expr::zero()]
        } else {
            num
        }),
        Expr::list(den),
    ]))
}

/// A variable for a command: the one given, else the one the expression has.
fn variable(given: Option<&Expr>, e: &Expr, name: &str) -> Result<String, String> {
    match given {
        Some(Expr::Sym(s)) => Ok(s.to_string()),
        Some(other) => Err(format!("{name}: {other} is not a variable")),
        None => {
            let syms = e.free_syms();
            match syms.len() {
                0 => Ok("x".to_string()),
                1 => Ok(syms
                    .iter()
                    .next()
                    .map(|s| s.to_string())
                    .unwrap_or_default()),
                _ if syms.contains("x") => Ok("x".to_string()),
                _ => Err(format!(
                    "{name}: name the variable, the expression has {}",
                    syms.len()
                )),
            }
        }
    }
}

/// The numerator and the denominator without fractions in either.
pub fn numden(e: &Expr) -> (Expr, Expr) {
    let (mut num, mut den) = crate::ratsimp::fraction(&crate::ratsimp::together(e));
    let mut lcm = rug::Integer::from(1);
    let terms: Vec<Expr> = match &num {
        Expr::Add(t) => t.to_vec(),
        other => vec![other.clone()],
    };
    for t in &terms {
        let (c, _) = crate::canon::coeff_and_rest(t);
        if let Some(r) = Expr::num(c).as_rat() {
            lcm = lcm.lcm(r.denom());
        }
    }
    if lcm != 1 {
        let by = Expr::integer(lcm);
        num = simplify(&crate::expand::expand(&mul(vec![by.clone(), num])));
        den = simplify(&mul(vec![by, den]));
    }
    if let Some(r) = den.as_rat()
        && r.cmp0() == Ordering::Less
    {
        (neg(num), neg(den))
    } else {
        (num, den)
    }
}

// ---- curves through points ----

struct Nodes {
    x: Vec<Expr>,
    y: Vec<Expr>,
}

fn nodes(x: &Expr, y: &Expr, name: &str) -> Result<Nodes, String> {
    let (xs, ys) = (coefficients(x, name)?, coefficients(y, name)?);
    if xs.len() != ys.len() || xs.len() < 2 {
        return Err(format!(
            "{name}: as many places as values, two or more ({} and {})",
            xs.len(),
            ys.len()
        ));
    }
    let mut pairs: Vec<(Expr, Expr)> = xs.iter().cloned().zip(ys.iter().cloned()).collect();
    let mut failed = None;
    pairs.sort_by(|p, q| {
        order(&p.0, &q.0).unwrap_or_else(|e| {
            failed = Some(e);
            Ordering::Equal
        })
    });
    if let Some(e) = failed {
        return Err(format!("{name}: {e}"));
    }
    if pairs.windows(2).any(|w| same(&w[0].0, &w[1].0)) {
        return Err(format!("{name}: the places must differ"));
    }
    let (x, y) = pairs.into_iter().unzip();
    Ok(Nodes { x, y })
}

impl Nodes {
    fn steps(&self) -> (Vec<Expr>, Vec<Expr>) {
        let h: Vec<Expr> = self
            .x
            .windows(2)
            .map(|w| tidy(sub(w[1].clone(), w[0].clone())))
            .collect();
        let d: Vec<Expr> = self
            .y
            .windows(2)
            .zip(&h)
            .map(|(w, h)| tidy(div(sub(w[1].clone(), w[0].clone()), h.clone())))
            .collect();
        (h, d)
    }

    /// The slopes at the places of the cubic spline with no knot at the second and the last
    /// but one place.
    fn spline_slopes(&self) -> Result<Vec<Expr>, String> {
        let n = self.x.len();
        let (h, d) = self.steps();
        if n == 2 {
            return Ok(vec![d[0].clone(), d[0].clone()]);
        }
        if n == 3 {
            // the parabola through the three points
            let c = div(
                sub(d[1].clone(), d[0].clone()),
                add(vec![h[0].clone(), h[1].clone()]),
            );
            let at = |t: Expr| tidy(add(vec![d[0].clone(), mul(vec![c.clone(), t])]));
            return Ok(vec![
                at(neg(h[0].clone())),
                at(h[0].clone()),
                at(add(vec![
                    h[0].clone(),
                    mul(vec![Expr::int(2), h[1].clone()]),
                ])),
            ]);
        }
        let (mut low, mut mid, mut up, mut rhs) = (
            vec![Expr::zero(); n],
            vec![Expr::zero(); n],
            vec![Expr::zero(); n],
            vec![Expr::zero(); n],
        );
        let two = Expr::int(2);
        let span = add(vec![h[0].clone(), h[1].clone()]);
        mid[0] = h[1].clone();
        up[0] = span.clone();
        rhs[0] = div(
            add(vec![
                mul(vec![
                    add(vec![h[0].clone(), mul(vec![two.clone(), span.clone()])]),
                    h[1].clone(),
                    d[0].clone(),
                ]),
                mul(vec![pow(h[0].clone(), two.clone()), d[1].clone()]),
            ]),
            span,
        );
        for i in 1..n - 1 {
            low[i] = h[i].clone();
            mid[i] = mul(vec![two.clone(), add(vec![h[i - 1].clone(), h[i].clone()])]);
            up[i] = h[i - 1].clone();
            rhs[i] = mul(vec![
                Expr::int(3),
                add(vec![
                    mul(vec![h[i].clone(), d[i - 1].clone()]),
                    mul(vec![h[i - 1].clone(), d[i].clone()]),
                ]),
            ]);
        }
        let span = add(vec![h[n - 2].clone(), h[n - 3].clone()]);
        low[n - 1] = span.clone();
        mid[n - 1] = h[n - 3].clone();
        rhs[n - 1] = div(
            add(vec![
                mul(vec![pow(h[n - 2].clone(), two.clone()), d[n - 3].clone()]),
                mul(vec![
                    add(vec![mul(vec![two, span.clone()]), h[n - 2].clone()]),
                    h[n - 3].clone(),
                    d[n - 2].clone(),
                ]),
            ]),
            span,
        );
        // (three diagonals: forward sweep, then back)
        for i in 1..n {
            if is_zero(&mid[i - 1]) {
                return Err("spline: the places give no spline".into());
            }
            let f = tidy(div(low[i].clone(), mid[i - 1].clone()));
            mid[i] = tidy(sub(mid[i].clone(), mul(vec![f.clone(), up[i - 1].clone()])));
            rhs[i] = tidy(sub(rhs[i].clone(), mul(vec![f, rhs[i - 1].clone()])));
        }
        let mut s = vec![Expr::zero(); n];
        for i in (0..n).rev() {
            let above = if i + 1 < n {
                mul(vec![up[i].clone(), s[i + 1].clone()])
            } else {
                Expr::zero()
            };
            s[i] = tidy(div(sub(rhs[i].clone(), above), mid[i].clone()));
        }
        Ok(s)
    }

    /// The slopes that keep the shape of the data (Fritsch and Carlson).
    fn pchip_slopes(&self) -> Result<Vec<Expr>, String> {
        let n = self.x.len();
        let (h, d) = self.steps();
        if n == 2 {
            return Ok(vec![d[0].clone(), d[0].clone()]);
        }
        let sign = |e: &Expr| order(e, &Expr::zero()).map(|o| o as i32);
        let mut s = vec![Expr::zero(); n];
        for k in 1..n - 1 {
            if sign(&d[k - 1])? * sign(&d[k])? > 0 {
                let w1 = add(vec![
                    mul(vec![Expr::int(2), h[k].clone()]),
                    h[k - 1].clone(),
                ]);
                let w2 = add(vec![
                    h[k].clone(),
                    mul(vec![Expr::int(2), h[k - 1].clone()]),
                ]);
                s[k] = tidy(div(
                    add(vec![w1.clone(), w2.clone()]),
                    add(vec![div(w1, d[k - 1].clone()), div(w2, d[k].clone())]),
                ));
            }
        }
        let end = |h0: &Expr, h1: &Expr, d0: &Expr, d1: &Expr| -> Res {
            let slope = tidy(div(
                sub(
                    mul(vec![
                        add(vec![mul(vec![Expr::int(2), h0.clone()]), h1.clone()]),
                        d0.clone(),
                    ]),
                    mul(vec![h0.clone(), d1.clone()]),
                ),
                add(vec![h0.clone(), h1.clone()]),
            ));
            if sign(&slope)? != sign(d0)? {
                return Ok(Expr::zero());
            }
            let steep = mul(vec![Expr::int(3), d0.clone()]);
            if sign(d0)? != sign(d1)?
                && order(
                    &func("abs", vec![slope.clone()]),
                    &func("abs", vec![steep.clone()]),
                )? == Ordering::Greater
            {
                return Ok(tidy(steep));
            }
            Ok(slope)
        };
        s[0] = end(&h[0], &h[1], &d[0], &d[1])?;
        s[n - 1] = end(&h[n - 2], &h[n - 3], &d[n - 2], &d[n - 3])?;
        Ok(s)
    }

    /// The slopes of Akima's curve as MATLAB changed it: flat where the data is flat.
    fn makima_slopes(&self) -> Result<Vec<Expr>, String> {
        let n = self.x.len();
        let (_, d) = self.steps();
        if n == 2 {
            return Ok(vec![d[0].clone(), d[0].clone()]);
        }
        let twice_less =
            |a: &Expr, b: &Expr| tidy(sub(mul(vec![Expr::int(2), a.clone()]), b.clone()));
        // two slopes more at each end
        let before1 = twice_less(&d[0], &d[1]);
        let before2 = twice_less(&before1, &d[0]);
        let after1 = twice_less(&d[n - 2], &d[n - 3]);
        let after2 = twice_less(&after1, &d[n - 2]);
        let mut all = vec![before2, before1];
        all.extend(d.iter().cloned());
        all.extend([after1, after2]);
        let abs = |e: Expr| func("abs", vec![tidy(e)]);
        let weight = |a: &Expr, b: &Expr| {
            tidy(add(vec![
                abs(sub(a.clone(), b.clone())),
                div(abs(add(vec![a.clone(), b.clone()])), Expr::int(2)),
            ]))
        };
        Ok((0..n)
            .map(|i| {
                // (all[i + 2] is the slope after place i)
                let w1 = weight(&all[i + 3], &all[i + 2]);
                let w2 = weight(&all[i + 1], &all[i]);
                let total = tidy(add(vec![w1.clone(), w2.clone()]));
                if is_zero(&total) {
                    Expr::zero()
                } else {
                    tidy(div(
                        add(vec![
                            mul(vec![w1, all[i + 1].clone()]),
                            mul(vec![w2, all[i + 2].clone()]),
                        ]),
                        total,
                    ))
                }
            })
            .collect())
    }

    /// The coefficients of the cubic on piece `i` with the slopes `s`, in `t = q - x[i]`, the
    /// highest first.
    fn cubic_coefficients(&self, i: usize, s: &[Expr]) -> [Expr; 4] {
        let h = sub(self.x[i + 1].clone(), self.x[i].clone());
        let d = div(sub(self.y[i + 1].clone(), self.y[i].clone()), h.clone());
        let c = div(
            sub(
                mul(vec![Expr::int(3), d.clone()]),
                add(vec![
                    mul(vec![Expr::int(2), s[i].clone()]),
                    s[i + 1].clone(),
                ]),
            ),
            h.clone(),
        );
        let b = div(
            sub(
                add(vec![s[i].clone(), s[i + 1].clone()]),
                mul(vec![Expr::int(2), d]),
            ),
            pow(h, Expr::int(2)),
        );
        [tidy(b), tidy(c), s[i].clone(), self.y[i].clone()]
    }

    /// The cubic on piece `i` with the slopes `s`, as a polynomial in `t = q - x[i]`.
    fn cubic(&self, i: usize, s: &[Expr], q: &Expr) -> Expr {
        let [b, c, slope, y] = self.cubic_coefficients(i, s);
        let t = sub(q.clone(), self.x[i].clone());
        add(vec![
            y,
            mul(vec![slope, t.clone()]),
            mul(vec![c, pow(t.clone(), Expr::int(2))]),
            mul(vec![b, pow(t, Expr::int(3))]),
        ])
    }

    fn line(&self, i: usize, q: &Expr) -> Expr {
        let h = sub(self.x[i + 1].clone(), self.x[i].clone());
        let d = div(sub(self.y[i + 1].clone(), self.y[i].clone()), h);
        add(vec![
            self.y[i].clone(),
            mul(vec![d, sub(q.clone(), self.x[i].clone())]),
        ])
    }

    /// The piece that holds `q`: `None` outside the places.
    fn piece(&self, q: &Expr) -> Result<Option<usize>, String> {
        let n = self.x.len();
        if order(q, &self.x[0])? == Ordering::Less || order(q, &self.x[n - 1])? == Ordering::Greater
        {
            return Ok(None);
        }
        for i in 0..n - 1 {
            if order(q, &self.x[i + 1])? != Ordering::Greater {
                return Ok(Some(i));
            }
        }
        Ok(Some(n - 2))
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Method {
    Linear,
    Nearest,
    Next,
    Previous,
    Pchip,
    Spline,
    Makima,
}

fn method(e: &Expr) -> Option<Method> {
    Some(match word(e)?.as_str() {
        "linear" => Method::Linear,
        "nearest" => Method::Nearest,
        "next" => Method::Next,
        "previous" => Method::Previous,
        "pchip" | "cubic" => Method::Pchip,
        "spline" => Method::Spline,
        "makima" => Method::Makima,
        _ => return None,
    })
}

/// The curve of a method through the points at `q`: a number, a list of numbers, or a name
/// (then all the pieces with their conditions).
fn interpolate(nodes: &Nodes, how: Method, extend: bool, q: &Expr) -> Res {
    let slopes = match how {
        Method::Spline => Some(nodes.spline_slopes()?),
        Method::Pchip => Some(nodes.pchip_slopes()?),
        Method::Makima => Some(nodes.makima_slopes()?),
        _ => None,
    };
    let n = nodes.x.len();
    let on_piece = |i: usize, q: &Expr| match &slopes {
        Some(s) => nodes.cubic(i, s, q),
        None => nodes.line(i, q),
    };
    let stepwise = matches!(how, Method::Nearest | Method::Next | Method::Previous);
    let at = |q: &Expr| -> Res {
        if let Expr::List(_) = q {
            return Err("interp1: a place is a number".into());
        }
        if !q.is_const() {
            if stepwise {
                return Err("interp1: this method needs numbers for the places asked for".into());
            }
            let mut pieces = Vec::new();
            if !extend {
                pieces.push(Expr::rel(Rel::Lt, q.clone(), nodes.x[0].clone()));
                pieces.push(Expr::undefined());
            }
            for i in 0..n - 1 {
                if i + 2 < n || !extend {
                    pieces.push(Expr::rel(Rel::Le, q.clone(), nodes.x[i + 1].clone()));
                }
                pieces.push(simplify(&crate::expand::expand(&on_piece(i, q))));
            }
            if !extend {
                pieces.push(Expr::undefined());
            }
            // (one curve on all pieces is that curve)
            if extend && pieces.iter().step_by(2).all(|v| *v == pieces[0]) {
                return Ok(pieces.swap_remove(0));
            }
            return Ok(func("piecewise", pieces));
        }
        let inside = nodes.piece(q)?;
        match how {
            Method::Nearest | Method::Next | Method::Previous => {
                let Some(i) = inside else {
                    return Ok(Expr::undefined());
                };
                let (lo, hi) = (&nodes.x[i], &nodes.x[i + 1]);
                let upper = match how {
                    Method::Next => !same(q, lo),
                    Method::Previous => same(q, hi),
                    _ => {
                        order(
                            &tidy(sub(q.clone(), lo.clone())),
                            &tidy(sub(hi.clone(), q.clone())),
                        )? != Ordering::Less
                    }
                };
                Ok(nodes.y[if upper { i + 1 } else { i }].clone())
            }
            _ => match inside {
                Some(i) => Ok(tidy(on_piece(i, q))),
                None if extend => {
                    let i = if order(q, &nodes.x[0])? == Ordering::Less {
                        0
                    } else {
                        n - 2
                    };
                    Ok(tidy(on_piece(i, q)))
                }
                None => Ok(Expr::undefined()),
            },
        }
    };
    match q {
        Expr::List(_) => {
            let failed = std::cell::RefCell::new(None);
            let out = map(q, &|x| {
                at(x).unwrap_or_else(|e| {
                    failed.borrow_mut().get_or_insert(e);
                    Expr::undefined()
                })
            });
            match failed.into_inner() {
                Some(e) => Err(e),
                None => Ok(out),
            }
        }
        _ => at(q),
    }
}

/// `[breaks, coefficients]`: the pieces of polynomials that `mkpp` and `spline(x, y)` give,
/// a row of coefficients (the highest first, in the distance from the break) for every piece.
fn pieces_of(pp: &Expr) -> Result<(Vec<Expr>, Rows), String> {
    let usage = "the pieces of polynomials are [breaks, coefficients]: mkpp([0, 1, 2], [[1, 0], [2, 1]]), with a row of coefficients for every piece";
    let Some([breaks, coefficients]) = pp.as_list() else {
        return Err(usage.to_string());
    };
    let breaks = breaks.as_list().filter(|b| b.len() >= 2).ok_or(usage)?;
    let pieces: Rows = match matrix::rows(coefficients) {
        Some(m) => m,
        None => vec![coefficients.as_list().ok_or(usage)?.to_vec()],
    };
    if pieces.len() != breaks.len() - 1 || pieces[0].is_empty() {
        return Err(format!(
            "{usage}; {} break(s) and {} row(s)",
            breaks.len(),
            pieces.len()
        ));
    }
    for w in breaks.windows(2) {
        if order(&w[0], &w[1])? != Ordering::Less {
            return Err("the breaks of the pieces must rise".to_string());
        }
    }
    Ok((breaks.to_vec(), pieces))
}

type Renamed = (
    crate::ratsimp::Gens,
    Vec<crate::poly::Poly>,
    Vec<(String, Expr)>,
);

/// Polynomials in the variables given (all their names without them), in that order of the
/// variables, and what gives the names back.
fn polynomials(polys: &[Expr], vars: Option<&Expr>, name: &str) -> Result<Renamed, String> {
    let polys: Vec<Expr> = polys
        .iter()
        .map(|p| match p {
            Expr::Rel(Rel::Eq, l, r) => sub((**l).clone(), (**r).clone()),
            other => other.clone(),
        })
        .collect();
    let vars: Vec<String> = match vars {
        Some(Expr::List(v)) => v
            .iter()
            .map(|x| {
                x.sym_name()
                    .map(str::to_string)
                    .ok_or(format!("{name}: the variables are names"))
            })
            .collect::<Result<_, _>>()?,
        Some(Expr::Sym(s)) => vec![s.to_string()],
        Some(_) => return Err(format!("{name}([polynomials], [variables])")),
        None => {
            let mut all = std::collections::BTreeSet::new();
            polys.iter().for_each(|p| p.collect_syms(&mut all));
            all.iter().map(|s| s.to_string()).collect()
        }
    };
    // (the names are swapped for ones that sort in the order asked for)
    let pairs: Vec<(String, Expr)> = vars
        .iter()
        .enumerate()
        .map(|(k, v)| (v.clone(), Expr::sym(&format!("#g{k:04}"))))
        .collect();
    let renamed: Vec<Expr> = polys
        .iter()
        .map(|p| crate::canon::subst_all(p, &pairs))
        .collect();
    let refs: Vec<&Expr> = renamed.iter().collect();
    let (gens, ps) =
        crate::ratsimp::common(&refs).ok_or_else(|| format!("{name} needs polynomials"))?;
    if !gens
        .kernels()
        .iter()
        .all(|k| k.sym_name().is_some_and(|s| s.starts_with("#g")))
    {
        return Err(format!(
            "{name}: the polynomials have more names than the variables given"
        ));
    }
    let back: Vec<(String, Expr)> = pairs
        .iter()
        .map(|(v, s)| (s.sym_name().unwrap_or_default().to_string(), Expr::sym(v)))
        .collect();
    Ok((gens, ps, back))
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    let list = |k: usize| coefficients(&a[k], name);
    Ok(Some(match (name, n) {
        ("poly", 1) => match matrix::rows(&a[0]) {
            Some(m) if m.len() == m[0].len() => Expr::list(matrix::charpoly_coefficients(&m)?),
            Some(_) => return Err("poly(matrix) needs a square matrix".into()),
            None => Expr::list(from_roots(list(0)?)),
        },
        ("polyval", 2) => {
            let p = list(0)?;
            map(&a[1], &|x| value(p, x))
        }
        ("polyvalm", 2) => {
            let p = list(0)?;
            let m = matrix::rows(&a[1])
                .filter(|m| m.len() == m[0].len())
                .ok_or("polyvalm(p, A) needs a square matrix")?;
            let mut out: Rows = vec![vec![Expr::zero(); m.len()]; m.len()];
            for c in p {
                out = matrix::matmul(&out, &m)?;
                for (i, row) in out.iter_mut().enumerate() {
                    for (j, x) in row.iter_mut().enumerate() {
                        *x = tidy(if i == j {
                            add(vec![x.clone(), c.clone()])
                        } else {
                            x.clone()
                        });
                    }
                }
            }
            matrix::from_rows(out)
        }
        ("polyfit", 3) => {
            let (x, y) = (list(0)?, list(1)?);
            let degree = whole(&a[2], "the degree", 0, 1000)? as usize;
            if x.len() != y.len() || x.len() <= degree {
                return Err(format!(
                    "polyfit(x, y, n): as many x as y, and more points than the degree ({} and {})",
                    x.len(),
                    y.len()
                ));
            }
            let design: Rows = x
                .iter()
                .map(|v| {
                    (0..=degree)
                        .map(|k| match degree - k {
                            0 => Expr::one(),
                            e => pow(v.clone(), Expr::int(e as i64)),
                        })
                        .collect()
                })
                .collect();
            mldivide(&matrix::from_rows(design), &Expr::list(y.to_vec()))?
        }
        ("polyder", 1) => Expr::list(derivative(list(0)?)),
        ("polyder", 2) => Expr::list(derivative(&convolve(list(0)?, list(1)?))),
        ("polyint", 1 | 2) => {
            let p = list(0)?;
            let top = p.len();
            let mut out: Vec<Expr> = p
                .iter()
                .enumerate()
                .map(|(k, c)| tidy(div(c.clone(), Expr::int((top - k) as i64))))
                .collect();
            out.push(a.get(1).cloned().unwrap_or_else(Expr::zero));
            Expr::list(out)
        }
        ("conv", 2 | 3) => {
            let (x, y) = (list(0)?, list(1)?);
            let full = convolve(x, y);
            match a.get(2).map(word) {
                None => Expr::list(full),
                Some(Some(w)) if w == "full" => Expr::list(full),
                Some(Some(w)) if w == "same" => {
                    let start = y.len() / 2;
                    Expr::list(full.into_iter().skip(start).take(x.len()).collect())
                }
                Some(Some(w)) if w == "valid" => {
                    if y.len() > x.len() || y.is_empty() {
                        Expr::list(Vec::new())
                    } else {
                        Expr::list(
                            full.into_iter()
                                .skip(y.len() - 1)
                                .take(x.len() - y.len() + 1)
                                .collect(),
                        )
                    }
                }
                Some(_) => {
                    return Err("conv(a, b), conv(a, b, \"same\") or conv(a, b, \"valid\")".into());
                }
            }
        }
        ("deconv", 2) => {
            let (q, r) = divide(list(0)?, list(1)?)?;
            Expr::list(vec![Expr::list(q), Expr::list(r)])
        }
        ("roots", 1) if a[0].as_list().is_some() => Expr::list(roots_of(list(0)?)?),
        ("residue", 2) if a[0].as_list().is_some() && a[1].as_list().is_some() => {
            residues(list(0)?, list(1)?)?
        }
        ("residue", 3) if a.iter().all(|x| x.as_list().is_some()) => {
            from_residues(list(0)?, list(1)?, list(2)?)?
        }
        ("poly2sym", 1 | 2) => {
            let x = a.get(1).cloned().unwrap_or_else(|| Expr::sym("x"));
            value(list(0)?, &x)
        }
        ("sym2poly" | "coeffs", 1..=3) => {
            let text = |e: &Expr| crate::radix::text_of(e).map(str::to_ascii_lowercase);
            let all = name == "sym2poly" || (n > 1 && text(&a[n - 1]).as_deref() == Some("all"));
            if n == 3 && !all {
                return Err(format!("{name}(p, x, \"all\")"));
            }
            let var = variable(a.get(1).filter(|g| text(g).is_none()), &a[0], name)?;
            let c = crate::ratsimp::coefficients(&a[0], &var)
                .ok_or_else(|| format!("{name}: {} is no polynomial in {var}", a[0]))?;
            let c: Vec<Expr> = c.iter().map(simplify).collect();
            if all {
                Expr::list(c.into_iter().rev().collect())
            } else {
                Expr::list(c.into_iter().filter(|x| !x.is_zero()).collect())
            }
        }
        ("horner", 1 | 2) => {
            let var = variable(a.get(1), &a[0], name)?;
            let c = crate::ratsimp::coefficients(&a[0], &var)
                .ok_or_else(|| format!("horner: {} is no polynomial in {var}", a[0]))?;
            let x = Expr::sym(&var);
            let mut out: Option<Expr> = None;
            for k in c.iter().rev() {
                out = Some(match out {
                    None => simplify(k),
                    Some(o) => add(vec![mul(vec![x.clone(), o]), simplify(k)]),
                });
            }
            out.unwrap_or_else(Expr::zero)
        }
        ("numden", 1) => {
            let (num, den) = numden(&a[0]);
            Expr::list(vec![num, den])
        }
        ("numer" | "denom", 1) => {
            let (num, den) = numden(&a[0]);
            if name == "numer" { num } else { den }
        }
        ("quorem", 2 | 3) => match (a[0].as_int(), a[1].as_int()) {
            (Some(x), Some(y)) if n == 2 => {
                if y.is_zero() {
                    return Err("quorem: division by zero".into());
                }
                let (q, r) = <(rug::Integer, rug::Integer)>::from(x.div_rem_ref(y));
                Expr::list(vec![Expr::integer(q), Expr::integer(r)])
            }
            _ => {
                let both = add(vec![a[0].clone(), a[1].clone()]);
                let var = variable(a.get(2), &both, name)?;
                let (q, r) = crate::ratsimp::div_rem(&a[0], &a[1], &var)
                    .ok_or("quorem: two polynomials are needed, the second not zero")?;
                Expr::list(vec![q, r])
            }
        },
        ("interp1", 2..=5) => {
            // interp1(x, y, q), interp1(y, q), each with a method and "extrap" after it
            let mut rest: Vec<&Expr> = a.iter().collect();
            let mut extend = None;
            let mut how = None;
            while let Some(last) = rest.last() {
                match (method(last), word(last).as_deref()) {
                    (Some(m), _) if crate::radix::text_of(last).is_some() => how = Some(m),
                    (_, Some("extrap")) if crate::radix::text_of(last).is_some() => {
                        extend = Some(true);
                    }
                    _ => break,
                }
                rest.pop();
            }
            let how = how.unwrap_or(Method::Linear);
            let extend = extend.unwrap_or(matches!(
                how,
                Method::Pchip | Method::Spline | Method::Makima
            ));
            let (x, y, q) = match rest.as_slice() {
                [x, y, q] => ((*x).clone(), *y, *q),
                [y, q] => {
                    let count = y.as_list().map_or(0, <[Expr]>::len);
                    (Expr::list((1..=count as i64).map(Expr::int).collect()), *y, *q)
                }
                _ => return Err("interp1(x, y, q), interp1(x, y, q, \"spline\"); methods: linear, nearest, next, previous, pchip, spline, makima; \"extrap\" goes on outside the places".into()),
            };
            interpolate(&nodes(&x, y, name)?, how, extend, q)?
        }
        ("spline" | "pchip" | "makima", 2) => {
            // the pieces themselves: [breaks, coefficients of every piece]
            let nodes = nodes(&a[0], &a[1], name)?;
            let slopes = match name {
                "spline" => nodes.spline_slopes()?,
                "pchip" => nodes.pchip_slopes()?,
                _ => nodes.makima_slopes()?,
            };
            let pieces: Rows = (0..nodes.x.len() - 1)
                .map(|i| nodes.cubic_coefficients(i, &slopes).to_vec())
                .collect();
            Expr::list(vec![Expr::list(nodes.x.clone()), matrix::from_rows(pieces)])
        }
        ("mkpp", 2) => {
            let (breaks, pieces) = pieces_of(&Expr::list(a.to_vec()))?;
            Expr::list(vec![Expr::list(breaks), matrix::from_rows(pieces)])
        }
        ("unmkpp", 1) => {
            let (breaks, pieces) = pieces_of(&a[0])?;
            let (count, degree) = (pieces.len() as i64, pieces[0].len() as i64);
            Expr::list(vec![
                Expr::list(breaks),
                matrix::from_rows(pieces),
                Expr::int(count),
                Expr::int(degree),
                Expr::one(),
            ])
        }
        ("ppval", 2) => {
            let (breaks, pieces) = pieces_of(&a[0])?;
            let at = |q: &Expr| -> Res {
                // the last piece that starts at or before q; the first and the last go on
                let mut k = 0;
                for (i, b) in breaks.iter().enumerate().take(pieces.len()).skip(1) {
                    if order(q, b).map_err(|e| format!("ppval: {e}"))? != Ordering::Less {
                        k = i;
                    }
                }
                Ok(tidy(value(&pieces[k], &sub(q.clone(), breaks[k].clone()))))
            };
            crate::arrays::map_result(&a[1], &at)?
        }
        ("polynomialreduce", 2 | 3) => {
            let divisors: Vec<Expr> = match a[1].as_list() {
                Some(items) => items.to_vec(),
                None => vec![a[1].clone()],
            };
            let mut all = vec![a[0].clone()];
            all.extend(divisors);
            let (gens, ps, back) = polynomials(&all, a.get(2), name)?;
            let rest = crate::poly::reduce(&ps[0], &ps[1..]);
            tidy(crate::canon::subst_all(&gens.expr(&rest), &back))
        }
        ("spline" | "pchip" | "makima", 3) => {
            let how = match name {
                "spline" => Method::Spline,
                "pchip" => Method::Pchip,
                _ => Method::Makima,
            };
            interpolate(&nodes(&a[0], &a[1], name)?, how, true, &a[2])?
        }
        ("interp2", 5 | 6) => {
            let (xs, ys) = (list(0)?, list(1)?);
            let z = matrix::rows(&a[2]).ok_or("interp2(x, y, Z, xq, yq): Z is a matrix")?;
            if z.len() != ys.len() || z[0].len() != xs.len() || xs.len() < 2 || ys.len() < 2 {
                return Err(format!(
                    "interp2(x, y, Z, xq, yq): Z has a row for every y and a column for every x ({} by {} for {} y and {} x)",
                    z.len(),
                    z[0].len(),
                    ys.len(),
                    xs.len()
                ));
            }
            let nearest = match a.get(5).map(word) {
                None => false,
                Some(Some(w)) if w == "linear" => false,
                Some(Some(w)) if w == "nearest" => true,
                Some(_) => return Err("interp2: the methods are \"linear\" and \"nearest\"".into()),
            };
            let cell = |v: &[Expr], q: &Expr| -> Result<Option<(usize, Expr)>, String> {
                let last = v.len() - 1;
                if order(q, &v[0])? == Ordering::Less || order(q, &v[last])? == Ordering::Greater {
                    return Ok(None);
                }
                let i = (0..last)
                    .find(|i| order(q, &v[i + 1]).is_ok_and(|o| o != Ordering::Greater))
                    .unwrap_or(last - 1);
                let part = tidy(div(
                    sub(q.clone(), v[i].clone()),
                    sub(v[i + 1].clone(), v[i].clone()),
                ));
                Ok(Some((i, part)))
            };
            let at = |xq: &Expr, yq: &Expr| -> Res {
                let (Some((i, u)), Some((j, v))) = (cell(xs, xq)?, cell(ys, yq)?) else {
                    return Ok(Expr::undefined());
                };
                if nearest {
                    let up = |p: &Expr| order(p, &Expr::half()).is_ok_and(|o| o != Ordering::Less);
                    return Ok(z[j + usize::from(up(&v))][i + usize::from(up(&u))].clone());
                }
                let one = Expr::one();
                let along = |row: &[Expr]| {
                    add(vec![
                        mul(vec![sub(one.clone(), u.clone()), row[i].clone()]),
                        mul(vec![u.clone(), row[i + 1].clone()]),
                    ])
                };
                Ok(tidy(add(vec![
                    mul(vec![sub(one.clone(), v.clone()), along(&z[j])]),
                    mul(vec![v.clone(), along(&z[j + 1])]),
                ])))
            };
            match (a[3].as_list(), a[4].as_list()) {
                (Some(xq), Some(yq)) if xq.len() == yq.len() => Expr::list(
                    xq.iter()
                        .zip(yq)
                        .map(|(p, q)| at(p, q))
                        .collect::<Result<_, _>>()?,
                ),
                (None, None) => at(&a[3], &a[4])?,
                _ => return Err("interp2: as many xq as yq".into()),
            }
        }
        ("gbasis", 1 | 2) => {
            let polys = a[0].as_list().ok_or("gbasis([polynomials], [variables])")?;
            let (gens, ps, back) = polynomials(polys, a.get(1), name)?;
            let basis = crate::poly::groebner(&ps, 20_000).ok_or("gbasis: too much work")?;
            Expr::list(
                basis
                    .iter()
                    .map(|p| crate::canon::subst_all(&gens.expr(p), &back))
                    .collect(),
            )
        }
        _ => return Ok(None),
    }))
}

pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    inner(name, a).transpose()
}
