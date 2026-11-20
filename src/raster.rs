//! Pictures made of pixels: lines, dots, boxes, triangles and text, each as far as it is said
//! to be, so that the nearer thing shows.
//!
//! Only `+ - * /` and the square root act on the numbers, so that a picture is the same on
//! every system.

use crate::imgfile::Image;

/// Red, green and blue from 0 to 1.
pub type Rgb = [f32; 3];

/// A place of a picture: across, down, and how far it is.
pub type Place = [f64; 3];

/// What a drawing is drawn on.
pub trait Surface {
    /// A line that is `2 * half` wide, with a colour at each end.
    fn segment(&mut self, a: Place, b: Place, half: f64, ca: Rgb, cb: Rgb);
    fn disc(&mut self, at: Place, radius: f64, color: Rgb);
    /// A box with sides along the picture.
    fn rect(&mut self, from: [f64; 2], to: [f64; 2], depth: f64, color: Rgb);
    fn triangle(&mut self, corners: [Place; 3], colors: [Rgb; 3]);
    /// Letters that are `8 * size` pixels high, from their upper left corner.
    fn text(&mut self, at: [f64; 2], size: usize, text: &str, color: Rgb);
    /// One whole pixel behind everything else.
    fn pixel(&mut self, x: usize, y: usize, color: Rgb);
}

/// Depths that differ by less are the same depth: there the later thing shows.
const SAME: f32 = 1e-4;
/// The depth of what lies behind everything else.
const FAR: f32 = 3e38;

pub struct Raster {
    pub width: usize,
    pub height: usize,
    color: Vec<Rgb>,
    cover: Vec<f32>,
    depth: Vec<f32>,
    /// Only what lies between these depths is drawn.
    pub window: Option<(f32, f32)>,
    /// Only the pixels of this box are drawn: left, top, right, bottom.
    pub clip: [i64; 4],
}

/// How a picture is made of what was drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct Tone {
    pub background: Rgb,
    pub gray: bool,
    /// The picture says how much of each pixel is drawn, and has no background.
    pub alpha: bool,
    /// The depths that become 0 and 1 of a channel of depths.
    pub depth: Option<(f32, f32)>,
}

/// The gray of a colour.
pub fn luma(c: Rgb) -> f32 {
    0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
}

pub fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * t)
}

impl Raster {
    pub fn new(width: usize, height: usize) -> Raster {
        Raster {
            width,
            height,
            color: vec![[0.0; 3]; width * height],
            cover: vec![0.0; width * height],
            depth: vec![FAR; width * height],
            window: None,
            clip: [0, 0, width as i64, height as i64],
        }
    }

    /// The whole picture again.
    pub fn unclip(&mut self) {
        self.clip = [0, 0, self.width as i64, self.height as i64];
    }

    /// How much of a pixel is drawn.
    pub fn cover(&self, x: usize, y: usize) -> f32 {
        self.cover[y * self.width + x]
    }

    pub fn color(&self, x: usize, y: usize) -> Rgb {
        self.color[y * self.width + x]
    }

    /// The nearest and the farthest depth of what is drawn.
    pub fn depths(&self) -> Option<(f32, f32)> {
        let mut ends: Option<(f32, f32)> = None;
        for (z, m) in self.depth.iter().zip(&self.cover) {
            if *m > 0.0 && z.is_finite() && *z < FAR {
                ends = Some(match ends {
                    Some((lo, hi)) => (lo.min(*z), hi.max(*z)),
                    None => (*z, *z),
                });
            }
        }
        ends
    }

    /// A thing covers a part of a pixel. The nearer thing shows as much as it covers, the
    /// farther one where the nearer does not. Things that lie beside each other (`beside`:
    /// dots, boxes) cover a pixel together; the pieces of one line cover it only as much as
    /// the piece that covers most of it, so that they do not add up.
    fn put(&mut self, x: i64, y: i64, color: Rgb, cover: f32, depth: f32, beside: bool) {
        let [left, top, right, bottom] = self.clip;
        if cover <= 0.0 || x < left || y < top || x >= right || y >= bottom {
            return;
        }
        if let Some((near, far)) = self.window
            && !(depth >= near && depth < far)
        {
            return;
        }
        let i = y as usize * self.width + x as usize;
        let had = self.cover[i];
        if had <= 0.0 {
            (self.color[i], self.cover[i], self.depth[i]) = (color, cover, depth);
            return;
        }
        let most = if beside {
            (had + cover).min(1.0)
        } else {
            had.max(cover)
        };
        let front = depth <= self.depth[i] + SAME;
        let share = if front {
            cover / most
        } else {
            (most - had) / most
        };
        self.color[i] = mix(self.color[i], color, share);
        // the depth of a pixel is the depth of the thing that shows most in it
        let most_of_it = if front {
            cover >= had || cover >= 0.5
        } else {
            cover > had && had < 0.5
        };
        if most_of_it {
            self.depth[i] = depth;
        }
        self.cover[i] = most;
    }

