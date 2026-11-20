//! More charts of data: bars and counts over a plane, a surface with its lines of height,
//! lines that follow a field, counts around a circle, the tree of a clustering, bubbles,
//! surfaces of an equation and of triangles, sorted bars with their running sum, a filled
//! shape, ribbons, and the charts that compare samples.

use crate::chart::{level_lines, roomy};
use crate::datachart::{
    at_share, chart, flat, heights, nothing_more, reals, span, wave, with_zero, xy,
};
use crate::expr::Expr;
use crate::numeric;
use crate::plot::{ends, label, number, value2, variable};
use crate::scene::{Drawing, Item, Paint, Point, Shade, Space, turn};
use std::rc::Rc;

/// The drawings of this module.
pub fn is_chart(name: &str) -> bool {
    matches!(
        name,
        "bar3"
            | "histogram2"
            | "surfc"
            | "streamline"
            | "polarhistogram"
            | "dendrogram"
            | "bubblechart"
            | "fimplicit3"
            | "isosurface"
            | "triplot"
            | "trisurf"
            | "pareto"
            | "fill"
            | "ribbon"
            | "normplot"
            | "parallelplot"
            | "swarmchart"
            | "violinplot"
            | "histfit"
            | "gscatter"
            | "andrewsplot"
            | "bar3h"
            | "cdfplot"
            | "plotmatrix"
            | "stackedplot"
            | "biplot"
    ) || is_control_chart(name)
}

/// Four corners of a face, around it, as rows of a surface with a gap after them.
fn quad(points: &mut Vec<Option<Point>>, corners: [Point; 4]) {
    let [a, b, c, d] = corners;
    points.extend([Some(a), Some(b), Some(d), Some(c), None, None]);
}

fn faces(points: Vec<Option<Point>>) -> Item {
    Item::Mesh {
        columns: 2,
        points,
        wires: 1,
    }
}

/// A box of space from the plane of height 0 up (or down) to its height.
fn pillar(points: &mut Vec<Option<Point>>, x: (f64, f64), y: (f64, f64), height: f64) {
    let (low, high) = (height.min(0.0), height.max(0.0));
    let corner = |i: usize, z: f64| -> Point {
        [
            if i == 1 || i == 2 { x.1 } else { x.0 },
            if i >= 2 { y.1 } else { y.0 },
            z,
        ]
    };
    quad(
        points,
        [
            corner(0, high),
            corner(1, high),
            corner(2, high),
            corner(3, high),
        ],
    );
    quad(
        points,
        [
            corner(0, low),
            corner(1, low),
            corner(2, low),
            corner(3, low),
        ],
    );
    for i in 0..4 {
        let j = (i + 1) % 4;
        quad(
            points,
            [
                corner(i, low),
                corner(j, low),
                corner(j, high),
                corner(i, high),
            ],
        );
    }
}

fn solid(items: Vec<Item>, ranges: [(f64, f64); 3], facts: String) -> Drawing {
    let wide = |(low, high): (f64, f64)| {
        if high > low {
            (low, high)
        } else {
            (low, low + 1.0)
        }
    };
    let mut drawing = Drawing::new(Space::Solid, items);
    drawing.stretch = Some(ranges.map(wide));
    drawing.axes = true;
    drawing.palette = "viridis";
    drawing.fog = 0.2;
    drawing.facts = vec![facts];
    drawing
}

fn matrix_of(e: &Expr, name: &str) -> Result<Vec<Vec<f64>>, String> {
    let rows = crate::matrix::rows(e)
        .filter(|m| !m.is_empty() && !m[0].is_empty())
        .ok_or_else(|| format!("{name} needs a matrix of numbers"))?;
    rows.iter()
        .map(|r| {
            r.iter()
                .map(|v| number(v).ok_or_else(|| format!("{name}: {v} is no real number")))
                .collect()
        })
        .collect()
}

/// `bar3(matrix)`: a bar of space for every entry, a row of the matrix along y.
fn bar3(a: &[Expr]) -> Result<Drawing, String> {
    let [z] = a else {
        return Err("bar3(matrix of heights): a bar for every entry".into());
    };
    let z = matrix_of(z, "bar3")?;
    let mut points = Vec::new();
    for (j, row) in z.iter().enumerate() {
        for (i, h) in row.iter().enumerate() {
            let (x, y) = (i as f64 + 1.0, j as f64 + 1.0);
            pillar(&mut points, (x - 0.4, x + 0.4), (y - 0.4, y + 0.4), *h);
        }
    }
    let all: Vec<f64> = z.iter().flatten().copied().collect();
    let (low, high) = with_zero(span(&all));
    Ok(solid(
        vec![faces(points)],
        [
            (0.5, z[0].len() as f64 + 0.5),
            (0.5, z.len() as f64 + 0.5),
            (low, high),
        ],
        format!(
            "{} bars in {} rows; heights from {} to {}",
            all.len(),
            z.len(),
            label(span(&all).0),
            label(span(&all).1)
        ),
    ))
}

/// `histogram2(x, y[, cells])`: how many pairs fall into every cell of a grid, as bars.
fn histogram2(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "histogram2([x values], [y values][, cells or [cells of x, cells of y]])";
    let (xs, ys, rest) = match a {
        [x, y, rest @ ..] => (reals(x, "histogram2")?, reals(y, "histogram2")?, rest),
        _ => return Err(usage.into()),
    };
    if xs.len() != ys.len() || xs.is_empty() || rest.len() > 1 {
        return Err(usage.into());
    }
    let cells = |e: &Expr| {
        number(e)
            .filter(|n| *n >= 1.0 && *n <= 200.0 && n.fract() == 0.0)
            .map(|n| n as usize)
    };
    let (nx, ny) = match rest.first() {
        None => (10, 10),
        Some(Expr::List(pair)) if pair.len() == 2 => {
            (cells(&pair[0]).ok_or(usage)?, cells(&pair[1]).ok_or(usage)?)
        }
        Some(n) => {
            let n = cells(n).ok_or(usage)?;
            (n, n)
        }
    };
    let wide = |(low, high): (f64, f64)| {
        if high > low {
            (low, high)
        } else {
            (low - 0.5, low + 0.5)
        }
    };
    let (rx, ry) = (wide(span(&xs)), wide(span(&ys)));
    let place = |v: f64, (low, high): (f64, f64), n: usize| -> usize {
        (((v - low) / (high - low) * n as f64) as usize).min(n - 1)
    };
    let mut counts = vec![vec![0u32; nx]; ny];
    for (x, y) in xs.iter().zip(&ys) {
        counts[place(*y, ry, ny)][place(*x, rx, nx)] += 1;
    }
    let (dx, dy) = ((rx.1 - rx.0) / nx as f64, (ry.1 - ry.0) / ny as f64);
    let mut points = Vec::new();
    let mut most = 0;
    for (j, row) in counts.iter().enumerate() {
        for (i, count) in row.iter().enumerate() {
            most = most.max(*count);
            if *count > 0 {
                let (x, y) = (rx.0 + dx * i as f64, ry.0 + dy * j as f64);
                pillar(&mut points, (x, x + dx), (y, y + dy), f64::from(*count));
            }
        }
    }
    Ok(solid(
        vec![faces(points)],
        [rx, ry, (0.0, f64::from(most))],
        format!(
            "{} pairs in {nx} by {ny} cells; the fullest cell holds {most}",
            xs.len()
        ),
    ))
}

/// `surfc(...)`: the surface of `surf` with its lines of equal height on the floor.
fn surfc(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "surfc(matrix of heights) or surfc([x values], [y values], matrix)";
    let (xs, ys, z) = match a {
        [z] => {
            let m = matrix_of(z, "surfc")?;
            (
                (1..=m[0].len()).map(|k| k as f64).collect::<Vec<f64>>(),
                (1..=m.len()).map(|k| k as f64).collect::<Vec<f64>>(),
                m,
            )
        }
        [x, y, z] => (
            reals(x, "surfc")?,
            reals(y, "surfc")?,
            matrix_of(z, "surfc")?,
        ),
        _ => return Err(usage.into()),
    };
    if z.len() != ys.len() || z[0].len() != xs.len() || xs.len() < 2 || ys.len() < 2 {
        return Err(usage.into());
    }
    let all: Vec<f64> = z.iter().flatten().copied().collect();
    let (low, high) = span(&all);
    let floor = low - 0.3 * if high > low { high - low } else { 1.0 };
    let points: Vec<Option<Point>> = z
        .iter()
        .enumerate()
        .flat_map(|(j, row)| {
            let (xs, ys) = (&xs, &ys);
            row.iter()
                .enumerate()
                .map(move |(i, h)| Some([xs[i], ys[j], *h]))
        })
        .collect();
    let (columns, rows) = (xs.len() - 1, ys.len() - 1);
    // a place between the points of the grid, by the share of the cell it lies in
    let between = |v: &[f64], at: f64| -> f64 {
        let k = (at.floor() as usize).min(v.len() - 2);
        v[k] + (at - k as f64) * (v[k + 1] - v[k])
    };
    let place = |i: f64, j: f64| -> Point { [between(&xs, i), between(&ys, j), floor] };
    let mut items = vec![Item::Mesh {
        columns: xs.len(),
        points,
        wires: 1,
    }];
    let levels = heights(None, (low, high))?;
    for (k, level) in levels.iter().enumerate() {
        let at = |i: usize, j: usize| Some(z[j][i] - level);
        let pairs = level_lines(&at, columns, rows, &place);
        if !pairs.is_empty() {
            items.push(Item::Segments {
                pairs,
                paint: Paint::Curve(k),
                weight: 1.0,
            });
        }
    }
    Ok(solid(
        items,
        [span(&xs), span(&ys), (floor, high)],
        format!(
            "the surface over {} by {} points; z from {} to {}; below it the lines of {} heights",
            xs.len(),
            ys.len(),
            label(low),
            label(high),
            levels.len()
        ),
    ))
}

