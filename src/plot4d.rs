//! A function of a complex number as a shape of four dimensions. The real and the imaginary
//! part of the argument and of the value lie on the width, the height, the frames and the
//! colours of a picture, each on one of them: the frames one upon the other are a body in
//! space, and the alpha channel says which of its places belong to the shape.

use crate::eval::Session;
use crate::expr::Expr;
use crate::imgfile::{self, Format, Image};
use crate::numeric::{self, Env};
use crate::picture::{self, Choices};
use crate::plot::label;
use crate::raster::{self, Rgb};
use rug::Complex;
use std::rc::Rc;

/// What a number of the function lies on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Width,
    Height,
    Frame,
    Color,
}

const AXES: [Axis; 4] = [Axis::Width, Axis::Height, Axis::Frame, Axis::Color];

/// The four numbers of the function, in the order of `arg_real`, `arg_imag`, `val_real`, `val_imag`.
pub(crate) const PARTS: [&str; 4] = [
    "the real part of z",
    "the imaginary part of z",
    "the real part of f(z)",
    "the imaginary part of f(z)",
];
const CHOICES: [&str; 4] = ["arg_real", "arg_imag", "val_real", "val_imag"];

fn word_of(value: &Expr) -> Option<String> {
    match value {
        Expr::Sym(word) => Some(word.to_ascii_lowercase()),
        other => crate::radix::text_of(other).map(str::to_ascii_lowercase),
    }
}

impl Axis {
    /// The side of a picture that a word names.
    pub fn named(value: &Expr) -> Option<Axis> {
        Some(match word_of(value)?.as_str() {
            "width" => Axis::Width,
            "height" => Axis::Height,
            "frame" | "frames" => Axis::Frame,
            "color" | "colour" | "colors" | "colours" => Axis::Color,
            _ => return None,
        })
    }

    fn index(self) -> usize {
        self as usize
    }

    fn word(self) -> &'static str {
        match self {
            Axis::Width => "width",
            Axis::Height => "height",
            Axis::Frame => "frame",
            Axis::Color => "color",
        }
    }
}

/// What belongs to the shape besides its surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Fill {
    #[default]
    Surface,
    /// The body from the surface down to the first frame.
    Below,
    /// The body from the surface up to the last frame.
    Above,
}

impl Fill {
    pub fn named(value: &Expr) -> Option<Fill> {
        Some(match word_of(value)?.as_str() {
            "surface" | "none" => Fill::Surface,
            "below" | "under" => Fill::Below,
            "above" | "over" => Fill::Above,
            _ => return None,
        })
    }
}

/// The sides of the four numbers: what was chosen, and for the others the sides that are
/// left, in the order width, height, frame, color.
pub fn layout(chosen: &[Option<Axis>; 4]) -> Result<[Axis; 4], String> {
    for (k, axis) in chosen.iter().enumerate() {
        if let Some(axis) = axis
            && let Some(other) = chosen[..k].iter().position(|a| a == &Some(*axis))
        {
            return Err(format!(
                "{} and {} both lie on the {}: each of width, height, frame and color takes one of arg_real, arg_imag, val_real, val_imag",
                CHOICES[other],
                CHOICES[k],
                axis.word()
            ));
        }
    }
    let mut free = AXES.iter().filter(|a| !chosen.contains(&Some(**a)));
    Ok(chosen.map(|axis| axis.unwrap_or_else(|| *free.next().expect("as many sides as numbers"))))
}

/// What is drawn.
pub struct Plan {
    pub f: Expr,
    pub var: Rc<str>,
    /// The box of the arguments: the ends of the real and of the imaginary part.
    pub re: (f64, f64),
    pub im: (f64, f64),
    /// The sides of the four numbers.
    pub axes: [Axis; 4],
    /// Pixels across, pixels down, frames and colours.
    pub sizes: [usize; 4],
    pub range_real: Option<(f64, f64)>,
    pub range_imag: Option<(f64, f64)>,
    pub fill: Fill,
    /// Frames that a place of the surface is seen in.
    pub thickness: usize,
    /// Degrees that the body is turned: yaw, pitch, roll.
    pub rotate: Option<[f64; 3]>,
    /// Every frame is stretched on its own to the whole picture and the whole palette.
    pub normalize: bool,
}

/// A body that is turned about the middle of its box, and made as much smaller as keeps
/// the box inside the picture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Turn {
    /// From a place of the body to its place in the picture, and back; both in a box from
    /// -1 to 1 along the width, the height (upwards) and the frames.
    to: [[f64; 3]; 3],
    back: [[f64; 3]; 3],
    /// How much smaller the body is.
    pub scale: f64,
}

impl Turn {
    /// Degrees about the frames (roll) first, then about the width (pitch), then about
    /// the upright axis (yaw).
    pub fn new(angles: [f64; 3]) -> Turn {
        let [(sy, cy), (sp, cp), (sr, cr)] = angles.map(|degrees| {
            // (quarter turns are exact)
            let quarters = degrees / 90.0;
            match quarters.fract() == 0.0 {
                true => [(0.0, 1.0), (1.0, 0.0), (0.0, -1.0), (-1.0, 0.0)]
                    [quarters.rem_euclid(4.0) as usize],
                false => degrees.to_radians().sin_cos(),
            }
        });
        let yaw = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
        let pitch = [[1.0, 0.0, 0.0], [0.0, cp, -sp], [0.0, sp, cp]];
        let roll = [[cr, -sr, 0.0], [sr, cr, 0.0], [0.0, 0.0, 1.0]];
        let times = |a: [[f64; 3]; 3], b: [[f64; 3]; 3]| {
            [0, 1, 2].map(|i| [0, 1, 2].map(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum::<f64>()))
        };
        let turned = times(yaw, times(pitch, roll));
        let reach = turned
            .iter()
            .map(|row| row.iter().map(|v| v.abs()).sum::<f64>())
            .fold(1.0, f64::max);
        let scale = 1.0 / reach;
        Turn {
            to: turned.map(|row| row.map(|v| v * scale)),
            back: [0, 1, 2].map(|i| [0, 1, 2].map(|j| turned[j][i] / scale)),
            scale,
        }
    }

    fn apply(m: &[[f64; 3]; 3], at: [f64; 3], sizes: &[usize; 4]) -> [f64; 3] {
        let inner = [0, 1, 2].map(|k| 2.0 * at[k] / sizes[k] as f64 - 1.0);
        [0, 1, 2].map(|i| {
            let moved: f64 = (0..3).map(|k| m[i][k] * inner[k]).sum();
            (moved + 1.0) / 2.0 * sizes[i] as f64
        })
    }
}

/// The voxels of a surface by the columns of the frames: for every pixel its frames, the
/// first frame first, each with its colour.
struct Columns {
    starts: Vec<u32>,
    voxels: Vec<(u16, u16, Facing)>,
}

impl Columns {
    fn new(slices: &[Vec<Voxel>], pixels: usize) -> Columns {
        let mut starts = vec![0u32; pixels + 1];
        for (pixel, ..) in slices.iter().flatten() {
            starts[*pixel as usize + 1] += 1;
        }
        for pixel in 0..pixels {
            starts[pixel + 1] += starts[pixel];
        }
        let mut next = starts.clone();
        let mut voxels = vec![(0, 0, [0; 3]); starts[pixels] as usize];
        for (frame, slice) in slices.iter().enumerate() {
            for (pixel, _, level, faces) in slice {
                let at = &mut next[*pixel as usize];
                voxels[*at as usize] = (frame as u16, *level, *faces);
                *at += 1;
            }
        }
        Columns { starts, voxels }
    }

    /// The colour of the body at a frame of a pixel: that of the nearest voxel of the
    /// surface that is not below the frame (the body below the surface), or not above it.
    fn at(&self, pixel: usize, frame: usize, fill: Fill) -> u16 {
        self.seen(pixel, frame, fill, 0)
            .map_or(EMPTY, |(level, _)| level)
    }

    /// The same with the direction that the surface faces, for a voxel that is no more
    /// than `near` frames from the surface.
    fn seen(&self, pixel: usize, frame: usize, fill: Fill, near: usize) -> Option<(u16, Facing)> {
        let column = &self.voxels[self.starts[pixel] as usize..self.starts[pixel + 1] as usize];
        let found = match fill {
            Fill::Below => column.iter().find(|(f, ..)| *f as usize >= frame),
            _ => column.iter().rev().find(|(f, ..)| *f as usize <= frame),
        };
        found.map(|(f, level, faces)| match (*f as usize).abs_diff(frame) <= near {
            true => (*level, *faces),
            false => (*level, [0; 3]),
        })
    }
}

