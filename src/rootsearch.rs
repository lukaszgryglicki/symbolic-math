//! The roots of an equation in one unknown that lie nearest to 0, as many as `roots = N` asks
//! for.
//!
//! The absolute value of a function that has a derivative has no least value but at a root, so
//! the function is sized up on a lattice of points around 0 and the search for a root starts
//! from each point that is smaller than its neighbours. The size may be taken less a part
//! that the caller knows of (`Sized`): a sum of exponentials rises so steeply to one side
//! that its bare size hides the roots there. A lattice of half the step must find the same nearest
//! roots inside the square before the square counts as searched; then the square grows until
//! it holds as many roots as were asked for. With real numbers only, the lattice is on the
//! real line, and a change of sign between two points is followed by halving.

use crate::numeric::bits_for;
use crate::numroots::find_root;
use crate::trace;
use rug::{Complex, Float};
use std::time::{Duration, Instant};

pub struct Search {
    /// The roots found, without the known ones, the nearest to 0 first.
    pub roots: Vec<Complex>,
    /// The half side of the square (the half length of the interval) that was searched.
    pub radius: f64,
    /// The search ended at its limit of time or of points, not because it was done.
    pub stopped: bool,
}

/// Points of a lattice.
type Points = Vec<(f64, f64)>;

/// Beyond this nothing is looked for.
const FAR: f64 = 524_288.0;

/// The most real roots that are listed for an interval (`between`), and that a search which
/// is to find every real root goes on for.
pub const MANY: usize = 256;

/// How far a search may go: a count of points of its lattices (the same on every machine,
/// for a search that nobody asked a count of roots of), or a time as well.
struct Budget {
    deadline: Option<Instant>,
    points: u64,
    most: u64,
}

impl Budget {
    /// One more point of a lattice; false when there are no more.
    fn point(&mut self) -> bool {
        self.points += 1;
        self.points <= self.most && (!self.points.is_multiple_of(256) || !self.late())
    }

    fn late(&self) -> bool {
        self.deadline.is_some_and(|end| Instant::now() >= end)
    }
}

/// The function at a point of the lattice: its value, and what is taken from the logarithm
/// of its size there (0 for the bare size; Re(m z) for a function that is looked at times
/// e^(-m z)).
pub type Sized<'a> = &'a mut dyn FnMut(&Complex) -> Option<(Complex, f64)>;

/// ln|v| less `scale`: -inf at a root, +inf where there is no value.
fn size(v: Option<&(Complex, f64)>) -> f64 {
    let Some((v, scale)) = v else {
        return f64::INFINITY;
    };
    let norm = Float::with_val(64, v.norm_ref());
    if norm.is_zero() {
        f64::NEG_INFINITY
    } else if !norm.is_finite() {
        f64::INFINITY
    } else {
        let size = norm.ln().to_f64() / 2.0 - scale;
        if size.is_nan() { f64::INFINITY } else { size }
    }
}

/// ln|z| as a double; -inf for 0.
pub fn ln_size(z: &Complex) -> f64 {
    let norm = Float::with_val(64, z.norm_ref());
    if norm.is_zero() {
        f64::NEG_INFINITY
    } else {
        norm.ln().to_f64() / 2.0
    }
}

fn modulus(z: &Complex) -> f64 {
    Float::with_val(64, z.abs_ref()).to_f64()
}

/// Inside the circle that the searched square holds.
fn inside(z: &Complex, radius: f64) -> bool {
    modulus(z) <= radius
}

/// Two numbers that are one root: as near to each other as two findings of a root are, next
/// to their own size (a root next to 0 is told apart from 0 and from another one there). Two
/// findings agree to nearly all their bits, and two roots that differ in the first half of
/// them are two: those of cos(x)^2 = x/10^24 lie in pairs that are 10^-12 apart.
pub fn same(a: &Complex, b: &Complex) -> bool {
    let (most, least) = (a.prec().0.max(b.prec().0), a.prec().0.min(b.prec().0));
    let gap = Float::with_val(64, Complex::with_val(most.max(64), a - b).abs_ref());
    let scale = Float::with_val(64, a.abs_ref()).max(&Float::with_val(64, b.abs_ref()));
    gap.is_zero() || gap < Float::with_val(64, &scale >> (least / 2).max(40))
}

/// A part that is rounding noise next to the other one is zero. (Next to the size of the
/// number itself: the root (1 + i) 10^-40 has both its parts.)
pub fn settled(z: Complex, digits: u32) -> Complex {
    let scale = Float::with_val(64, z.abs_ref());
    let noise = Float::with_val(64, &scale >> (bits_for(digits) - 8));
    if Float::with_val(64, z.imag().abs_ref()) < noise {
        Complex::with_val(z.prec(), z.real())
    } else if Float::with_val(64, z.real().abs_ref()) < noise {
        Complex::with_val(z.prec(), (0, z.imag()))
    } else {
        z
    }
}

/// The function is as large at conjugate points: half the plane tells all of it.
fn mirrored(f: Sized) -> bool {
    [(0.731, 0.417), (-1.913, 2.289), (3.377, -0.871)]
        .iter()
        .all(|(x, y)| {
            let above = size(f(&Complex::with_val(64, (*x, *y))).as_ref());
            let below = size(f(&Complex::with_val(64, (*x, -*y))).as_ref());
            above.is_finite()
                && below.is_finite()
                && (above - below).abs() <= 1e-9 * (1.0 + above.abs())
        })
}

