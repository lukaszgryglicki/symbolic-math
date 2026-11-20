//! Statistics of samples taken together: runs and spreads, the normal law of several
//! variables, shapes laid onto one another, factors without signs, and laws fitted to data.

use crate::arrays::tidy;
use crate::canon::{add, div, mul, neg, pow};
use crate::decomp::{self, CMat};
use crate::expr::Expr;
use crate::hypotests::{
    Chance, beyond, bits, counted, exact, farthest, fl, mean, points, positive, real, reals, shown,
    sums_of_choices, table,
};
use crate::numeric;
use crate::pencils;
use crate::radix::text_of;
use rug::float::{Constant, Special};
use rug::{Complex, Float, Integer, Rational};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;

type Res = Result<Expr, String>;
type Mat = Vec<Vec<Float>>;

fn wide(m: &Mat) -> CMat {
    m.iter()
        .map(|r| r.iter().map(|x| Complex::with_val(x.prec(), x)).collect())
        .collect()
}

fn narrow(m: &CMat) -> Mat {
    m.iter()
        .map(|r| r.iter().map(|z| z.real().clone()).collect())
        .collect()
}

fn out(m: &Mat) -> Expr {
    decomp::from_complex(&wide(m))
}

fn out_list(v: &[Float]) -> Expr {
    let wide: Vec<Complex> = v.iter().map(|x| Complex::with_val(x.prec(), x)).collect();
    Expr::list(decomp::from_complex_list(&wide))
}

/// A number that is nothing beside the size it is measured by shows as 0.
fn out_beside(x: &Float, size: &Float) -> Expr {
    let small = fl(size * fl(Float::i_pow_u(10, numeric::digits() + 2)).recip());
    match fl(x.abs_ref()) <= small {
        true => Expr::zero(),
        false => shown(x),
    }
}

fn product(a: &Mat, b: &Mat) -> Mat {
    a.iter()
        .map(|row| {
            (0..b[0].len())
                .map(|j| row.iter().zip(b).fold(fl(0), |s, (x, r)| s + fl(x * &r[j])))
                .collect()
        })
        .collect()
}

fn turned(a: &Mat) -> Mat {
    (0..a[0].len())
        .map(|j| a.iter().map(|r| r[j].clone()).collect())
        .collect()
}

fn squares(m: &Mat) -> Float {
    m.iter()
        .flatten()
        .fold(fl(0), |s, v| s + fl(v.square_ref()))
}

/// The rows with the mean of every column taken away, and those means.
fn centred(m: &Mat) -> (Mat, Vec<Float>) {
    let count = m.len() as u32;
    let means: Vec<Float> = (0..m[0].len())
        .map(|j| m.iter().fold(fl(0), |s, r| s + &r[j]) / count)
        .collect();
    let rows = m
        .iter()
        .map(|r| r.iter().zip(&means).map(|(x, c)| fl(x - c)).collect())
        .collect();
    (rows, means)
}

fn determinant(m: &Mat) -> Float {
    let n = m.len();
    let mut m = m.to_vec();
    let mut det = fl(1);
    for k in 0..n {
        let pivot = (k..n)
            .max_by(|i, j| {
                fl(m[*i][k].abs_ref())
                    .partial_cmp(&fl(m[*j][k].abs_ref()))
                    .unwrap_or(Ordering::Equal)
            })
            .unwrap_or(k);
        if m[pivot][k].is_zero() {
            return fl(0);
        }
        if pivot != k {
            m.swap(pivot, k);
            det = -det;
        }
        det *= &m[k][k];
        let head = m[k].clone();
        for row in m[k + 1..].iter_mut() {
            let factor = fl(&row[k] / &head[k]);
            for (x, h) in row.iter_mut().zip(&head).skip(k) {
                *x -= fl(&factor * h);
            }
        }
    }
    det
}

/// The values and the directions (in columns) that a symmetric matrix keeps (Jacobi turns).
fn eigen(a: &Mat) -> (Vec<Float>, Mat) {
    let n = a.len();
    let mut a = a.to_vec();
    let mut v: Mat = (0..n)
        .map(|i| (0..n).map(|j| fl(u32::from(i == j))).collect())
        .collect();
    let small = fl(1) >> (bits() - 8);
    let turn = |row: &mut Vec<Float>, p: usize, q: usize, c: &Float, s: &Float| {
        let (x, y) = (row[p].clone(), row[q].clone());
        row[p] = fl(c * &x) - fl(s * &y);
        row[q] = fl(s * &x) + fl(c * &y);
    };
    for _ in 0..200 {
        let top = a
            .iter()
            .flatten()
            .fold(fl(0), |s, x| s.max(&fl(x.abs_ref())));
        let limit = fl(&top * &small);
        let mut moved = false;
        for p in 0..n {
            for q in p + 1..n {
                if fl(a[p][q].abs_ref()) <= limit {
                    continue;
                }
                moved = true;
                let theta = fl(&a[q][q] - &a[p][p]) / fl(&a[p][q] * 2u32);
                let root = (fl(theta.square_ref()) + 1u32).sqrt();
                let mut t = (fl(theta.abs_ref()) + root).recip();
                if theta.is_sign_negative() {
                    t = -t;
                }
                let c = (fl(t.square_ref()) + 1u32).sqrt().recip();
                let s = fl(&t * &c);
                for row in a.iter_mut() {
                    turn(row, p, q, &c, &s);
                }
                let (upper, lower) = a.split_at_mut(q);
                for (x, y) in upper[p].iter_mut().zip(lower[0].iter_mut()) {
                    let (old_x, old_y) = (x.clone(), y.clone());
                    *x = fl(&c * &old_x) - fl(&s * &old_y);
                    *y = fl(&s * &old_x) + fl(&c * &old_y);
                }
                for row in v.iter_mut() {
                    turn(row, p, q, &c, &s);
                }
            }
        }
        if !moved {
            break;
        }
    }
    (a.iter().enumerate().map(|(i, r)| r[i].clone()).collect(), v)
}

/// The lower triangle L with `A = L*L'`; none when A is not positive definite.
fn cholesky(a: &Mat) -> Option<Mat> {
    let n = a.len();
    let mut l: Mat = vec![vec![fl(0); n]; n];
    for (i, given) in a.iter().enumerate() {
        let (done, rest) = l.split_at_mut(i);
        let row = &mut rest[0];
        for j in 0..=i {
            let mut s = given[j].clone();
            if j < i {
                for (x, y) in row[..j].iter().zip(&done[j][..j]) {
                    s -= fl(x * y);
                }
                row[j] = s / &done[j][j];
            } else {
                for x in row[..j].iter() {
                    s -= fl(x.square_ref());
                }
                if !positive(&s) {
                    return None;
                }
                row[j] = s.sqrt();
            }
        }
    }
    Some(l)
}

// ---- runs ----

fn choices(n: usize, k: usize) -> Integer {
    Integer::from(Integer::binomial_u(n as u32, k as u32))
}

fn runs_of(sides: &[bool]) -> usize {
    1 + sides.windows(2).filter(|w| w[0] != w[1]).count()
}

/// In how many orders so many values above a level and so many below it make every count
/// of runs.
fn runs_about(above: usize, below: usize) -> Vec<Integer> {
    let all = above + below;
    let mut counts = vec![Integer::new(); all + 2];
    if above == 0 || below == 0 {
        counts[1] = Integer::from(1);
        return counts;
    }
    let (u, d) = (above - 1, below - 1);
    for k in 1..=above.min(below) {
        counts[2 * k] = choices(u, k - 1) * choices(d, k - 1) * 2u32;
        if 2 * k < all {
            counts[2 * k + 1] =
                choices(u, k - 1) * choices(d, k) + choices(u, k) * choices(d, k - 1);
        }
    }
    counts
}

/// In how many orders so many values that all differ rise and fall in every count of runs.
fn runs_up_down(all: usize) -> Vec<Integer> {
    let mut counts = vec![Integer::new(); all + 1];
    if all < 2 {
        return counts;
    }
    counts[1] = Integer::from(2);
    for n in 3..=all {
        let mut next = vec![Integer::new(); all + 1];
        for r in 1..n {
            let mut ways = Integer::from(&counts[r] * r as u32);
            ways += Integer::from(&counts[r - 1] * 2u32);
            if r >= 2 {
                ways += Integer::from(&counts[r - 2] * (n - r) as u32);
            }
            next[r] = ways;
        }
        counts = next;
    }
    counts
}

/// Twice the smaller of the chances of so many or fewer and of so many or more.
fn tails(counts: &[Integer], seen: usize) -> Chance {
    let total = |part: &[Integer]| part.iter().fold(Integer::new(), |s, c| s + c);
    let (below, above) = (total(&counts[..=seen]), total(&counts[seen..]));
    Chance::twice(below.min(above), total(counts))
}

/// The chance by the normal law, the count moved half a step towards its mean.
fn near(runs: usize, mean: Float, var: Float) -> Chance {
    let away = fl(fl(runs as u64) - &mean).abs();
    if !positive(&var) || away <= 0.5 {
        return Chance::Near(fl(1));
    }
    Chance::Near(beyond(&((away - 0.5f64) / var.sqrt())))
}