pub(crate) const EMPTY: u16 = u16::MAX;
/// A voxel of a frame: its pixel (rows from the top), how far from its middle the surface
/// was met there, its colour, and the direction that the surface faces there.
type Voxel = (u32, u16, u16, Facing);
/// A direction across the width, upwards and along the frames, each from -127 to 127; all
/// zeros where none is known.
pub type Facing = [i8; 3];

/// The direction of a vector as small whole numbers.
fn facing(v: [f64; 3]) -> Facing {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    match length > 0.0 && length.is_finite() {
        true => v.map(|part| (part / length * 127.0).round() as i8),
        false => [0; 3],
    }
}
/// The most places where a function is worked out, and the most voxels of a shape.
const SAMPLES: usize = 1 << 24;
const VOXELS: usize = 1 << 25;

/// The shape: the voxels of a box of `width` by `height` pixels and `frames` frames.
pub struct Shape {
    pub sizes: [usize; 4],
    pub axes: [Axis; 4],
    /// The ends of the four numbers.
    pub ranges: [(f64, f64); 4],
    /// The values of a part go far beyond its ends, which were found without them.
    pub narrowed: [bool; 4],
    /// The voxels of the surface in every frame, the pixels in their order, each one once.
    /// A surface that is turned is turned here; a body is filled before it is turned.
    slices: Vec<Vec<Voxel>>,
    /// For a body: the voxels of its surface by columns.
    columns: Option<Columns>,
    pub turn: Option<Turn>,
    pub normalize: bool,
    /// Of a surface whose frames are stretched: the frames that became larger, and the
    /// frames that hold something.
    pub stretched: Option<(usize, usize)>,
    pub(crate) fill: Fill,
    thickness: usize,
    /// The voxels of the surface.
    pub voxels: usize,
}

fn value(f: &Expr, var: &Rc<str>, env: &mut Env, x: f64, y: f64) -> [f64; 2] {
    env.insert(var.clone(), Complex::with_val(64, (x, y)));
    match numeric::evalf_env(f, 64, env) {
        Ok(v) => {
            let (re, im) = (v.real().to_f64(), v.imag().to_f64());
            if re.is_finite() && im.is_finite() {
                [re, im]
            } else {
                [f64::NAN; 2]
            }
        }
        Err(_) => [f64::NAN; 2],
    }
}

/// The ends of a part of the values: those that are given, or the least and the greatest;
/// when a few values lie far out (near a pole) the ends are those of all but a hundredth at
/// each side.
fn span(
    values: impl Iterator<Item = f64>,
    given: Option<(f64, f64)>,
) -> Option<((f64, f64), bool)> {
    if let Some(ends) = given {
        return Some((ends, false));
    }
    let mut all: Vec<f64> = values.filter(|v| v.is_finite()).collect();
    if all.is_empty() {
        return None;
    }
    all.sort_by(f64::total_cmp);
    let (low, high) = (all[0], all[all.len() - 1]);
    let at = |share: f64| all[((all.len() - 1) as f64 * share).round() as usize];
    let (near, far) = (at(0.01), at(0.99));
    let (ends, narrowed) = if far > near && high - low > 3.0 * (far - near) {
        ((near, far), true)
    } else {
        ((low, high), false)
    };
    // (a part that stays the same gets room around it)
    Some(match ends.1 > ends.0 {
        true => (ends, narrowed),
        false => ((ends.0 - 0.5, ends.1 + 0.5), false),
    })
}

/// Every voxel once, with the colour of the surface where it is nearest to the middle.
fn sorted(slices: &mut [Vec<Voxel>]) -> usize {
    slices
        .iter_mut()
        .map(|slice| {
            slice.sort_unstable();
            slice.dedup_by_key(|voxel| voxel.0);
            slice.len()
        })
        .sum()
}

/// A frame that is stretched on its own: what of the width, of the height and of the colours
/// it shows, and how much larger.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Stretch {
    low: [f64; 3],
    scale: [f64; 3],
}

impl Stretch {
    /// From the ends of what lies in a frame across, upwards and in the colours.
    fn new(ends: &[(f64, f64); 3], sizes: &[usize; 4]) -> Stretch {
        let mut stretch = Stretch {
            low: [0.0; 3],
            scale: [1.0; 3],
        };
        for (k, size) in [sizes[0], sizes[1], sizes[3]].into_iter().enumerate() {
            let (size, (low, high)) = (size as f64, ends[k]);
            if low > high {
                continue;
            }
            match k {
                0 | 1 => {
                    let (low, high) = (low.max(0.0), high.min(size));
                    // (what is thinner than a voxel stays in the middle)
                    let room = (1.0 - (high - low)).max(0.0) / 2.0;
                    stretch.low[k] = low - room;
                    stretch.scale[k] = size / (high - low).max(1.0);
                }
                // one colour alone is the middle of the palette
                _ if high - low < 1e-9 => stretch.low[k] = low - size / 2.0,
                _ => {
                    stretch.low[k] = low;
                    stretch.scale[k] = size / (high - low);
                }
            }
        }
        stretch
    }

    /// A place of the body in the stretched frame.
    fn apply(&self, at: [f64; 4]) -> [f64; 4] {
        [
            (at[0] - self.low[0]) * self.scale[0],
            (at[1] - self.low[1]) * self.scale[1],
            at[2],
            (at[3] - self.low[2]) * self.scale[2],
        ]
    }
}

/// The pieces of a cell of the surface, each as small as a voxel: `plot` gets the frame of
/// a piece, its middle and its corners. With `stretches` the pieces are as small as a voxel
/// of their stretched frame. A piece of the size of a voxel gives its middle a colour;
/// pieces of half that size leave no voxel out.
fn pieces(
    corners: [[f64; 4]; 4],
    sizes: &[usize; 4],
    stretches: Option<&[Stretch]>,
    stack: &mut Vec<([[f64; 4]; 4], u8)>,
    plot: &mut impl FnMut(usize, [f64; 4], &[[f64; 4]; 4]),
) {
    let half = |a: [f64; 4], b: [f64; 4]| [0, 1, 2, 3].map(|k| (a[k] + b[k]) / 2.0);
    stack.push((corners, 0));
    while let Some((c, depth)) = stack.pop() {
        // nothing is drawn of what lies outside the box or beyond the colours
        let outside = (0..4).any(|axis| {
            let at = [c[0][axis], c[1][axis], c[2][axis], c[3][axis]];
            at.iter().all(|v| *v < 0.0) || at.iter().all(|v| *v > sizes[axis] as f64)
        });
        if outside {
            continue;
        }
        let middle = half(half(c[0], c[3]), half(c[1], c[2]));
        let frame = (middle[2].max(0.0) as usize).min(sizes[2] - 1);
        let larger = match stretches {
            Some(stretches) => [stretches[frame].scale[0], stretches[frame].scale[1], 1.0],
            None => [1.0; 3],
        };
        let along = |a: usize, b: usize| {
            (0..3)
                .map(|axis| (c[a][axis] - c[b][axis]).abs() * larger[axis])
                .fold(0.0, f64::max)
        };
        let (across, down) = (along(1, 0).max(along(3, 2)), along(2, 0).max(along(3, 1)));
        if ((across <= 1.0 && down <= 1.0) || depth >= 60)
            && (0..4).all(|axis| (0.0..=sizes[axis] as f64).contains(&middle[axis]))
        {
            plot(frame, middle, &c);
        }
        if (across <= 0.5 && down <= 0.5) || depth >= 60 {
            continue;
        }
        if across >= down {
            let (m0, m1) = (half(c[0], c[1]), half(c[2], c[3]));
            stack.push(([c[0], m0, c[2], m1], depth + 1));
            stack.push(([m0, c[1], m1, c[3]], depth + 1));
        } else {
            let (m0, m1) = (half(c[0], c[2]), half(c[1], c[3]));
            stack.push(([c[0], c[1], m0, m1], depth + 1));
            stack.push(([m0, m1, c[2], c[3]], depth + 1));
        }
    }
}

