//! Signals: the discrete Fourier and cosine transforms, filters, convolution and correlation.

use crate::arrays::{Arr, arr, grid, tidy, ungrid, whole, word};
use crate::canon::{add, div, mul, sub};
use crate::expr::Expr;
use crate::funcs::func;
use crate::matrix::{self, Rows};
use crate::numeric;
use rug::float::Constant;
use rug::{Complex, Float};

type Res = Result<Expr, String>;

/// Most points of a transform.
const MOST: usize = 1 << 20;

fn precision(n: usize) -> u32 {
    crate::decomp::precision() + usize::BITS - n.leading_zeros()
}

/// `e^(-2 pi i k / n)` for k = 0 .. n - 1 (the conjugates when `inverse`).
fn roots(n: usize, inverse: bool, prec: u32) -> Vec<Complex> {
    let pi = Float::with_val(prec, Constant::Pi);
    (0..n)
        .map(|k| {
            let angle = Float::with_val(prec, &pi * (2 * k as u64)) / (n as u64);
            let (sin, cos) = angle.sin_cos(Float::new(prec));
            Complex::with_val(prec, (cos, if inverse { sin } else { -sin }))
        })
        .collect()
}

/// The transform of a number of points that is a power of two.
fn radix2(x: &mut [Complex], inverse: bool, prec: u32) {
    let n = x.len();
    if n < 2 {
        return;
    }
    let bits = n.trailing_zeros();
    for i in 0..n {
        let j = i.reverse_bits() >> (usize::BITS - bits);
        if i < j {
            x.swap(i, j);
        }
    }
    let w = roots(n, inverse, prec);
    let mut len = 2;
    while len <= n {
        let step = n / len;
        for start in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let t = Complex::with_val(prec, &x[start + k + len / 2] * &w[k * step]);
                let u = x[start + k].clone();
                x[start + k] = Complex::with_val(prec, &u + &t);
                x[start + k + len / 2] = Complex::with_val(prec, &u - &t);
            }
        }
        len *= 2;
    }
}

/// The sums `y[k] = sum x[j] w^(jk)` for any number of points; not divided by n.
pub(crate) fn dft(x: &[Complex], inverse: bool, prec: u32) -> Vec<Complex> {
    let n = x.len();
    if n.is_power_of_two() {
        let mut y = x.to_vec();
        radix2(&mut y, inverse, prec);
        return y;
    }
    if n <= 64 {
        let w = roots(n, inverse, prec);
        return (0..n)
            .map(|k| {
                let mut s = Complex::new(prec);
                for (j, v) in x.iter().enumerate() {
                    s += Complex::with_val(prec, v * &w[(j * k) % n]);
                }
                s
            })
            .collect();
    }
    // Bluestein: jk = (j^2 + k^2 - (k - j)^2)/2 turns the sums into a convolution
    let chirp: Vec<Complex> = {
        let w = roots(2 * n, inverse, prec);
        (0..n).map(|j| w[(j * j) % (2 * n)].clone()).collect()
    };
    let m = (2 * n - 1).next_power_of_two();
    let mut a = vec![Complex::new(prec); m];
    let mut b = vec![Complex::new(prec); m];
    for j in 0..n {
        a[j] = Complex::with_val(prec, &x[j] * &chirp[j]);
        let back = Complex::with_val(prec, chirp[j].conj_ref());
        if j > 0 {
            b[m - j] = back.clone();
        }
        b[j] = back;
    }
    radix2(&mut a, false, prec);
    radix2(&mut b, false, prec);
    for (p, q) in a.iter_mut().zip(&b) {
        *p *= q;
    }
    radix2(&mut a, true, prec);
    (0..n)
        .map(|k| Complex::with_val(prec, &a[k] * &chirp[k]) / (m as u64))
        .collect()
}

/// The roots of one as exact values, when the cosines of their angles have closed forms.
fn exact_roots(n: usize, inverse: bool) -> Option<Vec<Expr>> {
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let angle = mul(vec![Expr::rat(2 * k as i64, n as i64), Expr::pi()]);
        let (c, s) = (func("cos", vec![angle.clone()]), func("sin", vec![angle]));
        if c.has_fn("cos") || s.has_fn("sin") {
            return None;
        }
        let turned = mul(vec![Expr::i(), s]);
        out.push(tidy(if inverse {
            add(vec![c, turned])
        } else {
            sub(c, turned)
        }));
    }
    Some(out)
}

fn has_float(v: &[Expr]) -> bool {
    v.iter().any(Expr::has_float)
}

