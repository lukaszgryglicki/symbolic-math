//! Systems of control: transfer functions, zeros and poles, state space; how they answer
//! in time and in frequency; their margins; and the gains that place poles or cost least.
//! A system with one input and one output is `tf(num, den)`, `zpk(z, p, k)`,
//! `ss(A, B, C, D)` or simply a ratio of polynomials in `s` (in `z` for sampled systems).

use crate::arrays::{tidy, word};
use crate::canon::{add, div, mul, neg, pow, sub};
use crate::decomp::{self, CMat};
use crate::eval::Session;
use crate::expr::Expr;
use crate::matrix::{self, Rows};
use crate::numeric;
use crate::pencils::solved;
use crate::radix::text_of;
use rug::ops::Pow;
use rug::{Complex, Float};

type Res = Result<Expr, String>;

/// A system with one input and one output: the coefficients of its two polynomials from
/// the highest power, and the time of a sample when it is sampled (-1: not said).
#[derive(Clone)]
struct Sys {
    num: Vec<Expr>,
    den: Vec<Expr>,
    ts: Option<Expr>,
}

fn list_of(e: &Expr) -> Vec<Expr> {
    match e.as_list() {
        Some(items) => items.to_vec(),
        None => vec![e.clone()],
    }
}

fn trimmed(mut p: Vec<Expr>) -> Vec<Expr> {
    while p.len() > 1 && p[0].is_zero() {
        p.remove(0);
    }
    if p.is_empty() {
        p.push(Expr::zero());
    }
    p
}

fn poly_expr(p: &[Expr], var: &Expr) -> Expr {
    let n = p.len();
    tidy(add(p
        .iter()
        .enumerate()
        .map(|(k, c)| {
            mul(vec![
                c.clone(),
                pow(var.clone(), Expr::int((n - 1 - k) as i64)),
            ])
        })
        .collect()))
}

fn poly_mul(a: &[Expr], b: &[Expr]) -> Vec<Expr> {
    let mut out = vec![Expr::zero(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j] = add(vec![out[i + j].clone(), mul(vec![x.clone(), y.clone()])]);
        }
    }
    out.into_iter().map(tidy).collect()
}

fn poly_add(a: &[Expr], b: &[Expr]) -> Vec<Expr> {
    let n = a.len().max(b.len());
    let at = |p: &[Expr], k: usize| -> Expr {
        match (k + p.len()).checked_sub(n) {
            Some(i) => p[i].clone(),
            None => Expr::zero(),
        }
    };
    trimmed(
        (0..n)
            .map(|k| tidy(add(vec![at(a, k), at(b, k)])))
            .collect(),
    )
}

fn poly_scaled(a: &[Expr], by: &Expr) -> Vec<Expr> {
    a.iter()
        .map(|c| tidy(mul(vec![c.clone(), by.clone()])))
        .collect()
}

fn poly_at(p: &[Expr], x: &Expr) -> Expr {
    tidy(p.iter().fold(Expr::zero(), |s, c| {
        add(vec![mul(vec![s, x.clone()]), c.clone()])
    }))
}

/// The slope of a polynomial.
fn poly_slope(p: &[Expr]) -> Vec<Expr> {
    let n = p.len();
    match n {
        0 | 1 => vec![Expr::zero()],
        _ => p[..n - 1]
            .iter()
            .enumerate()
            .map(|(k, c)| tidy(mul(vec![c.clone(), Expr::int((n - 1 - k) as i64)])))
            .collect(),
    }
}

/// A polynomial along the imaginary axis: p(j*w) = r(w) + j*i(w), both with the
/// coefficients of p up to signs.
fn along_axis(p: &[Expr]) -> (Vec<Expr>, Vec<Expr>) {
    let n = p.len();
    let (mut real, mut imaginary) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for (k, c) in p.iter().enumerate() {
        let power = n - 1 - k;
        let signed = if (power / 2) % 2 == 1 {
            tidy(neg(c.clone()))
        } else {
            c.clone()
        };
        match power % 2 {
            0 => {
                real.push(signed);
                imaginary.push(Expr::zero());
            }
            _ => {
                real.push(Expr::zero());
                imaginary.push(signed);
            }
        }
    }
    (real, imaginary)
}

impl Sys {
    fn var(&self) -> Expr {
        Expr::sym(if self.ts.is_some() { "z" } else { "s" })
    }

    fn expr(&self) -> Expr {
        let var = self.var();
        tidy(div(poly_expr(&self.num, &var), poly_expr(&self.den, &var)))
    }

    fn object(&self) -> Expr {
        let mut args = vec![Expr::list(self.num.clone()), Expr::list(self.den.clone())];
        if let Some(ts) = &self.ts {
            args.push(ts.clone());
        }
        Expr::call("tf", args)
    }

    fn order(&self) -> usize {
        self.den.len() - 1
    }

    fn with(&self, num: Vec<Expr>, den: Vec<Expr>) -> Sys {
        Sys {
            num: trimmed(num),
            den: trimmed(den),
            ts: self.ts.clone(),
        }
    }

    fn at(&self, x: &Expr) -> Expr {
        tidy(div(poly_at(&self.num, x), poly_at(&self.den, x)))
    }
}

pub(crate) fn is_system(e: &Expr) -> bool {
    ["tf", "zpk", "ss"].iter().any(|k| e.fn_args(k).is_some())
}

/// The matrices of `ss(A, B, C, D[, Ts])`.
fn state_space(e: &Expr) -> Option<([Rows; 4], Option<Expr>)> {
    let args = e.fn_args("ss").filter(|a| a.len() == 4 || a.len() == 5)?;
    // a number is a matrix of one entry; a flat list is a column of B and a row of C and D
    let m = |k: usize| -> Rows {
        match args[k].as_list() {
            Some(items) if items.iter().all(|x| x.as_list().is_some()) && !items.is_empty() => {
                items
                    .iter()
                    .map(|r| r.as_list().unwrap_or_default().to_vec())
                    .collect()
            }
            Some(items) if k == 1 => items.iter().map(|x| vec![x.clone()]).collect(),
            Some(items) => vec![items.to_vec()],
            None => vec![vec![args[k].clone()]],
        }
    };
    let (a, b, c, mut d) = (m(0), m(1), m(2), m(3));
    if d.len() == 1 && d[0].len() == 1 && d[0][0].is_zero() && !b.is_empty() {
        d = vec![vec![Expr::zero(); b[0].len()]; c.len()];
    }
    Some(([a, b, c, d], args.get(4).cloned()))
}

/// det(s*I - A) and the matrices of adj(s*I - A) from the highest power of s (Faddeev and
/// LeVerrier).
fn resolvent(a: &Rows) -> Result<(Vec<Expr>, Vec<Rows>), String> {
    let n = a.len();
    let mut coeffs = vec![Expr::one()];
    let mut parts = Vec::new();
    let mut m = matrix::identity(n);
    for k in 1..=n {
        parts.push(m.clone());
        let am = matrix::matmul(a, &m)?;
        let trace = add((0..n).map(|i| am[i][i].clone()).collect());
        let c = tidy(div(neg(trace), Expr::int(k as i64)));
        m = am
            .iter()
            .enumerate()
            .map(|(i, row)| {
                row.iter()
                    .enumerate()
                    .map(|(j, x)| match i == j {
                        true => tidy(add(vec![x.clone(), c.clone()])),
                        false => tidy(x.clone()),
                    })
                    .collect()
            })
            .collect();
        coeffs.push(c);
    }
    Ok((coeffs, parts))
}

/// The transfer function of `C*(s*I - A)^-1*B + D` for one input and one output.
fn ss_to_tf(m: &[Rows; 4]) -> Result<(Vec<Expr>, Vec<Expr>), String> {
    let [a, b, c, d] = m;
    let n = a.len();
    if a.iter().any(|r| r.len() != n) || b.len() != n || c.iter().any(|r| r.len() != n) {
        return Err("ss(A, B, C, D): A is square, B has its rows and C its columns".into());
    }
    if b.iter().any(|r| r.len() != 1) || c.len() != 1 || d.len() != 1 || d[0].len() != 1 {
        return Err("this needs a system with one input and one output".into());
    }
    let (den, parts) = resolvent(a)?;
    let mut num = vec![Expr::zero()];
    for part in &parts {
        let through = matrix::matmul(&matrix::matmul(c, part)?, b)?;
        num.push(through[0][0].clone());
    }
    let num = poly_add(&num, &poly_scaled(&den, &d[0][0]));
    Ok((num, den))
}

/// The companion form that MATLAB's tf2ss gives.
fn tf_to_ss(num: &[Expr], den: &[Expr]) -> Result<[Rows; 4], String> {
    let n = den.len() - 1;
    if num.len() > den.len() {
        return Err("a state space needs a transfer function whose numerator is of no higher degree than its denominator".into());
    }
    let lead = &den[0];
    let a: Vec<Expr> = den[1..]
        .iter()
        .map(|c| tidy(div(c.clone(), lead.clone())))
        .collect();
    let mut b = vec![Expr::zero(); den.len() - num.len()];
    b.extend(num.iter().map(|c| tidy(div(c.clone(), lead.clone()))));
    let d = b[0].clone();
    let c: Vec<Expr> = (0..n)
        .map(|k| tidy(sub(b[k + 1].clone(), mul(vec![d.clone(), a[k].clone()]))))
        .collect();
    let mut top = vec![
        a.iter()
            .map(|x| tidy(neg(x.clone())))
            .collect::<Vec<Expr>>(),
    ];
    for i in 1..n {
        top.push((0..n).map(|j| Expr::int(i64::from(j + 1 == i))).collect());
    }
    let input: Rows = (0..n).map(|i| vec![Expr::int(i64::from(i == 0))]).collect();
    Ok([top, input, vec![c], vec![vec![d]]])
}

/// A sampled system as one of running time with the same values: its unit circle becomes
/// the imaginary axis by z = (1 + s)/(1 - s), the frequency w the place tan(w*Ts/2).
fn unrolled(sys: &Sys) -> Sys {
    let n = sys.num.len().max(sys.den.len()) - 1;
    let turn = |p: &[Expr]| -> Vec<Expr> {
        let mut total = vec![Expr::zero()];
        for (i, c) in p.iter().enumerate() {
            let power = p.len() - 1 - i;
            let mut term = vec![c.clone()];
            for _ in 0..power {
                term = poly_mul(&term, &[Expr::one(), Expr::one()]);
            }
            for _ in power..n {
                term = poly_mul(&term, &[Expr::int(-1), Expr::one()]);
            }
            total = poly_add(&total, &term);
        }
        trimmed(total)
    };
    Sys {
        num: turn(&sys.num),
        den: turn(&sys.den),
        ts: None,
    }
}

/// The time between the samples of a sampled system; 1 where it is not said.
fn sample_time(sys: &Sys, prec: u32) -> Option<Float> {
    let ts = sys.ts.as_ref()?;
    Some(match numeric::real_value(ts, numeric::digits() + 10) {
        Some(t) if t > 0 => Float::with_val(prec, t),
        _ => Float::with_val(prec, 1),
    })
}

/// The frequency of a sampled system for a place on the axis of its unrolled form.
fn rolled(place: &Float, ts: &Option<Float>) -> Float {
    let prec = place.prec();
    match ts {
        None => place.clone(),
        Some(t) if place.is_infinite() => Float::with_val(prec, rug::float::Constant::Pi) / t,
        Some(t) => Float::with_val(prec, place.atan_ref()) * 2u32 / t,
    }
}

/// Coefficients of decimal numbers without what rounding has left where zero belongs.
fn dusted(p: Vec<Expr>) -> Vec<Expr> {
    if !p.iter().any(Expr::has_float) {
        return p;
    }
    let prec = crate::mfun::bits();
    let sizes: Vec<Option<Float>> = p
        .iter()
        .map(|c| {
            complex_of(c, prec)
                .ok()
                .map(|z| Float::with_val(prec, z.abs_ref()))
        })
        .collect();
    let top = sizes
        .iter()
        .flatten()
        .fold(Float::new(prec), |m, s| m.max(s));
    let least = top * Float::with_val(prec, Float::i_pow_u(10, numeric::digits() + 8)).recip();
    p.into_iter()
        .zip(sizes)
        .map(|(c, size)| match size {
            Some(s) if s <= least => Expr::zero(),
            _ => c,
        })
        .collect()
}

/// Whether an imaginary number stands in it.
fn has_i(e: &Expr) -> bool {
    e.any(&|x| x.as_num().is_some_and(|n| !n.is_real()))
}

fn complex_of(e: &Expr, prec: u32) -> Result<Complex, String> {
    numeric::evalf(e, prec).map_err(|_| format!("{e} is no number"))
}

fn to_cmat(m: &Rows, prec: u32) -> Result<CMat, String> {
    m.iter()
        .map(|r| r.iter().map(|x| complex_of(x, prec)).collect())
        .collect()
}

fn c_mul(a: &CMat, b: &CMat) -> CMat {
    let prec = a[0][0].prec().0;
    a.iter()
        .map(|row| {
            (0..b[0].len())
                .map(|j| {
                    row.iter().zip(b).fold(Complex::new(prec), |s, (x, r)| {
                        s + Complex::with_val(prec, x * &r[j])
                    })
                })
                .collect()
        })
        .collect()
}

fn c_add(a: &CMat, b: &CMat, sign: i32) -> CMat {
    a.iter()
        .zip(b)
        .map(|(r, s)| {
            r.iter()
                .zip(s)
                .map(|(x, y)| match sign >= 0 {
                    true => Complex::with_val(x.prec(), x + y),
                    false => Complex::with_val(x.prec(), x - y),
                })
                .collect()
        })
        .collect()
}

fn c_turned(a: &CMat) -> CMat {
    (0..a[0].len())
        .map(|j| a.iter().map(|r| r[j].clone()).collect())
        .collect()
}

fn c_unit(n: usize, prec: u32) -> CMat {
    (0..n)
        .map(|i| {
            (0..n)
                .map(|j| Complex::with_val(prec, u32::from(i == j)))
                .collect()
        })
        .collect()
}

fn c_size(a: &CMat) -> Float {
    let prec = a[0][0].prec().0;
    a.iter().flatten().fold(Float::new(prec), |s, z| {
        s.max(&Float::with_val(prec, z.abs_ref()))
    })
}

/// The solution X of `A'*X + X*A - X*G*X + Q = 0` that makes `A - G*X` stable, by the sign
/// of the matrix [[A, -G], [-Q, -A']]: steps of Newton towards the matrix of signs.
/// A balanced form: [A, B, C, D], the time between samples, the singular values of Hankel.
type Balanced = ([CMat; 4], Option<Expr>, Vec<Float>);

/// The singular values of Hankel from their squares, the largest first: one that is as
/// small against the largest as the rounding of the Gramians is no value, it is 0.
fn weights(squares: Vec<Float>, prec: u32) -> Vec<Float> {
    let roots: Vec<Float> = squares
        .into_iter()
        .map(|x| if x > 0 { x.sqrt() } else { Float::new(prec) })
        .collect();
    let least = roots
        .first()
        .map_or_else(|| Float::new(prec), |top| top.clone() >> (prec / 2 - 12));
    roots
        .into_iter()
        .map(|x| if x > least { x } else { Float::new(prec) })
        .collect()
}

/// Whether a list holds a system, in the lists inside it too.
fn lists_a_system(e: &Expr) -> bool {
    e.as_list()
        .is_some_and(|items| items.iter().any(|x| is_system(x) || lists_a_system(x)))
}

/// Whether a sum, a product or a power holds a system to be joined.
pub(crate) fn joins_a_system(e: &Expr) -> bool {
    match e {
        Expr::Add(parts) | Expr::Mul(parts) => {
            parts.iter().any(|x| is_system(x) || joins_a_system(x))
        }
        Expr::Pow(base, _) => is_system(base) || joins_a_system(base),
        _ => false,
    }
}

/// The lower triangle L with L*L' = M for a symmetric matrix of positive values.
fn cholesky(m: &CMat) -> Option<CMat> {
    let size = m.len();
    let prec = crate::mfun::bits();
    let mut lower: CMat = vec![vec![Complex::new(prec); size]; size];
    let top = m.iter().flatten().fold(Float::new(prec), |most, x| {
        most.max(&Float::with_val(prec, x.abs_ref()))
    });
    let least = top * (Float::with_val(prec, 1) >> (prec / 2));
    for i in 0..size {
        for j in 0..=i {
            let mut sum = m[i][j].clone();
            for (x, y) in lower[i].iter().zip(&lower[j]).take(j) {
                sum -= Complex::with_val(prec, x * Complex::with_val(prec, y.conj_ref()));
            }
            if i == j {
                let own = sum.real().clone();
                if own <= least {
                    return None;
                }
                lower[i][i] = Complex::with_val(prec, own.sqrt());
            } else {
                lower[i][j] = Complex::with_val(prec, &sum / &lower[j][j]);
            }
        }
    }
    Some(lower)
}

