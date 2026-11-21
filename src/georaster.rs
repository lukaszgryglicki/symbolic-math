//! Raster maps: grids of values over the earth (or over a plane) with a reference that says
//! where their cells or postings lie; places and cells, values between the samples, parts
//! and other sizes of a grid, slopes, areas, what can be seen from where, grids in files,
//! and the height of the geoid (EGM96).

use crate::arrays::{tidy, word};
use crate::canon::{add, div, mul, sub};
use crate::eval::Session;
use crate::expr::{Expr, Rel};
use crate::matrix::{self, Rows};
use crate::numeric;
use crate::texts::text;
use rug::Float;

type Res = Result<Expr, String>;

/// Where the samples of a grid lie: cells that fill the limits, or postings on them.
#[derive(Clone)]
struct Ref {
    geographic: bool,
    postings: bool,
    /// latitudes (or y), lowest first
    ylim: [Expr; 2],
    /// longitudes (or x)
    xlim: [Expr; 2],
    rows: usize,
    cols: usize,
    /// the first row is the northernmost one
    north: bool,
    /// the first column is the easternmost one
    east: bool,
}

fn int(k: usize) -> Expr {
    Expr::int(k as i64)
}

fn num(e: &Expr, who: &str) -> Result<Float, String> {
    crate::mfun::real(e, crate::mfun::bits(), who)
}

fn items(e: &Expr) -> Vec<Expr> {
    match e.as_list() {
        Some(l) => l.to_vec(),
        None => vec![e.clone()],
    }
}

fn nothing() -> Expr {
    Expr::undefined()
}

impl Ref {
    fn steps(&self, along_y: bool) -> usize {
        let count = if along_y { self.rows } else { self.cols };
        match self.postings {
            true => count.saturating_sub(1).max(1),
            false => count,
        }
    }

    /// The size of a cell, or the way between two postings.
    fn dy(&self) -> Expr {
        tidy(div(sub(self.ylim[1].clone(), self.ylim[0].clone()), int(self.steps(true))))
    }

    fn dx(&self) -> Expr {
        tidy(div(sub(self.xlim[1].clone(), self.xlim[0].clone()), int(self.steps(false))))
    }

    fn first(&self) -> Expr {
        match self.postings {
            true => Expr::one(),
            false => Expr::half(),
        }
    }

    fn record(&self) -> Expr {
        let pair = |p: &[Expr; 2]| Expr::list(p.to_vec());
        let kind = text(if self.postings { "postings" } else { "cells" });
        let size = Expr::list(vec![int(self.rows), int(self.cols)]);
        let (from_y, from_x) = (
            text(if self.north { "north" } else { "south" }),
            text(if self.east { "east" } else { "west" }),
        );
        let (step_y, step_x) = match (self.geographic, self.postings) {
            (true, false) => ("CellExtentInLatitude", "CellExtentInLongitude"),
            (true, true) => ("SampleSpacingInLatitude", "SampleSpacingInLongitude"),
            (false, false) => ("CellExtentInWorldY", "CellExtentInWorldX"),
            (false, true) => ("SampleSpacingInWorldY", "SampleSpacingInWorldX"),
        };
        let mut fields = match self.geographic {
            true => vec![
                ("LatitudeLimits", pair(&self.ylim)),
                ("LongitudeLimits", pair(&self.xlim)),
            ],
            false => vec![
                ("XWorldLimits", pair(&self.xlim)),
                ("YWorldLimits", pair(&self.ylim)),
            ],
        };
        fields.extend([
            ("RasterSize", size),
            ("RasterInterpretation", kind),
            ("ColumnsStartFrom", from_y),
            ("RowsStartFrom", from_x),
            (step_y, self.dy()),
            (step_x, self.dx()),
            ("CoordinateSystemType", text(if self.geographic { "geographic" } else { "planar" })),
        ]);
        crate::records::record(fields)
    }

    /// The place of the grid (column x, row y, counted from 1 at the middle of the first
    /// cell) of a place of the world.
    fn to_grid(&self, y: &Expr, x: &Expr) -> (Expr, Expr) {
        let along = |value: &Expr, limits: &[Expr; 2], step: Expr, turned: bool| -> Expr {
            let way = match turned {
                true => sub(limits[1].clone(), value.clone()),
                false => sub(value.clone(), limits[0].clone()),
            };
            tidy(add(vec![self.first(), div(way, step)]))
        };
        (along(x, &self.xlim, self.dx(), self.east), along(y, &self.ylim, self.dy(), self.north))
    }

    /// The place of the world [y, x] of a place of the grid.
    fn to_world(&self, column: &Expr, row: &Expr) -> (Expr, Expr) {
        let along = |place: &Expr, limits: &[Expr; 2], step: Expr, turned: bool| -> Expr {
            let way = mul(vec![sub(place.clone(), self.first()), step]);
            tidy(match turned {
                true => sub(limits[1].clone(), way),
                false => add(vec![limits[0].clone(), way]),
            })
        };
        (along(row, &self.ylim, self.dy(), self.north), along(column, &self.xlim, self.dx(), self.east))
    }

    /// A longitude a whole number of turns away that lies within the limits, if one does.
    fn wrapped(&self, x: &Expr, who: &str) -> Result<Expr, String> {
        if !self.geographic {
            return Ok(x.clone());
        }
        let (value, low, high) = (num(x, who)?, num(&self.xlim[0], who)?, num(&self.xlim[1], who)?);
        let turns = (Float::with_val(value.prec(), &value - &low) / 360u32).floor();
        if turns.is_zero() || Float::with_val(value.prec(), &value - Float::with_val(value.prec(), &turns * 360u32)) > high {
            return Ok(x.clone());
        }
        let whole = turns.to_integer().ok_or_else(|| format!("{who}: {x} is no longitude"))?;
        Ok(tidy(sub(x.clone(), mul(vec![Expr::integer(whole), Expr::int(360)]))))
    }

    fn holds(&self, y: &Expr, x: &Expr, who: &str) -> Result<bool, String> {
        let (y, x) = (num(y, who)?, num(x, who)?);
        Ok(y >= num(&self.ylim[0], who)? && y <= num(&self.ylim[1], who)? && x >= num(&self.xlim[0], who)? && x <= num(&self.xlim[1], who)?)
    }
}

/// `name = value` or `"name", value` among the last arguments: the plain ones and the pairs.
fn options(a: &[Expr]) -> (Vec<Expr>, Vec<(String, Expr)>) {
    let (mut plain, mut pairs) = (Vec::new(), Vec::new());
    let mut k = 0;
    while k < a.len() {
        match &a[k] {
            Expr::Rel(Rel::Eq, left, right) if word(left).is_some() => {
                pairs.push((word(left).unwrap_or_default(), (**right).clone()));
            }
            e if crate::radix::text_of(e).is_some()
                && k + 1 < a.len()
                && matches!(word(e).as_deref(), Some("columnsstartfrom" | "rowsstartfrom")) =>
            {
                pairs.push((word(e).unwrap_or_default(), a[k + 1].clone()));
                k += 1;
            }
            e => plain.push(e.clone()),
        }
        k += 1;
    }
    (plain, pairs)
}

