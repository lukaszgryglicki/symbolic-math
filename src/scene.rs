//! Drawings: things in the plane or in space, the point they are seen from, and their
//! pictures of pixels or of characters.
//!
//! As in `plot`, the numbers of a picture are 64-bit floats on which only `+ - * /` and the
//! square root act; sines and cosines come from the arbitrary-precision library. So a picture
//! is the same on every system.

use crate::plot::{self, Canvas};
use crate::raster::{CYCLE, Place, Raster, Rgb, Surface, Tone, along, luma, mix};
use rug::Float;

/// A point of space; the plane is where the third number is 0.
pub type Point = [f64; 3];

/// The colour of a thing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Paint {
    /// From the first colour of the palette to the last along the thing.
    Along,
    /// The colour of the k-th curve of a picture.
    Curve(usize),
    /// The colour of the axes: dark on a light ground, light on a dark one.
    Ink,
}

/// The colour of a point of a field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shade {
    /// A place along the palette, from 0 to 1.
    Level(f64),
    /// A colour, and how light it is from 0 to 1 for a picture without colours.
    Tint(Rgb, f64),
}

/// The colour of every point of the plane, where there is one.
pub type Field = Box<dyn Fn(f64, f64) -> Option<Shade>>;

/// What a field is told before it is asked: the places of a row of the picture.
pub type Ahead = Box<dyn Fn(&[(f64, f64)])>;

pub enum Item {
    /// A walk from the origin: step `k` goes `runs[k]` times (once without `runs`) along
    /// the direction `dirs[steps[k]]`.
    Trail {
        dirs: Vec<Point>,
        steps: Vec<u32>,
        runs: Vec<u32>,
    },
    /// A line from point to point, `weight` times as wide as the lines of the picture.
    Line {
        points: Vec<Point>,
        paint: Paint,
        weight: f64,
    },
    /// Lines from a point to another each.
    Segments {
        pairs: Vec<[Point; 2]>,
        paint: Paint,
        weight: f64,
    },
    /// Dots of the plane; `radius` in widths of a line, or in units of the drawing.
    Dots {
        points: Vec<[f64; 2]>,
        paint: Paint,
        radius: f64,
        units: bool,
    },
    /// Squares of side 1 around points of whole numbers of the plane.
    Squares {
        at: Vec<[i32; 2]>,
        paint: Paint,
    },
    /// Boxes of the plane: left, bottom, right, top.
    Boxes {
        boxes: Vec<[f64; 4]>,
        paint: Paint,
    },
    Field(Field),
    /// A surface over a grid with `columns` points in a row; lines along every `wires`-th
    /// row and column.
    Mesh {
        columns: usize,
        points: Vec<Option<Point>>,
        wires: usize,
    },
    /// A word at a point.
    Label {
        at: Point,
        text: String,
    },
    /// A body of small cubes with colours.
    Body(Body),
}

/// A side of a cube of a body that lies open: the cube (across, upwards, along the frames),
/// which of its six sides (the lower and the upper one across, upwards, along the frames),
/// its colour as a place in the colours of the body, and the direction that the surface
/// faces there (all zeros: that of the side).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    pub at: [u16; 3],
    pub side: u8,
    pub level: u16,
    pub facing: [i8; 3],
}

/// A body of cubes in a box: only the sides that can be seen from outside.
pub struct Body {
    /// Cubes across, upwards and along the third side.
    pub size: [usize; 3],
    /// The ends of the box along its three sides.
    pub ends: [(f64, f64); 3],
    pub faces: Vec<Face>,
    pub tints: Vec<Rgb>,
}

/// The colours of a fourth number: its name, its least and its greatest value, and the
/// colours from the least value on.
#[derive(Clone, Debug, PartialEq)]
pub struct Legend {
    pub name: String,
    pub ends: (f64, f64),
    pub colors: Vec<Rgb>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Space {
    /// The plane seen from above, with one scale.
    Flat,
    /// Space seen from a point.
    Solid,
    /// A chart with a range along each axis; `equal`: both with one scale.
    Chart {
        x: (f64, f64),
        y: (f64, f64),
        equal: bool,
    },
}

pub struct Drawing {
    pub items: Vec<Item>,
    pub space: Space,
    /// The ranges of space that fill the picture alike (a surface over a plane), where the
    /// three scales are not one.
    pub stretch: Option<[(f64, f64); 3]>,
    /// The names of the curves, when there are several.
    pub names: Vec<String>,
    /// What is to tell about the drawing.
    pub facts: Vec<String>,
    /// The palette that suits it.
    pub palette: &'static str,
    /// Light things on a dark ground.
    pub dark: bool,
    /// It has axes unless they are turned off.
    pub axes: bool,
    /// Many small things that may be fainter than a pixel: the picture is made as light as
    /// it can be.
    pub faint: bool,
    /// The height of the picture for a width of 1, when no height is chosen.
    pub aspect: Option<f64>,
    /// How much the farthest things fade into the background when nothing else is chosen.
    pub fog: f64,
    /// The x and the y axis of a chart hold logarithms to base 10: their numbers are the
    /// powers of ten.
    pub logs: [bool; 2],
    /// The sides of the box that a stretched drawing of space fills, the longest being 1.
    pub sides: [f64; 3],
    /// The names of the three axes of a drawing of space, instead of x, y and z.
    pub axis_names: Option<[String; 3]>,
    pub legend: Option<Legend>,
    /// Edges of areas that would show the steps of the pixels: the picture is drawn twice
    /// as large and made smaller.
    pub smooth: bool,
    /// A field whose values take long gets the places of every row before it is asked there.
    pub ahead: Option<Ahead>,
}

impl Drawing {
    pub fn new(space: Space, items: Vec<Item>) -> Drawing {
        Drawing {
            items,
            space,
            stretch: None,
            names: Vec::new(),
            facts: Vec::new(),
            palette: "rainbow",
            dark: !matches!(space, Space::Chart { .. }),
            axes: matches!(space, Space::Chart { .. }),
            faint: false,
            aspect: None,
            fog: if space == Space::Solid { 0.5 } else { 0.0 },
            logs: [false; 2],
            sides: [1.0, 1.0, 0.7],
            axis_names: None,
            legend: None,
            smooth: false,
            ahead: None,
        }
    }

    /// Every point of the drawing.
    pub fn points(&self, see: &mut dyn FnMut(Point)) {
        for item in &self.items {
            match item {
                Item::Trail { dirs, steps, runs } => {
                    let mut at = [0.0; 3];
                    see(at);
                    for (k, step) in steps.iter().enumerate() {
                        at = moved(at, dirs[*step as usize], runs.get(k).copied());
                        see(at);
                    }
                }
                Item::Line { points, .. } => points.iter().for_each(|p| see(*p)),
                Item::Segments { pairs, .. } => pairs.iter().flatten().for_each(|p| see(*p)),
                Item::Dots {
                    points,
                    radius,
                    units,
                    ..
                } => {
                    // (a dot whose size is in units of the drawing reaches so far)
                    let r = if *units { *radius } else { 0.0 };
                    for p in points {
                        see([p[0] - r, p[1] - r, 0.0]);
                        see([p[0] + r, p[1] + r, 0.0]);
                    }
                }
                Item::Squares { at, .. } => {
                    for p in at {
                        let (x, y) = (f64::from(p[0]), f64::from(p[1]));
                        see([x - 0.5, y - 0.5, 0.0]);
                        see([x + 0.5, y + 0.5, 0.0]);
                    }
                }
                Item::Boxes { boxes, .. } => {
                    for b in boxes {
                        see([b[0], b[1], 0.0]);
                        see([b[2], b[3], 0.0]);
                    }
                }
                Item::Mesh { points, .. } => points.iter().flatten().for_each(|p| see(*p)),
                Item::Label { at, .. } => see(*at),
                Item::Field(_) => {}
                Item::Body(body) => {
                    for k in 0..8usize {
                        see([0, 1, 2].map(|axis| match k >> axis & 1 {
                            1 => body.ends[axis].1,
                            _ => body.ends[axis].0,
                        }));
                    }
                }
            }
        }
    }

    /// The least and the largest of every coordinate.
    pub fn extent(&self) -> Option<[(f64, f64); 3]> {
        let mut ends: Option<[(f64, f64); 3]> = None;
        self.points(&mut |p| {
            if p.iter().all(|v| v.is_finite()) {
                ends = Some(match ends {
                    None => p.map(|v| (v, v)),
                    Some(e) => [0, 1, 2].map(|k| (e[k].0.min(p[k]), e[k].1.max(p[k]))),
                });
            }
        });
        ends
    }
}

/// A point after a run along a direction.
pub fn moved(at: Point, dir: Point, run: Option<u32>) -> Point {
    let run = f64::from(run.unwrap_or(1));
    [
        at[0] + dir[0] * run,
        at[1] + dir[1] * run,
        at[2] + dir[2] * run,
    ]
}

/// The sine and the cosine of `part` of a full turn.
pub fn turn(part: (u64, u64)) -> (f64, f64) {
    let (p, q) = (part.0 % part.1, part.1);
    // the quarters of a turn are exact
    if (4 * u128::from(p)).is_multiple_of(u128::from(q)) {
        return match 4 * u128::from(p) / u128::from(q) {
            0 => (0.0, 1.0),
            1 => (1.0, 0.0),
            2 => (0.0, -1.0),
            _ => (-1.0, 0.0),
        };
    }
    let angle = Float::with_val(128, rug::float::Constant::Pi) * 2u32 * p / q;
    (angle.clone().sin().to_f64(), angle.cos().to_f64())
}

/// The sine and the cosine of an angle in degrees.
pub fn sincos(degrees: f64) -> (f64, f64) {
    let quarters = degrees / 90.0;
    if quarters == quarters.floor() && quarters.abs() < 1e15 {
        return turn(((quarters as i64).rem_euclid(4) as u64, 4));
    }
    let angle =
        Float::with_val(128, degrees) * Float::with_val(128, rug::float::Constant::Pi) / 180u32;
    (angle.clone().sin().to_f64(), angle.cos().to_f64())
}

/// How a drawing is to be shown: the choices of the one who saves it.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub width: usize,
    pub height: usize,
    /// The width of a line in pixels.
    pub thickness: f64,
    /// The free rim, as a part of the smaller side.
    pub margin: f64,
    pub palette: Option<Vec<Rgb>>,
    pub color: Option<Rgb>,
    pub background: Option<Rgb>,
    pub axes: Option<bool>,
    pub grid: bool,
    /// How much the farthest things fade into the background, from 0 to 1.
    pub fog: Option<f64>,
    /// Degrees that the eye goes around the upright axis, and above the plane, and that the
    /// picture is turned.
    pub azimuth: Option<f64>,
    pub elevation: Option<f64>,
    pub roll: f64,
    pub zoom: f64,
    /// Pixels for one unit, instead of a picture that is filled.
    pub scale: Option<f64>,
    /// The point in the middle of the picture.
    pub center: Option<Point>,
    /// The point of the eye: things that are near it are larger.
    pub viewpoint: Option<Point>,
    /// How far the eye is, in radii of the drawing.
    pub distance: Option<f64>,
    pub frames: usize,
    /// Degrees that the eye goes around the drawing during the frames.
    pub turn: Option<f64>,
    /// The walks grow from frame to frame.
    pub grow: bool,
    /// Frames that are slabs of the depth, the nearest first.
    pub slices: usize,
    /// Three frames: the drawing from above, from the front and from the right.
    pub views: bool,
    /// Degrees between the two eyes of a picture for red and cyan glasses.
    pub stereo: Option<f64>,
    /// Words above a chart, under its x axis and over its y axis.
    pub title: Option<String>,
    pub xlabel: Option<String>,
    pub ylabel: Option<String>,
    /// The picture is drawn so many times as wide and as high as it will be (its pixels
    /// are the means of as many squares of pixels): lines and letters are as much larger.
    pub fine: usize,
}

