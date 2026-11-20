//! Plots drawn with characters: curves as Braille dots, or as ASCII marks with `ascii`.
//!
//! The numbers of a plot are 64-bit floats made from values of the arbitrary-precision
//! evaluation; only `+ - * /` act on them, so that a plot is the same on every system.

use crate::expr::Expr;
use crate::numeric::{self, Env};
use std::rc::Rc;

const MARKS: [char; 6] = ['*', '+', 'o', 'x', '#', '@'];
const USAGE: &str = "plot(f, x, a, b), plot([f, g], x, a, b); after them in any order: [low, high] for the vertical range, a width and a height in characters, ascii";

/// The commands that draw.
pub fn is_plot(name: &str) -> bool {
    matches!(
        name,
        "plot" | "parametricplot" | "polarplot" | "listplot" | "histogram" | "implicitplot"
    )
}

pub(crate) struct Options {
    pub(crate) width: usize,
    pub(crate) height: usize,
    /// A width or a height was given.
    pub(crate) sized: bool,
    pub(crate) ascii: bool,
    pub(crate) range: Option<(f64, f64)>,
}

pub(crate) fn number(e: &Expr) -> Option<f64> {
    numeric::real_value(e, 20).map(|v| v.to_f64())
}

pub(crate) fn options(rest: &[Expr]) -> Result<Options, String> {
    let mut o = Options {
        width: 64,
        height: 16,
        sized: false,
        ascii: false,
        range: None,
    };
    let mut sizes = Vec::new();
    for e in rest {
        match e {
            Expr::Sym(s) if &**s == "ascii" => o.ascii = true,
            Expr::List(pair) if pair.len() == 2 => {
                let (lo, hi) = (number(&pair[0]), number(&pair[1]));
                match (lo, hi) {
                    (Some(lo), Some(hi)) if lo < hi => o.range = Some((lo, hi)),
                    _ => {
                        return Err(format!(
                            "the range {e} needs two numbers, the lower one first"
                        ));
                    }
                }
            }
            _ => match e.as_i64().and_then(|k| usize::try_from(k).ok()) {
                Some(k) if (2..=400).contains(&k) => sizes.push(k),
                _ => return Err(format!("{e} is no option of a plot: {USAGE}")),
            },
        }
    }
    match sizes.as_slice() {
        [] => {}
        [w] => o.width = *w,
        [w, h] => (o.width, o.height) = (*w, *h),
        _ => return Err(USAGE.to_string()),
    }
    o.sized = !sizes.is_empty();
    o.width = o.width.max(8);
    o.height = o.height.clamp(2, 200);
    Ok(o)
}

/// The value of `f` where the variable is `x`: a real number, or nothing.
pub(crate) fn value(f: &Expr, var: &Rc<str>, x: f64) -> Option<f64> {
    let mut env = Env::new();
    env.insert(var.clone(), rug::Complex::with_val(64, (x, 0.0)));
    let v = numeric::evalf_env(f, 64, &env).ok()?;
    let (re, im) = (v.real().to_f64(), v.imag().to_f64());
    let real = im == 0.0 || im.abs() <= 1e-12 * (re.abs() + 1.0);
    (real && re.is_finite()).then_some(re)
}

/// Before `fs` are worked out at the places `xs`: what they need of `tet` is fetched at once.
pub(crate) fn fetched(fs: &[Expr], var: &Rc<str>, xs: &[f64]) {
    for f in fs {
        crate::tetration::fetch(
            f,
            64,
            xs.iter().map(|x| {
                let mut env = Env::new();
                env.insert(var.clone(), rug::Complex::with_val(64, (*x, 0.0)));
                env
            }),
        );
    }
}

/// The same for a function of two variables on a grid.
pub(crate) fn fetched2(f: &Expr, vars: &(Rc<str>, Rc<str>), places: &[(f64, f64)]) {
    crate::tetration::fetch(
        f,
        64,
        places.iter().map(|(x, y)| {
            let mut env = Env::new();
            env.insert(vars.0.clone(), rug::Complex::with_val(64, (*x, 0.0)));
            env.insert(vars.1.clone(), rug::Complex::with_val(64, (*y, 0.0)));
            env
        }),
    );
}