fn pair_of(e: &Expr, what: &str) -> Result<[Expr; 2], String> {
    match e.as_list() {
        Some([low, high]) => Ok([low.clone(), high.clone()]),
        _ => Err(format!("{what}: two numbers [from, to], not {e}")),
    }
}

/// The values of a list or of a matrix, with its shape.
fn shaped(e: &Expr) -> (Vec<Expr>, Option<(usize, usize)>, bool) {
    match (matrix::rows(e), e.as_list()) {
        (Some(rows), _) => (rows.concat(), Some((rows.len(), rows[0].len())), true),
        (None, Some(list)) => (list.to_vec(), None, true),
        (None, None) => (vec![e.clone()], None, false),
    }
}

fn reshaped(values: Vec<Expr>, shape: Option<(usize, usize)>, list: bool) -> Expr {
    match (shape, list) {
        (Some((_, cols)), _) => matrix::from_rows(values.chunks(cols).map(<[Expr]>::to_vec).collect()),
        (None, true) => Expr::list(values),
        (None, false) => values.into_iter().next().unwrap_or_else(nothing),
    }
}

impl Session {
    fn field(&mut self, record: &Expr, name: &str) -> Option<Expr> {
        self.call("getfield", &[record.clone(), text(name)]).ok()
    }

    /// The reference of a grid: a record that georefcells and its kin make, or the three
    /// numbers [cells a degree, northern latitude, western longitude] of older grids.
    fn grid_ref(&mut self, e: &Expr, size: Option<(usize, usize)>, who: &str) -> Result<Ref, String> {
        let usage = || format!("{who}: the reference of the grid is what georefcells, georefpostings, maprefcells or maprefpostings give (or [cells a degree, northern latitude, western longitude])");
        if let Some([density, north, west]) = e.as_list().filter(|l| l.iter().all(|x| x.as_list().is_none())) {
            let (rows, cols) = size.ok_or_else(usage)?;
            return Ok(Ref {
                geographic: true,
                postings: false,
                ylim: [tidy(sub(north.clone(), div(int(rows), density.clone()))), north.clone()],
                xlim: [west.clone(), tidy(add(vec![west.clone(), div(int(cols), density.clone())]))],
                rows,
                cols,
                north: false,
                east: false,
            });
        }
        if !crate::records::is_record(e) {
            return Err(usage());
        }
        let geographic = self.field(e, "LatitudeLimits").is_some();
        let (ylim, xlim) = match geographic {
            true => (self.field(e, "LatitudeLimits"), self.field(e, "LongitudeLimits")),
            false => (self.field(e, "YWorldLimits"), self.field(e, "XWorldLimits")),
        };
        let (Some(ylim), Some(xlim), Some(size_given)) = (ylim, xlim, self.field(e, "RasterSize")) else {
            return Err(usage());
        };
        let said = |s: &mut Session, name: &str| s.field(e, name).and_then(|v| word(&v));
        let sizes = items(&size_given);
        let (Some(rows), Some(cols)) = (sizes.first().and_then(Expr::as_i64), sizes.get(1).and_then(Expr::as_i64)) else {
            return Err(usage());
        };
        let found = Ref {
            geographic,
            postings: said(self, "RasterInterpretation").as_deref() == Some("postings"),
            ylim: pair_of(&ylim, who)?,
            xlim: pair_of(&xlim, who)?,
            rows: rows as usize,
            cols: cols as usize,
            north: said(self, "ColumnsStartFrom").as_deref() == Some("north"),
            east: said(self, "RowsStartFrom").as_deref() == Some("east"),
        };
        if let Some((r, c)) = size
            && (r, c) != (found.rows, found.cols)
        {
            return Err(format!(
                "{who}: the grid has {r} rows and {c} columns, its reference {} and {}",
                found.rows, found.cols
            ));
        }
        Ok(found)
    }

    /// The matrix of a grid and its reference.
    fn raster_of(&mut self, z: &Expr, r: &Expr, who: &str) -> Result<(Rows, Ref), String> {
        let rows = matrix::rows(z).ok_or_else(|| format!("{who}: the grid is a matrix of values"))?;
        let reference = self.grid_ref(r, Some((rows.len(), rows[0].len())), who)?;
        Ok((rows, reference))
    }

    fn new_ref(&mut self, name: &str, a: &[Expr]) -> Res {
        let (plain, pairs) = options(a);
        let geographic = name.starts_with("geo");
        let postings = name.ends_with("postings");
        let usage = || match geographic {
            true => format!("{name}([south, north], [west, east], [rows, columns]) or {name}([south, north], [west, east], degrees of latitude, degrees of longitude)[, ColumnsStartFrom = \"north\"][, RowsStartFrom = \"east\"]"),
            false => format!("{name}([x from, to], [y from, to], [rows, columns]) or {name}([x from, to], [y from, to], size in x, size in y)[, ColumnsStartFrom = \"north\"][, RowsStartFrom = \"east\"]"),
        };
        if !(3..=4).contains(&plain.len()) {
            return Err(usage());
        }
        let (first, second) = (pair_of(&plain[0], name)?, pair_of(&plain[1], name)?);
        let (ylim, xlim) = if geographic { (first, second) } else { (second, first) };
        let rising = |p: &[Expr; 2]| -> Result<bool, String> { Ok(num(&p[0], name)? < num(&p[1], name)?) };
        if !rising(&ylim)? || !rising(&xlim)? {
            return Err(usage());
        }
        if geographic && (num(&ylim[0], name)? < -90 || num(&ylim[1], name)? > 90 || Float::with_val(64, num(&xlim[1], name)? - num(&xlim[0], name)?) > 360) {
            return Err(format!("{name}: latitudes from -90 to 90, longitudes over 360 degrees at most"));
        }
        let (mut ylim, mut xlim) = (ylim, xlim);
        let (rows, cols) = match plain.len() {
            3 => {
                let sizes = items(&plain[2]);
                match (sizes.first().and_then(Expr::as_i64), sizes.get(1).and_then(Expr::as_i64)) {
                    (Some(r), Some(c)) if sizes.len() == 2 && r >= 1 && c >= 1 && r <= 100_000 && c <= 100_000 => (r as usize, c as usize),
                    _ => return Err(usage()),
                }
            }
            _ => {
                // the sizes of the cells: whole numbers of them, the far limits moved to fit
                let (step_y, step_x) = if geographic { (&plain[2], &plain[3]) } else { (&plain[3], &plain[2]) };
                let count = |limits: &mut [Expr; 2], step: &Expr| -> Result<usize, String> {
                    let wide = num(&limits[1], name)? - num(&limits[0], name)?;
                    let size = num(step, name)?;
                    if size <= 0 {
                        return Err(usage());
                    }
                    let cells = (wide / &size + Float::with_val(64, 1e-9)).floor().max(&Float::with_val(64, 1)).to_f64() as usize;
                    if cells > 100_000 {
                        return Err(format!("{name}: 100000 rows or columns at most"));
                    }
                    limits[1] = tidy(add(vec![limits[0].clone(), mul(vec![int(cells), step.clone()])]));
                    Ok(match postings {
                        true => cells + 1,
                        false => cells,
                    })
                };
                (count(&mut ylim, step_y)?, count(&mut xlim, step_x)?)
            }
        };
        if postings && (rows < 2 || cols < 2) {
            return Err(format!("{name}: postings need two rows and two columns at least"));
        }
        let mut made = Ref {
            geographic,
            postings,
            ylim,
            xlim,
            rows,
            cols,
            north: false,
            east: false,
        };
        for (key, value) in pairs {
            match (key.as_str(), word(&value).as_deref()) {
                ("columnsstartfrom", Some("north")) => made.north = true,
                ("columnsstartfrom", Some("south")) => made.north = false,
                ("rowsstartfrom", Some("east")) => made.east = true,
                ("rowsstartfrom", Some("west")) => made.east = false,
                _ => return Err(usage()),
            }
        }
        Ok(made.record())
    }