/// The transform of a list: exact for the few sizes whose roots of one are simple and for
/// lists with names, else in decimals.
fn transform(v: &[Expr], inverse: bool) -> Result<Vec<Expr>, String> {
    let n = v.len();
    if n == 0 {
        return Ok(Vec::new());
    }
    if n > MOST {
        return Err(format!("a transform of more than {MOST} points"));
    }
    let named = v.iter().any(|x| !x.is_const());
    let simple = matches!(n, 1 | 2 | 3 | 4 | 6 | 8 | 12);
    if !has_float(v)
        && (simple || (named && n <= 64))
        && let Some(w) = exact_roots(n, inverse)
    {
        return Ok((0..n)
            .map(|k| {
                let sum = add(v
                    .iter()
                    .enumerate()
                    .map(|(j, x)| mul(vec![x.clone(), w[(j * k) % n].clone()]))
                    .collect());
                let sum = if inverse {
                    div(sum, Expr::int(n as i64))
                } else {
                    sum
                };
                crate::simplify::simplify(&crate::expand::expand(&sum))
            })
            .collect());
    }
    if named {
        return Err("a transform of this size needs numbers".into());
    }
    let prec = precision(n);
    let x: Vec<Complex> = v
        .iter()
        .map(|e| numeric::evalf(e, prec).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    let mut y = dft(&x, inverse, prec);
    if inverse {
        for z in y.iter_mut() {
            *z /= n as u64;
        }
    }
    Ok(crate::decomp::from_complex_list(&y))
}

/// The list cut or filled with zeros to `n` entries.
fn sized(v: &[Expr], n: Option<usize>) -> Vec<Expr> {
    match n {
        None => v.to_vec(),
        Some(n) => (0..n)
            .map(|k| v.get(k).cloned().unwrap_or_else(Expr::zero))
            .collect(),
    }
}

/// What is done to a list.
type Each<'a> = dyn Fn(&[Expr]) -> Result<Vec<Expr>, String> + 'a;

/// A transform of a list, or of every column of a matrix.
fn along(e: &Expr, n: Option<usize>, f: &Each) -> Res {
    match arr(e) {
        Arr::Scalar(x) => Ok(Expr::list(f(&sized(&[x], n))?)),
        Arr::Vector(v) => Ok(Expr::list(f(&sized(&v, n))?)),
        Arr::Matrix(m) if m.len() == 1 => Ok(matrix::from_rows(vec![f(&sized(&m[0], n))?])),
        Arr::Matrix(m) => {
            let cols: Rows = matrix::transpose(&m)
                .iter()
                .map(|c| f(&sized(c, n)))
                .collect::<Result<_, _>>()?;
            Ok(matrix::from_rows(matrix::transpose(&cols)))
        }
    }
}

fn cosine(v: &[Expr], inverse: bool) -> Result<Vec<Expr>, String> {
    let n = v.len();
    if n == 0 {
        return Ok(Vec::new());
    }
    if n > 8192 {
        return Err("a cosine transform of more than 8192 points".into());
    }
    let probe = func(
        "cos",
        vec![mul(vec![Expr::rat(1, 2 * n as i64), Expr::pi()])],
    );
    if !has_float(v) && (!probe.has_fn("cos") || v.iter().any(|x| !x.is_const())) {
        let weight = |k: usize| {
            crate::canon::sqrt(if k == 0 {
                Expr::rat(1, n as i64)
            } else {
                Expr::rat(2, n as i64)
            })
        };
        let wave = |j: usize, k: usize| {
            func(
                "cos",
                vec![mul(vec![
                    Expr::rat(((2 * j + 1) * k) as i64, 2 * n as i64),
                    Expr::pi(),
                ])],
            )
        };
        return Ok((0..n)
            .map(|p| {
                let sum = add((0..n)
                    .map(|q| {
                        let (j, k) = if inverse { (p, q) } else { (q, p) };
                        mul(vec![v[q].clone(), weight(k), wave(j, k)])
                    })
                    .collect());
                crate::simplify::simplify(&crate::expand::expand(&sum))
            })
            .collect());
    }
    let prec = precision(n);
    let x: Vec<Complex> = v
        .iter()
        .map(|e| numeric::evalf(e, prec).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    let pi = Float::with_val(prec, Constant::Pi);
    let table: Vec<Float> = (0..4 * n)
        .map(|k| (Float::with_val(prec, &pi * (k as u64)) / (2 * n as u64)).cos())
        .collect();
    let (first, other) = (
        Float::with_val(prec, n as u64).recip().sqrt(),
        (Float::with_val(prec, 2) / (n as u64)).sqrt(),
    );
    let y: Vec<Complex> = (0..n)
        .map(|p| {
            let mut s = Complex::new(prec);
            for (q, value) in x.iter().enumerate() {
                let (j, k) = if inverse { (p, q) } else { (q, p) };
                let mut term = Complex::with_val(prec, value * &table[((2 * j + 1) * k) % (4 * n)]);
                if inverse {
                    term *= if k == 0 { &first } else { &other };
                }
                s += term;
            }
            if !inverse {
                s *= if p == 0 { &first } else { &other };
            }
            s
        })
        .collect();
    Ok(crate::decomp::from_complex_list(&y))
}

fn half_turn(e: &Expr, back: bool) -> Expr {
    let shift = |len: usize| {
        let k = if back { len.div_ceil(2) } else { len / 2 } as i64;
        move |v: &[Expr]| -> Vec<Expr> {
            let n = v.len() as i64;
            (0..n)
                .map(|i| v[(i - k).rem_euclid(n) as usize].clone())
                .collect()
        }
    };
    match arr(e) {
        Arr::Scalar(x) => x,
        Arr::Vector(v) => Expr::list(shift(v.len())(&v)),
        Arr::Matrix(m) => {
            let rows: Rows = {
                let n = m.len() as i64;
                let k = if back {
                    (m.len()).div_ceil(2)
                } else {
                    m.len() / 2
                } as i64;
                (0..n)
                    .map(|i| m[(i - k).rem_euclid(n) as usize].clone())
                    .collect()
            };
            let turn = shift(rows[0].len());
            matrix::from_rows(rows.iter().map(|r| turn(r)).collect())
        }
    }
}

fn conj(x: &Expr) -> Expr {
    if x.is_const() {
        func("conj", vec![x.clone()])
    } else {
        x.clone()
    }
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    let flat = |k: usize| -> Result<Vec<Expr>, String> {
        a[k].as_list()
            .filter(|l| l.iter().all(|x| x.as_list().is_none()))
            .map(<[Expr]>::to_vec)
            .ok_or_else(|| format!("{name} needs lists"))
    };
    Ok(Some(match (name, n) {
        ("fft" | "ifft", 1 | 2) => {
            let size = match a.get(1) {
                Some(k) => Some(whole(k, "the number of points", 1, MOST as i64)? as usize),
                None => None,
            };
            along(&a[0], size, &|v| transform(v, name == "ifft"))?
        }
        ("fft2" | "ifft2", 1) => {
            let m = matrix::rows(&a[0]).ok_or_else(|| format!("{name} needs a matrix"))?;
            let back = name == "ifft2";
            let cols: Rows = matrix::transpose(&m)
                .iter()
                .map(|c| transform(c, back))
                .collect::<Result<_, _>>()?;
            let rows: Rows = matrix::transpose(&cols)
                .iter()
                .map(|r| transform(r, back))
                .collect::<Result<_, _>>()?;
            matrix::from_rows(rows)
        }
        ("fftshift" | "ifftshift", 1) => half_turn(&a[0], name == "ifftshift"),
        ("dct" | "idct", 1 | 2) => {
            let size = match a.get(1) {
                Some(k) => Some(whole(k, "the number of points", 1, 8192)? as usize),
                None => None,
            };
            along(&a[0], size, &|v| cosine(v, name == "idct"))?
        }
        ("lfilter", 4) => {
            // from a state: the transposed direct form, as filtic gives the state
            let (b, den, x, mut state) = (flat(0)?, flat(1)?, flat(2)?, flat(3)?);
            let lead = den
                .first()
                .filter(|d| !crate::simplify::is_zero(d))
                .ok_or("filter(b, a, x, zi): the first coefficient of a must not be zero")?;
            if state.len() + 1 != b.len().max(den.len()) {
                return Err(format!(
                    "filter(b, a, x, zi): the state zi has {} value(s) for these coefficients, not {}",
                    b.len().max(den.len()) - 1,
                    state.len()
                ));
            }
            let scaled = |v: &[Expr]| -> Vec<Expr> {
                v.iter()
                    .map(|c| tidy(div(c.clone(), lead.clone())))
                    .collect()
            };
            let (b, den) = (scaled(&b), scaled(&den));
            let at = |v: &[Expr], k: usize| v.get(k).cloned().unwrap_or_else(Expr::zero);
            let mut y: Vec<Expr> = Vec::with_capacity(x.len());
            for value in &x {
                let out = tidy(add(vec![
                    mul(vec![at(&b, 0), value.clone()]),
                    at(&state, 0),
                ]));
                for i in 0..state.len() {
                    state[i] = tidy(add(vec![
                        mul(vec![at(&b, i + 1), value.clone()]),
                        at(&state, i + 1),
                        mul(vec![Expr::minus_one(), at(&den, i + 1), out.clone()]),
                    ]));
                }
                y.push(out);
            }
            Expr::list(y)
        }
        ("lfilter", 3) => {
            let (b, den, x) = (flat(0)?, flat(1)?, flat(2)?);
            let lead = den
                .first()
                .filter(|d| !crate::simplify::is_zero(d))
                .ok_or("filter(b, a, x): the first coefficient of a must not be zero")?;
            let mut y: Vec<Expr> = Vec::with_capacity(x.len());
            for i in 0..x.len() {
                let mut terms = Vec::new();
                for (k, c) in b.iter().enumerate().take(i + 1) {
                    terms.push(mul(vec![c.clone(), x[i - k].clone()]));
                }
                for (k, c) in den.iter().enumerate().take(i + 1).skip(1) {
                    terms.push(mul(vec![Expr::minus_one(), c.clone(), y[i - k].clone()]));
                }
                y.push(tidy(div(add(terms), lead.clone())));
            }
            Expr::list(y)
        }
        ("conv2", 2 | 3) => {
            let ((p, _), (q, _)) = (grid(&a[0]), grid(&a[1]));
            let filled = |m: &Rows| {
                m.first().is_some_and(|first| {
                    !first.is_empty() && m.iter().all(|r| r.len() == first.len())
                })
            };
            if !filled(&p) || !filled(&q) {
                return Err(
                    "conv2(A, B): both are matrices with entries, as many in each of their rows"
                        .into(),
                );
            }
            let (r, c) = (p.len() + q.len() - 1, p[0].len() + q[0].len() - 1);
            let mut sums = vec![vec![Vec::new(); c]; r];
            for (i, row) in p.iter().enumerate() {
                for (j, x) in row.iter().enumerate() {
                    for (k, other) in q.iter().enumerate() {
                        for (l, y) in other.iter().enumerate() {
                            sums[i + k][j + l].push(mul(vec![x.clone(), y.clone()]));
                        }
                    }
                }
            }
            let full: Rows = sums
                .into_iter()
                .map(|r| r.into_iter().map(|t| tidy(add(t))).collect())
                .collect();
            let cut = |rows: usize, cols: usize, top: usize, left: usize| -> Rows {
                full[top..top + rows]
                    .iter()
                    .map(|r| r[left..left + cols].to_vec())
                    .collect()
            };
            let out = match a.get(2).map(word) {
                None => full.clone(),
                Some(Some(w)) if w == "full" => full.clone(),
                Some(Some(w)) if w == "same" => {
                    cut(p.len(), p[0].len(), q.len() / 2, q[0].len() / 2)
                }
                Some(Some(w)) if w == "valid" => {
                    if q.len() > p.len() || q[0].len() > p[0].len() {
                        return Ok(Some(Expr::list(Vec::new())));
                    }
                    cut(
                        p.len() - q.len() + 1,
                        p[0].len() - q[0].len() + 1,
                        q.len() - 1,
                        q[0].len() - 1,
                    )
                }
                Some(_) => {
                    return Err(
                        "conv2(A, B), conv2(A, B, \"same\") or conv2(A, B, \"valid\")".into(),
                    );
                }
            };
            ungrid(out, true)
        }
        ("xcorr", 1 | 2) => {
            let x = flat(0)?;
            let y = if n == 2 { flat(1)? } else { x.clone() };
            let size = x.len().max(y.len());
            let (x, y) = (sized(&x, Some(size)), sized(&y, Some(size)));
            let turned: Vec<Expr> = y.iter().rev().map(conj).collect();
            Expr::list(crate::polys::convolve(&x, &turned))
        }
        ("upsample", 2) => {
            let x = flat(0)?;
            let k = whole(&a[1], "the factor", 1, 100_000)? as usize;
            if x.len().saturating_mul(k) > MOST {
                return Err(format!("a list of more than {MOST} entries"));
            }
            Expr::list(
                x.into_iter()
                    .flat_map(|v| {
                        let mut group = vec![v];
                        group.extend(std::iter::repeat_n(Expr::zero(), k - 1));
                        group
                    })
                    .collect(),
            )
        }
        ("downsample", 2 | 3) => {
            let x = flat(0)?;
            let k = whole(&a[1], "the factor", 1, 1_000_000_000)? as usize;
            let phase = if n == 3 {
                whole(&a[2], "the offset", 0, k as i64 - 1)? as usize
            } else {
                0
            };
            Expr::list(x.into_iter().skip(phase).step_by(k).collect())
        }
        _ => return Ok(None),
    }))
}

pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    inner(name, a).transpose()
}