/// `streamline([u, v], x, a, b, y, c, d[, starts])`: the lines that follow a field.
fn streamline(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "streamline([u(x, y), v(x, y)], x, a, b, y, c, d[, [[x, y], ...] to start from])";
    let (field, vars, x, y, rest) = match a {
        [Expr::List(field), xv, xa, xb, yv, ya, yb, rest @ ..]
            if field.len() == 2 && rest.len() <= 1 =>
        {
            (
                field,
                (variable(xv)?, variable(yv)?),
                ends(xa, xb)?,
                ends(ya, yb)?,
                rest,
            )
        }
        _ => return Err(usage.into()),
    };
    let starts: Vec<[f64; 2]> = match rest.first() {
        Some(list) => {
            let items = list.as_list().ok_or(usage)?;
            items
                .iter()
                .map(|p| {
                    reals(p, "streamline").and_then(|v| match v[..] {
                        [px, py] => Ok([px, py]),
                        _ => Err(usage.to_string()),
                    })
                })
                .collect::<Result<_, _>>()?
        }
        None => (0..6)
            .flat_map(|j| {
                (0..6).map(move |i| {
                    [
                        x.0 + (x.1 - x.0) * (i as f64 + 0.5) / 6.0,
                        y.0 + (y.1 - y.0) * (j as f64 + 0.5) / 6.0,
                    ]
                })
            })
            .collect(),
    };
    // the way of the field at a point, as a step of length one; none where it stops or
    // has no value
    let way = |p: [f64; 2]| -> Option<[f64; 2]> {
        let (u, v) = (
            value2(&field[0], &vars, p[0], p[1])?,
            value2(&field[1], &vars, p[0], p[1])?,
        );
        let long = (u * u + v * v).sqrt();
        (long.is_finite() && long > 1e-12).then(|| [u / long, v / long])
    };
    let inside = |p: [f64; 2]| p[0] >= x.0 && p[0] <= x.1 && p[1] >= y.0 && p[1] <= y.1;
    let (wide, tall) = (x.1 - x.0, y.1 - y.0);
    let step = (wide * wide + tall * tall).sqrt() / 240.0;
    let walk = |from: [f64; 2], side: f64| -> Vec<Point> {
        let mut out = Vec::new();
        let mut p = from;
        for _ in 0..480 {
            // the rule of Runge and Kutta with four slopes
            let go =
                |p: [f64; 2], d: [f64; 2], h: f64| [p[0] + side * h * d[0], p[1] + side * h * d[1]];
            let Some(k1) = way(p) else { break };
            let Some(k2) = way(go(p, k1, step / 2.0)) else {
                break;
            };
            let Some(k3) = way(go(p, k2, step / 2.0)) else {
                break;
            };
            let Some(k4) = way(go(p, k3, step)) else {
                break;
            };
            let mean = [
                (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]) / 6.0,
                (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]) / 6.0,
            ];
            p = go(p, mean, step);
            if !inside(p) {
                break;
            }
            out.push(flat(p[0], p[1]));
        }
        out
    };
    let mut items = Vec::new();
    for from in starts.iter().filter(|p| inside(**p)) {
        let mut points = walk(*from, -1.0);
        points.reverse();
        points.push(flat(from[0], from[1]));
        points.extend(walk(*from, 1.0));
        if points.len() >= 2 {
            items.push(Item::Line {
                points,
                paint: Paint::Along,
                weight: 1.0,
            });
        }
    }
    if items.is_empty() {
        return Err("streamline: the field has no real value at the points to start from".into());
    }
    let count = items.len();
    let mut drawing = Drawing::new(Space::Chart { x, y, equal: false }, items);
    drawing.facts = vec![format!(
        "{count} lines that follow the field, each from the first colour of the palette to the last"
    )];
    Ok(drawing)
}

/// Circles at the quarters of a radius and the two axes.
fn rings(reach: f64) -> Item {
    let mut pairs = Vec::new();
    for ring in 1..=4 {
        let r = reach * f64::from(ring) / 4.0;
        let corners: Vec<Point> = (0..=96u64)
            .map(|k| {
                let (s, c) = turn((k % 96, 96));
                flat(r * c, r * s)
            })
            .collect();
        pairs.extend(corners.windows(2).map(|w| [w[0], w[1]]));
    }
    pairs.push([flat(-reach, 0.0), flat(reach, 0.0)]);
    pairs.push([flat(0.0, -reach), flat(0.0, reach)]);
    Item::Segments {
        pairs,
        paint: Paint::Ink,
        weight: 0.5,
    }
}

/// `polarhistogram(angles[, parts])`: how many angles fall into every part of the circle.
fn polarhistogram(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "polarhistogram([angles in radians][, number of parts])";
    let (angles, parts) = match a {
        [t] => (reals(t, "polarhistogram")?, 12u64),
        [t, n] => (
            reals(t, "polarhistogram")?,
            number(n)
                .filter(|n| *n >= 1.0 && *n <= 360.0 && n.fract() == 0.0)
                .ok_or(usage)? as u64,
        ),
        _ => return Err(usage.into()),
    };
    if angles.is_empty() {
        return Err(usage.into());
    }
    // the sides of the parts, each part cut so that no piece is wider than a sixth of a turn
    let cuts = (6 / parts).max(1) * 3;
    let pieces = parts * cuts;
    let side = move |k: u64| {
        let (s, c) = turn((k % pieces, pieces));
        (c, s)
    };
    let cross = |u: (f64, f64), v: (f64, f64)| u.0 * v.1 - u.1 * v.0;
    let piece_of = move |p: (f64, f64)| -> u64 {
        (0..pieces)
            .find(|k| cross(side(*k), p) >= 0.0 && cross(p, side(k + 1)) > 0.0)
            .unwrap_or(0)
    };
    let mut counts = vec![0u32; parts as usize];
    for t in &angles {
        counts[(piece_of(wave(*t)) / cuts) as usize] += 1;
    }
    let most = counts.iter().copied().max().unwrap_or(1).max(1);
    let reach = f64::from(most);
    let filled = counts.clone();
    let field = move |px: f64, py: f64| -> Option<Shade> {
        let count = filled[(piece_of((px, py)) / cuts) as usize];
        (count > 0 && px * px + py * py <= f64::from(count * count))
            .then(|| Shade::Level(f64::from(count) / f64::from(most)))
    };
    // the rims of the parts
    let mut pairs = Vec::new();
    for (part, count) in counts.iter().enumerate().filter(|(_, c)| **c > 0) {
        let r = f64::from(*count);
        let (from, to) = (part as u64 * cuts, (part as u64 + 1) * cuts);
        let at = |k: u64| {
            let (c, s) = side(k);
            flat(r * c, r * s)
        };
        pairs.push([flat(0.0, 0.0), at(from)]);
        pairs.push([flat(0.0, 0.0), at(to)]);
        // (the arc in steps of a 96th of the part)
        let (c0, s0) = side(from);
        let (c1, s1) = side(to);
        let mut before = flat(r * c0, r * s0);
        for step in 1..=24u32 {
            let share = f64::from(step) / 24.0;
            let (x, y) = (c0 + (c1 - c0) * share, s0 + (s1 - s0) * share);
            let long = (x * x + y * y).sqrt();
            let next = flat(r * x / long, r * y / long);
            pairs.push([before, next]);
            before = next;
        }
    }
    let mut drawing = Drawing::new(
        Space::Chart {
            x: (-1.1 * reach, 1.1 * reach),
            y: (-1.1 * reach, 1.1 * reach),
            equal: true,
        },
        vec![
            Item::Field(Box::new(field)),
            rings(reach),
            Item::Segments {
                pairs,
                paint: Paint::Ink,
                weight: 1.0,
            },
        ],
    );
    drawing.aspect = Some(1.0);
    drawing.palette = "viridis";
    drawing.facts = vec![format!(
        "{} angles in {parts} parts of the circle, against the clock from the x axis: {}",
        angles.len(),
        counts
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    )];
    Ok(drawing)
}

/// `dendrogram(tree)`: the tree that `linkage` gives, every joining at its height.
fn dendrogram(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "dendrogram(tree): the rows [group, group, height] that linkage gives";
    let [tree] = a else {
        return Err(usage.into());
    };
    let rows = matrix_of(tree, "dendrogram")?;
    let leaves = rows.len() + 1;
    if rows.iter().any(|r| r.len() < 3) {
        return Err(usage.into());
    }
    let mut joins: Vec<(usize, usize, f64)> = Vec::new();
    for (k, row) in rows.iter().enumerate() {
        let id = |v: f64| -> Result<usize, String> {
            (v.fract() == 0.0 && v >= 1.0 && (v as usize) <= leaves + k)
                .then(|| v as usize - 1)
                .ok_or_else(|| usage.to_string())
        };
        joins.push((id(row[0])?, id(row[1])?, row[2]));
    }
    // the leaves from the left to the right, as the tree hangs from its last joining
    let mut order: Vec<usize> = Vec::new();
    let mut stack = vec![leaves + joins.len() - 1];
    while let Some(node) = stack.pop() {
        if node < leaves {
            order.push(node);
        } else {
            let (left, right, _) = joins[node - leaves];
            stack.push(right);
            stack.push(left);
        }
        if order.len() > leaves || stack.len() > 4 * leaves {
            return Err(usage.into());
        }
    }
    if order.len() != leaves {
        return Err(usage.into());
    }
    let mut place: Vec<(f64, f64)> = vec![(0.0, 0.0); leaves + joins.len()];
    for (rank, leaf) in order.iter().enumerate() {
        place[*leaf] = (rank as f64 + 1.0, 0.0);
    }
    let mut pairs = Vec::new();
    for (k, (left, right, height)) in joins.iter().enumerate() {
        let (l, r) = (place[*left], place[*right]);
        pairs.push([flat(l.0, l.1), flat(l.0, *height)]);
        pairs.push([flat(r.0, r.1), flat(r.0, *height)]);
        pairs.push([flat(l.0, *height), flat(r.0, *height)]);
        place[leaves + k] = ((l.0 + r.0) / 2.0, *height);
    }
    let top = joins.iter().fold(0.0f64, |m, j| m.max(j.2));
    let top = if top > 0.0 { top } else { 1.0 };
    let mut items = vec![Item::Segments {
        pairs,
        paint: Paint::Curve(0),
        weight: 1.0,
    }];
    for (rank, leaf) in order.iter().enumerate() {
        items.push(Item::Label {
            at: flat(rank as f64 + 1.0, -0.05 * top),
            text: (leaf + 1).to_string(),
        });
    }
    let mut drawing = Drawing::new(
        Space::Chart {
            x: (0.4, leaves as f64 + 0.6),
            y: (-0.1 * top, 1.05 * top),
            equal: false,
        },
        items,
    );
    drawing.facts = vec![format!(
        "the tree of {leaves} leaves, from the left: {}; the last joining at the height {}",
        order
            .iter()
            .map(|l| (l + 1).to_string())
            .collect::<Vec<_>>()
            .join(", "),
        label(top)
    )];
    Ok(drawing)
}

