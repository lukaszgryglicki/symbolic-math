//! Pictures as matrices of numbers: a gray picture is a matrix, a picture of colours a list
//! of the matrices of its channels. Numbers from 0 to 1 are shares of white, whole numbers
//! above 1 are samples.

use crate::arrays::{order, tidy, whole, word};
use crate::canon::{add, div, mul, sub};
use crate::expr::Expr;
use crate::funcs::func;
use crate::matrix::{self, Rows};
use crate::num::Rat;
use std::cmp::Ordering;

type Res = Result<Expr, String>;

/// The channels of a picture, and whether it was one matrix.
fn planes(e: &Expr, name: &str) -> Result<(Vec<Rows>, bool), String> {
    let plain = |m: &Rows| m.iter().flatten().all(|x| x.as_list().is_none());
    if let Some(m) = matrix::rows(e).filter(plain) {
        return Ok((vec![m], true));
    }
    let all: Option<Vec<Rows>> = e
        .as_list()
        .filter(|l| !l.is_empty())
        .and_then(|l| l.iter().map(|p| matrix::rows(p).filter(plain)).collect());
    match all {
        Some(all)
            if all
                .iter()
                .all(|p| p.len() == all[0].len() && p[0].len() == all[0][0].len()) =>
        {
            Ok((all, false))
        }
        _ => Err(format!(
            "{name} needs a picture: a matrix of numbers, or a list of the matrices of its channels"
        )),
    }
}

fn picture(planes: Vec<Rows>, single: bool) -> Expr {
    let mut all: Vec<Expr> = planes.into_iter().map(matrix::from_rows).collect();
    if single {
        all.remove(0)
    } else {
        Expr::list(all)
    }
}

/// Every channel of a picture changed alike.
fn each(e: &Expr, name: &str, f: &dyn Fn(&Rows) -> Result<Rows, String>) -> Res {
    let (all, single) = planes(e, name)?;
    Ok(picture(
        all.iter().map(f).collect::<Result<_, _>>()?,
        single,
    ))
}

fn number(e: &Expr, name: &str) -> Result<Rat, String> {
    if let Some(r) = e.as_rat() {
        return Ok(r.clone());
    }
    crate::numeric::real_value(e, crate::numeric::digits() + 10)
        .and_then(|f| f.to_rational())
        .ok_or_else(|| format!("{name}: {e} is not a real number"))
}

/// The nearest whole number, halves upward.
fn rounded(x: &Rat) -> i64 {
    Rat::from(x + Rat::from((1, 2)))
        .floor()
        .numer()
        .to_i64()
        .unwrap_or(0)
}

/// Whether the entries are samples: whole numbers, one of them above 1.
fn samples(all: &[Rows]) -> bool {
    let entries = || all.iter().flatten().flatten();
    entries().all(|x| x.as_i64().is_some()) && entries().any(|x| x.as_i64().is_some_and(|v| v > 1))
}

/// The largest sample of a number of bits.
fn top(bits: Option<&Expr>, name: &str) -> Result<i64, String> {
    match bits {
        None => Ok(255),
        Some(b) => Ok((1 << whole(b, &format!("the bits of {name}"), 1, 32)?) - 1),
    }
}

/// Where the middle of a pixel of the new picture lies in the old one, counted from 0.
fn place(k: usize, new: usize, old: usize) -> Rat {
    // (the middles of the pixels divide both pictures alike)
    Rat::from((2 * k as i64 + 1, 2)) * Rat::from((old as i64, new as i64)) - Rat::from((1, 2))
}

