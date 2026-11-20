//! Geometry of points given by their coordinates: areas of polygons, points in polygons, convex
//! hulls, Delaunay triangles, nearest points; all with exact tests.

use crate::arrays::{order, same, tidy, word};
use crate::canon::{add, div, mul, sub};
use crate::expr::Expr;
use crate::funcs::func;
use crate::matrix;
use crate::num::Rat;
use crate::numeric;
use std::cmp::Ordering;

type Res = Result<Expr, String>;
type Point = (Rat, Rat);

fn rational(e: &Expr, name: &str) -> Result<Rat, String> {
    if let Some(r) = e.as_rat() {
        return Ok(r.clone());
    }
    numeric::real_value(e, numeric::digits() + 10)
        .and_then(|f| f.to_rational())
        .ok_or_else(|| format!("{name}: {e} is not a real number"))
}

/// Points from two lists of coordinates, or from one list of `[x, y]` pairs.
fn points(a: &[Expr], name: &str) -> Result<Vec<Point>, String> {
    let usage = || format!("{name}([x values], [y values]) or {name}([[x, y], ...])");
    match a {
        [one] => one
            .as_list()
            .ok_or_else(usage)?
            .iter()
            .map(|p| match p.as_list() {
                Some([x, y]) => Ok((rational(x, name)?, rational(y, name)?)),
                _ => Err(usage()),
            })
            .collect(),
        [xs, ys] => {
            let (xs, ys) = (
                xs.as_list().ok_or_else(usage)?,
                ys.as_list().ok_or_else(usage)?,
            );
            if xs.len() != ys.len() {
                return Err(format!(
                    "{name}: {} x values and {} y values",
                    xs.len(),
                    ys.len()
                ));
            }
            xs.iter()
                .zip(ys)
                .map(|(x, y)| Ok((rational(x, name)?, rational(y, name)?)))
                .collect()
        }
        _ => Err(usage()),
    }
}

/// Twice the area of the triangle a, b, c: positive when they turn left.
fn turn(a: &Point, b: &Point, c: &Point) -> Rat {
    let (ux, uy) = (Rat::from(&b.0 - &a.0), Rat::from(&b.1 - &a.1));
    let (vx, vy) = (Rat::from(&c.0 - &a.0), Rat::from(&c.1 - &a.1));
    ux * vy - uy * vx
}

/// Positive when d lies inside the circle through a, b, c (which turn left).
fn in_circle(a: &Point, b: &Point, c: &Point, d: &Point) -> Rat {
    let row = |p: &Point| {
        let (x, y) = (Rat::from(&p.0 - &d.0), Rat::from(&p.1 - &d.1));
        let s = Rat::from(&x * &x) + Rat::from(&y * &y);
        (x, y, s)
    };
    let (a, b, c) = (row(a), row(b), row(c));
    let minor =
        |p: &(Rat, Rat, Rat), q: &(Rat, Rat, Rat)| Rat::from(&p.0 * &q.1) - Rat::from(&p.1 * &q.0);
    Rat::from(&a.2 * minor(&b, &c)) - Rat::from(&b.2 * minor(&a, &c))
        + Rat::from(&c.2 * minor(&a, &b))
}

/// The corners of the convex hull against the clock, from the lowest of the leftmost points.
fn hull(pts: &[Point]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..pts.len()).collect();
    order.sort_by(|i, j| pts[*i].cmp(&pts[*j]));
    order.dedup_by(|i, j| pts[*i] == pts[*j]);
    if order.len() < 3 {
        return order;
    }
    let mut chain: Vec<usize> = Vec::new();
    for pass in 0..2 {
        let start = chain.len();
        let list: Vec<usize> = if pass == 0 {
            order.clone()
        } else {
            order.iter().rev().copied().collect()
        };
        for k in list {
            while chain.len() >= start + 2
                && turn(
                    &pts[chain[chain.len() - 2]],
                    &pts[chain[chain.len() - 1]],
                    &pts[k],
                )
                .cmp0()
                    != Ordering::Greater
            {
                chain.pop();
            }
            chain.push(k);
        }
        chain.pop();
    }
    chain
}

