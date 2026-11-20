//! More of what MATLAB has: arrays of more than two sizes, matrices written by the places of
//! their entries, data with gaps, the kinds of values, and small things with names of their own.

use crate::arrays::{Arr, arr, tidy, whole, word};
use crate::canon::{add, div, mul, neg, pow, sub};
use crate::expr::Expr;
use crate::funcs::func;
use crate::matrix::{self, Rows};
use crate::numeric;
use crate::texts::text;
use rug::Float;
use std::cmp::Ordering;

type Res = Result<Expr, String>;

/// The sizes of a list of lists, as far down as all entries are lists of one length.
fn sizes(e: &Expr) -> Vec<usize> {
    let Some(items) = e.as_list() else {
        return Vec::new();
    };
    let mut out = vec![items.len()];
    let inner: Vec<Vec<usize>> = items.iter().map(sizes).collect();
    if let Some(first) = inner.first()
        && !first.is_empty()
        && inner.iter().all(|d| d == first)
    {
        out.extend(first.iter().copied());
    }
    out
}

/// The entries of an array with these sizes, the last index running fastest.
fn entries(e: &Expr, depth: usize, out: &mut Vec<Expr>) {
    match e.as_list() {
        Some(items) if depth > 0 => items.iter().for_each(|x| entries(x, depth - 1, out)),
        _ => out.push(e.clone()),
    }
}

/// The array with these sizes of entries that run with the last index fastest.
fn built(dims: &[usize], data: &[Expr]) -> Expr {
    match dims {
        [] => data.first().cloned().unwrap_or_else(Expr::undefined),
        [_] => Expr::list(data.to_vec()),
        [first, rest @ ..] => {
            let each = data.len() / (*first).max(1);
            Expr::list(
                (0..*first)
                    .map(|k| built(rest, &data[k * each..(k + 1) * each]))
                    .collect(),
            )
        }
    }
}

/// The order of the sizes that `permute` asks for, counted from 0.
fn turns(e: &Expr, count: usize, name: &str) -> Result<Vec<usize>, String> {
    let usage =
        || format!("{name}(array, order): the order names every size once, as [2, 1] or [3, 1, 2]");
    let items = e.as_list().ok_or_else(usage)?;
    let order: Vec<usize> = items
        .iter()
        .map(|x| whole(x, "a place of the order", 1, 64).map(|k| k as usize - 1))
        .collect::<Result<_, _>>()?;
    let mut seen = vec![false; order.len()];
    for k in &order {
        if *k >= order.len() || seen[*k] {
            return Err(usage());
        }
        seen[*k] = true;
    }
    if order.len() < count {
        return Err(format!(
            "{name}: the array has {count} sizes, the order names {}",
            order.len()
        ));
    }
    Ok(order)
}

fn permute(e: &Expr, order: &[usize]) -> Res {
    let mut dims = sizes(e);
    // (a flat list is one row, and sizes that the order names beyond those there are 1)
    if dims.len() == 1 {
        dims.insert(0, 1);
    }
    let depth = dims.len();
    while dims.len() < order.len() {
        dims.push(1);
    }
    let mut data = Vec::new();
    entries(e, if sizes(e).len() == 1 { 1 } else { depth }, &mut data);
    let total: usize = dims.iter().product();
    if data.len() != total {
        return Err("permute needs an array whose lists are equally long".into());
    }
    let turned: Vec<usize> = order.iter().map(|k| dims[*k]).collect();
    let mut out = Vec::with_capacity(total);
    let mut index = vec![0usize; order.len()];
    for _ in 0..total {
        // the entry of the old array whose index `order[r]` is the new index r
        let mut at = 0;
        let mut old = vec![0usize; dims.len()];
        for (r, k) in order.iter().enumerate() {
            old[*k] = index[r];
        }
        for (k, size) in dims.iter().enumerate() {
            at = at * size + old[k];
        }
        out.push(data[at].clone());
        for r in (0..index.len()).rev() {
            index[r] += 1;
            if index[r] < turned[r] {
                break;
            }
            index[r] = 0;
        }
    }
    Ok(built(&turned, &out))
}

/// The entries of a list or of every column (rows with `across`) of a matrix.
fn lists_of(e: &Expr, across: bool) -> (Rows, bool) {
    match arr(e) {
        Arr::Scalar(x) => (vec![vec![x]], false),
        Arr::Vector(v) => (vec![v], false),
        Arr::Matrix(m) if m.len() == 1 => (m, false),
        Arr::Matrix(m) if across => (m, true),
        Arr::Matrix(m) => (matrix::transpose(&m), true),
    }
}