/// The value of `f` at two variables.
pub(crate) fn value2(f: &Expr, vars: &(Rc<str>, Rc<str>), x: f64, y: f64) -> Option<f64> {
    let mut env = Env::new();
    env.insert(vars.0.clone(), rug::Complex::with_val(64, (x, 0.0)));
    env.insert(vars.1.clone(), rug::Complex::with_val(64, (y, 0.0)));
    let v = numeric::evalf_env(f, 64, &env).ok()?;
    let (re, im) = (v.real().to_f64(), v.imag().to_f64());
    (im.abs() <= 1e-12 * (re.abs() + 1.0) && re.is_finite()).then_some(re)
}

pub(crate) struct Canvas {
    pub(crate) w: usize,
    pub(crate) h: usize,
    pub(crate) ascii: bool,
    /// The dots of each cell (Braille bits), or its mark.
    pub(crate) cells: Vec<u32>,
    pub(crate) x: (f64, f64),
    pub(crate) y: (f64, f64),
    /// The lines of the axes run through the cells without dots.
    pub(crate) axes: bool,
    /// The y axis holds logarithms to base 10: its numbers are the powers of ten.
    pub(crate) log_y: bool,
}

impl Canvas {
    pub(crate) fn new(o: &Options, x: (f64, f64), y: (f64, f64)) -> Canvas {
        Canvas {
            w: o.width,
            h: o.height,
            ascii: o.ascii,
            cells: vec![0; o.width * o.height],
            x,
            y,
            axes: true,
            log_y: false,
        }
    }

    /// Dots across and down.
    pub(crate) fn dots(&self) -> (i64, i64) {
        if self.ascii {
            (self.w as i64, self.h as i64)
        } else {
            (2 * self.w as i64, 4 * self.h as i64)
        }
    }

    pub(crate) fn set(&mut self, i: i64, j: i64, mark: char) {
        let (dw, dh) = self.dots();
        if i < 0 || j < 0 || i >= dw || j >= dh {
            return;
        }
        let (i, j) = (i as usize, j as usize);
        if self.ascii {
            self.cells[j * self.w + i] = mark as u32;
        } else {
            let bit = match (i % 2, j % 4) {
                (0, 0) => 0x01,
                (0, 1) => 0x02,
                (0, 2) => 0x04,
                (0, _) => 0x40,
                (_, 0) => 0x08,
                (_, 1) => 0x10,
                (_, 2) => 0x20,
                _ => 0x80,
            };
            self.cells[(j / 4) * self.w + i / 2] |= bit;
        }
    }

    /// The dot of a point, as numbers that may lie outside.
    fn place(&self, x: f64, y: f64) -> (f64, f64) {
        let (dw, dh) = self.dots();
        (
            (x - self.x.0) / (self.x.1 - self.x.0) * (dw - 1) as f64,
            (self.y.1 - y) / (self.y.1 - self.y.0) * (dh - 1) as f64,
        )
    }

    fn point(&mut self, x: f64, y: f64, mark: char) {
        let (i, j) = self.place(x, y);
        if i.is_finite() && j.is_finite() && i.abs() < 1e9 && j.abs() < 1e9 {
            self.set(i.round() as i64, j.round() as i64, mark);
        }
    }

    /// The dots from one point to another.
    fn line(&mut self, from: (f64, f64), to: (f64, f64), mark: char) {
        let (dw, dh) = self.dots();
        let (a, b) = (self.place(from.0, from.1), self.place(to.0, to.1));
        let inside =
            |p: (f64, f64)| p.0 >= -1.0 && p.1 >= -1.0 && p.0 <= dw as f64 && p.1 <= dh as f64;
        if !(a.0.is_finite() && a.1.is_finite() && b.0.is_finite() && b.1.is_finite()) {
            return;
        }
        // the part of the way that is far outside is cut off
        let limit = 4.0 * (dw + dh) as f64;
        let clamp = |v: f64| v.clamp(-limit, limit);
        let (a, b) = ((clamp(a.0), clamp(a.1)), (clamp(b.0), clamp(b.1)));
        let across = (b.0 - a.0).abs();
        let down = (b.1 - a.1).abs();
        let steps = if across > down { across } else { down };
        let steps = steps.ceil().max(1.0);
        let count = steps as i64;
        for k in 0..=count {
            let t = k as f64 / steps;
            let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            if inside(p) {
                self.set(p.0.round() as i64, p.1.round() as i64, mark);
            }
        }
    }

