//! Solvers that work on functions: values at both ends of a stretch, equations that look
//! back in time, equations of heat in one dimension of space, and the least value of a
//! function sought without its slopes.

use crate::decomp::{CMat, from_complex_list};
use crate::eval::Session;
use crate::expr::Expr;
use crate::hypotests::positive;
use crate::mfun::real;
use crate::numeric;
use crate::odesys::{integrate, integrate_by};
use crate::pencils::solved;
use crate::texts::text;
use rug::float::Constant;
use rug::{Complex, Float};
use std::rc::Rc;

type Res = Result<Expr, String>;

/// The guess of the unknown functions at a point of the mesh.
type Guess = Box<dyn Fn(usize, &Float) -> Result<Vec<Complex>, String>>;

/// Expressions of named arguments, worked out with numbers.
struct Many {
    keys: Vec<Rc<str>>,
    items: Vec<Expr>,
}

impl Many {
    fn new(params: &[String], body: &Expr) -> Many {
        let mut items = Vec::new();
        flat(body, &mut items);
        Many {
            keys: params.iter().map(|p| Rc::from(p.as_str())).collect(),
            items,
        }
    }

    fn at(&self, args: &[Complex], prec: u32) -> Option<Vec<Complex>> {
        let mut env = numeric::Env::new();
        for (k, v) in self.keys.iter().zip(args) {
            env.insert(k.clone(), Complex::with_val(prec, v));
        }
        self.items
            .iter()
            .map(|r| numeric::evalf_env(r, prec, &env).ok())
            .collect()
    }
}

fn flat(e: &Expr, out: &mut Vec<Expr>) {
    match e.as_list() {
        Some(items) => items.iter().for_each(|x| flat(x, out)),
        None => out.push(e.clone()),
    }
}

fn size(z: &Complex) -> Float {
    Float::with_val(z.prec().0, z.abs_ref())
}

fn largest(v: &[Complex], prec: u32) -> Float {
    v.iter().fold(Float::new(prec), |s, z| {
        s.max(&Float::with_val(prec, z.abs_ref()))
    })
}

fn number(e: &Expr, prec: u32, name: &str) -> Result<Complex, String> {
    numeric::evalf_digits(e, numeric::digits() + 8)
        .map(|v| Complex::with_val(prec, v))
        .map_err(|_| format!("{name}: {e} is not a number"))
}

fn list_of(e: &Expr) -> Vec<Expr> {
    match e.as_list() {
        Some(items) => items.to_vec(),
        None => vec![e.clone()],
    }
}

/// Rows `[t, y, ...]`; a value that is nothing beside the largest of all shows as 0.
fn rows_of(times: &[Expr], values: &[Vec<Complex>]) -> Expr {
    let prec = values[0][0].prec().0;
    let top = values
        .iter()
        .flatten()
        .fold(Float::new(prec), |s, z| s.max(&size(z)));
    let small = top * Float::with_val(prec, Float::i_pow_u(10, numeric::digits() + 2)).recip();
    let clean = |x: &Float| match Float::with_val(prec, x.abs_ref()) <= small {
        true => Float::new(prec),
        false => x.clone(),
    };
    Expr::list(
        times
            .iter()
            .zip(values)
            .map(|(at, y)| {
                let y: Vec<Complex> = y
                    .iter()
                    .map(|z| Complex::with_val(prec, (clean(z.real()), clean(z.imag()))))
                    .collect();
                let mut row = vec![at.clone()];
                row.extend(from_complex_list(&y));
                Expr::list(row)
            })
            .collect(),
    )
}

/// The polynomial through values at the points of Chebyshev of a stretch.
struct Piece {
    from: Float,
    /// the points from the start of the stretch to its end, and the values there
    points: Vec<Float>,
    values: Vec<Vec<Complex>>,
}

impl Piece {
    fn new(from: &Float, to: &Float, count: usize, prec: u32) -> Piece {
        let half = Float::with_val(prec, to - from) / 2u32;
        let middle = Float::with_val(prec, to + from) / 2u32;
        let points = (0..=count)
            .map(|j| {
                let angle = Float::with_val(prec, Constant::Pi) * j as u32 / count as u32;
                Float::with_val(prec, &middle - Float::with_val(prec, &half * angle.cos()))
            })
            .collect();
        Piece {
            from: from.clone(),
            points,
            values: vec![Vec::new(); count + 1],
        }
    }

    fn at(&self, t: &Float, prec: u32) -> Vec<Complex> {
        let last = self.values.len() - 1;
        let size = self.values[0].len();
        let mut above = vec![Complex::new(prec); size];
        let mut below = Float::new(prec);
        for (j, (point, values)) in self.points.iter().zip(&self.values).enumerate() {
            let off = Float::with_val(prec, t - point);
            if off.is_zero() {
                return values.clone();
            }
            let mut weight = off.recip();
            if j == 0 || j == last {
                weight /= 2u32;
            }
            if j % 2 == 1 {
                weight = -weight;
            }
            for (sum, v) in above.iter_mut().zip(values) {
                *sum += Complex::with_val(prec, v * &weight);
            }
            below += weight;
        }
        above.into_iter().map(|z| z / &below).collect()
    }
}

impl Session {
    /// A function of so many arguments as expressions of fresh names.
    fn many(&mut self, f: &Expr, count: usize, name: &str) -> Result<Many, String> {
        let (params, body) = self.function_of(f, count, name)?;
        Ok(Many::new(&params, &body))
    }

