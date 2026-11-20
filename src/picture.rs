//! Drawings as statements and as files: what is chosen with `name = value`, the pictures of
//! characters, `save_png` and `save_jxl`.

use crate::expr::{Expr, Rel};
use crate::imgfile::{self, Encoding, Format};
use crate::raster::{self, Rgb};
use crate::scene::{self, Drawing, Look, Shot, Space};

/// What is chosen with `name = value`.
pub const NAMED: &[&str] = &[
    "width",
    "height",
    "bits",
    "channels",
    "level",
    "quality",
    "distance",
    "effort",
    "frames",
    "fps",
    "turn",
    "slices",
    "thickness",
    "margin",
    "palette",
    "color",
    "colour",
    "background",
    "fog",
    "azimuth",
    "elevation",
    "roll",
    "zoom",
    "scale",
    "center",
    "centre",
    "viewpoint",
    "perspective",
    "stereo",
    "title",
    "xlabel",
    "ylabel",
    "arg_real",
    "arg_imag",
    "val_real",
    "val_imag",
    "colors",
    "colours",
    "fill",
    "range_real",
    "range_imag",
    "rotate",
    "voxels",
];

/// What is chosen for the shape of plot4d alone.
pub const OF_SHAPES: &[&str] = &[
    "arg_real",
    "arg_imag",
    "val_real",
    "val_imag",
    "colors",
    "colours",
    "fill",
    "range_real",
    "range_imag",
    "rotate",
    "normalize",
];

/// What is chosen with a word.
pub const WORDS: &[&str] = &[
    "gray",
    "grey",
    "alpha",
    "lossless",
    "depth",
    "grow",
    "axes",
    "noaxes",
    "grid",
    "ascii",
    "reverse",
    "views",
    "stereo",
    "normalize",
];

/// The choices whose value is a name: a colour, a palette, a side of a picture, a kind of body.
pub fn takes_name(option: &str) -> bool {
    matches!(
        option,
        "color"
            | "colour"
            | "background"
            | "palette"
            | "arg_real"
            | "arg_imag"
            | "val_real"
            | "val_imag"
            | "fill"
    )
}

/// The commands that save a drawing.
pub fn is_save(name: &str) -> bool {
    matches!(
        name,
        "save_png" | "save_jxl" | "save" | "save_png_fast" | "save_jxl_fast" | "save_fast"
    )
}

/// The commands that draw, besides the plots of `plot`.
pub fn is_drawing(name: &str) -> bool {
    crate::walks::is_walk(name) || crate::chart::is_chart(name) || name == "plot4d" || name == "view4d"
}

/// The commands whose arguments may be choices of a picture.
pub fn takes_choices(name: &str) -> bool {
    is_save(name) || is_drawing(name) || crate::plot::is_plot(name)
}

/// The most pixels of a picture, and of one of its sides.
const PIXELS: usize = 1 << 24;
const SIDE: usize = 16384;
const FRAMES: usize = 10_000;

/// What was chosen for a drawing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Choices {
    pub look: Look,
    /// The names of what was chosen, in their order.
    pub given: Vec<String>,
    pub width: Option<usize>,
    pub height: Option<usize>,
    pub bits: Option<u32>,
    pub channels: Option<u32>,
    pub gray: bool,
    pub alpha: bool,
    pub depth: bool,
    pub level: Option<u8>,
    pub lossless: bool,
    pub quality: Option<f64>,
    pub distance: Option<f64>,
    pub effort: Option<u8>,
    pub fps: Option<f64>,
    pub ascii: bool,
    pub reverse: bool,
    /// plot4d: what the real and the imaginary part of the argument and of the value lie on.
    pub axes: [Option<crate::plot4d::Axis>; 4],
    /// plot4d: how many colours the palette is cut into.
    pub colors: Option<usize>,
    pub fill: Option<crate::plot4d::Fill>,
    /// plot4d: the real and the imaginary parts of the values that are drawn.
    pub range_real: Option<(f64, f64)>,
    pub range_imag: Option<(f64, f64)>,
    /// plot4d: degrees that the body is turned about the upright axis (yaw), about the
    /// width (pitch) and about the frames (roll).
    pub rotate: Option<[f64; 3]>,
    /// plot4d: every frame is stretched on its own.
    pub normalize: bool,
    /// view4d: the cubes of the body across, upwards and along its frames.
    pub voxels: Option<[usize; 3]>,
    /// A quick look: a small picture that is packed fast.
    pub fast: bool,
}

/// The name of a choice and its value, when an argument is one.
pub fn choice(arg: &Expr) -> Option<(&str, Option<&Expr>)> {
    match arg {
        Expr::Sym(word) if WORDS.contains(&&**word) => Some((word, None)),
        Expr::Rel(Rel::Eq, name, value) => {
            let name = name.sym_name()?;
            NAMED.contains(&name).then_some((name, Some(&**value)))
        }
        _ => None,
    }
}

/// What an argument that is no choice says, when it looks like one.
pub fn stray(arg: &Expr) -> Option<String> {
    let name = match arg {
        Expr::Rel(Rel::Eq, name, _) => name.sym_name()?,
        Expr::Sym(word) => word,
        _ => return None,
    };
    Some(format!(
        "{name} is nothing that is chosen for a picture: {} are chosen with name = value, {} with the word alone",
        NAMED.join(", "),
        WORDS.join(", ")
    ))
}

fn number(name: &str, value: &Expr, range: (f64, f64)) -> Result<f64, String> {
    crate::plot::number(&crate::radix::plain(value.clone()))
        .filter(|v| *v >= range.0 && *v <= range.1)
        .ok_or_else(|| {
            format!(
                "{name} is a number from {} to {}, not {value}",
                crate::plot::label(range.0),
                crate::plot::label(range.1)
            )
        })
}

fn whole(name: &str, value: &Expr, range: (usize, usize)) -> Result<usize, String> {
    crate::radix::plain(value.clone())
        .as_i64()
        .and_then(|v| usize::try_from(v).ok())
        .filter(|v| (range.0..=range.1).contains(v))
        .ok_or_else(|| {
            format!(
                "{name} is a whole number from {} to {}, not {value}",
                range.0, range.1
            )
        })
}

fn point(name: &str, value: &Expr) -> Result<[f64; 3], String> {
    let numbers: Option<Vec<f64>> = value
        .as_list()
        .map(|items| items.iter().map(crate::plot::number).collect())
        .unwrap_or_default();
    match numbers.as_deref() {
        Some([x, y, z]) => Ok([*x, *y, *z]),
        Some([x, y]) => Ok([*x, *y, 0.0]),
        _ => Err(format!("{name} is a point [x, y, z], not {value}")),
    }
}

