//! Tests of statistics that rank the data or count it, distances of samples from one
//! another, and points gathered into groups.

use crate::arrays::tidy;
use crate::canon::{add, div, mul, neg, pow, sub};
use crate::eval::Session;
use crate::expr::Expr;
use crate::funcs::func;
use crate::matrix::{self, Rows};
use crate::num::Num;
use crate::numeric;
use crate::radix::text_of;
use rug::ops::Pow;
use rug::{Complex, Float, Integer, Rational};
use std::cmp::Ordering;

type Res = Result<Expr, String>;

pub(crate) fn bits() -> u32 {
    numeric::bits_for(numeric::digits()) + 64
}

pub(crate) fn fl<T>(x: T) -> Float
where
    Float: rug::Assign<T>,
{
    Float::with_val(bits(), x)
}

pub(crate) fn shown(x: &Float) -> Expr {
    numeric::to_float_expr(Complex::with_val(x.prec(), x), numeric::digits())
}

pub(crate) fn exact(r: Rational) -> Expr {
    Expr::num(Num::real(r))
}

pub(crate) fn real(e: &Expr, name: &str) -> Result<Float, String> {
    let none = || format!("{name} needs real numbers, {e} is none");
    let v = numeric::evalf_digits(e, numeric::digits() + 10).map_err(|_| none())?;
    match v.imag().is_zero() && v.real().is_finite() {
        true => Ok(fl(v.real())),
        false => Err(none()),
    }
}

pub(crate) fn reals(e: &Expr, name: &str) -> Result<Vec<Float>, String> {
    let items = e
        .as_list()
        .filter(|l| !l.is_empty() && l.iter().all(|x| x.as_list().is_none()))
        .ok_or_else(|| format!("{name} needs a list of numbers"))?;
    items.iter().map(|x| real(x, name)).collect()
}

pub(crate) fn groups(e: &Expr, name: &str) -> Result<Vec<Vec<Float>>, String> {
    let usage = || format!("{name}([[values of a group], [values of another], ...])");
    let items = e.as_list().filter(|l| l.len() >= 2).ok_or_else(usage)?;
    items
        .iter()
        .map(|g| reals(g, name).map_err(|_| usage()))
        .collect()
}

pub(crate) fn table(e: &Expr, name: &str) -> Result<Vec<Vec<Float>>, String> {
    let rows = matrix::rows(e)
        .filter(|m| !m.is_empty() && !m[0].is_empty())
        .ok_or_else(|| format!("{name} needs a matrix of numbers"))?;
    rows.iter()
        .map(|r| r.iter().map(|x| real(x, name)).collect())
        .collect()
}

/// The points of a matrix of rows; a flat list is points of one coordinate.
pub(crate) fn points(e: &Expr, name: &str) -> Result<Vec<Vec<Float>>, String> {
    match e.as_list() {
        Some(items) if items.iter().all(|x| x.as_list().is_none()) => {
            Ok(reals(e, name)?.into_iter().map(|x| vec![x]).collect())
        }
        _ => table(e, name),
    }
}

pub(crate) fn sum(v: &[Float]) -> Float {
    v.iter().fold(fl(0), |s, x| s + x)
}

pub(crate) fn mean(v: &[Float]) -> Float {
    sum(v) / v.len() as u32
}

pub(crate) fn square(x: Float) -> Float {
    x.square()
}

/// The sum of the squares of the distances from the mean.
pub(crate) fn spread(v: &[Float]) -> Float {
    let m = mean(v);
    v.iter().fold(fl(0), |s, x| s + square(fl(x - &m)))
}

/// Twice the ranks of values (equal values share the mean of their places), and the sum
/// of t^3 - t over the groups of t equal values.
pub(crate) fn double_ranks(v: &[Float]) -> (Vec<u64>, u64) {
    let mut places: Vec<usize> = (0..v.len()).collect();
    places.sort_by(|a, b| v[*a].partial_cmp(&v[*b]).unwrap_or(Ordering::Equal));
    let mut ranks = vec![0; v.len()];
    let (mut from, mut ties) = (0, 0u64);
    while from < places.len() {
        let mut to = from;
        while to + 1 < places.len() && v[places[to + 1]] == v[places[from]] {
            to += 1;
        }
        for place in &places[from..=to] {
            ranks[*place] = (from + to + 2) as u64;
        }
        let t = (to - from + 1) as u64;
        ties += t * t * t - t;
        from = to + 1;
    }
    (ranks, ties)
}

/// How many choices of so many of the numbers have every sum; without a size: choices of
/// any number of them.
pub(crate) fn sums_of_choices(numbers: &[u64], size: Option<usize>) -> Vec<Integer> {
    let total = numbers.iter().sum::<u64>() as usize;
    let sizes = size.unwrap_or(0);
    let mut counts = vec![vec![Integer::new(); total + 1]; sizes + 1];
    counts[0][0] = Integer::from(1);
    for r in numbers {
        let r = *r as usize;
        for c in (0..=sizes).rev() {
            let from = if size.is_some() {
                c.checked_sub(1)
            } else {
                Some(0)
            };
            let Some(from) = from else {
                continue;
            };
            for s in (r..=total).rev() {
                if !counts[from][s - r].is_zero() {
                    let more = counts[from][s - r].clone();
                    counts[c][s] += more;
                }
            }
        }
    }
    counts.swap_remove(sizes)
}

/// Above zero; what is no number is not.
pub(crate) fn positive(x: &Float) -> bool {
    *x > 0
}

/// The chance of a standard normal value beyond z on either side.
pub(crate) fn beyond(z: &Float) -> Float {
    (fl(z.abs_ref()) / fl(2).sqrt()).erfc()
}

/// The chance of a chi-square value with so many degrees above x.
pub(crate) fn chi_above(x: &Float, degrees: usize) -> Float {
    if !positive(x) {
        return fl(1);
    }
    let half = fl(degrees as u32) / 2u32;
    let upper = half.clone().gamma_inc(&fl(x / 2u32));
    upper / half.gamma()
}

/// A probability: a fraction when it was counted, else a number.
pub(crate) enum Chance {
    Counted(Rational),
    Near(Float),
}

impl Chance {
    pub(crate) fn value(&self) -> Expr {
        match self {
            Chance::Counted(r) => exact(r.clone()),
            Chance::Near(x) => shown(x),
        }
    }

    pub(crate) fn small(&self) -> Expr {
        Expr::int(i64::from(match self {
            Chance::Counted(r) => *r < Rational::from((1, 20)),
            Chance::Near(x) => *x < 0.05,
        }))
    }

    pub(crate) fn twice(tail: Integer, all: Integer) -> Chance {
        Chance::Counted(Rational::from((tail * 2u32, all)).min(Rational::from(1)))
    }

    /// `[p, h]`: the probability first, as MATLAB gives it for the tests of ranks and signs.
    pub(crate) fn first(&self) -> Expr {
        Expr::list(vec![self.value(), self.small()])
    }

    /// `[h, p, more]`.
    pub(crate) fn second(&self, more: Vec<Expr>) -> Expr {
        let mut out = vec![self.small(), self.value()];
        out.extend(more);
        Expr::list(out)
    }
}