fn runstest(a: &[Expr]) -> Res {
    let name = "runstest";
    let x = reals(&a[0], name)?;
    let (mut level, mut rises, mut way) = (None, false, None);
    for o in &a[1..] {
        match text_of(o).map(str::to_lowercase).as_deref() {
            Some("ud") => rises = true,
            Some("method") => {}
            Some(_) => way = Some(o),
            None => level = Some(real(o, name)?),
        }
    }
    let (runs, p) = if rises {
        let steps: Vec<bool> = x
            .windows(2)
            .filter(|w| w[0] != w[1])
            .map(|w| w[1] > w[0])
            .collect();
        if steps.is_empty() {
            return Err("runstest needs values that change".into());
        }
        let (runs, all) = (runs_of(&steps), steps.len() + 1);
        if counted(way, name, all <= 50)? {
            if all > 1000 {
                return Err("runstest counts the orders of 1000 values at most".into());
            }
            (runs, tails(&runs_up_down(all), runs))
        } else {
            let all = all as u32;
            let (mean, var) = (fl(2 * all - 1) / 3u32, fl(16 * all - 29) / 90u32);
            (runs, near(runs, mean, var))
        }
    } else {
        let level = level.unwrap_or_else(|| mean(&x));
        let sides: Vec<bool> = x
            .iter()
            .filter(|v| **v != level)
            .map(|v| *v > level)
            .collect();
        if sides.is_empty() {
            return Err("runstest needs values away from the level".into());
        }
        let above = sides.iter().filter(|s| **s).count();
        let (below, runs) = (sides.len() - above, runs_of(&sides));
        if counted(way, name, true)? {
            (runs, tails(&runs_about(above, below), runs))
        } else {
            let all = fl(sides.len() as u64);
            let twice = fl(above as u64) * below as u64 * 2u32;
            let mean = fl(&twice / &all) + 1u32;
            let var = fl(&twice * fl(&twice - &all)) / (fl(all.square_ref()) * (all - 1u32));
            (runs, near(runs, mean, var))
        }
    };
    Ok(p.second(vec![Expr::int(runs as i64)]))
}

// ---- the spread of two samples ----

fn ansari(x: &[Float], y: &[Float], way: Option<&Expr>) -> Res {
    let name = "ansaribradley";
    let (m, all) = (x.len(), x.len() + y.len());
    let pooled: Vec<&Float> = x.iter().chain(y).collect();
    let mut places: Vec<usize> = (0..all).collect();
    places.sort_by(|a, b| {
        pooled[*a]
            .partial_cmp(pooled[*b])
            .unwrap_or(Ordering::Equal)
    });
    // the places count from both ends; equal values share the mean of theirs
    let mut scores = vec![Rational::new(); all];
    let mut from = 0;
    while from < all {
        let mut to = from;
        while to + 1 < all && pooled[places[to + 1]] == pooled[places[from]] {
            to += 1;
        }
        let total: u64 = (from..=to).map(|i| (i + 1).min(all - i) as u64).sum();
        let share = Rational::from((total, (to - from + 1) as u64));
        for place in &places[from..=to] {
            scores[*place] = share.clone();
        }
        from = to + 1;
    }
    let sum = |part: &[Rational]| part.iter().fold(Rational::new(), |s, r| s + r);
    let seen = sum(&scores[..m]);
    let p = if counted(way, name, all <= 25)? {
        if all > 100 {
            return Err("ansaribradley counts the choices among 100 values at most".into());
        }
        let scale = scores
            .iter()
            .fold(Integer::from(1), |s, r| s.lcm(r.denom()));
        let whole = |r: &Rational| -> u64 {
            Rational::from(r * &scale)
                .numer()
                .to_u64()
                .unwrap_or(u64::MAX)
        };
        let numbers: Vec<u64> = scores.iter().map(whole).collect();
        if numbers.iter().sum::<u64>() > 4_000_000 {
            return Err("ansaribradley: too many shared places to count every choice".into());
        }
        let counts = sums_of_choices(&numbers, Some(m));
        tails(&counts, whole(&seen) as usize)
    } else {
        let middle = sum(&scores) / all as u64;
        let spread = scores.iter().fold(Rational::new(), |s, r| {
            let off = Rational::from(r - &middle);
            s + Rational::from(off.square_ref())
        });
        let var = spread * (m as u64) * ((all - m) as u64) / (all as u64) / (all as u64 - 1);
        let var = fl(&var);
        if !positive(&var) {
            Chance::Near(fl(1))
        } else {
            let off = Rational::from(&seen - &Rational::from(&middle * m as u64));
            Chance::Near(beyond(&(fl(&off) / var.sqrt())))
        }
    };
    Ok(p.second(vec![exact(seen)]))
}

// ---- the normal law with the mean and the spread taken from the data ----

fn normal(x: &Float) -> Float {
    (fl(-x) / fl(2).sqrt()).erfc() / 2u32
}

/// The distance of Lilliefors and its chance by the formulas of Dallal and Wilkinson (small
/// chances) and of Stephens (the others).
fn lilliefors(x: &[Float]) -> Res {
    let n = x.len();
    if n < 4 {
        return Err("lillietest needs four values or more".into());
    }
    let middle = mean(x);
    let spread = x.iter().fold(fl(0), |s, v| s + fl(v - &middle).square());
    if !positive(&spread) {
        return Err("lillietest needs values that differ".into());
    }
    let sd = (spread / (n as u32 - 1)).sqrt();
    let mut sorted = x.to_vec();
    sorted.sort_by(|p, q| p.partial_cmp(q).unwrap_or(Ordering::Equal));
    let shares: Vec<Float> = sorted
        .iter()
        .map(|v| normal(&(fl(v - &middle) / &sd)))
        .collect();
    let most = farthest(&shares);
    let count = fl(n as u32);
    let (kd, nd) = match n <= 100 {
        true => (most.clone(), count.clone()),
        false => {
            let grown = (fl(&count / 100u32).ln() * 0.49f64).exp();
            (fl(&most * &grown), fl(100))
        }
    };
    let more = fl(&nd + 2.78019f64);
    let power = fl(kd.square_ref()) * -7.01256f64 * &more + fl(&kd * 2.99587f64) * more.sqrt()
        - 0.122119f64
        + fl(0.974598f64) / fl(nd.sqrt_ref())
        + fl(1.67997f64) / &nd;
    let mut p = power.exp();
    if p > 0.1 {
        let root = fl(count.sqrt_ref());
        let kk = (fl(&root - 0.01f64) + fl(0.85f64) / &root) * &most;
        let curve = |c: [f64; 5]| -> Float { c.iter().rev().fold(fl(0), |s, k| s * &kk + *k) };
        p = if kk <= 0.302 {
            fl(1)
        } else if kk <= 0.5 {
            curve([2.76773, -19.828315, 80.709644, -138.55152, 81.218052])
        } else if kk <= 0.9 {
            curve([-4.901232, 40.662806, -97.490286, 94.029866, -32.355711])
        } else if kk <= 1.31 {
            curve([6.198765, -19.558097, 23.186922, -12.234627, 2.423045])
        } else {
            fl(0)
        };
    }
    let p = p.clamp(&fl(0), &fl(1));
    let small = Expr::int(i64::from(p < 0.05));
    let chance = match p.is_zero() {
        true => Expr::zero(),
        false => numeric::to_float_expr(Complex::with_val(p.prec(), &p), 4),
    };
    Ok(Expr::list(vec![small, chance, shown(&most)]))
}

// ---- the normal law of several variables ----

pub(crate) type Rule = Rc<Vec<(Float, Float)>>;

thread_local! {
    static RULES: RefCell<HashMap<(usize, u32), Rule>> = RefCell::new(HashMap::new());
}

/// The points and the weights of Gauss and Legendre on the stretch from 0 to 1.
pub(crate) fn rule(count: usize, prec: u32) -> Rule {
    if let Some(known) = RULES.with(|r| r.borrow().get(&(count, prec)).cloned()) {
        return known;
    }
    let pi = Float::with_val(64, Constant::Pi);
    let size = count as u32;
    let mut out = Vec::with_capacity(count);
    for k in 0..size {
        let angle = Float::with_val(64, &pi * (4 * k + 3)) / (4 * size + 2);
        let mut x = Float::with_val(prec, angle.cos());
        let mut slope = Float::new(prec);
        for _ in 0..100 {
            let (mut before, mut here) = (Float::with_val(prec, 1), x.clone());
            for j in 2..=size {
                let next = (Float::with_val(prec, &x * &here) * (2 * j - 1)
                    - Float::with_val(prec, &before * (j - 1)))
                    / j;
                before = here;
                here = next;
            }
            slope = (Float::with_val(prec, &x * &here) - &before) * size
                / (Float::with_val(prec, x.square_ref()) - 1u32);
            let step = Float::with_val(prec, &here / &slope);
            x -= &step;
            let quiet = match (step.get_exp(), x.get_exp()) {
                (Some(s), Some(e)) => i64::from(s) < i64::from(e) - i64::from(prec) + 4,
                _ => true,
            };
            if quiet {
                break;
            }
        }
        let weight = Float::with_val(prec, 2)
            / ((Float::with_val(prec, 1) - Float::with_val(prec, x.square_ref()))
                * Float::with_val(prec, slope.square_ref()));
        out.push(((x + 1u32) / 2u32, weight / 2u32));
    }
    let made = Rc::new(out);
    RULES.with(|r| r.borrow_mut().insert((count, prec), made.clone()));
    made
}