/// A sum, mean and the like of the entries that are not undefined.
fn without_gaps(name: &str, a: &[Expr]) -> Res {
    let base = &name[3..];
    let across = match a.get(1) {
        None => false,
        Some(d) => match d.as_i64() {
            Some(1) => false,
            Some(2) => true,
            _ => {
                return Err(format!(
                    "{name}(x, dimension): the dimension is 1 or 2, not {d}"
                ));
            }
        },
    };
    let (lists, several) = lists_of(&a[0], across);
    let one = |v: &[Expr]| -> Res {
        let kept: Vec<Expr> = v.iter().filter(|x| !x.is_undefined()).cloned().collect();
        if kept.is_empty() {
            return Ok(if base == "sum" {
                Expr::zero()
            } else {
                Expr::undefined()
            });
        }
        crate::datafun::stat(base, &kept)
    };
    if several {
        Ok(Expr::list(
            lists.iter().map(|v| one(v)).collect::<Result<_, _>>()?,
        ))
    } else {
        one(&lists[0])
    }
}

/// The nearest number of 24 bits, as the single numbers of MATLAB are.
fn single(e: &Expr) -> Res {
    crate::arrays::map_result(e, &|x| {
        let v = numeric::evalf_digits(x, numeric::digits() + 10).map_err(|err| err.to_string())?;
        let part = |f: &Float| -> Result<Expr, String> {
            let short = Float::with_val(24, f);
            if !short.is_finite() {
                return Err(format!("single: {x} is no number"));
            }
            Ok(Expr::rational(short.to_rational().unwrap_or_default()))
        };
        let (re, im) = (part(v.real())?, part(v.imag())?);
        Ok(tidy(add(vec![re, mul(vec![Expr::i(), im])])))
    })
}

/// What kind of value this is, by the names of MATLAB.
fn class(e: &Expr) -> &'static str {
    if crate::radix::text_of(e).is_some() {
        return "char";
    }
    if crate::lists::lambda(e).is_some() {
        return "function_handle";
    }
    if crate::records::is_record(e) {
        return "struct";
    }
    if crate::records::is_table(e) {
        return "dictionary";
    }
    match e {
        Expr::Bool(_) => "logical",
        Expr::List(items) => {
            let kinds: Vec<&str> = items.iter().map(class).collect();
            match kinds.first() {
                None => "double",
                Some(first) if kinds.iter().all(|k| k == first) && *first != "char" => first,
                _ if kinds.iter().all(|k| matches!(*k, "double" | "sym")) => "sym",
                _ => "cell",
            }
        }
        other if other.is_const() && !other.is_undefined() => "double",
        other if other.is_undefined() => "double",
        _ => "sym",
    }
}

/// A matrix from the rows, the columns and the values of its entries; entries at one place
/// add up.
fn sparse(a: &[Expr]) -> Res {
    let usage = "sparse(A), sparse(rows, columns), sparse(i, j, values) or sparse(i, j, values, rows, columns)";
    let listed = |e: &Expr| -> Vec<Expr> {
        match arr(e) {
            Arr::Scalar(x) => vec![x],
            Arr::Vector(v) => v,
            Arr::Matrix(m) => matrix::transpose(&m).concat(),
        }
    };
    let (places, values, shape) = match a {
        [m] => return Ok(m.clone()),
        [r, c] => (
            (Vec::new(), Vec::new()),
            Vec::new(),
            Some((
                whole(r, "the rows", 1, 4096)?,
                whole(c, "the columns", 1, 4096)?,
            )),
        ),
        [i, j, v] => ((listed(i), listed(j)), listed(v), None),
        [i, j, v, r, c] => (
            (listed(i), listed(j)),
            listed(v),
            Some((
                whole(r, "the rows", 1, 4096)?,
                whole(c, "the columns", 1, 4096)?,
            )),
        ),
        _ => return Err(usage.to_string()),
    };
    let (is, js) = places;
    if is.len() != js.len() || !(values.len() == is.len() || values.len() == 1) {
        return Err(format!("{usage}: as many rows as columns and values"));
    }
    let at = |e: &Expr| whole(e, "the place of an entry", 1, 4096).map(|k| k as usize);
    let is: Vec<usize> = is.iter().map(at).collect::<Result<_, _>>()?;
    let js: Vec<usize> = js.iter().map(at).collect::<Result<_, _>>()?;
    let (rows, columns) = match shape {
        Some((r, c)) => (r as usize, c as usize),
        None => (
            is.iter().copied().max().ok_or(usage)?,
            js.iter().copied().max().ok_or(usage)?,
        ),
    };
    let mut out = vec![vec![Expr::zero(); columns]; rows];
    for (k, (i, j)) in is.iter().zip(&js).enumerate() {
        if *i > rows || *j > columns {
            return Err(format!(
                "sparse: the entry ({i}, {j}) lies outside {rows} by {columns}"
            ));
        }
        let v = values[if values.len() == 1 { 0 } else { k }].clone();
        out[i - 1][j - 1] = tidy(add(vec![out[i - 1][j - 1].clone(), v]));
    }
    Ok(matrix::from_rows(out))
}