fn resized(m: &Rows, rows: usize, columns: usize, nearest: bool) -> Rows {
    let (old_rows, old_columns) = (m.len(), m[0].len());
    let near = |at: &Rat, size: usize| rounded(at).clamp(0, size as i64 - 1) as usize;
    // the two neighbours of a place and the share of the second
    let between = |at: &Rat, size: usize| -> (usize, usize, Rat) {
        let low = at.clone().floor();
        let share = Rat::from(at - &low);
        let low = low.numer().to_i64().unwrap_or(0);
        let clamp = |v: i64| v.clamp(0, size as i64 - 1) as usize;
        (clamp(low), clamp(low + 1), share)
    };
    (0..rows)
        .map(|i| {
            let y = place(i, rows, old_rows);
            (0..columns)
                .map(|j| {
                    let x = place(j, columns, old_columns);
                    if nearest {
                        return m[near(&y, old_rows)][near(&x, old_columns)].clone();
                    }
                    let (i0, i1, v) = between(&y, old_rows);
                    let (j0, j1, u) = between(&x, old_columns);
                    let mix = |p: &Expr, q: &Expr, share: &Rat| {
                        add(vec![
                            p.clone(),
                            mul(vec![
                                Expr::rational(share.clone()),
                                sub(q.clone(), p.clone()),
                            ]),
                        ])
                    };
                    let upper = mix(&m[i0][j0], &m[i0][j1], &u);
                    let lower = mix(&m[i1][j0], &m[i1][j1], &u);
                    tidy(mix(&upper, &lower, &v))
                })
                .collect()
        })
        .collect()
}