/// The integral from 0 to 1 to so many digits after the point.
fn integral(
    f: &mut dyn FnMut(&Float) -> Result<Float, String>,
    digits: u32,
) -> Result<Float, String> {
    let prec = bits();
    let target = i64::from(numeric::bits_for(digits));
    let mut last: Option<Float> = None;
    for count in [24usize, 48, 96, 192, 384] {
        let mut total = Float::new(prec);
        for (x, w) in rule(count, prec).iter() {
            total += f(x)? * w;
        }
        if let Some(old) = &last {
            let diff = fl(&total - old);
            if diff.get_exp().is_none_or(|e| i64::from(e) < -target) {
                return Ok(total);
            }
        }
        last = Some(total);
    }
    // a slope that grows without bound at an end: the rule of tanh and sinh
    let mut failed = None;
    let found = crate::quad::quad(
        &mut |t: &Float| match f(t) {
            Ok(v) => Some(Complex::with_val(prec, v)),
            Err(e) => {
                failed = Some(e);
                None
            }
        },
        &fl(0),
        &fl(1),
        digits,
    );
    match (found, failed) {
        (Some(v), _) => Ok(fl(v.real())),
        (None, Some(e)) => Err(e),
        (None, None) => Err("mvncdf: the integral does not settle".into()),
    }
}

fn part(r: &Mat, keep: &[usize]) -> Mat {
    keep.iter()
        .map(|i| keep.iter().map(|j| r[*i][*j].clone()).collect())
        .collect()
}

const NOT_DEFINITE: &str =
    "mvncdf needs a matrix of covariances that is symmetric and positive definite";

/// The chance that standard normal variables with the correlations r are all below h, by the
/// identity of Plackett: the slope along a correlation is a density of two of the variables
/// times the chance of the others once those two are known.
fn below(h: &[Float], r: &Mat, digits: u32) -> Result<Float, String> {
    if h.iter().any(|x| x.is_infinite() && x.is_sign_negative()) {
        return Ok(fl(0));
    }
    if let Some(free) = h.iter().position(|x| x.is_infinite()) {
        let keep: Vec<usize> = (0..h.len()).filter(|i| *i != free).collect();
        let rest: Vec<Float> = keep.iter().map(|i| h[*i].clone()).collect();
        return below(&rest, &part(r, &keep), digits);
    }
    let d = h.len();
    match d {
        0 => return Ok(fl(1)),
        1 => return Ok(normal(&h[0])),
        _ => {}
    }
    let rest: Vec<usize> = (1..d).collect();
    let first = normal(&h[0]) * below(&h[1..], &part(r, &rest), digits + 2)?;
    if r[0][1..].iter().all(Float::is_zero) {
        return Ok(first);
    }
    let two_pi = Float::with_val(bits(), Constant::Pi) * 2u32;
    let mut slope = |t: &Float| -> Result<Float, String> {
        let mut total = fl(0);
        for j in 1..d {
            if r[0][j].is_zero() {
                continue;
            }
            let rho = fl(t * &r[0][j]);
            let less = fl(1) - fl(rho.square_ref());
            if !positive(&less) {
                return Err(NOT_DEFINITE.into());
            }
            let power = (fl(h[0].square_ref()) + fl(h[j].square_ref())
                - fl(&rho * &h[0]) * &h[j] * 2u32)
                / fl(&less * 2u32);
            let density = fl(-power).exp() / (fl(less.sqrt_ref()) * &two_pi);
            let others: Vec<usize> = (1..d).filter(|k| *k != j).collect();
            let chance = if others.is_empty() {
                fl(1)
            } else {
                // what the others share with the two known ones, and the weights of those two
                let ties: Vec<[Float; 4]> = others
                    .iter()
                    .map(|k| {
                        let (a, b) = (fl(t * &r[0][*k]), r[j][*k].clone());
                        let w0 = (fl(&a) - fl(&rho * &b)) / &less;
                        let wj = (fl(&b) - fl(&rho * &a)) / &less;
                        [a, b, w0, wj]
                    })
                    .collect();
                let cov: Mat = others
                    .iter()
                    .zip(&ties)
                    .map(|(k, [_, _, w0, wj])| {
                        others
                            .iter()
                            .zip(&ties)
                            .map(|(l, [a, b, _, _])| fl(&r[*k][*l]) - fl(w0 * a) - fl(wj * b))
                            .collect()
                    })
                    .collect();
                let mut sds = Vec::with_capacity(others.len());
                for (i, row) in cov.iter().enumerate() {
                    if !positive(&row[i]) {
                        return Err(NOT_DEFINITE.into());
                    }
                    sds.push(fl(row[i].sqrt_ref()));
                }
                let limits: Vec<Float> = others
                    .iter()
                    .zip(&ties)
                    .zip(&sds)
                    .map(|((k, [_, _, w0, wj]), sd)| {
                        (fl(&h[*k]) - fl(w0 * &h[0]) - fl(wj * &h[j])) / sd
                    })
                    .collect();
                let links: Mat = cov
                    .iter()
                    .enumerate()
                    .map(|(x, row)| {
                        row.iter()
                            .enumerate()
                            .map(|(y, c)| match x == y {
                                true => fl(1),
                                false => fl(c / &sds[x]) / &sds[y],
                            })
                            .collect()
                    })
                    .collect();
                below(&limits, &links, digits + 2)?
            };
            total += fl(&r[0][j] * &density) * chance;
        }
        Ok(total)
    };
    Ok(first + integral(&mut slope, digits)?)
}

/// The standard normal value below which the share u of all values lies: the rational
/// start of Acklam, then two steps of Halley.
fn normal_inverse(u: &Float) -> Float {
    const A: [f64; 6] = [
        -3.969683028665376e+01,
        2.209460984245205e+02,
        -2.759285104469687e+02,
        1.38357751867269e+02,
        -3.066479806614716e+01,
        2.506628277459239e+00,
    ];
    const B: [f64; 6] = [
        -5.447609879822406e+01,
        1.615858368580409e+02,
        -1.556989798598866e+02,
        6.680131188771972e+01,
        -1.328068155288572e+01,
        1.0,
    ];
    const C: [f64; 6] = [
        -7.784894002430293e-03,
        -3.223964580411365e-01,
        -2.400758277161838e+00,
        -2.549732539343734e+00,
        4.374664141464968e+00,
        2.938163982698783e+00,
    ];
    const D: [f64; 5] = [
        7.784695709041462e-03,
        3.224671290700398e-01,
        2.445134137142996e+00,
        3.754408661907416e+00,
        1.0,
    ];
    let p = u.prec();
    // (the upper half by the lower one: a share near 1 is known by what it lacks)
    if *u > 0.5 {
        return -normal_inverse(&Float::with_val(p, 1u32 - u));
    }
    let curve = |c: &[f64], x: &Float| c.iter().fold(Float::new(p), |s, k| s * x + *k);
    let tail = |share: Float| -> Float {
        let q = (share.ln() * -2i32).sqrt();
        curve(&C, &q) / curve(&D, &q)
    };
    let mut x = if *u < 0.02425 {
        tail(u.clone())
    } else {
        let q = Float::with_val(p, u - 0.5f64);
        let r = Float::with_val(p, q.square_ref());
        curve(&A, &r) * q / curve(&B, &r)
    };
    let root_two = Float::with_val(p, 2).sqrt();
    let root_two_pi = (Float::with_val(p, Constant::Pi) * 2u32).sqrt();
    for _ in 0..2 {
        let share = (Float::with_val(p, -&x) / &root_two).erfc() / 2u32;
        let off = (share - u) * &root_two_pi * (Float::with_val(p, x.square_ref()) / 2u32).exp();
        let bend = Float::with_val(p, &x * &off) / 2u32 + 1u32;
        x -= off / bend;
    }
    x
}

/// The steps of a lattice of a prime number of points for so many variables, one variable
/// after another: each step is the one with which smooth periodic functions are summed
/// best (the kernel of the second polynomial of Bernoulli).
fn lattice_steps(points: usize, variables: usize) -> Vec<usize> {
    let count = points as f64;
    let kernel: Vec<f64> = (0..points)
        .map(|t| {
            let x = t as f64 / count;
            2.0 * 9.869_604_401_089_358 * (x * x - x + 1.0 / 6.0)
        })
        .collect();
    let mut weights: Vec<f64> = kernel.iter().map(|k| 1.0 + k).collect();
    let mut steps = vec![1usize];
    while steps.len() < variables {
        let (mut best, mut least) = (1, f64::INFINITY);
        for step in 1..=points / 2 {
            let (mut at, mut sum) = (0, 0.0);
            for w in &weights {
                sum += w * kernel[at];
                at += step;
                if at >= points {
                    at -= points;
                }
            }
            if sum < least {
                (best, least) = (step, sum);
            }
        }
        let mut at = 0;
        for w in weights.iter_mut() {
            *w *= 1.0 + kernel[at];
            at += best;
            if at >= points {
                at -= points;
            }
        }
        steps.push(best);
    }
    steps
}

