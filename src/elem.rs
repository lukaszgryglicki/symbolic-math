//! Functions of numbers that MATLAB has by other names or with other habits: real roots,
//! rounding to digits, remainders, coordinates, bits, fractions near a number.

use crate::arrays::{order, tidy, whole, word};
use crate::canon::{add, div, mul, neg, pow, sub};
use crate::expr::Expr;
use crate::funcs::func;
use crate::num::Rat;
use crate::numeric;
use rug::Integer;
use std::cmp::Ordering;

type Res = Result<Expr, String>;

/// A function of several arguments entry by entry: lists of one size, numbers with all.
fn zipped(args: &[Expr], f: &dyn Fn(&[Expr]) -> Res) -> Res {
    let len = args
        .iter()
        .filter_map(|x| x.as_list().map(<[Expr]>::len))
        .max();
    let Some(len) = len else {
        return f(args);
    };
    if args
        .iter()
        .any(|x| x.as_list().is_some_and(|l| l.len() != len))
    {
        return Err("lists of different sizes".into());
    }
    let mut out = Vec::with_capacity(len);
    for k in 0..len {
        let row: Vec<Expr> = args
            .iter()
            .map(|x| match x.as_list() {
                Some(l) => l[k].clone(),
                None => x.clone(),
            })
            .collect();
        out.push(zipped(&row, f)?);
    }
    Ok(Expr::list(out))
}

fn real(e: &Expr) -> bool {
    e.is_const() && numeric::real_value(e, 20).is_some()
}

fn sign(e: &Expr) -> Result<Ordering, String> {
    order(e, &Expr::zero())
}

fn nthroot(x: &Expr, n: &Expr) -> Res {
    let k = n
        .as_i64()
        .filter(|k| *k != 0)
        .ok_or("nthroot(x, n): n is a whole number other than 0")?;
    let root = |v: Expr| tidy(pow(v, Expr::rat(1, k)));
    if !x.is_const() {
        return Ok(if k % 2 == 0 {
            root(x.clone())
        } else {
            mul(vec![
                func("sign", vec![x.clone()]),
                root(func("abs", vec![x.clone()])),
            ])
        });
    }
    if !real(x) {
        return Err(format!("nthroot: {x} is not a real number"));
    }
    match sign(x)? {
        Ordering::Less if k % 2 == 0 => Err(format!(
            "nthroot: {x} is negative and has no real root of the even order {k}"
        )),
        Ordering::Less => Ok(tidy(neg(root(neg(x.clone()))))),
        _ => Ok(root(x.clone())),
    }
}

/// The value when it is real, else an error that names the function.
fn must_be_real(value: Expr, name: &str) -> Res {
    let value = tidy(value);
    if !value.is_const() || real(&value) || value.is_infinite() {
        Ok(value)
    } else {
        Err(format!("{name}: the result {value} is not a real number"))
    }
}

/// The least whole p with `2^p >= |x|`.
fn nextpow2(x: &Expr) -> Res {
    if x.is_zero() {
        return Ok(Expr::neg_inf());
    }
    if let Some(r) = x.as_rat() {
        let size = r.clone().abs();
        let (num, den) = (size.numer().clone(), size.denom().clone());
        let mut p = i64::from(num.significant_bits()) - i64::from(den.significant_bits());
        let reaches = |p: i64| {
            let (mut top, mut low) = (den.clone(), num.clone());
            if p >= 0 {
                top <<= p as u32;
            } else {
                low <<= (-p) as u32;
            }
            top >= low
        };
        while !reaches(p) {
            p += 1;
        }
        while reaches(p - 1) {
            p -= 1;
        }
        return Ok(Expr::int(p));
    }
    if !x.is_const() {
        return Ok(func(
            "ceil",
            vec![func("log2", vec![func("abs", vec![x.clone()])])],
        ));
    }
    let size = numeric::real_value(&func("abs", vec![x.clone()]), 40)
        .ok_or_else(|| format!("nextpow2: {x} has no size"))?;
    Ok(Expr::integer(
        size.log2().ceil().to_integer().unwrap_or_default(),
    ))
}