/// The matrix of `rows` by `columns` with the columns of `b` along the diagonals `d`.
fn spdiags(a: &[Expr]) -> Res {
    let usage = "spdiags(B, d, rows, columns): the columns of B along the diagonals d (0 the main one, 1 the one above)";
    let [b, d, r, c] = a else {
        return Err(usage.to_string());
    };
    let (rows, columns) = (
        whole(r, "the rows", 1, 4096)? as usize,
        whole(c, "the columns", 1, 4096)? as usize,
    );
    let diagonals: Vec<i64> = match arr(d) {
        Arr::Scalar(x) => vec![whole(&x, "a diagonal", -4096, 4096)?],
        Arr::Vector(v) => v
            .iter()
            .map(|x| whole(x, "a diagonal", -4096, 4096))
            .collect::<Result<_, _>>()?,
        Arr::Matrix(_) => return Err(usage.to_string()),
    };
    // (a flat list is one column)
    let cols: Rows = match arr(b) {
        Arr::Scalar(x) => vec![vec![x]],
        Arr::Vector(v) => vec![v],
        Arr::Matrix(m) => matrix::transpose(&m),
    };
    if cols.len() != diagonals.len() {
        return Err(format!(
            "{usage}; {} columns and {} diagonals",
            cols.len(),
            diagonals.len()
        ));
    }
    let mut out = vec![vec![Expr::zero(); columns]; rows];
    for (col, k) in cols.iter().zip(&diagonals) {
        for (i, row) in out.iter_mut().enumerate() {
            let j = i as i64 + k;
            if j < 0 || j >= columns as i64 {
                continue;
            }
            // the entries are counted along the columns of a tall matrix, else along the rows
            let at = if rows >= columns { j as usize } else { i };
            if let Some(v) = col.get(at) {
                row[j as usize] = v.clone();
            }
        }
    }
    Ok(matrix::from_rows(out))
}

/// `[numerator, denominator]` of Pade's fraction for `e^(-t*s)`, the highest power first.
fn padecoef(t: &Expr, n: usize) -> Expr {
    use rug::Integer;
    let fact = |k: usize| Integer::from(Integer::factorial(k as u32));
    let c = |k: usize| -> Expr {
        let top = fact(2 * n - k) * fact(n);
        let bottom = fact(2 * n) * fact(k) * fact(n - k);
        Expr::rational(rug::Rational::from((top, bottom)))
    };
    let lead = mul(vec![c(n), pow(t.clone(), Expr::int(n as i64))]);
    let side = |sign: i64| -> Expr {
        Expr::list(
            (0..=n)
                .rev()
                .map(|k| {
                    let power = if k == 0 {
                        Expr::one()
                    } else {
                        pow(mul(vec![Expr::int(sign), t.clone()]), Expr::int(k as i64))
                    };
                    tidy(div(mul(vec![c(k), power]), lead.clone()))
                })
                .collect(),
        )
    };
    Expr::list(vec![side(-1), side(1)])
}

/// The second differences of a list over 4 times the square of the step, the ends on the
/// straight line through their two neighbours.
fn second_differences(v: &[Expr], step: &Expr) -> Vec<Expr> {
    let n = v.len();
    if n < 3 {
        return vec![Expr::zero(); n];
    }
    let scale = mul(vec![Expr::int(4), pow(step.clone(), Expr::int(2))]);
    let mut out = vec![Expr::zero(); n];
    for k in 1..n - 1 {
        out[k] = tidy(div(
            add(vec![
                v[k - 1].clone(),
                mul(vec![Expr::int(-2), v[k].clone()]),
                v[k + 1].clone(),
            ]),
            scale.clone(),
        ));
    }
    if n == 3 {
        out[0] = out[1].clone();
        out[2] = out[1].clone();
    } else {
        out[0] = tidy(sub(mul(vec![Expr::int(2), out[1].clone()]), out[2].clone()));
        out[n - 1] = tidy(sub(
            mul(vec![Expr::int(2), out[n - 2].clone()]),
            out[n - 3].clone(),
        ));
    }
    out
}

