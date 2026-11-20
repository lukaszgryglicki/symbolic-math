//! Sums and statistics of data the way MATLAB works them out: along a list, down the columns
//! of a matrix, or along the dimension that is named.

use crate::arrays::{
    Arr, LIMIT, arr, counted, map, mldivide, order, order_any, same, tidy, whole, word,
};
use crate::canon::{add, div, mul, neg, pow, sub};
use crate::expr::Expr;
use crate::funcs::func;
use crate::matrix::{self, Rows};
use crate::numeric;
use rug::ops::Pow;
use std::cmp::Ordering;

type Res = Result<Expr, String>;

#[derive(Clone, Copy, PartialEq)]
enum Dim {
    /// The first dimension that has more than one entry.
    First,
    Down,
    Across,
    All,
}

fn is_dim(e: &Expr) -> bool {
    matches!(e.as_i64(), Some(1 | 2)) || word(e).as_deref() == Some("all")
}

fn dim(e: Option<&Expr>, name: &str) -> Result<Dim, String> {
    match e {
        None => Ok(Dim::First),
        Some(x) => match (x.as_i64(), word(x).as_deref()) {
            (Some(1), _) => Ok(Dim::Down),
            (Some(2), _) => Ok(Dim::Across),
            (_, Some("all")) => Ok(Dim::All),
            _ => Err(format!(
                "{name}: the dimension is 1 (down the columns), 2 (along the rows) or \"all\", not {x}"
            )),
        },
    }
}

/// One value for a list, for every column or every row of a matrix, or for all entries.
fn reduce(e: &Expr, d: Dim, f: &mut dyn FnMut(&[Expr]) -> Res) -> Res {
    let each = |lists: Rows, f: &mut dyn FnMut(&[Expr]) -> Res| -> Res {
        Ok(Expr::list(
            lists.iter().map(|v| f(v)).collect::<Result<_, _>>()?,
        ))
    };
    match (arr(e), d) {
        (Arr::Scalar(x), _) => f(&[x]),
        (Arr::Vector(v), Dim::Down) => each(v.into_iter().map(|x| vec![x]).collect(), f),
        (Arr::Vector(v), _) => f(&v),
        (Arr::Matrix(m), Dim::All) => f(&m.concat()),
        (Arr::Matrix(m), Dim::First) if m.len() == 1 || m[0].len() == 1 => f(&m.concat()),
        (Arr::Matrix(m), Dim::Across) => each(m, f),
        (Arr::Matrix(m), _) => each(matrix::transpose(&m), f),
    }
}

/// What is done to a list.
type Each<'a> = dyn FnMut(&[Expr]) -> Result<Vec<Expr>, String> + 'a;

/// A list of the same length for a list, for every column or every row of a matrix.
fn running(e: &Expr, d: Dim, f: &mut Each) -> Res {
    match (arr(e), d) {
        (Arr::Scalar(x), _) => Ok(f(&[x])?.remove(0)),
        (Arr::Vector(v), Dim::Down) => Ok(Expr::list(
            v.iter()
                .map(|x| f(std::slice::from_ref(x)).map(|mut o| o.remove(0)))
                .collect::<Result<_, _>>()?,
        )),
        (Arr::Vector(v), _) => Ok(Expr::list(f(&v)?)),
        (Arr::Matrix(m), Dim::Across) => Ok(matrix::from_rows(
            m.iter().map(|r| f(r)).collect::<Result<_, _>>()?,
        )),
        (Arr::Matrix(m), Dim::First) if m.len() == 1 => Ok(matrix::from_rows(vec![f(&m[0])?])),
        (Arr::Matrix(_), Dim::All) => {
            Err("along \"all\" there is no list of the same shape".into())
        }
        (Arr::Matrix(m), _) => {
            let cols: Rows = matrix::transpose(&m)
                .iter()
                .map(|c| f(c))
                .collect::<Result<_, _>>()?;
            Ok(matrix::from_rows(matrix::transpose(&cols)))
        }
    }
}

fn from_stats(which: &str, v: &[Expr]) -> Res {
    crate::stats::call(which, &[Expr::list(v.to_vec())])
        .unwrap_or_else(|| Err(format!("{which} is not known")))
}

pub(crate) fn stat(name: &str, v: &[Expr]) -> Res {
    let n = v.len();
    Ok(match name {
        "sum" => tidy(add(v.to_vec())),
        "prod" => tidy(mul(v.to_vec())),
        _ if n == 0 => Expr::undefined(),
        "mean" => tidy(div(add(v.to_vec()), Expr::int(n as i64))),
        "median" => from_stats("median", v)?,
        "mode" => {
            let all = from_stats("mode", v)?;
            match all.as_list().and_then(<[Expr]>::first) {
                Some(first) => first.clone(),
                None => all,
            }
        }
        "var" | "std" | "pvar" | "pstd" if n == 1 => Expr::zero(),
        "var" => from_stats("variance", v)?,
        "std" => from_stats("stddev", v)?,
        "pvar" => from_stats("pvariance", v)?,
        "pstd" => from_stats("pstddev", v)?,
        "max" | "min" => func(name, v.to_vec()),
        "range" => tidy(sub(func("max", v.to_vec()), func("min", v.to_vec()))),
        "mad" => {
            let m = stat("median", v)?;
            let away: Vec<Expr> = v
                .iter()
                .map(|x| func("abs", vec![tidy(sub(x.clone(), m.clone()))]))
                .collect();
            stat("median", &away)?
        }
        "meanabsdev" => {
            let m = stat("mean", v)?;
            let away: Vec<Expr> = v
                .iter()
                .map(|x| func("abs", vec![tidy(sub(x.clone(), m.clone()))]))
                .collect();
            stat("mean", &away)?
        }
        _ => return Err(format!("{name} is not known")),
    })
}

