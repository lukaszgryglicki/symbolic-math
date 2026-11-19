//! Interpolation, least squares, Newton's method for systems, critical points.

use crate::canon::{add, div, mul, neg, powi, sub, subst};
use crate::diff::diff;
use crate::expand::expand;
use crate::expr::Expr;
use crate::matrix;
use crate::numeric::{self, Env};
use crate::simplify::simplify;
use rug::{Complex, Float};
use std::rc::Rc;

type Res = Result<Expr, String>;

/// The abscissas and ordinates of data: `[[x1, y1], ...]`, a list of values (at 1, 2, ...) or
/// two lists.
pub fn points(a: &[Expr]) -> Result<(Vec<Expr>, Vec<Expr>), String> {
    let usage = "the data must be [[x1, y1], [x2, y2], ...], a list of values, or two lists";
    match a {
        [Expr::List(xs), Expr::List(ys)]
            if xs.len() == ys.len() && !xs.iter().any(|x| matches!(x, Expr::List(_))) =>
        {
            Ok((xs.to_vec(), ys.to_vec()))
        }
        [Expr::List(items)] if !items.is_empty() => {
            if items
                .iter()
                .all(|p| p.as_list().is_some_and(|l| l.len() == 2))
            {
                let pair = |k: usize| -> Vec<Expr> {
                    items
                        .iter()
                        .map(|p| p.as_list().expect("checked")[k].clone())
                        .collect()
                };
                Ok((pair(0), pair(1)))
            } else if !items.iter().any(|p| matches!(p, Expr::List(_))) {
                Ok((
                    (1..=items.len() as i64).map(Expr::int).collect(),
                    items.to_vec(),
                ))
            } else {
                Err(usage.into())
            }
        }
        _ => Err(usage.into()),
    }
}

/// The polynomial of least degree through the points (Newton's divided differences).
pub fn interpolate(xs: &[Expr], ys: &[Expr], var: &Expr) -> Res {
    let n = xs.len();
    for i in 0..n {
        for j in 0..i {
            if crate::simplify::is_zero(&sub(xs[i].clone(), xs[j].clone())) {
                return Err(format!("two points have the same abscissa {}", xs[i]));
            }
        }
    }
    let mut table: Vec<Expr> = ys.to_vec();
    let mut coefficients = vec![table[0].clone()];
    for level in 1..n {
        for i in 0..n - level {
            table[i] = simplify(&div(
                sub(table[i + 1].clone(), table[i].clone()),
                sub(xs[i + level].clone(), xs[i].clone()),
            ));
        }
        coefficients.push(table[0].clone());
    }
    let mut value = Expr::zero();
    for k in (0..n).rev() {
        value = add(vec![
            coefficients[k].clone(),
            mul(vec![sub(var.clone(), xs[k].clone()), value]),
        ]);
    }
    Ok(simplify(&expand(&value)))
}

/// The combination of the basis functions nearest to the data in the sense of least squares.
pub fn fit(xs: &[Expr], ys: &[Expr], basis: &[Expr], var: &Expr) -> Res {
    if basis.is_empty() || basis.len() > xs.len() {
        return Err(format!(
            "{} function(s) cannot be fitted to {} point(s)",
            basis.len(),
            xs.len()
        ));
    }
    let design: Vec<Vec<Expr>> = xs
        .iter()
        .map(|x| basis.iter().map(|b| simplify(&subst(b, var, x))).collect())
        .collect();
    let m = basis.len();
    let normal: matrix::Rows = (0..m)
        .map(|i| {
            (0..m)
                .map(|j| {
                    simplify(&add(design
                        .iter()
                        .map(|r| mul(vec![r[i].clone(), r[j].clone()]))
                        .collect()))
                })
                .collect()
        })
        .collect();
    let right: Vec<Expr> = (0..m)
        .map(|i| {
            simplify(&add(design
                .iter()
                .zip(ys)
                .map(|(r, y)| mul(vec![r[i].clone(), y.clone()]))
                .collect()))
        })
        .collect();
    let solved = matrix::linsolve(&normal, &right)
        .map_err(|_| "the basis functions are dependent on these points".to_string())?;
    let cs = solved
        .as_list()
        .filter(|l| l.len() == m)
        .ok_or("no fit found")?;
    if cs
        .iter()
        .any(|c| c.free_syms().iter().any(|s| s.starts_with('t')))
        && xs.iter().all(Expr::is_const)
    {
        return Err("the basis functions are dependent on these points".into());
    }
    Ok(simplify(&add(cs
        .iter()
        .zip(basis)
        .map(|(c, b)| mul(vec![c.clone(), b.clone()]))
        .collect())))
}

