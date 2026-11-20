//! The least value of a function where conditions hold: equations, inequalities and bounds.
//! The conditions are paid for with multipliers and a growing penalty (the augmented
//! Lagrangian); every round is a descent with the gradients of the expressions themselves.

use crate::canon::{add, mul, sub};
use crate::diff::diff;
use crate::eval::Session;
use crate::expr::Expr;
use crate::matrix;
use crate::mfun::{Numeric, bits, real, value};
use crate::numeric;
use rug::Float;

type Res = Result<Expr, String>;

const USAGE: &str = "fmincon(f, start, A, b, Aeq, beq, lower, upper, conditions): the least f(x) with A*x <= b, Aeq*x = beq, lower <= x <= upper and conditions(x) = [[values that are <= 0], [values that are 0]]; [] leaves a part out";

/// A function of the point with its derivatives by every coordinate.
struct Smooth {
    value: Numeric,
    slopes: Vec<Numeric>,
}

impl Smooth {
    fn new(params: &[String], body: &Expr, prec: u32) -> Smooth {
        Smooth {
            value: Numeric::new(params, body, prec),
            slopes: params
                .iter()
                .map(|p| Numeric::new(params, &diff(body, p), prec))
                .collect(),
        }
    }

    fn at(&self, x: &[Float]) -> Result<(Float, Vec<Float>), String> {
        let slopes = self
            .slopes
            .iter()
            .map(|s| s.at(x))
            .collect::<Result<_, _>>()?;
        Ok((self.value.at(x)?, slopes))
    }
}

struct Problem {
    goal: Smooth,
    equal: Vec<Smooth>,
    below: Vec<Smooth>,
    prec: u32,
    /// How far from the start a descent may go before it counts as running away.
    far: Float,
}

const AWAY: &str = "fmincon: the function has no least value where the conditions hold";

/// The multipliers of the conditions and the weight of the penalty.
struct Prices {
    equal: Vec<Float>,
    below: Vec<Float>,
    weight: Float,
}

impl Problem {
    fn fl<T>(&self, x: T) -> Float
    where
        Float: rug::Assign<T>,
    {
        Float::with_val(self.prec, x)
    }

    /// The function with the conditions paid for, and its gradient.
    fn priced(&self, x: &[Float], prices: &Prices) -> Result<(Float, Vec<Float>), String> {
        let (mut total, mut slope) = self.goal.at(x)?;
        for (h, price) in self.equal.iter().zip(&prices.equal) {
            let (v, d) = h.at(x)?;
            let pull = self.fl(price + self.fl(&prices.weight * &v));
            total += self.fl(price * &v) + self.fl(v.square_ref()) * &prices.weight / 2u32;
            for (s, di) in slope.iter_mut().zip(&d) {
                *s += self.fl(&pull * di);
            }
        }
        for (g, price) in self.below.iter().zip(&prices.below) {
            let (v, d) = g.at(x)?;
            let pull = self
                .fl(price + self.fl(&prices.weight * &v))
                .max(&self.fl(0));
            total +=
                (self.fl(pull.square_ref()) - self.fl(price.square_ref())) / &prices.weight / 2u32;
            for (s, di) in slope.iter_mut().zip(&d) {
                *s += self.fl(&pull * di);
            }
        }
        Ok((total, slope))
    }

