//! Charts as drawings: the plots of `plot` as pictures of pixels, fields of colours, surfaces
//! and curves of space, graphs and matrices.

use crate::expr::{Expr, Rel};
use crate::numeric::{self, Env};
use crate::plot::{ends, label, number, spread, value, value2, variable};
use crate::raster::hue;
use crate::scene::{Drawing, Item, Paint, Point, Shade, Space, turn};
use rug::{Complex, Float};
use std::rc::Rc;

/// The drawings of this module, besides the plots of `plot`.
pub fn is_chart(name: &str) -> bool {
    matches!(
        name,
        "complexplot" | "densityplot" | "plot3d" | "parametricplot3d" | "graphplot" | "matrixplot"
    ) || crate::datachart::is_chart(name)
        || crate::morecharts::is_chart(name)
}

/// What may follow the arguments of a plot that is saved: a range for the values.
fn range_of(name: &str, rest: &[Expr]) -> Result<Option<(f64, f64)>, String> {
    let mut range = None;
    for e in rest {
        match e {
            Expr::List(pair) if pair.len() == 2 => match (number(&pair[0]), number(&pair[1])) {
                (Some(low), Some(high)) if low < high => range = Some((low, high)),
                _ => {
                    return Err(format!(
                        "the range {e} needs two numbers, the lower one first"
                    ));
                }
            },
            _ => {
                return Err(format!(
                    "{e} does not belong to a {name} that is saved: a range [low, high] of the values may follow its arguments; the size of the picture is chosen with width = ... and height = ..."
                ));
            }
        }
    }
    Ok(range)
}

fn nothing_more(name: &str, rest: &[Expr]) -> Result<(), String> {
    match rest.first() {
        None => Ok(()),
        Some(e) => Err(format!(
            "{e} is one argument too many of {name}; what is chosen for a picture is written as name = value"
        )),
    }
}

/// A range with a rim, so that what is drawn does not touch the frame.
pub(crate) fn roomy((low, high): (f64, f64)) -> (f64, f64) {
    let rim = if high > low { 0.04 * (high - low) } else { 1.0 };
    (low - rim, high + rim)
}

/// A chart of lines: the pieces of curves with their colours.
struct Pieces {
    items: Vec<Item>,
    piece: Vec<Point>,
    paint: Paint,
    weight: f64,
}

impl Pieces {
    fn new(paint: Paint, weight: f64) -> Pieces {
        Pieces {
            items: Vec::new(),
            piece: Vec::new(),
            paint,
            weight,
        }
    }

    /// The curve is broken here.
    fn cut(&mut self) {
        let points = std::mem::take(&mut self.piece);
        match points.len() {
            0 => {}
            1 => self.items.push(Item::Dots {
                points: vec![[points[0][0], points[0][1]]],
                paint: self.paint,
                radius: 0.5 * self.weight,
                units: false,
            }),
            _ => self.items.push(Item::Line {
                points,
                paint: self.paint,
                weight: self.weight,
            }),
        }
    }
}

/// `plot(f, x, a, b)` or `plot([f, g], x, a, b)`, then a range of the values.
fn curves(a: &[Expr], detail: usize) -> Result<Drawing, String> {
    let usage =
        "plot(f, x, a, b), plot([f, g], x, a, b); a range [low, high] of the values may follow";
    let [f, var, from, to, rest @ ..] = a else {
        return Err(usage.to_string());
    };
    let fs: Vec<Expr> = match f {
        Expr::List(items) if !items.is_empty() => items.to_vec(),
        other => vec![other.clone()],
    };
    let var = variable(var)?;
    let (x0, x1) = ends(from, to)?;
    let range = range_of("plot", rest)?;
    let count = (2 * detail).clamp(64, 20_000);
    let xs: Vec<f64> = (0..=count)
        .map(|k| x0 + (x1 - x0) * k as f64 / count as f64)
        .collect();
    crate::plot::fetched(&fs, &var, &xs);
    let all: Vec<Vec<Option<f64>>> = fs
        .iter()
        .map(|f| xs.iter().map(|x| value(f, &var, *x)).collect())
        .collect();
    let mut seen: Vec<f64> = all.iter().flatten().flatten().copied().collect();
    if seen.is_empty() {
        return Err(format!("{f} has no real value from {from} to {to}"));
    }
    let y = match range {
        Some(r) => r,
        None => roomy(spread(&mut seen).expect("a value")),
    };
    let span = y.1 - y.0;
    // (what is far outside is drawn as far as the frame cuts it off anyway)
    let (floor, roof) = (y.0 - 2.0 * span, y.1 + 2.0 * span);
    let mut items = Vec::new();
    for (k, ys) in all.iter().enumerate() {
        let mut pieces = Pieces::new(Paint::Curve(k), 1.0);
        let mut before: Option<f64> = None;
        for i in 0..=count {
            let Some(v) = ys[i] else {
                pieces.cut();
                before = None;
                continue;
            };
            // a jump over more than half of the picture: a pole unless the middle is
            // between the ends
            if let Some(last) = before
                && (v - last).abs() > 0.5 * span
            {
                let middle = value(&fs[k], &var, (xs[i - 1] + xs[i]) / 2.0);
                if !middle.is_some_and(|m| (m - last) * (m - v) <= 0.0) {
                    pieces.cut();
                }
            }
            pieces.piece.push([xs[i], v.clamp(floor, roof), 0.0]);
            before = Some(v);
        }
        pieces.cut();
        items.extend(pieces.items);
    }
    let mut drawing = Drawing::new(
        Space::Chart {
            x: (x0, x1),
            y,
            equal: false,
        },
        items,
    );
    if fs.len() > 1 {
        drawing.names = fs.iter().map(|f| f.to_string()).collect();
    }
    Ok(drawing)
}