/// `bubblechart(x, y, sizes)`: dots whose widths show a third value.
fn bubblechart(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "bubblechart([x values], [y values], [sizes])";
    let [x, y, sizes] = a else {
        return Err(usage.into());
    };
    let (xs, ys, sizes) = (
        reals(x, "bubblechart")?,
        reals(y, "bubblechart")?,
        reals(sizes, "bubblechart")?,
    );
    if xs.len() != ys.len() || xs.len() != sizes.len() || xs.is_empty() {
        return Err(usage.into());
    }
    let (least, most) = span(&sizes);
    let items = xs
        .iter()
        .zip(&ys)
        .zip(&sizes)
        .map(|((px, py), size)| Item::Dots {
            points: vec![[*px, *py]],
            paint: Paint::Curve(0),
            radius: match most > least {
                true => 3.0 + 15.0 * (size - least) / (most - least),
                false => 9.0,
            },
            units: false,
        })
        .collect();
    let mut drawing = Drawing::new(chart(span(&xs), span(&ys)), items);
    drawing.facts = vec![format!(
        "{} bubbles; the smallest stands for {}, the widest for {}",
        xs.len(),
        label(least),
        label(most)
    )];
    Ok(drawing)
}

fn value3(f: &Expr, vars: &[Rc<str>; 3], p: [f64; 3]) -> Option<f64> {
    let mut env = numeric::Env::new();
    for (name, v) in vars.iter().zip(p) {
        env.insert(name.clone(), rug::Complex::with_val(64, (v, 0.0)));
    }
    let v = numeric::evalf_env(f, 64, &env).ok()?;
    let (re, im) = (v.real().to_f64(), v.imag().to_f64());
    (im.abs() <= 1e-12 * (re.abs() + 1.0) && re.is_finite()).then_some(re)
}

/// The faces where values on a grid of space pass through zero: every cell is cut into six
/// pieces of four corners, and every piece gives a triangle or a face of four corners.
fn zero_faces(
    values: &dyn Fn(usize, usize, usize) -> Option<f64>,
    cells: [usize; 3],
    place: &dyn Fn(usize, usize, usize) -> Point,
) -> Vec<Option<Point>> {
    const PIECES: [[usize; 4]; 6] = [
        [0, 1, 3, 7],
        [0, 1, 5, 7],
        [0, 2, 3, 7],
        [0, 2, 6, 7],
        [0, 4, 5, 7],
        [0, 4, 6, 7],
    ];
    let mut out = Vec::new();
    for k in 0..cells[2] {
        for j in 0..cells[1] {
            for i in 0..cells[0] {
                let corner = |c: usize| (i + (c & 1), j + ((c >> 1) & 1), k + ((c >> 2) & 1));
                let found: Vec<Option<(f64, Point)>> = (0..8)
                    .map(|c| {
                        let (x, y, z) = corner(c);
                        values(x, y, z).map(|v| (v, place(x, y, z)))
                    })
                    .collect();
                for piece in PIECES {
                    let Some(ends) = piece.iter().map(|c| found[*c]).collect::<Option<Vec<_>>>()
                    else {
                        continue;
                    };
                    let cut = |p: usize, q: usize| -> Point {
                        let ((a, from), (b, to)) = (ends[p], ends[q]);
                        let share = a / (a - b);
                        [
                            from[0] + share * (to[0] - from[0]),
                            from[1] + share * (to[1] - from[1]),
                            from[2] + share * (to[2] - from[2]),
                        ]
                    };
                    let below: Vec<usize> = (0..4).filter(|c| ends[*c].0 < 0.0).collect();
                    let above: Vec<usize> = (0..4).filter(|c| ends[*c].0 >= 0.0).collect();
                    match (below.len(), above.len()) {
                        (1, 3) => {
                            let p = below[0];
                            let last = cut(p, above[2]);
                            quad(&mut out, [cut(p, above[0]), cut(p, above[1]), last, last]);
                        }
                        (3, 1) => {
                            let p = above[0];
                            let last = cut(p, below[2]);
                            quad(&mut out, [cut(p, below[0]), cut(p, below[1]), last, last]);
                        }
                        (2, 2) => quad(
                            &mut out,
                            [
                                cut(below[0], above[0]),
                                cut(below[0], above[1]),
                                cut(below[1], above[1]),
                                cut(below[1], above[0]),
                            ],
                        ),
                        _ => {}
                    }
                }
            }
        }
    }
    out
}

/// `fimplicit3(f, x, a, b, y, c, d, z, e, g)`: the surface where a function of three
/// variables is zero.
fn fimplicit3(a: &[Expr], detail: usize) -> Result<Drawing, String> {
    let usage = "fimplicit3(f(x, y, z), x, a, b, y, c, d, z, e, g): the surface where f is zero (an equation f = g will do)";
    let [f, xv, xa, xb, yv, ya, yb, zv, za, zb] = a else {
        return Err(usage.into());
    };
    let f = match f {
        Expr::Rel(crate::expr::Rel::Eq, l, r) => crate::canon::sub((**l).clone(), (**r).clone()),
        other => other.clone(),
    };
    let vars = [variable(xv)?, variable(yv)?, variable(zv)?];
    let ranges = [ends(xa, xb)?, ends(ya, yb)?, ends(za, zb)?];
    let n = (detail / 24).clamp(10, 28);
    let at = |i: usize, (low, high): (f64, f64)| low + (high - low) * i as f64 / n as f64;
    let place = |i: usize, j: usize, k: usize| -> Point {
        [at(i, ranges[0]), at(j, ranges[1]), at(k, ranges[2])]
    };
    let side = n + 1;
    let mut grid = Vec::with_capacity(side * side * side);
    for k in 0..side {
        for j in 0..side {
            for i in 0..side {
                grid.push(value3(&f, &vars, place(i, j, k)));
            }
        }
    }
    let values = |i: usize, j: usize, k: usize| grid[(k * side + j) * side + i];
    let points = zero_faces(&values, [n, n, n], &place);
    if points.is_empty() {
        return Err(
            "fimplicit3: the function is zero nowhere in these ranges, as far as a grid shows"
                .into(),
        );
    }
    let count = points.len() / 6;
    Ok(solid(
        vec![faces(points)],
        ranges,
        format!(
            "the surface where the function is zero, in {count} faces of a grid of {n} cells a side"
        ),
    ))
}

/// `isosurface(values, level)`: the surface where numbers on a grid of space pass a level.
fn isosurface(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "isosurface([matrix, matrix, ...], level): the surface where the numbers of a grid of space (a matrix for every z) pass the level";
    let [data, level] = a else {
        return Err(usage.into());
    };
    let level = number(level).ok_or(usage)?;
    let sheets = data.as_list().filter(|l| l.len() >= 2).ok_or(usage)?;
    let sheets: Vec<Vec<Vec<f64>>> = sheets
        .iter()
        .map(|m| matrix_of(m, "isosurface"))
        .collect::<Result<_, _>>()?;
    let (rows, columns) = (sheets[0].len(), sheets[0][0].len());
    if rows < 2
        || columns < 2
        || sheets
            .iter()
            .any(|m| m.len() != rows || m.iter().any(|r| r.len() != columns))
    {
        return Err(usage.into());
    }
    let values = |i: usize, j: usize, k: usize| Some(sheets[k][j][i] - level);
    let place = |i: usize, j: usize, k: usize| -> Point {
        [i as f64 + 1.0, j as f64 + 1.0, k as f64 + 1.0]
    };
    let points = zero_faces(&values, [columns - 1, rows - 1, sheets.len() - 1], &place);
    if points.is_empty() {
        return Err("isosurface: the numbers pass the level nowhere".into());
    }
    let count = points.len() / 6;
    Ok(solid(
        vec![faces(points)],
        [
            (1.0, columns as f64),
            (1.0, rows as f64),
            (1.0, sheets.len() as f64),
        ],
        format!("the surface of the level {} in {count} faces", label(level)),
    ))
}

/// The triangles of a list of triples of numbers of points, counted from 1.
fn triangles(e: &Expr, points: usize, name: &str) -> Result<Vec<[usize; 3]>, String> {
    let rows = matrix_of(e, name)?;
    rows.iter()
        .map(|r| {
            let id = |v: f64| {
                (v.fract() == 0.0 && v >= 1.0 && v as usize <= points).then(|| v as usize - 1)
            };
            match r[..] {
                [p, q, s] => match (id(p), id(q), id(s)) {
                    (Some(p), Some(q), Some(s)) => Ok([p, q, s]),
                    _ => Err(format!(
                        "{name}: a triangle names three of the {points} points"
                    )),
                },
                _ => Err(format!(
                    "{name}: the triangles are rows of three numbers of points"
                )),
            }
        })
        .collect()
}

/// `triplot(triangles, x, y)`: the sides of triangles of the plane.
fn triplot(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "triplot(triangles, [x values], [y values]) or triplot(triangles, [[x, y], ...]): the triangles as rows of three numbers of points, as delaunay gives them";
    let (tri, rest) = a.split_first().ok_or(usage)?;
    let (xs, ys, more) = xy("triplot", rest)?;
    nothing_more("triplot", more)?;
    let tri = triangles(tri, xs.len(), "triplot")?;
    let mut sides: Vec<(usize, usize)> = Vec::new();
    for t in &tri {
        for (p, q) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let side = (p.min(q), p.max(q));
            if !sides.contains(&side) {
                sides.push(side);
            }
        }
    }
    let pairs = sides
        .iter()
        .map(|(p, q)| [flat(xs[*p], ys[*p]), flat(xs[*q], ys[*q])])
        .collect();
    let mut drawing = Drawing::new(
        Space::Chart {
            x: roomy(span(&xs)),
            y: roomy(span(&ys)),
            equal: true,
        },
        vec![Item::Segments {
            pairs,
            paint: Paint::Curve(0),
            weight: 1.0,
        }],
    );
    drawing.facts = vec![format!(
        "{} triangles with {} sides between {} points",
        tri.len(),
        sides.len(),
        xs.len()
    )];
    Ok(drawing)
}