impl Default for Look {
    fn default() -> Look {
        Look {
            width: 1024,
            height: 0,
            thickness: 0.0,
            margin: 0.04,
            palette: None,
            color: None,
            background: None,
            axes: None,
            grid: false,
            fog: None,
            azimuth: None,
            elevation: None,
            roll: 0.0,
            zoom: 1.0,
            scale: None,
            center: None,
            viewpoint: None,
            distance: None,
            frames: 1,
            turn: None,
            grow: false,
            slices: 0,
            views: false,
            stereo: None,
            title: None,
            xlabel: None,
            ylabel: None,
            fine: 1,
        }
    }
}

fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(a: Point) -> Option<Point> {
    let length = dot(a, a).sqrt();
    (length > 0.0 && length.is_finite()).then(|| a.map(|v| v / length))
}

/// How the points of a drawing become places of a picture.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    /// A point is first moved by `-origin` and scaled by `factor`, coordinate by coordinate.
    origin: Point,
    factor: Point,
    /// The middle of the picture, and the directions of the picture in space: to the right,
    /// up, and toward the eye.
    center: Point,
    right: Point,
    up: Point,
    toward: Point,
    /// How far the eye is from the middle; without it all lines of sight are parallel.
    eye: Option<f64>,
    /// Pixels for a unit, across and down.
    scale: [f64; 2],
    /// The place of the middle.
    shift: [f64; 2],
}

impl View {
    /// A point to the right of the middle, above it and toward the eye, in units of space.
    fn seen(&self, p: Point) -> Option<Point> {
        let v = [0, 1, 2].map(|k| (p[k] - self.origin[k]) * self.factor[k] - self.center[k]);
        let (a, b, c) = (dot(v, self.right), dot(v, self.up), dot(v, self.toward));
        match self.eye {
            None => Some([a, b, c]),
            // (what is beside or behind the eye is not seen)
            Some(d) if c < 0.98 * d => {
                let near = d / (d - c);
                Some([a * near, b * near, c])
            }
            Some(_) => None,
        }
    }

    pub fn place(&self, p: Point) -> Option<Place> {
        let [a, b, c] = self.seen(p)?;
        Some([
            self.shift[0] + self.scale[0] * a,
            self.shift[1] - self.scale[1] * b,
            -c * self.scale[0],
        ])
    }

    /// The picture shows the plane from above with its axes along the sides.
    fn level(&self) -> bool {
        self.right == [1.0, 0.0, 0.0] && self.up == [0.0, 1.0, 0.0] && self.eye.is_none()
    }

    /// The point of the plane at a place of a level picture.
    fn back(&self, x: f64, y: f64) -> (f64, f64) {
        (
            ((x - self.shift[0]) / self.scale[0] + self.center[0]) / self.factor[0]
                + self.origin[0],
            ((self.shift[1] - y) / self.scale[1] + self.center[1]) / self.factor[1]
                + self.origin[1],
        )
    }

    /// The view from an eye that went `degrees` around the upright axis.
    fn turned(&self, degrees: f64) -> View {
        let (s, c) = sincos(degrees);
        let around = |v: Point| [v[0] * c - v[1] * s, v[0] * s + v[1] * c, v[2]];
        View {
            right: around(self.right),
            up: around(self.up),
            toward: around(self.toward),
            ..self.clone()
        }
    }
}

/// Round numbers that cut a range into about `parts` parts.
pub fn ticks(low: f64, high: f64, parts: usize) -> Vec<f64> {
    let span = high - low;
    if span.is_nan() || span <= 0.0 || !span.is_finite() || low.abs() > 1e300 || high.abs() > 1e300
    {
        return Vec::new();
    }
    let rough = span / parts.max(1) as f64;
    // the power of ten at or below it
    let mut power = 1.0;
    if rough >= 1.0 {
        while power * 10.0 <= rough {
            power *= 10.0;
        }
    } else {
        while power > rough {
            power /= 10.0;
        }
    }
    let step = [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|m| m * power)
        .find(|s| *s >= rough)
        .unwrap_or(10.0 * power);
    let first = (low / step).ceil();
    let last = (high / step).floor();
    if (last - first).is_nan() || last - first >= 1000.0 {
        return Vec::new();
    }
    (0..=(last - first) as i64)
        .map(|k| {
            let v = (first + k as f64) * step;
            // (a number that is zero but for the rounding)
            if v.abs() < step * 1e-9 { 0.0 } else { v }
        })
        .collect()
}

/// The numbers along a side of a chart, and where they stand.
type Marks = Vec<(f64, f64)>;

/// The parts of a picture that do not change from frame to frame.
pub struct Shot<'a> {
    drawing: &'a Drawing,
    look: Look,
    view: View,
    /// The nearest and the farthest depth of the drawing.
    depth: (f64, f64),
    /// The box of a chart in the picture: left, top, right, bottom.
    plot: [f64; 4],
    /// The drawing stands in that box, with numbers along its sides.
    framed: bool,
    /// The size of the letters: 8 pixels times this.
    letters: usize,
    palette: Vec<Rgb>,
    background: Rgb,
    ink: Rgb,
    axes: bool,
    fog: f64,
}

/// The palette with a name.
pub fn palette(name: &str) -> Option<Vec<Rgb>> {
    crate::raster::PALETTES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, stops)| stops.to_vec())
}

const OPTIONS_OF_SPACE: &str = "azimuth, elevation, roll, viewpoint, perspective, center, scale, zoom, fog, frames, turn, grow, slices, views and stereo belong to a drawing of a plane or of space (walk, walk3d, plot3d ...), not to a chart";
const SEVERAL_EYES: &str = "views and stereo show a drawing of space from several points: walk3d, primewalk3d, primemodwalk3d, plot3d, parametricplot3d";

/// The directions of a picture in space: to the right, up, and toward the eye, which stands
/// `azimuth` degrees around the upright axis and `elevation` degrees above the plane; the
/// picture is turned by `roll` degrees.
pub(crate) fn directions(azimuth: f64, elevation: f64, roll: f64) -> [Point; 3] {
    let (sa, ca) = sincos(azimuth - 90.0);
    let (se, ce) = sincos(elevation);
    seen_along([ce * ca, ce * sa, se], azimuth, roll)
}

/// The same for an eye in the direction `toward`; `azimuth` tells what is to the right when
/// the eye looks straight down or up.
fn seen_along(toward: Point, azimuth: f64, roll: f64) -> [Point; 3] {
    let right = if toward[0] == 0.0 && toward[1] == 0.0 {
        let (sa, ca) = sincos(azimuth - 90.0);
        [-sa, ca, 0.0]
    } else {
        unit([-toward[1], toward[0], 0.0]).unwrap_or([1.0, 0.0, 0.0])
    };
    let up = cross(toward, right);
    let (sr, cr) = sincos(roll);
    [
        [0, 1, 2].map(|k| cr * right[k] + sr * up[k]),
        [0, 1, 2].map(|k| cr * up[k] - sr * right[k]),
        toward,
    ]
}