    /// The value of a grid at a place of the grid: the nearest sample, between four of them
    /// along straight lines, or between sixteen along curves (cubic convolution).
    fn between(grid: &Rows, reference: &Ref, column: &Expr, row: &Expr, method: &str, who: &str) -> Res {
        let (rows, cols) = (reference.rows, reference.cols);
        // (in the half cells at the border the outermost samples stand)
        let clamp = |place: &Expr, most: usize| -> Result<Expr, String> {
            let value = num(place, who)?;
            Ok(if value < 1 {
                Expr::one()
            } else if value > most as u64 {
                int(most)
            } else {
                place.clone()
            })
        };
        let (x, y) = (clamp(column, cols)?, clamp(row, rows)?);
        let whole = |place: &Expr, most: usize| -> Result<(usize, Expr), String> {
            let value = num(place, who)?;
            let below = (value.floor().to_f64() as usize).clamp(1, most.max(2) - 1).min(most);
            Ok((below, tidy(sub(place.clone(), int(below)))))
        };
        let at = |r: usize, c: usize| grid[r.clamp(1, rows) - 1][c.clamp(1, cols) - 1].clone();
        match method {
            "nearest" => {
                let near = |place: &Expr, most: usize| -> Result<usize, String> {
                    let value = num(place, who)? + Float::with_val(64, 0.5);
                    Ok((value.floor().to_f64() as usize).clamp(1, most))
                };
                Ok(at(near(&y, rows)?, near(&x, cols)?))
            }
            "linear" | "bilinear" => {
                let ((c, fx), (r, fy)) = (whole(&x, cols)?, whole(&y, rows)?);
                let blend = |a: Expr, b: Expr, f: &Expr| add(vec![mul(vec![sub(Expr::one(), f.clone()), a]), mul(vec![f.clone(), b])]);
                Ok(tidy(blend(
                    blend(at(r, c), at(r, c + 1), &fx),
                    blend(at(r + 1, c), at(r + 1, c + 1), &fx),
                    &fy,
                )))
            }
            "cubic" | "bicubic" => {
                let ((c, fx), (r, fy)) = (whole(&x, cols)?, whole(&y, rows)?);
                // the weights of the four samples around a place (Keys, a = -1/2)
                let weights = |f: &Expr| -> Vec<Expr> {
                    let p = |k: i64| Expr::int(k);
                    let (f2, f3) = (mul(vec![f.clone(), f.clone()]), mul(vec![f.clone(), f.clone(), f.clone()]));
                    vec![
                        tidy(div(add(vec![mul(vec![p(-1), f3.clone()]), mul(vec![p(2), f2.clone()]), mul(vec![p(-1), f.clone()])]), p(2))),
                        tidy(div(add(vec![mul(vec![p(3), f3.clone()]), mul(vec![p(-5), f2.clone()]), p(2)]), p(2))),
                        tidy(div(add(vec![mul(vec![p(-3), f3.clone()]), mul(vec![p(4), f2.clone()]), f.clone()]), p(2))),
                        tidy(div(add(vec![f3, mul(vec![p(-1), f2])]), p(2))),
                    ]
                };
                let (wx, wy) = (weights(&fx), weights(&fy));
                let mut terms = Vec::new();
                for (i, weight_y) in wy.iter().enumerate() {
                    for (j, weight_x) in wx.iter().enumerate() {
                        // (beyond the border the outermost samples go on)
                        let value = at((r + i).saturating_sub(1).max(1), (c + j).saturating_sub(1).max(1));
                        terms.push(mul(vec![weight_y.clone(), weight_x.clone(), value]));
                    }
                }
                Ok(tidy(add(terms)))
            }
            other => Err(format!("{who}: the values between the samples are \"nearest\", \"linear\" or \"cubic\", not \"{other}\"")),
        }
    }

    /// The grid as numbers, for the work that needs many values.
    fn heights_of(grid: &Rows, who: &str) -> Result<Vec<Vec<f64>>, String> {
        grid.iter()
            .map(|row| {
                row.iter()
                    .map(|x| match x.is_undefined() {
                        true => Ok(f64::NAN),
                        false => num(x, who).map(|v| v.to_f64()),
                    })
                    .collect()
            })
            .collect()
    }

    /// Whether a place is seen from another over the heights of a grid on a round earth:
    /// [seen, seen along the way, the ways from the watcher, the heights of the ground,
    /// the latitudes, the longitudes].
    #[allow(clippy::too_many_arguments)]
    fn sight(heights: &[Vec<f64>], reference: &Ref, limits: [f64; 4], from: (f64, f64), to: (f64, f64), above: (f64, f64), radius: f64) -> (bool, Vec<bool>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let [south, north, west, east] = limits;
        let (rows, cols) = (reference.rows, reference.cols);
        let first = if reference.postings { 1.0 } else { 0.5 };
        let steps = |count: usize| if reference.postings { (count - 1).max(1) as f64 } else { count as f64 };
        let (dy, dx) = ((north - south) / steps(rows), (east - west) / steps(cols));
        let ground = |lat: f64, lon: f64| -> f64 {
            let mut lon = lon;
            while lon < west {
                lon += 360.0;
            }
            while lon > east && lon - 360.0 >= west {
                lon -= 360.0;
            }
            let y = first + if reference.north { (north - lat) / dy } else { (lat - south) / dy };
            let x = first + if reference.east { (east - lon) / dx } else { (lon - west) / dx };
            let (y, x) = (y.clamp(1.0, rows as f64), x.clamp(1.0, cols as f64));
            let (r, c) = ((y.floor() as usize).clamp(1, rows.max(2) - 1).min(rows), (x.floor() as usize).clamp(1, cols.max(2) - 1).min(cols));
            let (fy, fx) = (y - r as f64, x - c as f64);
            let at = |r: usize, c: usize| heights[r.clamp(1, rows) - 1][c.clamp(1, cols) - 1];
            (1.0 - fy) * ((1.0 - fx) * at(r, c) + fx * at(r, c + 1)) + fy * ((1.0 - fx) * at(r + 1, c) + fx * at(r + 1, c + 1))
        };
        let unit = |lat: f64, lon: f64| -> [f64; 3] {
            let (la, lo) = (lat.to_radians(), lon.to_radians());
            [la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin()]
        };
        let (a, b) = (unit(from.0, from.1), unit(to.0, to.1));
        let dot = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
        let whole = dot.acos();
        let count = ((whole.to_degrees() / (dy.min(dx) / 2.0)).ceil() as usize).clamp(1, 200_000);
        let (mut lats, mut lons, mut ways, mut grounds, mut seen) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let eye = radius + ground(from.0, from.1) + above.0;
        let mut steepest = f64::NEG_INFINITY;
        let mut last = true;
        for k in 0..=count {
            let share = k as f64 / count as f64;
            let angle = whole * share;
            // along the great circle
            let (lat, lon) = match whole < 1e-15 {
                true => from,
                false => {
                    let (wa, wb) = (((1.0 - share) * whole).sin() / whole.sin(), (share * whole).sin() / whole.sin());
                    let p = [wa * a[0] + wb * b[0], wa * a[1] + wb * b[1], wa * a[2] + wb * b[2]];
                    (p[2].clamp(-1.0, 1.0).asin().to_degrees(), p[1].atan2(p[0]).to_degrees())
                }
            };
            let height = ground(lat, lon);
            lats.push(lat);
            lons.push(lon);
            ways.push(angle * radius);
            grounds.push(height);
            if k == 0 {
                seen.push(true);
                continue;
            }
            // how steeply the watcher looks up to a point above this place
            let rise = |h: f64| {
                let far = radius + h;
                (far * angle.cos() - eye).atan2(far * angle.sin())
            };
            let target = if k == count { height + above.1 } else { height };
            let visible = rise(target) >= steepest - 1e-13;
            seen.push(visible);
            last = visible;
            steepest = steepest.max(rise(height));
        }
        (last, seen, ways, grounds, lats, lons)
    }