/// `trisurf(triangles, x, y, z)`: a surface of triangles of space.
fn trisurf(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "trisurf(triangles, [x values], [y values], [z values]): the triangles as rows of three numbers of points";
    let [tri, x, y, z] = a else {
        return Err(usage.into());
    };
    let (xs, ys, zs) = (
        reals(x, "trisurf")?,
        reals(y, "trisurf")?,
        reals(z, "trisurf")?,
    );
    if xs.len() != ys.len() || xs.len() != zs.len() || xs.is_empty() {
        return Err(usage.into());
    }
    let tri = triangles(tri, xs.len(), "trisurf")?;
    let mut points = Vec::new();
    for t in &tri {
        let p = |k: usize| -> Point { [xs[t[k]], ys[t[k]], zs[t[k]]] };
        quad(&mut points, [p(0), p(1), p(2), p(2)]);
    }
    Ok(solid(
        vec![faces(points)],
        [span(&xs), span(&ys), span(&zs)],
        format!(
            "a surface of {} triangles between {} points; z from {} to {}",
            tri.len(),
            xs.len(),
            label(span(&zs).0),
            label(span(&zs).1)
        ),
    ))
}

/// `pareto(values)`: the values as bars from the largest, with their running sum.
fn pareto(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "pareto([values]): bars from the largest value, and the running sum";
    let [values] = a else {
        return Err(usage.into());
    };
    let values = reals(values, "pareto")?;
    if values.is_empty() || values.iter().any(|v| *v < 0.0) {
        return Err("pareto needs values that are not negative".into());
    }
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|p, q| values[*q].total_cmp(&values[*p]).then(p.cmp(q)));
    let mut sum = 0.0;
    let (mut boxes, mut line) = (Vec::new(), Vec::new());
    for (rank, k) in order.iter().enumerate() {
        let x = rank as f64 + 1.0;
        boxes.push([x - 0.4, 0.0, x + 0.4, values[*k]]);
        sum += values[*k];
        line.push(flat(x, sum));
    }
    let dots = line.iter().map(|p| [p[0], p[1]]).collect();
    let mut drawing = Drawing::new(
        Space::Chart {
            x: (0.4, values.len() as f64 + 0.6),
            y: roomy((0.0, if sum > 0.0 { sum } else { 1.0 })),
            equal: false,
        },
        vec![
            Item::Boxes {
                boxes,
                paint: Paint::Curve(0),
            },
            Item::Line {
                points: line,
                paint: Paint::Curve(1),
                weight: 1.0,
            },
            Item::Dots {
                points: dots,
                paint: Paint::Curve(1),
                radius: 2.5,
                units: false,
            },
        ],
    );
    drawing.names = vec!["values".into(), "running sum".into()];
    drawing.facts = vec![format!(
        "{} bars from the largest: the values number {}; their sum is {}",
        values.len(),
        order
            .iter()
            .map(|k| (k + 1).to_string())
            .collect::<Vec<_>>()
            .join(", "),
        label(sum)
    )];
    Ok(drawing)
}

/// Whether a point lies inside a shape with corners: a ray from it crosses the rim an odd
/// number of times.
fn inside(corners: &[[f64; 2]], px: f64, py: f64) -> bool {
    let mut odd = false;
    for (k, p) in corners.iter().enumerate() {
        let q = corners[(k + 1) % corners.len()];
        if (p[1] > py) != (q[1] > py) && px < p[0] + (py - p[1]) * (q[0] - p[0]) / (q[1] - p[1]) {
            odd = !odd;
        }
    }
    odd
}

/// `fill(x, y)`: a shape with corners, filled.
fn fill(a: &[Expr]) -> Result<Drawing, String> {
    let (xs, ys, rest) = xy("fill", a)?;
    nothing_more("fill", rest)?;
    if xs.len() < 3 {
        return Err("fill([x values], [y values]): three corners or more".into());
    }
    let corners: Vec<[f64; 2]> = xs.iter().zip(&ys).map(|(x, y)| [*x, *y]).collect();
    let mut rim: Vec<Point> = corners.iter().map(|p| flat(p[0], p[1])).collect();
    rim.push(rim[0]);
    // the area by the sums of Gauss over the corners
    let twice: f64 = (0..corners.len())
        .map(|k| {
            let (p, q) = (corners[k], corners[(k + 1) % corners.len()]);
            p[0] * q[1] - q[0] * p[1]
        })
        .sum();
    let shape = corners.clone();
    let field = move |px: f64, py: f64| inside(&shape, px, py).then_some(Shade::Level(0.4));
    let mut drawing = Drawing::new(
        chart(span(&xs), span(&ys)),
        vec![
            Item::Field(Box::new(field)),
            Item::Line {
                points: rim,
                paint: Paint::Ink,
                weight: 1.0,
            },
        ],
    );
    drawing.palette = "viridis";
    drawing.facts = vec![format!(
        "a shape of {} corners; its area is {}",
        corners.len(),
        label(twice.abs() / 2.0)
    )];
    Ok(drawing)
}

/// `ribbon(matrix)`: the columns of a matrix as strips of space, one beside the other.
fn ribbon(a: &[Expr]) -> Result<Drawing, String> {
    let [z] = a else {
        return Err("ribbon(matrix): a strip for every column".into());
    };
    let z = matrix_of(z, "ribbon")?;
    if z.len() < 2 {
        return Err("ribbon(matrix): two rows or more".into());
    }
    let mut points = Vec::new();
    for column in 0..z[0].len() {
        let x = column as f64 + 1.0;
        for (row, values) in z.iter().enumerate() {
            let y = row as f64 + 1.0;
            points.extend([
                Some([x - 0.375, y, values[column]]),
                Some([x + 0.375, y, values[column]]),
            ]);
        }
        points.extend([None, None]);
    }
    let all: Vec<f64> = z.iter().flatten().copied().collect();
    Ok(solid(
        vec![faces(points)],
        [
            (0.5, z[0].len() as f64 + 0.5),
            (1.0, z.len() as f64),
            span(&all),
        ],
        format!(
            "{} strips of {} points; z from {} to {}",
            z[0].len(),
            z.len(),
            label(span(&all).0),
            label(span(&all).1)
        ),
    ))
}

/// The standard normal value below which a share of all values lies.
fn normal_place(share: f64) -> f64 {
    let p = 64;
    let z = rug::Complex::with_val(p, (2.0 * share - 1.0, 0.0));
    match crate::zetas::call("erfinv", &[z], p) {
        Some(Some(v)) => v.real().to_f64() * std::f64::consts::SQRT_2,
        _ => f64::NAN,
    }
}

/// `normplot(values)`: sorted values against the places that a normal law gives them.
fn normplot(a: &[Expr]) -> Result<Drawing, String> {
    let [data] = a else {
        return Err(
            "normplot([values]): the sorted values against their places under a normal law".into(),
        );
    };
    let mut values = reals(data, "normplot")?;
    if values.len() < 2 {
        return Err("normplot needs two values or more".into());
    }
    values.sort_by(f64::total_cmp);
    let n = values.len() as f64;
    let places: Vec<f64> = (0..values.len())
        .map(|k| normal_place((k as f64 + 0.5) / n))
        .collect();
    // the line through the quarters of the data and of the law
    let (q1, q3) = (at_share(&values, 0.25), at_share(&values, 0.75));
    let (z1, z3) = (normal_place(0.25), normal_place(0.75));
    let mut items = vec![Item::Dots {
        points: values.iter().zip(&places).map(|(v, z)| [*v, *z]).collect(),
        paint: Paint::Curve(0),
        radius: 2.5,
        units: false,
    }];
    let (low, high) = span(&values);
    if q3 > q1 {
        let slope = (z3 - z1) / (q3 - q1);
        let at = |x: f64| flat(x, z1 + slope * (x - q1));
        items.push(Item::Line {
            points: vec![at(low), at(high)],
            paint: Paint::Curve(1),
            weight: 1.0,
        });
    }
    let mut drawing = Drawing::new(chart((low, high), span(&places)), items);
    drawing.facts = vec![format!(
        "{} values against the standard normal places of their ranks; the line joins the quarters {} and {}",
        values.len(),
        label(q1),
        label(q3)
    )];
    Ok(drawing)
}

/// `parallelplot(matrix)`: every row as a line across an axis for every column.
fn parallelplot(a: &[Expr]) -> Result<Drawing, String> {
    let [data] = a else {
        return Err("parallelplot(matrix): a line for every row, an axis for every column".into());
    };
    let rows = matrix_of(data, "parallelplot")?;
    let columns = rows[0].len();
    if columns < 2 {
        return Err("parallelplot needs two columns or more".into());
    }
    let ranges: Vec<(f64, f64)> = (0..columns)
        .map(|j| span(&rows.iter().map(|r| r[j]).collect::<Vec<_>>()))
        .collect();
    let share = |v: f64, (low, high): (f64, f64)| {
        if high > low {
            (v - low) / (high - low)
        } else {
            0.5
        }
    };
    let mut items: Vec<Item> = vec![Item::Segments {
        pairs: (0..columns)
            .map(|j| [flat(j as f64 + 1.0, 0.0), flat(j as f64 + 1.0, 1.0)])
            .collect(),
        paint: Paint::Ink,
        weight: 0.5,
    }];
    for row in &rows {
        items.push(Item::Line {
            points: row
                .iter()
                .enumerate()
                .map(|(j, v)| flat(j as f64 + 1.0, share(*v, ranges[j])))
                .collect(),
            paint: Paint::Along,
            weight: 1.0,
        });
    }
    let mut drawing = Drawing::new(
        Space::Chart {
            x: (0.8, columns as f64 + 0.2),
            y: (-0.05, 1.05),
            equal: false,
        },
        items,
    );
    drawing.facts = vec![format!(
        "{} rows across {columns} axes, each from its least value to its largest: {}",
        rows.len(),
        ranges
            .iter()
            .map(|(low, high)| format!("{} to {}", label(*low), label(*high)))
            .collect::<Vec<_>>()
            .join("; ")
    )];
    Ok(drawing)
}

/// The values of groups: a list of lists, the columns of a matrix, or one list.
fn groups(e: &Expr, name: &str) -> Result<Vec<Vec<f64>>, String> {
    let items = e
        .as_list()
        .filter(|l| !l.is_empty())
        .ok_or_else(|| format!("{name}([values]) or {name}([[values], [values], ...])"))?;
    match items.iter().all(|g| g.as_list().is_some()) {
        true => items.iter().map(|g| reals(g, name)).collect(),
        false => Ok(vec![reals(e, name)?]),
    }
}