impl<'a> Shot<'a> {
    /// The view of a drawing in a picture of `width` by `height` pixels (or dots of a text).
    pub fn new(drawing: &'a Drawing, look: &Look) -> Result<Shot<'a>, String> {
        let mut look = look.clone();
        let chart = matches!(drawing.space, Space::Chart { .. });
        if look.height == 0 {
            look.height = match (drawing.space, drawing.aspect) {
                (_, Some(aspect)) => {
                    ((look.width as f64 * aspect.clamp(0.25, 4.0)).round() as usize).max(1)
                }
                (Space::Chart { equal: false, .. }, None) => (look.width * 3).div_ceil(4),
                _ => look.width,
            };
        }
        let (w, h) = (look.width as f64, look.height as f64);
        let least = if w < h { w } else { h };
        if look.thickness <= 0.0 {
            look.thickness = if chart && least >= 600.0 { 2.0 } else { 1.0 };
        }
        let fine = look.fine.max(1);
        if fine > 1 {
            look.thickness *= fine as f64;
            look.scale = look.scale.map(|s| s * fine as f64);
        }
        let background = look
            .background
            .unwrap_or(if drawing.dark { [0.0; 3] } else { [1.0; 3] });
        let ink = if luma(background) < 0.5 {
            [0.85; 3]
        } else {
            [0.1; 3]
        };
        // each eye of a picture for two eyes sees light and dark, no colours
        if look.stereo.is_some() && look.color.is_none() {
            look.color = Some(if luma(background) < 0.5 {
                [1.0; 3]
            } else {
                [0.0; 3]
            });
        }
        let palette = look
            .palette
            .clone()
            .or_else(|| palette(drawing.palette))
            .unwrap_or_else(|| vec![[1.0; 3]]);
        let axes = look.axes.unwrap_or(drawing.axes);
        let letters = (((least / fine as f64 + 250.0) / 500.0) as usize).max(1) * fine;
        let pad = (look.margin * least).round() + look.thickness;
        let mut view = View {
            origin: [0.0; 3],
            factor: [1.0; 3],
            center: [0.0; 3],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            toward: [0.0, 0.0, 1.0],
            eye: None,
            scale: [1.0; 2],
            shift: [0.0; 2],
        };
        // the eye is where it is without a word of the one who saves
        let plain = look.azimuth.is_none()
            && look.elevation.is_none()
            && look.roll == 0.0
            && look.viewpoint.is_none()
            && look.distance.is_none()
            && look.center.is_none()
            && look.scale.is_none()
            && look.zoom == 1.0
            && look.turn.is_none()
            && look.slices == 0
            && !look.views
            && look.stereo.is_none();
        if drawing.space != Space::Solid && (look.views || look.stereo.is_some()) {
            return Err(if chart {
                OPTIONS_OF_SPACE
            } else {
                SEVERAL_EYES
            }
            .to_string());
        }
        if !chart && (look.xlabel.is_some() || look.ylabel.is_some()) {
            return Err(
                "xlabel and ylabel name the axes of a chart (plot, bar, scatter ...); a title goes with every picture"
                    .to_string(),
            );
        }
        let ranges = match drawing.space {
            Space::Chart { x, y, equal } => {
                if !plain || look.fog.is_some() || look.frames > 1 || look.grow {
                    return Err(OPTIONS_OF_SPACE.to_string());
                }
                Some((x, y, equal))
            }
            // the plane from above with axes is a chart with one scale
            Space::Flat if axes && plain && (look.frames <= 1 || look.grow) => {
                drawing.extent().map(|ends| {
                    let wide = |(low, high): (f64, f64)| {
                        let rim = if high > low { 0.03 * (high - low) } else { 1.0 };
                        (low - rim, high + rim)
                    };
                    (wide(ends[0]), wide(ends[1]), true)
                })
            }
            _ => None,
        };
        if let Some((x, y, equal)) = ranges {
            // (lines of words above and below the chart take their room)
            let line = 1.5 * 8.0 * letters as f64;
            let above = line
                * (usize::from(look.title.is_some()) + usize::from(look.ylabel.is_some())) as f64;
            let below = line * usize::from(look.xlabel.is_some()) as f64;
            let mut plot = chart_box(
                &mut view,
                (w, h - above - below),
                x,
                y,
                equal,
                pad,
                axes.then_some((letters, drawing.logs[1])),
            );
            if above > 0.0 {
                plot[1] += above;
                plot[3] += above;
                view.shift[1] += above;
            }
            return Ok(Shot {
                drawing,
                look,
                view,
                depth: (0.0, 0.0),
                plot,
                framed: true,
                letters,
                palette,
                background,
                ink,
                axes,
                fog: 0.0,
            });
        }
        // space: the three ranges of a stretched drawing fill a box alike
        if let Some(ranges) = drawing.stretch {
            for (k, (low, high)) in ranges.into_iter().enumerate() {
                let half = (high - low) / 2.0;
                view.origin[k] = (low + high) / 2.0;
                view.factor[k] = if half > 0.0 {
                    drawing.sides[k] / half
                } else {
                    1.0
                };
            }
        }
        let (origin, factor) = (view.origin, view.factor);
        let model = |p: Point| [0, 1, 2].map(|k| (p[k] - origin[k]) * factor[k]);
        // with axes, the corners of the box around the drawing belong to the picture
        let corners: Vec<Point> = match (axes, drawing.stretch.or_else(|| drawing.extent())) {
            (true, Some(box_)) => (0..8usize)
                .map(|k| {
                    [0, 1, 2].map(|axis| {
                        if k >> axis & 1 == 1 {
                            box_[axis].1
                        } else {
                            box_[axis].0
                        }
                    })
                })
                .collect(),
            _ => Vec::new(),
        };
        let all = |see: &mut dyn FnMut(Point)| {
            drawing.points(see);
            corners.iter().for_each(|c| see(*c));
        };
        // (room for the names of the axes, and above for their ranges)
        // (the ends of named axes stand under a title, and the colours under them)
        let lines = if drawing.legend.is_some() { 5.25 } else { 4.0 }
            + if drawing.axis_names.is_some() && look.title.is_some() {
                1.5
            } else {
                0.0
            };
        let (pad, band) = if axes {
            (
                pad + 2.0 * 8.0 * letters as f64,
                lines * 8.0 * letters as f64,
            )
        } else {
            (pad, 0.0)
        };
        let mut ends: Option<[(f64, f64); 3]> = None;
        all(&mut |p| {
            let p = model(p);
            if p.iter().all(|v| v.is_finite()) {
                ends = Some(match ends {
                    None => p.map(|v| (v, v)),
                    Some(e) => [0, 1, 2].map(|k| (e[k].0.min(p[k]), e[k].1.max(p[k]))),
                });
            }
        });
        let ends = ends.unwrap_or([(0.0, 0.0); 3]);
        let center = match look.center {
            Some(c) => model(c),
            None => ends.map(|(low, high)| (low + high) / 2.0),
        };
        view.center = center;
        let mut radius = 0.0f64;
        all(&mut |p| {
            let p = model(p);
            let v = [p[0] - center[0], p[1] - center[1], p[2] - center[2]];
            let far = dot(v, v);
            if far.is_finite() && far > radius {
                radius = far;
            }
        });
        let radius = if radius > 0.0 { radius.sqrt() } else { 1.0 };
        // the directions of the picture
        let solid = drawing.space == Space::Solid;
        [view.right, view.up, view.toward] = match look.viewpoint {
            Some(eye) => {
                let eye = model(eye);
                let away = [eye[0] - center[0], eye[1] - center[1], eye[2] - center[2]];
                view.eye = Some(dot(away, away).sqrt());
                let toward = unit(away).ok_or(
                    "the viewpoint is the middle of the drawing: there is no direction to look in",
                )?;
                seen_along(toward, look.azimuth.unwrap_or(0.0), look.roll)
            }
            None => directions(
                look.azimuth.unwrap_or(if solid { 30.0 } else { 0.0 }),
                look.elevation.unwrap_or(if solid { 25.0 } else { 90.0 }),
                look.roll,
            ),
        };
        if let Some(d) = look.distance {
            if d.is_nan() || d <= 1.0 {
                return Err(format!(
                    "the distance of the eye is counted in radii of the drawing and is above 1, not {d}"
                ));
            }
            view.eye = Some(d * radius);
        }
        // the scale: the picture is filled; when the eye goes around, or the middle is
        // given, the ball around the middle fits in
        let spins = look.slices == 0
            && look.frames > 1
            && look.turn != Some(0.0)
            && !(look.grow && look.turn.is_none());
        let fixed = spins || look.center.is_some() || look.views;
        let mut bounds: Option<[(f64, f64); 3]> = None;
        if fixed {
            let seen = match view.eye {
                Some(d) if d > radius => radius * d / (d * d - radius * radius).sqrt(),
                Some(_) => 4.0 * radius,
                None => radius,
            };
            bounds = Some([(-seen, seen), (-seen, seen), (-radius, radius)]);
        } else {
            all(&mut |p| {
                if let Some(s) = view.seen(p)
                    && s.iter().all(|v| v.is_finite())
                {
                    bounds = Some(match bounds {
                        None => s.map(|v| (v, v)),
                        Some(e) => [0, 1, 2].map(|k| (e[k].0.min(s[k]), e[k].1.max(s[k]))),
                    });
                }
            });
        }
        let bounds = bounds.unwrap_or([(-1.0, 1.0); 3]);
        let span = |k: usize| {
            let s = bounds[k].1 - bounds[k].0;
            if s > 0.0 { s } else { f64::INFINITY }
        };
        let room = [w - 2.0 * pad, h - 2.0 * pad - band].map(|r| if r > 1.0 { r } else { 1.0 });
        let fill = {
            let (across, down) = (room[0] / span(0), room[1] / span(1));
            let s = if across < down { across } else { down };
            if s.is_finite() && s > 0.0 { s } else { 1.0 }
        };
        let scale = look.scale.unwrap_or(fill) * look.zoom;
        view.scale = [scale, scale];
        let middle = |k: usize| {
            if fixed || look.scale.is_some() {
                0.0
            } else {
                (bounds[k].0 + bounds[k].1) / 2.0
            }
        };
        view.shift = [
            w / 2.0 - scale * middle(0),
            (h + band) / 2.0 + scale * middle(1),
        ];
        let fog = look.fog.unwrap_or(drawing.fog).clamp(0.0, 1.0);
        Ok(Shot {
            drawing,
            look,
            view,
            depth: (-bounds[2].1 * scale, -bounds[2].0 * scale),
            plot: [pad, pad, w - pad, h - pad],
            framed: false,
            letters,
            palette,
            background,
            ink,
            axes,
            fog,
        })
    }

    pub fn size(&self) -> (usize, usize) {
        (self.look.width, self.look.height)
    }

    /// How many frames the picture has.
    pub fn frames(&self) -> usize {
        if self.look.slices > 0 {
            self.look.slices
        } else if self.look.views {
            3
        } else {
            self.look.frames.max(1)
        }
    }

    /// The frame `k` for the left and for the right eye of a picture for two eyes.
    pub fn pair(&self, k: usize) -> [Raster; 2] {
        let apart = self.look.stereo.unwrap_or(0.0);
        [
            self.seen_from(k, -apart / 2.0),
            self.seen_from(k, apart / 2.0),
        ]
    }

    /// How a picture is made of a frame.
    pub fn tone(&self, gray: bool, alpha: bool, depth: bool) -> Tone {
        Tone {
            background: self.background,
            gray,
            alpha,
            depth: depth.then_some((self.depth.0 as f32, self.depth.1 as f32)),
        }
    }

    /// The colour of a thing, at a place from 0 to 1 along it.
    fn paint(&self, paint: Paint, along_it: f64) -> Rgb {
        match (paint, self.look.color) {
            (Paint::Ink, _) => self.ink,
            (_, Some(color)) => color,
            (Paint::Along, None) => along(&self.palette, along_it),
            (Paint::Curve(k), None) => CYCLE[k % CYCLE.len()],
        }
    }

    /// A colour as it is seen at a depth: the far things fade into the background.
    fn faded(&self, color: Rgb, depth: f64) -> Rgb {
        let span = self.depth.1 - self.depth.0;
        if self.fog <= 0.0 || span.is_nan() || span <= 0.0 {
            return color;
        }
        let far = ((depth - self.depth.0) / (self.depth.1 - self.depth.0)).clamp(0.0, 1.0);
        mix(color, self.background, (self.fog * far) as f32)
    }

    /// The frame `k` of the picture.
    pub fn raster(&self, k: usize) -> Raster {
        self.seen_from(k, 0.0)
    }

    /// The frame `k`, seen by an eye that stands `aside` degrees further around the drawing.
    fn seen_from(&self, k: usize, aside: f64) -> Raster {
        let mut raster = Raster::new(self.look.width, self.look.height);
        let count = self.frames();
        let mut view = self.view.clone();
        let mut part = 1.0;
        if self.look.views {
            let (azimuth, elevation) = [(0.0, 90.0), (0.0, 0.0), (90.0, 0.0)][k.min(2)];
            [view.right, view.up, view.toward] = directions(azimuth, elevation, self.look.roll);
        }
        if aside != 0.0 {
            view = view.turned(aside);
        }
        if self.look.slices > 0 {
            let (near, far) = self.depth;
            let slab = (far - near) / count as f64;
            let from = near + slab * k as f64;
            // (the last slab holds the farthest things too)
            let to = if k + 1 == count {
                f64::INFINITY
            } else {
                from + slab
            };
            raster.window = Some((from as f32, to as f32));
        } else if count > 1 && !self.look.views {
            if self.look.grow {
                part = (k + 1) as f64 / count as f64;
            }
            let degrees = self
                .look
                .turn
                .unwrap_or(if self.look.grow { 0.0 } else { 360.0 });
            if degrees != 0.0 {
                view = view.turned(degrees * k as f64 / count as f64);
            }
        }
        self.draw(&view, part, &mut raster);
        if self.drawing.faint {
            raster.brighten();
        }
        raster
    }

    /// The drawing on a surface.
    fn draw<S: Surface + Area>(&self, view: &View, part: f64, on: &mut S) {
        let half = self.look.thickness / 2.0;
        let chart = self.framed;
        if chart && self.look.grid && self.axes {
            self.rules(view, on);
        }
        if chart {
            on.clip(Some(self.plot));
        }
        for item in &self.drawing.items {
            match item {
                Item::Trail { dirs, steps, runs } => {
                    let total = steps.len();
                    let shown = ((total as f64 * part).ceil() as usize).min(total);
                    let mut at = [0.0; 3];
                    let mut from = view.place(at);
                    for k in 0..shown {
                        at = moved(at, dirs[steps[k] as usize], runs.get(k).copied());
                        let to = view.place(at);
                        if let (Some(a), Some(b)) = (from, to) {
                            let color = |k: usize, p: &Place| {
                                self.faded(self.paint(Paint::Along, k as f64 / total as f64), p[2])
                            };
                            on.segment(a, b, half, color(k, &a), color(k + 1, &b));
                        }
                        from = to;
                    }
                }
                Item::Line {
                    points,
                    paint,
                    weight,
                } => {
                    let last = points.len().saturating_sub(1).max(1) as f64;
                    let mut from: Option<Place> = None;
                    for (k, p) in points.iter().enumerate() {
                        let to = view.place(*p);
                        if let (Some(a), Some(b)) = (from, to) {
                            let color = |k: usize, p: &Place| {
                                self.faded(self.paint(*paint, k as f64 / last), p[2])
                            };
                            on.segment(a, b, half * weight, color(k - 1, &a), color(k, &b));
                        }
                        from = to;
                    }
                }
                Item::Segments {
                    pairs,
                    paint,
                    weight,
                } => {
                    let last = pairs.len().saturating_sub(1).max(1) as f64;
                    for (k, [p, q]) in pairs.iter().enumerate() {
                        if let (Some(a), Some(b)) = (view.place(*p), view.place(*q)) {
                            let color = self.paint(*paint, k as f64 / last);
                            on.segment(
                                a,
                                b,
                                half * weight,
                                self.faded(color, a[2]),
                                self.faded(color, b[2]),
                            );
                        }
                    }
                }
                Item::Dots {
                    points,
                    paint,
                    radius,
                    units,
                } => {
                    let last = points.len().saturating_sub(1).max(1) as f64;
                    let radius = radius
                        * if *units {
                            view.scale[0]
                        } else {
                            self.look.thickness
                        };
                    for (k, p) in points.iter().enumerate() {
                        if let Some(at) = view.place([p[0], p[1], 0.0]) {
                            let color = self.faded(self.paint(*paint, k as f64 / last), at[2]);
                            on.disc(at, radius, color);
                        }
                    }
                }
                Item::Squares { at, paint } => {
                    let last = at.len().saturating_sub(1).max(1) as f64;
                    for (k, p) in at.iter().enumerate() {
                        let (x, y) = (f64::from(p[0]), f64::from(p[1]));
                        let color = self.paint(*paint, k as f64 / last);
                        self.patch(view, [x - 0.5, y - 0.5, x + 0.5, y + 0.5], color, on);
                    }
                }
                Item::Boxes { boxes, paint } => {
                    let last = boxes.len().saturating_sub(1).max(1) as f64;
                    for (k, b) in boxes.iter().enumerate() {
                        self.patch(view, *b, self.paint(*paint, k as f64 / last), on);
                    }
                }
                Item::Field(field) => {
                    let Some([left, top, right, bottom]) = on.area() else {
                        continue;
                    };
                    if !view.level() {
                        continue;
                    }
                    for j in top..bottom {
                        if let Some(ahead) = &self.drawing.ahead {
                            let row: Vec<(f64, f64)> = (left..right)
                                .map(|i| view.back(i as f64 + 0.5, j as f64 + 0.5))
                                .collect();
                            ahead(&row);
                        }
                        for i in left..right {
                            let (x, y) = view.back(i as f64 + 0.5, j as f64 + 0.5);
                            let color = match field(x, y) {
                                Some(Shade::Level(v)) => along(&self.palette, v),
                                Some(Shade::Tint(c, _)) => c,
                                None => continue,
                            };
                            on.pixel(i as usize, j as usize, color);
                        }
                    }
                }
                Item::Mesh {
                    columns,
                    points,
                    wires,
                } => self.mesh(view, *columns, points, *wires, on),
                Item::Body(body) => self.body(view, body, on),
                Item::Label { at, text } => {
                    if let Some(p) = view.place(*at) {
                        let size = self.letters;
                        let wide = (8 * size * text.chars().count()) as f64;
                        on.text(
                            [p[0] - wide / 2.0, p[1] - 4.0 * size as f64],
                            size,
                            text,
                            self.ink,
                        );
                    }
                }
            }
        }
        on.clip(None);
        if self.axes {
            if self.framed {
                self.frame(view, on);
            } else {
                self.cage(view, on);
            }
        }
        self.words(on);
        if chart && self.drawing.names.len() > 1 && on.area().is_some() {
            let size = self.letters;
            let step = 10.0 * size as f64;
            let (x, mut y) = (self.plot[0] + step, self.plot[1] + step / 2.0);
            // the names stand on a ground of their own, before the curves
            let names = &self.drawing.names;
            let longest = names.iter().map(|n| n.chars().count()).max().unwrap_or(0) as f64;
            on.rect(
                [x - step / 2.0, y - step / 4.0],
                [
                    x + 3.0 * step + 8.0 * size as f64 * longest,
                    y + step * names.len() as f64,
                ],
                -1.0,
                self.background,
            );
            for (k, name) in names.iter().enumerate() {
                let color = self.paint(Paint::Curve(k), 0.0);
                let middle = y + 4.0 * size as f64;
                on.segment(
                    [x, middle, -2.0],
                    [x + 2.0 * step, middle, -2.0],
                    self.look.thickness,
                    color,
                    color,
                );
                on.text([x + 2.5 * step, y], size, name, self.ink);
                y += step;
            }
        }
    }

    /// A box of the plane.
    fn patch<S: Surface>(&self, view: &View, b: [f64; 4], color: Rgb, on: &mut S) {
        let corner = |x: f64, y: f64| view.place([x, y, 0.0]);
        let (Some(p), Some(q), Some(r), Some(s)) = (
            corner(b[0], b[1]),
            corner(b[2], b[1]),
            corner(b[2], b[3]),
            corner(b[0], b[3]),
        ) else {
            return;
        };
        if view.level() {
            on.rect([p[0], p[1]], [r[0], r[1]], p[2], self.faded(color, p[2]));
        } else {
            let colors = [p, q, r, s].map(|c| self.faded(color, c[2]));
            on.triangle([p, q, r], [colors[0], colors[1], colors[2]]);
            on.triangle([p, r, s], [colors[0], colors[2], colors[3]]);
        }
    }

    /// A surface: its cells in the colours of their heights, lighter where they face the
    /// light, and lines along it.
    fn mesh<S: Surface>(
        &self,
        view: &View,
        columns: usize,
        points: &[Option<Point>],
        wires: usize,
        on: &mut S,
    ) {
        if columns < 2 {
            return;
        }
        let rows = points.len() / columns;
        let heights = points.iter().flatten().map(|p| p[2]);
        let low = heights.clone().fold(f64::INFINITY, f64::min);
        let high = heights.fold(f64::NEG_INFINITY, f64::max);
        let level = |p: &Point| {
            if high > low {
                (p[2] - low) / (high - low)
            } else {
                0.5
            }
        };
        let light = unit([-0.4, 0.5, 0.75]).unwrap_or([0.0, 0.0, 1.0]);
        let at = |i: usize, j: usize| points[j * columns + i];
        for j in 0..rows.saturating_sub(1) {
            for i in 0..columns - 1 {
                let (Some(a), Some(b), Some(c), Some(d)) =
                    (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1))
                else {
                    continue;
                };
                let (Some(sa), Some(sb), Some(sd)) = (view.seen(a), view.seen(b), view.seen(d))
                else {
                    continue;
                };
                let across = [sb[0] - sa[0], sb[1] - sa[1], sb[2] - sa[2]];
                let along_it = [sd[0] - sa[0], sd[1] - sa[1], sd[2] - sa[2]];
                let facing = unit(cross(across, along_it)).map_or(1.0, |n| dot(n, light).abs());
                let bright = (0.35 + 0.65 * facing) as f32;
                let (Some(pa), Some(pb), Some(pc), Some(pd)) =
                    (view.place(a), view.place(b), view.place(c), view.place(d))
                else {
                    continue;
                };
                let color = |p: &Point, placed: &Place| {
                    let c = self.paint(Paint::Along, level(p)).map(|v| v * bright);
                    self.faded(c, placed[2])
                };
                let colors = [
                    color(&a, &pa),
                    color(&b, &pb),
                    color(&c, &pc),
                    color(&d, &pd),
                ];
                on.triangle([pa, pb, pc], [colors[0], colors[1], colors[2]]);
                on.triangle([pa, pc, pd], [colors[0], colors[2], colors[3]]);
            }
        }
        // the lines lie a little before the surface
        let wires = wires.max(1);
        let half = self.look.thickness / 4.0;
        let mut wire = |a: Option<Point>, b: Option<Point>| {
            let (Some(a), Some(b)) = (a, b) else { return };
            let (Some(mut pa), Some(mut pb)) = (view.place(a), view.place(b)) else {
                return;
            };
            let color = |p: &Point, placed: &Place| {
                let c = mix(self.paint(Paint::Along, level(p)), [0.0; 3], 0.55);
                self.faded(c, placed[2])
            };
            let (ca, cb) = (color(&a, &pa), color(&b, &pb));
            pa[2] -= 1.5;
            pb[2] -= 1.5;
            on.segment(pa, pb, half, ca, cb);
        };
        for j in 0..rows {
            for i in 0..columns {
                if j % wires == 0 && i + 1 < columns {
                    wire(at(i, j), at(i + 1, j));
                }
                if i % wires == 0 && j + 1 < rows {
                    wire(at(i, j), at(i, j + 1));
                }
                // (the far rims close the surface)
                if j == rows - 1 && j % wires != 0 && i + 1 < columns {
                    wire(at(i, j), at(i + 1, j));
                }
                if i == columns - 1 && i % wires != 0 && j + 1 < rows {
                    wire(at(i, j), at(i, j + 1));
                }
            }
        }
    }