    /// A descent to the least priced value near a start, with a matrix that learns the
    /// curvature from the steps taken (Broyden, Fletcher, Goldfarb and Shanno).
    fn descend(
        &self,
        mut x: Vec<Float>,
        prices: &Prices,
        tiny: &Float,
    ) -> Result<Vec<Float>, String> {
        let n = x.len();
        let unit = |fl: &dyn Fn(u32) -> Float| -> Vec<Vec<Float>> {
            (0..n)
                .map(|i| (0..n).map(|j| fl(u32::from(i == j))).collect())
                .collect()
        };
        let fl = |k: u32| self.fl(k);
        let dot = |a: &[Float], b: &[Float]| {
            a.iter()
                .zip(b)
                .fold(self.fl(0), |s, (p, q)| s + self.fl(p * q))
        };
        let mut curve = unit(&fl);
        let mut stuck = 0;
        let (mut fx, mut gx) = self.priced(&x, prices)?;
        for _ in 0..5000 {
            if gx.iter().all(Float::is_zero) {
                return Ok(x);
            }
            let mut step: Vec<Float> = curve.iter().map(|row| -dot(row, &gx)).collect();
            let mut fall = dot(&gx, &step);
            if fall >= 0 || fall.is_nan() {
                curve = unit(&fl);
                step = gx.iter().map(|g| self.fl(-g)).collect();
                fall = dot(&gx, &step);
            }
            let mut size = self.fl(1);
            let mut taken = None;
            for _ in 0..(self.prec + 64) {
                let next: Vec<Float> = x
                    .iter()
                    .zip(&step)
                    .map(|(p, d)| self.fl(p + self.fl(d * &size)))
                    .collect();
                // (where the function has no value the step was too long)
                if let Ok((f_next, g_next)) = self.priced(&next, prices)
                    && f_next <= self.fl(&fx + self.fl(&fall * &size) / 10_000u32)
                {
                    taken = Some((next, f_next, g_next));
                    break;
                }
                size /= 2u32;
            }
            // (no shorter step goes down: this is the lowest point that the numbers can tell)
            let Some((next, f_next, g_next)) = taken else {
                return Ok(x);
            };
            let moved: Vec<Float> = next.iter().zip(&x).map(|(p, q)| self.fl(p - q)).collect();
            let turned: Vec<Float> = g_next
                .iter()
                .zip(&gx)
                .map(|(p, q)| self.fl(p - q))
                .collect();
            let far = moved
                .iter()
                .fold(self.fl(0), |m, v| m.max(&self.fl(v.abs_ref())));
            let wide = x
                .iter()
                .fold(self.fl(1), |m, v| m.max(&self.fl(v.abs_ref())));
            if wide > self.far {
                return Err(AWAY.into());
            }
            let gain = dot(&moved, &turned);
            if gain > 0 {
                // curve = (1 - s*y'/g)*curve*(1 - y*s'/g) + s*s'/g
                let along: Vec<Float> = curve.iter().map(|row| dot(row, &turned)).collect();
                let bend = dot(&turned, &along);
                let scale = self.fl(1) + self.fl(&bend / &gain);
                for i in 0..n {
                    for j in 0..n {
                        let grow = self.fl(&moved[i] * &moved[j]) * &scale / &gain;
                        let cut = (self.fl(&along[i] * &moved[j]) + self.fl(&moved[i] * &along[j]))
                            / &gain;
                        curve[i][j] += grow - cut;
                    }
                }
            }
            // (a step along which the slope did not rise teaches nothing: what was learnt
            // about the walls of the conditions stays)
            (x, fx, gx) = (next, f_next, g_next);
            if far <= self.fl(tiny * &wide) {
                // a short step is the end when it was a whole step of the learnt curvature;
                // one cut short many times is a descent that got stuck
                match size == 1u32 {
                    true => return Ok(x),
                    false => stuck += 1,
                }
                if stuck > 8 {
                    return Err(AWAY.into());
                }
            } else {
                stuck = 0;
            }
        }
        Err(AWAY.into())
    }
}

