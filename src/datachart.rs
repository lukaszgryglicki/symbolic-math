//! Charts of data: lines through points, dots, bars, steps, stems, pies, boxes of quartiles,
//! lines of equal height, arrows of a field, and points, curves and surfaces of space.

use crate::chart::{level_lines, roomy};
use crate::expr::Expr;
use crate::plot::{ends, label, number, value2, variable};
use crate::scene::{Drawing, Item, Paint, Point, Shade, Space, turn};

/// The drawings of this module.
pub fn is_chart(name: &str) -> bool {
    matches!(
        name,
        "lineplot"
            | "scatter"
            | "bar"
            | "barh"
            | "stairs"
            | "stem"
            | "errorbar"
            | "pie"
            | "loglog"
            | "semilogx"
            | "semilogy"
            | "contour"
            | "quiver"
            | "plot3"
            | "scatter3"
            | "surf"
            | "spy"
            | "boxplot"
            | "stem3"
            | "quiver3"
            | "contour3"
            | "waterfall"
            | "compass"
            | "feather"
            | "polarscatter"
    )
}

pub(crate) fn reals(e: &Expr, name: &str) -> Result<Vec<f64>, String> {
    e.as_list()
        .ok_or_else(|| format!("{name} needs lists of numbers, {e} is none"))?
        .iter()
        .map(|v| number(v).ok_or_else(|| format!("{name}: {v} is no real number")))
        .collect()
}

fn is_numbers(e: &Expr) -> bool {
    e.as_list()
        .is_some_and(|l| !l.is_empty() && l.iter().all(|v| number(v).is_some()))
}

/// The x and the y values of points, and the arguments that follow them.
pub(crate) type Points<'a> = (Vec<f64>, Vec<f64>, &'a [Expr]);

/// The points of `(y)`, `(x, y)` or `([[x, y], ...])`, and what follows them.
pub(crate) fn xy<'a>(name: &str, a: &'a [Expr]) -> Result<Points<'a>, String> {
    let usage =
        || format!("{name}([y values]), {name}([x values], [y values]) or {name}([[x, y], ...])");
    let first = a.first().ok_or_else(usage)?;
    let items = first
        .as_list()
        .filter(|l| !l.is_empty())
        .ok_or_else(usage)?;
    if items
        .iter()
        .all(|p| p.as_list().is_some_and(|l| l.len() == 2))
    {
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        for p in items {
            let pair = reals(p, name)?;
            xs.push(pair[0]);
            ys.push(pair[1]);
        }
        return Ok((xs, ys, &a[1..]));
    }
    let one = reals(first, name)?;
    match a.get(1) {
        Some(second) if is_numbers(second) => {
            let ys = reals(second, name)?;
            if ys.len() != one.len() {
                return Err(format!(
                    "{name}: {} x values and {} y values",
                    one.len(),
                    ys.len()
                ));
            }
            Ok((one, ys, &a[2..]))
        }
        _ => Ok(((1..=one.len()).map(|k| k as f64).collect(), one, &a[1..])),
    }
}

pub(crate) fn nothing_more(name: &str, rest: &[Expr]) -> Result<(), String> {
    match rest.first() {
        None => Ok(()),
        Some(e) => Err(format!(
            "{e} is one argument too many of {name}; what is chosen for a picture is written as name = value"
        )),
    }
}

pub(crate) fn span(v: &[f64]) -> (f64, f64) {
    (
        v.iter().copied().fold(f64::INFINITY, f64::min),
        v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    )
}

pub(crate) fn with_zero((low, high): (f64, f64)) -> (f64, f64) {
    (low.min(0.0), high.max(0.0))
}

pub(crate) fn chart(x: (f64, f64), y: (f64, f64)) -> Space {
    Space::Chart {
        x: roomy(x),
        y: roomy(y),
        equal: false,
    }
}

pub(crate) fn flat(x: f64, y: f64) -> Point {
    [x, y, 0.0]
}

/// The least distance of two different values, 1 for one value.
pub(crate) fn gap(v: &[f64]) -> f64 {
    let mut sorted = v.to_vec();
    sorted.sort_by(f64::total_cmp);
    let least = sorted
        .windows(2)
        .map(|w| w[1] - w[0])
        .filter(|d| *d > 0.0)
        .fold(f64::INFINITY, f64::min);
    if least.is_finite() { least } else { 1.0 }
}

fn log10(v: &[f64], name: &str) -> Result<Vec<f64>, String> {
    v.iter()
        .map(|x| {
            if *x > 0.0 {
                // (by the same rounding on every machine)
                Ok(rug::Float::with_val(64, *x).log10().to_f64())
            } else {
                Err(format!(
                    "{name}: a logarithmic axis needs positive numbers, {} is none",
                    label(*x)
                ))
            }
        })
        .collect()
}

