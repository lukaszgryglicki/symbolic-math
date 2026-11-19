//! Descriptive statistics of lists of numbers or expressions.

use crate::canon::{add, div, mul, pow, powi, sqrt, sub};
use crate::expr::Expr;
use crate::funcs::func;
use crate::numeric;
use crate::simplify::simplify;

type Res = Result<Expr, String>;

fn mean(items: &[Expr]) -> Expr {
    div(add(items.to_vec()), Expr::int(items.len() as i64))
}

/// Whether every entry is a real number.
fn all_real(items: &[Expr]) -> bool {
    items.iter().all(|x| {
        x.as_rat().is_some()
            || (x.is_const() && numeric::real_value(x, 20).is_some())
            || !x.is_const()
    })
}

/// `|x|^2`, written as `x^2` for real data.
fn square(x: Expr, real: bool) -> Expr {
    if real {
        powi(x, 2)
    } else {
        powi(func("abs", vec![x]), 2)
    }
}

fn central(items: &[Expr], k: i64) -> Expr {
    let m = mean(items);
    mean(
        &items
            .iter()
            .map(|x| powi(sub(x.clone(), m.clone()), k))
            .collect::<Vec<_>>(),
    )
}

/// The sum of the squared distances from the mean.
fn spread(items: &[Expr]) -> Expr {
    let m = mean(items);
    let real = all_real(items);
    add(items
        .iter()
        .map(|x| square(sub(x.clone(), m.clone()), real))
        .collect())
}

fn covariance(x: &[Expr], y: &[Expr]) -> Expr {
    let (mx, my) = (mean(x), mean(y));
    div(
        add(x
            .iter()
            .zip(y)
            .map(|(p, q)| mul(vec![sub(p.clone(), mx.clone()), sub(q.clone(), my.clone())]))
            .collect()),
        Expr::int(x.len() as i64 - 1),
    )
}