/// Whether a test counts every case: as asked, else as MATLAB decides by the sizes.
pub(crate) fn counted(option: Option<&Expr>, name: &str, by_size: bool) -> Result<bool, String> {
    match option.map(|o| text_of(o).map(str::to_lowercase)) {
        None => Ok(by_size),
        Some(Some(way)) if way == "exact" => Ok(true),
        Some(Some(way)) if way == "approximate" => Ok(false),
        Some(_) => Err(format!("{name}: the way is \"exact\" or \"approximate\"")),
    }
}

fn ranksum(x: &[Float], y: &[Float], way: Option<&Expr>) -> Result<Chance, String> {
    let (nx, ny) = (x.len(), y.len());
    let (small, large) = if nx <= ny { (x, y) } else { (y, x) };
    let all: Vec<Float> = small.iter().chain(large).cloned().collect();
    let (ranks, ties) = double_ranks(&all);
    let (count, n) = (small.len(), nx + ny);
    let twice: u64 = ranks[..count].iter().sum();
    if counted(way, "ranksum", nx.min(ny) < 10 && n < 20)? {
        if n > 60 {
            return Err("ranksum counts every case for 60 values at most".into());
        }
        let counts = sums_of_choices(&ranks, Some(count));
        let all: Integer = counts.iter().sum();
        let low: Integer = counts[..=twice as usize].iter().sum();
        let high: Integer = counts[twice as usize..].iter().sum();
        return Ok(Chance::twice(low.min(high), all));
    }
    let middle = fl(count as u64 * (n as u64 + 1)) / 2u32;
    let tied = fl(ties) / (n as u64 * (n as u64 - 1));
    let variance = fl(nx as u64 * ny as u64) * (fl(n as u64 + 1) - tied) / 12u32;
    if !positive(&variance) {
        return Ok(Chance::Near(fl(1)));
    }
    let off = fl(twice) / 2u32 - middle;
    let nearer = match off.partial_cmp(&0) {
        Some(Ordering::Greater) => off - 0.5f64,
        Some(Ordering::Less) => off + 0.5f64,
        _ => off,
    };
    Ok(Chance::Near(beyond(&(nearer / variance.sqrt()))))
}

/// The differences that the tests of signs look at: from zero, from a number or from
/// the values of a second list; those that are zero are left out.
fn differences(a: &[Expr], name: &str) -> Result<Vec<Float>, String> {
    let x = reals(&a[0], name)?;
    let d: Vec<Float> = match a.get(1) {
        Some(y) if y.as_list().is_some() => {
            let y = reals(y, name)?;
            if y.len() != x.len() {
                return Err(format!("{name}(x, y) needs two lists of one length"));
            }
            x.iter().zip(&y).map(|(p, q)| fl(p - q)).collect()
        }
        Some(m) if text_of(m).is_none() => {
            let m = real(m, name)?;
            x.iter().map(|p| fl(p - &m)).collect()
        }
        _ => x,
    };
    Ok(d.into_iter().filter(|v| !v.is_zero()).collect())
}

/// The option of a test of signs: the last argument when it is a word.
fn way_of(a: &[Expr]) -> Option<&Expr> {
    a.last().filter(|o| a.len() > 1 && text_of(o).is_some())
}

fn signrank(d: &[Float], way: Option<&Expr>) -> Result<Chance, String> {
    let n = d.len() as u64;
    if n == 0 {
        return Ok(Chance::Near(fl(1)));
    }
    let sizes: Vec<Float> = d.iter().map(|v| fl(v.abs_ref())).collect();
    let (ranks, ties) = double_ranks(&sizes);
    let below: u64 = ranks
        .iter()
        .zip(d)
        .filter(|(_, v)| v.is_sign_negative())
        .map(|(r, _)| *r)
        .sum();
    let twice = below.min(n * (n + 1) - below);
    if counted(way, "signrank", n <= 15)? {
        if n > 60 {
            return Err("signrank counts every case for 60 differences at most".into());
        }
        let counts = sums_of_choices(&ranks, None);
        let low: Integer = counts[..=twice as usize].iter().sum();
        return Ok(Chance::twice(low, Integer::from(1) << n as u32));
    }
    let off = fl(twice) / 2u32 - fl(n * (n + 1)) / 4u32;
    let variance = (fl(n * (n + 1) * (2 * n + 1)) - fl(ties) / 2u32) / 24u32;
    if !positive(&variance) {
        return Ok(Chance::Near(fl(1)));
    }
    let nearer = if off.is_zero() { off } else { off + 0.5f64 };
    Ok(Chance::Near(beyond(&(nearer / variance.sqrt()))))
}

fn signtest(d: &[Float], way: Option<&Expr>) -> Result<Chance, String> {
    let n = d.len() as u32;
    if n == 0 {
        return Ok(Chance::Near(fl(1)));
    }
    let above = d.iter().filter(|v| v.is_sign_positive()).count() as u32;
    let fewer = above.min(n - above);
    if counted(way, "signtest", n < 100)? {
        if n > 100_000 {
            return Err("signtest counts every case for 100000 differences at most".into());
        }
        let mut tail = Integer::new();
        for k in 0..=fewer {
            tail += Integer::from(n).binomial(k);
        }
        return Ok(Chance::twice(tail, Integer::from(1) << n));
    }
    let off = i64::from(above) * 2 - i64::from(n);
    let nearer = off - off.signum();
    Ok(Chance::Near(beyond(&(fl(nearer) / fl(n).sqrt()))))
}

fn kruskal(all: &[Vec<Float>]) -> Result<Float, String> {
    let values: Vec<Float> = all.iter().flatten().cloned().collect();
    let n = values.len() as u64;
    let (ranks, ties) = double_ranks(&values);
    let mut at = 0;
    let mut total = fl(0);
    for group in all {
        let twice: u64 = ranks[at..at + group.len()].iter().sum();
        total += square(fl(twice) / 2u32) / group.len() as u32;
        at += group.len();
    }
    let h = total * 12u32 / (n * (n + 1)) - 3 * (n + 1);
    let tied = fl(1) - fl(ties) / (n * n * n - n);
    if !positive(&tied) {
        return Ok(fl(1));
    }
    Ok(chi_above(&(h / tied), all.len() - 1))
}

fn friedman(rows: &[Vec<Float>]) -> Result<Float, String> {
    let (n, k) = (rows.len() as u64, rows[0].len() as u64);
    if k < 2 || n < 2 {
        return Err("friedman needs a matrix of two rows and two columns or more".into());
    }
    let mut sums = vec![0u64; k as usize];
    let mut ties = 0;
    for row in rows {
        let (ranks, tied) = double_ranks(row);
        ties += tied;
        for (s, r) in sums.iter_mut().zip(ranks) {
            *s += r;
        }
    }
    let squares = sums.iter().fold(fl(0), |s, r| s + square(fl(*r) / 2u32));
    let stat = squares * 12u32 / (k * n * (k + 1)) - 3 * n * (k + 1);
    let tied = fl(1) - fl(ties) / (n * k * (k * k - 1));
    if !positive(&tied) {
        return Ok(fl(1));
    }
    Ok(chi_above(&(stat / tied), k as usize - 1))
}