fn lines(name: &str, a: &[Expr]) -> Result<Drawing, String> {
    let (xs, ys, rest) = xy(name, a)?;
    nothing_more(name, rest)?;
    let (xs, ys) = match name {
        "loglog" => (log10(&xs, name)?, log10(&ys, name)?),
        "semilogx" => (log10(&xs, name)?, ys),
        "semilogy" => (xs, log10(&ys, name)?),
        _ => (xs, ys),
    };
    let mut items = Vec::new();
    match name {
        "scatter" => items.push(Item::Dots {
            points: xs.iter().zip(&ys).map(|(x, y)| [*x, *y]).collect(),
            paint: Paint::Curve(0),
            radius: 2.5,
            units: false,
        }),
        "stairs" => {
            let mut points = Vec::new();
            for k in 0..xs.len() {
                points.push(flat(xs[k], ys[k]));
                if k + 1 < xs.len() {
                    points.push(flat(xs[k + 1], ys[k]));
                }
            }
            items.push(Item::Line {
                points,
                paint: Paint::Curve(0),
                weight: 1.0,
            });
        }
        "stem" => {
            items.push(Item::Segments {
                pairs: xs
                    .iter()
                    .zip(&ys)
                    .map(|(x, y)| [flat(*x, 0.0), flat(*x, *y)])
                    .collect(),
                paint: Paint::Curve(0),
                weight: 1.0,
            });
            items.push(Item::Dots {
                points: xs.iter().zip(&ys).map(|(x, y)| [*x, *y]).collect(),
                paint: Paint::Curve(0),
                radius: 2.5,
                units: false,
            });
        }
        _ => items.push(Item::Line {
            points: xs.iter().zip(&ys).map(|(x, y)| flat(*x, *y)).collect(),
            paint: Paint::Curve(0),
            weight: 1.0,
        }),
    }
    let y = if name == "stem" {
        with_zero(span(&ys))
    } else {
        span(&ys)
    };
    let mut drawing = Drawing::new(chart(span(&xs), y), items);
    drawing.logs = [
        matches!(name, "loglog" | "semilogx"),
        matches!(name, "loglog" | "semilogy"),
    ];
    drawing.facts = vec![match name {
        "loglog" => format!("{} points; both axes go by powers of ten", xs.len()),
        "semilogx" => format!("{} points; the x axis goes by powers of ten", xs.len()),
        "semilogy" => format!("{} points; the y axis goes by powers of ten", xs.len()),
        _ => format!("{} points", xs.len()),
    }];
    Ok(drawing)
}

fn bars(name: &str, a: &[Expr]) -> Result<Drawing, String> {
    let (xs, ys, rest) = xy(name, a)?;
    nothing_more(name, rest)?;
    let half = 0.4 * gap(&xs);
    let boxes: Vec<[f64; 4]> = xs
        .iter()
        .zip(&ys)
        .map(|(x, y)| {
            if name == "barh" {
                [y.min(0.0), x - half, y.max(0.0), x + half]
            } else {
                [x - half, y.min(0.0), x + half, y.max(0.0)]
            }
        })
        .collect();
    let along = {
        let (low, high) = span(&xs);
        (low - 1.5 * half, high + 1.5 * half)
    };
    let values = roomy(with_zero(span(&ys)));
    let space = if name == "barh" {
        Space::Chart {
            x: values,
            y: along,
            equal: false,
        }
    } else {
        Space::Chart {
            x: along,
            y: values,
            equal: false,
        }
    };
    let mut drawing = Drawing::new(
        space,
        vec![Item::Boxes {
            boxes,
            paint: Paint::Curve(0),
        }],
    );
    drawing.facts = vec![format!("{} bars", xs.len())];
    Ok(drawing)
}

fn error_bars(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "errorbar([y values], [errors]) or errorbar([x values], [y values], [errors])";
    let (xs, ys, errs) = match a {
        [y, e] => {
            let ys = reals(y, "errorbar")?;
            (
                (1..=ys.len()).map(|k| k as f64).collect(),
                ys,
                reals(e, "errorbar")?,
            )
        }
        [x, y, e] => (
            reals(x, "errorbar")?,
            reals(y, "errorbar")?,
            reals(e, "errorbar")?,
        ),
        _ => return Err(usage.to_string()),
    };
    if xs.len() != ys.len() || ys.len() != errs.len() || xs.is_empty() {
        return Err(usage.to_string());
    }
    let cap = 0.15 * gap(&xs);
    let mut pairs = Vec::new();
    let (mut lows, mut highs) = (Vec::new(), Vec::new());
    for k in 0..xs.len() {
        let (x, low, high) = (xs[k], ys[k] - errs[k].abs(), ys[k] + errs[k].abs());
        pairs.push([flat(x, low), flat(x, high)]);
        pairs.push([flat(x - cap, low), flat(x + cap, low)]);
        pairs.push([flat(x - cap, high), flat(x + cap, high)]);
        lows.push(low);
        highs.push(high);
    }
    lows.extend(highs);
    let x = {
        let (low, high) = span(&xs);
        (low - 2.0 * cap, high + 2.0 * cap)
    };
    let mut drawing = Drawing::new(
        chart(x, span(&lows)),
        vec![
            Item::Line {
                points: xs.iter().zip(&ys).map(|(x, y)| flat(*x, *y)).collect(),
                paint: Paint::Curve(0),
                weight: 1.0,
            },
            Item::Segments {
                pairs,
                paint: Paint::Curve(0),
                weight: 1.0,
            },
            Item::Dots {
                points: xs.iter().zip(&ys).map(|(x, y)| [*x, *y]).collect(),
                paint: Paint::Curve(0),
                radius: 2.0,
                units: false,
            },
        ],
    );
    drawing.facts = vec![format!("{} points with the bars of their errors", xs.len())];
    Ok(drawing)
}

type Direction = (f64, f64);

/// The direction at a part of a turn, counted from the top against the clock.
fn from_top(part: f64) -> Direction {
    const GRAIN: u64 = 1 << 40;
    let steps = ((part + 0.25).rem_euclid(1.0) * GRAIN as f64).round() as u64;
    let (sine, cosine) = turn((steps % GRAIN, GRAIN));
    (cosine, sine)
}