    /// Makes a picture of many things that are smaller than a pixel as light as it can be:
    /// all covers grow alike, until only one pixel of 200 that are drawn is covered more
    /// than fully.
    pub fn brighten(&mut self) {
        const BINS: usize = 1000;
        let mut counts = [0usize; BINS + 1];
        let mut drawn = 0usize;
        for m in &self.cover {
            if *m > 0.0 {
                counts[((*m * BINS as f32) as usize).min(BINS)] += 1;
                drawn += 1;
            }
        }
        let mut above = 0usize;
        let mut level = BINS;
        while level > 0 && above + counts[level] <= drawn / 200 {
            above += counts[level];
            level -= 1;
        }
        // (the upper edge of the bin of that pixel)
        let most = (level + 1) as f32 / BINS as f32;
        if most < 1.0 {
            for m in self.cover.iter_mut() {
                *m = (*m / most).min(1.0);
            }
        }
    }

    /// The picture of what was drawn.
    pub fn image(&self, tone: &Tone) -> Image {
        let channels = if tone.gray { 1 } else { 3 } + usize::from(tone.alpha);
        let mut data = Vec::with_capacity(self.width * self.height * channels);
        for (c, m) in self.color.iter().zip(&self.cover) {
            let m = m.clamp(0.0, 1.0);
            let shown = if tone.alpha {
                if m > 0.0 { *c } else { tone.background }
            } else {
                mix(tone.background, *c, m)
            };
            if tone.gray {
                data.push(luma(shown).clamp(0.0, 1.0));
            } else {
                data.extend(shown.map(|v| v.clamp(0.0, 1.0)));
            }
            if tone.alpha {
                data.push(m);
            }
        }
        let depth = tone.depth.map(|(near, far)| {
            let span = if far > near { far - near } else { 1.0 };
            self.depth
                .iter()
                .zip(&self.cover)
                .map(|(z, m)| {
                    if *m > 0.0 {
                        ((z - near) / span).clamp(0.0, 1.0)
                    } else {
                        1.0
                    }
                })
                .collect()
        });
        Image {
            width: self.width,
            height: self.height,
            colors: if tone.gray { 1 } else { 3 },
            alpha: tone.alpha,
            data,
            depth,
        }
    }

    /// The rows of pixels whose middles lie from `low` to `high`.
    fn rows(&self, low: f64, high: f64) -> (i64, i64) {
        let first = (low - 0.5).ceil().max(self.clip[1] as f64);
        let last = (high - 0.5).floor().min((self.clip[3] - 1) as f64);
        (first as i64, last as i64)
    }

    fn columns(&self, low: f64, high: f64) -> (i64, i64) {
        let first = (low - 0.5).ceil().max(self.clip[0] as f64);
        let last = (high - 0.5).floor().min((self.clip[2] - 1) as f64);
        (first as i64, last as i64)
    }
}

/// How much of a pixel a band of half the width `half` covers, whose middle line is `d` away
/// from the middle of the pixel.
fn band(d: f64, half: f64) -> f64 {
    let upper = if d + 0.5 < half { d + 0.5 } else { half };
    let lower = if d - 0.5 > -half { d - 0.5 } else { -half };
    (upper - lower).clamp(0.0, 1.0)
}

fn finite(p: &Place) -> bool {
    p.iter().all(|v| v.is_finite() && v.abs() < 1e12)
}