fn cumulative(name: &str, v: &[Expr]) -> Vec<Expr> {
    let mut out: Vec<Expr> = Vec::with_capacity(v.len());
    for x in v {
        let next = match (out.last(), name) {
            (None, _) => x.clone(),
            (Some(p), "cumsum" | "accumulate") => tidy(add(vec![p.clone(), x.clone()])),
            (Some(p), "cumprod") => tidy(mul(vec![p.clone(), x.clone()])),
            (Some(p), "cummax") => func("max", vec![p.clone(), x.clone()]),
            (Some(p), _) => func("min", vec![p.clone(), x.clone()]),
        };
        out.push(next);
    }
    out
}

/// How far a window reaches back and ahead: `k` entries about the middle, or `[back, ahead]`.
fn window(e: &Expr) -> Result<(usize, usize), String> {
    let n = |x: &Expr, what: &str, lo: i64| whole(x, what, lo, LIMIT as i64).map(|k| k as usize);
    match e.as_list() {
        Some([b, f]) => Ok((n(b, "the reach back", 0)?, n(f, "the reach ahead", 0)?)),
        Some(_) => Err("a window is its length k or [back, ahead]".into()),
        None => {
            let k = n(e, "the length of the window", 1)?;
            Ok(if k % 2 == 1 {
                ((k - 1) / 2, (k - 1) / 2)
            } else {
                (k / 2, k / 2 - 1)
            })
        }
    }
}

fn moving(v: &[Expr], reach: (usize, usize), which: &str) -> Result<Vec<Expr>, String> {
    (0..v.len())
        .map(|i| {
            let lo = i.saturating_sub(reach.0);
            let hi = (i + reach.1).min(v.len() - 1);
            stat(which, &v[lo..=hi])
        })
        .collect()
}

/// The running areas under the points by the rule of trapezes; the first is 0.
fn trapezes(x: Option<&[Expr]>, y: &[Expr]) -> Result<Vec<Expr>, String> {
    if let Some(x) = x
        && x.len() != y.len()
    {
        return Err(format!("{} places and {} values", x.len(), y.len()));
    }
    let mut out = vec![Expr::zero()];
    for i in 1..y.len() {
        let width = match x {
            Some(x) => sub(x[i].clone(), x[i - 1].clone()),
            None => Expr::one(),
        };
        let piece = mul(vec![
            Expr::half(),
            width,
            add(vec![y[i].clone(), y[i - 1].clone()]),
        ]);
        let so_far = out[i - 1].clone();
        out.push(tidy(add(vec![so_far, piece])));
    }
    Ok(out)
}

fn slopes(y: &[Expr], x: Option<&[Expr]>, step: &Expr) -> Result<Vec<Expr>, String> {
    let n = y.len();
    if n < 2 {
        return Ok(vec![Expr::zero(); n]);
    }
    if let Some(x) = x
        && x.len() != n
    {
        return Err(format!("gradient: {} places and {n} values", x.len()));
    }
    let run = |i: usize, j: usize| match x {
        Some(x) => sub(x[j].clone(), x[i].clone()),
        None => mul(vec![Expr::int((j - i) as i64), step.clone()]),
    };
    Ok((0..n)
        .map(|i| {
            let (lo, hi) = (i.saturating_sub(1), (i + 1).min(n - 1));
            tidy(div(sub(y[hi].clone(), y[lo].clone()), run(lo, hi)))
        })
        .collect())
}

/// The value at the percent `p` of sorted data as MATLAB takes it: entry k of n stands at the
/// percent 100(k - 1/2)/n, a straight line runs between two entries.
fn percentile(sorted: &[Expr], p: &Expr) -> Res {
    let n = sorted.len();
    if n == 0 {
        return Ok(Expr::undefined());
    }
    let pos = tidy(add(vec![
        div(mul(vec![Expr::int(n as i64), p.clone()]), Expr::int(100)),
        Expr::half(),
    ]));
    let at = numeric::real_value(&pos, 30)
        .ok_or_else(|| format!("prctile: the percent {p} must be a number from 0 to 100"))?;
    let percent = numeric::real_value(p, 30)
        .map(|f| f.to_f64())
        .unwrap_or(-1.0);
    if !(0.0..=100.0).contains(&percent) {
        return Err(format!(
            "prctile: the percent {p} must be a number from 0 to 100"
        ));
    }
    if at <= 1 {
        return Ok(sorted[0].clone());
    }
    if at >= n as f64 {
        return Ok(sorted[n - 1].clone());
    }
    let k = at.floor().to_f64() as usize;
    let part = sub(pos, Expr::int(k as i64));
    Ok(tidy(add(vec![
        sorted[k - 1].clone(),
        mul(vec![part, sub(sorted[k].clone(), sorted[k - 1].clone())]),
    ])))
}

fn conj_num(x: Expr) -> Expr {
    if x.is_const() {
        func("conj", vec![x])
    } else {
        x
    }
}