fn pie(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "pie([values]) or pie([values], [\"names\"])";
    let (data, rest) = a.split_first().ok_or(usage)?;
    let values = reals(data, "pie")?;
    let total: f64 = values.iter().sum();
    if values.is_empty() || values.iter().any(|v| *v < 0.0) || total.is_nan() || total <= 0.0 {
        return Err("pie needs values that are not negative, one above zero".into());
    }
    let names: Vec<String> = match rest {
        [] => values
            .iter()
            .map(|v| format!("{}%", label((1000.0 * v / total).round() / 10.0)))
            .collect(),
        [list] => {
            let items = list
                .as_list()
                .filter(|l| l.len() == values.len())
                .ok_or(usage)?;
            items
                .iter()
                .map(|x| match crate::radix::text_of(x) {
                    Some(t) => t.to_string(),
                    None => format!("{x}"),
                })
                .collect()
        }
        _ => return Err(usage.to_string()),
    };
    // the pieces go against the clock from the top; a wide piece is cut into three, so
    // that a point is in a part when it lies between its two sides
    let mut parts: Vec<(Direction, Direction, usize)> = Vec::new();
    let mut labels = Vec::new();
    let mut at = 0.0;
    for (k, v) in values.iter().enumerate() {
        let share = v / total;
        if share <= 0.0 {
            continue;
        }
        for cut in 0..3 {
            let (from, to) = (
                at + share * cut as f64 / 3.0,
                at + share * (cut + 1) as f64 / 3.0,
            );
            parts.push((from_top(from), from_top(to), k));
        }
        let (c, s) = from_top(at + share / 2.0);
        labels.push(Item::Label {
            at: flat(1.32 * c, 1.22 * s),
            text: names[k].clone(),
        });
        at += share;
    }
    let count = values.len();
    let field = move |px: f64, py: f64| -> Option<Shade> {
        if px * px + py * py > 1.0 {
            return None;
        }
        let cross = |u: (f64, f64), v: (f64, f64)| u.0 * v.1 - u.1 * v.0;
        let piece = parts
            .iter()
            .find(|(from, to, _)| cross(*from, (px, py)) >= 0.0 && cross((px, py), *to) > 0.0)
            .or(parts.first())?;
        Some(Shade::Level(if count > 1 {
            piece.2 as f64 / (count - 1) as f64
        } else {
            0.5
        }))
    };
    let mut items = vec![Item::Field(Box::new(field))];
    items.extend(labels);
    let mut drawing = Drawing::new(
        Space::Chart {
            x: (-2.1, 2.1),
            y: (-1.5, 1.5),
            equal: true,
        },
        items,
    );
    drawing.axes = false;
    drawing.aspect = Some(3.0 / 4.2);
    drawing.facts = vec![format!(
        "{count} parts of the sum {}, from the top against the clock: {}",
        label(total),
        names.join(", ")
    )];
    Ok(drawing)
}

/// The heights of lines: a list of them, a number of them between the least and the largest
/// value, or eight.
pub(crate) fn heights(given: Option<&Expr>, (low, high): (f64, f64)) -> Result<Vec<f64>, String> {
    let even = |count: usize| -> Vec<f64> {
        (1..=count)
            .map(|k| low + (high - low) * k as f64 / (count + 1) as f64)
            .collect()
    };
    match given {
        None => Ok(even(8)),
        Some(e) => match (e.as_list(), e.as_i64()) {
            (Some(_), _) => reals(e, "contour"),
            (None, Some(k)) if (1..=200).contains(&k) => Ok(even(k as usize)),
            _ => Err(
                "contour: the heights are a list of numbers, or how many lines there are".into(),
            ),
        },
    }
}

/// The heights at the corners of `columns` by `rows` cells over the ranges x and y.
struct Heights {
    grid: Vec<Option<f64>>,
    columns: usize,
    rows: usize,
    x: (f64, f64),
    y: (f64, f64),
}

/// The lines of every height that crosses the ranges, and the ranges of x, y and the heights.
type Levels = (Vec<(f64, Vec<[Point; 2]>)>, [(f64, f64); 3]);

