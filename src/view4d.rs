//! The shape of `plot4d` as an eye outside of its box sees it: one picture of the body of
//! voxels, with what is hidden left out and with light on it. The frames of `plot4d` stand
//! upright, so that with the usual sides the picture shows the real part of f(z) as heights
//! over the plane of z, in the colours of the imaginary part.
//!
//! As everywhere in the pictures, only `+ - * /` and the square root act on the 64-bit
//! floats, so that a picture is the same on every system.

use crate::eval::Session;
use crate::expr::Expr;
use crate::picture::{self, Choices};
use crate::plot::label;
use crate::plot4d::{self, Axis, EMPTY, Fill, Frames, PARTS, Shape, Tints};
use crate::raster::Rgb;
use crate::scene::{self, Body, Drawing, Face, Item, Legend, Space};

/// The most sides of cubes that a body may show.
const FACES: usize = 1 << 25;
/// What is chosen for the body, and is taken here.
const OF_BODIES: &[&str] = &[
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
    "thickness",
    "voxels",
];
/// What the picture of characters takes besides that.
const OF_CHARACTERS: &[&str] = &[
    "width",
    "height",
    "ascii",
    "title",
    "azimuth",
    "elevation",
    "roll",
    "zoom",
    "perspective",
    "viewpoint",
    "center",
    "centre",
    "scale",
    "fog",
];
const SHORT: [&str; 4] = ["re z", "im z", "re f(z)", "im f(z)"];

/// The sides of the cubes of a shape that lie open, and the number of its cubes.
fn faces(shape: &Shape) -> Result<(Vec<Face>, usize), String> {
    let [width, height, frames, _] = shape.sizes;
    let pixels = width * height;
    let mut taken = Frames::facing(shape);
    let (mut before, mut here, mut after) = (
        vec![EMPTY; pixels],
        vec![EMPTY; pixels],
        vec![EMPTY; pixels],
    );
    let (mut facings, mut next) = (vec![[0i8; 3]; pixels], vec![[0i8; 3]; pixels]);
    let mut load = |k: usize, levels: &mut [u16], faces: &mut [[i8; 3]]| {
        taken.take(k);
        levels.copy_from_slice(&taken.levels);
        if let Some(known) = &taken.facings {
            faces.copy_from_slice(known);
        }
    };
    load(0, &mut here, &mut facings);
    let mut found = Vec::new();
    let mut cubes = 0usize;
    for k in 0..frames {
        match k + 1 < frames {
            true => load(k + 1, &mut after, &mut next),
            false => after.fill(EMPTY),
        }
        for row in 0..height {
            for column in 0..width {
                let i = row * width + column;
                let level = here[i];
                if level == EMPTY {
                    continue;
                }
                cubes += 1;
                let open = [
                    column == 0 || here[i - 1] == EMPTY,
                    column + 1 == width || here[i + 1] == EMPTY,
                    row + 1 == height || here[i + width] == EMPTY,
                    row == 0 || here[i - width] == EMPTY,
                    k == 0 || before[i] == EMPTY,
                    after[i] == EMPTY,
                ];
                for (side, open) in open.into_iter().enumerate() {
                    if open {
                        found.push(Face {
                            at: [column as u16, (height - 1 - row) as u16, k as u16],
                            side: side as u8,
                            level,
                            facing: facings[i],
                        });
                    }
                }
            }
        }
        if found.len() > FACES {
            return Err(format!(
                "more than {FACES} sides of the cubes of this body lie open: take fewer voxels, or narrower ranges of the values (range_real, range_imag)"
            ));
        }
        std::mem::swap(&mut before, &mut here);
        std::mem::swap(&mut here, &mut after);
        std::mem::swap(&mut facings, &mut next);
    }
    Ok((found, cubes))
}

