//! Solids of four dimensions: balls, the regular polychora, products of disks, balls and
//! polygons, cones, capsules, the tori of 4D, bodies of revolution, superquadrics and the
//! bodies of the bracket notation. A solid is `solid4d("name", numbers...)`; it can be
//! turned in the six planes of 4D, scaled, moved and cut. Whether a point is inside, its
//! bulk and its surface, its vertices and edges; as a body of voxels for plot4d and view4d
//! (three coordinates across, upwards and along the frames, the fourth in the colours),
//! and as a wire frame seen from outside the fourth dimension.

use crate::arrays::{tidy, word};
use crate::canon::{add, div, mul, pow};
use crate::expr::Expr;
use crate::scene::{Drawing, Item, Paint, Point, Space};
use crate::texts::text;

type Res = Result<Expr, String>;
type P4 = [f64; 4];
/// The vertices of a polytope and its sides n.x <= h.
type Hull = (Vec<P4>, Vec<(P4, f64)>);

/// A regular polygon with the circumradius 1 and a vertex on the first axis: whether a
/// point is inside, and how far out it lies (1 on the border).
fn polygon_gauge(sides: usize, x: f64, y: f64) -> f64 {
    let turn = std::f64::consts::PI / sides as f64;
    let mut most: f64 = 0.0;
    for k in 0..sides {
        let angle = (2 * k + 1) as f64 * turn;
        most = most.max(x * angle.cos() + y * angle.sin());
    }
    most / turn.cos()
}

/// A body of the bracket notation: round brackets are the root of the sum of the squares,
/// square ones the largest, angle ones the sum, of what they hold.
#[derive(Clone, Debug, PartialEq)]
enum Bracket {
    Axis(usize),
    Round(Vec<Bracket>),
    Square(Vec<Bracket>),
    Angle(Vec<Bracket>),
}

impl Bracket {
    fn parse(shape: &str) -> Result<Bracket, String> {
        let wrong = || {
            format!(
                "a body of brackets is written with I for an axis (or x, y, z, w) inside ( ), [ ] and < >, as \"[(II)(II)]\" for the duocylinder: not \"{shape}\""
            )
        };
        let signs: Vec<char> = shape.chars().filter(|c| !c.is_whitespace()).collect();
        let mut at = 0;
        let mut next_axis = 0;
        fn group(signs: &[char], at: &mut usize, next_axis: &mut usize) -> Option<Bracket> {
            let open = *signs.get(*at)?;
            let close = match open {
                '(' => ')',
                '[' => ']',
                '<' => '>',
                'I' | 'i' => {
                    *at += 1;
                    *next_axis += 1;
                    return (*next_axis <= 4).then(|| Bracket::Axis(*next_axis - 1));
                }
                'x' | 'y' | 'z' | 'w' => {
                    *at += 1;
                    return Some(Bracket::Axis("xyzw".find(open)?));
                }
                _ => return None,
            };
            *at += 1;
            let mut parts = Vec::new();
            while *signs.get(*at)? != close {
                parts.push(group(signs, at, next_axis)?);
            }
            *at += 1;
            if parts.is_empty() {
                return None;
            }
            Some(match open {
                '(' => Bracket::Round(parts),
                '[' => Bracket::Square(parts),
                _ => Bracket::Angle(parts),
            })
        }
        let found = group(&signs, &mut at, &mut next_axis).ok_or_else(wrong)?;
        if at != signs.len() || matches!(found, Bracket::Axis(_)) {
            return Err(wrong());
        }
        // every axis takes part
        let mut seen = [false; 4];
        fn mark(b: &Bracket, seen: &mut [bool; 4]) {
            match b {
                Bracket::Axis(k) => seen[*k] = true,
                Bracket::Round(p) | Bracket::Square(p) | Bracket::Angle(p) => {
                    p.iter().for_each(|x| mark(x, seen))
                }
            }
        }
        mark(&found, &mut seen);
        match seen.iter().all(|s| *s) {
            true => Ok(found),
            false => Err(wrong()),
        }
    }

    fn size(&self, p: &P4) -> f64 {
        match self {
            Bracket::Axis(k) => p[*k].abs(),
            Bracket::Round(parts) => parts.iter().map(|b| b.size(p).powi(2)).sum::<f64>().sqrt(),
            Bracket::Square(parts) => parts.iter().map(|b| b.size(p)).fold(0.0, f64::max),
            Bracket::Angle(parts) => parts.iter().map(|b| b.size(p)).sum(),
        }
    }
}

#[derive(Clone, Debug)]
enum Kind {
    Ball,
    /// The half-spaces n.x <= h of a convex polytope, and its vertices.
    Polytope {
        sides: Vec<(P4, f64)>,
        vertices: Vec<P4>,
    },
    Duopyramid(usize, usize),
    Spherinder,
    Cubinder,
    Duocylinder,
    PrismicCylinder(usize),
    /// A cone over a ball: the radius at w = -1 and at w = +1.
    Frustum(f64, f64),
    Coninder,
    Cylindrone,
    Dicone,
    Capsule(f64),
    Spheritorus(f64, f64),
    Torisphere(f64, f64),
    Tiger(f64, f64, f64),
    Ditorus(f64, f64, f64),
    Torinder(f64, f64),
    Paraboloid,
    Hyperboloid(f64),
    Segment(f64, f64),
    Lens(f64),
    Superquadric(f64),
    Brackets(Bracket),
    /// A regular polychoron whose cells pass through each other.
    Star(Box<Star>),
    /// A ball with spikes towards directions: the radius of the ball, the length of a
    /// spike and how sharp it is.
    Spiked(Vec<P4>, f64, f64, f64),
    /// The wall around the surface that a circle (a segment, for a band) sweeps while
    /// its plane goes once around xy and makes half-turns in zw: the two radii, half of
    /// the thickness, the half-turns, and the sines and cosines of the turns in xy and in
    /// zw at the places where the distance is tried first.
    Wall {
        band: bool,
        major: f64,
        minor: f64,
        tube: f64,
        twist: usize,
        turns: Vec<[f64; 4]>,
    },
    /// The Reuleaux triangle of the width 2 turned around w.
    Rotor,
    /// What the balls of one radius around some points have in common.
    Common(Vec<P4>, f64),
}

/// A regular polychoron {p, q, r} as all that it winds around: the simplices that its
/// middle, the middle of a cell, the middle of a face of that cell and an edge of that
/// face span. How many of them hold a point is the density there.
#[derive(Clone, Debug)]
struct Star {
    vertices: Vec<P4>,
    edges: Vec<(usize, usize)>,
    /// The hyperplanes of the cells, all at one distance from the middle, each with the
    /// directions that give a place of it in the frame of the first cell.
    normals: Vec<P4>,
    frames: Vec<[P4; 3]>,
    height: f64,
    /// The first cell in its frame: the directions of its faces, all at one distance,
    /// and its simplices: a face with the three planes through the middle of the cell
    /// that bound the simplex, each facing inwards.
    faces: Vec<[f64; 3]>,
    face_dist: f64,
    flags: Vec<(usize, [[f64; 3]; 3])>,
}

impl Star {
    fn of(marks: &[f64]) -> Result<Star, String> {
        let (mirrors, dual) = mirrors_of(marks)?;
        let unit = |v: P4| {
            let size = dot(&v, &v).sqrt();
            v.map(|x| x / size)
        };
        let image = |v: &P4, m: &P4| {
            let twice = 2.0 * dot(v, m);
            [0, 1, 2, 3].map(|k| v[k] - twice * m[k])
        };
        // a vertex, the edge to its image in the first mirror, the face that the first
        // two mirrors keep and the cell that the first three keep
        let v0 = unit(dual[0]);
        let v1 = image(&v0, &mirrors[0]);
        let n0 = unit(dual[3]);
        let height = dot(&n0, &v0);
        if !(height > 1e-9 && height < 1.0 - 1e-9) {
            return Err("the cells of this polychoron do not surround its middle".into());
        }
        let c0 = n0.map(|x| x * height);
        let along = dot(&dual[2], &n0);
        let e2 = unit([0, 1, 2, 3].map(|k| dual[2][k] - along * n0[k]));
        let (up, aside) = (dot(&v0, &n0), dot(&v0, &e2));
        let f0 = [0, 1, 2, 3].map(|k| n0[k] * up + e2[k] * aside);
        let vertices = orbit(&mirrors, v0)?;
        // the edges: the images of the first one
        let mut places = Points::default();
        for v in &vertices {
            places.add(*v);
        }
        let moved: Vec<Vec<usize>> = mirrors
            .iter()
            .map(|m| {
                vertices
                    .iter()
                    .map(|v| places.find(&image(v, m)).unwrap_or(0))
                    .collect()
            })
            .collect();
        let first = (
            0,
            places
                .find(&v1)
                .ok_or("the edges of this polychoron are not found")?,
        );
        let mut edges = std::collections::BTreeSet::from([first]);
        let mut queue = vec![first];
        while let Some((a, b)) = queue.pop() {
            for m in &moved {
                let pair = (m[a].min(m[b]), m[a].max(m[b]));
                if edges.insert(pair) {
                    queue.push(pair);
                }
            }
        }
        // the frame of the first cell
        let mut frame = [[0.0; 4]; 3];
        for (i, towards) in [f0, v0, v1].iter().enumerate() {
            let mut d = [0, 1, 2, 3].map(|k| towards[k] - c0[k]);
            for b in frame.iter().take(i) {
                let along = dot(&d, b);
                d = [0, 1, 2, 3].map(|k| d[k] - along * b[k]);
            }
            let size = dot(&d, &d).sqrt();
            if size < 1e-9 {
                return Err("the cells of this polychoron are flat".into());
            }
            frame[i] = d.map(|x| x / size);
        }
        let local = |p: &P4| -> [f64; 3] {
            let d = [0, 1, 2, 3].map(|k| p[k] - c0[k]);
            [dot(&d, &frame[0]), dot(&d, &frame[1]), dot(&d, &frame[2])]
        };
        // every cell, with the frame that the mirrors carry to it
        let mut cells = Points::default();
        cells.add(n0);
        let mut frames = vec![frame];
        let mut at = 0;
        while at < cells.all.len() {
            let (from, f) = (cells.all[at], frames[at]);
            for m in &mirrors {
                if cells.add(image(&from, m)) {
                    frames.push([image(&f[0], m), image(&f[1], m), image(&f[2], m)]);
                }
            }
            if frames.len() > 14400 {
                return Err("the group of these mirrors is not finite".into());
            }
            at += 1;
        }
        // the simplices of the first cell: the images of a face with an edge in its mirrors
        let (mut face_set, mut corner_set) = (Points::default(), Points::default());
        let index = |set: &mut Points, p: &P4| -> usize {
            set.find(p).unwrap_or_else(|| {
                set.add(*p);
                set.all.len() - 1
            })
        };
        let mut seen = std::collections::HashSet::new();
        let (fi, ai, bi) = (
            index(&mut face_set, &f0),
            index(&mut corner_set, &v0),
            index(&mut corner_set, &v1),
        );
        seen.insert((fi, ai.min(bi), ai.max(bi)));
        let mut flags_in_space = vec![(f0, v0, v1)];
        let mut at = 0;
        while at < flags_in_space.len() {
            let (f, a, b) = flags_in_space[at];
            for m in &mirrors[..3] {
                let g = (image(&f, m), image(&a, m), image(&b, m));
                let (fi, ai, bi) = (
                    index(&mut face_set, &g.0),
                    index(&mut corner_set, &g.1),
                    index(&mut corner_set, &g.2),
                );
                if seen.insert((fi, ai.min(bi), ai.max(bi))) {
                    flags_in_space.push(g);
                }
            }
            if flags_in_space.len() > 14400 {
                return Err("the group of these mirrors is not finite".into());
            }
            at += 1;
        }
        let face_dist = (0..4).map(|k| (f0[k] - c0[k]).powi(2)).sum::<f64>().sqrt();
        let faces: Vec<[f64; 3]> = face_set
            .all
            .iter()
            .map(|f| local(f).map(|x| x / face_dist))
            .collect();
        let cross = |p: &[f64; 3], q: &[f64; 3]| {
            [
                p[1] * q[2] - p[2] * q[1],
                p[2] * q[0] - p[0] * q[2],
                p[0] * q[1] - p[1] * q[0],
            ]
        };
        let dot3 = |p: &[f64; 3], q: &[f64; 3]| p[0] * q[0] + p[1] * q[1] + p[2] * q[2];
        let mut flags = Vec::with_capacity(flags_in_space.len());
        for (f, a, b) in &flags_in_space {
            let (lf, la, lb) = (local(f), local(a), local(b));
            let mut walls = [[0.0; 3]; 3];
            for (i, (p, q, inner)) in [(lf, la, lb), (lf, lb, la), (la, lb, lf)]
                .iter()
                .enumerate()
            {
                let n = cross(p, q);
                let size = dot3(&n, &n).sqrt();
                if size < 1e-9 {
                    return Err("a face of this polychoron is flat".into());
                }
                let sign = if dot3(&n, inner) > 0.0 { 1.0 } else { -1.0 };
                walls[i] = n.map(|x| sign * x / size);
            }
            flags.push((face_set.find(f).unwrap_or(0), walls));
        }
        Ok(Star {
            vertices,
            edges: edges.into_iter().collect(),
            normals: cells.all,
            frames,
            height,
            faces,
            face_dist,
            flags,
        })
    }

    /// How many times the first cell covers a place of its hyperplane.
    fn covers(&self, y: &[f64; 3]) -> usize {
        self.flags
            .iter()
            .filter(|(face, walls)| {
                let f = &self.faces[*face];
                f[0] * y[0] + f[1] * y[1] + f[2] * y[2] <= self.face_dist
                    && walls
                        .iter()
                        .all(|n| n[0] * y[0] + n[1] * y[1] + n[2] * y[2] >= 0.0)
            })
            .count()
    }

    /// How many times the polychoron winds around a point.
    fn density(&self, x: &P4) -> usize {
        let mut count = 0;
        for (n, frame) in self.normals.iter().zip(&self.frames) {
            let d = dot(n, x);
            if d <= 0.0 || d > self.height {
                continue;
            }
            let p = x.map(|v| v * self.height / d);
            if dot(&p, &p) > 1.0 + 1e-12 {
                continue;
            }
            count += self.covers(&[dot(&p, &frame[0]), dot(&p, &frame[1]), dot(&p, &frame[2])]);
        }
        count
    }