fn levels(name: &str, a: &[Expr], detail: usize) -> Result<Levels, String> {
    let usage = format!(
        "{name}(f(x, y), x, a, b, y, c, d), {name}(f, x, a, b, y, c, d, heights) or {name}(matrix)"
    );
    let (
        Heights {
            grid,
            columns,
            rows,
            x,
            y,
        },
        rest,
    ) = match a {
        [f, xv, xa, xb, yv, ya, yb, rest @ ..] if xv.sym_name().is_some() => {
            let vars = (variable(xv)?, variable(yv)?);
            let (x, y) = (ends(xa, xb)?, ends(ya, yb)?);
            let columns = (detail / 4).clamp(32, 300);
            let rows = columns;
            let mut grid = Vec::with_capacity((columns + 1) * (rows + 1));
            for j in 0..=rows {
                for i in 0..=columns {
                    let px = x.0 + (x.1 - x.0) * i as f64 / columns as f64;
                    let py = y.0 + (y.1 - y.0) * j as f64 / rows as f64;
                    grid.push(value2(f, &vars, px, py));
                }
            }
            (
                Heights {
                    grid,
                    columns,
                    rows,
                    x,
                    y,
                },
                rest,
            )
        }
        [m, rest @ ..] => {
            let lines = crate::matrix::rows(m).ok_or(usage.clone())?;
            if lines.len() < 2 || lines[0].len() < 2 {
                return Err(format!("{name}(matrix): two rows and two columns or more"));
            }
            let (rows, columns) = (lines.len() - 1, lines[0].len() - 1);
            let mut grid = Vec::new();
            for row in &lines {
                for v in row {
                    grid.push(Some(
                        number(v).ok_or_else(|| format!("{name}: {v} is no real number"))?,
                    ));
                }
            }
            let heights = Heights {
                grid,
                columns,
                rows,
                x: (1.0, columns as f64 + 1.0),
                y: (1.0, rows as f64 + 1.0),
            };
            (heights, rest)
        }
        _ => return Err(usage),
    };
    if rest.len() > 1 {
        return Err(usage);
    }
    let seen: Vec<f64> = grid
        .iter()
        .flatten()
        .copied()
        .filter(|v| v.is_finite())
        .collect();
    if seen.is_empty() {
        return Err(format!(
            "{name}: the function has no real value in these ranges"
        ));
    }
    let place = |i: f64, j: f64| -> Point {
        [
            x.0 + (x.1 - x.0) * i / columns as f64,
            y.0 + (y.1 - y.0) * j / rows as f64,
            0.0,
        ]
    };
    let mut lines = Vec::new();
    for level in heights(rest.first(), span(&seen))? {
        let at = |i: usize, j: usize| grid[j * (columns + 1) + i].map(|v| v - level);
        let pairs = level_lines(&at, columns, rows, &place);
        if !pairs.is_empty() {
            lines.push((level, pairs));
        }
    }
    if lines.is_empty() {
        return Err(format!(
            "{name}: no line of these heights crosses the ranges"
        ));
    }
    Ok((lines, [x, y, span(&seen)]))
}

fn contour(a: &[Expr], detail: usize) -> Result<Drawing, String> {
    let (lines, [x, y, _]) = levels("contour", a, detail)?;
    let names: Vec<String> = lines.iter().map(|(level, _)| label(*level)).collect();
    let items = lines
        .into_iter()
        .enumerate()
        .map(|(k, (_, pairs))| Item::Segments {
            pairs,
            paint: Paint::Curve(k),
            weight: 1.0,
        })
        .collect();
    let mut drawing = Drawing::new(Space::Chart { x, y, equal: false }, items);
    drawing.facts = vec![format!("the lines of the heights {}", names.join(", "))];
    drawing.names = names;
    Ok(drawing)
}

/// The lines of equal height, each at its height in space.
fn contour3(a: &[Expr], detail: usize) -> Result<Drawing, String> {
    let (lines, [x, y, z]) = levels("contour3", a, detail)?;
    let names: Vec<String> = lines.iter().map(|(level, _)| label(*level)).collect();
    let items = lines
        .into_iter()
        .enumerate()
        .map(|(k, (level, pairs))| Item::Segments {
            pairs: pairs
                .into_iter()
                .map(|[p, q]| [[p[0], p[1], level], [q[0], q[1], level]])
                .collect(),
            paint: Paint::Curve(k),
            weight: 1.0,
        })
        .collect();
    let mut drawing = Drawing::new(Space::Solid, items);
    drawing.stretch = Some([x, y, if z.1 > z.0 { z } else { (z.0, z.0 + 1.0) }]);
    drawing.axes = true;
    drawing.fog = 0.2;
    drawing.facts = vec![format!(
        "the lines of the heights {}, each at its height",
        names.join(", ")
    )];
    Ok(drawing)
}

/// An arrow from a point by a step: its line and the two strokes of its head.
fn arrow(from: [f64; 2], step: [f64; 2], pairs: &mut Vec<[Point; 2]>) -> [f64; 2] {
    let (dx, dy) = (step[0], step[1]);
    let tip = [from[0] + dx, from[1] + dy];
    pairs.push([flat(from[0], from[1]), flat(tip[0], tip[1])]);
    // two strokes of the head, a third of the arrow long
    for side in [-1.0, 1.0] {
        let back = [-0.3 * dx - side * 0.15 * dy, -0.3 * dy + side * 0.15 * dx];
        pairs.push([
            flat(tip[0], tip[1]),
            flat(tip[0] + back[0], tip[1] + back[1]),
        ]);
    }
    tip
}