    /// `bvp4c(f, conditions, mesh, guess[, parameters])`: functions with conditions at both
    /// ends of a stretch, by shooting from many points at once and the steps of Newton.
    fn bvp(&mut self, name: &str, a: &[Expr]) -> Res {
        let usage = format!(
            "{name}(@(x, y, z) [slope of y, slope of z], @(ya, za, yb, zb) [what is zero at the ends], [x0, ..., x1], [guess of y, guess of z])"
        );
        let (mesh, guess, known) = match a.len() {
            3 => {
                let get =
                    |s: &mut Session, field: &str| s.call("getfield", &[a[2].clone(), text(field)]);
                let known = self
                    .call("isfield", &[a[2].clone(), text("parameters")])
                    .is_ok_and(|v| v == Expr::Bool(true) || v == Expr::one());
                let params = match known {
                    true => Some(get(self, "parameters")?),
                    false => None,
                };
                (
                    get(self, "x").map_err(|_| usage.clone())?,
                    get(self, "y").map_err(|_| usage.clone())?,
                    params,
                )
            }
            4 => (a[2].clone(), a[3].clone(), None),
            _ => (a[2].clone(), a[3].clone(), Some(a[4].clone())),
        };
        let work = numeric::bits_for(numeric::digits()) + 24;
        let prec = work + 64;
        let marks = mesh
            .as_list()
            .filter(|m| m.len() >= 2)
            .ok_or(usage.clone())?;
        let places: Vec<Float> = marks
            .iter()
            .map(|m| real(m, prec, name))
            .collect::<Result<_, _>>()?;
        let rising = places[1] > places[0];
        if places
            .windows(2)
            .any(|w| w[0] == w[1] || (w[1] > w[0]) != rising)
        {
            return Err(format!(
                "{name}: the mesh goes one way, without equal points"
            ));
        }
        let params: Vec<Complex> = match &known {
            Some(p) => list_of(p)
                .iter()
                .map(|e| number(e, prec, name))
                .collect::<Result<_, _>>()?,
            None => Vec::new(),
        };
        // the guess: numbers, a function of x, or a row for every point of the mesh
        let function = self.function_like(&guess);
        let per_point = guess
            .as_list()
            .is_some_and(|g| g.len() == marks.len() && g.iter().all(|r| r.as_list().is_some()));
        let guess_at: Guess = if function {
            let shape = self.many(&guess, 1, name)?;
            Box::new(move |_, x| {
                shape
                    .at(&[Complex::with_val(prec, x)], prec)
                    .ok_or_else(|| "the guess has no value at a point of the mesh".to_string())
            })
        } else if per_point {
            let rows: Vec<Vec<Complex>> = list_of(&guess)
                .iter()
                .map(|r| list_of(r).iter().map(|e| number(e, prec, name)).collect())
                .collect::<Result<_, _>>()?;
            Box::new(move |k, _| Ok(rows[k].clone()))
        } else {
            let same: Vec<Complex> = list_of(&guess)
                .iter()
                .map(|e| number(e, prec, name))
                .collect::<Result<_, _>>()?;
            Box::new(move |_, _| Ok(same.clone()))
        };
        // points to shoot from: those of the mesh, with points between them while they are few
        let parts = (8usize.div_ceil(places.len() - 1)).max(1);
        let mut nodes: Vec<Float> = Vec::new();
        let mut starts: Vec<Vec<Complex>> = Vec::new();
        for (k, pair) in places.windows(2).enumerate() {
            let (first, second) = (guess_at(k, &pair[0])?, guess_at(k + 1, &pair[1])?);
            for part in 0..parts {
                let share = Float::with_val(prec, part as u32) / parts as u32;
                nodes.push(Float::with_val(
                    prec,
                    &pair[0] + Float::with_val(prec, &pair[1] - &pair[0]) * &share,
                ));
                starts.push(
                    first
                        .iter()
                        .zip(&second)
                        .map(|(p, q)| {
                            Complex::with_val(prec, p + Complex::with_val(prec, q - p) * &share)
                        })
                        .collect(),
                );
            }
        }
        nodes.push(places[places.len() - 1].clone());
        starts.push(guess_at(places.len() - 1, &places[places.len() - 1])?);
        let (n, m, last) = (starts[0].len(), params.len(), nodes.len() - 1);
        if n == 0 || starts.iter().any(|s| s.len() != n) {
            return Err(usage);
        }
        let field = self.many(&a[0], 1 + n + m, name)?;
        let ends = self.many(&a[1], 2 * n + m, name)?;
        if field.items.len() != n || ends.items.len() != n + m {
            return Err(format!(
                "{name}: {n} slopes and {} conditions at the ends are needed, {} and {} are given",
                n + m,
                field.items.len(),
                ends.items.len()
            ));
        }
        // from a point to the next with the unknowns of both; none where the path breaks
        let shoot =
            |from: usize, y: &[Complex], p: &[Complex], bits: u32| -> Option<Vec<Complex>> {
                let wide = bits + 64;
                let mut slopes = |t: &Float, y: &[Complex]| -> Option<Vec<Complex>> {
                    let mut args = vec![Complex::with_val(wide, t)];
                    args.extend(y.iter().cloned());
                    args.extend(p.iter().cloned());
                    field.at(&args, wide)
                };
                integrate(
                    &mut slopes,
                    &nodes[from],
                    &nodes[from + 1],
                    y.to_vec(),
                    bits,
                )
                .ok()
            };
        let unknowns = n * (last + 1) + m;
        // the ends that the stretches reach from their starts
        let reach = |z: &[Complex], bits: u32| -> Option<Vec<Vec<Complex>>> {
            let p = &z[n * (last + 1)..];
            (0..last)
                .map(|k| shoot(k, &z[k * n..(k + 1) * n], p, bits))
                .collect()
        };
        let at_ends = |z: &[Complex]| -> Option<Vec<Complex>> {
            let mut args: Vec<Complex> = z[..n].to_vec();
            args.extend(z[last * n..].iter().cloned());
            ends.at(&args, prec)
        };
        // what is not zero yet: the gaps between the stretches, then the conditions
        let misfit = |z: &[Complex], reached: &[Vec<Complex>]| -> Option<Vec<Complex>> {
            let mut out = Vec::with_capacity(unknowns);
            for (k, r) in reached.iter().enumerate() {
                for (v, s) in r.iter().zip(&z[(k + 1) * n..]) {
                    out.push(Complex::with_val(prec, v - s));
                }
            }
            out.extend(at_ends(z)?);
            Some(out)
        };
        let mut z: Vec<Complex> = starts.into_iter().flatten().collect();
        z.extend(params);
        let fails = || format!("{name} does not settle: another guess or a finer mesh may help");
        let no_values = || format!("{name}: the equations have no values along the guess");
        for bits in [40, work] {
            let small = Float::with_val(prec, 1) >> (bits - 6);
            let nudge = Float::with_val(prec, 1) >> (bits / 2 + 4);
            let mut reached = reach(&z, bits).ok_or_else(no_values)?;
            let mut left = misfit(&z, &reached).ok_or_else(no_values)?;
            let mut settled = false;
            for _ in 0..60 {
                let (off, scale) = (largest(&left, prec), largest(&z, prec) + 1u32);
                if off <= Float::with_val(prec, &scale * &small) {
                    settled = true;
                    break;
                }
                // the slopes of the misfit: a stretch answers to its own start and to the
                // parameters, the conditions to both ends and to the parameters
                let mut slopes: CMat = vec![vec![Complex::new(prec); unknowns]; unknowns];
                for k in 0..last {
                    for j in 0..n {
                        slopes[k * n + j][(k + 1) * n + j] = Complex::with_val(prec, -1);
                    }
                }
                for at in 0..unknowns {
                    let step = Float::with_val(prec, (size(&z[at]) + 1u32) * &nudge);
                    let mut moved = z.clone();
                    moved[at] += &step;
                    let is_param = at >= n * (last + 1);
                    let stretches = match is_param {
                        true => 0..last,
                        false => (at / n).min(last)..(at / n + 1).min(last),
                    };
                    for k in stretches {
                        let there = shoot(
                            k,
                            &moved[k * n..(k + 1) * n],
                            &moved[n * (last + 1)..],
                            bits,
                        )
                        .ok_or_else(fails)?;
                        for (j, (new, old)) in there.iter().zip(&reached[k]).enumerate() {
                            slopes[k * n + j][at] = Complex::with_val(prec, new - old) / &step;
                        }
                    }
                    if is_param || at < n || at >= last * n {
                        let there = at_ends(&moved).ok_or_else(fails)?;
                        for (j, (new, old)) in there.iter().zip(&left[last * n..]).enumerate() {
                            slopes[last * n + j][at] = Complex::with_val(prec, new - old) / &step;
                        }
                    }
                }
                let right: CMat = left
                    .iter()
                    .map(|r| vec![Complex::with_val(prec, -r)])
                    .collect();
                let change = solved(&slopes, &right).ok_or_else(|| {
                    format!("{name}: the conditions do not fix the solution near the guess")
                })?;
                // the whole step of Newton, or as much of it as makes the misfit smaller
                let mut share = Float::with_val(prec, 1);
                let mut better = None;
                for _ in 0..30 {
                    let trial: Vec<Complex> = z
                        .iter()
                        .zip(&change)
                        .map(|(v, d)| {
                            Complex::with_val(prec, v + Complex::with_val(prec, &d[0] * &share))
                        })
                        .collect();
                    if let Some(ends) = reach(&trial, bits)
                        && let Some(rest) = misfit(&trial, &ends)
                        && largest(&rest, prec) < off
                    {
                        better = Some((trial, ends, rest));
                        break;
                    }
                    share /= 2u32;
                }
                let Some((trial, ends, rest)) = better else {
                    // no smaller misfit: what is left is the rounding of the paths
                    settled = off <= Float::with_val(prec, &scale * &small) * 1024u32;
                    break;
                };
                (z, reached, left) = (trial, ends, rest);
            }
            if !settled {
                return Err(fails());
            }
        }
        // the rows at the points of the mesh: every `parts` nodes
        let values: Vec<Vec<Complex>> = (0..marks.len())
            .map(|k| z[k * parts * n..(k * parts + 1) * n].to_vec())
            .collect();
        let rows = rows_of(marks, &values);
        Ok(match m {
            0 => rows,
            _ => Expr::list(vec![
                rows,
                Expr::list(from_complex_list(&z[n * (last + 1)..])),
            ]),
        })
    }

    /// `deval(rows, x)`: the solution between the rows by a spline; `deval(rows, x, f)`: by
    /// the equation itself from the row before, to all digits.
    fn deval(&mut self, a: &[Expr]) -> Res {
        let name = "deval";
        let usage = "deval(rows, x[, f]): the rows [t, y, ...] that ode45, bvp4c or dde23 give, and the places";
        let rows = crate::matrix::rows(&a[0])
            .filter(|r| r.len() >= 2 && r[0].len() >= 2)
            .ok_or(usage)?;
        let places = list_of(&a[1]);
        let n = rows[0].len() - 1;
        let shape = |mut found: Vec<Expr>| match n {
            1 => found.swap_remove(0),
            _ => Expr::list(found),
        };
        let mut out = Vec::with_capacity(places.len());
        if a.len() == 2 {
            let times = Expr::list(rows.iter().map(|r| r[0].clone()).collect());
            for x in &places {
                let mut found = Vec::with_capacity(n);
                for k in 1..=n {
                    let values = Expr::list(rows.iter().map(|r| r[k].clone()).collect());
                    found.push(self.call("spline", &[times.clone(), values, x.clone()])?);
                }
                out.push(shape(found));
            }
        } else {
            let work = numeric::bits_for(numeric::digits()) + 24;
            let prec = work + 64;
            let field = self.many(&a[2], 1 + n, name)?;
            if field.items.len() != n {
                return Err(usage.into());
            }
            let times: Vec<Float> = rows
                .iter()
                .map(|r| real(&r[0], prec, name))
                .collect::<Result<_, _>>()?;
            let rising = times[times.len() - 1] > times[0];
            let mut slopes = |t: &Float, y: &[Complex]| -> Option<Vec<Complex>> {
                let mut args = vec![Complex::with_val(prec, t)];
                args.extend(y.iter().cloned());
                field.at(&args, prec)
            };
            for x in &places {
                let at = real(x, prec, name)?;
                // the last row that the path passes before the place
                let from = times
                    .iter()
                    .rposition(|t| if rising { *t <= at } else { *t >= at })
                    .unwrap_or(0);
                let start: Vec<Complex> = rows[from][1..]
                    .iter()
                    .map(|e| number(e, prec, name))
                    .collect::<Result<_, _>>()?;
                let y = integrate(&mut slopes, &times[from], &at, start, work)?;
                out.push(shape(from_complex_list(&y)));
            }
        }
        Ok(match a[1].as_list() {
            Some(_) => Expr::list(out),
            None => out.swap_remove(0),
        })
    }

