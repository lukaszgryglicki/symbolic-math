//! Walks along the digits of numbers and along the primes, and spirals of the primes.

use crate::expr::Expr;
use crate::plot::label;
use crate::radix::{self, Leading, MOST};
use crate::scene::{Drawing, Item, Paint, Point, Space, moved, turn};
use rug::Float;

/// The most primes of one drawing.
pub const PRIMES: u64 = 5_000_000;

/// The drawings of this module.
pub fn is_walk(name: &str) -> bool {
    matches!(
        name,
        "walk"
            | "walk3d"
            | "primewalk"
            | "primewalk3d"
            | "primemodwalk"
            | "primemodwalk3d"
            | "ulamspiral"
            | "sacksspiral"
    )
}

/// A whole number among the arguments, or the one that stands for a missing argument.
fn whole(e: Option<&Expr>, usual: u64, range: (u64, u64), what: &str) -> Result<u64, String> {
    match e {
        None => Ok(usual),
        Some(e) => radix::plain(e.clone())
            .as_i64()
            .and_then(|n| u64::try_from(n).ok())
            .filter(|n| (range.0..=range.1).contains(n))
            .ok_or_else(|| {
                format!(
                    "{what} is a whole number from {} to {}, not {e}",
                    range.0, range.1
                )
            }),
    }
}

/// The directions of the steps of a walk: one for every kind of step that occurs, and the
/// place of its kind for each step.
fn directions<T: Copy + Ord>(
    kinds: &[T],
    mut direction: impl FnMut(T) -> Point,
) -> (Vec<Point>, Vec<u32>) {
    let mut distinct = kinds.to_vec();
    distinct.sort_unstable();
    distinct.dedup();
    let dirs = distinct.iter().map(|k| direction(*k)).collect();
    let steps = kinds
        .iter()
        .map(|k| distinct.binary_search(k).unwrap_or(0) as u32)
        .collect();
    (dirs, steps)
}

/// The direction with the angle `a` of `m` parts of a turn around the upright axis, lifted
/// by `c` of `k` parts of a turn above the plane.
fn lifted(a: (u64, u64), c: (u64, u64)) -> Point {
    let (sin_a, cos_a) = turn(a);
    let (sin_c, cos_c) = turn(c);
    [cos_c * cos_a, cos_c * sin_a, sin_c]
}

/// Where a walk ends and how far it reaches.
fn reach(dirs: &[Point], steps: &[u32], runs: &[u32], solid: bool) -> String {
    let mut at = [0.0; 3];
    let (mut low, mut high) = (at, at);
    for (k, step) in steps.iter().enumerate() {
        at = moved(at, dirs[*step as usize], runs.get(k).copied());
        for axis in 0..3 {
            low[axis] = low[axis].min(at[axis]);
            high[axis] = high[axis].max(at[axis]);
        }
    }
    let axes = if solid { 3 } else { 2 };
    // (a sum of many steps that should be zero is a tiny number)
    let tidy = |v: f64| label(if v.abs() < 1e-9 { 0.0 } else { v });
    let point: Vec<String> = at[..axes].iter().map(|v| tidy(*v)).collect();
    let away = (at[0] * at[0] + at[1] * at[1] + at[2] * at[2]).sqrt();
    let ranges: Vec<String> = (0..axes)
        .map(|k| {
            format!(
                "{} from {} to {}",
                ["x", "y", "z"][k],
                tidy(low[k]),
                tidy(high[k])
            )
        })
        .collect();
    format!(
        "from the origin to ({}), {} away; {}",
        point.join(", "),
        tidy(away),
        ranges.join(", ")
    )
}

/// The first digits as they are written, with the point.
fn written(lead: &Leading, base: u32) -> String {
    let most = if base > radix::LETTERED { 8 } else { 24 };
    let shown = lead.digits.len().min(most);
    let mut text = radix::digits_text(&lead.digits[..shown.min(lead.whole)], base);
    if shown > lead.whole {
        text.push('.');
        text.push_str(&radix::digits_text(&lead.digits[lead.whole..shown], base));
    }
    if shown < lead.digits.len() || !lead.ended {
        text.push_str("...");
    }
    text
}

