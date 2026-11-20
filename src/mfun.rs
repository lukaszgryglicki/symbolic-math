//! Commands that work with functions and with other commands: zeros and minima found by
//! numbers, integrals of functions, differential equations step by step, fits of curves,
//! tests of statistics, and the helpers of symbolic work.

use crate::arrays::{order, tidy, whole, word};
use crate::canon::{add, div, mul, neg, pow, sub, subst, subst_all, subst_sym};
use crate::eval::Session;
use crate::expr::{Expr, Rel};
use crate::funcs::func;
use crate::matrix::{self, Rows};
use crate::numeric;
use crate::simplify::{is_zero, simplify};
use rug::{Complex, Float};
use std::cmp::Ordering;
use std::rc::Rc;

type Res = Result<Expr, String>;

const IMWRITE: &str = "imwrite(matrix, \"picture.png\"[, bits]): a matrix is a gray picture, a list of 3 matrices is red, green and blue, with 2 or 4 the last one is alpha; the name ends in .png or .jxl";

/// The planes of a picture: one matrix, or a list of matrices of one size.
fn planes(data: &Expr) -> Option<Vec<Vec<Vec<f64>>>> {
    let plane = |m: &Expr| -> Option<Vec<Vec<f64>>> {
        let rows = matrix::rows(m)?;
        rows.iter()
            .map(|r| {
                r.iter()
                    .map(crate::plot::number)
                    .collect::<Option<Vec<f64>>>()
            })
            .collect()
    };
    if let Some(one) = plane(data) {
        return Some(vec![one]);
    }
    let all: Vec<Vec<Vec<f64>>> = data.as_list()?.iter().map(plane).collect::<Option<_>>()?;
    let same = all
        .iter()
        .all(|p| p.len() == all[0].len() && p[0].len() == all[0][0].len());
    (matches!(all.len(), 1..=4) && same).then_some(all)
}

/// `imwrite(A, "picture.png")`: numbers from 0 to 1 are shares of white, whole numbers above 1
/// are samples of 8 or 16 bits.
fn write_picture(data: &Expr, file: &Expr, bits: Option<&Expr>) -> Result<Vec<String>, String> {
    use crate::imgfile::{Encoding, Format, Image, encode, write};
    let path = crate::radix::text_of(file).ok_or(IMWRITE)?;
    let lower = path.to_ascii_lowercase();
    let format = if lower.ends_with(".png") {
        Format::Png
    } else if lower.ends_with(".jxl") {
        Format::Jxl
    } else {
        return Err(IMWRITE.to_string());
    };
    let planes = planes(data).ok_or(IMWRITE)?;
    let (height, width) = (planes[0].len(), planes[0][0].len());
    let values = || planes.iter().flatten().flatten().copied();
    if values().any(|v| !v.is_finite() || v < 0.0) {
        return Err("imwrite: the samples of a picture are numbers from 0 on".into());
    }
    let most = values().fold(0.0, f64::max);
    let chosen = match bits {
        None => None,
        Some(b) => {
            let range = if format == Format::Png {
                "8 or 16"
            } else {
                "1 to 16"
            };
            let fits = |b: &i64| {
                if format == Format::Png {
                    matches!(b, 8 | 16)
                } else {
                    (1..=16).contains(b)
                }
            };
            Some(
                b.as_i64()
                    .filter(fits)
                    .ok_or_else(|| format!("imwrite: the bits of a sample are {range}, not {b}"))?
                    as u32,
            )
        }
    };
    let shares = most <= 1.0;
    let bits = match chosen {
        Some(b) => b,
        None if shares || most <= 255.0 => 8,
        None if most <= 65535.0 => 16,
        None => {
            return Err(
                "imwrite: samples are numbers from 0 to 1, or whole numbers up to 255 or 65535"
                    .into(),
            );
        }
    };
    let top = f64::from((1u32 << bits) - 1);
    if !shares && (most > top || values().any(|v| v.fract() != 0.0)) {
        return Err(format!(
            "imwrite: samples of {bits} bits are whole numbers up to {top}, or numbers from 0 to 1"
        ));
    }
    let scale = if shares { 1.0 } else { top };
    let mut samples = Vec::with_capacity(width * height * planes.len());
    for row in 0..height {
        for column in 0..width {
            for plane in &planes {
                samples.push((plane[row][column] / scale) as f32);
            }
        }
    }
    let image = Image {
        width,
        height,
        colors: if planes.len() >= 3 { 3 } else { 1 },
        alpha: planes.len() % 2 == 0,
        data: samples,
        depth: None,
    };
    let encoding = Encoding {
        format,
        bits,
        level: 9,
        distance: None,
        effort: 7,
        rate: (1, 1),
        alpha_bits: None,
    };
    let bytes = encode(1, &mut |_| Ok(image.clone()), &encoding)?;
    write(path, &bytes)?;
    let kind = ["gray", "gray + alpha", "RGB", "RGB + alpha"][planes.len() - 1];
    Ok(vec![format!(
        "saved {path}: {}, {width} x {height} pixels, {bits} bit{}, {kind}, {}, {} bytes",
        if format == Format::Png {
            "PNG"
        } else {
            "JPEG XL"
        },
        if bits == 1 { "" } else { "s" },
        if format == Format::Png {
            "level 9"
        } else {
            "lossless, effort 7"
        },
        bytes.len()
    )])
}

/// `imread("picture.png")`: the samples as whole numbers; a gray picture is one matrix, a
/// picture with more channels the list of their matrices. `frame`: which frame of a moving
/// picture, the first one is 1.
fn read_picture(file: &Expr, frame: Option<&Expr>) -> Res {
    let usage = "imread(\"picture.png\") or imread(\"picture.jxl\"); imread(\"picture.jxl\", 3): the third frame";
    let path = crate::radix::text_of(file).ok_or(usage)?;
    let frame = match frame {
        None => 0,
        Some(k) => k.as_i64().filter(|k| *k >= 1).ok_or(usage)? as usize - 1,
    };
    let p = crate::imgfile::read_frame(path, frame).map_err(|e| format!("imread: {e}"))?;
    let plane = |k: usize| -> Expr {
        Expr::list(
            (0..p.height)
                .map(|row| {
                    Expr::list(
                        (0..p.width)
                            .map(|column| {
                                Expr::int(i64::from(
                                    p.samples[(row * p.width + column) * p.channels + k],
                                ))
                            })
                            .collect(),
                    )
                })
                .collect(),
        )
    };
    Ok(if p.channels == 1 {
        plane(0)
    } else {
        Expr::list((0..p.channels).map(plane).collect())
    })
}

thread_local! {
    /// When `tic()` was called.
    static STARTED: std::cell::Cell<Option<std::time::Instant>> = const { std::cell::Cell::new(None) };
}

/// A function of real numbers ready to be worked out many times.
pub(crate) struct Numeric {
    params: Vec<Rc<str>>,
    body: Expr,
    prec: u32,
}

impl Numeric {
    pub(crate) fn new(params: &[String], body: &Expr, prec: u32) -> Numeric {
        Numeric {
            params: params.iter().map(|p| Rc::from(p.as_str())).collect(),
            body: body.clone(),
            prec,
        }
    }

    fn complex(&self, x: &[Float]) -> Result<Complex, String> {
        let mut env = numeric::Env::new();
        for (p, v) in self.params.iter().zip(x) {
            env.insert(p.clone(), Complex::with_val(self.prec, v));
        }
        numeric::evalf_env(&self.body, self.prec, &env).map_err(|e| e.to_string())
    }

    pub(crate) fn at(&self, x: &[Float]) -> Result<Float, String> {
        let (re, im) = self.complex(x)?.into_real_imag();
        if !re.is_finite() {
            return Err("the function has no finite value there".into());
        }
        if !im.is_zero() {
            let size = Float::with_val(self.prec, re.abs_ref()) + 1u32;
            if Float::with_val(self.prec, im.abs_ref()) > size >> (self.prec / 2) {
                return Err("the function has a value that is not real there".into());
            }
        }
        Ok(re)
    }
}

pub(crate) fn value(x: &Float) -> Expr {
    let prec = x.prec();
    numeric::to_float_expr(Complex::with_val(prec, x), numeric::digits())
}

pub(crate) fn bits() -> u32 {
    numeric::bits_for(numeric::digits()) + 64
}

pub(crate) fn real(e: &Expr, prec: u32, name: &str) -> Result<Float, String> {
    numeric::real_value(e, numeric::digits() + 10)
        .map(|v| Float::with_val(prec, v))
        .ok_or_else(|| format!("{name}: {e} is not a real number"))
}

/// The solution of a small system of linear equations with decimal numbers.
fn linear(mut m: Vec<Vec<Float>>, mut b: Vec<Float>) -> Option<Vec<Float>> {
    let n = b.len();
    for col in 0..n {
        let best = (col..n).max_by(|i, j| {
            m[*i][col]
                .clone()
                .abs()
                .partial_cmp(&m[*j][col].clone().abs())
                .unwrap_or(Ordering::Equal)
        })?;
        if m[best][col].is_zero() {
            return None;
        }
        m.swap(col, best);
        b.swap(col, best);
        for row in col + 1..n {
            let f = Float::with_val(m[row][col].prec(), &m[row][col] / &m[col][col]);
            if f.is_zero() {
                continue;
            }
            let (above, below) = m.split_at_mut(row);
            for (entry, pivot) in below[0][col..].iter_mut().zip(&above[col][col..]) {
                *entry -= Float::with_val(f.prec(), &f * pivot);
            }
            let step = Float::with_val(f.prec(), &f * &b[col]);
            b[row] -= step;
        }
    }
    let mut x = vec![Float::new(b[0].prec()); n];
    for row in (0..n).rev() {
        let mut s = b[row].clone();
        for k in row + 1..n {
            s -= Float::with_val(s.prec(), &m[row][k] * &x[k]);
        }
        x[row] = s / &m[row][row];
    }
    Some(x)
}

/// The name of a variable: the one given, else the only one of the expression, else `x`.
fn variable(given: Option<&Expr>, e: &Expr, name: &str) -> Result<String, String> {
    match given {
        Some(Expr::Sym(s)) => Ok(s.to_string()),
        Some(other) => Err(format!("{name}: {other} is not a variable")),
        None => {
            let syms = e.free_syms();
            match syms.len() {
                0 => Ok("x".to_string()),
                1 => Ok(syms
                    .iter()
                    .next()
                    .map(|s| s.to_string())
                    .unwrap_or_default()),
                _ if syms.contains("x") => Ok("x".to_string()),
                _ => Err(format!(
                    "{name}: name the variable, the expression has {}",
                    syms.len()
                )),
            }
        }
    }
}