/// `parametricplot([x(t), y(t)], t, a, b)` and `polarplot(r(t), t, a, b)`.
fn path(name: &str, a: &[Expr], detail: usize) -> Result<Drawing, String> {
    let usage =
        || format!("{name}: parametricplot([x(t), y(t)], t, a, b), polarplot(r(t), t, a, b)");
    let [f, var, from, to, rest @ ..] = a else {
        return Err(usage());
    };
    nothing_more(name, rest)?;
    let var = variable(var)?;
    let (fx, fy) = match (name, f) {
        ("parametricplot", Expr::List(pair)) if pair.len() == 2 => {
            (pair[0].clone(), pair[1].clone())
        }
        ("polarplot", r) if !matches!(r, Expr::List(_)) => {
            let t = Expr::sym(&var);
            let turned = |g: &str| {
                crate::canon::mul(vec![r.clone(), crate::funcs::func(g, vec![t.clone()])])
            };
            (turned("cos"), turned("sin"))
        }
        _ => return Err(usage()),
    };
    let (t0, t1) = ends(from, to)?;
    let count = (4 * detail).clamp(256, 40_000);
    let mut pieces = Pieces::new(Paint::Curve(0), 1.0);
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for k in 0..=count {
        let t = t0 + (t1 - t0) * k as f64 / count as f64;
        match (value(&fx, &var, t), value(&fy, &var, t)) {
            (Some(x), Some(y)) => {
                xs.push(x);
                ys.push(y);
                pieces.piece.push([x, y, 0.0]);
            }
            _ => pieces.cut(),
        }
    }
    pieces.cut();
    let (Some(x), Some(y)) = (spread(&mut xs), spread(&mut ys)) else {
        return Err(format!("{f} has no real value from {from} to {to}"));
    };
    let (x, y) = (roomy(x), roomy(y));
    let mut drawing = Drawing::new(Space::Chart { x, y, equal: true }, pieces.items);
    drawing.aspect = Some((y.1 - y.0) / (x.1 - x.0));
    Ok(drawing)
}

/// `listplot([y1, y2, ...])` or `listplot([[x1, y1], ...])`: the points.
fn points(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "listplot([y1, y2, ...]) or listplot([[x1, y1], [x2, y2], ...])";
    let [data, rest @ ..] = a else {
        return Err(usage.to_string());
    };
    let items = data.as_list().filter(|l| !l.is_empty()).ok_or(usage)?;
    let mut pts = Vec::new();
    for (k, item) in items.iter().enumerate() {
        let p = match item.as_list() {
            Some([x, y]) => (number(x), number(y)),
            Some(_) => return Err(usage.to_string()),
            None => (Some(k as f64 + 1.0), number(item)),
        };
        match p {
            (Some(x), Some(y)) => pts.push([x, y]),
            _ => return Err(format!("{item} is no point of real numbers")),
        }
    }
    let range = range_of("listplot", rest)?;
    let low = pts.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
    let high = pts.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
    let mut ys: Vec<f64> = pts.iter().map(|p| p[1]).collect();
    let y = match range {
        Some(r) => r,
        None => roomy(spread(&mut ys).expect("a value")),
    };
    Ok(Drawing::new(
        Space::Chart {
            x: roomy((low, high)),
            y,
            equal: false,
        },
        vec![Item::Dots {
            points: pts,
            paint: Paint::Curve(0),
            radius: 2.0,
            units: false,
        }],
    ))
}

/// `histogram(data)` or `histogram(data, bins)`: a bar for the values of each of equal
/// intervals.
fn bars(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "histogram([values]) or histogram([values], bins)";
    let (data, rest) = a.split_first().ok_or(usage)?;
    let items = data.as_list().filter(|l| !l.is_empty()).ok_or(usage)?;
    let values: Vec<f64> = items
        .iter()
        .map(|v| number(v).ok_or_else(|| format!("{v} is no real number")))
        .collect::<Result<_, _>>()?;
    let bins = match rest {
        [] => usize::BITS as usize - (values.len() - 1).leading_zeros() as usize + 1,
        [e] => e
            .as_i64()
            .and_then(|k| usize::try_from(k).ok())
            .filter(|k| (1..=200).contains(k))
            .ok_or(usage)?,
        _ => return Err(usage.to_string()),
    };
    let low = values.iter().copied().fold(f64::INFINITY, f64::min);
    let high = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let width = if high > low {
        (high - low) / bins as f64
    } else {
        1.0
    };
    let mut counts = vec![0usize; bins];
    for v in &values {
        let k = ((v - low) / width) as usize;
        counts[k.min(bins - 1)] += 1;
    }
    let most = counts.iter().copied().max().unwrap_or(1).max(1) as f64;
    let gap = 0.04 * width;
    let boxes = counts
        .iter()
        .enumerate()
        .filter(|(_, c)| **c > 0)
        .map(|(k, c)| {
            let from = low + width * k as f64;
            [from + gap, 0.0, from + width - gap, *c as f64]
        })
        .collect();
    let mut drawing = Drawing::new(
        Space::Chart {
            x: (low - 2.0 * gap, low + width * bins as f64 + 2.0 * gap),
            y: (0.0, 1.06 * most),
            equal: false,
        },
        vec![Item::Boxes {
            boxes,
            paint: Paint::Curve(0),
        }],
    );
    drawing.facts = vec![format!(
        "{} values in {bins} intervals of the width {} from {}: the bars are as high as the number of values in them, {} at most",
        values.len(),
        label(width),
        label(low),
        most
    )];
    Ok(drawing)
}