    /// The numbers along the sides of a chart, where they stand.
    fn marks(&self, view: &View) -> (Marks, Marks) {
        let [left, top, right, bottom] = self.plot;
        let (x0, y1) = view.back(left, top);
        let (x1, y0) = view.back(right, bottom);
        let across = ((right - left) / (70.0 * self.letters as f64)).clamp(2.0, 12.0) as usize;
        let down = ((bottom - top) / (40.0 * self.letters as f64)).clamp(2.0, 12.0) as usize;
        let xs = marks_of(x0, x1, across, self.drawing.logs[0])
            .into_iter()
            .filter_map(|v| view.place([v, y0, 0.0]).map(|p| (v, p[0])))
            .collect();
        let ys = marks_of(y0, y1, down, self.drawing.logs[1])
            .into_iter()
            .filter_map(|v| view.place([x0, v, 0.0]).map(|p| (v, p[1])))
            .collect();
        (xs, ys)
    }

    /// Faint lines at the numbers of a chart.
    fn rules<S: Surface>(&self, view: &View, on: &mut S) {
        let [left, top, right, bottom] = self.plot;
        let faint = mix(self.background, self.ink, 0.18);
        let (xs, ys) = self.marks(view);
        for (_, x) in xs {
            let x = crisp(x, 1);
            on.segment([x, top, 0.0], [x, bottom, 0.0], 0.5, faint, faint);
        }
        for (_, y) in ys {
            let y = crisp(y, 1);
            on.segment([left, y, 0.0], [right, y, 0.0], 0.5, faint, faint);
        }
    }