/// So many things, in words: "1 step", "2 steps".
fn counted(n: usize, thing: &str) -> String {
    format!("{n} {thing}{}", if n == 1 { "" } else { "s" })
}

/// The angle of one part of `parts` of a turn, in degrees.
fn degrees(parts: u64) -> String {
    label(360.0 / parts as f64)
}

fn trail(dirs: Vec<Point>, steps: Vec<u32>, runs: Vec<u32>, solid: bool, told: String) -> Drawing {
    let ends = reach(&dirs, &steps, &runs, solid);
    let space = if solid { Space::Solid } else { Space::Flat };
    let mut drawing = Drawing::new(space, vec![Item::Trail { dirs, steps, runs }]);
    drawing.facts = vec![told, ends];
    drawing
}

/// `walk(x, base, digits)`: a step for every digit, in the direction that the digit is of
/// the base's directions of the plane.
fn walk(a: &[Expr], notes: &mut Vec<String>) -> Result<Drawing, String> {
    let usage = "walk(x, base, digits): walk(pi, 10, 1000); the base is 10 and the digits are 1000 when they are left out";
    let x = a.first().ok_or(usage)?;
    let base = radix::base_of(a.get(1))?;
    let count = whole(a.get(2), 1000, (1, MOST as u64), "the number of digits")? as usize;
    let lead = radix::leading(x, base, count, notes).map_err(|why| {
        match why.ends_with("is no real number") {
            true => format!("{why}: walk3d(z, base, digits) walks along a complex number"),
            false => why,
        }
    })?;
    let (dirs, steps) = directions(&lead.digits, |d| {
        let (sin, cos) = turn((u64::from(d), u64::from(base)));
        [cos, sin, 0.0]
    });
    let negative = crate::numeric::evalf(x, 64).is_ok_and(|v| v.real().is_sign_negative());
    let told = format!(
        "{} along the digits {} of {x}{} in base {base}{}: the digit d is a step at {}*d degrees",
        counted(steps.len(), "step"),
        written(&lead, base),
        if negative { " without its sign" } else { "" },
        if lead.ended && lead.digits.len() < count {
            ", which end there"
        } else {
            ""
        },
        degrees(u64::from(base)),
    );
    Ok(trail(dirs, steps, Vec::new(), false, told))
}

/// `walk3d(z, base, digits)`: the digits of the real part turn a step around the upright
/// axis, the digits of the imaginary part lift it above the plane.
fn walk3d(a: &[Expr], notes: &mut Vec<String>) -> Result<Drawing, String> {
    let usage = "walk3d(z, base, digits): walk3d(e + pi*i, 12, 10000); the base is 10 and the digits are 1000 when they are left out";
    let z = a.first().ok_or(usage)?;
    let base = radix::base_of(a.get(1))?;
    let count = whole(a.get(2), 1000, (1, MOST as u64), "the number of digits")? as usize;
    let [re, im] = radix::leading_pair(z, base, count, notes)?;
    // (digits that ended go on with zeros)
    let length = re.digits.len().max(im.digits.len());
    let digit = |lead: &Leading, k: usize| lead.digits.get(k).copied().unwrap_or(0);
    let pairs: Vec<(u32, u32)> = (0..length)
        .map(|k| (digit(&re, k), digit(&im, k)))
        .collect();
    let parts = u64::from(base);
    let (dirs, steps) = directions(&pairs, |(a, c)| {
        lifted((u64::from(a), parts), (u64::from(c), parts))
    });
    let ended = re.ended && im.ended && length < count;
    let angle = degrees(parts);
    let told = format!(
        "{} along the digits {} of the real part and {} of the imaginary part of {z} in base {base}{}: the digits a and c of a place are a step at {angle}*a degrees around the z-axis, {angle}*c degrees above the plane",
        counted(steps.len(), "step"),
        written(&re, base),
        written(&im, base),
        if ended { ", which end there" } else { "" },
    );
    Ok(trail(dirs, steps, Vec::new(), true, told))
}

