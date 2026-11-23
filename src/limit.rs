//! Limits: substitution with infinity arithmetic, series, l'Hopital, numeric extrapolation.

use crate::assume::{self, Facts};
use crate::canon::{add, div, mul, neg, numer_denom, pow, sub, subst};
use crate::diff::diff;
use crate::expr::Expr;
use crate::numeric::{self, Env};
use crate::ratsimp;
use crate::series::{self, LOG};
use crate::simplify::{self, simplify};
use rug::{Complex, Float};
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Dir {
    Both,
    Right,
    Left,
}

const T: &str = "#t";

fn settled(v: &Expr) -> bool {
    if v.is_inf() || v.is_neg_inf() || v.is_zoo() {
        return true;
    }
    !v.any(&|x| x.is_infinite())
}

fn signed_inf(s: i32) -> Expr {
    if s < 0 { Expr::neg_inf() } else { Expr::inf() }
}

/// Limit of the leading term `c t^p` (with `c` a polynomial in `ln t`) as `t -> 0+`.
fn leading(c: &Expr, lo: i64) -> Option<Expr> {
    let cs = ratsimp::coefficients(c, LOG)?;
    let degree = cs.len().checked_sub(1)?;
    if cs.iter().any(|x| x.has_sym(T)) {
        return None;
    }
    if lo > 0 {
        return Some(Expr::zero());
    }
    let flip = if degree % 2 == 1 { -1 } else { 1 };
    if lo == 0 && degree == 0 {
        return Some(simplify(&cs[0]));
    }
    let top = simplify(&cs[degree]);
    match numeric::sign(&top) {
        Some(0) | None => {
            if top.is_const() && !top.is_zero() {
                Some(Expr::zoo())
            } else {
                None
            }
        }
        Some(s) => Some(signed_inf(s * flip)),
    }
}

fn numeric_at(g: &Expr, k: u32, prec: u32) -> Option<Complex> {
    let mut env = Env::new();
    env.insert(
        Rc::from(T),
        Complex::with_val(prec, Float::with_val(prec, 1) >> k),
    );
    numeric::evalf_env(g, prec, &env).ok()
}

/// Whether the values of `g` near `t = 0+` approach the proposed limit.
fn plausible(g: &Expr, value: &Expr) -> bool {
    if !g.free_syms().iter().all(|s| &**s == T) || !value.is_const() {
        return true;
    }
    let prec = 256;
    let (Some(a), Some(b)) = (numeric_at(g, 40, prec), numeric_at(g, 80, prec)) else {
        return true;
    };
    let size = |z: &Complex| Float::with_val(64, z.abs_ref());
    if value.is_inf() || value.is_neg_inf() || value.is_zoo() {
        return size(&b) >= size(&a) || size(&b) > 1e6;
    }
    let Ok(l) = numeric::evalf(value, prec) else {
        return true;
    };
    let (da, db) = (
        size(&Complex::with_val(prec, &a - &l)),
        size(&Complex::with_val(prec, &b - &l)),
    );
    db <= da.clone() + 1e-30 || db < Float::with_val(64, size(&l) + 1u32) * 1e-9
}

/// The limit as `t -> 0` from above; the steps of a way that finds none are not told.
fn right_limit(g: &Expr, depth: u32) -> Option<Expr> {
    crate::trace::trial(|| right_limit_rules(g, depth))
}