    /// The title above a picture; the names of the axes of a chart: of y over its upper left
    /// corner, of x under its numbers.
    fn words<S: Surface>(&self, on: &mut S) {
        let size = self.letters;
        let letter = 8.0 * size as f64;
        let wide = |text: &str| letter * text.chars().count() as f64;
        if !self.framed {
            if let Some(text) = &self.look.title {
                let middle = self.look.width as f64 / 2.0;
                on.text([middle - wide(text) / 2.0, letter], size, text, self.ink);
            }
            return;
        }
        let [left, top, right, bottom] = self.plot;
        let middle = (left + right) / 2.0;
        let mut line = top - 1.5 * letter;
        if let Some(text) = &self.look.ylabel {
            on.text([left, line], size, text, self.ink);
            line -= 1.5 * letter;
        }
        if let Some(text) = &self.look.title {
            on.text([middle - wide(text) / 2.0, line], size, text, self.ink);
        }
        if let Some(text) = &self.look.xlabel {
            let under = if self.axes {
                3.0 * letter
            } else {
                0.25 * letter
            };
            on.text(
                [middle - wide(text) / 2.0, bottom + under],
                size,
                text,
                self.ink,
            );
        }
    }

    /// The box of a chart with its numbers.
    fn frame<S: Surface>(&self, view: &View, on: &mut S) {
        let [left, top, right, bottom] = self.plot;
        let size = self.letters;
        let letter = 8.0 * size as f64;
        let half = 0.5 * size as f64;
        let ink = self.ink;
        let corners = [[left, top], [right, top], [right, bottom], [left, bottom]];
        for k in 0..4 {
            let (a, b) = (corners[k], corners[(k + 1) % 4]);
            on.segment([a[0], a[1], 0.0], [b[0], b[1], 0.0], half, ink, ink);
        }
        let (xs, ys) = self.marks(view);
        let tick = letter / 2.0;
        for (v, x) in xs {
            let x = crisp(x, size);
            on.segment([x, bottom, 0.0], [x, bottom + tick, 0.0], half, ink, ink);
            let text = mark(v, self.drawing.logs[0]);
            let wide = letter * text.len() as f64;
            on.text(
                [x - wide / 2.0, bottom + tick + letter / 2.0],
                size,
                &text,
                ink,
            );
        }
        for (v, y) in ys {
            let y = crisp(y, size);
            on.segment([left - tick, y, 0.0], [left, y, 0.0], half, ink, ink);
            let text = mark(v, self.drawing.logs[1]);
            let wide = letter * text.len() as f64;
            on.text(
                [left - tick - letter / 2.0 - wide, y - letter / 2.0],
                size,
                &text,
                ink,
            );
        }
    }

    /// The box around a drawing in space, with the ends of its ranges.
    fn cage<S: Surface>(&self, view: &View, on: &mut S) {
        let ends = match (self.drawing.stretch, self.drawing.extent()) {
            (Some(ranges), _) => ranges,
            (None, Some(ends)) => ends,
            (None, None) => return,
        };
        let corner = |k: usize| -> Point {
            [0, 1, 2].map(|axis| {
                if k >> axis & 1 == 1 {
                    ends[axis].1
                } else {
                    ends[axis].0
                }
            })
        };
        let ink = mix(self.background, self.ink, 0.7);
        let half = self.look.thickness / 4.0 + 0.25;
        for a in 0..8usize {
            for axis in 0..3 {
                let b = a | 1 << axis;
                if a == b {
                    continue;
                }
                if let (Some(p), Some(q)) = (view.place(corner(a)), view.place(corner(b))) {
                    on.segment(p, q, half, self.faded(ink, p[2]), self.faded(ink, q[2]));
                }
            }
        }
        // the name of each axis at the upper end of an edge of the box (the lowest edge in
        // the picture; for the upright axis the one most to the left), the ranges in a corner
        let size = self.letters;
        let letter = 8.0 * size as f64;
        let Some(middle) = view.place([0, 1, 2].map(|k| (ends[k].0 + ends[k].1) / 2.0)) else {
            return;
        };
        let (w, h) = (self.look.width as f64, self.look.height as f64);
        let mut line = if self.drawing.axis_names.is_some() && self.look.title.is_some() {
            1.2
        } else {
            0.0
        };
        for axis in 0..3usize {
            let span = ends[axis].1 - ends[axis].0;
            if span.is_nan() || span <= 0.0 {
                continue;
            }
            let name = match &self.drawing.axis_names {
                Some(names) => names[axis].as_str(),
                None => ["x", "y", "z"][axis],
            };
            let text = format!(
                "{name} from {} to {}",
                plot::label(ends[axis].0),
                plot::label(ends[axis].1)
            );
            on.text(
                [letter / 2.0, letter / 2.0 + line * 1.25 * letter],
                size,
                &text,
                self.ink,
            );
            line += 1.0;
            let mut best: Option<(f64, Place)> = None;
            for a in 0..8usize {
                if a >> axis & 1 == 1 {
                    continue;
                }
                let (Some(p), Some(q)) = (view.place(corner(a)), view.place(corner(a | 1 << axis)))
                else {
                    continue;
                };
                let rank = if axis == 2 {
                    -(p[0] + q[0])
                } else {
                    p[1] + q[1]
                };
                if best.as_ref().is_none_or(|b| rank > b.0) {
                    // (names of several letters stand at the middles of their edges, where
                    // they do not meet)
                    best = Some(match self.drawing.axis_names {
                        Some(_) => (rank, [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0, q[2]]),
                        None => (rank, q),
                    });
                }
            }
            let Some((_, end)) = best else { continue };
            // away from the middle of the box
            let away = [end[0] - middle[0], end[1] - middle[1]];
            let length = (away[0] * away[0] + away[1] * away[1]).sqrt();
            let away = if length > 0.0 {
                [away[0] / length, away[1] / length]
            } else {
                [0.0, 1.0]
            };
            let wide = letter * name.chars().count() as f64;
            let x = (end[0] + away[0] * (0.7 * letter + wide / 2.0) - wide / 2.0)
                .clamp(0.0, (w - wide).max(0.0));
            let y =
                (end[1] + away[1] * 1.2 * letter - letter / 2.0).clamp(0.0, (h - letter).max(0.0));
            on.text([x, y], size, name, self.ink);
        }
        // the colours of a fourth number in a row after its ends, the least value first
        if let Some(legend) = &self.drawing.legend {
            let text = format!(
                "{} from {} to {}",
                legend.name,
                plot::label(legend.ends.0),
                plot::label(legend.ends.1)
            );
            let top = letter / 2.0 + line * 1.25 * letter;
            on.text([letter / 2.0, top], size, &text, self.ink);
            let left = letter / 2.0 + letter * (text.chars().count() + 1) as f64;
            let room = w - left - letter / 2.0;
            let count = if room > 12.0 * letter {
                (12.0 * letter) as usize
            } else if room >= 1.0 {
                room as usize
            } else {
                0
            };
            for k in 0..count {
                let color = along(&legend.colors, k as f64 / (count.max(2) - 1) as f64);
                let x = left + k as f64;
                on.rect([x, top], [x + 1.0, top + letter], -1e9, color);
            }
        }
    }

    /// A body of small cubes: the sides that lie open toward the eye, each in the colour of
    /// its cube, lighter where the surface faces the light.
    fn body<S: Surface>(&self, view: &View, body: &Body, on: &mut S) {
        let step =
            [0, 1, 2].map(|k| (body.ends[k].1 - body.ends[k].0) / body.size[k].max(1) as f64);
        // how long a cube is along each side in the space of the picture
        let grow = [0, 1, 2].map(|k| step[k] * view.factor[k]);
        let light = unit([-0.4, 0.5, 0.75]).unwrap_or([0.0, 0.0, 1.0]);
        let eye = view
            .eye
            .map(|d| [0, 1, 2].map(|k| view.center[k] + view.toward[k] * d));
        let point = |at: [f64; 3]| [0, 1, 2].map(|k| body.ends[k].0 + step[k] * at[k]);
        for face in &body.faces {
            let axis = usize::from(face.side / 2).min(2);
            let upper = face.side % 2 == 1;
            let sign = if upper { 1.0 } else { -1.0 };
            let low = face.at.map(f64::from);
            let plane = low[axis] + if upper { 1.0 } else { 0.0 };
            // a side that faces away from the eye is not seen
            let seen = match eye {
                None => view.toward[axis] * grow[axis] * sign > 0.0,
                Some(eye) => {
                    let at = (body.ends[axis].0 + step[axis] * plane - view.origin[axis])
                        * view.factor[axis];
                    (eye[axis] - at) * grow[axis] * sign > 0.0
                }
            };
            if !seen {
                continue;
            }
            let (b, c) = ((axis + 1) % 3, (axis + 2) % 3);
            let corner = |db: f64, dc: f64| {
                let mut at = low;
                at[axis] = plane;
                at[b] += db;
                at[c] += dc;
                view.place(point(at))
            };
            let (Some(p), Some(q), Some(r), Some(s)) = (
                corner(0.0, 0.0),
                corner(1.0, 0.0),
                corner(1.0, 1.0),
                corner(0.0, 1.0),
            ) else {
                continue;
            };
            // the direction that the surface faces, as the picture sees it
            let facing = match face.facing {
                [0, 0, 0] => {
                    let mut d = [0.0; 3];
                    d[axis] = sign;
                    d
                }
                given => given.map(f64::from),
            };
            let across = [0, 1, 2].map(|k| {
                if grow[k] != 0.0 {
                    facing[k] / grow[k]
                } else {
                    0.0
                }
            });
            let turned = [
                dot(across, view.right),
                dot(across, view.up),
                dot(across, view.toward),
            ];
            let lit = unit(turned).map_or(1.0, |n| {
                // (the side that is seen is the one toward the eye)
                let n = if n[2] < 0.0 { n.map(|v| -v) } else { n };
                let facing = dot(n, light);
                if facing > 0.0 { facing } else { 0.0 }
            });
            let bright = (0.35 + 0.65 * lit) as f32;
            let base = self.look.color.unwrap_or_else(|| {
                body.tints
                    .get(usize::from(face.level))
                    .copied()
                    .unwrap_or([1.0; 3])
            });
            let color = self.faded(base.map(|v| v * bright), p[2]);
            on.triangle([p, q, r], [color; 3]);
            on.triangle([p, r, s], [color; 3]);
        }
    }
}