/// Bartlett's test that groups have one variance.
fn bartlett(all: &[Vec<Float>]) -> Result<Float, String> {
    if all.iter().any(|g| g.len() < 2) {
        return Err("vartestn needs two values or more in every group".into());
    }
    let (k, n) = (
        all.len() as u32,
        all.iter().map(Vec::len).sum::<usize>() as u32,
    );
    let mut pooled = fl(0);
    let (mut logs, mut parts) = (fl(0), fl(0));
    for group in all {
        let free = group.len() as u32 - 1;
        let variance = spread(group) / free;
        if !positive(&variance) {
            return Err("vartestn: a group has one value only, again and again".into());
        }
        pooled += fl(&variance * free);
        logs += variance.ln() * free;
        parts += fl(1) / free;
    }
    let pooled = pooled / (n - k);
    let stat = (pooled.ln() * (n - k) - logs) / (fl(1) + (parts - fl(1) / (n - k)) / (3 * (k - 1)));
    Ok(chi_above(&stat, k as usize - 1))
}

/// The exact test of a table of two rows and two columns: `[h, p, odds ratio]`.
fn fisher(e: &Expr) -> Res {
    let usage = "fishertest([[a, b], [c, d]]) needs four counts";
    let rows = matrix::rows(e).filter(|m| m.len() == 2 && m[0].len() == 2);
    let cells: Vec<u32> = rows
        .ok_or(usage)?
        .iter()
        .flatten()
        .map(|x| {
            x.as_int()
                .and_then(Integer::to_u32)
                .filter(|c| *c <= 1_000_000)
        })
        .collect::<Option<_>>()
        .ok_or(usage)?;
    let (a, b, c, d) = (cells[0], cells[1], cells[2], cells[3]);
    let (row, other, column) = (a + b, c + d, a + c);
    let ways = |x: u32| Integer::from(row).binomial(x) * Integer::from(other).binomial(column - x);
    let seen = ways(a);
    let (mut tail, mut all) = (Integer::new(), Integer::new());
    for x in column.saturating_sub(other)..=row.min(column) {
        let w = ways(x);
        if w <= seen {
            tail += &w;
        }
        all += w;
    }
    let p = Chance::Counted(Rational::from((tail, all)));
    let odds = match u64::from(b) * u64::from(c) {
        0 => Expr::inf(),
        under => exact(Rational::from((u64::from(a) * u64::from(d), under))),
    };
    Ok(p.second(vec![odds]))
}

/// The chance that the greatest distance between the shares of a sample of n and those of
/// its law stays below d (Marsaglia, Tsang and Wang), counted for samples up to 1000.
fn kolmogorov(n: usize, d: &Float) -> Float {
    let s = fl(d.square_ref()) * n as u64;
    let far = fl(1)
        - fl(2)
            * fl(-(fl(2.000071) + fl(0.331) / fl(n as u64).sqrt() + fl(1.409) / n as u64) * &s)
                .exp();
    if s > 7.24 || (s > 3.76 && n > 99) {
        return far;
    }
    let nd = fl(d * n as u64);
    let k = fl(nd.floor_ref()).to_f64() as usize + 1;
    let m = 2 * k - 1;
    if n > 1000 {
        // the law of large samples, nudged as Stephens does
        let root = fl(n as u64).sqrt();
        let lambda: Float = (root.clone() + 0.12f64 + fl(0.11) / root) * d;
        let mut above = fl(0);
        for j in 1..=101u32 {
            let term = fl(-(fl(lambda.square_ref()) * 2u32 * j * j)).exp();
            above += if j % 2 == 1 { term } else { -term };
        }
        return (fl(1) - above * 2u32).clamp(&fl(0), &fl(1));
    }
    let h = fl(k as u64) - nd;
    let mut grid = vec![vec![fl(0); m]; m];
    for (i, row) in grid.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            if i + 1 >= j {
                *cell = fl(1);
            }
        }
    }
    for (i, row) in grid.iter_mut().enumerate() {
        row[0] -= fl((&h).pow(i as u32 + 1));
    }
    for (i, cell) in grid[m - 1].iter_mut().enumerate() {
        *cell -= fl((&h).pow((m - i) as u32));
    }
    let corner = fl(&h * 2u32) - 1u32;
    if corner > 0 {
        grid[m - 1][0] += corner.pow(m as u32);
    }
    for (i, row) in grid.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            for g in 1..=(i + 1).saturating_sub(j) {
                *cell /= g as u32;
            }
        }
    }
    let times = |a: &[Vec<Float>], b: &[Vec<Float>]| -> Vec<Vec<Float>> {
        (0..m)
            .map(|i| {
                (0..m)
                    .map(|j| (0..m).fold(fl(0), |s, l| s + fl(&a[i][l] * &b[l][j])))
                    .collect()
            })
            .collect()
    };
    let mut power: Option<Vec<Vec<Float>>> = None;
    let mut base = grid;
    let mut left = n;
    while left > 0 {
        if left % 2 == 1 {
            power = Some(match power {
                Some(p) => times(&p, &base),
                None => base.clone(),
            });
        }
        left /= 2;
        if left > 0 {
            base = times(&base, &base);
        }
    }
    let mut out = power.map_or_else(|| fl(1), |p| p[k - 1][k - 1].clone());
    for i in 1..=n {
        out = out * i as u64 / n as u64;
    }
    out
}

/// The greatest distance between the shares of a sorted sample and the shares that a law
/// gives its values.
pub(crate) fn farthest(shares: &[Float]) -> Float {
    let n = shares.len() as u64;
    let mut most = fl(0);
    for (i, share) in shares.iter().enumerate() {
        let above = fl(i as u64 + 1) / n - share;
        let below = fl(share - fl(i as u64) / n);
        most = most.max(&above).max(&below);
    }
    most
}

/// The greatest distance between the shares of two samples, and its chance by the law of
/// large samples as MATLAB nudges it.
fn two_samples(x: &[Float], y: &[Float]) -> (Float, Float) {
    let sorted = |v: &[Float]| {
        let mut v = v.to_vec();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        v
    };
    let (x, y) = (sorted(x), sorted(y));
    let (mut i, mut j) = (0, 0);
    let mut most = fl(0);
    while i < x.len() && j < y.len() {
        let at = if x[i] <= y[j] {
            x[i].clone()
        } else {
            y[j].clone()
        };
        while i < x.len() && x[i] <= at {
            i += 1;
        }
        while j < y.len() && y[j] <= at {
            j += 1;
        }
        let apart = fl(i as u64) / x.len() as u64 - fl(j as u64) / y.len() as u64;
        most = most.max(&apart.abs());
    }
    let n = fl(x.len() as u64 * y.len() as u64) / (x.len() + y.len()) as u64;
    let root = n.sqrt();
    let lambda: Float = (root.clone() + 0.12f64 + fl(0.11) / root) * &most;
    let mut p = fl(0);
    for k in 1..=101u32 {
        let term = fl(-(fl(lambda.square_ref()) * 2u32 * k * k)).exp();
        p += if k % 2 == 1 { term } else { -term };
    }
    ((p * 2u32).clamp(&fl(0), &fl(1)), most)
}