fn names(e: &Expr, name: &str) -> Result<Vec<String>, String> {
    let one = |x: &Expr| {
        x.sym_name()
            .map(str::to_string)
            .ok_or_else(|| format!("{name}: {x} is not a variable"))
    };
    match e {
        Expr::List(items) => items.iter().map(one).collect(),
        other => Ok(vec![one(other)?]),
    }
}

fn as_equation(e: &Expr) -> Expr {
    match e {
        Expr::Rel(Rel::Eq, l, r) => sub((**l).clone(), (**r).clone()),
        other => other.clone(),
    }
}

/// The rows `[statistic, probability]` of a test: `h` is 1 when the probability is under 5%.
fn verdict(p: Expr) -> Expr {
    let p = numeric::n(&p, numeric::digits());
    let small = order(&p, &Expr::rat(1, 20)).is_ok_and(|o| o == Ordering::Less);
    Expr::list(vec![Expr::int(i64::from(small)), p])
}

fn mean(v: &[Expr]) -> Expr {
    tidy(div(add(v.to_vec()), Expr::int(v.len() as i64)))
}

/// `a` and `b` of `a*x + b`, for an `a` whose sign is known: with that sign.
fn straight(u: &Expr, x: &str) -> Option<(Expr, Expr, i64)> {
    let a = simplify(&crate::diff::diff(u, x));
    if !a.free_of(x) || is_zero(&a) {
        return None;
    }
    let b = simplify(&sub(u.clone(), mul(vec![a.clone(), Expr::sym(x)])));
    if !b.free_of(x) {
        return None;
    }
    let sign = if crate::assume::is_positive(&a) {
        1
    } else if crate::assume::is_negative(&a) {
        -1
    } else {
        return None;
    };
    Some((a, b, sign))
}

/// The Hilbert transform of `f(x)` at `t`: the mean of `f(x)/(t - x)` over all x, over pi.
fn hilbert(f: &Expr, x: &str, t: &Expr) -> Option<Expr> {
    if f.free_of(x) {
        return Some(Expr::zero());
    }
    let at = |u: &Expr| subst_sym(u, x, t);
    match f {
        Expr::Add(terms) => {
            let parts: Option<Vec<Expr>> = terms.iter().map(|p| hilbert(p, x, t)).collect();
            Some(add(parts?))
        }
        Expr::Mul(factors) => {
            let (free, bound): (Vec<Expr>, Vec<Expr>) =
                factors.iter().cloned().partition(|p| p.free_of(x));
            if !free.is_empty() {
                return Some(mul(vec![mul(free), hilbert(&mul(bound), x, t)?]));
            }
            // sin(x)/x, and x/(x^2 + c)
            let whole = mul(bound);
            let var = Expr::sym(x);
            let sinc = div(func("sin", vec![var.clone()]), var.clone());
            if is_zero(&simplify(&sub(whole.clone(), sinc))) {
                return Some(div(
                    sub(Expr::one(), func("cos", vec![t.clone()])),
                    t.clone(),
                ));
            }
            let over = simplify(&div(var.clone(), whole));
            let c = simplify(&sub(over, pow(var, Expr::int(2))));
            (c.free_of(x) && crate::assume::is_positive(&c)).then(|| {
                neg(div(
                    pow(c.clone(), Expr::half()),
                    add(vec![pow(t.clone(), Expr::int(2)), c]),
                ))
            })
        }
        Expr::Fn(name, args) if args.len() == 1 => {
            let (_, _, sign) = straight(&args[0], x)?;
            let (u, sign) = (at(&args[0]), Expr::int(sign));
            match &**name {
                "sin" => Some(neg(mul(vec![sign, func("cos", vec![u])]))),
                "cos" => Some(mul(vec![sign, func("sin", vec![u])])),
                "diracdelta" => {
                    let (a, b, _) = straight(&args[0], x)?;
                    // the spike stands at -b/a and weighs 1/|a|
                    let place = neg(div(b, a.clone()));
                    Some(div(
                        Expr::one(),
                        mul(vec![
                            Expr::pi(),
                            func("abs", vec![a]),
                            sub(t.clone(), place),
                        ]),
                    ))
                }
                _ => None,
            }
        }
        Expr::Pow(base, power) if base.is_sym("e") => {
            // e^(i*(a*x + b))
            let phase = simplify(&div((**power).clone(), Expr::i()));
            let (_, _, sign) = straight(&phase, x)?;
            Some(mul(vec![Expr::int(-sign), Expr::i(), at(f)]))
        }
        Expr::Pow(base, power) if power.as_i64() == Some(-1) => {
            if base.is_sym(x) {
                return Some(neg(mul(vec![
                    Expr::pi(),
                    func("diracdelta", vec![t.clone()]),
                ])));
            }
            // 1/(x^2 + c)
            let c = simplify(&sub((**base).clone(), pow(Expr::sym(x), Expr::int(2))));
            (c.free_of(x) && crate::assume::is_positive(&c)).then(|| {
                div(
                    t.clone(),
                    mul(vec![
                        pow(c.clone(), Expr::half()),
                        add(vec![pow(t.clone(), Expr::int(2)), c]),
                    ]),
                )
            })
        }
        _ => None,
    }
}

/// The parameters that make the sum of the squares of some expressions least: the steps of
/// Gauss and Newton with the damping of Levenberg and Marquardt, the slopes worked out exactly.
fn least_squares(params: &[String], left: &[Expr], start: &[Expr], name: &str) -> Res {
    let prec = bits();
    let k = params.len();
    let values: Vec<Numeric> = left.iter().map(|r| Numeric::new(params, r, prec)).collect();
    let slopes: Vec<Vec<Numeric>> = left
        .iter()
        .map(|r| {
            params
                .iter()
                .map(|p| Numeric::new(params, &crate::diff::diff(r, p), prec))
                .collect()
        })
        .collect();
    let mut p: Vec<Float> = start
        .iter()
        .map(|e| real(e, prec, name))
        .collect::<Result<_, _>>()?;
    let misfit = |p: &[Float]| -> Result<(Vec<Float>, Float), String> {
        let mut total = Float::new(prec);
        let mut r = Vec::with_capacity(values.len());
        for v in &values {
            let d = v.at(p)?;
            total += Float::with_val(prec, &d * &d);
            r.push(d);
        }
        Ok((r, total))
    };
    let (mut r, mut total) = misfit(&p).map_err(|e| format!("{name}: {e}"))?;
    let mut damp = Float::with_val(prec, 0.001);
    let tol = Float::with_val(prec, Float::i_pow_u(10, numeric::digits() + 2)).recip();
    for _ in 0..2000 {
        let mut normal = vec![vec![Float::new(prec); k]; k];
        let mut right = vec![Float::new(prec); k];
        for (row, d) in slopes.iter().zip(&r) {
            let row: Vec<Float> = row
                .iter()
                .map(|s| s.at(&p))
                .collect::<Result<_, _>>()
                .map_err(|e| format!("{name}: {e}"))?;
            for i in 0..k {
                right[i] -= Float::with_val(prec, &row[i] * d);
                for j in 0..k {
                    normal[i][j] += Float::with_val(prec, &row[i] * &row[j]);
                }
            }
        }
        let mut moved = false;
        for _ in 0..60 {
            let mut m = normal.clone();
            for (i, row) in m.iter_mut().enumerate() {
                let extra = Float::with_val(prec, &row[i] * &damp);
                row[i] += extra;
                if row[i].is_zero() {
                    row[i] = damp.clone();
                }
            }
            let Some(step) = linear(m, right.clone()) else {
                damp *= 10u32;
                continue;
            };
            let next: Vec<Float> = p
                .iter()
                .zip(&step)
                .map(|(x, s)| Float::with_val(prec, x + s))
                .collect();
            match misfit(&next) {
                Ok((r2, t2)) if t2 <= total => {
                    let mut size = Float::new(prec);
                    let mut change = Float::new(prec);
                    for (x, s) in next.iter().zip(&step) {
                        size = size.max(&Float::with_val(prec, x.abs_ref()));
                        change = change.max(&Float::with_val(prec, s.abs_ref()));
                    }
                    (p, r, total) = (next, r2, t2);
                    damp /= 10u32;
                    moved = change > Float::with_val(prec, (size + 1u32) * &tol);
                    break;
                }
                _ => damp *= 10u32,
            }
        }
        if !moved {
            break;
        }
    }
    Ok(Expr::list(p.iter().map(value).collect()))
}

fn variance(v: &[Expr]) -> Res {
    crate::stats::call("variance", &[Expr::list(v.to_vec())])
        .unwrap_or_else(|| Err("no variance".into()))
}

fn data<'a>(e: &'a Expr, name: &str) -> Result<&'a [Expr], String> {
    e.as_list()
        .filter(|l| l.len() >= 2 && l.iter().all(|x| x.as_list().is_none()))
        .ok_or_else(|| format!("{name} needs a list of two or more numbers"))
}

impl Session {
    /// A function given as `@(x) expr`, `x -> expr`, the name of a function or an expression
    /// with as many names as arguments: its arguments under fresh names, and its value.
    pub(crate) fn function_of(
        &mut self,
        f: &Expr,
        count: usize,
        name: &str,
    ) -> Result<(Vec<String>, Expr), String> {
        let fresh: Vec<String> = (0..count).map(|k| format!("_u{k}")).collect();
        let rename = |params: &[String], body: &Expr| {
            let pairs: Vec<(String, Expr)> = params
                .iter()
                .zip(&fresh)
                .map(|(p, f)| (p.clone(), Expr::sym(f)))
                .collect();
            subst_all(body, &pairs)
        };
        if let Some((params, body)) = crate::lists::lambda(f) {
            if params.len() != count {
                return Err(format!(
                    "{name} needs a function of {count} argument(s), {f} takes {}",
                    params.len()
                ));
            }
            let body = rename(&params, body);
            let own: Vec<Expr> = fresh.iter().map(|p| Expr::sym(p)).collect();
            let body = self.with_bound(&fresh, own, |s| s.eval(&body))?;
            return Ok((fresh, body));
        }
        if self.is_function(f) {
            let args: Vec<Expr> = fresh.iter().map(|p| Expr::sym(p)).collect();
            let body = self.apply(f, &args)?;
            return Ok((fresh, body));
        }
        let syms: Vec<String> = f.free_syms().iter().map(|s| s.to_string()).collect();
        if syms.len() == count || (syms.is_empty() && count == 1) {
            let body = rename(&syms, f);
            return Ok((fresh, body));
        }
        Err(format!(
            "{name} needs a function of {count} argument(s); {f} has {} name(s): @(x) ... says which are the arguments",
            syms.len()
        ))
    }