/// The lines where a function of two variables is zero, by the signs at the corners of the
/// cells of a grid.
pub(crate) fn level_lines(
    at: &dyn Fn(usize, usize) -> Option<f64>,
    columns: usize,
    rows: usize,
    place: &dyn Fn(f64, f64) -> Point,
) -> Vec<[Point; 2]> {
    let mut pairs = Vec::new();
    let mut row: Vec<Option<f64>> = (0..=columns).map(|i| at(i, 0)).collect();
    for j in 0..rows {
        let next: Vec<Option<f64>> = (0..=columns).map(|i| at(i, j + 1)).collect();
        for i in 0..columns {
            let (Some(bl), Some(br), Some(tr), Some(tl)) =
                (row[i], row[i + 1], next[i + 1], next[i])
            else {
                continue;
            };
            let kind = usize::from(bl > 0.0)
                | usize::from(br > 0.0) << 1
                | usize::from(tr > 0.0) << 2
                | usize::from(tl > 0.0) << 3;
            if kind == 0 || kind == 15 {
                continue;
            }
            // where the function is zero on a side, by the values at its ends
            let cross = |a: f64, b: f64| a / (a - b);
            let (x, y) = (i as f64, j as f64);
            let bottom = place(x + cross(bl, br), y);
            let right = place(x + 1.0, y + cross(br, tr));
            let top = place(x + cross(tl, tr), y + 1.0);
            let left = place(x, y + cross(bl, tl));
            let middle = bl + br + tr + tl > 0.0;
            match kind {
                1 | 14 => pairs.push([left, bottom]),
                2 | 13 => pairs.push([bottom, right]),
                3 | 12 => pairs.push([left, right]),
                4 | 11 => pairs.push([right, top]),
                6 | 9 => pairs.push([bottom, top]),
                7 | 8 => pairs.push([left, top]),
                // two corners across: the middle tells which two belong together
                5 if middle => pairs.extend([[bottom, right], [left, top]]),
                5 => pairs.extend([[left, bottom], [right, top]]),
                _ if middle => pairs.extend([[left, bottom], [right, top]]),
                _ => pairs.extend([[bottom, right], [left, top]]),
            }
        }
        row = next;
    }
    pairs
}

/// `implicitplot(f(x, y) = 0, x, a, b, y, c, d)`: where the two sides are equal.
fn level(a: &[Expr], detail: usize) -> Result<Drawing, String> {
    let usage = "implicitplot(x^2 + y^2 = 1, x, -1, 1, y, -1, 1)";
    let [f, xv, xa, xb, yv, ya, yb, rest @ ..] = a else {
        return Err(usage.to_string());
    };
    nothing_more("implicitplot", rest)?;
    let f = match f {
        Expr::Rel(Rel::Eq, l, r) => crate::canon::sub((**l).clone(), (**r).clone()),
        Expr::Rel(..) | Expr::List(_) => return Err(usage.to_string()),
        other => other.clone(),
    };
    let vars = (variable(xv)?, variable(yv)?);
    let (x, y) = (ends(xa, xb)?, ends(ya, yb)?);
    let aspect = (y.1 - y.0) / (x.1 - x.0);
    let columns = (detail / 3).clamp(32, 400);
    let rows = ((columns as f64 * aspect).round() as usize).clamp(8, 1200);
    let at = |i: usize, j: usize| {
        let px = x.0 + (x.1 - x.0) * i as f64 / columns as f64;
        let py = y.0 + (y.1 - y.0) * j as f64 / rows as f64;
        value2(&f, &vars, px, py)
    };
    let place = |i: f64, j: f64| -> Point {
        [
            x.0 + (x.1 - x.0) * i / columns as f64,
            y.0 + (y.1 - y.0) * j / rows as f64,
            0.0,
        ]
    };
    let pairs = level_lines(&at, columns, rows, &place);
    if pairs.is_empty() {
        return Err(
            "the two sides are equal nowhere in these ranges, as far as the picture shows".into(),
        );
    }
    let mut drawing = Drawing::new(
        Space::Chart { x, y, equal: true },
        vec![Item::Segments {
            pairs,
            paint: Paint::Curve(0),
            weight: 1.0,
        }],
    );
    drawing.aspect = Some(aspect);
    Ok(drawing)
}

/// A corner of a box of the complex plane.
fn corner(e: &Expr) -> Option<(f64, f64)> {
    if !e.is_const() {
        return None;
    }
    let v = numeric::evalf(e, 64).ok()?;
    let (re, im) = (v.real().to_f64(), v.imag().to_f64());
    (re.is_finite() && im.is_finite()).then_some((re, im))
}

/// `complexplot(f, z, a, b)`: the values of f in a box of the complex plane as colours: the
/// colour is the argument, the lightness the absolute value.
fn complex(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "complexplot(f, z, a, b) with the corners a and b of a box of the complex plane: complexplot(z^2 - 1, z, -2 - 2i, 2 + 2i); complexplot(f, z, r) is from -r - r*i to r + r*i, complexplot(f, z) from -2 - 2i to 2 + 2i";
    let [f, var, rest @ ..] = a else {
        return Err(usage.to_string());
    };
    let var = variable(var)?;
    let (x, y) = match rest {
        [] => ((-2.0, 2.0), (-2.0, 2.0)),
        [r] => match number(r) {
            Some(r) if r > 0.0 => ((-r, r), (-r, r)),
            _ => return Err(usage.to_string()),
        },
        [a, b] => {
            let (Some(a), Some(b)) = (corner(a), corner(b)) else {
                return Err(usage.to_string());
            };
            let ordered = |p: f64, q: f64| if p < q { (p, q) } else { (q, p) };
            let (x, y) = (ordered(a.0, b.0), ordered(a.1, b.1));
            if !(x.1 > x.0 && y.1 > y.0) {
                return Err(format!(
                    "the corners of the box differ in their real and in their imaginary part: {usage}"
                ));
            }
            (x, y)
        }
        _ => return Err(usage.to_string()),
    };
    let f = f.clone();
    // (values of tetration are asked of the program tet for a whole row at once)
    let ahead = crate::tetration::uses_tet(&f).then(|| {
        let (f, var) = (f.clone(), var.clone());
        Box::new(move |row: &[(f64, f64)]| {
            let places = row.iter().map(|(px, py)| {
                let mut env = Env::new();
                env.insert(var.clone(), Complex::with_val(64, (*px, *py)));
                env
            });
            crate::tetration::fetch(&f, 64, places);
        }) as crate::scene::Ahead
    });
    let field = move |px: f64, py: f64| -> Option<Shade> {
        let mut env = Env::new();
        env.insert(var.clone(), Complex::with_val(64, (px, py)));
        let v = numeric::evalf_env(&f, 64, &env).ok()?;
        if !(v.real().is_finite() && v.imag().is_finite()) {
            return Some(Shade::Tint([1.0; 3], 1.0));
        }
        let size = Float::with_val(64, v.abs_ref()).to_f64();
        let angle = if v.real().is_zero() && v.imag().is_zero() {
            0.0
        } else {
            Float::with_val(64, v.arg_ref()).to_f64()
        };
        // 1 is the full colour, 0 is black, the infinitely large is white
        let root = size.sqrt();
        let light = root / (1.0 + root);
        Some(Shade::Tint(
            hue(angle / std::f64::consts::TAU, light),
            light,
        ))
    };
    let mut drawing = Drawing::new(
        Space::Chart { x, y, equal: true },
        vec![Item::Field(Box::new(field))],
    );
    drawing.aspect = Some((y.1 - y.0) / (x.1 - x.0));
    drawing.ahead = ahead;
    drawing.facts = vec![
        "the colour is the argument of the value: red for the positive numbers, then yellow, green, cyan for the negative ones, blue, magenta; the lightness is its absolute value: black is 0, the full colour 1, white the infinitely large".to_string(),
    ];
    Ok(drawing)
}