/// The steps of gathering points into ever larger groups: `[group, group, distance]`,
/// the points are the groups 1 to n, every step makes the next one.
fn linkage(mut apart: Vec<Vec<Float>>, way: &str) -> Result<Vec<(usize, usize, Float)>, String> {
    let n = apart.len();
    let squares = matches!(way, "centroid" | "median" | "ward");
    if !matches!(way, "single" | "complete" | "average" | "weighted") && !squares {
        return Err(
            "linkage(points, way): the way is \"single\", \"complete\", \"average\", \
             \"weighted\", \"centroid\", \"median\" or \"ward\""
                .into(),
        );
    }
    if squares {
        apart.iter_mut().flatten().for_each(|d| d.square_mut());
    }
    let mut names: Vec<usize> = (1..=n).collect();
    let mut sizes = vec![1u32; n];
    let mut alive = vec![true; n];
    let mut out = Vec::new();
    for step in 0..n.saturating_sub(1) {
        let mut best: Option<(usize, usize)> = None;
        for i in (0..n).filter(|i| alive[*i]) {
            for j in (i + 1..n).filter(|j| alive[*j]) {
                if best.is_none_or(|(p, q)| apart[i][j] < apart[p][q]) {
                    best = Some((i, j));
                }
            }
        }
        let Some((i, j)) = best else {
            break;
        };
        let between = apart[i][j].clone();
        let (ni, nj) = (sizes[i], sizes[j]);
        for k in (0..n).filter(|k| alive[*k] && *k != i && *k != j) {
            let (di, dj) = (apart[i][k].clone(), apart[j][k].clone());
            let nk = sizes[k];
            let new = match way {
                "single" => di.min(&dj),
                "complete" => di.max(&dj),
                "average" => (di * ni + dj * nj) / (ni + nj),
                "weighted" => (di + dj) / 2u32,
                "centroid" => {
                    (di * ni + dj * nj) / (ni + nj)
                        - fl(&between * (ni * nj)) / ((ni + nj) * (ni + nj))
                }
                "median" => (di + dj) / 2u32 - fl(&between / 4u32),
                _ => (di * (ni + nk) + dj * (nj + nk) - fl(&between * nk)) / (ni + nj + nk),
            };
            apart[i][k] = new.clone();
            apart[k][i] = new;
        }
        let (first, second) = (names[i].min(names[j]), names[i].max(names[j]));
        out.push((
            first,
            second,
            if squares {
                between.max(&fl(0)).sqrt()
            } else {
                between
            },
        ));
        alive[j] = false;
        sizes[i] = ni + nj;
        names[i] = n + step + 1;
    }
    Ok(out)
}

/// The distances between the rows of a matrix of points.
pub(crate) fn distances(points: &[Vec<Float>]) -> Vec<Vec<Float>> {
    points
        .iter()
        .map(|p| {
            points
                .iter()
                .map(|q| {
                    p.iter()
                        .zip(q)
                        .fold(fl(0), |s, (a, b)| s + square(fl(a - b)))
                        .sqrt()
                })
                .collect()
        })
        .collect()
}

/// Labels from 1 in the order in which the groups first appear.
fn relabel(groups: &[usize]) -> Vec<Expr> {
    let mut seen: Vec<usize> = Vec::new();
    groups
        .iter()
        .map(|g| {
            let at = seen.iter().position(|s| s == g).unwrap_or_else(|| {
                seen.push(*g);
                seen.len() - 1
            });
            Expr::int(at as i64 + 1)
        })
        .collect()
}

/// The points in so many groups about their means: the first means are the points
/// farthest from one another, then every point goes to its nearest mean until none moves.
fn kmeans(points: &[Vec<Float>], count: usize) -> (Vec<usize>, Vec<Vec<Float>>) {
    let far = |p: &[Float], q: &[Float]| {
        p.iter()
            .zip(q)
            .fold(fl(0), |s, (a, b)| s + square(fl(a - b)))
    };
    let dims = points[0].len();
    let middle: Vec<Float> = (0..dims)
        .map(|d| mean(&points.iter().map(|p| p[d].clone()).collect::<Vec<_>>()))
        .collect();
    let farthest = |from: &dyn Fn(&[Float]) -> Float| -> usize {
        let mut best = 0;
        for k in 1..points.len() {
            if from(&points[k]) > from(&points[best]) {
                best = k;
            }
        }
        best
    };
    let mut means = vec![points[farthest(&|p| far(p, &middle))].clone()];
    while means.len() < count {
        let next = farthest(&|p| {
            means
                .iter()
                .map(|m| far(p, m))
                .fold(None, |least: Option<Float>, d| match least {
                    Some(l) if l <= d => Some(l),
                    _ => Some(d),
                })
                .unwrap_or_else(|| fl(0))
        });
        means.push(points[next].clone());
    }
    let mut labels = vec![usize::MAX; points.len()];
    for _ in 0..1000 {
        let nearest: Vec<usize> = points
            .iter()
            .map(|p| {
                let mut best = 0;
                for k in 1..count {
                    if far(p, &means[k]) < far(p, &means[best]) {
                        best = k;
                    }
                }
                best
            })
            .collect();
        if nearest == labels {
            break;
        }
        labels = nearest;
        for (k, m) in means.iter_mut().enumerate() {
            let own: Vec<&Vec<Float>> = points
                .iter()
                .zip(&labels)
                .filter(|(_, l)| **l == k)
                .map(|(p, _)| p)
                .collect();
            if !own.is_empty() {
                *m = (0..dims)
                    .map(|d| mean(&own.iter().map(|p| p[d].clone()).collect::<Vec<_>>()))
                    .collect();
            }
        }
    }
    (labels, means)
}

/// The middle value of a sorted list.
pub(crate) fn median(sorted: &[Float]) -> Float {
    let n = sorted.len();
    match n % 2 {
        1 => sorted[n / 2].clone(),
        _ => fl(&sorted[n / 2 - 1] + &sorted[n / 2]) / 2u32,
    }
}

/// The width that MATLAB gives the bells of a density drawn over a sample.
fn bell_width(x: &[Float]) -> Float {
    let sorted = |mut v: Vec<Float>| {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        v
    };
    let v = sorted(x.to_vec());
    let middle = median(&v);
    let off = sorted(v.iter().map(|a| fl(a - &middle).abs()).collect());
    let mut sigma = median(&off) / 0.6745f64;
    if !positive(&sigma) {
        sigma = fl(&v[v.len() - 1] - &v[0]);
    }
    match sigma > 0 {
        true => sigma * (fl(4) / (3 * x.len() as u32)).root(5),
        false => fl(1),
    }
}

fn density(x: &[Float], at: &Float, width: &Float) -> Float {
    let scale = fl(2) * fl(rug::float::Constant::Pi);
    let total = x.iter().fold(fl(0), |s, v| {
        let z = fl(at - v) / width;
        s + fl(-(z.square() / 2u32)).exp()
    });
    total / (scale.sqrt() * width * x.len() as u32)
}

fn exprs(m: &Expr, name: &str) -> Result<Rows, String> {
    match m.as_list() {
        Some(items) if !items.is_empty() && items.iter().all(|x| x.as_list().is_none()) => {
            Ok(vec![items.to_vec()])
        }
        _ => matrix::rows(m)
            .filter(|r| !r.is_empty())
            .ok_or_else(|| format!("{name} needs a matrix: every row is a point")),
    }
}

/// `d * inverse * d'` for a row d.
fn form(d: &[Expr], inverse: &Rows) -> Expr {
    let mut terms = Vec::new();
    for (i, di) in d.iter().enumerate() {
        for (j, dj) in d.iter().enumerate() {
            terms.push(mul(vec![di.clone(), inverse[i][j].clone(), dj.clone()]));
        }
    }
    tidy(add(terms))
}