/// The chance of a box by lattices of points, shifted and folded, as Genz takes the
/// variables one after another; the value and how far eight shifted lattices disagree.
fn lattice(lo: &[Float], hi: &[Float], r: &Mat) -> Result<(Float, Float), String> {
    const PRIMES: [u32; 25] = [
        2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89,
        97,
    ];
    let p = 64u32;
    let d = lo.len();
    let c = cholesky(r).ok_or(NOT_DEFINITE)?;
    let root_two = Float::with_val(p, 2).sqrt();
    let share = |x: &Float| -> Float { (Float::with_val(p, -x) / &root_two).erfc() / 2u32 };
    let edge = Float::with_val(p, 1) >> 60u32;
    let far_edge = Float::with_val(p, 1u32 - &edge);
    let roots: Vec<Float> = PRIMES
        .iter()
        .map(|q| Float::with_val(p, *q).sqrt().fract())
        .collect();
    let first = (
        share(&Float::with_val(p, &lo[0] / &c[0][0])),
        share(&Float::with_val(p, &hi[0] / &c[0][0])),
    );
    let shifts = 8u32;
    let (mut middle, mut error) = (Float::new(p), Float::new(p));
    for points in [1009u32, 4001] {
        let steps = lattice_steps(points as usize, d - 1);
        let mut found = Vec::with_capacity(shifts as usize);
        for shift in 1..=shifts {
            let mut total = Float::new(p);
            for n in 0..points {
                let (mut low, mut high) = first.clone();
                let mut f = Float::with_val(p, &high - &low);
                let mut y: Vec<Float> = Vec::with_capacity(d);
                for i in 1..d {
                    let place = (u64::from(n) * steps[i - 1] as u64 % u64::from(points)) as u32;
                    let moved = Float::with_val(p, place) / points
                        + Float::with_val(p, &roots[i - 1] * shift);
                    let folded = (moved.fract() * 2u32 - 1u32).abs();
                    let width = Float::with_val(p, &high - &low);
                    let at = folded * width + &low;
                    y.push(normal_inverse(&Float::with_val(
                        p,
                        at.clamp_ref(&edge, &far_edge),
                    )));
                    let known = c[i]
                        .iter()
                        .zip(&y)
                        .fold(Float::new(p), |s, (k, v)| s + Float::with_val(p, k * v));
                    low = share(&(Float::with_val(p, &lo[i] - &known) / &c[i][i]));
                    high = share(&(Float::with_val(p, &hi[i] - &known) / &c[i][i]));
                    f *= Float::with_val(p, &high - &low);
                }
                total += f;
            }
            found.push(total / points);
        }
        middle = found.iter().fold(Float::new(p), |s, v| s + v) / shifts;
        let spread = found.iter().fold(Float::new(p), |s, v| {
            s + Float::with_val(p, v - &middle).square()
        });
        error = (spread / (shifts * (shifts - 1))).sqrt();
        if error < 1e-7 {
            break;
        }
    }
    Ok((middle, error))
}

fn bound(e: &Expr, name: &str) -> Result<Float, String> {
    if *e == Expr::inf() {
        return Ok(Float::with_val(bits(), Special::Infinity));
    }
    if *e == neg(Expr::inf()) {
        return Ok(Float::with_val(bits(), Special::NegInfinity));
    }
    real(e, name)
}

/// The chance of a box of standard variables, with as many digits as are known of it.
fn box_chance(lo: &[Float], hi: &[Float], r: &Mat) -> Res {
    if lo.iter().zip(hi).any(|(l, h)| l >= h) {
        return Ok(Expr::zero());
    }
    // a variable that may be anything leaves the others as they are
    let bound_ones: Vec<usize> = (0..lo.len())
        .filter(|i| !(lo[*i].is_infinite() && hi[*i].is_infinite()))
        .collect();
    let pick = |v: &[Float]| -> Vec<Float> { bound_ones.iter().map(|i| v[*i].clone()).collect() };
    let (lo, hi, r) = (pick(lo), pick(hi), part(r, &bound_ones));
    let (d, digits) = (lo.len(), numeric::digits());
    if d == 0 {
        return Ok(Expr::one());
    }
    if d == 1 {
        return Ok(shown(&(normal(&hi[0]) - normal(&lo[0]))));
    }
    let few = |value: &Float, known: i64| -> Expr {
        if !positive(value) {
            return Expr::zero();
        }
        let lead = i64::from(value.get_exp().unwrap_or(0)) * 30103 / 100_000;
        match (known + lead.min(0)).clamp(0, i64::from(digits)) {
            0 => Expr::zero(),
            k => numeric::to_float_expr(Complex::with_val(value.prec(), value), k as u32),
        }
    };
    if d > 4 {
        if d > 25 {
            return Err("mvncdf takes 25 variables at most".into());
        }
        let (value, error) = lattice(&lo, &hi, &r)?;
        let value = value.clamp(&Float::new(64), &Float::with_val(64, 1));
        // three times the disagreement of the lattices bounds the digits that show
        let doubt = Float::with_val(64, &error * 3u32).max(&(Float::with_val(64, 1) >> 40u32));
        let known = -i64::from(doubt.get_exp().unwrap_or(0)) * 30103 / 100_000;
        return Ok(few(&value, known.clamp(2, 10)));
    }
    let free: Vec<usize> = (0..d).filter(|i| !lo[*i].is_infinite()).collect();
    let mut total = fl(0);
    for mask in 0u32..1 << free.len() {
        let mut corner = hi.clone();
        for (bit, i) in free.iter().enumerate() {
            if mask >> bit & 1 == 1 {
                corner[*i] = lo[*i].clone();
            }
        }
        let value = below(&corner, &r, digits + 4)?;
        match mask.count_ones() % 2 {
            0 => total += value,
            _ => total -= value,
        }
    }
    Ok(few(&total.min(&fl(1)), i64::from(digits)))
}

fn mvncdf(a: &[Expr]) -> Res {
    let name = "mvncdf";
    let usage = "mvncdf(x), mvncdf(x, mu), mvncdf(x, mu, sigma) or mvncdf(lower, upper, mu, sigma): points in lists or in the rows of a matrix";
    let (lows, highs, mu, sigma) = match a.len() {
        1 => (None, &a[0], None, None),
        2 => (None, &a[0], Some(&a[1]), None),
        3 => (None, &a[0], Some(&a[1]), Some(&a[2])),
        _ => (Some(&a[0]), &a[1], Some(&a[2]), Some(&a[3])),
    };
    let rows = |e: &Expr| -> Result<(Vec<Vec<Float>>, bool), String> {
        let items = e.as_list().filter(|l| !l.is_empty()).ok_or(usage)?;
        let many = items.iter().all(|x| x.as_list().is_some());
        let lists: Vec<&[Expr]> = match many {
            true => items.iter().filter_map(Expr::as_list).collect(),
            false => vec![items],
        };
        let found = lists
            .iter()
            .map(|l| l.iter().map(|x| bound(x, name)).collect())
            .collect::<Result<Vec<Vec<Float>>, String>>()?;
        Ok((found, many))
    };
    let (highs, many) = rows(highs)?;
    let d = highs[0].len();
    if d == 0 || highs.iter().any(|h| h.len() != d) {
        return Err(usage.into());
    }
    let lows = match lows {
        Some(l) => {
            let (lows, _) = rows(l)?;
            if lows.len() != highs.len() || lows.iter().any(|l| l.len() != d) {
                return Err("mvncdf: the lower and the upper corners are lists of one size".into());
            }
            lows
        }
        None => vec![vec![Float::with_val(bits(), Special::NegInfinity); d]; highs.len()],
    };
    let middle: Vec<Float> = match mu.and_then(Expr::as_list) {
        Some([]) => vec![fl(0); d],
        Some(items) if items.len() == d => items
            .iter()
            .map(|x| real(x, name))
            .collect::<Result<_, _>>()?,
        Some(_) => return Err(format!("mvncdf: the means are a list of {d} numbers")),
        None => match mu {
            Some(one) => vec![real(one, name)?; d],
            None => vec![fl(0); d],
        },
    };
    let cov: Mat = match sigma {
        None => Vec::new(),
        Some(s) => match s.as_list() {
            Some(items) if items.len() == d && items.iter().all(|x| x.as_list().is_none()) => {
                let spreads = reals(s, name)?;
                (0..d)
                    .map(|i| {
                        (0..d)
                            .map(|j| match i == j {
                                true => spreads[i].clone(),
                                false => fl(0),
                            })
                            .collect()
                    })
                    .collect()
            }
            _ => table(s, name)?,
        },
    };
    let (sds, r): (Vec<Float>, Mat) = match cov.is_empty() {
        true => (
            vec![fl(1); d],
            (0..d)
                .map(|i| (0..d).map(|j| fl(u32::from(i == j))).collect())
                .collect(),
        ),
        false => {
            if cov.len() != d || cov.iter().any(|row| row.len() != d) {
                return Err(format!(
                    "mvncdf: the covariances are a matrix of {d} rows and columns"
                ));
            }
            let even = (0..d).all(|i| (0..i).all(|j| cov[i][j] == cov[j][i]));
            if !even || cholesky(&cov).is_none() {
                return Err(NOT_DEFINITE.into());
            }
            let sds: Vec<Float> = (0..d).map(|i| fl(cov[i][i].sqrt_ref())).collect();
            let r = (0..d)
                .map(|i| {
                    (0..d)
                        .map(|j| match i == j {
                            true => fl(1),
                            false => fl(&cov[i][j] / &sds[i]) / &sds[j],
                        })
                        .collect()
                })
                .collect();
            (sds, r)
        }
    };
    let standard = |v: &[Float]| -> Vec<Float> {
        v.iter()
            .zip(&middle)
            .zip(&sds)
            .map(|((x, m), s)| match x.is_infinite() {
                true => x.clone(),
                false => fl(x - m) / s,
            })
            .collect()
    };
    let mut found = Vec::with_capacity(highs.len());
    for (l, h) in lows.iter().zip(&highs) {
        found.push(box_chance(&standard(l), &standard(h), &r)?);
    }
    Ok(match many {
        true => Expr::list(found),
        false => found.swap_remove(0),
    })
}