    /// The rows alone, without axes and labels.
    pub(crate) fn bare(&self) -> Vec<String> {
        (0..self.h)
            .map(|r| {
                let line: String = (0..self.w)
                    .map(|c| match self.cells[r * self.w + c] {
                        0 => ' ',
                        cell if self.ascii => char::from_u32(cell).unwrap_or('*'),
                        cell => char::from_u32(0x2800 + cell).unwrap_or('.'),
                    })
                    .collect();
                line.trim_end().to_string()
            })
            .collect()
    }

    /// The rows with the axes and the labels.
    pub(crate) fn render(&self, left: &str, right: &str) -> Vec<String> {
        let (dw, dh) = self.dots();
        let (bar, rule, cross, corner) = if self.ascii {
            ('|', '-', '+', '+')
        } else {
            ('│', '─', '┼', '└')
        };
        let per = if self.ascii { (1, 1) } else { (2, 4) };
        // the cells of the axes, when zero is in the ranges
        let zero = self.place(0.0, 0.0);
        let column = (self.axes && self.x.0 < 0.0 && self.x.1 > 0.0)
            .then(|| (zero.0.round() as i64).clamp(0, dw - 1) as usize / per.0);
        let row = (self.axes && self.y.0 < 0.0 && self.y.1 > 0.0)
            .then(|| (zero.1.round() as i64).clamp(0, dh - 1) as usize / per.1);
        let (top, bottom) = (
            crate::scene::mark(self.y.1, self.log_y),
            crate::scene::mark(self.y.0, self.log_y),
        );
        let margin = top.len().max(bottom.len()).max(1);
        let mut out = Vec::new();
        for r in 0..self.h {
            let name = if r == 0 {
                top.as_str()
            } else if r == self.h - 1 {
                bottom.as_str()
            } else if row == Some(r) {
                if self.log_y { "1" } else { "0" }
            } else {
                ""
            };
            let mut line = format!("{name:>margin$} {bar}");
            for c in 0..self.w {
                let cell = self.cells[r * self.w + c];
                let ch = if cell != 0 {
                    if self.ascii {
                        char::from_u32(cell).unwrap_or('*')
                    } else {
                        char::from_u32(0x2800 + cell).unwrap_or('.')
                    }
                } else {
                    match (row == Some(r), column == Some(c)) {
                        (true, true) => cross,
                        (true, false) => rule,
                        (false, true) => bar,
                        (false, false) => ' ',
                    }
                };
                line.push(ch);
            }
            out.push(line.trim_end().to_string());
        }
        out.push(format!(
            "{:>margin$} {corner}{}",
            "",
            rule.to_string().repeat(self.w)
        ));
        let gap = self
            .w
            .saturating_sub(left.chars().count() + right.chars().count());
        out.push(format!("{:>margin$}  {left}{}{right}", "", " ".repeat(gap)));
        out
    }
}