/// The points of a lattice of the square where the function is smaller than at the eight points
/// around: the nearest to 0 first; and the points of the lattice where the function is
/// exactly 0. `None` when the budget ran out.
fn square(
    f: Sized,
    radius: f64,
    n: i64,
    mirror: bool,
    most: usize,
    budget: &mut Budget,
) -> Option<(Points, Points)> {
    let h = radius / n as f64;
    let side = (2 * n + 1) as usize;
    let at = |i: i64, j: i64| (j + n) as usize * side + (i + n) as usize;
    let mut sizes = vec![f64::INFINITY; side * side];
    for j in (if mirror { 0 } else { -n })..=n {
        for i in -n..=n {
            if !budget.point() {
                return None;
            }
            let z = Complex::with_val(64, (i as f64 * h, j as f64 * h));
            sizes[at(i, j)] = size(f(&z).as_ref());
        }
    }
    if mirror {
        for j in -n..0 {
            for i in -n..=n {
                sizes[at(i, j)] = sizes[at(i, -j)];
            }
        }
    }
    let mut starts = Vec::new();
    let mut zeros = Vec::new();
    // (a least value on the border belongs to a root outside: the next square has it inside)
    for j in 1 - n..n {
        for i in 1 - n..n {
            let here = sizes[at(i, j)];
            if here == f64::INFINITY {
                continue;
            }
            if here == f64::NEG_INFINITY {
                zeros.push((i as f64 * h, j as f64 * h));
            }
            let (mut least, mut below) = (true, here == f64::NEG_INFINITY);
            for (di, dj) in [
                (-1, -1),
                (0, -1),
                (1, -1),
                (-1, 0),
                (1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
            ] {
                let there = sizes[at(i + di, j + dj)];
                least &= there >= here;
                below |= there > here;
            }
            if least && below {
                starts.push((i as f64 * h, j as f64 * h));
            }
        }
    }
    let nearest_first = |a: &(f64, f64), b: &(f64, f64)| {
        (a.0 * a.0 + a.1 * a.1)
            .total_cmp(&(b.0 * b.0 + b.1 * b.1))
            .then(a.0.total_cmp(&b.0))
            .then(a.1.total_cmp(&b.1))
    };
    starts.sort_by(nearest_first);
    starts.truncate(most);
    zeros.sort_by(nearest_first);
    Some((starts, zeros))
}

/// The points of a lattice of the interval from `lo` to `hi` (2 `n` steps) from which a real
/// root is looked for: where the sign changes (followed by halving to the root, not to a
/// pole) and where the function is smaller than at both neighbours, the nearest to `origin`
/// first; and the points of the lattice where the function is exactly 0.
fn line(
    f: Sized,
    (lo, hi): (f64, f64),
    n: i64,
    most: usize,
    origin: f64,
    budget: &mut Budget,
) -> Option<(Points, Points)> {
    let h = (hi - lo) / (2 * n) as f64;
    let at = |k: usize| lo + k as f64 * h;
    let mut values = Vec::with_capacity((2 * n + 1) as usize);
    for k in 0..=(2 * n) as usize {
        if !budget.point() {
            return None;
        }
        values.push(f(&Complex::with_val(64, (at(k), 0.0))));
    }
    let sizes: Vec<f64> = values.iter().map(|v| size(v.as_ref())).collect();
    // the sign of a real value: 0 for none, for zero and for a value that is not real
    let sign =
        |v: &Option<(Complex, f64)>| -> i32 {
            match v {
                Some((v, _))
                    if v.imag().is_zero() && !v.real().is_zero() && v.real().is_finite() =>
                {
                    if v.real().is_sign_negative() { -1 } else { 1 }
                }
                _ => 0,
            }
        };
    let mut starts = Vec::new();
    for k in 0..values.len() - 1 {
        let (left, right) = (sign(&values[k]), sign(&values[k + 1]));
        if left * right != -1 {
            continue;
        }
        let mut lo = Float::with_val(64, at(k));
        let mut hi = Float::with_val(64, at(k + 1));
        let mut middle = None;
        for _ in 0..48 {
            let mid = Float::with_val(64, &lo + &hi) / 2u32;
            let value = f(&Complex::with_val(64, (&mid, 0)));
            match sign(&value) {
                0 => {
                    middle = Some((mid, value));
                    break;
                }
                s if s == left => lo = mid.clone(),
                _ => hi = mid.clone(),
            }
            middle = Some((mid, value));
        }
        if let Some((mid, value)) = middle
            && size(value.as_ref()) < sizes[k].min(sizes[k + 1])
        {
            starts.push((mid.to_f64(), 0.0));
        }
    }
    for k in 1..values.len() - 1 {
        let (left, here, right) = (sizes[k - 1], sizes[k], sizes[k + 1]);
        if here == f64::INFINITY {
            continue;
        }
        if left >= here
            && right >= here
            && (here == f64::NEG_INFINITY || left > here || right > here)
        {
            starts.push((at(k), 0.0));
        }
    }
    let nearest_first = |a: &(f64, f64), b: &(f64, f64)| {
        (a.0 - origin)
            .abs()
            .total_cmp(&(b.0 - origin).abs())
            .then(a.0.total_cmp(&b.0))
    };
    starts.sort_by(nearest_first);
    starts.dedup();
    starts.truncate(most);
    let mut zeros: Vec<(f64, f64)> = (1..values.len() - 1)
        .filter(|k| sizes[*k] == f64::NEG_INFINITY)
        .map(|k| (at(k), 0.0))
        .collect();
    zeros.sort_by(nearest_first);
    Some((starts, zeros))
}

/// An equation that holds on whole stretches of numbers, as abs(x)/x = 1 and floor(x) = 2 do:
/// the function is exactly 0 at many points of the lattice that lie next to each other, and
/// between them too. (floor(x) = x holds at the whole numbers only, though a wide lattice has
/// no other points.)
fn stretching(f: Sized, zeros: &Points, step: f64, budget: &mut Budget) -> bool {
    let next = |a: f64, b: f64| (a - b).abs() < 1.5 * step;
    let mut pairs: Vec<((f64, f64), (f64, f64))> = Vec::new();
    let mut by_rows = zeros.clone();
    by_rows.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.total_cmp(&b.0)));
    pairs.extend(
        by_rows
            .windows(2)
            .filter(|w| w[0].1 == w[1].1 && next(w[0].0, w[1].0))
            .map(|w| (w[0], w[1])),
    );
    let mut by_columns = zeros.clone();
    by_columns.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    pairs.extend(
        by_columns
            .windows(2)
            .filter(|w| w[0].0 == w[1].0 && next(w[0].1, w[1].1))
            .map(|w| (w[0], w[1])),
    );
    if pairs.len() < 32 {
        return false;
    }
    // eight of the pairs, from all over the list: a point between the two
    let (mut looked, mut held) = (0, 0);
    for k in 0..8 {
        let (a, b) = pairs[k * (pairs.len() - 1) / 7];
        if !budget.point() {
            return false;
        }
        let between = (a.0 + 0.381966 * (b.0 - a.0), a.1 + 0.381966 * (b.1 - a.1));
        looked += 1;
        if size(f(&Complex::with_val(64, between)).as_ref()) == f64::NEG_INFINITY {
            held += 1;
        }
    }
    // (half of them: the lower half of a square that was not looked at, but taken to be as
    // the upper one, may hold none)
    2 * held >= looked
}