// ---- points from their distances, shapes laid onto one another ----

fn cmdscale(e: &Expr) -> Res {
    let name = "cmdscale";
    let usage =
        "cmdscale needs the distances of points: a symmetric matrix or the list that pdist gives";
    let flat = e
        .as_list()
        .is_some_and(|l| !l.is_empty() && l.iter().all(|x| x.as_list().is_none()));
    let mut apart: Mat = match flat {
        true => {
            let v = reals(e, name)?;
            let count = (2..=v.len() + 1)
                .find(|k| k * (k - 1) / 2 == v.len())
                .ok_or(usage)?;
            let mut m = vec![vec![fl(0); count]; count];
            let places = (0..count).flat_map(|i| (i + 1..count).map(move |j| (i, j)));
            for ((i, j), d) in places.zip(&v) {
                m[i][j] = d.clone();
                m[j][i] = d.clone();
            }
            m
        }
        false => table(e, name)?,
    };
    let n = apart.len();
    let even = apart.iter().all(|r| r.len() == n)
        && (0..n).all(|i| (0..i).all(|j| apart[i][j] == apart[j][i]));
    if !even {
        return Err(usage.into());
    }
    if (0..n).all(|i| apart[i][i] == 1u32) {
        // likenesses with ones on the diagonal become distances
        for x in apart.iter_mut().flatten() {
            let d = fl(1) - &*x;
            if d.is_sign_negative() && !d.is_zero() {
                return Err("cmdscale: likenesses are 1 at most".into());
            }
            *x = d.sqrt();
        }
    } else if (0..n).any(|i| !apart[i][i].is_zero()) {
        return Err(usage.into());
    }
    let squared: Mat = apart
        .iter()
        .map(|r| r.iter().map(|x| fl(x.square_ref())).collect())
        .collect();
    let count = n as u32;
    let of_rows: Vec<Float> = squared
        .iter()
        .map(|r| r.iter().fold(fl(0), |s, x| s + x) / count)
        .collect();
    let all = of_rows.iter().fold(fl(0), |s, x| s + x) / count;
    let centre: Mat = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| (fl(&squared[i][j]) - &of_rows[i] - &of_rows[j] + &all) / -2i32)
                .collect()
        })
        .collect();
    let (values, vectors) = eigen(&centre);
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|a, b| {
        values[*b]
            .partial_cmp(&values[*a])
            .unwrap_or(Ordering::Equal)
    });
    let top = values.iter().fold(fl(0), |s, x| s.max(&fl(x.abs_ref())));
    let small = fl(&top * (fl(1) >> (bits() / 2)));
    let kept: Vec<usize> = order
        .iter()
        .copied()
        .filter(|k| values[*k] > small)
        .collect();
    let mut y: Mat = match kept.is_empty() {
        true => vec![vec![fl(0)]; n],
        false => vectors
            .iter()
            .map(|row| {
                kept.iter()
                    .map(|k| fl(&row[*k] * fl(values[*k].sqrt_ref())))
                    .collect()
            })
            .collect(),
    };
    for j in 0..y[0].len() {
        let mut big = fl(0);
        for row in &y {
            if fl(row[j].abs_ref()) > fl(big.abs_ref()) {
                big = row[j].clone();
            }
        }
        if big.is_sign_negative() {
            for row in y.iter_mut() {
                row[j] = fl(-&row[j]);
            }
        }
    }
    let sorted: Vec<Float> = order.iter().map(|k| values[*k].clone()).collect();
    Ok(Expr::list(vec![out(&y), out_list(&sorted)]))
}

fn procrustes(a: &[Expr]) -> Res {
    let name = "procrustes";
    let usage = "procrustes(X, Y[, \"scaling\", false][, \"reflection\", true | false | \"best\"]): two matrices with a point in every row";
    let (x, y) = (points(&a[0], name)?, points(&a[1], name)?);
    let (n, m, mine) = (x.len(), x[0].len(), y[0].len());
    if y.len() != n || mine > m || !a.len().is_multiple_of(2) {
        return Err(usage.into());
    }
    let (mut scaling, mut mirror) = (true, None);
    for pair in a[2..].chunks(2) {
        match crate::arrays::word(&pair[0]).as_deref() {
            Some("scaling") => scaling = crate::arrays::truthy(&pair[1])?,
            Some("reflection") => {
                mirror = match crate::arrays::word(&pair[1]).as_deref() {
                    Some("best") => None,
                    _ => Some(crate::arrays::truthy(&pair[1])?),
                }
            }
            _ => return Err(usage.into()),
        }
    }
    let (x0, mid_x) = centred(&x);
    let (y0, mid_y) = centred(&y);
    let (size_x, size_y) = (squares(&x0), squares(&y0));
    if !positive(&size_x) {
        return Err("procrustes needs points of the first shape that differ".into());
    }
    let all_at = |row: &[Float]| -> Mat { vec![row.to_vec(); n] };
    if !positive(&size_y) {
        let turn: Mat = (0..mine)
            .map(|i| (0..m).map(|j| fl(u32::from(i == j))).collect())
            .collect();
        let shape = crate::records::record(vec![
            ("T", out(&turn)),
            ("b", Expr::zero()),
            ("c", out(&all_at(&mid_x))),
        ]);
        return Ok(Expr::list(vec![Expr::one(), out(&all_at(&mid_x)), shape]));
    }
    let (norm_x, norm_y) = (fl(size_x.sqrt_ref()), fl(size_y.sqrt_ref()));
    let unit = |rows: &Mat, norm: &Float| -> Mat {
        rows.iter()
            .map(|r| {
                let mut row: Vec<Float> = r.iter().map(|v| fl(v / norm)).collect();
                row.resize(m, fl(0));
                row
            })
            .collect()
    };
    let (xs, ys) = (unit(&x0, &norm_x), unit(&y0, &norm_y));
    let (u, mut s, v) = decomp::svd(&wide(&product(&turned(&xs), &ys)))?;
    let (u, mut v) = (narrow(&u), narrow(&v));
    let mut turn = product(&v, &turned(&u));
    if let Some(wanted) = mirror
        && (determinant(&turn).is_sign_negative() != wanted)
    {
        for row in v.iter_mut() {
            row[m - 1] = fl(-&row[m - 1]);
        }
        s[m - 1] = -s[m - 1].clone();
        turn = product(&v, &turned(&u));
    }
    let trace = s.iter().fold(fl(0), |t, x| t + x);
    let laid = product(&ys, &turn);
    let (b, d, grow) = match scaling {
        true => (
            fl(&trace * &norm_x) / &norm_y,
            fl(1) - fl(trace.square_ref()),
            fl(&norm_x * &trace),
        ),
        false => (
            fl(1),
            fl(1) + fl(&size_y / &size_x) - fl(&trace * 2u32) * &norm_y / &norm_x,
            norm_y.clone(),
        ),
    };
    let z: Mat = laid
        .iter()
        .map(|r| {
            r.iter()
                .zip(&mid_x)
                .map(|(v, c)| fl(v * &grow) + c)
                .collect()
        })
        .collect();
    turn.truncate(mine);
    let moved = product(&vec![mid_y.clone()], &turn);
    let shift: Vec<Float> = mid_x
        .iter()
        .zip(&moved[0])
        .map(|(c, t)| fl(c) - fl(&b * t))
        .collect();
    let shape = crate::records::record(vec![
        ("T", out(&turn)),
        (
            "b",
            match scaling {
                true => shown(&b),
                false => Expr::one(),
            },
        ),
        ("c", out(&all_at(&shift))),
    ]);
    Ok(Expr::list(vec![out_beside(&d, &fl(1)), out(&z), shape]))
}