/// The values of a function of two variables on a grid.
fn sampled(
    f: &Expr,
    vars: &(Rc<str>, Rc<str>),
    x: (f64, f64),
    y: (f64, f64),
    columns: usize,
    rows: usize,
) -> Vec<Option<f64>> {
    let places: Vec<(f64, f64)> = (0..=rows)
        .flat_map(|j| {
            let py = y.0 + (y.1 - y.0) * j as f64 / rows as f64;
            (0..=columns).map(move |i| (x.0 + (x.1 - x.0) * i as f64 / columns as f64, py))
        })
        .collect();
    crate::plot::fetched2(f, vars, &places);
    places
        .iter()
        .map(|(px, py)| value2(f, vars, *px, *py))
        .collect()
}

type TwoRanges<'a> = (&'a Expr, (Rc<str>, Rc<str>), (f64, f64), (f64, f64));

fn two_ranges<'a>(name: &str, a: &'a [Expr]) -> Result<TwoRanges<'a>, String> {
    let usage =
        format!("{name}(f, x, a, b, y, c, d): {name}(sin(x)*cos(y), x, -pi, pi, y, -pi, pi)");
    let [f, xv, xa, xb, yv, ya, yb, rest @ ..] = a else {
        return Err(usage);
    };
    nothing_more(name, rest)?;
    if matches!(f, Expr::List(_) | Expr::Rel(..)) {
        return Err(usage);
    }
    Ok((
        f,
        (variable(xv)?, variable(yv)?),
        ends(xa, xb)?,
        ends(ya, yb)?,
    ))
}

/// `densityplot(f, x, a, b, y, c, d)`: the values of f as the colours of a palette.
fn density(a: &[Expr]) -> Result<Drawing, String> {
    let (f, vars, x, y) = two_ranges("densityplot", a)?;
    // the range of the colours: the values on a grid, without the few that are far away
    let mut seen: Vec<f64> = sampled(f, &vars, x, y, 48, 48)
        .into_iter()
        .flatten()
        .collect();
    let (low, high) =
        spread(&mut seen).ok_or_else(|| format!("{f} has no real value in these ranges"))?;
    let f = f.clone();
    let field = move |px: f64, py: f64| -> Option<Shade> {
        let v = value2(&f, &vars, px, py)?;
        Some(Shade::Level(((v - low) / (high - low)).clamp(0.0, 1.0)))
    };
    let mut drawing = Drawing::new(
        Space::Chart { x, y, equal: false },
        vec![Item::Field(Box::new(field))],
    );
    drawing.palette = "viridis";
    drawing.facts = vec![format!(
        "the colours of the palette run from the value {} to {}",
        label(low),
        label(high)
    )];
    Ok(drawing)
}

/// `plot3d(f, x, a, b, y, c, d)`: the surface z = f(x, y).
fn surface(a: &[Expr], detail: usize) -> Result<Drawing, String> {
    let (f, vars, x, y) = two_ranges("plot3d", a)?;
    let cells = (detail / 8).clamp(24, 160);
    let values = sampled(f, &vars, x, y, cells, cells);
    let mut seen: Vec<f64> = values.iter().flatten().copied().collect();
    let (low, high) =
        spread(&mut seen).ok_or_else(|| format!("{f} has no real value in these ranges"))?;
    let points = values
        .iter()
        .enumerate()
        .map(|(k, v)| {
            let (i, j) = (k % (cells + 1), k / (cells + 1));
            // (a value far outside, as beside a pole, leaves a hole)
            v.filter(|z| *z >= low && *z <= high).map(|z| {
                [
                    x.0 + (x.1 - x.0) * i as f64 / cells as f64,
                    y.0 + (y.1 - y.0) * j as f64 / cells as f64,
                    z,
                ]
            })
        })
        .collect();
    let mut drawing = Drawing::new(
        Space::Solid,
        vec![Item::Mesh {
            columns: cells + 1,
            points,
            wires: (cells / 12).max(1),
        }],
    );
    drawing.stretch = Some([x, y, (low, high)]);
    drawing.axes = true;
    drawing.palette = "viridis";
    // (the colours of a surface tell its heights: they fade less than a walk)
    drawing.fog = 0.2;
    drawing.facts = vec![format!(
        "the surface z = {f} over {} by {} cells; z from {} to {}",
        cells,
        cells,
        label(low),
        label(high)
    )];
    Ok(drawing)
}