    pub(crate) fn georaster(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        let n = a.len();
        let who = name;
        Some((|| -> Res {
            Ok(match (name, n) {
                ("georefcells" | "georefpostings" | "maprefcells" | "maprefpostings", 3..=8) => self.new_ref(name, a)?,
                ("refvectogeorasterreference", 2) => {
                    let sizes = items(&a[1]);
                    let (Some(rows), Some(cols)) = (sizes.first().and_then(Expr::as_i64), sizes.get(1).and_then(Expr::as_i64)) else {
                        return Err("refvectogeorasterreference([cells a degree, northern latitude, western longitude], [rows, columns])".into());
                    };
                    self.grid_ref(&a[0], Some((rows as usize, cols as usize)), who)?.record()
                }
                ("geographictointrinsic" | "worldtointrinsic" | "geographictodiscrete" | "worldtodiscrete", 3) => {
                    let reference = self.grid_ref(&a[0], None, who)?;
                    // (of a map the x comes first, of the earth the latitude)
                    let (ys, xs) = if reference.geographic { (&a[1], &a[2]) } else { (&a[2], &a[1]) };
                    let ((ys, shape, list), (xs, _, _)) = (shaped(ys), shaped(xs));
                    if ys.len() != xs.len() {
                        return Err(format!("{name}: as many of one coordinate as of the other"));
                    }
                    let discrete = name.ends_with("discrete");
                    let (mut first, mut second) = (Vec::new(), Vec::new());
                    for (y, x) in ys.iter().zip(&xs) {
                        let x = reference.wrapped(x, who)?;
                        let (column, row) = reference.to_grid(y, &x);
                        match discrete {
                            false => {
                                first.push(column);
                                second.push(row);
                            }
                            true => match reference.holds(y, &x, who)? {
                                false => {
                                    first.push(nothing());
                                    second.push(nothing());
                                }
                                true => {
                                    let near = |place: &Expr, most: usize| -> Result<Expr, String> {
                                        let value = num(place, who)? + Float::with_val(64, 0.5);
                                        Ok(int((value.floor().to_f64() as usize).clamp(1, most)))
                                    };
                                    first.push(near(&row, reference.rows)?);
                                    second.push(near(&column, reference.cols)?);
                                }
                            },
                        }
                    }
                    Expr::list(vec![reshaped(first, shape, list), reshaped(second, shape, list)])
                }
                ("intrinsictogeographic" | "intrinsictoworld", 3) => {
                    let reference = self.grid_ref(&a[0], None, who)?;
                    let ((xs, shape, list), (ys, _, _)) = (shaped(&a[1]), shaped(&a[2]));
                    if ys.len() != xs.len() {
                        return Err(format!("{name}: as many columns as rows"));
                    }
                    let (mut first, mut second) = (Vec::new(), Vec::new());
                    for (x, y) in xs.iter().zip(&ys) {
                        let (world_y, world_x) = reference.to_world(x, y);
                        match reference.geographic {
                            true => {
                                first.push(world_y);
                                second.push(world_x);
                            }
                            false => {
                                first.push(world_x);
                                second.push(world_y);
                            }
                        }
                    }
                    Expr::list(vec![reshaped(first, shape, list), reshaped(second, shape, list)])
                }
                ("geointerp" | "mapinterp" | "ltln2val", 4 | 5) => {
                    let (grid, reference) = self.raster_of(&a[0], &a[1], who)?;
                    let method = match a.get(4) {
                        Some(m) => word(m).ok_or_else(|| format!("{name}(grid, reference, places, places[, \"nearest\" | \"linear\" | \"cubic\"])"))?,
                        None if name == "ltln2val" => "nearest".to_string(),
                        None => "linear".to_string(),
                    };
                    let (ys, xs) = if reference.geographic { (&a[2], &a[3]) } else { (&a[3], &a[2]) };
                    let ((ys, shape, list), (xs, _, _)) = (shaped(ys), shaped(xs));
                    if ys.len() != xs.len() {
                        return Err(format!("{name}: as many of one coordinate as of the other"));
                    }
                    let mut values = Vec::new();
                    for (y, x) in ys.iter().zip(&xs) {
                        let x = reference.wrapped(x, who)?;
                        values.push(match reference.holds(y, &x, who)? {
                            false => nothing(),
                            true => {
                                let (column, row) = reference.to_grid(y, &x);
                                Self::between(&grid, &reference, &column, &row, &method, who)?
                            }
                        });
                    }
                    reshaped(values, shape, list)
                }
                ("geocrop" | "mapcrop", 4) => {
                    let (grid, reference) = self.raster_of(&a[0], &a[1], who)?;
                    let (first, second) = (pair_of(&a[2], who)?, pair_of(&a[3], who)?);
                    let (ylim, xlim) = if reference.geographic { (first, second) } else { (second, first) };
                    // the rows and columns whose cells reach into the limits (postings: lie in them)
                    let kept = |limits: &[Expr; 2], own: &[Expr; 2], count: usize, step: &Expr, turned: bool| -> Result<(usize, usize), String> {
                        let (low, high) = (num(&limits[0], who)?, num(&limits[1], who)?);
                        let (start, size) = (num(&own[0], who)?, num(step, who)?);
                        let mut found: Vec<usize> = Vec::new();
                        for k in 0..count {
                            let (from, to) = match reference.postings {
                                true => {
                                    let at = Float::with_val(64, &start + Float::with_val(64, &size * k as u64));
                                    (at.clone(), at)
                                }
                                false => (
                                    Float::with_val(64, &start + Float::with_val(64, &size * k as u64)),
                                    Float::with_val(64, &start + Float::with_val(64, &size * (k + 1) as u64)),
                                ),
                            };
                            let inside = match reference.postings {
                                true => from >= low && to <= high,
                                false => to > low && from < high,
                            };
                            if inside {
                                found.push(k);
                            }
                        }
                        let (Some(first), Some(last)) = (found.first(), found.last()) else {
                            return Err(format!("{name}: the limits leave nothing of the grid"));
                        };
                        // counted from the first row or column of the matrix
                        Ok(match turned {
                            true => (count - 1 - last, count - 1 - first),
                            false => (*first, *last),
                        })
                    };
                    let (dy, dx) = (reference.dy(), reference.dx());
                    let (r0, r1) = kept(&ylim, &reference.ylim, reference.rows, &dy, reference.north)?;
                    let (c0, c1) = kept(&xlim, &reference.xlim, reference.cols, &dx, reference.east)?;
                    let part: Rows = grid[r0..=r1].iter().map(|row| row[c0..=c1].to_vec()).collect();
                    // the limits of what is kept
                    let edges = |own: &[Expr; 2], step: &Expr, from: usize, to: usize, count: usize, turned: bool| -> [Expr; 2] {
                        let (low, high) = match turned {
                            true => (count - 1 - to, count - 1 - from),
                            false => (from, to),
                        };
                        let more = usize::from(!reference.postings);
                        [
                            tidy(add(vec![own[0].clone(), mul(vec![int(low), step.clone()])])),
                            tidy(add(vec![own[0].clone(), mul(vec![int(high + more), step.clone()])])),
                        ]
                    };
                    let cut = Ref {
                        ylim: edges(&reference.ylim, &dy, r0, r1, reference.rows, reference.north),
                        xlim: edges(&reference.xlim, &dx, c0, c1, reference.cols, reference.east),
                        rows: r1 - r0 + 1,
                        cols: c1 - c0 + 1,
                        ..reference
                    };
                    Expr::list(vec![matrix::from_rows(part), cut.record()])
                }
                ("georesize" | "mapresize", 3 | 4) => {
                    let (grid, reference) = self.raster_of(&a[0], &a[1], who)?;
                    let scales = items(&a[2]);
                    let (sy, sx) = match scales.len() {
                        1 => (num(&scales[0], who)?, num(&scales[0], who)?),
                        2 => (num(&scales[0], who)?, num(&scales[1], who)?),
                        _ => return Err(format!("{name}(grid, reference, scale or [scale of the rows, scale of the columns][, \"nearest\" | \"linear\" | \"cubic\"])")),
                    };
                    if sy <= 0 || sx <= 0 {
                        return Err(format!("{name}: a scale is above zero"));
                    }
                    let method = match a.get(3) {
                        Some(m) => word(m).ok_or_else(|| format!("{name}: the method is \"nearest\", \"linear\" or \"cubic\""))?,
                        None => "cubic".to_string(),
                    };
                    let sized = |count: usize, scale: &Float| -> usize {
                        let steps = if reference.postings { count - 1 } else { count };
                        let more = (Float::with_val(64, scale * steps as u64) - Float::with_val(64, 1e-9)).ceil().to_f64().max(1.0) as usize;
                        if reference.postings { more + 1 } else { more }
                    };
                    let (rows, cols) = (sized(reference.rows, &sy), sized(reference.cols, &sx));
                    if rows.saturating_mul(cols) > 4_000_000 {
                        return Err(format!("{name}: four million samples at most"));
                    }
                    let other = Ref { rows, cols, ..reference.clone() };
                    let mut out: Rows = Vec::with_capacity(rows);
                    for r in 1..=rows {
                        let mut row = Vec::with_capacity(cols);
                        for c in 1..=cols {
                            let (y, x) = other.to_world(&int(c), &int(r));
                            let (column, line) = reference.to_grid(&y, &x);
                            row.push(Self::between(&grid, &reference, &column, &line, &method, who)?);
                        }
                        out.push(row);
                    }
                    Expr::list(vec![matrix::from_rows(out), other.record()])
                }
                ("gradientm", 2 | 3) => {
                    let (grid, reference) = self.raster_of(&a[0], &a[1], who)?;
                    if !reference.geographic {
                        return Err("gradientm takes a grid of the earth (georefcells or georefpostings)".into());
                    }
                    let (rows, cols) = (reference.rows, reference.cols);
                    if rows < 2 || cols < 2 {
                        return Err("gradientm: two rows and two columns at least".into());
                    }
                    let prec = crate::mfun::bits();
                    let values: Vec<Vec<Float>> = grid.iter().map(|row| row.iter().map(|x| num(x, who)).collect()).collect::<Result<_, _>>()?;
                    // the places of the samples, and the lengths between them on the ellipsoid
                    let mut lats = Vec::with_capacity(rows);
                    let mut lons = Vec::with_capacity(cols);
                    for r in 1..=rows {
                        lats.push(reference.to_world(&Expr::one(), &int(r)).0);
                    }
                    for c in 1..=cols {
                        lons.push(reference.to_world(&int(c), &Expr::one()).1);
                    }
                    // (lengths on the ellipsoid: WGS84 in metres unless another is given)
                    let ellipsoid = match a.get(2) {
                        Some(e) => e.clone(),
                        None => crate::geodesy::call("wgs84ellipsoid", &[]).ok_or("gradientm: no ellipsoid")??,
                    };
                    let extra = [ellipsoid];
                    let mut north_of: Vec<Float> = Vec::with_capacity(rows);
                    for r in 0..rows {
                        // the way north from the first row to this one (signed)
                        let mut args = vec![lats[0].clone(), lats[r].clone()];
                        args.extend(extra.iter().cloned());
                        let way = crate::geodesy::call("meridianarc", &args).ok_or("gradientm: no length of the meridian")??;
                        north_of.push(num(&way, who)?);
                    }
                    let mut east_step: Vec<Float> = Vec::with_capacity(rows);
                    for lat in &lats {
                        let mut args = vec![lons[0].clone(), lons[1].clone(), lat.clone()];
                        args.extend(extra.iter().cloned());
                        let way = crate::geodesy::call("departure", &args).ok_or("gradientm: no length of the parallel")??;
                        let mut step = num(&way, who)?;
                        if num(&lons[1], who)? < num(&lons[0], who)? {
                            step = -step;
                        }
                        east_step.push(step);
                    }
                    let (mut aspects, mut slopes, mut norths, mut easts) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
                    let pi = Float::with_val(prec, rug::float::Constant::Pi);
                    for r in 0..rows {
                        let (mut row_a, mut row_s, mut row_n, mut row_e) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
                        let (up, down) = (r.saturating_sub(1), (r + 1).min(rows - 1));
                        for c in 0..cols {
                            let (left, right) = (c.saturating_sub(1), (c + 1).min(cols - 1));
                            let to_north = Float::with_val(prec, &values[down][c] - &values[up][c])
                                / Float::with_val(prec, &north_of[down] - &north_of[up]);
                            let to_east = Float::with_val(prec, &values[r][right] - &values[r][left])
                                / Float::with_val(prec, &east_step[r] * (right - left) as u64);
                            let steep = (Float::with_val(prec, &to_north * &to_north) + Float::with_val(prec, &to_east * &to_east)).sqrt();
                            row_s.push(crate::mfun::value(&(steep.clone().atan() * 180u32 / &pi)));
                            row_a.push(match steep.is_zero() {
                                true => nothing(),
                                false => {
                                    // the course of the way down, clockwise from north
                                    let mut course = Float::with_val(prec, -&to_east).atan2(&Float::with_val(prec, -&to_north)) * 180u32 / &pi;
                                    if course < 0 {
                                        course += 360u32;
                                    }
                                    crate::mfun::value(&course)
                                }
                            });
                            row_n.push(crate::mfun::value(&to_north));
                            row_e.push(crate::mfun::value(&to_east));
                        }
                        aspects.push(row_a);
                        slopes.push(row_s);
                        norths.push(row_n);
                        easts.push(row_e);
                    }
                    Expr::list(vec![matrix::from_rows(aspects), matrix::from_rows(slopes), matrix::from_rows(norths), matrix::from_rows(easts)])
                }
                ("areamat", 2 | 3) => {
                    let (grid, reference) = self.raster_of(&a[0], &a[1], who)?;
                    if !reference.geographic || reference.postings {
                        return Err("areamat takes a grid of cells of the earth (georefcells)".into());
                    }
                    let mut total = Vec::new();
                    let mut cells = Vec::new();
                    for (r, row) in grid.iter().enumerate() {
                        let (lat, _) = reference.to_world(&Expr::one(), &int(r + 1));
                        let half = tidy(div(reference.dy(), Expr::int(2)));
                        let mut args = vec![tidy(sub(lat.clone(), half.clone())), Expr::zero(), tidy(add(vec![lat, half])), reference.dx()];
                        args.extend(a.get(2).cloned());
                        let cell = crate::geodesy::call("areaquad", &args).ok_or("areamat: no area of a cell")??;
                        let mut count = 0i64;
                        for x in row {
                            if crate::arrays::truthy(x).map_err(|_| "areamat: the grid holds true and false (or 1 and 0)".to_string())? {
                                count += 1;
                            }
                        }
                        total.push(mul(vec![Expr::int(count), cell.clone()]));
                        cells.push(cell);
                    }
                    Expr::list(vec![tidy(add(total)), Expr::list(cells)])
                }
                ("los2", 6..=9) | ("viewshed", 4..=7) => {
                    let (grid, reference) = self.raster_of(&a[0], &a[1], who)?;
                    if !reference.geographic {
                        return Err(format!("{name} takes a grid of the earth"));
                    }
                    let heights = Self::heights_of(&grid, who)?;
                    let f = |e: &Expr| -> Result<f64, String> { Ok(num(e, who)?.to_f64()) };
                    let limits = [f(&reference.ylim[0])?, f(&reference.ylim[1])?, f(&reference.xlim[0])?, f(&reference.xlim[1])?];
                    let points = if name == "los2" { 6 } else { 4 };
                    let above = (
                        a.get(points).map(&f).transpose()?.unwrap_or(0.0),
                        a.get(points + 1).map(&f).transpose()?.unwrap_or(0.0),
                    );
                    let radius = a.get(points + 2).map(&f).transpose()?.unwrap_or(6_371_000.0);
                    if radius <= 0.0 {
                        return Err(format!("{name}: the radius of the earth is above zero"));
                    }
                    let from = (f(&a[2])?, f(&a[3])?);
                    match name {
                        "los2" => {
                            let to = (f(&a[4])?, f(&a[5])?);
                            let (seen, along, ways, grounds, lats, lons) = Self::sight(&heights, &reference, limits, from, to, above, radius);
                            let numbers = |v: &[f64]| Expr::list(v.iter().map(|x| crate::mfun::value(&Float::with_val(53, *x))).collect());
                            Expr::list(vec![
                                Expr::Bool(seen),
                                Expr::list(along.iter().map(|b| Expr::Bool(*b)).collect()),
                                numbers(&ways),
                                numbers(&grounds),
                                numbers(&lats),
                                numbers(&lons),
                            ])
                        }
                        _ => {
                            let mut out: Rows = Vec::with_capacity(reference.rows);
                            for r in 1..=reference.rows {
                                let mut row = Vec::with_capacity(reference.cols);
                                for c in 1..=reference.cols {
                                    let (lat, lon) = reference.to_world(&int(c), &int(r));
                                    let to = (f(&lat)?, f(&lon)?);
                                    let (seen, ..) = Self::sight(&heights, &reference, limits, from, to, above, radius);
                                    row.push(Expr::int(i64::from(seen)));
                                }
                                out.push(row);
                            }
                            matrix::from_rows(out)
                        }
                    }
                }
                ("arcgridread" | "readgeoraster", 1) => {
                    let path = crate::radix::text_of(&a[0]).ok_or_else(|| format!("{name}(\"file.asc\")"))?;
                    crate::sandbox::allowed(path).map_err(|e| format!("{name}: {e}"))?;
                    let all = std::fs::read_to_string(path).map_err(|e| format!("{name}: {path}: {e}"))?;
                    let mut head: Vec<(String, String)> = Vec::new();
                    let mut numbers: Vec<&str> = Vec::new();
                    for line in all.lines() {
                        let mut words = line.split_whitespace();
                        match words.next() {
                            Some(first) if first.chars().next().is_some_and(char::is_alphabetic) => {
                                head.push((first.to_ascii_lowercase(), words.next().unwrap_or_default().to_string()));
                            }
                            Some(first) => {
                                numbers.push(first);
                                numbers.extend(words);
                            }
                            None => {}
                        }
                    }
                    let said = |key: &str| head.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
                    let bad = || format!("{name}: {path} is no grid of ESRI in text: ncols, nrows, xllcorner (or xllcenter), yllcorner (or yllcenter), cellsize, then the rows from the north");
                    let whole = |key: &str| said(key).and_then(|v| v.parse::<usize>().ok()).filter(|k| *k >= 1);
                    let (Some(cols), Some(rows)) = (whole("ncols"), whole("nrows")) else {
                        return Err(bad());
                    };
                    let mut value = |text: &str| -> Res {
                        let parsed = crate::parser::parse(text).map_err(|_| bad())?;
                        let stmt = parsed.stmts.first().ok_or_else(bad)?;
                        self.eval(stmt)
                    };
                    let size = value(&said("cellsize").ok_or_else(bad)?)?;
                    let centres = said("xllcenter").is_some();
                    let (x0, y0) = match centres {
                        true => (value(&said("xllcenter").ok_or_else(bad)?)?, value(&said("yllcenter").ok_or_else(bad)?)?),
                        false => (value(&said("xllcorner").ok_or_else(bad)?)?, value(&said("yllcorner").ok_or_else(bad)?)?),
                    };
                    if numbers.len() != rows * cols {
                        return Err(format!("{name}: {path} says {rows} rows of {cols} values and holds {}", numbers.len()));
                    }
                    let empty = said("nodata_value");
                    let mut grid: Rows = Vec::with_capacity(rows);
                    for r in 0..rows {
                        let mut row = Vec::with_capacity(cols);
                        for c in 0..cols {
                            let item = numbers[r * cols + c];
                            row.push(match empty.as_deref() == Some(item) {
                                true => nothing(),
                                false => value(item)?,
                            });
                        }
                        grid.push(row);
                    }
                    let half = tidy(div(size.clone(), Expr::int(2)));
                    let (x0, y0) = match centres {
                        true => (tidy(sub(x0, half.clone())), tidy(sub(y0, half))),
                        false => (x0, y0),
                    };
                    let reference = Ref {
                        geographic: true,
                        postings: false,
                        ylim: [y0.clone(), tidy(add(vec![y0, mul(vec![int(rows), size.clone()])]))],
                        xlim: [x0.clone(), tidy(add(vec![x0, mul(vec![int(cols), size])]))],
                        rows,
                        cols,
                        north: true,
                        east: false,
                    };
                    Expr::list(vec![matrix::from_rows(grid), reference.record()])
                }
                ("worldfileread", 1..=3) => {
                    let path = crate::radix::text_of(&a[0]).ok_or("worldfileread(\"file.tfw\", \"geographic\" | \"planar\", [rows, columns])")?;
                    crate::sandbox::allowed(path).map_err(|e| format!("worldfileread: {e}"))?;
                    let all = std::fs::read_to_string(path).map_err(|e| format!("worldfileread: {path}: {e}"))?;
                    let mut six = Vec::new();
                    for item in all.split_whitespace() {
                        let parsed = crate::parser::parse(item).map_err(|e| format!("worldfileread: {path}: \"{item}\": {e}"))?;
                        let stmt = parsed.stmts.first().ok_or("worldfileread: an empty line")?;
                        six.push(self.eval(stmt)?);
                    }
                    let [a_, d, b, e_, c, f_] = &six[..] else {
                        return Err(format!("worldfileread: {path} holds six numbers: the size of a cell in x, two turns, the size in y (negative: rows from the north), x and y of the middle of the first cell"));
                    };
                    if !d.is_zero() || !b.is_zero() {
                        return Err("worldfileread: a grid that is turned has no reference of cells".into());
                    }
                    let geographic = a.get(1).and_then(word).is_none_or(|w| w == "geographic");
                    let sizes = a.get(2).map(items).unwrap_or_default();
                    let (Some(rows), Some(cols)) = (sizes.first().and_then(Expr::as_i64), sizes.get(1).and_then(Expr::as_i64)) else {
                        return Err("worldfileread(\"file.tfw\", \"geographic\" | \"planar\", [rows, columns]): the size of the grid".into());
                    };
                    let (rows, cols) = (rows as usize, cols as usize);
                    let north = num(e_, who)? < 0;
                    let east = num(a_, who)? < 0;
                    let wide = self.call("abs", std::slice::from_ref(a_))?;
                    let tall = self.call("abs", std::slice::from_ref(e_))?;
                    let edges = |first: &Expr, size: &Expr, count: usize, turned: bool| -> [Expr; 2] {
                        let half = div(size.clone(), Expr::int(2));
                        match turned {
                            true => [tidy(sub(add(vec![first.clone(), half.clone()]), mul(vec![int(count), size.clone()]))), tidy(add(vec![first.clone(), half]))],
                            false => [tidy(sub(first.clone(), half.clone())), tidy(add(vec![sub(first.clone(), half), mul(vec![int(count), size.clone()])]))],
                        }
                    };
                    Ref {
                        geographic,
                        postings: false,
                        ylim: edges(f_, &tall, rows, north),
                        xlim: edges(c, &wide, cols, east),
                        rows,
                        cols,
                        north,
                        east,
                    }
                    .record()
                }
                ("egm96geoid", 1 | 2) | ("geoidheight", 2 | 3) => {
                    if let Some(model) = a.get(2) {
                        match word(model).as_deref() {
                            Some("egm96") => {}
                            Some(other) => {
                                return Err(format!("geoidheight: the model that is built in is \"EGM96\" (to degree 360), not \"{other}\""));
                            }
                            None => return Err("geoidheight(lat, lon[, \"EGM96\"])".into()),
                        }
                    }
                    let prec = crate::mfun::bits();
                    let (lats, lons, shape, list, grid) = match n {
                        1 => {
                            // the samples of a grid
                            let reference = self.grid_ref(&a[0], None, who)?;
                            if !reference.geographic {
                                return Err("egm96geoid(reference): a reference of the earth".into());
                            }
                            let lats: Vec<Expr> = (1..=reference.rows).map(|r| reference.to_world(&Expr::one(), &int(r)).0).collect();
                            let lons: Vec<Expr> = (1..=reference.cols).map(|c| reference.to_world(&int(c), &Expr::one()).1).collect();
                            (lats, lons, Some((reference.rows, reference.cols)), true, true)
                        }
                        _ => {
                            let ((lats, shape, list), (lons, _, _)) = (shaped(&a[0]), shaped(&a[1]));
                            if lats.len() != lons.len() {
                                return Err(format!("{name}: as many latitudes as longitudes"));
                            }
                            (lats, lons, shape, list, false)
                        }
                    };
                    if lats.len().saturating_mul(if grid { lons.len() } else { 1 }) > 1_000_000 {
                        return Err(format!("{name}: a million places at most"));
                    }
                    let numbers = |v: &[Expr]| -> Result<Vec<Float>, String> { v.iter().map(|x| num(x, who)).collect() };
                    let (lat_numbers, lon_numbers) = (numbers(&lats)?, numbers(&lons)?);
                    if lat_numbers.iter().any(|l| *l < -90 || *l > 90) {
                        return Err(format!("{name}: a latitude is from -90 to 90 degrees"));
                    }
                    let found = crate::geoid::heights(&lat_numbers, &lon_numbers, grid, prec + 32);
                    let values: Vec<Expr> = found.iter().map(|h| crate::mfun::value(&Float::with_val(prec, h))).collect();
                    reshaped(values, shape, list)
                }
                _ => return Err(String::new()),
            })
        })())
        .filter(|r| !matches!(r, Err(e) if e.is_empty()))
    }