/// `swarmchart(groups)`: the values of every group as dots that step aside for one another.
fn swarmchart(a: &[Expr]) -> Result<Drawing, String> {
    let [data] = a else {
        return Err("swarmchart([values]) or swarmchart([[values], [values], ...])".into());
    };
    let all = groups(data, "swarmchart")?;
    let flat_values: Vec<f64> = all.iter().flatten().copied().collect();
    if flat_values.is_empty() {
        return Err("swarmchart needs values".into());
    }
    let (low, high) = span(&flat_values);
    let reach = if high > low { high - low } else { 1.0 };
    // dots nearer than a 40th of the range step aside by a 25th of the room of a group
    let (near, aside) = (reach / 40.0, 0.04);
    let mut points = Vec::new();
    for (k, values) in all.iter().enumerate() {
        let mut sorted = values.clone();
        sorted.sort_by(f64::total_cmp);
        let mut placed: Vec<(f64, f64)> = Vec::new();
        for v in sorted {
            let mut offset = 0.0f64;
            for step in 0..200u32 {
                let side = if step % 2 == 1 { 1.0 } else { -1.0 };
                offset = side * f64::from(step.div_ceil(2)) * aside;
                let free = placed
                    .iter()
                    .all(|(o, y)| (o - offset).abs() >= aside * 0.99 || (y - v).abs() >= near);
                if free || offset.abs() >= 0.44 {
                    break;
                }
            }
            placed.push((offset, v));
            points.push([k as f64 + 1.0 + offset, v]);
        }
    }
    let mut drawing = Drawing::new(
        Space::Chart {
            x: (0.4, all.len() as f64 + 0.6),
            y: roomy((low, high)),
            equal: false,
        },
        vec![Item::Dots {
            points,
            paint: Paint::Curve(0),
            radius: 2.0,
            units: false,
        }],
    );
    drawing.facts = vec![format!(
        "{} values in {} groups, from {} to {}",
        flat_values.len(),
        all.len(),
        label(low),
        label(high)
    )];
    Ok(drawing)
}

fn bell(z: f64) -> f64 {
    rug::Float::with_val(64, -z * z / 2.0).exp().to_f64()
}

/// The width of the bells of a smooth density, by the rule of Silverman.
fn bell_width(sorted: &[f64]) -> f64 {
    let n = sorted.len() as f64;
    let mean = sorted.iter().sum::<f64>() / n;
    let sd =
        (sorted.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / (n - 1.0).max(1.0)).sqrt();
    let quarters = (at_share(sorted, 0.75) - at_share(sorted, 0.25)) / 1.34;
    let spread = if quarters > 0.0 { sd.min(quarters) } else { sd };
    // n^(-1/5) by roots: the fifth root is the root of the root of n^(4/5)... taken by MPFR
    let shrink = rug::Float::with_val(64, n).root(5).recip().to_f64();
    let width = 0.9 * spread * shrink;
    if width > 0.0 { width } else { 1.0 }
}

/// `violinplot(groups)`: the smooth density of every group, to both sides of its place.
fn violinplot(a: &[Expr]) -> Result<Drawing, String> {
    let [data] = a else {
        return Err("violinplot([values]) or violinplot([[values], [values], ...])".into());
    };
    let all = groups(data, "violinplot")?;
    if all.iter().any(|g| g.len() < 2) {
        return Err("violinplot needs two values or more in every group".into());
    }
    let mut items = Vec::new();
    let mut shapes: Vec<Vec<[f64; 2]>> = Vec::new();
    let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut medians = Vec::new();
    for (k, values) in all.iter().enumerate() {
        let mut sorted = values.clone();
        sorted.sort_by(f64::total_cmp);
        let width = bell_width(&sorted);
        let (from, to) = (
            sorted[0] - 2.0 * width,
            sorted[sorted.len() - 1] + 2.0 * width,
        );
        let steps = 60;
        let density: Vec<(f64, f64)> = (0..=steps)
            .map(|s| {
                let y = from + (to - from) * f64::from(s) / f64::from(steps);
                (y, sorted.iter().map(|v| bell((y - v) / width)).sum::<f64>())
            })
            .collect();
        let most = density.iter().fold(0.0f64, |m, d| m.max(d.1));
        let x = k as f64 + 1.0;
        let mut shape: Vec<[f64; 2]> = density
            .iter()
            .map(|(y, d)| [x + 0.4 * d / most, *y])
            .collect();
        shape.extend(density.iter().rev().map(|(y, d)| [x - 0.4 * d / most, *y]));
        let mut rim: Vec<Point> = shape.iter().map(|p| flat(p[0], p[1])).collect();
        rim.push(rim[0]);
        items.push(Item::Line {
            points: rim,
            paint: Paint::Ink,
            weight: 1.0,
        });
        let median = at_share(&sorted, 0.5);
        medians.push(median);
        items.push(Item::Segments {
            pairs: vec![[flat(x - 0.15, median), flat(x + 0.15, median)]],
            paint: Paint::Ink,
            weight: 2.0,
        });
        (low, high) = (low.min(from), high.max(to));
        shapes.push(shape);
    }
    let count = shapes.len();
    let field = move |px: f64, py: f64| -> Option<Shade> {
        let k = shapes.iter().position(|s| inside(s, px, py))?;
        Some(Shade::Level(if count > 1 {
            0.35 + 0.55 * k as f64 / (count - 1) as f64
        } else {
            0.5
        }))
    };
    items.insert(0, Item::Field(Box::new(field)));
    let mut drawing = Drawing::new(
        Space::Chart {
            x: (0.4, all.len() as f64 + 0.6),
            y: roomy((low, high)),
            equal: false,
        },
        items,
    );
    drawing.palette = "viridis";
    drawing.facts = vec![format!(
        "the densities of {} groups; their medians: {}",
        all.len(),
        medians
            .iter()
            .map(|m| label(*m))
            .collect::<Vec<_>>()
            .join(", ")
    )];
    Ok(drawing)
}

/// `histfit(values[, bins])`: the counts of the values in bins, and the normal law of their
/// mean and spread.
fn histfit(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "histfit([values][, number of bins])";
    let (values, bins) = match a {
        [v] => (reals(v, "histfit")?, None),
        [v, n] => (
            reals(v, "histfit")?,
            Some(
                number(n)
                    .filter(|n| *n >= 1.0 && *n <= 1000.0 && n.fract() == 0.0)
                    .ok_or(usage)? as usize,
            ),
        ),
        _ => return Err(usage.into()),
    };
    if values.len() < 2 {
        return Err("histfit needs two values or more".into());
    }
    let n = values.len() as f64;
    // (the root of the number of values, rounded up, as MATLAB takes it)
    let bins = bins.unwrap_or_else(|| n.sqrt().ceil() as usize);
    let (low, high) = span(&values);
    let high = if high > low { high } else { low + 1.0 };
    let wide = (high - low) / bins as f64;
    let mut counts = vec![0u32; bins];
    for v in &values {
        counts[(((v - low) / wide) as usize).min(bins - 1)] += 1;
    }
    let mean = values.iter().sum::<f64>() / n;
    let sd = (values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / (n - 1.0)).sqrt();
    let boxes = counts
        .iter()
        .enumerate()
        .map(|(k, c)| {
            [
                low + wide * k as f64,
                0.0,
                low + wide * (k as f64 + 1.0),
                f64::from(*c),
            ]
        })
        .collect();
    let mut items = vec![Item::Boxes {
        boxes,
        paint: Paint::Curve(0),
    }];
    let mut top = f64::from(counts.iter().copied().max().unwrap_or(1));
    if sd > 0.0 {
        let scale = n * wide / (sd * (2.0 * std::f64::consts::PI).sqrt());
        let curve: Vec<Point> = (0..=120)
            .map(|s| {
                let x = low - wide + (high - low + 2.0 * wide) * f64::from(s) / 120.0;
                flat(x, scale * bell((x - mean) / sd))
            })
            .collect();
        top = top.max(scale);
        items.push(Item::Line {
            points: curve,
            paint: Paint::Curve(1),
            weight: 1.5,
        });
    }
    let mut drawing = Drawing::new(
        Space::Chart {
            x: roomy((low - wide, high + wide)),
            y: roomy((0.0, top)),
            equal: false,
        },
        items,
    );
    drawing.names = vec!["counts".into(), "normal law".into()];
    drawing.facts = vec![format!(
        "{} values in {bins} bins: {}; the normal law of the mean {} and the spread {}",
        values.len(),
        counts
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", "),
        label(mean),
        label(sd)
    )];
    Ok(drawing)
}

/// `gscatter(x, y, groups)`: dots in the colour of their group.
fn gscatter(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "gscatter([x values], [y values], [groups]): a number or a name of its group for every point";
    let [x, y, marks] = a else {
        return Err(usage.into());
    };
    let (xs, ys) = (reals(x, "gscatter")?, reals(y, "gscatter")?);
    let marks = marks
        .as_list()
        .filter(|m| m.len() == xs.len())
        .ok_or(usage)?;
    if xs.len() != ys.len() || xs.is_empty() {
        return Err(usage.into());
    }
    let mut kinds: Vec<&Expr> = Vec::new();
    for m in marks {
        if !kinds.contains(&m) {
            kinds.push(m);
        }
    }
    kinds.sort_by(|p, q| crate::arrays::order_any(p, q));
    let items = kinds
        .iter()
        .enumerate()
        .map(|(k, kind)| Item::Dots {
            points: (0..xs.len())
                .filter(|i| marks[*i] == **kind)
                .map(|i| [xs[i], ys[i]])
                .collect(),
            paint: Paint::Curve(k),
            radius: 2.5,
            units: false,
        })
        .collect();
    let names: Vec<String> = kinds
        .iter()
        .map(|k| match crate::radix::text_of(k) {
            Some(t) => t.to_string(),
            None => format!("{k}"),
        })
        .collect();
    let mut drawing = Drawing::new(chart(span(&xs), span(&ys)), items);
    drawing.facts = vec![format!(
        "{} points in {} groups: {}",
        xs.len(),
        names.len(),
        names.join(", ")
    )];
    drawing.names = names;
    Ok(drawing)
}