/// `compass(u, v)`: arrows from the origin; `feather(u, v)`: arrows from 1, 2, 3 ... on the
/// x axis. One list of complex numbers gives both parts.
fn arrows_from(name: &str, a: &[Expr]) -> Result<Drawing, String> {
    let usage = format!("{name}([u values], [v values]) or {name}([complex numbers])");
    let steps: Vec<[f64; 2]> = match a {
        [u, v] => {
            let (u, v) = (reals(u, name)?, reals(v, name)?);
            if u.len() != v.len() {
                return Err(usage);
            }
            u.iter().zip(&v).map(|(p, q)| [*p, *q]).collect()
        }
        [z] => z
            .as_list()
            .ok_or(usage.clone())?
            .iter()
            .map(|w| {
                let part = |which: &str| {
                    number(&crate::funcs::func(which, vec![w.clone()]))
                        .ok_or_else(|| format!("{name}: {w} is no number"))
                };
                Ok([part("re")?, part("im")?])
            })
            .collect::<Result<_, String>>()?,
        _ => return Err(usage),
    };
    if steps.is_empty() {
        return Err(usage);
    }
    let mut pairs = Vec::new();
    let (mut xs, mut ys) = (vec![0.0], vec![0.0]);
    for (k, step) in steps.iter().enumerate() {
        let from = if name == "feather" {
            [k as f64 + 1.0, 0.0]
        } else {
            [0.0, 0.0]
        };
        let tip = arrow(from, *step, &mut pairs);
        xs.extend([from[0], tip[0]]);
        ys.extend([from[1], tip[1]]);
    }
    let space = if name == "compass" {
        // the origin in the middle, one scale
        let reach = xs
            .iter()
            .chain(&ys)
            .fold(0.0f64, |m, v| m.max(v.abs()))
            .max(1e-12)
            * 1.1;
        Space::Chart {
            x: (-reach, reach),
            y: (-reach, reach),
            equal: true,
        }
    } else {
        chart(span(&xs), span(&ys))
    };
    let mut drawing = Drawing::new(
        space,
        vec![Item::Segments {
            pairs,
            paint: Paint::Curve(0),
            weight: 1.0,
        }],
    );
    if name == "compass" {
        drawing.aspect = Some(1.0);
    }
    drawing.facts = vec![format!(
        "{} arrows from {}",
        steps.len(),
        if name == "feather" {
            "the places 1, 2, 3 ... on the x axis"
        } else {
            "the origin"
        }
    )];
    Ok(drawing)
}

/// The cosine and the sine of an angle in radians, by the same rounding on every machine.
pub(crate) fn wave(angle: f64) -> (f64, f64) {
    let (sine, cosine) = rug::Float::with_val(64, angle).sin_cos(rug::Float::new(64));
    (cosine.to_f64(), sine.to_f64())
}

/// `polarscatter(angles, radii)`: dots at their places around the origin.
fn polar_dots(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "polarscatter([angles], [radii]): the angles in radians";
    let [angles, radii] = a else {
        return Err(usage.to_string());
    };
    let (angles, radii) = (
        reals(angles, "polarscatter")?,
        reals(radii, "polarscatter")?,
    );
    if angles.len() != radii.len() || angles.is_empty() {
        return Err(usage.to_string());
    }
    let points: Vec<[f64; 2]> = angles
        .iter()
        .zip(&radii)
        .map(|(t, r)| {
            let (c, s) = wave(*t);
            [r * c, r * s]
        })
        .collect();
    let reach = radii.iter().fold(0.0f64, |m, r| m.max(r.abs())).max(1e-12) * 1.1;
    // circles at the quarters of the largest radius, and the two axes
    let mut rings = Vec::new();
    for ring in 1..=4 {
        let r = reach / 1.1 * ring as f64 / 4.0;
        let corners: Vec<Point> = (0..=96)
            .map(|k| {
                let (s, c) = turn((k % 96, 96));
                flat(r * c, r * s)
            })
            .collect();
        rings.extend(corners.windows(2).map(|w| [w[0], w[1]]));
    }
    rings.push([flat(-reach / 1.1, 0.0), flat(reach / 1.1, 0.0)]);
    rings.push([flat(0.0, -reach / 1.1), flat(0.0, reach / 1.1)]);
    let mut drawing = Drawing::new(
        Space::Chart {
            x: (-reach, reach),
            y: (-reach, reach),
            equal: true,
        },
        vec![
            Item::Segments {
                pairs: rings,
                paint: Paint::Ink,
                weight: 0.5,
            },
            Item::Dots {
                points,
                paint: Paint::Curve(0),
                radius: 2.5,
                units: false,
            },
        ],
    );
    drawing.aspect = Some(1.0);
    drawing.facts = vec![format!(
        "{} points around the origin; the circles at the quarters of the radius {}",
        angles.len(),
        label(reach / 1.1)
    )];
    Ok(drawing)
}

fn quiver(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "quiver([u(x, y), v(x, y)], x, a, b, y, c, d) or quiver([x values], [y values], [u values], [v values])";
    let (starts, arrows): (Vec<[f64; 2]>, Vec<[f64; 2]>) = match a {
        [Expr::List(field), xv, xa, xb, yv, ya, yb]
            if field.len() == 2 && xv.sym_name().is_some() =>
        {
            let vars = (variable(xv)?, variable(yv)?);
            let (x, y) = (ends(xa, xb)?, ends(ya, yb)?);
            let cells = 16;
            let (mut starts, mut arrows) = (Vec::new(), Vec::new());
            for j in 0..=cells {
                for i in 0..=cells {
                    let px = x.0 + (x.1 - x.0) * i as f64 / cells as f64;
                    let py = y.0 + (y.1 - y.0) * j as f64 / cells as f64;
                    if let (Some(u), Some(v)) = (
                        value2(&field[0], &vars, px, py),
                        value2(&field[1], &vars, px, py),
                    ) && u.is_finite()
                        && v.is_finite()
                    {
                        starts.push([px, py]);
                        arrows.push([u, v]);
                    }
                }
            }
            (starts, arrows)
        }
        [x, y, u, v] => {
            let (x, y, u, v) = (
                reals(x, "quiver")?,
                reals(y, "quiver")?,
                reals(u, "quiver")?,
                reals(v, "quiver")?,
            );
            if x.len() != y.len() || y.len() != u.len() || u.len() != v.len() {
                return Err(usage.to_string());
            }
            (
                x.iter().zip(&y).map(|(p, q)| [*p, *q]).collect(),
                u.iter().zip(&v).map(|(p, q)| [*p, *q]).collect(),
            )
        }
        _ => return Err(usage.to_string()),
    };
    if starts.is_empty() {
        return Err("quiver: the field has no real value in these ranges".into());
    }
    let xs: Vec<f64> = starts.iter().map(|p| p[0]).collect();
    let ys: Vec<f64> = starts.iter().map(|p| p[1]).collect();
    let longest = arrows
        .iter()
        .map(|d| (d[0] * d[0] + d[1] * d[1]).sqrt())
        .fold(0.0, f64::max);
    // the longest arrow is as long as the points are apart
    let room = gap(&xs).min(gap(&ys));
    let scale = if longest > 0.0 {
        0.9 * room / longest
    } else {
        0.0
    };
    let mut pairs = Vec::new();
    let (mut all_x, mut all_y) = (xs.clone(), ys.clone());
    for (p, d) in starts.iter().zip(&arrows) {
        let tip = arrow(*p, [d[0] * scale, d[1] * scale], &mut pairs);
        all_x.push(tip[0]);
        all_y.push(tip[1]);
    }
    let mut drawing = Drawing::new(
        chart(span(&all_x), span(&all_y)),
        vec![Item::Segments {
            pairs,
            paint: Paint::Curve(0),
            weight: 1.0,
        }],
    );
    drawing.facts = vec![format!(
        "{} arrows; the longest stands for the length {}",
        starts.len(),
        label(longest)
    )];
    Ok(drawing)
}