fn right_limit_rules(g: &Expr, depth: u32) -> Option<Expr> {
    use crate::trace::step;
    let t = Expr::sym(T);
    let direct = simplify(&subst(g, &t, &Expr::zero()));
    if settled(&direct) && !direct.is_infinite() {
        step(|| format!("{g} has a value at t = 0: {direct}"));
        return Some(direct);
    }
    for n in [6usize, 12, 24] {
        let Some(s) = series::expand(g, T, n) else {
            break;
        };
        let s = s.trim();
        if let Some(c) = s.c.first() {
            if let Some(v) = leading(c, s.lo) {
                step(|| {
                    let power = match (s.lo, s.den) {
                        (0, _) => String::new(),
                        (1, 1) => " t".to_string(),
                        (lo, 1) => format!(" t^{lo}"),
                        (lo, den) => format!(" t^({lo}/{den})"),
                    };
                    format!(
                        "the series of {g} in t begins with ({c}){power}: as t -> 0 that goes to {v}"
                    )
                });
                return Some(v);
            }
            break;
        }
    }
    if depth < 6 {
        if let Expr::Pow(b, e) = g
            && !b.has_sym(T)
        {
            let inner = right_limit(e, depth + 1)?;
            let v = simplify(&pow((**b).clone(), inner.clone()));
            if settled(&v) && !v.is_undefined() {
                step(|| format!("{g}: the base to the limit {inner} of the exponent, {v}"));
                return Some(v);
            }
        }
        // b^e with both depending on t: exp(lim e ln b)
        if let Expr::Pow(b, e) = g
            && b.has_sym(T)
            && e.has_sym(T)
        {
            step(|| {
                format!("{g} is e to the power of ({e}) ln({b}): the limit of that exponent first")
            });
            let inner = right_limit(
                &simplify(&mul(vec![
                    (**e).clone(),
                    crate::funcs::func("ln", vec![(**b).clone()]),
                ])),
                depth + 1,
            )?;
            let v = simplify(&pow(Expr::e(), inner.clone()));
            step(|| format!("e to the power of {inner}: {v}"));
            return Some(v);
        }
        let (num, den) = numer_denom(g);
        if den.has_sym(T) && num.has_sym(T) {
            let (ln, ld) = (right_limit(&num, depth + 1), right_limit(&den, depth + 1));
            if let (Some(a), Some(b)) = (&ln, &ld) {
                let both_zero = a.is_zero() && b.is_zero();
                let both_inf =
                    a.is_infinite() && b.is_infinite() && !a.is_undefined() && !b.is_undefined();
                if both_zero || both_inf {
                    let next = simplify(&div(diff(&num, T), diff(&den, T)));
                    if next.size() <= g.size() * 3 + 40 {
                        step(|| {
                            format!(
                                "{g} is {} there: by the rule of L'Hopital, the derivative of the numerator over that of the denominator, {next}",
                                if both_zero { "0/0" } else { "inf/inf" }
                            )
                        });
                        return right_limit(&next, depth + 1);
                    }
                } else {
                    let q = simplify(&div(a.clone(), b.clone()));
                    if settled(&q) && !q.is_undefined() {
                        step(|| {
                            format!(
                                "{g}: the limit {a} of the numerator over the limit {b} of the denominator, {q}"
                            )
                        });
                        return Some(q);
                    }
                    if b.is_zero() && !a.is_zero() && !a.is_infinite() {
                        // finite / 0: the sign of the denominator near 0 decides
                        let side = numeric_at(&den, 60, 256)?;
                        let s = numeric::sign(a)?
                            * if side.real().is_sign_negative() {
                                -1
                            } else {
                                1
                            };
                        step(|| {
                            format!(
                                "{g}: the numerator goes to {a}, the denominator to 0 from one side: infinite, with the sign of both"
                            )
                        });
                        return side.imag().is_zero().then(|| signed_inf(s));
                    }
                }
            }
        }
        if let Expr::Add(terms) = g {
            let parts: Option<Vec<Expr>> =
                terms.iter().map(|x| right_limit(x, depth + 1)).collect();
            if let Some(parts) = parts {
                let total = simplify(&add(parts));
                if settled(&total) && !total.is_undefined() {
                    step(|| format!("{g}: the sum of the limits of its terms, {total}"));
                    return Some(total);
                }
            }
        }
        // an exponential that dies out beats the powers and logarithms beside it
        let dies = |f: &Expr| match f {
            Expr::Pow(b, u) if b.is_sym("e") && u.has_sym(T) => {
                // the size is e to the real part of the exponent
                let real = if u.any(&|x| matches!(x, Expr::Num(n) if !n.im.is_zero())) {
                    match crate::complex::rect(u) {
                        Some((re, _)) => simplify(&re),
                        None => return false,
                    }
                } else {
                    (**u).clone()
                };
                let (num, den) = ratsimp::fraction(&real);
                real.has_sym(T)
                    && ratsimp::coefficients(&num, T).is_some()
                    && ratsimp::coefficients(&den, T).is_some()
                    && right_limit(&real, depth + 1).is_some_and(|l| l.is_neg_inf())
            }
            _ => false,
        };
        if dies(g) {
            step(|| format!("{g}: an exponential whose exponent goes to -inf goes to 0"));
            return Some(Expr::zero());
        }
        if let Expr::Mul(factors) = g {
            if factors.iter().any(dies) && factors.iter().all(|f| dies(f) || tame(f)) {
                step(|| {
                    format!(
                        "{g}: an exponential that dies out beats the powers and logarithms beside it, 0"
                    )
                });
                return Some(Expr::zero());
            }
            let parts: Option<Vec<Expr>> =
                factors.iter().map(|x| right_limit(x, depth + 1)).collect();
            if let Some(parts) = parts {
                let total = simplify(&mul(parts));
                if settled(&total) && !total.is_undefined() {
                    step(|| format!("{g}: the product of the limits of its factors, {total}"));
                    return Some(total);
                }
            }
        }
        if let Expr::Fn(name, args) = g
            && args.len() == 1
        {
            let inner = right_limit(&args[0], depth + 1)?;
            let v = simplify(&crate::funcs::func(name, vec![inner.clone()]));
            if settled(&v) && !v.is_undefined() {
                step(|| format!("{g}: {name} of the limit {inner} of what is inside, {v}"));
                return Some(v);
            }
        }
    }
    None
}