/// `parametricplot3d([x(t), y(t), z(t)], t, a, b)`: a curve of space.
fn space_curve(a: &[Expr], detail: usize) -> Result<Drawing, String> {
    let usage = "parametricplot3d([x(t), y(t), z(t)], t, a, b): parametricplot3d([cos(t), sin(t), t/10], t, 0, 20)";
    let [Expr::List(parts), var, from, to, rest @ ..] = a else {
        return Err(usage.to_string());
    };
    nothing_more("parametricplot3d", rest)?;
    if parts.len() != 3 {
        return Err(usage.to_string());
    }
    let var = variable(var)?;
    let (t0, t1) = ends(from, to)?;
    let count = (4 * detail).clamp(256, 40_000);
    let mut pieces = Pieces::new(Paint::Along, 1.5);
    let mut any = false;
    for k in 0..=count {
        let t = t0 + (t1 - t0) * k as f64 / count as f64;
        match (
            value(&parts[0], &var, t),
            value(&parts[1], &var, t),
            value(&parts[2], &var, t),
        ) {
            (Some(x), Some(y), Some(z)) => {
                any = true;
                pieces.piece.push([x, y, z]);
            }
            _ => pieces.cut(),
        }
    }
    pieces.cut();
    if !any {
        return Err(format!("{} has no real value from {from} to {to}", a[0]));
    }
    let mut drawing = Drawing::new(Space::Solid, pieces.items);
    drawing.axes = true;
    if let Some(ends) = drawing.extent() {
        let ranges: Vec<String> = (0..3)
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
            "the curve for {var} from {from} to {to}; {}",
            ranges.join(", ")
        )];
    }
    Ok(drawing)
}

/// `graphplot(g)`: the vertices on a circle, the first at the top and then with the clock.
fn graph(a: &[Expr]) -> Result<Drawing, String> {
    let [g, rest @ ..] = a else {
        return Err("graphplot(g): graphplot([[1, 2], [2, 3], [3, 1]])".to_string());
    };
    nothing_more("graphplot", rest)?;
    let (names, edges) = crate::graph::parts(g)?;
    let n = names.len();
    if n == 0 {
        return Err("the graph has no vertex".to_string());
    }
    let at: Vec<[f64; 2]> = (0..n)
        .map(|k| {
            let (sin, cos) = turn((k as u64, n as u64));
            [sin, cos]
        })
        .collect();
    let mut pairs = Vec::new();
    let mut items = Vec::new();
    // (the sine and the cosine of 25 degrees, for the heads of the arrows)
    let (s, c) = (0.42261826174069944, 0.9063077870366499);
    for (a, b, arrow, weight) in &edges {
        let (p, q) = (at[*a], at[*b]);
        if a == b {
            // a loop: a small circle on the outer side of the vertex
            let middle = [p[0] * 1.12, p[1] * 1.12];
            let ring: Vec<Point> = (0..=24)
                .map(|k| {
                    let (sin, cos) = turn((k % 24, 24));
                    [middle[0] + 0.12 * cos, middle[1] + 0.12 * sin, 0.0]
                })
                .collect();
            pairs.extend(ring.windows(2).map(|w| [w[0], w[1]]));
            continue;
        }
        pairs.push([[p[0], p[1], 0.0], [q[0], q[1], 0.0]]);
        let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
        let length = (dx * dx + dy * dy).sqrt();
        if *arrow && length > 0.0 {
            // the head stands before the dot of the vertex
            let (ux, uy) = (dx / length, dy / length);
            let tip = [q[0] - 0.045 * ux, q[1] - 0.045 * uy];
            for side in [1.0, -1.0] {
                let back = [-(ux * c - side * uy * s), -(uy * c + side * ux * s)];
                pairs.push([
                    [tip[0], tip[1], 0.0],
                    [tip[0] + 0.08 * back[0], tip[1] + 0.08 * back[1], 0.0],
                ]);
            }
        }
        if let Some(weight) = weight {
            // (a third of the way along, so that the weights of crossing edges do not meet)
            items.push(Item::Label {
                at: [p[0] + dx / 3.0, p[1] + dy / 3.0, 0.0],
                text: weight.clone(),
            });
        }
    }
    items.insert(
        0,
        Item::Segments {
            pairs,
            paint: Paint::Curve(0),
            weight: 1.0,
        },
    );
    items.push(Item::Dots {
        points: at.clone(),
        paint: Paint::Curve(1),
        radius: 4.0,
        units: false,
    });
    for (k, name) in names.iter().enumerate() {
        items.push(Item::Label {
            at: [at[k][0] * 1.16, at[k][1] * 1.16, 0.0],
            text: name.clone(),
        });
    }
    let mut drawing = Drawing::new(Space::Flat, items);
    drawing.dark = false;
    drawing.facts = vec![format!(
        "{n} {} on a circle, the first at the top and the others with the clock; {} {}",
        if n == 1 { "vertex" } else { "vertices" },
        edges.len(),
        if edges.len() == 1 { "edge" } else { "edges" },
    )];
    Ok(drawing)
}