/// `andrewsplot(matrix)`: every row as a curve of sines and cosines with its entries for
/// weights.
fn andrewsplot(a: &[Expr]) -> Result<Drawing, String> {
    let [data] = a else {
        return Err("andrewsplot(matrix): a curve for every row".into());
    };
    let rows = matrix_of(data, "andrewsplot")?;
    let steps = 96u64;
    let mut items = Vec::new();
    let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
    for row in &rows {
        let points: Vec<Point> = (0..=steps)
            .map(|s| {
                // t from -pi to pi: the turn of s / steps less a half turn
                let mut sum = row[0] / std::f64::consts::SQRT_2;
                for (k, v) in row.iter().enumerate().skip(1) {
                    let wave_number = k.div_ceil(2) as u64;
                    let (sine, cosine) =
                        turn(((s * wave_number + steps * wave_number / 2) % steps, steps));
                    sum += v * if k % 2 == 1 { sine } else { cosine };
                }
                let t =
                    -std::f64::consts::PI + 2.0 * std::f64::consts::PI * s as f64 / steps as f64;
                (low, high) = (low.min(sum), high.max(sum));
                flat(t, sum)
            })
            .collect();
        items.push(Item::Line {
            points,
            paint: Paint::Along,
            weight: 1.0,
        });
    }
    let mut drawing = Drawing::new(
        chart((-std::f64::consts::PI, std::f64::consts::PI), (low, high)),
        items,
    );
    drawing.facts = vec![format!(
        "{} curves x1/sqrt(2) + x2*sin(t) + x3*cos(t) + x4*sin(2t) + ... for t from -pi to pi; values from {} to {}",
        rows.len(),
        label(low),
        label(high)
    )];
    Ok(drawing)
}

/// `bar3h(matrix)`: the bars of `bar3` lying along x.
fn bar3h(a: &[Expr]) -> Result<Drawing, String> {
    let [z] = a else {
        return Err("bar3h(matrix of lengths): a lying bar for every entry".into());
    };
    let z = matrix_of(z, "bar3h")?;
    let mut points = Vec::new();
    for (j, row) in z.iter().enumerate() {
        for (i, h) in row.iter().enumerate() {
            let (y, up) = (i as f64 + 1.0, j as f64 + 1.0);
            let (from, to) = (h.min(0.0), h.max(0.0));
            let corner = |c: usize, x: f64| -> Point {
                [
                    x,
                    if c == 1 || c == 2 { y + 0.4 } else { y - 0.4 },
                    if c >= 2 { up + 0.4 } else { up - 0.4 },
                ]
            };
            for x in [from, to] {
                quad(
                    &mut points,
                    [corner(0, x), corner(1, x), corner(2, x), corner(3, x)],
                );
            }
            for c in 0..4 {
                let d = (c + 1) % 4;
                quad(
                    &mut points,
                    [
                        corner(c, from),
                        corner(d, from),
                        corner(d, to),
                        corner(c, to),
                    ],
                );
            }
        }
    }
    let all: Vec<f64> = z.iter().flatten().copied().collect();
    Ok(solid(
        vec![faces(points)],
        [
            with_zero(span(&all)),
            (0.5, z[0].len() as f64 + 0.5),
            (0.5, z.len() as f64 + 0.5),
        ],
        format!(
            "{} lying bars in {} rows; lengths from {} to {}",
            all.len(),
            z.len(),
            label(span(&all).0),
            label(span(&all).1)
        ),
    ))
}

/// `cdfplot(values)`: the share of the values at or below every value, in steps.
fn cdfplot(a: &[Expr]) -> Result<Drawing, String> {
    let [data] = a else {
        return Err("cdfplot([values]): the share of the values at or below every value".into());
    };
    let mut values = reals(data, "cdfplot")?;
    if values.is_empty() {
        return Err("cdfplot needs values".into());
    }
    values.sort_by(f64::total_cmp);
    let n = values.len() as f64;
    let (low, high) = span(&values);
    let rim = if high > low { 0.05 * (high - low) } else { 0.5 };
    let mut points = vec![flat(low - rim, 0.0)];
    for (k, v) in values.iter().enumerate() {
        points.push(flat(*v, k as f64 / n));
        points.push(flat(*v, (k as f64 + 1.0) / n));
    }
    points.push(flat(high + rim, 1.0));
    let mut drawing = Drawing::new(
        Space::Chart {
            x: (low - rim, high + rim),
            y: (-0.05, 1.05),
            equal: false,
        },
        vec![Item::Line {
            points,
            paint: Paint::Curve(0),
            weight: 1.0,
        }],
    );
    drawing.facts = vec![format!(
        "the shares of {} values from {} to {}; half of them at or below {}",
        values.len(),
        label(low),
        label(high),
        label(at_share(&values, 0.5))
    )];
    Ok(drawing)
}

/// Cells of a grid of charts in one drawing: a value of a cell goes to its share of the cell.
fn cell(column: usize, row: usize, rows: usize, share: (f64, f64)) -> [f64; 2] {
    [
        column as f64 + 0.08 + 0.84 * share.0,
        (rows - 1 - row) as f64 + 0.08 + 0.84 * share.1,
    ]
}

fn frames(columns: usize, rows: usize) -> Item {
    let mut pairs = Vec::new();
    for i in 0..columns {
        for j in 0..rows {
            let (x, y) = (i as f64 + 0.04, j as f64 + 0.04);
            let corners = [
                flat(x, y),
                flat(x + 0.92, y),
                flat(x + 0.92, y + 0.92),
                flat(x, y + 0.92),
            ];
            for k in 0..4 {
                pairs.push([corners[k], corners[(k + 1) % 4]]);
            }
        }
    }
    Item::Segments {
        pairs,
        paint: Paint::Ink,
        weight: 0.5,
    }
}

/// `plotmatrix(matrix)`: every column against every other, a chart for each pair; the
/// counts of a column in its own cell.
fn plotmatrix(a: &[Expr]) -> Result<Drawing, String> {
    let [data] = a else {
        return Err("plotmatrix(matrix): a chart for every pair of columns".into());
    };
    let rows = matrix_of(data, "plotmatrix")?;
    let n = rows[0].len();
    if n > 12 {
        return Err("plotmatrix takes twelve columns at most".into());
    }
    let ranges: Vec<(f64, f64)> = (0..n)
        .map(|j| span(&rows.iter().map(|r| r[j]).collect::<Vec<_>>()))
        .collect();
    let share = |v: f64, (low, high): (f64, f64)| {
        if high > low {
            (v - low) / (high - low)
        } else {
            0.5
        }
    };
    let mut dots = Vec::new();
    let mut boxes = Vec::new();
    for i in 0..n {
        for j in 0..n {
            if i == j {
                // the counts of the column in eight bins
                let mut counts = [0u32; 8];
                for r in &rows {
                    counts[((share(r[i], ranges[i]) * 8.0) as usize).min(7)] += 1;
                }
                let most = f64::from(counts.iter().copied().max().unwrap_or(1).max(1));
                for (k, c) in counts.iter().enumerate() {
                    let from = cell(j, i, n, (k as f64 / 8.0, 0.0));
                    let to = cell(j, i, n, ((k as f64 + 1.0) / 8.0, f64::from(*c) / most));
                    boxes.push([from[0], from[1], to[0], to[1]]);
                }
            } else {
                for r in &rows {
                    dots.push(cell(
                        j,
                        i,
                        n,
                        (share(r[j], ranges[j]), share(r[i], ranges[i])),
                    ));
                }
            }
        }
    }
    let mut items = vec![
        frames(n, n),
        Item::Boxes {
            boxes,
            paint: Paint::Curve(1),
        },
        Item::Dots {
            points: dots,
            paint: Paint::Curve(0),
            radius: 1.5,
            units: false,
        },
    ];
    for j in 0..n {
        items.push(Item::Label {
            at: flat(j as f64 + 0.5, n as f64 + 0.12),
            text: format!("column {}", j + 1),
        });
    }
    let mut drawing = Drawing::new(
        Space::Chart {
            x: (0.0, n as f64),
            y: (0.0, n as f64 + 0.3),
            equal: false,
        },
        items,
    );
    drawing.axes = false;
    drawing.aspect = Some(1.0);
    drawing.facts = vec![format!(
        "{n} columns of {} rows, each against each: the column of a chart runs along x, its row along y; from the least value to the largest: {}",
        rows.len(),
        ranges
            .iter()
            .map(|(low, high)| format!("{} to {}", label(*low), label(*high)))
            .collect::<Vec<_>>()
            .join("; ")
    )];
    Ok(drawing)
}

/// `stackedplot(matrix)`: the columns as lines, each in a chart of its own, one above the
/// other.
fn stackedplot(a: &[Expr]) -> Result<Drawing, String> {
    let [data] = a else {
        return Err("stackedplot(matrix): a chart for every column, one above the other".into());
    };
    let rows = matrix_of(data, "stackedplot")?;
    let n = rows[0].len();
    if rows.len() < 2 || n > 24 {
        return Err("stackedplot needs two rows or more and 24 columns at most".into());
    }
    let ranges: Vec<(f64, f64)> = (0..n)
        .map(|j| span(&rows.iter().map(|r| r[j]).collect::<Vec<_>>()))
        .collect();
    let share = |v: f64, (low, high): (f64, f64)| {
        if high > low {
            (v - low) / (high - low)
        } else {
            0.5
        }
    };
    let mut items = vec![frames(1, n)];
    for j in 0..n {
        items.push(Item::Line {
            points: rows
                .iter()
                .enumerate()
                .map(|(k, r)| {
                    let p = cell(
                        0,
                        j,
                        n,
                        (k as f64 / (rows.len() - 1) as f64, share(r[j], ranges[j])),
                    );
                    flat(p[0], p[1])
                })
                .collect(),
            paint: Paint::Curve(j),
            weight: 1.0,
        });
    }
    let mut drawing = Drawing::new(
        Space::Chart {
            x: (0.0, 1.0),
            y: (0.0, n as f64),
            equal: false,
        },
        items,
    );
    drawing.axes = false;
    drawing.facts = vec![format!(
        "{n} columns of {} rows from the top, each from its least value to its largest: {}",
        rows.len(),
        ranges
            .iter()
            .map(|(low, high)| format!("{} to {}", label(*low), label(*high)))
            .collect::<Vec<_>>()
            .join("; ")
    )];
    Ok(drawing)
}