    /// Whether a value stands for a function: `x -> expr` or the name of one.
    pub(crate) fn function_like(&self, e: &Expr) -> bool {
        crate::lists::lambda(e).is_some() || (e.sym_name().is_some() && self.is_function(e))
    }

    /// The density, the probability or the place of a distribution at every entry of a list.
    fn at_each(&mut self, name: &str, dist: &Expr, places: &Expr) -> Res {
        match places.as_list() {
            Some(items) => Ok(Expr::list(
                items
                    .iter()
                    .map(|x| self.at_each(name, dist, x))
                    .collect::<Result<_, _>>()?,
            )),
            None => self.call(name, &[dist.clone(), places.clone()]),
        }
    }

    /// A zero of a real function near a start or between two ends with a change of sign.
    fn fzero(&mut self, a: &[Expr]) -> Res {
        let (params, body) = self.function_of(&a[0], 1, "fzero")?;
        let prec = bits();
        let f = Numeric::new(&params, &body, prec);
        let at = |x: &Float| f.at(std::slice::from_ref(x));
        let opposite = |p: &Float, q: &Float| p.is_sign_negative() != q.is_sign_negative();
        let (mut lo, mut flo, mut hi, mut fhi) = match a[1].as_list() {
            Some([p, q]) => {
                let (p, q) = (real(p, prec, "fzero")?, real(q, prec, "fzero")?);
                let (fp, fq) = (at(&p)?, at(&q)?);
                if fp.is_zero() {
                    return Ok(value(&p));
                }
                if fq.is_zero() {
                    return Ok(value(&q));
                }
                if !opposite(&fp, &fq) {
                    return Err("fzero: the function has one sign at both ends of the range".into());
                }
                (p, fp, q, fq)
            }
            Some(_) => return Err("fzero(f, start) or fzero(f, [a, b])".into()),
            None => {
                let x0 = real(&a[1], prec, "fzero")?;
                let f0 = at(&x0).map_err(|e| format!("fzero: {e}"))?;
                if f0.is_zero() {
                    return Ok(value(&x0));
                }
                let mut dx = if x0.is_zero() {
                    Float::with_val(prec, 1) / 50u32
                } else {
                    Float::with_val(prec, x0.abs_ref()) / 50u32
                };
                let grow = Float::with_val(prec, 2).sqrt();
                let (mut left, mut right) = (true, true);
                let mut found = None;
                for _ in 0..2000 {
                    if !left && !right {
                        break;
                    }
                    dx *= &grow;
                    for (open, sign) in [(&mut left, -1), (&mut right, 1)] {
                        if !*open || found.is_some() {
                            continue;
                        }
                        let x = Float::with_val(prec, &x0 + Float::with_val(prec, &dx * sign));
                        match at(&x) {
                            Ok(fx) if fx.is_zero() => return Ok(value(&x)),
                            Ok(fx) if opposite(&fx, &f0) => found = Some((x, fx)),
                            Ok(_) => {}
                            Err(_) => *open = false,
                        }
                    }
                    if found.is_some() {
                        break;
                    }
                }
                let (x, fx) = found.ok_or("fzero: no change of sign found near the start")?;
                (x0, f0, x, fx)
            }
        };
        let tol = Float::with_val(prec, Float::i_pow_u(10, numeric::digits() + 3)).recip();
        let mut side = 0;
        for round in 0..20_000 {
            let width = Float::with_val(prec, &hi - &lo).abs();
            let size = Float::with_val(prec, hi.abs_ref()) + 1u32;
            if width <= Float::with_val(prec, &size * &tol) {
                break;
            }
            let mut c = Float::with_val(prec, &lo * &fhi) - Float::with_val(prec, &hi * &flo);
            c /= Float::with_val(prec, &fhi - &flo);
            let (least, most) = if lo < hi { (&lo, &hi) } else { (&hi, &lo) };
            if round % 64 == 63 || !(c > *least && c < *most) {
                c = Float::with_val(prec, &lo + &hi) / 2u32;
            }
            let fc = at(&c).map_err(|e| format!("fzero: {e}"))?;
            if fc.is_zero() {
                return Ok(value(&c));
            }
            if opposite(&fc, &fhi) {
                (lo, flo) = (c, fc);
                if side == 1 {
                    fhi /= 2u32;
                }
                side = 1;
            } else {
                (hi, fhi) = (c, fc);
                if side == -1 {
                    flo /= 2u32;
                }
                side = -1;
            }
        }
        Ok(value(
            if Float::with_val(prec, flo.abs_ref()) < Float::with_val(prec, fhi.abs_ref()) {
                &lo
            } else {
                &hi
            },
        ))
    }

    /// The place of the least value of a real function between two ends (golden section).
    fn fminbnd(&mut self, a: &[Expr]) -> Res {
        let (params, body) = self.function_of(&a[0], 1, "fminbnd")?;
        // (a minimum is seen through the square of the distance: twice the bits)
        let prec = 2 * bits();
        let f = Numeric::new(&params, &body, prec);
        let at = |x: &Float| {
            f.at(std::slice::from_ref(x))
                .map_err(|e| format!("fminbnd: {e}"))
        };
        let (mut lo, mut hi) = (real(&a[1], prec, "fminbnd")?, real(&a[2], prec, "fminbnd")?);
        if lo > hi {
            return Err("fminbnd(f, a, b): a must not be above b".into());
        }
        let ratio = (Float::with_val(prec, 5).sqrt() - 1u32) / 2u32;
        let inner = |lo: &Float, hi: &Float| {
            let step = Float::with_val(prec, hi - lo) * &ratio;
            (
                Float::with_val(prec, hi - &step),
                Float::with_val(prec, lo + &step),
            )
        };
        let (mut c, mut d) = inner(&lo, &hi);
        let (mut fc, mut fd) = (at(&c)?, at(&d)?);
        let tol = Float::with_val(prec, Float::i_pow_u(10, numeric::digits() + 2)).recip();
        for _ in 0..100_000 {
            let size =
                Float::with_val(prec, c.abs_ref()) + Float::with_val(prec, d.abs_ref()) + 1u32;
            if Float::with_val(prec, &hi - &lo) <= Float::with_val(prec, &size * &tol) {
                break;
            }
            if fc < fd {
                hi = d;
                (d, fd) = (c, fc);
                c = inner(&lo, &hi).0;
                fc = at(&c)?;
            } else {
                lo = c;
                (c, fc) = (d, fd);
                d = inner(&lo, &hi).1;
                fd = at(&d)?;
            }
        }
        Ok(value(&(Float::with_val(prec, &lo + &hi) / 2u32)))
    }

    /// The place of a least value of a function of one or more real numbers near a start
    /// (the simplex of Nelder and Mead).
    fn fminsearch(&mut self, a: &[Expr]) -> Res {
        let start: Vec<Expr> = match a[1].as_list() {
            Some(items) => items.to_vec(),
            None => vec![a[1].clone()],
        };
        let n = start.len();
        if n == 0 {
            return Err("fminsearch(f, start): the start is a number or a list of numbers".into());
        }
        let (params, body) = self.function_of(&a[0], n, "fminsearch")?;
        let prec = 2 * bits();
        let f = Numeric::new(&params, &body, prec);
        let at = |x: &[Float]| f.at(x).map_err(|e| format!("fminsearch: {e}"));
        let x0: Vec<Float> = start
            .iter()
            .map(|e| real(e, prec, "fminsearch"))
            .collect::<Result<_, _>>()?;
        let mut simplex: Vec<(Vec<Float>, Float)> = vec![(x0.clone(), at(&x0)?)];
        for k in 0..n {
            let mut x = x0.clone();
            if x[k].is_zero() {
                x[k] = Float::with_val(prec, 0.00025);
            } else {
                x[k] *= Float::with_val(prec, 1.05);
            }
            let fx = at(&x)?;
            simplex.push((x, fx));
        }
        let tol = Float::with_val(prec, Float::i_pow_u(10, numeric::digits() + 2)).recip();
        let along = |from: &[Float], to: &[Float], t: f64| -> Vec<Float> {
            from.iter()
                .zip(to)
                .map(|(p, q)| Float::with_val(prec, p + Float::with_val(prec, q - p) * t))
                .collect()
        };
        for _ in 0..200_000 {
            simplex.sort_by(|p, q| p.1.partial_cmp(&q.1).unwrap_or(Ordering::Equal));
            let mut spread = Float::new(prec);
            let mut size = Float::with_val(prec, 1);
            for (x, _) in &simplex[1..] {
                for (p, q) in x.iter().zip(&simplex[0].0) {
                    spread = spread.max(&Float::with_val(prec, p - q).abs());
                    size = size.max(&Float::with_val(prec, q.abs_ref()));
                }
            }
            if spread <= Float::with_val(prec, &size * &tol) {
                break;
            }
            let mut centre = vec![Float::new(prec); n];
            for (x, _) in &simplex[..n] {
                for (c, v) in centre.iter_mut().zip(x) {
                    *c += v;
                }
            }
            for c in centre.iter_mut() {
                *c /= n as u32;
            }
            let worst = simplex[n].clone();
            let reflected = along(&worst.0, &centre, 2.0);
            let fr = at(&reflected)?;
            if fr < simplex[0].1 {
                let expanded = along(&worst.0, &centre, 3.0);
                let fe = at(&expanded)?;
                simplex[n] = if fe < fr {
                    (expanded, fe)
                } else {
                    (reflected, fr)
                };
            } else if fr < simplex[n - 1].1 {
                simplex[n] = (reflected, fr);
            } else {
                let (toward, limit) = if fr < worst.1 {
                    (1.5, fr.clone())
                } else {
                    (0.5, worst.1.clone())
                };
                let pulled = along(&worst.0, &centre, toward);
                let fp = at(&pulled)?;
                if fp <= limit {
                    simplex[n] = (pulled, fp);
                } else {
                    let best = simplex[0].0.clone();
                    for item in simplex.iter_mut().skip(1) {
                        item.0 = along(&best, &item.0, 0.5);
                        item.1 = at(&item.0)?;
                    }
                }
            }
        }
        let best = &simplex[0].0;
        Ok(if a[1].as_list().is_some() {
            Expr::list(best.iter().map(value).collect())
        } else {
            value(&best[0])
        })
    }

