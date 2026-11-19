//! The real line: inequalities in one variable and extrema.

use crate::canon::{add, mul, sub, subst_sym};
use crate::expr::{Expr, Rel};
use crate::limit::{self, Dir};
use crate::ntheory;
use crate::numeric::{self, Env};
use crate::ratsimp;
use crate::simplify::simplify;
use crate::solve;
use rug::{Complex, Float, Integer};
use std::rc::Rc;

const PREC: u32 = 192;

/// Value of `f` at a real point when it is real there.
fn real_at(f: &Expr, var: &str, x: &Float) -> Option<Float> {
    let mut env = Env::new();
    env.insert(Rc::from(var), Complex::with_val(PREC, x));
    let v = numeric::evalf_env(f, PREC, &env).ok()?;
    let tiny = Float::with_val(64, v.abs_ref()) >> 120u32;
    (Float::with_val(64, v.imag().abs_ref()) <= tiny).then(|| v.real().clone())
}

fn value_of(e: &Expr) -> Option<Float> {
    numeric::real_value(e, 40).map(|v| Float::with_val(PREC, v))
}

/// Real solutions of an equation: single points and families `base + k step` with `step > 0`.
#[derive(Default)]
struct RealRoots {
    fixed: Vec<Expr>,
    families: Vec<(Expr, Expr)>,
}