/// `matrixplot(m)`: the entries of a matrix as the colours of a palette, the first row at
/// the top.
pub(crate) fn matrix(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "matrixplot(m): matrixplot([[1, 2], [3, 4]])";
    let [m, rest @ ..] = a else {
        return Err(usage.to_string());
    };
    nothing_more("matrixplot", rest)?;
    let lines = m.as_list().filter(|l| !l.is_empty()).ok_or(usage)?;
    let rows: Vec<&[Expr]> = if lines.iter().all(|r| r.as_list().is_some()) {
        lines.iter().filter_map(|r| r.as_list()).collect()
    } else {
        vec![lines]
    };
    let columns = rows[0].len();
    if columns == 0 || rows.iter().any(|r| r.len() != columns) {
        return Err(format!("the rows of a matrix are equally long: {usage}"));
    }
    let mut values = Vec::with_capacity(rows.len() * columns);
    for row in &rows {
        for entry in row.iter() {
            values.push(number(entry).ok_or_else(|| format!("{entry} is no real number"))?);
        }
    }
    let low = values.iter().copied().fold(f64::INFINITY, f64::min);
    let high = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let count = rows.len();
    let field = move |px: f64, py: f64| -> Option<Shade> {
        if !(px >= 0.0 && py >= 0.0) {
            return None;
        }
        let (i, j) = (px as usize, (count as f64 - py) as usize);
        let v = *values.get(j.min(count - 1) * columns + i.min(columns - 1))?;
        Some(Shade::Level(if high > low {
            (v - low) / (high - low)
        } else {
            0.5
        }))
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
    drawing.palette = "viridis";
    drawing.aspect = Some(count as f64 / columns as f64);
    drawing.facts = vec![format!(
        "{count} by {columns} entries, the first row at the top; the colours of the palette run from the entry {} to {}",
        label(low),
        label(high)
    )];
    Ok(drawing)
}

/// The drawing of a command; `detail`: the dots across the picture.
pub fn build(name: &str, a: &[Expr], detail: usize) -> Result<Drawing, String> {
    match name {
        "plot" => curves(a, detail),
        "parametricplot" | "polarplot" => path(name, a, detail),
        "listplot" => points(a),
        "histogram" => bars(a),
        "implicitplot" => level(a, detail),
        "complexplot" => complex(a),
        "densityplot" => density(a),
        "plot3d" => surface(a, detail),
        "parametricplot3d" => space_curve(a, detail),
        "graphplot" => graph(a),
        name if crate::morecharts::is_chart(name) => crate::morecharts::build(name, a, detail),
        name if crate::datachart::is_chart(name) => crate::datachart::build(name, a, detail),
        _ => matrix(a),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawn(name: &str, args: &[&str], detail: usize) -> Result<Drawing, String> {
        let a: Vec<Expr> = args
            .iter()
            .map(|text| {
                let parsed = crate::parser::parse(text).expect("an expression");
                crate::Session::new()
                    .eval(&parsed.stmts[0])
                    .expect("a value")
            })
            .collect();
        build(name, &a, detail)
    }

    fn lines(d: &Drawing) -> Vec<&Vec<Point>> {
        d.items
            .iter()
            .filter_map(|item| match item {
                Item::Line { points, .. } => Some(points),
                _ => None,
            })
            .collect()
    }

    fn field(d: &Drawing) -> &crate::scene::Field {
        match &d.items[0] {
            Item::Field(f) => f,
            _ => panic!("a field"),
        }
    }

    #[test]
    fn curves_and_their_poles() {
        let d = drawn("plot", &["[sin(x), cos(x)]", "x", "0", "2pi"], 100).expect("a drawing");
        assert_eq!(d.names, ["sin(x)", "cos(x)"]);
        let curves = lines(&d);
        assert_eq!(curves.len(), 2);
        assert_eq!(curves[0].len(), 201);
        assert!(curves[0][0][1].abs() < 1e-12 && (curves[1][0][1] - 1.0).abs() < 1e-12);
        match d.space {
            Space::Chart { x, y, equal } => {
                assert!(x.0 == 0.0 && (x.1 - std::f64::consts::TAU).abs() < 1e-12 && !equal);
                assert!((y.0 + 1.08).abs() < 1e-3 && (y.1 - 1.08).abs() < 1e-3);
            }
            _ => panic!("a chart"),
        }
        // the tangent has three branches from -3 to 3, and no line across a pole
        let d = drawn("plot", &["tan(x)", "x", "-3", "3", "[-5, 5]"], 200).expect("a drawing");
        let branches = lines(&d);
        assert_eq!(branches.len(), 3);
        for branch in &branches {
            // (far outside the picture the values are cut off)
            assert!(
                branch.windows(2).all(|w| w[1][1] >= w[0][1]),
                "a branch rises"
            );
            assert!(branch.iter().all(|p| p[1].abs() <= 25.0));
        }
        assert_eq!(
            d.space,
            Space::Chart {
                x: (-3.0, 3.0),
                y: (-5.0, 5.0),
                equal: false
            }
        );
        assert!(d.names.is_empty());
        // where there is no real value there is no curve
        let d = drawn("plot", &["sqrt(x)", "x", "-1", "1"], 100).expect("a drawing");
        let root = lines(&d);
        assert_eq!(root.len(), 1);
        assert!(root[0][0][0] >= 0.0 && root[0].len() == 101);
        assert_eq!(
            drawn("plot", &["ln(-1 - x^2)", "x", "0", "1"], 100).err(),
            Some("ln(-x^2 - 1) has no real value from 0 to 1".to_string())
        );
        let why = drawn("plot", &["sin(x)", "x", "0", "1", "40", "10"], 100)
            .err()
            .expect("sizes are no arguments");
        assert!(
            why.starts_with("40 does not belong to a plot that is saved"),
            "{why}"
        );
        // curves of points with one scale
        let d = drawn("polarplot", &["1", "t", "0", "2pi"], 64).expect("a drawing");
        let circle = lines(&d);
        assert_eq!(circle.len(), 1);
        assert!(
            circle[0]
                .iter()
                .all(|p| ((p[0] * p[0] + p[1] * p[1]).sqrt() - 1.0).abs() < 1e-12)
        );
        assert!(matches!(d.space, Space::Chart { equal: true, .. }));
        assert!((d.aspect.expect("an aspect") - 1.0).abs() < 1e-9);
        let d = drawn(
            "parametricplot",
            &["[2cos(t), sin(t)]", "t", "0", "2pi"],
            64,
        )
        .expect("a drawing");
        assert!((d.aspect.expect("an aspect") - 0.5).abs() < 1e-3);
        assert!(drawn("parametricplot", &["[t, t, t]", "t", "0", "1"], 64).is_err());
    }

    #[test]
    fn points_and_bars() {
        let d = drawn("listplot", &["[1, 4, 9]"], 100).expect("a drawing");
        match &d.items[0] {
            Item::Dots { points, units, .. } => {
                assert_eq!(points, &[[1.0, 1.0], [2.0, 4.0], [3.0, 9.0]]);
                assert!(!units);
            }
            _ => panic!("dots"),
        }
        let d = drawn("listplot", &["[[0, 1], [2, 3]]", "[0, 10]"], 100).expect("a drawing");
        assert!(matches!(d.space, Space::Chart { y: (0.0, 10.0), .. }));
        assert!(drawn("listplot", &["[1, x]"], 100).is_err());
        let d = drawn("histogram", &["[1, 2, 2, 3, 3, 3]", "3"], 100).expect("a drawing");
        match &d.items[0] {
            Item::Boxes { boxes, .. } => {
                let heights: Vec<f64> = boxes.iter().map(|b| b[3]).collect();
                assert_eq!(heights, [1.0, 2.0, 3.0]);
                assert!(boxes.iter().all(|b| b[1] == 0.0 && b[2] > b[0]));
                assert!(boxes[0][0] > 1.0 && boxes[2][2] < 3.0);
            }
            _ => panic!("boxes"),
        }
        assert_eq!(
            d.facts,
            [
                "6 values in 3 intervals of the width 0.6667 from 1: the bars are as high as the number of values in them, 3 at most"
            ]
        );
    }

    #[test]
    fn where_two_sides_are_equal() {
        let d = drawn(
            "implicitplot",
            &["x^2 + y^2 = 1", "x", "-2", "2", "y", "-1.5", "1.5"],
            300,
        )
        .expect("a drawing");
        let Item::Segments { pairs, .. } = &d.items[0] else {
            panic!("segments")
        };
        let mut length = 0.0;
        for [p, q] in pairs {
            for end in [p, q] {
                let far = (end[0] * end[0] + end[1] * end[1]).sqrt();
                assert!((far - 1.0).abs() < 2e-3, "{far}");
            }
            length += ((q[0] - p[0]) * (q[0] - p[0]) + (q[1] - p[1]) * (q[1] - p[1])).sqrt();
        }
        assert!((length - std::f64::consts::TAU).abs() < 0.01, "{length}");
        assert_eq!(d.aspect, Some(0.75));
        // two lines that cross: every piece lies on one of them
        let d = drawn(
            "implicitplot",
            &["x*y = 0", "x", "-1", "1.1", "y", "-1", "1.2"],
            150,
        )
        .expect("a drawing");
        let Item::Segments { pairs, .. } = &d.items[0] else {
            panic!("segments")
        };
        assert!(pairs.len() > 50);
        for [p, q] in pairs {
            assert!(
                (p[0].abs() < 0.03 && q[0].abs() < 0.03)
                    || (p[1].abs() < 0.03 && q[1].abs() < 0.03),
                "{p:?} {q:?}"
            );
        }
        assert_eq!(
            drawn(
                "implicitplot",
                &["x^2 + y^2 = -1", "x", "-1", "1", "y", "-1", "1"],
                100
            )
            .err(),
            Some(
                "the two sides are equal nowhere in these ranges, as far as the picture shows"
                    .to_string()
            )
        );
        // the cells one by one: a corner cut off, a line through, two corners across
        let place = |i: f64, j: f64| -> Point { [i, j, 0.0] };
        let values = [[-1.0, 1.0], [1.0, 1.0]];
        let pairs = level_lines(&|i, j| Some(values[j][i]), 1, 1, &place);
        assert_eq!(pairs, vec![[[0.0, 0.5, 0.0], [0.5, 0.0, 0.0]]]);
        let values = [[-1.0, -1.0], [3.0, 1.0]];
        let pairs = level_lines(&|i, j| Some(values[j][i]), 1, 1, &place);
        assert_eq!(pairs, vec![[[0.0, 0.25, 0.0], [1.0, 0.5, 0.0]]]);
        let values = [[1.0, -1.0], [-1.0, 1.0]];
        assert_eq!(
            level_lines(&|i, j| Some(values[j][i]), 1, 1, &place).len(),
            2
        );
        assert!(level_lines(&|_, _| None, 3, 3, &place).is_empty());
    }

    #[test]
    fn fields_of_colours() {
        let d = drawn("complexplot", &["z", "z", "2"], 100).expect("a drawing");
        assert_eq!(
            d.space,
            Space::Chart {
                x: (-2.0, 2.0),
                y: (-2.0, 2.0),
                equal: true
            }
        );
        let f = field(&d);
        // 1 is the full red, -1 the full cyan, i between yellow and green; 0 is black
        assert_eq!(f(1.0, 0.0), Some(Shade::Tint([1.0, 0.0, 0.0], 0.5)));
        assert_eq!(f(-1.0, 0.0), Some(Shade::Tint([0.0, 1.0, 1.0], 0.5)));
        assert_eq!(f(0.0, 1.0), Some(Shade::Tint([0.5, 1.0, 0.0], 0.5)));
        assert_eq!(f(0.0, 0.0), Some(Shade::Tint([0.0; 3], 0.0)));
        // 4 is lighter: two thirds of the way from black to white
        match f(4.0, 0.0) {
            Some(Shade::Tint(c, light)) => {
                assert!((light - 2.0 / 3.0).abs() < 1e-12);
                assert!((c[1] - 1.0 / 3.0).abs() < 1e-6 && c[0] == 1.0);
            }
            other => panic!("{other:?}"),
        }
        let d = drawn("complexplot", &["1/z", "z"], 100).expect("a drawing");
        assert!(matches!(
            field(&d)(0.0, 0.0),
            None | Some(Shade::Tint([1.0, 1.0, 1.0], _))
        ));
        let d = drawn("complexplot", &["z", "z", "2 + 3i", "-1 - i"], 100).expect("a drawing");
        assert_eq!(
            d.space,
            Space::Chart {
                x: (-1.0, 2.0),
                y: (-1.0, 3.0),
                equal: true
            }
        );
        assert!((d.aspect.expect("an aspect") - 4.0 / 3.0).abs() < 1e-12);
        assert!(drawn("complexplot", &["z", "z", "0"], 100).is_err());
        assert!(drawn("complexplot", &["z", "z", "1", "1 + i"], 100).is_err());
        assert!(drawn("complexplot", &["z", "z", "1", "2", "3"], 100).is_err());
        // the values of a function of two variables along the palette
        let d =
            drawn("densityplot", &["x + y", "x", "0", "1", "y", "0", "1"], 100).expect("a drawing");
        let f = field(&d);
        assert_eq!(f(0.0, 0.0), Some(Shade::Level(0.0)));
        assert_eq!(f(1.0, 1.0), Some(Shade::Level(1.0)));
        assert_eq!(f(0.25, 0.25), Some(Shade::Level(0.25)));
        assert_eq!(
            d.facts,
            ["the colours of the palette run from the value 0 to 2"]
        );
        assert_eq!(d.palette, "viridis");
        assert!(
            drawn(
                "densityplot",
                &["ln(-1 - x^2 - y^2)", "x", "0", "1", "y", "0", "1"],
                100
            )
            .is_err()
        );
        // the entries of a matrix: the first row at the top
        let d = drawn("matrixplot", &["[[1, 2, 3], [4, 5, 6]]"], 100).expect("a drawing");
        let f = field(&d);
        assert_eq!(f(0.5, 1.5), Some(Shade::Level(0.0)));
        assert_eq!(f(1.5, 1.5), Some(Shade::Level(0.2)));
        assert_eq!(f(2.5, 0.5), Some(Shade::Level(1.0)));
        assert_eq!(f(0.5, 0.5), Some(Shade::Level(0.6)));
        assert_eq!(
            d.space,
            Space::Chart {
                x: (0.0, 3.0),
                y: (0.0, 2.0),
                equal: true
            }
        );
        assert_eq!(d.aspect, Some(2.0 / 3.0));
        assert!(!d.axes);
        let d = drawn("matrixplot", &["[7, 7, 7]"], 100).expect("a drawing");
        assert_eq!(field(&d)(1.5, 0.5), Some(Shade::Level(0.5)));
        assert_eq!(
            drawn("matrixplot", &["[[1, 2], [3]]"], 100).err(),
            Some("the rows of a matrix are equally long: matrixplot(m): matrixplot([[1, 2], [3, 4]])".to_string())
        );
        assert_eq!(
            drawn("matrixplot", &["[[1, i]]"], 100).err(),
            Some("i is no real number".to_string())
        );
    }

    #[test]
    fn surfaces_and_curves_of_space() {
        let d = drawn("plot3d", &["x + y", "x", "0", "1", "y", "0", "1"], 240).expect("a drawing");
        let Item::Mesh {
            columns,
            points,
            wires,
        } = &d.items[0]
        else {
            panic!("a mesh")
        };
        assert_eq!((*columns, points.len(), *wires), (31, 31 * 31, 2));
        assert_eq!(points[0], Some([0.0, 0.0, 0.0]));
        assert_eq!(points[31 * 31 - 1], Some([1.0, 1.0, 2.0]));
        assert_eq!(points[30], Some([1.0, 0.0, 1.0]));
        assert_eq!(d.stretch, Some([(0.0, 1.0), (0.0, 1.0), (0.0, 2.0)]));
        assert!(d.axes && d.space == Space::Solid);
        assert_eq!(
            d.facts,
            ["the surface z = x + y over 30 by 30 cells; z from 0 to 2"]
        );
        // beside a pole the surface has a hole
        let d = drawn(
            "plot3d",
            &["1/(x^2 + y^2)", "x", "-1", "1", "y", "-1", "1"],
            240,
        )
        .expect("a drawing");
        let Item::Mesh { points, .. } = &d.items[0] else {
            panic!("a mesh")
        };
        let holes = points.iter().filter(|p| p.is_none()).count();
        assert!(holes > 0 && holes < points.len() / 4, "{holes}");
        assert!(drawn("plot3d", &["x", "x", "0", "1"], 100).is_err());
        let d = drawn(
            "parametricplot3d",
            &["[cos(t), sin(t), t]", "t", "0", "1"],
            64,
        )
        .expect("a drawing");
        let curve = lines(&d);
        assert_eq!((curve.len(), curve[0].len()), (1, 257));
        assert_eq!(curve[0][0], [1.0, 0.0, 0.0]);
        assert!((curve[0][256][2] - 1.0).abs() < 1e-12);
        assert!(d.axes && d.space == Space::Solid && d.stretch.is_none());
        assert_eq!(
            d.facts,
            ["the curve for t from 0 to 1; x from 0.5403 to 1, y from 0 to 0.8415, z from 0 to 1"]
        );
        assert!(drawn("parametricplot3d", &["[t, t]", "t", "0", "1"], 64).is_err());
    }

    #[test]
    fn graphs_on_a_circle() {
        let d = drawn("graphplot", &["[[1, 2], [2, 3], [3, 1]]"], 100).expect("a drawing");
        let Item::Segments { pairs, .. } = &d.items[0] else {
            panic!("segments")
        };
        assert_eq!(pairs.len(), 3);
        let dots = d
            .items
            .iter()
            .find_map(|item| match item {
                Item::Dots { points, .. } => Some(points.clone()),
                _ => None,
            })
            .expect("dots");
        // the first vertex at the top, the others with the clock
        assert_eq!(dots[0], [0.0, 1.0]);
        assert!((dots[1][0] - 0.75f64.sqrt()).abs() < 1e-12 && (dots[1][1] + 0.5).abs() < 1e-12);
        assert!((dots[2][0] + 0.75f64.sqrt()).abs() < 1e-12);
        let labels: Vec<&str> = d
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Label { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(labels, ["1", "2", "3"]);
        assert_eq!(
            d.facts,
            ["3 vertices on a circle, the first at the top and the others with the clock; 3 edges"]
        );
        assert!(!d.dark && d.space == Space::Flat);
        let count = |text: &str| {
            let d = drawn("graphplot", &[text], 100).expect("a drawing");
            let labels = d
                .items
                .iter()
                .filter(|i| matches!(i, Item::Label { .. }))
                .count();
            match &d.items[0] {
                Item::Segments { pairs, .. } => (pairs.len(), labels),
                _ => panic!("segments"),
            }
        };
        // an arrow has a head of two lines, a loop is a ring, a weight is a word
        assert_eq!(count("[1 -> 2]"), (3, 2));
        assert_eq!(count("[[1, 1]]"), (24, 1));
        assert_eq!(count("[[1, 2, 5]]"), (1, 3));
        assert_eq!(count("[[1]]"), (0, 1));
        assert!(drawn("graphplot", &["5"], 100).is_err());
    }
}