    /// The lowest and the highest place between two ends of the line `from + t along`
    /// that the polychoron winds around: going along a line the density changes only on
    /// the hyperplane of a cell, by what the cell covers there.
    fn fibre(&self, from: &P4, along: &P4, ends: (f64, f64)) -> Option<(f64, f64)> {
        let mut steps: Vec<(f64, i64)> = Vec::new();
        for (n, frame) in self.normals.iter().zip(&self.frames) {
            let rate = dot(n, along);
            if rate.abs() < 1e-14 {
                continue;
            }
            let t = (self.height - dot(n, from)) / rate;
            let p = [0, 1, 2, 3].map(|k| from[k] + t * along[k]);
            if dot(&p, &p) > 1.0 + 1e-12 {
                continue;
            }
            let covered =
                self.covers(&[dot(&p, &frame[0]), dot(&p, &frame[1]), dot(&p, &frame[2])]) as i64;
            if covered > 0 {
                // going on, the place comes below the cell, or leaves
                steps.push((t, if rate < 0.0 { covered } else { -covered }));
            }
        }
        steps.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let (mut low, mut high): (Option<f64>, Option<f64>) = (None, None);
        let mut density = 0;
        for (k, (t, change)) in steps.iter().enumerate() {
            density += change;
            let Some((next, _)) = steps.get(k + 1) else {
                break;
            };
            if density > 0 {
                let (a, b) = (t.max(ends.0), next.min(ends.1));
                if a <= b {
                    low = Some(low.map_or(a, |l: f64| l.min(a)));
                    high = Some(high.map_or(b, |h: f64| h.max(b)));
                }
            }
        }
        Some((low?, high?))
    }
}

/// The least distance of a point to the profile of a wall, squared, where the plane of
/// the profile has turned by the angles whose sines and cosines are given.
fn wall_distance(q: &P4, [su, cu, sh, ch]: [f64; 4], band: bool, major: f64, minor: f64) -> f64 {
    let (x, y) = (q[0] * cu + q[1] * su, q[1] * cu - q[0] * su);
    let (z, w) = (q[2] * ch + q[3] * sh, q[3] * ch - q[2] * sh);
    let dx = x - major;
    if band {
        let dz = (z.abs() - minor).max(0.0);
        y * y + w * w + dx * dx + dz * dz
    } else {
        let e = (dx * dx + z * z).sqrt() - minor;
        y * y + w * w + e * e
    }
}

/// The sines and cosines of the turns of a wall at the places that are tried first.
fn wall_turns(twist: usize) -> Vec<[f64; 4]> {
    let steps = 96 * (2 + twist);
    (0..steps)
        .map(|k| {
            let u = std::f64::consts::TAU * k as f64 / steps as f64;
            let ((su, cu), (sh, ch)) = (u.sin_cos(), (twist as f64 / 2.0 * u).sin_cos());
            [su, cu, sh, ch]
        })
        .collect()
}

/// Whether a point lies within a distance of the surface that the profile sweeps.
fn in_wall(
    q: &P4,
    band: bool,
    major: f64,
    minor: f64,
    tube: f64,
    twist: usize,
    turns: &[[f64; 4]],
) -> bool {
    // (the lengths in xy and in zw bound the distance from below)
    let (xy, zw) = (
        (q[0] * q[0] + q[1] * q[1]).sqrt(),
        (q[2] * q[2] + q[3] * q[3]).sqrt(),
    );
    let dx = xy - major;
    // (the root of the sum of squares, not hypot of the system: the same on every system)
    let both = |a: f64, b: f64| (a * a + b * b).sqrt();
    let aside = if band {
        both(dx, (zw - minor).max(0.0))
    } else {
        (both(dx, zw) - minor).abs()
    };
    if aside > tube {
        return false;
    }
    let steps = turns.len() as i64;
    let step = std::f64::consts::TAU / steps as f64;
    // the profile lies on the side of xy that its turn points to: only a turn near the
    // direction of the point in xy brings it near enough
    let (first, count) = match xy > tube {
        true => {
            let (towards, near) = (q[1].atan2(q[0]), (tube / xy).asin());
            let first = ((towards - near) / step).floor() as i64 - 1;
            (
                first,
                (((towards + near) / step).ceil() as i64 + 1 - first + 1).min(steps),
            )
        }
        false => (0, steps),
    };
    let value = |k: i64| wall_distance(q, turns[k.rem_euclid(steps) as usize], band, major, minor);
    let at = |u: f64| {
        let ((su, cu), (sh, ch)) = (u.sin_cos(), (twist as f64 / 2.0 * u).sin_cos());
        wall_distance(q, [su, cu, sh, ch], band, major, minor)
    };
    // a point of the profile moves no faster than this while u grows
    let far = if band { major } else { major + minor };
    let speed = (far * far + (twist as f64 / 2.0 * minor).powi(2)).sqrt();
    let (mut before, mut here) = (value(first - 1), value(first));
    for k in first..first + count {
        let after = value(k + 1);
        if here <= tube * tube {
            return true;
        }
        if here <= before && here <= after && here.sqrt() <= tube + speed * step {
            // the least value near a place that lies lower than its neighbours
            let golden = (5f64.sqrt() - 1.0) / 2.0;
            let (mut a, mut b) = (step * (k as f64 - 1.0), step * (k as f64 + 1.0));
            let (mut c, mut d) = (b - golden * (b - a), a + golden * (b - a));
            let (mut fc, mut fd) = (at(c), at(d));
            for _ in 0..24 {
                if fc.min(fd) <= tube * tube {
                    return true;
                }
                if fc < fd {
                    (b, d, fd) = (d, c, fc);
                    c = b - golden * (b - a);
                    fc = at(c);
                } else {
                    (a, c, fc) = (c, d, fd);
                    d = a + golden * (b - a);
                    fd = at(d);
                }
            }
            if fc.min(fd) <= tube * tube {
                return true;
            }
        }
        (before, here) = (here, after);
    }
    false
}