/// The first `n` primes.
fn primes(n: u64) -> Result<Vec<u32>, String> {
    let last = crate::ntheory::nth_prime(n)
        .and_then(|p| p.to_u64())
        .filter(|p| *p <= crate::ntheory::PRIMES_UP_TO_MAX)
        .ok_or("so many primes are not at hand")?;
    Ok(crate::ntheory::primes_up_to(last)
        .into_iter()
        .map(|p| p as u32)
        .collect())
}

fn count_of(a: &[Expr], least: u64, usage: &str) -> Result<u64, String> {
    a.first().ok_or(usage)?;
    whole(a.first(), 0, (least, PRIMES), "the number of primes")
}

/// `primewalk(n, m, k)`: a step for every whole number from 2 to the n-th prime, and at
/// every prime a turn by `k` of `m` parts of a full turn.
fn primewalk(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "primewalk(n, m, k): a step for every whole number up to the n-th prime, at every prime a turn by k/m of a full turn; m is 4 and k is 1 when they are left out";
    let n = count_of(a, 2, usage)?;
    let m = whole(a.get(1), 4, (2, 1_000_000), "the number of directions")?;
    let k = whole(a.get(2), 1, (1, m - 1), "the turn, in directions,")?;
    let p = primes(n)?;
    let dirs = (0..m)
        .map(|d| {
            let (sin, cos) = turn((d, m));
            [cos, sin, 0.0]
        })
        .collect();
    let steps = (0..p.len() as u64 - 1)
        .map(|side| (side * k % m) as u32)
        .collect();
    let runs = p.windows(2).map(|w| w[1] - w[0]).collect();
    let last = p[p.len() - 1];
    let told = format!(
        "{} along the first {n} primes, from 2 to {last}: a step for every whole number, at every prime a turn to the left by {} degrees, so that every side is as long as the gap between two primes",
        counted(p.len() - 1, "side"),
        label(360.0 * k as f64 / m as f64),
    );
    Ok(trail(dirs, steps, runs, false, told))
}

/// `primewalk3d(n, m)`: as `primewalk`, with the directions of space in their turn.
fn primewalk3d(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "primewalk3d(n, m): a step for every whole number up to the n-th prime, at every prime the next of m directions of space: 3 (x, y, z), 4 (the corners of a tetrahedron) or 6 (x, y, z, -x, -y, -z); m is 6 when it is left out";
    let n = count_of(a, 2, usage)?;
    let m = whole(a.get(1), 6, (3, 6), "the number of directions")?;
    let third = 1.0 / 3f64.sqrt();
    let (dirs, named): (Vec<Point>, &str) = match m {
        3 => (
            vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            "the directions x, y, z",
        ),
        4 => (
            [
                [1.0, 1.0, 1.0],
                [1.0, -1.0, -1.0],
                [-1.0, 1.0, -1.0],
                [-1.0, -1.0, 1.0],
            ]
            .map(|d| d.map(|v| v * third))
            .to_vec(),
            "the directions to the four corners of a tetrahedron",
        ),
        6 => (
            vec![
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [-1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 0.0, -1.0],
            ],
            "the directions x, y, z, -x, -y, -z",
        ),
        _ => return Err(usage.to_string()),
    };
    let p = primes(n)?;
    let steps = (0..p.len() as u64 - 1)
        .map(|side| (side % m) as u32)
        .collect();
    let runs = p.windows(2).map(|w| w[1] - w[0]).collect();
    let last = p[p.len() - 1];
    let told = format!(
        "{} along the first {n} primes, from 2 to {last}: a step for every whole number, at every prime the next of {named}, so that every side is as long as the gap between two primes",
        counted(p.len() - 1, "side"),
    );
    Ok(trail(dirs, steps, runs, true, told))
}