/// Whether the expression grows no faster than a power near `t = 0`: no exponentials, and only
/// logarithms and bounded functions.
fn tame(e: &Expr) -> bool {
    !e.any(&|x| match x {
        Expr::Pow(_, p) => p.has_sym(T),
        Expr::Fn(n, a) => {
            a.iter().any(|y| y.has_sym(T))
                && !matches!(
                    &**n,
                    "ln" | "sin" | "cos" | "atan" | "erf" | "abs" | "sign" | "tanh"
                )
        }
        _ => false,
    })
}

/// Numeric estimate of the limit as `t -> 0+` when the values settle.
fn numeric_limit(g: &Expr) -> Option<Expr> {
    if !g.free_syms().iter().all(|s| &**s == T) {
        return None;
    }
    let digits = numeric::digits();
    let prec = numeric::bits_for(digits) * 4 + 256;
    let v: Vec<Complex> = [60u32, 120, 180, 240]
        .iter()
        .filter_map(|k| numeric_at(g, *k, prec))
        .collect();
    if v.len() < 4 {
        return None;
    }
    let size = |z: Complex| Float::with_val(64, z.abs_ref());
    let (d1, d2, d3) = (
        size(Complex::with_val(prec, &v[1] - &v[0])),
        size(Complex::with_val(prec, &v[2] - &v[1])),
        size(Complex::with_val(prec, &v[3] - &v[2])),
    );
    let scale = size(v[3].clone()) + 1u32;
    let tiny = Float::with_val(64, &scale >> (numeric::bits_for(digits) + 8));
    if d3 <= tiny && d2 <= Float::with_val(64, &d1 + &tiny) {
        crate::trace::step(|| {
            format!(
                "no rule gives the limit: the values at t = 2^-60, 2^-120, 2^-180 and 2^-240 settle to {digits} digits"
            )
        });
        return Some(numeric::to_float_expr(
            Complex::with_val(numeric::bits_for(digits) + 32, &v[3]),
            digits,
        ));
    }
    None
}