/// The fraction near `x` that the continued fraction with the nearest whole numbers reaches
/// first within `tol`.
fn rat(x: &Expr, tol: Option<&Expr>) -> Res {
    let exact: Rat = match x.as_rat() {
        Some(r) => r.clone(),
        None => {
            let digits = numeric::digits() + 10;
            let v = numeric::real_value(x, digits)
                .ok_or_else(|| format!("rat: {x} is not a real number"))?;
            v.to_rational().ok_or("rat: the number is not finite")?
        }
    };
    let tol: Rat = match tol {
        Some(t) => t
            .as_rat()
            .cloned()
            .or_else(|| numeric::real_value(t, 30).and_then(|f| f.to_rational()))
            .ok_or("rat(x, tol): the tolerance is a real number")?,
        None => exact.clone().abs() / Rat::from(1_000_000),
    };
    let half = Rat::from((1, 2));
    let (mut p1, mut q1, mut p0, mut q0) = (
        Integer::from(1),
        Integer::new(),
        Integer::new(),
        Integer::from(1),
    );
    let mut y = exact.clone();
    for _ in 0..10_000 {
        // (halves go away from zero)
        let d: Integer = if y.cmp0() == Ordering::Less {
            -Rat::from(&half - &y).floor().into_numer_denom().0
        } else {
            Rat::from(&y + &half).floor().into_numer_denom().0
        };
        let frac = Rat::from(&y - Rat::from(d.clone()));
        let (p, q) = (Integer::from(&d * &p1) + &p0, Integer::from(&d * &q1) + &q0);
        (p0, q0) = (p1, q1);
        (p1, q1) = (p, q);
        let near = Rat::from((p1.clone(), q1.clone()));
        if frac.cmp0() == Ordering::Equal || Rat::from(&near - &exact).abs() <= tol {
            break;
        }
        y = frac.recip();
    }
    Ok(Expr::rational(Rat::from((p1, q1))))
}

fn bits(e: &Expr, name: &str) -> Result<Integer, String> {
    e.as_int()
        .filter(|k| k.cmp0() != Ordering::Less)
        .cloned()
        .ok_or_else(|| format!("{name} needs whole numbers that are not negative, {e} is none"))
}

