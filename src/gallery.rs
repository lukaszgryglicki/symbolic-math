//! Matrices with names (the gallery of MATLAB), its surface with three peaks, the colours of
//! a palette as numbers, and counts of pairs in the cells of a grid.

use crate::arrays::{order, tidy, whole, word};
use crate::canon::{add, div, mul, neg, pow, sub};
use crate::expr::Expr;
use crate::funcs::func;
use crate::matrix;
use std::cmp::Ordering;

type Res = Result<Expr, String>;

const KINDS: &str = "\"minij\", \"moler\", \"lehmer\", \"fiedler\", \"frank\", \"tridiag\", \
    \"ris\", \"kms\", \"parter\", \"pei\", \"clement\", \"grcar\", \"jordbloc\", \"circul\", \
    \"cauchy\" and \"lotkin\"";

/// A square matrix whose entry is a function of its row and column, both from 1.
fn filled(n: usize, f: &dyn Fn(i64, i64) -> Expr) -> Expr {
    matrix::from_rows(
        (1..=n as i64)
            .map(|i| (1..=n as i64).map(|j| tidy(f(i, j))).collect())
            .collect(),
    )
}

fn gallery(kind: &str, a: &[Expr]) -> Res {
    let size = |k: usize| -> Result<usize, String> {
        let e = a
            .get(k)
            .ok_or_else(|| format!("gallery(\"{kind}\", n) needs the size"))?;
        Ok(whole(e, "the size", 1, 2000)? as usize)
    };
    let other = |k: usize, otherwise: Expr| a.get(k).cloned().unwrap_or(otherwise);
    let list = |k: usize| -> Result<Vec<Expr>, String> {
        match a.get(k) {
            Some(e) => match (e.as_list(), e.as_int().and_then(rug::Integer::to_i64)) {
                (Some(items), _) => Ok(items.to_vec()),
                // (a size alone stands for 1, 2, ..., n)
                (None, Some(n)) if (1..=2000).contains(&n) => Ok((1..=n).map(Expr::int).collect()),
                _ => Err(format!(
                    "gallery(\"{kind}\", list): a list of numbers or a size"
                )),
            },
            None => Err(format!("gallery(\"{kind}\", list) needs a list")),
        }
    };
    Ok(match kind {
        "minij" => filled(size(0)?, &|i, j| Expr::int(i.min(j))),
        "moler" => {
            let alpha = other(1, Expr::int(-1));
            filled(size(0)?, &|i, j| match i == j {
                true => add(vec![
                    mul(vec![Expr::int(i - 1), pow(alpha.clone(), Expr::int(2))]),
                    Expr::one(),
                ]),
                false => add(vec![
                    mul(vec![
                        Expr::int(i.min(j) - 1),
                        pow(alpha.clone(), Expr::int(2)),
                    ]),
                    alpha.clone(),
                ]),
            })
        }
        "lehmer" => filled(size(0)?, &|i, j| Expr::rat(i.min(j), i.max(j))),
        "fiedler" => {
            let c = list(0)?;
            filled(c.len(), &|i, j| {
                func(
                    "abs",
                    vec![sub(c[i as usize - 1].clone(), c[j as usize - 1].clone())],
                )
            })
        }
        "frank" => {
            let n = size(0)? as i64;
            let flipped = a.get(1).is_some_and(|k| !k.is_zero());
            let entry = |i: i64, j: i64| match () {
                () if j >= i => n - j + 1,
                () if j == i - 1 => n - j,
                () => 0,
            };
            // (the other form is the same matrix seen from its far corner)
            filled(n as usize, &|i, j| {
                Expr::int(match flipped {
                    true => entry(n + 1 - j, n + 1 - i),
                    false => entry(i, j),
                })
            })
        }
        "tridiag" => {
            let (below, on, above) = (
                other(1, Expr::int(-1)),
                other(2, Expr::int(2)),
                other(3, Expr::int(-1)),
            );
            filled(size(0)?, &|i, j| match j - i {
                -1 => below.clone(),
                0 => on.clone(),
                1 => above.clone(),
                _ => Expr::zero(),
            })
        }
        "ris" => {
            let n = size(0)? as i64;
            filled(n as usize, &|i, j| Expr::rat(1, 2 * (n - i - j) + 3))
        }
        "kms" => {
            let rho = other(1, Expr::half());
            filled(size(0)?, &|i, j| pow(rho.clone(), Expr::int((i - j).abs())))
        }
        "parter" => filled(size(0)?, &|i, j| Expr::rat(2, 2 * (i - j) + 1)),
        "pei" => {
            let alpha = other(1, Expr::one());
            filled(size(0)?, &|i, j| match i == j {
                true => add(vec![alpha.clone(), Expr::one()]),
                false => Expr::one(),
            })
        }
        "clement" => {
            let n = size(0)? as i64;
            let symmetric = a.get(1).is_some_and(|k| !k.is_zero());
            filled(n as usize, &|i, j| match (j - i, symmetric) {
                (1, false) => Expr::int(i),
                (-1, false) => Expr::int(n - j),
                (1 | -1, true) => {
                    let k = i.min(j);
                    pow(Expr::int(k * (n - k)), Expr::half())
                }
                _ => Expr::zero(),
            })
        }
        "grcar" => {
            let k = match a.get(1) {
                Some(k) => whole(k, "the number of diagonals above", 0, 2000)?,
                None => 3,
            };
            filled(size(0)?, &|i, j| {
                Expr::int(match j - i {
                    -1 => -1,
                    d if (0..=k).contains(&d) => 1,
                    _ => 0,
                })
            })
        }
        "jordbloc" => {
            let lambda = other(1, Expr::one());
            filled(size(0)?, &|i, j| match j - i {
                0 => lambda.clone(),
                1 => Expr::one(),
                _ => Expr::zero(),
            })
        }
        "circul" => {
            let v = list(0)?;
            let n = v.len() as i64;
            filled(v.len(), &|i, j| v[(j - i).rem_euclid(n) as usize].clone())
        }
        "cauchy" => {
            let x = list(0)?;
            let y = match a.get(1) {
                Some(_) => list(1)?,
                None => x.clone(),
            };
            if x.len() != y.len() {
                return Err("gallery(\"cauchy\", x, y): two lists of one length".into());
            }
            filled(x.len(), &|i, j| {
                div(
                    Expr::one(),
                    add(vec![x[i as usize - 1].clone(), y[j as usize - 1].clone()]),
                )
            })
        }
        "lotkin" => filled(size(0)?, &|i, j| match i {
            1 => Expr::one(),
            _ => Expr::rat(1, i + j - 1),
        }),
        _ => return Err(format!("gallery(kind, ...): the kind is one of {KINDS}")),
    })
}