fn canoncorr(a: &[Expr]) -> Res {
    let name = "canoncorr";
    let (x, y) = (points(&a[0], name)?, points(&a[1], name)?);
    let n = x.len();
    let depend = "canoncorr: the columns depend on one another, or the rows are too few";
    if y.len() != n {
        return Err("canoncorr(X, Y): two matrices with a row for every observation".into());
    }
    let ((x0, _), (y0, _)) = (centred(&x), centred(&y));
    let (p, q) = (x0[0].len(), y0[0].len());
    if n <= p.max(q) {
        return Err(depend.into());
    }
    let thin = |m: &Mat| -> Result<(Mat, Mat), String> {
        let cols = m[0].len();
        let (q, r) = pencils::qr(&wide(m));
        let q: Mat = narrow(&q)
            .into_iter()
            .map(|row| row[..cols].to_vec())
            .collect();
        let mut r = narrow(&r);
        r.truncate(cols);
        let top = r
            .iter()
            .flatten()
            .fold(fl(0), |s, x| s.max(&fl(x.abs_ref())));
        let small = fl(&top * (fl(1) >> (bits() / 2)));
        match r
            .iter()
            .enumerate()
            .all(|(k, row)| fl(row[k].abs_ref()) > small)
        {
            true => Ok((q, r)),
            false => Err(depend.into()),
        }
    };
    let ((qx, rx), (qy, ry)) = (thin(&x0)?, thin(&y0)?);
    let (u, s, v) = decomp::svd(&wide(&product(&turned(&qx), &qy)))?;
    let d = p.min(q);
    let root = fl(n as u32 - 1).sqrt();
    let weights = |r: &Mat, turns: &CMat| -> Result<Mat, String> {
        let first: CMat = turns.iter().map(|row| row[..d].to_vec()).collect();
        let found = pencils::solved(&wide(r), &first).ok_or(depend)?;
        Ok(narrow(&found)
            .into_iter()
            .map(|row| row.into_iter().map(|v| v * &root).collect())
            .collect())
    };
    let (mut wx, mut wy) = (weights(&rx, &u)?, weights(&ry, &v)?);
    for j in 0..d {
        let mut big = fl(0);
        for row in &wx {
            if fl(row[j].abs_ref()) > fl(big.abs_ref()) {
                big = row[j].clone();
            }
        }
        if big.is_sign_negative() {
            for row in wx.iter_mut().chain(wy.iter_mut()) {
                row[j] = fl(-&row[j]);
            }
        }
    }
    let r: Vec<Float> = s[..d]
        .iter()
        .map(|v| v.clone().clamp(&fl(0), &fl(1)))
        .collect();
    let (ux, vy) = (product(&x0, &wx), product(&y0, &wy));
    Ok(Expr::list(vec![
        out(&wx),
        out(&wy),
        out_list(&r),
        out(&ux),
        out(&vy),
    ]))
}

// ---- factors without signs ----

fn nnmf(e: &Expr, rank: &Expr) -> Res {
    let name = "nnmf";
    let a = table(e, name)?;
    let (m, n) = (a.len(), a[0].len());
    if a.iter().any(|r| r.len() != n) || a.iter().flatten().any(Float::is_sign_negative) {
        return Err("nnmf needs a matrix of numbers that are zero or more".into());
    }
    let k = crate::arrays::whole(rank, "the number of factors", 1, m.min(n) as i64)? as usize;
    let size = squares(&a).sqrt();
    if size.is_zero() {
        return Ok(Expr::list(vec![
            out(&vec![vec![fl(0); k]; m]),
            out(&vec![vec![fl(0); n]; k]),
            Expr::zero(),
        ]));
    }
    // the start: the parts of one sign of the leading singular pairs
    let (u, s, v) = decomp::svd(&wide(&a))?;
    let (u, v) = (narrow(&u), narrow(&v));
    let mut w: Mat = vec![vec![fl(0); k]; m];
    let mut h: Mat = vec![vec![fl(0); n]; k];
    let length = |v: &[Float]| v.iter().fold(fl(0), |s, x| s + fl(x.square_ref())).sqrt();
    for j in 0..k {
        let (x, y): (Vec<Float>, Vec<Float>) = (
            u.iter().map(|r| r[j].clone()).collect(),
            v.iter().map(|r| r[j].clone()).collect(),
        );
        let side = |v: &[Float], plus: bool| -> Vec<Float> {
            v.iter()
                .map(|t| match t.is_sign_positive() == plus {
                    true => fl(t.abs_ref()),
                    false => fl(0),
                })
                .collect()
        };
        let (xp, xn, yp, yn) = (
            side(&x, true),
            side(&x, false),
            side(&y, true),
            side(&y, false),
        );
        let (plus, minus) = (length(&xp) * length(&yp), length(&xn) * length(&yn));
        let (left, right, weight) = match plus >= minus {
            true => (xp, yp, plus),
            false => (xn, yn, minus),
        };
        let (long_left, long_right) = (length(&left), length(&right));
        if !positive(&weight) {
            continue;
        }
        let root = fl(&s[j] * &weight).sqrt();
        for (row, t) in w.iter_mut().zip(&left) {
            row[j] = fl(t / &long_left) * &root;
        }
        for (cell, t) in h[j].iter_mut().zip(&right) {
            *cell = fl(t / &long_right) * &root;
        }
    }
    let misfit = |w: &Mat, h: &Mat| -> Float {
        let fit = product(w, h);
        a.iter()
            .flatten()
            .zip(fit.iter().flatten())
            .fold(fl(0), |s, (x, y)| s + fl(x - y).square())
            .sqrt()
    };
    let done = fl(&size * fl(Float::i_pow_u(10, numeric::digits() + 4)).recip());
    let mut last = misfit(&w, &h);
    for _ in 0..3000 {
        // one factor at a time, the others held (least squares cut off at zero)
        for j in 0..k {
            let column: Vec<Float> = w.iter().map(|r| r[j].clone()).collect();
            let own = column.iter().fold(fl(0), |s, x| s + fl(x.square_ref()));
            if positive(&own) {
                let with_others: Vec<Float> = (0..k)
                    .map(|l| {
                        w.iter()
                            .zip(&column)
                            .fold(fl(0), |s, (r, c)| s + fl(&r[l] * c))
                    })
                    .collect();
                for t in 0..n {
                    let gain = a
                        .iter()
                        .zip(&column)
                        .fold(fl(0), |s, (r, c)| s + fl(&r[t] * c));
                    let held = (0..k).fold(fl(0), |s, l| s + fl(&with_others[l] * &h[l][t]));
                    let next = fl(&h[j][t]) + fl(&gain - &held) / &own;
                    h[j][t] = next.max(&fl(0));
                }
            }
            let row = h[j].clone();
            let own = row.iter().fold(fl(0), |s, x| s + fl(x.square_ref()));
            if positive(&own) {
                let with_others: Vec<Float> = (0..k)
                    .map(|l| h[l].iter().zip(&row).fold(fl(0), |s, (x, c)| s + fl(x * c)))
                    .collect();
                for (line, data) in w.iter_mut().zip(&a) {
                    let gain = data.iter().zip(&row).fold(fl(0), |s, (x, c)| s + fl(x * c));
                    let held = (0..k).fold(fl(0), |s, l| s + fl(&with_others[l] * &line[l]));
                    let next = fl(&line[j]) + fl(&gain - &held) / &own;
                    line[j] = next.max(&fl(0));
                }
            }
        }
        let now = misfit(&w, &h);
        let gained = fl(&last - &now);
        let settled = now <= done || gained <= fl(&now * (fl(1) >> (bits() - 16)));
        last = now;
        if settled {
            break;
        }
    }
    // rows of H of length one, the columns of W from the longest
    for j in 0..k {
        let long = length(&h[j]);
        if positive(&long) {
            for cell in h[j].iter_mut() {
                *cell /= &long;
            }
            for row in w.iter_mut() {
                row[j] *= &long;
            }
        }
    }
    let longs: Vec<Float> = (0..k)
        .map(|j| length(&w.iter().map(|r| r[j].clone()).collect::<Vec<_>>()))
        .collect();
    let mut order: Vec<usize> = (0..k).collect();
    order.sort_by(|a, b| longs[*b].partial_cmp(&longs[*a]).unwrap_or(Ordering::Equal));
    let w: Mat = w
        .iter()
        .map(|r| order.iter().map(|j| r[*j].clone()).collect())
        .collect();
    let h: Mat = order.iter().map(|j| h[*j].clone()).collect();
    let rest = fl(&last / fl((m * n) as u32).sqrt());
    Ok(Expr::list(vec![out(&w), out(&h), out_beside(&rest, &size)]))
}

// ---- laws fitted to data ----

fn data<'a>(e: &'a Expr, name: &str) -> Result<&'a [Expr], String> {
    e.as_list()
        .filter(|v| !v.is_empty() && v.iter().all(|x| x.as_list().is_none()))
        .ok_or_else(|| format!("{name} needs a list of numbers"))
}

fn above_zero(e: &Expr, name: &str) -> Result<Vec<Float>, String> {
    let x = reals(e, name)?;
    match x.iter().all(positive) {
        true => Ok(x),
        false => Err(format!("{name} needs numbers above zero")),
    }
}

fn logs(x: &[Float]) -> Vec<Float> {
    x.iter().map(|v| fl(v.ln_ref())).collect()
}

fn trigamma(x: &Float) -> Result<Float, String> {
    let p = bits();
    let args = [Complex::with_val(p, 1), Complex::with_val(p, x)];
    match crate::special::call("polygamma", &args, p) {
        Some(Some(v)) => Ok(fl(v.real())),
        _ => Err("the slope of digamma is not known there".into()),
    }
}

fn digamma(x: &Float) -> Float {
    fl(x.digamma_ref())
}

fn settled(step: &Float, at: &Float) -> bool {
    match (step.get_exp(), at.get_exp()) {
        (Some(s), Some(e)) => i64::from(s) < i64::from(e) - i64::from(bits()) + 24,
        _ => true,
    }
}