    /// The parameters of a model `f(p1, ..., pk, x)` that fit points best (least squares;
    /// the steps of Gauss and Newton, damped as Levenberg and Marquardt do).
    fn curve_fit(&mut self, a: &[Expr]) -> Res {
        let usage = "lsqcurvefit(@(a, b, x) model, [start of a, of b], [x values], [y values])";
        let start = a[1].as_list().filter(|s| !s.is_empty()).ok_or(usage)?;
        let (xs, ys) = (a[2].as_list().ok_or(usage)?, a[3].as_list().ok_or(usage)?);
        if xs.len() != ys.len() || xs.len() < start.len() {
            return Err(format!(
                "lsqcurvefit: {} x values, {} y values and {} parameters",
                xs.len(),
                ys.len(),
                start.len()
            ));
        }
        let k = start.len();
        let (params, body) = self.function_of(&a[0], k + 1, "lsqcurvefit")?;
        // what is left of every y value when the model is taken off
        let left: Vec<Expr> = xs
            .iter()
            .zip(ys)
            .map(|(x, y)| sub(y.clone(), subst_sym(&body, &params[k], x)))
            .collect();
        least_squares(&params[..k], &left, start, "lsqcurvefit")
    }

    /// `lsqnonlin(@(a, b) [what is left], [start of a, of b])`: the parameters that make the
    /// sum of the squares least.
    fn nonlinear_squares(&mut self, a: &[Expr]) -> Res {
        let usage = "lsqnonlin(@(a, b) [expressions], [start of a, of b]): the a and b that make the sum of the squares of the expressions least";
        let start: Vec<Expr> = match a[1].as_list() {
            Some(items) if !items.is_empty() => items.to_vec(),
            Some(_) => return Err(usage.to_string()),
            None => vec![a[1].clone()],
        };
        let (params, body) = self.function_of(&a[0], start.len(), "lsqnonlin")?;
        let left: Vec<Expr> = match body.as_list() {
            Some(items) => items.to_vec(),
            None => vec![body],
        };
        if left.len() < start.len() {
            return Err(format!(
                "lsqnonlin: {} expression(s) for {} parameter(s): as many or more are needed",
                left.len(),
                start.len()
            ));
        }
        let found = least_squares(&params, &left, &start, "lsqnonlin")?;
        Ok(match (a[1].as_list(), found.as_list()) {
            (None, Some([one])) => one.clone(),
            _ => found,
        })
    }

    /// `integral(f, a, b)`, `integral2(f, xa, xb, ya, yb)`, `integral3(...)`: the integral of
    /// a function; the inner ends may be functions of the outer variables.
    fn integral_of(&mut self, name: &str, a: &[Expr]) -> Res {
        let dims = (a.len() - 1) / 2;
        let (params, body) = self.function_of(&a[0], dims, name)?;
        let mut args = vec![body];
        for k in 0..dims {
            let outer: Vec<Expr> = params[..k].iter().map(|p| Expr::sym(p)).collect();
            args.push(Expr::sym(&params[k]));
            for end in [&a[1 + 2 * k], &a[2 + 2 * k]] {
                args.push(if k > 0 && self.function_like(end) {
                    self.apply(end, &outer)?
                } else {
                    end.clone()
                });
            }
        }
        // (the names of the arguments stand for themselves while the integral is worked out)
        let own: Vec<Expr> = params.iter().map(|p| Expr::sym(p)).collect();
        let exact = self.with_bound(&params, own, |s| s.call("integrate", &args))?;
        // (an integral without a closed form is a number all the same)
        if exact.has_fn("integrate") {
            return self.call("N", &[exact]);
        }
        Ok(exact)
    }

    /// `ode45(f, [t0, t1], y0)`: the solution of `y' = f(t, y)` at ten equal steps; with more
    /// times in the list at those times. Several unknowns: `f(t, y1, y2)` gives a list.
    fn ode(&mut self, name: &str, a: &[Expr]) -> Res {
        let usage = format!(
            "{name}(@(t, y) slope, [t0, t1], y0); several unknowns: {name}(@(t, x, y) [slope of x, slope of y], [t0, t1], [x0, y0])"
        );
        let y0: Vec<Expr> = match a[2].as_list() {
            Some(items) => items.to_vec(),
            None => vec![a[2].clone()],
        };
        let n = y0.len();
        let span = a[1]
            .as_list()
            .filter(|s| s.len() >= 2)
            .ok_or(usage.clone())?;
        let (params, body) = self.function_of(&a[0], n + 1, name)?;
        let rates: Vec<Expr> = match body.as_list() {
            Some(items) if items.len() == n => items.to_vec(),
            None if n == 1 => vec![body],
            _ => return Err(usage),
        };
        let digits = numeric::digits();
        let work = numeric::bits_for(digits) + 24;
        let prec = work + 64;
        let times: Vec<Expr> = if span.len() == 2 {
            (0..=10)
                .map(|k| {
                    tidy(add(vec![
                        span[0].clone(),
                        mul(vec![
                            Expr::rat(k, 10),
                            sub(span[1].clone(), span[0].clone()),
                        ]),
                    ]))
                })
                .collect()
        } else {
            span.to_vec()
        };
        let keys: Vec<Rc<str>> = params.iter().map(|p| Rc::from(p.as_str())).collect();
        let mut field = |t: &Float, y: &[Complex]| -> Option<Vec<Complex>> {
            let mut env = numeric::Env::new();
            env.insert(keys[0].clone(), Complex::with_val(prec, t));
            for (k, v) in keys[1..].iter().zip(y) {
                env.insert(k.clone(), v.clone());
            }
            rates
                .iter()
                .map(|r| numeric::evalf_env(r, prec, &env).ok())
                .collect()
        };
        let mut y: Vec<Complex> = y0
            .iter()
            .map(|e| {
                numeric::evalf_digits(e, digits + 8)
                    .map(|v| Complex::with_val(prec, v))
                    .map_err(|_| format!("{name}: the start value {e} is not a number"))
            })
            .collect::<Result<_, _>>()?;
        let mut t = real(&times[0], prec, name)?;
        let row = |at: &Expr, y: &[Complex]| {
            let mut row = vec![at.clone()];
            row.extend(crate::decomp::from_complex_list(y));
            Expr::list(row)
        };
        let mut rows = vec![row(&times[0], &y)];
        for at in &times[1..] {
            let next = real(at, prec, name)?;
            y = crate::odesys::integrate(&mut field, &t, &next, y, work)?;
            t = next;
            rows.push(row(at, &y));
        }
        Ok(Expr::list(rows))
    }