impl Session {
    /// The chance of a value above x under a distribution of this program.
    fn above(&mut self, law: Expr, x: &Float, name: &str) -> Result<Float, String> {
        let below = self.call("cdf", &[law, shown(x)])?;
        Ok(fl(1) - real(&below, name)?)
    }

    fn anova2(&mut self, e: &Expr, repeats: usize) -> Res {
        let usage =
            "anova2(matrix, repeats): the columns are one factor, every `repeats` rows the other";
        let rows = table(e, "anova2")?;
        let (a, n) = (rows[0].len(), repeats);
        if a < 2 || rows.len() % n != 0 || rows.len() / n < 2 {
            return Err(usage.into());
        }
        let b = rows.len() / n;
        let all: Vec<Float> = rows.iter().flatten().cloned().collect();
        let grand = mean(&all);
        let column: Vec<Float> = (0..a)
            .map(|j| mean(&rows.iter().map(|r| r[j].clone()).collect::<Vec<_>>()))
            .collect();
        let block: Vec<Float> = (0..b)
            .map(|i| mean(&rows[i * n..(i + 1) * n].concat()))
            .collect();
        let cell = |i: usize, j: usize| {
            mean(
                &rows[i * n..(i + 1) * n]
                    .iter()
                    .map(|r| r[j].clone())
                    .collect::<Vec<_>>(),
            )
        };
        let of_columns =
            column.iter().fold(fl(0), |s, m| s + square(fl(m - &grand))) * (b * n) as u32;
        let of_blocks =
            block.iter().fold(fl(0), |s, m| s + square(fl(m - &grand))) * (a * n) as u32;
        let mut both = fl(0);
        let mut within = fl(0);
        for i in 0..b {
            for j in 0..a {
                let m = cell(i, j);
                both += square(fl(&m - &block[i]) - &column[j] + &grand) * n as u32;
                for row in &rows[i * n..(i + 1) * n] {
                    within += square(fl(&row[j] - &m));
                }
            }
        }
        let free_both = (a - 1) * (b - 1);
        let (error, free) = match n {
            1 => (both.clone(), free_both),
            _ => (within, a * b * (n - 1)),
        };
        let unit = error / free as u32;
        if !positive(&unit) {
            return Err("anova2: the values leave nothing unexplained to measure against".into());
        }
        let mut chance = |of: Float, degrees: usize| -> Res {
            let ratio = of / degrees as u32 / &unit;
            let law = Expr::call(
                "fdist",
                vec![Expr::int(degrees as i64), Expr::int(free as i64)],
            );
            Ok(shown(&self.above(law, &ratio, "anova2")?))
        };
        let mut out = vec![chance(of_columns, a - 1)?, chance(of_blocks, b - 1)?];
        if n > 1 {
            out.push(chance(both, free_both)?);
        }
        Ok(Expr::list(out))
    }

    fn kstest(&mut self, a: &[Expr]) -> Res {
        let mut x = reals(&a[0], "kstest")?;
        x.sort_by(|p, q| p.partial_cmp(q).unwrap_or(Ordering::Equal));
        let shares: Vec<Float> = match a.get(1) {
            Some(law) => x
                .iter()
                .map(|v| real(&self.call("cdf", &[law.clone(), shown(v)])?, "kstest"))
                .collect::<Result<_, _>>()?,
            None => x
                .iter()
                .map(|v| (fl(-v) / fl(2).sqrt()).erfc() / 2u32)
                .collect(),
        };
        let most = farthest(&shares);
        let p = Chance::Near((fl(1) - kolmogorov(x.len(), &most)).clamp(&fl(0), &fl(1)));
        Ok(p.second(vec![shown(&most)]))
    }

    /// A straight fit with what measures it: the coefficients (the constant first), their
    /// errors, t values and probabilities, and how much of the spread the fit explains.
    fn fitlm(&mut self, a: &[Expr]) -> Res {
        let usage = "fitlm(x, y): a list of numbers or a matrix with a row for every value of y";
        let y = reals(&a[1], "fitlm")?;
        let x = points(&a[0], "fitlm")?;
        let (count, terms) = (y.len(), x[0].len() + 1);
        if x.len() != count || count <= terms || x.iter().any(|r| r.len() + 1 != terms) {
            return Err(usage.into());
        }
        let design: Vec<Vec<Float>> = x
            .iter()
            .map(|r| std::iter::once(fl(1)).chain(r.iter().cloned()).collect())
            .collect();
        // the inverse of X'X by elimination, beside the unit matrix
        let mut grid: Vec<Vec<Float>> = (0..terms)
            .map(|i| {
                (0..2 * terms)
                    .map(|j| match j < terms {
                        true => design.iter().fold(fl(0), |s, r| s + fl(&r[i] * &r[j])),
                        false => fl(u32::from(j - terms == i)),
                    })
                    .collect()
            })
            .collect();
        for k in 0..terms {
            let pivot = (k..terms)
                .max_by(|p, q| {
                    let (p, q) = (fl(grid[*p][k].abs_ref()), fl(grid[*q][k].abs_ref()));
                    p.partial_cmp(&q).unwrap_or(Ordering::Equal)
                })
                .unwrap_or(k);
            grid.swap(k, pivot);
            if fl(grid[k][k].abs_ref()) < fl(1) >> (bits() / 2) {
                return Err("fitlm: the columns of x depend on one another".into());
            }
            let lead = grid[k][k].clone();
            grid[k].iter_mut().for_each(|v| *v /= &lead);
            for i in (0..terms).filter(|i| *i != k) {
                let factor = grid[i][k].clone();
                let pivot_row = grid[k].clone();
                for (cell, lead) in grid[i].iter_mut().zip(&pivot_row) {
                    *cell -= fl(lead * &factor);
                }
            }
        }
        let inverse: Vec<Vec<Float>> = grid.iter().map(|r| r[terms..].to_vec()).collect();
        let moments: Vec<Float> = (0..terms)
            .map(|i| {
                design
                    .iter()
                    .zip(&y)
                    .fold(fl(0), |s, (r, v)| s + fl(&r[i] * v))
            })
            .collect();
        let beta: Vec<Float> = inverse
            .iter()
            .map(|r| {
                r.iter()
                    .zip(&moments)
                    .fold(fl(0), |s, (p, q)| s + fl(p * q))
            })
            .collect();
        let left = design.iter().zip(&y).fold(fl(0), |s, (r, v)| {
            let fitted = r.iter().zip(&beta).fold(fl(0), |t, (p, q)| t + fl(p * q));
            s + square(fl(v - fitted))
        });
        let total = spread(&y);
        let free = (count - terms) as u32;
        let unit = fl(&left / free);
        let exact_fit = unit <= fl(&total >> (bits() - 16)) || unit.is_nan();
        let errors: Vec<Float> = (0..terms)
            .map(|i| fl(&unit * &inverse[i][i]).max(&fl(0)).sqrt())
            .collect();
        let law = Expr::call("studenttdist", vec![Expr::int(i64::from(free))]);
        let (mut ratios, mut chances) = (Vec::new(), Vec::new());
        for (b, e) in beta.iter().zip(&errors) {
            if exact_fit {
                ratios.push(Expr::inf());
                chances.push(Expr::zero());
                continue;
            }
            let t = fl(b / e);
            chances.push(shown(
                &(self.above(law.clone(), &fl(t.abs_ref()), "fitlm")? * 2u32),
            ));
            ratios.push(shown(&t));
        }
        let all = |v: &[Float]| Expr::list(v.iter().map(shown).collect());
        let explained = fl(1) - fl(&left / &total);
        let adjusted = fl(1) - fl(&unit / fl(&total / (count as u32 - 1)));
        let (ratio, chance) = match (exact_fit, terms) {
            (_, 1) => (Expr::undefined(), Expr::undefined()),
            (true, _) => (Expr::inf(), Expr::zero()),
            (false, _) => {
                let f = fl(&total - &left) / (terms as u32 - 1) / &unit;
                let law = Expr::call(
                    "fdist",
                    vec![Expr::int(terms as i64 - 1), Expr::int(i64::from(free))],
                );
                (shown(&f), shown(&self.above(law, &f, "fitlm")?))
            }
        };
        Ok(crate::records::record(vec![
            ("coefficients", all(&beta)),
            ("se", all(&errors)),
            ("tstat", Expr::list(ratios)),
            ("pvalue", Expr::list(chances)),
            ("rsquared", shown(&explained)),
            ("adjusted", shown(&adjusted)),
            ("rmse", shown(&unit.sqrt())),
            ("dfe", Expr::int(i64::from(free))),
            ("fstat", ratio),
            ("p", chance),
        ]))
    }