/// The place nearest to `v` where a line that is `width` pixels wide covers whole pixels.
fn crisp(v: f64, width: usize) -> f64 {
    if width % 2 == 1 {
        v.floor() + 0.5
    } else {
        v.round()
    }
}

/// The numbers along an axis; of logarithms the whole ones, where there are two or more.
fn marks_of(low: f64, high: f64, parts: usize, log: bool) -> Vec<f64> {
    let all = ticks(low, high, parts);
    let whole: Vec<f64> = all.iter().copied().filter(|v| v.fract() == 0.0).collect();
    if log && whole.len() >= 2 { whole } else { all }
}

/// The number at a mark of an axis; on an axis of logarithms the power of ten.
pub(crate) fn mark(v: f64, log: bool) -> String {
    if !log {
        return plot::label(v);
    }
    if v.fract() != 0.0 || v.abs() > 300.0 {
        return plot::label(Float::with_val(64, v).exp10().to_f64());
    }
    match v as i32 {
        k @ 0..=5 => format!("1{}", "0".repeat(k as usize)),
        k @ -4..=-1 => format!("0.{}1", "0".repeat((-k - 1) as usize)),
        k => format!("1e{k}"),
    }
}

/// The view of a chart: its ranges fill a box of the picture (left, top, right, bottom) that
/// leaves room for the numbers of the axes, whose letters are 8 pixels high times the size in
/// `letters`, which also tells whether the y axis holds logarithms.
fn chart_box(
    view: &mut View,
    (w, h): (f64, f64),
    x: (f64, f64),
    y: (f64, f64),
    equal: bool,
    pad: f64,
    letters: Option<(usize, bool)>,
) -> [f64; 4] {
    let (mut x, mut y) = (x, y);
    let mut plot = [pad, pad, w - pad, h - pad];
    let mut scale = [1.0; 2];
    // twice: the numbers of the axes depend on the ranges, and the ranges of a chart with
    // one scale on the room that the numbers leave
    for _ in 0..2 {
        if let Some((size, log_y)) = letters {
            let letter = 8.0 * size as f64;
            let widest = marks_of(y.0, y.1, 6, log_y)
                .iter()
                .map(|v| mark(*v, log_y).len())
                .max()
                .unwrap_or(1) as f64;
            plot = [
                pad + (widest + 1.5) * letter,
                pad + letter,
                w - pad - 3.0 * letter,
                h - pad - 2.5 * letter,
            ];
        }
        if plot[2] - plot[0] < 8.0 || plot[3] - plot[1] < 8.0 {
            plot = [0.0, 0.0, w, h];
        } else if let Some((size, _)) = letters {
            // the lines of the frame cover whole pixels
            plot = plot.map(|v| crisp(v, size));
        }
        let (across, down) = (plot[2] - plot[0], plot[3] - plot[1]);
        scale = [across / (x.1 - x.0), down / (y.1 - y.0)];
        if !equal {
            break;
        }
        let one = if scale[0] < scale[1] {
            scale[0]
        } else {
            scale[1]
        };
        scale = [one, one];
        let (mx, my) = ((x.0 + x.1) / 2.0, (y.0 + y.1) / 2.0);
        x = (mx - across / one / 2.0, mx + across / one / 2.0);
        y = (my - down / one / 2.0, my + down / one / 2.0);
    }
    view.scale = scale;
    view.shift = [plot[0] - x.0 * scale[0], plot[3] + y.0 * scale[1]];
    plot
}

/// The part of a surface that things are drawn in.
pub trait Area {
    /// Only this box is drawn in from now on: left, top, right, bottom; or all again.
    fn clip(&mut self, plot: Option<[f64; 4]>);
    /// The pixels that a field fills, when the surface has pixels of colours.
    fn area(&self) -> Option<[i64; 4]>;
}

impl Area for Raster {
    fn clip(&mut self, plot: Option<[f64; 4]>) {
        match plot {
            Some([left, top, right, bottom]) => {
                self.clip = [
                    (left.ceil() as i64).max(0),
                    (top.ceil() as i64).max(0),
                    (right.floor() as i64).min(self.width as i64),
                    (bottom.floor() as i64).min(self.height as i64),
                ];
            }
            None => self.unclip(),
        }
    }

    fn area(&self) -> Option<[i64; 4]> {
        Some(self.clip)
    }
}

/// A picture of dots: Braille characters, or marks with `ascii`.
struct Dotted {
    canvas: Canvas,
}

impl Dotted {
    fn dot(&mut self, x: f64, y: f64) {
        if x.is_finite() && y.is_finite() && x.abs() < 1e9 && y.abs() < 1e9 {
            self.canvas.set(x.round() as i64, y.round() as i64, '*');
        }
    }
}

impl Area for Dotted {
    fn clip(&mut self, _: Option<[f64; 4]>) {}

    fn area(&self) -> Option<[i64; 4]> {
        None
    }
}

impl Surface for Dotted {
    fn segment(&mut self, a: Place, b: Place, _: f64, _: Rgb, _: Rgb) {
        let (dw, dh) = self.canvas.dots();
        let limit = 4.0 * (dw + dh) as f64;
        if ![a[0], a[1], b[0], b[1]].iter().all(|v| v.is_finite()) {
            return;
        }
        let clamp = |v: f64| v.clamp(-limit, limit);
        let (a, b) = ([clamp(a[0]), clamp(a[1])], [clamp(b[0]), clamp(b[1])]);
        let (across, down) = ((b[0] - a[0]).abs(), (b[1] - a[1]).abs());
        let steps = if across > down { across } else { down }.ceil().max(1.0);
        for k in 0..=steps as i64 {
            let t = k as f64 / steps;
            self.dot(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t);
        }
    }

    fn disc(&mut self, at: Place, radius: f64, _: Rgb) {
        self.dot(at[0], at[1]);
        if radius >= 1.5 && !self.canvas.ascii {
            for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                self.dot(at[0] + dx, at[1] + dy);
            }
        }
    }

    fn rect(&mut self, from: [f64; 2], to: [f64; 2], _: f64, _: Rgb) {
        let (dw, dh) = self.canvas.dots();
        // the dots whose middles lie in the box, or the one nearest to a box between them
        let ordered = |a: f64, b: f64, most: i64| {
            let (low, high) = if a < b { (a, b) } else { (b, a) };
            let (first, last) = (low.ceil(), high.floor());
            let (first, last) = if first > last {
                let middle = ((low + high) / 2.0).round();
                (middle, middle)
            } else {
                (first, last)
            };
            (first.max(0.0) as i64, last.min((most - 1) as f64) as i64)
        };
        if ![from[0], from[1], to[0], to[1]]
            .iter()
            .all(|v| v.is_finite())
        {
            return;
        }
        let (left, right) = ordered(from[0], to[0], dw);
        let (top, bottom) = ordered(from[1], to[1], dh);
        for j in top..=bottom {
            for i in left..=right {
                self.canvas.set(i, j, '*');
            }
        }
    }

    fn triangle(&mut self, _: [Place; 3], _: [Rgb; 3]) {}

    fn text(&mut self, _: [f64; 2], _: usize, _: &str, _: Rgb) {}

    fn pixel(&mut self, _: usize, _: usize, _: Rgb) {}
}

/// The marks of a field from its darkest to its lightest places: Braille cells with more
/// and more dots, or characters.
const DOTS: [u32; 9] = [0x00, 0x04, 0x24, 0x34, 0x36, 0x76, 0xf6, 0xfe, 0xff];
const MARKS: &[u8] = b" .:-=+*#%@";