fn space_points(name: &str, a: &[Expr]) -> Result<Drawing, String> {
    let usage = format!("{name}([x values], [y values], [z values]) or {name}([[x, y, z], ...])");
    let points: Vec<Point> = match a {
        [one] => one
            .as_list()
            .ok_or(usage.clone())?
            .iter()
            .map(|p| {
                let v = reals(p, name)?;
                match v[..] {
                    [x, y, z] => Ok([x, y, z]),
                    _ => Err(usage.clone()),
                }
            })
            .collect::<Result<_, _>>()?,
        [x, y, z] => {
            let (x, y, z) = (reals(x, name)?, reals(y, name)?, reals(z, name)?);
            if x.len() != y.len() || y.len() != z.len() {
                return Err(usage);
            }
            (0..x.len()).map(|k| [x[k], y[k], z[k]]).collect()
        }
        _ => return Err(usage),
    };
    if points.is_empty() {
        return Err(usage);
    }
    // the three ranges fill the picture alike, as those of a surface do
    let ends: [(f64, f64); 3] = [0, 1, 2].map(|k| {
        let (low, high) = span(&points.iter().map(|p| p[k]).collect::<Vec<f64>>());
        let (low, high) = if name == "stem3" && k == 2 {
            with_zero((low, high))
        } else {
            (low, high)
        };
        if high > low {
            (low, high)
        } else {
            (low - 0.5, high + 0.5)
        }
    });
    let items = if name == "plot3" {
        vec![Item::Line {
            points: points.clone(),
            paint: Paint::Along,
            weight: 1.5,
        }]
    } else {
        // a small star of three strokes at every point
        let mut pairs = Vec::new();
        for p in &points {
            for k in 0..3 {
                let size = 0.015 * (ends[k].1 - ends[k].0);
                let (mut from, mut to) = (*p, *p);
                from[k] -= size;
                to[k] += size;
                pairs.push([from, to]);
            }
            if name == "stem3" {
                pairs.push([[p[0], p[1], 0.0], *p]);
            }
        }
        vec![Item::Segments {
            pairs,
            paint: Paint::Curve(0),
            weight: 1.5,
        }]
    };
    let mut drawing = Drawing::new(Space::Solid, items);
    drawing.stretch = Some(ends);
    drawing.axes = true;
    let told: Vec<String> = (0..3)
        .map(|k| {
            format!(
                "{} from {} to {}",
                ["x", "y", "z"][k],
                label(ends[k].0),
                label(ends[k].1)
            )
        })
        .collect();
    drawing.facts = vec![format!(
        "{} points of space; {}",
        points.len(),
        told.join(", ")
    )];
    Ok(drawing)
}