/// What a search is asked for.
#[derive(Clone, Copy)]
pub struct Ask<'a> {
    /// How many roots, the nearest to 0.
    pub want: usize,
    pub digits: u32,
    /// Only real roots, on the real line.
    pub real: bool,
    /// Roots that are known already: they are not looked for again, and they count.
    pub known: &'a [Complex],
    /// Numbers that solve what is searched and will not be listed (the equation as it was
    /// given has no value there, or the unknown may not be such a number): they are not looked
    /// for again, and they do not count.
    pub others: &'a [Complex],
    /// Farther than this from 0 only the real line is searched.
    pub plane: f64,
    /// Somebody asked for this count, and the search may take its time (20 seconds and more
    /// for many roots); else it ends after a fixed count of points, the same on every machine.
    pub asked: bool,
    /// Which numbers the unknown may be: a root that is none of them is no root here (it is
    /// not listed and does not count). With real roots only, `Some(Less)` and `Some(Greater)`
    /// also bound the search to the negative or to the positive half of the line.
    pub fits: Fits<'a>,
}

/// The numbers that an unknown may be: the half of the real line that holds them all when
/// there is one, and whether a number is one of them.
#[derive(Clone, Copy)]
pub struct Fits<'a> {
    pub side: Option<std::cmp::Ordering>,
    pub test: &'a dyn Fn(&Complex) -> bool,
}

impl Fits<'_> {
    /// Every number.
    pub const ALL: Fits<'static> = Fits {
        side: None,
        test: &|_| true,
    };
}

/// Whose roots are followed from the starting points of a lattice.
struct Finder<'a> {
    digits: u32,
    real: bool,
    known: &'a [Complex],
    others: &'a [Complex],
    fits: Fits<'a>,
}