    /// The lines of the commands that write a grid to a file.
    pub(crate) fn georaster_written(&mut self, name: &str, a: &[Expr]) -> Result<Vec<String>, String> {
        let cell = |x: &Expr| format!("{x}");
        match (name, a) {
            ("arcgridwrite", [file, z, r]) => {
                let path = crate::radix::text_of(file).ok_or("arcgridwrite(\"file.asc\", grid, reference)")?;
                let (grid, reference) = self.raster_of(z, r, name)?;
                if reference.postings {
                    return Err("arcgridwrite takes a grid of cells".into());
                }
                let (dy, dx) = (reference.dy(), reference.dx());
                if num(&dy, name)? != num(&dx, name)? {
                    return Err("arcgridwrite: a grid of ESRI in text has square cells".into());
                }
                let mut out = format!(
                    "ncols {}\nnrows {}\nxllcorner {}\nyllcorner {}\ncellsize {}\nNODATA_value -9999\n",
                    reference.cols,
                    reference.rows,
                    cell(&numeric::n(&reference.xlim[0], numeric::digits())),
                    cell(&numeric::n(&reference.ylim[0], numeric::digits())),
                    cell(&numeric::n(&dx, numeric::digits()))
                );
                // the rows from the north, the columns from the west
                let mut rows: Vec<&Vec<Expr>> = grid.iter().collect();
                if !reference.north {
                    rows.reverse();
                }
                for row in rows {
                    let mut items: Vec<String> = row
                        .iter()
                        .map(|x| match x.is_undefined() {
                            true => "-9999".to_string(),
                            false => cell(&numeric::n(x, numeric::digits())),
                        })
                        .collect();
                    if reference.east {
                        items.reverse();
                    }
                    out.push_str(&items.join(" "));
                    out.push('\n');
                }
                crate::sandbox::allowed(path)
                    .and_then(|()| crate::sandbox::spend(out.len()))
                    .map_err(|e| format!("arcgridwrite: {e}"))?;
                std::fs::write(path, &out).map_err(|e| format!("arcgridwrite: {path}: {e}"))?;
                Ok(vec![format!("saved {path}: {} row(s) of {} value(s), {} bytes", reference.rows, reference.cols, out.len())])
            }
            ("worldfilewrite", [r, file]) => {
                let path = crate::radix::text_of(file).ok_or("worldfilewrite(reference, \"file.tfw\")")?;
                let reference = self.grid_ref(r, None, name)?;
                if reference.postings {
                    return Err("worldfilewrite takes a reference of cells".into());
                }
                let (dy, dx) = (reference.dy(), reference.dx());
                let (first_y, first_x) = reference.to_world(&Expr::one(), &Expr::one());
                let signed = |step: &Expr, turned: bool| match turned {
                    true => tidy(mul(vec![Expr::int(-1), step.clone()])),
                    false => step.clone(),
                };
                let lines = [signed(&dx, reference.east), Expr::zero(), Expr::zero(), signed(&dy, reference.north), first_x, first_y];
                let out: String = lines.iter().map(|x| format!("{}\n", numeric::n(x, numeric::digits()))).collect();
                crate::sandbox::allowed(path)
                    .and_then(|()| crate::sandbox::spend(out.len()))
                    .map_err(|e| format!("worldfilewrite: {e}"))?;
                std::fs::write(path, &out).map_err(|e| format!("worldfilewrite: {path}: {e}"))?;
                Ok(vec![format!("saved {path}: 6 numbers, {} bytes", out.len())])
            }
            ("arcgridwrite", _) => Err("arcgridwrite(\"file.asc\", grid, reference)".into()),
            _ => Err("worldfilewrite(reference, \"file.tfw\")".into()),
        }
    }

