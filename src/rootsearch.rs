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

fn same(a: &Complex, b: &Complex) -> bool {
    let gap = Float::with_val(64, Complex::with_val(64, a - b).abs_ref());
    let scale = Float::with_val(64, a.abs_ref()) + 1u32;
    gap < Float::with_val(64, &scale >> 40u32)
}

/// A part that is rounding noise next to the other one is zero.
fn settled(z: Complex, digits: u32) -> Complex {
    let scale = Float::with_val(64, z.abs_ref()) + 1u32;
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

/// The points of a lattice of the interval from which a real root is looked for: where the sign
/// changes (followed by halving to the root, not to a pole) and where the function is smaller
/// than at both neighbours; and the points of the lattice where the function is exactly 0.
fn line(
    f: Sized,
    radius: f64,
    n: i64,
    most: usize,
    budget: &mut Budget,
) -> Option<(Points, Points)> {
    let h = radius / n as f64;
    let mut values = Vec::with_capacity((2 * n + 1) as usize);
    for i in -n..=n {
        if !budget.point() {
            return None;
        }
        values.push(f(&Complex::with_val(64, (i as f64 * h, 0.0))));
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
        let mut lo = Float::with_val(64, (k as i64 - n) as f64 * h);
        let mut hi = Float::with_val(64, (k as i64 + 1 - n) as f64 * h);
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
            starts.push(((k as i64 - n) as f64 * h, 0.0));
        }
    }
    let nearest_first =
        |a: &(f64, f64), b: &(f64, f64)| a.0.abs().total_cmp(&b.0.abs()).then(a.0.total_cmp(&b.0));
    starts.sort_by(nearest_first);
    starts.dedup();
    starts.truncate(most);
    let mut zeros: Vec<(f64, f64)> = (1..values.len() - 1)
        .filter(|k| sizes[*k] == f64::NEG_INFINITY)
        .map(|k| ((k as i64 - n) as f64 * h, 0.0))
        .collect();
    zeros.sort_by(nearest_first);
    Some((starts, zeros))
}

/// The roots of `f` nearest to 0 until `want` of them, the `known` ones counted, lie in the
/// square that was searched; with `real` only real roots, on the real line. Farther than
/// `plane` from 0 only the real line is searched. `asked`: somebody asked for this count, and
/// the search may take its time (20 seconds and more for many roots); else it ends after a
/// fixed count of points, the same on every machine. `sized` is the function as the lattice
/// looks at it.
#[allow(clippy::too_many_arguments)]
pub fn nearest(
    f: &mut dyn FnMut(&Complex) -> Option<Complex>,
    sized: Sized,
    want: usize,
    digits: u32,
    real: bool,
    known: &[Complex],
    plane: f64,
    asked: bool,
) -> Search {
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
    trace::step(|| {
        format!(
            "search for the {want} roots nearest to 0 {}",
            if real {
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
        let mut n = first;
        let mut before: Option<Vec<f64>> = None;
        loop {
            let most = 8 * want + 256;
            let starts = if flat {
                line(sized, radius, n, most, &mut budget)
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
            let mut tried = 0;
            let step = radius / n as f64;
            // an equation that holds at many points of a lattice holds on whole stretches of
            // numbers: a few of those points are examples enough, and no roots are counted
            let stretches = zeros.len() > 32;
            let zero_count = zeros.len();
            let starts: Vec<(f64, f64)> = if stretches {
                // (real ones first: they are the examples that are easy to read; they solve
                // the equation as they are)
                let mut examples = zeros;
                examples.sort_by_key(|p| p.1 != 0.0);
                for (x, y) in examples.into_iter().take(4) {
                    let z = Complex::with_val(bits_for(digits) + 48, (x, y));
                    if !(real && y != 0.0) && !roots.iter().chain(known).any(|w| same(&z, w)) {
                        roots.push(z);
                    }
                }
                Vec::new()
            } else {
                starts
            };
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
                let start = Complex::with_val(64, (x, y));
                let Some(z) = find_root(f, &start, digits) else {
                    trace::detail(|| format!("from {}: no root", trace::number(&start)));
                    continue;
                };
                let z = settled(z, digits);
                if real && !z.imag().is_zero() {
                    continue;
                }
                let new = !roots.iter().chain(known).any(|w| same(&z, w));
                trace::detail(|| {
                    format!(
                        "from {}: {}{}",
                        trace::number(&start),
                        trace::number(&z),
                        if new { "" } else { " (known)" }
                    )
                });
                if new {
                    // a root next to the real line hides its conjugate from the lattice: it is
                    // tried as a starting point of its own
                    let twin = (!real && !z.imag().is_zero())
                        .then(|| Complex::with_val(64, z.conj_ref()))
                        .and_then(|start| find_root(f, &start, digits))
                        .map(|w| settled(w, digits));
                    roots.push(z);
                    if let Some(w) = twin
                        && !roots.iter().chain(known).any(|known| same(&w, known))
                    {
                        trace::detail(|| {
                            format!("its conjugate is a root too: {}", trace::number(&w))
                        });
                        roots.push(w);
                    }
                }
            }
            let now = nearest(&roots, radius);
            trace::step(|| {
                format!(
                    "{} from -{radius} to {radius}, step {step}: {}, {} inside{}",
                    if flat { "the line" } else { "the square" },
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
            if stretches {
                trace::step(|| {
                    format!(
                        "the function is exactly 0 at {zero_count} points of the lattice: the equation holds on whole stretches of numbers"
                    )
                });
                searched = radius;
                break 'grow;
            }
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
        nearest(&mut value, &mut sized, want, 32, real, known, plane, asked)
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
        let found = nearest(
            &mut f,
            &mut sized,
            20,
            32,
            false,
            &zero,
            f64::INFINITY,
            true,
        );
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
}