    /// `dde23(f, lags, history, span)`: an equation whose slope looks back by fixed lags,
    /// solved stretch by stretch between the points where the past breaks.
    fn dde(&mut self, a: &[Expr]) -> Res {
        let name = "dde23";
        let usage = "dde23(@(t, y, z) slope, [lag], history, [t0, t1]): z is y at t - lag; several unknowns and lags: @(t, x, y, x1, y1, x2, y2) [slopes] with x1, y1 at the first lag and x2, y2 at the second";
        let work = numeric::bits_for(numeric::digits()) + 24;
        let prec = work + 64;
        let lags: Vec<Float> = list_of(&a[1])
            .iter()
            .map(|e| real(e, prec, name))
            .collect::<Result<_, _>>()?;
        if lags.is_empty() || !lags.iter().all(positive) {
            return Err("dde23: the lags are numbers above zero".into());
        }
        let span = a[3].as_list().filter(|s| s.len() >= 2).ok_or(usage)?;
        let times: Vec<Expr> = if span.len() == 2 {
            (0..=10)
                .map(|k| {
                    crate::arrays::tidy(crate::canon::add(vec![
                        span[0].clone(),
                        crate::canon::mul(vec![
                            Expr::rat(k, 10),
                            crate::canon::sub(span[1].clone(), span[0].clone()),
                        ]),
                    ]))
                })
                .collect()
        } else {
            span.to_vec()
        };
        let marks: Vec<Float> = times
            .iter()
            .map(|t| real(t, prec, name))
            .collect::<Result<_, _>>()?;
        if marks.windows(2).any(|w| w[1] <= w[0]) {
            return Err("dde23: the times go forward".into());
        }
        let (start, end) = (marks[0].clone(), marks[marks.len() - 1].clone());
        // the past: numbers, or a function of the time
        let moving = self.function_like(&a[2]) || !a[2].free_syms().is_empty();
        let past: Many = match moving {
            true => self.many(&a[2], 1, name)?,
            false => Many::new(&[], &a[2]),
        };
        let before =
            |t: &Float| -> Option<Vec<Complex>> { past.at(&[Complex::with_val(prec, t)], prec) };
        let first = before(&start).ok_or("dde23: the history has no value at the start")?;
        let n = first.len();
        let field = self.many(&a[0], 1 + n + n * lags.len(), name)?;
        if field.items.len() != n {
            return Err(usage.into());
        }
        // where a lag carries a break of the past forward
        let near = Float::with_val(prec, &end - &start) >> (work / 2);
        let mut breaks = vec![start.clone()];
        let mut at = 0;
        while at < breaks.len() {
            for lag in &lags {
                let next = Float::with_val(prec, &breaks[at] + lag);
                let known = breaks
                    .iter()
                    .any(|b| Float::with_val(prec, b - &next).abs() <= near);
                if next < end && !known {
                    breaks.push(next);
                }
            }
            at += 1;
            if breaks.len() > 400 {
                return Err(
                    "dde23: the lags are too small beside the span (more than 400 stretches)"
                        .into(),
                );
            }
        }
        breaks.push(end.clone());
        breaks.sort_by(|p, q| p.partial_cmp(q).unwrap_or(std::cmp::Ordering::Equal));
        let mut pieces: Vec<Piece> = Vec::new();
        let mut y = first.clone();
        let small = Float::with_val(prec, 1) >> (work - 12);
        for pair in breaks.windows(2) {
            let found = {
                let looked = |t: &Float| -> Option<Vec<Complex>> {
                    if *t <= start {
                        return before(t);
                    }
                    let piece = pieces.iter().rev().find(|p| *t >= p.from)?;
                    Some(piece.at(t, prec))
                };
                let mut slopes = |t: &Float, y: &[Complex]| -> Option<Vec<Complex>> {
                    let mut args = vec![Complex::with_val(prec, t)];
                    args.extend(y.iter().cloned());
                    for lag in &lags {
                        args.extend(looked(&Float::with_val(prec, t - lag))?);
                    }
                    field.at(&args, prec)
                };
                // the points of Chebyshev, twice as many until the polynomial through them
                // gives the path between them
                let mut count = 8usize;
                let mut piece = Piece::new(&pair[0], &pair[1], count, prec);
                piece.values[0] = y.clone();
                for j in 1..=count {
                    let value = integrate_by(
                        &mut slopes,
                        &piece.points[j - 1],
                        &piece.points[j],
                        piece.values[j - 1].clone(),
                        work,
                        1,
                    )?;
                    piece.values[j] = value;
                }
                loop {
                    // twice the points: every old point stays, a new one comes between two
                    let mut finer = Piece::new(&pair[0], &pair[1], 2 * count, prec);
                    for (j, values) in piece.values.iter().enumerate() {
                        finer.values[2 * j] = values.clone();
                    }
                    let mut worst = Float::new(prec);
                    let mut scale = Float::with_val(prec, 1);
                    for j in (1..2 * count).step_by(2) {
                        let value = integrate_by(
                            &mut slopes,
                            &finer.points[j - 1],
                            &finer.points[j],
                            finer.values[j - 1].clone(),
                            work,
                            1,
                        )?;
                        let guessed = piece.at(&finer.points[j], prec);
                        for (v, g) in value.iter().zip(&guessed) {
                            worst = worst.max(&size(&Complex::with_val(prec, v - g)));
                            scale = scale.max(&size(v));
                        }
                        finer.values[j] = value;
                    }
                    piece = finer;
                    count *= 2;
                    if worst <= Float::with_val(prec, &scale * &small) {
                        break;
                    }
                    if count > 512 {
                        return Err(
                            "dde23: the solution turns too sharply for the points of a stretch"
                                .into(),
                        );
                    }
                }
                piece
            };
            y = found.values[found.values.len() - 1].clone();
            pieces.push(found);
        }
        let values: Vec<Vec<Complex>> = marks
            .iter()
            .map(|mark| match pieces.iter().rev().find(|p| *mark >= p.from) {
                Some(piece) if *mark > start => piece.at(mark, prec),
                _ => first.clone(),
            })
            .collect();
        Ok(rows_of(&times, &values))
    }

    pub(crate) fn solver(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        Some(match (name, a.len()) {
            ("bvp4c", 3..=5) => self.bvp(name, a),
            ("bvpinit", 2 | 3) => {
                let mut fields = vec![("x", a[0].clone()), ("y", a[1].clone())];
                if let Some(p) = a.get(2) {
                    fields.push(("parameters", p.clone()));
                }
                Ok(crate::records::record(fields))
            }
            ("deval", 2 | 3) => self.deval(a),
            ("dde23", 4) => self.dde(a),
            _ => return self.solver_more(name, a),
        })
    }
}

// ---- equations that are stiff: heat in one dimension of space ----

/// The right sides F and the diagonal of M of a system `M*z' = F`, at the unknowns and the
/// time; none where they have no values.
type Stiff<'a> = &'a mut dyn FnMut(&[Complex], &Float) -> Option<(Vec<Complex>, Vec<Complex>)>;