/// `biplot(weights)`: an arrow from the origin for every variable, by its weights in two
/// directions (the first two columns of what `pca` gives).
fn biplot(a: &[Expr]) -> Result<Drawing, String> {
    let [data] = a else {
        return Err("biplot(matrix): a row of two weights for every variable".into());
    };
    let rows = matrix_of(data, "biplot")?;
    if rows[0].len() < 2 {
        return Err("biplot needs two columns or more".into());
    }
    let mut pairs = Vec::new();
    let mut items = Vec::new();
    let reach = rows
        .iter()
        .fold(0.0f64, |m, r| m.max(r[0].abs()).max(r[1].abs()))
        .max(1e-12);
    for (k, r) in rows.iter().enumerate() {
        pairs.push([flat(0.0, 0.0), flat(r[0], r[1])]);
        for side in [-1.0, 1.0] {
            let back = [
                -0.12 * r[0] - side * 0.06 * r[1],
                -0.12 * r[1] + side * 0.06 * r[0],
            ];
            pairs.push([flat(r[0], r[1]), flat(r[0] + back[0], r[1] + back[1])]);
        }
        items.push(Item::Label {
            at: flat(1.12 * r[0], 1.12 * r[1]),
            text: (k + 1).to_string(),
        });
    }
    items.insert(
        0,
        Item::Segments {
            pairs,
            paint: Paint::Curve(0),
            weight: 1.0,
        },
    );
    let mut drawing = Drawing::new(
        Space::Chart {
            x: (-1.3 * reach, 1.3 * reach),
            y: (-1.3 * reach, 1.3 * reach),
            equal: true,
        },
        items,
    );
    drawing.aspect = Some(1.0);
    drawing.facts = vec![format!(
        "{} variables as arrows by their two weights; the longest reaches {}",
        rows.len(),
        label(reach)
    )];
    Ok(drawing)
}

/// The charts of systems of control. They get what `Session::control_chart` makes of a
/// system: lists of real numbers.
pub fn is_control_chart(name: &str) -> bool {
    matches!(
        name,
        "bodeplot"
            | "nyquistplot"
            | "pzplot"
            | "rlocusplot"
            | "stepplot"
            | "impulseplot"
            | "initialplot"
            | "nicholsplot"
            | "zplane"
    )
}

/// The logarithm to base 10, by the same rounding on every machine.
fn log10_of(x: f64) -> f64 {
    rug::Float::with_val(64, x).log10().to_f64()
}

/// Crosses at points: two short lines each.
fn crosses(at: &[(f64, f64)], arm: f64) -> Item {
    let mut pairs = Vec::new();
    for (x, y) in at {
        pairs.push([flat(x - arm, y - arm), flat(x + arm, y + arm)]);
        pairs.push([flat(x - arm, y + arm), flat(x + arm, y - arm)]);
    }
    Item::Segments {
        pairs,
        paint: Paint::Curve(1),
        weight: 1.5,
    }
}

/// Circles around points, each of `parts` short lines.
fn circles(at: &[(f64, f64)], radius: f64, paint: Paint, weight: f64, parts: u64) -> Item {
    let mut pairs = Vec::new();
    for (x, y) in at {
        let corners: Vec<Point> = (0..=parts)
            .map(|k| {
                let (s, c) = turn((k % parts, parts));
                flat(x + radius * c, y + radius * s)
            })
            .collect();
        pairs.extend(corners.windows(2).map(|w| [w[0], w[1]]));
    }
    Item::Segments {
        pairs,
        paint,
        weight,
    }
}

fn pairs_of(re: &Expr, im: &Expr, name: &str) -> Result<Vec<(f64, f64)>, String> {
    Ok(reals(re, name)?.into_iter().zip(reals(im, name)?).collect())
}

/// A square of the plane around points and the origin: both ranges and the length of the
/// arm of a mark.
fn plane_of(points: &[(f64, f64)]) -> ((f64, f64), (f64, f64), f64) {
    let (mut x, mut y) = ((0.0f64, 0.0f64), (0.0f64, 0.0f64));
    for (re, im) in points {
        x = (x.0.min(*re), x.1.max(*re));
        y = (y.0.min(*im), y.1.max(*im));
    }
    let reach = (x.1 - x.0).max(y.1 - y.0).max(1e-9);
    let half = 0.58 * reach;
    let around = |(low, high): (f64, f64)| {
        let middle = (low + high) / 2.0;
        (middle - half, middle + half)
    };
    (around(x), around(y), 0.012 * reach)
}

/// "1 pole", "3 poles".
fn counted(count: usize, one: &str) -> String {
    match count {
        1 => format!("1 {one}"),
        _ => format!("{count} {one}s"),
    }
}

fn cross_lines(x: (f64, f64), y: (f64, f64)) -> Item {
    Item::Segments {
        pairs: vec![
            [flat(x.0, 0.0), flat(x.1, 0.0)],
            [flat(0.0, y.0), flat(0.0, y.1)],
        ],
        paint: Paint::Ink,
        weight: 0.5,
    }
}

/// `bodeplot(system)`: the gain in decibels above, the phase in degrees below, both along
/// the frequency by powers of ten.
fn bodeplot(a: &[Expr]) -> Result<Drawing, String> {
    let name = "bodeplot";
    let [ws, gains, phases] = a else {
        return Err("bodeplot(system[, [frequencies]])".into());
    };
    let (ws, gains, phases) = (reals(ws, name)?, reals(gains, name)?, reals(phases, name)?);
    if ws.len() < 2 || ws.iter().any(|w| *w <= 0.0) {
        return Err("bodeplot needs two frequencies or more, all above zero".into());
    }
    let xs: Vec<f64> = ws.iter().map(|w| log10_of(*w)).collect();
    let decibels: Vec<Option<f64>> = gains
        .iter()
        .map(|g| (*g > 0.0).then(|| 20.0 * log10_of(*g)))
        .collect();
    let known: Vec<f64> = decibels.iter().flatten().copied().collect();
    if known.is_empty() {
        return Err("bodeplot: the gain is zero at every frequency".into());
    }
    let wide = |(low, high): (f64, f64)| {
        if high > low {
            (low, high)
        } else {
            (low - 1.0, high + 1.0)
        }
    };
    let (x_range, ranges) = (span(&xs), [wide(span(&known)), wide(span(&phases))]);
    let share = |v: f64, (low, high): (f64, f64)| (v - low) / (high - low);
    let place = |row: usize, x: f64, y: f64| {
        let p = cell(0, row, 2, (x, y));
        flat(p[0], p[1])
    };
    let mut items = vec![frames(1, 2)];
    // a line at every power of ten, and where the gain is 1 and the phase half a turn back
    let mut pairs = Vec::new();
    for power in (x_range.0.ceil() as i64)..=(x_range.1.floor() as i64) {
        let x = share(power as f64, x_range);
        for row in 0..2 {
            pairs.push([place(row, x, 0.0), place(row, x, 1.0)]);
        }
        items.push(Item::Label {
            at: flat(place(1, x, 0.0)[0], 0.0),
            text: format!("1e{power}"),
        });
    }
    for (row, level) in [(0, 0.0), (1, -180.0)] {
        if level > ranges[row].0 && level < ranges[row].1 {
            let y = share(level, ranges[row]);
            pairs.push([place(row, 0.0, y), place(row, 1.0, y)]);
        }
    }
    items.push(Item::Segments {
        pairs,
        paint: Paint::Ink,
        weight: 0.4,
    });
    let mut piece = Vec::new();
    for (x, value) in xs.iter().zip(&decibels) {
        match value {
            Some(v) => piece.push(place(0, share(*x, x_range), share(*v, ranges[0]))),
            None if piece.len() > 1 => items.push(Item::Line {
                points: std::mem::take(&mut piece),
                paint: Paint::Curve(0),
                weight: 1.0,
            }),
            None => piece.clear(),
        }
    }
    if piece.len() > 1 {
        items.push(Item::Line {
            points: piece,
            paint: Paint::Curve(0),
            weight: 1.0,
        });
    }
    items.push(Item::Line {
        points: xs
            .iter()
            .zip(&phases)
            .map(|(x, v)| place(1, share(*x, x_range), share(*v, ranges[1])))
            .collect(),
        paint: Paint::Curve(1),
        weight: 1.0,
    });
    for (row, unit) in [(0, "dB"), (1, "degrees")] {
        for (edge, value) in [(0.96, ranges[row].1), (0.04, ranges[row].0)] {
            items.push(Item::Label {
                at: place(row, 0.09, edge),
                text: format!("{} {unit}", label(value)),
            });
        }
    }
    let mut drawing = Drawing::new(
        Space::Chart {
            x: (0.0, 1.0),
            y: (-0.06, 2.0),
            equal: false,
        },
        items,
    );
    drawing.axes = false;
    drawing.facts = vec![format!(
        "{} frequencies from {} to {} by powers of ten; above the gain from {} dB to {} dB, below the phase from {} to {} degrees",
        ws.len(),
        label(ws[0]),
        label(ws[ws.len() - 1]),
        label(ranges[0].0),
        label(ranges[0].1),
        label(ranges[1].0),
        label(ranges[1].1)
    )];
    Ok(drawing)
}

/// `nyquistplot(system)`: the response in the plane as the frequency grows, its mirror for
/// the frequencies below zero, and a cross at -1.
fn nyquistplot(a: &[Expr]) -> Result<Drawing, String> {
    let [re, im] = a else {
        return Err("nyquistplot(system[, [frequencies]])".into());
    };
    let points = pairs_of(re, im, "nyquistplot")?;
    if points.len() < 2 {
        return Err("nyquistplot needs two frequencies or more".into());
    }
    let mut all = points.clone();
    all.extend(points.iter().map(|(x, y)| (*x, -y)));
    all.push((-1.0, 0.0));
    let (x, y, arm) = plane_of(&all);
    let nearest = points
        .iter()
        .map(|(re, im)| ((re + 1.0) * (re + 1.0) + im * im).sqrt())
        .fold(f64::INFINITY, f64::min);
    let line = |sign: f64, paint: Paint| Item::Line {
        points: points.iter().map(|(re, im)| flat(*re, sign * im)).collect(),
        paint,
        weight: 1.0,
    };
    let mut drawing = Drawing::new(
        Space::Chart { x, y, equal: true },
        vec![
            cross_lines(x, y),
            line(-1.0, Paint::Curve(2)),
            line(1.0, Paint::Curve(0)),
            crosses(&[(-1.0, 0.0)], arm),
        ],
    );
    drawing.aspect = Some(1.0);
    drawing.facts = vec![format!(
        "the response at {} frequencies and its mirror; a cross at -1, which the response comes as near as {}",
        points.len(),
        label(nearest)
    )];
    Ok(drawing)
}