/// The triangles of Delaunay: no point lies inside the circle through the corners of one.
fn delaunay(pts: &[Point]) -> Result<Vec<[usize; 3]>, String> {
    let mut order: Vec<usize> = (0..pts.len()).collect();
    order.sort_by(|i, j| pts[*i].cmp(&pts[*j]));
    order.dedup_by(|i, j| pts[*i] == pts[*j]);
    let sign = |a: usize, b: usize, c: usize| turn(&pts[a], &pts[b], &pts[c]).cmp0();
    let mut tris: Vec<[usize; 3]> = Vec::new();
    let mut border: Vec<usize> = Vec::new();
    let mut line: Vec<usize> = Vec::new();
    for &p in &order {
        if border.is_empty() {
            // the points so far lie on one line
            if line.len() < 2 || sign(line[0], line[line.len() - 1], p) == Ordering::Equal {
                line.push(p);
                continue;
            }
            let left = sign(line[0], line[line.len() - 1], p) == Ordering::Greater;
            for w in line.windows(2) {
                tris.push(if left {
                    [w[0], w[1], p]
                } else {
                    [w[1], w[0], p]
                });
            }
            border = if left {
                line.clone()
            } else {
                line.iter().rev().copied().collect()
            };
            border.push(p);
            continue;
        }
        let n = border.len();
        let seen: Vec<bool> = (0..n)
            .map(|k| sign(border[k], border[(k + 1) % n], p) == Ordering::Less)
            .collect();
        let Some(first) = (0..n).find(|k| seen[*k] && !seen[(k + n - 1) % n]) else {
            continue;
        };
        let mut count = 0;
        while seen[(first + count) % n] {
            let (u, v) = (border[(first + count) % n], border[(first + count + 1) % n]);
            tris.push([v, u, p]);
            count += 1;
        }
        // the corners between the first and the last edge seen give way to p
        let mut next: Vec<usize> = Vec::with_capacity(n + 2 - count);
        for k in 0..=(n - count) {
            next.push(border[(first + count + k) % n]);
        }
        next.push(p);
        border = next;
    }
    if tris.is_empty() {
        return Err("delaunay: the points lie on one line".into());
    }
    // edges are turned until every one is right
    for _ in 0..10 * tris.len() * tris.len() + 100 {
        let mut sides: std::collections::HashMap<(usize, usize), (usize, usize)> =
            std::collections::HashMap::new();
        for (t, tri) in tris.iter().enumerate() {
            for k in 0..3 {
                sides.insert((tri[k], tri[(k + 1) % 3]), (t, tri[(k + 2) % 3]));
            }
        }
        let mut turned = false;
        for (t, tri) in tris.clone().iter().enumerate() {
            for k in 0..3 {
                let (a, b, c) = (tri[k], tri[(k + 1) % 3], tri[(k + 2) % 3]);
                let Some(&(other, d)) = sides.get(&(b, a)) else {
                    continue;
                };
                if in_circle(&pts[a], &pts[b], &pts[c], &pts[d]).cmp0() == Ordering::Greater {
                    tris[t] = [a, d, c];
                    tris[other] = [d, b, c];
                    turned = true;
                    break;
                }
            }
            if turned {
                break;
            }
        }
        if !turned {
            break;
        }
    }
    for tri in tris.iter_mut() {
        let least = (0..3).min_by_key(|k| tri[*k]).unwrap_or(0);
        tri.rotate_left(least);
    }
    tris.sort_unstable();
    Ok(tris)
}

fn inside(q: &Point, poly: &[Point]) -> bool {
    let n = poly.len();
    let mut crossings = 0;
    for k in 0..n {
        let (a, b) = (&poly[k], &poly[(k + 1) % n]);
        // on the edge counts as inside
        if turn(a, b, q).cmp0() == Ordering::Equal
            && q.0 >= *(&a.0).min(&b.0)
            && q.0 <= *(&a.0).max(&b.0)
            && q.1 >= *(&a.1).min(&b.1)
            && q.1 <= *(&a.1).max(&b.1)
        {
            return true;
        }
        if (a.1 > q.1) != (b.1 > q.1) {
            let side = turn(a, b, q).cmp0();
            let up = b.1 > a.1;
            if (side == Ordering::Greater) == up {
                crossings += 1;
            }
        }
    }
    crossings % 2 == 1
}