/// The drawing as lines of characters, `width` by `height` of them.
pub fn text(
    drawing: &Drawing,
    width: usize,
    height: usize,
    ascii: bool,
) -> Result<Vec<String>, String> {
    let options = plot::Options {
        width,
        height,
        sized: true,
        ascii,
        range: None,
    };
    let (per_x, per_y) = if ascii { (1, 1) } else { (2, 4) };
    let look = |w: usize, h: usize| Look {
        width: w,
        height: h,
        margin: 0.0,
        thickness: 1.0,
        axes: Some(false),
        ..Look::default()
    };
    match drawing.space {
        Space::Chart { x, y, equal } => {
            let (x, y) = if equal {
                plot::square(&options, x, y)
            } else {
                (x, y)
            };
            let mut canvas = Canvas::new(&options, x, y);
            canvas.log_y = drawing.logs[1];
            let field = drawing.items.iter().find_map(|item| match item {
                Item::Field(f) => Some(f),
                _ => None,
            });
            if let Some(field) = field {
                canvas.axes = false;
                for r in 0..height {
                    let py = y.1 - (y.1 - y.0) * (r as f64 + 0.5) / height as f64;
                    if let Some(ahead) = &drawing.ahead {
                        let row: Vec<(f64, f64)> = (0..width)
                            .map(|c| (x.0 + (x.1 - x.0) * (c as f64 + 0.5) / width as f64, py))
                            .collect();
                        ahead(&row);
                    }
                    for c in 0..width {
                        let px = x.0 + (x.1 - x.0) * (c as f64 + 0.5) / width as f64;
                        let light = match field(px, py) {
                            Some(Shade::Level(v)) => v.clamp(0.0, 1.0),
                            Some(Shade::Tint(_, light)) => light.clamp(0.0, 1.0),
                            None => continue,
                        };
                        canvas.cells[r * width + c] = if ascii {
                            let k = (light * (MARKS.len() - 1) as f64 + 0.5) as usize;
                            u32::from(MARKS[k.min(MARKS.len() - 1)])
                        } else {
                            let k = (light * (DOTS.len() - 1) as f64 + 0.5) as usize;
                            DOTS[k.min(DOTS.len() - 1)]
                        };
                    }
                }
                return Ok(canvas.render(&plot::label(x.0), &plot::label(x.1)));
            }
            let (dw, dh) = canvas.dots();
            let mut shot = Shot::new(drawing, &look(dw as usize, dh as usize))?;
            // the first and the last dot are the ends of the ranges
            shot.view.scale = [(dw - 1) as f64 / (x.1 - x.0), (dh - 1) as f64 / (y.1 - y.0)];
            shot.view.shift = [
                -x.0 * shot.view.scale[0],
                (dh - 1) as f64 + y.0 * shot.view.scale[1],
            ];
            let mut dotted = Dotted { canvas };
            shot.draw(&shot.view, 1.0, &mut dotted);
            Ok(dotted
                .canvas
                .render(&mark(x.0, drawing.logs[0]), &mark(x.1, drawing.logs[0])))
        }
        Space::Flat => {
            let ends = drawing.extent().ok_or("there is nothing to draw")?;
            let pad = |(low, high): (f64, f64)| {
                if high > low {
                    (low, high)
                } else {
                    (low - 1.0, high + 1.0)
                }
            };
            let (x, y) = plot::square(&options, pad(ends[0]), pad(ends[1]));
            let canvas = Canvas::new(&options, x, y);
            let (dw, dh) = canvas.dots();
            let mut shot = Shot::new(drawing, &look(dw as usize, dh as usize))?;
            shot.view.center = [0.0; 3];
            shot.view.scale = [(dw - 1) as f64 / (x.1 - x.0), (dh - 1) as f64 / (y.1 - y.0)];
            shot.view.shift = [
                -x.0 * shot.view.scale[0],
                (dh - 1) as f64 + y.0 * shot.view.scale[1],
            ];
            let mut dotted = Dotted { canvas };
            shot.draw(&shot.view, 1.0, &mut dotted);
            Ok(dotted.canvas.render(&plot::label(x.0), &plot::label(x.1)))
        }
        Space::Solid => {
            // a dot of an ascii picture is twice as high as wide
            let (dw, dh) = (width * per_x, height * per_y * if ascii { 2 } else { 1 });
            let mut shot = Shot::new(drawing, &look(dw, dh))?;
            if ascii {
                shot.view.scale[1] /= 2.0;
                shot.view.shift[1] /= 2.0;
            }
            let canvas = Canvas::new(&options, (0.0, 1.0), (0.0, 1.0));
            let mut dotted = Dotted { canvas };
            shot.draw(&shot.view, 1.0, &mut dotted);
            Ok(dotted.canvas.bare())
        }
    }
}