impl Finder<'_> {
    fn old(&self, roots: &[Complex], z: &Complex) -> bool {
        roots
            .iter()
            .chain(self.known)
            .chain(self.others)
            .any(|w| same(z, w))
    }

    /// The root that a starting point leads to, added to `roots` when it is a new one; with
    /// it its conjugate, and a root next to it. `zero`: the function is exactly 0 at the
    /// starting point; `step`: the step of the lattice that the point is of.
    fn follow(
        &self,
        f: &mut dyn FnMut(&Complex) -> Option<Complex>,
        (x, y): (f64, f64),
        zero: bool,
        step: f64,
        roots: &mut Vec<Complex>,
    ) {
        let start = Complex::with_val(64, (x, y));
        // a point of the lattice where the function is 0 with all the bits of the search is
        // a root as it is (floor(x) = x holds at the whole numbers, and next to them the
        // function has no slope to follow)
        let wide = Complex::with_val(bits_for(self.digits) + 48, (x, y));
        let found = match zero && f(&wide).is_some_and(|v| v.is_zero()) {
            true => Some(wide),
            false => find_root(f, &start, self.digits),
        };
        let Some(z) = found else {
            trace::detail(|| format!("from {}: no root", trace::number(&start)));
            return;
        };
        let z = settled(z, self.digits);
        if (self.real && !z.imag().is_zero()) || !(self.fits.test)(&z) {
            return;
        }
        let new = !self.old(roots, &z);
        trace::detail(|| {
            format!(
                "from {}: {}{}",
                trace::number(&start),
                trace::number(&z),
                if new { "" } else { " (known)" }
            )
        });
        if !new {
            return;
        }
        // a root next to the real line hides its conjugate from the lattice: it is tried as a
        // starting point of its own
        let twin = (!self.real && !z.imag().is_zero())
            .then(|| Complex::with_val(64, z.conj_ref()))
            .and_then(|start| find_root(f, &start, self.digits))
            .map(|w| settled(w, self.digits));
        roots.push(z.clone());
        if let Some(w) = twin
            && !self.old(roots, &w)
            && (self.fits.test)(&w)
        {
            trace::detail(|| format!("its conjugate is a root too: {}", trace::number(&w)));
            roots.push(w);
        }
        self.neighbour(f, &start, &z, step, roots);
    }

    /// Two roots that lie much nearer to each other than the points of a lattice show as one
    /// least value there, and the search from it finds one of them. That there is another
    /// one is told by the slope at the root that was found: a part of a step of the lattice
    /// away, the function is much larger than that slope makes it. The other root is a root
    /// of the function over `x - root`.
    fn neighbour(
        &self,
        f: &mut dyn FnMut(&Complex) -> Option<Complex>,
        start: &Complex,
        root: &Complex,
        step: f64,
        roots: &mut Vec<Complex>,
    ) {
        let bits = bits_for(self.digits) + 48;
        // (to no side in particular: a point in the middle between two roots tells nothing)
        let aside = if self.real {
            (0.43 * step, 0.0)
        } else {
            (0.344 * step, 0.258 * step)
        };
        let way = 0.43 * step;
        let probe = Complex::with_val(bits, root) + Complex::with_val(bits, aside);
        let Some(value) = f(&probe) else {
            return;
        };
        let Some(slope) = crate::numroots::slope_at(f, root, self.digits) else {
            return;
        };
        let across = Float::with_val(64, slope.abs_ref()) * way;
        let value = Float::with_val(64, value.abs_ref());
        if !value.is_finite() || value <= Float::with_val(64, &across * 2u32) {
            return;
        }
        let mut rest = |x: &Complex| -> Option<Complex> {
            let prec = x.prec().0;
            let gap = Complex::with_val(prec, x - root);
            if gap.is_zero() {
                return None;
            }
            // (times the conjugate and by the square of the size, not a quotient of complex
            // numbers: MPC takes as many bits for one as its parts are apart)
            let turned = Complex::with_val(prec, gap.conj_ref());
            let norm = Float::with_val(prec, gap.norm_ref());
            Some(f(x)? * turned / norm)
        };
        let Some(other) = find_root(&mut rest, start, self.digits) else {
            return;
        };
        let other = settled(other, self.digits);
        if (self.real && !other.imag().is_zero())
            || !(self.fits.test)(&other)
            || self.old(roots, &other)
        {
            return;
        }
        trace::detail(|| format!("a root next to it: {}", trace::number(&other)));
        roots.push(other);
    }
}

/// The points of a lattice where the function is exactly 0 on whole stretches: a few of them
/// are examples, the real ones first (they are the ones that are easy to read).
fn examples(zeros: Points, who: &Finder, roots: &mut Vec<Complex>) {
    let mut zeros = zeros;
    zeros.sort_by_key(|p| p.1 != 0.0);
    let fitting: Vec<Complex> = zeros
        .into_iter()
        .map(|point| Complex::with_val(bits_for(who.digits) + 48, point))
        .filter(|z| !(who.real && !z.imag().is_zero()) && (who.fits.test)(z))
        .take(4)
        .collect();
    for z in fitting {
        if !who.old(roots, &z) {
            roots.push(z);
        }
    }
}