/// A number of an axis with four digits.
pub(crate) fn label(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    let text = format!("{v:.3e}");
    let (digits, exponent) = text.split_once('e').unwrap_or((&text, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    if !(-4..=5).contains(&exponent) {
        let short = digits.trim_end_matches('0').trim_end_matches('.');
        return format!("{short}e{exponent}");
    }
    let places = (3 - exponent).max(0) as usize;
    let plain = format!("{v:.places$}");
    if plain.contains('.') {
        plain
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    } else {
        plain
    }
}

/// The text of an end of the range: the expression when it is short, else its number.
pub(crate) fn end(e: &Expr, v: f64) -> String {
    let text = e.to_string();
    let fraction = e.as_rat().is_some_and(|r| !r.is_integer());
    if text.chars().count() <= 12 && !fraction {
        text
    } else {
        label(v)
    }
}

/// A range for the values: all of them, or the middle of them when a few are far away (as
/// next to a pole).
pub(crate) fn spread(values: &mut [f64]) -> Option<(f64, f64)> {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let (first, last) = (*values.first()?, *values.last()?);
    let n = values.len();
    let (low, high) = (values[n * 4 / 100], values[(n * 96 / 100).min(n - 1)]);
    let (mut lo, mut hi) = if last - first > 6.0 * (high - low) && high > low {
        let pad = (high - low) * 0.25;
        (low - pad, high + pad)
    } else {
        (first, last)
    };
    if hi <= lo {
        let pad = if lo == 0.0 { 1.0 } else { lo.abs() * 0.5 };
        (lo, hi) = (lo - pad, hi + pad);
    }
    Some((lo, hi))
}

pub(crate) fn variable(e: &Expr) -> Result<Rc<str>, String> {
    e.sym_name()
        .map(Rc::from)
        .ok_or_else(|| format!("{e} is not a variable"))
}

pub(crate) fn ends(a: &Expr, b: &Expr) -> Result<(f64, f64), String> {
    match (number(a), number(b)) {
        (Some(a), Some(b)) if a < b => Ok((a, b)),
        _ => Err(format!("from {a} to {b} is no range of real numbers")),
    }
}

/// `plot(f, x, a, b, ...)`: one curve or several.
fn curves(a: &[Expr]) -> Result<Vec<String>, String> {
    let [f, var, from, to, rest @ ..] = a else {
        return Err(USAGE.to_string());
    };
    let fs: Vec<Expr> = match f {
        Expr::List(items) if !items.is_empty() => items.to_vec(),
        other => vec![other.clone()],
    };
    let var = variable(var)?;
    let (x0, x1) = ends(from, to)?;
    let o = options(rest)?;
    let dots = if o.ascii { o.width } else { 2 * o.width };
    let count = 4 * dots;
    let xs: Vec<f64> = (0..=count)
        .map(|k| x0 + (x1 - x0) * k as f64 / count as f64)
        .collect();
    fetched(&fs, &var, &xs);
    let all: Vec<Vec<Option<f64>>> = fs
        .iter()
        .map(|f| xs.iter().map(|x| value(f, &var, *x)).collect())
        .collect();
    let mut seen: Vec<f64> = all.iter().flatten().flatten().copied().collect();
    if seen.is_empty() {
        return Err(format!("{f} has no real value from {from} to {to}"));
    }
    let y = match o.range {
        Some(r) => r,
        None => spread(&mut seen).expect("a value"),
    };
    let span = y.1 - y.0;
    let mut canvas = Canvas::new(&o, (x0, x1), y);
    for (k, ys) in all.iter().enumerate() {
        let mark = MARKS[k % MARKS.len()];
        for i in 0..count {
            match (ys[i], ys[i + 1]) {
                (Some(a), Some(b)) => {
                    // a jump over more than half of the picture: a pole unless the middle is
                    // between the ends
                    if (b - a).abs() > 0.5 * span {
                        let middle = value(&fs[k], &var, (xs[i] + xs[i + 1]) / 2.0);
                        let between = middle.is_some_and(|m| (m - a) * (m - b) <= 0.0);
                        if !between {
                            canvas.point(xs[i], a, mark);
                            canvas.point(xs[i + 1], b, mark);
                            continue;
                        }
                    }
                    canvas.line((xs[i], a), (xs[i + 1], b), mark);
                }
                (Some(a), None) => canvas.point(xs[i], a, mark),
                (None, Some(b)) => canvas.point(xs[i + 1], b, mark),
                (None, None) => {}
            }
        }
    }
    let mut lines = canvas.render(&end(from, x0), &end(to, x1));
    if o.ascii && fs.len() > 1 {
        let names: Vec<String> = fs
            .iter()
            .enumerate()
            .map(|(k, f)| format!("{} {f}", MARKS[k % MARKS.len()]))
            .collect();
        lines.push(names.join("   "));
    }
    Ok(lines)
}

/// The same step along both axes around the points, for the shape of the cells.
pub(crate) fn square(o: &Options, x: (f64, f64), y: (f64, f64)) -> ((f64, f64), (f64, f64)) {
    // (a Braille dot is as wide as it is high, a character is twice as high as wide)
    let (dw, dh) = if o.ascii {
        (o.width as f64, 2.0 * o.height as f64)
    } else {
        (2.0 * o.width as f64, 4.0 * o.height as f64)
    };
    let (mut xs, mut ys) = (x.1 - x.0, y.1 - y.0);
    if xs <= 0.0 {
        xs = 1.0;
    }
    if ys <= 0.0 {
        ys = 1.0;
    }
    let step = if xs / dw > ys / dh { xs / dw } else { ys / dh } * 1.05;
    let (cx, cy) = ((x.0 + x.1) / 2.0, (y.0 + y.1) / 2.0);
    (
        (cx - step * dw / 2.0, cx + step * dw / 2.0),
        (cy - step * dh / 2.0, cy + step * dh / 2.0),
    )
}

/// `parametricplot([x(t), y(t)], t, a, b)` and `polarplot(r(t), t, a, b)`.
fn path(name: &str, a: &[Expr]) -> Result<Vec<String>, String> {
    let usage =
        || format!("{name}: parametricplot([x(t), y(t)], t, a, b), polarplot(r(t), t, a, b)");
    let [f, var, from, to, rest @ ..] = a else {
        return Err(usage());
    };
    let var = variable(var)?;
    let (fx, fy) = match (name, f) {
        ("parametricplot", Expr::List(pair)) if pair.len() == 2 => {
            (pair[0].clone(), pair[1].clone())
        }
        ("polarplot", r) if !matches!(r, Expr::List(_)) => {
            let t = Expr::sym(&var);
            let turn = |g: &str| {
                crate::canon::mul(vec![r.clone(), crate::funcs::func(g, vec![t.clone()])])
            };
            (turn("cos"), turn("sin"))
        }
        _ => return Err(usage()),
    };
    let (t0, t1) = ends(from, to)?;
    let o = options(rest)?;
    let count = 16 * o.width;
    let points: Vec<Option<(f64, f64)>> = (0..=count)
        .map(|k| {
            let t = t0 + (t1 - t0) * k as f64 / count as f64;
            Some((value(&fx, &var, t)?, value(&fy, &var, t)?))
        })
        .collect();
    let mut xs: Vec<f64> = points.iter().flatten().map(|p| p.0).collect();
    let mut ys: Vec<f64> = points.iter().flatten().map(|p| p.1).collect();
    let (Some(x), Some(y)) = (spread(&mut xs), spread(&mut ys)) else {
        return Err(format!("{f} has no real value from {from} to {to}"));
    };
    let (x, y) = square(&o, x, y);
    let mut canvas = Canvas::new(&o, x, y);
    for pair in points.windows(2) {
        match (pair[0], pair[1]) {
            (Some(p), Some(q)) => canvas.line(p, q, '*'),
            (Some(p), None) | (None, Some(p)) => canvas.point(p.0, p.1, '*'),
            (None, None) => {}
        }
    }
    Ok(canvas.render(&label(x.0), &label(x.1)))
}

/// `listplot([y1, y2, ...])` or `listplot([[x1, y1], ...])`: the points.
fn points(a: &[Expr]) -> Result<Vec<String>, String> {
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
            (Some(x), Some(y)) => pts.push((x, y)),
            _ => return Err(format!("{item} is no point of real numbers")),
        }
    }
    let o = options(rest)?;
    let mut xs: Vec<f64> = pts.iter().map(|p| p.0).collect();
    let mut ys: Vec<f64> = pts.iter().map(|p| p.1).collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut x = (xs[0], xs[xs.len() - 1]);
    if x.1 <= x.0 {
        x = (x.0 - 1.0, x.1 + 1.0);
    }
    let y = match o.range {
        Some(r) => r,
        None => spread(&mut ys).expect("a value"),
    };
    let mut canvas = Canvas::new(&o, x, y);
    for p in &pts {
        // a point is a small cross of dots
        let (i, j) = canvas.place(p.0, p.1);
        if i.is_finite() && j.is_finite() && i.abs() < 1e9 && j.abs() < 1e9 {
            let (i, j) = (i.round() as i64, j.round() as i64);
            let arms: &[(i64, i64)] = if o.ascii {
                &[(0, 0)]
            } else {
                &[(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)]
            };
            for (di, dj) in arms {
                canvas.set(i + di, j + dj, '*');
            }
        }
    }
    Ok(canvas.render(&label(x.0), &label(x.1)))
}

