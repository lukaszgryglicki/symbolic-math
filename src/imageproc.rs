//! More of the handling of pictures as matrices: weights and filters, shapes (dilation,
//! erosion and what is made of them), parts that hang together, edges, levels of gray,
//! noise, measures, colours and moves. The numbers are doubles here, as the samples of
//! pictures are; numbers from 0 to 1 are shares of white, whole numbers above 1 samples.

use crate::expr::Expr;
use crate::matfile::double;
use crate::radix::text_of;

type Res = Result<Expr, String>;
type Plane = Vec<Vec<f64>>;

/// What lies beyond the border of a picture.
#[derive(Clone, Copy, PartialEq)]
enum Pad {
    Value(f64),
    Replicate,
    Symmetric,
    Circular,
}

fn value(e: &Expr) -> Option<f64> {
    match e {
        Expr::Bool(b) => Some(f64::from(u8::from(*b))),
        _ => crate::plot::number(e),
    }
}

fn plane(e: &Expr) -> Option<Plane> {
    let rows = crate::matrix::rows(e)?;
    let width = rows.first()?.len();
    (width > 0 && rows.iter().all(|r| r.len() == width)).then_some(())?;
    rows.iter().map(|r| r.iter().map(value).collect()).collect()
}

/// The channels of a picture, and whether it was one matrix.
fn planes(e: &Expr, name: &str) -> Result<(Vec<Plane>, bool), String> {
    if let Some(one) = plane(e) {
        return Ok((vec![one], true));
    }
    let all: Option<Vec<Plane>> = e
        .as_list()
        .filter(|l| !l.is_empty())
        .and_then(|l| l.iter().map(plane).collect());
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

fn gray(e: &Expr, name: &str) -> Result<Plane, String> {
    plane(e).ok_or_else(|| format!("{name} needs a picture of one channel: a matrix of numbers"))
}

fn matrix(p: &Plane) -> Expr {
    Expr::list(
        p.iter()
            .map(|r| Expr::list(r.iter().map(|v| double(*v)).collect()))
            .collect(),
    )
}

fn picture(all: &[Plane], single: bool) -> Expr {
    match single {
        true => matrix(&all[0]),
        false => Expr::list(all.iter().map(matrix).collect()),
    }
}

fn each(e: &Expr, name: &str, f: &dyn Fn(&Plane) -> Result<Plane, String>) -> Res {
    let (all, single) = planes(e, name)?;
    let done: Vec<Plane> = all.iter().map(f).collect::<Result<_, _>>()?;
    Ok(picture(&done, single))
}

fn at(p: &Plane, i: isize, j: isize, pad: Pad) -> f64 {
    let (rows, cols) = (p.len() as isize, p[0].len() as isize);
    if (0..rows).contains(&i) && (0..cols).contains(&j) {
        return p[i as usize][j as usize];
    }
    let fold = |k: isize, n: isize| -> usize {
        match pad {
            Pad::Replicate => k.clamp(0, n - 1) as usize,
            Pad::Circular => k.rem_euclid(n) as usize,
            _ => {
                let m = k.rem_euclid(2 * n);
                (if m < n { m } else { 2 * n - 1 - m }) as usize
            }
        }
    };
    match pad {
        Pad::Value(v) => v,
        _ => p[fold(i, rows)][fold(j, cols)],
    }
}

/// The weights laid on every pixel (not turned round): as large as the picture.
fn weighed(p: &Plane, h: &Plane, pad: Pad) -> Plane {
    let (hr, hc) = (h.len() as isize, h[0].len() as isize);
    let (cr, cc) = ((hr - 1) / 2, (hc - 1) / 2);
    (0..p.len() as isize)
        .map(|i| {
            (0..p[0].len() as isize)
                .map(|j| {
                    let mut sum = 0.0;
                    for a in 0..hr {
                        for b in 0..hc {
                            let w = h[a as usize][b as usize];
                            if w != 0.0 {
                                sum += w * at(p, i + a - cr, j + b - cc, pad);
                            }
                        }
                    }
                    sum
                })
                .collect()
        })
        .collect()
}

/// The values under the places of a shape that are not zero, around every pixel.
fn under(p: &Plane, shape: &Plane, pad: Pad, turned: bool, f: &dyn Fn(&mut [f64]) -> f64) -> Plane {
    let (hr, hc) = (shape.len() as isize, shape[0].len() as isize);
    let (cr, cc) = ((hr - 1) / 2, (hc - 1) / 2);
    let mut found = Vec::new();
    (0..p.len() as isize)
        .map(|i| {
            (0..p[0].len() as isize)
                .map(|j| {
                    found.clear();
                    for a in 0..hr {
                        for b in 0..hc {
                            if shape[a as usize][b as usize] != 0.0 {
                                let (di, dj) = (a - cr, b - cc);
                                found.push(if turned {
                                    at(p, i - di, j - dj, pad)
                                } else {
                                    at(p, i + di, j + dj, pad)
                                });
                            }
                        }
                    }
                    f(&mut found)
                })
                .collect()
        })
        .collect()
}

fn largest(v: &mut [f64]) -> f64 {
    v.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

fn least(v: &mut [f64]) -> f64 {
    v.iter().copied().fold(f64::INFINITY, f64::min)
}

fn dilated(p: &Plane, shape: &Plane) -> Plane {
    under(p, shape, Pad::Value(f64::NEG_INFINITY), true, &largest)
}

fn eroded(p: &Plane, shape: &Plane) -> Plane {
    under(p, shape, Pad::Value(f64::INFINITY), false, &least)
}

fn ones(rows: usize, cols: usize) -> Plane {
    vec![vec![1.0; cols]; rows]
}

fn bell(size: (usize, usize), sigma: f64) -> Plane {
    let middle = |n: usize| (n as f64 - 1.0) / 2.0;
    let mut h: Plane = (0..size.0)
        .map(|i| {
            (0..size.1)
                .map(|j| {
                    let (di, dj) = (i as f64 - middle(size.0), j as f64 - middle(size.1));
                    crate::detmath::exp(-(di * di + dj * dj) / (2.0 * sigma * sigma))
                })
                .collect()
        })
        .collect();
    let top = h.iter().flatten().copied().fold(0.0, f64::max);
    let mut sum = 0.0;
    for v in h.iter_mut().flatten() {
        if *v < f64::EPSILON * top {
            *v = 0.0;
        }
        sum += *v;
    }
    if sum != 0.0 {
        for v in h.iter_mut().flatten() {
            *v /= sum;
        }
    }
    h
}

/// What a picture counts in: 1 for shares, 255 or 65535 for whole samples.
fn scale_of(all: &[Plane]) -> f64 {
    let top = all.iter().flatten().flatten().copied().fold(0.0, f64::max);
    let whole = all.iter().flatten().flatten().all(|v| v.fract() == 0.0);
    match (top <= 1.0, whole) {
        (true, _) => 1.0,
        (false, true) if top <= 255.0 => 255.0,
        (false, true) if top <= 65535.0 => 65535.0,
        _ => top,
    }
}

fn counts(p: &Plane, scale: f64, bins: usize) -> Vec<f64> {
    let mut all = vec![0.0; bins];
    for v in p.iter().flatten() {
        let k = ((v / scale).clamp(0.0, 1.0) * (bins as f64 - 1.0)).round() as usize;
        all[k] += 1.0;
    }
    all
}

/// The level of Otsu for counts of gray levels: the share of the way from the first to
/// the last where the two sides differ most.
fn otsu(counts: &[f64]) -> f64 {
    let total: f64 = counts.iter().sum();
    if total == 0.0 || counts.len() < 2 {
        return 0.0;
    }
    let mean_all: f64 = counts
        .iter()
        .enumerate()
        .map(|(k, c)| k as f64 * c)
        .sum::<f64>()
        / total;
    let (mut weight, mut mean, mut best, mut found) = (0.0, 0.0, -1.0, Vec::new());
    for (k, c) in counts.iter().enumerate().take(counts.len() - 1) {
        weight += c / total;
        mean += k as f64 * c / total;
        if weight <= 0.0 || weight >= 1.0 {
            continue;
        }
        let between = (mean_all * weight - mean).powi(2) / (weight * (1.0 - weight));
        if between > best * (1.0 + 1e-12) + 1e-300 {
            best = between;
            found = vec![k];
        } else if (between - best).abs() <= 1e-12 * best.abs() {
            found.push(k);
        }
    }
    match found.is_empty() {
        true => 0.0,
        false => {
            found.iter().sum::<usize>() as f64 / found.len() as f64 / (counts.len() as f64 - 1.0)
        }
    }
}

/// The numbers of the parts of a picture that hang together (0 outside), numbered as
/// they are met going down the columns, and how many there are.
fn labels(p: &Plane, eight: bool) -> (Vec<Vec<usize>>, usize) {
    let (rows, cols) = (p.len(), p[0].len());
    let mut out = vec![vec![0usize; cols]; rows];
    let mut count = 0;
    let mut stack = Vec::new();
    for j in 0..cols {
        for i in 0..rows {
            if p[i][j] == 0.0 || out[i][j] != 0 {
                continue;
            }
            count += 1;
            out[i][j] = count;
            stack.push((i, j));
            while let Some((a, b)) = stack.pop() {
                for (da, db) in [
                    (-1i64, 0i64),
                    (1, 0),
                    (0, -1),
                    (0, 1),
                    (-1, -1),
                    (-1, 1),
                    (1, -1),
                    (1, 1),
                ]
                .iter()
                .take(if eight { 8 } else { 4 })
                {
                    let (x, y) = (a as i64 + da, b as i64 + db);
                    if x >= 0
                        && y >= 0
                        && (x as usize) < rows
                        && (y as usize) < cols
                        && p[x as usize][y as usize] != 0.0
                        && out[x as usize][y as usize] == 0
                    {
                        out[x as usize][y as usize] = count;
                        stack.push((x as usize, y as usize));
                    }
                }
            }
        }
    }
    (out, count)
}

fn connectivity(e: Option<&Expr>, usual: bool, name: &str) -> Result<bool, String> {
    match e.map(Expr::as_i64) {
        None => Ok(usual),
        Some(Some(4)) => Ok(false),
        Some(Some(8)) => Ok(true),
        _ => Err(format!(
            "{name}: pixels hang together by their 4 sides or by their 8 neighbours"
        )),
    }
}

/// The squares of the distances to the nearest pixel that is not zero.
#[allow(clippy::needless_range_loop)]
fn distances(p: &Plane) -> Plane {
    // along one line: the lowest of the parabolas over the places with their heights
    fn line(f: &[f64]) -> Vec<f64> {
        let n = f.len();
        let (mut v, mut z, mut k) = (vec![0usize; n], vec![0.0; n + 1], 0usize);
        let mut started = false;
        for q in 0..n {
            if f[q].is_infinite() {
                continue;
            }
            if !started {
                (v[0], z[0], z[1], k, started) = (q, f64::NEG_INFINITY, f64::INFINITY, 0, true);
                continue;
            }
            loop {
                let s = ((f[q] + (q * q) as f64) - (f[v[k]] + (v[k] * v[k]) as f64))
                    / (2.0 * q as f64 - 2.0 * v[k] as f64);
                if s <= z[k] && k > 0 {
                    k -= 1;
                } else {
                    k += 1;
                    (v[k], z[k], z[k + 1]) = (q, s, f64::INFINITY);
                    break;
                }
            }
        }
        if !started {
            return vec![f64::INFINITY; n];
        }
        let mut out = vec![0.0; n];
        let mut k = 0;
        for (q, o) in out.iter_mut().enumerate() {
            while z[k + 1] < q as f64 {
                k += 1;
            }
            *o = (q as f64 - v[k] as f64).powi(2) + f[v[k]];
        }
        out
    }
    let (rows, cols) = (p.len(), p[0].len());
    let mut d: Plane = p
        .iter()
        .map(|r| {
            line(
                &r.iter()
                    .map(|v| if *v != 0.0 { 0.0 } else { f64::INFINITY })
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    for j in 0..cols {
        let column: Vec<f64> = (0..rows).map(|i| d[i][j]).collect();
        for (i, v) in line(&column).into_iter().enumerate() {
            d[i][j] = v;
        }
    }
    d
}

fn shape_of(e: &Expr, name: &str) -> Result<Plane, String> {
    match plane(e) {
        Some(p) => Ok(p),
        None => match value(e) {
            // a number: a square of that many pixels
            Some(n) if n >= 1.0 && n.fract() == 0.0 && n <= 999.0 => {
                Ok(ones(n as usize, n as usize))
            }
            _ => Err(format!(
                "{name}: a shape is a matrix of ones and zeros, as strel(\"disk\", 2) gives one"
            )),
        },
    }
}

fn strel(a: &[Expr]) -> Res {
    let usage = "strel(\"disk\", r), strel(\"square\", w), strel(\"rectangle\", [rows, columns]), strel(\"diamond\", r), strel(\"line\", length, degrees), strel(\"octagon\", r): a shape as a matrix of ones and zeros";
    let kind = a.first().and_then(text_of).ok_or(usage)?;
    let size = |k: usize| -> Result<f64, String> {
        a.get(k)
            .and_then(value)
            .filter(|v| *v >= 0.0 && *v <= 500.0)
            .ok_or_else(|| usage.to_string())
    };
    let round = |r: f64, inside: &dyn Fn(f64, f64) -> bool| -> Plane {
        let n = r.floor() as i64;
        (-n..=n)
            .map(|i| {
                (-n..=n)
                    .map(|j| f64::from(u8::from(inside(i as f64, j as f64))))
                    .collect()
            })
            .collect()
    };
    Ok(matrix(&match kind {
        "disk" => {
            let r = size(1)?;
            round(r, &|i, j| i * i + j * j <= r * r + 1e-9)
        }
        "diamond" => {
            let r = size(1)?;
            round(r, &|i, j| i.abs() + j.abs() <= r + 1e-9)
        }
        "octagon" => {
            let r = size(1)?;
            if r % 3.0 != 0.0 {
                return Err("strel(\"octagon\", r): r is a multiple of 3".into());
            }
            round(r, &|i, j| i.abs() + j.abs() <= r + r / 3.0 + 1e-9)
        }
        "square" => {
            let w = size(1)?.max(1.0) as usize;
            ones(w, w)
        }
        "rectangle" => {
            let sizes = a
                .get(1)
                .and_then(Expr::as_list)
                .filter(|l| l.len() == 2)
                .ok_or(usage)?;
            let (rows, cols) = (
                value(&sizes[0]).ok_or(usage)?,
                value(&sizes[1]).ok_or(usage)?,
            );
            if !(1.0..=500.0).contains(&rows) || !(1.0..=500.0).contains(&cols) {
                return Err(usage.into());
            }
            ones(rows as usize, cols as usize)
        }
        "line" => {
            let (length, angle) = (
                size(1)?,
                a.get(2).and_then(value).ok_or(usage)?.to_radians(),
            );
            // the pixels nearest to the line through the middle
            let (dx, dy) = (
                (length - 1.0) / 2.0 * crate::detmath::cos(angle),
                -(length - 1.0) / 2.0 * crate::detmath::sin(angle),
            );
            let (half_r, half_c) = (dy.abs().round() as i64, dx.abs().round() as i64);
            let mut out = vec![vec![0.0; (2 * half_c + 1) as usize]; (2 * half_r + 1) as usize];
            let steps = half_r.max(half_c).max(1);
            for s in -steps..=steps {
                let share = s as f64 / steps as f64;
                let (i, j) = (
                    (share * dy).round() as i64 + half_r,
                    (share * dx).round() as i64 + half_c,
                );
                out[i as usize][j as usize] = 1.0;
            }
            out
        }
        _ => return Err(usage.into()),
    }))
}

fn fspecial(a: &[Expr]) -> Res {
    let usage = "fspecial(\"average\", size), fspecial(\"gaussian\", size, sigma), fspecial(\"laplacian\", alpha), fspecial(\"log\", size, sigma), fspecial(\"prewitt\"), fspecial(\"sobel\"), fspecial(\"unsharp\", alpha): the weights of a filter";
    let kind = a.first().and_then(text_of).ok_or(usage)?;
    let sizes = |k: usize, usual: usize| -> Result<(usize, usize), String> {
        let one = |e: &Expr| {
            value(e)
                .filter(|v| *v >= 1.0 && *v <= 999.0 && v.fract() == 0.0)
                .map(|v| v as usize)
                .ok_or_else(|| usage.to_string())
        };
        match a.get(k) {
            None => Ok((usual, usual)),
            Some(e) => match e.as_list() {
                Some([r, c]) => Ok((one(r)?, one(c)?)),
                Some(_) => Err(usage.into()),
                None => Ok((one(e)?, one(e)?)),
            },
        }
    };
    let number = |k: usize, usual: f64| -> Result<f64, String> {
        match a.get(k) {
            None => Ok(usual),
            Some(e) => value(e).ok_or_else(|| usage.to_string()),
        }
    };
    let laplacian = |alpha: f64| -> Plane {
        let alpha = alpha.clamp(0.0, 1.0);
        let (h1, h2) = (alpha / (alpha + 1.0), (1.0 - alpha) / (alpha + 1.0));
        vec![
            vec![h1, h2, h1],
            vec![h2, -4.0 / (alpha + 1.0), h2],
            vec![h1, h2, h1],
        ]
    };
    Ok(matrix(&match kind {
        "average" => {
            let (r, c) = sizes(1, 3)?;
            vec![vec![1.0 / (r * c) as f64; c]; r]
        }
        "gaussian" => {
            let sigma = number(2, 0.5)?;
            if sigma <= 0.0 {
                return Err("fspecial: a sigma above zero".into());
            }
            bell(sizes(1, 3)?, sigma)
        }
        "laplacian" => laplacian(number(1, 0.2)?),
        "log" => {
            let ((r, c), sigma) = (sizes(1, 5)?, number(2, 0.5)?);
            if sigma <= 0.0 {
                return Err("fspecial: a sigma above zero".into());
            }
            let h = bell((r, c), sigma);
            let middle = |n: usize| (n as f64 - 1.0) / 2.0;
            let mut h1: Plane = (0..r)
                .map(|i| {
                    (0..c)
                        .map(|j| {
                            h[i][j]
                                * ((i as f64 - middle(r)).powi(2) + (j as f64 - middle(c)).powi(2)
                                    - 2.0 * sigma * sigma)
                                / sigma.powi(4)
                        })
                        .collect()
                })
                .collect();
            let mean = h1.iter().flatten().sum::<f64>() / (r * c) as f64;
            for v in h1.iter_mut().flatten() {
                *v -= mean;
            }
            h1
        }
        "prewitt" => vec![
            vec![1.0, 1.0, 1.0],
            vec![0.0, 0.0, 0.0],
            vec![-1.0, -1.0, -1.0],
        ],
        "sobel" => vec![
            vec![1.0, 2.0, 1.0],
            vec![0.0, 0.0, 0.0],
            vec![-1.0, -2.0, -1.0],
        ],
        "unsharp" => {
            let mut h = laplacian(number(1, 0.2)?);
            for v in h.iter_mut().flatten() {
                *v = -*v;
            }
            h[1][1] += 1.0;
            h
        }
        _ => return Err(usage.into()),
    }))
}

/// The two slopes of a picture by a method: across (x) and down (y).
fn slopes(p: &Plane, method: &str) -> Result<(Plane, Plane), String> {
    let grid = |rows: &[[f64; 3]; 3]| -> Plane { rows.iter().map(|r| r.to_vec()).collect() };
    Ok(match method {
        "sobel" | "prewitt" => {
            let m = if method == "sobel" { 2.0 } else { 1.0 };
            let down = grid(&[[-1.0, -m, -1.0], [0.0, 0.0, 0.0], [1.0, m, 1.0]]);
            let across = grid(&[[-1.0, 0.0, 1.0], [-m, 0.0, m], [-1.0, 0.0, 1.0]]);
            (weighed(p, &across, Pad::Replicate), weighed(p, &down, Pad::Replicate))
        }
        "central" | "intermediate" => {
            let (rows, cols) = (p.len(), p[0].len());
            let slope = |before: f64, here: f64, after: Option<f64>, first: bool| match (method, after) {
                ("central", Some(next)) if !first => (next - before) / 2.0,
                (_, Some(next)) => next - here,
                (_, None) => here - before,
            };
            let across = (0..rows).map(|i| (0..cols).map(|j| slope(p[i][j.saturating_sub(1)], p[i][j], p[i].get(j + 1).copied(), j == 0)).collect()).collect();
            let down = (0..rows).map(|i| (0..cols).map(|j| slope(p[i.saturating_sub(1)][j], p[i][j], p.get(i + 1).map(|r| r[j]), i == 0)).collect()).collect();
            (across, down)
        }
        "roberts" => {
            let a: Plane = vec![vec![1.0, 0.0], vec![0.0, -1.0]];
            let b: Plane = vec![vec![0.0, 1.0], vec![-1.0, 0.0]];
            (weighed(p, &a, Pad::Replicate), weighed(p, &b, Pad::Replicate))
        }
        _ => return Err("the slopes of a picture by \"sobel\", \"prewitt\", \"central\", \"intermediate\" or \"roberts\"".into()),
    })
}

fn edges(a: &[Expr]) -> Res {
    let usage = "edge(picture[, \"sobel\" | \"prewitt\" | \"roberts\" | \"canny\"[, threshold]]): 1 where a picture has an edge";
    let p = gray(a.first().ok_or(usage)?, "edge")?;
    let method = match a.get(1) {
        Some(m) => text_of(m).ok_or(usage)?.to_lowercase(),
        None => "sobel".to_string(),
    };
    let (rows, cols) = (p.len(), p[0].len());
    let scale = scale_of(std::slice::from_ref(&p));
    let p: Plane = p
        .iter()
        .map(|r| r.iter().map(|v| v / scale).collect())
        .collect();
    if method == "canny" {
        let sigma = 2f64.sqrt();
        let size = 2 * (3.0 * sigma).ceil() as usize + 1;
        let smooth = weighed(&p, &bell((size, size), sigma), Pad::Replicate);
        let (gx, gy) = slopes(&smooth, "sobel")?;
        let mut size_of: Plane = (0..rows)
            .map(|i| {
                (0..cols)
                    .map(|j| crate::detmath::hypot(gx[i][j], gy[i][j]))
                    .collect()
            })
            .collect();
        let top = size_of.iter().flatten().copied().fold(0.0, f64::max);
        if top > 0.0 {
            for v in size_of.iter_mut().flatten() {
                *v /= top;
            }
        }
        let (low, high) = match a.get(2) {
            Some(t) => match t.as_list() {
                Some([lo, hi]) => (value(lo).ok_or(usage)?, value(hi).ok_or(usage)?),
                Some(_) => return Err(usage.into()),
                None => {
                    let hi = value(t).ok_or(usage)?;
                    (0.4 * hi, hi)
                }
            },
            None => {
                // seven of ten pixels are no edges
                let mut all: Vec<f64> = size_of.iter().flatten().copied().collect();
                all.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let hi = all[((all.len() as f64 * 0.7) as usize).min(all.len() - 1)].max(1e-12);
                (0.4 * hi, hi)
            }
        };
        // the largest along the slope
        let mut thin = vec![vec![0.0; cols]; rows];
        for i in 0..rows {
            for j in 0..cols {
                let angle = crate::detmath::atan2(gy[i][j], gx[i][j])
                    .to_degrees()
                    .rem_euclid(180.0);
                let (di, dj): (isize, isize) = if !(22.5..157.5).contains(&angle) {
                    (0, 1)
                } else if angle < 67.5 {
                    (1, 1)
                } else if angle < 112.5 {
                    (1, 0)
                } else {
                    (1, -1)
                };
                let (before, after) = (
                    at(&size_of, i as isize - di, j as isize - dj, Pad::Value(0.0)),
                    at(&size_of, i as isize + di, j as isize + dj, Pad::Value(0.0)),
                );
                if size_of[i][j] >= before && size_of[i][j] > after {
                    thin[i][j] = size_of[i][j];
                }
            }
        }
        // strong edges, and the weak ones that hang on them
        let weak: Plane = thin
            .iter()
            .map(|r| r.iter().map(|v| f64::from(u8::from(*v > low))).collect())
            .collect();
        let (numbered, count) = labels(&weak, true);
        let mut kept = vec![false; count + 1];
        for i in 0..rows {
            for j in 0..cols {
                if thin[i][j] > high {
                    kept[numbered[i][j]] = true;
                }
            }
        }
        return Ok(matrix(
            &numbered
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|k| f64::from(u8::from(*k > 0 && kept[*k])))
                        .collect()
                })
                .collect(),
        ));
    }
    let (factor, share) = match method.as_str() {
        "sobel" => (4.0, 8.0),
        "prewitt" => (4.0, 6.0),
        "roberts" => (6.0, 2.0),
        _ => return Err(usage.into()),
    };
    let (mut gx, mut gy) = slopes(&p, &method)?;
    for v in gx.iter_mut().chain(gy.iter_mut()).flatten() {
        *v /= share;
    }
    let b: Plane = (0..rows)
        .map(|i| {
            (0..cols)
                .map(|j| gx[i][j].powi(2) + gy[i][j].powi(2))
                .collect()
        })
        .collect();
    let cutoff = match a.get(2) {
        Some(t) => value(t).filter(|v| *v >= 0.0).ok_or(usage)?.powi(2),
        None => factor * b.iter().flatten().sum::<f64>() / (rows * cols) as f64,
    };
    // an edge: above the cutoff and the largest across the way of the stronger slope
    let out: Plane = (0..rows)
        .map(|i| {
            (0..cols)
                .map(|j| {
                    let around = |di: isize, dj: isize| {
                        at(&b, i as isize + di, j as isize + dj, Pad::Value(0.0))
                    };
                    let here = b[i][j];
                    let (ax, ay) = (gx[i][j].abs(), gy[i][j].abs());
                    let thin = method == "roberts"
                        || (ax >= ay - 100.0 * f64::EPSILON
                            && around(0, -1) <= here
                            && here > around(0, 1))
                        || (ay >= ax - 100.0 * f64::EPSILON
                            && around(-1, 0) <= here
                            && here > around(1, 0));
                    f64::from(u8::from(here > cutoff && thin))
                })
                .collect()
        })
        .collect();
    Ok(matrix(&out))
}

/// The transform of cosines of the columns (or back), with the factors that keep lengths.
fn cosines(p: &Plane, back: bool) -> Plane {
    let n = p.len();
    let factor = |k: usize| {
        if k == 0 {
            (1.0 / n as f64).sqrt()
        } else {
            (2.0 / n as f64).sqrt()
        }
    };
    let wave = |x: usize, k: usize| {
        crate::detmath::cos(
            std::f64::consts::PI * (2.0 * x as f64 + 1.0) * k as f64 / (2.0 * n as f64),
        )
    };
    (0..n)
        .map(|k| {
            (0..p[0].len())
                .map(|j| match back {
                    false => factor(k) * (0..n).map(|x| p[x][j] * wave(x, k)).sum::<f64>(),
                    true => (0..n)
                        .map(|u| factor(u) * p[u][j] * wave(k, u))
                        .sum::<f64>(),
                })
                .collect()
        })
        .collect()
}

fn turned(p: &Plane) -> Plane {
    (0..p[0].len())
        .map(|j| p.iter().map(|r| r[j]).collect())
        .collect()
}

fn colours(e: &Expr, name: &str, f: &dyn Fn([f64; 3]) -> [f64; 3]) -> Res {
    let (all, _) = planes(e, name)?;
    if all.len() != 3 {
        return Err(format!(
            "{name} needs a picture of three channels: a list of three matrices"
        ));
    }
    let (rows, cols) = (all[0].len(), all[0][0].len());
    let mut out = vec![vec![vec![0.0; cols]; rows]; 3];
    for i in 0..rows {
        for j in 0..cols {
            let made = f([all[0][i][j], all[1][i][j], all[2][i][j]]);
            for k in 0..3 {
                out[k][i][j] = made[k];
            }
        }
    }
    Ok(picture(&out, false))
}

fn rgb_to_hsv([r, g, b]: [f64; 3]) -> [f64; 3] {
    let (top, low) = (r.max(g).max(b), r.min(g).min(b));
    let spread = top - low;
    let hue = if spread == 0.0 {
        0.0
    } else if top == r {
        ((g - b) / spread).rem_euclid(6.0)
    } else if top == g {
        (b - r) / spread + 2.0
    } else {
        (r - g) / spread + 4.0
    } / 6.0;
    [hue, if top == 0.0 { 0.0 } else { spread / top }, top]
}

fn hsv_to_rgb([h, s, v]: [f64; 3]) -> [f64; 3] {
    let h6 = (h.rem_euclid(1.0)) * 6.0;
    let part = |n: f64| {
        let k = (n + h6).rem_euclid(6.0);
        v - v * s * k.min(4.0 - k).clamp(0.0, 1.0)
    };
    [part(5.0), part(3.0), part(1.0)]
}

const WHITE: [f64; 3] = [0.950_47, 1.0, 1.088_83];

fn rgb_to_lab(rgb: [f64; 3]) -> [f64; 3] {
    let linear = rgb.map(|c| {
        if c <= 0.040_45 {
            c / 12.92
        } else {
            crate::detmath::pow((c + 0.055) / 1.055, 2.4)
        }
    });
    let xyz = [
        0.412_456_4 * linear[0] + 0.357_576_1 * linear[1] + 0.180_437_5 * linear[2],
        0.212_672_9 * linear[0] + 0.715_152_2 * linear[1] + 0.072_175 * linear[2],
        0.019_333_9 * linear[0] + 0.119_192 * linear[1] + 0.950_304_1 * linear[2],
    ];
    let f = |t: f64| {
        if t > (6.0f64 / 29.0).powi(3) {
            crate::detmath::cbrt(t)
        } else {
            t / (3.0 * (6.0f64 / 29.0).powi(2)) + 4.0 / 29.0
        }
    };
    let (fx, fy, fz) = (
        f(xyz[0] / WHITE[0]),
        f(xyz[1] / WHITE[1]),
        f(xyz[2] / WHITE[2]),
    );
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn lab_to_rgb([l, a, b]: [f64; 3]) -> [f64; 3] {
    let fy = (l + 16.0) / 116.0;
    let back = |t: f64| {
        if t > 6.0 / 29.0 {
            t.powi(3)
        } else {
            3.0 * (6.0f64 / 29.0).powi(2) * (t - 4.0 / 29.0)
        }
    };
    let xyz = [
        WHITE[0] * back(fy + a / 500.0),
        WHITE[1] * back(fy),
        WHITE[2] * back(fy - b / 200.0),
    ];
    let linear = [
        3.240_454_2 * xyz[0] - 1.537_138_5 * xyz[1] - 0.498_531_4 * xyz[2],
        -0.969_266 * xyz[0] + 1.876_010_8 * xyz[1] + 0.041_556 * xyz[2],
        0.055_643_4 * xyz[0] - 0.204_025_9 * xyz[1] + 1.057_225_2 * xyz[2],
    ];
    linear.map(|c| {
        if c <= 0.003_130_8 {
            12.92 * c
        } else {
            1.055 * crate::detmath::pow(c, 1.0 / 2.4) - 0.055
        }
    })
}

#[allow(clippy::needless_range_loop, clippy::redundant_closure_call)]
pub(crate) fn call(name: &str, a: &[Expr]) -> Option<Res> {
    let n = a.len();
    let number = |k: usize, usual: f64| -> Result<f64, String> {
        match a.get(k) {
            None => Ok(usual),
            Some(e) => value(e).ok_or_else(|| format!("{name}: {e} is no number")),
        }
    };
    let sizes = |k: usize, usual: usize| -> Result<(usize, usize), String> {
        let one = |e: &Expr| {
            value(e)
                .filter(|v| *v >= 1.0 && *v <= 999.0 && v.fract() == 0.0)
                .map(|v| v as usize)
                .ok_or_else(|| format!("{name}: a size is a whole number from 1"))
        };
        match a.get(k) {
            None => Ok((usual, usual)),
            Some(e) => match e.as_list() {
                Some([r, c]) => Ok((one(r)?, one(c)?)),
                Some(_) => Err(format!("{name}: a size or [rows, columns]")),
                None => Ok((one(e)?, one(e)?)),
            },
        }
    };
    let word_at = |k: usize| a.get(k).and_then(text_of).map(str::to_lowercase);
    let binary = |p: &Plane| -> Plane {
        p.iter()
            .map(|r| r.iter().map(|v| f64::from(u8::from(*v != 0.0))).collect())
            .collect()
    };
    Some(match (name, n) {
        ("fspecial", 1..=3) => fspecial(a),
        ("strel", 2..=3) => strel(a),
        ("imgaussfilt", 1 | 2) => (|| -> Res {
            let sigma = number(1, 0.5)?;
            if sigma <= 0.0 || sigma > 200.0 {
                return Err("imgaussfilt(picture, sigma): a sigma above zero".into());
            }
            let size = 2 * (2.0 * sigma).ceil() as usize + 1;
            let (across, down) = (bell((1, size), sigma), bell((size, 1), sigma));
            each(&a[0], name, &|p| {
                Ok(weighed(
                    &weighed(p, &across, Pad::Replicate),
                    &down,
                    Pad::Replicate,
                ))
            })
        })(),
        ("imboxfilt", 1 | 2) => (|| -> Res {
            let (r, c) = sizes(1, 3)?;
            if r % 2 == 0 || c % 2 == 0 {
                return Err("imboxfilt(picture, size): an odd size".into());
            }
            let h = vec![vec![1.0 / (r * c) as f64; c]; r];
            each(&a[0], name, &|p| Ok(weighed(p, &h, Pad::Replicate)))
        })(),
        ("medfilt2", 1..=3) => (|| -> Res {
            let (r, c) = sizes(1, 3)?;
            let pad = match word_at(2).as_deref() {
                None | Some("zeros") => Pad::Value(0.0),
                Some("symmetric") => Pad::Symmetric,
                _ => {
                    return Err(
                        "medfilt2(picture, [rows, columns], \"zeros\" | \"symmetric\")".into(),
                    );
                }
            };
            each(&a[0], name, &|p| {
                Ok(under(p, &ones(r, c), pad, false, &|v| {
                    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let half = v.len() / 2;
                    if v.len() % 2 == 1 {
                        v[half]
                    } else {
                        (v[half - 1] + v[half]) / 2.0
                    }
                }))
            })
        })(),
        ("ordfilt2", 3) => (|| -> Res {
            let shape = shape_of(&a[2], name)?;
            let places = shape.iter().flatten().filter(|v| **v != 0.0).count();
            let order = number(1, 1.0)?;
            if order < 1.0 || order > places as f64 || order.fract() != 0.0 {
                return Err(format!(
                    "ordfilt2(picture, order, shape): the order is from 1 to the {places} places of the shape"
                ));
            }
            each(&a[0], name, &|p| {
                Ok(under(p, &shape, Pad::Value(0.0), false, &|v| {
                    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    v[order as usize - 1]
                }))
            })
        })(),
        ("stdfilt" | "rangefilt", 1 | 2) => (|| -> Res {
            let shape = match a.get(1) {
                Some(e) => shape_of(e, name)?,
                None => ones(3, 3),
            };
            each(&a[0], name, &|p| {
                Ok(under(p, &shape, Pad::Symmetric, false, &|v| match name {
                    "rangefilt" => largest(v) - least(v),
                    _ => {
                        let mean = v.iter().sum::<f64>() / v.len() as f64;
                        (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
                            / (v.len() as f64 - 1.0).max(1.0))
                        .sqrt()
                    }
                }))
            })
        })(),
        ("padarray", 2..=4) => (|| -> Res {
            let usage = "padarray(picture, [rows, columns][, value | \"replicate\" | \"symmetric\" | \"circular\"][, \"both\" | \"pre\" | \"post\"])";
            let by = a[1]
                .as_list()
                .map(<[Expr]>::to_vec)
                .unwrap_or_else(|| vec![a[1].clone(), Expr::zero()]);
            let amount = |k: usize| {
                by.get(k)
                    .and_then(value)
                    .filter(|v| *v >= 0.0 && *v <= 10_000.0 && v.fract() == 0.0)
                    .map(|v| v as isize)
                    .ok_or(usage)
            };
            let (r, c) = (amount(0)?, if by.len() > 1 { amount(1)? } else { 0 });
            let (mut pad, mut side) = (Pad::Value(0.0), "both".to_string());
            for e in &a[2..] {
                match (text_of(e), value(e)) {
                    (Some("replicate"), _) => pad = Pad::Replicate,
                    (Some("symmetric"), _) => pad = Pad::Symmetric,
                    (Some("circular"), _) => pad = Pad::Circular,
                    (Some(w @ ("both" | "pre" | "post")), _) => side = w.to_string(),
                    (None, Some(v)) => pad = Pad::Value(v),
                    _ => return Err(usage.into()),
                }
            }
            let (before, after) = (
                if side == "post" { 0 } else { 1 },
                if side == "pre" { 0 } else { 1 },
            );
            each(&a[0], name, &|p| {
                let (rows, cols) = (p.len() as isize, p[0].len() as isize);
                Ok((-r * before..rows + r * after)
                    .map(|i| {
                        (-c * before..cols + c * after)
                            .map(|j| at(p, i, j, pad))
                            .collect()
                    })
                    .collect())
            })
        })(),
        ("imsharpen", 1 | 3 | 5) => (|| -> Res {
            let (mut radius, mut amount) = (1.0, 0.8);
            for pair in a[1..].chunks(2) {
                match (text_of(&pair[0]).map(str::to_lowercase).as_deref(), value(&pair[1])) {
                    (Some("radius"), Some(v)) if v > 0.0 => radius = v,
                    (Some("amount"), Some(v)) => amount = v,
                    _ => return Err("imsharpen(picture[, \"Radius\", r][, \"Amount\", a]): the picture with what a blur takes away added again".into()),
                }
            }
            let size = 2 * (2.0 * radius).ceil() as usize + 1;
            let (across, down) = (bell((1, size), radius), bell((size, 1), radius));
            each(&a[0], name, &|p| {
                let soft = weighed(&weighed(p, &across, Pad::Replicate), &down, Pad::Replicate);
                Ok((0..p.len())
                    .map(|i| {
                        (0..p[0].len())
                            .map(|j| p[i][j] + amount * (p[i][j] - soft[i][j]))
                            .collect()
                    })
                    .collect())
            })
        })(),
        ("imdilate" | "imerode" | "imopen" | "imclose" | "imtophat" | "imbothat", 2) => {
            (|| -> Res {
                let shape = shape_of(&a[1], name)?;
                each(&a[0], name, &|p| {
                    let less = |x: &Plane, y: &Plane| -> Plane {
                        (0..x.len())
                            .map(|i| (0..x[0].len()).map(|j| x[i][j] - y[i][j]).collect())
                            .collect()
                    };
                    Ok(match name {
                        "imdilate" => dilated(p, &shape),
                        "imerode" => eroded(p, &shape),
                        "imopen" => dilated(&eroded(p, &shape), &shape),
                        "imclose" => eroded(&dilated(p, &shape), &shape),
                        "imtophat" => less(p, &dilated(&eroded(p, &shape), &shape)),
                        _ => less(&eroded(&dilated(p, &shape), &shape), p),
                    })
                })
            })()
        }
        ("bwperim", 1 | 2) => (|| -> Res {
            let eight = connectivity(a.get(1), false, name)?;
            let p = binary(&gray(&a[0], name)?);
            let shape: Plane = if eight {
                ones(3, 3)
            } else {
                vec![
                    vec![0.0, 1.0, 0.0],
                    vec![1.0, 1.0, 1.0],
                    vec![0.0, 1.0, 0.0],
                ]
            };
            let inner = under(&p, &shape, Pad::Value(0.0), false, &least);
            Ok(matrix(
                &(0..p.len())
                    .map(|i| {
                        (0..p[0].len())
                            .map(|j| f64::from(u8::from(p[i][j] != 0.0 && inner[i][j] == 0.0)))
                            .collect()
                    })
                    .collect(),
            ))
        })(),
        ("imfill", 1 | 2) => (|| -> Res {
            if n == 2 && word_at(1).as_deref() != Some("holes") {
                return Err(
                    "imfill(picture, \"holes\"): the holes of the parts of a picture filled".into(),
                );
            }
            let p = binary(&gray(&a[0], name)?);
            // the background that hangs on the border by the sides of its pixels
            let outside: Plane = p
                .iter()
                .map(|r| r.iter().map(|v| 1.0 - v).collect())
                .collect();
            let (numbered, count) = labels(&outside, false);
            let (rows, cols) = (p.len(), p[0].len());
            let mut open = vec![false; count + 1];
            for i in 0..rows {
                for j in 0..cols {
                    if i == 0 || j == 0 || i + 1 == rows || j + 1 == cols {
                        open[numbered[i][j]] = true;
                    }
                }
            }
            Ok(matrix(
                &(0..rows)
                    .map(|i| {
                        (0..cols)
                            .map(|j| f64::from(u8::from(p[i][j] != 0.0 || !open[numbered[i][j]])))
                            .collect()
                    })
                    .collect(),
            ))
        })(),
        ("bwlabel" | "bwareaopen" | "imclearborder" | "bwconncomp" | "regionprops", 1..=3) => {
            (|| -> Res {
                let p = gray(&a[0], name)?;
                let extra = usize::from(name == "bwareaopen");
                if n > 2 + extra || (name == "bwareaopen" && n < 2) {
                    return Err(format!("{name}: too many or too few arguments"));
                }
                let eight = connectivity(a.get(1 + extra), true, name)?;
                let (rows, cols) = (p.len(), p[0].len());
                // (regionprops also takes a matrix of numbers of parts)
                let numbered_already =
                    name == "regionprops" && p.iter().flatten().any(|v| *v > 1.0);
                if numbered_already
                    && p.iter()
                        .flatten()
                        .any(|v| *v > (rows * cols) as f64 || v.fract() != 0.0)
                {
                    return Err("regionprops: the numbers of the parts are whole numbers, no more than the picture has pixels".into());
                }
                let (numbered, count) = match numbered_already {
                    true => (
                        p.iter()
                            .map(|r| r.iter().map(|v| v.max(0.0) as usize).collect())
                            .collect(),
                        p.iter().flatten().copied().fold(0.0, f64::max) as usize,
                    ),
                    false => labels(&p, eight),
                };
                let mut area = vec![0usize; count + 1];
                for k in numbered.iter().flatten() {
                    area[*k] += 1;
                }
                let kept = |keep: &dyn Fn(usize) -> bool| -> Expr {
                    matrix(
                        &numbered
                            .iter()
                            .map(|r| {
                                r.iter()
                                    .map(|k| f64::from(u8::from(*k > 0 && keep(*k))))
                                    .collect()
                            })
                            .collect(),
                    )
                };
                Ok(match name {
                    "bwlabel" => matrix(
                        &numbered
                            .iter()
                            .map(|r| r.iter().map(|k| *k as f64).collect())
                            .collect(),
                    ),
                    "bwareaopen" => {
                        let least = number(1, 1.0)?;
                        kept(&|k| area[k] as f64 >= least)
                    }
                    "imclearborder" => {
                        let mut touches = vec![false; count + 1];
                        for i in 0..rows {
                            for j in 0..cols {
                                if i == 0 || j == 0 || i + 1 == rows || j + 1 == cols {
                                    touches[numbered[i][j]] = true;
                                }
                            }
                        }
                        kept(&|k| !touches[k])
                    }
                    _ => {
                        // the pixels of every part, numbered down the columns from 1
                        let mut places: Vec<Vec<(usize, usize)>> = vec![Vec::new(); count + 1];
                        for j in 0..cols {
                            for i in 0..rows {
                                places[numbered[i][j]].push((i, j));
                            }
                        }
                        if name == "bwconncomp" {
                            let lists = places[1..]
                                .iter()
                                .map(|part| {
                                    Expr::list(
                                        part.iter()
                                            .map(|(i, j)| Expr::int((j * rows + i + 1) as i64))
                                            .collect(),
                                    )
                                })
                                .collect();
                            return Ok(crate::records::record(vec![
                                ("Connectivity", Expr::int(if eight { 8 } else { 4 })),
                                (
                                    "ImageSize",
                                    Expr::list(vec![
                                        Expr::int(rows as i64),
                                        Expr::int(cols as i64),
                                    ]),
                                ),
                                ("NumObjects", Expr::int(count as i64)),
                                ("PixelIdxList", crate::cells::made(lists)),
                            ]));
                        }
                        Expr::list(
                            places[1..]
                                .iter()
                                .map(|part| {
                                    let size = part.len() as f64;
                                    let (low_i, high_i) = (
                                        part.iter().map(|p| p.0).min().unwrap_or(0),
                                        part.iter().map(|p| p.0).max().unwrap_or(0),
                                    );
                                    let (low_j, high_j) = (
                                        part.iter().map(|p| p.1).min().unwrap_or(0),
                                        part.iter().map(|p| p.1).max().unwrap_or(0),
                                    );
                                    let (wide, high) =
                                        ((high_j + 1 - low_j) as f64, (high_i + 1 - low_i) as f64);
                                    let middle = |f: &dyn Fn(&(usize, usize)) -> usize| {
                                        if part.is_empty() {
                                            f64::NAN
                                        } else {
                                            part.iter().map(|p| f(p) as f64 + 1.0).sum::<f64>()
                                                / size
                                        }
                                    };
                                    crate::records::record(vec![
                                        ("Area", double(size)),
                                        (
                                            "Centroid",
                                            Expr::list(vec![
                                                double(middle(&|p| p.1)),
                                                double(middle(&|p| p.0)),
                                            ]),
                                        ),
                                        (
                                            "BoundingBox",
                                            Expr::list(vec![
                                                double(low_j as f64 + 0.5),
                                                double(low_i as f64 + 0.5),
                                                double(wide),
                                                double(high),
                                            ]),
                                        ),
                                        (
                                            "EquivDiameter",
                                            double((4.0 * size / std::f64::consts::PI).sqrt()),
                                        ),
                                        (
                                            "Extent",
                                            double(if part.is_empty() {
                                                f64::NAN
                                            } else {
                                                size / (wide * high)
                                            }),
                                        ),
                                    ])
                                })
                                .collect(),
                        )
                    }
                })
            })()
        }
        ("bweuler", 1 | 2) => (|| -> Res {
            let eight = connectivity(a.get(1), true, name)?;
            let p = binary(&gray(&a[0], name)?);
            // the squares of four pixels with one, three, and two across of them set
            let (mut one, mut three, mut across) = (0.0, 0.0, 0.0);
            for i in -1..p.len() as isize {
                for j in -1..p[0].len() as isize {
                    let q = [
                        at(&p, i, j, Pad::Value(0.0)),
                        at(&p, i, j + 1, Pad::Value(0.0)),
                        at(&p, i + 1, j, Pad::Value(0.0)),
                        at(&p, i + 1, j + 1, Pad::Value(0.0)),
                    ];
                    match q.iter().sum::<f64>() as u8 {
                        1 => one += 1.0,
                        3 => three += 1.0,
                        2 if q[0] == q[3] => across += 1.0,
                        _ => {}
                    }
                }
            }
            Ok(double(
                (one - three + if eight { -2.0 * across } else { 2.0 * across }) / 4.0,
            ))
        })(),
        ("bwdist", 1) => gray(&a[0], name).map(|p| {
            matrix(
                &distances(&p)
                    .iter()
                    .map(|r| r.iter().map(|v| v.sqrt()).collect())
                    .collect(),
            )
        }),
        ("edge", 1..=3) => edges(a),
        ("imgradientxy" | "imgradient", 1 | 2) => (|| -> Res {
            let p = gray(&a[0], name)?;
            let method = word_at(1).unwrap_or_else(|| "sobel".to_string());
            let (gx, gy) = slopes(&p, &method).map_err(|e| format!("{name}: {e}"))?;
            Ok(match name {
                "imgradientxy" => Expr::list(vec![matrix(&gx), matrix(&gy)]),
                _ => {
                    let size: Plane = (0..p.len())
                        .map(|i| {
                            (0..p[0].len())
                                .map(|j| crate::detmath::hypot(gx[i][j], gy[i][j]))
                                .collect()
                        })
                        .collect();
                    let way: Plane = (0..p.len())
                        .map(|i| {
                            (0..p[0].len())
                                .map(|j| {
                                    if gx[i][j] == 0.0 && gy[i][j] == 0.0 {
                                        0.0
                                    } else {
                                        crate::detmath::atan2(-gy[i][j], gx[i][j]).to_degrees()
                                    }
                                })
                                .collect()
                        })
                        .collect();
                    Expr::list(vec![matrix(&size), matrix(&way)])
                }
            })
        })(),
        ("graythresh", 1) => gray(&a[0], name)
            .map(|p| double(otsu(&counts(&p, scale_of(std::slice::from_ref(&p)), 256)))),
        ("otsuthresh", 1) => (|| -> Res {
            let all: Vec<f64> = a[0]
                .as_list()
                .ok_or("otsuthresh([counts]): how many pixels have every level of gray")?
                .iter()
                .map(|c| {
                    value(c)
                        .filter(|v| *v >= 0.0)
                        .ok_or("otsuthresh: counts that are not negative")
                })
                .collect::<Result<_, _>>()?;
            Ok(double(otsu(&all)))
        })(),
        ("stretchlim" | "imadjust" | "histeq", 1..=4) => {
            (|| -> Res {
                let (all, single) = planes(&a[0], name)?;
                let scale = scale_of(&all);
                // where the lowest and the highest hundredth of the pixels end
                let limits = |p: &Plane, low: f64, high: f64| -> (f64, f64) {
                    let found = counts(p, scale, 256);
                    let total: f64 = found.iter().sum();
                    let mut sum = 0.0;
                    let (mut first, mut last) = (None, None);
                    for (k, c) in found.iter().enumerate() {
                        sum += c / total;
                        if first.is_none() && sum > low {
                            first = Some(k);
                        }
                        if last.is_none() && sum >= high {
                            last = Some(k);
                        }
                    }
                    match (first, last) {
                        (Some(a), Some(b)) if a < b => (a as f64 / 255.0, b as f64 / 255.0),
                        _ => (0.0, 1.0),
                    }
                };
                let pair = |e: &Expr, what: &str| -> Result<(f64, f64), String> {
                    match e.as_list() {
                        Some([lo, hi]) => Ok((value(lo).ok_or(what)?, value(hi).ok_or(what)?)),
                        _ => Err(format!("{name}: {what}")),
                    }
                };
                let again = |v: f64| if scale > 1.0 { (v * scale).round() } else { v };
                match name {
                    "stretchlim" => {
                        let (low, high) = match a.get(1) {
                            Some(e) => match e.as_list() {
                                Some(_) => pair(e, "[the low share, the high share]")?,
                                None => {
                                    let t = value(e).ok_or("stretchlim(picture, share)")?;
                                    (t, 1.0 - t)
                                }
                            },
                            None => (0.01, 0.99),
                        };
                        let found: Vec<(f64, f64)> =
                            all.iter().map(|p| limits(p, low, high)).collect();
                        Ok(match single {
                            true => Expr::list(vec![double(found[0].0), double(found[0].1)]),
                            false => Expr::list(vec![
                                Expr::list(found.iter().map(|f| double(f.0)).collect()),
                                Expr::list(found.iter().map(|f| double(f.1)).collect()),
                            ]),
                        })
                    }
                    "imadjust" => {
                        let out = match a.get(2) {
                            Some(e) => pair(e, "[the low value, the high value] to come out")?,
                            None => (0.0, 1.0),
                        };
                        let gamma = number(3, 1.0)?;
                        if gamma <= 0.0 {
                            return Err("imadjust: a gamma above zero".into());
                        }
                        let done: Vec<Plane> = all
                        .iter()
                        .map(|p| {
                            let (low, high) = match a.get(1).filter(|e| e.as_list().is_some_and(|l| !l.is_empty())) {
                                Some(e) => pair(e, "[the low value, the high value] that go in")?,
                                None => limits(p, 0.01, 0.99),
                            };
                            if low >= high {
                                return Err("imadjust: the low value that goes in is below the high one".to_string());
                            }
                            Ok(p.iter().map(|r| r.iter().map(|v| again(out.0 + (out.1 - out.0) * crate::detmath::pow(((v / scale).clamp(low, high) - low) / (high - low), gamma))).collect()).collect())
                        })
                        .collect::<Result<_, String>>()?;
                        Ok(picture(&done, single))
                    }
                    _ => {
                        let levels = match a.get(1) {
                            Some(e) => value(e)
                                .filter(|v| *v >= 2.0 && *v <= 65536.0 && v.fract() == 0.0)
                                .ok_or("histeq(picture, levels): two levels at least")?
                                as usize,
                            None => 64,
                        };
                        let done: Vec<Plane> = all
                            .iter()
                            .map(|p| {
                                // every level goes where the sums of the counts come nearest to a flat spread
                                let found = counts(p, scale, 256);
                                let total: f64 = found.iter().sum();
                                let mut sums = Vec::with_capacity(256);
                                let mut sum = 0.0;
                                for c in &found {
                                    sum += c;
                                    sums.push(sum);
                                }
                                let flat: Vec<f64> = (1..=levels)
                                    .map(|k| total * k as f64 / levels as f64)
                                    .collect();
                                let slack: Vec<f64> = (0..256)
                                    .map(|k| {
                                        if k == 0 {
                                            found[0].min(found[1]) / 2.0
                                        } else {
                                            found[k - 1].min(found[k]) / 2.0
                                        }
                                    })
                                    .collect();
                                let map: Vec<f64> = (0..256)
                                    .map(|k| {
                                        let slack = if k + 1 < 256 {
                                            found[k].min(found[k + 1]) / 2.0
                                        } else {
                                            slack[k]
                                        };
                                        let mut best = (f64::INFINITY, 0usize);
                                        for (m, target) in flat.iter().enumerate() {
                                            let mut err = target - sums[k] + slack;
                                            if err < -total * f64::EPSILON.sqrt() {
                                                err = total;
                                            }
                                            if err < best.0 {
                                                best = (err, m);
                                            }
                                        }
                                        best.1 as f64 / (levels as f64 - 1.0)
                                    })
                                    .collect();
                                p.iter()
                                    .map(|r| {
                                        r.iter()
                                            .map(|v| {
                                                again(
                                                    map[((v / scale).clamp(0.0, 1.0) * 255.0)
                                                        .round()
                                                        as usize],
                                                )
                                            })
                                            .collect()
                                    })
                                    .collect()
                            })
                            .collect();
                        Ok(picture(&done, single))
                    }
                }
            })()
        }
        ("imnoise", 2..=4) => (|| -> Res {
            let usage = "imnoise(picture, \"gaussian\", mean, variance), imnoise(picture, \"salt & pepper\", density) or imnoise(picture, \"speckle\", variance): a picture with noise from the stream of rng";
            let kind = word_at(1).ok_or(usage)?;
            let (all, single) = planes(&a[0], name)?;
            let scale = scale_of(&all);
            let again = |v: f64| {
                if scale > 1.0 {
                    (v.clamp(0.0, 1.0) * scale).round()
                } else {
                    v.clamp(0.0, 1.0)
                }
            };
            let (rows, cols) = (all[0].len(), all[0][0].len());
            let draw = |which: &str| -> f64 {
                crate::random::call(which, &[])
                    .and_then(Result::ok)
                    .and_then(|v| crate::plot::number(&v))
                    .unwrap_or(0.0)
            };
            let mut out = all.clone();
            // (the numbers are drawn down the columns, channel after channel, as MATLAB fills its arrays)
            match kind.as_str() {
                "gaussian" => {
                    let (mean, spread) = (number(2, 0.0)?, number(3, 0.01)?);
                    if spread < 0.0 {
                        return Err(usage.into());
                    }
                    for p in out.iter_mut() {
                        for j in 0..cols {
                            for i in 0..rows {
                                p[i][j] =
                                    again(p[i][j] / scale + spread.sqrt() * draw("randn") + mean);
                            }
                        }
                    }
                }
                "salt & pepper" | "salt and pepper" | "saltpepper" => {
                    let density = number(2, 0.05)?;
                    if !(0.0..=1.0).contains(&density) {
                        return Err(usage.into());
                    }
                    for p in out.iter_mut() {
                        for j in 0..cols {
                            for i in 0..rows {
                                let x = draw("rand");
                                if x < density / 2.0 {
                                    p[i][j] = 0.0;
                                } else if x < density {
                                    p[i][j] = scale;
                                }
                            }
                        }
                    }
                }
                "speckle" => {
                    let spread = number(2, 0.05)?;
                    if spread < 0.0 {
                        return Err(usage.into());
                    }
                    for p in out.iter_mut() {
                        for j in 0..cols {
                            for i in 0..rows {
                                let v = p[i][j] / scale;
                                p[i][j] =
                                    again(v + (12.0 * spread).sqrt() * v * (draw("rand") - 0.5));
                            }
                        }
                    }
                }
                _ => return Err(usage.into()),
            }
            Ok(picture(&out, single))
        })(),
        ("mean2" | "std2" | "entropy", 1) => (|| -> Res {
            let (all, _) = planes(&a[0], name)?;
            let values: Vec<f64> = all.iter().flatten().flatten().copied().collect();
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            Ok(double(match name {
                "mean2" => mean,
                "std2" => (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
                    / (values.len() as f64 - 1.0).max(1.0))
                .sqrt(),
                _ => {
                    let scale = scale_of(&all);
                    let mut found = vec![0.0; 256];
                    for v in &values {
                        found[((v / scale).clamp(0.0, 1.0) * 255.0).round() as usize] += 1.0;
                    }
                    -found
                        .iter()
                        .filter(|c| **c > 0.0)
                        .map(|c| {
                            c / values.len() as f64 * crate::detmath::log2(c / values.len() as f64)
                        })
                        .sum::<f64>()
                }
            }))
        })(),
        ("corr2" | "immse" | "psnr" | "imabsdiff", 2 | 3) => (|| -> Res {
            let ((x, single), (y, _)) = (planes(&a[0], name)?, planes(&a[1], name)?);
            if x.len() != y.len() || x[0].len() != y[0].len() || x[0][0].len() != y[0][0].len() {
                return Err(format!("{name}: two pictures of one size"));
            }
            if n == 3 && name != "psnr" {
                return Err(format!("{name} takes two pictures"));
            }
            let (u, v): (Vec<f64>, Vec<f64>) = (
                x.iter().flatten().flatten().copied().collect(),
                y.iter().flatten().flatten().copied().collect(),
            );
            let count = u.len() as f64;
            let error = u.iter().zip(&v).map(|(a, b)| (a - b).powi(2)).sum::<f64>() / count;
            Ok(match name {
                "imabsdiff" => picture(
                    &x.iter()
                        .zip(&y)
                        .map(|(p, q)| {
                            (0..p.len())
                                .map(|i| {
                                    (0..p[0].len()).map(|j| (p[i][j] - q[i][j]).abs()).collect()
                                })
                                .collect()
                        })
                        .collect::<Vec<Plane>>(),
                    single,
                ),
                "immse" => double(error),
                "psnr" => {
                    let peak = match a.get(2) {
                        Some(e) => value(e)
                            .filter(|v| *v > 0.0)
                            .ok_or("psnr(picture, reference, peak): a peak above zero")?,
                        None => scale_of(&x).max(scale_of(&y)),
                    };
                    double(10.0 * crate::detmath::log10(peak * peak / error))
                }
                _ => {
                    let (mu, mv) = (u.iter().sum::<f64>() / count, v.iter().sum::<f64>() / count);
                    let (mut both, mut su, mut sv) = (0.0, 0.0, 0.0);
                    for (a, b) in u.iter().zip(&v) {
                        both += (a - mu) * (b - mv);
                        su += (a - mu).powi(2);
                        sv += (b - mv).powi(2);
                    }
                    double(both / (su * sv).sqrt())
                }
            })
        })(),
        ("rgb2hsv", 1) => colours(&a[0], name, &rgb_to_hsv),
        ("hsv2rgb", 1) => colours(&a[0], name, &hsv_to_rgb),
        ("rgb2lab", 1) => colours(&a[0], name, &rgb_to_lab),
        ("lab2rgb", 1) => colours(&a[0], name, &lab_to_rgb),
        ("rgb2ycbcr", 1) => colours(&a[0], name, &|[r, g, b]| {
            [
                (16.0 + 65.481 * r + 128.553 * g + 24.966 * b) / 255.0,
                (128.0 - 37.797 * r - 74.203 * g + 112.0 * b) / 255.0,
                (128.0 + 112.0 * r - 93.786 * g - 18.214 * b) / 255.0,
            ]
        }),
        ("ycbcr2rgb", 1) => colours(&a[0], name, &|[y, cb, cr]| {
            let (y, cb, cr) = (255.0 * y - 16.0, 255.0 * cb - 128.0, 255.0 * cr - 128.0);
            [
                (298.082 * y + 408.583 * cr) / 65280.0,
                (298.082 * y - 100.291 * cb - 208.120 * cr) / 65280.0,
                (298.082 * y + 516.412 * cb) / 65280.0,
            ]
            .map(|v| v.clamp(0.0, 1.0))
        }),
        ("ind2rgb", 2) => (|| -> Res {
            let usage = "ind2rgb(matrix of numbers of colours, [[r, g, b], ...]): the picture of three channels with the colours of a list";
            let (p, map) = (
                gray(&a[0], name)?,
                plane(&a[1]).filter(|m| m[0].len() == 3).ok_or(usage)?,
            );
            let colour = |v: f64, k: usize| map[(v.max(1.0) as usize - 1).min(map.len() - 1)][k];
            Ok(Expr::list(
                (0..3)
                    .map(|k| {
                        matrix(
                            &p.iter()
                                .map(|r| r.iter().map(|v| colour(*v, k)).collect())
                                .collect(),
                        )
                    })
                    .collect(),
            ))
        })(),
        ("imtranslate", 2) => (|| -> Res {
            let by = a[1].as_list().filter(|l| l.len() == 2).ok_or("imtranslate(picture, [right, down]): a picture moved, zeros where nothing comes from")?;
            let (dx, dy) = (
                value(&by[0]).ok_or("imtranslate: numbers of pixels")?,
                value(&by[1]).ok_or("imtranslate: numbers of pixels")?,
            );
            each(&a[0], name, &|p| {
                Ok((0..p.len())
                    .map(|i| {
                        (0..p[0].len())
                            .map(|j| {
                                // straight lines between the four pixels around where it comes from
                                let (y, x) = (i as f64 - dy, j as f64 - dx);
                                let (y0, x0) = (y.floor(), x.floor());
                                let (fy, fx) = (y - y0, x - x0);
                                let get =
                                    |a: f64, b: f64| at(p, a as isize, b as isize, Pad::Value(0.0));
                                (1.0 - fy) * ((1.0 - fx) * get(y0, x0) + fx * get(y0, x0 + 1.0))
                                    + fy * ((1.0 - fx) * get(y0 + 1.0, x0)
                                        + fx * get(y0 + 1.0, x0 + 1.0))
                            })
                            .collect()
                    })
                    .collect())
            })
        })(),
        ("impyramid", 1 | 2) => (|| -> Res {
            let way = word_at(1).unwrap_or_else(|| "reduce".to_string());
            let taps = [0.0625, 0.25, 0.375, 0.25, 0.0625];
            let (across, down): (Plane, Plane) =
                (vec![taps.to_vec()], taps.iter().map(|t| vec![*t]).collect());
            each(&a[0], name, &|p| match way.as_str() {
                "reduce" => {
                    let soft = weighed(&weighed(p, &across, Pad::Replicate), &down, Pad::Replicate);
                    Ok((0..p.len().div_ceil(2))
                        .map(|i| {
                            (0..p[0].len().div_ceil(2))
                                .map(|j| soft[2 * i][2 * j])
                                .collect()
                        })
                        .collect())
                }
                "expand" => {
                    let (rows, cols) = (2 * p.len() - 1, 2 * p[0].len() - 1);
                    let spread: Plane = (0..rows)
                        .map(|i| {
                            (0..cols)
                                .map(|j| {
                                    if i % 2 == 0 && j % 2 == 0 {
                                        4.0 * p[i / 2][j / 2]
                                    } else {
                                        0.0
                                    }
                                })
                                .collect()
                        })
                        .collect();
                    Ok(weighed(
                        &weighed(&spread, &across, Pad::Value(0.0)),
                        &down,
                        Pad::Value(0.0),
                    ))
                }
                _ => Err("impyramid(picture, \"reduce\" | \"expand\")".to_string()),
            })
        })(),
        ("integralimage", 1) => gray(&a[0], name).map(|p| {
            let (rows, cols) = (p.len(), p[0].len());
            let mut out = vec![vec![0.0; cols + 1]; rows + 1];
            for i in 0..rows {
                for j in 0..cols {
                    out[i + 1][j + 1] = p[i][j] + out[i][j + 1] + out[i + 1][j] - out[i][j];
                }
            }
            matrix(&out)
        }),
        ("normxcorr2" | "xcorr2", 1 | 2) => (|| -> Res {
            let (t, p) = match n {
                1 => (gray(&a[0], name)?, gray(&a[0], name)?),
                _ if name == "xcorr2" => (gray(&a[1], name)?, gray(&a[0], name)?),
                _ => (gray(&a[0], name)?, gray(&a[1], name)?),
            };
            let (tr, tc, pr, pc) = (t.len(), t[0].len(), p.len(), p[0].len());
            if name == "normxcorr2" && (tr > pr || tc > pc) {
                return Err(
                    "normxcorr2(template, picture): the template is no larger than the picture"
                        .into(),
                );
            }
            if (pr + tr) * (pc + tc) * tr * tc > 400_000_000 {
                return Err(format!("{name}: the pictures are too large"));
            }
            let mean_t = t.iter().flatten().sum::<f64>() / (tr * tc) as f64;
            let spread_t = t
                .iter()
                .flatten()
                .map(|v| (v - mean_t).powi(2))
                .sum::<f64>();
            Ok(matrix(
                &(0..pr + tr - 1)
                    .map(|i| {
                        (0..pc + tc - 1)
                            .map(|j| {
                                // the template with its last corner on the pixel (i, j)
                                let get = |a: usize, b: usize| {
                                    at(
                                        &p,
                                        i as isize + a as isize + 1 - tr as isize,
                                        j as isize + b as isize + 1 - tc as isize,
                                        Pad::Value(0.0),
                                    )
                                };
                                if name == "xcorr2" {
                                    return (0..tr)
                                        .map(|a| (0..tc).map(|b| t[a][b] * get(a, b)).sum::<f64>())
                                        .sum();
                                }
                                let mean = (0..tr)
                                    .map(|a| (0..tc).map(|b| get(a, b)).sum::<f64>())
                                    .sum::<f64>()
                                    / (tr * tc) as f64;
                                let (mut both, mut spread) = (0.0, 0.0);
                                for a in 0..tr {
                                    for b in 0..tc {
                                        let v = get(a, b) - mean;
                                        both += (t[a][b] - mean_t) * v;
                                        spread += v * v;
                                    }
                                }
                                let under = (spread * spread_t).sqrt();
                                if under > 1e-12 * (1.0 + spread_t) {
                                    both / under
                                } else {
                                    0.0
                                }
                            })
                            .collect()
                    })
                    .collect(),
            ))
        })(),
        ("dct2" | "idct2", 1) => gray(&a[0], name).map(|p| {
            let back = name == "idct2";
            matrix(&turned(&cosines(&turned(&cosines(&p, back)), back)))
        }),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    fn said(input: &str) -> String {
        crate::eval::run_line(input).map_or_else(|e| format!("error: {e}"), |lines| lines.join("|"))
    }

    #[test]
    fn shapes_parts_and_levels() {
        assert_eq!(
            said("strel(\"disk\", 1)"),
            "[[0, 1, 0], [1, 1, 1], [0, 1, 0]]"
        );
        assert_eq!(
            said("imdilate([[0, 0, 0], [0, 1, 0], [0, 0, 0]], strel(\"square\", 3))"),
            "[[1, 1, 1], [1, 1, 1], [1, 1, 1]]"
        );
        assert_eq!(
            said("imerode([[1, 1, 1, 1], [1, 1, 1, 1], [1, 1, 0, 1]], [[1, 1, 1]])"),
            "[[1, 1, 1, 1], [1, 1, 1, 1], [1, 0, 0, 0]]"
        );
        assert_eq!(
            said("bwlabel([[1, 0, 1], [1, 0, 0], [0, 0, 1]])"),
            "[[1, 0, 2], [1, 0, 0], [0, 0, 3]]"
        );
        assert_eq!(said("bwlabel([[1, 0], [0, 1]], 4)"), "[[1, 0], [0, 2]]");
        assert_eq!(
            said("imfill([[1, 1, 1], [1, 0, 1], [1, 1, 1]], \"holes\")"),
            "[[1, 1, 1], [1, 1, 1], [1, 1, 1]]"
        );
        assert_eq!(
            said("bwdist([[0, 0, 0], [0, 1, 0], [0, 0, 0]])"),
            "[[1.4142135623730951, 1, 1.4142135623730951], [1, 0, 1], [1.4142135623730951, 1, 1.4142135623730951]]"
        );
        assert_eq!(said("bweuler([[1, 1, 1], [1, 0, 1], [1, 1, 1]])"), "0");
        assert_eq!(
            said("medfilt2([[1, 2, 3], [4, 100, 6], [7, 8, 9]])"),
            "[[0, 2, 0], [2, 6, 3], [0, 6, 0]]"
        );
        assert_eq!(
            said("fspecial(\"sobel\")"),
            "[[1, 2, 1], [0, 0, 0], [-1, -2, -1]]"
        );
        assert_eq!(
            said("graythresh([[0, 0, 255, 255], [0, 10, 250, 255]])"),
            "0.5078431372549019"
        );
        assert_eq!(
            said("imadjust([[0.2, 0.4], [0.6, 0.8]], [0.2, 0.8], [0, 1])"),
            "[[0, 0.3333333333333333], [0.6666666666666665, 1]]"
        );
        assert_eq!(
            said("getfield(part(regionprops([[1, 1, 0], [0, 0, 0], [0, 1, 1]]), 2), \"Centroid\")"),
            "[2.5, 3]"
        );
        assert_eq!(
            said("integralimage([[1, 2], [3, 4]])"),
            "[[0, 0, 0], [0, 1, 3], [0, 4, 10]]"
        );
        assert_eq!(
            said("padarray([[1, 2], [3, 4]], [1, 1], \"symmetric\")"),
            "[[1, 1, 2, 2], [1, 1, 2, 2], [3, 3, 4, 4], [3, 3, 4, 4]]"
        );
        assert_eq!(
            said("rgb2hsv([[[1]], [[0]], [[0]]])"),
            "[[[0]], [[1]], [[1]]]"
        );
        assert!(said("strel(\"blob\", 2)").contains("strel("));
        assert!(said("bwlabel([[1, 0]], 6)").contains("4 sides"));
    }
}