/// Where an eye stands that shows much of a body: the degrees around it and above its
/// floor (`below`: under it). Of three heights the lowest from which four fifths are seen
/// of what is seen from straight above; around the body the place that shows most.
fn eye(body: &Body, sides: [f64; 3], below: bool) -> (f64, f64) {
    const GRID: usize = 96;
    let radius = (sides[0] * sides[0] + sides[1] * sides[1] + sides[2] * sides[2]).sqrt();
    let cell = 2.0 * radius / GRID as f64;
    // the middles of open sides in the box of the picture; of a large body not all of them
    let stride = (body.faces.len() / 300_000).max(1);
    let middles: Vec<[f64; 3]> = body
        .faces
        .iter()
        .step_by(stride)
        .map(|face| {
            let axis = usize::from(face.side / 2).min(2);
            let mut at = face.at.map(|v| f64::from(v) + 0.5);
            at[axis] += if face.side % 2 == 1 { 0.5 } else { -0.5 };
            [0, 1, 2].map(|k| (2.0 * at[k] / body.size[k] as f64 - 1.0) * sides[k])
        })
        .collect();
    let dot = |a: &[f64; 3], b: &[f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let mut nearest = vec![f64::INFINITY; GRID * GRID];
    // how many of the sides are no farther than what is nearest at their place
    let mut seen = |azimuth: f64, elevation: f64| -> usize {
        let [right, up, toward] = scene::directions(azimuth, elevation, 0.0);
        let place = |p: &[f64; 3]| {
            let cell_of = |along: &[f64; 3]| {
                (((dot(p, along) + radius) / cell).max(0.0) as usize).min(GRID - 1)
            };
            (cell_of(&up) * GRID + cell_of(&right), -dot(p, &toward))
        };
        nearest.fill(f64::INFINITY);
        for p in &middles {
            let (at, depth) = place(p);
            if depth < nearest[at] {
                nearest[at] = depth;
            }
        }
        middles
            .iter()
            .filter(|p| {
                let (at, depth) = place(p);
                depth <= nearest[at] + 1.5 * cell
            })
            .count()
    };
    let sign = if below { -1.0 } else { 1.0 };
    let all = seen(30.0, sign * 90.0);
    let mut best = (30.0, sign * 25.0, 0usize);
    for elevation in [25.0, 40.0, 55.0] {
        let mut most = (30.0, 0usize);
        for azimuth in [30.0, 60.0, 120.0, 150.0, 210.0, 240.0, 300.0, 330.0] {
            let count = seen(azimuth, sign * elevation);
            if count > most.1 {
                most = (azimuth, count);
            }
        }
        if most.1 > best.2 {
            best = (most.0, sign * elevation, most.1);
        }
        if 5 * most.1 >= 4 * all {
            return (most.0, sign * elevation);
        }
    }
    (best.0, best.1)
}

impl Session {
    /// The drawing of `view4d(f, z, a, b)` with what is chosen for it there and in the
    /// arguments `more` of the command that saves it. `pixels`: for a picture of pixels.
    pub(crate) fn view4d(
        &mut self,
        raw: &[Expr],
        more: &[Expr],
        pixels: bool,
        fast: bool,
    ) -> Result<(Drawing, Choices), String> {
        let (plain, mut choices) = self.chosen("view4d", raw, more)?;
        choices.fast = fast;
        let (f, var, re, im) = self.function_in_a_box("view4d", &plain)?;
        if let Some(other) = choices.given.iter().find(|g| {
            *g == "grow" || (!pixels && !OF_BODIES.iter().chain(OF_CHARACTERS).any(|n| n == g))
        }) {
            return Err(match other.as_str() {
                "grow" => "grow belongs to the walks, whose steps come one after another".into(),
                _ => format!(
                    "{other} is chosen for a picture that is saved: save_png(\"view4d.png\", view4d(...), {other}{}); the picture of characters takes {}, {}",
                    if picture::WORDS.contains(&other.as_str()) {
                        ""
                    } else {
                        " = ..."
                    },
                    OF_CHARACTERS.join(", "),
                    OF_BODIES.join(", ")
                ),
            });
        }
        let function = format!("{f}");
        let mut plan = plot4d::plan_of("view4d", f, var, (re, im), &choices)?;
        // as many cubes along a side as half the pixels across the picture show well
        let usual = match (pixels, choices.width) {
            (true, width) => (width.unwrap_or(if fast { 256 } else { 1024 }) / 2).clamp(32, 512),
            (false, width) => width.unwrap_or(64).clamp(32, 128),
        };
        let [across, upwards, along] = choices.voxels.unwrap_or([usual; 3]);
        let colours = match pixels {
            true => choices.colors.unwrap_or(if fast { 256 } else { 512 }),
            false => choices.colors.unwrap_or(2),
        };
        plan.sizes = [across, upwards, along, colours];
        let gray = choices.gray || choices.channels.is_some_and(|n| n <= 2);
        let (name, mut stops) = match &choices.look.palette {
            Some(stops) => (
                crate::raster::PALETTES
                    .iter()
                    .find(|(_, known)| *known == stops.as_slice())
                    .map(|(name, _)| *name),
                stops.clone(),
            ),
            None => {
                let name = if gray { "gray" } else { "rainbow" };
                (Some(name), scene::palette(name).unwrap_or_default())
            }
        };
        if choices.reverse {
            stops.reverse();
        }
        let tints: Vec<Rgb> = match Tints::new(colours, Some(stops.as_slice())) {
            Tints::Rgb(colours) => colours,
            Tints::Gray(_) => Vec::new(),
        };
        let shape = plot4d::shape(&plan)?;
        let (faces, cubes) = faces(&shape)?;
        // the sides of the box: what lies on the width, the height and the frames
        let part = |axis: Axis| shape.axes.iter().position(|a| *a == axis).unwrap_or(0);
        let parts = [Axis::Width, Axis::Height, Axis::Frame].map(part);
        let coloured = part(Axis::Color);
        // (a body that is turned or stretched has no numbers along its box any more)
        let moved = shape.turn.is_some() || shape.normalize;
        let ends = match moved {
            true => [across, upwards, along].map(|n| (0.0, n as f64)),
            false => parts.map(|p| shape.ranges[p]),
        };
        let longest = across.max(upwards).max(along) as f64;
        let sides = [across, upwards, along].map(|n| n as f64 / longest);
        let body = Body {
            size: [across, upwards, along],
            ends,
            faces,
            tints: tints.clone(),
        };
        let open = body.faces.len();
        // the eye, where none is chosen
        let look = &mut choices.look;
        let found = (look.azimuth.is_none()
            && look.elevation.is_none()
            && look.viewpoint.is_none()
            && !look.views)
            .then(|| eye(&body, sides, shape.fill == Fill::Above));
        if let Some((azimuth, elevation)) = found {
            look.azimuth = Some(azimuth);
            look.elevation = Some(elevation);
        }
        look.thickness = 0.0;
        choices.given.retain(|g| !OF_BODIES.contains(&g.as_str()));
        let mut drawing = Drawing::new(Space::Solid, vec![Item::Body(body)]);
        drawing.stretch = Some(ends);
        drawing.sides = sides;
        drawing.axes = true;
        drawing.smooth = true;
        drawing.fog = 0.25;
        drawing.axis_names = Some(match moved {
            true => ["width", "height", "frames"].map(String::from),
            false => parts.map(|p| SHORT[p].to_string()),
        });
        if !shape.normalize {
            drawing.legend = Some(Legend {
                name: SHORT[coloured].to_string(),
                ends: shape.ranges[coloured],
                colors: tints,
            });
        }
        let scale = format!(
            "{colours} colours of the palette {}{}, the least value first",
            name.unwrap_or("that is given"),
            if choices.reverse { " reversed" } else { "" }
        );
        let side = |k: usize| {
            let (low, high) = shape.ranges[parts[k]];
            format!("{} from {} to {}", PARTS[parts[k]], label(low), label(high))
        };
        let (low, high) = shape.ranges[coloured];
        let mut facts = vec![
            format!(
                "view4d of {function}: the shape of plot4d as an eye outside of its box sees it; the frames of plot4d stand upright"
            ),
            format!(
                "the box: x is {}; y is {}; z, upright, is {}; the colours are {} from {} to {}{}",
                side(0),
                side(1),
                side(2),
                PARTS[coloured],
                label(low),
                label(high),
                if pixels {
                    format!(" ({scale})")
                } else {
                    String::new()
                }
            ),
        ];
        facts.extend(plot4d::notes(&shape));
        facts.push(format!(
            "the body has {cubes} cubes, of which {open} sides lie open"
        ));
        if let Some((azimuth, elevation)) = found {
            facts.push(format!(
                "the eye stands at azimuth = {}, elevation = {} (found: the lowest of three heights that shows most of what can be seen, and around the box the place that shows most); azimuth, elevation, roll, viewpoint, perspective and zoom choose another",
                label(azimuth),
                label(elevation)
            ));
        }
        drawing.facts = facts;
        Ok((drawing, choices))
    }

    /// `view4d(f, z, a, b)` as a statement: the picture as lines of characters.
    pub(crate) fn view4d_shown(&mut self, raw: &[Expr]) -> Result<Vec<String>, String> {
        let (drawing, choices) = self.view4d(raw, &[], false, false)?;
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
        let height = limited(choices.height, "height", (2, 200), 24)?;
        let mut lines = picture::worded(
            scene::lights(&drawing, &choices.look, width, height, choices.ascii)?,
            &choices.look,
        );
        lines.extend(drawing.facts.iter().cloned());
        lines.push(
            "save_png(\"view4d.png\", view4d(...)) saves the picture in colours: 1024 x 1024 pixels of a body of 512 x 512 x 512 voxels".to_string(),
        );
        Ok(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plot4d::Plan;
    use crate::scene::{Look, Shot};
    use std::rc::Rc;

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
            axes: [Axis::Width, Axis::Height, Axis::Frame, Axis::Color],
            sizes,
            range_real: None,
            range_imag: None,
            fill: Fill::Surface,
            thickness: 1,
            rotate: None,
            normalize: false,
        }
    }

    /// The drawing of a command as it is written, with its choices.
    fn drawn(text: &str) -> (Drawing, Choices) {
        let call = crate::parser::parse(text)
            .expect("a command")
            .stmts
            .remove(0);
        let Expr::Fn(_, args) = call else {
            panic!("{text} is no command")
        };
        Session::new()
            .view4d(&args, &[], true, false)
            .expect("a drawing")
    }

    fn body(drawing: &Drawing) -> &Body {
        match &drawing.items[0] {
            Item::Body(body) => body,
            _ => panic!("no body"),
        }
    }

    /// The open sides of a shape, found in the whole box of its voxels at once.
    fn plainly(shape: &Shape) -> (Vec<([u16; 3], u8, u16)>, usize) {
        let [width, height, frames, _] = shape.sizes;
        let mut taken = Frames::new(shape);
        let mut all = Vec::new();
        for k in 0..frames {
            taken.take(k);
            all.extend_from_slice(&taken.levels);
        }
        let at = |x: i64, y: i64, k: i64| -> u16 {
            let inside = x >= 0
                && y >= 0
                && k >= 0
                && x < width as i64
                && y < height as i64
                && k < frames as i64;
            match inside {
                true => {
                    all[(k as usize * height + (height - 1 - y as usize)) * width + x as usize]
                }
                false => EMPTY,
            }
        };
        let (mut found, mut cubes) = (Vec::new(), 0);
        for k in 0..frames as i64 {
            for y in 0..height as i64 {
                for x in 0..width as i64 {
                    let level = at(x, y, k);
                    if level == EMPTY {
                        continue;
                    }
                    cubes += 1;
                    let beside = [
                        (x - 1, y, k),
                        (x + 1, y, k),
                        (x, y - 1, k),
                        (x, y + 1, k),
                        (x, y, k - 1),
                        (x, y, k + 1),
                    ];
                    for (side, (i, j, l)) in beside.into_iter().enumerate() {
                        if at(i, j, l) == EMPTY {
                            found.push(([x as u16, y as u16, k as u16], side as u8, level));
                        }
                    }
                }
            }
        }
        found.sort_unstable();
        (found, cubes)
    }

    #[test]
    fn the_open_sides_of_the_cubes() {
        let mut plans = vec![
            plan("z", [8, 8, 8, 4]),
            plan("z^2", [12, 10, 9, 5]),
            plan("sin(z)", [16, 16, 16, 8]),
            plan("1/z", [16, 12, 20, 8]),
        ];
        let more: Vec<Plan> = plans
            .iter()
            .flat_map(|p| {
                let with = |change: &dyn Fn(&mut Plan)| {
                    let mut other = plan("z", p.sizes);
                    other.f = p.f.clone();
                    change(&mut other);
                    other
                };
                vec![
                    with(&|p| p.fill = Fill::Below),
                    with(&|p| p.fill = Fill::Above),
                    with(&|p| p.thickness = 3),
                    with(&|p| p.rotate = Some([30.0, 20.0, 10.0])),
                    with(&|p| {
                        p.fill = Fill::Below;
                        p.rotate = Some([90.0, 0.0, 0.0]);
                    }),
                    with(&|p| p.normalize = true),
                    with(&|p| {
                        p.fill = Fill::Below;
                        p.normalize = true;
                    }),
                    with(&|p| {
                        p.axes = [Axis::Color, Axis::Frame, Axis::Width, Axis::Height];
                    }),
                ]
            })
            .collect();
        plans.extend(more);
        for plan in &plans {
            let shape = plot4d::shape(plan).expect("a shape");
            let (found, cubes) = faces(&shape).expect("sides");
            let mut mine: Vec<([u16; 3], u8, u16)> =
                found.iter().map(|f| (f.at, f.side, f.level)).collect();
            mine.sort_unstable();
            let (theirs, count) = plainly(&shape);
            assert_eq!(cubes, count, "{}", plan.f);
            assert_eq!(mine, theirs, "{}", plan.f);
            // a surface of one layer has as many cubes as voxels
            if plan.fill == Fill::Surface && plan.thickness == 1 {
                assert_eq!(cubes, shape.voxels, "{}", plan.f);
            }
            // every cube of a box shows its sides at the walls of the box
            assert!(found.iter().all(|f| f.side < 6));
        }
    }

    #[test]
    fn where_the_surface_faces() {
        // the heights are the real part of z: a plane that rises across the width
        let shape = plot4d::shape(&plan("z", [16, 16, 16, 4])).expect("a shape");
        let (found, _) = faces(&shape).expect("sides");
        assert!(!found.is_empty());
        for face in &found {
            let [across, upwards, along] = face.facing.map(i32::from);
            assert_eq!(upwards, 0, "{face:?}");
            assert_eq!(across, -along, "{face:?}");
            assert!((88..=92).contains(&across.abs()), "{face:?}");
        }
        // the real part of i*z falls upwards
        let shape = plot4d::shape(&plan("i*z", [16, 16, 16, 4])).expect("a shape");
        let (found, _) = faces(&shape).expect("sides");
        for face in &found {
            let [across, upwards, along] = face.facing.map(i32::from);
            assert_eq!(across, 0, "{face:?}");
            assert_eq!(upwards, along, "{face:?}");
        }
        // a flat floor faces along the frames; the cubes under it face as their sides do
        let mut flat = plan("3 + i*re(z)", [8, 8, 8, 4]);
        flat.range_real = Some((0.0, 4.0));
        flat.fill = Fill::Below;
        let shape = plot4d::shape(&flat).expect("a shape");
        let (found, cubes) = faces(&shape).expect("sides");
        // (3 of 0 to 4 is the frame 6 of 8: the floor and what is under it)
        assert_eq!(cubes, 8 * 8 * 7);
        for face in &found {
            match face.at[2] {
                6 => assert_eq!(face.facing.map(i32::from).map(i32::abs), [0, 0, 127]),
                _ => assert_eq!(face.facing, [0; 3]),
            }
        }
        assert_eq!(found.iter().filter(|f| f.side == 5).count(), 64);
        assert_eq!(found.iter().filter(|f| f.side == 4).count(), 64);
        assert_eq!(found.iter().filter(|f| f.side < 4).count(), 4 * 8 * 7);
    }

    #[test]
    fn the_eye_that_is_found() {
        // a wedge that is high where the real part of z is large is seen from its low side
        let (drawing, choices) = drawn("view4d(z, voxels = 24, fill = below)");
        let azimuth = choices.look.azimuth.expect("an eye");
        assert!(azimuth > 180.0 && azimuth < 360.0, "{azimuth}");
        assert_eq!(choices.look.elevation, Some(25.0));
        assert!(drawing.facts.iter().any(|f| f.contains("the eye stands")));
        // and the mirrored wedge from the other side
        let (_, choices) = drawn("view4d(-z, voxels = 24, fill = below)");
        let azimuth = choices.look.azimuth.expect("an eye");
        assert!(azimuth > 0.0 && azimuth < 180.0, "{azimuth}");
        // a body that hangs is seen from below
        let (_, choices) = drawn("view4d(z, voxels = 24, fill = above)");
        assert_eq!(choices.look.elevation, Some(-25.0));
        // the same answer every time
        let (_, again) = drawn("view4d(z, voxels = 24, fill = above)");
        assert_eq!(choices.look.azimuth, again.look.azimuth);
        // an eye that is chosen stays, and nothing is said about one that was found
        for text in [
            "view4d(z, voxels = 8, azimuth = 77)",
            "view4d(z, voxels = 8, elevation = 10)",
            "view4d(z, voxels = 8, viewpoint = [1, 2, 3])",
            "view4d(z, voxels = 8, views)",
        ] {
            let (drawing, choices) = drawn(text);
            assert!(
                !drawing.facts.iter().any(|f| f.contains("the eye stands")),
                "{text}"
            );
            if text.contains("azimuth") {
                assert_eq!(choices.look.azimuth, Some(77.0));
                assert_eq!(choices.look.elevation, None);
            }
        }
    }

    #[test]
    fn what_is_chosen_for_the_body() {
        let (drawing, choices) = drawn(
            "view4d(z^2, voxels = [8, 12, 16], colors = 5, thickness = 2, fill = above, width = 100, palette = heat)",
        );
        let made = body(&drawing);
        assert_eq!(made.size, [8, 12, 16]);
        assert_eq!(made.tints.len(), 5);
        assert_eq!(drawing.sides, [0.5, 0.75, 1.0]);
        // the sides of the box are those of the numbers
        assert_eq!(made.ends, [(-2.0, 2.0), (-2.0, 2.0), (-4.0, 4.0)]);
        assert_eq!(
            drawing.axis_names,
            Some(["re z", "im z", "re f(z)"].map(String::from))
        );
        let legend = drawing.legend.as_ref().expect("the colours");
        assert_eq!((legend.name.as_str(), legend.ends), ("im f(z)", (-8.0, 8.0)));
        // what belongs to the body is taken; the rest is left to the picture
        assert_eq!(choices.given, ["width", "palette"]);
        assert_eq!(choices.look.thickness, 0.0);
        // without a word the cubes are half the pixels across, 512 at most
        for (text, cubes) in [
            ("view4d(z, width = 100)", 50),
            ("view4d(z, width = 40)", 32),
            ("view4d(z, width = 2000)", 512),
        ] {
            assert_eq!(body_of(text), [cubes; 3], "{text}");
        }
        // a body that is turned or stretched has no numbers along its box
        for text in [
            "view4d(z^2, voxels = 8, rotate(30))",
            "view4d(z^2, voxels = 8, normalize)",
        ] {
            let (drawing, _) = drawn(text);
            assert_eq!(body(&drawing).ends, [(0.0, 8.0); 3], "{text}");
            assert_eq!(
                drawing.axis_names,
                Some(["width", "height", "frames"].map(String::from)),
                "{text}"
            );
            assert_eq!(drawing.legend.is_some(), text.contains("rotate"), "{text}");
        }
    }

    fn body_of(text: &str) -> [usize; 3] {
        let (drawing, _) = drawn(text);
        body(&drawing).size
    }

    /// The picture of a drawing as its choices say, without making it finer.
    fn picture(drawing: &Drawing, choices: &Choices, size: usize) -> crate::raster::Raster {
        let look = Look {
            width: size,
            height: size,
            margin: 0.0,
            ..choices.look.clone()
        };
        Shot::new(drawing, &look).expect("a view").raster(0)
    }

    #[test]
    fn the_picture_of_a_body() {
        // from straight above the floor of 3 + i*re(z) fills the picture; its colours are
        // the imaginary part of the value, which is the real part of z: darker to the left
        let (drawing, choices) = drawn(
            "view4d(3 + i*re(z), voxels = 16, range_real = [0, 4], azimuth = 0, elevation = 90, noaxes, fog = 0, palette = [black, white], colors = 16)",
        );
        let raster = picture(&drawing, &choices, 64);
        // (a rim as wide as a line stays free)
        for y in 0..64 {
            for x in 0..64 {
                let inside = (1..63).contains(&x) && (1..63).contains(&y);
                assert_eq!(raster.cover(x, y) == 1.0, inside, "{x} {y}");
            }
        }
        let light = |x: usize, y: usize| crate::raster::luma(raster.color(x, y));
        for y in [2, 30, 61] {
            assert!(light(2, y) < light(20, y) && light(20, y) < light(61, y));
            assert_eq!(light(8, y), light(8, 40));
        }
        // the floor faces the eye: as light as the light from the upper left makes it
        let facing = 0.75 / (0.4f64 * 0.4 + 0.5 * 0.5 + 0.75 * 0.75).sqrt();
        let bright = (0.35 + 0.65 * facing) as f32;
        let [red, green, blue] = raster.color(62, 5);
        assert!((red - bright).abs() < 1e-6 && red == green && green == blue);

        // a body under the floor seen from the front: the wall of the least imaginary part
        // of z hides everything else
        let (drawing, choices) = drawn(
            "view4d(3 + z, voxels = 16, range_real = [0, 6], fill = below, azimuth = 0, elevation = 0, noaxes, fog = 0, palette = [[1, 0, 0], [0, 0, 1]], colors = 16)",
        );
        let raster = picture(&drawing, &choices, 64);
        // (the imaginary part of the value is that of z: the front wall has the first colour)
        let front = raster.color(3, 60);
        assert!(front[0] > 0.5 && front[1] == 0.0 && front[2] == 0.0, "{front:?}");
        for x in [2, 20, 40, 61] {
            assert_eq!(raster.color(x, 61), front);
        }
        // the wall is as high as the real part of z says: low to the left, high to the right
        assert_eq!(raster.cover(2, 20), 0.0);
        assert_eq!(raster.cover(61, 20), 1.0);
        // from behind the last colour shows
        let (drawing, choices) = drawn(
            "view4d(3 + z, voxels = 16, range_real = [0, 6], fill = below, azimuth = 180, elevation = 0, noaxes, fog = 0, palette = [[1, 0, 0], [0, 0, 1]], colors = 16)",
        );
        let raster = picture(&drawing, &choices, 64);
        let back = raster.color(30, 61);
        assert!(back[2] > 0.5 && back[0] == 0.0, "{back:?}");
        // one colour for the whole body leaves the light alone
        let (drawing, choices) = drawn(
            "view4d(3 + i*re(z), voxels = 16, range_real = [0, 4], azimuth = 0, elevation = 90, noaxes, fog = 0, color = white)",
        );
        let raster = picture(&drawing, &choices, 32);
        assert_eq!(raster.color(3, 3), raster.color(28, 20));
    }

    #[test]
    fn pictures_of_characters() {
        let mut session = Session::new();
        let shown = |session: &mut Session, text: &str| {
            let call = crate::parser::parse(text)
                .expect("a command")
                .stmts
                .remove(0);
            let Expr::Fn(_, args) = call else {
                panic!("{text} is no command")
            };
            session.view4d_shown(&args)
        };
        let lines = shown(&mut session, "view4d(z^2, width = 30, height = 9)").expect("lines");
        assert!(lines[..9].iter().all(|l| l.chars().count() <= 30));
        assert!(
            lines[..9]
                .iter()
                .flat_map(|l| l.chars())
                .all(|c| c == ' ' || ('\u{2800}'..='\u{28ff}').contains(&c))
        );
        assert!(lines[..9].iter().any(|l| l.chars().count() > 10));
        assert!(lines[9].starts_with("view4d of z^2: "));
        assert!(lines.last().expect("a line").starts_with("save_png("));
        let lines =
            shown(&mut session, "view4d(z^2, width = 30, height = 9, ascii)").expect("lines");
        assert!(
            lines[..9]
                .iter()
                .flat_map(|l| l.chars())
                .all(|c| " .:-=+*#%@".contains(c))
        );
        // a title stands above the picture
        let lines = shown(
            &mut session,
            "view4d(z^2, width = 30, height = 9, title = \"a square\")",
        )
        .expect("lines");
        assert_eq!(lines[0], "a square");
        for (text, why) in [
            ("view4d(z^2, palette = heat)", "palette is chosen for a picture that is saved"),
            ("view4d(z^2, frames = 3)", "frames is chosen for a picture that is saved"),
            ("view4d(z^2, grow)", "grow belongs to the walks"),
            ("view4d(z^2, width = 3)", "the width of a picture of characters is from 8 to 400"),
            ("view4d(z^2, thickness = 1.5)", "the thickness of view4d is a whole number"),
            ("view4d(x*y)", "has several names: view4d(f, z)"),
            ("view4d(7)", "view4d draws a function of a complex number"),
            ("view4d(z, z, 0)", "view4d(f, z, a, b)"),
        ] {
            let said = shown(&mut session, text).expect_err(text);
            assert!(said.contains(why), "{text}: {said}");
        }
    }
}