fn solve_linear(mut a: Vec<Vec<Complex>>, mut b: Vec<Complex>, prec: u32) -> Option<Vec<Complex>> {
    let n = b.len();
    for col in 0..n {
        let pivot = (col..n).max_by(|i, j| {
            let (x, y) = (
                Float::with_val(64, a[*i][col].abs_ref()),
                Float::with_val(64, a[*j][col].abs_ref()),
            );
            x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)
        })?;
        if a[pivot][col].is_zero() {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        for row in col + 1..n {
            let factor = Complex::with_val(prec, &a[row][col] / &a[col][col]);
            if factor.is_zero() {
                continue;
            }
            let (upper, lower) = a.split_at_mut(row);
            for (target, source) in lower[0].iter_mut().zip(&upper[col]).skip(col) {
                *target -= Complex::with_val(prec, &factor * source);
            }
            let t = Complex::with_val(prec, &factor * &b[col]);
            b[row] -= t;
        }
    }
    let mut x = vec![Complex::new(prec); n];
    for row in (0..n).rev() {
        let mut total = b[row].clone();
        for k in row + 1..n {
            total -= Complex::with_val(prec, &a[row][k] * &x[k]);
        }
        x[row] = total / &a[row][row];
    }
    Some(x)
}

/// The first derivatives of the equations by the unknowns.
pub fn jacobian(eqs: &[Expr], vars: &[String]) -> Vec<Vec<Expr>> {
    eqs.iter()
        .map(|f| vars.iter().map(|v| simplify(&diff(f, v))).collect())
        .collect()
}

/// A solution of `n` equations in `n` unknowns near `start`, by Newton's method.
pub fn newton_system(
    eqs: &[Expr],
    vars: &[String],
    start: &[Complex],
    digits: u32,
) -> Option<Vec<Complex>> {
    newton_from(eqs, &jacobian(eqs, vars), vars, start, digits)
}

/// Newton's method with the derivatives `jac` of the equations.
pub fn newton_from(
    eqs: &[Expr],
    jac: &[Vec<Expr>],
    vars: &[String],
    start: &[Complex],
    digits: u32,
) -> Option<Vec<Complex>> {
    let bits = numeric::bits_for(digits);
    let prec = bits + 64;
    let keys: Vec<Rc<str>> = vars.iter().map(|v| Rc::from(v.as_str())).collect();
    let at = |x: &[Complex]| -> Env {
        let mut env = Env::new();
        for (k, v) in keys.iter().zip(x) {
            env.insert(k.clone(), v.clone());
        }
        env
    };
    let values = |x: &[Complex]| -> Option<Vec<Complex>> {
        let env = at(x);
        eqs.iter()
            .map(|f| numeric::evalf_env(f, prec, &env).ok())
            .collect()
    };
    let size = |v: &[Complex]| -> Float {
        v.iter()
            .map(|z| Float::with_val(64, z.abs_ref()))
            .fold(Float::new(64), |m, x| m.max(&x))
    };
    let mut x: Vec<Complex> = start.iter().map(|z| Complex::with_val(prec, z)).collect();
    let mut f = values(&x)?;
    for _ in 0..300 {
        let env = at(&x);
        let j: Option<Vec<Vec<Complex>>> = jac
            .iter()
            .map(|row| {
                row.iter()
                    .map(|e| numeric::evalf_env(e, prec, &env).ok())
                    .collect()
            })
            .collect();
        let rhs: Vec<Complex> = f.iter().map(|v| Complex::with_val(prec, -v)).collect();
        let step = solve_linear(j?, rhs, prec)?;
        // shorter steps while the equations get worse, and no step out of all bounds
        let before = size(&f);
        let (reach, limit) = (size(&step), (size(&x) + 1u32) << 20u32);
        if !reach.is_finite() {
            return None;
        }
        let mut scale = if reach > limit {
            Float::with_val(prec, &limit / &reach)
        } else {
            Float::with_val(prec, 1)
        };
        let mut moved = None;
        for _ in 0..40 {
            let trial: Vec<Complex> = x
                .iter()
                .zip(&step)
                .map(|(a, d)| Complex::with_val(prec, a + Complex::with_val(prec, d * &scale)))
                .collect();
            if let Some(ft) = values(&trial)
                && (size(&ft) < before || before.is_zero())
            {
                moved = Some((trial, ft));
                break;
            }
            scale /= 2u32;
        }
        let small =
            size(&step) * &scale <= (size(&x) + 1u32) * (Float::with_val(64, 1) >> (bits + 8));
        match moved {
            Some((trial, ft)) => {
                x = trial;
                f = ft;
            }
            None if small || before.is_zero() => return Some(x),
            None => return None,
        }
        if small {
            // one more step squares the error away
            let env = at(&x);
            let j: Option<Vec<Vec<Complex>>> = jac
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|e| numeric::evalf_env(e, prec, &env).ok())
                        .collect()
                })
                .collect();
            let rhs: Vec<Complex> = f.iter().map(|v| Complex::with_val(prec, -v)).collect();
            if let Some(last) = j.and_then(|j| solve_linear(j, rhs, prec)) {
                for (a, d) in x.iter_mut().zip(&last) {
                    *a += d;
                }
            }
            return Some(x);
        }
    }
    None
}