/// The value at the fraction `q` of the sorted data, between two entries by a straight line.
fn quantile(sorted: &[Expr], q: &Expr) -> Res {
    let n = sorted.len();
    let h = simplify(&mul(vec![Expr::int(n as i64 - 1), q.clone()]));
    let at = numeric::real_value(&h, 20)
        .filter(|v| !v.is_sign_negative() || v.is_zero())
        .ok_or("quantile(list, q): q must be a number from 0 to 1")?;
    let lower = at
        .clone()
        .floor()
        .to_integer()
        .and_then(|k| k.to_usize())
        .filter(|k| *k < n)
        .ok_or("quantile(list, q): q must be a number from 0 to 1")?;
    if lower + 1 >= n {
        if at > lower as f64 {
            return Err("quantile(list, q): q must be a number from 0 to 1".into());
        }
        return Ok(sorted[lower].clone());
    }
    let part = sub(h, Expr::int(lower as i64));
    Ok(simplify(&add(vec![
        sorted[lower].clone(),
        mul(vec![
            part,
            sub(sorted[lower + 1].clone(), sorted[lower].clone()),
        ]),
    ])))
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let known = matches!(
        (name, a.len()),
        (
            "median"
                | "mode"
                | "variance"
                | "stddev"
                | "pvariance"
                | "pstddev"
                | "skewness"
                | "kurtosis"
                | "geometricmean"
                | "harmonicmean"
                | "rms"
                | "meandeviation"
                | "quartiles"
                | "iqr",
            1
        ) | (
            "covariance" | "correlation" | "quantile" | "moment" | "centralmoment",
            2
        )
    );
    if !known {
        return Ok(None);
    }
    let data = |k: usize| -> Result<&[Expr], String> {
        a[k].as_list()
            .filter(|l| !l.is_empty() && !l.iter().any(|x| matches!(x, Expr::List(_))))
            .ok_or_else(|| format!("{name} needs a list of values that is not empty"))
    };
    let items = data(0)?;
    let n = items.len();
    let count = Expr::int(n as i64);
    let several = |what: &str| -> Result<(), String> {
        if n < 2 {
            Err(format!("{what} needs two values or more"))
        } else {
            Ok(())
        }
    };
    let ordered = || -> Result<Vec<Expr>, String> {
        if !items
            .iter()
            .all(|x| x.is_const() && numeric::real_value(x, 20).is_some())
        {
            return Err(format!("{name} needs real numbers"));
        }
        Ok(crate::lists::sorted(items))
    };
    let order = |e: &Expr| {
        e.as_i64()
            .filter(|k| (0..=1000).contains(k))
            .ok_or_else(|| format!("{name}(list, k): k must be a whole number from 0 to 1000"))
    };
    let value = match name {
        "median" => quantile(&ordered()?, &Expr::half())?,
        "quantile" => match &a[1] {
            Expr::List(qs) => {
                let sorted = ordered()?;
                Expr::list(
                    qs.iter()
                        .map(|q| quantile(&sorted, q))
                        .collect::<Result<_, _>>()?,
                )
            }
            q => quantile(&ordered()?, q)?,
        },
        "quartiles" | "iqr" => {
            let sorted = ordered()?;
            let q = |k: i64| quantile(&sorted, &Expr::rat(k, 4));
            if name == "iqr" {
                simplify(&sub(q(3)?, q(1)?))
            } else {
                Expr::list(vec![q(1)?, q(2)?, q(3)?])
            }
        }
        "mode" => {
            let mut best: Vec<Expr> = Vec::new();
            let mut most = 0;
            let mut seen: Vec<&Expr> = Vec::new();
            for x in items {
                if seen.contains(&x) {
                    continue;
                }
                seen.push(x);
                let c = items.iter().filter(|y| *y == x).count();
                if c > most {
                    most = c;
                    best = vec![x.clone()];
                } else if c == most {
                    best.push(x.clone());
                }
            }
            if best.len() == 1 {
                best.pop().expect("one entry")
            } else {
                Expr::list(crate::lists::sorted(&best))
            }
        }
        "variance" | "stddev" => {
            several(name)?;
            let v = div(spread(items), Expr::int(n as i64 - 1));
            simplify(&if name == "stddev" { sqrt(v) } else { v })
        }
        "pvariance" | "pstddev" => {
            let v = div(spread(items), count);
            simplify(&if name == "pstddev" { sqrt(v) } else { v })
        }
        "covariance" | "correlation" => {
            let other = data(1)?;
            if other.len() != n {
                return Err(format!("{name}: the two lists must be equally long"));
            }
            several(name)?;
            let c = covariance(items, other);
            simplify(&if name == "correlation" {
                div(
                    c,
                    sqrt(mul(vec![
                        covariance(items, items),
                        covariance(other, other),
                    ])),
                )
            } else {
                c
            })
        }
        "moment" => {
            let k = order(&a[1])?;
            simplify(&mean(
                &items.iter().map(|x| powi(x.clone(), k)).collect::<Vec<_>>(),
            ))
        }
        "centralmoment" => simplify(&central(items, order(&a[1])?)),
        "skewness" => {
            several(name)?;
            simplify(&div(
                central(items, 3),
                pow(simplify(&central(items, 2)), Expr::rat(3, 2)),
            ))
        }
        "kurtosis" => {
            several(name)?;
            simplify(&div(
                central(items, 4),
                powi(simplify(&central(items, 2)), 2),
            ))
        }
        "geometricmean" => simplify(&pow(mul(items.to_vec()), div(Expr::one(), count))),
        "harmonicmean" => simplify(&div(
            count,
            add(items.iter().map(|x| powi(x.clone(), -1)).collect()),
        )),
        "rms" => {
            let real = all_real(items);
            simplify(&sqrt(mean(
                &items
                    .iter()
                    .map(|x| square(x.clone(), real))
                    .collect::<Vec<_>>(),
            )))
        }
        "meandeviation" => {
            let m = mean(items);
            simplify(&mean(
                &items
                    .iter()
                    .map(|x| func("abs", vec![sub(x.clone(), m.clone())]))
                    .collect::<Vec<_>>(),
            ))
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    inner(name, a).transpose()
}