fn real_roots(g: &Expr, var: &str) -> Result<RealRoots, String> {
    let mut out = RealRoots::default();
    if !g.has_sym(var) {
        return Ok(out);
    }
    let solved = solve::solve(g, var)?;
    let mut failure = None;
    for r in &solved.roots {
        let params: Vec<&String> = solved
            .params
            .iter()
            .filter(|p| r.value.has_sym(p))
            .collect();
        match params.as_slice() {
            [] => {
                if value_of(&r.value).is_some() {
                    out.fixed.push(r.value.clone());
                }
            }
            [k] => {
                let at = |n: i64| simplify(&subst_sym(&r.value, k, &Expr::int(n)));
                let base = at(0);
                let step = simplify(&sub(at(1), base.clone()));
                match (value_of(&base), value_of(&step)) {
                    (Some(_), Some(s)) if !s.is_zero() => {
                        out.families.push((
                            base,
                            if s.is_sign_negative() {
                                crate::canon::neg(step)
                            } else {
                                step
                            },
                        ));
                    }
                    _ => {
                        // a family leaving the real line: at most one member is real
                        let member = (-3..=3).map(at).find(|m| value_of(m).is_some());
                        out.fixed.extend(member);
                    }
                }
            }
            _ => {
                failure = Some(
                    "solutions with several integer parameters are not supported here".to_string(),
                )
            }
        }
    }
    solve::release(&solved.params);
    match failure {
        Some(e) => Err(e),
        None => Ok(out),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Zero,
    Pole,
    Edge,
}

/// Expressions whose real zeros are poles or bounds of the real domain of `f`.
fn domain_edges(f: &Expr, var: &str) -> Vec<(Expr, Kind)> {
    let mut out = Vec::new();
    f.walk(&mut |e| match e {
        Expr::Pow(b, x) if b.has_sym(var) && x.as_rat().is_some_and(|r| !r.is_integer()) => {
            out.push(((**b).clone(), Kind::Edge))
        }
        Expr::Fn(n, a) if a.len() == 1 && a[0].has_sym(var) => match &**n {
            "ln" => out.push((a[0].clone(), Kind::Edge)),
            "tan" => out.push((crate::funcs::func("cos", vec![a[0].clone()]), Kind::Pole)),
            "asin" | "acos" | "atanh" => {
                out.push((sub(a[0].clone(), Expr::one()), Kind::Edge));
                out.push((add(vec![a[0].clone(), Expr::one()]), Kind::Edge));
            }
            "acosh" => out.push((sub(a[0].clone(), Expr::one()), Kind::Edge)),
            _ => {}
        },
        _ => {}
    });
    out
}

/// Expressions at whose real zeros `f` may have a corner.
fn kinks(f: &Expr, var: &str) -> Vec<Expr> {
    let mut out = Vec::new();
    f.walk(&mut |e| match e {
        Expr::Fn(n, a) if matches!(&**n, "abs" | "sign") && a.len() == 1 && a[0].has_sym(var) => {
            out.push(a[0].clone())
        }
        Expr::Fn(n, a) if matches!(&**n, "min" | "max") => {
            for (i, x) in a.iter().enumerate() {
                out.extend(a[i + 1..].iter().map(|y| sub(x.clone(), y.clone())));
            }
        }
        Expr::Fn(n, a) if &**n == "piecewise" => {
            let mut edges = Vec::new();
            for (cond, _) in crate::logic::pieces(a).0 {
                crate::logic::boundaries(cond, &mut edges);
            }
            out.extend(edges.into_iter().filter(|g| g.has_sym(var)));
        }
        Expr::Fn(n, a)
            if matches!(&**n, "heaviside" | "unitstep") && a.len() == 1 && a[0].has_sym(var) =>
        {
            out.push(a[0].clone())
        }
        _ => {}
    });
    out
}

const STEPS: [&str; 5] = ["floor", "ceil", "round", "trunc", "frac"];

/// A real bound of an interval: `None` for an infinite one.
fn bound(e: &Expr) -> Option<Option<Float>> {
    if e.is_infinite() {
        Some(None)
    } else {
        value_of(e).map(Some)
    }
}

/// Points strictly between `a < b` where `f` has a corner or a jump: zeros of the arguments of abs and
/// sign, crossings of min and max, steps of floor and its kin. `None` when they cannot be listed.
pub fn corners(f: &Expr, var: &str, a: &Expr, b: &Expr) -> Option<Vec<Expr>> {
    let (lo, hi) = (bound(a)?, bound(b)?);
    let inside = |v: &Float| lo.as_ref().is_none_or(|l| v > l) && hi.as_ref().is_none_or(|h| v < h);
    let mut points: Vec<(Float, Expr)> = Vec::new();
    let mut keep = |e: Expr| {
        if let Some(v) = value_of(&e).filter(|v| inside(v))
            && !points
                .iter()
                .any(|(p, _)| Float::with_val(64, p - &v).abs() < 1e-30)
        {
            points.push((v, e));
        }
    };
    for g in kinks(f, var) {
        let roots = real_roots(&g, var).ok()?;
        roots.fixed.into_iter().for_each(&mut keep);
        for (base, step) in roots.families {
            let (l, h) = (lo.as_ref()?, hi.as_ref()?);
            let (bv, sv) = (value_of(&base)?, value_of(&step)?);
            let first = Float::with_val(PREC, Float::with_val(PREC, l - &bv) / &sv)
                .ceil()
                .to_integer()?
                .to_i64()?;
            let last = Float::with_val(PREC, Float::with_val(PREC, h - &bv) / &sv)
                .floor()
                .to_integer()?
                .to_i64()?;
            if last - first > 2000 {
                return None;
            }
            (first..=last).for_each(|k| {
                keep(simplify(&add(vec![
                    base.clone(),
                    mul(vec![Expr::int(k), step.clone()]),
                ])))
            });
        }
    }
    let mut steps = Vec::new();
    f.walk(&mut |e| {
        if let Expr::Fn(n, args) = e
            && STEPS.contains(&&**n)
            && args.len() == 1
            && args[0].has_sym(var)
        {
            steps.push((n.to_string(), args[0].clone()));
        }
    });
    for (name, u) in steps {
        // u = c0 + c1 x jumps where it is whole (round: whole plus a half)
        let c = ratsimp::coefficients(&u, var).filter(|c| c.len() == 2)?;
        let (c0, c1) = (value_of(&c[0])?, value_of(&c[1])?);
        let (l, h) = (lo.as_ref()?, hi.as_ref()?);
        let shift = if name == "round" { 0.5 } else { 0.0 };
        let end = |x: &Float| -> Float {
            Float::with_val(PREC, &c0 + Float::with_val(PREC, &c1 * x)) - shift
        };
        let ends: [Float; 2] = [end(l), end(h)];
        let (from, to) = if ends[0] <= ends[1] {
            (&ends[0], &ends[1])
        } else {
            (&ends[1], &ends[0])
        };
        let (first, last) = (
            from.clone().ceil().to_integer()?.to_i64()?,
            to.clone().floor().to_integer()?.to_i64()?,
        );
        if last - first > 2000 {
            return None;
        }
        for k in first..=last {
            let level = if name == "round" {
                Expr::rat(2 * k + 1, 2)
            } else {
                Expr::int(k)
            };
            keep(simplify(&crate::canon::div(
                sub(level, c[0].clone()),
                c[1].clone(),
            )));
        }
    }
    points.sort_by(|x, y| x.0.total_cmp(&y.0));
    Some(points.into_iter().map(|(_, e)| e).collect())
}

/// `f` with abs, sign, min, max, floor and its kin replaced by what they are between `a` and `b`.
pub fn smooth_between(f: &Expr, var: &str, a: &Expr, b: &Expr) -> Option<Expr> {
    let mid = match (bound(a)?, bound(b)?) {
        (Some(l), Some(h)) => Float::with_val(PREC, &l + &h) / 2u32,
        (Some(l), None) => l + 1u32,
        (None, Some(h)) => h - 1u32,
        (None, None) => Float::with_val(PREC, 0.1234),
    };
    let mut ok = true;
    let out = crate::canon::map_bottom_up(f, &mut |e| {
        let Expr::Fn(n, args) = &e else { return e };
        if !args.iter().any(|x| x.has_sym(var)) {
            return e;
        }
        let at = |u: &Expr| real_at(u, var, &mid);
        if &**n == "piecewise" {
            // the piece that holds in the middle holds between the corners
            let x = Expr::sym(var);
            let point = crate::numeric::to_float_expr(
                rug::Complex::with_val(PREC, &mid),
                crate::numeric::digits(),
            );
            let (pairs, otherwise) = crate::logic::pieces(args);
            for (cond, value) in pairs {
                match crate::logic::truth(&crate::canon::subst(cond, &x, &point)) {
                    Some(true) => return value.clone(),
                    Some(false) => {}
                    None => {
                        ok = false;
                        return e;
                    }
                }
            }
            return match otherwise {
                Some(value) => value.clone(),
                None => {
                    ok = false;
                    e
                }
            };
        }
        let replaced = match (&**n, args.len()) {
            ("abs", 1) => at(&args[0]).filter(|v| !v.is_zero()).map(|v| {
                if v.is_sign_negative() {
                    crate::canon::neg(args[0].clone())
                } else {
                    args[0].clone()
                }
            }),
            ("sign", 1) => at(&args[0])
                .filter(|v| !v.is_zero())
                .map(|v| Expr::int(if v.is_sign_negative() { -1 } else { 1 })),
            ("heaviside" | "unitstep", 1) => at(&args[0])
                .filter(|v| !v.is_zero())
                .map(|v| Expr::int(i64::from(!v.is_sign_negative()))),
            ("floor" | "ceil" | "round" | "trunc", 1) => {
                at(&e).and_then(|v| v.to_integer()).map(Expr::integer)
            }
            ("frac", 1) => at(&args[0])
                .and_then(|v| v.trunc().to_integer())
                .map(|k| sub(args[0].clone(), Expr::integer(k))),
            ("min" | "max", _) => {
                let values: Option<Vec<Float>> = args.iter().map(&at).collect();
                values.and_then(|vs| {
                    let pick = |x: &&(usize, &Float), y: &&(usize, &Float)| x.1.total_cmp(y.1);
                    let indexed: Vec<(usize, &Float)> = vs.iter().enumerate().collect();
                    let best = if &**n == "min" {
                        indexed.iter().min_by(pick)
                    } else {
                        indexed.iter().max_by(pick)
                    };
                    best.map(|(i, _)| args[*i].clone())
                })
            }
            _ => return e,
        };
        replaced.unwrap_or_else(|| {
            ok = false;
            e
        })
    });
    ok.then_some(out)
}

struct Point {
    at: Float,
    exact: Expr,
    /// What the point is for each relation.
    kinds: Vec<Option<Kind>>,
}

fn holds(op: Rel, v: &Float) -> bool {
    match op {
        Rel::Lt => v.is_sign_negative() && !v.is_zero(),
        Rel::Le => v.is_sign_negative() || v.is_zero(),
        Rel::Gt => v.is_sign_positive() && !v.is_zero(),
        Rel::Ge => v.is_sign_positive() || v.is_zero(),
        Rel::Eq => v.is_zero(),
        Rel::Ne => !v.is_zero(),
    }
}

fn insert(points: &mut Vec<Point>, exact: Expr, which: usize, count: usize, kind: Kind) {
    let Some(at) = value_of(&exact) else { return };
    let near = |a: &Float, b: &Float| {
        let scale = Float::with_val(64, a.abs_ref()) + 1u32;
        Float::with_val(64, a - b).abs() < scale >> 100u32
    };
    let merged = |old: Option<Kind>| match (old, kind) {
        (Some(Kind::Pole), _) | (_, Kind::Pole) => Kind::Pole,
        (Some(Kind::Zero), _) | (_, Kind::Zero) => Kind::Zero,
        _ => Kind::Edge,
    };
    match points.iter_mut().find(|p| near(&p.at, &at)) {
        Some(p) => p.kinds[which] = Some(merged(p.kinds[which])),
        None => {
            let mut kinds = vec![None; count];
            kinds[which] = Some(kind);
            points.push(Point { at, exact, kinds });
        }
    }
}

fn free_name(rels: &[(Expr, Rel)], var: &str) -> String {
    ["k", "n", "m", "j"]
        .iter()
        .find(|c| **c != var && !rels.iter().any(|(f, _)| f.has_sym(c)))
        .unwrap_or(&"k_")
        .to_string()
}

/// One end of a solution piece.
struct End {
    value: Option<Expr>,
    closed: bool,
}

fn piece(var: &str, lo: &End, hi: &End) -> String {
    let lt = |closed: bool| if closed { "<=" } else { "<" };
    match (&lo.value, &hi.value) {
        (None, None) => format!("{var} is any real number"),
        (Some(a), Some(b)) if a == b => format!("{var} = {a}"),
        (Some(a), Some(b)) => format!("{a} {} {var} {} {b}", lt(lo.closed), lt(hi.closed)),
        (Some(a), None) => format!("{var} {} {a}", if lo.closed { ">=" } else { ">" }),
        (None, Some(b)) => format!("{var} {} {b}", lt(hi.closed)),
    }
}

/// Solves the relations `f op 0`, all at once, over the reals; returns the printed solution.
pub fn inequalities(rels: &[(Expr, Rel)], var: &str) -> Result<Vec<String>, String> {
    let rels: Vec<(Expr, Rel)> = rels.iter().map(|(f, op)| (simplify(f), *op)).collect();
    let count = rels.len();
    let fail = |e: String| format!("cannot solve the inequality: {e}");
    // (relation, base, step or none, kind)
    let mut fixed: Vec<(usize, Expr, Kind)> = Vec::new();
    let mut families: Vec<(usize, Expr, Expr, Kind)> = Vec::new();
    for (i, (f, _)) in rels.iter().enumerate() {
        let (num, den) = ratsimp::fraction(f);
        let mut sources = vec![
            (real_roots(&num, var).map_err(fail)?, Kind::Zero),
            (real_roots(&den, var).map_err(fail)?, Kind::Pole),
        ];
        for (g, kind) in domain_edges(f, var) {
            if let Ok(r) = real_roots(&g, var) {
                sources.push((r, kind));
            }
        }
        for (roots, kind) in sources {
            fixed.extend(roots.fixed.into_iter().map(|r| (i, r, kind)));
            families.extend(roots.families.into_iter().map(|(b, s)| (i, b, s, kind)));
        }
    }
    let inside = |x: &Float| {
        rels.iter()
            .all(|(f, op)| real_at(f, var, x).is_some_and(|v| holds(*op, &v)))
    };
    let on = |p: &Point| {
        rels.iter().zip(&p.kinds).all(|((f, op), kind)| match kind {
            Some(Kind::Zero) => matches!(op, Rel::Le | Rel::Ge | Rel::Eq),
            Some(Kind::Pole) => false,
            _ => real_at(f, var, &p.at).is_some_and(|v| holds(*op, &v)),
        })
    };
    let everything = format!("{var} is any real number");
    let nothing = "no real solution".to_string();
    let mut points: Vec<Point> = Vec::new();
    if families.is_empty() {
        for (i, r, kind) in fixed {
            insert(&mut points, r, i, count, kind);
        }
        points.sort_by(|a, b| a.at.total_cmp(&b.at));
        let n = points.len();
        // flags: interval 0, point 0, interval 1, ..., point n-1, interval n
        let mut flags = Vec::with_capacity(2 * n + 1);
        for i in 0..=n {
            let sample = match (
                i.checked_sub(1).map(|j| &points[j].at),
                points.get(i).map(|p| &p.at),
            ) {
                (None, None) => Float::with_val(PREC, 0.1234),
                (None, Some(b)) => Float::with_val(PREC, b - 1u32),
                (Some(a), None) => Float::with_val(PREC, a + 1u32),
                (Some(a), Some(b)) => Float::with_val(PREC, a + b) / 2u32,
            };
            flags.push(inside(&sample));
            if i < n {
                flags.push(on(&points[i]));
            }
        }
        let mut pieces = Vec::new();
        let mut i = 0;
        while i < flags.len() {
            if !flags[i] {
                i += 1;
                continue;
            }
            let mut j = i;
            while j + 1 < flags.len() && flags[j + 1] {
                j += 1;
            }
            let end = |t: usize, low: bool| -> End {
                if t % 2 == 1 {
                    return End {
                        value: Some(points[t / 2].exact.clone()),
                        closed: true,
                    };
                }
                let k = t / 2;
                let p = if low {
                    k.checked_sub(1)
                } else {
                    (k < n).then_some(k)
                };
                End {
                    value: p.map(|p| points[p].exact.clone()),
                    closed: false,
                }
            };
            pieces.push(piece(var, &end(i, true), &end(j, false)));
            i = j + 1;
        }
        return Ok(vec![if pieces.is_empty() {
            nothing
        } else {
            pieces.join(" or ")
        }]);
    }
    if !fixed.is_empty() {
        return Err(fail(
            "it has both isolated and periodic critical points".to_string(),
        ));
    }
    // common period of the families, then the period of the relations themselves
    let no_period = || fail("the critical points have no common period".to_string());
    let first = value_of(&families[0].2).ok_or_else(no_period)?;
    let (mut top, mut low) = (Integer::from(1), Integer::from(1));
    for (i, family) in families.iter().enumerate() {
        let ratio = Float::with_val(PREC, value_of(&family.2).ok_or_else(no_period)? / &first);
        let r = ntheory::best_rational(
            &ratio.to_rational().ok_or_else(no_period)?,
            &Integer::from(1000),
        );
        if Float::with_val(PREC, Float::with_val(PREC, &r) - &ratio).abs() > 1e-30 {
            return Err(no_period());
        }
        if i == 0 {
            (top, low) = (r.numer().clone(), r.denom().clone());
        } else {
            top = top.lcm(r.numer());
            low = low.gcd(r.denom());
        }
    }
    let common = simplify(&mul(vec![
        families[0].2.clone(),
        Expr::rational(rug::Rational::from((top, low))),
    ]));
    let repeats = |p: &Float| {
        rels.iter().all(|(f, _)| {
            [0.37, 1.91, -2.63].iter().all(|x| {
                let x = Float::with_val(PREC, *x);
                let shifted = Float::with_val(PREC, &x + p);
                match (real_at(f, var, &x), real_at(f, var, &shifted)) {
                    (Some(a), Some(b)) => {
                        Float::with_val(64, &a - &b).abs()
                            <= (Float::with_val(64, a.abs_ref()) + 1u32) >> 100u32
                    }
                    (None, None) => true,
                    _ => false,
                }
            })
        })
    };
    let period = [1, 2, 3, 4, 6]
        .iter()
        .map(|m| simplify(&mul(vec![Expr::int(*m), common.clone()])))
        .find(|p| value_of(p).is_some_and(|v| repeats(&v)))
        .ok_or_else(|| fail("the relation is not periodic".to_string()))?;
    let span = value_of(&period).unwrap();
    let last = Float::with_val(PREC, &span - Float::with_val(PREC, &span >> 80u32));
    for (i, base, step, kind) in &families {
        let (b, s) = (value_of(base).unwrap(), value_of(step).unwrap());
        let start = (-Float::with_val(PREC, &b / &s)).ceil();
        let mut k = start
            .to_integer()
            .and_then(|k| k.to_i64())
            .ok_or_else(|| fail("the critical points are too far".to_string()))?;
        while Float::with_val(PREC, &b + Float::with_val(PREC, &s * k)) < last
            && points.len() <= 400
        {
            insert(
                &mut points,
                simplify(&add(vec![
                    base.clone(),
                    mul(vec![Expr::int(k), step.clone()]),
                ])),
                *i,
                count,
                *kind,
            );
            k += 1;
        }
    }
    points.sort_by(|a, b| a.at.total_cmp(&b.at));
    let n = points.len();
    if n == 0 {
        return Ok(vec![if inside(&Float::with_val(PREC, 0.1234)) {
            everything
        } else {
            nothing
        }]);
    }
    // cyclic flags: point 0, interval (0, 1), point 1, ..., interval (n-1, 0 + period)
    let mut flags = Vec::with_capacity(2 * n);
    for i in 0..n {
        flags.push(on(&points[i]));
        let next = if i + 1 < n {
            points[i + 1].at.clone()
        } else {
            Float::with_val(PREC, &points[0].at + &span)
        };
        flags.push(inside(
            &(Float::with_val(PREC, &points[i].at + &next) / 2u32),
        ));
    }
    let Some(start) = flags.iter().position(|x| !*x) else {
        return Ok(vec![everything]);
    };
    let name = free_name(&rels, var);
    let known = crate::assume::get(&name);
    crate::assume::set(&name, crate::assume::Facts::integer());
    let k = Expr::sym(&name);
    let shifted = |index: usize, turn: usize| -> Expr {
        let turns = (index / n) as i64 - turn as i64;
        simplify(&add(vec![
            points[index % n].exact.clone(),
            mul(vec![Expr::int(turns), period.clone()]),
            mul(vec![k.clone(), period.clone()]),
        ]))
    };
    let mut pieces = Vec::new();
    let mut i = start + 1;
    while i < start + 2 * n {
        if !flags[i % (2 * n)] {
            i += 1;
            continue;
        }
        let mut j = i;
        while j + 1 < start + 2 * n && flags[(j + 1) % (2 * n)] {
            j += 1;
        }
        // item t: even = point t/2, odd = the interval after point t/2
        let turn = (i / 2) / n;
        let lo = End {
            value: Some(shifted(i / 2, turn)),
            closed: i % 2 == 0,
        };
        let hi = End {
            value: Some(shifted(if j % 2 == 0 { j / 2 } else { j / 2 + 1 }, turn)),
            closed: j % 2 == 0,
        };
        pieces.push(piece(var, &lo, &hi));
        i = j + 1;
    }
    crate::assume::clear(&name);
    if known != crate::assume::Facts::default() {
        crate::assume::set(&name, known);
    }
    Ok(vec![if pieces.is_empty() {
        nothing
    } else {
        format!("{}, {name} is an integer", pieces.join(" or "))
    }])
}

/// Smallest (or largest) value of `f` on the real line or on `[lo, hi]`: the value and where it is taken.
pub fn extremum(
    f: &Expr,
    var: &str,
    lo: Option<&Expr>,
    hi: Option<&Expr>,
    minimize: bool,
    notes: &mut Vec<String>,
) -> Result<Expr, String> {
    let f = simplify(f);
    let x = Expr::sym(var);
    let better = |a: &Float, b: &Float| if minimize { a < b } else { a > b };
    let bound = |e: Option<&Expr>| -> Result<Option<Float>, String> {
        match e {
            Some(e) if e.is_infinite() => Ok(None),
            Some(e) => value_of(e)
                .map(Some)
                .ok_or_else(|| format!("{e} is not a real number")),
            None => Ok(None),
        }
    };
    let (low, high) = (bound(lo)?, bound(hi)?);
    let inside =
        |v: &Float| low.as_ref().is_none_or(|l| v >= l) && high.as_ref().is_none_or(|h| v <= h);
    let df = simplify(&crate::diff::diff(&f, var));
    if f.free_syms().iter().any(|s| **s != *var) {
        // parameters: the stationary points of the right curvature, when there is exactly one
        let solved = solve::solve(&df, var)?;
        solve::release(&solved.params);
        let second = crate::diff::diff(&df, var);
        let wanted = if minimize { 1 } else { -1 };
        let mut hits = solved
            .roots
            .iter()
            .filter(|r| !solved.params.iter().any(|p| r.value.has_sym(p)))
            .filter_map(|r| {
                let curve = numeric::sign(&simplify(&crate::canon::subst(&second, &x, &r.value)));
                (curve == Some(wanted)).then(|| r.value.clone())
            });
        return match (hits.next(), hits.next(), solved.roots.len()) {
            (Some(p), None, 1) => {
                notes.push(format!(
                    "{var} = {p} is the only stationary point; it is a local {}",
                    if minimize { "minimum" } else { "maximum" }
                ));
                Ok(Expr::list(vec![
                    simplify(&crate::canon::subst(&f, &x, &p)),
                    Expr::eq(x.clone(), p),
                ]))
            }
            _ => Err(format!(
                "cannot compare the values of {f}: they depend on parameters"
            )),
        };
    }
    let (num, den) = ratsimp::fraction(&df);
    let critical =
        real_roots(&num, var).map_err(|e| format!("cannot find the critical points: {e}"))?;
    let mut places: Vec<Expr> = critical.fixed.clone();
    let mut singular = real_roots(&den, var).unwrap_or_default();
    for (g, _) in domain_edges(&f, var) {
        if let Ok(r) = real_roots(&g, var) {
            singular.fixed.extend(r.fixed);
        }
    }
    places.extend(singular.fixed.iter().cloned());
    for g in kinks(&f, var) {
        if let Ok(r) = real_roots(&g, var) {
            places.extend(r.fixed);
        }
    }
    for (base, step) in critical.families.iter().chain(&singular.families) {
        let (b, s) = (value_of(base).unwrap(), value_of(step).unwrap());
        let range = match (&low, &high) {
            (Some(l), Some(h)) => {
                let first = Float::with_val(PREC, Float::with_val(PREC, l - &b) / &s).ceil();
                let last = Float::with_val(PREC, Float::with_val(PREC, h - &b) / &s).floor();
                match (
                    first.to_integer().and_then(|k| k.to_i64()),
                    last.to_integer().and_then(|k| k.to_i64()),
                ) {
                    (Some(a), Some(z)) if z - a <= 400 => a..=z,
                    _ => return Err("too many critical points in the interval".to_string()),
                }
            }
            _ => -1..=1,
        };
        places.extend(range.map(|k| {
            simplify(&add(vec![
                base.clone(),
                mul(vec![Expr::int(k), step.clone()]),
            ]))
        }));
    }
    places.extend(lo.filter(|e| !e.is_infinite()).cloned());
    places.extend(hi.filter(|e| !e.is_infinite()).cloned());
    let mut best: Option<(Float, Expr, Vec<Expr>)> = None;
    for p in places {
        let Some(at) = value_of(&p) else { continue };
        if !inside(&at) {
            continue;
        }
        let value = simplify(&crate::canon::subst(&f, &x, &p));
        let Some(v) = value_of(&value) else { continue };
        match &mut best {
            Some((bv, _, at_points))
                if Float::with_val(64, &v - &*bv).abs()
                    <= (Float::with_val(64, bv.abs_ref()) + 1u32) >> 90u32 =>
            {
                if !at_points.contains(&p) {
                    at_points.push(p);
                }
            }
            Some((bv, _, _)) if !better(&v, bv) => {}
            _ => best = Some((v, value, vec![p])),
        }
    }
    // the ends of the line and the poles: unbounded, or a value that is only approached
    let unbounded = if minimize {
        Expr::neg_inf()
    } else {
        Expr::inf()
    };
    let word = if minimize { "below" } else { "above" };
    let mut approached: Option<(Float, Expr, String)> = None;
    let mut ends: Vec<(Expr, Dir, String)> = Vec::new();
    if low.is_none() {
        ends.push((Expr::neg_inf(), Dir::Both, format!("{var} -> -inf")));
    }
    if high.is_none() {
        ends.push((Expr::inf(), Dir::Both, format!("{var} -> inf")));
    }
    for p in &singular.fixed {
        let Some(v) = value_of(p).filter(|v| inside(v)) else {
            continue;
        };
        if low.as_ref().is_none_or(|l| v > *l) {
            ends.push((p.clone(), Dir::Left, format!("{var} -> {p} from the left")));
        }
        if high.as_ref().is_none_or(|h| v < *h) {
            ends.push((
                p.clone(),
                Dir::Right,
                format!("{var} -> {p} from the right"),
            ));
        }
    }
    for (point, dir, how) in ends {
        let Ok((l, _)) = limit::limit(&f, var, &point, dir) else {
            continue;
        };
        if l == unbounded {
            notes.push(format!("{f} is unbounded {word} ({how})"));
            return Ok(unbounded);
        }
        if let Some(v) = value_of(&l)
            && approached.as_ref().is_none_or(|(a, _, _)| better(&v, a))
        {
            approached = Some((v, l, how));
        }
    }
    match (best, approached) {
        (Some((bv, _, _)), Some((av, value, how))) if better(&av, &bv) => {
            notes.push(format!(
                "the value {value} is approached ({how}) but not taken"
            ));
            Ok(Expr::list(vec![value]))
        }
        (None, Some((_, value, how))) => {
            notes.push(format!(
                "the value {value} is approached ({how}) but not taken"
            ));
            Ok(Expr::list(vec![value]))
        }
        (Some((_, value, mut points)), _) => {
            if points.iter().any(|p| !matches!(p, Expr::Float(_))) {
                points.retain(|p| !matches!(p, Expr::Float(_)));
            }
            let mut items = vec![value];
            items.extend(points.into_iter().map(|p| Expr::eq(x.clone(), p)));
            Ok(Expr::list(items))
        }
        (None, None) => Err(format!(
            "no {} found for {f}",
            if minimize { "minimum" } else { "maximum" }
        )),
    }
}