/// The points where every first derivative of `f` vanishes, with the value there and the kind
/// of the point by the signs of the leading minors of the matrix of second derivatives.
pub fn critical_points(f: &Expr, vars: &[String]) -> Res {
    let grad: Vec<Expr> = vars.iter().map(|v| simplify(&diff(f, v))).collect();
    let solved = crate::system::solve_system(&grad, vars)?;
    crate::solve::release(&solved.params);
    let hessian: Vec<Vec<Expr>> = grad
        .iter()
        .map(|g| vars.iter().map(|v| simplify(&diff(g, v))).collect())
        .collect();
    let mut out = Vec::new();
    for assignment in &solved.solutions {
        let at = |e: &Expr| -> Expr {
            simplify(&assignment.iter().fold(e.clone(), |acc, (v, value)| {
                subst(&acc, &Expr::sym(v), value)
            }))
        };
        let point: Vec<Expr> = vars
            .iter()
            .map(|v| {
                assignment
                    .iter()
                    .find(|(name, _)| name == v)
                    .map_or_else(|| Expr::sym(v), |(_, value)| value.clone())
            })
            .collect();
        if point.iter().any(|p| {
            p.is_const() && numeric::evalf_digits(p, 20).is_ok_and(|z| !z.imag().is_zero())
        }) {
            continue;
        }
        let h: matrix::Rows = hessian
            .iter()
            .map(|r| r.iter().map(&at).collect())
            .collect();
        // leading minors: all positive at a minimum, alternating from a negative one at a maximum
        let mut signs = Vec::new();
        for k in 1..=vars.len() {
            let minor: matrix::Rows = h[..k].iter().map(|r| r[..k].to_vec()).collect();
            signs.push(
                matrix::det(&minor)
                    .ok()
                    .and_then(|d| numeric::sign(&simplify(&d))),
            );
        }
        let kind = if signs.iter().any(Option::is_none) {
            "undecided"
        } else if signs.last() == Some(&Some(0)) {
            "degenerate"
        } else if signs.iter().all(|s| *s == Some(1)) {
            "minimum"
        } else if signs
            .iter()
            .enumerate()
            .all(|(k, s)| *s == Some(if k % 2 == 0 { -1 } else { 1 }))
        {
            "maximum"
        } else {
            "saddle"
        };
        out.push(Expr::list(vec![
            if point.len() == 1 {
                point[0].clone()
            } else {
                Expr::list(point)
            },
            at(f),
            Expr::sym(kind),
        ]));
    }
    let _ = (neg, powi);
    Ok(Expr::list(out))
}