/// `quiver3(x, y, z, u, v, w)`: arrows of space, each with two strokes of a head.
fn arrows_of_space(a: &[Expr]) -> Result<Drawing, String> {
    let usage =
        "quiver3([x], [y], [z], [u], [v], [w]): an arrow by (u, v, w) from every point (x, y, z)";
    let [x, y, z, u, v, w] = a else {
        return Err(usage.to_string());
    };
    let lists: Vec<Vec<f64>> = [x, y, z, u, v, w]
        .iter()
        .map(|e| reals(e, "quiver3"))
        .collect::<Result<_, _>>()?;
    let count = lists[0].len();
    if count == 0 || lists.iter().any(|l| l.len() != count) {
        return Err(usage.to_string());
    }
    let mut pairs = Vec::new();
    let arrows = (0..count).map(|k| [0, 1, 2, 3, 4, 5].map(|list: usize| lists[list][k]));
    for [x, y, z, u, v, w] in arrows {
        let (from, step) = ([x, y, z], [u, v, w]);
        let tip = [from[0] + step[0], from[1] + step[1], from[2] + step[2]];
        pairs.push([from, tip]);
        // across the arrow: its step crossed with the axis that it leans on least
        let axis = if step[2].abs() <= step[0].abs().min(step[1].abs()) {
            [0.0, 0.0, 1.0]
        } else if step[1].abs() <= step[0].abs() {
            [0.0, 1.0, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let across = [
            step[1] * axis[2] - step[2] * axis[1],
            step[2] * axis[0] - step[0] * axis[2],
            step[0] * axis[1] - step[1] * axis[0],
        ];
        let length = |v: &[f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        let (long, wide) = (length(&step), length(&across));
        if wide > 0.0 {
            for side in [-1.0, 1.0] {
                let back: Point = [0, 1, 2]
                    .map(|c| tip[c] - 0.3 * step[c] + side * 0.15 * long * across[c] / wide);
                pairs.push([tip, back]);
            }
        }
    }
    let mut drawing = Drawing::new(
        Space::Solid,
        vec![Item::Segments {
            pairs,
            paint: Paint::Curve(0),
            weight: 1.5,
        }],
    );
    drawing.axes = true;
    drawing.facts = vec![format!("{count} arrows of space")];
    Ok(drawing)
}

/// `waterfall(Z)`: the rows of a matrix as lines of space, one behind the other.
fn waterfall(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "waterfall(matrix of heights) or waterfall([x values], [y values], matrix): a line for every row";
    let (xs, ys, z) = match a {
        [z] => {
            let m = crate::matrix::rows(z).ok_or(usage)?;
            (
                (1..=m[0].len()).map(|k| k as f64).collect::<Vec<f64>>(),
                (1..=m.len()).map(|k| k as f64).collect::<Vec<f64>>(),
                m,
            )
        }
        [x, y, z] => (
            reals(x, "waterfall")?,
            reals(y, "waterfall")?,
            crate::matrix::rows(z).ok_or(usage)?,
        ),
        _ => return Err(usage.to_string()),
    };
    if z.len() != ys.len() || z[0].len() != xs.len() || xs.len() < 2 {
        return Err(usage.to_string());
    }
    let mut items = Vec::new();
    let mut seen = Vec::new();
    for (j, row) in z.iter().enumerate() {
        let heights: Vec<f64> = row
            .iter()
            .map(|v| number(v).ok_or_else(|| format!("waterfall: {v} is no real number")))
            .collect::<Result<_, _>>()?;
        seen.extend(heights.iter().copied());
        items.push(Item::Line {
            points: xs
                .iter()
                .zip(&heights)
                .map(|(x, h)| [*x, ys[j], *h])
                .collect(),
            paint: Paint::Curve(j),
            weight: 1.5,
        });
    }
    let (low, high) = span(&seen);
    let wide = |(low, high): (f64, f64)| {
        if high > low {
            (low, high)
        } else {
            (low - 0.5, high + 0.5)
        }
    };
    let mut drawing = Drawing::new(Space::Solid, items);
    drawing.stretch = Some([span(&xs), wide(span(&ys)), wide((low, high))]);
    drawing.axes = true;
    drawing.fog = 0.2;
    drawing.facts = vec![format!(
        "{} lines of {} points; z from {} to {}",
        ys.len(),
        xs.len(),
        label(low),
        label(high)
    )];
    Ok(drawing)
}

fn surf(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "surf(matrix of heights) or surf([x values], [y values], matrix): a row of the matrix for every y";
    let (xs, ys, z) = match a {
        [z] => {
            let m = crate::matrix::rows(z).ok_or(usage)?;
            (
                (1..=m[0].len()).map(|k| k as f64).collect::<Vec<f64>>(),
                (1..=m.len()).map(|k| k as f64).collect::<Vec<f64>>(),
                m,
            )
        }
        [x, y, z] => (
            reals(x, "surf")?,
            reals(y, "surf")?,
            crate::matrix::rows(z).ok_or(usage)?,
        ),
        _ => return Err(usage.to_string()),
    };
    if z.len() != ys.len() || z[0].len() != xs.len() || xs.len() < 2 || ys.len() < 2 {
        return Err(usage.to_string());
    }
    let mut points = Vec::with_capacity(xs.len() * ys.len());
    let mut seen = Vec::new();
    for (j, row) in z.iter().enumerate() {
        for (i, v) in row.iter().enumerate() {
            let h = number(v).filter(|h| h.is_finite());
            if let Some(h) = h {
                seen.push(h);
            }
            points.push(h.map(|h| [xs[i], ys[j], h]));
        }
    }
    if seen.is_empty() {
        return Err("surf: the matrix has no real number".into());
    }
    let (low, high) = span(&seen);
    let mut drawing = Drawing::new(
        Space::Solid,
        vec![Item::Mesh {
            columns: xs.len(),
            points,
            wires: 1,
        }],
    );
    drawing.stretch = Some([
        span(&xs),
        span(&ys),
        (low, if high > low { high } else { low + 1.0 }),
    ]);
    drawing.axes = true;
    drawing.palette = "viridis";
    drawing.fog = 0.2;
    drawing.facts = vec![format!(
        "the surface over {} by {} points; z from {} to {}",
        xs.len(),
        ys.len(),
        label(low),
        label(high)
    )];
    Ok(drawing)
}

fn spy(a: &[Expr]) -> Result<Drawing, String> {
    let [m] = a else {
        return Err("spy(matrix)".to_string());
    };
    let rows = crate::matrix::rows(m).ok_or("spy(matrix)")?;
    let (count, columns) = (rows.len(), rows[0].len());
    let marks: Vec<bool> = rows
        .iter()
        .flatten()
        .map(|x| !x.is_zero() && !crate::simplify::is_zero(x))
        .collect();
    let there = marks.iter().filter(|m| **m).count();
    // a square with a rim of the ground around it for every entry that is not zero
    let field = move |px: f64, py: f64| -> Option<Shade> {
        if !(px >= 0.0 && py >= 0.0) {
            return None;
        }
        let (i, j) = (
            (px as usize).min(columns - 1),
            ((count as f64 - py) as usize).min(count - 1),
        );
        let (fx, fy) = (px - px.floor(), py - py.floor());
        let inside = (0.1..=0.9).contains(&fx) && (0.1..=0.9).contains(&fy);
        (marks[j * columns + i] && inside)
            .then_some(Shade::Tint(crate::raster::rgb(0x1f77b4), 0.35))
    };
    let mut drawing = Drawing::new(
        Space::Chart {
            x: (0.0, columns as f64),
            y: (0.0, count as f64),
            equal: true,
        },
        vec![Item::Field(Box::new(field))],
    );
    drawing.axes = false;
    drawing.aspect = Some(count as f64 / columns as f64);
    drawing.facts = vec![format!(
        "{count} by {columns} entries, {there} of them not zero; the first row at the top"
    )];
    Ok(drawing)
}

/// The value at a share of sorted data, as `prctile` takes it.
pub(crate) fn at_share(sorted: &[f64], share: f64) -> f64 {
    let n = sorted.len();
    let pos = n as f64 * share + 0.5;
    if pos <= 1.0 {
        return sorted[0];
    }
    if pos >= n as f64 {
        return sorted[n - 1];
    }
    let k = pos.floor() as usize;
    sorted[k - 1] + (pos - k as f64) * (sorted[k] - sorted[k - 1])
}

fn boxplot(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "boxplot([values]) or boxplot([[values], [values], ...])";
    let [data] = a else {
        return Err(usage.to_string());
    };
    let items = data.as_list().filter(|l| !l.is_empty()).ok_or(usage)?;
    let groups: Vec<Vec<f64>> = if items.iter().all(|g| g.as_list().is_some()) {
        items
            .iter()
            .map(|g| reals(g, "boxplot"))
            .collect::<Result<_, _>>()?
    } else {
        vec![reals(data, "boxplot")?]
    };
    let mut pairs = Vec::new();
    let mut dots = Vec::new();
    let mut all = Vec::new();
    let mut told = Vec::new();
    for (k, group) in groups.iter().enumerate() {
        if group.is_empty() {
            return Err(usage.to_string());
        }
        let mut sorted = group.clone();
        sorted.sort_by(f64::total_cmp);
        let (q1, q2, q3) = (
            at_share(&sorted, 0.25),
            at_share(&sorted, 0.5),
            at_share(&sorted, 0.75),
        );
        let reach = 1.5 * (q3 - q1);
        let low = sorted
            .iter()
            .copied()
            .find(|v| *v >= q1 - reach)
            .unwrap_or(q1);
        let high = sorted
            .iter()
            .rev()
            .copied()
            .find(|v| *v <= q3 + reach)
            .unwrap_or(q3);
        let x = k as f64 + 1.0;
        let (l, r) = (x - 0.3, x + 0.3);
        pairs.extend([
            [flat(l, q1), flat(r, q1)],
            [flat(l, q3), flat(r, q3)],
            [flat(l, q1), flat(l, q3)],
            [flat(r, q1), flat(r, q3)],
            [flat(l, q2), flat(r, q2)],
            [flat(x, q3), flat(x, high)],
            [flat(x, q1), flat(x, low)],
            [flat(x - 0.15, high), flat(x + 0.15, high)],
            [flat(x - 0.15, low), flat(x + 0.15, low)],
        ]);
        dots.extend(
            sorted
                .iter()
                .filter(|v| **v < low || **v > high)
                .map(|v| [x, *v]),
        );
        all.extend(sorted.iter().copied());
        told.push(format!(
            "quartiles {}, {}, {}",
            label(q1),
            label(q2),
            label(q3)
        ));
    }
    let mut drawn = vec![Item::Segments {
        pairs,
        paint: Paint::Curve(0),
        weight: 1.0,
    }];
    if !dots.is_empty() {
        drawn.push(Item::Dots {
            points: dots,
            paint: Paint::Curve(1),
            radius: 2.0,
            units: false,
        });
    }
    let mut drawing = Drawing::new(
        Space::Chart {
            x: (0.4, groups.len() as f64 + 0.6),
            y: roomy(span(&all)),
            equal: false,
        },
        drawn,
    );
    drawing.facts = vec![told.join("; ")];
    Ok(drawing)
}

pub fn build(name: &str, a: &[Expr], detail: usize) -> Result<Drawing, String> {
    match name {
        "bar" | "barh" => bars(name, a),
        "errorbar" => error_bars(a),
        "pie" => pie(a),
        "contour" => contour(a, detail),
        "contour3" => contour3(a, detail),
        "quiver" => quiver(a),
        "quiver3" => arrows_of_space(a),
        "compass" | "feather" => arrows_from(name, a),
        "polarscatter" => polar_dots(a),
        "waterfall" => waterfall(a),
        "plot3" | "scatter3" | "stem3" => space_points(name, a),
        "surf" => surf(a),
        "spy" => spy(a),
        "boxplot" => boxplot(a),
        _ => lines(name, a),
    }
}