/// The roots of `f` nearest to 0 until `want` of them, the `known` ones counted, lie in the
/// square that was searched; with `real` only real roots, on the real line. `sized` is the
/// function as the lattice looks at it.
pub fn nearest(f: &mut dyn FnMut(&Complex) -> Option<Complex>, sized: Sized, ask: &Ask) -> Search {
    let Ask {
        want,
        digits,
        real,
        known,
        others,
        plane,
        asked,
        fits,
    } = *ask;
    let who = Finder {
        digits,
        real,
        known,
        others,
        fits,
    };
    // the half of the line that is searched: all of it for an unknown of any sign
    let half = match fits.side.filter(|_| real) {
        Some(std::cmp::Ordering::Less) => (-1.0, 0.0),
        Some(std::cmp::Ordering::Greater) => (0.0, 1.0),
        _ => (-1.0, 1.0),
    };
    let seconds = (20 + want as u64 / 10).min(300);
    let mut budget = Budget {
        deadline: asked.then(|| Instant::now() + Duration::from_secs(seconds)),
        points: 0,
        most: if asked { 8_000_000 } else { 150_000 },
    };
    // (without a count that was asked for, the plane has its share of the points: when it
    // is spent the real line goes on)
    let mut plane_open = true;
    let mirror = !real && mirrored(sized);
    let mut roots: Vec<Complex> = Vec::new();
    // the sizes of the roots inside the circle, the `want` smallest of them
    let nearest = |roots: &[Complex], radius: f64| -> Vec<f64> {
        let mut sizes: Vec<f64> = roots
            .iter()
            .chain(known)
            .filter(|z| inside(z, radius))
            .map(modulus)
            .collect();
        sizes.sort_by(f64::total_cmp);
        sizes.truncate(want);
        sizes
    };
    let (mut radius, mut searched, mut stopped) = (4.0, 0.0, false);
    // the step of the finest lattice that found several roots more: where the roots lie as
    // close as those of sin(50 x) = x, a wider square begins no coarser than that
    let mut fine = f64::INFINITY;
    trace::step(|| {
        format!(
            "search for the {want} roots nearest to 0 {}",
            if real && half.0 == 0.0 {
                "among the positive numbers: the sign and the size of the function on a lattice of points"
            } else if real && half.1 == 0.0 {
                "among the negative numbers: the sign and the size of the function on a lattice of points"
            } else if real {
                "on the real line: the sign and the size of the function on a lattice of points"
            } else if mirror {
                "in the plane: the size of the function on a lattice of points (the upper half tells the lower one)"
            } else {
                "in the plane: the size of the function on a lattice of points"
            }
        )
    });
    let _nest = trace::nest();
    'grow: loop {
        let flat = real || radius > plane || !plane_open;
        let (first, finest) = if flat { (256, 1 << 18) } else { (32, 1024) };
        let mut n: i64 = first;
        while n < finest && radius / n as f64 > fine {
            n *= 2;
        }
        let mut before: Option<Vec<f64>> = None;
        loop {
            let most = 8 * want + 256;
            let ends = (half.0 * radius, half.1 * radius);
            let starts = if flat {
                line(sized, ends, n, most, 0.0, &mut budget)
            } else {
                square(sized, radius, n, mirror, most, &mut budget)
            };
            let (starts, zeros) = match starts {
                Some(found) => found,
                None if !flat && !asked => {
                    trace::step(|| {
                        "the points for the plane are spent: the search goes on along the real line"
                            .to_string()
                    });
                    plane_open = false;
                    budget.most += 60_000;
                    continue 'grow;
                }
                None => {
                    stopped = true;
                    break 'grow;
                }
            };
            let step = if flat {
                (ends.1 - ends.0) / (2 * n) as f64
            } else {
                radius / n as f64
            };
            // an equation that holds on whole stretches of numbers: a few of their points
            // are examples enough, and no roots are counted
            if stretching(sized, &zeros, step, &mut budget) {
                let count = zeros.len();
                examples(zeros, &who, &mut roots);
                trace::step(|| {
                    format!(
                        "the function is exactly 0 at {count} points of the lattice from -{radius} to {radius}, and between them: the equation holds on whole stretches of numbers"
                    )
                });
                searched = radius;
                break 'grow;
            }
            let (mut tried, had) = (0, roots.len());
            for (x, y) in starts {
                if budget.late() {
                    stopped = true;
                    break 'grow;
                }
                // (a starting point well beyond the roots that are wanted finds none of them)
                let sizes = nearest(&roots, radius);
                if sizes.len() >= want
                    && (x * x + y * y).sqrt() > sizes[want - 1] * 1.25 + 4.0 * step
                {
                    continue;
                }
                tried += 1;
                who.follow(f, (x, y), zeros.contains(&(x, y)), step, &mut roots);
            }
            // (the first lattice of a wider square finds what is new there; a finer one of the
            // same square finds what the coarser one could not tell apart)
            if before.is_some() && roots.len() >= had + 4 {
                fine = fine.min(step);
            }
            let now = nearest(&roots, radius);
            trace::step(|| {
                format!(
                    "{} from {} to {}, step {step}: {}, {} inside{}",
                    if flat { "the line" } else { "the square" },
                    if flat { ends.0 } else { -radius },
                    if flat { ends.1 } else { radius },
                    trace::count(tried, "starting point"),
                    trace::count(
                        roots
                            .iter()
                            .chain(known)
                            .filter(|z| inside(z, radius))
                            .count(),
                        "root"
                    ),
                    if now.len() >= want {
                        format!(", the {want} nearest within {}", now[want - 1])
                    } else {
                        String::new()
                    }
                )
            });
            // done here when half the step finds the same nearest roots
            let same = before.as_ref().is_some_and(|old| {
                old.len() == now.len()
                    && old
                        .iter()
                        .zip(&now)
                        .all(|(a, b)| (a - b).abs() <= 1e-9 * (1.0 + b))
            });
            if same || n >= finest {
                break;
            }
            before = Some(now);
            n *= 2;
        }
        searched = radius;
        if nearest(&roots, radius).len() >= want || radius >= FAR {
            break;
        }
        // a square many times as wide as all the roots that were found, and no more of them
        // in it: an equation with so few roots is not followed to the end of the numbers
        let farthest = roots.iter().chain(known).map(modulus).fold(0.0, f64::max);
        if farthest > 0.0 && radius >= (64.0 * farthest).max(256.0) {
            trace::step(|| {
                format!(
                    "no more roots as far as {radius}, many times as far as the ones that were found: the search ends"
                )
            });
            break;
        }
        radius *= 2.0;
    }
    roots.sort_by(|a, b| {
        modulus(a)
            .total_cmp(&modulus(b))
            .then_with(|| a.real().to_f64().total_cmp(&b.real().to_f64()))
            .then_with(|| a.imag().to_f64().total_cmp(&b.imag().to_f64()))
    });
    Search {
        roots,
        radius: searched,
        stopped,
    }
}

