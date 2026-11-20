//! More of the design of filters: filters of least squares, by samples of the frequency and
//! with equal ripples (Parks and McClellan), another sampling rate, lattices, phases, delays,
//! partial fractions in 1/z and the warping of time.

use crate::arrays::word;
use crate::eval::Session;
use crate::expr::Expr;
use crate::numeric;
use crate::sigdesign::{
    Res, bits, complexes, count, cx, dec, fft, int, items, lfilter, pi, poly_mul, reals,
    response_at, shown, shown_c, window, window_of,
};
use rug::{Complex, Float};

fn zero() -> Float {
    Float::new(bits())
}

fn fl(x: &Float) -> Float {
    Float::with_val(bits(), x)
}

/// A design whose equations lose digits: worked again with more digits until the answer
/// stays.
fn steady(work: &mut dyn FnMut() -> Result<Vec<Float>, String>) -> Result<Vec<Float>, String> {
    let digits = numeric::digits();
    let mut before: Option<Vec<Float>> = None;
    let mut more = 0;
    for _ in 0..5 {
        numeric::set_digits(digits + more);
        let got = work();
        numeric::set_digits(digits);
        let got = got?;
        let least = Float::with_val(64, Float::i_pow_u(10, digits + 2)).recip();
        let same = before.as_ref().is_some_and(|old| {
            old.len() == got.len()
                && old.iter().zip(&got).all(|(a, b)| {
                    let apart = Float::with_val(64, a - b).abs();
                    let size = Float::with_val(64, b.abs_ref()).max(&Float::with_val(64, 1));
                    apart <= size * &least
                })
        });
        if same {
            return Ok(got.iter().map(fl).collect());
        }
        before = Some(got);
        more = if more == 0 { digits + 20 } else { 2 * more };
    }
    Ok(before.unwrap_or_default().iter().map(fl).collect())
}

/// The bands of a design: the angles of their edges (pi is half the sampling rate), the
/// gains that are wanted at the edges, a weight for every band.
struct Bands {
    edges: Vec<Float>,
    gains: Vec<Float>,
    weights: Vec<Float>,
}

impl Bands {
    fn read(a: &[Expr], who: &str, apart: bool) -> Result<Bands, String> {
        let (edges, gains) = (reals(&a[1], who)?, reals(&a[2], who)?);
        let usage = || {
            format!(
                "{who}(n, f, a[, w]): the edges f of the bands in pairs from 0 to 1 (1 is half the sampling rate), rising; the gain a at every edge; a weight w for every band"
            )
        };
        if edges.len() < 2 || edges.len() % 2 == 1 || gains.len() != edges.len() {
            return Err(usage());
        }
        let weights = match a.get(3).filter(|e| word(e).is_none()) {
            Some(e) => reals(e, who)?,
            None => vec![int(1); edges.len() / 2],
        };
        let rising = edges.windows(2).enumerate().all(|(k, w)| {
            // (the bands of a filter of equal ripples lie apart)
            if k % 2 == 0 || apart {
                w[0] < w[1]
            } else {
                w[0] <= w[1]
            }
        });
        if !rising
            || edges[0] < 0
            || edges[edges.len() - 1] > 1
            || weights.len() != edges.len() / 2
            || weights.iter().any(|w| *w <= 0)
        {
            return Err(usage());
        }
        Ok(Bands {
            edges: edges.iter().map(|f| pi() * f).collect(),
            gains,
            weights,
        })
    }

    fn count(&self) -> usize {
        self.weights.len()
    }

    /// The gain that is wanted in a band at an angle.
    fn wanted(&self, band: usize, w: &Float) -> Float {
        let (low, high) = (&self.edges[2 * band], &self.edges[2 * band + 1]);
        let (from, to) = (&self.gains[2 * band], &self.gains[2 * band + 1]);
        let along = Float::with_val(bits(), w - low) / Float::with_val(bits(), high - low);
        Float::with_val(bits(), to - from) * along + from
    }

    /// A gain is wanted at half the sampling rate, where a filter with an even number of
    /// taps has none.
    fn open_at_the_end(&self) -> bool {
        let last = self.edges.len() - 1;
        Float::with_val(bits(), &self.edges[last] - pi()).abs() < Float::with_val(bits(), 1e-30)
            && !self.gains[last].is_zero()
    }
}

/// The taps of a filter from the coefficients of its cosines: `c[k]` of `cos(k w)` for an
/// odd number of taps, of `cos((k + 1/2) w)` for an even one.
fn taps_of(c: &[Float], taps: usize) -> Vec<Float> {
    let mut h = vec![zero(); taps];
    if taps % 2 == 1 {
        let middle = taps / 2;
        h[middle] = fl(&c[0]);
        for k in 1..c.len() {
            let half = Float::with_val(bits(), &c[k] / 2u32);
            h[middle - k] = half.clone();
            h[middle + k] = half;
        }
    } else {
        let half_way = taps / 2;
        for (k, value) in c.iter().enumerate() {
            let half = Float::with_val(bits(), value / 2u32);
            h[half_way - 1 - k] = half.clone();
            h[half_way + k] = half;
        }
    }
    h
}