/// The surface with three peaks that MATLAB draws to show its plots.
fn peak(x: &Expr, y: &Expr) -> Expr {
    let square = |e: Expr| pow(e, Expr::int(2));
    let bell = |x: Expr, y: Expr| func("exp", vec![neg(add(vec![square(x), square(y)]))]);
    let one = Expr::one;
    add(vec![
        mul(vec![
            Expr::int(3),
            square(sub(one(), x.clone())),
            bell(x.clone(), add(vec![y.clone(), one()])),
        ]),
        mul(vec![
            Expr::int(-10),
            add(vec![
                div(x.clone(), Expr::int(5)),
                neg(pow(x.clone(), Expr::int(3))),
                neg(pow(y.clone(), Expr::int(5))),
            ]),
            bell(x.clone(), y.clone()),
        ]),
        mul(vec![
            Expr::rat(-1, 3),
            bell(add(vec![x.clone(), one()]), y.clone()),
        ]),
    ])
}

/// The colours of a palette at so many places, as rows [red, green, blue] from 0 to 1.
fn colours(name: &str, count: usize) -> Res {
    let stops = crate::raster::PALETTES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, stops)| *stops)
        .ok_or_else(|| {
            let all: Vec<&str> = crate::raster::PALETTES.iter().map(|(n, _)| *n).collect();
            format!("colormap(name, count): the palettes are {}", all.join(", "))
        })?;
    let byte = |c: f32| (c * 255.0).round() as i64;
    let last = stops.len() as i64 - 1;
    Ok(Expr::list(
        (0..count as i64)
            .map(|k| {
                // the place k/(count - 1) of the way along the stops
                let (num, den) = match count {
                    1 => (0, 1),
                    _ => (k * last, count as i64 - 1),
                };
                let (from, part) = (
                    (num / den).min(last - 1).max(0),
                    num - (num / den).min(last - 1).max(0) * den,
                );
                let (lo, hi) = (stops[from as usize], stops[(from + 1).min(last) as usize]);
                Expr::list(
                    (0..3)
                        .map(|c| {
                            let (p, q) = (byte(lo[c]), byte(hi[c]));
                            Expr::rat(p * den + (q - p) * part, 255 * den)
                        })
                        .collect(),
                )
            })
            .collect(),
    ))
}

/// The edges of so many cells of one width from the least to the greatest value.
fn edges(v: &[Expr], spec: &Expr, name: &str) -> Result<Vec<Expr>, String> {
    if let Some(given) = spec.as_list() {
        return match given.len() >= 2 {
            true => Ok(given.to_vec()),
            false => Err(format!("{name}: two edges or more")),
        };
    }
    let cells = whole(spec, "the number of cells", 1, 100_000)?;
    let (lo, hi) = (
        crate::datafun::stat("min", v)?,
        crate::datafun::stat("max", v)?,
    );
    let width = sub(hi.clone(), lo.clone());
    Ok((0..=cells)
        .map(|k| match k == cells {
            true => hi.clone(),
            false => tidy(add(vec![
                lo.clone(),
                mul(vec![Expr::rat(k, cells), width.clone()]),
            ])),
        })
        .collect())
}