fn one_sided(f: &Expr, var: &str, point: &Expr, right: bool) -> Option<(Expr, bool)> {
    let x = Expr::sym(var);
    let t = Expr::sym(T);
    let at = if point.is_inf() {
        pow(t.clone(), Expr::minus_one())
    } else if point.is_neg_inf() {
        neg(pow(t.clone(), Expr::minus_one()))
    } else if right {
        add(vec![point.clone(), t.clone()])
    } else {
        sub(point.clone(), t.clone())
    };
    assume::set(T, Facts::positive());
    let g = simplify(&subst(f, &x, &at));
    crate::trace::step(|| {
        format!(
            "{}: with {var} = {at} it is the limit of {g} as t -> 0 from above",
            if point.is_infinite() {
                format!("as {var} -> {point}")
            } else if right {
                "from the right".to_string()
            } else {
                "from the left".to_string()
            }
        )
    });
    let _nest = crate::trace::nest();
    let found = right_limit(&g, 0)
        .filter(|v| plausible(&g, v))
        .map(|v| (v, false));
    let result = found.or_else(|| numeric_limit(&g).map(|v| (v, true)));
    assume::clear(T);
    result
}

/// `(limit, approximate)`.
pub fn limit(f: &Expr, var: &str, point: &Expr, dir: Dir) -> Result<(Expr, bool), String> {
    if !f.has_sym(var) {
        return Ok((f.clone(), false));
    }
    let given = f;
    let f = simplify(f);
    let infinite = point.is_inf() || point.is_neg_inf();
    crate::trace::step(|| {
        if f == *given {
            format!("the limit of {f} as {var} -> {point}")
        } else {
            format!("the limit of {given} as {var} -> {point}: the function simplifies to {f}")
        }
    });
    let _nest = crate::trace::nest();
    if !infinite {
        // a point where the function is defined and nothing degenerates
        let direct = simplify(&subst(&f, &Expr::sym(var), point));
        let regular = !direct.any(&|x| x.is_infinite())
            && !f.any(&|x| matches!(x, Expr::Fn(n, _) if matches!(&**n, "floor" | "ceil" | "round" | "trunc" | "frac" | "sign" | "abs" | "arg")));
        if regular {
            crate::trace::step(|| {
                format!(
                    "the function has a value at {var} = {point}, where nothing of it breaks down: {direct}"
                )
            });
            return Ok((direct, false));
        }
        crate::trace::step(|| {
            format!(
                "{var} = {point} cannot be put in (0/0, a division by 0 or a jump): the function is followed towards the point"
            )
        });
    }
    if infinite {
        let direct = simplify(&subst(&f, &Expr::sym(var), point));
        if settled(&direct) && !direct.is_undefined() {
            crate::trace::step(|| format!("with {var} = {point} every part has a value: {direct}"));
            return Ok((direct, false));
        }
    }
    let fail = || format!("no limit found for {f} as {var} -> {point}");
    if infinite || dir != Dir::Both {
        return one_sided(&f, var, point, dir != Dir::Left).ok_or_else(fail);
    }
    let (r, l) = (
        one_sided(&f, var, point, true),
        one_sided(&f, var, point, false),
    );
    match (r, l) {
        (Some((a, pa)), Some((b, pb))) => {
            let same = a == b
                || (!a.is_infinite() && !b.is_infinite() && {
                    if pa || pb {
                        numeric::evalf(&sub(a.clone(), b.clone()), 128)
                            .is_ok_and(|d| Float::with_val(64, d.abs_ref()) < 1e-25)
                    } else {
                        simplify::is_zero(&sub(a.clone(), b.clone()))
                    }
                });
            if same {
                Ok((a, pa || pb))
            } else {
                Err(format!(
                    "the limit does not exist: from the left it is {b}, from the right {a}"
                ))
            }
        }
        _ => Err(fail()),
    }
}