/// A function of two places for two numbers, two lists or two matrices of one shape.
fn pairwise(x: &Expr, y: &Expr, f: &dyn Fn(&Expr, &Expr) -> Res, name: &str) -> Res {
    match (x.as_list(), y.as_list()) {
        (Some(xs), Some(ys)) if xs.len() == ys.len() => Ok(Expr::list(
            xs.iter()
                .zip(ys)
                .map(|(p, q)| pairwise(p, q, f, name))
                .collect::<Result<_, _>>()?,
        )),
        (None, None) => f(x, y),
        _ => Err(format!(
            "{name}: the places asked for are two numbers, or two lists or matrices of one shape"
        )),
    }
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    Ok(Some(match (name, n) {
        ("polyarea", 1 | 2) => {
            // (the corners may be any expressions here)
            let (xs, ys): (Vec<Expr>, Vec<Expr>) = match a {
                [one] => one
                    .as_list()
                    .ok_or("polyarea([[x, y], ...])")?
                    .iter()
                    .map(|p| match p.as_list() {
                        Some([x, y]) => Ok((x.clone(), y.clone())),
                        _ => Err("polyarea([[x, y], ...])".to_string()),
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .unzip(),
                _ => {
                    let (xs, ys) = (
                        a[0].as_list().ok_or("polyarea([x values], [y values])")?,
                        a[1].as_list().ok_or("polyarea([x values], [y values])")?,
                    );
                    if xs.len() != ys.len() {
                        return Err("polyarea: as many x values as y values".into());
                    }
                    (xs.to_vec(), ys.to_vec())
                }
            };
            let count = xs.len();
            let twice = add((0..count)
                .map(|k| {
                    let j = (k + 1) % count;
                    sub(
                        mul(vec![xs[k].clone(), ys[j].clone()]),
                        mul(vec![xs[j].clone(), ys[k].clone()]),
                    )
                })
                .collect());
            tidy(div(func("abs", vec![tidy(twice)]), Expr::int(2)))
        }
        ("inpolygon", 4) => {
            let poly = points(&a[2..], name)?;
            if poly.len() < 3 {
                return Err("inpolygon: a polygon has three corners or more".into());
            }
            match (a[0].as_list(), a[1].as_list()) {
                (Some(xs), Some(ys)) if xs.len() == ys.len() => Expr::list(
                    xs.iter()
                        .zip(ys)
                        .map(|(x, y)| {
                            Ok(Expr::Bool(inside(
                                &(rational(x, name)?, rational(y, name)?),
                                &poly,
                            )))
                        })
                        .collect::<Result<_, String>>()?,
                ),
                (None, None) => Expr::Bool(inside(
                    &(rational(&a[0], name)?, rational(&a[1], name)?),
                    &poly,
                )),
                _ => {
                    return Err(
                        "inpolygon(x, y, [corner x values], [corner y values]): as many x as y"
                            .into(),
                    );
                }
            }
        }
        ("convhull" | "convexhull", 1 | 2) => {
            let pts = points(a, name)?;
            let corners = hull(&pts);
            if name == "convhull" {
                let mut out: Vec<Expr> = corners.iter().map(|k| Expr::int(*k as i64 + 1)).collect();
                out.extend(corners.first().map(|k| Expr::int(*k as i64 + 1)));
                Expr::list(out)
            } else {
                Expr::list(
                    corners
                        .iter()
                        .map(|k| {
                            Expr::list(vec![
                                Expr::rational(pts[*k].0.clone()),
                                Expr::rational(pts[*k].1.clone()),
                            ])
                        })
                        .collect(),
                )
            }
        }
        ("delaunay", 1 | 2) => {
            let pts = points(a, name)?;
            if pts.len() > 2000 {
                return Err("delaunay: more than 2000 points".into());
            }
            Expr::list(
                delaunay(&pts)?
                    .into_iter()
                    .map(|t| Expr::list(t.iter().map(|k| Expr::int(*k as i64 + 1)).collect()))
                    .collect(),
            )
        }
        ("griddata", 5 | 6) => {
            let pts = points(&a[..2], name)?;
            let values = a[2]
                .as_list()
                .filter(|v| v.len() == pts.len())
                .ok_or("griddata(x, y, values, xq, yq): a value for every point")?;
            let nearest = match a.get(5).and_then(word).as_deref() {
                None | Some("linear") => false,
                Some("nearest") => true,
                Some(other) => {
                    return Err(format!(
                        "griddata: the methods are \"linear\" and \"nearest\", not {other}"
                    ));
                }
            };
            let triangles = if nearest { Vec::new() } else { delaunay(&pts)? };
            let at = |x: &Expr, y: &Expr| -> Res {
                let q = (rational(x, name)?, rational(y, name)?);
                if nearest {
                    let far = |p: &Point| {
                        let (dx, dy) = (Rat::from(&p.0 - &q.0), Rat::from(&p.1 - &q.1));
                        Rat::from(&dx * &dx) + Rat::from(&dy * &dy)
                    };
                    let best = (0..pts.len())
                        .min_by(|i, j| far(&pts[*i]).cmp(&far(&pts[*j])))
                        .unwrap_or(0);
                    return Ok(values[best].clone());
                }
                for t in &triangles {
                    let [p, r, s] = [&pts[t[0]], &pts[t[1]], &pts[t[2]]];
                    let whole = turn(p, r, s);
                    // the shares of the three corners: the areas across from them
                    let shares = [turn(&q, r, s), turn(p, &q, s), turn(p, r, &q)];
                    if shares.iter().all(|w| w.cmp0() != Ordering::Less) {
                        return Ok(tidy(add((0..3)
                            .map(|k| {
                                mul(vec![
                                    Expr::rational(Rat::from(&shares[k] / &whole)),
                                    values[t[k]].clone(),
                                ])
                            })
                            .collect())));
                    }
                }
                Ok(Expr::undefined())
            };
            pairwise(&a[3], &a[4], &at, name)?
        }
        ("tsearchn", 3) => {
            let pts = points(std::slice::from_ref(&a[0]), name)?;
            let triangles: Vec<[usize; 3]> = matrix::rows(&a[1])
                .ok_or("tsearchn([points], [triangles], point)")?
                .iter()
                .map(|t| match t[..] {
                    [ref i, ref j, ref k] => {
                        let at = |e: &Expr| {
                            e.as_i64()
                                .filter(|v| (1..=pts.len() as i64).contains(v))
                                .map(|v| v as usize - 1)
                                .ok_or_else(|| format!("tsearchn: {e} is no number of a point"))
                        };
                        Ok([at(i)?, at(j)?, at(k)?])
                    }
                    _ => Err("tsearchn: a triangle is three numbers of points".to_string()),
                })
                .collect::<Result<_, _>>()?;
            let find = |q: &[Expr]| -> Res {
                let [x, y] = q else {
                    return Err("tsearchn: a point is [x, y]".to_string());
                };
                let q = (rational(x, name)?, rational(y, name)?);
                for (k, t) in triangles.iter().enumerate() {
                    let [p, r, s] = [&pts[t[0]], &pts[t[1]], &pts[t[2]]];
                    let sides = [turn(&q, r, s), turn(p, &q, s), turn(p, r, &q)];
                    let left = turn(p, r, s).cmp0();
                    if sides.iter().all(|w| w.cmp0() == left || w.is_zero()) {
                        return Ok(Expr::int(k as i64 + 1));
                    }
                }
                Ok(Expr::undefined())
            };
            match matrix::rows(&a[2]) {
                Some(queries) => {
                    Expr::list(queries.iter().map(|q| find(q)).collect::<Result<_, _>>()?)
                }
                None => find(
                    a[2].as_list()
                        .ok_or("tsearchn([points], [triangles], point)")?,
                )?,
            }
        }
        ("rectint", 2) => {
            let rect = |e: &Expr| -> Result<[Expr; 4], String> {
                match e.as_list() {
                    Some([x, y, w, h]) => Ok([x.clone(), y.clone(), w.clone(), h.clone()]),
                    _ => Err("rectint([x, y, width, height], [x, y, width, height])".into()),
                }
            };
            let (p, q) = (rect(&a[0])?, rect(&a[1])?);
            let overlap = |lo1: &Expr, len1: &Expr, lo2: &Expr, len2: &Expr| {
                let high = func(
                    "min",
                    vec![
                        add(vec![lo1.clone(), len1.clone()]),
                        add(vec![lo2.clone(), len2.clone()]),
                    ],
                );
                func(
                    "max",
                    vec![
                        Expr::zero(),
                        sub(high, func("max", vec![lo1.clone(), lo2.clone()])),
                    ],
                )
            };
            tidy(mul(vec![
                overlap(&p[0], &p[2], &q[0], &q[2]),
                overlap(&p[1], &p[3], &q[1], &q[3]),
            ]))
        }
        ("dsearchn", 2) => {
            let pts = matrix::rows(&a[0]).ok_or("dsearchn([points], point)")?;
            let nearest = |q: &[Expr]| -> Res {
                if q.len() != pts[0].len() {
                    return Err("dsearchn: the points have different numbers of coordinates".into());
                }
                let far = |p: &[Expr]| {
                    tidy(add(p
                        .iter()
                        .zip(q)
                        .map(|(x, y)| crate::canon::pow(sub(x.clone(), y.clone()), Expr::int(2)))
                        .collect()))
                };
                let mut best = (0, far(&pts[0]));
                for (k, p) in pts.iter().enumerate().skip(1) {
                    let d = far(p);
                    if order(&d, &best.1)? == Ordering::Less {
                        best = (k, d);
                    }
                }
                Ok(Expr::int(best.0 as i64 + 1))
            };
            match matrix::rows(&a[1]) {
                Some(queries) => Expr::list(
                    queries
                        .iter()
                        .map(|q| nearest(q))
                        .collect::<Result<_, _>>()?,
                ),
                None => nearest(a[1].as_list().ok_or("dsearchn([points], point)")?)?,
            }
        }
        ("rotx" | "roty" | "rotz", 1) => {
            let angle = mul(vec![a[0].clone(), div(Expr::pi(), Expr::int(180))]);
            let (c, s) = (
                tidy(func("cos", vec![angle.clone()])),
                tidy(func("sin", vec![angle])),
            );
            let (o, l, ms) = (Expr::zero(), Expr::one(), crate::canon::neg(s.clone()));
            matrix::from_rows(match name {
                "rotx" => vec![
                    vec![l, o.clone(), o.clone()],
                    vec![o.clone(), c.clone(), ms],
                    vec![o, s, c],
                ],
                "roty" => vec![
                    vec![c.clone(), o.clone(), s],
                    vec![o.clone(), l, o.clone()],
                    vec![ms, o, c],
                ],
                _ => vec![
                    vec![c.clone(), ms, o.clone()],
                    vec![s, c, o.clone()],
                    vec![o.clone(), o, l],
                ],
            })
        }
        ("voronoi", 1 | 2) => {
            // the corners of the cells: the centres of the circles through the Delaunay triangles
            let pts = points(a, name)?;
            let mut centres: Vec<Expr> = Vec::new();
            for t in delaunay(&pts)? {
                let (p, q, r) = (&pts[t[0]], &pts[t[1]], &pts[t[2]]);
                let d = turn(p, q, r) * Rat::from(2);
                let sq = |v: &Point| Rat::from(&v.0 * &v.0) + Rat::from(&v.1 * &v.1);
                let (sp, sq_, sr) = (sq(p), sq(q), sq(r));
                let x = (Rat::from(&sp * Rat::from(&q.1 - &r.1))
                    + Rat::from(&sq_ * Rat::from(&r.1 - &p.1))
                    + Rat::from(&sr * Rat::from(&p.1 - &q.1)))
                    / &d;
                let y = (Rat::from(&sp * Rat::from(&r.0 - &q.0))
                    + Rat::from(&sq_ * Rat::from(&p.0 - &r.0))
                    + Rat::from(&sr * Rat::from(&q.0 - &p.0)))
                    / &d;
                let centre = Expr::list(vec![Expr::rational(x), Expr::rational(y)]);
                if !centres.iter().any(|c| same(c, &centre)) {
                    centres.push(centre);
                }
            }
            Expr::list(centres)
        }
        _ => return Ok(None),
    }))
}

pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    inner(name, a).transpose()
}