/// The solution of `A*x = b` for a matrix with entries near its diagonal only (so many to
/// each side), rows exchanged for the largest entry; none when A has no inverse.
fn banded(mut a: CMat, mut b: Vec<Complex>, band: usize) -> Option<Vec<Complex>> {
    let n = b.len();
    for k in 0..n {
        let rows = (k + band + 1).min(n);
        let pivot = (k..rows).max_by(|i, j| {
            size(&a[*i][k])
                .partial_cmp(&size(&a[*j][k]))
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;
        if a[pivot][k].is_zero() {
            return None;
        }
        a.swap(k, pivot);
        b.swap(k, pivot);
        let columns = (k + 2 * band + 1).min(n);
        let (head, tail) = a.split_at_mut(k + 1);
        let (lead, lead_b) = (&head[k], b[k].clone());
        for (row, right) in tail[..rows - k - 1].iter_mut().zip(b[k + 1..].iter_mut()) {
            if row[k].is_zero() {
                continue;
            }
            let factor = Complex::with_val(row[k].prec(), &row[k] / &lead[k]);
            for (x, l) in row[k..columns].iter_mut().zip(&lead[k..columns]) {
                *x -= Complex::with_val(l.prec(), &factor * l);
            }
            *right -= Complex::with_val(lead_b.prec(), &factor * &lead_b);
        }
    }
    let mut x = b;
    for k in (0..n).rev() {
        let columns = (k + 2 * band + 1).min(n);
        let mut sum = x[k].clone();
        for j in k + 1..columns {
            sum -= Complex::with_val(sum.prec(), &a[k][j] * &x[j]);
        }
        x[k] = sum / &a[k][k];
    }
    Some(x)
}

/// The slopes of F by the unknowns (unknowns that lie more than the band apart are moved
/// together) and by the time.
fn stiff_slopes(
    f: Stiff,
    z: &[Complex],
    t: &Float,
    band: usize,
    prec: u32,
) -> Option<(CMat, Vec<Complex>)> {
    let n = z.len();
    let (base, _) = f(z, t)?;
    let nudge = Float::with_val(prec, 1) >> (prec / 2);
    let mut slopes: CMat = vec![vec![Complex::new(prec); n]; n];
    let groups = (2 * band + 1).min(n);
    for group in 0..groups {
        let mut moved = z.to_vec();
        let steps: Vec<Float> = (group..n)
            .step_by(groups)
            .map(|i| Float::with_val(prec, (size(&z[i]) + 1u32) * &nudge))
            .collect();
        for (i, step) in (group..n).step_by(groups).zip(&steps) {
            moved[i] += step;
        }
        let (there, _) = f(&moved, t)?;
        for (i, step) in (group..n).step_by(groups).zip(&steps) {
            for r in i.saturating_sub(band)..(i + band + 1).min(n) {
                slopes[r][i] = Complex::with_val(prec, &there[r] - &base[r]) / step;
            }
        }
    }
    let step = Float::with_val(prec, (Float::with_val(prec, t.abs_ref()) + 1u32) * &nudge);
    let (there, _) = f(z, &Float::with_val(prec, t + &step))?;
    let by_time = there
        .iter()
        .zip(&base)
        .map(|(a, b)| Complex::with_val(prec, a - b) / &step)
        .collect();
    Some((slopes, by_time))
}

/// The solution of `M*z' = F(z, t)` at `t1` from `z(t0)`: steps of Euler's rule with the
/// slopes of F taken into the left side, 1, 2, 3, ... substeps, extrapolated to step 0.
/// Stiff equations and equations without slopes (zeros in M) go through. The step and the
/// number of substeps are chosen for the least work on the way.
fn stiff(
    f: Stiff,
    band: usize,
    t0: &Float,
    t1: &Float,
    z0: Vec<Complex>,
    bits: u32,
    first: &mut Float,
) -> Result<Vec<Complex>, String> {
    const COLUMNS: usize = 16;
    let prec = bits + 64;
    let (mut t, mut z) = (Float::with_val(prec, t0), z0);
    let span = Float::with_val(prec, t1 - t0);
    if span.is_zero() {
        return Ok(z);
    }
    let mut h = match first.is_zero() {
        true => Float::with_val(prec, &span / 64u32),
        false => first.clone(),
    };
    let none = || "pdepe: the equations have no values at a point of the mesh".to_string();
    let allowed = Float::with_val(64, 1) >> bits;
    // what a step costs: the slopes, then 1 + 2 + ... + k substeps
    let cost = |rows: usize| (2 * band + 2 + rows * (rows + 1) / 2) as u32;
    let mut work = 0u64;
    loop {
        let left = Float::with_val(prec, t1 - &t);
        if left.is_zero() || left.is_sign_negative() != span.is_sign_negative() {
            *first = h;
            return Ok(z);
        }
        let wished = h.clone();
        let mut last = false;
        if Float::with_val(prec, h.abs_ref()) >= Float::with_val(prec, left.abs_ref()) {
            h = left;
            last = true;
        }
        let (slopes, by_time) = stiff_slopes(f, &z, &t, band, prec).ok_or_else(none)?;
        let mut table: Vec<Vec<Vec<Complex>>> = Vec::new();
        let mut accepted: Option<Vec<Complex>> = None;
        // the step that every number of rows would have liked, as a share of this one
        let mut liked: Vec<Float> = Vec::new();
        'columns: for k in 0..COLUMNS {
            let count = k as u32 + 1;
            let step = Float::with_val(prec, &h / count);
            let mut now = z.clone();
            for part in 0..count {
                work += 1;
                let at = Float::with_val(prec, &t + Float::with_val(prec, &step * part));
                let Some((right, weights)) = f(&now, &at) else {
                    break 'columns;
                };
                // (M - h*J) * change = h*F + h^2 * dF/dt
                let matrix: CMat = slopes
                    .iter()
                    .zip(&weights)
                    .enumerate()
                    .map(|(i, (row, w))| {
                        row.iter()
                            .enumerate()
                            .map(|(j, s)| {
                                let part = -Complex::with_val(prec, s * &step);
                                match i == j {
                                    true => part + w,
                                    false => part,
                                }
                            })
                            .collect()
                    })
                    .collect();
                let side: Vec<Complex> = right
                    .iter()
                    .zip(&by_time)
                    .map(|(r, d)| {
                        (Complex::with_val(prec, r) + Complex::with_val(prec, d * &step)) * &step
                    })
                    .collect();
                let Some(change) = banded(matrix, side, band) else {
                    break 'columns;
                };
                for (v, c) in now.iter_mut().zip(&change) {
                    *v += c;
                }
            }
            let mut line = vec![now];
            for j in 0..k {
                // T(k, j+1) = T(k, j) + (T(k, j) - T(k-1, j))/(n_k/n_(k-j-1) - 1)
                let weight = Float::with_val(prec, count) / (k - j) as u32 - 1u32;
                let next: Vec<Complex> = line[j]
                    .iter()
                    .zip(&table[k - 1][j])
                    .map(|(a, b)| {
                        Complex::with_val(prec, a + Complex::with_val(prec, a - b) / &weight)
                    })
                    .collect();
                line.push(next);
            }
            if k >= 1 {
                let (best, lower) = (&line[k], &line[k - 1]);
                let scale = largest(best, prec) + 1u32;
                let off = best.iter().zip(lower).fold(Float::new(prec), |s, (a, b)| {
                    s.max(&size(&Complex::with_val(prec, a - b)))
                });
                let off =
                    Float::with_val(64, &off / &scale).max(&(Float::with_val(64, 1) >> 400u32));
                // the error falls as the step to the power of the rows
                let share = (Float::with_val(64, &allowed * 0.65f64) / &off).root(count) * 0.94f64;
                liked.push(share.clamp(&Float::with_val(64, 0.02), &Float::with_val(64, 8)));
                if off <= allowed && k >= 2 {
                    accepted = Some(best.clone());
                    table.push(line);
                    break;
                }
            } else {
                liked.push(Float::with_val(64, 1));
            }
            table.push(line);
        }
        if work > 4_000_000 {
            return Err("pdepe: too many steps of time".into());
        }
        // the rows whose liked step costs least for the way it covers
        let rows = liked.len();
        let per_way = |k: usize| Float::with_val(64, cost(k + 1)) / &liked[k];
        let best = (2..rows).min_by(|i, j| {
            per_way(*i)
                .partial_cmp(&per_way(*j))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        match accepted {
            Some(found) => {
                t += &h;
                z = found;
                let mut next = match best {
                    Some(k) => {
                        let mut share = liked[k].clone();
                        if k + 1 == rows && rows < COLUMNS - 1 {
                            // the last row was the best: one row more may be better still
                            share *= Float::with_val(64, cost(rows + 1)) / cost(rows);
                        }
                        Float::with_val(prec, &h * &share)
                    }
                    None => h.clone(),
                };
                if last {
                    // a step cut short at a time that was asked for says little
                    if Float::with_val(prec, next.abs_ref())
                        < Float::with_val(prec, wished.abs_ref())
                    {
                        next = wished;
                    }
                    *first = next;
                    return Ok(z);
                }
                h = next;
            }
            None => {
                let share = match rows {
                    0 | 1 => Float::with_val(64, 0.1),
                    _ => liked[rows - 1].clone().min(&Float::with_val(64, 0.5)),
                };
                h *= share;
                if Float::with_val(prec, h.abs_ref())
                    < Float::with_val(prec, span.abs_ref()) >> 70u32
                {
                    return Err("pdepe: the steps of time do not get on (a start that the conditions at the ends contradict?)".into());
                }
            }
        }
    }
}

// ---- the least value of a function, sought without its slopes ----

/// A function to make least, the box it stays in and the rows `A*x <= b` it keeps.
struct Target {
    params: Vec<String>,
    body: Expr,
    lower: Vec<Option<Float>>,
    upper: Vec<Option<Float>>,
    rows: Vec<(Vec<Float>, Float)>,
}

impl Target {
    fn allowed(&self, x: &[Float]) -> bool {
        let inside = x
            .iter()
            .zip(&self.lower)
            .zip(&self.upper)
            .all(|((v, lo), hi)| {
                lo.as_ref().is_none_or(|l| v >= l) && hi.as_ref().is_none_or(|h| v <= h)
            });
        inside
            && self.rows.iter().all(|(row, right)| {
                let prec = x[0].prec();
                let sum = row.iter().zip(x).fold(Float::new(prec), |s, (c, v)| {
                    s + Float::with_val(prec, c * v)
                });
                sum <= *right
            })
    }

    /// The function with numbers of so many bits; none outside the box and where it has
    /// no real value.
    fn at(&self, f: &crate::mfun::Numeric, x: &[Float]) -> Option<Float> {
        match self.allowed(x) {
            true => f.at(x).ok(),
            false => None,
        }
    }

    fn width(&self, k: usize, prec: u32) -> (Float, Float) {
        let lo = self.lower[k]
            .clone()
            .unwrap_or_else(|| Float::with_val(prec, -10));
        let hi = self.upper[k]
            .clone()
            .unwrap_or_else(|| Float::with_val(prec, 10));
        (Float::with_val(prec, lo), Float::with_val(prec, hi))
    }

    /// The search of Hooke and Jeeves from a point: steps along the axes, a stride along
    /// the way that helped, and half the step when nothing helps, down to the smallest step.
    fn pattern(
        &self,
        f: &crate::mfun::Numeric,
        start: Vec<Float>,
        mut step: Float,
        least: &Float,
        budget: &mut u64,
    ) -> Option<(Vec<Float>, Float)> {
        let prec = start[0].prec();
        let mut base = start;
        let mut low = self.at(f, &base)?;
        let mut round = 0u64;
        let explore = |from: &[Float],
                       value: &Float,
                       step: &Float,
                       budget: &mut u64|
         -> (Vec<Float>, Float) {
            let (mut x, mut fx) = (from.to_vec(), value.clone());
            for k in 0..x.len() {
                for side in [1i32, -1] {
                    let kept = x[k].clone();
                    x[k] = Float::with_val(prec, &kept + Float::with_val(prec, step * side));
                    *budget = budget.saturating_sub(1);
                    match self.at(f, &x) {
                        Some(v) if v < fx => {
                            fx = v;
                            break;
                        }
                        _ => x[k] = kept,
                    }
                }
            }
            (x, fx)
        };
        while *budget > 0 {
            let (mut x, mut fx) = explore(&base, &low, &step, budget);
            if fx < low {
                // strides: as far again from the new point, then a look around
                loop {
                    let stride: Vec<Float> = x
                        .iter()
                        .zip(&base)
                        .map(|(new, old)| Float::with_val(prec, new * 2u32) - old)
                        .collect();
                    (base, low) = (x.clone(), fx.clone());
                    *budget = budget.saturating_sub(1);
                    let Some(there) = self.at(f, &stride) else {
                        break;
                    };
                    let (far, f_far) = explore(&stride, &there, &step, budget);
                    if f_far < low && *budget > 0 {
                        (x, fx) = (far, f_far);
                    } else {
                        break;
                    }
                }
            } else {
                // no axis helps: slanted ways, which a ridge with a kink may need
                let mut helped = false;
                for way in ways(base.len(), &mut round) {
                    let x: Vec<Float> = base
                        .iter()
                        .zip(&way)
                        .map(|(v, w)| Float::with_val(prec, v + Float::with_val(prec, &step * *w)))
                        .collect();
                    *budget = budget.saturating_sub(1);
                    if let Some(v) = self.at(f, &x)
                        && v < low
                    {
                        (base, low, helped) = (x, v, true);
                        break;
                    }
                }
                if helped {
                    continue;
                }
                step /= 2u32;
                let scale = base.iter().fold(Float::with_val(prec, 1), |s, v| {
                    s.max(&Float::with_val(prec, v.abs_ref()))
                });
                if step < Float::with_val(prec, &scale * least) {
                    break;
                }
            }
        }
        Some((base, low))
    }

    /// The start moved into the box and under the rows, by steps onto whatever it breaks.
    fn inside(&self, x: &mut [Float]) -> bool {
        let prec = x[0].prec();
        for _ in 0..20_000 {
            for ((v, lo), hi) in x.iter_mut().zip(&self.lower).zip(&self.upper) {
                if let Some(l) = lo.as_ref().filter(|l| *v < **l) {
                    *v = Float::with_val(prec, l);
                }
                if let Some(h) = hi.as_ref().filter(|h| *v > **h) {
                    *v = Float::with_val(prec, h);
                }
            }
            let mut broken = false;
            for (row, right) in &self.rows {
                let sum = row
                    .iter()
                    .zip(x.iter())
                    .fold(Float::new(prec), |s, (c, v)| {
                        s + Float::with_val(prec, c * v)
                    });
                let long = row.iter().fold(Float::new(prec), |s, c| {
                    s + Float::with_val(prec, c.square_ref())
                });
                if sum > *right && !long.is_zero() {
                    broken = true;
                    // a little past the edge, so that rounding leaves the point inside
                    let over = (Float::with_val(prec, &sum - right) * 1.000001f64
                        + Float::with_val(prec, right.abs_ref()) * 1e-12f64
                        + 1e-12f64)
                        / &long;
                    for (v, c) in x.iter_mut().zip(row) {
                        *v -= Float::with_val(prec, c * &over);
                    }
                }
            }
            if !broken && self.allowed(x) {
                return true;
            }
        }
        false
    }
}

/// Ways that are no axis: for few unknowns every mix of steps forward, back and none, for
/// more of them every pair of axes; then ways drawn evenly by the numbers of Halton, new
/// ones every round.
fn ways(n: usize, round: &mut u64) -> Vec<Vec<f64>> {
    const PRIMES: [u64; 12] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];
    let mut out: Vec<Vec<f64>> = Vec::new();
    if n == 1 {
        return out;
    }
    if n <= 4 {
        for code in 0..3usize.pow(n as u32) {
            let way: Vec<f64> = (0..n)
                .map(|k| [0.0, 1.0, -1.0][code / 3usize.pow(k as u32) % 3])
                .collect();
            if way.iter().filter(|w| **w != 0.0).count() >= 2 {
                out.push(way);
            }
        }
    } else {
        for i in 0..n {
            for j in i + 1..n {
                for (a, b) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                    let mut way = vec![0.0; n];
                    (way[i], way[j]) = (a, b);
                    out.push(way);
                }
            }
        }
    }
    for _ in 0..n {
        *round += 1;
        let way: Vec<f64> = (0..n)
            .map(|k| {
                // the digits of the round, mirrored at the point, in the base of a prime
                let base = PRIMES[k % PRIMES.len()] + 2 * (k / PRIMES.len()) as u64;
                let (mut index, mut share, mut value) = (*round, 1.0, 0.0);
                while index > 0 {
                    share /= base as f64;
                    value += share * (index % base) as f64;
                    index /= base;
                }
                2.0 * value - 1.0
            })
            .collect();
        out.push(way.iter().map(|w| -w).collect());
        out.push(way);
    }
    out
}

impl Session {
    /// The function, its start or the count of its unknowns, and the box and rows that
    /// hold it: `(f, x0)`, `(f, x0, lb, ub)` or `(f, x0, A, b, Aeq, beq, lb, ub)`.
    fn target(
        &mut self,
        name: &str,
        a: &[Expr],
        counted: bool,
    ) -> Result<(Target, Vec<Expr>), String> {
        let usage = match counted {
            true => format!(
                "{name}(f, number of unknowns[, lower bounds, upper bounds]) or {name}(f, number, A, b, [], [], lower, upper)"
            ),
            false => format!(
                "{name}(f, start[, lower bounds, upper bounds]) or {name}(f, start, A, b, [], [], lower, upper)"
            ),
        };
        let start: Vec<Expr> = match counted {
            true => vec![
                Expr::zero();
                crate::arrays::whole(&a[1], "the number of unknowns", 1, 1000)? as usize
            ],
            false => list_of(&a[1]),
        };
        let n = start.len();
        if n == 0 {
            return Err(usage);
        }
        let prec = 2 * crate::mfun::bits();
        let empty = |k: usize| {
            a.get(k)
                .is_none_or(|e| e.as_list().is_some_and(|l| l.is_empty()))
        };
        let (rows_at, bounds_at) = match a.len() {
            2 => (None, None),
            4 => (None, Some(2)),
            8 => (Some(2), Some(6)),
            _ => return Err(usage),
        };
        let mut rows = Vec::new();
        if let Some(k) = rows_at {
            if !empty(k + 2) || !empty(k + 3) {
                return Err(format!(
                    "{name} takes rows A*x <= b and bounds; equations among the unknowns are for fmincon"
                ));
            }
            if !empty(k) {
                let matrix = match a[k].as_list() {
                    Some(flat) if flat.iter().all(|x| x.as_list().is_none()) => vec![flat.to_vec()],
                    _ => crate::matrix::rows(&a[k]).ok_or(usage.clone())?,
                };
                let right = list_of(&a[k + 1]);
                if matrix.len() != right.len() || matrix.iter().any(|r| r.len() != n) {
                    return Err(usage);
                }
                for (row, b) in matrix.iter().zip(&right) {
                    let row: Vec<Float> = row
                        .iter()
                        .map(|c| real(c, prec, name))
                        .collect::<Result<_, _>>()?;
                    rows.push((row, real(b, prec, name)?));
                }
            }
        }
        let bound = |k: Option<usize>| -> Result<Vec<Option<Float>>, String> {
            let Some(k) = k.filter(|k| !empty(*k)) else {
                return Ok(vec![None; n]);
            };
            let given = list_of(&a[k]);
            if given.len() != n {
                return Err(format!("{name}: a bound for every one of the {n} unknowns"));
            }
            given
                .iter()
                .map(|b| match b.is_inf() || b.is_neg_inf() {
                    true => Ok(None),
                    false => real(b, prec, name).map(Some),
                })
                .collect()
        };
        let lower = bound(bounds_at)?;
        let upper = bound(bounds_at.map(|k| k + 1))?;
        if lower
            .iter()
            .zip(&upper)
            .any(|(lo, hi)| matches!((lo, hi), (Some(lo), Some(hi)) if lo > hi))
        {
            return Err(format!("{name}: a lower bound lies above its upper bound"));
        }
        let (params, body) = self.function_of(&a[0], n, name)?;
        Ok((
            Target {
                params,
                body,
                lower,
                upper,
                rows,
            },
            start,
        ))
    }

    /// The least point found, polished by the search of Hooke and Jeeves to the digits of
    /// the session.
    fn polished(
        &self,
        name: &str,
        target: &Target,
        from: Vec<Float>,
        step: f64,
        single: bool,
    ) -> Res {
        let digits = numeric::digits();
        let prec = 2 * crate::mfun::bits();
        let quick = crate::mfun::Numeric::new(&target.params, &target.body, 64);
        let exact = crate::mfun::Numeric::new(&target.params, &target.body, prec);
        let none = || {
            format!(
                "{name}: the function has no real value at the start, or the start is outside the bounds"
            )
        };
        let mut budget = 400_000u64;
        let mut rough: Vec<Float> = from.iter().map(|v| Float::with_val(64, v)).collect();
        if !target.inside(&mut rough) {
            return Err(format!(
                "{name}: no point is inside the bounds and under the rows A*x <= b"
            ));
        }
        let coarse = Float::with_val(64, 1e-9);
        let (near, _) = target
            .pattern(
                &quick,
                rough,
                Float::with_val(64, step),
                &coarse,
                &mut budget,
            )
            .ok_or_else(none)?;
        let endless =
            || format!("{name}: the search does not settle: the function may have no least value");
        if budget == 0 {
            return Err(endless());
        }
        let fine = Float::with_val(prec, Float::i_pow_u(10, digits + 2)).recip();
        let near: Vec<Float> = near.iter().map(|v| Float::with_val(prec, v)).collect();
        let mut budget = 400_000u64;
        let (best, _) = target
            .pattern(
                &exact,
                near,
                Float::with_val(prec, 1e-7),
                &fine,
                &mut budget,
            )
            .ok_or_else(none)?;
        if budget == 0 {
            return Err(endless());
        }
        // what is less than the steps of the search can tell from nothing shows as 0
        let scale = best.iter().fold(Float::with_val(prec, 1), |s, v| {
            s.max(&Float::with_val(prec, v.abs_ref()))
        });
        let small = scale * Float::with_val(prec, Float::i_pow_u(10, digits)).recip();
        let shown = |v: &Float| match Float::with_val(prec, v.abs_ref()) <= small {
            true => Expr::zero(),
            false => crate::mfun::value(v),
        };
        Ok(match single {
            true => shown(&best[0]),
            false => Expr::list(best.iter().map(shown).collect()),
        })
    }

    /// The searches that draw numbers: a swarm, a walk that cools, generations, many starts.
    fn drawn(&mut self, name: &str, a: &[Expr]) -> Res {
        let counted = matches!(name, "particleswarm" | "ga");
        let (target, start) = self.target(name, a, counted)?;
        let n = start.len();
        let p = 64u32;
        let f = crate::mfun::Numeric::new(&target.params, &target.body, p);
        let mut draws = crate::random::Draws::new();
        let spans: Vec<(Float, Float)> = (0..n).map(|k| target.width(k, p)).collect();
        let anywhere = |draws: &mut crate::random::Draws| -> Vec<Float> {
            spans
                .iter()
                .map(|(lo, hi)| Float::with_val(p, lo + Float::with_val(p, hi - lo) * draws.unit()))
                .collect()
        };
        let held = |x: &mut Vec<Float>| {
            for (v, (lo, hi)) in x.iter_mut().zip(&spans) {
                if target.lower.iter().any(Option::is_some)
                    || target.upper.iter().any(Option::is_some)
                {
                    *v = v.clone().clamp(lo, hi);
                }
            }
        };
        let worth = |x: &[Float]| target.at(&f, x);
        let better = |a: &Option<Float>, b: &Option<Float>| match (a, b) {
            (Some(x), Some(y)) => x < y,
            (Some(_), None) => true,
            _ => false,
        };
        let given: Vec<Float> = match counted {
            true => Vec::new(),
            false => start
                .iter()
                .map(|e| real(e, p, name))
                .collect::<Result<_, _>>()?,
        };
        let (mut x, mut fx): (Vec<Float>, Option<Float>) = (Vec::new(), None);
        let mut heat = Float::with_val(p, 100);
        let mut best: (Vec<Float>, Option<Float>) = match counted {
            true => {
                let x = anywhere(&mut draws);
                let v = worth(&x);
                (x, v)
            }
            false => {
                let v = worth(&given);
                (given.clone(), v)
            }
        };
        match name {
            "particleswarm" => {
                let count = (10 * n).clamp(20, 100);
                let mut swarm: Vec<Vec<Float>> = (0..count).map(|_| anywhere(&mut draws)).collect();
                let mut speeds: Vec<Vec<Float>> = (0..count)
                    .map(|_| {
                        spans
                            .iter()
                            .map(|(lo, hi)| {
                                Float::with_val(p, hi - lo) * (draws.unit() * 2u32 - 1u32)
                            })
                            .collect()
                    })
                    .collect();
                let mut own: Vec<(Vec<Float>, Option<Float>)> =
                    swarm.iter().map(|x| (x.clone(), worth(x))).collect();
                for (x, v) in &own {
                    if better(v, &best.1) {
                        best = (x.clone(), v.clone());
                    }
                }
                let mut idle = 0;
                for _ in 0..200 * n {
                    let mut gained = false;
                    for k in 0..count {
                        for j in 0..n {
                            let toward_own = Float::with_val(p, &own[k].0[j] - &swarm[k][j])
                                * draws.unit()
                                * 1.49618f64;
                            let toward_best = Float::with_val(p, &best.0[j] - &swarm[k][j])
                                * draws.unit()
                                * 1.49618f64;
                            speeds[k][j] = Float::with_val(p, &speeds[k][j] * 0.7298f64)
                                + toward_own
                                + toward_best;
                            swarm[k][j] += &speeds[k][j];
                        }
                        held(&mut swarm[k]);
                        let v = worth(&swarm[k]);
                        if better(&v, &own[k].1) {
                            own[k] = (swarm[k].clone(), v.clone());
                            if better(&v, &best.1) {
                                best = (swarm[k].clone(), v);
                                gained = true;
                            }
                        }
                    }
                    idle = match gained {
                        true => 0,
                        false => idle + 1,
                    };
                    if idle >= 30 {
                        break;
                    }
                }
            }
            "simulannealbnd" => {
                // rounds of cooling, each from the best point so far and less hot than the last
                for (round, start) in (0..2400 * n as u32).map(|r| (r, r % 400 == 0)) {
                    if start {
                        (x, fx) = best.clone();
                        heat = Float::with_val(
                            p,
                            [100.0, 30.0, 10.0, 3.0, 1.0, 0.3][(round / 400) as usize % 6],
                        );
                    }
                    // a step as long as the heat, in a direction drawn evenly
                    let way: Vec<Float> = (0..n).map(|_| draws.normal()).collect();
                    let long = way
                        .iter()
                        .fold(Float::new(p), |s, v| s + Float::with_val(p, v.square_ref()))
                        .sqrt();
                    if long.is_zero() {
                        continue;
                    }
                    let mut next: Vec<Float> = x
                        .iter()
                        .zip(&way)
                        .map(|(v, w)| Float::with_val(p, v + Float::with_val(p, w / &long) * &heat))
                        .collect();
                    held(&mut next);
                    let value = worth(&next);
                    let take = match (&value, &fx) {
                        (Some(new), Some(old)) if new >= old => {
                            let rise = Float::with_val(p, new - old) / &heat;
                            draws.unit() < (rise.exp() + 1u32).recip()
                        }
                        (Some(_), _) => true,
                        (None, _) => false,
                    };
                    if take {
                        (x, fx) = (next, value);
                        if better(&fx, &best.1) {
                            best = (x.clone(), fx.clone());
                        }
                    }
                    heat *= 0.95f64;
                }
            }
            "ga" => {
                let count = if n <= 5 { 50 } else { 200 };
                let mut people: Vec<(Vec<Float>, Option<Float>)> = (0..count)
                    .map(|_| {
                        let x = anywhere(&mut draws);
                        let v = worth(&x);
                        (x, v)
                    })
                    .collect();
                let rounds = 100 * n;
                let mut idle = 0;
                for round in 0..rounds {
                    people.sort_by(|a, b| match (&a.1, &b.1) {
                        (Some(x), Some(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    });
                    if better(&people[0].1, &best.1) {
                        best = people[0].clone();
                        idle = 0;
                    } else {
                        idle += 1;
                        if idle >= 50 {
                            break;
                        }
                    }
                    // the best stay; the others come from two parents, each the better of two drawn
                    let elite = (count / 20).max(1);
                    let mut next: Vec<(Vec<Float>, Option<Float>)> = people[..elite].to_vec();
                    let shrink = 1.0 - round as f64 / rounds as f64;
                    while next.len() < count {
                        let parent = |draws: &mut crate::random::Draws| -> usize {
                            let (i, j) = (draws.below(count), draws.below(count));
                            i.min(j)
                        };
                        let (first, second) = (parent(&mut draws), parent(&mut draws));
                        let mut child: Vec<Float> = (0..n)
                            .map(|j| {
                                let share = draws.unit();
                                let (u, v) = (&people[first].0[j], &people[second].0[j]);
                                Float::with_val(p, u + Float::with_val(p, v - u) * share)
                            })
                            .collect();
                        if draws.unit() < 0.2 {
                            for (v, (lo, hi)) in child.iter_mut().zip(&spans) {
                                *v += Float::with_val(p, hi - lo) * draws.normal() * (0.1 * shrink);
                            }
                        }
                        held(&mut child);
                        let value = worth(&child);
                        next.push((child, value));
                    }
                    people = next;
                }
            }
            _ => {
                // many starts in the box, the search of Hooke and Jeeves from the best of them
                let mut starts: Vec<(Vec<Float>, Option<Float>)> = (0..200)
                    .map(|_| {
                        let x = anywhere(&mut draws);
                        let v = worth(&x);
                        (x, v)
                    })
                    .collect();
                starts.push(best.clone());
                starts.sort_by(|a, b| match (&a.1, &b.1) {
                    (Some(x), Some(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                });
                let coarse = Float::with_val(p, 1e-6);
                for (x, v) in starts.into_iter().take(12) {
                    if v.is_none() {
                        continue;
                    }
                    let mut budget = 20_000u64;
                    if let Some((found, low)) =
                        target.pattern(&f, x, Float::with_val(p, 0.5), &coarse, &mut budget)
                    {
                        let low = Some(low);
                        if better(&low, &best.1) {
                            best = (found, low);
                        }
                    }
                }
            }
        }
        if best.1.is_none() {
            return Err(format!(
                "{name}: the function has no real value at the points tried"
            ));
        }
        self.polished(
            name,
            &target,
            best.0,
            0.01,
            n == 1 && a[1].as_list().is_none(),
        )
    }

    /// `fminimax(f, start, ...)` and `fgoalattain(f, start, goals, weights, ...)`: the
    /// largest of several functions made least, by one unknown more that bounds them all.
    fn bounded_all(&mut self, name: &str, a: &[Expr]) -> Res {
        let usage = match name {
            "fminimax" => "fminimax(@(x, y) [f, g, ...], start[, A, b, Aeq, beq, lower, upper])",
            _ => {
                "fgoalattain(@(x, y) [f, g, ...], start, goals, weights[, A, b, Aeq, beq, lower, upper])"
            }
        };
        let start = list_of(&a[1]);
        let n = start.len();
        let (params, body) = self.function_of(&a[0], n, name)?;
        let parts = list_of(&body);
        let more = if name == "fminimax" { 2 } else { 4 };
        let (goals, weights) = match name {
            "fminimax" => (
                vec![Expr::zero(); parts.len()],
                vec![Expr::one(); parts.len()],
            ),
            _ => (list_of(&a[2]), list_of(&a[3])),
        };
        if n == 0
            || goals.len() != parts.len()
            || weights.len() != parts.len()
            || !matches!(a.len() - more, 0 | 2 | 4 | 6)
        {
            return Err(usage.into());
        }
        let bound = Expr::sym("_bound");
        let mut names: Vec<Expr> = params.iter().map(|p| Expr::sym(p)).collect();
        names.push(bound.clone());
        let under: Vec<Expr> = parts
            .iter()
            .zip(&goals)
            .zip(&weights)
            .map(|((f, goal), weight)| {
                crate::canon::sub(
                    crate::canon::sub(
                        f.clone(),
                        crate::canon::mul(vec![weight.clone(), bound.clone()]),
                    ),
                    goal.clone(),
                )
            })
            .collect();
        let rule = |body: Expr| Expr::call("rule", vec![Expr::list(names.clone()), body]);
        // the start of the bound: the largest of the functions at the start
        let prec = crate::mfun::bits();
        let at: Vec<Float> = start
            .iter()
            .map(|e| real(e, prec, name))
            .collect::<Result<_, _>>()?;
        let mut top: Option<Float> = None;
        for ((f, goal), weight) in parts.iter().zip(&goals).zip(&weights) {
            let value = crate::mfun::Numeric::new(&params, f, prec)
                .at(&at)
                .map_err(|e| format!("{name}: {e}"))?;
            let (goal, weight) = (real(goal, prec, name)?, real(weight, prec, name)?);
            if weight.is_zero() {
                continue;
            }
            let need = Float::with_val(prec, &value - &goal) / &weight;
            top = Some(match top {
                Some(t) => t.max(&need),
                None => need,
            });
        }
        let mut begin = start.clone();
        begin.push(crate::mfun::value(&top.unwrap_or_else(|| Float::new(prec))));
        let empty = Expr::list(Vec::new());
        let given = |k: usize| a.get(more + k).cloned().unwrap_or_else(|| empty.clone());
        let is_empty = |e: &Expr| e.as_list().is_some_and(|l| l.is_empty());
        // one column more for the bound in the rows, no bounds of its own
        let wider = |m: Expr| -> Expr {
            if is_empty(&m) {
                return m;
            }
            let rows = match m.as_list() {
                Some(flat) if flat.iter().all(|x| x.as_list().is_none()) => vec![flat.to_vec()],
                _ => crate::matrix::rows(&m).unwrap_or_default(),
            };
            Expr::list(
                rows.into_iter()
                    .map(|mut r| {
                        r.push(Expr::zero());
                        Expr::list(r)
                    })
                    .collect(),
            )
        };
        let longer = |b: Expr, end: Expr| -> Expr {
            if is_empty(&b) {
                return b;
            }
            let mut items = list_of(&b);
            items.push(end);
            Expr::list(items)
        };
        let args = vec![
            rule(bound.clone()),
            Expr::list(begin),
            wider(given(0)),
            given(1),
            wider(given(2)),
            given(3),
            longer(given(4), crate::canon::neg(Expr::inf())),
            longer(given(5), Expr::inf()),
            rule(Expr::list(vec![Expr::list(under), empty.clone()])),
        ];
        let found = self.fmincon(&args)?;
        let mut all = list_of(&found);
        all.truncate(n);
        // what is nothing beside the largest of the unknowns shows as 0
        let sizes: Vec<f64> = all
            .iter()
            .map(|v| {
                numeric::real_value(&crate::funcs::func("abs", vec![v.clone()]), 20)
                    .map_or(1.0, |size| size.to_f64())
            })
            .collect();
        let top = sizes.iter().fold(1.0f64, |s, v| s.max(*v));
        for (v, size) in all.iter_mut().zip(&sizes) {
            if *size <= top * 0.1f64.powi(numeric::digits() as i32) {
                *v = Expr::zero();
            }
        }
        Ok(match a[1].as_list() {
            Some(_) => Expr::list(all),
            None => all.swap_remove(0),
        })
    }

    /// `pdepe(m, pde, start, ends, mesh, times)`: an equation of heat in one dimension of
    /// space, `c*u_t = x^-m*(x^m*f)_x + s`, on a mesh; the times by steps that stiff
    /// equations allow.
    fn pdepe(&mut self, a: &[Expr]) -> Res {
        let name = "pdepe";
        let usage = "pdepe(m, @(x, t, u, ux) [c, f, s], @(x) start, @(xl, ul, xr, ur, t) [pl, ql, pr, qr], mesh, times): c*u_t = x^-m*(x^m*f)_x + s with p + q*f = 0 at both ends";
        let shape = crate::arrays::whole(&a[0], "the symmetry m", 0, 2)? as u32;
        let bits = numeric::bits_for(13);
        let prec = bits + 64;
        let mesh: Vec<Float> = list_of(&a[4])
            .iter()
            .map(|e| real(e, prec, name))
            .collect::<Result<_, _>>()?;
        let times = list_of(&a[5]);
        let marks: Vec<Float> = times
            .iter()
            .map(|e| real(e, prec, name))
            .collect::<Result<_, _>>()?;
        if mesh.len() < 3
            || marks.len() < 2
            || mesh.windows(2).any(|w| w[1] <= w[0])
            || marks.windows(2).any(|w| w[1] <= w[0])
        {
            return Err(format!(
                "{usage}; the mesh has three points or more, the times two or more, both rising"
            ));
        }
        if mesh.len() > 2000 {
            return Err("pdepe: 2000 points of the mesh at most".into());
        }
        if shape > 0 && mesh[0].is_sign_negative() {
            return Err("pdepe: with m = 1 or 2 the mesh begins at 0 or above".into());
        }
        let begin = self.many(&a[2], 1, name)?;
        let n = begin.items.len();
        let inside = self.many(&a[1], 2 + 2 * n, name)?;
        let ends = self.many(&a[3], 3 + 2 * n, name)?;
        if n == 0 || inside.items.len() != 3 * n || ends.items.len() != 4 * n {
            return Err(usage.into());
        }
        let nodes = mesh.len();
        let power =
            |x: &Float, k: u32| -> Float { Float::with_val(prec, rug::ops::Pow::pow(x, k)) };
        let middles: Vec<Float> = mesh
            .windows(2)
            .map(|w| Float::with_val(prec, &w[0] + &w[1]) / 2u32)
            .collect();
        // the shares of the cells that lie left and right of every point
        let lefts: Vec<Float> = (1..nodes)
            .map(|i| (power(&mesh[i], shape + 1) - power(&middles[i - 1], shape + 1)) / (shape + 1))
            .collect();
        let rights: Vec<Float> = (0..nodes - 1)
            .map(|i| (power(&middles[i], shape + 1) - power(&mesh[i], shape + 1)) / (shape + 1))
            .collect();
        let singular = shape > 0 && mesh[0].is_zero();
        let mut system = |z: &[Complex], t: &Float| -> Option<(Vec<Complex>, Vec<Complex>)> {
            let time = Complex::with_val(prec, t);
            // c, f, s in the middle of every stretch
            let mut cells: Vec<Vec<Complex>> = Vec::with_capacity(nodes - 1);
            for i in 0..nodes - 1 {
                let wide = Float::with_val(prec, &mesh[i + 1] - &mesh[i]);
                let mut args = vec![Complex::with_val(prec, &middles[i]), time.clone()];
                for k in 0..n {
                    args.push(Complex::with_val(prec, &z[i * n + k] + &z[(i + 1) * n + k]) / 2u32);
                }
                for k in 0..n {
                    args.push(Complex::with_val(prec, &z[(i + 1) * n + k] - &z[i * n + k]) / &wide);
                }
                cells.push(inside.at(&args, prec)?);
            }
            let mut args = vec![Complex::with_val(prec, &mesh[0])];
            args.extend(z[..n].iter().cloned());
            args.push(Complex::with_val(prec, &mesh[nodes - 1]));
            args.extend(z[(nodes - 1) * n..].iter().cloned());
            args.push(time);
            let edge = ends.at(&args, prec)?;
            let mut right = Vec::with_capacity(nodes * n);
            let mut weights = Vec::with_capacity(nodes * n);
            for i in 0..nodes {
                for k in 0..n {
                    let (c, f, s) = (k, n + k, 2 * n + k);
                    if i == 0 {
                        let cell = &cells[0];
                        let flow = Complex::with_val(prec, &cell[f] * power(&middles[0], shape))
                            + Complex::with_val(prec, &cell[s] * &rights[0]);
                        if singular {
                            weights.push(Complex::with_val(prec, &cell[c] * &rights[0]));
                            right.push(flow);
                        } else {
                            let (p, q) = (&edge[k], &edge[n + k]);
                            weights.push(Complex::with_val(prec, &cell[c] * &rights[0]) * q);
                            right.push(
                                flow * q + Complex::with_val(prec, p * power(&mesh[0], shape)),
                            );
                        }
                    } else if i == nodes - 1 {
                        let cell = &cells[nodes - 2];
                        let (p, q) = (&edge[2 * n + k], &edge[3 * n + k]);
                        let flow = Complex::with_val(prec, &cell[s] * &lefts[i - 1])
                            - Complex::with_val(prec, &cell[f] * power(&middles[i - 1], shape));
                        weights.push(Complex::with_val(prec, &cell[c] * &lefts[i - 1]) * q);
                        right.push(flow * q - Complex::with_val(prec, p * power(&mesh[i], shape)));
                    } else {
                        let (before, after) = (&cells[i - 1], &cells[i]);
                        weights.push(
                            Complex::with_val(prec, &before[c] * &lefts[i - 1])
                                + Complex::with_val(prec, &after[c] * &rights[i]),
                        );
                        right.push(
                            Complex::with_val(prec, &after[f] * power(&middles[i], shape))
                                - Complex::with_val(
                                    prec,
                                    &before[f] * power(&middles[i - 1], shape),
                                )
                                + Complex::with_val(prec, &before[s] * &lefts[i - 1])
                                + Complex::with_val(prec, &after[s] * &rights[i]),
                        );
                    }
                }
            }
            Some((right, weights))
        };
        let mut z: Vec<Complex> = Vec::with_capacity(nodes * n);
        for x in &mesh {
            z.extend(
                begin
                    .at(&[Complex::with_val(prec, x)], prec)
                    .ok_or("pdepe: the start has no value at a point of the mesh")?,
            );
        }
        let band = 2 * n - 1;
        // equations without slopes (ends held at a value) hold at the start too
        let none = || "pdepe: the equations have no values at the start".to_string();
        for _ in 0..30 {
            let (right, weights) = system(&z, &marks[0]).ok_or_else(none)?;
            let still: Vec<usize> = (0..z.len()).filter(|i| weights[*i].is_zero()).collect();
            let off = still
                .iter()
                .fold(Float::new(prec), |s, i| s.max(&size(&right[*i])));
            if still.is_empty() || off <= (largest(&z, prec) + 1u32) >> (bits + 8) {
                break;
            }
            let (slopes, _) =
                stiff_slopes(&mut system, &z, &marks[0], band, prec).ok_or_else(none)?;
            let part: CMat = still
                .iter()
                .map(|i| still.iter().map(|j| slopes[*i][*j].clone()).collect())
                .collect();
            let side: Vec<Complex> = still
                .iter()
                .map(|i| Complex::with_val(prec, -&right[*i]))
                .collect();
            let change = banded(part, side, band)
                .ok_or("pdepe: the conditions at the ends do not fix the values there")?;
            for (i, c) in still.iter().zip(&change) {
                z[*i] += c;
            }
        }
        let mut found: Vec<Vec<Complex>> = vec![z.clone()];
        let mut step = Float::new(prec);
        for pair in marks.windows(2) {
            z = stiff(&mut system, band, &pair[0], &pair[1], z, bits, &mut step)?;
            found.push(z.clone());
        }
        // twelve digits show; what is nothing beside the largest value shows as 0
        let top = found
            .iter()
            .flatten()
            .fold(Float::new(prec), |s, v| s.max(&size(v)));
        let small = top >> 44u32;
        let shown = |v: &Complex| -> Expr {
            let clean = |x: &Float| match Float::with_val(prec, x.abs_ref()) <= small {
                true => Float::new(prec),
                false => x.clone(),
            };
            let v = Complex::with_val(prec, (clean(v.real()), clean(v.imag())));
            match v.is_zero() {
                true => Expr::zero(),
                false => numeric::to_float_expr(v, 12),
            }
        };
        let sheet = |k: usize| -> Expr {
            Expr::list(
                found
                    .iter()
                    .map(|z| Expr::list((0..nodes).map(|i| shown(&z[i * n + k])).collect()))
                    .collect(),
            )
        };
        Ok(match n {
            1 => sheet(0),
            _ => Expr::list((0..n).map(sheet).collect()),
        })
    }

    /// `pdeval(m, mesh, values, x)`: the values of a row of pdepe between the points of the
    /// mesh, and their slopes: straight lines, as the cells of pdepe take them.
    fn pdeval(&mut self, a: &[Expr]) -> Res {
        let usage =
            "pdeval(m, mesh, values, x): a row of what pdepe gives, and places inside the mesh";
        crate::arrays::whole(&a[0], "the symmetry m", 0, 2)?;
        let (mesh, values) = (list_of(&a[1]), list_of(&a[2]));
        if mesh.len() < 2 || mesh.len() != values.len() {
            return Err(usage.into());
        }
        let prec = crate::mfun::bits();
        let marks: Vec<Float> = mesh
            .iter()
            .map(|e| real(e, prec, "pdeval"))
            .collect::<Result<_, _>>()?;
        let mut found = Vec::new();
        let mut slopes = Vec::new();
        for x in list_of(&a[3]) {
            let at = real(&x, prec, "pdeval")?;
            if at < marks[0] || at > marks[marks.len() - 1] {
                return Err("pdeval: the places lie inside the mesh".into());
            }
            let k = marks[1..marks.len() - 1]
                .iter()
                .filter(|m| **m <= at)
                .count();
            let slope = crate::canon::div(
                crate::canon::sub(values[k + 1].clone(), values[k].clone()),
                crate::canon::sub(mesh[k + 1].clone(), mesh[k].clone()),
            );
            let rise = crate::canon::mul(vec![
                slope.clone(),
                crate::canon::sub(x.clone(), mesh[k].clone()),
            ]);
            found.push(crate::arrays::tidy(crate::canon::add(vec![
                values[k].clone(),
                rise,
            ])));
            slopes.push(crate::arrays::tidy(slope));
        }
        Ok(match a[3].as_list() {
            Some(_) => Expr::list(vec![Expr::list(found), Expr::list(slopes)]),
            None => Expr::list(vec![found.swap_remove(0), slopes.swap_remove(0)]),
        })
    }

    pub(crate) fn solver_more(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        Some(match (name, a.len()) {
            ("pdepe", 6) => self.pdepe(a),
            ("pdeval", 4) => self.pdeval(a),
            ("patternsearch", 2 | 4 | 8) => (|| {
                let (target, start) = self.target(name, a, false)?;
                let from: Vec<Float> = start
                    .iter()
                    .map(|e| real(e, 64, name))
                    .collect::<Result<_, _>>()?;
                self.polished(name, &target, from, 1.0, a[1].as_list().is_none())
            })(),
            ("particleswarm" | "simulannealbnd" | "ga" | "globalsearch", 2 | 4 | 8) => {
                self.drawn(name, a)
            }
            ("fminimax", 2 | 4 | 6 | 8) | ("fgoalattain", 4 | 6 | 8 | 10) => {
                self.bounded_all(name, a)
            }
            _ => return None,
        })
    }
}