/// A colour: a name, six digits of base 16 in quotes, or [red, green, blue] from 0 to 1.
fn color(name: &str, value: &Expr) -> Result<Rgb, String> {
    let named = |text: &str| {
        let lower = text.to_ascii_lowercase();
        raster::COLORS
            .iter()
            .find(|(n, _)| *n == lower)
            .map(|(_, c)| *c)
            .or_else(|| raster::coded(text))
    };
    let found = match value {
        Expr::Sym(word) => named(word),
        Expr::List(items) if items.len() == 3 => {
            let parts: Option<Vec<f64>> = items.iter().map(crate::plot::number).collect();
            parts
                .filter(|p| p.iter().all(|v| (0.0..=1.0).contains(v)))
                .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32])
        }
        other => crate::radix::text_of(other).and_then(named),
    };
    found.ok_or_else(|| {
        let names: Vec<&str> = raster::COLORS.iter().map(|(n, _)| *n).collect();
        format!(
            "{name} is a colour: one of {}, or \"#rrggbb\", or [red, green, blue] with numbers from 0 to 1; not {value}",
            names.join(", ")
        )
    })
}

/// A palette: a name, or a list of colours.
fn palette(value: &Expr) -> Result<Vec<Rgb>, String> {
    let named = |text: &str| scene::palette(&text.to_ascii_lowercase());
    let found = match value {
        Expr::Sym(word) => named(word),
        Expr::List(items) if items.len() >= 2 => {
            return items
                .iter()
                .map(|item| color("a colour of the palette", item))
                .collect();
        }
        other => crate::radix::text_of(other).and_then(named),
    };
    found.ok_or_else(|| {
        let names: Vec<&str> = raster::PALETTES.iter().map(|(n, _)| *n).collect();
        format!(
            "the palette is one of {}, or a list of colours; not {value}",
            names.join(", ")
        )
    })
}

impl Choices {
    /// Takes a choice: its name, and its value as it is after the evaluation.
    pub fn set(&mut self, name: &str, value: Option<&Expr>) -> Result<(), String> {
        self.given.push(name.to_string());
        let look = &mut self.look;
        let Some(value) = value else {
            match name {
                "gray" | "grey" => self.gray = true,
                "alpha" => self.alpha = true,
                "lossless" => self.lossless = true,
                "depth" => self.depth = true,
                "grow" => look.grow = true,
                "axes" => look.axes = Some(true),
                "noaxes" => look.axes = Some(false),
                "grid" => look.grid = true,
                "ascii" => self.ascii = true,
                "views" => look.views = true,
                "normalize" => self.normalize = true,
                "stereo" => look.stereo = Some(4.0),
                _ => self.reverse = true,
            }
            return Ok(());
        };
        let degrees = (-1e9, 1e9);
        match name {
            "width" => self.width = Some(whole(name, value, (1, SIDE))?),
            "height" => self.height = Some(whole(name, value, (1, SIDE))?),
            "bits" => self.bits = Some(whole(name, value, (1, 16))? as u32),
            "channels" => self.channels = Some(whole(name, value, (1, 4))? as u32),
            "level" => self.level = Some(whole(name, value, (0, 9))? as u8),
            "quality" => self.quality = Some(number(name, value, (0.0, 100.0))?),
            "distance" => self.distance = Some(number(name, value, (0.0, 25.0))?),
            "effort" => self.effort = Some(whole(name, value, (1, 10))? as u8),
            "frames" => look.frames = whole(name, value, (1, FRAMES))?,
            "fps" => self.fps = Some(number(name, value, (0.001, 1000.0))?),
            "turn" => look.turn = Some(number(name, value, degrees)?),
            "slices" => look.slices = whole(name, value, (1, FRAMES))?,
            "thickness" => look.thickness = number(name, value, (0.05, 200.0))?,
            "margin" => look.margin = number(name, value, (0.0, 0.45))?,
            "palette" => look.palette = Some(palette(value)?),
            "color" | "colour" => look.color = Some(color(name, value)?),
            "background" => look.background = Some(color(name, value)?),
            "fog" => look.fog = Some(number(name, value, (0.0, 1.0))?),
            "azimuth" => look.azimuth = Some(number(name, value, degrees)?),
            "elevation" => look.elevation = Some(number(name, value, (-90.0, 90.0))?),
            "roll" => look.roll = number(name, value, degrees)?,
            "zoom" => look.zoom = number(name, value, (1e-6, 1e6))?,
            "scale" => look.scale = Some(number(name, value, (1e-12, 1e12))?),
            "center" | "centre" => look.center = Some(point(name, value)?),
            "viewpoint" => look.viewpoint = Some(point(name, value)?),
            "stereo" => look.stereo = Some(number(name, value, (0.1, 30.0))?),
            "arg_real" | "arg_imag" | "val_real" | "val_imag" => {
                let axis = crate::plot4d::Axis::named(value).ok_or_else(|| {
                    format!(
                        "{name} is one of width, height, frame, color: what this part of the argument z or of the value f(z) lies on; not {value}"
                    )
                })?;
                let part = ["arg_real", "arg_imag", "val_real", "val_imag"]
                    .iter()
                    .position(|n| *n == name)
                    .unwrap_or(0);
                self.axes[part] = Some(axis);
            }
            "colors" | "colours" => self.colors = Some(whole(name, value, (2, 65535))?),
            "fill" => {
                self.fill = Some(crate::plot4d::Fill::named(value).ok_or_else(|| {
                    format!(
                        "fill is one of surface, below, above: the surface alone, or the body from it down to the first frame or up to the last one; not {value}"
                    )
                })?);
            }
            "range_real" | "range_imag" => {
                let ends: Option<Vec<f64>> = value
                    .as_list()
                    .map(|items| items.iter().map(crate::plot::number).collect())
                    .unwrap_or_default();
                let ends = match ends.as_deref() {
                    Some([low, high]) if low < high => (*low, *high),
                    _ => {
                        return Err(format!(
                            "{name} is [least, greatest]: the {} parts of the values that are drawn; not {value}",
                            if name == "range_real" {
                                "real"
                            } else {
                                "imaginary"
                            }
                        ));
                    }
                };
                if name == "range_real" {
                    self.range_real = Some(ends);
                } else {
                    self.range_imag = Some(ends);
                }
            }
            "rotate" => {
                let angles: Option<Vec<f64>> = value
                    .as_list()
                    .map(|items| items.iter().map(crate::plot::number).collect())
                    .unwrap_or_else(|| crate::plot::number(value).map(|yaw| vec![yaw]));
                self.rotate = Some(match angles.as_deref() {
                    Some([yaw]) => [*yaw, 0.0, 0.0],
                    Some([yaw, pitch]) => [*yaw, *pitch, 0.0],
                    Some([yaw, pitch, roll]) => [*yaw, *pitch, *roll],
                    _ => {
                        return Err(format!(
                            "rotate(yaw, pitch, roll) or rotate = [yaw, pitch, roll]: the degrees that the body of plot4d is turned about its upright axis, about its width and about its frames; not {value}"
                        ));
                    }
                });
            }
            "voxels" => {
                let sides: Option<Vec<usize>> = match value.as_list() {
                    Some(items) => items
                        .iter()
                        .map(|item| whole(name, item, (2, 1024)).ok())
                        .collect(),
                    None => whole(name, value, (2, 1024)).ok().map(|n| vec![n; 3]),
                };
                self.voxels = Some(match sides.as_deref() {
                    Some([w, h, d]) => [*w, *h, *d],
                    _ => {
                        return Err(format!(
                            "voxels is a whole number from 2 to 1024, or [across, upwards, frames] of such numbers: the cubes that the body of view4d is made of; not {value}"
                        ));
                    }
                });
            }
            "title" | "xlabel" | "ylabel" => {
                let text = match crate::radix::text_of(value) {
                    Some(t) => t.to_string(),
                    None => format!("{value}"),
                };
                match name {
                    "title" => look.title = Some(text),
                    "xlabel" => look.xlabel = Some(text),
                    _ => look.ylabel = Some(text),
                }
            }
            _ => {
                let far = crate::plot::number(value).filter(|d| *d > 1.0 && *d <= 1e9);
                look.distance = Some(far.ok_or_else(|| {
                    format!(
                        "perspective is the distance of the eye in radii of the drawing, a number above 1, not {value}"
                    )
                })?);
            }
        }
        Ok(())
    }