/// The filter with the least weighted square error against the gains of the bands.
fn least_squares(order: usize, bands: &Bands) -> Result<Vec<Float>, String> {
    let taps = order + 1;
    let odd = taps % 2 == 1;
    let unknowns = if odd { taps.div_ceil(2) } else { taps / 2 };
    let shift = if odd { zero() } else { dec("0.5") };
    let at = |k: usize| Float::with_val(bits(), k as u64) + &shift;
    // the integral of cos(p w) over a band
    let cosines = |p: &Float, low: &Float, high: &Float| -> Float {
        match p.is_zero() {
            true => Float::with_val(bits(), high - low),
            false => {
                (Float::with_val(bits(), p * high).sin() - Float::with_val(bits(), p * low).sin())
                    / p
            }
        }
    };
    let mut q = vec![vec![zero(); unknowns]; unknowns];
    let mut d = vec![zero(); unknowns];
    for band in 0..bands.count() {
        let (low, high) = (&bands.edges[2 * band], &bands.edges[2 * band + 1]);
        let (from, to) = (&bands.gains[2 * band], &bands.gains[2 * band + 1]);
        let weight = &bands.weights[band];
        let slope = Float::with_val(bits(), to - from) / Float::with_val(bits(), high - low);
        let offset = Float::with_val(bits(), from - Float::with_val(bits(), &slope * low));
        for i in 0..unknowns {
            let mi = at(i);
            // the integral of (offset + slope w) cos(mi w)
            let part = match mi.is_zero() {
                true => {
                    Float::with_val(bits(), &offset * Float::with_val(bits(), high - low))
                        + Float::with_val(bits(), &slope / 2u32)
                            * (Float::with_val(bits(), high * high)
                                - Float::with_val(bits(), low * low))
                }
                false => {
                    let end = |w: &Float| -> Float {
                        let angle = Float::with_val(bits(), &mi * w);
                        let line =
                            Float::with_val(bits(), &offset + Float::with_val(bits(), &slope * w));
                        line * angle.clone().sin() / &mi
                            + Float::with_val(bits(), &slope * angle.cos())
                                / Float::with_val(bits(), &mi * &mi)
                    };
                    end(high) - end(low)
                }
            };
            d[i] += Float::with_val(bits(), weight * part);
            for (j, mj) in (i..unknowns).map(|j| (j, at(j))) {
                let both = cosines(&Float::with_val(bits(), &mi - &mj), low, high)
                    + cosines(&Float::with_val(bits(), &mi + &mj), low, high);
                let value = Float::with_val(bits(), weight * both) / 2u32;
                q[i][j] += &value;
                if j != i {
                    q[j][i] += &value;
                }
            }
        }
    }
    let c = crate::mfun::linear(q, d).ok_or("firls: these bands do not say what the filter is")?;
    Ok(taps_of(&c, taps))
}