/// The cell of a value: from an edge up to the next, the last cell with its far edge.
fn cell_of(x: &Expr, edges: &[Expr]) -> Result<Option<usize>, String> {
    let last = edges.len() - 1;
    if order(x, &edges[0])? == Ordering::Less || order(x, &edges[last])? == Ordering::Greater {
        return Ok(None);
    }
    for (k, edge) in edges.iter().enumerate().take(last).skip(1) {
        if order(x, edge)? == Ordering::Less {
            return Ok(Some(k - 1));
        }
    }
    Ok(Some(last - 1))
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    Ok(Some(match (name, n) {
        ("gallery", 2..) => {
            let kind = word(&a[0]).ok_or("gallery(kind, ...) needs the kind in quotes")?;
            gallery(&kind.to_lowercase(), &a[1..])?
        }
        ("peaks", 0 | 1) => {
            let count = match a.first() {
                Some(c) => whole(c, "the size", 2, 1000)?,
                None => 49,
            };
            let at = |k: i64| Expr::rat(6 * k - 3 * (count - 1), count - 1);
            let digits = crate::numeric::digits();
            Expr::list(
                (0..count)
                    .map(|i| {
                        Expr::list(
                            (0..count)
                                .map(|j| crate::numeric::n(&peak(&at(j), &at(i)), digits))
                                .collect(),
                        )
                    })
                    .collect(),
            )
        }
        ("peaks", 2) => tidy(peak(&a[0], &a[1])),
        ("colormap", 1 | 2) => {
            let palette =
                word(&a[0]).ok_or("colormap(name, count) needs the name of a palette in quotes")?;
            let count = match a.get(1) {
                Some(c) => whole(c, "the number of colours", 1, 65_536)? as usize,
                None => 256,
            };
            colours(&palette.to_lowercase(), count)?
        }
        ("histcounts2", 3 | 4) => {
            let usage = "histcounts2(x, y, cells) or histcounts2(x, y, [cells of x, cells of y]) or histcounts2(x, y, edges of x, edges of y)";
            let (x, y) = (a[0].as_list().ok_or(usage)?, a[1].as_list().ok_or(usage)?);
            if x.len() != y.len() || x.is_empty() {
                return Err("histcounts2 needs two lists of one length".into());
            }
            let (ex, ey) = match (n, a[2].as_list()) {
                (4, _) => (edges(x, &a[2], name)?, edges(y, &a[3], name)?),
                (_, Some([cx, cy])) => (edges(x, cx, name)?, edges(y, cy, name)?),
                (_, None) => (edges(x, &a[2], name)?, edges(y, &a[2], name)?),
                _ => return Err(usage.into()),
            };
            let mut counts = vec![vec![0i64; ey.len() - 1]; ex.len() - 1];
            for (p, q) in x.iter().zip(y) {
                if let (Some(i), Some(j)) = (cell_of(p, &ex)?, cell_of(q, &ey)?) {
                    counts[i][j] += 1;
                }
            }
            Expr::list(
                counts
                    .into_iter()
                    .map(|row| Expr::list(row.into_iter().map(Expr::int).collect()))
                    .collect(),
            )
        }
        _ => return Ok(None),
    }))
}

pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    inner(name, a).transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palettes_are_fractions_of_their_stops() {
        for (name, stops) in crate::raster::PALETTES {
            // the ends are the first and the last stop, whatever the count
            let end = |stop: [f32; 3]| {
                Expr::list(
                    stop.iter()
                        .map(|c| Expr::rat((c * 255.0).round() as i64, 255))
                        .collect(),
                )
            };
            for count in [2, 3, 7, 64] {
                let all = colours(name, count).expect("colours");
                let rows = all.as_list().expect("rows");
                assert_eq!(rows.len(), count, "{name}");
                assert_eq!(rows[0], end(stops[0]), "{name}");
                assert_eq!(rows[count - 1], end(stops[stops.len() - 1]), "{name}");
            }
            let one = colours(name, 1).expect("colours");
            assert_eq!(one, Expr::list(vec![end(stops[0])]), "{name}");
        }
        assert!(colours("no such palette", 3).is_err());
    }

    #[test]
    fn cells_hold_their_near_edge_and_the_last_its_far_one() {
        let edges: Vec<Expr> = [0, 1, 2].into_iter().map(Expr::int).collect();
        let cell = |p: i64, q: i64| cell_of(&Expr::rat(p, q), &edges);
        assert_eq!(cell(0, 1), Ok(Some(0)));
        assert_eq!(cell(1, 1), Ok(Some(1)));
        assert_eq!(cell(2, 1), Ok(Some(1)));
        assert_eq!(cell(1, 2), Ok(Some(0)));
        assert_eq!(cell(-1, 2), Ok(None));
        assert_eq!(cell(5, 2), Ok(None));
    }
}