/// The real roots of `f` from `lo` to `hi`, the `known` ones apart: all that a lattice of the
/// interval shows, made finer until half its step finds the same ones. `stopped`: there are
/// more of them than `MANY`, or no lattice in reach showed the same roots twice. (`radius` is
/// half the length of the interval.)
pub fn between(
    f: &mut dyn FnMut(&Complex) -> Option<Complex>,
    sized: Sized,
    (lo, hi): (f64, f64),
    digits: u32,
    known: &[Complex],
) -> Search {
    let who = Finder {
        digits,
        real: true,
        known,
        others: &[],
        fits: Fits::ALL,
    };
    // (no limit of time: the same points on every machine)
    let mut budget = Budget {
        deadline: None,
        points: 0,
        most: 150_000,
    };
    let (center, half) = ((lo + hi) / 2.0, (hi - lo) / 2.0);
    // (a little wider: a root at an end lies between two points)
    let radius = half * 1.03125;
    let within = |z: &Complex| {
        let x = z.real().to_f64();
        x >= lo - (radius - half) && x <= hi + (radius - half)
    };
    let mut roots: Vec<Complex> = Vec::new();
    let mut stopped = false;
    let (mut n, finest): (i64, i64) = (256, 1 << 16);
    let mut before: Option<Vec<f64>> = None;
    trace::step(|| {
        format!(
            "search for the real roots from {lo} to {hi}: the sign and the size of the function on a lattice of points"
        )
    });
    let _nest = trace::nest();
    loop {
        let ends = (center - radius, center + radius);
        let Some((starts, zeros)) = line(sized, ends, n, usize::MAX, center, &mut budget) else {
            stopped = true;
            break;
        };
        let step = radius / n as f64;
        if stretching(sized, &zeros, step, &mut budget) {
            let count = zeros.len();
            examples(zeros, &who, &mut roots);
            trace::step(|| {
                format!(
                    "the function is exactly 0 at {count} points of the lattice, and between them: the equation holds on whole stretches of numbers"
                )
            });
            break;
        }
        if starts.len() > 8 * MANY {
            stopped = true;
            break;
        }
        let tried = starts.len();
        for start in starts {
            who.follow(f, start, zeros.contains(&start), step, &mut roots);
        }
        let mut now: Vec<f64> = roots
            .iter()
            .chain(known)
            .filter(|z| within(z))
            .map(|z| z.real().to_f64())
            .collect();
        now.sort_by(f64::total_cmp);
        trace::step(|| {
            format!(
                "the line from {lo} to {hi}, step {step}: {}, {} inside",
                trace::count(tried, "starting point"),
                trace::count(now.len(), "root"),
            )
        });
        if now.len() > MANY {
            stopped = true;
            break;
        }
        let same = before.as_ref().is_some_and(|old| {
            old.len() == now.len()
                && old
                    .iter()
                    .zip(&now)
                    .all(|(a, b)| (a - b).abs() <= 1e-9 * (1.0 + b.abs()))
        });
        if same {
            break;
        }
        if n >= finest {
            stopped = true;
            break;
        }
        before = Some(now);
        n *= 2;
    }
    roots.sort_by(|a, b| a.real().to_f64().total_cmp(&b.real().to_f64()));
    Search {
        roots,
        radius: half,
        stopped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sizes(search: &Search) -> Vec<f64> {
        search.roots.iter().map(modulus).collect()
    }

    /// The search for a function that is looked at by its bare size.
    fn search(
        f: fn(&Complex) -> Option<Complex>,
        want: usize,
        real: bool,
        known: &[Complex],
        plane: f64,
        asked: bool,
    ) -> Search {
        let mut value = f;
        let mut sized = |z: &Complex| f(z).map(|v| (v, 0.0));
        let ask = Ask {
            want,
            digits: 32,
            real,
            known,
            others: &[],
            plane,
            asked,
            fits: Fits::ALL,
        };
        nearest(&mut value, &mut sized, &ask)
    }

    fn sine_less_its_argument(z: &Complex) -> Option<Complex> {
        let p = z.prec().0;
        Some(Complex::with_val(p, z.sin_ref()) - z)
    }

    #[test]
    fn the_roots_nearest_to_zero_in_the_plane() {
        // sin(z) = z without its root 0: pairs at 7.9925..., 14.2984..., 20.5769...
        let zero = [Complex::new(64)];
        for (want, count, farthest) in [
            (1, 0, 0.0),
            (2, 4, 7.992542086),
            (5, 4, 7.992542086),
            (6, 8, 14.29846814),
        ] {
            let found = search(
                sine_less_its_argument,
                want,
                false,
                &zero,
                f64::INFINITY,
                true,
            );
            assert!(!found.stopped);
            let found = sizes(&found);
            // (the search may know of more than it was asked for: the nearest are what counts)
            let near: Vec<f64> = found.iter().copied().take(count).collect();
            assert_eq!(near.len(), count, "{want}: {found:?}");
            if count > 0 {
                assert!(
                    (near[count - 1] - farthest).abs() < 1e-8,
                    "{want}: {found:?}"
                );
                assert!(
                    found.iter().skip(count).all(|s| *s > farthest + 1.0),
                    "{want}: {found:?}"
                );
            }
        }
    }

    #[test]
    fn real_roots_only_on_the_real_line() {
        // x sin(x) = 1: 1.114..., 2.772..., 6.439..., 9.317... and their negatives
        fn f(z: &Complex) -> Option<Complex> {
            let p = z.prec().0;
            Some(Complex::with_val(p, z * Complex::with_val(p, z.sin_ref())) - 1u32)
        }
        let found = search(f, 6, true, &[], f64::INFINITY, true);
        assert!(found.roots.iter().all(|z| z.imag().is_zero()));
        let found = sizes(&found);
        assert!(found.len() >= 6, "{found:?}");
        for (size, want) in found.iter().zip([
            1.1141571409,
            1.1141571409,
            2.7726047083,
            2.7726047083,
            6.4391172384,
            6.4391172384,
        ]) {
            assert!((size - want).abs() < 1e-9, "{found:?}");
        }
    }

    #[test]
    fn a_sum_of_exponentials_is_looked_at_balanced() {
        // e^((1 + i) z) (cos(z) cosh(z) - 1) as the sum of its exponentials: its roots are
        // +-a and +-i a for a = 4.7300..., 7.8532..., but its bare size rises like
        // e^(Re(z) - Im(z)) and hides the ones to the right from a lattice; times
        // e^(-(1 + i) z) it rises alike to all sides
        fn terms(z: &Complex) -> [Complex; 5] {
            let p = z.prec().0;
            let i = Complex::with_val(p, (0, 1));
            let iz = Complex::with_val(p, z * &i);
            let quarter = |w: Complex| Complex::with_val(p, w.exp()) / 4u32;
            [
                quarter(Complex::with_val(p, &iz * 2u32)),
                quarter(Complex::with_val(p, &iz * 2u32) + Complex::with_val(p, z * 2u32)),
                quarter(Complex::with_val(p, z * 2u32)),
                -Complex::with_val(p, &iz + z).exp(),
                Complex::with_val(p, (0.25, 0)),
            ]
        }
        // (without the root that 0 is four times)
        fn value(z: &Complex) -> Option<Complex> {
            if z.is_zero() {
                return None;
            }
            let p = z.prec().0;
            let total = terms(z).iter().fold(Complex::new(p), |a, t| a + t);
            Some(total / Complex::with_val(p, z.square_ref()).square())
        }
        let mut f = value;
        let mut sized = |z: &Complex| value(z).map(|v| (v, z.real().to_f64() - z.imag().to_f64()));
        let zero = [Complex::new(64)];
        let ask = Ask {
            want: 20,
            digits: 32,
            real: false,
            known: &zero,
            others: &[],
            plane: f64::INFINITY,
            asked: true,
            fits: Fits::ALL,
        };
        let found = nearest(&mut f, &mut sized, &ask);
        let found = sizes(&found);
        assert!(found.len() >= 20, "{found:?}");
        let fours = [
            4.7300407449,
            7.8532046241,
            10.9956078380,
            14.1371654913,
            17.2787596574,
        ];
        for (k, size) in found.iter().take(20).enumerate() {
            assert!((size - fours[k / 4]).abs() < 1e-9, "{found:?}");
        }
    }

    #[test]
    fn a_search_that_nobody_asked_a_count_of_ends_by_itself() {
        // (no limit of time: the same points of the lattice on every machine)
        fn f(z: &Complex) -> Option<Complex> {
            let p = z.prec().0;
            Some(Complex::with_val(p, z.cos_ref()) - z)
        }
        let found = search(f, 8, false, &[], 64.0, false);
        assert!(!found.stopped);
        assert!(sizes(&found).len() >= 8);
    }

    #[test]
    fn an_equation_that_holds_on_stretches_gives_examples() {
        // 0 for every number from 2 to 3 (and with imaginary parts below 1), 1 elsewhere
        fn f(z: &Complex) -> Option<Complex> {
            let inside = *z.real() >= 2 && *z.real() < 3 && *z.imag() >= 0 && *z.imag() < 1;
            Some(Complex::with_val(z.prec(), u32::from(!inside)))
        }
        let found = search(f, 5, false, &[], f64::INFINITY, true);
        assert!(!found.stopped);
        assert!(!found.roots.is_empty() && found.roots.len() <= 4);
        assert!(found.roots.iter().all(|z| *z.real() >= 2 && *z.real() < 3));
    }

    /// x sin(x) = 1.
    fn waves(z: &Complex) -> Option<Complex> {
        let p = z.prec().0;
        Some(Complex::with_val(p, z * Complex::with_val(p, z.sin_ref())) - 1u32)
    }

    fn reals(search: &Search) -> Vec<f64> {
        search.roots.iter().map(|z| z.real().to_f64()).collect()
    }

    #[test]
    fn every_real_root_of_an_interval() {
        let mut f = waves;
        let mut sized = |z: &Complex| waves(z).map(|v| (v, 0.0));
        let found = between(&mut f, &mut sized, (20.0, 30.0), 32, &[]);
        assert!(!found.stopped);
        let inside: Vec<f64> = reals(&found)
            .into_iter()
            .filter(|x| (20.0..=30.0).contains(x))
            .collect();
        let want = [
            21.945_565_498_819_684,
            25.172_477_611_734_38,
            28.238_914_350_639,
        ];
        assert_eq!(inside.len(), want.len(), "{inside:?}");
        for (x, w) in inside.iter().zip(want) {
            assert!((x - w).abs() < 1e-12, "{inside:?}");
        }
        // a root that is known is not found again
        let place = Float::parse("25.172477611734379387").expect("a number");
        let known = [Complex::with_val(200, (Float::with_val(200, place), 0.0))];
        let found = between(&mut f, &mut sized, (20.0, 30.0), 32, &known);
        // (the known root is told to 20 digits: the one that is found differs from there on)
        assert!(
            found
                .roots
                .iter()
                .filter(|z| *z.real() > 20 && *z.real() < 30)
                .count()
                <= 3
        );
        // more roots than are listed for an interval: the search says that it did not end
        fn many(z: &Complex) -> Option<Complex> {
            let p = z.prec().0;
            Some(Complex::with_val(
                p,
                Complex::with_val(p, z * 1000u32).sin_ref(),
            ))
        }
        let mut f = many;
        let mut sized = |z: &Complex| many(z).map(|v| (v, 0.0));
        assert!(between(&mut f, &mut sized, (0.5, 10.5), 32, &[]).stopped);
    }

    #[test]
    fn a_root_next_to_the_one_that_was_found_is_found_too() {
        // (x - 1)^2 = cos(x)/10^18: the roots 1 +- 7.35...e-10, which a lattice shows as one
        // least value and no change of sign
        fn pair(z: &Complex) -> Option<Complex> {
            let p = z.prec().0;
            let small = Float::with_val(p, Float::parse("1e-18").expect("a number"));
            let square = Complex::with_val(p, z - 1u32).square();
            Some(square - Complex::with_val(p, z.cos_ref()) * small)
        }
        let found = search(pair, 2, true, &[], f64::INFINITY, true);
        let mut found = reals(&found);
        found.sort_by(f64::total_cmp);
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(
            (found[0] - 0.999_999_999_264_947_4).abs() < 1e-15,
            "{found:?}"
        );
        assert!(
            (found[1] - 1.000_000_000_735_052_6).abs() < 1e-15,
            "{found:?}"
        );
    }

    #[test]
    fn only_the_numbers_that_the_unknown_may_be() {
        // x sin(x) = 1 among the positive numbers, among the negative ones: the half line
        let mut f = waves;
        let mut sized = |z: &Complex| waves(z).map(|v| (v, 0.0));
        for (side, sign) in [
            (std::cmp::Ordering::Greater, 1.0),
            (std::cmp::Ordering::Less, -1.0),
        ] {
            let test = move |z: &Complex| z.real().to_f64() * sign > 0.0;
            let ask = Ask {
                want: 3,
                digits: 32,
                real: true,
                known: &[],
                others: &[],
                plane: f64::INFINITY,
                asked: true,
                fits: Fits {
                    side: Some(side),
                    test: &test,
                },
            };
            let found = reals(&nearest(&mut f, &mut sized, &ask));
            assert!(found.len() >= 3, "{found:?}");
            for (x, want) in found.iter().zip([
                1.114_157_140_871_93,
                2.772_604_708_265_991,
                6.439_117_238_417_246,
            ]) {
                assert!((x - sign * want).abs() < 1e-12, "{found:?}");
            }
        }
    }

    #[test]
    fn whole_numbers_on_a_lattice_are_no_stretch() {
        // floor(x) = x holds at the whole numbers only; a lattice whose step is a whole
        // number has no other points, and next to a whole number the function has no slope
        fn steps(z: &Complex) -> Option<Complex> {
            if !z.imag().is_zero() {
                return None;
            }
            let down = Float::with_val(z.prec().0, z.real().floor_ref());
            Some(Complex::with_val(z.prec(), (down - z.real(), 0)))
        }
        let found = search(steps, 9, true, &[], f64::INFINITY, true);
        let mut found = reals(&found);
        found.sort_by(f64::total_cmp);
        assert!(found.len() >= 9, "{found:?}");
        assert!(found.iter().all(|x| x.fract() == 0.0), "{found:?}");
        for whole in -4..=4 {
            assert!(found.contains(&f64::from(whole)), "{found:?}");
        }
    }

    #[test]
    fn roots_that_lie_close_are_found_in_wider_squares_too() {
        // sin(50 z) = z: 31 real roots from -1 to 1, 0.063 apart, and rows of others beside
        // the real line with the same gaps; a square that is wider begins with the step that
        // the one before it needed
        fn dense(z: &Complex) -> Option<Complex> {
            let p = z.prec().0;
            Some(Complex::with_val(p, Complex::with_val(p, z * 50u32).sin_ref()) - z)
        }
        let found = search(dense, 120, false, &[], f64::INFINITY, true);
        assert!(!found.stopped);
        let found = sizes(&found);
        assert!(found.len() >= 120, "{} roots", found.len());
        // (the 120 nearest lie within 4.2 of 0: beyond 1 there are four of them for each
        // step of pi/50 away)
        assert!(found[119] < 4.2, "the 120th root is {} away", found[119]);
    }
}