fn del2(u: &Expr, step: &Expr) -> Res {
    Ok(match arr(u) {
        Arr::Scalar(_) => Expr::zero(),
        Arr::Vector(v) => Expr::list(second_differences(&v, step)),
        Arr::Matrix(m) if m.len() == 1 => matrix::from_rows(vec![second_differences(&m[0], step)]),
        Arr::Matrix(m) if m[0].len() == 1 => {
            let column: Vec<Expr> = m.iter().map(|r| r[0].clone()).collect();
            matrix::from_rows(
                second_differences(&column, step)
                    .into_iter()
                    .map(|x| vec![x])
                    .collect(),
            )
        }
        Arr::Matrix(m) => {
            let along: Rows = m.iter().map(|r| second_differences(r, step)).collect();
            let down = matrix::transpose(
                &matrix::transpose(&m)
                    .iter()
                    .map(|c| second_differences(c, step))
                    .collect(),
            );
            matrix::from_rows(
                along
                    .iter()
                    .zip(&down)
                    .map(|(p, q)| {
                        p.iter()
                            .zip(q)
                            .map(|(x, y)| tidy(add(vec![x.clone(), y.clone()])))
                            .collect()
                    })
                    .collect(),
            )
        }
    })
}

/// A list with `n` entries whose transform has the waves of the transform of `x`.
fn interpft(x: &[Expr], n: usize) -> Res {
    let m = x.len();
    if m == 0 || n < m {
        return Err(format!(
            "interpft(x, n): n is the number of entries wanted, {m} or more"
        ));
    }
    let transform = |name: &str, v: Vec<Expr>| -> Result<Vec<Expr>, String> {
        let out = crate::signal::call(name, &[Expr::list(v)])
            .unwrap_or_else(|| Err(format!("{name} is not known")))?;
        Ok(out.as_list().map(<[Expr]>::to_vec).unwrap_or_default())
    };
    let waves = transform("fft", x.to_vec())?;
    let middle = (m + 1).div_ceil(2);
    let mut wide: Vec<Expr> = waves[..middle].to_vec();
    wide.extend(vec![Expr::zero(); n - m]);
    wide.extend(waves[middle..].iter().cloned());
    if m.is_multiple_of(2) && n > m {
        let half = tidy(div(wide[middle - 1].clone(), Expr::int(2)));
        wide[middle - 1] = half.clone();
        wide[middle - 1 + n - m] = half;
    }
    let scale = Expr::rat(n as i64, m as i64);
    let real = x.iter().all(crate::assume::is_real);
    Ok(Expr::list(
        transform("ifft", wide)?
            .into_iter()
            .map(|v| {
                let v = tidy(mul(vec![scale.clone(), v]));
                if real { tidy(func("re", vec![v])) } else { v }
            })
            .collect(),
    ))
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    let through = |module: Option<Res>, what: &str| -> Res {
        module.unwrap_or_else(|| Err(format!("{name}: {what} is not known")))
    };
    Ok(Some(match (name, n) {
        ("condest", 1) => through(
            crate::decomp::call("cond", &[a[0].clone(), Expr::one()]),
            "cond",
        )?,
        ("normest", 1) => through(crate::decomp::call("norm", &[a[0].clone()]), "norm")?,
        ("filter2", 2 | 3) => {
            let turned = through(
                crate::arrays::call("rot90", &[a[0].clone(), Expr::int(2)]),
                "rot90",
            )?;
            let shape = a.get(2).cloned().unwrap_or_else(|| text("same"));
            through(
                crate::signal::call("conv2", &[a[1].clone(), turned, shape]),
                "conv2",
            )?
        }
        ("newline", 0) => text("\n"),
        ("meanabs", 1) => {
            let (lists, _) = lists_of(&a[0], false);
            let all: Vec<Expr> = lists.concat();
            if all.is_empty() {
                Expr::undefined()
            } else {
                tidy(div(
                    add(all.iter().map(|x| func("abs", vec![x.clone()])).collect()),
                    Expr::int(all.len() as i64),
                ))
            }
        }
        ("nansum" | "nanmean" | "nanmax" | "nanmin" | "nanstd" | "nanvar" | "nanmedian", 1 | 2) => {
            without_gaps(name, a)?
        }
        ("single", 1) => single(&a[0])?,
        ("cast", 2) => {
            let kind = word(&a[1]).ok_or("cast(x, \"int8\"): the kind is a text")?;
            let x = std::slice::from_ref(&a[0]);
            match kind.as_str() {
                "int8" | "int16" | "int32" | "int64" | "uint8" | "uint16" | "uint32" | "uint64" => {
                    through(crate::elem::call(&kind, x), &kind)?
                }
                "single" => single(&a[0])?,
                "double" | "logical" | "char" => through(crate::texts::call(&kind, x), &kind)?,
                _ => {
                    return Err(format!(
                        "cast: the kinds are int8 to uint64, single, double, logical and char, not {kind}"
                    ));
                }
            }
        }
        ("class", 1) => text(class(&a[0])),
        ("isa", 2) => {
            let kind = word(&a[1]).ok_or("isa(x, \"double\"): the kind is a text")?;
            let is = class(&a[0]);
            Expr::Bool(match kind.as_str() {
                "numeric" | "float" => is == "double",
                "string" => is == "char",
                other => is == other,
            })
        }
        ("sparse", 1 | 2 | 3 | 5) => sparse(a)?,
        ("full", 1) => a[0].clone(),
        ("issparse", 1) => Expr::Bool(false),
        ("nzmax", 1) => through(crate::arrays::call("nnz", a), "nnz")?,
        ("spones", 1) => crate::arrays::map(&a[0], &|x| {
            Expr::int(i64::from(!x.is_zero() && !crate::simplify::is_zero(x)))
        }),
        ("speye", 1 | 2) => {
            let rows = whole(&a[0], "the size", 1, 4096)? as usize;
            let columns = match a.get(1) {
                Some(c) => whole(c, "the size", 1, 4096)? as usize,
                None => rows,
            };
            matrix::from_rows(
                (0..rows)
                    .map(|i| (0..columns).map(|j| Expr::int(i64::from(i == j))).collect())
                    .collect(),
            )
        }
        ("spdiags", 4) => spdiags(a)?,
        ("squeeze", 1) => {
            let dims = sizes(&a[0]);
            if dims.len() <= 2 {
                a[0].clone()
            } else {
                let mut data = Vec::new();
                entries(&a[0], dims.len(), &mut data);
                let kept: Vec<usize> = dims.iter().copied().filter(|d| *d != 1).collect();
                built(&kept, &data)
            }
        }
        ("permute", 2) => {
            let order = turns(&a[1], sizes(&a[0]).len().max(2), name)?;
            permute(&a[0], &order)?
        }
        ("ipermute", 2) => {
            let order = turns(&a[1], sizes(&a[0]).len().max(2), name)?;
            let mut back = vec![0; order.len()];
            for (r, k) in order.iter().enumerate() {
                back[*k] = r;
            }
            permute(&a[0], &back)?
        }
        ("num2cell", 1) => a[0].clone(),
        ("cell2mat", 1) => {
            let items = a[0].as_list().ok_or("cell2mat needs a list of matrices")?;
            let joined = |parts: &[Expr], which: &str| -> Res {
                through(crate::arrays::call(which, parts), which)
            };
            let blocks = |e: &Expr| {
                e.as_list()
                    .is_some_and(|l| l.iter().all(|x| matrix::rows(x).is_some()))
            };
            if items.iter().all(blocks) {
                let rows: Vec<Expr> = items
                    .iter()
                    .map(|r| joined(r.as_list().unwrap_or_default(), "horzcat"))
                    .collect::<Result<_, _>>()?;
                joined(&rows, "vertcat")?
            } else {
                joined(items, "horzcat")?
            }
        }
        ("signim", 1) => crate::arrays::map_result(&a[0], &|z| {
            let v = numeric::evalf_digits(z, numeric::digits() + 10).map_err(|e| e.to_string())?;
            let (re, im) = (v.real(), v.imag());
            let sign = match im.cmp0() {
                Some(Ordering::Greater) => 1,
                Some(Ordering::Less) => -1,
                _ => match re.cmp0() {
                    Some(Ordering::Greater) => -1,
                    Some(Ordering::Less) => 1,
                    _ => 0,
                },
            };
            Ok(Expr::int(sign))
        })?,
        ("padecoef", 1 | 2) => {
            let order = match a.get(1) {
                Some(k) => whole(k, "the order", 0, 60)? as usize,
                None => 1,
            };
            padecoef(&a[0], order)
        }
        ("crosstab", 2) => {
            let (x, y) = (
                a[0].as_list().ok_or("crosstab needs two lists")?,
                a[1].as_list().ok_or("crosstab needs two lists")?,
            );
            if x.len() != y.len() {
                return Err("crosstab: as many entries in one list as in the other".into());
            }
            let (kinds_x, kinds_y) = (
                crate::lists::sorted(&distinct(x)),
                crate::lists::sorted(&distinct(y)),
            );
            matrix::from_rows(
                kinds_x
                    .iter()
                    .map(|p| {
                        kinds_y
                            .iter()
                            .map(|q| {
                                let count =
                                    x.iter().zip(y).filter(|(u, v)| *u == p && *v == q).count();
                                Expr::int(count as i64)
                            })
                            .collect()
                    })
                    .collect(),
            )
        }
        ("pdist", 1) => {
            let m = matrix::rows(&a[0]).ok_or("pdist needs a matrix with a point in every row")?;
            let mut out = Vec::new();
            for i in 0..m.len() {
                for j in i + 1..m.len() {
                    let squares: Vec<Expr> = m[i]
                        .iter()
                        .zip(&m[j])
                        .map(|(p, q)| {
                            pow(func("abs", vec![sub(p.clone(), q.clone())]), Expr::int(2))
                        })
                        .collect();
                    out.push(tidy(pow(add(squares), Expr::half())));
                }
            }
            Expr::list(out)
        }
        ("squareform", 1) => match arr(&a[0]) {
            Arr::Matrix(m) if m.len() == m[0].len() => {
                let mut out = Vec::new();
                for i in 0..m.len() {
                    out.extend(m[i][i + 1..].iter().cloned());
                }
                Expr::list(out)
            }
            Arr::Vector(v) => {
                // n*(n - 1)/2 distances are those of n points
                let count = (1..=v.len() + 1)
                    .find(|k| k * (k - 1) / 2 == v.len())
                    .ok_or("squareform: a list of n*(n - 1)/2 distances")?;
                let mut out = vec![vec![Expr::zero(); count]; count];
                let places = (0..count).flat_map(|i| (i + 1..count).map(move |j| (i, j)));
                for ((i, j), d) in places.zip(&v) {
                    out[i][j] = d.clone();
                    out[j][i] = d.clone();
                }
                matrix::from_rows(out)
            }
            _ => return Err("squareform needs a list of distances or a square matrix".into()),
        },
        ("normfit" | "expfit", 1) => {
            let data = a[0]
                .as_list()
                .filter(|v| !v.is_empty() && v.iter().all(|x| x.as_list().is_none()))
                .ok_or_else(|| format!("{name} needs a list of numbers"))?;
            let mean = crate::datafun::stat("mean", data)?;
            if name == "expfit" {
                mean
            } else {
                Expr::list(vec![mean, crate::datafun::stat("std", data)?])
            }
        }
        ("partialcorr", 1) => {
            let r = through(crate::datafun::call("corrcoef", a), "corrcoef")?;
            let r = matrix::rows(&r)
                .ok_or("partialcorr needs a matrix with a variable in every column")?;
            let p = matrix::inverse(&r)
                .map_err(|_| "partialcorr: the columns depend on each other".to_string())?;
            let size = p.len();
            matrix::from_rows(
                (0..size)
                    .map(|i| {
                        (0..size)
                            .map(|j| {
                                if i == j {
                                    Expr::one()
                                } else {
                                    tidy(neg(div(
                                        p[i][j].clone(),
                                        pow(
                                            mul(vec![p[i][i].clone(), p[j][j].clone()]),
                                            Expr::half(),
                                        ),
                                    )))
                                }
                            })
                            .collect()
                    })
                    .collect(),
            )
        }
        ("del2", 1 | 2) => del2(&a[0], &a.get(1).cloned().unwrap_or_else(Expr::one))?,
        ("interpft", 2) => {
            let x = a[0].as_list().ok_or("interpft needs a list")?;
            interpft(
                x,
                whole(&a[1], "the number of entries", 1, 1 << 20)? as usize,
            )?
        }
        _ => return Ok(None),
    }))
}