/// The covariances of the columns; `population` divides by n and not by n - 1.
fn covariances(cols: &Rows, population: bool) -> Result<Rows, String> {
    let n = cols[0].len();
    if n == 0 || cols.iter().any(|c| c.len() != n) {
        return Err("cov needs columns of one length that are not empty".into());
    }
    let centred: Rows = cols
        .iter()
        .map(|c| {
            let m = div(add(c.clone()), Expr::int(n as i64));
            c.iter().map(|x| tidy(sub(x.clone(), m.clone()))).collect()
        })
        .collect();
    let by = Expr::int(if population || n == 1 {
        n as i64
    } else {
        n as i64 - 1
    });
    Ok(centred
        .iter()
        .map(|p| {
            centred
                .iter()
                .map(|q| {
                    let total: Vec<Expr> = p
                        .iter()
                        .zip(q)
                        .map(|(x, y)| mul(vec![conj_num(x.clone()), y.clone()]))
                        .collect();
                    tidy(div(add(total), by.clone()))
                })
                .collect()
        })
        .collect())
}

/// The data of `cov` and `corrcoef` as columns, and the flag of the population.
fn columns_of(name: &str, a: &[Expr]) -> Result<(Rows, bool), String> {
    let flag = |e: &Expr| matches!(e.as_i64(), Some(0 | 1)) && e.as_list().is_none();
    let (data, population) = match a {
        [x] => (vec![x], false),
        [x, w] if flag(w) => (vec![x], w.is_one()),
        [x, y] => (vec![x, y], false),
        [x, y, w] if flag(w) => (vec![x, y], w.is_one()),
        _ => return Err(format!("{name}(x), {name}(x, y), {name}(matrix)")),
    };
    let mut cols: Rows = Vec::new();
    let pair = data.len() == 2;
    for d in data {
        match arr(d) {
            Arr::Scalar(x) => cols.push(vec![x]),
            Arr::Vector(v) => cols.push(v),
            Arr::Matrix(m) if pair || m.len() == 1 || m[0].len() == 1 => {
                cols.push(m.concat());
            }
            Arr::Matrix(m) => cols.extend(matrix::transpose(&m)),
        }
    }
    Ok((cols, population))
}

/// The data less the polynomial of least squares of a degree through the points 0, 1, 2, ...
fn detrended(v: &[Expr], degree: usize) -> Result<Vec<Expr>, String> {
    if v.len() <= degree {
        return Ok(vec![Expr::zero(); v.len()]);
    }
    let design: Rows = (0..v.len())
        .map(|t| {
            (0..=degree)
                .map(|k| Expr::integer(rug::Integer::from(t).pow(k as u32)))
                .collect()
        })
        .collect();
    let fit = mldivide(&matrix::from_rows(design.clone()), &Expr::list(v.to_vec()))?;
    let c = fit.as_list().ok_or("detrend: no fit")?;
    Ok(v.iter()
        .zip(&design)
        .map(|(x, row)| {
            let line: Vec<Expr> = row
                .iter()
                .zip(c)
                .map(|(p, k)| mul(vec![p.clone(), k.clone()]))
                .collect();
            tidy(sub(x.clone(), add(line)))
        })
        .collect())
}

fn unwrapped(v: &[Expr], tol: &Expr) -> Result<Vec<Expr>, String> {
    let two_pi = mul(vec![Expr::int(2), Expr::pi()]);
    let mut out: Vec<Expr> = Vec::with_capacity(v.len());
    let mut fix = Expr::zero();
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            let step = tidy(sub(x.clone(), v[i - 1].clone()));
            let turns = numeric::real_value(
                &div(add(vec![step.clone(), Expr::pi()]), two_pi.clone()),
                30,
            )
            .ok_or("unwrap needs real angles")?
            .floor();
            let turns = turns.to_integer().ok_or("unwrap needs real angles")?;
            let mut small = tidy(sub(
                step.clone(),
                mul(vec![Expr::integer(turns), two_pi.clone()]),
            ));
            if same(&small, &neg(Expr::pi())) && order(&step, &Expr::zero())? == Ordering::Greater {
                small = Expr::pi();
            }
            if order(&func("abs", vec![step.clone()]), tol)? != Ordering::Less {
                fix = tidy(add(vec![fix, sub(small, step)]));
            }
        }
        out.push(tidy(add(vec![x.clone(), fix.clone()])));
    }
    Ok(out)
}