    /// A chart of a grid of the earth as the chart of a matrix that draws it: north at
    /// the top, west at the left.
    pub(crate) fn map_chart(&mut self, name: &str, args: &[Expr]) -> Result<Option<(&'static str, Vec<Expr>)>, String> {
        let target = match name {
            "geoshow" => "matrixplot",
            "contourm" => "contour",
            "surfm" => "surf",
            _ => return Ok(None),
        };
        let [z, r, rest @ ..] = args else {
            return Err(format!("{name}(grid, reference)"));
        };
        let (grid, reference) = self.raster_of(z, r, name)?;
        let mut rows: Rows = grid;
        if reference.east {
            for row in &mut rows {
                row.reverse();
            }
        }
        let mut out = match target {
            "surf" => {
                // the heights over the longitudes and latitudes
                if reference.north {
                    rows.reverse();
                }
                let upright = Ref { north: false, east: false, ..reference.clone() };
                let lons: Vec<Expr> = (1..=reference.cols).map(|c| upright.to_world(&int(c), &Expr::one()).1).collect();
                let lats: Vec<Expr> = (1..=reference.rows).map(|r| upright.to_world(&Expr::one(), &int(r)).0).collect();
                vec![Expr::list(lons), Expr::list(lats), matrix::from_rows(rows)]
            }
            _ => {
                // (the first row of a picture is its top)
                if !reference.north {
                    rows.reverse();
                }
                vec![matrix::from_rows(rows)]
            }
        };
        out.extend(rest.iter().cloned());
        Ok(Some((target, out)))
    }
}