fn riccati(a: &CMat, g: &CMat, q: &CMat) -> Result<CMat, String> {
    let n = a.len();
    let prec = a[0][0].prec().0;
    let at = c_turned(a);
    let mut z: CMat = (0..2 * n)
        .map(|i| {
            (0..2 * n)
                .map(|j| match (i < n, j < n) {
                    (true, true) => a[i][j].clone(),
                    (true, false) => -g[i][j - n].clone(),
                    (false, true) => -q[i - n][j].clone(),
                    (false, false) => -at[i - n][j - n].clone(),
                })
                .collect()
        })
        .collect();
    let unit = c_unit(2 * n, prec);
    let none = || {
        "the equation of Riccati has no stabilizing solution (a pole on the border of stability, or a system that cannot be steered)".to_string()
    };
    let mut settled = false;
    for _ in 0..200 {
        let inverse = solved(&z, &unit).ok_or_else(none)?;
        let next: CMat = c_add(&z, &inverse, 1)
            .into_iter()
            .map(|r| r.into_iter().map(|x| x / 2u32).collect())
            .collect();
        let moved = c_size(&c_add(&next, &z, -1));
        let size = c_size(&next);
        z = next;
        if moved <= size >> (prec - 24) {
            settled = true;
            break;
        }
    }
    if !settled {
        return Err(none());
    }
    // [W12; W22 + I] * X = -[W11 + I; W21], by least squares
    let left: CMat = (0..2 * n)
        .map(|i| {
            (0..n)
                .map(|j| match i >= n && i - n == j {
                    true => Complex::with_val(prec, &z[i][n + j] + 1u32),
                    false => z[i][n + j].clone(),
                })
                .collect()
        })
        .collect();
    let right: CMat = (0..2 * n)
        .map(|i| {
            (0..n)
                .map(|j| match i < n && i == j {
                    true => -Complex::with_val(prec, &z[i][j] + 1u32),
                    false => -z[i][j].clone(),
                })
                .collect()
        })
        .collect();
    let lt: CMat = c_turned(&left)
        .into_iter()
        .map(|r| r.into_iter().map(|x| x.conj()).collect())
        .collect();
    solved(&c_mul(&lt, &left), &c_mul(&lt, &right)).ok_or_else(none)
}

/// The solution X of `A'*X*A - X - A'*X*B*(R + B'*X*B)^-1*B'*X*A + Q = 0` by doubling:
/// every step squares the map of the closed loop.
fn riccati_sampled(a: &CMat, g: &CMat, q: &CMat) -> Result<CMat, String> {
    let n = a.len();
    let prec = a[0][0].prec().0;
    let unit = c_unit(n, prec);
    let none = || "the sampled equation of Riccati has no stabilizing solution".to_string();
    let (mut ak, mut gk, mut hk) = (a.clone(), g.clone(), q.clone());
    for _ in 0..200 {
        let w = c_add(&unit, &c_mul(&gk, &hk), 1);
        let w_inverse = solved(&w, &unit).ok_or_else(none)?;
        let aw = c_mul(&ak, &w_inverse);
        let akt = c_turned(&ak);
        let next_a = c_mul(&aw, &ak);
        let next_g = c_add(&gk, &c_mul(&c_mul(&aw, &gk), &akt), 1);
        let next_h = c_add(&hk, &c_mul(&c_mul(&c_mul(&akt, &hk), &w_inverse), &ak), 1);
        let moved = c_size(&c_add(&next_h, &hk, -1));
        let size = c_size(&next_h) + 1u32;
        (ak, gk, hk) = (next_a, next_g, next_h);
        if moved <= size >> (prec - 24) {
            return Ok(hk);
        }
    }
    Err(none())
}

impl Session {
    /// The coefficients of a polynomial in a variable, from the highest power.
    /// The matrices of a stable system and its Gramians of steering and of watching.
    fn gramians(
        &mut self,
        e: &Expr,
        who: &str,
    ) -> Result<([Rows; 4], Option<Expr>, CMat, CMat), String> {
        let prec = crate::mfun::bits();
        let (m, ts) = self.with_states(e, who)?;
        let numbers =
            decomp::to_complex(&m[0]).map_err(|_| format!("{who}: a system of numbers"))?;
        let (_, triangle) = decomp::schur(&numbers)?;
        let stable = (0..triangle.len()).all(|k| match &ts {
            None => triangle[k][k].real().is_sign_negative() && !triangle[k][k].real().is_zero(),
            Some(_) => Float::with_val(prec, triangle[k][k].abs_ref()) < 1,
        });
        if !stable {
            return Err(format!(
                "{who}: the system is not stable, its Gramians have no values"
            ));
        }
        let name = if ts.is_some() { "dlyap" } else { "lyap" };
        let steering = matrix::matmul(&m[1], &matrix::transpose(&m[1]))?;
        let watching = matrix::matmul(&matrix::transpose(&m[2]), &m[2])?;
        let wc = self.lyapunov(
            name,
            &[matrix::from_rows(m[0].clone()), matrix::from_rows(steering)],
        )?;
        let wo = self.lyapunov(
            name,
            &[
                matrix::from_rows(matrix::transpose(&m[0])),
                matrix::from_rows(watching),
            ],
        )?;
        let numbers = |e: &Expr| -> Result<CMat, String> {
            to_cmat(&matrix::rows(e).unwrap_or_default(), prec)
                .map_err(|_| format!("{who}: a system of numbers"))
        };
        let (wc, wo) = (numbers(&wc)?, numbers(&wo)?);
        Ok((m, ts, wc, wo))
    }