impl Surface for Raster {
    fn segment(&mut self, a: Place, b: Place, half: f64, ca: Rgb, cb: Rgb) {
        if !finite(&a) || !finite(&b) || half <= 0.0 {
            return;
        }
        let reach = half + 0.5;
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let length2 = dx * dx + dy * dy;
        let (low, high) = if a[1] < b[1] {
            (a[1], b[1])
        } else {
            (b[1], a[1])
        };
        let (first, last) = self.rows(low - reach, high + reach);
        for j in first..=last {
            let cy = j as f64 + 0.5;
            // the part of the line that is near this row
            let (from, to) = if dy.abs() < 1e-9 {
                (a[0], b[0])
            } else {
                let t0 = ((cy - reach - a[1]) / dy).clamp(0.0, 1.0);
                let t1 = ((cy + reach - a[1]) / dy).clamp(0.0, 1.0);
                (a[0] + dx * t0, a[0] + dx * t1)
            };
            let (from, to) = if from < to { (from, to) } else { (to, from) };
            let (left, right) = self.columns(from - reach, to + reach);
            for i in left..=right {
                let (px, py) = (i as f64 + 0.5 - a[0], cy - a[1]);
                let t = if length2 > 0.0 {
                    ((px * dx + py * dy) / length2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let (ex, ey) = (px - t * dx, py - t * dy);
                let cover = band((ex * ex + ey * ey).sqrt(), half);
                if cover > 0.0 {
                    let depth = a[2] + (b[2] - a[2]) * t;
                    self.put(
                        i,
                        j,
                        mix(ca, cb, t as f32),
                        cover as f32,
                        depth as f32,
                        false,
                    );
                }
            }
        }
    }

    fn disc(&mut self, at: Place, radius: f64, color: Rgb) {
        if !finite(&at) || radius <= 0.0 {
            return;
        }
        // a dot that is smaller than a pixel is as bright as it is large
        let small = if radius < 0.5 {
            4.0 * radius * radius
        } else {
            1.0
        };
        let reach = radius + 0.5;
        let (first, last) = self.rows(at[1] - reach, at[1] + reach);
        let (left, right) = self.columns(at[0] - reach, at[0] + reach);
        for j in first..=last {
            for i in left..=right {
                let (ex, ey) = (i as f64 + 0.5 - at[0], j as f64 + 0.5 - at[1]);
                let cover = (reach - (ex * ex + ey * ey).sqrt()).clamp(0.0, 1.0) * small;
                self.put(i, j, color, cover as f32, at[2] as f32, true);
            }
        }
    }

    fn rect(&mut self, from: [f64; 2], to: [f64; 2], depth: f64, color: Rgb) {
        let all = [from[0], from[1], to[0], to[1], depth];
        if !all.iter().all(|v| v.is_finite() && v.abs() < 1e12) {
            return;
        }
        let (x0, x1) = if from[0] < to[0] {
            (from[0], to[0])
        } else {
            (to[0], from[0])
        };
        let (y0, y1) = if from[1] < to[1] {
            (from[1], to[1])
        } else {
            (to[1], from[1])
        };
        let (first, last) = self.rows(y0 - 0.5, y1 + 0.5);
        let (left, right) = self.columns(x0 - 0.5, x1 + 0.5);
        // the part of the pixel from `at` to `at + 1` that lies from `low` to `high`
        let share = |at: f64, low: f64, high: f64| {
            let upper = if at + 1.0 < high { at + 1.0 } else { high };
            let lower = if at > low { at } else { low };
            (upper - lower).clamp(0.0, 1.0)
        };
        for j in first..=last {
            let down = share(j as f64, y0, y1);
            for i in left..=right {
                let cover = down * share(i as f64, x0, x1);
                self.put(i, j, color, cover as f32, depth as f32, true);
            }
        }
    }

    fn triangle(&mut self, p: [Place; 3], colors: [Rgb; 3]) {
        if !p.iter().all(finite) {
            return;
        }
        let area =
            (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]);
        if area == 0.0 {
            return;
        }
        let least = |k: usize| p[0][k].min(p[1][k]).min(p[2][k]);
        let most = |k: usize| p[0][k].max(p[1][k]).max(p[2][k]);
        let (first, last) = self.rows(least(1), most(1));
        let (left, right) = self.columns(least(0), most(0));
        // the side of the line from p[a] to p[b] on which a point lies
        let side = |a: usize, b: usize, x: f64, y: f64| {
            (p[b][0] - p[a][0]) * (y - p[a][1]) - (p[b][1] - p[a][1]) * (x - p[a][0])
        };
        for j in first..=last {
            for i in left..=right {
                let (x, y) = (i as f64 + 0.5, j as f64 + 0.5);
                let w = [
                    side(1, 2, x, y) / area,
                    side(2, 0, x, y) / area,
                    side(0, 1, x, y) / area,
                ];
                if w.iter().any(|v| *v < 0.0) {
                    continue;
                }
                let depth = w[0] * p[0][2] + w[1] * p[1][2] + w[2] * p[2][2];
                let color = [0, 1, 2].map(|k| {
                    (w[0] * f64::from(colors[0][k])
                        + w[1] * f64::from(colors[1][k])
                        + w[2] * f64::from(colors[2][k])) as f32
                });
                self.put(i, j, color, 1.0, depth as f32, true);
            }
        }
    }

    fn text(&mut self, at: [f64; 2], size: usize, text: &str, color: Rgb) {
        if !at.iter().all(|v| v.is_finite() && v.abs() < 1e9) {
            return;
        }
        let window = self.window.take();
        let (x0, y0) = (at[0].round() as i64, at[1].round() as i64);
        let size = size.max(1) as i64;
        for (k, c) in text.chars().enumerate() {
            let glyph = font8x8::legacy::BASIC_LEGACY[if c.is_ascii() { c as usize } else { 63 }];
            for (row, bits) in glyph.iter().enumerate() {
                for column in 0..8 {
                    if bits >> column & 1 == 0 {
                        continue;
                    }
                    let (x, y) = (x0 + (8 * k as i64 + column) * size, y0 + row as i64 * size);
                    for j in 0..size {
                        for i in 0..size {
                            self.put(x + i, y + j, color, 1.0, -FAR, false);
                        }
                    }
                }
            }
        }
        self.window = window;
    }

    fn pixel(&mut self, x: usize, y: usize, color: Rgb) {
        if x < self.width && y < self.height {
            let window = self.window.take();
            self.put(x as i64, y as i64, color, 1.0, FAR, true);
            self.window = window;
        }
    }
}

/// The colours of a row of colours at a place from 0 to 1 along it.
pub fn along(stops: &[Rgb], t: f64) -> Rgb {
    let Some(last) = stops.len().checked_sub(1) else {
        return [1.0; 3];
    };
    if last == 0 || t.is_nan() || t <= 0.0 {
        return stops[0];
    }
    if t >= 1.0 {
        return stops[last];
    }
    let at = t * last as f64;
    let k = (at as usize).min(last - 1);
    mix(stops[k], stops[k + 1], (at - k as f64) as f32)
}

/// A colour of the colour circle: `turn` from 0 to 1 goes from red over yellow, green, cyan,
/// blue and magenta back to red; `light` from 0 (black) over 0.5 (the full colour) to 1 (white).
pub fn hue(turn: f64, light: f64) -> Rgb {
    let turn = turn - turn.floor();
    let at = turn * 6.0;
    let part = (at as usize).min(5);
    let f = (at - part as f64) as f32;
    let full: Rgb = match part {
        0 => [1.0, f, 0.0],
        1 => [1.0 - f, 1.0, 0.0],
        2 => [0.0, 1.0, f],
        3 => [0.0, 1.0 - f, 1.0],
        4 => [f, 0.0, 1.0],
        _ => [1.0, 0.0, 1.0 - f],
    };
    let light = light.clamp(0.0, 1.0);
    if light <= 0.5 {
        mix([0.0; 3], full, (2.0 * light) as f32)
    } else {
        mix(full, [1.0; 3], (2.0 * light - 1.0) as f32)
    }
}

pub(crate) const fn rgb(code: u32) -> Rgb {
    [
        (code >> 16 & 255) as f32 / 255.0,
        (code >> 8 & 255) as f32 / 255.0,
        (code & 255) as f32 / 255.0,
    ]
}

/// The rows of colours that have names.
pub const PALETTES: &[(&str, &[Rgb])] = &[
    (
        "rainbow",
        &[
            rgb(0xff0000),
            rgb(0xff8000),
            rgb(0xffff00),
            rgb(0x00e000),
            rgb(0x00d0ff),
            rgb(0x2040ff),
            rgb(0xb000ff),
        ],
    ),
    (
        "fire",
        &[
            rgb(0x200000),
            rgb(0xc00000),
            rgb(0xff6000),
            rgb(0xffd000),
            rgb(0xffffc0),
        ],
    ),
    (
        "ice",
        &[
            rgb(0x001040),
            rgb(0x0050c0),
            rgb(0x30b0f0),
            rgb(0xb0f0ff),
            rgb(0xffffff),
        ],
    ),
    (
        "viridis",
        &[
            rgb(0x440154),
            rgb(0x472d7b),
            rgb(0x3b528b),
            rgb(0x2c728e),
            rgb(0x21918c),
            rgb(0x28ae80),
            rgb(0x5ec962),
            rgb(0xaddc30),
            rgb(0xfde725),
        ],
    ),
    (
        "plasma",
        &[
            rgb(0x0d0887),
            rgb(0x5c01a6),
            rgb(0x9c179e),
            rgb(0xcc4778),
            rgb(0xed7953),
            rgb(0xfdb42f),
            rgb(0xf0f921),
        ],
    ),
    (
        "heat",
        &[
            rgb(0x000080),
            rgb(0x0000ff),
            rgb(0x00ffff),
            rgb(0x00ff00),
            rgb(0xffff00),
            rgb(0xff0000),
            rgb(0x800000),
        ],
    ),
    (
        "ocean",
        &[
            rgb(0x001020),
            rgb(0x004060),
            rgb(0x008080),
            rgb(0x60c0a0),
            rgb(0xf0fff0),
        ],
    ),
    (
        "sunset",
        &[
            rgb(0x2d0a4e),
            rgb(0x8b1a6b),
            rgb(0xd9435e),
            rgb(0xf78c4b),
            rgb(0xfde68a),
        ],
    ),
    (
        "parula",
        &[
            rgb(0x352a87),
            rgb(0x0363e1),
            rgb(0x1485d4),
            rgb(0x06a7c6),
            rgb(0x38b99e),
            rgb(0x92bf73),
            rgb(0xd9ba56),
            rgb(0xfcce2e),
            rgb(0xf9fb0e),
        ],
    ),
    (
        "turbo",
        &[
            rgb(0x30123b),
            rgb(0x4662d7),
            rgb(0x36aaf9),
            rgb(0x1ae4b6),
            rgb(0x72fe5e),
            rgb(0xc7ef34),
            rgb(0xfaba39),
            rgb(0xf66b19),
            rgb(0xca2a04),
            rgb(0x7a0403),
        ],
    ),
    (
        "jet",
        &[
            rgb(0x000080),
            rgb(0x0000ff),
            rgb(0x0080ff),
            rgb(0x00ffff),
            rgb(0x80ff80),
            rgb(0xffff00),
            rgb(0xff8000),
            rgb(0xff0000),
            rgb(0x800000),
        ],
    ),
    (
        "hot",
        &[
            rgb(0x000000),
            rgb(0x550000),
            rgb(0xaa0000),
            rgb(0xff0000),
            rgb(0xff5500),
            rgb(0xffaa00),
            rgb(0xffff00),
            rgb(0xffff80),
            rgb(0xffffff),
        ],
    ),
    (
        "hsv",
        &[
            rgb(0xff0000),
            rgb(0xffff00),
            rgb(0x00ff00),
            rgb(0x00ffff),
            rgb(0x0000ff),
            rgb(0xff00ff),
            rgb(0xff0000),
        ],
    ),
    (
        "bone",
        &[
            rgb(0x000000),
            rgb(0x1c1c27),
            rgb(0x38384d),
            rgb(0x545474),
            rgb(0x707a8f),
            rgb(0x8ba1ab),
            rgb(0xa7c7c7),
            rgb(0xd3e3e3),
            rgb(0xffffff),
        ],
    ),
    (
        "copper",
        &[
            rgb(0x000000),
            rgb(0x402819),
            rgb(0x805033),
            rgb(0xbf784c),
            rgb(0xff9f65),
            rgb(0xffc77f),
        ],
    ),
    (
        "pink",
        &[
            rgb(0x000000),
            rgb(0x704a4a),
            rgb(0x9f6868),
            rgb(0xc38080),
            rgb(0xd0aa93),
            rgb(0xddcca5),
            rgb(0xe9e9b4),
            rgb(0xf4f4dd),
            rgb(0xffffff),
        ],
    ),
    ("spring", &[rgb(0xff00ff), rgb(0xffff00)]),
    ("summer", &[rgb(0x008066), rgb(0xffff66)]),
    ("autumn", &[rgb(0xff0000), rgb(0xffff00)]),
    ("winter", &[rgb(0x0000ff), rgb(0x00ff80)]),
    ("gray", &[rgb(0x404040), rgb(0xffffff)]),
    ("grey", &[rgb(0x404040), rgb(0xffffff)]),
    ("ink", &[rgb(0x000000), rgb(0x808080)]),
    ("cool", &[rgb(0x00ffff), rgb(0xff00ff)]),
    ("warm", &[rgb(0xffe000), rgb(0xff0000)]),
];

/// The colours that have names.
pub const COLORS: &[(&str, Rgb)] = &[
    ("black", rgb(0x000000)),
    ("white", rgb(0xffffff)),
    ("red", rgb(0xe02020)),
    ("green", rgb(0x10a030)),
    ("blue", rgb(0x2060e0)),
    ("yellow", rgb(0xf0d000)),
    ("cyan", rgb(0x00c0d0)),
    ("magenta", rgb(0xd020c0)),
    ("orange", rgb(0xf08000)),
    ("purple", rgb(0x8030c0)),
    ("brown", rgb(0x8b5a2b)),
    ("pink", rgb(0xff80b0)),
    ("gray", rgb(0x808080)),
    ("grey", rgb(0x808080)),
    ("navy", rgb(0x001f5f)),
    ("lime", rgb(0x80ff00)),
    ("gold", rgb(0xffc000)),
    ("silver", rgb(0xc0c0c0)),
];

/// The colours of the curves of one picture, one after the other.
pub const CYCLE: [Rgb; 8] = [
    rgb(0x1f77b4),
    rgb(0xd62728),
    rgb(0x2ca02c),
    rgb(0xff7f0e),
    rgb(0x9467bd),
    rgb(0x17becf),
    rgb(0xe377c2),
    rgb(0x8c564b),
];

/// A colour written with six digits of base 16, as `ff8000` or `#ff8000`.
pub fn coded(text: &str) -> Option<Rgb> {
    let digits = text.strip_prefix('#').unwrap_or(text);
    if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(digits, 16).ok().map(rgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: Rgb = [1.0; 3];
    const RED: Rgb = [1.0, 0.0, 0.0];
    const BLUE: Rgb = [0.0, 0.0, 1.0];

    fn total(r: &Raster) -> f64 {
        r.cover.iter().map(|c| f64::from(*c)).sum()
    }

    #[test]
    fn a_line_covers_its_width_times_its_length() {
        // through the middles of a row: that row and nothing else
        let mut r = Raster::new(20, 9);
        r.segment([2.0, 4.5, 0.0], [18.0, 4.5, 0.0], 0.5, WHITE, WHITE);
        for x in 3..17 {
            assert_eq!(r.cover(x, 4), 1.0);
            assert_eq!(r.cover(x, 3), 0.0);
            assert_eq!(r.cover(x, 5), 0.0);
        }
        // between two rows: half of each
        let mut r = Raster::new(20, 9);
        r.segment([2.0, 4.0, 0.0], [18.0, 4.0, 0.0], 0.5, WHITE, WHITE);
        for x in 3..17 {
            assert_eq!((r.cover(x, 3), r.cover(x, 4)), (0.5, 0.5));
        }
        // any direction, any width: the area of the line, with round ends
        for (half, to) in [
            (0.5, [50.0, 31.0]),
            (0.5, [13.0, 57.0]),
            (1.5, [55.0, 9.0]),
            (0.25, [40.0, 44.0]),
            (3.0, [9.0, 50.0]),
        ] {
            let mut r = Raster::new(64, 64);
            let from = [7.3, 6.1];
            r.segment(
                [from[0], from[1], 0.0],
                [to[0], to[1], 0.0],
                half,
                WHITE,
                WHITE,
            );
            let length = ((to[0] - from[0]) * (to[0] - from[0])
                + (to[1] - from[1]) * (to[1] - from[1]))
                .sqrt();
            let area = 2.0 * half * length + std::f64::consts::PI * half * half;
            let seen = total(&r);
            assert!(
                (seen - area).abs() < 0.08 * area + 1.0,
                "{half} {to:?}: {seen} against {area}"
            );
        }
    }

    #[test]
    fn the_pieces_of_a_line_do_not_add_up() {
        let (from, to) = ([3.2, 5.7], [58.4, 41.3]);
        let mut whole = Raster::new(64, 48);
        whole.segment(
            [from[0], from[1], 0.0],
            [to[0], to[1], 0.0],
            0.5,
            WHITE,
            WHITE,
        );
        let mut pieces = Raster::new(64, 48);
        let count = 400;
        for k in 0..count {
            let at = |k: usize| {
                let t = k as f64 / count as f64;
                [
                    from[0] + (to[0] - from[0]) * t,
                    from[1] + (to[1] - from[1]) * t,
                    0.0,
                ]
            };
            pieces.segment(at(k), at(k + 1), 0.5, WHITE, WHITE);
        }
        for (a, b) in whole.cover.iter().zip(&pieces.cover) {
            assert!((a - b).abs() < 0.02, "{a} against {b}");
        }
    }

    #[test]
    fn the_nearer_thing_shows() {
        let across = |r: &mut Raster, depth: f64, color: Rgb| {
            r.segment([1.0, 5.5, depth], [10.0, 5.5, depth], 0.5, color, color)
        };
        let down = |r: &mut Raster, depth: f64, color: Rgb| {
            r.segment([5.5, 1.0, depth], [5.5, 10.0, depth], 0.5, color, color)
        };
        // whichever is drawn first
        let mut r = Raster::new(11, 11);
        across(&mut r, 1.0, RED);
        down(&mut r, 2.0, BLUE);
        assert_eq!(r.color(5, 5), RED);
        let mut r = Raster::new(11, 11);
        down(&mut r, 2.0, BLUE);
        across(&mut r, 1.0, RED);
        assert_eq!(r.color(5, 5), RED);
        assert_eq!(r.color(5, 2), BLUE);
        // at the same depth the later one
        let mut r = Raster::new(11, 11);
        across(&mut r, 1.0, RED);
        down(&mut r, 1.0, BLUE);
        assert_eq!(r.color(5, 5), BLUE);
        // beside a nearer line that covers a pixel half, the farther one shows in the rest
        let mut r = Raster::new(11, 11);
        r.segment([1.0, 5.0, 1.0], [10.0, 5.0, 1.0], 0.5, RED, RED);
        assert_eq!(r.cover(3, 4), 0.5);
        r.rect([0.0, 0.0], [11.0, 11.0], 2.0, BLUE);
        assert_eq!(r.cover(3, 4), 1.0);
        assert_eq!(r.color(3, 4), [0.5, 0.0, 0.5]);
        assert_eq!(r.color(3, 8), BLUE);
        // and a nearer line that comes later takes its half
        let mut r = Raster::new(11, 11);
        r.rect([0.0, 0.0], [11.0, 11.0], 2.0, BLUE);
        r.segment([1.0, 5.0, 1.0], [10.0, 5.0, 1.0], 0.5, RED, RED);
        assert_eq!(r.color(3, 4), [0.5, 0.0, 0.5]);
        assert_eq!(r.depths(), Some((1.0, 2.0)));
    }

    #[test]
    fn boxes_dots_and_triangles_cover_their_area() {
        let mut r = Raster::new(16, 16);
        r.rect([2.25, 3.5], [9.75, 11.0], 0.0, WHITE);
        assert_eq!(r.cover(2, 3), 0.75 * 0.5);
        assert_eq!(r.cover(5, 3), 0.5);
        assert_eq!(r.cover(9, 10), 0.75);
        assert_eq!(r.cover(5, 5), 1.0);
        assert_eq!(r.cover(10, 5), 0.0);
        assert!((total(&r) - 7.5 * 7.5).abs() < 1e-4);
        for radius in [0.2, 0.5, 2.0, 6.5] {
            let mut r = Raster::new(32, 32);
            r.disc([15.3, 16.2, 0.0], radius, WHITE);
            let area = std::f64::consts::PI * radius * radius;
            assert!(
                (total(&r) - area).abs() < 0.12 * area + 0.3,
                "{radius}: {}",
                total(&r)
            );
        }
        // boxes that lie beside each other cover a pixel together, as do dots
        let mut r = Raster::new(4, 1);
        r.rect([0.0, 0.0], [1.5, 1.0], 0.0, RED);
        r.rect([1.5, 0.0], [3.25, 1.0], 0.0, BLUE);
        assert_eq!(
            (r.cover(1, 0), r.cover(2, 0), r.cover(3, 0)),
            (1.0, 1.0, 0.25)
        );
        assert_eq!(r.color(1, 0), [0.5, 0.0, 0.5]);
        let mut r = Raster::new(3, 3);
        r.disc([1.5, 1.5, 0.0], 0.1, WHITE);
        let one = r.cover(1, 1);
        assert!(one > 0.01 && one < 0.05, "{one}");
        for _ in 0..9 {
            r.disc([1.5, 1.5, 0.0], 0.1, WHITE);
        }
        assert!((r.cover(1, 1) - 10.0 * one).abs() < 1e-5);
        for _ in 0..90 {
            r.disc([1.5, 1.5, 0.0], 0.1, WHITE);
        }
        assert_eq!(r.cover(1, 1), 1.0);
        // two triangles that share a side leave no gap and no pixel twice
        let mut r = Raster::new(32, 32);
        let corners = [
            [2.2, 3.1, 0.0],
            [29.4, 5.7, 0.0],
            [27.9, 28.8, 0.0],
            [4.6, 25.3, 0.0],
        ];
        r.triangle([corners[0], corners[1], corners[2]], [WHITE; 3]);
        r.triangle([corners[0], corners[2], corners[3]], [WHITE; 3]);
        let area = 0.5
            * ((corners[2][0] - corners[0][0]) * (corners[3][1] - corners[1][1])
                - (corners[3][0] - corners[1][0]) * (corners[2][1] - corners[0][1]))
                .abs();
        assert!(
            (total(&r) - area).abs() < 0.03 * area,
            "{} against {area}",
            total(&r)
        );
        assert!(r.cover.iter().all(|c| *c == 0.0 || *c == 1.0));
        for y in 8..22 {
            for x in 8..24 {
                assert_eq!(r.cover(x, y), 1.0);
            }
        }
        // the colours and the depths of the corners in between
        let mut r = Raster::new(8, 8);
        r.triangle(
            [[0.0, 0.0, 0.0], [8.0, 0.0, 8.0], [0.0, 8.0, 0.0]],
            [RED, BLUE, RED],
        );
        let c = r.color(3, 1);
        assert!((c[2] - 3.5 / 8.0).abs() < 1e-6 && (c[0] - 4.5 / 8.0).abs() < 1e-6);
        assert!((r.depth[8 + 3] - 3.5).abs() < 1e-6);
    }

    #[test]
    fn faint_pictures_grow_light() {
        let mut r = Raster::new(100, 10);
        for k in 0..1000usize {
            // covers from 0.0001 to 0.1
            r.rect(
                [(k % 100) as f64, (k / 100) as f64],
                [
                    (k % 100) as f64 + 1.0,
                    (k / 100) as f64 + 0.0001 * (k + 1) as f64,
                ],
                0.0,
                WHITE,
            );
        }
        let before: Vec<f32> = r.cover.clone();
        r.brighten();
        let gain = r.cover[500] / before[500];
        assert!((9.5..10.6).contains(&gain), "{gain}");
        for (a, b) in before.iter().zip(&r.cover) {
            assert!((b - (a * gain).min(1.0)).abs() < 1e-5);
        }
        // what is light already stays as it is
        let mut r = Raster::new(4, 4);
        r.rect([0.0, 0.0], [2.0, 2.0], 0.0, WHITE);
        r.rect([2.0, 2.0], [4.0, 2.5], 0.0, WHITE);
        let before = r.cover.clone();
        r.brighten();
        assert_eq!(before, r.cover);
    }

    #[test]
    fn only_the_clip_and_the_window_are_drawn() {
        let mut r = Raster::new(10, 10);
        r.clip = [2, 3, 6, 7];
        r.rect([0.0, 0.0], [10.0, 10.0], 0.0, WHITE);
        assert_eq!(total(&r), 16.0);
        assert_eq!(
            (r.cover(2, 3), r.cover(1, 3), r.cover(5, 6), r.cover(6, 6)),
            (1.0, 0.0, 1.0, 0.0)
        );
        r.unclip();
        r.window = Some((1.0, 2.0));
        r.rect([0.0, 0.0], [10.0, 1.0], 0.5, WHITE);
        r.rect([0.0, 1.0], [10.0, 2.0], 1.0, WHITE);
        r.rect([0.0, 2.0], [10.0, 3.0], 2.0, WHITE);
        assert_eq!(
            (r.cover(0, 0), r.cover(0, 1), r.cover(0, 2)),
            (0.0, 1.0, 0.0)
        );
        // letters and whole pixels do not belong to a depth
        r.text([0.0, 0.0], 1, "I", RED);
        assert_eq!(r.color(2, 0), RED);
        r.pixel(9, 9, BLUE);
        assert_eq!(r.color(9, 9), BLUE);
    }

    #[test]
    fn letters() {
        let mut r = Raster::new(40, 20);
        r.text([1.0, 2.0], 2, "Il", WHITE);
        let glyph = font8x8::legacy::BASIC_LEGACY['I' as usize];
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..8usize {
                let on = bits >> column & 1 == 1;
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let got = r.cover(1 + 2 * column + dx, 2 + 2 * row + dy);
                    assert_eq!(got == 1.0, on, "row {row} column {column}");
                }
            }
        }
        assert!(total(&r) > f64::from(4 * glyph.iter().map(|b| b.count_ones()).sum::<u32>()));
        // a letter that the font lacks is a question mark
        let (mut a, mut b) = (Raster::new(8, 8), Raster::new(8, 8));
        a.text([0.0, 0.0], 1, "π", WHITE);
        b.text([0.0, 0.0], 1, "?", WHITE);
        assert_eq!(a.cover, b.cover);
    }