pub(crate) fn is_map_chart(name: &str) -> bool {
    matches!(name, "geoshow" | "contourm" | "surfm")
}

#[cfg(test)]
mod tests {
    fn said(input: &str) -> String {
        crate::eval::run_line(input).map_or_else(|e| format!("error: {e}"), |lines| lines.join("|"))
    }

    #[test]
    fn places_and_cells() {
        // two rows of three cells of 10 degrees: the first row is the southern one
        let r = "georefcells([0, 20], [0, 30], [2, 3])";
        assert_eq!(said(&format!("geographictointrinsic({r}, 5, 15)")), "[2, 1]");
        assert_eq!(said(&format!("intrinsictogeographic({r}, 2, 1)")), "[5, 15]");
        assert_eq!(said(&format!("geographictodiscrete({r}, [5, 19, 50], [15, 29, 0])")), "[[1, 2, undefined], [2, 3, undefined]]");
        assert_eq!(
            said(&format!("geographictointrinsic(georefcells([0, 20], [0, 30], [2, 3], ColumnsStartFrom = \"north\"), 5, 15)")),
            "[2, 2]"
        );
        // postings lie on the limits
        assert_eq!(said("geographictointrinsic(georefpostings([0, 20], [0, 30], [3, 4]), 10, 30)"), "[4, 2]");
        assert_eq!(said("getfield(georefcells([0, 20], [0, 30], 10, 5), \"RasterSize\")"), "[2, 6]");
    }