    /// How finely a drawing is worked out: the dots across the picture.
    /// What is chosen for plot4d alone, when something of it is.
    pub fn of_shapes(&self) -> Option<&str> {
        self.given
            .iter()
            .map(String::as_str)
            .find(|g| OF_SHAPES.contains(g))
    }

    pub fn detail(&self, pixels: bool) -> usize {
        if pixels {
            self.width.unwrap_or(if self.fast { 256 } else { 1024 })
        } else {
            2 * self.width.unwrap_or(64)
        }
    }

    fn has(&self, name: &str) -> bool {
        self.given.iter().any(|g| g == name)
    }
}

/// The drawing as lines of characters with what is to tell about it.
pub fn show(command: &str, drawing: &Drawing, choices: &Choices) -> Result<Vec<String>, String> {
    if let Some(other) = choices.of_shapes() {
        return Err(format!("{other} belongs to plot4d, not to {command}"));
    }
    if choices.has("voxels") {
        return Err(format!("voxels belongs to view4d, not to {command}"));
    }
    if let Some(other) = choices.given.iter().find(|g| {
        !matches!(
            g.as_str(),
            "width" | "height" | "ascii" | "title" | "xlabel" | "ylabel"
        )
    }) {
        return Err(format!(
            "{other} is chosen for a picture that is saved: save_png(\"{command}.png\", {command}(...), {other}{}); the picture of characters takes width, height and ascii",
            if WORDS.contains(&other.as_str()) {
                ""
            } else {
                " = ..."
            }
        ));
    }
    let limited =
        |given: Option<usize>, name: &str, range: (usize, usize), usual: usize| match given {
            None => Ok(usual),
            Some(v) if (range.0..=range.1).contains(&v) => Ok(v),
            Some(v) => Err(format!(
                "the {name} of a picture of characters is from {} to {}, not {v}",
                range.0, range.1
            )),
        };
    let width = limited(choices.width, "width", (8, 400), 64)?;
    let usual = match drawing.space {
        Space::Chart { .. } => 16,
        _ => 20,
    };
    let height = limited(choices.height, "height", (2, 200), usual)?;
    let mut lines = worded(
        scene::text(drawing, width, height, choices.ascii)?,
        &choices.look,
    );
    lines.extend(drawing.facts.iter().cloned());
    Ok(lines)
}

/// A picture of characters with its words: the title above it, the names of the axes below.
pub fn worded(mut lines: Vec<String>, look: &Look) -> Vec<String> {
    if let Some(title) = &look.title {
        lines.insert(0, title.clone());
    }
    for (axis, name) in [("x", &look.xlabel), ("y", &look.ylabel)] {
        if let Some(name) = name {
            lines.push(format!("{axis}: {name}"));
        }
    }
    lines
}

/// The arguments of a plot of characters without its title and the names of its axes, which
/// are told apart.
pub fn words_apart(args: &[Expr]) -> (Vec<Expr>, Vec<(&str, &Expr)>) {
    let mut plain = Vec::new();
    let mut words = Vec::new();
    for arg in args {
        match choice(arg) {
            Some((name @ ("title" | "xlabel" | "ylabel"), Some(value))) => {
                words.push((name, value))
            }
            _ => plain.push(arg.clone()),
        }
    }
    (plain, words)
}

/// The kind of file that a name ends with.
pub fn format_of(command: &str, path: &str) -> Result<Format, String> {
    let lower = path.to_ascii_lowercase();
    match command {
        "save_png" | "save_png_fast" => Ok(Format::Png),
        "save_jxl" | "save_jxl_fast" => Ok(Format::Jxl),
        _ if lower.ends_with(".png") || lower.ends_with(".apng") => Ok(Format::Png),
        _ if lower.ends_with(".jxl") => Ok(Format::Jxl),
        _ => Err(format!(
            "save tells the kind of file by the end of its name, .png or .jxl, which \"{path}\" lacks: save_png and save_jxl take any name"
        )),
    }
}

/// How the file is written, from what was chosen; `effort` is that of JPEG XL unless one is chosen.
pub(crate) fn encoding_of(
    format: Format,
    choices: &Choices,
    effort: u8,
) -> Result<Encoding, String> {
    let png = format == Format::Png;
    let bits = choices.bits.unwrap_or(8);
    let mut encoding = Encoding {
        format,
        bits,
        level: choices.level.unwrap_or(if choices.fast { 1 } else { 9 }),
        distance: None,
        effort: choices.effort.unwrap_or(effort),
        rate: rate(choices.fps.unwrap_or(12.0)),
        alpha_bits: None,
    };
    if png {
        if !matches!(bits, 8 | 16) {
            return Err(format!(
                "a PNG file has 8 or 16 bits, not {bits}: save_jxl takes 1 to 16"
            ));
        }
        for (given, name) in [
            (choices.lossless, "lossless"),
            (choices.quality.is_some(), "quality"),
            (choices.distance.is_some(), "distance"),
            (choices.effort.is_some(), "effort"),
            (choices.depth, "depth"),
        ] {
            if given {
                return Err(format!(
                    "{name} belongs to save_jxl: a PNG file is lossless, and its level from 0 to 9 says how hard it is packed"
                ));
            }
        }
    } else {
        if choices.level.is_some() {
            return Err("level belongs to save_png: save_jxl takes effort from 1 to 10, and quality or distance for a lossy picture".into());
        }
        let lossy = match (choices.quality, choices.distance) {
            (Some(_), Some(_)) => {
                return Err("quality or distance, not both: quality = 90 is distance = 1".into());
            }
            (Some(q), None) if q < 100.0 => Some(imgfile::distance_of(q as f32)),
            (None, Some(d)) if d > 0.0 => Some(d as f32),
            // (a quick look is lossy unless it is said otherwise)
            (None, None) if choices.fast && !choices.lossless => Some(1.0),
            _ => None,
        };
        if choices.lossless && lossy.is_some() {
            return Err("lossless, or lossy with quality or distance: not both".into());
        }
        encoding.distance = lossy;
    }
    Ok(encoding)
}