/// The filter of a number of taps whose weighted error against the gains of the bands has
/// the least greatest size: the exchange of Remez, with the greatest errors sought on the
/// whole bands.
fn equal_ripples(order: usize, bands: &Bands) -> Result<Vec<Float>, String> {
    let taps = order + 1;
    let even = taps.is_multiple_of(2);
    let r = if even { taps / 2 } else { taps.div_ceil(2) };
    // a filter with an even number of taps is cos(w/2) times a sum of r cosines
    let lean = |w: &Float| -> Float {
        match even {
            true => Float::with_val(bits(), w / 2u32).cos(),
            false => int(1),
        }
    };
    let total = (0..bands.count()).fold(zero(), |sum, b| {
        sum + Float::with_val(bits(), &bands.edges[2 * b + 1] - &bands.edges[2 * b])
    });
    // a place of all bands laid end to end
    let place = |along: &Float| -> (usize, Float) {
        let mut left = fl(along);
        for band in 0..bands.count() {
            let wide = Float::with_val(bits(), &bands.edges[2 * band + 1] - &bands.edges[2 * band]);
            if left <= wide || band + 1 == bands.count() {
                return (
                    band,
                    Float::with_val(bits(), &bands.edges[2 * band] + left.min(&wide)),
                );
            }
            left -= wide;
        }
        (0, fl(&bands.edges[0]))
    };
    let mut points: Vec<(usize, Float)> = (0..=r)
        .map(|k| {
            place(&(Float::with_val(bits(), &total * (2 * k as u64 + 1)) / (2 * (r as u64 + 1))))
        })
        .collect();
    let tiny = Float::with_val(bits(), 1) >> (bits() / 3);
    let done = Float::with_val(bits(), Float::i_pow_u(10, numeric::digits() + 6)).recip();
    let mut density = 16usize;
    for _ in 0..200 {
        // the sum of cosines through the points, with the error of one size and of signs in turn
        let xs: Vec<Float> = points.iter().map(|(_, w)| w.clone().cos()).collect();
        let lifts: Vec<Float> = (0..=r)
            .map(|k| {
                let mut product = int(1);
                for j in 0..=r {
                    if j != k {
                        product *= Float::with_val(bits(), &xs[k] - &xs[j]);
                    }
                }
                product.recip()
            })
            .collect();
        let (mut above, mut below) = (zero(), zero());
        let wanted: Vec<(Float, Float)> = points
            .iter()
            .map(|(band, w)| {
                let q = lean(w);
                (
                    bands.wanted(*band, w) / &q,
                    Float::with_val(bits(), &bands.weights[*band] * &q),
                )
            })
            .collect();
        for k in 0..=r {
            above += Float::with_val(bits(), &lifts[k] * &wanted[k].0);
            let term = Float::with_val(bits(), &lifts[k] / &wanted[k].1);
            if k % 2 == 0 {
                below += term;
            } else {
                below -= term;
            }
        }
        if below.is_zero() {
            return Err("firpm: these bands do not say what the filter is".into());
        }
        let ripple = above / below;
        let ys: Vec<Float> = (0..=r)
            .map(|k| {
                let step = Float::with_val(bits(), &ripple / &wanted[k].1);
                match k % 2 == 0 {
                    true => Float::with_val(bits(), &wanted[k].0 - step),
                    false => Float::with_val(bits(), &wanted[k].0 + step),
                }
            })
            .collect();
        let sum_at = |w: &Float| -> Float {
            let x = w.clone().cos();
            let (mut top, mut bottom) = (zero(), zero());
            for k in 0..=r {
                let apart = Float::with_val(bits(), &x - &xs[k]);
                if apart.is_zero() {
                    return fl(&ys[k]);
                }
                let part = Float::with_val(bits(), &lifts[k] / apart);
                top += Float::with_val(bits(), &part * &ys[k]);
                bottom += part;
            }
            top / bottom
        };
        let error = |band: usize, w: &Float| -> Float {
            let missed = bands.wanted(band, w) - lean(w) * sum_at(w);
            missed * &bands.weights[band]
        };
        // the places where the error is greatest: on a grid first, then between its places
        let mut found: Vec<(usize, Float, Float)> = Vec::new();
        for band in 0..bands.count() {
            let (low, high) = (&bands.edges[2 * band], &bands.edges[2 * band + 1]);
            let wide = Float::with_val(bits(), high - low);
            let share = Float::with_val(64, &wide / &total).to_f64();
            let cells = ((density * r) as f64 * share).ceil().max(4.0) as usize;
            let grid: Vec<Float> = (0..=cells)
                .map(|k| {
                    Float::with_val(
                        bits(),
                        low + Float::with_val(bits(), &wide * k as u64) / cells as u64,
                    )
                })
                .collect();
            let errors: Vec<Float> = grid.iter().map(|w| error(band, w)).collect();
            let size = |k: usize| errors[k].clone().abs();
            for k in 0..=cells {
                let here = size(k);
                if here.is_zero() {
                    continue;
                }
                let inner = k > 0 && k < cells;
                let peak = match (k == 0, k == cells) {
                    (true, _) => here > size(1),
                    (_, true) => here > size(cells - 1),
                    _ => here >= size(k - 1) && here > size(k + 1),
                };
                if !peak {
                    continue;
                }
                if !inner {
                    found.push((band, grid[k].clone(), errors[k].clone()));
                    continue;
                }
                // (the golden section between the neighbours of the grid)
                let sign = if errors[k] > 0 { 1 } else { -1 };
                let height = |w: &Float| -> Float { error(band, w) * sign };
                let ratio = (Float::with_val(bits(), 5).sqrt() - 1u32) / 2u32;
                let (mut from, mut to) = (grid[k - 1].clone(), grid[k + 1].clone());
                let mut inner_low = Float::with_val(
                    bits(),
                    &to - Float::with_val(bits(), &ratio * Float::with_val(bits(), &to - &from)),
                );
                let mut inner_high = Float::with_val(
                    bits(),
                    &from + Float::with_val(bits(), &ratio * Float::with_val(bits(), &to - &from)),
                );
                let (mut at_low, mut at_high) = (height(&inner_low), height(&inner_high));
                while Float::with_val(bits(), &to - &from) > Float::with_val(bits(), &tiny * &wide)
                {
                    if at_low > at_high {
                        to = inner_high;
                        inner_high = inner_low;
                        at_high = at_low;
                        inner_low = Float::with_val(
                            bits(),
                            &to - Float::with_val(
                                bits(),
                                &ratio * Float::with_val(bits(), &to - &from),
                            ),
                        );
                        at_low = height(&inner_low);
                    } else {
                        from = inner_low;
                        inner_low = inner_high;
                        at_low = at_high;
                        inner_high = Float::with_val(
                            bits(),
                            &from
                                + Float::with_val(
                                    bits(),
                                    &ratio * Float::with_val(bits(), &to - &from),
                                ),
                        );
                        at_high = height(&inner_high);
                    }
                }
                let best = Float::with_val(bits(), &from + &to) / 2u32;
                let value = error(band, &best);
                found.push((band, best, value));
            }
        }
        // signs in turn: of neighbours with one sign the greater stays
        let mut turns: Vec<(usize, Float, Float)> = Vec::new();
        for item in found {
            match turns.last_mut() {
                Some(last) if (last.2 > 0) == (item.2 > 0) => {
                    if item.2.clone().abs() > last.2.clone().abs() {
                        *last = item;
                    }
                }
                _ => turns.push(item),
            }
        }
        // r + 1 of them: the least ones go, at an end alone, inside with a neighbour
        while turns.len() > r + 1 {
            let size = |k: usize| turns[k].2.clone().abs();
            let last = turns.len() - 1;
            if turns.len() == r + 2 {
                let end = if size(0) < size(last) { 0 } else { last };
                turns.remove(end);
                continue;
            }
            let least = (0..=last)
                .min_by(|a, b| {
                    size(*a)
                        .partial_cmp(&size(*b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(0);
            if least == 0 || least == last {
                turns.remove(least);
            } else {
                let beside = if size(least - 1) < size(least + 1) {
                    least - 1
                } else {
                    least
                };
                turns.drain(beside..=beside + 1);
            }
        }
        if turns.len() < r + 1 {
            // (a grid that is too coarse misses a turn)
            if density >= 128 {
                return Err(
                    "firpm: the errors of this filter do not turn often enough: take other bands or another order".into(),
                );
            }
            density *= 2;
            continue;
        }
        let greatest = turns.iter().fold(zero(), |m, t| m.max(&t.2.clone().abs()));
        let least = turns
            .iter()
            .fold(greatest.clone(), |m, t| m.min(&t.2.clone().abs()));
        let settled = Float::with_val(bits(), &greatest - &least)
            <= Float::with_val(bits(), &greatest * &done);
        points = turns.into_iter().map(|(band, w, _)| (band, w)).collect();
        if settled || greatest.is_zero() {
            // the coefficients of the cosines from the values of the sum at as many places
            let values: Vec<Float> = (0..r)
                .map(|j| sum_at(&(pi() * (2 * j as u64 + 1) / (2 * r as u64))))
                .collect();
            let c: Vec<Float> = (0..r)
                .map(|k| {
                    let mut sum = zero();
                    for (j, value) in values.iter().enumerate() {
                        let angle = pi() * (k as u64 * (2 * j as u64 + 1)) / (2 * r as u64);
                        sum += Float::with_val(bits(), value * angle.cos());
                    }
                    sum * (if k == 0 { 1u32 } else { 2u32 }) / r as u64
                })
                .collect();
            if !even {
                return Ok(taps_of(&c, taps));
            }
            // cos(w/2) cos(k w) = (cos((k + 1/2) w) + cos((k - 1/2) w))/2
            let halves: Vec<Float> = (0..r)
                .map(|j| {
                    let own = match j == 0 {
                        true => fl(&c[0]),
                        false => Float::with_val(bits(), &c[j] / 2u32),
                    };
                    match c.get(j + 1) {
                        Some(next) => own + Float::with_val(bits(), next / 2u32),
                        None => own,
                    }
                })
                .collect();
            return Ok(taps_of(&halves, taps));
        }
    }
    Err("firpm: the exchange of the greatest errors did not come to an end".into())
}

/// A straight line between the points of a broken line; at a jump the value after it.
fn between(xs: &[Float], ys: &[Float], x: &Float) -> Float {
    let after = xs.iter().rposition(|point| point <= x).unwrap_or(0);
    match xs.get(after + 1) {
        None => fl(&ys[after]),
        Some(next) => {
            let along = Float::with_val(bits(), x - &xs[after])
                / Float::with_val(bits(), next - &xs[after]);
            Float::with_val(bits(), &ys[after + 1] - &ys[after]) * along + &ys[after]
        }
    }
}

/// The filter whose response goes through the gains `m` at the frequencies `f`: samples
/// of the response on a grid, the transform back, a window (as `fir2` of Octave).
fn sampled(
    order: usize,
    f: &[Float],
    m: &[Float],
    grid: Option<usize>,
    ramp: Option<usize>,
    win: &[Float],
) -> Result<Vec<Float>, String> {
    let taps = order + 1;
    let grid = grid
        .unwrap_or(if taps < 1024 { 512 } else { taps })
        .next_power_of_two();
    if 2 * grid < taps {
        return Err("fir2: the grid is at least half as long as the filter".into());
    }
    let ramp = ramp.unwrap_or(grid / 25);
    let (mut xs, mut ys) = (f.to_vec(), m.to_vec());
    if ramp > 0 {
        // a jump becomes a slope around its place, which stays a point of the line
        let half = Float::with_val(bits(), ramp as u64) / (2 * grid as u64);
        let mut moved = f.to_vec();
        let mut kept = Vec::new();
        for k in 0..f.len().saturating_sub(1) {
            if f[k] == f[k + 1] {
                moved[k] = Float::with_val(bits(), &f[k] - &half);
                moved[k + 1] = Float::with_val(bits(), &f[k + 1] + &half);
                kept.push(f[k].clone());
            }
        }
        moved.extend(kept);
        for x in &mut moved {
            if *x < 0 {
                *x = zero();
            } else if *x > 1 {
                *x = int(1);
            }
        }
        moved.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        moved.dedup();
        ys = moved.iter().map(|x| between(f, m, x)).collect();
        xs = moved;
    }
    let values: Vec<Float> = (0..=grid)
        .map(|k| between(&xs, &ys, &(Float::with_val(bits(), k as u64) / grid as u64)))
        .collect();
    let mirrored = |holes: usize| -> Vec<Complex> {
        let mut all: Vec<Complex> = values.iter().map(cx).collect();
        all.extend(std::iter::repeat_n(cx(&zero()), holes));
        all.extend(values[1..grid].iter().rev().map(cx));
        all
    };
    let h: Vec<Float> = if order.is_multiple_of(2) {
        let b = fft(&mirrored(0), true);
        let half = order / 2;
        (b.len() - half..b.len())
            .chain(0..=half)
            .map(|k| b[k].real().clone())
            .collect()
    } else {
        // (zeros in the middle halve the steps of the time; every second value is taken)
        let b = fft(&mirrored(2 * grid), true);
        (b.len() - order..b.len())
            .step_by(2)
            .chain((1..=order).step_by(2))
            .map(|k| Float::with_val(bits(), b[k].real() * 2u32))
            .collect()
    };
    Ok(h.iter()
        .zip(win)
        .map(|(x, w)| Float::with_val(bits(), x * w))
        .collect())
}

/// The values at another rate: `up - 1` zeros after every value, a lowpass of sinc with
/// Kaiser's window, every `down`-th value (as `resample_poly` of SciPy).
fn another_rate(x: &[Float], up: usize, down: usize) -> Result<Vec<Float>, String> {
    let gcd = |mut a: usize, mut b: usize| {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    };
    let shared = gcd(up, down);
    let (up, down) = (up / shared, down / shared);
    if up == 1 && down == 1 {
        return Ok(x.to_vec());
    }
    let most = up.max(down);
    let half = 10 * most;
    let size = 2 * half + 1;
    let win = window("kaiser", size, Some(&int(5)))?;
    let edge = int(1) / most as u64;
    let mut h: Vec<Float> = (0..size)
        .map(|k| {
            let m = int(k as i64 - half as i64);
            let ideal = match m.is_zero() {
                true => edge.clone(),
                false => (pi() * &edge * &m).sin() / (pi() * &m),
            };
            ideal * &win[k]
        })
        .collect();
    let sum = h.iter().fold(zero(), |s, v| s + v);
    for v in &mut h {
        *v = Float::with_val(bits(), &*v / &sum) * up as u64;
    }
    let out = (x.len() * up).div_ceil(down);
    let before = down - half % down;
    let cut = (half + before) / down;
    let reached = |long: usize| ((x.len() - 1) * up + long - 1) / down + 1;
    let mut after = 0;
    while reached(size + before + after) < out + cut {
        after += 1;
    }
    let mut padded = vec![zero(); before];
    padded.extend(h);
    padded.extend(vec![zero(); after]);
    let mut wide = vec![zero(); (x.len() - 1) * up + 1];
    for (k, v) in x.iter().enumerate() {
        wide[k * up] = v.clone();
    }
    let full = poly_mul(&wide, &padded);
    Ok(full.into_iter().step_by(down).skip(cut).take(out).collect())
}

/// The reflections of a polynomial `1 + a1/z + ...` and the polynomials of all lower orders
/// (the first one is the given one).
fn stepped_down(a: &[Float], who: &str) -> Result<(Vec<Float>, Vec<Vec<Float>>), String> {
    let mut now: Vec<Float> = a
        .iter()
        .map(|x| Float::with_val(bits(), x / &a[0]))
        .collect();
    let mut all = vec![now.clone()];
    let mut reflections = Vec::new();
    for m in (1..now.len()).rev() {
        let k = now[m].clone();
        let rest = int(1) - Float::with_val(bits(), &k * &k);
        if rest.clone().abs() < Float::with_val(bits(), 1) >> (bits() / 2) {
            return Err(format!(
                "{who}: a reflection of the size 1 has no lattice (a zero lies on the unit circle)"
            ));
        }
        now = (0..m)
            .map(|i| {
                (Float::with_val(bits(), &now[i]) - Float::with_val(bits(), &k * &now[m - i]))
                    / &rest
            })
            .collect();
        all.push(now.clone());
        reflections.push(k);
    }
    reflections.reverse();
    Ok((reflections, all))
}

/// The polynomial of reflections.
fn stepped_up(k: &[Float]) -> Vec<Float> {
    let mut a = vec![int(1)];
    for (m, reflection) in k.iter().enumerate() {
        let before = a.clone();
        a.push(zero());
        for i in 1..=m + 1 {
            let mirrored = &before[m + 1 - i];
            a[i] += Float::with_val(bits(), reflection * mirrored);
        }
    }
    a
}

/// The filter of a lattice with a ladder: numerator and denominator.
fn of_lattice(k: &[Float], v: &[Float]) -> (Vec<Float>, Vec<Float>) {
    let a = stepped_up(k);
    let mut lower = vec![vec![int(1)]];
    for m in 1..=k.len() {
        lower.push(stepped_up(&k[..m]));
    }
    let mut b = vec![zero(); k.len() + 1];
    for (m, weight) in v.iter().enumerate().take(k.len() + 1) {
        // the backward polynomial of the order m is the forward one turned round
        for (i, c) in lower[m].iter().rev().enumerate() {
            b[i] += Float::with_val(bits(), weight * c);
        }
    }
    (b, a)
}

impl Session {
    pub(crate) fn sigmore(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        let n = a.len();
        Some((|| -> Res {
            let who = name;
            let list = |k: usize| reals(&a[k], who);
            let pair = |b: Vec<Float>, a: Vec<Float>| Expr::list(vec![shown(&b), shown(&a)]);
            Ok(match (name, n) {
                ("firls" | "firpm", 3 | 4) => {
                    let mut order = count(&a[0], "the order", 1, 300)?;
                    let least = name == "firls";
                    if Bands::read(a, who, !least)?.open_at_the_end() && order % 2 == 1 {
                        // (a gain at half the sampling rate: an even order)
                        order += 1;
                    }
                    let taps = steady(&mut || {
                        let bands = Bands::read(a, who, !least)?;
                        match least {
                            true => least_squares(order, &bands),
                            false => equal_ripples(order, &bands),
                        }
                    })?;
                    shown(&taps)
                }
                ("fir2", 3..=6) => {
                    let order = count(&a[0], "the order", 1, 4096)?;
                    let (f, m) = (list(1)?, list(2)?);
                    let ordered = f.windows(2).all(|w| w[0] <= w[1]);
                    if f.len() < 2 || f.len() != m.len() || !ordered || !f[0].is_zero() || f[f.len() - 1] != 1 {
                        return Err("fir2(n, f, m): the frequencies f rise from 0 to 1 (1 is half the sampling rate), with a gain m at each; two equal ones are a jump".into());
                    }
                    let (mut numbers, mut shape) = (Vec::new(), None);
                    for e in &a[3..] {
                        match e.as_i64() {
                            Some(v) if v >= 0 => numbers.push(v as usize),
                            _ => shape = Some(e),
                        }
                    }
                    if numbers.len() > 2 {
                        return Err("fir2(n, f, m[, points of the grid[, points of a slope]][, window])".into());
                    }
                    let win = window_of(shape, order + 1, "hamming", who)?;
                    shown(&sampled(order, &f, &m, numbers.first().copied(), numbers.get(1).copied(), &win)?)
                }
                ("resample", 3) => {
                    let x = list(0)?;
                    let up = count(&a[1], "the factor up", 1, 1 << 12)?;
                    let down = count(&a[2], "the factor down", 1, 1 << 12)?;
                    if x.is_empty() {
                        return Err("resample needs values".into());
                    }
                    shown(&another_rate(&x, up, down)?)
                }
                ("phasez" | "phasedelay", 1..=3) => {
                    let b = list(0)?;
                    let den = if n > 1 { list(1)? } else { vec![int(1)] };
                    let points = if n > 2 { count(&a[2], "the number of frequencies", 1, 1 << 16)? } else { 512 };
                    if b.is_empty() || den.is_empty() || den[0].is_zero() {
                        return Err(format!("{name}(b, a, n): the coefficients of a filter; a[1] is not 0"));
                    }
                    let ws: Vec<Float> = (0..points).map(|k| pi() * k as u64 / points as u64).collect();
                    let mut phases: Vec<Float> = Vec::with_capacity(points);
                    let mut turns = zero();
                    let mut before: Option<Float> = None;
                    for w in &ws {
                        let phase = response_at(&b, &den, w).arg().real().clone();
                        if let Some(old) = &before {
                            let step = Float::with_val(bits(), &phase - old);
                            if step > pi() {
                                turns -= pi() * 2u32;
                            } else if step < -pi() {
                                turns += pi() * 2u32;
                            }
                        }
                        before = Some(phase.clone());
                        phases.push(phase + &turns);
                    }
                    if name == "phasedelay" {
                        // (at the frequency 0 the delay is the slope of the phase there)
                        let slope = |p: &[Float]| -> Float {
                            let weighted = p.iter().enumerate().fold(zero(), |s, (k, c)| s + Float::with_val(bits(), c * k as u64));
                            let plain = p.iter().fold(zero(), |s, c| s + c);
                            match plain.is_zero() {
                                true => zero(),
                                false => weighted / plain,
                            }
                        };
                        let first = slope(&b) - slope(&den);
                        phases = phases
                            .iter()
                            .zip(&ws)
                            .map(|(p, w)| match w.is_zero() {
                                true => first.clone(),
                                false => Float::with_val(bits(), -p) / w,
                            })
                            .collect();
                    }
                    Expr::list(vec![shown(&phases), shown(&ws)])
                }
                ("zerophase", 1..=3) => {
                    let b = list(0)?;
                    let den = if n > 1 { list(1)? } else { vec![int(1)] };
                    let points = if n > 2 { count(&a[2], "the number of frequencies", 1, 1 << 16)? } else { 512 };
                    let top = b.iter().fold(zero(), |m, x| m.max(&x.clone().abs()));
                    let small = Float::with_val(bits(), &top * (Float::with_val(bits(), 1) >> (bits() / 2)));
                    let mirrored = |sign: i32| {
                        b.iter().zip(b.iter().rev()).all(|(x, y)| Float::with_val(bits(), x - Float::with_val(bits(), y * sign)).abs() <= small)
                    };
                    let (even, odd) = (mirrored(1), mirrored(-1));
                    if b.is_empty() || den.len() != 1 || den[0].is_zero() || !(even || odd) {
                        return Err("zerophase(b): the real response of a filter of linear phase: taps that read the same from both ends, or the same with the other sign".into());
                    }
                    let middle = Float::with_val(bits(), b.len() as u64 - 1) / 2u32;
                    let ws: Vec<Float> = (0..points).map(|k| pi() * k as u64 / points as u64).collect();
                    let values: Vec<Float> = ws
                        .iter()
                        .map(|w| {
                            // the response without the delay of half the length
                            let back = Float::with_val(bits(), w * &middle);
                            let (sin, cos) = back.sin_cos(zero());
                            let turned = response_at(&b, &den, w) * Complex::with_val(bits(), (cos, sin));
                            match even {
                                true => turned.real().clone(),
                                false => turned.imag().clone(),
                            }
                        })
                        .collect();
                    Expr::list(vec![shown(&values), shown(&ws)])
                }
                ("finddelay", 2) => {
                    let (x, y) = (list(0)?, list(1)?);
                    if x.is_empty() || y.is_empty() {
                        return Err("finddelay(x, y): two lists of values".into());
                    }
                    // the shift of y against x at which they agree most
                    let mut best: Option<(i64, Float)> = None;
                    let slack = Float::with_val(bits(), 1) >> (bits() / 2);
                    for d in -(x.len() as i64 - 1)..=(y.len() as i64 - 1) {
                        let mut sum = zero();
                        for (k, v) in x.iter().enumerate() {
                            if let Some(other) = usize::try_from(k as i64 + d).ok().and_then(|at| y.get(at)) {
                                sum += Float::with_val(bits(), v * other);
                            }
                        }
                        let size = sum.abs();
                        let better = match &best {
                            None => true,
                            Some((at, most)) => {
                                let margin = Float::with_val(bits(), most * &slack);
                                if size > Float::with_val(bits(), most + &margin) {
                                    true
                                } else if size >= Float::with_val(bits(), most - &margin) {
                                    // (of equal ones the nearest to 0, of two as near the later)
                                    d.abs() < at.abs() || (d.abs() == at.abs() && d > *at)
                                } else {
                                    false
                                }
                            }
                        };
                        if better {
                            best = Some((d, size));
                        }
                    }
                    match best {
                        Some((d, size)) if !size.is_zero() => Expr::int(d),
                        _ => Expr::zero(),
                    }
                }
                ("dtw", 2 | 3) => {
                    let (x, y) = (list(0)?, list(1)?);
                    let squared = match a.get(2).and_then(word).as_deref() {
                        None | Some("absolute" | "euclidean") => false,
                        Some("squared") => true,
                        Some(_) => return Err("dtw(x, y[, \"absolute\" or \"squared\"])".into()),
                    };
                    if x.is_empty() || y.is_empty() || x.len() * y.len() > 4_000_000 {
                        return Err("dtw(x, y): two lists of values (4 000 000 pairs at most)".into());
                    }
                    let cost = |i: usize, j: usize| -> Float {
                        let apart = Float::with_val(bits(), &x[i] - &y[j]);
                        match squared {
                            true => apart.square(),
                            false => apart.abs(),
                        }
                    };
                    let (rows, columns) = (x.len(), y.len());
                    let mut total: Vec<Float> = Vec::with_capacity(rows * columns);
                    for i in 0..rows {
                        for j in 0..columns {
                            let before = match (i, j) {
                                (0, 0) => zero(),
                                (0, _) => total[j - 1].clone(),
                                (_, 0) => total[(i - 1) * columns].clone(),
                                _ => {
                                    let (up, left, both) = (
                                        &total[(i - 1) * columns + j],
                                        &total[i * columns + j - 1],
                                        &total[(i - 1) * columns + j - 1],
                                    );
                                    both.clone().min(up).min(left)
                                }
                            };
                            total.push(before + cost(i, j));
                        }
                    }
                    // the way back: through both lists at once where that costs no more
                    let (mut i, mut j) = (rows - 1, columns - 1);
                    let (mut ix, mut iy) = (vec![i + 1], vec![j + 1]);
                    while i > 0 || j > 0 {
                        let at = |i: usize, j: usize| &total[i * columns + j];
                        (i, j) = match (i, j) {
                            (0, _) => (0, j - 1),
                            (_, 0) => (i - 1, 0),
                            _ => {
                                let (both, up, left) = (at(i - 1, j - 1), at(i - 1, j), at(i, j - 1));
                                if both <= up && both <= left {
                                    (i - 1, j - 1)
                                } else if up <= left {
                                    (i - 1, j)
                                } else {
                                    (i, j - 1)
                                }
                            }
                        };
                        ix.push(i + 1);
                        iy.push(j + 1);
                    }
                    ix.reverse();
                    iy.reverse();
                    let whole = |v: Vec<usize>| Expr::list(v.into_iter().map(|k| Expr::int(k as i64)).collect());
                    Expr::list(vec![crate::mfun::value(&total[rows * columns - 1]), whole(ix), whole(iy)])
                }
                ("residuez", 2) => {
                    let turned = |e: &Expr| Expr::list(items(e).into_iter().rev().collect());
                    if items(&a[1]).is_empty() || items(&a[0]).is_empty() {
                        return Err("residuez(b, a): the coefficients of a ratio of polynomials in 1/z".into());
                    }
                    // in x = 1/z the parts are those of a ratio of polynomials in x
                    let parts = self.call("residue", &[turned(&a[0]), turned(&a[1])])?;
                    let [r, p, k] = &items(&parts)[..] else {
                        return Err("residuez: the parts of this ratio were not found".into());
                    };
                    let (r, p) = (complexes(r, who)?, complexes(p, who)?);
                    let close = |x: &Complex, y: &Complex| {
                        Complex::with_val(bits(), x - y).abs().real().clone()
                            <= Float::with_val(bits(), 1) >> (bits() / 2)
                    };
                    let mut poles = Vec::new();
                    let mut residues = Vec::new();
                    for (at, (res, pole)) in r.iter().zip(&p).enumerate() {
                        if pole.real().is_zero() && pole.imag().is_zero() {
                            return Err("residuez: a[1] is not 0".into());
                        }
                        // a pole that stands again has the next power
                        let power = 1 + p[..at].iter().rev().take_while(|q| close(q, pole)).count() as u32;
                        let inverse = Complex::with_val(bits(), pole.recip_ref());
                        let factor = Complex::with_val(bits(), -&inverse);
                        let mut scale = Complex::with_val(bits(), 1);
                        for _ in 0..power {
                            scale *= &factor;
                        }
                        residues.push(Complex::with_val(bits(), res * &scale));
                        poles.push(inverse);
                    }
                    Expr::list(vec![shown_c(&residues), shown_c(&poles), turned(k)])
                }
                ("tf2latc", 1 | 2) => {
                    let b = list(0)?;
                    if b.is_empty() || b[0].is_zero() {
                        return Err("tf2latc(b) or tf2latc(b, a): coefficients whose first one is not 0".into());
                    }
                    if n == 1 {
                        return Ok(shown(&stepped_down(&b, who)?.0));
                    }
                    let den = list(1)?;
                    if den.is_empty() || den[0].is_zero() || b.len() > den.len() {
                        return Err("tf2latc(b, a): a[1] is not 0, and b is no longer than a".into());
                    }
                    let (k, lower) = stepped_down(&den, who)?;
                    // the ladder: from the highest power down, what is left of b
                    let mut left: Vec<Float> = b.iter().map(|x| Float::with_val(bits(), x / &den[0])).collect();
                    left.resize(den.len(), zero());
                    let mut ladder = vec![zero(); den.len()];
                    for m in (0..den.len()).rev() {
                        let weight = left[m].clone();
                        let forward = &lower[den.len() - 1 - m];
                        for (i, c) in forward.iter().rev().enumerate() {
                            left[i] -= Float::with_val(bits(), &weight * c);
                        }
                        ladder[m] = weight;
                    }
                    pair(k, ladder)
                }
                ("latc2tf", 1 | 2) => {
                    let k = list(0)?;
                    match a.get(1).map(|e| (word(e), e)) {
                        None => shown(&stepped_up(&k)),
                        Some((Some(kind), _)) if kind == "fir" => shown(&stepped_up(&k)),
                        Some((Some(kind), _)) if kind == "allpole" => pair(vec![int(1)], stepped_up(&k)),
                        Some((Some(_), _)) => return Err("latc2tf(k), latc2tf(k, \"allpole\"), latc2tf(k, v)".into()),
                        Some((None, e)) => {
                            let v = reals(e, who)?;
                            if v.len() > k.len() + 1 {
                                return Err("latc2tf(k, v): the ladder has one value more than the lattice at most".into());
                            }
                            let (b, den) = of_lattice(&k, &v);
                            pair(b, den)
                        }
                    }
                }
                ("latcfilt", 2 | 3) => {
                    let k = list(0)?;
                    let x = list(n - 1)?;
                    let (b, den) = match n {
                        2 => (stepped_up(&k), vec![int(1)]),
                        _ => {
                            let v = list(1)?;
                            if v.len() > k.len() + 1 {
                                return Err("latcfilt(k, v, x): the ladder has one value more than the lattice at most".into());
                            }
                            of_lattice(&k, &v)
                        }
                    };
                    shown(&lfilter(&b, &den, &x, None))
                }
                ("fftfilt", 2) => {
                    let (b, x) = (list(0)?, list(1)?);
                    if b.is_empty() {
                        return Err("fftfilt(b, x): the taps of a filter and values".into());
                    }
                    shown(&lfilter(&b, &[int(1)], &x, None))
                }
                ("filtic", 3 | 4) => {
                    let (b, den, y) = (list(0)?, list(1)?, list(2)?);
                    let x = if n > 3 { list(3)? } else { Vec::new() };
                    if den.is_empty() || den[0].is_zero() || b.is_empty() {
                        return Err("filtic(b, a, y[, x]): the coefficients of a filter (a[1] is not 0), its last outputs and inputs, the latest first".into());
                    }
                    let scaled = |p: &[Float]| -> Vec<Float> { p.iter().map(|c| Float::with_val(bits(), c / &den[0])).collect() };
                    let (b, den) = (scaled(&b), scaled(&den));
                    let size = b.len().max(den.len()) - 1;
                    let at = |v: &[Float], k: usize| v.get(k).cloned().unwrap_or_else(zero);
                    let state: Vec<Float> = (0..size)
                        .map(|m| {
                            let mut sum = zero();
                            for (k, c) in b.iter().enumerate().skip(m + 1) {
                                sum += Float::with_val(bits(), c * at(&x, k - m - 1));
                            }
                            for (k, c) in den.iter().enumerate().skip(m + 1) {
                                sum -= Float::with_val(bits(), c * at(&y, k - m - 1));
                            }
                            sum
                        })
                        .collect();
                    shown(&state)
                }
                _ => return Err(String::new()),
            })
        })())
        .filter(|r| !matches!(r, Err(e) if e.is_empty()))
    }
}

#[cfg(test)]
mod tests {
    fn said(input: &str) -> String {
        crate::eval::run_line(input).map_or_else(|e| format!("error: {e}"), |lines| lines.join("|"))
    }

    #[test]
    fn filters_of_least_squares_and_of_equal_ripples() {
        // half of the band: the middle tap is 1/2, the next ones 1/pi, every second one 0
        assert_eq!(
            said("firls(4, [0, 1/2, 1/2, 1], [1, 1, 0, 0])"),
            "[0, 0.31830988618379067153776752674503, 0.5, 0.31830988618379067153776752674503, 0]"
        );
        // three taps with ripples of one size: 1/2 - sqrt(5)/10 beside 1/2
        assert!(
            said("firpm(2, [0, 0.2, 0.8, 1], [1, 1, 0, 0])")
                .starts_with("[0.276393202250021030359082633")
        );
        assert_eq!(
            said("islinphase(remez(6, [0, 0.4, 0.6, 1], [1, 1, 0, 0], [1, 10]))"),
            "true"
        );
        assert_eq!(
            said("length(firls(5, [0, 0.4, 0.6, 1], [0, 0, 1, 1]))"),
            "7"
        );
        assert!(said("firpm(4, [0, 0.5, 0.3, 1], [1, 1, 0, 0])").starts_with("error: firpm("));
    }

    #[test]
    fn lattices_go_there_and_back() {
        assert_eq!(said("tf2latc([1, 1/2, 1/4])"), "[0.4, 0.25]");
        assert_eq!(said("latc2tf(tf2latc([1, 1/2, 1/4]))"), "[1, 0.5, 0.25]");
        assert_eq!(
            said("latcfilt([2/5, 1/4], [1, 0, 0, 0])"),
            "[1, 0.5, 0.25, 0]"
        );
        assert!(said("tf2latc([1, 1/2, 1])").contains("unit circle"));
    }

    #[test]
    fn shifts_ways_and_parts() {
        assert_eq!(said("finddelay([1, 2, 3, 0, 0], [0, 0, 1, 2, 3])"), "2");
        assert_eq!(
            said("dtw([1, 2, 3, 4], [1, 1, 2, 3, 4])"),
            "[0, [1, 1, 2, 3, 4], [1, 2, 3, 4, 5]]"
        );
        assert_eq!(
            said("residuez([1, 2], [1, -3/4, 1/8])"),
            "[[10, -9], [0.5, 0.25], []]"
        );
        assert_eq!(said("resample([1, 2, 3, 4], 1, 1)"), "[1, 2, 3, 4]");
        assert_eq!(
            said("fftfilt([1, -1], [1, 4, 9, 16, 25])"),
            "[1, 3, 5, 7, 9]"
        );
        assert_eq!(
            said("part(phasedelay([0, 0, 1], [1], 4), 1)"),
            "[2, 2, 2, 2]"
        );
    }

    #[test]
    fn a_filter_goes_on_from_its_state() {
        assert_eq!(
            said("filter([1, 1], [1, -1/2], [1, 2, 3, 4])"),
            "[1, 7/2, 27/4, 83/8]"
        );
        // the second half from the state that the first half leaves
        assert_eq!(
            said("filter([1, 1], [1, -1/2], [3, 4], filtic([1, 1], [1, -1/2], [7/2, 1], [2, 1]))"),
            "[6.75, 10.375]"
        );
        assert_eq!(
            said("filter([1, 1], [1, -1/2], [1, 1, 1], [4])"),
            "[5, 9/2, 17/4]"
        );
        assert!(
            said("filter([1, 1], [1, -1/2], [1, 1, 1], [4, 5])").contains("the state zi has 1")
        );
    }
}