    #[test]
    fn the_picture_of_what_was_drawn() {
        let mut r = Raster::new(3, 1);
        r.rect([0.0, 0.0], [1.0, 1.0], 4.0, RED);
        r.rect([1.0, 0.0], [1.5, 1.0], 8.0, BLUE);
        let tone = Tone {
            background: [1.0, 1.0, 1.0],
            gray: false,
            alpha: false,
            depth: None,
        };
        let plain = r.image(&tone);
        assert_eq!(
            (plain.colors, plain.alpha, plain.depth.is_none()),
            (3, false, true)
        );
        assert_eq!(plain.data, [1.0, 0.0, 0.0, 0.5, 0.5, 1.0, 1.0, 1.0, 1.0]);
        let clear = r.image(&Tone {
            alpha: true,
            ..tone.clone()
        });
        assert_eq!(
            clear.data,
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.5, 1.0, 1.0, 1.0, 0.0]
        );
        let gray = r.image(&Tone {
            gray: true,
            depth: Some((4.0, 8.0)),
            ..tone
        });
        assert_eq!(gray.colors, 1);
        assert!((gray.data[0] - 0.299).abs() < 1e-6 && (gray.data[2] - 1.0).abs() < 1e-6);
        assert_eq!(gray.depth, Some(vec![0.0, 1.0, 1.0]));
    }

    #[test]
    fn colours() {
        let stops = [RED, WHITE, BLUE];
        assert_eq!(along(&stops, 0.0), RED);
        assert_eq!(along(&stops, 0.5), WHITE);
        assert_eq!(along(&stops, 1.0), BLUE);
        assert_eq!(along(&stops, 0.25), [1.0, 0.5, 0.5]);
        assert_eq!(along(&stops, -3.0), RED);
        assert_eq!(along(&stops, 7.0), BLUE);
        assert_eq!(along(&stops, f64::NAN), RED);
        assert_eq!(hue(0.0, 0.5), [1.0, 0.0, 0.0]);
        assert_eq!(hue(1.0 / 3.0, 0.5), [0.0, 1.0, 0.0]);
        assert_eq!(hue(0.5, 0.5), [0.0, 1.0, 1.0]);
        assert_eq!(hue(1.5, 0.5), [0.0, 1.0, 1.0]);
        assert_eq!(hue(0.25, 0.0), [0.0; 3]);
        assert_eq!(hue(0.25, 1.0), [1.0; 3]);
        assert_eq!(hue(0.0, 0.75), [1.0, 0.5, 0.5]);
        assert_eq!(coded("ff8000"), Some([1.0, 128.0 / 255.0, 0.0]));
        assert_eq!(coded("#000000"), Some([0.0; 3]));
        assert_eq!(coded("ff800"), None);
        assert_eq!(coded("gg0000"), None);
        for (name, stops) in PALETTES {
            assert!(stops.len() >= 2, "{name}");
        }
    }
}