/// `histogram(data)` or `histogram(data, bins)`: how many values fall into each of equal
/// intervals.
fn bars(a: &[Expr]) -> Result<Vec<String>, String> {
    let usage = "histogram([values]) or histogram([values], bins)";
    let (data, rest) = a.split_first().ok_or(usage)?;
    let items = data.as_list().filter(|l| !l.is_empty()).ok_or(usage)?;
    let values: Vec<f64> = items
        .iter()
        .map(|v| number(v).ok_or_else(|| format!("{v} is no real number")))
        .collect::<Result<_, _>>()?;
    let (mut bins, mut ascii) = (None, false);
    for e in rest {
        match e {
            Expr::Sym(s) if &**s == "ascii" => ascii = true,
            _ => match e.as_i64().and_then(|k| usize::try_from(k).ok()) {
                Some(k) if (1..=200).contains(&k) => bins = Some(k),
                _ => return Err(usage.to_string()),
            },
        }
    }
    // (as many intervals as the binary digits of the number of values, and one more)
    let bins =
        bins.unwrap_or(usize::BITS as usize - (values.len() - 1).leading_zeros() as usize + 1);
    let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let width = if hi > lo {
        (hi - lo) / bins as f64
    } else {
        1.0
    };
    let mut counts = vec![0usize; bins];
    for v in &values {
        let k = ((v - lo) / width) as usize;
        counts[k.min(bins - 1)] += 1;
    }
    let most = counts.iter().copied().max().unwrap_or(1).max(1);
    let edges: Vec<(String, String)> = (0..bins)
        .map(|k| {
            (
                label(lo + width * k as f64),
                label(lo + width * (k + 1) as f64),
            )
        })
        .collect();
    let (wa, wb) = (
        edges.iter().map(|e| e.0.len()).max().unwrap_or(1),
        edges.iter().map(|e| e.1.len()).max().unwrap_or(1),
    );
    let (bar, block) = if ascii { ('|', '#') } else { ('│', '█') };
    Ok(counts
        .iter()
        .zip(&edges)
        .map(|(c, (from, to))| {
            let length = (c * 48).div_ceil(most);
            let blocks = block.to_string().repeat(length);
            format!("{from:>wa$} .. {to:<wb$} {bar}{blocks} {c}")
        })
        .collect())
}