    /// The balanced form of a stable system that can be steered: [A, B, C, D], the time
    /// between samples, and the values of Hankel (both Gramians are the diagonal of them).
    fn balanced(&mut self, e: &Expr, who: &str) -> Result<Balanced, String> {
        let prec = crate::mfun::bits();
        let (m, ts, wc, wo) = self.gramians(e, who)?;
        let size = wc.len();
        // states that are not steered or not seen are taken out first: the balanced form is
        // that of what is left
        let without = |session: &mut Session, what: &str| -> Result<Balanced, String> {
            let less = session.call("minreal", std::slice::from_ref(e))?;
            match session.with_states(&less, who)?.0[0].len() < size {
                true => session.balanced(&less, who),
                false => Err(format!("{who}: {what}")),
            }
        };
        let Some(lower) = cholesky(&wc) else {
            return without(self, "the input does not reach every state of the system");
        };
        let inner = c_mul(&c_mul(&c_turned(&lower), &wo), &lower);
        let (u, t) = decomp::schur(&inner)?;
        let mut order: Vec<usize> = (0..size).collect();
        order.sort_by(|i, j| {
            t[*j][*j]
                .real()
                .partial_cmp(t[*i][*i].real())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let values = weights(
            order.iter().map(|k| t[*k][*k].real().clone()).collect(),
            prec,
        );
        if values.iter().any(|v| v.is_zero()) {
            return without(self, "the output does not show every state of the system");
        }
        let unit = c_unit(size, prec);
        let lower_inverse =
            solved(&lower, &unit).ok_or_else(|| format!("{who}: the Gramian has no inverse"))?;
        let spread = c_mul(&lower, &u);
        let gathered = c_mul(&c_turned(&u), &lower_inverse);
        // T = Lc U S^(-1/2) and its inverse S^(1/2) U' Lc^-1, the states in the order of their values
        let mut to: CMat = vec![vec![Complex::new(prec); size]; size];
        let mut back = to.clone();
        for (place, k) in order.iter().enumerate() {
            let root = values[place].clone().sqrt();
            // (the sign of a state is free: its largest part in T counts as positive)
            let largest = (0..size)
                .max_by(|i, j| {
                    let (x, y) = (
                        Float::with_val(prec, spread[*i][*k].abs_ref()),
                        Float::with_val(prec, spread[*j][*k].abs_ref()),
                    );
                    x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(0);
            let sign = if spread[largest][*k].real().is_sign_negative() {
                -1
            } else {
                1
            };
            for i in 0..size {
                to[i][place] = Complex::with_val(prec, &spread[i][*k] / &root) * sign;
                back[place][i] = Complex::with_val(prec, &gathered[*k][i] * &root) * sign;
            }
        }
        let numbers =
            |rows: &Rows| to_cmat(rows, prec).map_err(|_| format!("{who}: a system of numbers"));
        let (a, b, c, d) = (
            numbers(&m[0])?,
            numbers(&m[1])?,
            numbers(&m[2])?,
            numbers(&m[3])?,
        );
        Ok((
            [
                c_mul(&c_mul(&back, &a), &to),
                c_mul(&back, &b),
                c_mul(&c, &to),
                d,
            ],
            ts,
            values,
        ))
    }

    /// The system whose answer to an impulse is the output of a system left alone from the
    /// state x0: C*inv(s*I - A)*x0, and z times that for a sampled one.
    fn left_alone(&mut self, system: &Expr, x0: &Expr, who: &str) -> Result<Sys, String> {
        let (m, ts) = self.with_states(system, who)?;
        let start: Vec<Expr> = list_of(x0).iter().flat_map(list_of).collect();
        if start.len() != m[0].len() {
            return Err(format!(
                "{who}(system, x0, ...): x0 has the {} states of the system",
                m[0].len()
            ));
        }
        let column: Rows = start.into_iter().map(|x| vec![x]).collect();
        let (mut num, den) =
            ss_to_tf(&[m[0].clone(), column, m[2].clone(), vec![vec![Expr::zero()]]])?;
        if ts.is_some() {
            num.push(Expr::zero());
        }
        Ok(Sys {
            num: trimmed(num),
            den,
            ts,
        })
    }

    fn poly_of(&mut self, p: &Expr, var: &Expr, who: &str) -> Result<Vec<Expr>, String> {
        if !p.has(var) {
            return Ok(vec![p.clone()]);
        }
        let all = self
            .call("sym2poly", &[p.clone(), var.clone()])
            .map_err(|_| format!("{who}: {p} is no polynomial in {var}"))?;
        Ok(trimmed(list_of(&all)))
    }

    /// A ratio of polynomials in s or z as a system.
    fn sys_of_expr(&mut self, e: &Expr, ts: Option<Expr>, who: &str) -> Result<Sys, String> {
        let sampled = ts.is_some() || (e.has(&Expr::sym("z")) && !e.has(&Expr::sym("s")));
        let var = Expr::sym(if sampled { "z" } else { "s" });
        let both = self.call("numden", &[tidy(e.clone())])?;
        let [num, den] = &list_of(&both)[..] else {
            return Err(format!("{who}: {e} is no ratio of polynomials"));
        };
        Ok(Sys {
            num: self.poly_of(num, &var, who)?,
            den: self.poly_of(den, &var, who)?,
            ts: match (sampled, ts) {
                (true, None) => Some(Expr::int(-1)),
                (_, ts) => ts,
            },
        })
    }

    fn sys_of(&mut self, e: &Expr, who: &str) -> Result<Sys, String> {
        if let Some(args) = e.fn_args("tf").filter(|a| a.len() == 2 || a.len() == 3) {
            let (num, den) = (trimmed(list_of(&args[0])), trimmed(list_of(&args[1])));
            if den.len() == 1 && den[0].is_zero() {
                return Err(format!(
                    "{who}: the denominator of a transfer function is not zero"
                ));
            }
            return Ok(Sys {
                num,
                den,
                ts: args.get(2).cloned(),
            });
        }
        if let Some(args) = e.fn_args("zpk").filter(|a| a.len() == 3 || a.len() == 4) {
            let poly = |s: &mut Session, roots: &Expr| -> Result<Vec<Expr>, String> {
                match list_of(roots).is_empty() || roots.as_list().is_some_and(|l| l.is_empty()) {
                    true => Ok(vec![Expr::one()]),
                    false => Ok(list_of(&s.call("poly", &[Expr::list(list_of(roots))])?)),
                }
            };
            let num = poly_scaled(&poly(self, &args[0])?, &args[2]);
            let den = poly(self, &args[1])?;
            return Ok(Sys {
                num,
                den,
                ts: args.get(3).cloned(),
            });
        }
        if let Some((m, ts)) = state_space(e) {
            let (num, den) = ss_to_tf(&m).map_err(|e| format!("{who}: {e}"))?;
            return Ok(Sys {
                num: trimmed(num),
                den,
                ts,
            });
        }
        if e.as_list().is_some() || text_of(e).is_some() {
            return Err(format!(
                "{who} needs a system: tf(num, den), zpk(zeros, poles, gain), ss(A, B, C, D) or a ratio of polynomials in s (in z for a sampled system)"
            ));
        }
        self.sys_of_expr(e, None, who)
    }

    /// The matrices of a system that has states (a gain alone has none).
    fn with_states(&mut self, e: &Expr, who: &str) -> Result<([Rows; 4], Option<Expr>), String> {
        let (m, ts) = self.matrices(e, who)?;
        if m[0].first().is_none_or(|row| row.is_empty()) {
            return Err(format!(
                "{who}: the system has no states, it is a gain alone"
            ));
        }
        Ok((m, ts))
    }

    /// The matrices of a system: of ss(...) as they are, else the companion form.
    fn matrices(&mut self, e: &Expr, who: &str) -> Result<([Rows; 4], Option<Expr>), String> {
        if let Some(found) = state_space(e) {
            return Ok(found);
        }
        let sys = self.sys_of(e, who)?;
        Ok((
            tf_to_ss(&sys.num, &sys.den).map_err(|e| format!("{who}: {e}"))?,
            sys.ts,
        ))
    }

    fn roots_of(&mut self, p: &[Expr]) -> Result<Vec<Expr>, String> {
        match p.len() {
            0 | 1 => Ok(Vec::new()),
            _ => Ok(list_of(&self.call("roots", &[Expr::list(p.to_vec())])?)),
        }
    }

    fn real_of(&mut self, e: &Expr) -> Option<Float> {
        numeric::real_value(e, numeric::digits() + 10)
    }

    /// Whether every pole lies where a system is stable: left of the imaginary axis, or
    /// inside the unit circle for a sampled one.
    fn stable(&mut self, sys: &Sys) -> Result<bool, String> {
        let poles = self.roots_of(&sys.den)?;
        let prec = crate::mfun::bits();
        for p in &poles {
            let z = complex_of(p, prec)?;
            let inside = match sys.ts {
                Some(_) => Float::with_val(prec, z.abs_ref()) < 1u32,
                None => z.real().is_sign_negative() && !z.real().is_zero(),
            };
            if !inside {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// The point of the plane that stands for the frequency w: i*w, or e^(i*w*Ts).
    fn point(sys: &Sys, w: &Expr) -> Expr {
        match &sys.ts {
            None => mul(vec![Expr::i(), w.clone()]),
            Some(ts) => {
                let ts = if order_of(ts) == Some(std::cmp::Ordering::Greater) {
                    ts.clone()
                } else {
                    Expr::one()
                };
                pow(Expr::e(), mul(vec![Expr::i(), w.clone(), ts]))
            }
        }
    }

    /// The frequencies for a picture: three decades around the zeros and poles.
    fn frequencies(&mut self, sys: &Sys, count: usize) -> Result<Vec<Expr>, String> {
        let prec = 64;
        let mut sizes: Vec<f64> = Vec::new();
        for p in self
            .roots_of(&sys.den)?
            .iter()
            .chain(self.roots_of(&sys.num)?.iter())
        {
            let z = complex_of(p, prec)?;
            let size = match &sys.ts {
                None => Float::with_val(prec, z.abs_ref()).to_f64(),
                // (the frequency of a point of the sampled plane: |ln z|)
                Some(_) => {
                    Float::with_val(prec, Complex::with_val(prec, z.ln_ref()).abs_ref()).to_f64()
                }
            };
            if size.is_finite() && size > 1e-9 {
                sizes.push(size);
            }
        }
        let (mut low, mut high) = (
            sizes.iter().copied().fold(f64::INFINITY, f64::min),
            sizes.iter().copied().fold(0.0, f64::max),
        );
        if sizes.is_empty() {
            (low, high) = (1.0, 1.0);
        }
        // whole decades: the powers of ten around the smallest and the largest size
        let decade = |v: f64, up: bool| -> i32 {
            let mut k = 0i32;
            let mut p = 1.0f64;
            while p > v {
                p /= 10.0;
                k -= 1;
            }
            while p * 10.0 <= v {
                p *= 10.0;
                k += 1;
            }
            if up && p < v { k + 1 } else { k }
        };
        let (from, mut to) = (decade(low, false) - 1, decade(high, true) + 1);
        if let Some(ts) = &sys.ts {
            // a sampled system repeats beyond pi/Ts
            let nyquist = match self.real_of(ts).filter(|t| *t > 0) {
                Some(t) => std::f64::consts::PI / t.to_f64(),
                None => std::f64::consts::PI,
            };
            to = to.min(decade(nyquist, false));
            if to <= from {
                to = from + 1;
            }
        }
        Ok((0..count)
            .map(|k| {
                let power = Expr::rat(
                    i64::from(from) * (count as i64 - 1) + i64::from(to - from) * k as i64,
                    count as i64 - 1,
                );
                tidy(pow(Expr::int(10), power))
            })
            .collect())
    }

    /// The gain and the phase (in degrees, without jumps) of a system at frequencies.
    fn bode_data(&mut self, sys: &Sys, ws: &[Expr]) -> Result<(Vec<Float>, Vec<Float>), String> {
        let prec = crate::mfun::bits();
        let (mut gains, mut phases) = (Vec::new(), Vec::new());
        let half_turn = Float::with_val(prec, 180);
        let mut before: Option<Float> = None;
        for w in ws {
            let h = complex_of(&sys.at(&Self::point(sys, w)), prec)?;
            gains.push(Float::with_val(prec, h.abs_ref()));
            let mut phase = Float::with_val(prec, h.arg_ref()) * &half_turn
                / Float::with_val(prec, rug::float::Constant::Pi);
            if let Some(b) = &before {
                // (the nearest of the angles that differ by whole turns)
                let turns = (Float::with_val(prec, b - &phase) / 360u32).round();
                phase += turns * 360u32;
            }
            before = Some(phase.clone());
            phases.push(phase);
        }
        Ok((gains, phases))
    }

    /// The system that a function of several systems gives, as an object.
    fn combined(&mut self, e: Expr, ts: Option<Expr>, who: &str) -> Res {
        Ok(self.sys_of_expr(&e, ts, who)?.object())
    }

    fn shared_ts(a: &Sys, b: &Sys, who: &str) -> Result<Option<Expr>, String> {
        match (&a.ts, &b.ts) {
            (None, None) => Ok(None),
            (Some(x), Some(y)) if x == y || *y == Expr::int(-1) => Ok(Some(x.clone())),
            (Some(x), Some(y)) if *x == Expr::int(-1) => Ok(Some(y.clone())),
            (Some(_), None) | (None, Some(_)) => {
                // a number among sampled systems is a gain
                let gain = if a.ts.is_none() { a } else { b };
                match gain.order() == 0 && gain.num.len() == 1 {
                    true => Ok(a.ts.clone().or_else(|| b.ts.clone())),
                    false => Err(format!(
                        "{who}: a sampled system and one of running time do not go together"
                    )),
                }
            }
            _ => Err(format!("{who}: the systems are sampled at different times")),
        }
    }

    /// The answer in time to an input with the transform `drive`: an expression of t for
    /// running time; for a sampled system the first samples.
    fn response(&mut self, sys: &Sys, drive: Expr, times: Option<&Expr>, who: &str) -> Res {
        match &sys.ts {
            None => {
                let (s, t) = (Expr::sym("s"), Expr::sym("t"));
                let out = self.call("invlaplace", &[tidy(mul(vec![sys.expr(), drive])), s, t])?;
                match times {
                    None => Ok(out),
                    Some(at) => self.values_at(&out, at),
                }
            }
            Some(_) => {
                let count = match times {
                    Some(n) => {
                        crate::arrays::whole(n, "the number of samples", 1, 100_000)? as usize
                    }
                    None => 20,
                };
                // the input as a ratio of polynomials in z, then long division
                let all = self.sys_of_expr(&mul(vec![sys.expr(), drive]), sys.ts.clone(), who)?;
                samples(&all, count)
            }
        }
    }

    /// An answer in t at a time or at a list of times, as numbers.
    fn values_at(&mut self, answer: &Expr, at: &Expr) -> Res {
        let t = Expr::sym("t");
        let mut values = Vec::new();
        for x in list_of(at) {
            let v = crate::canon::subst(answer, &t, &x);
            values.push(match numeric::evalf_digits(&v, numeric::digits() + 6) {
                Ok(z) => numeric::to_float_expr(
                    Complex::with_val(crate::mfun::bits(), z.real()),
                    numeric::digits(),
                ),
                Err(_) => self.eval(&v)?,
            });
        }
        Ok(match at.as_list() {
            Some(_) => Expr::list(values),
            None => values.swap_remove(0),
        })
    }

    /// The answer of a system to an input u(t) at times, by the equations of its states
    /// x' = A*x + B*u, y = C*x + D*u from rest, step by step.
    fn simulated(&mut self, sys: &Sys, input: &Expr, at: &Expr) -> Res {
        let who = "lsim";
        let bits = crate::mfun::bits();
        let prec = bits + 64;
        let m = tf_to_ss(&sys.num, &sys.den)?;
        let [a, b, c, d] = [
            to_cmat(&m[0], prec)?,
            to_cmat(&m[1], prec)?,
            to_cmat(&m[2], prec)?,
            to_cmat(&m[3], prec)?,
        ];
        let size = a.len();
        let u = crate::mfun::Numeric::new(&["t".to_string()], input, prec);
        let mut slopes = |t: &Float, x: &[Complex]| -> Option<Vec<Complex>> {
            let drive = u.at(std::slice::from_ref(t)).ok()?;
            Some(
                (0..size)
                    .map(|i| {
                        let mut sum = Complex::with_val(prec, &b[i][0] * &drive);
                        for j in 0..size {
                            sum += Complex::with_val(prec, &a[i][j] * &x[j]);
                        }
                        sum
                    })
                    .collect(),
            )
        };
        let start = Float::new(prec);
        let mut values = Vec::new();
        for x in list_of(at) {
            let end = self
                .real_of(&x)
                .ok_or_else(|| format!("{who}: {x} is no time"))?;
            if end.is_sign_negative() && !end.is_zero() {
                return Err(format!(
                    "{who}: the answer starts at the time 0, {x} is before it"
                ));
            }
            let end = Float::with_val(prec, end);
            let state = crate::odesys::integrate(
                &mut slopes,
                &start,
                &end,
                vec![Complex::new(prec); size],
                bits,
            )?;
            let drive = u
                .at(std::slice::from_ref(&end))
                .map_err(|e| format!("{who}: {e}"))?;
            let mut y = Complex::with_val(prec, &d[0][0] * &drive);
            for j in 0..size {
                y += Complex::with_val(prec, &c[0][j] * &state[j]);
            }
            values.push(crate::mfun::value(y.real()));
        }
        Ok(match at.as_list() {
            Some(_) => Expr::list(values),
            None => values.swap_remove(0),
        })
    }

    /// The names that the transforms use (s, z, t) are no values of the one who asks while
    /// a system is worked on: `s := tf("s")` is a usual way to write systems.
    fn without_names<T>(&mut self, work: impl FnOnce(&mut Session) -> T) -> T {
        let hidden: Vec<(String, Expr)> = ["s", "z", "t", "_k"]
            .iter()
            .filter_map(|name| self.vars.remove(*name).map(|v| (name.to_string(), v)))
            .collect();
        let out = work(self);
        self.vars.extend(hidden);
        out
    }

    /// Systems that are added (`sum`) or multiplied, with numbers and ratios of polynomials
    /// in s or z among them: one system.
    /// A sum or a product of values: systems among them are joined, entry by entry where
    /// lists stand among them.
    pub(crate) fn joined(&mut self, sum: bool, items: Vec<Expr>) -> Res {
        let listed = items.iter().any(|e| e.as_list().is_some());
        if !listed && items.iter().any(is_system) {
            return self.systems_joined(sum, &items);
        }
        let systems = listed && items.iter().any(|e| is_system(e) || lists_a_system(e));
        let all = if sum {
            matrix::add(items)?
        } else {
            matrix::mul(items)?
        };
        if systems {
            self.systems_in_lists(all)
        } else {
            Ok(all)
        }
    }

    /// A power of values: of a system, or of a list that holds systems.
    pub(crate) fn raised(&mut self, base: Expr, exponent: Expr) -> Res {
        if is_system(&base) {
            return self.system_power(&base, &exponent);
        }
        let systems = lists_a_system(&base);
        let all = matrix::pow(base, exponent)?;
        if systems {
            self.systems_in_lists(all)
        } else {
            Ok(all)
        }
    }

    /// The entries of lists worked out again where systems wait in sums, products and powers.
    fn systems_in_lists(&mut self, e: Expr) -> Res {
        match &e {
            Expr::List(items) => Ok(Expr::list(
                items
                    .iter()
                    .map(|x| self.systems_in_lists(x.clone()))
                    .collect::<Result<_, _>>()?,
            )),
            _ if joins_a_system(&e) => self.eval(&e),
            _ => Ok(e),
        }
    }

    pub(crate) fn systems_joined(&mut self, sum: bool, items: &[Expr]) -> Res {
        let who = if sum {
            "a sum with a system"
        } else {
            "a product with a system"
        };
        self.without_names(|session| {
            let mut all: Option<Sys> = None;
            for item in items {
                let other = || {
                    format!(
                        "{who}: {item} is no system, no number and no ratio of polynomials in s or z"
                    )
                };
                let next = match is_system(item) {
                    true => session.sys_of(item, who)?,
                    false if item.as_list().is_some() || matches!(item, Expr::Rel(..)) => {
                        return Err(other());
                    }
                    false => session
                        .sys_of_expr(item, None, who)
                        .map_err(|_| other())?,
                };
                all = Some(match all {
                    None => next,
                    Some(so_far) => {
                        let ts = Self::shared_ts(&so_far, &next, who)?;
                        let parts = vec![so_far.expr(), next.expr()];
                        let e = if sum { add(parts) } else { mul(parts) };
                        session.sys_of_expr(&e, ts, who)?
                    }
                });
            }
            Ok(all.map_or_else(Expr::zero, |sys| sys.object()))
        })
    }

    /// A whole power of a system; the power -1 turns it upside down.
    pub(crate) fn system_power(&mut self, base: &Expr, exponent: &Expr) -> Res {
        let who = "a power of a system";
        if exponent.as_i64().is_none() {
            return Err(format!(
                "{who} is a whole one, as G^2 or G^-1: not {exponent}"
            ));
        }
        self.without_names(|session| {
            let sys = session.sys_of(base, who)?;
            if exponent.as_i64().is_some_and(|k| k < 0) && sys.num.iter().all(Expr::is_zero) {
                return Err(format!("{who}: a system that is zero has no inverse"));
            }
            let e = pow(sys.expr(), exponent.clone());
            session.combined(e, sys.ts.clone(), who)
        })
    }

    pub(crate) fn control(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        self.without_names(|session| session.control_of(name, a))
    }

    fn control_of(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        let n = a.len();
        // names that other values have too
        let first_system = a.first().is_some_and(is_system);
        // the operations by their names join systems as the signs do
        if a.iter().any(is_system) {
            let negated = |session: &mut Session, e: &Expr| {
                session.systems_joined(false, &[Expr::int(-1), e.clone()])
            };
            let joined = match (name, n) {
                ("plus", 2..) => Some(self.systems_joined(true, a)),
                ("times" | "mtimes", 2..) => Some(self.systems_joined(false, a)),
                ("minus", 2) => Some(
                    negated(self, &a[1])
                        .and_then(|less| self.systems_joined(true, &[a[0].clone(), less])),
                ),
                ("uminus", 1) => Some(negated(self, &a[0])),
                ("uplus", 1) => Some(Ok(a[0].clone())),
                ("rdivide" | "mrdivide", 2) | ("ldivide" | "mldivide", 2) => {
                    let (over, under) = match name.starts_with('l') || name == "mldivide" {
                        true => (&a[1], &a[0]),
                        false => (&a[0], &a[1]),
                    };
                    let turned = match is_system(under) {
                        true => self.system_power(under, &Expr::int(-1)),
                        false => Ok(pow(under.clone(), Expr::int(-1))),
                    };
                    Some(turned.and_then(|t| self.systems_joined(false, &[over.clone(), t])))
                }
                ("power" | "mpower", 2) if is_system(&a[0]) => {
                    Some(self.system_power(&a[0], &a[1]))
                }
                _ => None,
            };
            if joined.is_some() {
                return joined;
            }
        }
        match name {
            "series" if !(n == 2 && first_system && is_system(&a[1])) => return None,
            "norm" | "inv" | "sigma" if !first_system => return None,
            // (of a system or of a ratio in s or z; of anything else it is that of a matrix)
            "bandwidth"
                if !(first_system
                    || a.first().is_some_and(|e| e.has_sym("s") || e.has_sym("z"))) =>
            {
                return None;
            }
            "pade" if !(n == 2 && a[1].as_i64().is_some()) => return None,
            _ => {}
        }
        Some((|| -> Res {
            let who = name;
            Ok(match (name, n) {
                ("tf", 1..=3) => match (text_of(&a[0]), n) {
                    (Some(var @ ("s" | "z")), 1 | 2) => {
                        let ts = match (var, a.get(1)) {
                            ("z", Some(ts)) => Some(ts.clone()),
                            ("z", None) => Some(Expr::int(-1)),
                            ("s", None) => None,
                            _ => return Err("tf(\"s\") or tf(\"z\"[, Ts])".into()),
                        };
                        Sys { num: vec![Expr::one(), Expr::zero()], den: vec![Expr::one()], ts }.object()
                    }
                    (None, 1) => self.sys_of(&a[0], who)?.object(),
                    (None, 2 | 3) if a[0].as_list().is_some() || a[1].as_list().is_some() || (a[0].is_const() && a[1].is_const()) => {
                        let sys = self.sys_of(&Expr::call("tf", a.to_vec()), who)?;
                        sys.object()
                    }
                    (None, 2) => self.sys_of_expr(&a[0], Some(a[1].clone()), who)?.object(),
                    _ => return Err("tf([numerator], [denominator][, Ts]), tf(\"s\"), tf(system)".into()),
                },
                ("zpk", 1) => {
                    let sys = self.sys_of(&a[0], who)?;
                    let (zeros, poles) = (self.roots_of(&sys.num)?, self.roots_of(&sys.den)?);
                    let gain = tidy(div(sys.num[0].clone(), sys.den[0].clone()));
                    let mut args = vec![Expr::list(zeros), Expr::list(poles), gain];
                    args.extend(sys.ts);
                    Expr::call("zpk", args)
                }
                ("zpk", 3 | 4) => {
                    self.sys_of(&Expr::call("zpk", a.to_vec()), who)?;
                    Expr::call("zpk", a.to_vec())
                }
                ("ss", 1) => {
                    let (m, ts) = self.matrices(&a[0], who)?;
                    let mut args: Vec<Expr> = m.into_iter().map(matrix::from_rows).collect();
                    args.extend(ts);
                    Expr::call("ss", args)
                }
                ("ss", 4 | 5) => {
                    let (m, _) = state_space(&Expr::call("ss", a.to_vec())).ok_or("ss(A, B, C, D[, Ts]): four matrices")?;
                    let size = m[0].len();
                    if m[0].iter().any(|r| r.len() != size) || m[1].len() != size || m[2].iter().any(|r| r.len() != size) || m[3].len() != m[2].len() || m[3][0].len() != m[1][0].len() {
                        return Err("ss(A, B, C, D): A is n by n, B has n rows, C has n columns, D a row for every output and a column for every input".into());
                    }
                    Expr::call("ss", a.to_vec())
                }
                ("tfdata", 1) => {
                    let sys = self.sys_of(&a[0], who)?;
                    Expr::list(vec![Expr::list(sys.num), Expr::list(sys.den)])
                }
                ("zpkdata", 1) => {
                    let sys = self.sys_of(&a[0], who)?;
                    Expr::list(vec![
                        Expr::list(self.roots_of(&sys.num)?),
                        Expr::list(self.roots_of(&sys.den)?),
                        tidy(div(sys.num[0].clone(), sys.den[0].clone())),
                    ])
                }
                ("ssdata", 1) => {
                    let (m, _) = self.matrices(&a[0], who)?;
                    Expr::list(m.into_iter().map(matrix::from_rows).collect())
                }
                ("tf2sym", 1) => self.sys_of(&a[0], who)?.expr(),
                ("tf2ss", 2) => {
                    let m = tf_to_ss(&trimmed(list_of(&a[0])), &trimmed(list_of(&a[1])))?;
                    Expr::list(m.into_iter().map(matrix::from_rows).collect())
                }
                ("ss2tf", 4) => {
                    let (m, _) = state_space(&Expr::call("ss", a.to_vec())).ok_or("ss2tf(A, B, C, D)")?;
                    let (num, den) = ss_to_tf(&m)?;
                    Expr::list(vec![Expr::list(num), Expr::list(den)])
                }
                ("tf2zp", 2) => {
                    let (num, den) = (trimmed(list_of(&a[0])), trimmed(list_of(&a[1])));
                    Expr::list(vec![
                        Expr::list(self.roots_of(&num)?),
                        Expr::list(self.roots_of(&den)?),
                        tidy(div(num[0].clone(), den[0].clone())),
                    ])
                }
                ("zp2tf", 3) => {
                    let sys = self.sys_of(&Expr::call("zpk", a.to_vec()), who)?;
                    Expr::list(vec![Expr::list(sys.num), Expr::list(sys.den)])
                }
                ("ss2zp", 4) => {
                    let sys = self.sys_of(&Expr::call("ss", a.to_vec()), who)?;
                    Expr::list(vec![
                        Expr::list(self.roots_of(&sys.num)?),
                        Expr::list(self.roots_of(&sys.den)?),
                        tidy(div(sys.num[0].clone(), sys.den[0].clone())),
                    ])
                }
                ("zp2ss", 3) => {
                    let sys = self.sys_of(&Expr::call("zpk", a.to_vec()), who)?;
                    Expr::list(tf_to_ss(&sys.num, &sys.den)?.into_iter().map(matrix::from_rows).collect())
                }
                ("isct" | "isdt", 1) => {
                    let sys = self.sys_of(&a[0], who)?;
                    Expr::Bool(sys.ts.is_some() == (name == "isdt"))
                }
                ("isproper", 1) => {
                    let sys = self.sys_of(&a[0], who)?;
                    Expr::Bool(sys.num.len() <= sys.den.len())
                }
                ("isstable", 1) => {
                    let sys = self.sys_of(&a[0], who)?;
                    Expr::Bool(self.stable(&sys)?)
                }
                ("pole", 1) => {
                    let sys = self.sys_of(&a[0], who)?;
                    Expr::list(self.roots_of(&sys.den)?)
                }
                ("zero", 1) => {
                    let sys = self.sys_of(&a[0], who)?;
                    Expr::list(self.roots_of(&sys.num)?)
                }
                ("dcgain", 1) => {
                    let sys = self.sys_of(&a[0], who)?;
                    let at = if sys.ts.is_some() { Expr::one() } else { Expr::zero() };
                    match poly_at(&sys.den, &at).is_zero() {
                        true => Expr::inf(),
                        false => sys.at(&at),
                    }
                }
                ("damp", 1) => {
                    // [natural frequencies, damping ratios, poles], the slowest first
                    let sys = self.sys_of(&a[0], who)?;
                    let prec = crate::mfun::bits();
                    let mut rows: Vec<(Float, Expr, Expr, Expr)> = Vec::new();
                    for p in self.roots_of(&sys.den)? {
                        // (a sampled pole stands for ln(z)/Ts of running time)
                        let place = match &sys.ts {
                            None => p.clone(),
                            Some(ts) => {
                                let ts = if order_of(ts) == Some(std::cmp::Ordering::Greater) { ts.clone() } else { Expr::one() };
                                tidy(div(crate::funcs::func("ln", vec![p.clone()]), ts))
                            }
                        };
                        let z = complex_of(&place, prec)?;
                        let size = Float::with_val(prec, z.abs_ref());
                        let (wn, zeta) = match !place.any(&|x| matches!(x, Expr::Float(_))) {
                            true => {
                                let wn = self.call("abs", std::slice::from_ref(&place))?;
                                let zeta = match size.is_zero() {
                                    true => Expr::int(-1),
                                    false => {
                                        let real = self.call("re", std::slice::from_ref(&place))?;
                                        self.call("simplify", &[div(neg(real), wn.clone())])?
                                    }
                                };
                                (wn, zeta)
                            }
                            false => {
                                let zeta = match size.is_zero() {
                                    true => Float::with_val(prec, -1),
                                    false => -Float::with_val(prec, z.real() / &size),
                                };
                                (crate::mfun::value(&size), crate::mfun::value(&zeta))
                            }
                        };
                        rows.push((size, wn, zeta, p));
                    }
                    rows.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal));
                    Expr::list(vec![
                        Expr::list(rows.iter().map(|r| r.1.clone()).collect()),
                        Expr::list(rows.iter().map(|r| r.2.clone()).collect()),
                        Expr::list(rows.iter().map(|r| r.3.clone()).collect()),
                    ])
                }
                ("series" | "parallel", 2) | ("feedback", 2 | 3) => {
                    let (x, y) = (self.sys_of(&a[0], who)?, self.sys_of(&a[1], who)?);
                    let ts = Self::shared_ts(&x, &y, who)?;
                    let (g, h) = (x.expr(), y.expr());
                    // (the expressions of sampled systems are in z, the others in s: a gain has neither)
                    let e = match name {
                        "series" => mul(vec![g, h]),
                        "parallel" => add(vec![g, h]),
                        _ => {
                            let sign = match a.get(2) {
                                Some(s) if *s == Expr::one() => -1,
                                Some(s) if *s == Expr::int(-1) => 1,
                                Some(_) => return Err("feedback(G, H[, sign]): the sign is -1 (the usual, negative feedback) or +1".into()),
                                None => 1,
                            };
                            div(g.clone(), add(vec![Expr::one(), mul(vec![Expr::int(sign), g, h])]))
                        }
                    };
                    self.combined(e, ts, who)?
                }
                ("minreal", 1) => {
                    let sys = self.sys_of(&a[0], who)?;
                    let ts = sys.ts.clone();
                    let simpler = self.call("simplify", &[sys.expr()])?;
                    self.combined(simpler, ts, who)?
                }
                ("inv", 1) => {
                    let sys = self.sys_of(&a[0], who)?;
                    sys.with(sys.den.clone(), sys.num.clone()).object()
                }
                ("evalfr", 2) => {
                    let sys = self.sys_of(&a[0], who)?;
                    sys.at(&a[1])
                }
                ("freqresp", 2) => {
                    let sys = self.sys_of(&a[0], who)?;
                    let values: Vec<Expr> = list_of(&a[1]).iter().map(|w| sys.at(&Self::point(&sys, w))).collect();
                    match a[1].as_list() {
                        Some(_) => Expr::list(values),
                        None => values[0].clone(),
                    }
                }
                ("bode" | "nyquist", 1 | 2) => {
                    let sys = self.sys_of(&a[0], who)?;
                    let ws = match a.get(1) {
                        Some(w) => list_of(w),
                        None => self.frequencies(&sys, 61)?,
                    };
                    let shown = |v: &[Float]| Expr::list(v.iter().map(crate::mfun::value).collect());
                    let mut out = match name {
                        "bode" => {
                            let (gains, phases) = self.bode_data(&sys, &ws)?;
                            vec![shown(&gains), shown(&phases)]
                        }
                        _ => {
                            let prec = crate::mfun::bits();
                            let (mut re, mut im) = (Vec::new(), Vec::new());
                            for w in &ws {
                                let h = complex_of(&sys.at(&Self::point(&sys, w)), prec)?;
                                re.push(h.real().clone());
                                im.push(h.imag().clone());
                            }
                            vec![shown(&re), shown(&im)]
                        }
                    };
                    if a.get(1).is_none() {
                        out.push(Expr::list(ws));
                    }
                    Expr::list(out)
                }
                ("margin" | "allmargin", 1) => self.margins(&a[0], name == "allmargin")?,
                ("bandwidth", 1 | 2) => self.bandwidth(&a[0], a.get(1))?,
                ("hinfnorm", 1) | ("norm", 2) if name == "hinfnorm" || a.get(1).is_some_and(|k| k.is_inf() || word(k).as_deref() == Some("inf")) => self.peak_gain(&a[0])?,
                ("h2norm", 1) | ("norm", 1 | 2) => {
                    if n == 2 && a[1] != Expr::int(2) {
                        return Err("norm(system, 2) or norm(system, inf)".into());
                    }
                    self.h2(&a[0])?
                }
                ("step" | "impulse", 1 | 2) => {
                    let sys = self.sys_of(&a[0], who)?;
                    let drive = match (name, &sys.ts) {
                        ("step", None) => pow(Expr::sym("s"), Expr::int(-1)),
                        ("step", Some(_)) => div(Expr::sym("z"), sub(Expr::sym("z"), Expr::one())),
                        _ => Expr::one(),
                    };
                    self.response(&sys, drive, a.get(1), who)?
                }
                ("lsim", 2 | 3) => {
                    let sys = self.sys_of(&a[0], who)?;
                    match &sys.ts {
                        None => {
                            let (s, t) = (Expr::sym("s"), Expr::sym("t"));
                            let unknown = |e: &Expr| e.has_fn("laplace") || e.has_fn("invlaplace");
                            let notes = self.notes.len();
                            let found = match self.call("laplace", &[a[1].clone(), t, s]) {
                                Ok(drive) if !unknown(&drive) => self.response(&sys, drive, None, who).ok().filter(|out| !unknown(out)),
                                _ => None,
                            };
                            if found.is_none() {
                                self.notes.truncate(notes);
                            }
                            match (found, a.get(2)) {
                                (Some(out), None) => out,
                                (Some(out), Some(at)) => self.values_at(&out, at)?,
                                // (no transform of the input: the equations of the states, step by step)
                                (None, Some(at)) => self.simulated(&sys, &a[1], at)?,
                                (None, None) => {
                                    return Err("lsim: the transform of this input is not known; lsim(system, u(t), [times]) gives the values".into());
                                }
                            }
                        }
                        Some(_) => {
                            // the samples of the input through the equation of differences
                            let input = a[1].as_list().ok_or("lsim(sampled system, [samples of the input])")?;
                            filtered(&sys, input)?
                        }
                    }
                }
                ("stepinfo", 1) => self.stepinfo(&a[0])?,
                ("c2d", 2 | 3) => {
                    let found = self.sampled(&a[0], &a[1], a.get(2))?;
                    self.monic(&found, who)?
                }
                ("d2c", 1 | 2) => {
                    let found = self.running(&a[0], a.get(1))?;
                    self.monic(&found, who)?
                }
                ("pid", 1..=4) => {
                    let zero = Expr::zero();
                    let (kp, ki, kd, tf) = (&a[0], a.get(1).unwrap_or(&zero), a.get(2).unwrap_or(&zero), a.get(3).unwrap_or(&zero));
                    let s = Expr::sym("s");
                    let e = add(vec![
                        kp.clone(),
                        div(ki.clone(), s.clone()),
                        div(mul(vec![kd.clone(), s.clone()]), add(vec![mul(vec![tf.clone(), s]), Expr::one()])),
                    ]);
                    self.combined(e, None, who)?
                }
                ("pade", 2) => {
                    let order = crate::arrays::whole(&a[1], "the order", 0, 60)?;
                    // c_k = (2n - k)! n! / ((2n)! k! (n - k)!), with (-T)^k above and T^k below
                    let mut c = Expr::one();
                    let (mut num, mut den) = (Vec::new(), Vec::new());
                    for k in 0..=order {
                        if k > 0 {
                            c = tidy(mul(vec![c, Expr::rat(order - k + 1, (2 * order - k + 1) * k)]));
                        }
                        let power = pow(a[0].clone(), Expr::int(k));
                        den.push(tidy(mul(vec![c.clone(), power.clone()])));
                        num.push(tidy(mul(vec![c.clone(), power, Expr::int(if k % 2 == 0 { 1 } else { -1 })])));
                    }
                    num.reverse();
                    den.reverse();
                    let lead = den[0].clone();
                    let scale = |p: Vec<Expr>| Expr::list(p.into_iter().map(|x| tidy(div(x, lead.clone()))).collect());
                    Expr::list(vec![scale(num), scale(den)])
                }
                ("rlocus", 1 | 2) => {
                    let sys = self.sys_of(&a[0], who)?;
                    let gains = match a.get(1) {
                        Some(k) => list_of(k),
                        None => [0, 1, 2, 5, 10, 20, 50, 100].iter().map(|k| Expr::rat(*k, 10)).chain([20, 50, 100].iter().map(|k| Expr::int(*k))).collect(),
                    };
                    let mut out = Vec::new();
                    for k in &gains {
                        let closed = poly_add(&sys.den, &poly_scaled(&sys.num, k));
                        out.push(Expr::list(self.roots_of(&closed)?));
                    }
                    match a.get(1) {
                        Some(k) if k.as_list().is_none() => out.swap_remove(0),
                        Some(_) => Expr::list(out),
                        None => Expr::list(vec![Expr::list(out), Expr::list(gains)]),
                    }
                }
                ("esort" | "dsort", 1) => {
                    let prec = crate::mfun::bits();
                    let mut keyed: Vec<(Float, Expr)> = Vec::new();
                    for p in list_of(&a[0]) {
                        let z = complex_of(&p, prec)?;
                        keyed.push((
                            match name {
                                "esort" => z.real().clone(),
                                _ => Float::with_val(prec, z.abs_ref()),
                            },
                            p,
                        ));
                    }
                    keyed.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap_or(std::cmp::Ordering::Equal));
                    Expr::list(keyed.into_iter().map(|k| k.1).collect())
                }
                ("ctrb", 2) | ("obsv", 2) => {
                    let (m, other) = (matrix::rows(&a[0]).ok_or("a square matrix A")?, matrix::rows(&a[1]).ok_or("a matrix")?);
                    let size = m.len();
                    let mut blocks = vec![other.clone()];
                    for _ in 1..size {
                        let last = &blocks[blocks.len() - 1];
                        blocks.push(match name {
                            "ctrb" => matrix::matmul(&m, last)?,
                            _ => matrix::matmul(last, &m)?,
                        });
                    }
                    match name {
                        "ctrb" => matrix::from_rows(
                            (0..size)
                                .map(|i| blocks.iter().flat_map(|b| b[i].iter().map(|x| tidy(x.clone()))).collect())
                                .collect(),
                        ),
                        _ => matrix::from_rows(blocks.into_iter().flatten().map(|r| r.into_iter().map(tidy).collect()).collect()),
                    }
                }
                ("lyap", 2 | 3) | ("dlyap", 2) => self.lyapunov(name, a)?,
                ("gram", 2 | 3) => {
                    // gram(A, B) and gram(A, C, "o"), or gram(system, "c" | "o")
                    let (m, other, kind) = match (is_system(&a[0]), n) {
                        (true, 2) => {
                            let (m, _) = self.matrices(&a[0], who)?;
                            let kind = word(&a[1]).ok_or("gram(system, \"c\" | \"o\")")?;
                            let other = if kind == "o" { m[2].clone() } else { m[1].clone() };
                            (m[0].clone(), other, kind)
                        }
                        (false, _) => (
                            matrix::rows(&a[0]).ok_or("gram(A, B)")?,
                            matrix::rows(&a[1]).ok_or("gram(A, B)")?,
                            a.get(2).and_then(word).unwrap_or_else(|| "c".into()),
                        ),
                        _ => return Err("gram(system, \"c\" | \"o\") or gram(A, B[, \"o\"])".into()),
                    };
                    // (the integral is there only when every eigenvalue lies left of the axis)
                    let numbers = decomp::to_complex(&m).map_err(|_| "gram: a matrix of numbers".to_string())?;
                    let (_, triangle) = decomp::schur(&numbers)?;
                    if (0..triangle.len()).any(|k| !triangle[k][k].real().is_sign_negative() || triangle[k][k].real().is_zero()) {
                        return Err("gram: the system is not stable, its Gramian has no value".into());
                    }
                    let (m, q) = match kind.as_str() {
                        "c" => (m, matrix::matmul(&other, &matrix::transpose(&other))?),
                        "o" => (matrix::transpose(&m), matrix::matmul(&matrix::transpose(&other), &other)?),
                        _ => return Err("gram: \"c\" (steering) or \"o\" (watching)".into()),
                    };
                    self.lyapunov("lyap", &[matrix::from_rows(m), matrix::from_rows(q)])?
                }
                ("care" | "dare" | "icare" | "idare", 4) | ("lqr" | "dlqr", 4 | 5) | ("lqe", 5) => self.gains(name, a)?,
                ("acker" | "place", 3) => self.placed(name, a)?,
                ("hsvd", 1) => {
                    let prec = crate::mfun::bits();
                    let (_, _, wc, wo) = self.gramians(&a[0], who)?;
                    let (_, triangle) = decomp::schur(&c_mul(&wc, &wo))?;
                    let mut squares: Vec<Float> =
                        (0..triangle.len()).map(|k| triangle[k][k].real().clone()).collect();
                    squares.sort_by(|x, y| y.partial_cmp(x).unwrap_or(std::cmp::Ordering::Equal));
                    Expr::list(weights(squares, prec).iter().map(crate::mfun::value).collect())
                }
                ("balreal", 1) | ("balred", 2 | 3) => {
                    let prec = crate::mfun::bits();
                    let ([m, b, c, d], ts, values) = self.balanced(&a[0], who)?;
                    let object = |parts: [&CMat; 4], ts: &Option<Expr>| {
                        let mut args: Vec<Expr> = parts.iter().map(|p| decomp::from_complex(p)).collect();
                        if let Some(ts) = ts {
                            args.push(ts.clone());
                        }
                        Expr::call("ss", args)
                    };
                    if name == "balreal" {
                        return Ok(Expr::list(vec![
                            object([&m, &b, &c, &d], &ts),
                            Expr::list(values.iter().map(crate::mfun::value).collect()),
                        ]));
                    }
                    // (states that the system does not need are gone already)
                    let (size, given) = (m.len(), self.with_states(&a[0], who)?.0[0].len());
                    let kept = (crate::arrays::whole(&a[1], "the order", 1, given as i64)? as usize).min(size);
                    let cut = match a.get(2).and_then(word).as_deref() {
                        None | Some("matchdc") => false,
                        Some("truncate") => true,
                        Some(_) => return Err("balred(system, order[, \"matchdc\" or \"truncate\"])".into()),
                    };
                    let block = |x: &CMat, rows: std::ops::Range<usize>, columns: std::ops::Range<usize>| -> CMat {
                        rows.map(|i| x[i][columns.clone()].to_vec()).collect()
                    };
                    let (a11, b1, c1) = (block(&m, 0..kept, 0..kept), block(&b, 0..kept, 0..1), block(&c, 0..1, 0..kept));
                    if cut || kept == size {
                        return Ok(object([&a11, &b1, &c1, &d], &ts));
                    }
                    // the states that go are at rest: their part of the gain at the frequency 0 stays
                    let (a12, a21, a22) = (block(&m, 0..kept, kept..size), block(&m, kept..size, 0..kept), block(&m, kept..size, kept..size));
                    let (b2, c2) = (block(&b, kept..size, 0..1), block(&c, 0..1, kept..size));
                    let (inner, sign) = match &ts {
                        None => (a22, -1),
                        Some(_) => (c_add(&c_unit(size - kept, prec), &a22, -1), 1),
                    };
                    let unit = c_unit(size - kept, prec);
                    let inverse = solved(&inner, &unit).ok_or("balred: the states that go cannot be at rest")?;
                    let through = |left: &CMat, right: &CMat| c_mul(&c_mul(left, &inverse), right);
                    object(
                        [
                            &c_add(&a11, &through(&a12, &a21), sign),
                            &c_add(&b1, &through(&a12, &b2), sign),
                            &c_add(&c1, &through(&c2, &a21), sign),
                            &c_add(&d, &through(&c2, &b2), sign),
                        ],
                        &ts,
                    )
                }
                ("covar", 2) => {
                    let (m, ts, _, _) = self.gramians(&a[0], who)?;
                    let spread = poly_scaled(&matrix::matmul(&m[1], &matrix::transpose(&m[1]))?.concat(), &a[1]);
                    let size = m[0].len();
                    let noise: Rows = spread.chunks(size).map(|r| r.to_vec()).collect();
                    let lyap = if ts.is_some() { "dlyap" } else { "lyap" };
                    let states = self.lyapunov(lyap, &[matrix::from_rows(m[0].clone()), matrix::from_rows(noise)])?;
                    let p = matrix::rows(&states).ok_or("covar: the spread of the states was not found")?;
                    let through = matrix::matmul(&matrix::matmul(&m[2], &p)?, &matrix::transpose(&m[2]))?[0][0].clone();
                    let direct = m[3][0][0].clone();
                    let output = match (&ts, direct.is_zero()) {
                        (_, true) => tidy(through),
                        // (noise that goes straight to the output of a system of running time has no bound)
                        (None, false) => Expr::inf(),
                        (Some(_), false) => tidy(add(vec![through, mul(vec![direct.clone(), direct, a[1].clone()])])),
                    };
                    Expr::list(vec![output, states])
                }
                ("dlqe", 5) => {
                    let prec = crate::mfun::bits();
                    let usage = "dlqe(A, G, C, Q, R): matrices that fit; Q and R symmetric, R positive definite";
                    let get = |k: usize| -> Result<CMat, String> {
                        let rows = matrix::rows(&a[k]).unwrap_or_else(|| vec![vec![a[k].clone()]]);
                        to_cmat(&rows, prec).map_err(|_| usage.to_string())
                    };
                    let (m, g, c, q, r) = (get(0)?, get(1)?, get(2)?, get(3)?, get(4)?);
                    let size = m.len();
                    if m.iter().any(|x| x.len() != size) || g.len() != size || c.iter().any(|x| x.len() != size)
                        || q.len() != g[0].len() || r.len() != c.len()
                    {
                        return Err(usage.into());
                    }
                    let noise = c_mul(&c_mul(&g, &q), &c_turned(&g));
                    let ct = c_turned(&c);
                    let unit = c_unit(r.len(), prec);
                    let r_inverse = solved(&r, &unit).ok_or(usage)?;
                    let x = riccati_sampled(&c_turned(&m), &c_mul(&c_mul(&ct, &r_inverse), &c), &noise)?;
                    let seen = c_add(&c_mul(&c_mul(&c, &x), &ct), &r, 1);
                    let inverse = solved(&seen, &unit).ok_or(usage)?;
                    let gain = c_mul(&c_mul(&x, &ct), &inverse);
                    let after = c_mul(&c_add(&c_unit(size, prec), &c_mul(&gain, &c), -1), &x);
                    let closed = c_add(&m, &c_mul(&c_mul(&m, &gain), &c), -1);
                    let poles = self.call("eigenvalues", &[decomp::from_complex(&closed)])?;
                    Expr::list(vec![
                        decomp::from_complex(&gain),
                        decomp::from_complex(&x),
                        decomp::from_complex(&after),
                        poles,
                    ])
                }
                ("lyapchol" | "dlyapchol", 2) => {
                    let prec = crate::mfun::bits();
                    let usage = format!("{name}(A, B): a stable square matrix A and a matrix B with its rows");
                    let (m, b) = (matrix::rows(&a[0]).ok_or(usage.clone())?, matrix::rows(&a[1]).ok_or(usage.clone())?);
                    if b.len() != m.len() {
                        return Err(usage);
                    }
                    let spread = matrix::matmul(&b, &matrix::transpose(&b))?;
                    let lyap = if name == "lyapchol" { "lyap" } else { "dlyap" };
                    let x = self.lyapunov(lyap, &[a[0].clone(), matrix::from_rows(spread)])?;
                    let numbers = to_cmat(&matrix::rows(&x).unwrap_or_default(), prec).map_err(|_| usage.clone())?;
                    let lower = cholesky(&numbers)
                        .ok_or_else(|| format!("{name}: the solution is not positive definite (A is not stable, or B does not reach every state)"))?;
                    decomp::from_complex(&c_turned(&lower))
                }
                ("initial", 2 | 3) => {
                    let sys = self.left_alone(&a[0], &a[1], who)?;
                    self.response(&sys, Expr::one(), a.get(2), who)?
                }
                ("gensig", 2..=4) => {
                    let kind = word(&a[0]).ok_or("gensig(\"sine\" | \"square\" | \"pulse\", period[, duration[, step]])")?;
                    let period = a[1].clone();
                    let duration = a.get(2).cloned().unwrap_or_else(|| mul(vec![Expr::int(5), period.clone()]));
                    let step = a.get(3).cloned().unwrap_or_else(|| div(period.clone(), Expr::int(64)));
                    let positive = |e: &Expr| numeric::real_value(e, 20).is_some_and(|v| v > 0);
                    if !positive(&period) || !positive(&duration) || !positive(&step) {
                        return Err("gensig(kind, period[, duration[, step]]): positive numbers".into());
                    }
                    let steps = numeric::real_value(&div(duration, step.clone()), 30)
                        .map(|v| v.floor().to_f64())
                        .filter(|v| *v <= 100_000.0)
                        .ok_or("gensig: 100000 steps at most")? as i64;
                    let (mut values, mut times) = (Vec::new(), Vec::new());
                    for k in 0..=steps {
                        let t = tidy(mul(vec![Expr::int(k), step.clone()]));
                        let turns = tidy(div(t.clone(), period.clone()));
                        let whole = self.call("floor", std::slice::from_ref(&turns))?;
                        let part = numeric::real_value(&sub(turns.clone(), whole), 30).map_or(0.0, |v| v.to_f64());
                        let width = numeric::real_value(&div(step.clone(), period.clone()), 30).map_or(0.0, |v| v.to_f64());
                        values.push(match kind.as_str() {
                            "sine" | "sin" => self.eval(&Expr::call("sin", vec![mul(vec![Expr::int(2), Expr::pi(), turns])]))?,
                            "square" => Expr::int(i64::from(part >= 0.5)),
                            "pulse" => Expr::int(i64::from(part < width * (1.0 - 1e-9))),
                            _ => return Err("gensig: the kinds are \"sine\", \"square\" and \"pulse\"".into()),
                        });
                        times.push(t);
                    }
                    Expr::list(vec![Expr::list(values), Expr::list(times)])
                }
                ("lqi", 3 | 6) => {
                    let usage = "lqi(system, Q, R) or lqi(A, B, C, D, Q, R): weights Q for the states and the integrals of the outputs, R for the inputs";
                    let rows = |k: usize| matrix::rows(&a[k]).unwrap_or_else(|| vec![vec![a[k].clone()]]);
                    let ([m, b, c, d], ts) = match n {
                        3 => self.with_states(&a[0], who)?,
                        _ => ([rows(0), rows(1), rows(2), rows(3)], None),
                    };
                    let (states, outputs) = (m.len(), c.len());
                    if b.len() != states || c.iter().any(|r| r.len() != states) || d.len() != outputs {
                        return Err(usage.into());
                    }
                    // the integral of the error is a state more for every output (a sum over the
                    // samples of a sampled system, with the time between them)
                    let step = ts.clone().unwrap_or_else(Expr::one);
                    let more = |i: usize, j: usize| match (i == j, &ts) {
                        (true, Some(_)) => Expr::one(),
                        _ => Expr::zero(),
                    };
                    let mut wide: Rows = m
                        .iter()
                        .map(|r| r.iter().cloned().chain(std::iter::repeat_n(Expr::zero(), outputs)).collect())
                        .collect();
                    wide.extend(c.iter().enumerate().map(|(i, r)| {
                        r.iter()
                            .map(|x| tidy(neg(mul(vec![x.clone(), step.clone()]))))
                            .chain((0..outputs).map(|j| more(i, j)))
                            .collect::<Vec<_>>()
                    }));
                    let mut inputs: Rows = b.clone();
                    inputs.extend(d.iter().map(|r| {
                        r.iter().map(|x| tidy(neg(mul(vec![x.clone(), step.clone()])))).collect::<Vec<_>>()
                    }));
                    let solver = if ts.is_some() { "dlqr" } else { "lqr" };
                    self.call(solver, &[matrix::from_rows(wide), matrix::from_rows(inputs), a[n - 2].clone(), a[n - 1].clone()])?
                }
                ("filt", 1..=3) => {
                    // polynomials in 1/z: both as long as the longer one, then they are in z
                    let (mut num, mut den) = (list_of(&a[0]), a.get(1).map_or_else(|| vec![Expr::one()], list_of));
                    let length = num.len().max(den.len());
                    num.resize(length, Expr::zero());
                    den.resize(length, Expr::zero());
                    let ts = a.get(2).cloned().unwrap_or_else(|| Expr::int(-1));
                    self.call("tf", &[Expr::list(num), Expr::list(den), ts])?
                }
                ("ss2ss", 2) => {
                    let (m, ts) = self.with_states(&a[0], who)?;
                    let to = matrix::rows(&a[1]).ok_or("ss2ss(system, T): a square matrix T with an inverse")?;
                    if to.len() != m[0].len() || to.iter().any(|r| r.len() != to.len()) {
                        return Err(format!("ss2ss(system, T): T has the {} states of the system", m[0].len()));
                    }
                    let back = self.call("inv", std::slice::from_ref(&a[1]))?;
                    let back = matrix::rows(&back).ok_or("ss2ss: T has no inverse")?;
                    let clean = |rows: Rows| matrix::from_rows(rows.into_iter().map(|r| r.into_iter().map(tidy).collect()).collect());
                    let mut args = vec![
                        clean(matrix::matmul(&matrix::matmul(&to, &m[0])?, &back)?),
                        clean(matrix::matmul(&to, &m[1])?),
                        clean(matrix::matmul(&m[2], &back)?),
                        matrix::from_rows(m[3].clone()),
                    ];
                    args.extend(ts);
                    Expr::call("ss", args)
                }
                ("sigma", 1 | 2) => {
                    // (one input and one output: the one singular value is the gain)
                    let both = self.call("bode", a)?;
                    let parts = list_of(&both);
                    match parts.len() {
                        3 => Expr::list(vec![parts[0].clone(), parts[2].clone()]),
                        _ => parts[0].clone(),
                    }
                }
                ("ord2", 2) => {
                    let (rate, damping) = (a[0].clone(), a[1].clone());
                    Sys {
                        num: vec![Expr::one()],
                        den: vec![Expr::one(), tidy(mul(vec![Expr::int(2), damping, rate.clone()])), tidy(pow(rate, Expr::int(2)))],
                        ts: None,
                    }
                    .object()
                }
                ("d2d", 2) => {
                    let running = self.running(&a[0], None)?;
                    self.sampled(&running, &a[1], None)?
                }
                ("reg", 3) => {
                    let (m, ts) = self.with_states(&a[0], who)?;
                    let size = m[0].len();
                    let gain: Vec<Expr> = list_of(&a[1]).iter().flat_map(list_of).collect();
                    let watcher: Vec<Expr> = list_of(&a[2]).iter().flat_map(list_of).collect();
                    if gain.len() != size || watcher.len() != size {
                        return Err(format!("reg(system, K, L): the gain K of the states and the gain L of the watcher have the {size} states of the system"));
                    }
                    let direct = m[3][0][0].clone();
                    // x' = (A - B K - L C + L D K) x + L y, u = -K x
                    let own: Rows = (0..size)
                        .map(|i| {
                            (0..size)
                                .map(|j| {
                                    tidy(add(vec![
                                        m[0][i][j].clone(),
                                        neg(mul(vec![m[1][i][0].clone(), gain[j].clone()])),
                                        neg(mul(vec![watcher[i].clone(), m[2][0][j].clone()])),
                                        mul(vec![watcher[i].clone(), direct.clone(), gain[j].clone()]),
                                    ]))
                                })
                                .collect()
                        })
                        .collect();
                    let mut args = vec![
                        matrix::from_rows(own),
                        matrix::from_rows(watcher.into_iter().map(|x| vec![x]).collect()),
                        matrix::from_rows(vec![gain.into_iter().map(neg).map(tidy).collect()]),
                        matrix::from_rows(vec![vec![Expr::zero()]]),
                    ];
                    args.extend(ts);
                    Expr::call("ss", args)
                }
                _ => return Err(String::new()),
            })
        })())
        .filter(|r| !matches!(r, Err(e) if e.is_empty()))
    }

    /// A system with 1 as the first coefficient of its denominator.
    fn monic(&mut self, system: &Expr, who: &str) -> Res {
        let sys = self.sys_of(system, who)?;
        let lead = sys.den[0].clone();
        if lead.is_one() {
            return Ok(sys.object());
        }
        let mut scaled = |p: &[Expr]| -> Result<Vec<Expr>, String> {
            p.iter()
                .map(|c| self.call("simplify", &[div(c.clone(), lead.clone())]))
                .collect()
        };
        let (num, den) = (scaled(&sys.num)?, scaled(&sys.den)?);
        Ok(sys.with(num, den).object())
    }

    /// `lyap(A, Q)`: A*X + X*A' + Q = 0; `lyap(A, B, C)`: A*X + X*B + C = 0;
    /// `dlyap(A, Q)`: A*X*A' - X + Q = 0, each as equations in the entries of X.
    fn lyapunov(&mut self, name: &str, a: &[Expr]) -> Res {
        let usage = "lyap(A, Q), lyap(A, B, C) or dlyap(A, Q): square matrices";
        let m = matrix::rows(&a[0]).ok_or(usage)?;
        let (right, q) = match a.len() {
            3 => (
                matrix::rows(&a[1]).ok_or(usage)?,
                matrix::rows(&a[2]).ok_or(usage)?,
            ),
            _ => (matrix::transpose(&m), matrix::rows(&a[1]).ok_or(usage)?),
        };
        let (rows, columns) = (m.len(), right.len());
        if m.iter().any(|r| r.len() != rows)
            || right.iter().any(|r| r.len() != columns)
            || q.len() != rows
            || q.iter().any(|r| r.len() != columns)
        {
            return Err(usage.into());
        }
        // the unknown X(i, j) has the place i*columns + j
        let size = rows * columns;
        let mut left = vec![vec![Expr::zero(); size]; size];
        for (i, across) in m.iter().enumerate() {
            for j in 0..columns {
                let row = i * columns + j;
                for (k, weight) in across.iter().enumerate() {
                    for (l, down) in right.iter().enumerate() {
                        let column = k * columns + l;
                        let entry = match name {
                            // (A X A')(i, j) = sum A(i, k) X(k, l) A'(l, j)
                            "dlyap" => add(vec![
                                mul(vec![weight.clone(), down[j].clone()]),
                                Expr::int(-i64::from(i == k && j == l)),
                            ]),
                            _ => add(vec![
                                if l == j { weight.clone() } else { Expr::zero() },
                                if k == i {
                                    down[j].clone()
                                } else {
                                    Expr::zero()
                                },
                            ]),
                        };
                        left[row][column] = tidy(entry);
                    }
                }
            }
        }
        let side: Vec<Expr> = q.iter().flatten().map(|x| tidy(neg(x.clone()))).collect();
        let found = matrix::linsolve(&left, &side).map_err(|_| {
            format!("{name}: the equation has no one solution (two eigenvalues cancel each other)")
        })?;
        let flat = list_of(&found);
        if flat.len() != size {
            return Err(format!("{name}: the equation has no one solution"));
        }
        Ok(matrix::from_rows(
            flat.chunks(columns)
                .map(|r| r.iter().cloned().map(tidy).collect())
                .collect(),
        ))
    }

    /// The equations of Riccati and the gains that cost least: care, dare, lqr, dlqr, lqe.
    fn gains(&mut self, name: &str, a: &[Expr]) -> Res {
        let prec = crate::mfun::bits();
        let usage = format!(
            "{name}(A, B, Q, R): matrices that fit; Q and R symmetric, R positive definite"
        );
        let get = |k: usize| -> Result<CMat, String> {
            let rows = matrix::rows(&a[k]).unwrap_or_else(|| vec![vec![a[k].clone()]]);
            to_cmat(&rows, prec).map_err(|_| usage.clone())
        };
        let (m, input, q, r) = match name {
            // lqe(A, G, C, Q, R): the gain of the watcher is that of the turned system
            "lqe" => {
                let (m, g, c, q, r) = (get(0)?, get(1)?, get(2)?, get(3)?, get(4)?);
                (
                    c_turned(&m),
                    c_turned(&c),
                    c_mul(&c_mul(&g, &q), &c_turned(&g)),
                    r,
                )
            }
            _ => (get(0)?, get(1)?, get(2)?, get(3)?),
        };
        let size = m.len();
        if m.iter().any(|x| x.len() != size)
            || input.len() != size
            || q.len() != size
            || q.iter().any(|x| x.len() != size)
            || r.len() != input[0].len()
            || r.iter().any(|x| x.len() != r.len())
        {
            return Err(usage);
        }
        let cross = match (name, a.get(4)) {
            ("lqr" | "dlqr", Some(_)) => Some(get(4)?),
            _ => None,
        };
        let unit = c_unit(r.len(), prec);
        let r_inverse = solved(&r, &unit).ok_or_else(|| usage.clone())?;
        let bt = c_turned(&input);
        let sampled = matches!(name, "dare" | "idare" | "dlqr");
        // a cross weight N turns into A - B R^-1 N' and Q - N R^-1 N'
        let (m_used, q_used) = match &cross {
            Some(nn) => {
                let shift = c_mul(&c_mul(&input, &r_inverse), &c_turned(nn));
                (
                    c_add(&m, &shift, -1),
                    c_add(&q, &c_mul(&c_mul(nn, &r_inverse), &c_turned(nn)), -1),
                )
            }
            None => (m.clone(), q.clone()),
        };
        let g = c_mul(&c_mul(&input, &r_inverse), &bt);
        let x = match sampled {
            true => riccati_sampled(&m_used, &g, &q_used)?,
            false => riccati(&m_used, &g, &q_used)?,
        };
        // the gain K with u = -K*x
        let mut gain = match sampled {
            true => {
                let inner = c_add(&r, &c_mul(&c_mul(&bt, &x), &input), 1);
                solved(&inner, &c_mul(&c_mul(&bt, &x), &m_used)).ok_or_else(|| usage.clone())?
            }
            false => c_mul(&c_mul(&r_inverse, &bt), &x),
        };
        if let Some(nn) = &cross {
            gain = c_add(&gain, &c_mul(&r_inverse, &c_turned(nn)), 1);
        }
        let closed = c_add(&m, &c_mul(&input, &gain), -1);
        let poles = self.call("eigenvalues", &[decomp::from_complex(&closed)])?;
        let (x, gain) = (decomp::from_complex(&x), decomp::from_complex(&gain));
        Ok(match name {
            "lqr" | "dlqr" => Expr::list(vec![gain, x, poles]),
            "lqe" => {
                let turned = matrix::rows(&gain)
                    .map(|g| matrix::from_rows(matrix::transpose(&g)))
                    .unwrap_or(gain);
                Expr::list(vec![turned, x, poles])
            }
            _ => Expr::list(vec![x, gain, poles]),
        })
    }

    /// The gain K that puts the eigenvalues of A - B*K where they are wanted (Ackermann).
    fn placed(&mut self, name: &str, a: &[Expr]) -> Res {
        let usage = format!("{name}(A, B, [poles]): as many poles as A has rows");
        let m = matrix::rows(&a[0]).ok_or(usage.clone())?;
        let input = matrix::rows(&a[1]).ok_or(usage.clone())?;
        let wanted = list_of(&a[2]);
        let size = m.len();
        if m.iter().any(|r| r.len() != size) || input.len() != size || wanted.len() != size {
            return Err(usage);
        }
        let inputs = input[0].len();
        if name == "acker" && inputs != 1 {
            return Err("acker takes one input; place takes several".into());
        }
        // several inputs act as one through a mix of them: the first mix that steers all
        let mixes: Vec<Vec<Expr>> = match inputs {
            1 => vec![vec![Expr::one()]],
            _ => {
                let mut mixes = vec![vec![Expr::one(); inputs]];
                mixes.push((1..=inputs as i64).map(Expr::int).collect());
                mixes.push((1..=inputs as i64).map(|k| Expr::int(k * k - 2)).collect());
                mixes
            }
        };
        let wanted_poly = list_of(&self.call("poly", &[Expr::list(wanted)])?);
        for mix in &mixes {
            let one: Rows = input
                .iter()
                .map(|r| {
                    vec![tidy(add(r
                        .iter()
                        .zip(mix)
                        .map(|(x, g)| mul(vec![x.clone(), g.clone()]))
                        .collect()))]
                })
                .collect();
            // the matrix [b, A b, ..., A^(n-1) b]
            let mut columns = vec![one.clone()];
            for _ in 1..size {
                columns.push(matrix::matmul(&m, &columns[columns.len() - 1])?);
            }
            let steer: Rows = (0..size)
                .map(|i| columns.iter().map(|c| tidy(c[i][0].clone())).collect())
                .collect();
            let Ok(inverse) = matrix::inverse(&steer) else {
                continue;
            };
            // the wanted polynomial of A
            let mut value: Rows = vec![vec![Expr::zero(); size]; size];
            let mut power = matrix::identity(size);
            for c in wanted_poly.iter().rev() {
                for i in 0..size {
                    for j in 0..size {
                        value[i][j] = add(vec![
                            value[i][j].clone(),
                            mul(vec![c.clone(), power[i][j].clone()]),
                        ]);
                    }
                }
                power = matrix::matmul(&power, &m)?;
            }
            let last: Rows = vec![inverse[size - 1].clone()];
            let k = matrix::matmul(&last, &value)?;
            // back to all inputs: K = mix * k
            let full: Rows = mix
                .iter()
                .map(|g| {
                    k[0].iter()
                        .map(|x| tidy(mul(vec![g.clone(), x.clone()])))
                        .collect()
                })
                .collect();
            return Ok(match inputs {
                1 => Expr::list(full.into_iter().next().unwrap_or_default()),
                _ => matrix::from_rows(full),
            });
        }
        Err(format!(
            "{name}: the system cannot be steered everywhere (its matrix [B, A*B, ...] has no full rank)"
        ))
    }

    /// Where the gain is one and where the phase is half a turn, and what is to spare there.
    fn margins(&mut self, e: &Expr, all: bool) -> Res {
        let who = "margin";
        let given = self.sys_of(e, who)?;
        let prec = crate::mfun::bits();
        let ts = sample_time(&given, prec);
        let sys = match ts {
            Some(_) => unrolled(&given),
            None => given.clone(),
        };
        // N(jw)*conj(D(jw)) = (Nr*Dr + Ni*Di) + j*(Ni*Dr - Nr*Di), polynomials in w
        let ((nr, ni), (dr, di)) = (along_axis(&sys.num), along_axis(&sys.den));
        let im = poly_add(
            &poly_mul(&ni, &dr),
            &poly_scaled(&poly_mul(&nr, &di), &Expr::int(-1)),
        );
        let positive = |s: &mut Session, coeffs: &[Expr]| -> Result<Vec<Float>, String> {
            let mut found = Vec::new();
            for r in s.roots_of(&trimmed(coeffs.to_vec()))? {
                let z = complex_of(&r, prec)?;
                let small = Float::with_val(prec, z.abs_ref()) >> (prec / 2);
                if Float::with_val(prec, z.imag().abs_ref()) <= small && *z.real() > small {
                    found.push(z.real().clone());
                }
            }
            found.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
            found.dedup_by(|x, y| {
                Float::with_val(prec, &*x - &*y).abs()
                    <= Float::with_val(prec, y.abs_ref()) >> (prec / 2)
            });
            Ok(found)
        };
        let gain_at = |x: &Float| -> Result<Complex, String> {
            complex_of(&sys.at(&mul(vec![Expr::i(), crate::mfun::value(x)])), prec)
        };
        // the phase is half a turn where im = 0 and re < 0
        let mut gain_margins: Vec<(Float, Float)> = Vec::new();
        for x in positive(self, &im)? {
            let h = gain_at(&x)?;
            if h.real().is_sign_negative() && !h.real().is_zero() {
                gain_margins.push((Float::with_val(prec, h.abs_ref()).recip(), rolled(&x, &ts)));
            }
        }
        // (a sampled system at its highest frequency, z = -1)
        if ts.is_some() && !poly_at(&given.den, &Expr::int(-1)).is_zero() {
            let h = complex_of(&given.at(&Expr::int(-1)), prec)?;
            if h.real().is_sign_negative() && !h.real().is_zero() {
                gain_margins.push((
                    Float::with_val(prec, h.abs_ref()).recip(),
                    rolled(&Float::with_val(prec, f64::INFINITY), &ts),
                ));
            }
        }
        // the gain is one where |N|^2 = |D|^2
        let size = |r: &[Expr], i: &[Expr]| poly_add(&poly_mul(r, r), &poly_mul(i, i));
        let unit = poly_add(
            &size(&nr, &ni),
            &poly_scaled(&size(&dr, &di), &Expr::int(-1)),
        );
        let mut phase_margins: Vec<(Float, Float)> = Vec::new();
        let half_turn = Float::with_val(prec, 180);
        for x in positive(self, &unit)? {
            let h = gain_at(&x)?;
            let mut phase = Float::with_val(prec, h.arg_ref()) * &half_turn
                / Float::with_val(prec, rug::float::Constant::Pi);
            // (the phase as it comes down from the low frequencies: between -360 and 0)
            if phase > 0 {
                phase -= 360u32;
            }
            phase_margins.push((phase + 180u32, rolled(&x, &ts)));
        }
        let shown = crate::mfun::value;
        if all {
            let closed = self.sys_of_expr(
                &div(given.expr(), add(vec![Expr::one(), given.expr()])),
                given.ts.clone(),
                who,
            )?;
            let stable = self.stable(&closed)?;
            return Ok(crate::records::record(vec![
                (
                    "GainMargin",
                    Expr::list(gain_margins.iter().map(|m| shown(&m.0)).collect()),
                ),
                (
                    "GMFrequency",
                    Expr::list(gain_margins.iter().map(|m| shown(&m.1)).collect()),
                ),
                (
                    "PhaseMargin",
                    Expr::list(phase_margins.iter().map(|m| shown(&m.0)).collect()),
                ),
                (
                    "PMFrequency",
                    Expr::list(phase_margins.iter().map(|m| shown(&m.1)).collect()),
                ),
                ("Stable", Expr::Bool(stable)),
            ]));
        }
        // the margins that are nearest to losing stability
        let nearest_gain = gain_margins.iter().min_by(|x, y| {
            let far = |m: &Float| Float::with_val(prec, m.ln_ref()).abs();
            far(&x.0)
                .partial_cmp(&far(&y.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let nearest_phase = phase_margins.iter().min_by(|x, y| {
            Float::with_val(prec, x.0.abs_ref())
                .partial_cmp(&Float::with_val(prec, y.0.abs_ref()))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let (gm, wcg) = match nearest_gain {
            Some((m, x)) => (shown(m), shown(x)),
            None => (Expr::inf(), Expr::undefined()),
        };
        let (pm, wcp) = match nearest_phase {
            Some((m, x)) => (shown(m), shown(x)),
            None => (Expr::inf(), Expr::undefined()),
        };
        Ok(Expr::list(vec![gm, pm, wcg, wcp]))
    }

    /// The squared gain |H(jw)|^2 as a ratio of two polynomials in w.
    fn squared_gain(sys: &Sys) -> (Vec<Expr>, Vec<Expr>) {
        let size = |p: &[Expr]| {
            let (r, i) = along_axis(p);
            trimmed(poly_add(&poly_mul(&r, &r), &poly_mul(&i, &i)))
        };
        (size(&sys.num), size(&sys.den))
    }

    /// The first frequency where the gain has fallen 3 dB (or `drop` dB, a number below
    /// zero) under the gain at rest.
    fn bandwidth(&mut self, e: &Expr, drop: Option<&Expr>) -> Res {
        let who = "bandwidth";
        let given = self.sys_of(e, who)?;
        let prec = crate::mfun::bits();
        let ts = sample_time(&given, prec);
        let sys = match ts {
            Some(_) => unrolled(&given),
            None => given,
        };
        let drop = match drop {
            Some(d) if order_of(d) == Some(std::cmp::Ordering::Less) => d.clone(),
            Some(_) => {
                return Err("bandwidth(system, drop): the drop in dB is below zero, as -3".into());
            }
            None => Expr::int(-3),
        };
        let rest = sys.at(&Expr::zero());
        if poly_at(&sys.den, &Expr::zero()).is_zero() || rest.is_zero() {
            return Ok(Expr::undefined());
        }
        let (top, bottom) = Self::squared_gain(&sys);
        // |N|^2 - |D|^2 * dc^2 * 10^(drop/10) = 0
        let rest = complex_of(&rest, prec)?;
        let share = Float::with_val(prec, 10).pow(
            self.real_of(&drop)
                .ok_or("bandwidth: the drop is a number")?
                / 10u32,
        );
        let level = crate::mfun::value(&(Float::with_val(prec, rest.norm_ref()) * share));
        let p = poly_add(&top, &poly_scaled(&bottom, &neg(level)));
        let mut least: Option<Float> = None;
        for r in self.roots_of(&p)? {
            let z = complex_of(&r, prec)?;
            let small = Float::with_val(prec, z.abs_ref()) >> (prec / 2);
            if Float::with_val(prec, z.imag().abs_ref()) <= small
                && *z.real() > small
                && least.as_ref().is_none_or(|l| z.real() < l)
            {
                least = Some(z.real().clone());
            }
        }
        Ok(match least {
            Some(x) => crate::mfun::value(&rolled(&x, &ts)),
            None => Expr::inf(),
        })
    }

    /// The largest gain over all frequencies and where it is reached.
    fn peak_gain(&mut self, e: &Expr) -> Res {
        let who = "hinfnorm";
        let given = self.sys_of(e, who)?;
        if !self.stable(&given)? {
            return Ok(Expr::list(vec![Expr::inf(), Expr::undefined()]));
        }
        let prec = crate::mfun::bits();
        let ts = sample_time(&given, prec);
        let sys = match ts {
            Some(_) => unrolled(&given),
            None => given,
        };
        let (top, bottom) = Self::squared_gain(&sys);
        // the slope of top/bottom is zero where top'*bottom - top*bottom' = 0
        let coeffs = trimmed(poly_add(
            &poly_mul(&poly_slope(&top), &bottom),
            &poly_scaled(&poly_mul(&top, &poly_slope(&bottom)), &Expr::int(-1)),
        ));
        let mut places = vec![Float::new(prec)];
        if coeffs.len() > 1 {
            for r in self.roots_of(&coeffs)? {
                let z = complex_of(&r, prec)?;
                let small = Float::with_val(prec, z.abs_ref()) >> (prec / 2);
                if Float::with_val(prec, z.imag().abs_ref()) <= small && *z.real() > small {
                    places.push(z.real().clone());
                }
            }
        }
        let mut best: Option<(Float, Float)> = None;
        for x in places {
            let h = complex_of(&sys.at(&mul(vec![Expr::i(), crate::mfun::value(&x)])), prec)?;
            let gain = Float::with_val(prec, h.abs_ref());
            if best.as_ref().is_none_or(|b| gain > b.0) {
                best = Some((gain, x));
            }
        }
        // the gain far out: the ratio of the leading coefficients when the degrees agree
        if sys.num.len() == sys.den.len() {
            let far = complex_of(&tidy(div(sys.num[0].clone(), sys.den[0].clone())), prec)?;
            let gain = Float::with_val(prec, far.abs_ref());
            if best.as_ref().is_none_or(|b| gain > b.0) {
                let place = match ts {
                    Some(_) => {
                        crate::mfun::value(&rolled(&Float::with_val(prec, f64::INFINITY), &ts))
                    }
                    None => Expr::inf(),
                };
                return Ok(Expr::list(vec![crate::mfun::value(&gain), place]));
            }
        }
        let (gain, x) = best.ok_or("hinfnorm: the gain has no value")?;
        Ok(Expr::list(vec![
            crate::mfun::value(&gain),
            crate::mfun::value(&rolled(&x, &ts)),
        ]))
    }

    /// The root of the energy of the answer to a blow: of trace(C*W*C') with A*W + W*A' + B*B' = 0.
    fn h2(&mut self, e: &Expr) -> Res {
        let who = "h2norm";
        let sys = self.sys_of(e, who)?;
        let sampled = sys.ts.is_some();
        // (a sampled system may answer at once: its samples are summed, not integrated)
        if sys.num.len() > sys.den.len()
            || (!sampled && sys.num.len() == sys.den.len())
            || !self.stable(&sys)?
        {
            return Ok(Expr::inf());
        }
        let m = tf_to_ss(&sys.num, &sys.den)?;
        if m[0].is_empty() {
            return self.call("abs", &[m[3][0][0].clone()]);
        }
        let q = matrix::matmul(&m[1], &matrix::transpose(&m[1]))?;
        let name = if sampled { "dlyap" } else { "lyap" };
        let w = self.lyapunov(
            name,
            &[matrix::from_rows(m[0].clone()), matrix::from_rows(q)],
        )?;
        let w = matrix::rows(&w).ok_or("h2norm: no solution")?;
        let through = matrix::matmul(&matrix::matmul(&m[2], &w)?, &matrix::transpose(&m[2]))?;
        let direct = pow(m[3][0][0].clone(), Expr::int(2));
        self.call(
            "simplify",
            &[pow(add(vec![through[0][0].clone(), direct]), Expr::half())],
        )
    }

    /// How the answer to a step rises, overshoots and settles.
    fn stepinfo(&mut self, e: &Expr) -> Res {
        let who = "stepinfo";
        let sys = self.sys_of(e, who)?;
        if sys.ts.is_some() {
            return Err("stepinfo takes systems of running time".into());
        }
        if !self.stable(&sys)? {
            return Err(
                "stepinfo: the system is not stable, its answer to a step settles nowhere".into(),
            );
        }
        let prec = crate::mfun::bits();
        let t = Expr::sym("t");
        let answer = self.response(&sys, pow(Expr::sym("s"), Expr::int(-1)), None, who)?;
        let slope = self.call("diff", &[answer.clone(), t.clone()])?;
        let y = crate::mfun::Numeric::new(&["t".to_string()], &answer, prec);
        let dy = crate::mfun::Numeric::new(&["t".to_string()], &slope, prec);
        let last = complex_of(&sys.at(&Expr::zero()), prec)?.real().clone();
        // the slowest pole says how long to look: until e^(-40) of its part is left
        let mut slowest = Float::with_val(prec, f64::INFINITY);
        for p in self.roots_of(&sys.den)? {
            let z = complex_of(&p, prec)?;
            slowest = slowest.min(&Float::with_val(prec, z.real().abs_ref()));
        }
        let horizon = Float::with_val(prec, 40) / &slowest;
        let count = 4000u32;
        let at = |k: u32| Float::with_val(prec, &horizon * k) / count;
        let value = |x: &Float| {
            y.at(std::slice::from_ref(x))
                .map_err(|e| format!("{who}: {e}"))
        };
        let mut values = Vec::with_capacity(count as usize + 1);
        for k in 0..=count {
            // (the answer at 0 is its limit from the right)
            let x = if k == 0 {
                Float::with_val(prec, &horizon >> 60u32)
            } else {
                at(k)
            };
            values.push(value(&x)?);
        }
        // a place between two samples where a function changes its sign, by halving
        let crossing =
            |f: &dyn Fn(&Float) -> Result<Float, String>, from: u32| -> Result<Float, String> {
                let (mut lo, mut hi) = (at(from), at(from + 1));
                let rising = f(&hi)?.is_sign_positive();
                for _ in 0..prec + 8 {
                    let mid = Float::with_val(prec, &lo + &hi) / 2u32;
                    if f(&mid)?.is_sign_positive() == rising {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                Ok(hi)
            };
        let first_above = |tenths: u32| -> Result<Float, String> {
            let level = Float::with_val(prec, &last * tenths) / 10u32;
            let passed = |v: &Float| match last.is_sign_positive() {
                true => *v >= level,
                false => *v <= level,
            };
            let k = (1..=count)
                .find(|k| passed(&values[*k as usize]))
                .unwrap_or(count);
            let f = |x: &Float| {
                value(x).map(|v| match last.is_sign_positive() {
                    true => v - &level,
                    false => Float::with_val(prec, &level - v),
                })
            };
            crossing(&f, k - 1)
        };
        let rise = Float::with_val(prec, first_above(9)? - first_above(1)?);
        // the last time the answer is farther than 2% from where it settles
        let band = Float::with_val(prec, last.abs_ref()) / 50u32;
        let outside = |v: &Float| Float::with_val(prec, v - &last).abs() > band;
        let settle = match (0..=count).rev().find(|k| outside(&values[*k as usize])) {
            Some(k) if k < count => {
                let above = values[k as usize] > last;
                let edge = match above {
                    true => Float::with_val(prec, &last + &band),
                    false => Float::with_val(prec, &last - &band),
                };
                let f = |x: &Float| value(x).map(|v| v - &edge);
                crossing(&f, k)?
            }
            _ => Float::new(prec),
        };
        // the peak: the largest sample, made exact where the slope changes its sign
        let size = |v: &Float| Float::with_val(prec, v.abs_ref());
        let top = (0..=count)
            .max_by(|i, j| {
                size(&values[*i as usize])
                    .partial_cmp(&size(&values[*j as usize]))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(count);
        let slope_at = |x: &Float| {
            dy.at(std::slice::from_ref(x))
                .map_err(|e| format!("{who}: {e}"))
        };
        let (peak_time, peak) = match top > 0
            && top < count
            && slope_at(&at(top - 1))?.is_sign_positive()
                != slope_at(&at(top + 1))?.is_sign_positive()
        {
            true => {
                let from = if slope_at(&at(top - 1))?.is_sign_positive()
                    != slope_at(&at(top))?.is_sign_positive()
                {
                    top - 1
                } else {
                    top
                };
                let x = crossing(&slope_at, from)?;
                let v = value(&x)?;
                (x, v)
            }
            false if top == count => (Float::with_val(prec, f64::INFINITY), last.clone()),
            false => (at(top), values[top as usize].clone()),
        };
        let over = match last.is_zero() {
            true => Float::new(prec),
            false => (Float::with_val(prec, size(&peak) - size(&last)) / size(&last) * 100u32)
                .max(&Float::new(prec)),
        };
        // how far the answer goes the wrong way
        let wrong = values
            .iter()
            .map(|v| {
                if last.is_sign_positive() {
                    Float::with_val(prec, -v)
                } else {
                    v.clone()
                }
            })
            .fold(Float::new(prec), |m, v| m.max(&v));
        let under = match last.is_zero() {
            true => Float::new(prec),
            false => wrong / size(&last) * 100u32,
        };
        let shown = |x: &Float| match x.is_infinite() {
            true => Expr::inf(),
            false => crate::arrays::tidy(crate::mfun::value(x)),
        };
        let clean = |x: &Float| match size(x) <= size(&last) >> (prec / 2) {
            true => Expr::zero(),
            false => shown(x),
        };
        // the least and the largest value once the answer has risen to nine tenths; where
        // it settles counts too
        let risen = first_above(9)?;
        let from = (1..=count).find(|k| at(*k) >= risen).unwrap_or(count);
        let extreme = |largest: bool| -> Result<Float, String> {
            let better = |x: &Float, y: &Float| if largest { x > y } else { x < y };
            let mut best = Float::with_val(prec, &last * 9u32) / 10u32;
            if better(&last, &best) {
                best = last.clone();
            }
            let mut spot = None;
            for k in from..=count {
                if better(&values[k as usize], &best) {
                    best = values[k as usize].clone();
                    spot = Some(k);
                }
            }
            // (a turn of the answer between samples: where the slope changes its sign)
            if let Some(k) = spot.filter(|k| *k > 0 && *k < count)
                && slope_at(&at(k - 1))?.is_sign_positive()
                    != slope_at(&at(k + 1))?.is_sign_positive()
            {
                let start = if slope_at(&at(k - 1))?.is_sign_positive()
                    != slope_at(&at(k))?.is_sign_positive()
                {
                    k - 1
                } else {
                    k
                };
                let x = crossing(&slope_at, start)?;
                if x >= risen {
                    best = value(&x)?;
                }
            }
            Ok(best)
        };
        let (least, largest) = (extreme(false)?, extreme(true)?);
        Ok(crate::records::record(vec![
            ("RiseTime", shown(&rise)),
            ("SettlingTime", shown(&settle)),
            ("SettlingMin", shown(&least)),
            ("SettlingMax", shown(&largest)),
            ("Overshoot", clean(&over)),
            ("Undershoot", clean(&under)),
            ("Peak", shown(&size(&peak))),
            ("PeakTime", shown(&peak_time)),
        ]))
    }

    /// `c2d(system, Ts[, method])`: a system of running time as a sampled one.
    fn sampled(&mut self, e: &Expr, ts: &Expr, method: Option<&Expr>) -> Res {
        let who = "c2d";
        let sys = self.sys_of(e, who)?;
        if sys.ts.is_some() {
            return Err("c2d takes a system of running time".into());
        }
        if order_of(ts) != Some(std::cmp::Ordering::Greater) {
            return Err("c2d(system, Ts[, method]): the time of a sample is above zero".into());
        }
        let (s, z) = (Expr::sym("s"), Expr::sym("z"));
        let method = match method {
            Some(m) => word(m)
                .ok_or("c2d: the method is \"zoh\", \"tustin\", \"matched\" or \"impulse\"")?,
            None => "zoh".to_string(),
        };
        match method.as_str() {
            "tustin" | "bilinear" => {
                // s = (2/Ts)*(z - 1)/(z + 1)
                let map = mul(vec![
                    div(Expr::int(2), ts.clone()),
                    div(sub(z.clone(), Expr::one()), add(vec![z, Expr::one()])),
                ]);
                let e = crate::canon::subst(&sys.expr(), &s, &map);
                let simpler = self.call("simplify", &[e])?;
                self.combined(simpler, Some(ts.clone()), who)
            }
            "matched" => {
                // every zero and pole p goes to e^(p*Ts); zeros at infinity go to -1; the
                // gains at rest agree
                let moved = |r: &Expr| tidy(pow(Expr::e(), mul(vec![r.clone(), ts.clone()])));
                let (zeros, poles) = (self.roots_of(&sys.num)?, self.roots_of(&sys.den)?);
                let mut new_zeros: Vec<Expr> = zeros.iter().map(moved).collect();
                let new_poles: Vec<Expr> = poles.iter().map(moved).collect();
                while new_zeros.len() + 1 < new_poles.len() {
                    new_zeros.push(Expr::int(-1));
                }
                let shape = |roots: &[Expr], at: &Expr| {
                    mul(roots.iter().map(|r| sub(at.clone(), r.clone())).collect())
                };
                let ratio = div(shape(&new_zeros, &z), shape(&new_poles, &z));
                // match the gain at s = 0 and z = 1 (at a point beside them when a pole lies there)
                let (at_s, at_z) = match poly_at(&sys.den, &Expr::zero()).is_zero() {
                    false => (Expr::zero(), Expr::one()),
                    true => (Expr::one(), tidy(pow(Expr::e(), ts.clone()))),
                };
                let gain = div(sys.at(&at_s), crate::canon::subst(&ratio, &z, &at_z));
                let top = self.call("expand", &[mul(vec![gain, shape(&new_zeros, &z)])])?;
                let den = self.call("expand", &[shape(&new_poles, &z)])?;
                self.combined(div(top, den), Some(ts.clone()), who)
            }
            "impulse" => {
                // Ts times the transform of the samples of the answer to a blow
                let (t, k) = (Expr::sym("t"), Expr::sym("_k"));
                let blow = self.call("invlaplace", &[sys.expr(), s, t.clone()])?;
                let at = crate::canon::subst(&blow, &t, &mul(vec![k.clone(), ts.clone()]));
                let e = self.call("ztransform", &[at, k, z])?;
                let simpler = self.call("simplify", &[mul(vec![ts.clone(), e])])?;
                self.combined(simpler, Some(ts.clone()), who)
            }
            "zoh" => {
                // [[Ad, Bd], [0, I]] = exp([[A, B], [0, 0]] * Ts)
                let m = tf_to_ss(&sys.num, &sys.den)?;
                let size = m[0].len();
                if size == 0 {
                    return Ok(Sys {
                        ts: Some(ts.clone()),
                        ..sys
                    }
                    .object());
                }
                let mut joined: Rows = Vec::new();
                for (state, input) in m[0].iter().zip(&m[1]).take(size) {
                    let mut row: Vec<Expr> = state
                        .iter()
                        .map(|x| tidy(mul(vec![x.clone(), ts.clone()])))
                        .collect();
                    row.push(tidy(mul(vec![input[0].clone(), ts.clone()])));
                    joined.push(row);
                }
                joined.push(vec![Expr::zero(); size + 1]);
                // in closed form for one or two states of fractions; else by decimal numbers
                let closed = joined.iter().flatten().any(|x| !x.is_const())
                    || (size <= 2 && joined.iter().flatten().all(|x| x.as_rat().is_some()));
                let joined = matrix::from_rows(joined);
                let grown = match closed {
                    true => self
                        .call("matrixexp", std::slice::from_ref(&joined))
                        .ok()
                        .filter(|g| matrix::rows(g).is_some() && !has_i(g)),
                    false => None,
                };
                let grown = match grown {
                    Some(g) => g,
                    None => {
                        let numbers =
                            decomp::to_complex(&matrix::rows(&joined).unwrap_or_default())?;
                        decomp::from_complex(&crate::matfun::exp(&numbers))
                    }
                };
                let grown = matrix::rows(&grown)
                    .ok_or("c2d: the exponential of the matrix has no value")?;
                if grown.len() < size || grown.iter().any(|row| row.len() <= size) {
                    return Err("c2d: the exponential of the matrix has no value".into());
                }
                let a: Rows = (0..size).map(|i| grown[i][..size].to_vec()).collect();
                let b: Rows = (0..size).map(|i| vec![grown[i][size].clone()]).collect();
                let (num, den) = ss_to_tf(&[a, b, m[2].clone(), m[3].clone()])?;
                let mut clean = |p: Vec<Expr>| -> Result<Vec<Expr>, String> {
                    p.into_iter().map(|x| self.call("simplify", &[x])).collect()
                };
                let (num, den) = (clean(num)?, clean(den)?);
                Ok(Sys {
                    num: trimmed(dusted(num)),
                    den: dusted(den),
                    ts: Some(ts.clone()),
                }
                .object())
            }
            _ => Err(
                "c2d: the method is \"zoh\" (the usual), \"tustin\", \"matched\" or \"impulse\""
                    .into(),
            ),
        }
    }

    /// `d2c(system[, method])`: a sampled system as one of running time.
    fn running(&mut self, e: &Expr, method: Option<&Expr>) -> Res {
        let who = "d2c";
        let sys = self.sys_of(e, who)?;
        let ts = match &sys.ts {
            Some(ts) if order_of(ts) == Some(std::cmp::Ordering::Greater) => ts.clone(),
            _ => {
                return Err(
                    "d2c takes a sampled system with its time of a sample: tf(num, den, Ts)".into(),
                );
            }
        };
        let (s, z) = (Expr::sym("s"), Expr::sym("z"));
        let method = match method {
            Some(m) => word(m).ok_or("d2c: the method is \"zoh\" or \"tustin\"")?,
            None => "zoh".to_string(),
        };
        match method.as_str() {
            "tustin" | "bilinear" => {
                // z = (1 + s*Ts/2)/(1 - s*Ts/2)
                let half = div(mul(vec![s, ts]), Expr::int(2));
                let map = div(add(vec![Expr::one(), half.clone()]), sub(Expr::one(), half));
                let e = crate::canon::subst(&sys.expr(), &z, &map);
                let simpler = self.call("simplify", &[e])?;
                self.combined(simpler, None, who)
            }
            "zoh" => {
                // [[Ad, Bd], [0, I]] = exp([[A, B], [0, 0]] * Ts)
                let m = tf_to_ss(&sys.num, &sys.den)?;
                let size = m[0].len();
                let mut joined: Rows = Vec::new();
                for (state, input) in m[0].iter().zip(&m[1]).take(size) {
                    let mut row = state.clone();
                    row.push(input[0].clone());
                    joined.push(row);
                }
                let mut last = vec![Expr::zero(); size];
                last.push(Expr::one());
                joined.push(last);
                let closed = joined.iter().flatten().any(|x| !x.is_const())
                    || (size <= 2 && joined.iter().flatten().all(|x| x.as_rat().is_some()));
                let joined = matrix::from_rows(joined);
                let log = match closed {
                    true => self
                        .call("matrixlog", std::slice::from_ref(&joined))
                        .ok()
                        .filter(|g| matrix::rows(g).is_some() && !has_i(g)),
                    false => None,
                };
                let log = match log {
                    Some(g) => g,
                    None => {
                        let numbers =
                            decomp::to_complex(&matrix::rows(&joined).unwrap_or_default())?;
                        let log = crate::matfun::log(&numbers).ok_or(
                            "d2c: no system of running time gives this one when its input is held (a pole at zero or below zero)",
                        )?;
                        decomp::from_complex(&log)
                    }
                };
                let log =
                    matrix::rows(&log).ok_or("d2c: the logarithm of the matrix has no value")?;
                let scaled = |x: &Expr| tidy(div(x.clone(), ts.clone()));
                let a: Rows = (0..size)
                    .map(|i| (0..size).map(|j| scaled(&log[i][j])).collect())
                    .collect();
                let b: Rows = (0..size).map(|i| vec![scaled(&log[i][size])]).collect();
                let (num, den) = ss_to_tf(&[a, b, m[2].clone(), m[3].clone()])?;
                Ok(Sys {
                    num: trimmed(dusted(num)),
                    den: dusted(den),
                    ts: None,
                }
                .object())
            }
            _ => Err("d2c: the method is \"zoh\" (the usual) or \"tustin\"".into()),
        }
    }

    /// The numbers that a picture of a system needs.
    /// The real and the imaginary parts of numbers of the plane, as two lists.
    fn parts(values: &[Expr]) -> Result<[Expr; 2], String> {
        let (mut re, mut im) = (Vec::new(), Vec::new());
        for v in values {
            let z = complex_of(v, 64)?;
            re.push(crate::mfun::value(z.real()));
            im.push(crate::mfun::value(z.imag()));
        }
        Ok([Expr::list(re), Expr::list(im)])
    }

    /// What the chart of a system draws, as lists of real numbers.
    pub(crate) fn control_chart(&mut self, name: &str, a: &[Expr]) -> Result<Vec<Expr>, String> {
        self.without_names(|session| session.control_chart_of(name, a))
    }

    fn control_chart_of(&mut self, name: &str, a: &[Expr]) -> Result<Vec<Expr>, String> {
        let who = name;
        if name == "zplane" {
            let parts = crate::sigdesign::apart(name, a).transpose()?;
            // the filter b(1/z)/a(1/z) as a sampled system
            let filter = match parts.as_deref().unwrap_or(a) {
                [b] => Expr::call(
                    "tf",
                    vec![b.clone(), Expr::list(vec![Expr::one()]), Expr::one()],
                ),
                [b, den] => {
                    // (both written in powers of 1/z: of one length as polynomials in z)
                    let (mut top, mut bottom) = (list_of(b), list_of(den));
                    let size = top.len().max(bottom.len());
                    top.resize(size, Expr::zero());
                    bottom.resize(size, Expr::zero());
                    Expr::call("tf", vec![Expr::list(top), Expr::list(bottom), Expr::one()])
                }
                _ => return Err("zplane(b, a): the coefficients of a sampled filter".into()),
            };
            return self.control_chart("pzplot", &[filter]);
        }
        if name == "initialplot" {
            let [system, x0, rest @ ..] = a else {
                return Err("initialplot(system, x0[, the last time])".into());
            };
            let mut args = vec![self.left_alone(system, x0, who)?.object()];
            args.extend(rest.iter().cloned());
            return self.control_chart_of("impulseplot", &args);
        }
        let first = a.first().ok_or_else(|| format!("{name}(system)"))?;
        if a.len() > 2 {
            return Err(format!("{name}(system) or {name}(system, one thing more)"));
        }
        let sys = self.sys_of(first, who)?;
        let shown = |v: &[Float]| Expr::list(v.iter().map(crate::mfun::value).collect());
        match name {
            "bodeplot" | "nyquistplot" | "nicholsplot" => {
                let ws = match a.get(1) {
                    Some(w) => list_of(w),
                    None => self.frequencies(&sys, 241)?,
                };
                if name != "nyquistplot" {
                    let (gains, phases) = self.bode_data(&sys, &ws)?;
                    return Ok(vec![Expr::list(ws), shown(&gains), shown(&phases)]);
                }
                let values: Vec<Expr> = ws.iter().map(|w| sys.at(&Self::point(&sys, w))).collect();
                Ok(Self::parts(&values)?.to_vec())
            }
            "pzplot" => {
                let (poles, zeros) = (self.roots_of(&sys.den)?, self.roots_of(&sys.num)?);
                let mut out = Self::parts(&poles)?.to_vec();
                out.extend(Self::parts(&zeros)?);
                out.push(Expr::Bool(sys.ts.is_some()));
                Ok(out)
            }
            "rlocusplot" => {
                // the gains: none, then steps that grow by an eighth of themselves
                let mut gains = vec![Expr::zero()];
                let mut k = Float::with_val(64, 1) / 1024u32;
                while k < 16384 {
                    gains.push(crate::mfun::value(&k));
                    k *= Float::with_val(64, 9) / 8u32;
                }
                let gains = match a.get(1) {
                    Some(g) => list_of(g),
                    None => gains,
                };
                // (the roots by numbers: fast for any degree)
                let (prec, digits) = (crate::mfun::bits(), numeric::digits());
                let numbers = |p: &[Expr]| -> Result<Vec<Expr>, String> {
                    p.iter()
                        .map(|c| complex_of(c, prec).map(|z| numeric::to_float_expr(z, digits)))
                        .collect()
                };
                let (num, den) = (numbers(&sys.num)?, numbers(&sys.den)?);
                let mut all = Vec::new();
                for g in &gains {
                    let g = numeric::to_float_expr(complex_of(g, prec)?, digits);
                    let closed = poly_add(&den, &poly_scaled(&num, &g));
                    all.extend(self.roots_of(&closed)?);
                }
                let (poles, zeros) = (self.roots_of(&sys.den)?, self.roots_of(&sys.num)?);
                let mut out = Self::parts(&all)?.to_vec();
                out.extend(Self::parts(&poles)?);
                out.extend(Self::parts(&zeros)?);
                out.push(Expr::int(gains.len() as i64));
                Ok(out)
            }
            _ => {
                // stepplot and impulseplot: the times and the answers
                let step = name == "stepplot";
                match &sys.ts {
                    None => {
                        let drive = if step {
                            pow(Expr::sym("s"), Expr::int(-1))
                        } else {
                            Expr::one()
                        };
                        let answer = self.response(&sys, drive, None, who)?;
                        let prec = 96;
                        // as long as the slowest pole needs (ten times its time), or 10
                        let mut slowest = f64::INFINITY;
                        for p in self.roots_of(&sys.den)? {
                            let z = complex_of(&p, 64)?;
                            let part = z.real().to_f64().abs();
                            if part > 1e-9 {
                                slowest = slowest.min(part);
                            }
                        }
                        let horizon = match a.get(1).and_then(|t| self.real_of(t)) {
                            Some(t) => t.to_f64(),
                            None if slowest.is_finite() => 10.0 / slowest,
                            None => 10.0,
                        };
                        let y = crate::mfun::Numeric::new(&["t".to_string()], &answer, prec);
                        let (mut ts, mut ys) = (Vec::new(), Vec::new());
                        for k in 0..=200u32 {
                            let x = Float::with_val(prec, horizon * f64::from(k.max(1)) / 200.0)
                                / if k == 0 { 1e6 } else { 1.0 };
                            if let Ok(v) = y.at(std::slice::from_ref(&x)) {
                                ts.push(if k == 0 { Float::new(prec) } else { x });
                                ys.push(v);
                            }
                        }
                        // the answer at the start is its limit from the right: the ratio of the
                        // leading coefficients when the degrees allow one, else nothing
                        let ahead = sys.den.len() + usize::from(step);
                        if let Some(first) = ys.first_mut() {
                            *first = match sys.num.len() + 1 == ahead {
                                true => complex_of(
                                    &tidy(div(sys.num[0].clone(), sys.den[0].clone())),
                                    prec,
                                )?
                                .real()
                                .clone(),
                                false => Float::new(prec),
                            };
                        }
                        // (what is left of rounding near the start is nothing)
                        let top = ys.iter().fold(Float::new(prec), |m, v| {
                            m.max(&Float::with_val(prec, v.abs_ref()))
                        });
                        for v in &mut ys {
                            if Float::with_val(prec, v.abs_ref())
                                < Float::with_val(prec, &top >> 40u32)
                            {
                                *v = Float::new(prec);
                            }
                        }
                        Ok(vec![shown(&ts), shown(&ys), Expr::Bool(false)])
                    }
                    Some(_) => {
                        let count = match a.get(1) {
                            Some(n) => {
                                crate::arrays::whole(n, "the number of samples", 1, 5000)? as usize
                            }
                            None => 40,
                        };
                        let drive = if step {
                            div(Expr::sym("z"), sub(Expr::sym("z"), Expr::one()))
                        } else {
                            Expr::one()
                        };
                        let values =
                            self.response(&sys, drive, Some(&Expr::int(count as i64)), who)?;
                        Ok(vec![
                            Expr::list((0..count as i64).map(Expr::int).collect()),
                            values,
                            Expr::Bool(true),
                        ])
                    }
                }
            }
        }
    }
}

fn order_of(e: &Expr) -> Option<std::cmp::Ordering> {
    crate::arrays::order(e, &Expr::zero()).ok()
}

/// The first samples of the answer to a blow of a sampled system: its numerator over its
/// denominator by long division in 1/z.
fn samples(sys: &Sys, count: usize) -> Res {
    let mut input = vec![Expr::one()];
    input.resize(count, Expr::zero());
    filtered(sys, &input)
}

/// The output of a sampled system for the samples of an input, by its equation of
/// differences.
fn filtered(sys: &Sys, input: &[Expr]) -> Res {
    let (num, den) = (&sys.num, &sys.den);
    if num.len() > den.len() {
        return Err("a sampled system whose numerator is of higher degree than its denominator looks ahead in time".into());
    }
    // y(k) = (sum b(j) u(k - j) - sum a(j) y(k - j)) / a(0), the numerator shifted by the excess of poles
    let lag = den.len() - num.len();
    let mut out: Vec<Expr> = Vec::with_capacity(input.len());
    for k in 0..input.len() {
        let mut terms = Vec::new();
        for (j, b) in num.iter().enumerate() {
            if let Some(i) = k.checked_sub(j + lag) {
                terms.push(mul(vec![b.clone(), input[i].clone()]));
            }
        }
        for (j, a) in den.iter().enumerate().skip(1) {
            if let Some(i) = k.checked_sub(j) {
                terms.push(neg(mul(vec![a.clone(), out[i].clone()])));
            }
        }
        out.push(tidy(div(add(terms), den[0].clone())));
    }
    Ok(Expr::list(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ints(v: &[i64]) -> Vec<Expr> {
        v.iter().map(|k| Expr::int(*k)).collect()
    }

    #[test]
    fn a_state_space_and_its_transfer_function() {
        let m = tf_to_ss(&ints(&[1, 2]), &ints(&[1, 3, 2])).unwrap();
        assert_eq!(m[0], vec![ints(&[-3, -2]), ints(&[1, 0])]);
        let (num, den) = ss_to_tf(&m).unwrap();
        assert_eq!(trimmed(num), ints(&[1, 2]));
        assert_eq!(den, ints(&[1, 3, 2]));
    }

    #[test]
    fn samples_follow_the_equation_of_differences() {
        // 1/(z - 1/2): 0, 1, 1/2, 1/4
        let sys = Sys {
            num: ints(&[1]),
            den: vec![Expr::one(), Expr::rat(-1, 2)],
            ts: Some(Expr::one()),
        };
        let found = samples(&sys, 4).unwrap();
        assert_eq!(
            found,
            Expr::list(vec![
                Expr::zero(),
                Expr::one(),
                Expr::rat(1, 2),
                Expr::rat(1, 4)
            ])
        );
    }

    #[test]
    fn the_equation_of_riccati_is_solved() {
        // x' = u, cost x^2 + u^2: X = 1
        let prec = 200;
        let one = |v: f64| vec![vec![Complex::with_val(prec, v)]];
        let x = riccati(&one(0.0), &one(1.0), &one(1.0)).unwrap();
        assert!(Float::with_val(prec, x[0][0].real() - 1u32).abs() < 1e-40);
        // x(k+1) = x(k) + u(k), cost x^2 + u^2: X = (1 + sqrt(5))/2
        let x = riccati_sampled(&one(1.0), &one(1.0), &one(1.0)).unwrap();
        let golden = (Float::with_val(prec, 5).sqrt() + 1u32) / 2u32;
        assert!(Float::with_val(prec, x[0][0].real() - golden).abs() < 1e-40);
    }

    fn said(input: &str) -> String {
        crate::eval::run_line(input).map_or_else(|e| format!("error: {e}"), |lines| lines.join("|"))
    }

    #[test]
    fn the_triangle_of_a_positive_matrix() {
        let prec = crate::mfun::bits();
        let m = |rows: [[i32; 2]; 2]| -> CMat {
            rows.iter()
                .map(|r| r.iter().map(|x| Complex::with_val(prec, *x)).collect())
                .collect()
        };
        let lower = cholesky(&m([[4, 2], [2, 3]])).unwrap();
        assert_eq!(lower[0][0].real().to_f64(), 2.0);
        assert_eq!(lower[1][0].real().to_f64(), 1.0);
        assert!(lower[0][1].real().is_zero());
        assert!((lower[1][1].real().to_f64() - 2f64.sqrt()).abs() < 1e-15);
        assert!(cholesky(&m([[1, 2], [2, 1]])).is_none());
        assert!(cholesky(&m([[1, 1], [1, 1]])).is_none());
    }

    #[test]
    fn values_below_the_rounding_are_nothing() {
        let prec = 128;
        let squares = [4.0, 1.0, 1e-60, -1e-40]
            .iter()
            .map(|x| Float::with_val(prec, *x))
            .collect();
        let found: Vec<f64> = weights(squares, prec).iter().map(Float::to_f64).collect();
        assert_eq!(found, vec![2.0, 1.0, 0.0, 0.0]);
        assert!(weights(Vec::new(), prec).is_empty());
    }

    #[test]
    fn systems_are_joined_where_they_stand() {
        assert_eq!(
            said("tf([1], [1, 1]) + tf([1], [1, 2])"),
            "tf([2, 3], [1, 3, 2])"
        );
        assert_eq!(said("2/tf([1, 3], [1, 1]) - 1"), "tf([1, -1], [1, 3])");
        assert_eq!(
            said("s := tf(\"s\"); g := 1/(s^2 + 3s + 2); pole(g)"),
            "tf([1, 0], [1])|tf([1], [1, 3, 2])|[-2, -1]"
        );
        assert_eq!(said("[tf([1], [1, 1]), 1] + 1"), "[tf([1, 2], [1, 1]), 2]");
        assert_eq!(
            said("tf([1], [1, 1])*[1, 2]"),
            "[tf([1], [1, 1]), tf([2], [1, 1])]"
        );
        assert_eq!(
            said("[[tf([1], [1, 1]), tf([1], [1, 2])]]*[[tf([1], [1, 3])], [2]]"),
            "[[tf([2, 9, 8], [1, 6, 11, 6])]]"
        );
        assert_eq!(
            said("sum([tf([1], [1, 1]), tf([1], [1, 2])])"),
            "tf([2, 3], [1, 3, 2])"
        );
        assert_eq!(said("[1, 2] + 1"), "[2, 3]");
        assert!(said("tf([1], [1, 1])^(1/2)").starts_with("error: a power of a system"));
        assert!(said("tf([1], [1, -1/2], 1) + tf([1], [1, 1])").starts_with("error: a sum"));
    }

    #[test]
    fn the_weights_of_the_states_and_fewer_of_them() {
        // 1/((s + 1)(s + 2)): the values of Hankel are (sqrt(17) +- 3)/24
        assert!(said("hsvd(tf([1], [1, 3, 2]))").starts_with("[0.29679606773406918957589207"));
        assert_eq!(said("hsvd(tf([1, 1], [1, 3, 2]))"), "[0.25, 0]");
        assert_eq!(said("dcgain(balred(tf([1, 2], [1, 3, 5, 4]), 1))"), "0.5");
        assert_eq!(
            said("pole(balred(tf([1], [1, 3, 2]), 2, \"truncate\"))"),
            said("pole(part(balreal(tf([1], [1, 3, 2])), 1))")
        );
        // a state that the system does not need is taken out first
        assert_eq!(
            said("balreal(tf([1, 1], [1, 3, 2]))"),
            "[ss([[-2]], [[1]], [[1]], [[0]]), [0.25]]"
        );
        assert_eq!(
            said("tf(balred(ss([[-1, 2], [0, -3]], [[1], [1]], [[1, 1]], [[0]]), 2))"),
            "tf([2], [1, 1])"
        );
        assert_eq!(
            said("hsvd(5)"),
            "error: hsvd: the system has no states, it is a gain alone"
        );
        assert_eq!(
            said("covar(tf([1], [1, 3, 2]), 1)"),
            "[1/12, [[1/6, 0], [0, 1/12]]]"
        );
    }

    #[test]
    fn a_system_left_alone_and_its_controller() {
        let plant = "ss([[0, 1], [-2, -3]], [[0], [1]], [[1, 0]], [[0]])";
        assert_eq!(
            said(&format!("initial({plant}, [1, 0])")),
            "2e^(-t) - e^(-2t)"
        );
        assert_eq!(
            said("initial(ss([[1/2]], [[1]], [[1]], [[0]], 1), [3], 4)"),
            "[3, 3/2, 3/4, 3/8]"
        );
        assert_eq!(
            said(&format!("reg({plant}, [[1, 2]], [[3], [4]])")),
            "ss([[-3, 1], [-7, -5]], [[3], [4]], [[-1, -2]], [[0]])"
        );
        assert_eq!(said("ord2(2, 1/2)"), "tf([1], [1, 2, 4])");
        assert_eq!(
            said("filt([1], [1, 1/2, 1/4])"),
            "tf([1, 0, 0], [1, 1/2, 1/4], -1)"
        );
        assert_eq!(said("sigma(6)"), "12");
        assert_eq!(
            said("gensig(\"square\", 4, 4, 1)"),
            "[[0, 0, 1, 1, 0], [0, 1, 2, 3, 4]]"
        );
    }
}