    #[test]
    fn values_between_the_samples_and_parts_of_a_grid() {
        let z = "[[1, 2, 3], [4, 5, 6]]";
        let r = "georefcells([0, 20], [0, 30], [2, 3])";
        assert_eq!(said(&format!("geointerp({z}, {r}, 10, 15)")), "7/2|  ~ 3.5");
        assert_eq!(said(&format!("geointerp({z}, {r}, [5, 15], [5, 25], \"nearest\")")), "[1, 6]");
        assert_eq!(said(&format!("geointerp({z}, {r}, 30, 15)")), "undefined");
        assert_eq!(said(&format!("part(geocrop({z}, {r}, [0, 10], [10, 30]), 1)")), "[[2, 3]]");
        assert_eq!(
            said(&format!("getfield(part(geocrop({z}, {r}, [0, 10], [10, 30]), 2), \"LongitudeLimits\")")),
            "[10, 30]"
        );
        assert_eq!(said(&format!("part(georesize({z}, {r}, 2, \"nearest\"), 1)")), "[[1, 1, 2, 2, 3, 3], [1, 1, 2, 2, 3, 3], [4, 4, 5, 5, 6, 6], [4, 4, 5, 5, 6, 6]]");
    }

    #[test]
    fn what_is_seen_over_a_wall() {
        // flat ground with a wall in the middle column
        let z = "[[0, 0, 500, 0, 0], [0, 0, 500, 0, 0], [0, 0, 500, 0, 0]]";
        let r = "georefcells([0, 0.03], [0, 0.05], [3, 5])";
        assert_eq!(said(&format!("part(los2({z}, {r}, 0.015, 0.005, 0.015, 0.045, 10, 10), 1)")), "false");
        assert_eq!(said(&format!("part(los2({z}, {r}, 0.015, 0.005, 0.015, 0.015, 10, 10), 1)")), "true");
        assert_eq!(said(&format!("part(viewshed({z}, {r}, 0.015, 0.005, 10), 2)")), "[1, 1, 1, 0, 0]");
    }
}