/// `implicitplot(f(x, y) = 0, x, a, b, y, c, d)`: where the two sides are equal.
fn level(a: &[Expr]) -> Result<Vec<String>, String> {
    let usage = "implicitplot(x^2 + y^2 = 1, x, -1, 1, y, -1, 1)";
    let [f, xv, xa, xb, yv, ya, yb, rest @ ..] = a else {
        return Err(usage.to_string());
    };
    let f = match f {
        Expr::Rel(crate::expr::Rel::Eq, l, r) => crate::canon::sub((**l).clone(), (**r).clone()),
        Expr::Rel(..) | Expr::List(_) => return Err(usage.to_string()),
        other => other.clone(),
    };
    let vars = (variable(xv)?, variable(yv)?);
    let (x, y) = (ends(xa, xb)?, ends(ya, yb)?);
    let mut o = options(rest)?;
    if !o.sized {
        // the same step along both axes, as far as the width of a line allows
        let across = 2.0 * o.height as f64 * (x.1 - x.0) / (y.1 - y.0);
        o.width = (across.round() as usize).clamp(16, 96);
    }
    let mut canvas = Canvas::new(&o, x, y);
    let (dw, dh) = canvas.dots();
    // the sign at the corners of the dots: a dot where it changes
    let at = |i: i64, j: i64| {
        let px = x.0 + (x.1 - x.0) * (i as f64 - 0.5) / (dw - 1) as f64;
        let py = y.1 - (y.1 - y.0) * (j as f64 - 0.5) / (dh - 1) as f64;
        value2(&f, &vars, px, py)
    };
    let mut signs = vec![None; ((dw + 1) * (dh + 1)) as usize];
    for j in 0..=dh {
        for i in 0..=dw {
            signs[(j * (dw + 1) + i) as usize] = at(i, j).map(|v| v > 0.0);
        }
    }
    let mut any = false;
    for j in 0..dh {
        for i in 0..dw {
            let corner = |di: i64, dj: i64| signs[((j + dj) * (dw + 1) + i + di) as usize];
            let all = [corner(0, 0), corner(1, 0), corner(0, 1), corner(1, 1)];
            let known: Vec<bool> = all.iter().flatten().copied().collect();
            if known.len() == 4 && known.iter().any(|s| *s) && known.iter().any(|s| !*s) {
                canvas.set(i, j, '*');
                any = true;
            }
        }
    }
    if !any {
        return Err(
            "the two sides are equal nowhere in these ranges, as far as the picture shows".into(),
        );
    }
    Ok(canvas.render(&end(xa, x.0), &end(xb, x.1)))
}

/// The lines of a plot.
pub fn draw(name: &str, a: &[Expr]) -> Result<Vec<String>, String> {
    match name {
        "plot" => curves(a),
        "parametricplot" | "polarplot" => path(name, a),
        "listplot" => points(a),
        "histogram" => bars(a),
        _ => level(a),
    }
}