/// Saves the drawing as a file and tells what was saved.
pub fn save(
    path: &str,
    format: Format,
    drawing: &Drawing,
    choices: &Choices,
) -> Result<Vec<String>, String> {
    if choices.ascii {
        return Err("ascii belongs to the picture of characters, which is not saved".to_string());
    }
    if path.is_empty() {
        return Err("the name of the file is empty".to_string());
    }
    if let Some(other) = choices.of_shapes() {
        return Err(format!("{other} belongs to plot4d, not to this drawing"));
    }
    if choices.has("voxels") {
        return Err("voxels belongs to view4d, not to this drawing".to_string());
    }
    let png = format == Format::Png;
    let mut look = choices.look.clone();
    look.width = choices
        .width
        .unwrap_or(if choices.fast { 256 } else { 1024 });
    look.height = choices.height.unwrap_or(0);
    if choices.reverse {
        let mut stops = look
            .palette
            .clone()
            .or_else(|| scene::palette(drawing.palette))
            .unwrap_or_default();
        stops.reverse();
        look.palette = Some(stops);
    }
    if look.slices > 0 && choices.has("frames") {
        return Err(
            "slices are the frames of the picture: slices = n or frames = n, not both".into(),
        );
    }
    if look.views && (choices.has("frames") || look.slices > 0 || look.grow || look.turn.is_some())
    {
        return Err(
            "views are three frames of their own: not with frames, slices, turn or grow".into(),
        );
    }
    if look.frames <= 1 && (look.grow || look.turn.is_some()) {
        return Err(format!(
            "{} needs frames: frames = n",
            if look.grow { "grow" } else { "turn" }
        ));
    }
    let (gray, alpha) = match choices.channels {
        None => (choices.gray, choices.alpha),
        Some(n) => {
            let (gray, alpha) = (n <= 2, n % 2 == 0);
            if (choices.gray && !gray) || (choices.alpha && !alpha) {
                return Err(format!(
                    "channels = {n} is {}: this does not go with {}",
                    kind(gray, alpha, false),
                    if choices.gray && !gray {
                        "gray"
                    } else {
                        "alpha"
                    }
                ));
            }
            (gray, alpha)
        }
    };
    let encoding = encoding_of(format, choices, if choices.fast { 3 } else { 7 })?;
    let bits = encoding.bits;
    if look.stereo.is_some() && (gray || alpha || choices.depth) {
        return Err(format!(
            "stereo is a picture of red and cyan for glasses of these colours: this does not go with {}",
            if gray {
                "gray"
            } else if alpha {
                "alpha"
            } else {
                "depth"
            }
        ));
    }
    // (a body with edges of areas is drawn twice as large when that is not too much)
    let high = if look.height == 0 {
        look.width
    } else {
        look.height
    };
    let fine = if drawing.smooth && 4 * look.width * high <= PIXELS {
        2
    } else {
        1
    };
    look.fine = fine;
    look.width *= fine;
    look.height *= fine;
    let shot = Shot::new(drawing, &look)?;
    let (width, height) = shot.size();
    let (width, height) = (width / fine, height / fine);
    if width * height > PIXELS {
        return Err(format!(
            "a picture has {PIXELS} pixels at most ({} by {}), not {width} by {height}",
            1 << 12,
            1 << 12
        ));
    }
    let count = shot.frames();
    let tone = shot.tone(gray, alpha, choices.depth);
    let bytes = imgfile::encode(
        count,
        &mut |k| {
            let image = match look.stereo {
                None => shot.raster(k).image(&tone),
                Some(_) => {
                    let lights = shot.tone(true, false, false);
                    let [left, right] = shot.pair(k);
                    two_eyes(&left.image(&lights), &right.image(&lights))
                }
            };
            Ok(match fine {
                1 => image,
                _ => smaller(&image, fine),
            })
        },
        &encoding,
    )?;
    imgfile::write(path, &bytes)?;
    let mut told = vec![
        if png { "PNG" } else { "JPEG XL" }.to_string(),
        format!("{width} x {height} pixels"),
        format!("{bits} bit{}", if bits == 1 { "" } else { "s" }),
        match look.stereo {
            None => kind(gray, alpha, choices.depth).to_string(),
            Some(apart) => format!(
                "red for the left eye and cyan for the right one, {} degrees apart",
                crate::plot::label(apart)
            ),
        },
    ];
    if count > 1 {
        let (n, d) = encoding.rate;
        let rate = if d == 1 {
            n.to_string()
        } else {
            crate::plot::label(f64::from(n) / f64::from(d))
        };
        told.push(format!(
            "{count} {}, {rate} a second",
            if look.slices > 0 {
                "slices as frames"
            } else if look.views {
                "views as frames (from above, from the front, from the right)"
            } else {
                "frames"
            }
        ));
    }
    if png {
        told.push(format!("level {}", encoding.level));
    } else {
        told.push(match encoding.distance {
            None => "lossless".to_string(),
            Some(d) => format!("lossy at the distance {}", crate::plot::label(f64::from(d))),
        });
        told.push(format!("effort {}", encoding.effort));
    }
    told.push(format!("{} bytes", bytes.len()));
    let mut lines = drawing.facts.clone();
    lines.push(format!("saved {path}: {}", told.join(", ")));
    Ok(lines)
}

/// A picture for glasses with a red and a cyan glass: the left eye sees the red of it, the
/// right eye the green and the blue.
fn two_eyes(left: &imgfile::Image, right: &imgfile::Image) -> imgfile::Image {
    let mut data = Vec::with_capacity(3 * left.data.len());
    for (l, r) in left.data.iter().zip(&right.data) {
        data.extend([*l, *r, *r]);
    }
    imgfile::Image {
        width: left.width,
        height: left.height,
        colors: 3,
        alpha: false,
        data,
        depth: None,
    }
}

/// A picture whose pixels are the means of squares of `by` times `by` pixels; where the
/// picture says how much of a pixel is drawn, the colours count as much as is drawn.
fn smaller(image: &imgfile::Image, by: usize) -> imgfile::Image {
    let channels = image.channels();
    let (width, height) = (image.width / by, image.height / by);
    let square = (by * by) as f32;
    let mut data = Vec::with_capacity(width * height * channels);
    let mut depth = image
        .depth
        .as_ref()
        .map(|_| Vec::with_capacity(width * height));
    for row in 0..height {
        for column in 0..width {
            let places = (0..by).flat_map(|j| {
                (0..by).map(move |i| (row * by + j) * image.width + column * by + i)
            });
            let mut sums = [0.0f32; 4];
            let (mut drawn, mut far) = (0.0f32, 0.0f32);
            for at in places {
                let pixel = &image.data[at * channels..(at + 1) * channels];
                let share = if image.alpha {
                    pixel[channels - 1]
                } else {
                    1.0
                };
                for (sum, sample) in sums.iter_mut().zip(&pixel[..image.colors]) {
                    *sum += share * sample;
                }
                drawn += share;
                if let Some(depths) = &image.depth {
                    far += depths[at];
                }
            }
            for sum in &sums[..image.colors] {
                data.push(if drawn > 0.0 { sum / drawn } else { 0.0 });
            }
            if image.alpha {
                data.push(drawn / square);
            }
            if let Some(depth) = &mut depth {
                depth.push(far / square);
            }
        }
    }
    imgfile::Image {
        width,
        height,
        colors: image.colors,
        alpha: image.alpha,
        data,
        depth,
    }
}