    /// Whether a condition holds for all values of its names, as far as can be shown.
    fn always(&mut self, cond: &Expr) -> Result<bool, String> {
        if let Some(t) = crate::logic::truth(cond) {
            return Ok(t);
        }
        match cond {
            Expr::Rel(op, l, r) => {
                let d = simplify(&sub((**l).clone(), (**r).clone()));
                let zero = is_zero(&d) || numeric::is_zero_identity(&d) == Some(true);
                Ok(match op {
                    Rel::Eq => zero,
                    Rel::Ne => false,
                    _ => {
                        let by_facts = match op {
                            Rel::Lt => crate::assume::is_negative(&d),
                            Rel::Gt => crate::assume::is_positive(&d),
                            Rel::Le => zero || crate::assume::is_nonnegative(&neg(d.clone())),
                            _ => zero || crate::assume::is_nonnegative(&d),
                        };
                        by_facts || {
                            let syms = d.free_syms();
                            syms.len() == 1 && {
                                let var = syms
                                    .iter()
                                    .next()
                                    .map(|s| s.to_string())
                                    .unwrap_or_default();
                                crate::reals::inequalities(&[(d.clone(), *op)], &var).is_ok_and(
                                    |lines| lines == [format!("{var} is any real number")],
                                )
                            }
                        }
                    }
                })
            }
            Expr::Fn(name, items) if &**name == "and" => {
                for c in items.iter() {
                    if !self.always(c)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Expr::Fn(name, items) if &**name == "or" => {
                for c in items.iter() {
                    if self.always(c)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Expr::List(items) => {
                for c in items.iter() {
                    if !self.always(c)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            other => Err(format!("isalways needs a condition, {other} is none")),
        }
    }

    /// The scalar potential of a field: `undefined` when the field has none.
    fn potential(&mut self, a: &[Expr]) -> Res {
        let field = a[0].as_list().ok_or("potential([field], [variables])")?;
        let vars = names(&a[1], "potential")?;
        if field.len() != vars.len() {
            return Err("potential: as many parts of the field as variables".into());
        }
        for i in 0..vars.len() {
            for j in i + 1..vars.len() {
                let cross = sub(
                    crate::diff::diff(&field[i], &vars[j]),
                    crate::diff::diff(&field[j], &vars[i]),
                );
                if !is_zero(&simplify(&cross)) {
                    return Ok(Expr::undefined());
                }
            }
        }
        let mut p = Expr::zero();
        for (part, v) in field.iter().zip(&vars) {
            let rest = simplify(&sub(part.clone(), crate::diff::diff(&p, v)));
            if rest.is_zero() {
                continue;
            }
            let piece = self.call("integrate", &[rest, Expr::sym(v)])?;
            if piece.has_fn("integrate") {
                return Err("potential: an integral has no closed form".into());
            }
            p = simplify(&add(vec![p, piece]));
        }
        let base: Vec<Expr> = match a.get(2) {
            Some(Expr::List(items)) if items.len() == vars.len() => items.to_vec(),
            Some(_) => return Err("potential([field], [variables], [base point])".into()),
            None => vec![Expr::zero(); vars.len()],
        };
        let pairs: Vec<(String, Expr)> = vars.iter().cloned().zip(base).collect();
        let at_base = simplify(&subst_all(&p, &pairs));
        Ok(
            if at_base.is_const() && !at_base.is_infinite() && !at_base.is_undefined() {
                simplify(&sub(p, at_base))
            } else {
                p
            },
        )
    }

    /// A vector potential of a field in space without sources: `undefined` when it has some.
    fn vector_potential(&mut self, a: &[Expr]) -> Res {
        let field = a[0]
            .as_list()
            .filter(|f| f.len() == 3)
            .ok_or("vectorpotential([three parts of the field], [x, y, z])")?;
        let vars = names(&a[1], "vectorpotential")?;
        if vars.len() != 3 {
            return Err("vectorpotential([three parts of the field], [x, y, z])".into());
        }
        let source = add(field
            .iter()
            .zip(&vars)
            .map(|(f, v)| crate::diff::diff(f, v))
            .collect());
        if !is_zero(&simplify(&source)) {
            return Ok(Expr::undefined());
        }
        let t = Expr::sym("_t");
        let scaled: Vec<(String, Expr)> = vars
            .iter()
            .map(|v| (v.clone(), mul(vec![t.clone(), Expr::sym(v)])))
            .collect();
        let along: Vec<Expr> = field.iter().map(|f| subst_all(f, &scaled)).collect();
        let x: Vec<Expr> = vars.iter().map(|v| Expr::sym(v)).collect();
        let cross = [
            sub(
                mul(vec![along[1].clone(), x[2].clone()]),
                mul(vec![along[2].clone(), x[1].clone()]),
            ),
            sub(
                mul(vec![along[2].clone(), x[0].clone()]),
                mul(vec![along[0].clone(), x[2].clone()]),
            ),
            sub(
                mul(vec![along[0].clone(), x[1].clone()]),
                mul(vec![along[1].clone(), x[0].clone()]),
            ),
        ];
        let mut out = Vec::new();
        for c in cross {
            let inside = simplify(&mul(vec![t.clone(), c]));
            let args = [inside, t.clone(), Expr::zero(), Expr::one()];
            let part = self.with_bound(&["_t".to_string()], vec![t.clone()], |s| {
                s.call("integrate", &args)
            })?;
            if part.has_fn("integrate") {
                return Err("vectorpotential: an integral has no closed form".into());
            }
            out.push(simplify(&part));
        }
        Ok(Expr::list(out))
    }

    /// The inverse function: `x` in terms of `y` from `y = f(x)`, written with the variable.
    fn finverse(&mut self, a: &[Expr]) -> Res {
        let var = variable(a.get(1), &a[0], "finverse")?;
        let y = Expr::sym("#y");
        let solved = crate::solve::solve(&sub(a[0].clone(), y.clone()), &var)?;
        let pairs: Vec<(String, Expr)> = solved
            .params
            .iter()
            .map(|p| (p.clone(), Expr::zero()))
            .collect();
        crate::solve::release(&solved.params);
        if solved.roots.is_empty() {
            return Err(format!("finverse: {} has no inverse that was found", a[0]));
        }
        if solved.roots.len() > 1 {
            self.notes.push(format!(
                "the function has {} inverses; this is one of them",
                solved.roots.len()
            ));
        }
        // (of several branches the one that is largest at a positive place)
        let branches: Vec<Expr> = solved
            .roots
            .iter()
            .map(|r| {
                simplify(&subst_sym(
                    &subst_all(&r.value, &pairs),
                    "#y",
                    &Expr::sym(&var),
                ))
            })
            .collect();
        let size = |e: &Expr| {
            numeric::real_value(&subst_sym(e, &var, &Expr::int(2)), 20)
                .map(|f| f.to_f64())
                .unwrap_or(f64::NEG_INFINITY)
        };
        let mut best = 0;
        for k in 1..branches.len() {
            if size(&branches[k]) > size(&branches[best]) {
                best = k;
            }
        }
        Ok(branches[best].clone())
    }

    /// The rules of `rewrite(expr, target)`.
    fn rewrite(&mut self, e: &Expr, target: &str) -> Res {
        let half = |x: &Expr| div(x.clone(), Expr::int(2));
        let f1 = |name: &str, x: &Expr| func(name, vec![x.clone()]);
        let two = Expr::int(2);
        let one = Expr::one;
        let i = Expr::i;
        let sqrt = |x: Expr| pow(x, Expr::half());
        let known = [
            "exp",
            "sincos",
            "sinhcosh",
            "tan",
            "sin",
            "cos",
            "log",
            "heaviside",
            "piecewise",
            "gamma",
            "factorial",
            "erf",
            "sqrt",
        ];
        if !known.contains(&target) {
            return Err(format!(
                "rewrite(expr, target): the targets are {}",
                known.map(|k| format!("\"{k}\"")).join(", ")
            ));
        }
        if target == "exp" {
            return self.call("rewrite_exp", std::slice::from_ref(e));
        }
        let mut step = |x: Expr| -> Expr {
            let (name, args) = match &x {
                Expr::Fn(name, args) => (name.to_string(), args.to_vec()),
                Expr::Pow(b, t) if b.is_sym("e") && target == "sinhcosh" => {
                    return add(vec![f1("cosh", t), f1("sinh", t)]);
                }
                Expr::Pow(b, t) if b.is_sym("e") && target == "sincos" => {
                    // e^(a + i b) = e^a (cos(b) + i sin(b)): b from the terms with an i
                    let terms: Vec<Expr> = match &**t {
                        Expr::Add(terms) => terms.to_vec(),
                        other => vec![other.clone()],
                    };
                    let (mut plain, mut turned) = (Vec::new(), Vec::new());
                    for term in terms {
                        let (c, rest) = crate::canon::coeff_and_rest(&term);
                        let part = |r: &crate::num::Rat| {
                            mul(vec![
                                Expr::num(crate::num::Num::real(r.clone())),
                                rest.clone(),
                            ])
                        };
                        if !c.re.is_zero() {
                            plain.push(part(&c.re));
                        }
                        if !c.im.is_zero() {
                            turned.push(part(&c.im));
                        }
                    }
                    if turned.is_empty() {
                        return x;
                    }
                    let angle = add(turned);
                    let wave = add(vec![f1("cos", &angle), mul(vec![i(), f1("sin", &angle)])]);
                    return if plain.is_empty() {
                        wave
                    } else {
                        mul(vec![f1("exp", &add(plain)), wave])
                    };
                }
                Expr::Pow(b, t)
                    if target == "cos"
                        && b.fn_arg("sin").is_some()
                        && t.as_i64().is_some_and(|k| k > 0 && k % 2 == 0) =>
                {
                    let (u, k) = (b.fn_arg("sin").cloned(), t.as_i64().unwrap_or(2));
                    let u = u.unwrap_or_else(Expr::zero);
                    return pow(
                        sub(one(), pow(f1("cos", &u), two.clone())),
                        Expr::int(k / 2),
                    );
                }
                _ => return x,
            };
            let u = args.first().cloned().unwrap_or_else(Expr::zero);
            let (s, c) = (f1("sin", &u), f1("cos", &u));
            match (target, name.as_str(), args.len()) {
                ("sincos", "tan", 1) => div(s, c),
                ("sincos", "cot", 1) => div(c, s),
                ("sincos", "sec", 1) => div(one(), c),
                ("sincos", "csc", 1) => div(one(), s),
                ("sinhcosh", "tanh", 1) => div(f1("sinh", &u), f1("cosh", &u)),
                ("sinhcosh", "coth", 1) => div(f1("cosh", &u), f1("sinh", &u)),
                ("sinhcosh", "sech", 1) => div(one(), f1("cosh", &u)),
                ("sinhcosh", "csch", 1) => div(one(), f1("sinh", &u)),
                ("tan", "sin" | "cos" | "cot" | "sec" | "csc", 1) => {
                    let t = f1("tan", &half(&u));
                    let t2 = pow(t.clone(), two.clone());
                    let sin = div(mul(vec![two.clone(), t]), add(vec![one(), t2.clone()]));
                    let cos = div(sub(one(), t2.clone()), add(vec![one(), t2]));
                    match name.as_str() {
                        "sin" => sin,
                        "cos" => cos,
                        "cot" => div(one(), f1("tan", &u)),
                        "sec" => div(one(), cos),
                        _ => div(one(), sin),
                    }
                }
                ("sin", "cos" | "tan" | "cot" | "sec" | "csc", 1) => {
                    let cos = sub(
                        one(),
                        mul(vec![two.clone(), pow(f1("sin", &half(&u)), two.clone())]),
                    );
                    match name.as_str() {
                        "cos" => cos,
                        "tan" => div(s, cos),
                        "cot" => div(cos, s),
                        "sec" => div(one(), cos),
                        _ => div(one(), s),
                    }
                }
                ("cos", "sin" | "tan" | "cot" | "sec" | "csc", 1) => {
                    let sin = f1("cos", &sub(u.clone(), div(Expr::pi(), two.clone())));
                    match name.as_str() {
                        "sin" => sin,
                        "tan" => div(sin, c),
                        "cot" => div(c, sin),
                        "sec" => div(one(), c),
                        _ => div(one(), sin),
                    }
                }
                ("log", "asinh", 1) => f1(
                    "ln",
                    &add(vec![
                        u.clone(),
                        sqrt(add(vec![pow(u.clone(), two.clone()), one()])),
                    ]),
                ),
                ("log", "acosh", 1) => f1(
                    "ln",
                    &add(vec![
                        u.clone(),
                        mul(vec![
                            sqrt(sub(u.clone(), one())),
                            sqrt(add(vec![u.clone(), one()])),
                        ]),
                    ]),
                ),
                ("log", "atanh", 1) => div(
                    sub(
                        f1("ln", &add(vec![one(), u.clone()])),
                        f1("ln", &sub(one(), u.clone())),
                    ),
                    two.clone(),
                ),
                ("log", "acoth", 1) => div(
                    sub(
                        f1("ln", &add(vec![one(), div(one(), u.clone())])),
                        f1("ln", &sub(one(), div(one(), u.clone()))),
                    ),
                    two.clone(),
                ),
                ("log", "asin", 1) => mul(vec![
                    neg(i()),
                    f1(
                        "ln",
                        &add(vec![
                            mul(vec![i(), u.clone()]),
                            sqrt(sub(one(), pow(u.clone(), two.clone()))),
                        ]),
                    ),
                ]),
                ("log", "acos", 1) => mul(vec![
                    neg(i()),
                    f1(
                        "ln",
                        &add(vec![
                            u.clone(),
                            mul(vec![i(), sqrt(sub(one(), pow(u.clone(), two.clone())))]),
                        ]),
                    ),
                ]),
                ("log", "atan", 1) => mul(vec![
                    div(i(), two.clone()),
                    sub(
                        f1("ln", &sub(one(), mul(vec![i(), u.clone()]))),
                        f1("ln", &add(vec![one(), mul(vec![i(), u.clone()])])),
                    ),
                ]),
                ("heaviside", "sign", 1) => sub(mul(vec![two.clone(), f1("heaviside", &u)]), one()),
                ("heaviside", "abs", 1) => mul(vec![
                    u.clone(),
                    sub(mul(vec![two.clone(), f1("heaviside", &u)]), one()),
                ]),
                ("piecewise", "abs", 1) => func(
                    "piecewise",
                    vec![
                        Expr::rel(Rel::Ge, u.clone(), Expr::zero()),
                        u.clone(),
                        neg(u.clone()),
                    ],
                ),
                ("piecewise", "sign", 1) => func(
                    "piecewise",
                    vec![
                        Expr::rel(Rel::Gt, u.clone(), Expr::zero()),
                        one(),
                        Expr::rel(Rel::Lt, u.clone(), Expr::zero()),
                        Expr::minus_one(),
                        Expr::zero(),
                    ],
                ),
                ("piecewise", "heaviside", 1) => func(
                    "piecewise",
                    vec![
                        Expr::rel(Rel::Gt, u.clone(), Expr::zero()),
                        one(),
                        Expr::rel(Rel::Lt, u.clone(), Expr::zero()),
                        Expr::zero(),
                        Expr::half(),
                    ],
                ),
                ("piecewise", "max" | "min", 2) => {
                    let (p, q) = (args[0].clone(), args[1].clone());
                    let op = if name == "max" { Rel::Ge } else { Rel::Le };
                    func("piecewise", vec![Expr::rel(op, p.clone(), q.clone()), p, q])
                }
                ("gamma", "factorial", 1) => f1("gamma", &add(vec![u.clone(), one()])),
                ("gamma", "binomial", 2) => {
                    let (p, q) = (args[0].clone(), args[1].clone());
                    div(
                        f1("gamma", &add(vec![p.clone(), one()])),
                        mul(vec![
                            f1("gamma", &add(vec![q.clone(), one()])),
                            f1("gamma", &add(vec![sub(p, q), one()])),
                        ]),
                    )
                }
                ("gamma", "beta", 2) => {
                    let (p, q) = (args[0].clone(), args[1].clone());
                    div(
                        mul(vec![f1("gamma", &p), f1("gamma", &q)]),
                        f1("gamma", &add(vec![p, q])),
                    )
                }
                ("factorial", "gamma", 1) => f1("factorial", &sub(u.clone(), one())),
                ("erf", "erfc", 1) => sub(one(), f1("erf", &u)),
                ("erf", "erfi", 1) => mul(vec![neg(i()), f1("erf", &mul(vec![i(), u.clone()]))]),
                ("sqrt", "abs", 1) if crate::assume::is_real(&u) => {
                    sqrt(pow(u.clone(), two.clone()))
                }
                ("sqrt", "abs", 1) => sqrt(mul(vec![u.clone(), f1("conj", &u)])),
                _ => x,
            }
        };
        Ok(crate::canon::map_bottom_up(e, &mut step))
    }

    /// Logarithms of positive things put together: `a*ln(u) + b*ln(v)` is `ln(u^a * v^b)`.
    fn combine_logs(e: &Expr) -> Expr {
        let mut step = |x: Expr| -> Expr {
            let Expr::Add(terms) = &x else {
                return x;
            };
            let (mut logs, mut rest) = (Vec::new(), Vec::new());
            for t in terms.iter() {
                let (c, r) = crate::canon::coeff_and_rest(t);
                match r.fn_arg("ln") {
                    Some(u)
                        if Expr::num(c.clone()).as_rat().is_some()
                            && crate::assume::is_positive(u) =>
                    {
                        logs.push(pow(u.clone(), Expr::num(c)));
                    }
                    _ => rest.push(t.clone()),
                }
            }
            if logs.len() < 2 {
                return x;
            }
            rest.push(Expr::call("ln", vec![simplify(&mul(logs))]));
            if rest.len() == 1 {
                rest.remove(0)
            } else {
                Expr::Add(rest.into())
            }
        };
        crate::canon::map_bottom_up(e, &mut step)
    }

    /// The commands of this module; `None` for other names.
    pub(crate) fn matlab(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        let n = a.len();
        Some(match (name, n) {
            ("pdf" | "cdf" | "quantile", 2)
                if a[1].as_list().is_some() && a[0].as_list().is_none() =>
            {
                self.at_each(name, &a[0], &a[1])
            }
            ("fzero", 2) => self.fzero(a),
            ("fminbnd", 3) => self.fminbnd(a),
            ("fminsearch", 2) => self.fminsearch(a),
            ("fmincon", 2..=9) => self.fmincon(a),
            // the ways of MATLAB to a solution step by step: here the solution itself
            ("pcg" | "gmres" | "bicg" | "bicgstab" | "cgs" | "minres" | "qmr" | "symmlq" | "tfqmr", 2..) => {
                self.call("linsolve", &a[..2])
            }
            ("lsqr", 2..) => self.call("mldivide", &a[..2]),
            ("lsqcurvefit", 4) => self.curve_fit(a),
            ("lsqnonlin", 2) => self.nonlinear_squares(a),
            ("reshape", 3) => self.call(
                "reshape",
                &[a[0].clone(), Expr::list(vec![a[1].clone(), a[2].clone()])],
            ),
            ("mellin", 3) => {
                // the integral of x^(s - 1)*f(x) from 0 on
                let (x, s) = (a[1].clone(), a[2].clone());
                let kernel = mul(vec![pow(x.clone(), sub(s, Expr::one())), a[0].clone()]);
                self.call("integrate", &[kernel, x, Expr::zero(), Expr::inf()])
            }
            ("anova1", 1) => (|| {
                let usage = "anova1([[values of a group], [values of another], ...]): the probability that groups with one mean differ as much";
                let groups: Vec<&[Expr]> = a[0]
                    .as_list()
                    .filter(|g| g.len() >= 2)
                    .ok_or(usage)?
                    .iter()
                    .map(|g| g.as_list().filter(|v| !v.is_empty()).ok_or(usage.to_string()))
                    .collect::<Result<_, _>>()?;
                let all: Vec<Expr> = groups.concat();
                let (count, kinds) = (all.len() as i64, groups.len() as i64);
                if count <= kinds {
                    return Err(usage.to_string());
                }
                let grand = mean(&all);
                let square = |x: Expr| pow(x, Expr::int(2));
                let between = add(groups
                    .iter()
                    .map(|g| {
                        mul(vec![
                            Expr::int(g.len() as i64),
                            square(sub(mean(g), grand.clone())),
                        ])
                    })
                    .collect());
                let within = add(groups
                    .iter()
                    .flat_map(|g| {
                        let m = mean(g);
                        g.iter().map(move |x| square(sub(x.clone(), m.clone())))
                    })
                    .collect());
                if is_zero(&simplify(&within)) {
                    return Ok(Expr::zero());
                }
                let stat = div(
                    div(between, Expr::int(kinds - 1)),
                    div(within, Expr::int(count - kinds)),
                );
                let stat = numeric::n(&simplify(&stat), numeric::digits());
                let dist = Expr::call("fdist", vec![Expr::int(kinds - 1), Expr::int(count - kinds)]);
                let below = self.call("cdf", &[dist, stat])?;
                Ok(numeric::n(&sub(Expr::one(), below), numeric::digits()))
            })(),
            ("integrate", 3) if self.function_like(&a[0]) => self.integral_of("integral", a),
            ("quad", 3) | ("integral2", 5) | ("integral3", 7) => self.integral_of(name, a),
            ("ode45", 3) => self.ode(name, a),
            ("feval", 1..) => self.apply(&a[0], &a[1..]),
            ("bsxfun", 3) => {
                let f = a[0].clone();
                let failed = std::cell::RefCell::new(None);
                let this = std::cell::RefCell::new(&mut *self);
                let out = matrix::zip(&a[1], &a[2], &|x, y| {
                    this.borrow_mut()
                        .apply(&f, &[x.clone(), y.clone()])
                        .unwrap_or_else(|e| {
                            failed.borrow_mut().get_or_insert(e);
                            Expr::undefined()
                        })
                });
                match failed.into_inner() {
                    Some(e) => Err(e),
                    None => out,
                }
            }
            ("funm", 2) => (|| {
                let w = Expr::sym("#w");
                let body = self.apply(&a[1], std::slice::from_ref(&w))?;
                self.call("matrixfunction", &[body, w, a[0].clone()])
            })(),
            ("lsqminnorm", 2) => (|| {
                let p = self.call("pinv", std::slice::from_ref(&a[0]))?;
                matrix::mul(vec![p, a[1].clone()])
            })(),
            ("fsolve", 2) => (|| {
                let start: Vec<Expr> = match a[1].as_list() {
                    Some(items) => items.to_vec(),
                    None => vec![a[1].clone()],
                };
                let (params, body) = self.function_of(&a[0], start.len(), "fsolve")?;
                let vars: Vec<Expr> = params.iter().map(|p| Expr::sym(p)).collect();
                let found = if start.len() == 1 && body.as_list().is_none() {
                    self.call("nsolve", &[body, vars[0].clone(), start[0].clone()])?
                } else {
                    self.call("nsolve", &[body, Expr::list(vars), Expr::list(start.clone())])?
                };
                // (the answers are x -> value)
                Ok(match found.as_list() {
                    Some(items) => Expr::list(
                        items
                            .iter()
                            .map(|r| match r.fn_args("rule") {
                                Some([_, v]) => v.clone(),
                                _ => r.clone(),
                            })
                            .collect(),
                    ),
                    None => found,
                })
            })(),
            ("str2num", 1) => (|| {
                let text = crate::radix::text_of(&a[0]).ok_or("str2num needs a text in quotes")?;
                let funcs: Vec<String> = self.funcs.keys().cloned().collect();
                let parsed = crate::parser::parse_with(text, &|f| funcs.iter().any(|g| g == f))?;
                let mut last = Expr::list(Vec::new());
                for stmt in &parsed.stmts {
                    last = self.eval(stmt)?;
                }
                Ok(if parsed.decimal {
                    numeric::n(&last, numeric::digits())
                } else {
                    last
                })
            })(),
            ("str2double", 1) if a[0].as_list().is_some() => (|| {
                let items = a[0].as_list().unwrap_or(&[]);
                Ok(Expr::list(
                    items
                        .iter()
                        .map(|x| self.call("str2double", std::slice::from_ref(x)))
                        .collect::<Result<_, _>>()?,
                ))
            })(),
            ("str2double", 1) => (|| {
                let Some(text) = crate::radix::text_of(&a[0]) else {
                    return Ok(Expr::undefined());
                };
                let plain = text.trim().chars().all(|c| {
                    c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | 'e' | 'E' | 'i' | 'j' | ' ')
                });
                let word = text.trim().trim_start_matches(['+', '-']).to_ascii_lowercase();
                if !(plain && text.chars().any(|c| c.is_ascii_digit()))
                    && !matches!(word.as_str(), "inf" | "nan")
                {
                    return Ok(Expr::undefined());
                }
                match crate::parser::parse(text) {
                    Ok(parsed) if parsed.stmts.len() == 1 => {
                        let v = self.eval(&parsed.stmts[0]).unwrap_or_else(|_| Expr::undefined());
                        Ok(if parsed.decimal {
                            numeric::n(&v, numeric::digits())
                        } else {
                            v
                        })
                    }
                    _ => Ok(Expr::undefined()),
                }
            })(),
            ("random", 1 | 2) if crate::dist::is_distribution(&a[0]) => (|| {
                let draw = |s: &mut Session| {
                    s.call("quantile", &[a[0].clone(), crate::random::uniform()])
                };
                match a.get(1) {
                    None => draw(self),
                    Some(k) => {
                        let count = whole(k, "the number of values", 0, 1_000_000)?;
                        Ok(Expr::list(
                            (0..count).map(|_| draw(self)).collect::<Result<_, _>>()?,
                        ))
                    }
                }
            })(),
            ("limit", 4) if crate::radix::text_of(&a[3]).is_some() => (|| {
                let side = match word(&a[3]).as_deref() {
                    Some("left") => "left",
                    Some("right") => "right",
                    _ => return Err("limit(f, x, a, \"left\") or limit(f, x, a, \"right\")".to_string()),
                };
                self.call("limit", &[a[0].clone(), a[1].clone(), a[2].clone(), Expr::sym(side)])
            })(),
            ("subst", 3) if a[1].as_list().is_some() => (|| {
                let (olds, news) = (a[1].as_list().unwrap_or_default(), a[2].as_list().unwrap_or_default());
                if olds.len() != news.len() {
                    return Err("subst(expr, [x, y], [values]): a value for every name".to_string());
                }
                // (all at once: through names that nothing else has)
                let mut out = a[0].clone();
                for (k, old) in olds.iter().enumerate() {
                    out = subst(&out, old, &Expr::sym(&format!("#s{k}")));
                }
                for (k, new) in news.iter().enumerate() {
                    out = subst_sym(&out, &format!("#s{k}"), new);
                }
                self.eval(&out).map(tidy)
            })(),
            ("finverse", 1 | 2) => self.finverse(a),
            ("htrans" | "ihtrans", 3) => (|| {
                let usage = format!("{name}(f, x, t): the transform of f(x) as a function of t");
                let (x, t) = (
                    a[1].sym_name().ok_or(usage.clone())?.to_string(),
                    a[2].clone(),
                );
                let f = crate::expand::expand(&a[0]);
                let turned = hilbert(&f, &x, &t).ok_or_else(|| {
                    format!(
                        "{name}: the transform is known of sums of sin(a*x + b), cos(a*x + b), e^(i*(a*x + b)), 1/(x^2 + c), x/(x^2 + c), sin(x)/x, 1/x, diracdelta(x - c) and of numbers, not of {}",
                        a[0]
                    )
                })?;
                // (the transform back is the transform with the other sign)
                Ok(simplify(&if name == "htrans" { turned } else { neg(turned) }))
            })(),
            ("compose", 2..=5) => (|| {
                let (f, g) = (&a[0], &a[1]);
                let (x, y, z) = match n {
                    2 => {
                        let y = variable(None, g, "compose")?;
                        (variable(None, f, "compose")?, y.clone(), Expr::sym(&y))
                    }
                    3 => (variable(None, f, "compose")?, variable(None, g, "compose")?, a[2].clone()),
                    4 => (variable(Some(&a[2]), f, "compose")?, variable(None, g, "compose")?, a[3].clone()),
                    _ => (variable(Some(&a[2]), f, "compose")?, variable(Some(&a[3]), g, "compose")?, a[4].clone()),
                };
                let inner = subst_sym(g, &y, &z);
                Ok(simplify(&subst_sym(f, &x, &inner)))
            })(),
            ("symvar", 1 | 2) => (|| {
                let mut syms: Vec<String> = a[0].free_syms().iter().map(|s| s.to_string()).collect();
                if let Some(k) = a.get(1) {
                    let count = whole(k, "the number of variables", 0, 1_000_000)? as usize;
                    let far = |s: &String| {
                        let c = s.chars().next().unwrap_or('x') as i64;
                        let d = (c - 'x' as i64).abs();
                        (if s.len() == 1 { 0 } else { 1 }, d, -c)
                    };
                    syms.sort_by_key(far);
                    syms.truncate(count);
                }
                Ok(Expr::list(syms.iter().map(|s| Expr::sym(s)).collect()))
            })(),
            ("has", 2) => Ok(match (&a[0], &a[1]) {
                (Expr::List(items), sub) => Expr::list(
                    items
                        .iter()
                        .map(|x| Expr::Bool(Self::has_any(x, sub)))
                        .collect(),
                ),
                (e, sub) => Expr::Bool(Self::has_any(e, sub)),
            }),
            ("children", 1) => Ok(Expr::list(match &a[0] {
                Expr::Rel(_, l, r) => vec![(**l).clone(), (**r).clone()],
                Expr::Pow(b, x) => vec![(**b).clone(), (**x).clone()],
                other if other.children().is_empty() => vec![other.clone()],
                other => other.children().to_vec(),
            })),
            ("lhs" | "rhs", 1) => match &a[0] {
                Expr::Rel(_, l, r) => Ok(if name == "lhs" { (**l).clone() } else { (**r).clone() }),
                other => Err(format!("{name} needs an equation or an inequality, {other} is none")),
            },
            ("isalways", 1) => self.always(&a[0]).map(Expr::Bool),
            ("combine", 1) => Ok(simplify(&a[0])),
            ("combine", 2) => match word(&a[1]).as_deref() {
                Some("sincos" | "sinhcosh") => self.call("trigreduce", std::slice::from_ref(&a[0])),
                Some("log") => Ok(Self::combine_logs(&a[0])),
                Some("exp" | "power") => Ok(simplify(&a[0])),
                _ => Err("combine(expr, target): the targets are \"sincos\", \"sinhcosh\", \"log\", \"exp\"".into()),
            },
            ("rewrite", 2) => match word(&a[1]) {
                Some(target) => self.rewrite(&a[0], &target),
                None => Err("rewrite(expr, \"target\")".into()),
            },
            ("potential", 2 | 3) => self.potential(a),
            ("vectorpotential", 2) => self.vector_potential(a),
            ("equationstomatrix", 1 | 2) => (|| {
                let eqs: Vec<Expr> = match a[0].as_list() {
                    Some(items) => items.iter().map(as_equation).collect(),
                    None => vec![as_equation(&a[0])],
                };
                let vars: Vec<String> = match a.get(1) {
                    Some(v) => names(v, "equationstomatrix")?,
                    None => {
                        let mut all = std::collections::BTreeSet::new();
                        eqs.iter().for_each(|e| e.collect_syms(&mut all));
                        all.iter().map(|s| s.to_string()).collect()
                    }
                };
                let zeros: Vec<(String, Expr)> = vars.iter().map(|v| (v.clone(), Expr::zero())).collect();
                let (mut rows, mut right): (Rows, Vec<Expr>) = (Vec::new(), Vec::new());
                for e in &eqs {
                    let row: Vec<Expr> = vars.iter().map(|v| simplify(&crate::diff::diff(e, v))).collect();
                    if row.iter().any(|c| vars.iter().any(|v| c.has_sym(v))) {
                        return Err(format!("equationstomatrix: {e} = 0 is not linear in the variables"));
                    }
                    right.push(simplify(&neg(subst_all(e, &zeros))));
                    rows.push(row);
                }
                Ok(Expr::list(vec![matrix::from_rows(rows), Expr::list(right)]))
            })(),
            ("eliminate", 2) => (|| {
                let eqs = a[0].as_list().ok_or("eliminate([equations], [variables])")?;
                let gone = names(&a[1], "eliminate")?;
                let polys: Vec<Expr> = eqs.iter().map(as_equation).collect();
                let mut all = std::collections::BTreeSet::new();
                polys.iter().for_each(|e| e.collect_syms(&mut all));
                let mut order: Vec<Expr> = gone.iter().map(|g| Expr::sym(g)).collect();
                order.extend(
                    all.iter()
                        .filter(|s| !gone.iter().any(|g| **g == ***s))
                        .map(|s| Expr::sym(s)),
                );
                let basis = crate::polys::call("gbasis", &[Expr::list(polys), Expr::list(order)])
                    .ok_or("eliminate: no basis")??;
                Ok(Expr::list(
                    basis
                        .as_list()
                        .unwrap_or_default()
                        .iter()
                        .filter(|p| gone.iter().all(|g| !p.has_sym(g)))
                        .cloned()
                        .collect(),
                ))
            })(),
            ("isolate", 2) => (|| {
                let eq = as_equation(&a[0]);
                let (target, var) = match &a[1] {
                    Expr::Sym(s) => (eq, s.to_string()),
                    other => (subst(&eq, other, &Expr::sym("#iso")), "#iso".to_string()),
                };
                let solved = crate::solve::solve(&target, &var)?;
                let pairs: Vec<(String, Expr)> = solved.params.iter().map(|p| (p.clone(), Expr::zero())).collect();
                crate::solve::release(&solved.params);
                let first = solved.roots.first().ok_or("isolate: no solution found")?;
                Ok(Expr::call(
                    "hold",
                    vec![Expr::eq(a[1].clone(), simplify(&subst_all(&first.value, &pairs)))],
                ))
            })(),
            ("assumptions", 0) => Ok(Expr::list(
                crate::assume::all()
                    .into_iter()
                    .filter(|(_, facts)| !facts.describe().is_empty())
                    .map(|(sym, facts)| crate::texts::text(&format!("{sym} is {}", facts.describe().join(", "))))
                    .collect(),
            )),
            ("ttest", 1 | 2) => (|| {
                let x = data(&a[0], name)?;
                let values: Vec<Expr> = match a.get(1) {
                    Some(Expr::List(y)) if y.len() == x.len() => {
                        x.iter().zip(y.iter()).map(|(p, q)| sub(p.clone(), q.clone())).collect()
                    }
                    Some(Expr::List(_)) => return Err("ttest(x, y): two lists of one length".to_string()),
                    Some(m) => x.iter().map(|p| sub(p.clone(), m.clone())).collect(),
                    None => x.to_vec(),
                };
                let count = values.len() as i64;
                let t = div(mean(&values), pow(div(variance(&values)?, Expr::int(count)), Expr::half()));
                self.two_sided("studenttdist", vec![Expr::int(count - 1)], t)
            })(),
            ("ttest2", 2) => (|| {
                let (x, y) = (data(&a[0], name)?, data(&a[1], name)?);
                let (p, q) = (x.len() as i64, y.len() as i64);
                let pooled = div(
                    add(vec![
                        mul(vec![Expr::int(p - 1), variance(x)?]),
                        mul(vec![Expr::int(q - 1), variance(y)?]),
                    ]),
                    Expr::int(p + q - 2),
                );
                let t = div(
                    sub(mean(x), mean(y)),
                    pow(mul(vec![pooled, Expr::rat(p + q, p * q)]), Expr::half()),
                );
                self.two_sided("studenttdist", vec![Expr::int(p + q - 2)], t)
            })(),
            ("ztest", 3) => (|| {
                let x = data(&a[0], name)?;
                let z = div(
                    sub(mean(x), a[1].clone()),
                    div(a[2].clone(), pow(Expr::int(x.len() as i64), Expr::half())),
                );
                self.two_sided("normaldist", vec![Expr::zero(), Expr::one()], z)
            })(),
            ("vartest", 2) => (|| {
                let x = data(&a[0], name)?;
                let df = x.len() as i64 - 1;
                let stat = div(mul(vec![Expr::int(df), variance(x)?]), a[1].clone());
                self.either_tail("chisquaredist", vec![Expr::int(df)], stat)
            })(),
            ("vartest2", 2) => (|| {
                let (x, y) = (data(&a[0], name)?, data(&a[1], name)?);
                let stat = div(variance(x)?, variance(y)?);
                self.either_tail(
                    "fdist",
                    vec![Expr::int(x.len() as i64 - 1), Expr::int(y.len() as i64 - 1)],
                    stat,
                )
            })(),
            ("splitapply", 3) => (|| {
                let (values, groups) = (
                    a[1].as_list().ok_or("splitapply(f, values, groups)")?,
                    a[2].as_list().ok_or("splitapply(f, values, groups)")?,
                );
                if values.len() != groups.len() {
                    return Err("splitapply: a group for every value".to_string());
                }
                let mut keys: Vec<Expr> = Vec::new();
                for g in groups {
                    if !keys.contains(g) {
                        keys.push(g.clone());
                    }
                }
                let keys = crate::lists::sorted(&keys);
                let mut out = Vec::new();
                for k in &keys {
                    let part: Vec<Expr> = values
                        .iter()
                        .zip(groups)
                        .filter(|(_, g)| *g == k)
                        .map(|(v, _)| v.clone())
                        .collect();
                    out.push(self.apply(&a[0], &[Expr::list(part)])?);
                }
                Ok(Expr::list(out))
            })(),
            ("assert", 1 | 2) => {
                match crate::arrays::call("all", &[a[0].clone(), crate::texts::text("all")]) {
                    Some(Ok(Expr::Bool(true))) => Ok(Expr::Bool(true)),
                    _ => Err(Self::failed(a)),
                }
            }
            ("readmatrix", 1) => self.read_matrix(&a[0]),
            ("imread", 1 | 2) => read_picture(&a[0], a.get(1)),
            ("imframes", 1) => match crate::radix::text_of(&a[0]) {
                Some(path) => crate::imgfile::frames(path)
                    .map(|count| Expr::int(count as i64))
                    .map_err(|e| format!("imframes: {e}")),
                None => Err("imframes(\"picture.jxl\"): how many frames the picture has".into()),
            },
            ("tic", 0) => {
                STARTED.with(|s| s.set(Some(std::time::Instant::now())));
                Ok(Expr::Bool(true))
            }
            ("toc", 0) => match STARTED.with(std::cell::Cell::get) {
                Some(at) => {
                    let prec = bits();
                    Ok(value(&Float::with_val(prec, at.elapsed().as_secs_f64())))
                }
                None => Err("toc: tic() starts the clock".into()),
            },
            ("who", 0) => {
                let mut all: Vec<String> = self.vars.keys().cloned().collect();
                all.extend(self.funcs.keys().cloned());
                all.sort();
                Ok(Expr::list(all.iter().map(|s| crate::texts::text(s)).collect()))
            }
            ("error", 1..) => Err(match crate::radix::text_of(&a[0]) {
                Some(format) => crate::texts::sprintf(format, &a[1..]).unwrap_or_else(|_| format.to_string()),
                None => format!("{}", a[0]),
            }),
            _ => return None,
        })
    }

    /// `clear()` forgets every name that was given a value or defined as a function,
    /// `clear(x, f)` those names.
    pub(crate) fn clear(&mut self, raw: &[Expr]) -> Res {
        if raw.is_empty() {
            self.vars.clear();
            self.funcs.clear();
            return Ok(Expr::Bool(true));
        }
        for item in raw {
            let name = item
                .sym_name()
                .ok_or_else(|| format!("clear(name, ...): {item} is not a name"))?;
            self.vars.remove(name);
            self.funcs.remove(name);
        }
        Ok(Expr::Bool(true))
    }

    /// The lines of `disp(x)`, `fprintf(format, values)` and `writematrix(A, file)`.
    pub(crate) fn shown(&mut self, name: &str, a: &[Expr]) -> Result<Vec<String>, String> {
        let lines = |text: &str| -> Vec<String> {
            let text = text.strip_suffix('\n').unwrap_or(text);
            text.split('\n').map(str::to_string).collect()
        };
        let cell = |x: &Expr| match crate::radix::text_of(x) {
            Some(t) => t.to_string(),
            None => format!("{x}"),
        };
        match (name, a) {
            ("fprintf", [first, rest @ ..]) if crate::radix::text_of(first).is_some() => {
                let format = crate::radix::text_of(first).unwrap_or_default();
                Ok(lines(&crate::texts::sprintf(format, rest)?))
            }
            ("pretty", [x]) => Ok(crate::codegen::pretty(x)),
            ("disp", [x]) if crate::tables::is_table(x) => {
                Ok(crate::tables::lines(x).unwrap_or_default())
            }
            ("writetable", [t, file]) => {
                let path = crate::radix::text_of(file).ok_or("writetable(table, \"file.csv\")")?;
                let (text, rows, columns) = crate::tables::file_text(t)
                    .ok_or("writetable(table, \"file.csv\"): the first argument is a table")?;
                crate::sandbox::allowed(path)
                    .and_then(|()| crate::sandbox::spend(text.len()))
                    .map_err(|e| format!("writetable: {e}"))?;
                std::fs::write(path, &text).map_err(|e| format!("writetable: {path}: {e}"))?;
                Ok(vec![format!(
                    "saved {path}: {rows} row(s) of {columns} variable(s), {} bytes",
                    text.len()
                )])
            }
            ("disp" | "fprintf", [x]) => Ok(match (crate::radix::text_of(x), matrix::rows(x)) {
                (Some(t), _) => lines(t),
                (None, Some(m)) => {
                    let cells: Vec<Vec<String>> =
                        m.iter().map(|r| r.iter().map(cell).collect()).collect();
                    let widths: Vec<usize> = (0..m[0].len())
                        .map(|j| {
                            cells
                                .iter()
                                .map(|r| r[j].chars().count())
                                .max()
                                .unwrap_or(0)
                        })
                        .collect();
                    cells
                        .iter()
                        .map(|r| {
                            r.iter()
                                .zip(&widths)
                                .map(|(c, w)| format!("{}{c}", " ".repeat(w - c.chars().count())))
                                .collect::<Vec<_>>()
                                .join("  ")
                        })
                        .collect()
                }
                (None, None) => match x.as_list() {
                    Some(items) if !items.is_empty() => {
                        vec![items.iter().map(cell).collect::<Vec<_>>().join("  ")]
                    }
                    _ => vec![cell(x)],
                },
            }),
            ("imwrite", [data, file, rest @ ..]) if rest.len() <= 1 => {
                write_picture(data, file, rest.first())
            }
            ("writematrix", [m, file]) => {
                let path =
                    crate::radix::text_of(file).ok_or("writematrix(matrix, \"file.csv\")")?;
                let (rows, _) = crate::arrays::grid(m);
                let mut text = String::new();
                for row in &rows {
                    text.push_str(&row.iter().map(cell).collect::<Vec<_>>().join(","));
                    text.push('\n');
                }
                crate::sandbox::allowed(path)
                    .and_then(|()| crate::sandbox::spend(text.len()))
                    .map_err(|e| format!("writematrix: {e}"))?;
                std::fs::write(path, &text).map_err(|e| format!("writematrix: {path}: {e}"))?;
                Ok(vec![format!(
                    "saved {path}: {} row(s) of {} number(s), {} bytes",
                    rows.len(),
                    rows[0].len(),
                    text.len()
                )])
            }
            _ => Err(format!(
                "{name}: disp(x), fprintf(format, values), writematrix(matrix, \"file.csv\") and imwrite(matrix, \"picture.png\")"
            )),
        }
    }

    /// The numbers of a file: lines of numbers with commas, semicolons, tabs or blanks between.
    fn read_matrix(&mut self, file: &Expr) -> Res {
        let path = crate::radix::text_of(file).ok_or("readmatrix(\"file.csv\")")?;
        crate::sandbox::allowed(path).map_err(|e| format!("readmatrix: {e}"))?;
        let text = std::fs::read_to_string(path).map_err(|e| format!("readmatrix: {path}: {e}"))?;
        let mut rows: Rows = Vec::new();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let cells: Vec<&str> = if line.contains([',', ';', '\t']) {
                line.split([',', ';', '\t']).collect()
            } else {
                line.split_whitespace().collect()
            };
            let mut row = Vec::new();
            for c in cells {
                let c = c.trim();
                row.push(if c.is_empty() {
                    Expr::undefined()
                } else {
                    let parsed = crate::parser::parse(c)
                        .map_err(|e| format!("readmatrix: {path}: \"{c}\": {e}"))?;
                    let stmt = parsed
                        .stmts
                        .first()
                        .ok_or_else(|| format!("readmatrix: {path}: an empty entry"))?;
                    let v = self.eval(stmt)?;
                    if parsed.decimal {
                        numeric::n(&v, numeric::digits())
                    } else {
                        v
                    }
                });
            }
            rows.push(row);
        }
        if rows.is_empty() {
            return Ok(Expr::list(Vec::new()));
        }
        if rows.iter().any(|r| r.len() != rows[0].len()) {
            return Err(format!(
                "readmatrix: {path}: the lines have different numbers of entries"
            ));
        }
        Ok(crate::arrays::ungrid(rows, false))
    }

    fn failed(a: &[Expr]) -> String {
        match a.get(1).and_then(crate::radix::text_of) {
            Some(text) => text.to_string(),
            None => format!("assert: {} does not hold", a[0]),
        }
    }

    fn has_any(e: &Expr, sub: &Expr) -> bool {
        match sub {
            Expr::List(items) => items.iter().any(|s| e.has(s)),
            one => e.has(one),
        }
    }

    /// `[h, p]` of a statistic whose distribution is symmetric about zero.
    fn two_sided(&mut self, dist: &str, params: Vec<Expr>, stat: Expr) -> Res {
        let stat = numeric::n(&simplify(&stat), numeric::digits());
        let below = self.call("cdf", &[Expr::call(dist, params), func("abs", vec![stat])])?;
        Ok(verdict(mul(vec![Expr::int(2), sub(Expr::one(), below)])))
    }

    /// `[h, p]` of a statistic whose distribution has two unlike tails.
    fn either_tail(&mut self, dist: &str, params: Vec<Expr>, stat: Expr) -> Res {
        let stat = numeric::n(&simplify(&stat), numeric::digits());
        let below = numeric::n(
            &self.call("cdf", &[Expr::call(dist, params), stat])?,
            numeric::digits(),
        );
        let above = tidy(sub(Expr::one(), below.clone()));
        let tail = if order(&below, &above).is_ok_and(|o| o == Ordering::Less) {
            below
        } else {
            above
        };
        Ok(verdict(mul(vec![Expr::int(2), tail])))
    }
}