/// The shape and the scale of the gamma law that makes the data likeliest.
fn gamma_fit(x: &[Float], name: &str) -> Result<(Float, Float), String> {
    let middle = mean(x);
    let s = fl(middle.ln_ref()) - mean(&logs(x));
    if !positive(&s) {
        return Err(format!("{name} needs values that differ"));
    }
    let lean = fl(&s - 3u32);
    let mut a = (fl(3u32 - &s) + (fl(lean.square_ref()) + fl(&s * 24u32)).sqrt()) / fl(&s * 12u32);
    for _ in 0..200 {
        let off = fl(a.ln_ref()) - digamma(&a) - &s;
        let slope = fl(a.recip_ref()) - trigamma(&a)?;
        let step = off / slope;
        let mut next = fl(&a - &step);
        if !positive(&next) {
            next = fl(&a / 2u32);
        }
        let quiet = settled(&step, &a);
        a = next;
        if quiet {
            break;
        }
    }
    let scale = middle / &a;
    Ok((a, scale))
}

/// The scale and the shape of the law of Weibull that makes the data likeliest.
fn weibull_fit(x: &[Float], name: &str) -> Result<(Float, Float), String> {
    let top = x.iter().fold(fl(0), |s, v| s.max(v));
    let l: Vec<Float> = x.iter().map(|v| fl(v / &top).ln()).collect();
    let middle = mean(&l);
    let spread = l.iter().fold(fl(0), |s, v| s + fl(v - &middle).square());
    if !positive(&spread) {
        return Err(format!("{name} needs values that differ"));
    }
    let count = x.len() as u32;
    let mut b = fl(1.2825f64) / (spread / (count - 1).max(1)).sqrt();
    let sums = |b: &Float| -> [Float; 3] {
        let mut out = [fl(0), fl(0), fl(0)];
        for v in &l {
            let power = fl(v * b).exp();
            out[0] += &power;
            out[1] += fl(&power * v);
            out[2] += fl(&power * v) * v;
        }
        out
    };
    for _ in 0..500 {
        let [s0, s1, s2] = sums(&b);
        let ratio = fl(&s1 / &s0);
        let off = fl(&ratio - fl(b.recip_ref())) - &middle;
        let slope = fl(&s2 / &s0) - fl(ratio.square_ref()) + fl(b.square_ref()).recip();
        let step = off / slope;
        let mut next = fl(&b - &step);
        if !positive(&next) {
            next = fl(&b / 2u32);
        }
        let quiet = settled(&step, &b);
        b = next;
        if quiet {
            break;
        }
    }
    let [s0, _, _] = sums(&b);
    let scale = top * (fl(s0 / count).ln() / &b).exp();
    Ok((scale, b))
}

/// The two shapes of the beta law that make the data likeliest.
fn beta_fit(x: &[Float], name: &str) -> Result<(Float, Float), String> {
    if x.iter().any(|v| !positive(v) || *v >= 1u32) {
        return Err(format!("{name} needs numbers between 0 and 1"));
    }
    let middle = mean(x);
    let count = x.len() as u32;
    let var = x.iter().fold(fl(0), |s, v| s + fl(v - &middle).square()) / count;
    if !positive(&var) {
        return Err(format!("{name} needs values that differ"));
    }
    let first = mean(&logs(x));
    let second = mean(&x.iter().map(|v| fl(1u32 - v).ln()).collect::<Vec<_>>());
    let less = fl(1u32 - &middle);
    let mut t = fl(&middle * &less) / &var - 1u32;
    if !positive(&t) {
        t = fl(1);
    }
    let (mut a, mut b) = (fl(&middle * &t), fl(&less * &t));
    for _ in 0..500 {
        let both = fl(&a + &b);
        let (f1, f2) = (
            digamma(&a) - digamma(&both) - &first,
            digamma(&b) - digamma(&both) - &second,
        );
        let shared = trigamma(&both)?;
        let (j11, j22) = (trigamma(&a)? - &shared, trigamma(&b)? - &shared);
        let j12 = fl(-&shared);
        let det = fl(&j11 * &j22) - fl(j12.square_ref());
        let mut da = (fl(&f1 * &j22) - fl(&f2 * &j12)) / &det;
        let mut db = (fl(&f2 * &j11) - fl(&f1 * &j12)) / &det;
        while !positive(&fl(&a - &da)) || !positive(&fl(&b - &db)) {
            da /= 2u32;
            db /= 2u32;
        }
        let quiet = settled(&da, &a) && settled(&db, &b);
        a -= da;
        b -= db;
        if quiet {
            break;
        }
    }
    Ok((a, b))
}

/// The count of successes and their chance of the negative binomial law that makes the data
/// likeliest; none when the data spreads no more than its mean.
fn nbin_fit(x: &[Float], name: &str) -> Result<(Float, Float), String> {
    if x.iter().any(|v| v.is_sign_negative() || !v.is_integer()) {
        return Err(format!("{name} needs whole numbers from 0"));
    }
    let middle = mean(x);
    let count = x.len() as u32;
    let var = x.iter().fold(fl(0), |s, v| s + fl(v - &middle).square()) / (count - 1).max(1);
    if var <= middle {
        return Err(format!(
            "{name}: the data spreads no more than its mean, no negative binomial law is the likeliest"
        ));
    }
    let mut r = fl(middle.square_ref()) / fl(&var - &middle);
    for _ in 0..500 {
        let mut off = (fl(&r / fl(&r + &middle)).ln() - digamma(&r)) * count;
        let mut slope = (fl(r.recip_ref()) - fl(&r + &middle).recip() - trigamma(&r)?) * count;
        for v in x {
            let at = fl(v + &r);
            off += digamma(&at);
            slope += trigamma(&at)?;
        }
        let step = off / slope;
        let mut next = fl(&r - &step);
        if !positive(&next) {
            next = fl(&r / 2u32);
        }
        let quiet = settled(&step, &r);
        r = next;
        if quiet {
            break;
        }
    }
    let chance = fl(&r / fl(&r + &middle));
    Ok((r, chance))
}

/// The name of a law without spaces, in small letters, as its short stem.
fn stem(e: &Expr) -> Option<&'static str> {
    let word: String = crate::arrays::word(e)?
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    Some(match word.as_str() {
        "normal" | "norm" | "gaussian" => "normal",
        "exponential" | "exp" => "exponential",
        "poisson" | "poiss" => "poisson",
        "rayleigh" | "rayl" => "rayleigh",
        "lognormal" | "logn" => "lognormal",
        "gamma" | "gam" => "gamma",
        "weibull" | "wbl" => "weibull",
        "beta" => "beta",
        "uniform" | "unif" => "uniform",
        "discreteuniform" | "unid" => "discreteuniform",
        "geometric" | "geo" => "geometric",
        "binomial" | "bino" => "binomial",
        "bernoulli" => "bernoulli",
        "negativebinomial" | "nbin" => "negativebinomial",
        _ => return None,
    })
}

const LAWS: &str = "normal, exponential, poisson, rayleigh, lognormal, gamma, weibull, beta, uniform, discrete uniform, geometric, binomial (with \"ntrials\", n), bernoulli, negative binomial";

/// The parameters of a law as MATLAB orders them; the spreads of the normal laws divide by
/// the count (the likeliest) or by one less.
fn estimates(
    e: &Expr,
    law: &str,
    trials: Option<&Expr>,
    likeliest: bool,
    name: &str,
) -> Result<Vec<Expr>, String> {
    let all = data(e, name)?;
    let count = Expr::int(all.len() as i64);
    let stat = |kind: &str| crate::datafun::stat(kind, all);
    let spread = match likeliest {
        true => "pstd",
        false => "std",
    };
    let pair = |(a, b): (Float, Float)| vec![shown(&a), shown(&b)];
    Ok(match law {
        "normal" => vec![stat("mean")?, stat(spread)?],
        "exponential" | "poisson" | "bernoulli" => vec![stat("mean")?],
        "rayleigh" => {
            let squares: Vec<Expr> = all.iter().map(|x| pow(x.clone(), Expr::int(2))).collect();
            vec![tidy(pow(
                div(add(squares), mul(vec![Expr::int(2), count])),
                Expr::half(),
            ))]
        }
        "lognormal" => {
            let l = logs(&above_zero(e, name)?);
            let middle = mean(&l);
            let apart = l.iter().fold(fl(0), |s, v| s + fl(v - &middle).square());
            let by = match likeliest {
                true => l.len(),
                false => l.len().max(2) - 1,
            };
            pair((middle, (apart / by as u32).sqrt()))
        }
        "gamma" => pair(gamma_fit(&above_zero(e, name)?, name)?),
        "weibull" => pair(weibull_fit(&above_zero(e, name)?, name)?),
        "beta" => pair(beta_fit(&reals(e, name)?, name)?),
        "uniform" => vec![stat("min")?, stat("max")?],
        "discreteuniform" => vec![stat("max")?],
        "geometric" => vec![tidy(pow(
            add(vec![Expr::one(), stat("mean")?]),
            Expr::minus_one(),
        ))],
        "binomial" => {
            let n =
                trials.ok_or_else(|| format!("{name}: the binomial law needs \"ntrials\", n"))?;
            vec![n.clone(), tidy(div(stat("mean")?, n.clone()))]
        }
        _ => pair(nbin_fit(&reals(e, name)?, name)?),
    })
}