/// The channels of a picture in words.
fn kind(gray: bool, alpha: bool, depth: bool) -> &'static str {
    match (gray, alpha, depth) {
        (true, false, false) => "gray",
        (true, true, false) => "gray + alpha",
        (false, false, false) => "RGB",
        (false, true, false) => "RGB + alpha",
        (true, false, true) => "gray + depth",
        (true, true, true) => "gray + alpha + depth",
        (false, false, true) => "RGB + depth",
        (false, true, true) => "RGB + alpha + depth",
    }
}

/// Frames a second as a fraction of whole numbers.
fn rate(fps: f64) -> (u32, u32) {
    if fps == fps.floor() {
        return (fps as u32, 1);
    }
    let (mut n, mut d) = ((fps * 1000.0).round() as u32, 1000u32);
    let mut divisor = 2;
    while divisor <= 5 {
        if n.is_multiple_of(divisor) && d.is_multiple_of(divisor) {
            n /= divisor;
            d /= divisor;
        } else {
            divisor += 1;
        }
    }
    (n.max(1), d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str) -> Expr {
        crate::parser::parse(text).expect("an expression").stmts[0].clone()
    }

    fn chosen(pairs: &[&str]) -> Result<Choices, String> {
        let mut choices = Choices::default();
        for text in pairs {
            // (as the arguments of a command that draws)
            let call = parsed(&format!("walk({text})"));
            let Expr::Fn(_, args) = &call else {
                panic!("a call")
            };
            let (name, value) =
                choice(&args[0]).ok_or_else(|| stray(&args[0]).unwrap_or_default())?;
            choices.set(name, value)?;
        }
        Ok(choices)
    }

    #[test]
    fn what_is_chosen() {
        let c = chosen(&[
            "width = 800",
            "height = 600",
            "bits = 16",
            "gray",
            "alpha",
            "thickness = 2.5",
            "azimuth = -45",
            "elevation = 30",
            "roll = 10",
            "zoom = 2",
            "frames = 36",
            "turn = 180",
            "fog = 0.25",
            "margin = 0.1",
            "viewpoint = [1, 2, 3]",
            "center = [0, 0]",
            "perspective = 3",
            "scale = 4",
            "grid",
            "noaxes",
            "grow",
            "fps = 12.5",
            "level = 3",
            "effort = 9",
            "quality = 80",
            "channels = 2",
        ])
        .expect("choices");
        assert_eq!(
            (c.width, c.height, c.bits),
            (Some(800), Some(600), Some(16))
        );
        assert!(c.gray && c.alpha && c.look.grid && c.look.grow);
        assert_eq!(c.look.axes, Some(false));
        assert_eq!(
            (c.look.thickness, c.look.azimuth, c.look.elevation),
            (2.5, Some(-45.0), Some(30.0))
        );
        assert_eq!(
            (c.look.roll, c.look.zoom, c.look.frames, c.look.turn),
            (10.0, 2.0, 36, Some(180.0))
        );
        assert_eq!((c.look.fog, c.look.margin), (Some(0.25), 0.1));
        assert_eq!(c.look.viewpoint, Some([1.0, 2.0, 3.0]));
        assert_eq!(c.look.center, Some([0.0, 0.0, 0.0]));
        assert_eq!((c.look.distance, c.look.scale), (Some(3.0), Some(4.0)));
        assert_eq!(
            (c.fps, c.level, c.effort, c.quality, c.channels),
            (Some(12.5), Some(3), Some(9), Some(80.0), Some(2))
        );
        assert_eq!(c.given.len(), 26);
        // colours and palettes by name, by digits and by parts
        let c = chosen(&["color = red", "background = \"#102030\"", "palette = fire"])
            .expect("choices");
        assert_eq!(c.look.color, Some(raster::COLORS[2].1));
        assert_eq!(
            c.look.background,
            Some([16.0 / 255.0, 32.0 / 255.0, 48.0 / 255.0])
        );
        assert_eq!(c.look.palette.as_ref().map(Vec::len), Some(5));
        let c = chosen(&[
            "colour = [0, 0.5, 1]",
            "background = silver",
            "palette = [black, \"ff0000\", [1, 1, 0]]",
        ])
        .expect("choices");
        assert_eq!(c.look.color, Some([0.0, 0.5, 1.0]));
        assert_eq!(c.look.background, Some([192.0 / 255.0; 3]));
        assert_eq!(
            c.look.palette,
            Some(vec![[0.0; 3], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]])
        );
        for (text, why) in [
            (
                "width = 0",
                "width is a whole number from 1 to 16384, not 0",
            ),
            (
                "width = 2.5",
                "width is a whole number from 1 to 16384, not 5/2",
            ),
            ("bits = 17", "bits is a whole number from 1 to 16, not 17"),
            ("level = 10", "level is a whole number from 0 to 9, not 10"),
            (
                "quality = 101",
                "quality is a number from 0 to 100, not 101",
            ),
            (
                "elevation = 100",
                "elevation is a number from -90 to 90, not 100",
            ),
            (
                "perspective = 1",
                "perspective is the distance of the eye in radii of the drawing, a number above 1, not 1",
            ),
            ("viewpoint = 3", "viewpoint is a point [x, y, z], not 3"),
            ("fog = x", "fog is a number from 0 to 1, not x"),
        ] {
            assert_eq!(chosen(&[text]).err(), Some(why.to_string()), "{text}");
        }
        let why = chosen(&["color = mauve"]).expect_err("no colour");
        assert!(
            why.starts_with("color is a colour: one of black, white, red,")
                && why.ends_with("not mauve"),
            "{why}"
        );
        let why = chosen(&["palette = mauve"]).expect_err("no palette");
        assert!(
            why.starts_with("the palette is one of rainbow, fire, ice,"),
            "{why}"
        );
        let why = chosen(&["widht = 3"]).expect_err("no choice");
        assert!(
            why.starts_with("widht is nothing that is chosen for a picture: width, height, bits,"),
            "{why}"
        );
    }

    /// A directory of its own for the files of a test.
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("symbolic-math-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory");
        dir
    }

    fn drawn(name: &str, args: &[&str]) -> Drawing {
        let a: Vec<Expr> = args
            .iter()
            .map(|text| crate::Session::new().eval(&parsed(text)).expect("a value"))
            .collect();
        crate::walks::build(name, &a, &mut Vec::new()).expect("a drawing")
    }

    /// The choices of a small picture, with more of them.
    fn small(more: &[&str]) -> Choices {
        let mut all = vec!["width = 48", "height = 40"];
        all.extend(more);
        chosen(&all).expect("choices")
    }

    /// The frames of a drawing as the program makes them, as whole numbers of so many bits.
    fn frames_of(
        drawing: &Drawing,
        choices: &Choices,
        bits: u32,
        gray: bool,
        alpha: bool,
    ) -> Vec<Vec<u32>> {
        let mut look = choices.look.clone();
        look.width = choices.width.unwrap_or(1024);
        look.height = choices.height.unwrap_or(0);
        let shot = Shot::new(drawing, &look).expect("a view");
        let tone = shot.tone(gray, alpha, false);
        (0..shot.frames())
            .map(|k| crate::imgfile::testing::whole(&shot.raster(k).image(&tone).data, bits))
            .collect()
    }

    /// The channels of a frame of a JPEG XL file one after the other for each pixel, as a
    /// PNG file has them.
    fn mixed(planes: &[Vec<f32>], bits: u32) -> Vec<u32> {
        let pixels = planes[0].len();
        let mut out = Vec::with_capacity(pixels * planes.len());
        for k in 0..pixels {
            for plane in planes {
                out.push(crate::imgfile::step(plane[k], (1 << bits) - 1));
            }
        }
        out
    }

    #[test]
    fn saved_files_hold_the_picture() {
        use crate::imgfile::testing::{read_jxl, read_png};
        let dir = scratch("saved");
        let file = |name: &str| dir.join(name).to_string_lossy().into_owned();
        let drawing = drawn("walk3d", &["e + pi*i", "12", "300"]);
        // a PNG file: the facts, then what was saved; the pixels of the frame
        let path = file("walk.png");
        let choices = small(&[]);
        let lines = save(&path, Format::Png, &drawing, &choices).expect("a file");
        let bytes = std::fs::read(&path).expect("the file");
        assert_eq!(lines[..2], drawing.facts[..]);
        assert_eq!(
            lines[2],
            format!(
                "saved {path}: PNG, 48 x 40 pixels, 8 bits, RGB, level 9, {} bytes",
                bytes.len()
            )
        );
        let (shape, frames) = read_png(&bytes);
        assert_eq!((shape.width, shape.height), (48, 40));
        assert_eq!(
            (shape.color_type, shape.bit_depth),
            (png::ColorType::Rgb, png::BitDepth::Eight)
        );
        let want = frames_of(&drawing, &choices, 8, false, false);
        assert_eq!(frames, want);
        assert!(
            want[0].iter().filter(|v| **v > 0).count() > 300,
            "a walk is drawn"
        );
        // 16 bits, gray with alpha
        let choices = small(&["bits = 16", "gray", "alpha", "level = 0"]);
        let lines = save(&path, Format::Png, &drawing, &choices).expect("a file");
        assert!(
            lines[2].contains("16 bits, gray + alpha, level 0, "),
            "{}",
            lines[2]
        );
        let (shape, frames) = read_png(&std::fs::read(&path).expect("the file"));
        assert_eq!(
            (shape.color_type, shape.bit_depth),
            (png::ColorType::GrayscaleAlpha, png::BitDepth::Sixteen)
        );
        assert_eq!(frames, frames_of(&drawing, &choices, 16, true, true));
        // channels = 4 is colours with alpha; the same with the word
        for choices in [small(&["channels = 4"]), small(&["alpha"])] {
            save(&path, Format::Png, &drawing, &choices).expect("a file");
            let (shape, frames) = read_png(&std::fs::read(&path).expect("the file"));
            assert_eq!(shape.color_type, png::ColorType::Rgba);
            assert_eq!(frames, frames_of(&drawing, &choices, 8, false, true));
        }
        // a JPEG XL file without loss has the same pixels as the PNG file
        let jxl = file("walk.jxl");
        let choices = small(&[]);
        let lines = save(&jxl, Format::Jxl, &drawing, &choices).expect("a file");
        let bytes = std::fs::read(&jxl).expect("the file");
        assert_eq!(
            lines[2],
            format!(
                "saved {jxl}: JPEG XL, 48 x 40 pixels, 8 bits, RGB, lossless, effort 7, {} bytes",
                bytes.len()
            )
        );
        let (_, frames, names) = read_jxl(&bytes);
        assert!(names.is_empty());
        assert_eq!(frames.len(), 1);
        assert_eq!(mixed(&frames[0], 8), want[0]);
        // 12 bits of gray
        let choices = small(&["bits = 12", "gray", "lossless", "effort = 2"]);
        let lines = save(&jxl, Format::Jxl, &drawing, &choices).expect("a file");
        assert!(
            lines[2].contains("12 bits, gray, lossless, effort 2, "),
            "{}",
            lines[2]
        );
        let (image, frames, _) = read_jxl(&std::fs::read(&jxl).expect("the file"));
        assert_eq!(
            image.image_header().metadata.bit_depth.bits_per_sample(),
            12
        );
        assert_eq!(
            mixed(&frames[0], 12),
            frames_of(&drawing, &choices, 12, true, false)[0]
        );
        // with loss: a smaller file with nearly the same picture
        let exact = bytes.len();
        let choices = small(&["quality = 60"]);
        let lines = save(&jxl, Format::Jxl, &drawing, &choices).expect("a file");
        assert!(lines[2].contains("lossy at the distance "), "{}", lines[2]);
        let bytes = std::fs::read(&jxl).expect("the file");
        assert!(bytes.len() < exact);
        let (_, frames, _) = read_jxl(&bytes);
        let lossy = mixed(&frames[0], 8);
        let off: u64 = lossy
            .iter()
            .zip(&want[0])
            .map(|(a, b)| u64::from(a.abs_diff(*b)))
            .sum();
        assert!(off / (lossy.len() as u64) < 24, "{off}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn frames_slices_and_depth() {
        use crate::imgfile::testing::{read_jxl, read_png};
        let dir = scratch("frames");
        let file = |name: &str| dir.join(name).to_string_lossy().into_owned();
        let drawing = drawn("walk3d", &["e + pi*i", "12", "300"]);
        // the eye goes around: every frame differs from the one before, in both kinds of file
        let choices = small(&["frames = 4", "fps = 2.5"]);
        let want = frames_of(&drawing, &choices, 8, false, false);
        assert_eq!(want.len(), 4);
        assert!(want.windows(2).all(|w| w[0] != w[1]));
        let path = file("turn.png");
        let lines = save(&path, Format::Png, &drawing, &choices).expect("a file");
        assert!(
            lines[2].contains("RGB, 4 frames, 2.5 a second, level 9, "),
            "{}",
            lines[2]
        );
        assert_eq!(read_png(&std::fs::read(&path).expect("the file")).1, want);
        let jxl = file("turn.jxl");
        save(&jxl, Format::Jxl, &drawing, &choices).expect("a file");
        let (image, frames, _) = read_jxl(&std::fs::read(&jxl).expect("the file"));
        assert_eq!(frames.len(), 4);
        for (frame, want) in frames.iter().zip(&want) {
            assert_eq!(&mixed(frame, 8), want);
        }
        let animation = image
            .image_header()
            .metadata
            .animation
            .as_ref()
            .expect("an animation");
        assert_eq!((animation.tps_numerator, animation.tps_denominator), (5, 2));
        // a part of a turn: the first frame is the picture without frames
        let part = small(&["frames = 3", "turn = 90"]);
        let still = frames_of(
            &drawing,
            &small(&["frames = 3", "turn = 0"]),
            8,
            false,
            false,
        );
        assert_eq!(still[0], still[2]);
        let turned = frames_of(&drawing, &part, 8, false, false);
        assert!(turned[0] != turned[1] && turned[1] != turned[2]);
        // the walk grows: every frame has what the one before has, and more
        let growing = frames_of(&drawing, &small(&["frames = 5", "grow"]), 8, false, false);
        let lit = |frame: &Vec<u32>| frame.iter().filter(|v| **v > 0).count();
        assert!(growing.windows(2).all(|w| lit(&w[0]) < lit(&w[1])));
        assert_eq!(
            growing[4],
            frames_of(&drawing, &small(&[]), 8, false, false)[0]
        );
        // slabs of depth: together they hold every pixel of the picture
        let choices = small(&["slices = 6", "fog = 0"]);
        let slabs = frames_of(&drawing, &choices, 8, false, false);
        assert_eq!(slabs.len(), 6);
        let whole = &frames_of(&drawing, &small(&["fog = 0"]), 8, false, false)[0];
        let lit =
            |frame: &Vec<u32>, pixel: usize| frame[3 * pixel..3 * pixel + 3].iter().any(|v| *v > 0);
        for pixel in 0..48 * 40 {
            assert_eq!(
                lit(whole, pixel),
                slabs.iter().any(|s| lit(s, pixel)),
                "pixel {pixel}"
            );
        }
        let lines = save(&jxl, Format::Jxl, &drawing, &choices).expect("a file");
        assert!(
            lines[2].contains("RGB, 6 slices as frames, 12 a second, lossless"),
            "{}",
            lines[2]
        );
        assert_eq!(read_jxl(&std::fs::read(&jxl).expect("the file")).1.len(), 6);
        // a channel of depths: 1 where nothing is drawn, less where the walk is
        let choices = small(&["depth"]);
        let lines = save(&jxl, Format::Jxl, &drawing, &choices).expect("a file");
        assert!(
            lines[2].contains("8 bits, RGB + depth, lossless"),
            "{}",
            lines[2]
        );
        let (_, frames, names) = read_jxl(&std::fs::read(&jxl).expect("the file"));
        assert_eq!(names, ["depth"]);
        let look = Look {
            width: 48,
            height: 40,
            ..choices.look.clone()
        };
        let shot = Shot::new(&drawing, &look).expect("a view");
        let raster = shot.raster(0);
        let image = raster.image(&shot.tone(false, false, true));
        let depths = image.depth.as_ref().expect("depths");
        assert_eq!(
            crate::imgfile::testing::whole(&frames[0][3], 8),
            crate::imgfile::testing::whole(depths, 8)
        );
        let mut near = 0;
        for (k, d) in depths.iter().enumerate() {
            if raster.cover(k % 48, k / 48) == 0.0 {
                assert_eq!(*d, 1.0, "pixel {k}");
            } else if *d < 0.9 {
                near += 1;
            }
        }
        assert!(near > 100, "{near}");
        assert!(
            depths.iter().any(|d| *d < 0.05),
            "the nearest thing has the depth 0"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn views_and_two_eyes() {
        use crate::imgfile::testing::{read_png, whole};
        let dir = scratch("eyes");
        let path = dir.join("eyes.png").to_string_lossy().into_owned();
        let drawing = drawn("walk3d", &["e + pi*i", "12", "300"]);
        // three views are three frames
        let choices = small(&["views"]);
        let lines = save(&path, Format::Png, &drawing, &choices).expect("a file");
        assert!(
            lines[2].contains("RGB, 3 views as frames (from above, from the front, from the right), 12 a second, level 9, "),
            "{}",
            lines[2]
        );
        let (_, frames) = read_png(&std::fs::read(&path).expect("the file"));
        assert_eq!(frames, frames_of(&drawing, &choices, 8, false, false));
        assert_eq!(frames.len(), 3);
        // two eyes: the red of the picture is what the left eye sees, the green and the
        // blue what the right eye sees
        let choices = small(&["stereo = 6"]);
        let lines = save(&path, Format::Png, &drawing, &choices).expect("a file");
        assert!(
            lines[2].contains("8 bits, red for the left eye and cyan for the right one, 6 degrees apart, level 9, "),
            "{}",
            lines[2]
        );
        let (shape, frames) = read_png(&std::fs::read(&path).expect("the file"));
        assert_eq!(shape.color_type, png::ColorType::Rgb);
        let look = Look {
            width: 48,
            height: 40,
            ..choices.look.clone()
        };
        let shot = Shot::new(&drawing, &look).expect("a view");
        let lights = shot.tone(true, false, false);
        let [left, right] = shot.pair(0).map(|eye| whole(&eye.image(&lights).data, 8));
        assert!(left != right);
        for (k, pixel) in frames[0].chunks(3).enumerate() {
            assert_eq!(pixel, [left[k], right[k], right[k]], "pixel {k}");
        }
        // both together: three frames for two eyes
        let lines =
            save(&path, Format::Png, &drawing, &small(&["stereo", "views"])).expect("a file");
        assert!(
            lines[2].contains("4 degrees apart, 3 views as frames"),
            "{}",
            lines[2]
        );
        for (more, why) in [
            (
                vec!["views", "frames = 3"],
                "views are three frames of their own: not with frames, slices, turn or grow",
            ),
            (
                vec!["views", "slices = 3"],
                "views are three frames of their own: not with frames, slices, turn or grow",
            ),
            (
                vec!["stereo", "gray"],
                "stereo is a picture of red and cyan for glasses of these colours: this does not go with gray",
            ),
            (
                vec!["stereo", "alpha"],
                "stereo is a picture of red and cyan for glasses of these colours: this does not go with alpha",
            ),
        ] {
            let choices = chosen(&more).expect("choices");
            assert_eq!(
                save(&path, Format::Png, &drawing, &choices).err(),
                Some(why.to_string())
            );
        }
        assert_eq!(
            chosen(&["stereo = 31"]).err(),
            Some("stereo is a number from 0.1 to 30, not 31".to_string())
        );
        let flat = drawn("walk", &["pi", "10", "50"]);
        let why = save(&path, Format::Png, &flat, &small(&["views"])).expect_err("a plane");
        assert!(
            why.starts_with("views and stereo show a drawing of space from several points"),
            "{why}"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn what_cannot_be_saved() {
        let dir = scratch("refused");
        let path = dir.join("no.png").to_string_lossy().into_owned();
        let drawing = drawn("walk", &["pi", "10", "50"]);
        for (format, more, why) in [
            (
                Format::Png,
                vec!["ascii"],
                "ascii belongs to the picture of characters, which is not saved",
            ),
            (
                Format::Png,
                vec!["bits = 12"],
                "a PNG file has 8 or 16 bits, not 12: save_jxl takes 1 to 16",
            ),
            (
                Format::Png,
                vec!["lossless"],
                "lossless belongs to save_jxl: a PNG file is lossless, and its level from 0 to 9 says how hard it is packed",
            ),
            (
                Format::Png,
                vec!["effort = 3"],
                "effort belongs to save_jxl: a PNG file is lossless, and its level from 0 to 9 says how hard it is packed",
            ),
            (
                Format::Png,
                vec!["depth"],
                "depth belongs to save_jxl: a PNG file is lossless, and its level from 0 to 9 says how hard it is packed",
            ),
            (
                Format::Jxl,
                vec!["level = 3"],
                "level belongs to save_png: save_jxl takes effort from 1 to 10, and quality or distance for a lossy picture",
            ),
            (
                Format::Jxl,
                vec!["quality = 50", "distance = 1"],
                "quality or distance, not both: quality = 90 is distance = 1",
            ),
            (
                Format::Jxl,
                vec!["lossless", "distance = 1"],
                "lossless, or lossy with quality or distance: not both",
            ),
            (Format::Png, vec!["grow"], "grow needs frames: frames = n"),
            (
                Format::Png,
                vec!["turn = 90"],
                "turn needs frames: frames = n",
            ),
            (
                Format::Png,
                vec!["slices = 3", "frames = 3"],
                "slices are the frames of the picture: slices = n or frames = n, not both",
            ),
            (
                Format::Png,
                vec!["channels = 3", "gray"],
                "channels = 3 is RGB: this does not go with gray",
            ),
            (
                Format::Png,
                vec!["channels = 1", "alpha"],
                "channels = 1 is gray: this does not go with alpha",
            ),
            (
                Format::Png,
                vec!["width = 4097", "height = 4096"],
                "a picture has 16777216 pixels at most (4096 by 4096), not 4097 by 4096",
            ),
        ] {
            let choices = chosen(&more).expect("choices");
            assert_eq!(
                save(&path, format, &drawing, &choices).err(),
                Some(why.to_string())
            );
        }
        assert_eq!(
            save("", Format::Png, &drawing, &Choices::default()).err(),
            Some("the name of the file is empty".to_string())
        );
        // lossless with the quality 100 or the distance 0 is no contradiction
        for more in [vec!["lossless", "quality = 100"], vec!["distance = 0"]] {
            let mut all = vec!["width = 16"];
            all.extend(more);
            let lines = save(
                &path,
                Format::Jxl,
                &drawing,
                &chosen(&all).expect("choices"),
            )
            .expect("a file");
            assert!(lines[2].contains(", lossless, "), "{}", lines[2]);
        }
        // nothing is left of a file that was refused, and no part of a file that was written
        let left: Vec<String> = std::fs::read_dir(&dir)
            .expect("the directory")
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .collect();
        assert_eq!(left, ["no.png"]);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn pictures_of_characters() {
        let drawing = drawn("walk", &["1/4", "4", "10"]);
        let lines = show(
            "walk",
            &drawing,
            &chosen(&["width = 12", "height = 3", "ascii"]).expect("choices"),
        )
        .expect("lines");
        // three rows, the rule and the ranges, then the facts
        assert_eq!(lines.len(), 5 + 2);
        assert_eq!(lines[5..], drawing.facts[..]);
        assert!(lines[..3].iter().any(|l| l.contains('*')));
        let usual = show("walk", &drawing, &Choices::default()).expect("lines");
        assert_eq!(usual.len(), 20 + 2 + 2);
        for (more, why) in [
            (
                vec!["gray"],
                "gray is chosen for a picture that is saved: save_png(\"walk.png\", walk(...), gray); the picture of characters takes width, height and ascii",
            ),
            (
                vec!["bits = 16"],
                "bits is chosen for a picture that is saved: save_png(\"walk.png\", walk(...), bits = ...); the picture of characters takes width, height and ascii",
            ),
            (
                vec!["width = 4"],
                "the width of a picture of characters is from 8 to 400, not 4",
            ),
            (
                vec!["height = 201"],
                "the height of a picture of characters is from 2 to 200, not 201",
            ),
        ] {
            let choices = chosen(&more).expect("choices");
            assert_eq!(
                show("walk", &drawing, &choices).err(),
                Some(why.to_string())
            );
        }
    }

    #[test]
    fn frames_a_second() {
        assert_eq!(rate(12.0), (12, 1));
        assert_eq!(rate(12.5), (25, 2));
        assert_eq!(rate(29.97), (2997, 100));
        assert_eq!(rate(0.5), (1, 2));
        assert_eq!(rate(0.001), (1, 1000));
    }

    #[test]
    fn a_picture_made_smaller() {
        // four pixels become one: their mean
        let image = imgfile::Image {
            width: 4,
            height: 2,
            colors: 1,
            alpha: false,
            data: vec![0.0, 1.0, 0.5, 0.5, 1.0, 0.0, 0.25, 0.75],
            depth: Some(vec![0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0]),
        };
        let half = smaller(&image, 2);
        assert_eq!((half.width, half.height, half.colors), (2, 1, 1));
        assert_eq!(half.data, [0.5, 0.5]);
        assert_eq!(half.depth, Some(vec![0.5, 1.0]));
        // where a part of the pixels is drawn, the colour is that of the drawn part and
        // alpha says how much is drawn
        let image = imgfile::Image {
            width: 2,
            height: 2,
            colors: 3,
            alpha: true,
            data: vec![
                1.0, 0.5, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.5, 0.0, 0.5,
            ],
            depth: None,
        };
        let one = smaller(&image, 2);
        assert_eq!((one.width, one.height, one.alpha), (1, 1, true));
        assert_eq!(one.data, [1.0, 0.5, 0.0, 0.375]);
        // nothing drawn: no colour, no alpha
        let empty = imgfile::Image {
            data: [0.3, 0.3, 0.3, 0.0].repeat(4),
            ..image
        };
        assert_eq!(smaller(&empty, 2).data, [0.0; 4]);
    }

    #[test]
    fn the_kind_of_file() {
        assert_eq!(format_of("save_png", "a.jxl"), Ok(Format::Png));
        assert_eq!(format_of("save_jxl", "a"), Ok(Format::Jxl));
        assert_eq!(format_of("save", "dir/a.PNG"), Ok(Format::Png));
        assert_eq!(format_of("save", "a.jxl"), Ok(Format::Jxl));
        assert!(format_of("save", "a.gif").is_err());
    }
}