/// The shape of a function.
pub fn shape(plan: &Plan) -> Result<Shape, String> {
    let sizes = plan.sizes;
    let side = |part: usize| plan.axes[part].index();
    let (na, nb) = (sizes[side(0)], sizes[side(1)]);
    if (na + 1).saturating_mul(nb + 1) > SAMPLES {
        return Err(format!(
            "plot4d works the function out at {} by {} places: more than the {SAMPLES} that it takes",
            na + 1,
            nb + 1
        ));
    }
    let mut env = Env::new();
    let x_of = |i: f64| plan.re.0 + (plan.re.1 - plan.re.0) * i / na as f64;
    let y_of = |j: f64| plan.im.0 + (plan.im.1 - plan.im.0) * j / nb as f64;
    let mut values = Vec::with_capacity((na + 1) * (nb + 1));
    for j in 0..=nb {
        for i in 0..=na {
            values.push(value(
                &plan.f,
                &plan.var,
                &mut env,
                x_of(i as f64),
                y_of(j as f64),
            ));
        }
    }
    let none = || "plot4d: the function has no value in this box".to_string();
    let (real, narrow_real) =
        span(values.iter().map(|v| v[0]), plan.range_real).ok_or_else(none)?;
    let (imag, narrow_imag) =
        span(values.iter().map(|v| v[1]), plan.range_imag).ok_or_else(none)?;
    let ranges = [plan.re, plan.im, real, imag];
    let turn = plan
        .rotate
        .filter(|angles| *angles != [0.0; 3])
        .map(Turn::new);
    // where the four numbers of a place lie: across, down (from below), in the frames, in the colours
    let place = |q: [f64; 4]| -> [f64; 4] {
        let mut at = [0.0; 4];
        for part in 0..4 {
            let (low, high) = ranges[part];
            at[side(part)] = (q[part] - low) / (high - low) * sizes[side(part)] as f64;
        }
        // (a surface is turned as it is drawn; a body is filled first)
        if let (Some(turn), Fill::Surface) = (&turn, plan.fill) {
            let [x, y, t] = Turn::apply(&turn.to, [at[0], at[1], at[2]], &sizes);
            at = [x, y, t, at[3]];
        }
        at
    };
    let corner = |i: usize, j: usize| -> Option<[f64; 4]> {
        let v = values[j * (na + 1) + i];
        (!v[0].is_nan()).then(|| place([x_of(i as f64), y_of(j as f64), v[0], v[1]]))
    };
    let half = |a: [f64; 4], b: [f64; 4]| [0, 1, 2, 3].map(|k| (a[k] + b[k]) / 2.0);
    // the cells that are drawn: those with a value at every corner and without a jump of the
    // values inside (a pole, a cut of a branch is no surface): where the corners lie far
    // apart the middle tells
    let mut drawn = vec![false; na * nb];
    for j in 0..nb {
        for i in 0..na {
            let (Some(p00), Some(p10), Some(p01), Some(p11)) = (
                corner(i, j),
                corner(i + 1, j),
                corner(i, j + 1),
                corner(i + 1, j + 1),
            ) else {
                continue;
            };
            let corners = [p00, p10, p01, p11];
            let wide = [2, 3]
                .into_iter()
                .filter(|part| side(*part) < 3)
                .map(|part| {
                    let at: Vec<f64> = corners.iter().map(|c| c[side(part)]).collect();
                    let (low, high) = (
                        at.iter().cloned().fold(f64::MAX, f64::min),
                        at.iter().cloned().fold(f64::MIN, f64::max),
                    );
                    (part, high - low)
                })
                .filter(|(_, extent)| *extent > 8.0)
                .collect::<Vec<_>>();
            if !wide.is_empty() {
                let (x, y) = (x_of(i as f64 + 0.5), y_of(j as f64 + 0.5));
                let v = value(&plan.f, &plan.var, &mut env, x, y);
                if v[0].is_nan() {
                    continue;
                }
                let (middle, mean) = (
                    place([x, y, v[0], v[1]]),
                    half(half(p00, p11), half(p10, p01)),
                );
                if wide.iter().any(|(part, extent)| {
                    (middle[side(*part)] - mean[side(*part)]).abs() > 0.25 * extent
                }) {
                    continue;
                }
            }
            drawn[j * na + i] = true;
        }
    }
    let cells = || {
        (0..nb)
            .flat_map(|j| (0..na).map(move |i| (i, j)))
            .filter(|(i, j)| drawn[j * na + i])
            .map(|(i, j)| {
                [(i, j), (i + 1, j), (i, j + 1), (i + 1, j + 1)]
                    .map(|(i, j)| corner(i, j).expect("a value"))
            })
    };
    let mut stack = Vec::new();
    // every frame stretched on its own: where the surface lies in each frame, then the
    // surface again in pieces that are small in the stretched frames
    let stretches = (plan.normalize && plan.fill == Fill::Surface).then(|| {
        let mut ends = vec![[(f64::MAX, f64::MIN); 3]; sizes[2]];
        for corners in cells() {
            pieces(
                corners,
                &sizes,
                None,
                &mut stack,
                &mut |frame, middle, piece| {
                    // across and upwards as far as the pieces reach, the colours of their middles
                    for (end, axis) in ends[frame].iter_mut().zip([0, 1]) {
                        for corner in piece {
                            *end = (end.0.min(corner[axis]), end.1.max(corner[axis]));
                        }
                    }
                    ends[frame][2] = (
                        ends[frame][2].0.min(middle[3]),
                        ends[frame][2].1.max(middle[3]),
                    );
                },
            );
        }
        ends.iter()
            .map(|ends| Stretch::new(ends, &sizes))
            .collect::<Vec<_>>()
    });
    let mut slices: Vec<Vec<Voxel>> = vec![Vec::new(); sizes[2]];
    let (mut pushed, mut kept) = (0usize, 0usize);
    for corners in cells() {
        pieces(
            corners,
            &sizes,
            stretches.as_deref(),
            &mut stack,
            &mut |frame, middle, piece| {
                let seen = match &stretches {
                    Some(stretches) => stretches[frame].apply(middle),
                    None => middle,
                };
                // the direction that the piece faces: across both of its sides
                let (u, v) = (
                    [0, 1, 2].map(|axis| piece[1][axis] - piece[0][axis]),
                    [0, 1, 2].map(|axis| piece[2][axis] - piece[0][axis]),
                );
                let larger = match &stretches {
                    Some(stretches) => stretches[frame].scale,
                    None => [1.0; 3],
                };
                let faces = facing([
                    (u[1] * v[2] - u[2] * v[1]) * larger[1],
                    (u[2] * v[0] - u[0] * v[2]) * larger[0],
                    (u[0] * v[1] - u[1] * v[0]) * larger[0] * larger[1],
                ]);
                let cell =
                    [0, 1, 2, 3].map(|axis| (seen[axis].max(0.0) as usize).min(sizes[axis] - 1));
                let far: f64 = (0..3)
                    .map(|axis| (seen[axis] - cell[axis] as f64 - 0.5).powi(2))
                    .sum();
                let row = sizes[1] - 1 - cell[1];
                slices[cell[2]].push((
                    (row * sizes[0] + cell[0]) as u32,
                    (far / 0.75 * 65535.0).min(65535.0) as u16,
                    cell[3] as u16,
                    faces,
                ));
                pushed += 1;
            },
        );
        if pushed - kept > VOXELS / 2 {
            kept = sorted(&mut slices);
            pushed = kept;
            if kept > VOXELS {
                return Err(format!(
                    "the shape has more than {VOXELS} voxels: take a smaller width, height or frames, or narrower ranges of the values (range_real, range_imag)"
                ));
            }
        }
    }
    let voxels = sorted(&mut slices);
    if voxels == 0 {
        return Err("plot4d: nothing of the function lies in the box of the picture".to_string());
    }
    let columns = (plan.fill != Fill::Surface).then(|| Columns::new(&slices, sizes[0] * sizes[1]));
    let stretched = stretches.as_ref().map(|stretches| {
        let held = slices.iter().filter(|slice| !slice.is_empty()).count();
        let larger = stretches
            .iter()
            .zip(&slices)
            .filter(|(stretch, slice)| {
                !slice.is_empty() && (stretch.scale[0] > 1.02 || stretch.scale[1] > 1.02)
            })
            .count();
        (larger, held)
    });
    Ok(Shape {
        sizes,
        axes: plan.axes,
        ranges,
        narrowed: [false, false, narrow_real, narrow_imag],
        slices,
        columns,
        turn,
        normalize: plan.normalize,
        stretched,
        fill: plan.fill,
        thickness: plan.thickness.max(1),
        voxels,
    })
}