/// The sides of the hull of the vertices of a polyhedron in xyz around the origin whose
/// edges all have the least distance between two vertices.
fn hull_sides3(vertices: &[P4]) -> Vec<(P4, f64)> {
    let apart = |a: &P4, b: &P4| (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt();
    let mut edge = f64::MAX;
    for i in 0..vertices.len() {
        for j in i + 1..vertices.len() {
            edge = edge.min(apart(&vertices[i], &vertices[j]));
        }
    }
    let mut found = Points::default();
    let mut heights = Vec::new();
    for v in vertices {
        let near: Vec<P4> = vertices
            .iter()
            .filter(|w| {
                let d = apart(v, w);
                d > 1e-9 && d <= edge * (1.0 + 1e-6)
            })
            .map(|w| [0, 1, 2, 3].map(|k| w[k] - v[k]))
            .collect();
        for a in 0..near.len() {
            for b in a + 1..near.len() {
                let (p, q) = (&near[a], &near[b]);
                let mut n = [
                    p[1] * q[2] - p[2] * q[1],
                    p[2] * q[0] - p[0] * q[2],
                    p[0] * q[1] - p[1] * q[0],
                    0.0,
                ];
                let size = dot(&n, &n).sqrt();
                if size < 1e-9 * edge * edge {
                    continue;
                }
                n = n.map(|x| x / size);
                let mut h = dot(&n, v);
                if h < 0.0 {
                    (n, h) = (n.map(|x| -x), -h);
                }
                if h < 1e-12 || vertices.iter().any(|w| dot(&n, w) > h + 1e-9) {
                    continue;
                }
                if found.add(n) {
                    heights.push(h);
                }
            }
        }
    }
    found.all.into_iter().zip(heights).collect()
}

/// The vertices of the uniform prism or antiprism over a polygon: all edges are equal,
/// the axis is z and the vertices lie at the distance 1.
fn prism_vertices(sides: usize, anti: bool) -> Vec<P4> {
    let a = std::f64::consts::PI / sides as f64;
    let h = if anti {
        ((crate::detmath::cos(a) - crate::detmath::cos(2.0 * a)) / 2.0).sqrt()
    } else {
        crate::detmath::sin(a)
    };
    let s = 1.0 / (1.0 + h * h).sqrt();
    let mut out = Vec::with_capacity(2 * sides);
    for k in 0..sides {
        let top = 2.0 * a * k as f64;
        let bottom = if anti { top + a } else { top };
        out.push([
            s * crate::detmath::cos(top),
            s * crate::detmath::sin(top),
            s * h,
            0.0,
        ]);
        out.push([
            s * crate::detmath::cos(bottom),
            s * crate::detmath::sin(bottom),
            -s * h,
            0.0,
        ]);
    }
    out
}

/// The vertices of the snub cube (p = 4) or of the snub dodecahedron (p = 5): the images,
/// under the turns of the group of {p, 3}, of the point that lies equally far from its
/// images under the three turns about the corners of a triangle of mirrors.
fn snub_vertices(p: f64) -> Result<Vec<P4>, String> {
    let (mirrors, dual) = mirrors_of(&[p, 3.0])?;
    let image = |v: &P4, m: &P4| {
        let twice = 2.0 * dot(v, m);
        [0, 1, 2, 3].map(|k| v[k] - twice * m[k])
    };
    let turn = |i: usize, j: usize, v: &P4| image(&image(v, &mirrors[j]), &mirrors[i]);
    let apart = |a: &P4, b: &P4| (0..4).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>();
    let f = |x: &P4| {
        let (a, b, c) = (
            apart(x, &turn(0, 1, x)),
            apart(x, &turn(1, 2, x)),
            apart(x, &turn(0, 2, x)),
        );
        [a - c, b - c, dot(x, x) - 1.0]
    };
    let unit = |v: P4| {
        let size = dot(&v, &v).sqrt();
        v.map(|x| x / size)
    };
    let (d0, d1, d2) = (unit(dual[0]), unit(dual[1]), unit(dual[2]));
    let mut x = unit([0, 1, 2, 3].map(|k| d0[k] + d1[k] + d2[k]));
    let det = |m: &[[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    for _ in 0..100 {
        let r = f(&x);
        if r.iter().map(|v| v.abs()).sum::<f64>() < 1e-15 {
            break;
        }
        // the equations are of the second degree: central differences give their slopes
        let mut j = [[0.0; 3]; 3];
        for c in 0..3 {
            let (mut up, mut down) = (x, x);
            up[c] += 1e-3;
            down[c] -= 1e-3;
            let (fu, fd) = (f(&up), f(&down));
            for k in 0..3 {
                j[k][c] = (fu[k] - fd[k]) / 2e-3;
            }
        }
        let whole = det(&j);
        if whole == 0.0 {
            break;
        }
        for c in 0..3 {
            let mut m = j;
            for k in 0..3 {
                m[k][c] = r[k];
            }
            x[c] -= det(&m) / whole;
        }
    }
    if f(&x).iter().map(|v| v.abs()).sum::<f64>() > 1e-12 {
        return Err("no vertex of the snub polyhedron was found".into());
    }
    let mut found = Points::default();
    found.add(x);
    let mut at = 0;
    while at < found.all.len() && found.all.len() <= 14400 {
        let from = found.all[at];
        found.add(turn(0, 1, &from));
        found.add(turn(1, 2, &from));
        at += 1;
    }
    Ok(found.all)
}

/// A uniform polyhedron in xyz with the circumradius 1: its vertices and its sides.
fn polyhedron(base: &str) -> Option<Result<Hull, String>> {
    const POLYGONS: [(&str, usize); 10] = [
        ("triangular", 3),
        ("square", 4),
        ("pentagonal", 5),
        ("hexagonal", 6),
        ("heptagonal", 7),
        ("octagonal", 8),
        ("enneagonal", 9),
        ("nonagonal", 9),
        ("decagonal", 10),
        ("dodecagonal", 12),
    ];
    let hull = |vertices: Vec<P4>| {
        let sides = hull_sides3(&vertices);
        match sides.len() >= 4 {
            true => Ok((vertices, sides)),
            false => Err("this polyhedron has no faces".to_string()),
        }
    };
    for (ending, anti) in [("antiprism", true), ("prism", false)] {
        if let Some(polygon) = base.strip_suffix(ending) {
            let sides = POLYGONS
                .iter()
                .find(|(name, _)| *name == polygon)
                .map(|(_, n)| *n)
                .or_else(|| {
                    polygon
                        .strip_suffix("gonal")
                        .and_then(|n| n.parse::<usize>().ok())
                })?;
            return Some(match (3..=360).contains(&sides) {
                true => hull(prism_vertices(sides, anti)),
                false => Err("a polygon has 3 to 360 sides".to_string()),
            });
        }
    }
    match base {
        "snubcube" => Some(snub_vertices(4.0).and_then(hull)),
        "snubdodecahedron" => Some(snub_vertices(5.0).and_then(hull)),
        _ => {
            let (marks, rings) = diagram(base, 2)?;
            Some(wythoff(&marks, &rings))
        }
    }
}

/// The marks of a Schläfli symbol {p, q, r}; a mark may be a fraction as 5/2.
fn schlafli(symbol: &str) -> Option<Vec<f64>> {
    let inner = symbol.strip_prefix('{')?.strip_suffix('}')?;
    inner
        .split(',')
        .map(|part| {
            let mut pieces = part.split('/');
            let (above, below) = (
                pieces.next()?.parse::<u32>().ok()?,
                pieces.next().map_or(Some(1), |b| b.parse::<u32>().ok())?,
            );
            (pieces.next().is_none() && below >= 1 && above > 2 * below)
                .then(|| f64::from(above) / f64::from(below))
        })
        .collect()
}

/// The regular star polychora by their names.
const STARS: [(&str, &str); 12] = [
    ("icosahedral120cell", "{3,5,5/2}"),
    ("faceted600cell", "{3,5,5/2}"),
    ("smallstellated120cell", "{5/2,5,3}"),
    ("great120cell", "{5,5/2,5}"),
    ("grand120cell", "{5,3,5/2}"),
    ("greatstellated120cell", "{5/2,3,5}"),
    ("grandstellated120cell", "{5/2,5,5/2}"),
    ("greatgrand120cell", "{5,5/2,3}"),
    ("greaticosahedral120cell", "{3,5/2,5}"),
    ("greatfaceted600cell", "{3,5/2,5}"),
    ("grand600cell", "{3,3,5/2}"),
    ("greatgrandstellated120cell", "{5/2,3,3}"),
];

/// How many of some directions are independent.
fn rank(vectors: &[P4]) -> usize {
    let mut basis: Vec<P4> = Vec::new();
    for v in vectors {
        let mut d = *v;
        let size = dot(&d, &d).sqrt();
        for b in &basis {
            let along = dot(&d, b);
            d = [0, 1, 2, 3].map(|k| d[k] - along * b[k]);
        }
        let left = dot(&d, &d).sqrt();
        if left > 1e-6 * size.max(1e-300) {
            basis.push(d.map(|x| x / left));
        }
    }
    basis.len()
}

/// Where a solid is: what the threads of a drawing share.
#[derive(Clone, Debug)]
pub(crate) struct Shape4 {
    kind: Kind,
    /// From a place of the world back to the place of the solid: a shift, then a matrix.
    back: [P4; 4],
    shift: P4,
}

/// A solid of four dimensions with what was done to it.
#[derive(Clone, Debug)]
pub(crate) struct Solid {
    pub(crate) form: Shape4,
    name: &'static str,
    /// The numbers of the solid as they were given.
    numbers: Vec<Expr>,
    /// From a place of the solid to its place in the world.
    out: [P4; 4],
    /// The factors that it was scaled by along its four axes, and whether it was turned
    /// after a scaling that is not the same along all of them.
    scales: [Expr; 4],
    skew: bool,
    /// How far from its middle the solid reaches, before it was turned, scaled and moved.
    reach: f64,
    /// The hyperplane w = c that cuts it, when it is cut.
    pub(crate) cut: Option<f64>,
    written: Expr,
}

const UNIT: [P4; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

fn times(a: &[P4; 4], b: &[P4; 4]) -> [P4; 4] {
    let mut out = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            out[i][j] = (0..4).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    out
}

fn applied(m: &[P4; 4], p: &P4) -> P4 {
    [0, 1, 2, 3].map(|i| (0..4).map(|k| m[i][k] * p[k]).sum())
}

/// Every way to give signs to the entries of a point and to put them in another order
/// (only the even orders when `even`), each point once.
fn arrangements(base: [f64; 4], even: bool) -> Vec<P4> {
    let mut out: Vec<P4> = Vec::new();
    let orders: Vec<[usize; 4]> = {
        let mut all = Vec::new();
        for a in 0..4 {
            for b in 0..4 {
                for c in 0..4 {
                    for d in 0..4 {
                        let o = [a, b, c, d];
                        if (0..4).all(|k| o.iter().filter(|x| **x == k).count() == 1) {
                            let swaps = (0..4)
                                .flat_map(|i| (i + 1..4).map(move |j| (i, j)))
                                .filter(|(i, j)| o[*i] > o[*j])
                                .count();
                            if !even || swaps % 2 == 0 {
                                all.push(o);
                            }
                        }
                    }
                }
            }
        }
        all
    };
    for order in orders {
        for signs in 0..16u32 {
            let p: P4 = [0, 1, 2, 3].map(|k| {
                let v = base[order[k]];
                if signs >> k & 1 == 1 { -v } else { v }
            });
            if !out
                .iter()
                .any(|q| (0..4).all(|k| (q[k] - p[k]).abs() < 1e-9))
            {
                out.push(p);
            }
        }
    }
    out
}

fn normalized(points: Vec<P4>) -> Vec<P4> {
    points
        .into_iter()
        .map(|p| {
            let far = p.iter().map(|x| x * x).sum::<f64>().sqrt();
            p.map(|x| x / far)
        })
        .collect()
}

/// The vertices of a regular polychoron, at the distance 1 from the middle.
fn regular(cells: usize) -> Vec<P4> {
    let phi = (1.0 + 5f64.sqrt()) / 2.0;
    let five = 5f64.sqrt();
    match cells {
        5 => normalized(vec![
            [1.0, 1.0, 1.0, -1.0 / five],
            [1.0, -1.0, -1.0, -1.0 / five],
            [-1.0, 1.0, -1.0, -1.0 / five],
            [-1.0, -1.0, 1.0, -1.0 / five],
            [0.0, 0.0, 0.0, 4.0 / five],
        ]),
        8 => arrangements([0.5, 0.5, 0.5, 0.5], false),
        16 => arrangements([1.0, 0.0, 0.0, 0.0], false),
        24 => normalized(arrangements([1.0, 1.0, 0.0, 0.0], false)),
        600 => {
            let mut all = arrangements([1.0, 0.0, 0.0, 0.0], false);
            all.extend(arrangements([0.5, 0.5, 0.5, 0.5], false));
            all.extend(arrangements([phi / 2.0, 0.5, 0.5 / phi, 0.0], true));
            all
        }
        _ => {
            let (a, b) = (1.0 / phi, 1.0 / (phi * phi));
            let mut all = arrangements([0.0, 0.0, 2.0, 2.0], false);
            all.extend(arrangements([1.0, 1.0, 1.0, five], false));
            all.extend(arrangements([b, phi, phi, phi], false));
            all.extend(arrangements([a, a, a, phi * phi], false));
            all.extend(arrangements([0.0, b, 1.0, phi * phi], true));
            all.extend(arrangements([0.0, a, phi, five], true));
            all.extend(arrangements([a, 1.0, phi, 2.0], true));
            normalized(all)
        }
    }
}

/// The half-spaces of the hull of vertices whose sides face the given directions.
fn sides_of(vertices: &[P4], normals: &[P4]) -> Vec<(P4, f64)> {
    normals
        .iter()
        .map(|n| {
            let far = vertices
                .iter()
                .map(|v| (0..4).map(|k| v[k] * n[k]).sum::<f64>())
                .fold(f64::MIN, f64::max);
            (*n, far)
        })
        .collect()
}

/// The sides of the hull of the vertices of a polytope around the origin whose edges all
/// have the least distance between two vertices: every side lies in the flat through a
/// vertex and three of its neighbours.
fn hull_sides(vertices: &[P4]) -> Vec<(P4, f64)> {
    let apart = |a: &P4, b: &P4| (0..4).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt();
    let mut edge = f64::MAX;
    for i in 0..vertices.len() {
        for j in i + 1..vertices.len() {
            edge = edge.min(apart(&vertices[i], &vertices[j]));
        }
    }
    let mut found = Points::default();
    let mut heights = Vec::new();
    for v in vertices {
        let near: Vec<P4> = vertices
            .iter()
            .filter(|w| {
                let d = apart(v, w);
                d > 1e-9 && d <= edge * (1.0 + 1e-6)
            })
            .map(|w| [0, 1, 2, 3].map(|k| w[k] - v[k]))
            .collect();
        for a in 0..near.len() {
            for b in a + 1..near.len() {
                for c in b + 1..near.len() {
                    // the direction square to three directions
                    let (p, q, r) = (&near[a], &near[b], &near[c]);
                    let minor = |i: usize, j: usize, k: usize| {
                        p[i] * (q[j] * r[k] - q[k] * r[j]) - p[j] * (q[i] * r[k] - q[k] * r[i])
                            + p[k] * (q[i] * r[j] - q[j] * r[i])
                    };
                    let mut n = [
                        minor(1, 2, 3),
                        -minor(0, 2, 3),
                        minor(0, 1, 3),
                        -minor(0, 1, 2),
                    ];
                    let size = dot(&n, &n).sqrt();
                    if size < 1e-9 * edge.powi(3) {
                        continue;
                    }
                    n = n.map(|x| x / size);
                    let mut h = dot(&n, v);
                    if h < 0.0 {
                        (n, h) = (n.map(|x| -x), -h);
                    }
                    if h < 1e-12 || vertices.iter().any(|w| dot(&n, w) > h + 1e-9) {
                        continue;
                    }
                    if found.add(n) {
                        heights.push(h);
                    }
                }
            }
        }
    }
    found.all.into_iter().zip(heights).collect()
}

/// The area of a flat polygon without dents, from its corners in any order.
fn area_of(corners: &[P4]) -> f64 {
    let count = corners.len() as f64;
    let middle = [0, 1, 2, 3].map(|k| corners.iter().map(|c| c[k]).sum::<f64>() / count);
    let from = |c: &P4| [0, 1, 2, 3].map(|k| c[k] - middle[k]);
    let first = from(&corners[0]);
    let size = dot(&first, &first).sqrt();
    if size < 1e-12 {
        return 0.0;
    }
    let across = first.map(|x| x / size);
    let mut up: Option<P4> = None;
    for c in &corners[1..] {
        let d = from(c);
        let along = dot(&d, &across);
        let rest = [0, 1, 2, 3].map(|k| d[k] - along * across[k]);
        let left = dot(&rest, &rest).sqrt();
        if left > 1e-9 * size {
            up = Some(rest.map(|x| x / left));
            break;
        }
    }
    let Some(up) = up else { return 0.0 };
    let mut flat: Vec<(f64, f64)> = corners
        .iter()
        .map(|c| (dot(&from(c), &across), dot(&from(c), &up)))
        .collect();
    flat.sort_by(|a, b| {
        a.1.atan2(a.0)
            .partial_cmp(&b.1.atan2(b.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut twice = 0.0;
    for k in 0..flat.len() {
        let (a, b) = (flat[k], flat[(k + 1) % flat.len()]);
        twice += a.0 * b.1 - a.1 * b.0;
    }
    twice.abs() / 2.0
}

/// The bulk and the surface of a polychoron around the origin: every side is a
/// polyhedron whose faces it shares with other sides.
fn polychoron_content(vertices: &[P4], sides: &[(P4, f64)]) -> (f64, f64) {
    let on: Vec<Vec<usize>> = sides
        .iter()
        .map(|(n, h)| {
            (0..vertices.len())
                .filter(|k| (dot(n, &vertices[*k]) - h).abs() < 1e-9)
                .collect()
        })
        .collect();
    let mut at: Vec<Vec<usize>> = vec![Vec::new(); vertices.len()];
    for (f, own) in on.iter().enumerate() {
        for v in own {
            at[*v].push(f);
        }
    }
    let (mut bulk, mut surface) = (0.0, 0.0);
    for (f, own) in on.iter().enumerate() {
        if own.len() < 4 {
            continue;
        }
        let (n, h) = &sides[f];
        let count = own.len() as f64;
        let middle = [0, 1, 2, 3].map(|k| own.iter().map(|v| vertices[*v][k]).sum::<f64>() / count);
        // the sides that share a face with this one
        let mut shared: std::collections::BTreeMap<usize, Vec<usize>> =
            std::collections::BTreeMap::new();
        for v in own {
            for g in &at[*v] {
                if *g != f {
                    shared.entry(*g).or_default().push(*v);
                }
            }
        }
        let mut cell = 0.0;
        for (g, common) in shared {
            if common.len() < 3 {
                continue;
            }
            let (m, hg) = &sides[g];
            let along = dot(m, n) / dot(n, n);
            let within = [0, 1, 2, 3].map(|k| m[k] - along * n[k]);
            let size = dot(&within, &within).sqrt();
            if size < 1e-12 {
                continue;
            }
            let far = (hg - dot(m, &middle)) / size;
            let corners: Vec<P4> = common.iter().map(|v| vertices[*v]).collect();
            cell += far * area_of(&corners) / 3.0;
        }
        surface += cell;
        bulk += h / dot(n, n).sqrt() * cell / 4.0;
    }
    (bulk, surface)
}

/// A number with twelve digits as an exact fraction.
fn rounded(v: f64, digits: i32) -> Expr {
    if v == 0.0 || !v.is_finite() {
        return Expr::zero();
    }
    let power = v.abs().log10().floor() as i32 - (digits - 1);
    let whole = (v / 10f64.powi(power)).round() as i64;
    tidy(mul(vec![
        Expr::int(whole),
        pow(Expr::int(10), Expr::int(i64::from(power))),
    ]))
}

fn polygon(sides: usize) -> Vec<[f64; 2]> {
    (0..sides)
        .map(|k| {
            let angle = std::f64::consts::TAU * k as f64 / sides as f64;
            [crate::detmath::cos(angle), crate::detmath::sin(angle)]
        })
        .collect()
}

/// Points that are kept once each, found again within a small distance.
#[derive(Default)]
struct Points {
    cells: std::collections::HashMap<[i64; 4], Vec<usize>>,
    all: Vec<P4>,
}

impl Points {
    const GRID: f64 = 1e-5;
    const NEAR: f64 = 1e-8;

    fn find(&self, v: &P4) -> Option<usize> {
        let low = v.map(|c| ((c - Self::NEAR) / Self::GRID + 0.5).floor() as i64);
        let high = v.map(|c| ((c + Self::NEAR) / Self::GRID + 0.5).floor() as i64);
        for a in low[0]..=high[0] {
            for b in low[1]..=high[1] {
                for c in low[2]..=high[2] {
                    for d in low[3]..=high[3] {
                        for i in self.cells.get(&[a, b, c, d]).into_iter().flatten() {
                            let apart: f64 = (0..4).map(|k| (self.all[*i][k] - v[k]).powi(2)).sum();
                            if apart.sqrt() <= Self::NEAR {
                                return Some(*i);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn add(&mut self, v: P4) -> bool {
        if self.find(&v).is_some() {
            return false;
        }
        let key = v.map(|c| (c / Self::GRID + 0.5).floor() as i64);
        self.all.push(v);
        self.cells.entry(key).or_default().push(self.all.len() - 1);
        true
    }
}

fn dot(a: &P4, b: &P4) -> f64 {
    (0..4).map(|k| a[k] * b[k]).sum()
}

/// The mirrors of a Coxeter diagram that is a line: neighbours meet at the angle pi over
/// their mark, the others are square to each other. With them the points that lie on
/// every mirror but one.
fn mirrors_of(marks: &[f64]) -> Result<(Vec<P4>, Vec<P4>), String> {
    let count = marks.len() + 1;
    let mut normals = vec![[0.0; 4]; count];
    normals[0][0] = 1.0;
    for k in 1..count {
        if marks[k - 1] <= 1.0 {
            return Err("a mark of a Coxeter diagram is above 1".into());
        }
        let a = -crate::detmath::cos(std::f64::consts::PI / marks[k - 1]) / normals[k - 1][k - 1];
        if a * a >= 1.0 - 1e-12 {
            return Err("this Coxeter diagram has no finite group".into());
        }
        normals[k][k - 1] = a;
        normals[k][k] = (1.0 - a * a).sqrt();
    }
    let mut dual = vec![[0.0; 4]; count];
    for (i, point) in dual.iter_mut().enumerate() {
        let mut before = 0.0;
        for j in 0..count {
            let mut right = if i == j { 1.0 } else { 0.0 };
            if j > 0 {
                right -= normals[j][j - 1] * before;
            }
            before = right / normals[j][j];
            point[j] = before;
        }
    }
    Ok((normals, dual))
}

/// Every image of a point in the mirrors.
fn orbit(mirrors: &[P4], seed: P4) -> Result<Vec<P4>, String> {
    let mut found = Points::default();
    found.add(seed);
    let mut at = 0;
    while at < found.all.len() {
        let p = found.all[at];
        for n in mirrors {
            let twice = 2.0 * dot(&p, n);
            found.add([0, 1, 2, 3].map(|k| p[k] - twice * n[k]));
        }
        if found.all.len() > 14400 {
            return Err("the group of these mirrors is not finite".into());
        }
        at += 1;
    }
    Ok(found.all)
}

/// The dimension of the flat that points span.
fn spanned(points: &[P4]) -> usize {
    let mut basis: Vec<P4> = Vec::new();
    for p in &points[1..] {
        let mut d = [0, 1, 2, 3].map(|k| p[k] - points[0][k]);
        let size = dot(&d, &d).sqrt();
        for b in &basis {
            let along = dot(&d, b);
            d = [0, 1, 2, 3].map(|k| d[k] - along * b[k]);
        }
        let left = dot(&d, &d).sqrt();
        if left > 1e-6 * size.max(1.0) {
            basis.push(d.map(|x| x / left));
        }
    }
    basis.len()
}

/// The uniform polytope of a Coxeter diagram with rings: its vertices at the distance 1
/// and its sides n.x <= h (a polyhedron in xyz for two marks, a polychoron for three).
fn wythoff(marks: &[f64], rings: &[bool]) -> Result<Hull, String> {
    let (mirrors, dual) = mirrors_of(marks)?;
    let mut seed = [0.0; 4];
    for (i, ring) in rings.iter().enumerate() {
        if *ring {
            seed = [0, 1, 2, 3].map(|k| seed[k] + dual[i][k]);
        }
    }
    let size = dot(&seed, &seed).sqrt();
    if size == 0.0 {
        return Err("a uniform polytope has a ring at one node of its diagram at least".into());
    }
    let vertices = orbit(&mirrors, seed.map(|x| x / size))?;
    let mut sides = Points::default();
    let mut heights = Vec::new();
    for d in &dual {
        let size = dot(d, d).sqrt();
        for u in orbit(&mirrors, d.map(|x| x / size))? {
            let far = vertices.iter().map(|v| dot(&u, v)).fold(f64::MIN, f64::max);
            if far <= 1e-9 {
                continue;
            }
            let on: Vec<P4> = vertices
                .iter()
                .filter(|v| dot(&u, v) > far - 1e-9)
                .copied()
                .collect();
            if spanned(&on) == marks.len() && sides.add(u) {
                heights.push(far);
            }
        }
    }
    if sides.all.len() < marks.len() + 2 {
        return Err("this diagram gives a flat figure".into());
    }
    Ok((vertices, sides.all.into_iter().zip(heights).collect()))
}

/// The marks and the rings of a uniform polytope by its name ("truncated 120-cell",
/// "cuboctahedron") or by its diagram ("x5o3o3x"), for 3 or 2 marks.
fn diagram(name: &str, marks: usize) -> Option<(Vec<f64>, Vec<bool>)> {
    // a diagram: x for a ring, o for a node without, numbers between them
    let written: Vec<char> = name.chars().collect();
    if written
        .iter()
        .all(|c| matches!(c, 'x' | 'o' | '/') || c.is_ascii_digit())
        && written.iter().any(char::is_ascii_digit)
    {
        let (mut rings, mut found) = (Vec::new(), Vec::new());
        let mut number = String::new();
        for c in &written {
            match c {
                'x' | 'o' => {
                    if !rings.is_empty() {
                        let mut parts = number.split('/');
                        let (above, below) = (
                            parts.next()?.parse::<f64>().ok()?,
                            parts.next().map_or(Some(1.0), |b| b.parse::<f64>().ok())?,
                        );
                        found.push(above / below);
                        number.clear();
                    }
                    rings.push(*c == 'x');
                }
                c if c.is_ascii_digit() || *c == '/' => number.push(*c),
                _ => return None,
            }
        }
        return (found.len() == marks
            && number.is_empty()
            && found.iter().all(|m| m.fract() == 0.0))
        .then_some((found, rings));
    }
    let known: &[(&str, &[f64], &[bool])] = match marks {
        3 => &[
            ("5cell", &[3.0, 3.0, 3.0], &[true, false, false, false]),
            ("tesseract", &[4.0, 3.0, 3.0], &[true, false, false, false]),
            ("8cell", &[4.0, 3.0, 3.0], &[true, false, false, false]),
            ("16cell", &[3.0, 3.0, 4.0], &[true, false, false, false]),
            ("24cell", &[3.0, 4.0, 3.0], &[true, false, false, false]),
            ("120cell", &[5.0, 3.0, 3.0], &[true, false, false, false]),
            ("600cell", &[3.0, 3.0, 5.0], &[true, false, false, false]),
        ],
        _ => &[
            ("tetrahedron", &[3.0, 3.0], &[true, false, false]),
            ("cube", &[4.0, 3.0], &[true, false, false]),
            ("octahedron", &[3.0, 4.0], &[true, false, false]),
            ("dodecahedron", &[5.0, 3.0], &[true, false, false]),
            ("icosahedron", &[3.0, 5.0], &[true, false, false]),
            ("cuboctahedron", &[4.0, 3.0], &[false, true, false]),
            ("icosidodecahedron", &[5.0, 3.0], &[false, true, false]),
            ("rhombicuboctahedron", &[4.0, 3.0], &[true, false, true]),
            ("rhombicosidodecahedron", &[5.0, 3.0], &[true, false, true]),
        ],
    };
    let changes: &[(&str, &[usize])] = &[
        ("runcicantitruncated", &[0, 1, 2, 3]),
        ("runcicantellated", &[0, 2, 3]),
        ("cantitruncated", &[0, 1, 2]),
        ("runcitruncated", &[0, 1, 3]),
        ("omnitruncated", &[0, 1, 2, 3]),
        ("birectified", &[2]),
        ("bitruncated", &[1, 2]),
        ("cantellated", &[0, 2]),
        ("runcinated", &[0, 3]),
        ("rectified", &[1]),
        ("truncated", &[0, 1]),
        ("", &[0]),
    ];
    for (change, ringed) in changes {
        let Some(base) = name.strip_prefix(change) else {
            continue;
        };
        // (the truncated cuboctahedron and icosidodecahedron have all three rings)
        if marks == 2
            && *change == "truncated"
            && matches!(base, "cuboctahedron" | "icosidodecahedron")
        {
            let first = if base == "cuboctahedron" { 4.0 } else { 5.0 };
            return Some((vec![first, 3.0], vec![true, true, true]));
        }
        if let Some((_, found, rings)) = known.iter().find(|(known_name, _, _)| *known_name == base)
        {
            if change.is_empty() {
                return Some((found.to_vec(), rings.to_vec()));
            }
            let all = *change == "omnitruncated";
            if !rings[0]
                || rings[1..].iter().any(|r| *r)
                || (!all && ringed.iter().any(|k| *k > marks))
            {
                return None;
            }
            return Some((
                found.to_vec(),
                (0..=marks).map(|k| ringed.contains(&k)).collect(),
            ));
        }
    }
    // a prism over a polygon: "7gonalprism"
    if marks == 2
        && let Some(sides) = name
            .strip_suffix("gonalprism")
            .and_then(|n| n.parse::<f64>().ok())
        && sides >= 3.0
    {
        return Some((vec![sides, 2.0], vec![true, false, true]));
    }
    None
}

const NAMES: &str = "ball (hypersphere, glome), 5-cell, tesseract (8-cell), 16-cell, 24-cell, 120-cell, 600-cell, the uniform polychora (rectified, truncated, bitruncated, cantellated, runcinated, cantitruncated, runcitruncated or omnitruncated before one of them, or a diagram as x5o3o3x; dual before it for the dual), snub 24-cell, grand antiprism, the ten regular star polychora (small stellated 120-cell ..., or a symbol as {5/2,5,3}), ellipsoid a b c d, prism, pyramid and bipyramid over a polyhedron, duoprism p q, duopyramid p q, spherinder, cubinder, duocylinder, prismic cylinder sides, hypercone, hyperfrustum lower upper, coninder, cylindrone, dicone, hypercapsule half-length, spheritorus R r, torisphere R r, tiger (duotorus) R1 R2 r, ditorus R a r, torinder R r, paraboloid, hyperboloid waist, spherical segment lower upper, lens thickness, superquadric power, bracketope \"shape\", star cell cells core spike sharpness, Klein bottle R r thickness half-turns, projective plane radius thickness, Moebius band R half-width thickness half-turns, reuleaux \"rotor\" | \"simplex\"";

pub(crate) fn is_solid(e: &Expr) -> bool {
    e.fn_args("solid4d").is_some()
}

impl Solid {
    /// The solid that `solid4d("name", numbers..., what was done to it...)` writes.
    pub(crate) fn read(e: &Expr) -> Result<Solid, String> {
        let args = e
            .fn_args("solid4d")
            .ok_or("a solid of four dimensions: solid4d(\"name\", numbers...)")?;
        let usage = || format!("solid4d(\"name\", numbers...): the solids are {NAMES}");
        let name: String = args
            .first()
            .and_then(word)
            .ok_or_else(usage)?
            .to_lowercase()
            .chars()
            .filter(|c| !matches!(c, ' ' | '-' | '_'))
            .collect();
        // the dual of a polytope: "dual" before its name
        let (name, dual) = match name.strip_prefix("dual") {
            Some(rest) if !rest.is_empty() => (rest.to_string(), true),
            _ => (name, false),
        };
        let mut numbers: Vec<Expr> = Vec::new();
        let mut done: Vec<&[Expr]> = Vec::new();
        for a in &args[1..] {
            match a.as_list() {
                Some(items) if items.first().and_then(word).is_some() => done.push(items),
                _ if done.is_empty() => numbers.push(a.clone()),
                _ => return Err(usage()),
            }
        }
        let value = |k: usize, usual: f64| -> Result<f64, String> {
            match numbers.get(k) {
                None => Ok(usual),
                Some(e) => crate::plot::number(e)
                    .filter(|v| v.is_finite())
                    .ok_or_else(|| format!("solid4d: {e} is no number")),
            }
        };
        let count = |k: usize, usual: usize| -> Result<usize, String> {
            match numbers.get(k) {
                None => Ok(usual),
                Some(e) => e
                    .as_i64()
                    .filter(|n| (3..=360).contains(n))
                    .map(|n| n as usize)
                    .ok_or_else(|| format!("solid4d: a polygon has 3 to 360 sides, not {e}")),
            }
        };
        let positive = |v: f64, what: &str| match v > 0.0 {
            true => Ok(v),
            false => Err(format!("solid4d: {what} is above zero")),
        };
        let most = |n: usize| match numbers.len() <= n {
            true => Ok(()),
            false => Err(format!(
                "solid4d(\"{}\"): {n} number{} at most",
                args[0],
                if n == 1 { "" } else { "s" }
            )),
        };
        let polytope = |cells: usize| -> Kind {
            let vertices = regular(cells);
            Kind::Polytope {
                sides: hull_sides(&vertices),
                vertices,
            }
        };
        let (kind, canonical, reach): (Kind, &'static str, f64) = match name.as_str() {
            "ball" | "hypersphere" | "glome" | "sphere" | "3sphere" => {
                most(0)?;
                (Kind::Ball, "ball", 1.0)
            }
            "5cell" | "cell5" | "pentachoron" | "simplex" => {
                (most(0).map(|()| polytope(5))?, "5-cell", 1.0)
            }
            "8cell" | "cell8" | "tesseract" | "hypercube" => {
                (most(0).map(|()| polytope(8))?, "tesseract", 1.0)
            }
            "16cell" | "cell16" | "orthoplex" | "hexadecachoron" => {
                (most(0).map(|()| polytope(16))?, "16-cell", 1.0)
            }
            "24cell" | "cell24" | "icositetrachoron" => {
                (most(0).map(|()| polytope(24))?, "24-cell", 1.0)
            }
            "120cell" | "cell120" | "hecatonicosachoron" => {
                (most(0).map(|()| polytope(120))?, "120-cell", 1.0)
            }
            "600cell" | "cell600" | "hexacosichoron" => {
                (most(0).map(|()| polytope(600))?, "600-cell", 1.0)
            }
            "duoprism" => {
                most(2)?;
                let (p, q) = (count(0, 4)?, count(1, 4)?);
                let mut vertices = Vec::new();
                for a in polygon(p) {
                    for b in polygon(q) {
                        vertices.push([a[0], a[1], b[0], b[1]]);
                    }
                }
                let mut normals = Vec::new();
                for k in 0..p {
                    let angle = (2 * k + 1) as f64 * std::f64::consts::PI / p as f64;
                    normals.push([
                        crate::detmath::cos(angle),
                        crate::detmath::sin(angle),
                        0.0,
                        0.0,
                    ]);
                }
                for k in 0..q {
                    let angle = (2 * k + 1) as f64 * std::f64::consts::PI / q as f64;
                    normals.push([
                        0.0,
                        0.0,
                        crate::detmath::cos(angle),
                        crate::detmath::sin(angle),
                    ]);
                }
                (
                    Kind::Polytope {
                        sides: sides_of(&vertices, &normals),
                        vertices,
                    },
                    "duoprism",
                    2f64.sqrt(),
                )
            }
            "duopyramid" => {
                most(2)?;
                (
                    Kind::Duopyramid(count(0, 4)?, count(1, 4)?),
                    "duopyramid",
                    1.0,
                )
            }
            "spherinder" => (
                most(0).map(|()| Kind::Spherinder)?,
                "spherinder",
                2f64.sqrt(),
            ),
            "cubinder" => (most(0).map(|()| Kind::Cubinder)?, "cubinder", 3f64.sqrt()),
            "duocylinder" => (
                most(0).map(|()| Kind::Duocylinder)?,
                "duocylinder",
                2f64.sqrt(),
            ),
            "prismiccylinder" => {
                most(1)?;
                (
                    Kind::PrismicCylinder(count(0, 6)?),
                    "prismic cylinder",
                    2f64.sqrt(),
                )
            }
            "hypercone" | "cone" => (
                most(0).map(|()| Kind::Frustum(0.0, 1.0))?,
                "hypercone",
                2f64.sqrt(),
            ),
            "hyperfrustum" | "frustum" => {
                most(2)?;
                let (lower, upper) = (value(0, 0.5)?, value(1, 1.0)?);
                if lower < 0.0 || upper < 0.0 || lower + upper <= 0.0 {
                    return Err(
                        "solid4d: the radii of a frustum are not negative, and one is above zero"
                            .into(),
                    );
                }
                (
                    Kind::Frustum(lower, upper),
                    "hyperfrustum",
                    (1.0 + lower.max(upper).powi(2)).sqrt(),
                )
            }
            "coninder" => (most(0).map(|()| Kind::Coninder)?, "coninder", 3f64.sqrt()),
            "cylindrone" => (
                most(0).map(|()| Kind::Cylindrone)?,
                "cylindrone",
                3f64.sqrt(),
            ),
            "dicone" => (most(0).map(|()| Kind::Dicone)?, "dicone", 2f64.sqrt()),
            "hypercapsule" | "capsule" => {
                most(1)?;
                let half = value(0, 1.0)?;
                if half < 0.0 {
                    return Err("solid4d: the half length of a capsule is not negative".into());
                }
                (Kind::Capsule(half), "hypercapsule", 1.0 + half)
            }
            "spheritorus" | "torisphere" | "torinder" => {
                most(2)?;
                let (major, minor) = (
                    positive(value(0, 1.0)?, "the larger radius")?,
                    positive(value(1, 0.4)?, "the radius of the tube")?,
                );
                match name.as_str() {
                    "spheritorus" => (
                        Kind::Spheritorus(major, minor),
                        "spheritorus",
                        major + minor,
                    ),
                    "torisphere" => (Kind::Torisphere(major, minor), "torisphere", major + minor),
                    _ => (
                        Kind::Torinder(major, minor),
                        "torinder",
                        ((major + minor).powi(2) + 1.0).sqrt(),
                    ),
                }
            }
            "tiger" | "duotorus" => {
                most(3)?;
                let (a, b, r) = (
                    positive(value(0, 1.0)?, "a radius")?,
                    positive(value(1, 1.0)?, "a radius")?,
                    positive(value(2, 0.3)?, "the radius of the tube")?,
                );
                (
                    Kind::Tiger(a, b, r),
                    "tiger",
                    ((a + r).powi(2) + (b + r).powi(2)).sqrt(),
                )
            }
            "ditorus" => {
                most(3)?;
                let (a, b, r) = (
                    positive(value(0, 1.0)?, "a radius")?,
                    positive(value(1, 0.5)?, "a radius")?,
                    positive(value(2, 0.2)?, "the radius of the tube")?,
                );
                (Kind::Ditorus(a, b, r), "ditorus", a + b + r)
            }
            "paraboloid" => (
                most(0).map(|()| Kind::Paraboloid)?,
                "paraboloid",
                2f64.sqrt(),
            ),
            "hyperboloid" => {
                most(1)?;
                let waist = value(0, 0.5)?;
                if !(0.0..=1.0).contains(&waist) {
                    return Err("solid4d: the waist of a hyperboloid is from 0 to 1".into());
                }
                (Kind::Hyperboloid(waist), "hyperboloid", 2f64.sqrt())
            }
            "sphericalsegment" | "segment" => {
                most(2)?;
                let (lower, upper) = (value(0, 0.0)?, value(1, 1.0)?);
                if !(-1.0 <= lower && lower < upper && upper <= 1.0) {
                    return Err("solid4d: a segment of the ball lies between two heights from -1 to 1, the lower one first".into());
                }
                (Kind::Segment(lower, upper), "spherical segment", 1.0)
            }
            "lens" => {
                most(1)?;
                let thick = value(0, 0.6)?;
                if !(thick > 0.0 && thick <= 2.0) {
                    return Err("solid4d: the thickness of a lens is above 0 and 2 at most".into());
                }
                (Kind::Lens(thick), "lens", 1.0)
            }
            "superquadric" => {
                most(1)?;
                let power = positive(value(0, 4.0)?, "the power")?;
                (
                    Kind::Superquadric(power),
                    "superquadric",
                    (2.0 * crate::detmath::pow(4.0, -1.0 / power)).max(1.0),
                )
            }
            "bracketope" | "brackets" => {
                most(1)?;
                let shape = numbers.first().and_then(crate::radix::text_of).ok_or(
                    "solid4d(\"bracketope\", \"shape\"): the shape in brackets, as \"[(II)(II)]\"",
                )?;
                (Kind::Brackets(Bracket::parse(shape)?), "bracketope", 2.0)
            }
            "prism" | "pyramid" | "bipyramid" => {
                most(1)?;
                let base: String = numbers
                    .first()
                    .and_then(word)
                    .ok_or_else(|| format!("solid4d(\"{name}\", \"polyhedron\"): over a polyhedron, as \"cube\", \"truncated icosahedron\", \"snub cube\", \"7-gonal prism\", \"pentagonal antiprism\" or a diagram \"x5x3o\""))?
                    .to_lowercase()
                    .chars()
                    .filter(|c| !matches!(c, ' ' | '-' | '_'))
                    .collect();
                let (vertices, sides) = polyhedron(&base).ok_or_else(|| format!("solid4d: {} is no uniform polyhedron: the five regular ones, cuboctahedron, icosidodecahedron, rhombicuboctahedron, rhombicosidodecahedron, truncated ..., snub cube, snub dodecahedron, n-gonal prism, n-gonal antiprism, or a diagram as \"x5x3o\"", numbers[0]))??;
                // the sides of the polyhedron become sides of the solid of four dimensions
                let unit = |n: P4, h: f64| {
                    let size = dot(&n, &n).sqrt();
                    (n.map(|x| x / size), h / size)
                };
                let at = |v: &P4, w: f64| [v[0], v[1], v[2], w];
                let (all, corners, canonical, reach): (Vec<(P4, f64)>, Vec<P4>, &'static str, f64) =
                    match name.as_str() {
                        "prism" => {
                            // as long as an edge of the base
                            let mut edge = f64::MAX;
                            for i in 0..vertices.len() {
                                for j in i + 1..vertices.len() {
                                    edge = edge.min(
                                        (0..4)
                                            .map(|k| (vertices[i][k] - vertices[j][k]).powi(2))
                                            .sum::<f64>()
                                            .sqrt(),
                                    );
                                }
                            }
                            let half = edge / 2.0;
                            let mut all = sides.clone();
                            all.extend([
                                ([0.0, 0.0, 0.0, 1.0], half),
                                ([0.0, 0.0, 0.0, -1.0], half),
                            ]);
                            let corners = vertices
                                .iter()
                                .flat_map(|v| [at(v, half), at(v, -half)])
                                .collect();
                            (all, corners, "prism", (1.0 + half * half).sqrt())
                        }
                        "pyramid" => {
                            // the tip at w = -1, the base at w = 1
                            let mut all: Vec<(P4, f64)> = sides
                                .iter()
                                .map(|(n, h)| unit([n[0], n[1], n[2], -h / 2.0], h / 2.0))
                                .collect();
                            all.push(([0.0, 0.0, 0.0, 1.0], 1.0));
                            let mut corners: Vec<P4> =
                                vertices.iter().map(|v| at(v, 1.0)).collect();
                            corners.push([0.0, 0.0, 0.0, -1.0]);
                            (all, corners, "pyramid", 2f64.sqrt())
                        }
                        _ => {
                            let all = sides
                                .iter()
                                .flat_map(|(n, h)| {
                                    [
                                        unit([n[0], n[1], n[2], *h], *h),
                                        unit([n[0], n[1], n[2], -h], *h),
                                    ]
                                })
                                .collect();
                            let mut corners: Vec<P4> =
                                vertices.iter().map(|v| at(v, 0.0)).collect();
                            corners.extend([[0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0, -1.0]]);
                            (all, corners, "bipyramid", 1.0)
                        }
                    };
                (
                    Kind::Polytope {
                        sides: all,
                        vertices: corners,
                    },
                    canonical,
                    reach,
                )
            }
            "ellipsoid" => {
                most(4)?;
                for k in 0..4 {
                    positive(value(k, 1.0)?, "a half axis of an ellipsoid")?;
                }
                (Kind::Ball, "ellipsoid", 1.0)
            }
            "snub24cell" | "grandantiprism" => {
                most(0)?;
                // the vertices of the 600-cell without those of a 24-cell in it, or
                // without two rings of ten in two planes that are square to each other
                let phi = (1.0 + 5f64.sqrt()) / 2.0;
                let towards = {
                    let q = [phi / 2.0, 0.5, 0.5 / phi, 0.0];
                    let rest = [0.0, q[1], q[2], q[3]];
                    let size = dot(&rest, &rest).sqrt();
                    rest.map(|x| x / size)
                };
                let snub = name == "snub24cell";
                let vertices: Vec<P4> = regular(600)
                    .into_iter()
                    .filter(|v| match snub {
                        true => {
                            let halves = v.iter().filter(|c| (c.abs() - 0.5).abs() < 1e-9).count();
                            halves != 4 && v.iter().all(|c| c.abs() < 1.0 - 1e-9)
                        }
                        false => {
                            let within = v[0] * v[0] + dot(v, &towards).powi(2);
                            within > 1e-9 && within < 1.0 - 1e-9
                        }
                    })
                    .collect();
                (
                    Kind::Polytope {
                        sides: hull_sides(&vertices),
                        vertices,
                    },
                    if snub {
                        "snub 24-cell"
                    } else {
                        "grand antiprism"
                    },
                    1.0,
                )
            }
            "starcell" | "spikedball" => {
                most(4)?;
                let family = numbers.first().map_or(Some(8), Expr::as_i64);
                let dirs = match family {
                    Some(5) => regular(5),
                    Some(8) => regular(16),
                    Some(16) => regular(8),
                    Some(24) => regular(24),
                    Some(120) => regular(600),
                    Some(600) => regular(120),
                    _ => return Err("solid4d(\"star cell\", cells, core, spike, sharpness): the spikes of the 5-, 8-, 16-, 24-, 120- or 600-cell".into()),
                };
                let (core, spike, sharp) = (
                    value(1, 0.45)?,
                    value(2, 0.55)?,
                    positive(value(3, 6.0)?, "the sharpness")?,
                );
                if core < 0.0 || spike < 0.0 || core + spike <= 0.0 {
                    return Err("solid4d: the radius of the ball and the length of the spikes are not negative, and one is above zero".into());
                }
                (
                    Kind::Spiked(dirs, core, spike, sharp),
                    "star cell",
                    core + spike,
                )
            }
            "kleinbottle" | "projectiveplane" | "mobiusband" | "moebiusband" | "möbiusband" => {
                let band = name.ends_with("band");
                let plane = name == "projectiveplane";
                most(if plane { 2 } else { 4 })?;
                let (major, minor, thick, turns) = match plane {
                    true => {
                        let radius = positive(value(0, 0.5)?, "the radius")?;
                        (
                            radius,
                            radius,
                            positive(value(1, 0.1)?, "the thickness")?,
                            Some(1),
                        )
                    }
                    false => (
                        positive(value(0, 1.0)?, "the larger radius")?,
                        positive(
                            value(1, 0.4)?,
                            if band {
                                "the half width"
                            } else {
                                "the radius of the circle"
                            },
                        )?,
                        positive(value(2, 0.1)?, "the thickness")?,
                        numbers.get(3).map_or(Some(1), Expr::as_i64),
                    ),
                };
                let twist = turns
                    .filter(|t| (0..=16).contains(t))
                    .ok_or("solid4d: the plane of the profile makes 0 to 16 half-turns")?
                    as usize;
                if !band && major <= minor && (major != minor || twist != 1) {
                    return Err("solid4d: the larger radius of a Klein bottle is above the radius of its circle (equal radii with one half-turn give the projective plane)".into());
                }
                let far = if band {
                    (major * major + minor * minor).sqrt()
                } else {
                    major + minor
                };
                let canonical = match (band, plane) {
                    (true, _) => "Moebius band",
                    (_, true) => "projective plane",
                    _ => "Klein bottle",
                };
                (
                    Kind::Wall {
                        band,
                        major,
                        minor,
                        tube: thick / 2.0,
                        twist,
                        turns: wall_turns(twist),
                    },
                    canonical,
                    far + thick / 2.0,
                )
            }
            "reuleaux" | "reuleauxsolid" | "reuleauxrotor" | "reuleauxsimplex" => {
                most(1)?;
                let which = match name.as_str() {
                    "reuleauxrotor" => "rotor".to_string(),
                    "reuleauxsimplex" => "simplex".to_string(),
                    _ => numbers
                        .first()
                        .map_or(Some("rotor".to_string()), word)
                        .unwrap_or_default(),
                };
                match which.as_str() {
                    "rotor" => (Kind::Rotor, "Reuleaux rotor", 2.0 / 3f64.sqrt()),
                    "simplex" => {
                        let corners = regular(5);
                        let edge = (0..4).map(|k| (corners[0][k] - corners[1][k]).powi(2)).sum::<f64>().sqrt();
                        (Kind::Common(corners, edge), "Reuleaux simplex", 1.0)
                    }
                    _ => return Err("solid4d(\"reuleaux\", \"rotor\" | \"simplex\"): the Reuleaux triangle turned around w, or what five balls around the corners of a 5-cell have in common".into()),
                }
            }
            other if other.starts_with('{') || STARS.iter().any(|(known, _)| *known == other) => {
                most(0)?;
                let symbol = STARS
                    .iter()
                    .find(|(known, _)| *known == other)
                    .map_or(other, |(_, symbol)| symbol);
                let marks = schlafli(symbol).filter(|m| m.len() == 3).ok_or(
                    "solid4d: a regular polychoron by its Schläfli symbol, as \"{5/2,5,3}\"",
                )?;
                match marks.iter().all(|m| m.fract() == 0.0) {
                    true => {
                        let (vertices, sides) = wythoff(&marks, &[true, false, false, false])?;
                        (Kind::Polytope { sides, vertices }, "polychoron", 1.0)
                    }
                    false => (
                        Kind::Star(Box::new(Star::of(&marks)?)),
                        "star polychoron",
                        1.0,
                    ),
                }
            }
            other => {
                // a uniform polychoron by its name or its diagram
                let (marks, rings) = diagram(other, 3).ok_or_else(usage)?;
                most(0)?;
                let (vertices, sides) = wythoff(&marks, &rings)?;
                (Kind::Polytope { sides, vertices }, "polychoron", 1.0)
            }
        };
        let (kind, canonical, reach) = match (dual, kind) {
            (false, kind) => (kind, canonical, reach),
            // the sides become the vertices and the vertices the sides
            (true, Kind::Polytope { sides, vertices }) => {
                let corners: Vec<P4> = sides.iter().map(|(n, h)| n.map(|x| x / h)).collect();
                let far = corners.iter().map(|c| dot(c, c).sqrt()).fold(0.0, f64::max);
                let sides = vertices
                    .iter()
                    .map(|v| {
                        let size = dot(v, v).sqrt();
                        (v.map(|x| x / size), 1.0 / (far * size))
                    })
                    .collect();
                (
                    Kind::Polytope {
                        sides,
                        vertices: corners.into_iter().map(|c| c.map(|x| x / far)).collect(),
                    },
                    "polychoron",
                    1.0,
                )
            }
            (true, _) => return Err("solid4d: only a polytope has a dual".into()),
        };
        let mut solid = Solid {
            form: Shape4 {
                kind,
                back: UNIT,
                shift: [0.0; 4],
            },
            name: canonical,
            numbers,
            out: UNIT,
            scales: [Expr::one(), Expr::one(), Expr::one(), Expr::one()],
            skew: false,
            reach,
            cut: None,
            written: e.clone(),
        };
        if canonical == "ellipsoid" {
            let axes: Vec<Expr> = (0..4)
                .map(|k| solid.numbers.get(k).cloned().unwrap_or_else(Expr::one))
                .collect();
            solid.change(&[text("scale"), Expr::list(axes)])?;
        }
        for step in done {
            solid.change(step)?;
        }
        Ok(solid)
    }

    /// What was done to a solid: ["rotate", "xw", degrees], ["scale", s or [four]],
    /// ["move", [four]], ["section", c].
    fn change(&mut self, step: &[Expr]) -> Result<(), String> {
        let what = step.first().and_then(word).unwrap_or_default();
        let number = |e: &Expr| {
            crate::plot::number(e)
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("solid4d: {e} is no number"))
        };
        let (matrix, inverse): ([P4; 4], [P4; 4]) = match (what.as_str(), step.len()) {
            ("rotate", 3) => {
                let plane = word(&step[1]).unwrap_or_default();
                let axis = |c: char| "xyzw".find(c);
                let mut letters = plane.chars();
                let (Some(i), Some(j), None) = (letters.next().and_then(axis), letters.next().and_then(axis), letters.next()) else {
                    return Err(format!("a solid turns in one of the planes \"xy\", \"xz\", \"xw\", \"yz\", \"yw\", \"zw\", not {}", step[1]));
                };
                if i == j {
                    return Err(format!("a plane has two axes, not {}", step[1]));
                }
                let angle = number(&step[2])?.to_radians();
                let turned = |angle: f64| {
                    let mut m = UNIT;
                    let (sine, cosine) = crate::detmath::sin_cos(angle);
                    (m[i][i], m[j][j], m[i][j], m[j][i]) = (cosine, cosine, -sine, sine);
                    m
                };
                let same = self.scales.windows(2).all(|w| w[0] == w[1]);
                self.skew |= !same;
                (turned(angle), turned(-angle))
            }
            ("scale", 2) => {
                let items: Vec<Expr> = match step[1].as_list() {
                    Some(l) if l.len() == 4 => l.to_vec(),
                    None => vec![step[1].clone(); 4],
                    _ => return Err("a solid is scaled by a number or by four of them, one for every axis".into()),
                };
                let mut m = UNIT;
                let mut back = UNIT;
                for (k, item) in items.iter().enumerate() {
                    let factor = number(item)?;
                    if factor <= 0.0 {
                        return Err("a solid is scaled by numbers above zero".into());
                    }
                    (m[k][k], back[k][k]) = (factor, 1.0 / factor);
                    // (along the axes of the solid only while it is not turned)
                    self.scales[k] = tidy(mul(vec![self.scales[k].clone(), item.clone()]));
                }
                let turned = self.out.iter().enumerate().any(|(i, row)| row.iter().enumerate().any(|(j, x)| i != j && x.abs() > 1e-12));
                let same = items.windows(2).all(|w| w[0] == w[1]);
                self.skew |= turned && !same;
                (m, back)
            }
            ("move", 2) => {
                let by = step[1].as_list().filter(|l| l.len() == 4).ok_or("a solid is moved by four numbers, [x, y, z, w]")?;
                for (k, item) in by.iter().enumerate() {
                    self.form.shift[k] += number(item)?;
                }
                return Ok(());
            }
            ("section", 2) => {
                self.cut = Some(number(&step[1])?);
                return Ok(());
            }
            _ => return Err("what is done to a solid: rotate4d(solid, \"xw\", degrees), scale4d(solid, s), move4d(solid, [x, y, z, w]), section4d(solid, w)".into()),
        };
        self.out = times(&matrix, &self.out);
        self.form.back = times(&self.form.back, &inverse);
        self.form.shift = applied(&matrix, &self.form.shift);
        Ok(())
    }

    /// How far from the origin the solid reaches at most.
    pub(crate) fn reach(&self) -> f64 {
        let stretch = self
            .out
            .iter()
            .flatten()
            .map(|x| x * x)
            .sum::<f64>()
            .sqrt()
            .min(
                // (a turn stretches nothing: the largest factor of the scaling bounds it)
                self.scales
                    .iter()
                    .filter_map(crate::plot::number)
                    .fold(0.0, f64::max)
                    .max(1e-300),
            );
        let moved = self.form.shift.iter().map(|x| x * x).sum::<f64>().sqrt();
        self.reach * stretch + moved
    }

    pub(crate) fn holds(&self, p: &P4) -> bool {
        self.form.holds(p)
    }
}

impl Shape4 {
    /// Whether a point of the world is in the solid.
    pub(crate) fn holds(&self, p: &P4) -> bool {
        let q = applied(
            &self.back,
            &[
                p[0] - self.shift[0],
                p[1] - self.shift[1],
                p[2] - self.shift[2],
                p[3] - self.shift[3],
            ],
        );
        let [x, y, z, w] = q;
        let (xy, zw, xyz) = (
            (x * x + y * y).sqrt(),
            (z * z + w * w).sqrt(),
            (x * x + y * y + z * z).sqrt(),
        );
        match &self.kind {
            Kind::Ball => xyz * xyz + w * w <= 1.0,
            Kind::Polytope { sides, .. } => sides
                .iter()
                .all(|(n, h)| n[0] * x + n[1] * y + n[2] * z + n[3] * w <= *h + 1e-12),
            Kind::Duopyramid(p, q) => polygon_gauge(*p, x, y) + polygon_gauge(*q, z, w) <= 1.0,
            Kind::Spherinder => xyz <= 1.0 && w.abs() <= 1.0,
            Kind::Cubinder => xy <= 1.0 && z.abs() <= 1.0 && w.abs() <= 1.0,
            Kind::Duocylinder => xy <= 1.0 && zw <= 1.0,
            Kind::PrismicCylinder(sides) => xy <= 1.0 && polygon_gauge(*sides, z, w) <= 1.0,
            Kind::Frustum(lower, upper) => {
                w.abs() <= 1.0 && xyz <= lower + (upper - lower) * (w + 1.0) / 2.0
            }
            Kind::Coninder => w.abs() <= 1.0 && z.abs() <= 1.0 && xy <= (z + 1.0) / 2.0,
            Kind::Cylindrone => {
                w.abs() <= 1.0 && xy <= (w + 1.0) / 2.0 && z.abs() <= (w + 1.0) / 2.0
            }
            Kind::Dicone => {
                w.abs() <= 1.0 && xy <= (w + 1.0) / 2.0 && z.abs() <= 1.0 - (w + 1.0) / 2.0
            }
            Kind::Capsule(half) => {
                let along = w.clamp(-half, *half);
                xyz * xyz + (w - along).powi(2) <= 1.0
            }
            Kind::Spheritorus(major, minor) => {
                (xy - major).powi(2) + z * z + w * w <= minor * minor
            }
            Kind::Torisphere(major, minor) => (xyz - major).powi(2) + w * w <= minor * minor,
            Kind::Tiger(a, b, r) => (xy - a).powi(2) + (zw - b).powi(2) <= r * r,
            Kind::Ditorus(a, b, r) => {
                (((xy - a).powi(2) + z * z).sqrt() - b).powi(2) + w * w <= r * r
            }
            Kind::Torinder(major, minor) => {
                w.abs() <= 1.0 && (xy - major).powi(2) + z * z <= minor * minor
            }
            Kind::Paraboloid => w.abs() <= 1.0 && xyz * xyz <= (w + 1.0) / 2.0,
            Kind::Hyperboloid(waist) => {
                w.abs() <= 1.0 && xyz * xyz <= waist * waist + (1.0 - waist * waist) * w * w
            }
            Kind::Segment(lower, upper) => w >= *lower && w <= *upper && xyz * xyz + w * w <= 1.0,
            Kind::Lens(thick) => {
                // two balls whose borders meet in the sphere of the radius 1 in w = 0
                let half = thick / 2.0;
                let middle = (1.0 - half * half) / (2.0 * half);
                let radius = half + middle;
                xyz * xyz + (w.abs() + middle).powi(2) <= radius * radius
            }
            Kind::Superquadric(power) => {
                // (a whole power by products: the same on every system)
                let whole =
                    (power.fract() == 0.0 && (1.0..=64.0).contains(power)).then_some(*power as i32);
                let raised = |v: &f64| match whole {
                    Some(k) => v.abs().powi(k),
                    None => v.abs().powf(*power),
                };
                q.iter().map(raised).sum::<f64>() <= 1.0
            }
            Kind::Brackets(shape) => shape.size(&q) <= 1.0,
            Kind::Star(star) => dot(&q, &q) < 1e-24 || star.density(&q) > 0,
            Kind::Spiked(dirs, core, spike, sharp) => {
                let far = dot(&q, &q).sqrt();
                far <= *core || {
                    let best = dirs.iter().map(|d| dot(d, &q)).fold(0.0, f64::max);
                    far <= core + spike * (best / far).min(1.0).powf(*sharp)
                }
            }
            Kind::Wall {
                band,
                major,
                minor,
                tube,
                twist,
                turns,
            } => in_wall(&q, *band, *major, *minor, *tube, *twist, turns),
            Kind::Rotor => {
                let (top, base) = (2.0 / 3f64.sqrt(), -1.0 / 3f64.sqrt());
                (xyz * xyz + (w - top).powi(2))
                    .sqrt()
                    .max(((xyz + 1.0).powi(2) + (w - base).powi(2)).sqrt())
                    <= 2.0
            }
            Kind::Common(corners, radius) => corners
                .iter()
                .all(|c| (0..4).map(|k| (q[k] - c[k]).powi(2)).sum::<f64>() <= radius * radius),
        }
    }

    /// How many times the solid covers a point of the world: the density of a star
    /// polychoron, else 1 inside and 0 outside.
    fn density(&self, p: &P4) -> usize {
        match &self.kind {
            Kind::Star(star) => star.density(&applied(
                &self.back,
                &[0, 1, 2, 3].map(|k| p[k] - self.shift[k]),
            )),
            _ => usize::from(self.holds(p)),
        }
    }

    /// The part of the line through a place along an axis that lies in the solid: its
    /// lowest and its highest point between two ends.
    pub(crate) fn fibre(&self, at: &P4, axis: usize, ends: (f64, f64)) -> Option<(f64, f64)> {
        if let Kind::Polytope { sides, .. } = &self.kind {
            // every side bounds the line from one end
            let mut start = *at;
            start[axis] = 0.0;
            let from = applied(&self.back, &[0, 1, 2, 3].map(|k| start[k] - self.shift[k]));
            let along = [0, 1, 2, 3].map(|k| self.back[k][axis]);
            let (mut low, mut high) = ends;
            for (n, h) in sides {
                let (rate, left) = (dot(n, &along), h + 1e-12 - dot(n, &from));
                if rate > 1e-14 {
                    high = high.min(left / rate);
                } else if rate < -1e-14 {
                    low = low.max(left / rate);
                } else if left < 0.0 {
                    return None;
                }
                if low > high {
                    return None;
                }
            }
            return Some((low, high));
        }
        if let Kind::Star(star) = &self.kind {
            let mut start = *at;
            start[axis] = 0.0;
            let from = applied(&self.back, &[0, 1, 2, 3].map(|k| start[k] - self.shift[k]));
            return star.fibre(&from, &[0, 1, 2, 3].map(|k| self.back[k][axis]), ends);
        }
        const SAMPLES: usize = 40;
        let inside = |v: f64| {
            let mut p = *at;
            p[axis] = v;
            self.holds(&p)
        };
        let place = |k: usize| ends.0 + (ends.1 - ends.0) * (k as f64 + 0.5) / SAMPLES as f64;
        let first = (0..SAMPLES).find(|k| inside(place(*k)))?;
        let last = (first..SAMPLES).rev().find(|k| inside(place(*k)))?;
        let edge = |mut within: f64, mut without: f64| {
            for _ in 0..12 {
                let middle = (within + without) / 2.0;
                match inside(middle) {
                    true => within = middle,
                    false => without = middle,
                }
            }
            within
        };
        let low = match first {
            0 => match inside(ends.0) {
                true => ends.0,
                false => edge(place(0), ends.0),
            },
            k => edge(place(k), place(k - 1)),
        };
        let high = match last + 1 == SAMPLES {
            true => match inside(ends.1) {
                true => ends.1,
                false => edge(place(last), ends.1),
            },
            false => edge(place(last), place(last + 1)),
        };
        Some((low, high))
    }
}

impl Solid {
    /// The vertices of a polytope in the world.
    fn corners(&self) -> Option<Vec<P4>> {
        let vertices = match &self.form.kind {
            Kind::Polytope { vertices, .. } => vertices,
            Kind::Star(star) => &star.vertices,
            _ => return None,
        };
        Some(
            vertices
                .iter()
                .map(|v| {
                    let p = applied(&self.out, v);
                    [0, 1, 2, 3].map(|k| p[k] + self.form.shift[k])
                })
                .collect(),
        )
    }

    /// The pairs of vertices that an edge joins: the sides that hold both of them leave
    /// one line only.
    fn edges(&self) -> Option<Vec<(usize, usize)>> {
        let (vertices, sides) = match &self.form.kind {
            Kind::Polytope { vertices, sides } => (vertices, sides),
            Kind::Star(star) => return Some(star.edges.clone()),
            _ => return None,
        };
        let mut on: Vec<Vec<u32>> = vec![Vec::new(); vertices.len()];
        let mut shared: std::collections::HashMap<(u32, u32), u8> =
            std::collections::HashMap::new();
        for (f, (n, h)) in sides.iter().enumerate() {
            let own: Vec<u32> = (0..vertices.len())
                .filter(|k| (dot(n, &vertices[*k]) - h).abs() < 1e-9)
                .map(|k| k as u32)
                .collect();
            for (i, a) in own.iter().enumerate() {
                on[*a as usize].push(f as u32);
                for b in &own[i + 1..] {
                    let count = shared.entry((*a, *b)).or_insert(0);
                    *count = count.saturating_add(1);
                }
            }
        }
        let mut pairs: Vec<(usize, usize)> = shared
            .into_iter()
            .filter(|((a, b), count)| {
                *count >= 3 && {
                    let common: Vec<P4> = on[*a as usize]
                        .iter()
                        .filter(|f| on[*b as usize].contains(f))
                        .map(|f| sides[*f as usize].0)
                        .collect();
                    rank(&common) == 3
                }
            })
            .map(|((a, b), _)| (a as usize, b as usize))
            .collect();
        pairs.sort_unstable();
        Some(pairs)
    }

    pub(crate) fn shown(&self) -> String {
        format!("{}", self.written)
    }

    /// The exact bulk (the content of four dimensions), where a formula gives it.
    fn bulk(&self) -> Option<Expr> {
        let pi = Expr::pi;
        let n = |k: usize, usual: Expr| self.numbers.get(k).cloned().unwrap_or(usual);
        let rat = Expr::rat;
        let sq = |e: Expr| pow(e, Expr::int(2));
        let polygon_area = |sides: Expr| {
            // (p/2) sin(2 pi/p)
            mul(vec![
                div(sides.clone(), Expr::int(2)),
                crate::funcs::func("sin", vec![div(mul(vec![Expr::int(2), pi()]), sides)]),
            ])
        };
        let unit = match &self.form.kind {
            Kind::Ball => div(sq(pi()), Expr::int(2)),
            Kind::Polytope { .. } => match self.name {
                // with the circumradius 1
                "5-cell" => mul(vec![rat(25, 384), pow(Expr::int(5), rat(1, 2))]),
                "tesseract" => Expr::one(),
                "16-cell" => rat(2, 3),
                "24-cell" => Expr::int(2),
                "duoprism" => mul(vec![
                    polygon_area(n(0, Expr::int(4))),
                    polygon_area(n(1, Expr::int(4))),
                ]),
                _ => {
                    let Kind::Polytope { sides, vertices } = &self.form.kind else {
                        return None;
                    };
                    rounded(polychoron_content(vertices, sides).0, 12)
                }
            },
            Kind::Dicone => div(pi(), Expr::int(3)),
            Kind::Duopyramid(_, _) => div(
                mul(vec![
                    polygon_area(n(0, Expr::int(4))),
                    polygon_area(n(1, Expr::int(4))),
                ]),
                Expr::int(6),
            ),
            Kind::Spherinder => mul(vec![rat(8, 3), pi()]),
            Kind::Cubinder => mul(vec![Expr::int(4), pi()]),
            Kind::Duocylinder => sq(pi()),
            Kind::PrismicCylinder(_) => mul(vec![pi(), polygon_area(n(0, Expr::int(6)))]),
            Kind::Frustum(_, _) => {
                let (a, b) = match self.name {
                    "hypercone" => (Expr::zero(), Expr::one()),
                    _ => (n(0, rat(1, 2)), n(1, Expr::one())),
                };
                // the integral of 4/3 pi r^3 over the height 2
                mul(vec![
                    rat(2, 3),
                    pi(),
                    add(vec![
                        pow(a.clone(), Expr::int(3)),
                        mul(vec![sq(a.clone()), b.clone()]),
                        mul(vec![a, sq(b.clone())]),
                        pow(b, Expr::int(3)),
                    ]),
                ])
            }
            Kind::Coninder => mul(vec![rat(4, 3), pi()]),
            Kind::Cylindrone => pi(),
            Kind::Capsule(_) => add(vec![
                div(sq(pi()), Expr::int(2)),
                mul(vec![rat(8, 3), pi(), n(0, Expr::one())]),
            ]),
            Kind::Spheritorus(major, minor) if major >= minor => mul(vec![
                rat(8, 3),
                sq(pi()),
                n(0, Expr::one()),
                pow(n(1, rat(2, 5)), Expr::int(3)),
            ]),
            Kind::Torisphere(major, minor) if major >= minor => {
                let (big, small) = (n(0, Expr::one()), n(1, rat(2, 5)));
                add(vec![
                    mul(vec![Expr::int(4), sq(pi()), sq(big), sq(small.clone())]),
                    mul(vec![sq(pi()), pow(small, Expr::int(4))]),
                ])
            }
            Kind::Tiger(a, b, r) if a >= r && b >= r => mul(vec![
                Expr::int(4),
                pow(pi(), Expr::int(3)),
                n(0, Expr::one()),
                n(1, Expr::one()),
                sq(n(2, rat(3, 10))),
            ]),
            Kind::Ditorus(a, b, r) if b >= r && *a >= b + r => mul(vec![
                Expr::int(4),
                pow(pi(), Expr::int(3)),
                n(0, Expr::one()),
                n(1, rat(1, 2)),
                sq(n(2, rat(1, 5))),
            ]),
            Kind::Torinder(major, minor) if major >= minor => mul(vec![
                Expr::int(4),
                sq(pi()),
                n(0, Expr::one()),
                sq(n(1, rat(2, 5))),
            ]),
            Kind::Paraboloid => mul(vec![rat(16, 15), pi()]),
            Kind::Superquadric(_) => {
                let p = n(0, Expr::int(4));
                let gamma = |x: Expr| crate::funcs::func("gamma", vec![x]);
                div(
                    pow(
                        mul(vec![
                            Expr::int(2),
                            gamma(add(vec![Expr::one(), div(Expr::one(), p.clone())])),
                        ]),
                        Expr::int(4),
                    ),
                    gamma(add(vec![Expr::one(), div(Expr::int(4), p)])),
                )
            }
            _ => return None,
        };
        Some(tidy(mul(vec![unit, mul(self.scales.to_vec())])))
    }

    /// The exact surface (the content of three dimensions of the border), for a solid
    /// that keeps its shape.
    fn border(&self) -> Option<Expr> {
        let same = self.scales.windows(2).all(|w| w[0] == w[1]);
        if !same || self.skew {
            return None;
        }
        let pi = Expr::pi;
        let n = |k: usize, usual: Expr| self.numbers.get(k).cloned().unwrap_or(usual);
        let rat = Expr::rat;
        let sq = |e: Expr| pow(e, Expr::int(2));
        let root = |e: Expr| pow(e, rat(1, 2));
        let sin_of = |sides: Expr, twice: bool| {
            crate::funcs::func(
                "sin",
                vec![div(
                    mul(vec![Expr::int(if twice { 2 } else { 1 }), pi()]),
                    sides,
                )],
            )
        };
        let area = |sides: Expr| mul(vec![div(sides.clone(), Expr::int(2)), sin_of(sides, true)]);
        let around = |sides: Expr| mul(vec![Expr::int(2), sides.clone(), sin_of(sides, false)]);
        let unit = match &self.form.kind {
            Kind::Ball => mul(vec![Expr::int(2), sq(pi())]),
            Kind::Polytope { .. } => match self.name {
                "tesseract" => Expr::int(8),
                // 16 regular tetrahedra of the edge sqrt(2)
                "16-cell" => rat(16, 3),
                // 24 octahedra of the edge 1
                "24-cell" => mul(vec![Expr::int(8), root(Expr::int(2))]),
                "duoprism" => {
                    let (p, q) = (n(0, Expr::int(4)), n(1, Expr::int(4)));
                    add(vec![
                        mul(vec![area(p.clone()), around(q.clone())]),
                        mul(vec![area(q), around(p)]),
                    ])
                }
                _ => {
                    let Kind::Polytope { sides, vertices } = &self.form.kind else {
                        return None;
                    };
                    rounded(polychoron_content(vertices, sides).1, 12)
                }
            },
            Kind::Spherinder => mul(vec![rat(32, 3), pi()]),
            Kind::Cubinder => mul(vec![Expr::int(16), pi()]),
            Kind::Duocylinder => mul(vec![Expr::int(4), sq(pi())]),
            Kind::PrismicCylinder(_) => {
                let sides = n(0, Expr::int(6));
                add(vec![
                    mul(vec![Expr::int(2), pi(), area(sides.clone())]),
                    mul(vec![pi(), around(sides)]),
                ])
            }
            Kind::Frustum(_, _) => {
                let (a, b) = match self.name {
                    "hypercone" => (Expr::zero(), Expr::one()),
                    _ => (n(0, rat(1, 2)), n(1, Expr::one())),
                };
                let slant = root(add(vec![
                    Expr::int(4),
                    sq(add(vec![a.clone(), mul(vec![Expr::int(-1), b.clone()])])),
                ]));
                add(vec![
                    mul(vec![
                        rat(4, 3),
                        pi(),
                        add(vec![
                            pow(a.clone(), Expr::int(3)),
                            pow(b.clone(), Expr::int(3)),
                        ]),
                    ]),
                    mul(vec![
                        rat(4, 3),
                        pi(),
                        slant,
                        add(vec![sq(a.clone()), mul(vec![a, b.clone()]), sq(b)]),
                    ]),
                ])
            }
            Kind::Capsule(_) => add(vec![
                mul(vec![Expr::int(2), sq(pi())]),
                mul(vec![Expr::int(8), pi(), n(0, Expr::one())]),
            ]),
            Kind::Spheritorus(major, minor) if major >= minor => mul(vec![
                Expr::int(8),
                sq(pi()),
                n(0, Expr::one()),
                sq(n(1, rat(2, 5))),
            ]),
            Kind::Torisphere(major, minor) if major >= minor => {
                let (big, small) = (n(0, Expr::one()), n(1, rat(2, 5)));
                add(vec![
                    mul(vec![Expr::int(8), sq(pi()), sq(big), small.clone()]),
                    mul(vec![Expr::int(4), sq(pi()), pow(small, Expr::int(3))]),
                ])
            }
            Kind::Tiger(a, b, r) if a >= r && b >= r => mul(vec![
                Expr::int(8),
                pow(pi(), Expr::int(3)),
                n(0, Expr::one()),
                n(1, Expr::one()),
                n(2, rat(3, 10)),
            ]),
            Kind::Ditorus(a, b, r) if b >= r && *a >= b + r => mul(vec![
                Expr::int(8),
                pow(pi(), Expr::int(3)),
                n(0, Expr::one()),
                n(1, rat(1, 2)),
                n(2, rat(1, 5)),
            ]),
            Kind::Torinder(major, minor) if major >= minor => {
                let (big, small) = (n(0, Expr::one()), n(1, rat(2, 5)));
                add(vec![
                    mul(vec![Expr::int(4), sq(pi()), big.clone(), sq(small.clone())]),
                    mul(vec![Expr::int(8), sq(pi()), big, small]),
                ])
            }
            _ => return None,
        };
        Some(tidy(mul(vec![
            unit,
            pow(self.scales[0].clone(), Expr::int(3)),
        ])))
    }

    /// The bulk by lines along w through the middles of the cells of a grid: the parts of
    /// every line that lie in the solid, their ends found by halving.
    fn counted(&self) -> f64 {
        const CELLS: usize = 128;
        let half = self.reach() * 1.02;
        // (a thin wall needs samples that are nearer to each other)
        let samples = match &self.form.kind {
            Kind::Wall { tube, .. } => ((4.0 * half / tube).ceil() as usize).clamp(48, 480),
            Kind::Star(_) | Kind::Spiked(..) => 96,
            _ => 48,
        };
        let step = 2.0 * half / CELLS as f64;
        let place = |k: usize| -half + step * (k as f64 + 0.5);
        let threads = std::thread::available_parallelism()
            .map_or(4, |n| n.get())
            .min(CELLS);
        let form = &self.form;
        let reach = self.reach();
        let total: f64 = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..threads)
                .map(|t| {
                    scope.spawn(move || {
                        let mut inside = 0.0;
                        for i in (t..CELLS).step_by(threads) {
                            for j in 0..CELLS {
                                for k in 0..CELLS {
                                    let mut p = [place(i), place(j), place(k), 0.0];
                                    if p[0] * p[0] + p[1] * p[1] + p[2] * p[2] > reach * reach {
                                        continue;
                                    }
                                    let mut holds = |w: f64| {
                                        p[3] = w;
                                        form.holds(&p)
                                    };
                                    let at =
                                        |s: usize| -half + 2.0 * half * s as f64 / samples as f64;
                                    let mut before = holds(at(0));
                                    for s in 1..=samples {
                                        let now = holds(at(s));
                                        match (before, now) {
                                            (true, true) => inside += at(s) - at(s - 1),
                                            (false, false) => {}
                                            _ => {
                                                // the place of the border between the two
                                                let (mut within, mut without) = if before {
                                                    (at(s - 1), at(s))
                                                } else {
                                                    (at(s), at(s - 1))
                                                };
                                                for _ in 0..14 {
                                                    let middle = (within + without) / 2.0;
                                                    match holds(middle) {
                                                        true => within = middle,
                                                        false => without = middle,
                                                    }
                                                }
                                                inside += if before {
                                                    within - at(s - 1)
                                                } else {
                                                    at(s) - within
                                                };
                                            }
                                        }
                                        before = now;
                                    }
                                }
                            }
                        }
                        inside
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap_or(0.0)).sum()
        });
        total * step.powi(3)
    }
}

/// The lines of a solid in 4D: the edges of a polytope, else the borders of its cuts by
/// planes of two axes at several places of the other two.
fn wires(solid: &Solid) -> Vec<[P4; 2]> {
    if let (Some(corners), Some(edges)) = (solid.corners(), solid.edges()) {
        return edges
            .into_iter()
            .map(|(i, j)| [corners[i], corners[j]])
            .collect();
    }
    const CELLS: usize = 72;
    let half = solid.reach() * 1.02;
    let step = 2.0 * half / CELLS as f64;
    let mut lines = Vec::new();
    let places = [-0.5, 0.0, 0.5];
    for (i, j) in [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)] {
        let others: Vec<usize> = (0..4).filter(|k| *k != i && *k != j).collect();
        for a in places {
            for b in places {
                let point = |u: f64, v: f64| -> P4 {
                    let mut p = [0.0; 4];
                    (p[i], p[j], p[others[0]], p[others[1]]) = (
                        u,
                        v,
                        a * half * 0.9 + solid.form.shift[others[0]],
                        b * half * 0.9 + solid.form.shift[others[1]],
                    );
                    p
                };
                let grid: Vec<bool> = (0..=CELLS)
                    .flat_map(|r| (0..=CELLS).map(move |c| (r, c)))
                    .map(|(r, c)| {
                        solid.holds(&point(-half + step * c as f64, -half + step * r as f64))
                    })
                    .collect();
                let at = |r: usize, c: usize| grid[r * (CELLS + 1) + c];
                // where the border crosses the side of a cell between an inner and an outer corner
                let crossing = |p: (f64, f64), q: (f64, f64)| -> (f64, f64) {
                    let (mut within, mut without) = (p, q);
                    for _ in 0..10 {
                        let middle = ((within.0 + without.0) / 2.0, (within.1 + without.1) / 2.0);
                        match solid.holds(&point(middle.0, middle.1)) {
                            true => within = middle,
                            false => without = middle,
                        }
                    }
                    within
                };
                for r in 0..CELLS {
                    for c in 0..CELLS {
                        let corners = [(r, c), (r, c + 1), (r + 1, c + 1), (r + 1, c)];
                        let inside: Vec<bool> = corners.iter().map(|(r, c)| at(*r, *c)).collect();
                        if inside.iter().all(|x| *x) || !inside.iter().any(|x| *x) {
                            continue;
                        }
                        let xy = |(r, c): (usize, usize)| {
                            (-half + step * c as f64, -half + step * r as f64)
                        };
                        let mut cuts = Vec::new();
                        for k in 0..4 {
                            let (p, q) = (corners[k], corners[(k + 1) % 4]);
                            match (at(p.0, p.1), at(q.0, q.1)) {
                                (true, false) => cuts.push(crossing(xy(p), xy(q))),
                                (false, true) => cuts.push(crossing(xy(q), xy(p))),
                                _ => {}
                            }
                        }
                        for pair in cuts.chunks(2) {
                            if let [p, q] = pair {
                                lines.push([point(p.0, p.1), point(q.0, q.1)]);
                            }
                        }
                    }
                }
            }
        }
    }
    lines
}

/// `wire4d(solid[, "plane"[, "plane"]])`: the lines of a solid as an eye outside of the
/// fourth dimension sees them: what is farther along w is smaller. With frames the solid
/// turns once in the plane that is named ("xw" unless given; two planes: in both).
pub(crate) fn wire(a: &[Expr], frames: usize) -> Result<Drawing, String> {
    let usage = "wire4d(solid4d(\"name\", ...)[, \"xw\"[, \"yz\"]]): the lines of a solid of four dimensions; with frames = n it turns in the planes that are named";
    let solid = Solid::read(a.first().ok_or(usage)?)?;
    if a.len() > 3 {
        return Err(usage.into());
    }
    let mut planes: Vec<(usize, usize)> = Vec::new();
    for e in &a[1..] {
        let plane = word(e).unwrap_or_default();
        let axis = |c: char| "xyzw".find(c);
        let mut letters = plane.chars();
        match (
            letters.next().and_then(axis),
            letters.next().and_then(axis),
            letters.next(),
        ) {
            (Some(i), Some(j), None) if i != j => planes.push((i, j)),
            _ => {
                return Err(format!(
                    "a solid turns in one of the planes \"xy\", \"xz\", \"xw\", \"yz\", \"yw\", \"zw\", not {e}"
                ));
            }
        }
    }
    if planes.is_empty() {
        planes.push((0, 3));
    }
    let lines = wires(&solid);
    if lines.is_empty() {
        return Err("wire4d: the solid has no lines in its box".into());
    }
    let reach = solid.reach();
    let eye = 3.0 * reach;
    let stage = |share: f64| -> Item {
        let angle = std::f64::consts::TAU * share;
        let turned = |p: &P4| -> P4 {
            let mut q = *p;
            for (i, j) in &planes {
                let (a, b) = (q[*i], q[*j]);
                let (sine, cosine) = crate::detmath::sin_cos(angle);
                (q[*i], q[*j]) = (a * cosine - b * sine, a * sine + b * cosine);
            }
            q
        };
        // the lines that lie farther along w come first: the colours go along w
        let mut all: Vec<([P4; 2], f64)> = lines
            .iter()
            .map(|pair| {
                let (p, q) = (turned(&pair[0]), turned(&pair[1]));
                ([p, q], (p[3] + q[3]) / 2.0)
            })
            .collect();
        all.sort_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal));
        let flat = |p: &P4| -> Point {
            let scale = eye / (eye - p[3]);
            [p[0] * scale, p[1] * scale, p[2] * scale]
        };
        Item::Segments {
            pairs: all
                .iter()
                .map(|(pair, _)| [flat(&pair[0]), flat(&pair[1])])
                .collect(),
            paint: Paint::Along,
            weight: 1.0,
        }
    };
    let mut drawing = Drawing::new(Space::Solid, vec![stage(0.0)]);
    if frames > 1 {
        drawing.stages = (0..frames)
            .map(|k| vec![stage(k as f64 / frames as f64)])
            .collect();
    }
    let names = |(i, j): &(usize, usize)| {
        format!("{}{}", ["x", "y", "z", "w"][*i], ["x", "y", "z", "w"][*j])
    };
    drawing.facts = vec![format!(
        "wire4d of {}: {} lines{}, seen from the place {} along w: what lies farther along w is smaller, the colours go along w{}",
        solid.shown(),
        lines.len(),
        if solid.corners().is_some() {
            " (the edges)"
        } else {
            " (the borders of cuts by planes of two axes)"
        },
        crate::plot::label(eye),
        match frames > 1 {
            true => format!(
                "; in {frames} frames it turns once in the plane{} {}",
                if planes.len() == 1 { "" } else { "s" },
                planes.iter().map(names).collect::<Vec<_>>().join(" and ")
            ),
            false => String::new(),
        }
    )];
    Ok(drawing)
}

/// What the colours of a solid show for plot4d.
pub(crate) fn colours_of(solid: &Solid, fill: crate::plot4d::Fill, axis: &str) -> String {
    use crate::plot4d::Fill;
    match (solid.cut, fill) {
        (Some(c), _) => format!(
            "how deep the place lies in the solid along {axis}, in the cut {axis} = {}",
            crate::plot::label(c)
        ),
        (None, Fill::Below) => format!("the lowest {axis} of the solid"),
        (None, Fill::Between) => format!("how far the solid reaches along {axis}"),
        _ => format!("the highest {axis} of the solid"),
    }
}

/// The commands of solids of four dimensions.
pub(crate) fn call(name: &str, a: &[Expr]) -> Option<Res> {
    let n = a.len();
    let solid_at = |k: usize| -> Result<Solid, String> {
        Solid::read(
            a.get(k)
                .ok_or_else(|| format!("{name}(solid4d(\"name\", ...), ...)"))?,
        )
    };
    let changed = |step: Vec<Expr>| -> Res {
        let args = a[0]
            .fn_args("solid4d")
            .ok_or_else(|| format!("{name}(solid4d(\"name\", ...), ...)"))?;
        let mut all = args.to_vec();
        all.push(Expr::list(step));
        let made = Expr::call("solid4d", all);
        Solid::read(&made)?;
        Ok(made)
    };
    Some(match (name, n) {
        ("solid4d", 1..) => (|| -> Res {
            let made = Expr::call("solid4d", a.to_vec());
            Solid::read(&made)?;
            Ok(made)
        })(),
        ("rotate4d", 3) => changed(vec![text("rotate"), a[1].clone(), a[2].clone()]),
        ("scale4d", 2) => changed(vec![text("scale"), a[1].clone()]),
        ("move4d", 2) => changed(vec![text("move"), a[1].clone()]),
        ("section4d", 2) => changed(vec![text("section"), a[1].clone()]),
        ("inside4d", 2) => {
            (|| -> Res {
                let solid = solid_at(0)?;
                let one = |p: &Expr| -> Res {
                    let items = p.as_list().filter(|l| l.len() == 4).ok_or("inside4d(solid, [x, y, z, w]): a point of four numbers (or a list of points)")?;
                    let mut q = [0.0; 4];
                    for (k, item) in items.iter().enumerate() {
                        q[k] = crate::plot::number(item)
                            .ok_or_else(|| format!("inside4d: {item} is no number"))?;
                    }
                    Ok(Expr::Bool(solid.holds(&q)))
                };
                match a[1].as_list() {
                    Some(points) if points.iter().all(|p| p.as_list().is_some()) => Ok(Expr::list(
                        points.iter().map(one).collect::<Result<_, _>>()?,
                    )),
                    _ => one(&a[1]),
                }
            })()
        }
        ("density4d", 2) => {
            (|| -> Res {
                let solid = solid_at(0)?;
                let one = |p: &Expr| -> Res {
                    let items = p.as_list().filter(|l| l.len() == 4).ok_or("density4d(solid, [x, y, z, w]): a point of four numbers (or a list of points)")?;
                    let mut q = [0.0; 4];
                    for (k, item) in items.iter().enumerate() {
                        q[k] = crate::plot::number(item)
                            .ok_or_else(|| format!("density4d: {item} is no number"))?;
                    }
                    Ok(Expr::int(solid.form.density(&q) as i64))
                };
                match a[1].as_list() {
                    Some(points) if points.iter().all(|p| p.as_list().is_some()) => Ok(Expr::list(
                        points.iter().map(one).collect::<Result<_, _>>()?,
                    )),
                    _ => one(&a[1]),
                }
            })()
        }
        ("volume4d", 1) => (|| -> Res {
            let solid = solid_at(0)?;
            Ok(match solid.bulk() {
                Some(exact) => exact,
                // (by counting the cells of a grid: three digits)
                None => rounded(solid.counted(), 3),
            })
        })(),
        ("surface4d", 1) => (|| -> Res {
            let solid = solid_at(0)?;
            solid.border().ok_or_else(|| format!("surface4d: no formula gives the surface of {} as it is (a solid that is scaled along one axis more than along another, or one without a formula)", solid.shown()))
        })(),
        ("vertices4d", 1) => (|| -> Res {
            let solid = solid_at(0)?;
            let corners = solid
                .corners()
                .ok_or_else(|| format!("vertices4d: {} is no polytope", solid.shown()))?;
            Ok(Expr::list(
                corners
                    .iter()
                    .map(|p| {
                        Expr::list(
                            p.iter()
                                .map(|x| {
                                    crate::mfun::value(&rug::Float::with_val(
                                        53,
                                        if x.abs() < 1e-15 { 0.0 } else { *x },
                                    ))
                                })
                                .collect(),
                        )
                    })
                    .collect(),
            ))
        })(),
        ("edges4d", 1) => (|| -> Res {
            let solid = solid_at(0)?;
            let edges = solid
                .edges()
                .ok_or_else(|| format!("edges4d: {} is no polytope", solid.shown()))?;
            Ok(Expr::list(
                edges
                    .iter()
                    .map(|(i, j)| {
                        Expr::list(vec![Expr::int(*i as i64 + 1), Expr::int(*j as i64 + 1)])
                    })
                    .collect(),
            ))
        })(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn said(input: &str) -> String {
        crate::eval::run_line(input).map_or_else(|e| format!("error: {e}"), |lines| lines.join("|"))
    }

    #[test]
    fn the_regular_polychora_have_their_vertices_and_edges() {
        for (name, vertices, edges) in [
            ("5-cell", 5, 10),
            ("tesseract", 16, 32),
            ("16-cell", 8, 24),
            ("24-cell", 24, 96),
            ("600-cell", 120, 720),
            ("120-cell", 600, 1200),
        ] {
            let solid = Solid::read(&Expr::call("solid4d", vec![text(name)])).expect("a solid");
            assert_eq!(solid.corners().expect("vertices").len(), vertices, "{name}");
            assert_eq!(solid.edges().expect("edges").len(), edges, "{name}");
            // the middle is inside, a vertex lies on the border, beyond it is outside
            let corner = solid.corners().expect("vertices")[0];
            assert!(
                solid.holds(&[0.0; 4])
                    && solid.holds(&corner.map(|x| x * 0.999))
                    && !solid.holds(&corner.map(|x| x * 1.001)),
                "{name}"
            );
        }
        assert_eq!(said("length(edges4d(solid4d(\"duoprism\", 3, 5)))"), "30");
    }

    #[test]
    fn points_bulks_and_surfaces() {
        assert_eq!(
            said("inside4d(solid4d(\"duocylinder\"), [[1/2, 1/2, 1/2, 1/2], [1, 1/2, 0, 0]])"),
            "[true, false]"
        );
        assert_eq!(
            said("volume4d(solid4d(\"ball\"))"),
            "pi^2/2|  ~ 4.9348022005446793094172454999381"
        );
        assert_eq!(
            said("surface4d(scale4d(solid4d(\"ball\"), 2))"),
            "16pi^2|  ~ 157.91367041742973790135185599802"
        );
        assert_eq!(
            said("volume4d(scale4d(solid4d(\"tesseract\"), [1, 2, 3, 4]))"),
            "24"
        );
        assert_eq!(
            said("volume4d(solid4d(\"duocylinder\"))"),
            "pi^2|  ~ 9.8696044010893586188344909998762"
        );
        assert_eq!(
            said("volume4d(solid4d(\"tiger\", 2, 3, 1/2))"),
            "6pi^3|  ~ 186.03766008179892105285789040261"
        );
        // turned and moved: the same point of the solid
        assert_eq!(
            said(
                "inside4d(move4d(rotate4d(solid4d(\"tesseract\"), \"xw\", 45), [3, 0, 0, 0]), [[3.6, 0, 0, 0], [3.8, 0, 0, 0]])"
            ),
            "[true, false]"
        );
        assert!(said("solid4d(\"cube\")").contains("the solids are"));
        assert!(said("solid4d(\"bracketope\", \"[(II)(II)\")").contains("brackets"));
        assert_eq!(
            said(
                "inside4d(solid4d(\"bracketope\", \"<(II)(II)>\"), [[0.4, 0, 0.5, 0], [0.4, 0.4, 0.5, 0]])"
            ),
            "[true, false]"
        );
    }

    #[test]
    fn a_star_polychoron_is_what_its_simplices_cover() {
        // the same simplices give the convex polychora that the sides of the same mirrors give
        for (marks, name) in [
            ([3.0, 3.0, 5.0], "{3,3,5}"),
            ([4.0, 3.0, 3.0], "{4,3,3}"),
            ([3.0, 4.0, 3.0], "{3,4,3}"),
            ([5.0, 3.0, 3.0], "{5,3,3}"),
            ([3.0, 3.0, 3.0], "{3,3,3}"),
        ] {
            let star = Star::of(&marks).expect("a polychoron");
            let solid = Solid::read(&Expr::call("solid4d", vec![text(name)])).expect("a solid");
            let mut seed = 7u64;
            let mut next = || {
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (seed >> 11) as f64 / (1u64 << 53) as f64 * 2.2 - 1.1
            };
            for _ in 0..3000 {
                let p = [next(), next(), next(), next()];
                assert_eq!(
                    star.density(&p),
                    usize::from(solid.holds(&p)),
                    "{name} at {p:?}"
                );
            }
        }
        // along a line: the ends that the steps of the density give are those of many samples
        for symbol in ["{5/2,5,3}", "{5,5/2,5}", "{3,3,5/2}", "{5/2,3,3}"] {
            let solid = Solid::read(&Expr::call("solid4d", vec![text(symbol)])).expect("a solid");
            let mut seed = 11u64;
            let mut next = || {
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (seed >> 11) as f64 / (1u64 << 53) as f64 * 1.6 - 0.8
            };
            let mut found = 0;
            for _ in 0..200 {
                let at = [next(), next(), next(), 0.0];
                let fibre = solid.form.fibre(&at, 3, (-1.2, 1.2));
                let holds = |w: f64| solid.holds(&[at[0], at[1], at[2], w]);
                let inside: Vec<f64> = (0..=2400)
                    .map(|k| -1.2 + 2.4 * k as f64 / 2400.0)
                    .filter(|w| holds(*w))
                    .collect();
                match fibre {
                    // (a line may clip a ridge: a part thinner than the samples lie apart)
                    Some((low, high)) => {
                        assert!(
                            !holds(low - 1e-6) && !holds(high + 1e-6),
                            "{symbol} at {at:?}: outside of {low} {high}"
                        );
                        assert!(
                            high - low < 4e-6 || (holds(low + 1e-6) && holds(high - 1e-6)),
                            "{symbol} at {at:?}: inside of {low} {high}"
                        );
                        assert!(
                            inside
                                .iter()
                                .all(|w| (low - 1e-9..=high + 1e-9).contains(w)),
                            "{symbol} at {at:?}: {low} {high} against {inside:?}"
                        );
                        found += 1;
                    }
                    None => assert!(
                        inside.is_empty(),
                        "{symbol} at {at:?}: nothing against {inside:?}"
                    ),
                }
            }
            assert!(found > 20, "{symbol}: {found} lines through it");
        }
        assert_eq!(
            said("density4d(solid4d(\"grand 600-cell\"), [0.001, 0.002, -0.001, 0.0005])"),
            "191"
        );
        assert_eq!(
            said(
                "[length(vertices4d(solid4d(\"snub 24-cell\"))), length(edges4d(solid4d(\"snub 24-cell\"))), length(edges4d(solid4d(\"dual rectified tesseract\")))]"
            ),
            "[96, 432, 88]"
        );
    }

    #[test]
    fn walls_spikes_and_bodies_of_one_width() {
        // on the surface, and farther from it than half of the thickness
        assert_eq!(
            said(
                "inside4d(solid4d(\"Klein bottle\"), [[1.4, 0, 0, 0], [1.4, 0, 0, 0.04], [1.4, 0, 0, 0.06], [1, 0, 0.4, 0], [0, 0, 0, 0]])"
            ),
            "[true, true, false, true, false]"
        );
        assert_eq!(
            said(
                "inside4d(solid4d(\"Moebius band\"), [[1, 0, 0.39, 0], [1, 0, 0.47, 0], [-1, 0, 0, 0.39], [-1, 0, 0.39, 0]])"
            ),
            "[true, false, true, false]"
        );
        assert_eq!(
            said(
                "inside4d(solid4d(\"projective plane\"), [[0, 0, 0, 0], [1, 0, 0, 0], [0.5, 0, 0.5, 0], [0.5, 0, 0, 0]])"
            ),
            "[true, true, true, false]"
        );
        assert_eq!(
            said(
                "inside4d(solid4d(\"reuleaux\"), [[0, 0, 0, 1.15], [0, 0, 0, 1.16], [0.99, 0, 0, -0.57], [1.01, 0, 0, -0.57]])"
            ),
            "[true, false, true, false]"
        );
        assert_eq!(
            said(
                "inside4d(solid4d(\"star cell\", 8), [[0.99, 0, 0, 0], [1.01, 0, 0, 0], [0.35, 0.35, 0, 0], [0.4, 0.4, 0, 0]])"
            ),
            "[true, false, true, false]"
        );
        assert_eq!(
            said("volume4d(solid4d(\"ellipsoid\", 1, 2, 3, 4))"),
            "12pi^2|  ~ 118.43525281307230342601389199851"
        );
        assert!(said("solid4d(\"dual ball\")").contains("only a polytope has a dual"));
        assert!(said("solid4d(\"Klein bottle\", 0.3, 0.4)").contains("larger radius"));
        assert!(said("solid4d(\"prism\", \"snub cub\")").contains("is no uniform polyhedron"));
    }

    #[test]
    fn counting_is_near_the_formula() {
        let ball = Solid::read(&Expr::call("solid4d", vec![text("ball")])).expect("a solid");
        let exact = std::f64::consts::PI.powi(2) / 2.0;
        assert!(
            (ball.counted() - exact).abs() < 0.01 * exact,
            "{}",
            ball.counted()
        );
    }
}