/// The number of bits of a kind of whole number of MATLAB, or of a count given.
fn width(e: Option<&Expr>) -> Result<u32, String> {
    let Some(e) = e else {
        return Ok(64);
    };
    if let Some(k) = e.as_i64().filter(|k| (1..=1_000_000).contains(k)) {
        return Ok(k as u32);
    }
    match word(e).as_deref() {
        Some("uint8" | "int8") => Ok(8),
        Some("uint16" | "int16") => Ok(16),
        Some("uint32" | "int32") => Ok(32),
        Some("uint64" | "int64") => Ok(64),
        _ => Err(format!(
            "the width is a number of bits or \"uint8\", \"uint16\", \"uint32\", \"uint64\", not {e}"
        )),
    }
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    let numbers = |k: usize| a[k].free_syms().is_empty();
    Ok(Some(match (name, n) {
        ("nthroot", 2) => zipped(a, &|v| nthroot(&v[0], &v[1]))?,
        ("realsqrt", 1) => zipped(a, &|v| must_be_real(pow(v[0].clone(), Expr::half()), name))?,
        ("reallog", 1) => zipped(a, &|v| must_be_real(func("ln", vec![v[0].clone()]), name))?,
        ("realpow", 2) => zipped(a, &|v| must_be_real(pow(v[0].clone(), v[1].clone()), name))?,
        ("nextpow2", 1) => zipped(a, &|v| nextpow2(&v[0]))?,
        ("rem", 2) if numbers(0) && numbers(1) => zipped(a, &|v| {
            if v[1].is_zero() {
                return Ok(Expr::undefined());
            }
            Ok(tidy(sub(
                v[0].clone(),
                mul(vec![
                    v[1].clone(),
                    func("trunc", vec![tidy(div(v[0].clone(), v[1].clone()))]),
                ]),
            )))
        })?,
        ("idivide", 2 | 3) => {
            let how = match a.get(2).map(word) {
                None => "trunc",
                Some(Some(w)) => match w.as_str() {
                    "fix" => "trunc",
                    "round" => "round",
                    "floor" => "floor",
                    "ceil" => "ceil",
                    _ => {
                        return Err(
                            "idivide(a, b, \"fix\" | \"round\" | \"floor\" | \"ceil\")".into()
                        );
                    }
                },
                Some(None) => {
                    return Err("idivide(a, b, \"fix\" | \"round\" | \"floor\" | \"ceil\")".into());
                }
            };
            zipped(&a[..2], &|v| {
                Ok(func(how, vec![tidy(div(v[0].clone(), v[1].clone()))]))
            })?
        }
        ("round", 2 | 3) => {
            let places = whole(&a[1], "the number of digits", -100_000, 100_000)?;
            let significant = match a.get(2).map(word) {
                None => false,
                Some(Some(w)) if w == "decimals" => false,
                Some(Some(w)) if w == "significant" => true,
                Some(_) => return Err("round(x, n), round(x, n, \"significant\")".into()),
            };
            if significant && places < 1 {
                return Err("round(x, n, \"significant\"): n is 1 or more".into());
            }
            zipped(&a[..1], &|v| {
                let x = &v[0];
                let shift = if significant {
                    if x.is_zero() {
                        return Ok(Expr::zero());
                    }
                    let size = numeric::real_value(&func("abs", vec![x.clone()]), 40)
                        .ok_or_else(|| format!("round: {x} is not a real number"))?;
                    let lead = size.log10().floor().to_integer().unwrap_or_default();
                    places - 1 - lead.to_i64().unwrap_or(0)
                } else {
                    places
                };
                let scale = pow(Expr::int(10), Expr::int(shift));
                Ok(tidy(div(
                    func("round", vec![tidy(mul(vec![x.clone(), scale.clone()]))]),
                    scale,
                )))
            })?
        }
        ("airy", 2) => {
            let which = match whole(&a[0], "the kind of airy(k, x)", 0, 3)? {
                0 => "airyai",
                1 => "airyaiprime",
                2 => "airybi",
                _ => "airybiprime",
            };
            zipped(&a[1..], &|v| Ok(func(which, v.to_vec())))?
        }
        ("besselh", 3) => {
            let which = match whole(&a[1], "the kind of besselh(nu, k, x)", 1, 2)? {
                1 => "hankel1",
                _ => "hankel2",
            };
            func(which, vec![a[0].clone(), a[2].clone()])
        }
        ("cart2pol", 2 | 3) => {
            let mut out = vec![
                zipped(&a[..2], &|v| {
                    Ok(tidy(func("atan2", vec![v[1].clone(), v[0].clone()])))
                })?,
                zipped(&a[..2], &|v| {
                    Ok(tidy(crate::canon::sqrt(add(vec![
                        pow(v[0].clone(), Expr::int(2)),
                        pow(v[1].clone(), Expr::int(2)),
                    ]))))
                })?,
            ];
            out.extend(a.get(2).cloned());
            Expr::list(out)
        }
        ("pol2cart", 2 | 3) => {
            let mut out = vec![
                zipped(&a[..2], &|v| {
                    Ok(tidy(mul(vec![
                        v[1].clone(),
                        func("cos", vec![v[0].clone()]),
                    ])))
                })?,
                zipped(&a[..2], &|v| {
                    Ok(tidy(mul(vec![
                        v[1].clone(),
                        func("sin", vec![v[0].clone()]),
                    ])))
                })?,
            ];
            out.extend(a.get(2).cloned());
            Expr::list(out)
        }
        ("cart2sph", 3) => {
            let flat = |v: &[Expr]| {
                crate::canon::sqrt(add(vec![
                    pow(v[0].clone(), Expr::int(2)),
                    pow(v[1].clone(), Expr::int(2)),
                ]))
            };
            Expr::list(vec![
                zipped(a, &|v| {
                    Ok(tidy(func("atan2", vec![v[1].clone(), v[0].clone()])))
                })?,
                zipped(a, &|v| {
                    Ok(tidy(func("atan2", vec![v[2].clone(), tidy(flat(v))])))
                })?,
                zipped(a, &|v| {
                    Ok(tidy(crate::canon::sqrt(add(vec![
                        pow(v[0].clone(), Expr::int(2)),
                        pow(v[1].clone(), Expr::int(2)),
                        pow(v[2].clone(), Expr::int(2)),
                    ]))))
                })?,
            ])
        }
        ("sph2cart", 3) => {
            let part = |f1: &'static str, f2: Option<&'static str>| {
                move |v: &[Expr]| -> Res {
                    let mut factors = vec![v[2].clone(), func(f1, vec![v[1].clone()])];
                    if let Some(f2) = f2 {
                        factors.push(func(f2, vec![v[0].clone()]));
                    }
                    Ok(tidy(mul(factors)))
                }
            };
            Expr::list(vec![
                zipped(a, &part("cos", Some("cos")))?,
                zipped(a, &part("cos", Some("sin")))?,
                zipped(a, &part("sin", None))?,
            ])
        }
        ("rat", 1 | 2) => zipped(&a[..1], &|v| rat(&v[0], a.get(1)))?,
        ("int8" | "int16" | "int32" | "int64" | "uint8" | "uint16" | "uint32" | "uint64", 1) => {
            let signed = !name.starts_with('u');
            let width: u32 = name
                .trim_start_matches(['u', 'i', 'n', 't'])
                .parse()
                .unwrap_or(64);
            let (low, high) = if signed {
                (
                    -(Integer::from(1) << (width - 1)),
                    (Integer::from(1) << (width - 1)) - 1u32,
                )
            } else {
                (Integer::new(), (Integer::from(1) << width) - 1u32)
            };
            zipped(a, &|v| {
                let x = &v[0];
                if x.is_undefined() {
                    return Ok(Expr::zero());
                }
                if x.is_inf() {
                    return Ok(Expr::integer(high.clone()));
                }
                if x.is_neg_inf() {
                    return Ok(Expr::integer(low.clone()));
                }
                let rounded = func("round", vec![x.clone()]);
                let k = match rounded.as_int() {
                    Some(k) => k.clone(),
                    None => numeric::real_value(x, numeric::digits() + 5)
                        .and_then(|f| f.round().to_integer())
                        .ok_or_else(|| format!("{name}: {x} is not a real number"))?,
                };
                Ok(Expr::integer(k.clamp(&low, &high)))
            })?
        }
        ("bitand" | "bitor" | "bitxor", 2) => zipped(a, &|v| {
            let (x, y) = (bits(&v[0], name)?, bits(&v[1], name)?);
            Ok(Expr::integer(match name {
                "bitand" => x & y,
                "bitor" => x | y,
                _ => x ^ y,
            }))
        })?,
        ("bitshift", 2 | 3) => {
            let wide = match a.get(2) {
                Some(w) => Some(width(Some(w))?),
                None => None,
            };
            zipped(&a[..2], &|v| {
                let x = bits(&v[0], name)?;
                let k = whole(&v[1], "the shift", -1_000_000, 1_000_000)?;
                let mut out = if k >= 0 {
                    x << k as u32
                } else {
                    x >> (-k) as u32
                };
                if let Some(w) = wide {
                    out.keep_bits_mut(w);
                }
                Ok(Expr::integer(out))
            })?
        }
        ("bitcmp", 1 | 2) => {
            let w = width(a.get(1))?;
            zipped(&a[..1], &|v| {
                let x = bits(&v[0], name)?;
                let all = (Integer::from(1) << w) - 1u32;
                if x > all {
                    return Err(format!("bitcmp: {x} has more than {w} bits"));
                }
                Ok(Expr::integer(all - x))
            })?
        }
        ("bitget", 2) => zipped(a, &|v| {
            let x = bits(&v[0], name)?;
            let k = whole(&v[1], "the bit", 1, 1_000_000)? as u32;
            Ok(Expr::int(i64::from(x.get_bit(k - 1))))
        })?,
        ("bitset", 2 | 3) => {
            let on = match a.get(2) {
                None => true,
                Some(v) => crate::arrays::truthy(v)?,
            };
            zipped(&a[..2], &|v| {
                let mut x = bits(&v[0], name)?;
                let k = whole(&v[1], "the bit", 1, 1_000_000)? as u32;
                x.set_bit(k - 1, on);
                Ok(Expr::integer(x))
            })?
        }
        _ => return Ok(None),
    }))
}

pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    inner(name, a).transpose()
}