/// The frames of a shape.
pub struct Frames<'a> {
    shape: &'a Shape,
    /// The colour of every pixel of a frame; `EMPTY` where the shape is not.
    pub levels: Vec<u16>,
    spare: Vec<u16>,
    /// The directions that the voxels of the frame face, when they are asked for.
    pub facings: Option<Vec<Facing>>,
}

impl<'a> Frames<'a> {
    pub fn new(shape: &'a Shape) -> Frames<'a> {
        Frames {
            shape,
            levels: vec![EMPTY; shape.sizes[0] * shape.sizes[1]],
            spare: Vec::new(),
            facings: None,
        }
    }

    /// The frames with the directions that their voxels face.
    pub fn facing(shape: &'a Shape) -> Frames<'a> {
        Frames {
            facings: Some(vec![[0; 3]; shape.sizes[0] * shape.sizes[1]]),
            ..Frames::new(shape)
        }
    }

    /// The cut of a body through a frame made as large as the picture, its colours spread
    /// over the whole palette.
    fn stretch(&mut self) {
        let [width, height, _, colours] = self.shape.sizes;
        let (mut across, mut down, mut tones) = ((width, 0), (height, 0), (EMPTY, 0u16));
        for (pixel, level) in self
            .levels
            .iter()
            .enumerate()
            .filter(|(_, level)| **level != EMPTY)
        {
            let (column, row) = (pixel % width, pixel / width);
            across = (across.0.min(column), across.1.max(column));
            down = (down.0.min(row), down.1.max(row));
            tones = (tones.0.min(*level), tones.1.max(*level));
        }
        if across.0 > across.1 {
            return;
        }
        let (wide, high) = (across.1 + 1 - across.0, down.1 + 1 - down.0);
        let spread = (tones.1 - tones.0) as f64;
        let tone = |level: f64| match spread > 0.0 {
            true => ((level - tones.0 as f64) / spread * (colours - 1) as f64)
                .round()
                .clamp(0.0, (colours - 1) as f64),
            false => (colours / 2) as f64,
        } as u16;
        // a place between four pixels of the cut belongs to the body when most of what is
        // around it does, and has the colour that lies between theirs
        let levels = &self.levels;
        let at = |column: f64, row: f64| -> u16 {
            let (x, y) = (
                column.clamp(across.0 as f64, across.1 as f64),
                row.clamp(down.0 as f64, down.1 as f64),
            );
            let (x0, y0) = (
                (x as usize).min(across.1.max(1) - 1).max(across.0),
                (y as usize).min(down.1.max(1) - 1).max(down.0),
            );
            let (x1, y1) = ((x0 + 1).min(across.1), (y0 + 1).min(down.1));
            let (fx, fy) = (
                (x - x0 as f64).clamp(0.0, 1.0),
                (y - y0 as f64).clamp(0.0, 1.0),
            );
            let (mut held, mut sum) = (0.0, 0.0);
            for (px, py, weight) in [
                (x0, y0, (1.0 - fx) * (1.0 - fy)),
                (x1, y0, fx * (1.0 - fy)),
                (x0, y1, (1.0 - fx) * fy),
                (x1, y1, fx * fy),
            ] {
                let level = levels[py * width + px];
                if level != EMPTY {
                    held += weight;
                    sum += weight * level as f64;
                }
            }
            match held >= 0.5 {
                true => tone(sum / held),
                false => EMPTY,
            }
        };
        self.spare.clear();
        self.spare.extend((0..width * height).map(|pixel| {
            let (column, row) = (pixel % width, pixel / width);
            at(
                across.0 as f64 + (column as f64 + 0.5) * wide as f64 / width as f64 - 0.5,
                down.0 as f64 + (row as f64 + 0.5) * high as f64 / height as f64 - 0.5,
            )
        }));
        // (the directions are those of the nearest places of the cut)
        if let Some(facings) = &mut self.facings {
            let nearest = |at: f64, ends: (usize, usize)| {
                at.round().clamp(ends.0 as f64, ends.1 as f64) as usize
            };
            let moved: Vec<Facing> = (0..width * height)
                .map(|pixel| {
                    let (column, row) = (pixel % width, pixel / width);
                    let x = across.0 as f64 + (column as f64 + 0.5) * wide as f64 / width as f64;
                    let y = down.0 as f64 + (row as f64 + 0.5) * high as f64 / height as f64;
                    facings[nearest(y - 0.5, down) * width + nearest(x - 0.5, across)]
                })
                .collect();
            *facings = moved;
        }
        std::mem::swap(&mut self.levels, &mut self.spare);
    }

    /// Works out the frame `k`.
    pub fn take(&mut self, k: usize) {
        let shape = self.shape;
        let [width, height, frames, _] = shape.sizes;
        match (&shape.columns, &shape.turn) {
            (None, _) => {
                self.levels.fill(EMPTY);
                for slice in &shape.slices[(k + 1).saturating_sub(shape.thickness)..=k] {
                    for (pixel, _, level, faces) in slice {
                        self.levels[*pixel as usize] = *level;
                        if let Some(facings) = &mut self.facings {
                            facings[*pixel as usize] = *faces;
                        }
                    }
                }
            }
            (Some(columns), None) => match &mut self.facings {
                None => {
                    for (pixel, level) in self.levels.iter_mut().enumerate() {
                        *level = columns.at(pixel, k, shape.fill);
                    }
                }
                Some(facings) => {
                    for (pixel, level) in self.levels.iter_mut().enumerate() {
                        (*level, facings[pixel]) = columns
                            .seen(pixel, k, shape.fill, 0)
                            .unwrap_or((EMPTY, [0; 3]));
                    }
                }
            },
            // every place of the picture shows what the body has where the place comes from
            (Some(columns), Some(turn)) => {
                for (pixel, level) in self.levels.iter_mut().enumerate() {
                    let here = [
                        (pixel % width) as f64 + 0.5,
                        (height - 1 - pixel / width) as f64 + 0.5,
                        k as f64 + 0.5,
                    ];
                    let from = Turn::apply(&turn.back, here, &shape.sizes);
                    let inside = from[0] >= 0.0
                        && from[1] >= 0.0
                        && from[2] >= 0.0
                        && from[0] < width as f64
                        && from[1] < height as f64
                        && from[2] < frames as f64;
                    let found = inside
                        .then(|| {
                            let source = (height - 1 - from[1] as usize) * width + from[0] as usize;
                            columns.seen(source, from[2] as usize, shape.fill, 2)
                        })
                        .flatten();
                    *level = found.map_or(EMPTY, |(level, _)| level);
                    if let (Some(facings), Some((_, faces))) = (&mut self.facings, found) {
                        facings[pixel] = facing([0, 1, 2].map(|i| {
                            (0..3)
                                .map(|j| turn.to[i][j] * f64::from(faces[j]))
                                .sum::<f64>()
                        }));
                    }
                }
            }
        }
        if shape.normalize && shape.columns.is_some() {
            self.stretch();
        }
    }

    /// The frame `k` as a picture with alpha: 1 where the shape is, 0 elsewhere.
    pub fn image(&mut self, k: usize, tints: &Tints) -> Image {
        self.take(k);
        let channels = tints.channels();
        let mut data = vec![0.0f32; self.levels.len() * channels];
        for (pixel, level) in self
            .levels
            .iter()
            .enumerate()
            .filter(|(_, level)| **level != EMPTY)
        {
            let at = pixel * channels;
            match tints {
                Tints::Gray(grays) => data[at] = grays[*level as usize],
                Tints::Rgb(colours) => data[at..at + 3].copy_from_slice(&colours[*level as usize]),
            }
            data[at + channels - 1] = 1.0;
        }
        Image {
            width: self.shape.sizes[0],
            height: self.shape.sizes[1],
            colors: channels - 1,
            alpha: true,
            data,
            depth: None,
        }
    }
}

/// The colours of the fourth number, from its least value to its greatest.
pub enum Tints {
    Gray(Vec<f32>),
    Rgb(Vec<Rgb>),
}

impl Tints {
    pub fn new(count: usize, stops: Option<&[Rgb]>) -> Tints {
        let share = |k: usize| k as f64 / (count.max(2) - 1) as f64;
        match stops {
            None => Tints::Gray((0..count).map(|k| share(k) as f32).collect()),
            Some(stops) => Tints::Rgb((0..count).map(|k| raster::along(stops, share(k))).collect()),
        }
    }

    fn channels(&self) -> usize {
        match self {
            Tints::Gray(_) => 2,
            Tints::Rgb(_) => 4,
        }
    }

    /// How many of the colours differ in a file with so many bits a sample.
    fn different(&self, bits: u32) -> usize {
        let max = (1u32 << bits) - 1;
        let mut all: Vec<[u32; 3]> = match self {
            Tints::Gray(grays) => grays
                .iter()
                .map(|g| [imgfile::step(*g, max), 0, 0])
                .collect(),
            Tints::Rgb(colours) => colours
                .iter()
                .map(|c| c.map(|v| imgfile::step(v, max)))
                .collect(),
        };
        all.sort_unstable();
        all.dedup();
        all.len()
    }
}

/// The least and the greatest of a part of a number.
pub(crate) type Ends = (f64, f64);

/// The corners of the box of the arguments as numbers.
fn corner(e: &Expr) -> Option<(f64, f64)> {
    if !e.is_const() {
        return None;
    }
    let v = numeric::evalf(e, 64).ok()?;
    let (re, im) = (v.real().to_f64(), v.imag().to_f64());
    (re.is_finite() && im.is_finite()).then_some((re, im))
}

const USAGE: &str = "plot4d(f, z, a, b): the function f of the complex number z in the box with the corners a and b, as plot4d(sin(z), z, -2 - 2i, 2 + 2i); plot4d(f, z, r) is from -r - r*i to r + r*i, plot4d(f, z) from -2 - 2i to 2 + 2i; the name z may be left out when f has one name";
const OF_FILES: &[&str] = &[
    "width",
    "height",
    "frames",
    "colors",
    "colours",
    "palette",
    "reverse",
    "bits",
    "gray",
    "grey",
    "alpha",
    "channels",
    "lossless",
    "quality",
    "distance",
    "effort",
    "level",
    "fps",
    "thickness",
    "fill",
    "arg_real",
    "arg_imag",
    "val_real",
    "val_imag",
    "range_real",
    "range_imag",
    "rotate",
    "normalize",
];
const OF_CHARACTERS: &[&str] = &[
    "width",
    "height",
    "frames",
    "ascii",
    "reverse",
    "thickness",
    "fill",
    "arg_real",
    "arg_imag",
    "val_real",
    "val_imag",
    "range_real",
    "range_imag",
    "rotate",
    "normalize",
];
const MARKS: &[u8] = b".:-=+*#%@";

impl Session {
    /// The function of a command like plot4d, the name of its complex number and the box of
    /// that number, from the arguments that are no choices.
    pub(crate) fn function_in_a_box(
        &mut self,
        name: &str,
        plain: &[Expr],
    ) -> Result<(Expr, Rc<str>, Ends, Ends), String> {
        let usage = || USAGE.replace("plot4d", name);
        let mut args = self.eval_args(plain)?;
        // a function may be written as a text
        if let Some(text) = args.first().and_then(crate::radix::text_of) {
            let parsed = {
                let funcs = &self.funcs;
                crate::parser::parse_with(text, &|name| funcs.contains_key(name))?
            };
            let [statement] = parsed.stmts.as_slice() else {
                return Err(usage());
            };
            args[0] = self.eval(statement)?;
        }
        let Some(f) = args.first().cloned() else {
            return Err(usage());
        };
        if f.as_list().is_some() || matches!(f, Expr::Rel(..) | Expr::Bool(_)) {
            return Err(usage());
        }
        let (var, rest): (Rc<str>, &[Expr]) = match args.get(1) {
            Some(Expr::Sym(name)) if !crate::expr::is_const_name(name) => {
                (name.clone(), &args[2..])
            }
            _ => {
                let names = f.free_syms();
                let mut names = names.iter().filter(|n| !crate::expr::is_const_name(n));
                match (names.next(), names.next()) {
                    (Some(name), None) => (name.clone(), &args[1..]),
                    (None, _) => {
                        return Err(format!(
                            "{name} draws a function of a complex number, and {f} has no name in it: {}",
                            usage()
                        ));
                    }
                    _ => {
                        return Err(format!(
                            "{f} has several names: {name}(f, z) says which one is the complex number"
                        ));
                    }
                }
            }
        };
        let (re, im) = match rest {
            [] => ((-2.0, 2.0), (-2.0, 2.0)),
            [r] => match crate::plot::number(r) {
                Some(r) if r > 0.0 => ((-r, r), (-r, r)),
                _ => return Err(usage()),
            },
            [a, b] => {
                let (Some(a), Some(b)) = (corner(a), corner(b)) else {
                    return Err(usage());
                };
                let ordered = |p: f64, q: f64| if p < q { (p, q) } else { (q, p) };
                let (re, im) = (ordered(a.0, b.0), ordered(a.1, b.1));
                if !(re.1 > re.0 && im.1 > im.0) {
                    return Err(format!(
                        "the corners of the box differ in their real and in their imaginary part: {}",
                        usage()
                    ));
                }
                (re, im)
            }
            _ => return Err(usage()),
        };
        Ok((f, var, re, im))
    }

    /// `plot4d(f, z, a, b)`: as a picture of characters (the frame in the middle), or saved
    /// in a file whose frames are the third dimension and whose colours are the fourth.
    pub(crate) fn plot4d(
        &mut self,
        raw: &[Expr],
        more: &[Expr],
        file: Option<(&str, Format, bool)>,
    ) -> Result<Vec<String>, String> {
        let (plain, mut choices) = self.chosen("plot4d", raw, more)?;
        choices.fast = file.is_some_and(|(_, _, fast)| fast);
        let (f, var, re, im) = self.function_in_a_box("plot4d", &plain)?;
        let allowed = if file.is_some() {
            OF_FILES
        } else {
            OF_CHARACTERS
        };
        if let Some(other) = choices
            .given
            .iter()
            .find(|g| !allowed.contains(&g.as_str()))
        {
            return Err(match file {
                None if OF_FILES.contains(&other.as_str()) => format!(
                    "{other} is chosen for the file of plot4d: save_jxl(\"plot4d.jxl\", plot4d(...), {other}{}); the picture of characters takes {}",
                    if picture::WORDS.contains(&other.as_str()) {
                        ""
                    } else {
                        " = ..."
                    },
                    OF_CHARACTERS.join(", ")
                ),
                _ => format!(
                    "{other} does not belong to plot4d: its file takes {}",
                    OF_FILES.join(", ")
                ),
            });
        }
        let given = |name: &str| choices.given.iter().any(|g| g == name);
        let frames = given("frames").then_some(choices.look.frames);
        let function = format!("{f}");
        let mut plan = plan_of("plot4d", f, var, (re, im), &choices)?;
        match file {
            None => self.plot4d_characters(&mut plan, &choices, frames, &function),
            Some((path, format, _)) => {
                plot4d_file(&mut plan, &choices, frames, &function, path, format)
            }
        }
    }

    fn plot4d_characters(
        &mut self,
        plan: &mut Plan,
        choices: &Choices,
        frames: Option<usize>,
        function: &str,
    ) -> Result<Vec<String>, String> {
        let limited =
            |given: Option<usize>, name: &str, range: (usize, usize), usual: usize| match given {
                None => Ok(usual),
                Some(v) if (range.0..=range.1).contains(&v) => Ok(v),
                Some(v) => Err(format!(
                    "the {name} of a picture of characters is from {} to {}, not {v}",
                    range.0, range.1
                )),
            };
        plan.sizes = [
            limited(choices.width, "width", (8, 400), 64)?,
            limited(choices.height, "height", (2, 200), 24)?,
            limited(frames, "frames", (1, 512), 16)?,
            MARKS.len(),
        ];
        let shape = shape(plan)?;
        let middle = plan.sizes[2] / 2;
        let mut taken = Frames::new(&shape);
        taken.take(middle);
        let mut lines: Vec<String> = taken
            .levels
            .chunks(plan.sizes[0])
            .map(|row| {
                let line: String = row
                    .iter()
                    .map(|level| match *level {
                        EMPTY => ' ',
                        level => {
                            let level = if choices.reverse {
                                MARKS.len() - 1 - level as usize
                            } else {
                                level as usize
                            };
                            MARKS[level] as char
                        }
                    })
                    .collect();
                line.trim_end().to_string()
            })
            .collect();
        let part = plan
            .axes
            .iter()
            .position(|a| *a == Axis::Frame)
            .unwrap_or(0);
        let (low, high) = shape.ranges[part];
        let cut = |k: usize| low + (high - low) * k as f64 / plan.sizes[2] as f64;
        lines.push(format!(
            "plot4d of {function}: frame {} of {}, where {} is from {} to {}; the characters {} are the colours, the least value first",
            middle + 1,
            plan.sizes[2],
            PARTS[part],
            label(cut(middle)),
            label(cut(middle + 1)),
            String::from_utf8_lossy(MARKS)
        ));
        lines.extend(facts(&shape, None));
        lines.push(
            "save_jxl(\"plot4d.jxl\", plot4d(...)) saves the whole shape: 512 frames of 512 x 512 pixels in 512 colours, with alpha where the shape is".to_string(),
        );
        Ok(lines)
    }
}

/// What is drawn of a function in a box, from what was chosen; the sizes are still to set.
pub(crate) fn plan_of(
    name: &str,
    f: Expr,
    var: Rc<str>,
    (re, im): (Ends, Ends),
    choices: &Choices,
) -> Result<Plan, String> {
    let thickness = match choices.given.iter().any(|g| g == "thickness") {
        true if choices.look.thickness.fract() != 0.0 || choices.look.thickness < 1.0 => {
            return Err(format!(
                "the thickness of {name} is a whole number of frames, 1 or more"
            ));
        }
        true => choices.look.thickness as usize,
        false => 1,
    };
    Ok(Plan {
        f,
        var,
        re,
        im,
        axes: layout(&choices.axes)?,
        sizes: [0; 4],
        range_real: choices.range_real,
        range_imag: choices.range_imag,
        fill: choices.fill.unwrap_or_default(),
        thickness,
        rotate: choices.rotate,
        normalize: choices.normalize,
    })
}

/// What the sides of the picture show, and how much of its box the shape takes.
fn facts(shape: &Shape, colours: Option<&str>) -> Vec<String> {
    let mut told = Vec::new();
    for axis in AXES {
        let part = shape.axes.iter().position(|a| *a == axis).unwrap_or(0);
        let (low, high) = shape.ranges[part];
        told.push(format!(
            "the {}: {} from {} to {}{}",
            match axis {
                Axis::Width => "width",
                Axis::Height => "height",
                Axis::Frame => "frames",
                Axis::Color => "colours",
            },
            PARTS[part],
            label(low),
            label(high),
            if axis == Axis::Height {
                ", upwards"
            } else {
                ""
            }
        ));
    }
    let mut lines = vec![format!(
        "{}{}",
        told.join("; "),
        colours.map(|c| format!(" ({c})")).unwrap_or_default()
    )];
    lines.extend(notes(shape));
    lines
}

/// What is to tell about a shape besides its sides: values that are left out, its turn, its
/// stretched frames, how much of its box it takes.
pub(crate) fn notes(shape: &Shape) -> Vec<String> {
    let mut lines = Vec::new();
    for (part, name) in [(2, "range_real"), (3, "range_imag")] {
        if shape.narrowed[part] {
            lines.push(format!(
                "{} goes far beyond these ends (near a pole): what lies outside is left out; {name} = [least, greatest] sets other ends",
                PARTS[part]
            ));
        }
    }
    if let Some(turn) = &shape.turn {
        lines.push(format!(
            "after that the body is turned{}: about the middle of its box, first about the frames (roll), then about the width (pitch), then about the upright axis (yaw)",
            match turn.scale < 1.0 {
                true => format!(" and made smaller by the factor {}, so that its box stays in the picture", label(turn.scale)),
                false => String::new(),
            }
        ));
    }
    if shape.normalize {
        lines.push(
            "every frame is stretched on its own (normalize): what it holds fills the picture and takes the whole palette, so the frames together are no body of one scale any more".to_string(),
        );
        if let Some((larger, held)) = shape.stretched
            && larger < held
        {
            let frames = shape
                .axes
                .iter()
                .position(|a| *a == Axis::Frame)
                .unwrap_or(0);
            lines.push(format!(
                "{larger} of {held} frames became larger; the others reach from edge to edge both ways as they are{}",
                match frames >= 2 {
                    true => ": with a part of z on the frames, as arg_imag = frame, val_real = height, a frame is the curve of the function along a line, which normalize makes as high as the picture",
                    false => "",
                }
            ));
        }
    }
    let [width, height, frames, _] = shape.sizes;
    let filled = match shape.fill {
        Fill::Surface => "",
        Fill::Below => "; the body below it is filled",
        Fill::Above => "; the body above it is filled",
    };
    lines.push(format!(
        "{} voxels of the box of {width} x {height} x {frames} belong to the surface ({} %){filled}",
        shape.voxels,
        label(100.0 * shape.voxels as f64 / (width * height * frames) as f64)
    ));
    lines
}

fn plot4d_file(
    plan: &mut Plan,
    choices: &Choices,
    frames: Option<usize>,
    function: &str,
    path: &str,
    format: Format,
) -> Result<Vec<String>, String> {
    if path.is_empty() {
        return Err("the name of the file is empty".to_string());
    }
    let png = format == Format::Png;
    let gray = match choices.channels {
        None => choices.gray,
        Some(2) => true,
        Some(4) if !choices.gray => false,
        Some(n) => {
            return Err(format!(
                "channels = {n}: a picture of plot4d always has alpha, which says where the shape is: channels = 2 (gray + alpha) or 4 (RGB + alpha){}",
                if n == 4 { ", and gray has 2" } else { "" }
            ));
        }
    };
    // lossless files are packed as tightly as the encoder can, unless an effort is chosen;
    // a quick look is small and packed fast
    let lossy = !png
        && (choices.quality.is_some_and(|q| q < 100.0)
            || choices.distance.is_some_and(|d| d > 0.0));
    let effort = match (choices.fast, lossy) {
        (true, _) => 3,
        (false, true) => 7,
        (false, false) => 10,
    };
    let mut encoding = picture::encoding_of(format, choices, effort)?;
    if choices.fps.is_none() {
        encoding.rate = (16, 1);
    }
    if !png {
        encoding.alpha_bits = Some(1);
    }
    let usual = if choices.fast { 256 } else { 512 };
    let colours = choices.colors.unwrap_or(usual);
    plan.sizes = [
        choices.width.unwrap_or(usual),
        choices.height.unwrap_or(usual),
        frames.unwrap_or(usual),
        colours,
    ];
    let [width, height, count, _] = plan.sizes;
    if width * height > 1 << 24 || (width * height).saturating_mul(count) > 1 << 31 {
        return Err(format!(
            "the box of plot4d has {} pixels a frame and {} voxels at most, not {width} x {height} x {count}",
            1 << 24,
            1u64 << 31
        ));
    }
    let (name, mut stops) = match &choices.look.palette {
        Some(stops) => (
            raster::PALETTES
                .iter()
                .find(|(_, known)| *known == stops.as_slice())
                .map(|(name, _)| *name),
            stops.clone(),
        ),
        None => (
            Some("rainbow"),
            crate::scene::palette("rainbow").unwrap_or_default(),
        ),
    };
    if choices.reverse {
        stops.reverse();
    }
    let tints = Tints::new(colours, (!gray).then_some(stops.as_slice()));
    let shape = shape(plan)?;
    let mut taken = Frames::new(&shape);
    let bytes = imgfile::encode(count, &mut |k| Ok(taken.image(k, &tints)), &encoding)?;
    imgfile::write(path, &bytes)?;
    let different = tints.different(encoding.bits);
    let scale = format!(
        "{colours} {}{}{}",
        match (gray, name) {
            (true, _) => "levels of gray".to_string(),
            (false, Some(name)) => format!("colours of the palette {name}"),
            (false, None) => "colours of the palette that is given".to_string(),
        },
        if choices.reverse && !gray {
            " reversed"
        } else {
            ""
        },
        if different < colours {
            format!(
                ", of which {different} differ with {} bit{} a sample",
                encoding.bits,
                if encoding.bits == 1 { "" } else { "s" }
            )
        } else {
            String::new()
        }
    );
    let mut lines = vec![format!("plot4d of {function}")];
    lines.extend(facts(
        &shape,
        Some(&format!("{scale}, the least value first")),
    ));
    let (n, d) = encoding.rate;
    let rate = if d == 1 {
        n.to_string()
    } else {
        label(f64::from(n) / f64::from(d))
    };
    let mut told = vec![
        if png { "PNG" } else { "JPEG XL" }.to_string(),
        format!("{width} x {height} pixels"),
        match count {
            1 => "1 frame".to_string(),
            _ => format!("{count} frames, {rate} a second"),
        },
        format!(
            "{} bit{}",
            encoding.bits,
            if encoding.bits == 1 { "" } else { "s" }
        ),
        format!(
            "{} + alpha{}",
            if gray { "gray" } else { "RGB" },
            if png { "" } else { " of 1 bit" }
        ),
    ];
    if png {
        told.push(format!("level {}", encoding.level));
    } else {
        told.push(match encoding.distance {
            None => "lossless".to_string(),
            Some(d) => format!(
                "lossy at the distance {} (alpha stays exact)",
                label(f64::from(d))
            ),
        });
        told.push(format!("effort {}", encoding.effort));
    }
    told.push(format!("{} bytes", bytes.len()));
    lines.push(format!("saved {path}: {}", told.join(", ")));
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(text: &str, sizes: [usize; 4]) -> Plan {
        let f = crate::parser::parse(text)
            .expect("an expression")
            .stmts
            .remove(0);
        Plan {
            f,
            var: Rc::from("z"),
            re: (-2.0, 2.0),
            im: (-2.0, 2.0),
            axes: AXES,
            sizes,
            range_real: None,
            range_imag: None,
            fill: Fill::Surface,
            thickness: 1,
            rotate: None,
            normalize: false,
        }
    }

    fn frame(shape: &Shape, k: usize) -> Vec<u16> {
        let mut frames = Frames::new(shape);
        frames.take(k);
        frames.levels
    }

    #[test]
    fn sides() {
        assert_eq!(layout(&[None; 4]), Ok(AXES));
        // (the example of the request: the imaginary part of z on the frames)
        assert_eq!(
            layout(&[None, Some(Axis::Frame), None, None]),
            Ok([Axis::Width, Axis::Frame, Axis::Height, Axis::Color])
        );
        assert_eq!(
            layout(&[Some(Axis::Color), None, None, Some(Axis::Width)]),
            Ok([Axis::Color, Axis::Height, Axis::Frame, Axis::Width])
        );
        assert!(layout(&[Some(Axis::Frame), Some(Axis::Frame), None, None]).is_err());
        assert_eq!(Axis::named(&Expr::sym("Frames")), Some(Axis::Frame));
        assert_eq!(Axis::named(&Expr::sym("colour")), Some(Axis::Color));
        assert_eq!(Axis::named(&Expr::sym("depth")), None);
        assert_eq!(Fill::named(&Expr::sym("below")), Some(Fill::Below));
        assert_eq!(Fill::named(&Expr::sym("inside")), None);
    }

    #[test]
    fn the_identity_is_a_plane() {
        // z itself: the frame is the column, the colour is the row from below
        let shape = shape(&plan("z", [8, 8, 8, 8])).expect("a shape");
        assert_eq!(shape.ranges, [(-2.0, 2.0); 4]);
        assert_eq!(shape.voxels, 64);
        for k in 0..8 {
            let levels = frame(&shape, k);
            for (pixel, level) in levels.iter().enumerate() {
                let (row, column) = (pixel / 8, pixel % 8);
                assert_eq!(
                    *level,
                    if column == k { (7 - row) as u16 } else { EMPTY },
                    "frame {k} pixel {pixel}"
                );
            }
        }
    }

    #[test]
    fn bodies_and_thickness() {
        let mut flat = plan("z", [8, 8, 8, 8]);
        flat.fill = Fill::Below;
        let below = shape(&flat).expect("a shape");
        flat.fill = Fill::Above;
        let above = shape(&flat).expect("a shape");
        for k in 0..8 {
            let (low, high) = (frame(&below, k), frame(&above, k));
            for pixel in 0..64 {
                let (row, column) = (pixel / 8, pixel % 8);
                let colour = (7 - row) as u16;
                assert_eq!(low[pixel], if column >= k { colour } else { EMPTY });
                assert_eq!(high[pixel], if column <= k { colour } else { EMPTY });
            }
        }
        // the frames may be asked for in any order
        let mut frames = Frames::new(&below);
        frames.take(5);
        frames.take(2);
        assert_eq!(frames.levels, frame(&below, 2));
        let mut frames = Frames::new(&above);
        frames.take(6);
        frames.take(1);
        assert_eq!(frames.levels, frame(&above, 1));
        flat.fill = Fill::Surface;
        flat.thickness = 3;
        let thick = shape(&flat).expect("a shape");
        let levels = frame(&thick, 4);
        assert_eq!(levels.iter().filter(|l| **l != EMPTY).count(), 24);
        assert_eq!(frame(&thick, 0).iter().filter(|l| **l != EMPTY).count(), 8);
    }

    #[test]
    fn turned_bodies() {
        // quarter turns are exact: the sides change places
        let turn = Turn::new([90.0, 0.0, 0.0]);
        assert_eq!(turn.scale, 1.0);
        assert_eq!(
            turn.to,
            [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]]
        );
        assert_eq!(
            Turn::new([0.0, 0.0, 180.0]).to,
            [[-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]]
        );
        assert_eq!(
            Turn::new([360.0, -720.0, 0.0]).to,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        );
        // an eighth of a turn about one axis: smaller by sqrt(2), and back is the inverse
        let eighth = Turn::new([45.0, 0.0, 0.0]);
        assert!((eighth.scale - 0.5f64.sqrt()).abs() < 1e-12);
        for turn in [
            eighth,
            Turn::new([30.0, 20.0, 10.0]),
            Turn::new([-75.0, 130.0, 260.0]),
        ] {
            let sizes = [8, 6, 4, 2];
            let at = [1.5, 4.25, 3.0];
            let there = Turn::apply(&turn.to, at, &sizes);
            let again = Turn::apply(&turn.back, there, &sizes);
            assert!((0..3).all(|k| (again[k] - at[k]).abs() < 1e-9), "{again:?}");
            // the corners of the box stay inside
            for corner in 0..8 {
                let corner = [0, 1, 2].map(|k| {
                    if corner >> k & 1 == 1 {
                        sizes[k] as f64
                    } else {
                        0.0
                    }
                });
                let moved = Turn::apply(&turn.to, corner, &sizes);
                assert!(
                    (0..3).all(|k| (-1e-9..=sizes[k] as f64 + 1e-9).contains(&moved[k])),
                    "{moved:?}"
                );
            }
        }
        // z itself is the plane frame = column; a quarter turn about the upright axis
        // (yaw) makes it the plane frame = 7 - column ... of the same count of voxels
        let mut turned = plan("z", [8, 8, 8, 8]);
        turned.rotate = Some([180.0, 0.0, 0.0]);
        let half = shape(&turned).expect("a shape");
        assert_eq!(half.voxels, 64);
        for k in 0..8 {
            let levels = frame(&half, k);
            for (pixel, level) in levels.iter().enumerate() {
                let (row, column) = (pixel / 8, pixel % 8);
                // (the half turn about the upright axis mirrors the width and the frames:
                // the plane stays where it was)
                assert_eq!(
                    *level,
                    if column == k { (7 - row) as u16 } else { EMPTY },
                    "frame {k} pixel {pixel}"
                );
            }
        }
        turned.rotate = Some([0.0, 0.0, 90.0]);
        let rolled = shape(&turned).expect("a shape");
        assert_eq!(rolled.voxels, 64);
        for k in 0..8 {
            // a quarter roll: the width goes up, what was up goes to the left
            for (pixel, level) in frame(&rolled, k).iter().enumerate() {
                let (row, column) = (pixel / 8, pixel % 8);
                assert_eq!(
                    *level,
                    if 7 - row == k {
                        (7 - column) as u16
                    } else {
                        EMPTY
                    },
                    "frame {k} pixel {pixel}"
                );
            }
        }
        // no turn at all is no turn
        turned.rotate = Some([0.0; 3]);
        assert!(shape(&turned).expect("a shape").turn.is_none());
        // a body that is turned by a quarter roll is the body turned
        let mut body = plan("z", [8, 8, 8, 8]);
        body.fill = Fill::Below;
        let straight = shape(&body).expect("a shape");
        body.rotate = Some([0.0, 0.0, 90.0]);
        let rolled = shape(&body).expect("a shape");
        for k in 0..8 {
            let (plain, turned) = (frame(&straight, k), frame(&rolled, k));
            for (pixel, level) in turned.iter().enumerate() {
                let (row, column) = (pixel / 8, pixel % 8);
                // the place (column, 7 - row) of the turned picture comes from (7 - row, 7 - column)
                let from = column * 8 + (7 - row);
                assert_eq!(*level, plain[from], "frame {k} pixel {pixel}");
            }
        }
        // a slanted body is the body made smaller: 288 voxels times the cube of the factor
        body.rotate = Some([30.0, 20.0, 10.0]);
        let slanted = shape(&body).expect("a shape");
        let filled: usize = (0..8)
            .map(|k| frame(&slanted, k).iter().filter(|l| **l != EMPTY).count())
            .sum();
        let scale = slanted.turn.expect("a turn").scale;
        let expected = 288.0 * scale.powi(3);
        assert!(
            (filled as f64 - expected).abs() < 0.25 * expected,
            "{filled} for {expected}"
        );
    }

    #[test]
    fn frames_stretched_on_their_own() {
        // what lies from 2 to 4 of 8 across fills the width; colours from 1 to 3 the palette
        let stretch = Stretch::new(&[(2.0, 4.0), (0.0, 8.0), (1.0, 3.0)], &[8, 8, 4, 16]);
        assert_eq!(stretch.low, [2.0, 0.0, 1.0]);
        assert_eq!(stretch.scale, [4.0, 1.0, 8.0]);
        assert_eq!(stretch.apply([3.0, 4.0, 1.5, 2.0]), [4.0, 4.0, 1.5, 8.0]);
        // nothing in a frame, and one colour alone (the middle of the palette)
        let empty = Stretch::new(&[(f64::MAX, f64::MIN); 3], &[8, 8, 4, 16]);
        assert_eq!((empty.low, empty.scale), ([0.0; 3], [1.0; 3]));
        let one = Stretch::new(&[(0.0, 8.0), (3.25, 3.75), (5.0, 5.0)], &[8, 8, 4, 16]);
        assert_eq!(one.apply([1.0, 3.5, 0.0, 5.0]), [1.0, 4.0, 0.0, 8.0]);
        // z itself: a frame holds one column of the plane; stretched, the column is the
        // whole frame, with the colours of its rows
        let mut flat = plan("z", [8, 8, 8, 8]);
        flat.normalize = true;
        let stretched = shape(&flat).expect("a shape");
        assert_eq!(stretched.voxels, 512);
        for k in 0..8 {
            for (pixel, level) in frame(&stretched, k).iter().enumerate() {
                assert_eq!(*level, (7 - pixel / 8) as u16, "frame {k} pixel {pixel}");
            }
        }
        // colours that take the middle half of the palette take all of it in every frame
        flat.normalize = false;
        flat.range_imag = Some((-4.0, 4.0));
        let narrow = shape(&flat).expect("a shape");
        let levels = |shape: &Shape| -> Vec<u16> {
            let mut all: Vec<u16> = frame(shape, 3)
                .into_iter()
                .filter(|l| *l != EMPTY)
                .collect();
            all.sort_unstable();
            all.dedup();
            all
        };
        assert_eq!(levels(&narrow), [2, 3, 4, 5]);
        flat.normalize = true;
        let wide = shape(&flat).expect("a shape");
        assert_eq!(levels(&wide), [0, 1, 2, 3, 4, 5, 6, 7]);
        // a body: its cut through a frame is made as large as the picture
        let mut body = plan("z", [8, 8, 8, 8]);
        body.fill = Fill::Below;
        let plain = shape(&body).expect("a shape");
        assert_eq!(frame(&plain, 6).iter().filter(|l| **l != EMPTY).count(), 16);
        body.normalize = true;
        let filled = shape(&body).expect("a shape");
        for k in 0..8 {
            for (pixel, level) in frame(&filled, k).iter().enumerate() {
                assert_eq!(*level, (7 - pixel / 8) as u16, "frame {k} pixel {pixel}");
            }
        }
        // a frame with one colour: the middle of the palette
        let mut even = plan("3", [4, 4, 4, 8]);
        even.fill = Fill::Above;
        even.normalize = true;
        let even = shape(&even).expect("a shape");
        assert!(frame(&even, 3).iter().all(|l| *l == 4));
    }

    #[test]
    fn steep_places_have_no_holes() {
        // 4z: four frames a column; every column of pixels is in the frames without a gap
        let mut steep = plan("4*z", [16, 4, 64, 4]);
        steep.range_real = Some((-8.0, 8.0));
        let shape = shape(&steep).expect("a shape");
        let mut seen = vec![Vec::new(); 16];
        for k in 0..64 {
            for (pixel, level) in frame(&shape, k).iter().enumerate() {
                if *level != EMPTY && pixel / 16 == 0 {
                    seen[pixel % 16].push(k);
                }
            }
        }
        for (column, frames) in seen.iter().enumerate() {
            assert!(frames.len() >= 4, "column {column}: {frames:?}");
            assert!(
                frames.windows(2).all(|w| w[1] == w[0] + 1),
                "column {column}: {frames:?}"
            );
        }
        assert!(seen.windows(2).all(|w| w[1][0] <= w[0][w[0].len() - 1] + 1));
    }

    #[test]
    fn poles_are_left_out() {
        let shape = shape(&plan("1/z", [64, 64, 64, 16])).expect("a shape");
        assert!(shape.narrowed[2] && shape.narrowed[3]);
        assert!(shape.ranges[2].1 < 10.0 && shape.ranges[2].0 > -10.0);
        // no wall through all the frames at the pole
        let middle = 32 * 64 + 32;
        let through = (0..64)
            .filter(|k| frame(&shape, *k)[middle] != EMPTY)
            .count();
        assert!(through <= 4, "{through}");
        assert!(super::shape(&plan("1/0*z", [8, 8, 8, 8])).is_err());
    }

    #[test]
    fn other_sides() {
        // the argument on the colours and on the frames, the value on the picture: every
        // frame is the picture of a line of the plane, a line again for z itself
        let mut turned = plan("z", [8, 8, 8, 8]);
        turned.axes = [Axis::Color, Axis::Frame, Axis::Width, Axis::Height];
        let shape = shape(&turned).expect("a shape");
        assert_eq!(shape.voxels, 64);
        for k in 0..8 {
            let levels = frame(&shape, k);
            for (pixel, level) in levels.iter().enumerate() {
                let (row, column) = (pixel / 8, pixel % 8);
                assert_eq!(*level, if 7 - row == k { column as u16 } else { EMPTY });
            }
        }
    }

    #[test]
    fn colours_of_a_file() {
        let stops = [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0, 1.0]];
        let Tints::Rgb(colours) = Tints::new(5, Some(&stops)) else {
            panic!("colours");
        };
        assert_eq!(
            colours,
            vec![
                [0.0; 3],
                [0.0, 0.0, 0.5],
                [0.0, 0.0, 1.0],
                [0.5, 0.5, 1.0],
                [1.0; 3]
            ]
        );
        assert_eq!(Tints::new(512, None).different(9), 512);
        assert_eq!(Tints::new(512, None).different(8), 256);
        let cube = crate::scene::palette("cube").expect("the cube");
        assert_eq!(Tints::new(512, Some(&cube)).different(3), 512);
        let rainbow = crate::scene::palette("rainbow").expect("the rainbow");
        assert_eq!(Tints::new(512, Some(&rainbow)).different(8), 512);
        // a picture: colours and alpha
        let shape = shape(&plan("z", [4, 4, 4, 4])).expect("a shape");
        let image = Frames::new(&shape).image(1, &Tints::new(4, Some(&stops)));
        assert_eq!(
            (image.width, image.height, image.colors, image.alpha),
            (4, 4, 3, true)
        );
        let alpha: Vec<f32> = image.data.iter().skip(3).step_by(4).cloned().collect();
        assert_eq!(alpha, [0.0, 1.0, 0.0, 0.0].repeat(4));
        assert_eq!(image.data[4..8], [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(image.data[52..56], [0.0, 0.0, 0.0, 1.0]);
    }
}