/// A picture turned against the clock about its middle, as large as it has to be; a place
/// that the old picture does not cover is 0.
fn turned(m: &Rows, degrees: &Expr, name: &str) -> Result<Rows, String> {
    let quarters = tidy(div(degrees.clone(), Expr::int(90)));
    if let Some(k) = quarters.as_i64() {
        let mut out = m.clone();
        for _ in 0..k.rem_euclid(4) {
            // a quarter turn: the last column becomes the first row
            out = matrix::transpose(&out).into_iter().rev().collect();
        }
        return Ok(out);
    }
    let angle = number(degrees, name)?.to_f64();
    let (sine, cosine) = crate::scene::sincos(angle);
    let (h, w) = (m.len() as f64, m[0].len() as f64);
    let wide = (w * cosine.abs() + h * sine.abs() - 1e-9).ceil().max(1.0);
    let high = (w * sine.abs() + h * cosine.abs() - 1e-9).ceil().max(1.0);
    let (rows, columns) = (high as usize, wide as usize);
    Ok((0..rows)
        .map(|i| {
            (0..columns)
                .map(|j| {
                    // from the middle of the new pixel back into the old picture (y runs down)
                    let (x, y) = (j as f64 + 0.5 - wide / 2.0, i as f64 + 0.5 - high / 2.0);
                    let (u, v) = (x * cosine - y * sine, x * sine + y * cosine);
                    let (col, row) = ((u + w / 2.0).floor(), (v + h / 2.0).floor());
                    if col >= 0.0 && row >= 0.0 && col < w && row < h {
                        m[row as usize][col as usize].clone()
                    } else {
                        Expr::zero()
                    }
                })
                .collect()
        })
        .collect())
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    Ok(Some(match (name, n) {
        ("rgb2gray", 1) => {
            let (all, _) = planes(&a[0], name)?;
            if all.len() < 3 {
                return Err("rgb2gray needs the three matrices [red, green, blue]".into());
            }
            let whole_numbers = samples(&all);
            let weights = [(2989, 10000), (587, 1000), (114, 1000)];
            matrix::from_rows(
                (0..all[0].len())
                    .map(|i| {
                        (0..all[0][0].len())
                            .map(|j| {
                                let gray = tidy(add((0..3)
                                    .map(|c| {
                                        mul(vec![
                                            Expr::rational(Rat::from(weights[c])),
                                            all[c][i][j].clone(),
                                        ])
                                    })
                                    .collect()));
                                match gray.as_rat() {
                                    Some(r) if whole_numbers => Expr::int(rounded(r)),
                                    _ => gray,
                                }
                            })
                            .collect()
                    })
                    .collect(),
            )
        }
        ("im2double", 1 | 2) => {
            let top = top(a.get(1), name)?;
            crate::arrays::map(&a[0], &|x| tidy(div(x.clone(), Expr::int(top))))
        }
        ("im2uint8" | "im2uint16", 1) => {
            let top = if name == "im2uint8" { 255 } else { 65535 };
            crate::arrays::map_result(&a[0], &|x| {
                let scaled = number(x, name)? * Rat::from(top);
                Ok(Expr::int(rounded(&scaled).clamp(0, top)))
            })?
        }
        ("imcomplement", 1) => {
            let (all, _) = planes(&a[0], name)?;
            let most = if !samples(&all) {
                1
            } else if all
                .iter()
                .flatten()
                .flatten()
                .all(|x| x.as_i64().is_some_and(|v| v <= 255))
            {
                255
            } else {
                65535
            };
            crate::arrays::map(&a[0], &|x| tidy(sub(Expr::int(most), x.clone())))
        }
        ("imcrop", 2) => {
            let usage = "imcrop(picture, [x, y, width, height]): the columns from x to x + width and the rows from y to y + height";
            let Some([x, y, w, h]) = a[1].as_list() else {
                return Err(usage.to_string());
            };
            let (x, y) = (
                whole(x, "x", 1, 1 << 30)? as usize,
                whole(y, "y", 1, 1 << 30)? as usize,
            );
            let (w, h) = (
                whole(w, "the width", 0, 1 << 30)? as usize,
                whole(h, "the height", 0, 1 << 30)? as usize,
            );
            each(&a[0], name, &|m| {
                if y + h > m.len() || x + w > m[0].len() {
                    return Err(format!(
                        "imcrop: the picture has {} rows and {} columns",
                        m.len(),
                        m[0].len()
                    ));
                }
                Ok(m[y - 1..y + h]
                    .iter()
                    .map(|r| r[x - 1..x + w].to_vec())
                    .collect())
            })?
        }
        ("imresize", 2 | 3) => {
            let nearest = match a.get(2).and_then(word).as_deref() {
                None | Some("bilinear") => false,
                Some("nearest") => true,
                Some(other) => {
                    return Err(format!(
                        "imresize: the methods are \"bilinear\" and \"nearest\", not {other}"
                    ));
                }
            };
            let size = |e: &Expr| -> Result<usize, String> {
                whole(e, "a size of the new picture", 1, 1 << 14).map(|v| v as usize)
            };
            each(&a[0], name, &|m| {
                let (rows, columns) = match a[1].as_list() {
                    Some([r, c]) => (size(r)?, size(c)?),
                    Some(_) => {
                        return Err(
                            "imresize(picture, [rows, columns]) or imresize(picture, scale)".into(),
                        );
                    }
                    None => {
                        let scale = number(&a[1], name)?;
                        if scale.cmp0() != Ordering::Greater {
                            return Err("imresize: the scale is a number above 0".into());
                        }
                        let scaled = |old: usize| {
                            Rat::from(&scale * Rat::from(old as i64))
                                .ceil()
                                .numer()
                                .to_usize()
                                .filter(|v| (1..=1 << 14).contains(v))
                                .ok_or("imresize: the new picture has 1 to 16384 pixels a side")
                        };
                        (scaled(m.len())?, scaled(m[0].len())?)
                    }
                };
                Ok(resized(m, rows, columns, nearest))
            })?
        }
        ("imrotate", 2) => each(&a[0], name, &|m| turned(m, &a[1], name))?,
        ("imfilter", 2 | 3) => {
            // the weights laid on the picture (turned round for "conv"), the picture as large as it was
            let weights = match a.get(2).and_then(word).as_deref() {
                None | Some("corr") => crate::arrays::call("rot90", &[a[1].clone(), Expr::int(2)])
                    .unwrap_or_else(|| Err("imfilter: the weights are a matrix".into()))?,
                Some("conv") => a[1].clone(),
                Some(other) => {
                    return Err(format!(
                        "imfilter(picture, weights, \"conv\"): \"corr\" or \"conv\", not {other}"
                    ));
                }
            };
            let (all, single) = planes(&a[0], name)?;
            let filtered: Vec<Rows> = all
                .iter()
                .map(|m| {
                    let out = crate::signal::call(
                        "conv2",
                        &[
                            matrix::from_rows(m.clone()),
                            weights.clone(),
                            crate::texts::text("same"),
                        ],
                    )
                    .unwrap_or_else(|| Err("imfilter: the weights are a matrix".into()))?;
                    matrix::rows(&out)
                        .ok_or_else(|| "imfilter: the weights are a matrix".to_string())
                })
                .collect::<Result<_, _>>()?;
            picture(filtered, single)
        }
        ("imhist", 1 | 2) => {
            // how many pixels have every sample from 0 to the largest of the bits
            let (all, _) = planes(&a[0], name)?;
            let top = top(a.get(1), name)? as usize;
            let mut counts = vec![0i64; top + 1];
            for x in all.iter().flatten().flatten() {
                match x.as_i64() {
                    Some(v) if (0..=top as i64).contains(&v) => counts[v as usize] += 1,
                    _ => {
                        return Err(format!(
                            "imhist: the samples are whole numbers from 0 to {top}, {x} is none"
                        ));
                    }
                }
            }
            Expr::list(counts.into_iter().map(Expr::int).collect())
        }
        ("imbinarize", 1 | 2) => {
            // 1 where a pixel is lighter than the level, the middle of the range without one
            let (all, _) = planes(&a[0], name)?;
            let level = match a.get(1) {
                Some(l) => l.clone(),
                None => {
                    let entries: Vec<Expr> = all.iter().flatten().flatten().cloned().collect();
                    tidy(div(
                        add(vec![func("max", entries.clone()), func("min", entries)]),
                        Expr::int(2),
                    ))
                }
            };
            crate::arrays::map_result(&a[0], &|x| {
                Ok(Expr::int(i64::from(order(x, &level)? == Ordering::Greater)))
            })?
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

    fn matrix_of(rows: &[&[i64]]) -> Rows {
        rows.iter()
            .map(|r| r.iter().map(|k| Expr::int(*k)).collect())
            .collect()
    }

    #[test]
    fn the_middles_of_pixels_keep_their_places() {
        // twice as large: the middle of the first new pixel lies a quarter before the middle
        // of the first old one, of the second a quarter after it
        assert_eq!(place(0, 4, 2), Rat::from((-1, 4)));
        assert_eq!(place(1, 4, 2), Rat::from((1, 4)));
        assert_eq!(place(3, 4, 2), Rat::from((5, 4)));
        // the same size: every pixel on itself
        assert_eq!(place(2, 5, 5), Rat::from(2));
    }

    #[test]
    fn a_picture_of_another_size() {
        let m = matrix_of(&[&[0, 4], &[8, 12]]);
        // the nearest pixel: every pixel four times
        assert_eq!(
            resized(&m, 4, 4, true),
            matrix_of(&[
                &[0, 0, 4, 4],
                &[0, 0, 4, 4],
                &[8, 8, 12, 12],
                &[8, 8, 12, 12]
            ])
        );
        // straight lines between the middles, the rim held
        assert_eq!(
            resized(&m, 2, 4, false),
            matrix_of(&[&[0, 1, 3, 4], &[8, 9, 11, 12]])
        );
        assert_eq!(resized(&m, 2, 2, false), m);
        // half as large: the mean of two pixels each
        assert_eq!(
            resized(&matrix_of(&[&[0, 4, 8, 12]]), 1, 2, false),
            matrix_of(&[&[2, 10]])
        );
    }

    #[test]
    fn quarter_turns_are_exact() {
        let m = matrix_of(&[&[1, 2], &[3, 4]]);
        let turn = |k: i64| turned(&m, &Expr::int(90 * k), "imrotate").expect("a turn");
        assert_eq!(turn(1), matrix_of(&[&[2, 4], &[1, 3]]));
        assert_eq!(turn(2), matrix_of(&[&[4, 3], &[2, 1]]));
        assert_eq!(turn(-1), matrix_of(&[&[3, 1], &[4, 2]]));
        assert_eq!(turn(4), m);
        // any other angle gives a larger picture with the old one in its middle
        let fives = vec![vec![Expr::int(5); 4]; 4];
        let wide = turned(&fives, &Expr::int(45), "imrotate").expect("a turn");
        assert_eq!((wide.len(), wide[0].len()), (6, 6));
        assert_eq!(wide[3][3], Expr::int(5));
        assert_eq!(wide[0][0], Expr::zero());
    }
}