/// `primemodwalk(n, m)`: a step for every prime, in the direction that its remainder is of
/// the `m` directions of the plane.
fn primemodwalk(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "primemodwalk(n, m): a step for each of the first n primes p, at 360*(p mod m)/m degrees; m is 8 when it is left out";
    let n = count_of(a, 1, usage)?;
    let m = whole(a.get(1), 8, (2, u64::from(u32::MAX)), "the modulus")?;
    let p = primes(n)?;
    let rest: Vec<u32> = p.iter().map(|p| (u64::from(*p) % m) as u32).collect();
    let (dirs, steps) = directions(&rest, |r| {
        let (sin, cos) = turn((u64::from(r), m));
        [cos, sin, 0.0]
    });
    let told = format!(
        "{}, one for every prime p up to {}: at {}*(p mod {m}) degrees",
        counted(p.len(), "step"),
        p[p.len() - 1],
        degrees(m),
    );
    Ok(trail(dirs, steps, Vec::new(), false, told))
}

/// `primemodwalk3d(n, m, k)`: the remainder of a prime by `m` turns its step around the
/// upright axis, the remainder by `k` lifts it above the plane.
fn primemodwalk3d(a: &[Expr]) -> Result<Drawing, String> {
    let usage = "primemodwalk3d(n, m, k): a step for each of the first n primes p, at 360*(p mod m)/m degrees around the z-axis and 360*(p mod k)/k degrees above the plane; m is 8 and k is 12 when they are left out";
    let n = count_of(a, 1, usage)?;
    let m = whole(a.get(1), 8, (2, u64::from(u32::MAX)), "the modulus")?;
    let k = whole(a.get(2), 12, (2, u64::from(u32::MAX)), "the second modulus")?;
    let p = primes(n)?;
    let rest: Vec<(u32, u32)> = p
        .iter()
        .map(|p| ((u64::from(*p) % m) as u32, (u64::from(*p) % k) as u32))
        .collect();
    let (dirs, steps) = directions(&rest, |(r, s)| lifted((u64::from(r), m), (u64::from(s), k)));
    let told = format!(
        "{}, one for every prime p up to {}: at {}*(p mod {m}) degrees around the z-axis, {}*(p mod {k}) degrees above the plane",
        counted(p.len(), "step"),
        p[p.len() - 1],
        degrees(m),
        degrees(k),
    );
    Ok(trail(dirs, steps, Vec::new(), true, told))
}

/// The place of a whole number on the square spiral: 1 in the middle, 2 to its right, then
/// around against the clock.
pub fn ulam(k: u64) -> [i32; 2] {
    if k <= 1 {
        return [0, 0];
    }
    // the ring of the number, and how far along the ring it is
    let ring = (k - 1).isqrt().div_ceil(2) as i64;
    let side = 2 * ring;
    let along = k as i64 - (side - 1) * (side - 1);
    let (x, y) = match (along - 1) / side {
        0 => (ring, along - ring),
        1 => (ring - (along - side), ring),
        2 => (-ring, ring - (along - 2 * side)),
        _ => (along - 3 * side - ring, -ring),
    };
    [x as i32, y as i32]
}

/// `ulamspiral(n)`: the first n primes on the square spiral of the whole numbers.
fn ulamspiral(a: &[Expr]) -> Result<Drawing, String> {
    let n = count_of(
        a,
        1,
        "ulamspiral(n): the first n primes on the square spiral of the whole numbers",
    )?;
    let p = primes(n)?;
    let at = p.iter().map(|p| ulam(u64::from(*p))).collect();
    let mut drawing = Drawing::new(
        Space::Flat,
        vec![Item::Squares {
            at,
            paint: Paint::Along,
        }],
    );
    drawing.faint = true;
    drawing.facts = vec![format!(
        "the first {} up to {} on the square spiral of the whole numbers: 1 in the middle, 2 to its right, then around against the clock",
        counted(p.len(), "prime"),
        p[p.len() - 1]
    )];
    Ok(drawing)
}

/// The place of a whole number on the spiral of Sacks: as far from the middle as its square
/// root, and as many turns around it.
pub fn sacks(k: u64) -> [f64; 2] {
    let root = Float::with_val(96, k).sqrt();
    let part = root.clone().fract() * Float::with_val(96, rug::float::Constant::Pi) * 2u32;
    let far = (k as f64).sqrt();
    // the squares lie on the axis
    let whole = k.isqrt();
    if whole * whole == k {
        return [far, 0.0];
    }
    [far * part.clone().cos().to_f64(), far * part.sin().to_f64()]
}