    pub(crate) fn hypotest(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        let n = a.len();
        Some((|| -> Res {
            Ok(match (name, n) {
                ("ranksum", 2 | 3) => {
                    ranksum(&reals(&a[0], name)?, &reals(&a[1], name)?, a.get(2))?.first()
                }
                ("signrank", 1..=3) => signrank(&differences(a, name)?, way_of(a))?.first(),
                ("signtest", 1..=3) => signtest(&differences(a, name)?, way_of(a))?.first(),
                ("kruskalwallis", 1) => shown(&kruskal(&groups(&a[0], name)?)?),
                ("friedman", 1) => shown(&friedman(&table(&a[0], name)?)?),
                ("vartestn", 1) => shown(&bartlett(&groups(&a[0], name)?)?),
                ("fishertest", 1) => fisher(&a[0])?,
                ("chi2gof", 2 | 3) => {
                    let (seen, wanted) = (reals(&a[0], name)?, reals(&a[1], name)?);
                    let fitted = match a.get(2) {
                        Some(k) => crate::arrays::whole(k, "the number of fitted parameters", 0, 1000)? as usize,
                        None => 0,
                    };
                    if seen.len() != wanted.len() || seen.len() < fitted + 2 {
                        return Err("chi2gof(counts, expected counts or shares): two lists of one length".into());
                    }
                    if wanted.iter().any(|w| !positive(w)) {
                        return Err("chi2gof needs expected counts above zero".into());
                    }
                    let scale = sum(&seen) / sum(&wanted);
                    let stat = seen.iter().zip(&wanted).fold(fl(0), |s, (o, w)| {
                        let e = fl(w * &scale);
                        s + square(fl(o - &e)) / e
                    });
                    let p = Chance::Near(chi_above(&stat, seen.len() - 1 - fitted));
                    p.second(vec![shown(&stat)])
                }
                ("kstest", 1 | 2) => self.kstest(a)?,
                ("kstest2", 2) => {
                    let (p, most) = two_samples(&reals(&a[0], name)?, &reals(&a[1], name)?);
                    Chance::Near(p).second(vec![shown(&most)])
                }
                ("jbtest", 1) => {
                    let x = reals(&a[0], name)?;
                    let m = mean(&x);
                    let moment = |k: u32| {
                        x.iter().fold(fl(0), |s, v| s + fl(v - &m).pow(k)) / x.len() as u32
                    };
                    let second = moment(2);
                    if !positive(&second) {
                        return Err("jbtest needs values that differ".into());
                    }
                    let skew = moment(3) / fl(second.clone().pow(3u32)).sqrt();
                    let peak = moment(4) / second.square() - 3u32;
                    let stat = (skew.square() + peak.square() / 4u32) * x.len() as u32 / 6u32;
                    Chance::Near(fl(-(stat.clone() / 2u32)).exp()).second(vec![shown(&stat)])
                }
                ("fitlm", 2) => self.fitlm(a)?,
                ("anova2", 1 | 2) => {
                    let repeats = match a.get(1) {
                        Some(r) => crate::arrays::whole(r, "the number of repeats", 1, 1_000_000)? as usize,
                        None => 1,
                    };
                    self.anova2(&a[0], repeats)?
                }
                ("mahal", 2) => {
                    let (y, x) = (exprs(&a[0], name)?, exprs(&a[1], name)?);
                    let (count, dims) = (x.len(), x[0].len());
                    if count < 2 || y.iter().chain(&x).any(|r| r.len() != dims) {
                        return Err("mahal(points, sample): rows of one length, two rows of the sample or more".into());
                    }
                    let middle: Vec<Expr> = (0..dims)
                        .map(|j| div(add(x.iter().map(|r| r[j].clone()).collect()), Expr::int(count as i64)))
                        .collect();
                    let off = |r: &[Expr]| -> Vec<Expr> {
                        r.iter().zip(&middle).map(|(v, m)| sub(v.clone(), m.clone())).collect()
                    };
                    let spread: Rows = (0..dims)
                        .map(|i| {
                            (0..dims)
                                .map(|j| {
                                    let terms = x
                                        .iter()
                                        .map(|r| {
                                            let d = off(r);
                                            mul(vec![d[i].clone(), d[j].clone()])
                                        })
                                        .collect();
                                    tidy(div(add(terms), Expr::int(count as i64 - 1)))
                                })
                                .collect()
                        })
                        .collect();
                    let inverse = matrix::inverse(&spread)
                        .map_err(|_| "mahal: the sample lies in fewer dimensions than it has".to_string())?;
                    let all: Vec<Expr> = y.iter().map(|r| form(&off(r), &inverse)).collect();
                    match a[0].as_list().is_some_and(|l| l.iter().all(|v| v.as_list().is_none())) {
                        true => all[0].clone(),
                        false => Expr::list(all),
                    }
                }
                ("mvnpdf", 1..=3) => {
                    let x = exprs(&a[0], name)?;
                    let dims = x[0].len();
                    let middle: Vec<Expr> = match a.get(1) {
                        Some(m) => m.as_list().filter(|l| l.len() == dims).ok_or("mvnpdf(x, mean, covariance): a mean for every coordinate")?.to_vec(),
                        None => vec![Expr::zero(); dims],
                    };
                    let spread = match a.get(2) {
                        Some(s) => matrix::rows(s).filter(|m| m.len() == dims && m[0].len() == dims).ok_or("mvnpdf(x, mean, covariance): a square matrix of covariances")?,
                        None => matrix::identity(dims),
                    };
                    let inverse = matrix::inverse(&spread).map_err(|_| "mvnpdf: the covariances have no inverse".to_string())?;
                    let under = mul(vec![
                        pow(mul(vec![Expr::int(2), Expr::pi()]), Expr::int(dims as i64)),
                        matrix::det(&spread)?,
                    ]);
                    let all: Vec<Expr> = x
                        .iter()
                        .map(|r| {
                            if r.len() != dims {
                                return Err("mvnpdf: rows of one length".to_string());
                            }
                            let d: Vec<Expr> = r.iter().zip(&middle).map(|(v, m)| sub(v.clone(), m.clone())).collect();
                            let power = div(neg(form(&d, &inverse)), Expr::int(2));
                            Ok(tidy(div(func("exp", vec![power]), pow(under.clone(), Expr::half()))))
                        })
                        .collect::<Result<_, _>>()?;
                    match a[0].as_list().is_some_and(|l| l.iter().all(|v| v.as_list().is_none())) {
                        true => all[0].clone(),
                        false => Expr::list(all),
                    }
                }
                ("mvnrnd", 2 | 3) => {
                    let middle = a[0].as_list().filter(|l| !l.is_empty()).ok_or("mvnrnd(mean, covariance, count)")?;
                    let dims = middle.len();
                    let count = match a.get(2) {
                        Some(c) => crate::arrays::whole(c, "the number of points", 1, 1_000_000)? as usize,
                        None => 1,
                    };
                    let root = matrix::rows(&self.call("chol", &[a[1].clone()])?)
                        .filter(|m| m.len() == dims)
                        .ok_or("mvnrnd(mean, covariance, count): covariances for every pair of coordinates")?;
                    // (as MATLAB draws them: a matrix of count by dims, filled down its columns)
                    let mut drawn = vec![vec![Expr::zero(); dims]; count];
                    for j in 0..dims {
                        for row in drawn.iter_mut() {
                            row[j] = crate::random::normal_value();
                        }
                    }
                    let mut out = Vec::new();
                    for z in drawn {
                        let moved = matrix::matmul(&vec![z], &root)?;
                        let row: Vec<Expr> = moved[0]
                            .iter()
                            .zip(middle)
                            .map(|(v, m)| numeric::n(&add(vec![v.clone(), m.clone()]), numeric::digits()))
                            .collect();
                        out.push(Expr::list(row));
                    }
                    match a.get(2) {
                        Some(_) => Expr::list(out),
                        None => out.remove(0),
                    }
                }
                ("ksdensity", 1..=3) => {
                    let x = reals(&a[0], name)?;
                    let width = match a.get(2) {
                        Some(w) => real(w, name)?,
                        None => bell_width(&x),
                    };
                    if !positive(&width) {
                        return Err("ksdensity needs a width above zero".into());
                    }
                    match a.get(1) {
                        Some(at) if at.as_list().is_some() => Expr::list(
                            reals(at, name)?.iter().map(|p| shown(&density(&x, p, &width))).collect(),
                        ),
                        Some(at) => shown(&density(&x, &real(at, name)?, &width)),
                        None => {
                            let least = x.iter().fold(x[0].clone(), |m, v| m.min(v)) - fl(&width * 3u32);
                            let most = x.iter().fold(x[0].clone(), |m, v| m.max(v)) + fl(&width * 3u32);
                            let places: Vec<Float> = (0..100u32)
                                .map(|k| fl(&least + fl(&most - &least) * k / 99u32))
                                .collect();
                            Expr::list(vec![
                                Expr::list(places.iter().map(|p| shown(&density(&x, p, &width))).collect()),
                                Expr::list(places.iter().map(shown).collect()),
                            ])
                        }
                    }
                }
                ("jackknife", 2) => {
                    let items = a[1].as_list().filter(|l| l.len() >= 2).ok_or("jackknife(f, list): a list of two entries or more")?;
                    let mut out = Vec::new();
                    for skip in 0..items.len() {
                        let rest: Vec<Expr> = items
                            .iter()
                            .enumerate()
                            .filter(|(k, _)| *k != skip)
                            .map(|(_, x)| x.clone())
                            .collect();
                        out.push(self.apply(&a[0], &[Expr::list(rest)])?);
                    }
                    Expr::list(out)
                }
                ("bootstrp", 3) => {
                    let count = crate::arrays::whole(&a[0], "the number of samples", 1, 1_000_000)? as usize;
                    let items = a[2].as_list().filter(|l| !l.is_empty()).ok_or("bootstrp(count, f, list)")?;
                    let size = Expr::int(items.len() as i64);
                    let mut out = Vec::new();
                    for _ in 0..count {
                        let drawn = crate::random::call("randi", &[size.clone(), Expr::int(1), size.clone()])
                            .ok_or("no random numbers")??;
                        let picks: Vec<Expr> = exprs(&drawn, name)?[0]
                            .iter()
                            .map(|k| {
                                let at = k.as_int().and_then(Integer::to_usize).unwrap_or(1);
                                items[at.clamp(1, items.len()) - 1].clone()
                            })
                            .collect();
                        out.push(self.apply(&a[1], &[Expr::list(picks)])?);
                    }
                    Expr::list(out)
                }
                ("grpstats", 2 | 3) => {
                    let usage = "grpstats(values, groups, what): a group for every value";
                    let (values, labels) = (a[0].as_list().ok_or(usage)?, a[1].as_list().ok_or(usage)?);
                    if values.len() != labels.len() || values.is_empty() {
                        return Err(usage.into());
                    }
                    let mut keys: Vec<Expr> = Vec::new();
                    for g in labels {
                        if !keys.contains(g) {
                            keys.push(g.clone());
                        }
                    }
                    let keys = crate::lists::sorted(&keys);
                    let kinds: Vec<String> = match a.get(2) {
                        Some(k) => match k.as_list() {
                            Some(all) => all
                                .iter()
                                .map(|w| text_of(w).map(str::to_lowercase).ok_or(usage))
                                .collect::<Result<_, _>>()?,
                            None => vec![text_of(k).map(str::to_lowercase).ok_or(usage)?],
                        },
                        None => vec!["mean".to_string()],
                    };
                    let mut out = Vec::new();
                    for kind in &kinds {
                        let command = match kind.as_str() {
                            "numel" | "count" => "length",
                            "sem" | "mean" | "std" | "var" | "min" | "max" | "median" | "range" | "sum" => kind.as_str(),
                            other => return Err(format!("grpstats: \"{other}\" is none of \"mean\", \"std\", \"var\", \"sem\", \"min\", \"max\", \"median\", \"range\", \"sum\" and \"numel\"")),
                        };
                        let mut column = Vec::new();
                        for key in &keys {
                            let part: Vec<Expr> = values
                                .iter()
                                .zip(labels)
                                .filter(|(_, g)| *g == key)
                                .map(|(v, _)| v.clone())
                                .collect();
                            let size = Expr::int(part.len() as i64);
                            // (one value alone is not spread at all)
                            if part.len() == 1 && matches!(command, "sem" | "std" | "var") {
                                column.push(Expr::zero());
                                continue;
                            }
                            let part = Expr::list(part);
                            column.push(match command {
                                "sem" => {
                                    let spread = self.apply(&Expr::sym("std"), &[part])?;
                                    tidy(div(spread, pow(size, Expr::half())))
                                }
                                "range" => {
                                    let most = self.apply(&Expr::sym("max"), std::slice::from_ref(&part))?;
                                    tidy(sub(most, self.apply(&Expr::sym("min"), &[part])?))
                                }
                                command => self.apply(&Expr::sym(command), &[part])?,
                            });
                        }
                        out.push(Expr::list(column));
                    }
                    match a.get(2).is_some_and(|k| k.as_list().is_some()) {
                        true => Expr::list(out),
                        false => out.remove(0),
                    }
                }
                ("linkage", 1 | 2) => {
                    let way = match a.get(1) {
                        Some(w) => text_of(w).map(str::to_lowercase).ok_or("linkage(points, way) needs the way in quotes")?,
                        None => "single".to_string(),
                    };
                    let apart = match a[0].as_list() {
                        // (a flat list holds the distances of the pairs, as pdist gives them)
                        Some(flat) if !flat.is_empty() && flat.iter().all(|x| x.as_list().is_none()) => {
                            let d = reals(&a[0], name)?;
                            let count = (1..=d.len() + 1).find(|m| m * (m - 1) / 2 >= d.len()).unwrap_or(2);
                            if count * (count - 1) / 2 != d.len() {
                                return Err("linkage: a list of distances has one for every pair of points".into());
                            }
                            let mut grid = vec![vec![fl(0); count]; count];
                            let pairs = (0..count).flat_map(|i| (i + 1..count).map(move |j| (i, j)));
                            for ((i, j), far) in pairs.zip(&d) {
                                grid[i][j] = far.clone();
                                grid[j][i] = far.clone();
                            }
                            grid
                        }
                        _ => distances(&table(&a[0], name)?),
                    };
                    if apart.len() < 2 {
                        return Err("linkage needs two points or more".into());
                    }
                    Expr::list(
                        linkage(apart, &way)?
                            .into_iter()
                            .map(|(i, j, d)| Expr::list(vec![Expr::int(i as i64), Expr::int(j as i64), shown(&d)]))
                            .collect(),
                    )
                }
                ("cluster", 2) => {
                    let usage = "cluster(steps of linkage, count): the points in so many groups";
                    let steps = matrix::rows(&a[0]).filter(|m| !m.is_empty() && m[0].len() == 3).ok_or(usage)?;
                    let points = steps.len() + 1;
                    let count = crate::arrays::whole(&a[1], "the number of groups", 1, points as i64)? as usize;
                    let heights: Vec<Float> = steps
                        .iter()
                        .map(|row| real(&row[2], name))
                        .collect::<Result<_, _>>()?;
                    // the cut is as high as the last step that leaves so many groups; steps of
                    // that very height are taken too, so there may be fewer groups
                    let taken = match points - count {
                        0 => 0,
                        last => {
                            let cut = &heights[last - 1];
                            last + heights[last..].iter().take_while(|h| *h <= cut).count()
                        }
                    };
                    let mut group: Vec<usize> = (1..=points).collect();
                    for (step, row) in steps.iter().take(taken).enumerate() {
                        let id = |x: &Expr| {
                            x.as_int()
                                .and_then(Integer::to_usize)
                                .filter(|k| (1..=points + step).contains(k))
                                .ok_or(usage)
                        };
                        let (i, j) = (id(&row[0])?, id(&row[1])?);
                        for g in group.iter_mut().filter(|g| **g == i || **g == j) {
                            *g = points + step + 1;
                        }
                    }
                    Expr::list(relabel(&group))
                }
                ("kmeans", 2) => {
                    let all = points(&a[0], name)?;
                    let count = crate::arrays::whole(&a[1], "the number of groups", 1, all.len() as i64)? as usize;
                    let (labels, means) = kmeans(&all, count);
                    // (the groups are numbered as their points first appear)
                    let mut order: Vec<usize> = Vec::new();
                    for l in &labels {
                        if !order.contains(l) {
                            order.push(*l);
                        }
                    }
                    let flat = a[0].as_list().is_some_and(|l| l.iter().all(|x| x.as_list().is_none()));
                    let centres: Vec<Expr> = order
                        .iter()
                        .map(|k| match flat {
                            true => shown(&means[*k][0]),
                            false => Expr::list(means[*k].iter().map(shown).collect()),
                        })
                        .collect();
                    Expr::list(vec![Expr::list(relabel(&labels)), Expr::list(centres)])
                }
                _ => return Err(String::new()),
            })
        })())
        .filter(|r| !matches!(r, Err(e) if e.is_empty()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floats(v: &[f64]) -> Vec<Float> {
        v.iter().map(|x| fl(*x)).collect()
    }

    #[test]
    fn ranks_share_places() {
        let (ranks, ties) = double_ranks(&floats(&[3.0, 1.0, 3.0, 2.0, 3.0]));
        assert_eq!(ranks, vec![8, 2, 8, 4, 8]);
        assert_eq!(ties, 24);
        // choices of two of [1, 2, 3]: the sums 3, 4, 5 once each
        let counts = sums_of_choices(&[1, 2, 3], Some(2));
        assert_eq!(counts, [0, 0, 0, 1, 1, 1, 0].map(Integer::from).to_vec());
        // any of [1, 2]: nothing, 1, 2, both
        assert_eq!(
            sums_of_choices(&[1, 2], None),
            [1, 1, 1, 1].map(Integer::from).to_vec()
        );
    }

    #[test]
    fn counted_chances_are_fractions() {
        let p = ranksum(&floats(&[1.0, 2.0, 3.0]), &floats(&[4.0, 5.0, 6.0]), None);
        assert!(matches!(p, Ok(Chance::Counted(r)) if r == Rational::from((1, 10))));
        let p = signrank(&floats(&[1.0, 2.0, 3.0, 4.0, 5.0]), None);
        assert!(matches!(p, Ok(Chance::Counted(r)) if r == Rational::from((1, 16))));
        let p = signtest(&floats(&[1.0, 2.0, 3.0, 4.0, -5.0]), None);
        assert!(matches!(p, Ok(Chance::Counted(r)) if r == Rational::from((3, 8))));
    }

    #[test]
    fn the_law_of_kolmogorov() {
        // (worked out with 50 digits by the same matrices; SciPy agrees where it counts too)
        for (n, d, wanted) in [
            (10, 0.274, 0.628_479_615_456_504_3),
            (100, 0.05, 0.046_784_028_936_427_5),
            (1000, 0.02, 0.189_102_868_929_787_85),
            (5, 0.6, 0.969_92),
        ] {
            let got = kolmogorov(n, &fl(d)).to_f64();
            assert!(
                (got - wanted).abs() < 1e-9 * wanted.max(1e-3),
                "K({n}, {d}) = {got}"
            );
        }
    }

    #[test]
    fn points_gather_from_the_nearest() {
        let apart = distances(&[
            floats(&[0.0]),
            floats(&[1.0]),
            floats(&[3.0]),
            floats(&[7.0]),
        ]);
        let steps = linkage(apart.clone(), "single").expect("steps");
        let plain: Vec<(usize, usize, f64)> =
            steps.iter().map(|(i, j, d)| (*i, *j, d.to_f64())).collect();
        assert_eq!(plain, vec![(1, 2, 1.0), (3, 5, 2.0), (4, 6, 4.0)]);
        let steps = linkage(apart, "complete").expect("steps");
        assert_eq!(steps[2].2.to_f64(), 7.0);
        let (labels, means) = kmeans(
            &[
                floats(&[0.0]),
                floats(&[1.0]),
                floats(&[10.0]),
                floats(&[11.0]),
            ],
            2,
        );
        assert_eq!(labels[0], labels[1]);
        assert_eq!(labels[2], labels[3]);
        assert_ne!(labels[0], labels[2]);
        let mut centres: Vec<f64> = means.iter().map(|m| m[0].to_f64()).collect();
        centres.sort_by(f64::total_cmp);
        assert_eq!(centres, vec![0.5, 10.5]);
    }
}