fn sorted_real(v: &[Expr]) -> Result<Vec<Expr>, String> {
    let mut failed = None;
    let mut out = v.to_vec();
    out.sort_by(|x, y| {
        order(x, y).unwrap_or_else(|e| {
            failed = Some(e);
            Ordering::Equal
        })
    });
    match failed {
        Some(e) => Err(e),
        None => Ok(out),
    }
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    let is_list = |k: usize| a.get(k).is_some_and(|x| x.as_list().is_some());
    let is_matrix = |k: usize| a.get(k).is_some_and(|x| matrix::rows(x).is_some());
    let has_dim = |k: usize| a.get(k).is_some_and(is_dim);
    let data = |k: usize| counted(&a[k]).unwrap_or_else(|| a[k].clone());
    let numbers = |k: usize| is_list(k) && a[k].free_syms().is_empty();
    let which = match name {
        "sum" | "total" => "sum",
        "product" => "prod",
        "avg" => "mean",
        "variance" => "var",
        "stddev" => "std",
        other => other,
    };
    Ok(Some(match (name, n) {
        ("sum" | "total" | "avg", 1) if counted(&a[0]).is_some() => {
            reduce(&data(0), Dim::First, &mut |v| stat(which, v))?
        }
        ("product" | "avg" | "median" | "mode" | "max" | "min", 1) if is_matrix(0) => {
            reduce(&a[0], Dim::First, &mut |v| stat(which, v))?
        }
        ("sum" | "total" | "product" | "avg" | "median" | "mode", 2)
            if is_list(0) && has_dim(1) =>
        {
            reduce(&data(0), dim(a.get(1), name)?, &mut |v| stat(which, v))?
        }
        ("variance" | "stddev", 1) if is_matrix(0) => {
            reduce(&a[0], Dim::First, &mut |v| stat(which, v))?
        }
        ("variance" | "stddev", 2 | 3) if is_list(0) => {
            let population = match a[1].as_i64() {
                Some(0) => false,
                Some(1) => true,
                _ => {
                    return Err(format!(
                        "{name}(x, w): w is 0 to divide by n - 1 and 1 to divide by n"
                    ));
                }
            };
            let which = match (which, population) {
                ("var", true) => "pvar",
                ("std", true) => "pstd",
                (w, _) => w,
            };
            reduce(&a[0], dim(a.get(2), name)?, &mut |v| stat(which, v))?
        }
        ("max" | "min", 2) if is_list(0) || is_list(1) => {
            matrix::zip(&a[0], &a[1], &|x, y| func(name, vec![x.clone(), y.clone()]))?
        }
        ("max" | "min", 3) if is_list(0) && a[1].as_list().is_some_and(<[Expr]>::is_empty) => {
            reduce(&a[0], dim(a.get(2), name)?, &mut |v| stat(which, v))?
        }
        ("accumulate", 2) if is_list(0) && has_dim(1) => {
            running(&data(0), dim(a.get(1), name)?, &mut |v| {
                Ok(cumulative(name, v))
            })?
        }
        ("cumprod" | "cummax" | "cummin", 1 | 2) => {
            running(&data(0), dim(a.get(1), name)?, &mut |v| {
                Ok(cumulative(name, v))
            })?
        }
        ("bounds", 1 | 2) => {
            let d = dim(a.get(1), name)?;
            Expr::list(vec![
                reduce(&a[0], d, &mut |v| stat("min", v))?,
                reduce(&a[0], d, &mut |v| stat("max", v))?,
            ])
        }
        ("peak2peak", 1 | 2) => reduce(&a[0], dim(a.get(1), name)?, &mut |v| stat("range", v))?,
        ("maxk" | "mink", 2) => {
            let v = a[0].as_list().ok_or_else(|| format!("{name}(list, k)"))?;
            let k = whole(&a[1], "the number of entries", 0, LIMIT as i64)? as usize;
            let mut all = sorted_real(v)?;
            if name == "maxk" {
                all.reverse();
            }
            all.truncate(k);
            Expr::list(all)
        }
        (
            "movmean" | "movsum" | "movmedian" | "movmax" | "movmin" | "movstd" | "movvar"
            | "movprod" | "movmad",
            2 | 3,
        ) => {
            let reach = window(&a[1])?;
            let which = &name[3..];
            running(&a[0], dim(a.get(2), name)?, &mut |v| {
                moving(v, reach, which)
            })?
        }
        ("trapz" | "cumtrapz", 1 | 2) => {
            let y = &a[n - 1];
            let (places, scale) = match (n, a[0].as_list()) {
                (2, Some(x)) => (Some(x.to_vec()), Expr::one()),
                (2, None) => (None, a[0].clone()),
                _ => (None, Expr::one()),
            };
            let mut areas = |v: &[Expr]| -> Result<Vec<Expr>, String> {
                Ok(trapezes(places.as_deref(), v)?
                    .into_iter()
                    .map(|x| tidy(mul(vec![scale.clone(), x])))
                    .collect())
            };
            if name == "cumtrapz" {
                running(y, Dim::First, &mut areas)?
            } else {
                reduce(y, Dim::First, &mut |v| {
                    Ok(areas(v)?.pop().unwrap_or_else(Expr::zero))
                })?
            }
        }
        ("diff", 1 | 2) if numbers(0) && (n == 1 || a[1].as_i64().is_some_and(|k| k >= 0)) => {
            let times = if n == 2 {
                whole(&a[1], "the order", 0, 1000)?
            } else {
                1
            };
            let mut out = a[0].clone();
            for _ in 0..times {
                let items = out.as_list().unwrap_or_default().to_vec();
                let flat = matrix::rows(&out).is_some_and(|m| m.len() == 1);
                out = if flat {
                    let row = items[0].as_list().unwrap_or_default();
                    matrix::from_rows(vec![
                        row.windows(2)
                            .map(|w| tidy(sub(w[1].clone(), w[0].clone())))
                            .collect(),
                    ])
                } else {
                    Expr::list(
                        items
                            .windows(2)
                            .map(|w| {
                                matrix::add(vec![
                                    w[1].clone(),
                                    matrix::mul(vec![Expr::minus_one(), w[0].clone()])?,
                                ])
                            })
                            .collect::<Result<_, _>>()?,
                    )
                };
            }
            out
        }
        ("grad", 1 | 2) if numbers(0) && (n == 1 || a[1].free_syms().is_empty()) => {
            let (places, step) = match a.get(1) {
                Some(Expr::List(x)) => (Some(x.to_vec()), Expr::one()),
                Some(h) => (None, h.clone()),
                None => (None, Expr::one()),
            };
            match arr(&a[0]) {
                Arr::Matrix(m) if m.len() > 1 && m[0].len() > 1 => {
                    if places.is_some() {
                        return Err("gradient(matrix, h): one step for both directions".into());
                    }
                    let along: Rows = m
                        .iter()
                        .map(|r| slopes(r, None, &step))
                        .collect::<Result<_, _>>()?;
                    let down: Rows = matrix::transpose(&m)
                        .iter()
                        .map(|c| slopes(c, None, &step))
                        .collect::<Result<_, _>>()?;
                    Expr::list(vec![
                        matrix::from_rows(along),
                        matrix::from_rows(matrix::transpose(&down)),
                    ])
                }
                _ => running(&a[0], Dim::First, &mut |v| {
                    slopes(v, places.as_deref(), &step)
                })?,
            }
        }
        ("prctile", 2 | 3) => {
            let d = dim(a.get(2), name)?;
            let one = |v: &[Expr], p: &Expr| percentile(&sorted_real(v)?, p);
            match a[1].as_list() {
                None => reduce(&a[0], d, &mut |v| one(v, &a[1]))?,
                Some(ps) => {
                    let per: Vec<Expr> = ps
                        .iter()
                        .map(|p| reduce(&a[0], d, &mut |v| one(v, p)))
                        .collect::<Result<_, _>>()?;
                    Expr::list(per)
                }
            }
        }
        ("zscore", 1..=3) => {
            let which = match a.get(1).map(Expr::as_i64) {
                None | Some(Some(0)) => "std",
                Some(Some(1)) => "pstd",
                _ => {
                    return Err(
                        "zscore(x, w): w is 0 to divide by n - 1 and 1 to divide by n".into(),
                    );
                }
            };
            running(&a[0], dim(a.get(2), name)?, &mut |v| {
                let (m, s) = (stat("mean", v)?, stat(which, v)?);
                let flat = crate::simplify::is_zero(&s);
                Ok(v.iter()
                    .map(|x| {
                        let d = sub(x.clone(), m.clone());
                        tidy(if flat { d } else { div(d, s.clone()) })
                    })
                    .collect())
            })?
        }
        ("rescale", 1 | 3) => {
            let (lo, hi) = if n == 3 {
                (a[1].clone(), a[2].clone())
            } else {
                (Expr::zero(), Expr::one())
            };
            let all = crate::arrays::column_major(&a[0]);
            let (least, most) = (stat("min", &all)?, stat("max", &all)?);
            let width = tidy(sub(most.clone(), least.clone()));
            let room = sub(hi.clone(), lo.clone());
            if crate::simplify::is_zero(&width) {
                map(&a[0], &|_| hi.clone())
            } else {
                map(&a[0], &|x| {
                    tidy(add(vec![
                        lo.clone(),
                        mul(vec![
                            div(sub(x.clone(), least.clone()), width.clone()),
                            room.clone(),
                        ]),
                    ]))
                })
            }
        }
        ("trimmean", 2 | 3) => {
            let percent = numeric::real_value(&a[1], 30)
                .map(|f| f.to_f64())
                .filter(|p| (0.0..100.0).contains(p))
                .ok_or("trimmean(x, percent): the percent is a number from 0 to under 100")?;
            reduce(&a[0], dim(a.get(2), name)?, &mut |v| {
                let all = sorted_real(v)?;
                let k = all.len() as f64 * percent / 200.0;
                // (a half goes down, as MATLAB rounds here)
                let cut = (k - 0.5).ceil().max(0.0) as usize;
                if 2 * cut >= all.len() {
                    return Ok(Expr::undefined());
                }
                stat("mean", &all[cut..all.len() - cut])
            })?
        }
        ("mad", 1..=3) => {
            let which = match a.get(1).map(Expr::as_i64) {
                None | Some(Some(0)) => "meanabsdev",
                Some(Some(1)) => "mad",
                _ => return Err("mad(x, flag): flag 0 for the mean of the distances from the mean, 1 for the median of the distances from the median".into()),
            };
            reduce(&a[0], dim(a.get(2), name)?, &mut |v| stat(which, v))?
        }
        ("cov" | "corrcoef", 1..=3) => {
            let (cols, population) = columns_of(name, a)?;
            let c = covariances(&cols, population)?;
            if name == "cov" {
                if c.len() == 1 {
                    c[0][0].clone()
                } else {
                    matrix::from_rows(c)
                }
            } else {
                let size = c.len();
                let r: Rows = (0..size)
                    .map(|i| {
                        (0..size)
                            .map(|j| {
                                if i == j {
                                    Expr::one()
                                } else {
                                    tidy(div(
                                        c[i][j].clone(),
                                        pow(
                                            mul(vec![c[i][i].clone(), c[j][j].clone()]),
                                            Expr::half(),
                                        ),
                                    ))
                                }
                            })
                            .collect()
                    })
                    .collect();
                if size == 1 {
                    Expr::one()
                } else {
                    matrix::from_rows(r)
                }
            }
        }
        ("histcounts", 2) => {
            let v = a[0]
                .as_list()
                .ok_or("histcounts(list, edges) or histcounts(list, bins)")?;
            let mut all = Vec::new();
            for x in v {
                match x.as_list() {
                    Some(inner) => all.extend(inner.iter().cloned()),
                    None => all.push(x.clone()),
                }
            }
            let edges: Vec<Expr> = match a[1].as_list() {
                Some(e) if e.len() >= 2 => e.to_vec(),
                Some(_) => return Err("histcounts: two edges or more".into()),
                None => {
                    let bins = whole(&a[1], "the number of bins", 1, LIMIT as i64)?;
                    let (lo, hi) = (stat("min", &all)?, stat("max", &all)?);
                    let width = sub(hi.clone(), lo.clone());
                    (0..=bins)
                        .map(|k| {
                            if k == bins {
                                hi.clone()
                            } else {
                                tidy(add(vec![
                                    lo.clone(),
                                    mul(vec![Expr::rat(k, bins), width.clone()]),
                                ]))
                            }
                        })
                        .collect()
                }
            };
            let mut counts = vec![0i64; edges.len() - 1];
            let last = edges.len() - 2;
            for x in &all {
                for k in 0..=last {
                    let above = order(x, &edges[k])? != Ordering::Less;
                    let below = match order(x, &edges[k + 1])? {
                        Ordering::Less => true,
                        Ordering::Equal => k == last,
                        Ordering::Greater => false,
                    };
                    if above && below {
                        counts[k] += 1;
                        break;
                    }
                }
            }
            Expr::list(counts.into_iter().map(Expr::int).collect())
        }
        ("detrend", 1 | 2) => {
            let degree = match a.get(1) {
                None => 1,
                Some(d) => match word(d).as_deref() {
                    Some("constant") => 0,
                    Some("linear") => 1,
                    _ => whole(d, "the degree", 0, 100)? as usize,
                },
            };
            running(&a[0], Dim::First, &mut |v| detrended(v, degree))?
        }
        ("unwrap", 1 | 2) => {
            let tol = a.get(1).cloned().unwrap_or_else(Expr::pi);
            running(&a[0], Dim::First, &mut |v| unwrapped(v, &tol))?
        }
        ("accumarray", 2 | 3) => {
            let places = a[0].as_list().ok_or("accumarray(places, values)")?;
            let values: Vec<Expr> = match a[1].as_list() {
                Some(v) if v.len() == places.len() => v.to_vec(),
                Some(_) => return Err("accumarray: a value for every place".into()),
                None => vec![a[1].clone(); places.len()],
            };
            let at = |e: &Expr| whole(e, "a place", 1, LIMIT as i64).map(|k| k as usize);
            if places
                .iter()
                .all(|p| p.as_list().is_some_and(|l| l.len() == 2))
                && !places.is_empty()
            {
                let mut cells: Vec<(usize, usize)> = Vec::new();
                for p in places {
                    let l = p.as_list().unwrap_or_default();
                    cells.push((at(&l[0])?, at(&l[1])?));
                }
                let (mut rows, mut cols) = (0, 0);
                for (r, c) in &cells {
                    rows = rows.max(*r);
                    cols = cols.max(*c);
                }
                if let Some(s) = a.get(2).and_then(Expr::as_list).filter(|s| s.len() == 2) {
                    rows = rows.max(at(&s[0])?);
                    cols = cols.max(at(&s[1])?);
                }
                if rows.saturating_mul(cols) > LIMIT {
                    return Err(format!("an array of more than {LIMIT} entries"));
                }
                let mut sums = vec![vec![Vec::new(); cols]; rows];
                for ((r, c), v) in cells.into_iter().zip(values) {
                    sums[r - 1][c - 1].push(v);
                }
                matrix::from_rows(
                    sums.into_iter()
                        .map(|r| r.into_iter().map(|v| tidy(add(v))).collect())
                        .collect(),
                )
            } else {
                let cells: Vec<usize> = places.iter().map(at).collect::<Result<_, _>>()?;
                let mut size = cells.iter().copied().max().unwrap_or(0);
                if let Some(s) = a.get(2) {
                    size = size.max(at(s)?);
                }
                let mut sums = vec![Vec::new(); size];
                for (c, v) in cells.into_iter().zip(values) {
                    sums[c - 1].push(v);
                }
                Expr::list(sums.into_iter().map(|v| tidy(add(v))).collect())
            }
        }
        ("tabulate", 1) => {
            let v = a[0].as_list().ok_or("tabulate needs a list")?;
            let mut seen: Vec<(Expr, i64)> = Vec::new();
            for x in crate::lists::sorted(v) {
                match seen.last_mut() {
                    Some((y, k)) if same(y, &x) => *k += 1,
                    _ => seen.push((x, 1)),
                }
            }
            let total = v.len() as i64;
            Expr::list(
                seen.into_iter()
                    .map(|(x, k)| Expr::list(vec![x, Expr::int(k), Expr::rat(100 * k, total)]))
                    .collect(),
            )
        }
        ("tiedrank", 1) => {
            let v = a[0].as_list().ok_or("tiedrank needs a list")?;
            let mut places: Vec<usize> = (0..v.len()).collect();
            places.sort_by(|i, j| order_any(&v[*i], &v[*j]));
            let mut ranks = vec![Expr::zero(); v.len()];
            let mut i = 0;
            while i < places.len() {
                let mut j = i;
                while j + 1 < places.len() && same(&v[places[j + 1]], &v[places[i]]) {
                    j += 1;
                }
                let rank = Expr::rat((i + j + 2) as i64, 2);
                for p in &places[i..=j] {
                    ranks[*p] = rank.clone();
                }
                i = j + 1;
            }
            Expr::list(ranks)
        }
        ("isoutlier" | "rmoutliers", 1 | 2) => {
            let v = a[0]
                .as_list()
                .ok_or_else(|| format!("{name} needs a list"))?;
            let far = |x: &Expr, mid: &Expr, reach: &Expr| -> Result<bool, String> {
                Ok(
                    order(&func("abs", vec![tidy(sub(x.clone(), mid.clone()))]), reach)?
                        == Ordering::Greater,
                )
            };
            let flags: Vec<bool> = match a.get(1).map(word) {
                None => {
                    // three scaled median distances from the median
                    let scale = div(
                        Expr::minus_one(),
                        mul(vec![
                            pow(Expr::int(2), Expr::half()),
                            func("erfcinv", vec![Expr::rat(3, 2)]),
                        ]),
                    );
                    let (mid, spread) = (stat("median", v)?, stat("mad", v)?);
                    let reach = mul(vec![Expr::int(3), scale, spread]);
                    v.iter()
                        .map(|x| far(x, &mid, &reach))
                        .collect::<Result<_, _>>()?
                }
                Some(Some(w)) if w == "median" => {
                    return inner(name, &a[..1]);
                }
                Some(Some(w)) if w == "mean" => {
                    let (mid, spread) = (stat("mean", v)?, stat("std", v)?);
                    let reach = mul(vec![Expr::int(3), spread]);
                    v.iter()
                        .map(|x| far(x, &mid, &reach))
                        .collect::<Result<_, _>>()?
                }
                Some(Some(w)) if w == "quartiles" => {
                    let sorted = sorted_real(v)?;
                    let (q1, q3) = (
                        percentile(&sorted, &Expr::int(25))?,
                        percentile(&sorted, &Expr::int(75))?,
                    );
                    let reach = mul(vec![Expr::rat(3, 2), sub(q3.clone(), q1.clone())]);
                    let (low, high) = (tidy(sub(q1, reach.clone())), tidy(add(vec![q3, reach])));
                    v.iter()
                        .map(|x| {
                            Ok(order(x, &low)? == Ordering::Less
                                || order(x, &high)? == Ordering::Greater)
                        })
                        .collect::<Result<_, String>>()?
                }
                Some(_) => {
                    return Err(format!(
                        "{name}(list), {name}(list, \"mean\"), {name}(list, \"quartiles\")"
                    ));
                }
            };
            if name == "isoutlier" {
                Expr::list(flags.into_iter().map(Expr::Bool).collect())
            } else {
                Expr::list(
                    v.iter()
                        .zip(flags)
                        .filter(|(_, out)| !out)
                        .map(|(x, _)| x.clone())
                        .collect(),
                )
            }
        }
        ("ismissing", 1) => map(&a[0], &|x| Expr::Bool(x.is_undefined())),
        ("rmmissing", 1) => match arr(&a[0]) {
            Arr::Matrix(m) => {
                let kept: Rows = m
                    .into_iter()
                    .filter(|r| !r.iter().any(Expr::is_undefined))
                    .collect();
                if kept.is_empty() {
                    Expr::list(Vec::new())
                } else {
                    matrix::from_rows(kept)
                }
            }
            Arr::Vector(v) => Expr::list(v.into_iter().filter(|x| !x.is_undefined()).collect()),
            Arr::Scalar(x) => x,
        },
        ("fillmissing", 2 | 3) => {
            let v = a[0].as_list().ok_or("fillmissing needs a list")?;
            let how = word(&a[1]).unwrap_or_default();
            let known: Vec<usize> = (0..v.len()).filter(|k| !v[*k].is_undefined()).collect();
            let before = |k: usize| known.iter().rev().find(|j| **j < k).copied();
            let after = |k: usize| known.iter().find(|j| **j > k).copied();
            let mut out = v.to_vec();
            for k in 0..v.len() {
                if !v[k].is_undefined() {
                    continue;
                }
                out[k] = match (how.as_str(), before(k), after(k)) {
                    ("constant", _, _) => a
                        .get(2)
                        .cloned()
                        .ok_or("fillmissing(list, \"constant\", value)")?,
                    ("previous", Some(p), _) => v[p].clone(),
                    ("next", _, Some(q)) => v[q].clone(),
                    ("nearest", Some(p), Some(q)) => v[if k - p <= q - k { p } else { q }].clone(),
                    ("nearest", Some(p), None) | ("nearest", None, Some(p)) => v[p].clone(),
                    ("linear", Some(p), Some(q)) => tidy(add(vec![
                        v[p].clone(),
                        mul(vec![
                            Expr::rat((k - p) as i64, (q - p) as i64),
                            sub(v[q].clone(), v[p].clone()),
                        ]),
                    ])),
                    ("previous" | "next" | "nearest" | "linear", _, _) => Expr::undefined(),
                    _ => {
                        return Err(
                            "fillmissing(list, \"constant\", value), or with \"previous\", \"next\", \"nearest\", \"linear\""
                                .into(),
                        );
                    }
                };
            }
            Expr::list(out)
        }
        ("discretize", 2) => {
            let edges = a[1]
                .as_list()
                .filter(|e| e.len() >= 2)
                .ok_or("discretize(values, edges): two edges or more")?;
            let last = edges.len() - 2;
            let bin = |x: &Expr| -> Res {
                for k in 0..=last {
                    let above = order(x, &edges[k])? != Ordering::Less;
                    let below = match order(x, &edges[k + 1])? {
                        Ordering::Less => true,
                        Ordering::Equal => k == last,
                        Ordering::Greater => false,
                    };
                    if above && below {
                        return Ok(Expr::int(k as i64 + 1));
                    }
                }
                Ok(Expr::undefined())
            };
            match a[0].as_list() {
                Some(v) => Expr::list(v.iter().map(bin).collect::<Result<_, _>>()?),
                None => bin(&a[0])?,
            }
        }
        ("findgroups", 1) => {
            let v = a[0].as_list().ok_or("findgroups needs a list")?;
            let mut keys: Vec<Expr> = Vec::new();
            for x in crate::lists::sorted(v) {
                if !keys.last().is_some_and(|l| same(l, &x)) {
                    keys.push(x);
                }
            }
            Expr::list(
                v.iter()
                    .map(|x| {
                        Expr::int(
                            keys.iter()
                                .position(|k| same(k, x))
                                .map_or(0, |p| p as i64 + 1),
                        )
                    })
                    .collect(),
            )
        }
        ("ecdf", 1) => {
            let v = sorted_real(a[0].as_list().ok_or("ecdf needs a list")?)?;
            let total = v.len() as i64;
            let (mut values, mut shares) = (Vec::new(), Vec::new());
            for (k, x) in v.iter().enumerate() {
                if v.get(k + 1).is_some_and(|next| same(next, x)) {
                    continue;
                }
                values.push(x.clone());
                shares.push(Expr::rat(k as i64 + 1, total));
            }
            Expr::list(vec![Expr::list(values), Expr::list(shares)])
        }
        ("cplxpair", 1) => {
            let v = a[0].as_list().ok_or("cplxpair needs a list")?;
            let digits = numeric::digits();
            let mut parts: Vec<(f64, f64, Expr)> = Vec::new();
            for x in v {
                let z = numeric::evalf_digits(x, digits)
                    .map_err(|_| format!("cplxpair: {x} is not a number"))?;
                parts.push((z.real().to_f64(), z.imag().to_f64(), x.clone()));
            }
            let tiny = |im: f64, re: f64| im.abs() <= 1e-12 * (1.0 + re.abs());
            let mut reals: Vec<(f64, Expr)> = parts
                .iter()
                .filter(|p| tiny(p.1, p.0))
                .map(|p| (p.0, p.2.clone()))
                .collect();
            reals.sort_by(|p, q| p.0.total_cmp(&q.0));
            let mut lower: Vec<&(f64, f64, Expr)> = parts
                .iter()
                .filter(|p| !tiny(p.1, p.0) && p.1 < 0.0)
                .collect();
            let mut upper: Vec<&(f64, f64, Expr)> = parts
                .iter()
                .filter(|p| !tiny(p.1, p.0) && p.1 > 0.0)
                .collect();
            lower.sort_by(|p, q| p.0.total_cmp(&q.0).then(q.1.total_cmp(&p.1)));
            let mut out = Vec::new();
            for low in lower {
                let near = |u: &&(f64, f64, Expr)| {
                    (u.0 - low.0).abs() <= 1e-9 * (1.0 + low.0.abs())
                        && (u.1 + low.1).abs() <= 1e-9 * (1.0 + low.1.abs())
                };
                let at = upper
                    .iter()
                    .position(near)
                    .ok_or_else(|| format!("cplxpair: {} has no conjugate in the list", low.2))?;
                out.push(low.2.clone());
                out.push(upper.remove(at).2.clone());
            }
            if let Some(odd) = upper.first() {
                return Err(format!("cplxpair: {} has no conjugate in the list", odd.2));
            }
            out.extend(reals.into_iter().map(|p| p.1));
            Expr::list(out)
        }
        ("regress", 2) => mldivide(&a[1], &a[0])?,
        ("correlation", 3 | 4) => {
            let kind = word(&a[n - 1]).unwrap_or_default();
            if n == 4 && word(&a[2]).as_deref() != Some("type") {
                return Err("corr(x, y, \"type\", \"spearman\")".into());
            }
            let (x, y) = (
                a[0].as_list().ok_or("corr(x, y, kind)")?,
                a[1].as_list().ok_or("corr(x, y, kind)")?,
            );
            if x.len() != y.len() || x.len() < 2 {
                return Err("corr: two lists of one length, two entries or more".into());
            }
            let pearson = |p: &Expr, q: &Expr| {
                crate::stats::call("correlation", &[p.clone(), q.clone()])
                    .unwrap_or_else(|| Err("no correlation".into()))
            };
            let ranks = |v: &Expr| {
                inner("tiedrank", std::slice::from_ref(v)).map(|r| r.unwrap_or_else(Expr::zero))
            };
            match kind.as_str() {
                "pearson" => pearson(&a[0], &a[1])?,
                "spearman" => pearson(&ranks(&a[0])?, &ranks(&a[1])?)?,
                "kendall" => {
                    let sign = |p: &Expr, q: &Expr| order(p, q).map(|o| o as i64);
                    let (mut score, mut tied_x, mut tied_y) = (0i64, 0i64, 0i64);
                    for i in 0..x.len() {
                        for j in i + 1..x.len() {
                            let (sx, sy) = (sign(&x[i], &x[j])?, sign(&y[i], &y[j])?);
                            score += sx * sy;
                            tied_x += i64::from(sx == 0);
                            tied_y += i64::from(sy == 0);
                        }
                    }
                    let pairs = (x.len() * (x.len() - 1) / 2) as i64;
                    tidy(div(
                        Expr::int(score),
                        pow(Expr::int((pairs - tied_x) * (pairs - tied_y)), Expr::half()),
                    ))
                }
                _ => {
                    return Err(
                        "corr(x, y, kind): the kinds are \"pearson\", \"spearman\", \"kendall\""
                            .into(),
                    );
                }
            }
        }
        _ => return Ok(None),
    }))
}

pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    inner(name, a).transpose()
}