/// `sacksspiral(n)`: the first n primes on the spiral of Sacks.
fn sacksspiral(a: &[Expr]) -> Result<Drawing, String> {
    let n = count_of(
        a,
        1,
        "sacksspiral(n): the first n primes on the spiral of Sacks",
    )?;
    let p = primes(n)?;
    let points = p.iter().map(|p| sacks(u64::from(*p))).collect();
    let mut drawing = Drawing::new(
        Space::Flat,
        vec![Item::Dots {
            points,
            paint: Paint::Along,
            radius: 0.5,
            units: true,
        }],
    );
    drawing.faint = true;
    drawing.facts = vec![format!(
        "the first {} up to {} on the spiral of Sacks: the whole number k is sqrt(k) away from the middle after sqrt(k) turns, so that the squares lie on the x-axis",
        counted(p.len(), "prime"),
        p[p.len() - 1]
    )];
    Ok(drawing)
}

/// The drawing of a command of this module.
pub fn build(name: &str, a: &[Expr], notes: &mut Vec<String>) -> Result<Drawing, String> {
    let most = match name {
        "primewalk" | "primemodwalk3d" => 3,
        "walk" | "walk3d" => 3,
        "primewalk3d" | "primemodwalk" => 2,
        _ => 1,
    };
    if a.len() > most {
        return Err(format!(
            "{name} takes {most} argument{} at most; what else belongs to a picture is written as name = value",
            if most == 1 { "" } else { "s" }
        ));
    }
    match name {
        "walk" => walk(a, notes),
        "walk3d" => walk3d(a, notes),
        "primewalk" => primewalk(a),
        "primewalk3d" => primewalk3d(a),
        "primemodwalk" => primemodwalk(a),
        "primemodwalk3d" => primemodwalk3d(a),
        "ulamspiral" => ulamspiral(a),
        _ => sacksspiral(a),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(text: &str) -> Expr {
        let parsed = crate::parser::parse(text).expect("an expression");
        crate::Session::new()
            .eval(&parsed.stmts[0])
            .expect("a value")
    }

    fn drawn(name: &str, args: &[&str]) -> Drawing {
        let a: Vec<Expr> = args.iter().map(|t| number(t)).collect();
        build(name, &a, &mut Vec::new()).expect("a drawing")
    }

    /// The points of the walk of a drawing.
    fn path(d: &Drawing) -> Vec<Point> {
        let mut points = Vec::new();
        d.points(&mut |p| points.push(p));
        points
    }

    fn near(p: Point, q: [f64; 3]) -> bool {
        (0..3).all(|k| (p[k] - q[k]).abs() < 1e-12)
    }

    #[test]
    fn a_step_for_every_digit() {
        // 0.25 in base 4 is 0.1: the digits 0 and 1 are a step to the right and one up
        let d = drawn("walk", &["1/4", "4", "10"]);
        let p = path(&d);
        assert_eq!(p.len(), 3);
        assert!(near(p[1], [1.0, 0.0, 0.0]) && near(p[2], [1.0, 1.0, 0.0]));
        assert_eq!(d.space, Space::Flat);
        assert_eq!(
            d.facts,
            [
                "2 steps along the digits 0.1 of 1/4 in base 4, which end there: the digit d is a step at 90*d degrees",
                "from the origin to (1, 1), 1.414 away; x from 0 to 1, y from 0 to 1",
            ]
        );
        // the digits 3, 1, 4, 1 of pi in base 10, by their angles
        let d = drawn("walk", &["pi", "10", "4"]);
        let p = path(&d);
        let mut at = [0.0f64; 2];
        for (k, digit) in [3u32, 1, 4, 1].into_iter().enumerate() {
            let angle = f64::from(digit) * std::f64::consts::PI / 5.0;
            at = [at[0] + angle.cos(), at[1] + angle.sin()];
            assert!((p[k + 1][0] - at[0]).abs() < 1e-12 && (p[k + 1][1] - at[1]).abs() < 1e-12);
        }
        assert!(d.facts[0].starts_with("4 steps along the digits 3.141... of pi in base 10: the digit d is a step at 36*d degrees"));
        // a number without a count has 1000 digits, a whole number its own
        assert_eq!(path(&drawn("walk", &["sqrt(2)"])).len(), 1001);
        assert_eq!(path(&drawn("walk", &["2^64", "2"])).len(), 66);
        // in base 2 the walk stays on a line
        let d = drawn("walk", &["pi", "2", "300"]);
        assert!(path(&d).iter().all(|p| p[1] == 0.0 && p[2] == 0.0));
        // the block of digits that repeats moves the walk on and on by the same way
        let p = path(&drawn("walk", &["1/7", "10", "121"]));
        let round = [p[7][0] - p[1][0], p[7][1] - p[1][1]];
        for k in 1..19 {
            let moved = [
                p[1 + 6 * (k + 1)][0] - p[1 + 6 * k][0],
                p[1 + 6 * (k + 1)][1] - p[1 + 6 * k][1],
            ];
            assert!((moved[0] - round[0]).abs() < 1e-9 && (moved[1] - round[1]).abs() < 1e-9);
        }
        // a large base: only the digits that occur have a direction
        let d = drawn("walk", &["pi", "4000000000", "50"]);
        assert_eq!(path(&d).len(), 51);
        for (args, why) in [
            (
                vec!["1 + i"],
                "1 + i is no real number: walk3d(z, base, digits) walks along a complex number",
            ),
            (vec!["x"], "x is no number"),
            (
                vec!["pi", "1"],
                "the base is a whole number from 2 to 4294967295, not 1",
            ),
            (
                vec!["pi", "10", "0"],
                "the number of digits is a whole number from 1 to 10000000, not 0",
            ),
            (
                vec!["pi", "10", "10", "10"],
                "walk takes 3 arguments at most; what else belongs to a picture is written as name = value",
            ),
        ] {
            let a: Vec<Expr> = args.iter().map(|t| number(t)).collect();
            assert_eq!(
                build("walk", &a, &mut Vec::new()).err(),
                Some(why.to_string())
            );
        }
    }

    #[test]
    fn two_angles_for_a_complex_number() {
        // a real number walks in the plane as walk does
        let flat = path(&drawn("walk", &["pi", "12", "200"]));
        let solid = path(&drawn("walk3d", &["pi", "12", "200"]));
        assert_eq!(flat, solid);
        // an imaginary number walks in the plane of x and z
        let p = path(&drawn("walk3d", &["i/4", "4", "10"]));
        assert_eq!(p.len(), 3);
        assert!(near(p[1], [1.0, 0.0, 0.0]) && near(p[2], [1.0, 0.0, 1.0]));
        // the digits of both parts, place by place: 12.5 and 0.25 in base 10
        let d = drawn("walk3d", &["12.5 + 0.25i", "10", "10"]);
        let p = path(&d);
        assert_eq!(p.len(), 5);
        let mut at = [0.0f64; 3];
        for (k, (a, c)) in [(1u32, 0u32), (2, 0), (5, 2), (0, 5)]
            .into_iter()
            .enumerate()
        {
            let (t, f) = (
                f64::from(a) * std::f64::consts::PI / 5.0,
                f64::from(c) * std::f64::consts::PI / 5.0,
            );
            at = [
                at[0] + f.cos() * t.cos(),
                at[1] + f.cos() * t.sin(),
                at[2] + f.sin(),
            ];
            assert!(near(p[k + 1], at), "step {k}");
        }
        assert_eq!(d.space, Space::Solid);
        assert!(d.facts[0].starts_with("4 steps along the digits 12.5 of the real part and 00.25 of the imaginary part of 25/2 + i/4 in base 10, which end there: the digits a and c of a place are a step at 36*a degrees around the z-axis, 36*c degrees above the plane"), "{}", d.facts[0]);
        // every step has the length 1
        let p = path(&drawn("walk3d", &["e + pi*i", "12", "500"]));
        for pair in p.windows(2) {
            let d: f64 = (0..3)
                .map(|k| (pair[1][k] - pair[0][k]) * (pair[1][k] - pair[0][k]))
                .sum();
            assert!((d - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn walks_along_the_primes() {
        // 2 3 5 7 11 13: sides 1 2 2 4 2, a quarter turn to the left at every prime
        let d = drawn("primewalk", &["6"]);
        let p = path(&d);
        let want = [
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 2.0],
            [-1.0, 2.0],
            [-1.0, -2.0],
            [1.0, -2.0],
        ];
        assert_eq!(p.len(), want.len());
        for (p, w) in p.iter().zip(want) {
            assert!(near(*p, [w[0], w[1], 0.0]), "{p:?} against {w:?}");
        }
        assert_eq!(
            d.facts[0],
            "5 sides along the first 6 primes, from 2 to 13: a step for every whole number, at every prime a turn to the left by 90 degrees, so that every side is as long as the gap between two primes"
        );
        assert_eq!(
            d.facts[1],
            "from the origin to (1, -2), 2.236 away; x from -1 to 1, y from -2 to 2"
        );
        // three directions, two of them at a time
        let p = path(&drawn("primewalk", &["4", "3", "2"]));
        assert!(near(p[1], [1.0, 0.0, 0.0]));
        assert!((p[2][0] - 0.0).abs() < 1e-12 && (p[2][1] + 3f64.sqrt()).abs() < 1e-12);
        // in space: x, y, z, -x, -y, -z in their turn
        let p = path(&drawn("primewalk3d", &["8"]));
        let want = [
            [0, 0, 0],
            [1, 0, 0],
            [1, 2, 0],
            [1, 2, 2],
            [-3, 2, 2],
            [-3, 0, 2],
            [-3, 0, -2],
            [-1, 0, -2],
        ];
        assert_eq!(p.len(), want.len());
        for (p, w) in p.iter().zip(want) {
            assert!(near(*p, w.map(f64::from)));
        }
        let p = path(&drawn("primewalk3d", &["5", "3"]));
        assert!(near(p[4], [1.0 + 4.0, 2.0, 2.0]));
        let p = path(&drawn("primewalk3d", &["5", "4"]));
        let third = 1.0 / 3f64.sqrt();
        assert!(near(p[1], [third, third, third]));
        // a step for every prime by its remainder: 2 3 5 7 by 4 are 2 3 1 3
        let p = path(&drawn("primemodwalk", &["4", "4"]));
        let want = [
            [0.0, 0.0],
            [-1.0, 0.0],
            [-1.0, -1.0],
            [-1.0, 0.0],
            [-1.0, -1.0],
        ];
        for (p, w) in p.iter().zip(want) {
            assert!(near(*p, [w[0], w[1], 0.0]));
        }
        // by 8 the four directions of the odd remainders even out: no way that grows with n
        let p = path(&drawn("primemodwalk", &["20000"]));
        let end = p[p.len() - 1];
        assert!((end[0] * end[0] + end[1] * end[1]).sqrt() < 400.0);
        // two remainders, two angles
        let d = drawn("primemodwalk3d", &["3", "4", "4"]);
        let p = path(&d);
        // 2: half a turn around and half a turn above: back to +x
        assert!(near(p[1], [1.0, 0.0, 0.0]));
        // 3: three quarters around, three quarters above: straight down
        assert!(near(p[2], [1.0, 0.0, -1.0]));
        // 5: a quarter around, a quarter above: straight up
        assert!(near(p[3], [1.0, 0.0, 0.0]));
        assert_eq!(d.space, Space::Solid);
        for (name, args, why) in [
            (
                "primewalk",
                vec!["1"],
                "the number of primes is a whole number from 2 to 5000000, not 1",
            ),
            (
                "primewalk",
                vec!["10", "1"],
                "the number of directions is a whole number from 2 to 1000000, not 1",
            ),
            (
                "primewalk",
                vec!["10", "4", "4"],
                "the turn, in directions, is a whole number from 1 to 3, not 4",
            ),
            (
                "primewalk3d",
                vec!["10", "7"],
                "the number of directions is a whole number from 3 to 6, not 7",
            ),
            (
                "primemodwalk",
                vec!["0"],
                "the number of primes is a whole number from 1 to 5000000, not 0",
            ),
            (
                "primemodwalk",
                vec!["10", "1"],
                "the modulus is a whole number from 2 to 4294967295, not 1",
            ),
            (
                "ulamspiral",
                vec!["x"],
                "the number of primes is a whole number from 1 to 5000000, not x",
            ),
            (
                "sacksspiral",
                vec!["10", "2"],
                "sacksspiral takes 1 argument at most; what else belongs to a picture is written as name = value",
            ),
        ] {
            let a: Vec<Expr> = args.iter().map(|t| number(t)).collect();
            assert_eq!(
                build(name, &a, &mut Vec::new()).err(),
                Some(why.to_string()),
                "{name}"
            );
        }
        assert!(build("primewalk3d", &[number("10"), number("5")], &mut Vec::new()).is_err());
    }

    #[test]
    fn spirals_of_the_primes() {
        // the square spiral: every whole number beside the one before it, none twice
        let want = [
            [0, 0],
            [1, 0],
            [1, 1],
            [0, 1],
            [-1, 1],
            [-1, 0],
            [-1, -1],
            [0, -1],
            [1, -1],
            [2, -1],
            [2, 0],
        ];
        for (k, w) in want.iter().enumerate() {
            assert_eq!(ulam(k as u64 + 1), *w, "{}", k + 1);
        }
        let mut seen = std::collections::HashSet::new();
        let mut last = ulam(1);
        assert!(seen.insert(last));
        for k in 2..=20_000u64 {
            let at = ulam(k);
            assert_eq!((at[0] - last[0]).abs() + (at[1] - last[1]).abs(), 1, "{k}");
            assert!(seen.insert(at), "{k}");
            last = at;
        }
        // the odd squares on one diagonal, the even ones on the other
        for r in 1..50i32 {
            let odd = (2 * r as u64 + 1) * (2 * r as u64 + 1);
            assert_eq!(ulam(odd), [r, -r]);
            assert_eq!(ulam(4 * (r as u64) * (r as u64)), [1 - r, r]);
        }
        let d = drawn("ulamspiral", &["5"]);
        match &d.items[0] {
            Item::Squares { at, .. } => {
                assert_eq!(at, &[[1, 0], [1, 1], [-1, 1], [-1, -1], [2, 0]])
            }
            _ => panic!("squares"),
        }
        // the spiral of Sacks: the squares on the axis, a turn from square to square
        for k in 1..40u64 {
            assert_eq!(sacks(k * k), [k as f64, 0.0]);
        }
        for k in [2u64, 3, 7, 1000, 86_028_121] {
            let [x, y] = sacks(k);
            let root = (k as f64).sqrt();
            assert!(((x * x + y * y).sqrt() - root).abs() < 1e-9 * root);
            let angle = 2.0 * std::f64::consts::PI * (root - root.floor());
            assert!(
                (x - root * angle.cos()).abs() < 1e-6 * root
                    && (y - root * angle.sin()).abs() < 1e-6 * root,
                "{k}"
            );
        }
        let d = drawn("sacksspiral", &["100"]);
        match &d.items[0] {
            Item::Dots { points, units, .. } => {
                assert_eq!(points.len(), 100);
                assert!(*units);
            }
            _ => panic!("dots"),
        }
        assert!(d.facts[0].starts_with("the first 100 primes up to 541 on the spiral of Sacks"));
        assert!(drawn("ulamspiral", &["1"]).facts[0].starts_with("the first 1 prime up to 2 on"));
        assert!(
            drawn("walk", &["0"]).facts[0]
                .starts_with("1 step along the digits 0 of 0 in base 10, which end there")
        );
        assert!(drawn("walk", &["-5/4", "10", "9"]).facts[0].starts_with(
            "3 steps along the digits 1.25 of -5/4 without its sign in base 10, which end there"
        ));
    }
}