/// The options after the data: the name of the law (with or without "distribution" before
/// it) and the number of trials.
fn asked<'a>(
    a: &'a [Expr],
    name: &str,
) -> Result<(Option<&'static str>, Option<&'a Expr>), String> {
    let (mut law, mut trials) = (None, None);
    let mut rest = a[1..].iter();
    while let Some(o) = rest.next() {
        match crate::arrays::word(o).as_deref() {
            Some("distribution") => {}
            Some("ntrials") => {
                trials = Some(
                    rest.next()
                        .ok_or_else(|| format!("{name}: \"ntrials\" needs a number"))?,
                )
            }
            _ => law = Some(stem(o).ok_or_else(|| format!("{name} knows the laws {LAWS}"))?),
        }
    }
    Ok((law, trials))
}

/// How well every point sits in its group: the mean distance to the nearest other group
/// less that to its own, over the larger of the two (1 well inside, below 0 misplaced).
fn silhouette(a: &[Expr]) -> Res {
    let name = "silhouette";
    let usage = "silhouette(points, [groups][, \"euclidean\" | \"sqeuclidean\"]): a row for every point, a group for every row";
    let all = points(&a[0], name)?;
    let marks = a[1]
        .as_list()
        .filter(|m| m.len() == all.len())
        .ok_or(usage)?;
    let squared = match a.get(2).and_then(crate::arrays::word).as_deref() {
        None | Some("sqeuclidean") => true,
        Some("euclidean") => false,
        Some(_) => return Err(usage.into()),
    };
    let apart = |p: &[Float], q: &[Float]| -> Float {
        let sum = p
            .iter()
            .zip(q)
            .fold(fl(0), |s, (x, y)| s + fl(x - y).square());
        match squared {
            true => sum,
            false => sum.sqrt(),
        }
    };
    let mut kinds: Vec<&Expr> = Vec::new();
    for m in marks {
        if !kinds.contains(&m) {
            kinds.push(m);
        }
    }
    let mut out = Vec::with_capacity(all.len());
    for (i, p) in all.iter().enumerate() {
        // the mean distance to the points of every group, the point itself left out
        let mut own: Option<Float> = None;
        let mut nearest: Option<Float> = None;
        for kind in &kinds {
            let others: Vec<usize> = (0..all.len())
                .filter(|j| *j != i && marks[*j] == **kind)
                .collect();
            if others.is_empty() {
                continue;
            }
            let mean =
                others.iter().fold(fl(0), |s, j| s + apart(p, &all[*j])) / others.len() as u32;
            if marks[i] == **kind {
                own = Some(mean);
            } else if nearest.as_ref().is_none_or(|n| mean < *n) {
                nearest = Some(mean);
            }
        }
        out.push(match (own, nearest) {
            (Some(a), Some(b)) => {
                let top = fl(a.max_ref(&b));
                match top.is_zero() {
                    true => Expr::zero(),
                    false => shown(&(fl(&b - &a) / top)),
                }
            }
            // (a point alone in its group, or one group only)
            (None, Some(_)) => Expr::zero(),
            _ => Expr::one(),
        });
    }
    Ok(Expr::list(out))
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    let pair = |(first, second): (Float, Float)| Expr::list(vec![shown(&first), shown(&second)]);
    let one_or_all = |mut v: Vec<Expr>| match v.len() {
        1 => v.swap_remove(0),
        _ => Expr::list(v),
    };
    Ok(Some(match (name, n) {
        ("runstest", 1..=4) => runstest(a)?,
        ("ansaribradley", 2 | 3) => ansari(&reals(&a[0], name)?, &reals(&a[1], name)?, a.get(2))?,
        ("lillietest", 1) => lilliefors(&reals(&a[0], name)?)?,
        ("mvncdf", 1..=4) => mvncdf(a)?,
        ("cmdscale", 1) => cmdscale(&a[0])?,
        ("procrustes", 2..=6) => procrustes(a)?,
        ("canoncorr", 2) => canoncorr(a)?,
        ("nnmf", 2) => nnmf(&a[0], &a[1])?,
        ("silhouette", 2 | 3) => silhouette(a)?,
        ("poissfit", 1) => one_or_all(estimates(&a[0], "poisson", None, true, name)?),
        ("unifit", 1) => one_or_all(estimates(&a[0], "uniform", None, true, name)?),
        ("raylfit", 1) => one_or_all(estimates(&a[0], "rayleigh", None, true, name)?),
        ("lognfit", 1) => one_or_all(estimates(&a[0], "lognormal", None, false, name)?),
        ("gamfit", 1) => pair(gamma_fit(&above_zero(&a[0], name)?, name)?),
        ("wblfit", 1) => pair(weibull_fit(&above_zero(&a[0], name)?, name)?),
        ("betafit", 1) => pair(beta_fit(&reals(&a[0], name)?, name)?),
        ("nbinfit", 1) => pair(nbin_fit(&reals(&a[0], name)?, name)?),
        ("binofit", 2) => {
            let share = |x: &Expr, n: &Expr| tidy(div(x.clone(), n.clone()));
            match (a[0].as_list(), a[1].as_list()) {
                (Some(x), Some(n)) if x.len() == n.len() => {
                    Expr::list(x.iter().zip(n).map(|(x, n)| share(x, n)).collect())
                }
                (Some(x), None) => Expr::list(x.iter().map(|x| share(x, &a[1])).collect()),
                (None, None) => share(&a[0], &a[1]),
                _ => {
                    return Err("binofit(successes, trials): numbers or lists of one length".into());
                }
            }
        }
        ("mle", 1..=5) => {
            let (law, trials) = asked(a, name)?;
            let law = law.unwrap_or("normal");
            let mut found = estimates(&a[0], law, trials, true, name)?;
            if law == "binomial" {
                found.remove(0);
            }
            one_or_all(found)
        }
        ("fitdist", 2..=4) => {
            let (law, trials) = asked(a, name)?;
            let law = law.ok_or_else(|| format!("{name}(data, \"name of the law\"): {LAWS}"))?;
            let found = match law {
                "bernoulli" => vec![
                    Expr::one(),
                    estimates(&a[0], law, trials, false, name)?.swap_remove(0),
                ],
                _ => estimates(&a[0], law, trials, false, name)?,
            };
            let title = match law {
                "bernoulli" => "binomial",
                other => other,
            };
            crate::mstats::named(title, found, name)
                .ok_or_else(|| format!("{name} knows the laws {LAWS}"))??
        }
        _ => return Ok(None),
    }))
}

pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    inner(name, a).transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_are_counted_in_every_order() {
        let about = runs_about(3, 2);
        let total = about.iter().fold(Integer::new(), |s, c| s + c);
        assert_eq!(total, choices(5, 2));
        assert_eq!(about[2], 2);
        assert_eq!(about[5], 1);
        let rises = runs_up_down(5);
        let total = rises.iter().fold(Integer::new(), |s, c| s + c);
        assert_eq!(total, 120);
        assert_eq!(rises[1], 2);
        assert_eq!(rises[4], 32);
    }

    #[test]
    fn the_rule_of_gauss_integrates_powers() {
        let prec = 200;
        let mut total = Float::new(prec);
        for (x, w) in rule(24, prec).iter() {
            total += Float::with_val(prec, x.square_ref()).square() * w;
        }
        let off = Float::with_val(prec, total - 0.2f64).abs();
        assert!(off < 1e-15);
    }

    #[test]
    fn the_normal_law_is_inverted() {
        let p = 64;
        for u in [1e-12, 0.001, 0.02, 0.3, 0.5, 0.77, 0.99, 1.0 - 1e-9] {
            let u = Float::with_val(p, u);
            // (2u - 1 keeps a share near 0 only with more bits)
            let wide = 4 * p;
            let z = Complex::with_val(wide, (Float::with_val(wide, &u * 2u32) - 1u32, 0));
            let sure = crate::zetas::call("erfinv", &[z], wide).unwrap().unwrap();
            let sure = Float::with_val(p, sure.real() * Float::with_val(wide, 2).sqrt());
            let off = Float::with_val(p, normal_inverse(&u) - &sure).abs();
            assert!(off < 1e-13, "{u}: {off}");
        }
    }

    #[test]
    fn lattices_step_by_the_best_steps() {
        let steps = lattice_steps(1009, 4);
        assert_eq!(steps.len(), 4);
        assert_eq!(steps[0], 1);
        assert!(steps.iter().all(|s| (1..=504).contains(s)));
        assert_ne!(steps[1], steps[2]);
    }

    #[test]
    fn a_symmetric_matrix_keeps_its_directions() {
        let m: Mat = vec![vec![fl(2), fl(1)], vec![fl(1), fl(2)]];
        let (values, vectors) = eigen(&m);
        let mut sorted: Vec<f64> = values.iter().map(Float::to_f64).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((sorted[0] - 1.0).abs() < 1e-20 && (sorted[1] - 3.0).abs() < 1e-20);
        let back = product(&turned(&vectors), &vectors);
        assert!((back[0][0].to_f64() - 1.0).abs() < 1e-20 && back[0][1].to_f64().abs() < 1e-20);
    }
}