/// A body of space as lines of characters, `width` by `height` of them, seen as `look`
/// says: more dots (or heavier marks with `ascii`) where it is lighter.
pub fn lights(
    drawing: &Drawing,
    look: &Look,
    width: usize,
    height: usize,
    ascii: bool,
) -> Result<Vec<String>, String> {
    let look = Look {
        width: 2 * width,
        height: 4 * height,
        margin: 0.0,
        thickness: 1.0,
        axes: Some(false),
        color: Some([1.0; 3]),
        background: Some([0.0; 3]),
        palette: None,
        grid: false,
        frames: 1,
        turn: None,
        grow: false,
        slices: 0,
        views: false,
        stereo: None,
        title: None,
        xlabel: None,
        ylabel: None,
        ..look.clone()
    };
    let shot = Shot::new(drawing, &look)?;
    let raster = shot.raster(0);
    let options = plot::Options {
        width,
        height,
        sized: true,
        ascii,
        range: None,
    };
    let mut canvas = Canvas::new(&options, (0.0, 1.0), (0.0, 1.0));
    let marks = if ascii { MARKS.len() } else { DOTS.len() };
    for r in 0..height {
        for c in 0..width {
            let (mut light, mut covered) = (0.0f32, false);
            for j in 0..4 {
                for i in 0..2 {
                    let (x, y) = (2 * c + i, 4 * r + j);
                    let cover = raster.cover(x, y).clamp(0.0, 1.0);
                    if cover > 0.0 {
                        covered = true;
                        light += cover * luma(raster.color(x, y));
                    }
                }
            }
            if !covered {
                continue;
            }
            let light = f64::from(light / 8.0).clamp(0.0, 1.0);
            let k = ((light * (marks - 1) as f64 + 0.5) as usize).clamp(1, marks - 1);
            canvas.cells[r * width + c] = if ascii { u32::from(MARKS[k]) } else { DOTS[k] };
        }
    }
    Ok(canvas.bare())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_walk() -> Drawing {
        // four steps around a square, then two up
        Drawing::new(
            Space::Solid,
            vec![Item::Trail {
                dirs: vec![
                    [1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [-1.0, 0.0, 0.0],
                    [0.0, -1.0, 0.0],
                    [0.0, 0.0, 1.0],
                ],
                steps: vec![0, 1, 2, 3, 4],
                runs: vec![4, 4, 4, 4, 2],
            }],
        )
    }

    #[test]
    fn sines_and_cosines() {
        assert_eq!(turn((0, 7)), (0.0, 1.0));
        assert_eq!(turn((1, 4)), (1.0, 0.0));
        assert_eq!(turn((5, 10)), (0.0, -1.0));
        assert_eq!(turn((9, 12)), (-1.0, 0.0));
        assert_eq!(turn((13, 12)), turn((1, 12)));
        let (s, c) = turn((1, 12));
        assert!((s - 0.5).abs() < 1e-15 && (c - 0.75f64.sqrt()).abs() < 1e-15);
        let (s, c) = turn((1, 8));
        assert_eq!(s, c);
        assert_eq!(sincos(90.0), (1.0, 0.0));
        assert_eq!(sincos(-90.0), (-1.0, 0.0));
        assert_eq!(sincos(720.0), (0.0, 1.0));
        let (s, c) = sincos(30.0);
        assert!((s - 0.5).abs() < 1e-15 && (c - 0.75f64.sqrt()).abs() < 1e-15);
    }

    #[test]
    fn round_numbers_of_axes() {
        assert_eq!(ticks(0.0, 10.0, 5), [0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
        assert_eq!(ticks(-1.0, 1.0, 4), [-1.0, -0.5, 0.0, 0.5, 1.0]);
        assert_eq!(ticks(0.0, 1.0, 10).len(), 11);
        assert_eq!(ticks(3.0, 3.0, 5), Vec::<f64>::new());
        assert_eq!(ticks(0.0, f64::INFINITY, 5), Vec::<f64>::new());
        let big = ticks(-2.5e7, 7.5e7, 5);
        assert_eq!(big, [-2e7, 0.0, 2e7, 4e7, 6e7]);
        let small = ticks(0.001, 0.0045, 4);
        let labels: Vec<String> = small.iter().map(|v| plot::label(*v)).collect();
        assert_eq!(labels, ["0.001", "0.002", "0.003", "0.004"]);
        for (low, high, parts) in [
            (0.0, 6.5, 8),
            (-17.0, 2301.0, 6),
            (1e-9, 3e-9, 3),
            (-0.3, 0.3, 7),
        ] {
            let t = ticks(low, high, parts);
            assert!(
                t.len() >= 2 && t.len() <= 2 * parts + 2,
                "{low} {high}: {t:?}"
            );
            assert!(t.iter().all(|v| *v >= low && *v <= high));
        }
    }

    #[test]
    fn the_plane_from_above() {
        let drawing = Drawing::new(
            Space::Flat,
            vec![Item::Line {
                points: vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 5.0, 0.0]],
                paint: Paint::Ink,
                weight: 1.0,
            }],
        );
        let look = Look {
            width: 120,
            height: 100,
            margin: 0.1,
            thickness: 1.0,
            ..Look::default()
        };
        let shot = Shot::new(&drawing, &look).expect("a view");
        // 10 pixels and the width of a line are free on every side: 98 pixels for 10 units
        let at = |p: Point| shot.view.place(p).expect("a place");
        let near = |p: Place, x: f64, y: f64| {
            (p[0] - x).abs() < 1e-9 && (p[1] - y).abs() < 1e-9 && p[2] == 0.0
        };
        assert!(near(at([0.0, 0.0, 0.0]), 11.0, 74.5));
        assert!(near(at([10.0, 5.0, 0.0]), 109.0, 25.5));
        assert!(shot.view.level());
        let (x, y) = shot.view.back(11.0, 74.5);
        assert!(x.abs() < 1e-9 && y.abs() < 1e-9);
        assert_eq!(shot.frames(), 1);
        // the same picture seen from below the plane is its mirror
        let below = Shot::new(
            &drawing,
            &Look {
                elevation: Some(-90.0),
                ..look.clone()
            },
        )
        .expect("a view");
        assert!(near(
            below.view.place([10.0, 5.0, 0.0]).expect("a place"),
            109.0,
            74.5
        ));
        // turned by a quarter, the long side stands upright
        let turned = Shot::new(
            &drawing,
            &Look {
                roll: 90.0,
                ..look.clone()
            },
        )
        .expect("a view");
        let (a, b) = (
            turned.view.place([0.0, 0.0, 0.0]).expect("a place"),
            turned.view.place([10.0, 0.0, 0.0]).expect("a place"),
        );
        assert!((a[0] - b[0]).abs() < 1e-9);
        assert!((a[1] - b[1]).abs() > 70.0);
        // a scale of the user: so many pixels for a unit, the middle in the middle
        let scaled = Shot::new(
            &drawing,
            &Look {
                scale: Some(3.0),
                ..look
            },
        )
        .expect("a view");
        assert!(near(
            scaled.view.place([5.0, 2.5, 0.0]).expect("a place"),
            60.0,
            50.0
        ));
        assert!(near(
            scaled.view.place([6.0, 2.5, 0.0]).expect("a place"),
            63.0,
            50.0
        ));
    }

    #[test]
    fn space_from_a_point() {
        let drawing = square_walk();
        let look = Look {
            width: 200,
            height: 200,
            thickness: 1.0,
            margin: 0.0,
            ..Look::default()
        };
        // from the front: x across, z up, and the nearer side of the square has the smaller depth
        let front = Shot::new(
            &drawing,
            &Look {
                azimuth: Some(0.0),
                elevation: Some(0.0),
                ..look.clone()
            },
        )
        .expect("a view");
        let at = |shot: &Shot, p: Point| shot.view.place(p).expect("a place");
        let (a, b, c) = (
            at(&front, [0.0, 0.0, 0.0]),
            at(&front, [4.0, 0.0, 0.0]),
            at(&front, [0.0, 0.0, 2.0]),
        );
        assert!(b[0] > a[0] && (b[1] - a[1]).abs() < 1e-9);
        assert!(c[1] < a[1] && (c[0] - a[0]).abs() < 1e-9);
        assert!(at(&front, [0.0, 4.0, 0.0])[2] > a[2]);
        // the picture is filled: the square is 4 wide and 2 high
        assert!((b[0] - a[0] - 198.0).abs() < 1e-9);
        // from the right side the y axis lies across
        let side = Shot::new(
            &drawing,
            &Look {
                azimuth: Some(90.0),
                elevation: Some(0.0),
                ..look.clone()
            },
        )
        .expect("a view");
        let (a, b) = (at(&side, [0.0, 0.0, 0.0]), at(&side, [0.0, 4.0, 0.0]));
        assert!(b[0] > a[0] + 100.0 && (b[1] - a[1]).abs() < 1e-9);
        // a viewpoint: the same direction as the angles, and near things are larger
        let far = Shot::new(
            &drawing,
            &Look {
                viewpoint: Some([2.0, -98.0, 1.0]),
                ..look.clone()
            },
        )
        .expect("a view");
        let near_side = at(&far, [4.0, 0.0, 0.0])[0] - at(&far, [0.0, 0.0, 0.0])[0];
        let far_side = at(&far, [4.0, 4.0, 0.0])[0] - at(&far, [0.0, 4.0, 0.0])[0];
        assert!(near_side > far_side && far_side > 0.9 * near_side);
        assert!(
            Shot::new(
                &drawing,
                &Look {
                    viewpoint: Some([2.0, 2.0, 1.0]),
                    ..look.clone()
                }
            )
            .is_err()
        );
        assert!(
            Shot::new(
                &drawing,
                &Look {
                    distance: Some(0.5),
                    ..look.clone()
                }
            )
            .is_err()
        );
        // what is behind the eye is not seen
        let inside = Shot::new(
            &drawing,
            &Look {
                viewpoint: Some([2.0, 1.0, 1.0]),
                ..look.clone()
            },
        )
        .expect("a view");
        assert_eq!(inside.view.place([2.0, -3.0, 1.0]), None);
        // frames of a turn: the same scale in all of them, a full turn comes back
        let spin = Shot::new(
            &drawing,
            &Look {
                frames: 4,
                ..look.clone()
            },
        )
        .expect("a view");
        assert_eq!(spin.frames(), 4);
        let quarter = spin.view.turned(90.0);
        let back = quarter.turned(270.0);
        for k in 0..3 {
            assert!((back.right[k] - spin.view.right[k]).abs() < 1e-12);
        }
        assert_eq!(quarter.scale, spin.view.scale);
        // slabs of the depth are frames
        let slices = Shot::new(
            &drawing,
            &Look {
                slices: 5,
                ..look.clone()
            },
        )
        .expect("a view");
        assert_eq!(slices.frames(), 5);
        let drawn = |r: &Raster| {
            (0..200)
                .flat_map(|y| (0..200).map(move |x| (x, y)))
                .filter(|(x, y)| r.cover(*x, *y) > 0.0)
                .count() as f64
        };
        let all: f64 = (0..5).map(|k| drawn(&slices.raster(k))).sum();
        let whole = Shot::new(&drawing, &look).expect("a view");
        let one = drawn(&whole.raster(0));
        assert!(all >= one && all < 1.2 * one, "{all} against {one}");
    }

    #[test]
    fn several_eyes() {
        let drawing = square_walk();
        let look = Look {
            width: 120,
            height: 120,
            thickness: 1.0,
            ..Look::default()
        };
        let same = |a: &Raster, b: &Raster| {
            (0..120).all(|y| {
                (0..120).all(|x| a.cover(x, y) == b.cover(x, y) && a.color(x, y) == b.color(x, y))
            })
        };
        // three views: from above, from the front and from the right, all with one scale
        let views = Shot::new(
            &drawing,
            &Look {
                views: true,
                ..look.clone()
            },
        )
        .expect("a view");
        assert_eq!(views.frames(), 3);
        let ends = drawing.extent().expect("points");
        let middle = ends.map(|(low, high)| (low + high) / 2.0);
        for (k, (azimuth, elevation)) in [(0.0, 90.0), (0.0, 0.0), (90.0, 0.0)]
            .into_iter()
            .enumerate()
        {
            let one = Shot::new(
                &drawing,
                &Look {
                    azimuth: Some(azimuth),
                    elevation: Some(elevation),
                    center: Some(middle),
                    ..look.clone()
                },
            )
            .expect("a view");
            assert!(same(&views.raster(k), &one.raster(0)), "view {k}");
        }
        assert!(
            !same(&views.raster(0), &views.raster(1)) && !same(&views.raster(1), &views.raster(2))
        );
        // two eyes: what is nearer than the middle stands further right for the left eye,
        // what is farther stands further left
        let lines = Drawing::new(
            Space::Solid,
            vec![
                Item::Line {
                    points: vec![[0.0, -5.0, -3.0], [0.0, -5.0, -1.0]],
                    paint: Paint::Ink,
                    weight: 1.0,
                },
                Item::Line {
                    points: vec![[0.0, 5.0, 1.0], [0.0, 5.0, 3.0]],
                    paint: Paint::Ink,
                    weight: 1.0,
                },
                Item::Line {
                    points: vec![[-4.0, 0.0, 0.0], [4.0, 0.0, 0.0]],
                    paint: Paint::Ink,
                    weight: 1.0,
                },
            ],
        );
        let eyes = Shot::new(
            &lines,
            &Look {
                azimuth: Some(0.0),
                elevation: Some(0.0),
                stereo: Some(4.0),
                ..look.clone()
            },
        )
        .expect("a view");
        let [left, right] = eyes.pair(0);
        // the middle of what is drawn in some rows, across
        let across = |r: &Raster, rows: std::ops::Range<usize>| {
            let (mut sum, mut weight) = (0.0, 0.0);
            for y in rows {
                for x in 0..120 {
                    sum += x as f64 * f64::from(r.cover(x, y));
                    weight += f64::from(r.cover(x, y));
                }
            }
            assert!(weight > 3.0);
            sum / weight
        };
        // (the far line is above the middle of the picture, the near one below)
        let (near_left, near_right) = (across(&left, 75..110), across(&right, 75..110));
        let (far_left, far_right) = (across(&left, 10..45), across(&right, 10..45));
        assert!(near_left > near_right + 2.0, "{near_left} {near_right}");
        assert!(far_left < far_right - 2.0, "{far_left} {far_right}");
        assert!(((near_left + near_right) / 2.0 - 59.5).abs() < 0.6);
        // one eye in the middle sees both lines above each other
        let one = Shot::new(
            &lines,
            &Look {
                azimuth: Some(0.0),
                elevation: Some(0.0),
                ..look.clone()
            },
        )
        .expect("a view");
        let both = one.raster(0);
        assert!((across(&both, 75..110) - across(&both, 10..45)).abs() < 1e-6);
        // only drawings of space have several eyes
        let flat = Drawing::new(Space::Flat, Vec::new());
        for wish in [
            Look {
                views: true,
                ..look.clone()
            },
            Look {
                stereo: Some(4.0),
                ..look.clone()
            },
        ] {
            assert_eq!(
                Shot::new(&flat, &wish).err(),
                Some(SEVERAL_EYES.to_string())
            );
        }
    }

    #[test]
    fn charts_have_no_view_from_a_point() {
        let chart = Drawing::new(
            Space::Chart {
                x: (0.0, 1.0),
                y: (0.0, 1.0),
                equal: false,
            },
            Vec::new(),
        );
        let look = Look {
            azimuth: Some(10.0),
            ..Look::default()
        };
        assert_eq!(
            Shot::new(&chart, &look).err(),
            Some(OPTIONS_OF_SPACE.to_string())
        );
        let shot = Shot::new(&chart, &Look::default()).expect("a view");
        assert_eq!(shot.size(), (1024, 768));
        // the ends of the ranges are the corners of the box
        let [left, top, right, bottom] = shot.plot;
        assert_eq!(shot.view.place([0.0, 0.0, 0.0]), Some([left, bottom, 0.0]));
        let p = shot.view.place([1.0, 1.0, 0.0]).expect("a place");
        assert!((p[0] - right).abs() < 1e-9 && (p[1] - top).abs() < 1e-9);
        // one scale for both axes: a wide picture shows more of x
        let equal = Drawing::new(
            Space::Chart {
                x: (-1.0, 1.0),
                y: (-1.0, 1.0),
                equal: true,
            },
            Vec::new(),
        );
        let shot = Shot::new(
            &equal,
            &Look {
                width: 800,
                height: 400,
                axes: Some(false),
                ..Look::default()
            },
        )
        .expect("a view");
        assert_eq!(shot.view.scale[0], shot.view.scale[1]);
        let (x0, y0) = shot.view.back(shot.plot[0], shot.plot[3]);
        assert!((y0 + 1.0).abs() < 1e-9 && x0 < -1.9);
    }

    #[test]
    fn pictures_of_characters() {
        let lines = text(&square_walk(), 20, 6, false).expect("a picture");
        assert_eq!(lines.len(), 6);
        assert!(
            lines
                .iter()
                .any(|l| l.chars().any(|c| ('\u{2801}'..='\u{28ff}').contains(&c)))
        );
        let flat = Drawing::new(
            Space::Flat,
            vec![Item::Line {
                points: vec![
                    [0.0, 0.0, 0.0],
                    [8.0, 0.0, 0.0],
                    [8.0, 4.0, 0.0],
                    [0.0, 4.0, 0.0],
                    [0.0, 0.0, 0.0],
                ],
                paint: Paint::Ink,
                weight: 1.0,
            }],
        );
        let lines = text(&flat, 16, 4, true).expect("a picture");
        // the box of marks, its ranges at the sides
        assert_eq!(lines.len(), 6);
        assert!(lines[0].starts_with(" 4.1 |"), "{}", lines[0]);
        assert!(
            lines[5].contains("-0.2") && lines[5].trim_end().ends_with("8.2"),
            "{}",
            lines[5]
        );
        assert!(lines.iter().map(|l| l.matches('*').count()).sum::<usize>() >= 30);
        // a field: a mark for every cell, darker and lighter
        let field = Drawing::new(
            Space::Chart {
                x: (0.0, 1.0),
                y: (0.0, 1.0),
                equal: false,
            },
            vec![Item::Field(Box::new(|x, _| Some(Shade::Level(x))))],
        );
        let lines = text(&field, 10, 2, true).expect("a picture");
        assert_eq!(lines[0], "1 | .:-=+*#%@");
        let lines = text(&field, 9, 1, false).expect("a picture");
        assert!(lines[0].ends_with("⠄⠤⠴⠶⡶⣶⣾⣿"), "{}", lines[0]);
    }
}