/// The entries of a list without the ones that came before.
fn distinct(items: &[Expr]) -> Vec<Expr> {
    let mut out: Vec<Expr> = Vec::new();
    for x in items {
        if !out.contains(x) {
            out.push(x.clone());
        }
    }
    out
}

pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    inner(name, a).transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(v: &[i64]) -> Expr {
        Expr::list(v.iter().map(|k| Expr::int(*k)).collect())
    }

    #[test]
    fn arrays_of_three_sizes_turn() {
        // 2 by 3 by 2, the entries 1 to 12 with the last index fastest
        let data: Vec<Expr> = (1..=12).map(Expr::int).collect();
        let cube = built(&[2, 3, 2], &data);
        assert_eq!(sizes(&cube), [2, 3, 2]);
        let turned = permute(&cube, &[2, 0, 1]).expect("an order");
        assert_eq!(sizes(&turned), [2, 2, 3]);
        // the entry (k, i, j) of the turned array is the entry (i, j, k) of the cube
        let mut flat = Vec::new();
        entries(&turned, 3, &mut flat);
        for k in 0..2 {
            for i in 0..2 {
                for j in 0..3 {
                    let want = (i * 3 + j) * 2 + k + 1;
                    assert_eq!(flat[(k * 2 + i) * 3 + j], Expr::int(want as i64));
                }
            }
        }
        // and back again
        let back = permute(&turned, &[1, 2, 0]).expect("an order");
        assert_eq!(back, cube);
        // a flat list is a row: turned it is a column
        assert_eq!(
            permute(&list(&[1, 2, 3]), &[1, 0]).expect("an order"),
            Expr::list(vec![list(&[1]), list(&[2]), list(&[3])])
        );
    }

    #[test]
    fn matrices_by_their_places() {
        let m = sparse(&[list(&[1, 2, 2]), list(&[1, 3, 3]), list(&[5, 6, 7])]).expect("a matrix");
        assert_eq!(m, Expr::list(vec![list(&[5, 0, 0]), list(&[0, 0, 13])]));
        assert!(
            sparse(&[
                list(&[3]),
                list(&[1]),
                list(&[1]),
                Expr::int(2),
                Expr::int(2)
            ])
            .is_err()
        );
        // the columns of B along the diagonals of a square matrix, counted by its columns
        let b = Expr::list(vec![list(&[1, 4]), list(&[2, 5]), list(&[3, 6])]);
        let m = spdiags(&[b, list(&[0, 1]), Expr::int(3), Expr::int(3)]).expect("a matrix");
        assert_eq!(
            m,
            Expr::list(vec![list(&[1, 5, 0]), list(&[0, 2, 6]), list(&[0, 0, 3])])
        );
    }

    #[test]
    fn kinds_of_values() {
        assert_eq!(class(&Expr::int(3)), "double");
        assert_eq!(class(&list(&[1, 2])), "double");
        assert_eq!(class(&Expr::Bool(true)), "logical");
        assert_eq!(class(&text("a")), "char");
        assert_eq!(class(&Expr::sym("x")), "sym");
        assert_eq!(
            class(&Expr::list(vec![Expr::int(1), Expr::sym("x")])),
            "sym"
        );
        assert_eq!(class(&Expr::list(vec![Expr::int(1), text("a")])), "cell");
    }

    #[test]
    fn fractions_of_pade() {
        // e^(-s) is about (2 - s)/(2 + s), and (s^2 - 6s + 12)/(s^2 + 6s + 12)
        assert_eq!(
            padecoef(&Expr::one(), 1),
            Expr::list(vec![list(&[-1, 2]), list(&[1, 2])])
        );
        assert_eq!(
            padecoef(&Expr::one(), 2),
            Expr::list(vec![list(&[1, -6, 12]), list(&[1, 6, 12])])
        );
    }

    #[test]
    fn second_differences_of_squares() {
        // of x^2 the second difference is 2 everywhere: 2/4 with the ends on the line
        let squares: Vec<Expr> = (0..6).map(|k| Expr::int(k * k)).collect();
        assert_eq!(
            second_differences(&squares, &Expr::one()),
            vec![Expr::half(); 6]
        );
        assert_eq!(
            second_differences(&squares[..2], &Expr::one()),
            vec![Expr::zero(); 2]
        );
    }
}