/// `nicholsplot(system)`: the gain in decibels against the phase in degrees as the frequency
/// grows, and a cross where the phase is -180 degrees and the gain 1.
fn nicholsplot(a: &[Expr]) -> Result<Drawing, String> {
    let name = "nicholsplot";
    let [_, gains, phases] = a else {
        return Err("nicholsplot(system[, [frequencies]])".into());
    };
    let (gains, phases) = (reals(gains, name)?, reals(phases, name)?);
    let points: Vec<(f64, f64)> = phases
        .iter()
        .zip(&gains)
        .filter(|(_, gain)| **gain > 0.0)
        .map(|(phase, gain)| (*phase, 20.0 * log10_of(*gain)))
        .collect();
    if points.len() < 2 {
        return Err("nicholsplot needs two frequencies or more where the gain is not zero".into());
    }
    let wide = |(low, high): (f64, f64)| {
        if high > low {
            (low, high)
        } else {
            (low - 1.0, high + 1.0)
        }
    };
    let phase_span = wide(span(&points.iter().map(|p| p.0).chain([-180.0]).collect::<Vec<_>>()));
    let gain_span = wide(span(&points.iter().map(|p| p.1).chain([0.0]).collect::<Vec<_>>()));
    let (across, upwards) = ((phase_span.1 - phase_span.0) / 90.0, (gain_span.1 - gain_span.0) / 50.0);
    let mut drawing = Drawing::new(
        chart(phase_span, gain_span),
        vec![
            Item::Line {
                points: points.iter().map(|(phase, gain)| flat(*phase, *gain)).collect(),
                paint: Paint::Curve(0),
                weight: 1.0,
            },
            Item::Segments {
                pairs: vec![
                    [flat(-180.0 - across, -upwards), flat(-180.0 + across, upwards)],
                    [flat(-180.0 - across, upwards), flat(-180.0 + across, -upwards)],
                ],
                paint: Paint::Curve(1),
                weight: 1.5,
            },
        ],
    );
    drawing.facts = vec![format!(
        "the gain from {} dB to {} dB against the phase from {} to {} degrees at {} frequencies; a cross at -180 degrees and 0 dB",
        label(gain_span.0),
        label(gain_span.1),
        label(phase_span.0),
        label(phase_span.1),
        points.len()
    )];
    Ok(drawing)
}

/// `pzplot(system)`: a cross at every pole, a circle at every zero; for a sampled system
/// the unit circle too.
fn pzplot(a: &[Expr]) -> Result<Drawing, String> {
    let name = "pzplot";
    let [pole_re, pole_im, zero_re, zero_im, sampled] = a else {
        return Err("pzplot(system)".into());
    };
    let (poles, zeros) = (
        pairs_of(pole_re, pole_im, name)?,
        pairs_of(zero_re, zero_im, name)?,
    );
    let sampled = *sampled == Expr::Bool(true);
    let mut all: Vec<(f64, f64)> = poles.iter().chain(&zeros).copied().collect();
    if sampled {
        all.extend([(1.0, 1.0), (-1.0, -1.0)]);
    }
    if all.is_empty() {
        all.push((1.0, 1.0));
    }
    let (x, y, arm) = plane_of(&all);
    let mut items = vec![cross_lines(x, y)];
    if sampled {
        items.push(circles(&[(0.0, 0.0)], 1.0, Paint::Ink, 0.5, 240));
    }
    items.push(circles(&zeros, arm, Paint::Curve(0), 1.5, 24));
    items.push(crosses(&poles, arm));
    let mut drawing = Drawing::new(Space::Chart { x, y, equal: true }, items);
    drawing.aspect = Some(1.0);
    let farthest = match sampled {
        true => poles
            .iter()
            .map(|(re, im)| (re * re + im * im).sqrt())
            .fold(0.0, f64::max),
        false => poles.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
    };
    drawing.facts = vec![match (poles.is_empty(), sampled) {
        (true, _) => format!("no poles and {} (circles)", counted(zeros.len(), "zero")),
        (false, true) => format!(
            "{} (crosses) and {} (circles) with the unit circle; the pole farthest out at the distance {}",
            counted(poles.len(), "pole"),
            counted(zeros.len(), "zero"),
            label(farthest)
        ),
        (false, false) => format!(
            "{} (crosses) and {} (circles); the pole farthest right has the real part {}",
            counted(poles.len(), "pole"),
            counted(zeros.len(), "zero"),
            label(farthest)
        ),
    }];
    Ok(drawing)
}

/// `rlocusplot(system)`: where the poles of the closed loop go as the gain grows, from the
/// poles (crosses) towards the zeros (circles) or far away.
fn rlocusplot(a: &[Expr]) -> Result<Drawing, String> {
    let name = "rlocusplot";
    let [re, im, pole_re, pole_im, zero_re, zero_im, count] = a else {
        return Err("rlocusplot(system[, [gains]])".into());
    };
    let (roots, poles, zeros) = (
        pairs_of(re, im, name)?,
        pairs_of(pole_re, pole_im, name)?,
        pairs_of(zero_re, zero_im, name)?,
    );
    let gains = number(count).unwrap_or(0.0) as usize;
    // the picture holds the poles, the zeros and the roots that are not much farther away
    let near: Vec<(f64, f64)> = poles.iter().chain(&zeros).copied().collect();
    let (span_x, span_y, _) = plane_of(&near);
    let reach = 2.5 * (span_x.1 - span_x.0).max(span_y.1 - span_y.0);
    let shown: Vec<(f64, f64)> = roots
        .iter()
        .copied()
        .filter(|(x, y)| x.abs() <= reach && y.abs() <= reach)
        .collect();
    let all: Vec<(f64, f64)> = near.iter().chain(&shown).copied().collect();
    let (x, y, arm) = plane_of(&all);
    let mut drawing = Drawing::new(
        Space::Chart { x, y, equal: true },
        vec![
            cross_lines(x, y),
            Item::Dots {
                points: shown.iter().map(|(x, y)| [*x, *y]).collect(),
                paint: Paint::Curve(0),
                radius: 1.2,
                units: false,
            },
            circles(&zeros, arm, Paint::Curve(2), 1.5, 24),
            crosses(&poles, arm),
        ],
    );
    drawing.aspect = Some(1.0);
    drawing.facts = vec![format!(
        "the poles of the closed loop for {gains} gains: from the {} of the system (crosses) towards its {} (circles) or away without end",
        counted(poles.len(), "pole"),
        counted(zeros.len(), "zero")
    )];
    Ok(drawing)
}

/// `stepplot(system)`, `impulseplot(system)`, `initialplot(system, x0)`: the answer along the
/// time; in steps for a sampled system.
fn answerplot(name: &str, a: &[Expr]) -> Result<Drawing, String> {
    let [ts, ys, sampled] = a else {
        return Err(match name {
            "initialplot" => "initialplot(system, x0[, the last time])".to_string(),
            _ => format!("{name}(system[, the last time])"),
        });
    };
    let (ts, ys) = (reals(ts, name)?, reals(ys, name)?);
    if ts.len() < 2 || ts.len() != ys.len() {
        return Err(format!("{name}: the answer has too few values"));
    }
    let sampled = *sampled == Expr::Bool(true);
    let mut points = Vec::new();
    for k in 0..ts.len() {
        if sampled && k > 0 {
            points.push(flat(ts[k], ys[k - 1]));
        }
        points.push(flat(ts[k], ys[k]));
    }
    let (low, high) = span(&ys);
    let mut drawing = Drawing::new(
        chart(span(&ts), with_zero((low, high))),
        vec![Item::Line {
            points,
            paint: Paint::Curve(0),
            weight: 1.0,
        }],
    );
    drawing.facts = vec![format!(
        "the answer {} from {} to {}: values from {} to {}, at the end {}",
        if sampled {
            "at the samples"
        } else {
            "at the times"
        },
        label(ts[0]),
        label(ts[ts.len() - 1]),
        label(low),
        label(high),
        label(ys[ys.len() - 1])
    )];
    Ok(drawing)
}

pub fn build(name: &str, a: &[Expr], detail: usize) -> Result<Drawing, String> {
    match name {
        "bodeplot" => bodeplot(a),
        "nyquistplot" => nyquistplot(a),
        "pzplot" | "zplane" => pzplot(a),
        "rlocusplot" => rlocusplot(a),
        "stepplot" | "impulseplot" | "initialplot" => answerplot(name, a),
        "nicholsplot" => nicholsplot(a),
        "bar3" => bar3(a),
        "histogram2" => histogram2(a),
        "surfc" => surfc(a),
        "streamline" => streamline(a),
        "polarhistogram" => polarhistogram(a),
        "dendrogram" => dendrogram(a),
        "bubblechart" => bubblechart(a),
        "fimplicit3" => fimplicit3(a, detail),
        "isosurface" => isosurface(a),
        "triplot" => triplot(a),
        "trisurf" => trisurf(a),
        "pareto" => pareto(a),
        "fill" => fill(a),
        "ribbon" => ribbon(a),
        "normplot" => normplot(a),
        "parallelplot" => parallelplot(a),
        "swarmchart" => swarmchart(a),
        "violinplot" => violinplot(a),
        "histfit" => histfit(a),
        "gscatter" => gscatter(a),
        "bar3h" => bar3h(a),
        "cdfplot" => cdfplot(a),
        "plotmatrix" => plotmatrix(a),
        "stackedplot" => stackedplot(a),
        "biplot" => biplot(a),
        _ => andrewsplot(a),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sphere_is_cut_into_faces() {
        let side = 9;
        let at = |i: usize| -1.2 + 2.4 * i as f64 / (side - 1) as f64;
        let values = |i: usize, j: usize, k: usize| {
            Some(at(i) * at(i) + at(j) * at(j) + at(k) * at(k) - 1.0)
        };
        let place = |i: usize, j: usize, k: usize| -> Point { [at(i), at(j), at(k)] };
        let points = zero_faces(&values, [side - 1; 3], &place);
        assert!(points.len() >= 6 * 100);
        // every corner of a face lies near the sphere
        for p in points.iter().flatten() {
            let r = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
            assert!((r - 1.0).abs() < 0.08, "{r}");
        }
    }

    #[test]
    fn a_point_is_inside_or_outside_a_shape() {
        let square = [[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]];
        assert!(inside(&square, 1.0, 1.0));
        assert!(!inside(&square, 3.0, 1.0));
        assert!(!inside(&square, 1.0, -0.5));
    }

    #[test]
    fn the_normal_law_has_its_places() {
        assert!(normal_place(0.5).abs() < 1e-15);
        assert!((normal_place(0.975) - 1.959963984540054).abs() < 1e-12);
    }
}