impl Session {
    pub(crate) fn fmincon(&mut self, a: &[Expr]) -> Res {
        let name = "fmincon";
        let start: Vec<Expr> = match a[1].as_list() {
            Some(items) => items.to_vec(),
            None => vec![a[1].clone()],
        };
        let n = start.len();
        if n == 0 {
            return Err(USAGE.into());
        }
        let prec = 2 * bits();
        let (params, body) = self.function_of(&a[0], n, name)?;
        let names: Vec<Expr> = params.iter().map(|p| Expr::sym(p)).collect();
        let empty = |k: usize| {
            a.get(k)
                .is_none_or(|e| e.as_list().is_some_and(|l| l.is_empty()))
        };
        // A*x - b for the rows of a matrix (a flat list is one row) and a list of numbers
        let rows = |m: usize, b: usize| -> Result<Vec<Expr>, String> {
            if empty(m) && empty(b) {
                return Ok(Vec::new());
            }
            let (m, b) = (&a[m], &a[b]);
            let rows = match m.as_list() {
                Some(flat) if flat.iter().all(|x| x.as_list().is_none()) => vec![flat.to_vec()],
                _ => matrix::rows(m).ok_or(USAGE)?,
            };
            let right: Vec<Expr> = match b.as_list() {
                Some(items) => items.to_vec(),
                None => vec![b.clone()],
            };
            if rows.len() != right.len() || rows.iter().any(|r| r.len() != n) {
                return Err(format!(
                    "fmincon: a matrix of conditions has a column for every one of the {n} unknowns and a number on the right for every row"
                ));
            }
            Ok(rows
                .iter()
                .zip(&right)
                .map(|(row, r)| {
                    let left = row
                        .iter()
                        .zip(&names)
                        .map(|(c, x)| mul(vec![c.clone(), x.clone()]))
                        .collect();
                    sub(add(left), r.clone())
                })
                .collect())
        };
        let mut below = rows(2, 3)?;
        let mut equal = rows(4, 5)?;
        for (k, lower) in [(6, true), (7, false)] {
            if empty(k) {
                continue;
            }
            let bounds = match a[k].as_list() {
                Some(items) => items.to_vec(),
                None => vec![a[k].clone()],
            };
            if bounds.len() != n {
                return Err(format!(
                    "fmincon: a bound for every one of the {n} unknowns"
                ));
            }
            for (bound, x) in bounds.iter().zip(&names) {
                if bound.is_inf() || bound.is_neg_inf() {
                    continue;
                }
                below.push(match lower {
                    true => sub(bound.clone(), x.clone()),
                    false => sub(x.clone(), bound.clone()),
                });
            }
        }
        let mut problem = Problem {
            goal: Smooth::new(&params, &body, prec),
            equal: equal
                .drain(..)
                .map(|e| Smooth::new(&params, &e, prec))
                .collect(),
            below: below
                .drain(..)
                .map(|e| Smooth::new(&params, &e, prec))
                .collect(),
            prec,
            far: Float::with_val(prec, 1),
        };
        if !empty(8) {
            let (own, both) = self.function_of(&a[8], n, name)?;
            let parts: Vec<Vec<Expr>> = match both.as_list() {
                Some([Expr::List(under), Expr::List(zero)]) => vec![under.to_vec(), zero.to_vec()],
                Some(flat) if flat.iter().all(|x| x.as_list().is_none()) => {
                    vec![flat.to_vec(), Vec::new()]
                }
                Some(_) => return Err(USAGE.into()),
                None => vec![vec![both.clone()], Vec::new()],
            };
            problem
                .below
                .extend(parts[0].iter().map(|e| Smooth::new(&own, e, prec)));
            problem
                .equal
                .extend(parts[1].iter().map(|e| Smooth::new(&own, e, prec)));
        }
        let mut x: Vec<Float> = start
            .iter()
            .map(|e| real(e, prec, name))
            .collect::<Result<_, _>>()?;
        let zero = || Float::new(prec);
        let mut prices = Prices {
            equal: vec![zero(); problem.equal.len()],
            below: vec![zero(); problem.below.len()],
            weight: Float::with_val(prec, 10),
        };
        let tiny = Float::with_val(prec, Float::i_pow_u(10, numeric::digits() + 4)).recip();
        let free = problem.equal.is_empty() && problem.below.is_empty();
        let wide = x.iter().fold(Float::with_val(prec, 1), |m, v| {
            m.max(&Float::with_val(prec, v.abs_ref()))
        });
        problem.far = wide * 1e12f64;
        for round in 0..90 {
            let next = match problem.descend(x.clone(), &prices, &tiny) {
                Ok(next) => next,
                // (a penalty too light to hold the function back: a heavier one from here)
                Err(_) if !free && prices.weight < 1e45 => {
                    prices.weight *= 100u32;
                    continue;
                }
                Err(why) => return Err(why),
            };
            let mut off = zero();
            for (h, price) in problem.equal.iter().zip(prices.equal.iter_mut()) {
                let v = h.value.at(&next)?;
                *price += Float::with_val(prec, &prices.weight * &v);
                off = off.max(&v.abs());
            }
            for (g, price) in problem.below.iter().zip(prices.below.iter_mut()) {
                let v = g.value.at(&next)?;
                *price =
                    Float::with_val(prec, &*price + Float::with_val(prec, &prices.weight * &v))
                        .max(&zero());
                off = off.max(&v);
            }
            let moved = next.iter().zip(&x).fold(zero(), |m, (p, q)| {
                m.max(&Float::with_val(prec, p - q).abs())
            });
            let wide = next.iter().fold(Float::with_val(prec, 1), |m, v| {
                m.max(&Float::with_val(prec, v.abs_ref()))
            });
            x = next;
            if free || (round > 0 && off <= tiny && moved <= Float::with_val(prec, &tiny * &wide)) {
                break;
            }
            if round == 89 {
                return Err("fmincon: no point was found where the conditions hold and the function is least".into());
            }
            if prices.weight < 1e45 {
                prices.weight *= 10u32;
            }
        }
        Ok(match a[1].as_list() {
            Some(_) => Expr::list(x.iter().map(value).collect()),
            None => value(&x[0]),
        })
    }
}
