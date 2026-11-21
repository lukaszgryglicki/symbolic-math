//! Small networks of neurons: the functions that their units apply, their losses, the
//! operations of their layers, the steps of their training, and networks of dense layers.
//! A network is a plain value: a list of layers [weights, biases, "function"], where a
//! row of the weights belongs to a unit. Rows of data are observations.

use crate::arrays::{map_result, order, whole, word};
use crate::canon::{add, div, mul, neg, pow, powi, sub};
use crate::expr::Expr;
use crate::funcs::func;
use crate::numeric;
use crate::simplify::simplify;
use rug::Float;
use std::cmp::Ordering;

type Res = Result<Expr, String>;

/// Rows of values.
type Rows = Vec<Vec<Expr>>;

fn bits() -> u32 {
    crate::mfun::bits()
}

pub(crate) fn fl<T>(x: T) -> Float
where
    Float: rug::Assign<T>,
{
    Float::with_val(bits(), x)
}

pub(crate) fn zero() -> Float {
    Float::new(bits())
}

pub(crate) fn real(e: &Expr, who: &str) -> Result<Float, String> {
    crate::mfun::real(e, bits(), who)
}

pub(crate) fn shown(x: &Float) -> Expr {
    crate::mfun::value(x)
}

pub(crate) fn text(e: &Expr) -> Option<String> {
    word(e).map(|w| {
        w.chars()
            .filter(|c| !matches!(c, ' ' | '-' | '_'))
            .collect()
    })
}

fn int(k: i64) -> Expr {
    Expr::int(k)
}

/// Whether a number is below, at or above 0, when it is one.
fn side(x: &Expr) -> Option<Ordering> {
    numeric::real_value(x, numeric::digits() + 10).and_then(|v| v.cmp0())
}

fn exp(x: Expr) -> Expr {
    func("exp", vec![x])
}

fn abs(x: Expr) -> Expr {
    func("abs", vec![x])
}

fn half(x: Expr) -> Expr {
    div(x, int(2))
}

/// The first name of a function of a unit.
pub(crate) fn unit_name(w: &str) -> Option<&'static str> {
    Some(match w {
        "relu" | "poslin" => "relu",
        "leakyrelu" => "leakyrelu",
        "clippedrelu" => "clippedrelu",
        "elu" => "elu",
        "gelu" => "gelu",
        "swish" | "silu" => "swish",
        "mish" => "mish",
        "softplus" => "softplus",
        "softsign" | "elliotsig" => "softsign",
        "sigmoid" | "logsig" | "logistic" => "sigmoid",
        "tanh" | "tansig" => "tanh",
        "purelin" | "linear" | "identity" => "purelin",
        "satlin" => "satlin",
        "satlins" => "satlins",
        "hardlim" => "hardlim",
        "hardlims" => "hardlims",
        "radbas" => "radbas",
        "tribas" => "tribas",
        "netinv" => "netinv",
        "softmax" => "softmax",
        "compet" => "compet",
        "rbf" => "rbf",
        _ => return None,
    })
}

fn unit_names() -> String {
    "relu (poslin), leakyrelu, clippedrelu, elu, gelu, swish, mish, softplus, softsign (elliotsig), sigmoid (logsig), tanh (tansig), \
     purelin, satlin, satlins, hardlim, hardlims, radbas, tribas, netinv, softmax, compet"
        .to_string()
}

/// The number that a function of a unit takes besides its value, unless one is given.
pub(crate) fn usual(name: &str) -> Option<Expr> {
    match name {
        "leakyrelu" => Some(Expr::rat(1, 100)),
        "clippedrelu" => Some(int(6)),
        "elu" => Some(int(1)),
        _ => None,
    }
}

/// A function of a unit for one value: exact, and for a name a formula that has slopes.
fn unit(name: &str, x: &Expr, extra: Option<&Expr>) -> Res {
    let given = extra.cloned().or_else(|| usual(name));
    let s = side(x);
    let cut = matches!(
        name,
        "relu"
            | "leakyrelu"
            | "clippedrelu"
            | "satlin"
            | "satlins"
            | "elu"
            | "hardlim"
            | "hardlims"
            | "tribas"
    );
    if cut && s.is_none() && x.is_const() {
        return Err(format!("{name}: {x} is not a real number"));
    }
    let below = s == Some(Ordering::Less);
    let one = int(1);
    Ok(simplify(&match name {
        "relu" => match s {
            Some(_) if below => int(0),
            Some(_) => x.clone(),
            None => half(add(vec![x.clone(), abs(x.clone())])),
        },
        "leakyrelu" => {
            let a = given.expect("a slope");
            match s {
                Some(_) if below => mul(vec![a, x.clone()]),
                Some(_) => x.clone(),
                None => half(add(vec![
                    mul(vec![add(vec![one.clone(), a.clone()]), x.clone()]),
                    mul(vec![sub(one, a), abs(x.clone())]),
                ])),
            }
        }
        "clippedrelu" | "satlin" => {
            let top = match name {
                "satlin" => one,
                _ => given.expect("a ceiling"),
            };
            match s {
                Some(_) if below => int(0),
                Some(_) => match order(x, &top)? {
                    Ordering::Greater => top,
                    _ => x.clone(),
                },
                None => half(add(vec![
                    sub(abs(x.clone()), abs(sub(x.clone(), top.clone()))),
                    top,
                ])),
            }
        }
        "satlins" => match s {
            Some(_) => match (order(x, &int(-1))?, order(x, &one)?) {
                (Ordering::Less, _) => int(-1),
                (_, Ordering::Greater) => one,
                _ => x.clone(),
            },
            None => half(sub(
                abs(add(vec![x.clone(), one.clone()])),
                abs(sub(x.clone(), one)),
            )),
        },
        "elu" => {
            let a = given.expect("a scale");
            match s {
                Some(Ordering::Greater) => x.clone(),
                Some(_) => mul(vec![a, sub(exp(x.clone()), one)]),
                None => add(vec![
                    half(add(vec![x.clone(), abs(x.clone())])),
                    mul(vec![a, sub(exp(half(sub(x.clone(), abs(x.clone())))), one)]),
                ]),
            }
        }
        "gelu" => mul(vec![
            half(x.clone()),
            add(vec![
                one,
                func("erf", vec![div(x.clone(), pow(int(2), Expr::rat(1, 2)))]),
            ]),
        ]),
        "swish" => div(x.clone(), add(vec![one, exp(neg(x.clone()))])),
        "mish" => mul(vec![
            x.clone(),
            func(
                "tanh",
                vec![func("ln", vec![add(vec![one, exp(x.clone())])])],
            ),
        ]),
        "softplus" => func("ln", vec![add(vec![one, exp(x.clone())])]),
        "softsign" => div(x.clone(), add(vec![one, abs(x.clone())])),
        "sigmoid" => div(one.clone(), add(vec![one, exp(neg(x.clone()))])),
        "tanh" => func("tanh", vec![x.clone()]),
        "purelin" => x.clone(),
        "hardlim" => match s {
            Some(_) if below => int(0),
            Some(_) => one,
            None => Expr::call("hardlim", vec![x.clone()]),
        },
        "hardlims" => match s {
            Some(_) if below => int(-1),
            Some(_) => one,
            None => Expr::call("hardlims", vec![x.clone()]),
        },
        "radbas" => exp(neg(powi(x.clone(), 2))),
        "tribas" => {
            let rest = sub(one, abs(x.clone()));
            match side(&simplify(&rest)) {
                Some(Ordering::Greater) => rest,
                Some(_) => int(0),
                None => half(add(vec![rest.clone(), abs(rest)])),
            }
        }
        "netinv" => div(one, x.clone()),
        _ => {
            return Err(format!(
                "{name} is no function of one unit: the functions are {}",
                unit_names()
            ));
        }
    }))
}

/// The entries of a list that are largest: the place of the first one.
fn first_largest(items: &[Expr], who: &str) -> Result<usize, String> {
    if items.is_empty() {
        return Err(format!("{who} needs values"));
    }
    let mut best = 0;
    for (k, item) in items.iter().enumerate().skip(1) {
        if order(item, &items[best]).map_err(|_| format!("{who}: {item} is not a real number"))?
            == Ordering::Greater
        {
            best = k;
        }
    }
    Ok(best)
}

/// The exponentials of a list over their sum: with decimal numbers by numbers, else exact.
fn softmax(items: &[Expr], who: &str) -> Result<Vec<Expr>, String> {
    if items.is_empty() {
        return Ok(Vec::new());
    }
    if items.iter().any(|x| x.has_float()) {
        let values = items
            .iter()
            .map(|x| real(x, who))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(soft(&values).iter().map(shown).collect());
    }
    let powers: Vec<Expr> = items.iter().map(|x| exp(x.clone())).collect();
    let total = add(powers.clone());
    Ok(powers
        .into_iter()
        .map(|p| simplify(&div(p, total.clone())))
        .collect())
}

pub(crate) fn soft(values: &[Float]) -> Vec<Float> {
    let top = values.iter().fold(values[0].clone(), |a, b| a.max(b));
    let powers: Vec<Float> = values.iter().map(|v| fl(v - &top).exp()).collect();
    let total = powers.iter().fold(zero(), |a, b| a + b);
    powers.into_iter().map(|p| p / &total).collect()
}

/// A matrix as rows, or a flat list as one row.
fn rows(e: &Expr) -> Vec<Vec<Expr>> {
    match e.as_list() {
        Some(items) if !items.is_empty() && items.iter().all(|r| r.as_list().is_some()) => items
            .iter()
            .map(|r| r.as_list().expect("a row").to_vec())
            .collect(),
        Some(items) => vec![items.to_vec()],
        None => vec![vec![e.clone()]],
    }
}

fn nested(e: &Expr) -> bool {
    e.as_list()
        .is_some_and(|items| !items.is_empty() && items.iter().all(|r| r.as_list().is_some()))
}

fn matrix(m: Vec<Vec<Expr>>) -> Expr {
    Expr::list(m.into_iter().map(Expr::list).collect())
}

fn even(m: &[Vec<Expr>], who: &str) -> Result<usize, String> {
    let width = m.first().map_or(0, Vec::len);
    match m.iter().all(|r| r.len() == width) {
        true => Ok(width),
        false => Err(format!("{who}: the rows must have the same length")),
    }
}

fn total(items: impl IntoIterator<Item = Expr>) -> Expr {
    add(items.into_iter().collect())
}

/// Observations and what they should be, row by row; a flat list is one observation
/// unless the other one is a list of rows.
fn pairs(y: &Expr, t: &Expr, who: &str) -> Result<(Rows, Rows), String> {
    let (ys, ts) = (rows(y), rows(t));
    if ys.len() != ts.len() || ys.iter().zip(&ts).any(|(a, b)| a.len() != b.len()) || ys.is_empty()
    {
        return Err(format!(
            "{who}: the values and the targets must have the same shape"
        ));
    }
    Ok((ys, ts))
}

/// The losses.
fn loss(name: &str, a: &[Expr]) -> Res {
    let (ys, ts) = pairs(&a[0], &a[1], name)?;
    // a flat list: every entry is an observation, except for the entropy of one row
    let flat = !nested(&a[0]);
    let count = ys.iter().map(Vec::len).sum::<usize>() as i64;
    let observations = match (flat, name) {
        (true, "crossentropy") => 1,
        (true, _) => count,
        (false, _) => ys.len() as i64,
    };
    let each = |f: &dyn Fn(&Expr, &Expr) -> Expr| -> Expr {
        total(
            ys.iter()
                .zip(&ts)
                .flat_map(|(yr, tr)| yr.iter().zip(tr).map(|(y, t)| f(y, t)).collect::<Vec<_>>()),
        )
    };
    let square = |y: &Expr, t: &Expr| powi(sub(y.clone(), t.clone()), 2);
    let size = |y: &Expr, t: &Expr| abs(sub(y.clone(), t.clone()));
    Ok(simplify(&match name {
        "mse" => div(each(&square), int(count)),
        "sse" => each(&square),
        "mae" => div(each(&size), int(count)),
        "l2loss" => div(each(&square), int(observations)),
        "l1loss" => div(each(&size), int(observations)),
        "crossentropy" => neg(div(
            each(&|y, t| match side(t) {
                Some(Ordering::Equal) => int(0),
                _ => mul(vec![t.clone(), func("ln", vec![y.clone()])]),
            }),
            int(observations),
        )),
        "huber" => {
            let edge = match a.get(2) {
                Some(e) => e.clone(),
                None => int(1),
            };
            let mut sum = Vec::new();
            for (yr, tr) in ys.iter().zip(&ts) {
                for (y, t) in yr.iter().zip(tr) {
                    let off = simplify(&size(y, t));
                    sum.push(
                        match order(&off, &edge)
                            .map_err(|_| format!("{name}: the values must be real numbers"))?
                        {
                            Ordering::Greater => {
                                mul(vec![edge.clone(), sub(off, half(edge.clone()))])
                            }
                            _ => half(powi(off, 2)),
                        },
                    );
                }
            }
            div(total(sum), int(observations))
        }
        _ => return Err(format!("{name} is no loss")),
    }))
}

fn labels_of(e: &Expr) -> Vec<Expr> {
    match e.as_list() {
        Some(items) => items.to_vec(),
        None => vec![e.clone()],
    }
}

/// The values of a list without the ones that come again, in rising order (texts by letters).
fn classes_of(items: &[Expr]) -> Vec<Expr> {
    let mut all: Vec<Expr> = Vec::new();
    for item in items {
        if !all.contains(item) {
            all.push(item.clone());
        }
    }
    all.sort_by(crate::arrays::order_any);
    all
}

fn place(classes: &[Expr], label: &Expr, who: &str) -> Result<usize, String> {
    classes
        .iter()
        .position(|c| crate::arrays::same(c, label))
        .ok_or_else(|| {
            format!(
                "{who}: {label} is none of the classes {}",
                Expr::list(classes.to_vec())
            )
        })
}

fn count(e: &Expr, what: &str, lo: i64, hi: i64) -> Result<usize, String> {
    whole(e, what, lo, hi).map(|k| k as usize)
}

/// What follows the values of a command: "name", value, "name", value, ...
fn choices<'a>(
    rest: &'a [Expr],
    names: &[&str],
    who: &str,
) -> Result<Vec<(String, &'a Expr)>, String> {
    let mut out = Vec::new();
    let mut k = 0;
    while k < rest.len() {
        let name = text(&rest[k])
            .filter(|w| names.contains(&w.as_str()))
            .ok_or_else(|| {
                format!(
                    "{who}: {} is not a choice of this command: {}",
                    rest[k],
                    names
                        .iter()
                        .map(|n| format!("\"{n}\""))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
        let value = rest
            .get(k + 1)
            .ok_or_else(|| format!("{who}: a value must follow \"{name}\""))?;
        out.push((name, value));
        k += 2;
    }
    Ok(out)
}

/// The sum of the products of a window with what it lies on.
fn window_sum(x: &[Vec<Expr>], w: &[Vec<Expr>], top: usize, left: usize) -> Expr {
    let mut terms = Vec::new();
    for (i, row) in w.iter().enumerate() {
        for (j, weight) in row.iter().enumerate() {
            terms.push(mul(vec![weight.clone(), x[top + i][left + j].clone()]));
        }
    }
    total(terms)
}

/// Zeros around a matrix.
fn padded(x: &[Vec<Expr>], before: [usize; 2], after: [usize; 2]) -> Vec<Vec<Expr>> {
    let width = x.first().map_or(0, Vec::len) + before[1] + after[1];
    let mut out = vec![vec![int(0); width]; before[0]];
    for row in x {
        let mut made = vec![int(0); before[1]];
        made.extend(row.iter().cloned());
        made.extend(vec![int(0); after[1]]);
        out.push(made);
    }
    out.extend(vec![vec![int(0); width]; after[0]]);
    out
}

/// Two numbers of a choice: one for both directions, or [down, across].
fn two(e: &Expr, what: &str, lo: i64) -> Result<[usize; 2], String> {
    match e.as_list() {
        Some([a, b]) => Ok([count(a, what, lo, 100_000)?, count(b, what, lo, 100_000)?]),
        Some(_) => Err(format!("{what} is a number or [down, across], not {e}")),
        None => {
            let k = count(e, what, lo, 100_000)?;
            Ok([k, k])
        }
    }
}

/// The operations of layers that work with exact numbers.
fn layer(name: &str, a: &[Expr]) -> Res {
    let n = a.len();
    match name {
        "fullyconnect" => {
            let w = rows(&a[1]);
            let width = even(&w, name)?;
            let bias: Vec<Expr> = match a.get(2) {
                Some(b) => labels_of(b),
                None => vec![int(0); w.len()],
            };
            if bias.len() != w.len() {
                return Err(format!(
                    "{name}: as many biases as rows of the weights, not {} and {}",
                    bias.len(),
                    w.len()
                ));
            }
            let one = |x: &[Expr]| -> Res {
                if x.len() != width {
                    return Err(format!(
                        "{name}: the weights take {width} values, not {}",
                        x.len()
                    ));
                }
                Ok(Expr::list(
                    w.iter()
                        .zip(&bias)
                        .map(|(row, b)| {
                            simplify(&add(vec![
                                total(
                                    row.iter()
                                        .zip(x)
                                        .map(|(p, q)| mul(vec![p.clone(), q.clone()])),
                                ),
                                b.clone(),
                            ]))
                        })
                        .collect(),
                ))
            };
            match nested(&a[0]) {
                true => Ok(Expr::list(
                    rows(&a[0])
                        .iter()
                        .map(|x| one(x))
                        .collect::<Result<_, _>>()?,
                )),
                false => one(&labels_of(&a[0])),
            }
        }
        "dlconv" | "maxpool" | "avgpool" => {
            let flat = !nested(&a[0]);
            let x = rows(&a[0]);
            even(&x, name)?;
            let (window, rest): (Vec<Vec<Expr>>, &[Expr]) = match name {
                "dlconv" => {
                    let w = rows(&a[1]);
                    even(&w, name)?;
                    (w, &a[2..])
                }
                _ => {
                    let size = two(&a[1], "the size of the pool", 1)?;
                    let size = match flat {
                        true => [1, size[1]],
                        false => size,
                    };
                    (vec![vec![int(1); size[1]]; size[0]], &a[2..])
                }
            };
            // a bias may come before the choices
            let (bias, rest) = match rest.first() {
                Some(b) if name == "dlconv" && text(b).is_none() => (b.clone(), &rest[1..]),
                _ => (int(0), rest),
            };
            let (mut stride, mut before, mut after) = ([1, 1], [0, 0], [0, 0]);
            let size = [window.len(), window[0].len()];
            let mut same = false;
            for (key, value) in choices(rest, &["stride", "padding"], name)? {
                match key.as_str() {
                    "stride" => stride = two(value, "the stride", 1)?,
                    _ if text(value).as_deref() == Some("same") => same = true,
                    _ => {
                        before = two(value, "the padding", 0)?;
                        after = before;
                    }
                }
            }
            if flat {
                stride[0] = 1;
                before[0] = 0;
                after[0] = 0;
            }
            if same {
                // as many places as the data has, over the stride
                for d in 0..2 {
                    let length = match d {
                        0 => x.len(),
                        _ => x[0].len(),
                    };
                    let wanted = length.div_ceil(stride[d]);
                    let all = ((wanted - 1) * stride[d] + size[d]).saturating_sub(length);
                    before[d] = all / 2;
                    after[d] = all - all / 2;
                }
            }
            let x = padded(&x, before, after);
            if x.len() < size[0] || x[0].len() < size[1] {
                return Err(format!(
                    "{name}: the window of {} by {} is larger than the data",
                    size[0], size[1]
                ));
            }
            let mut out = Vec::new();
            for top in (0..=x.len() - size[0]).step_by(stride[0]) {
                let mut row = Vec::new();
                for left in (0..=x[0].len() - size[1]).step_by(stride[1]) {
                    row.push(match name {
                        "dlconv" => {
                            simplify(&add(vec![window_sum(&x, &window, top, left), bias.clone()]))
                        }
                        "avgpool" => simplify(&div(
                            window_sum(&x, &window, top, left),
                            int((size[0] * size[1]) as i64),
                        )),
                        _ => {
                            let cells: Vec<Expr> = (0..size[0])
                                .flat_map(|i| (0..size[1]).map(move |j| (i, j)))
                                .map(|(i, j)| x[top + i][left + j].clone())
                                .collect();
                            cells[first_largest(&cells, name)?].clone()
                        }
                    });
                }
                out.push(row);
            }
            Ok(match flat {
                true => Expr::list(out.remove(0)),
                false => matrix(out),
            })
        }
        "embed" => {
            let table = rows(&a[1]);
            let one = |k: &Expr| -> Res {
                Ok(Expr::list(
                    table[count(
                        k,
                        "the number of a row of the weights",
                        1,
                        table.len() as i64,
                    )? - 1]
                        .clone(),
                ))
            };
            match a[0].as_list() {
                Some(items) => Ok(Expr::list(items.iter().map(one).collect::<Result<_, _>>()?)),
                None => one(&a[0]),
            }
        }
        "onehotencode" | "ind2vec" => {
            let labels = labels_of(&a[0]);
            let classes = match (name, a.get(1)) {
                ("ind2vec", given) => {
                    let most = labels
                        .iter()
                        .map(|l| count(l, "an index", 1, 1_000_000))
                        .collect::<Result<Vec<_>, _>>()?
                        .into_iter()
                        .max()
                        .unwrap_or(0);
                    let most = match given {
                        Some(e) => count(e, "the number of rows", most as i64, 1_000_000)?,
                        None => most,
                    };
                    (1..=most as i64).map(int).collect()
                }
                (_, Some(e)) => labels_of(e),
                (_, None) => classes_of(&labels),
            };
            let mut out = Vec::new();
            for label in &labels {
                let at = place(&classes, label, name)?;
                out.push(
                    (0..classes.len())
                        .map(|k| int(i64::from(k == at)))
                        .collect::<Vec<_>>(),
                );
            }
            Ok(match name {
                // a column for every index, as MATLAB has it
                "ind2vec" => matrix(
                    (0..classes.len())
                        .map(|c| out.iter().map(|row| row[c].clone()).collect())
                        .collect(),
                ),
                _ if a[0].as_list().is_none() => Expr::list(out.remove(0)),
                _ => matrix(out),
            })
        }
        "onehotdecode" => {
            let scores = rows(&a[0]);
            let width = even(&scores, name)?;
            let classes = match a.get(1) {
                Some(e) => labels_of(e),
                None => (1..=width as i64).map(int).collect(),
            };
            if classes.len() != width {
                return Err(format!(
                    "{name}: {width} scores in a row but {} classes",
                    classes.len()
                ));
            }
            let found = scores
                .iter()
                .map(|row| first_largest(row, name).map(|k| classes[k].clone()))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(match nested(&a[0]) {
                true => Expr::list(found),
                false => found[0].clone(),
            })
        }
        "vec2ind" => {
            let m = rows(&a[0]);
            let width = even(&m, name)?;
            let mut out = Vec::new();
            for c in 0..width {
                let column: Vec<Expr> = m.iter().map(|row| row[c].clone()).collect();
                out.push(int(first_largest(&column, name)? as i64 + 1));
            }
            Ok(Expr::list(out))
        }
        "compet" => {
            let one = |items: &[Expr]| -> Res {
                let at = first_largest(items, name)?;
                Ok(Expr::list(
                    (0..items.len()).map(|k| int(i64::from(k == at))).collect(),
                ))
            };
            match nested(&a[0]) {
                true => Ok(Expr::list(
                    rows(&a[0])
                        .iter()
                        .map(|r| one(r))
                        .collect::<Result<_, _>>()?,
                )),
                false => one(&labels_of(&a[0])),
            }
        }
        "softmax" => match nested(&a[0]) {
            true => Ok(matrix(
                rows(&a[0])
                    .iter()
                    .map(|r| softmax(r, name))
                    .collect::<Result<_, _>>()?,
            )),
            false => Ok(Expr::list(softmax(&labels_of(&a[0]), name)?)),
        },
        "confusionmat" => {
            let (known, guessed) = (labels_of(&a[0]), labels_of(&a[1]));
            if known.len() != guessed.len() {
                return Err(format!(
                    "{name}: as many known classes as guessed ones, not {} and {}",
                    known.len(),
                    guessed.len()
                ));
            }
            let classes = match a.get(2) {
                Some(e) => labels_of(e),
                None => classes_of(&[known.clone(), guessed.clone()].concat()),
            };
            let mut counts = vec![vec![0i64; classes.len()]; classes.len()];
            for (k, g) in known.iter().zip(&guessed) {
                counts[place(&classes, k, name)?][place(&classes, g, name)?] += 1;
            }
            Ok(matrix(
                counts
                    .into_iter()
                    .map(|row| row.into_iter().map(int).collect())
                    .collect(),
            ))
        }
        "mapminmax" | "mapstd" => {
            let (lo, hi) = match (name, n) {
                ("mapminmax", 1) => (int(-1), int(1)),
                ("mapstd", 1) => (int(0), int(1)),
                (_, 3) => (a[1].clone(), a[2].clone()),
                _ => return Err(format!("{name}(x) or {name}(x, a, b)")),
            };
            let one = |row: &[Expr]| -> Res {
                if row.is_empty() {
                    return Ok(Expr::list(Vec::new()));
                }
                let count = int(row.len() as i64);
                Ok(Expr::list(match name {
                    "mapminmax" => {
                        let top = row[first_largest(row, name)?].clone();
                        let negated: Vec<Expr> = row.iter().map(|x| neg(x.clone())).collect();
                        let bottom = row[first_largest(&negated, name)?].clone();
                        let range = simplify(&sub(top, bottom.clone()));
                        let range = match side(&range) {
                            Some(Ordering::Equal) => int(1),
                            _ => range,
                        };
                        row.iter()
                            .map(|x| {
                                simplify(&add(vec![
                                    div(
                                        mul(vec![
                                            sub(hi.clone(), lo.clone()),
                                            sub(x.clone(), bottom.clone()),
                                        ]),
                                        range.clone(),
                                    ),
                                    lo.clone(),
                                ]))
                            })
                            .collect()
                    }
                    _ => {
                        let mean = simplify(&div(total(row.iter().cloned()), count));
                        let spread = match row.len() {
                            1 => int(0),
                            k => simplify(&pow(
                                div(
                                    total(
                                        row.iter().map(|x| powi(sub(x.clone(), mean.clone()), 2)),
                                    ),
                                    int(k as i64 - 1),
                                ),
                                Expr::rat(1, 2),
                            )),
                        };
                        let spread = match side(&spread) {
                            Some(Ordering::Equal) => int(1),
                            _ => spread,
                        };
                        row.iter()
                            .map(|x| {
                                simplify(&add(vec![
                                    mul(vec![
                                        div(sub(x.clone(), mean.clone()), spread.clone()),
                                        hi.clone(),
                                    ]),
                                    lo.clone(),
                                ]))
                            })
                            .collect()
                    }
                }))
            };
            match nested(&a[0]) {
                true => Ok(Expr::list(
                    rows(&a[0])
                        .iter()
                        .map(|r| one(r))
                        .collect::<Result<_, _>>()?,
                )),
                false => one(&labels_of(&a[0])),
            }
        }
        "divideblock" => {
            let all = count(&a[0], "the number of observations", 1, 100_000_000)?;
            let shares = match n {
                1 => vec![fl(0.7), fl(0.15), fl(0.15)],
                4 => a[1..]
                    .iter()
                    .map(|e| real(e, name))
                    .collect::<Result<Vec<_>, _>>()?,
                _ => {
                    return Err(format!(
                        "{name}(n) or {name}(n, share to train, share to check, share to test)"
                    ));
                }
            };
            let whole_share = fl(&shares[0] + &shares[1]) + &shares[2];
            if shares.iter().any(|s| *s < 0) || whole_share <= 0 {
                return Err(format!("{name}: the shares are not negative and not all 0"));
            }
            let part = |s: &Float| (fl(s / &whole_share) * all as u32).round().to_f64() as usize;
            let (check, test) = (part(&shares[1]).min(all), part(&shares[2]).min(all));
            let test = test.min(all - check);
            let train = all - check - test;
            let run = |from: usize, to: usize| {
                Expr::list((from + 1..=to).map(|k| int(k as i64)).collect())
            };
            Ok(Expr::list(vec![
                run(0, train),
                run(train, train + check),
                run(train + check, all),
            ]))
        }
        "dlgradient" => {
            let slope = |v: &Expr| -> Res {
                let name_of = v
                    .sym_name()
                    .ok_or_else(|| format!("{name}: {v} is no name of a variable"))?;
                Ok(crate::arrays::map(&a[0], &|y| {
                    simplify(&crate::diff::diff(y, name_of))
                }))
            };
            match a[1].as_list() {
                Some(items) => Ok(Expr::list(
                    items.iter().map(slope).collect::<Result<_, _>>()?,
                )),
                None => slope(&a[1]),
            }
        }
        _ => Err(format!("{name} is no operation of a layer")),
    }
}

/// Zeros in the shape of nested lists.
fn zeros_like(e: &Expr) -> Expr {
    match e.as_list() {
        Some(items) => Expr::list(items.iter().map(zeros_like).collect()),
        None => int(0),
    }
}

/// A function of the entries that lie at the same place of nested lists of one shape.
fn zip(f: &dyn Fn(&[Expr]) -> Expr, parts: &[Expr], who: &str) -> Res {
    match parts[0].as_list() {
        Some(items) => {
            let mut out = Vec::with_capacity(items.len());
            for k in 0..items.len() {
                let next = parts
                    .iter()
                    .map(|p| match p.as_list() {
                        Some(l) if l.len() == items.len() => Ok(l[k].clone()),
                        Some(_) => Err(format!("{who}: the lists must have the same shape")),
                        None => Ok(p.clone()),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                out.push(zip(f, &next, who)?);
            }
            Ok(Expr::list(out))
        }
        None => Ok(simplify(&f(parts))),
    }
}

/// The steps that change what is learned: every entry of nested lists for itself.
fn step(name: &str, a: &[Expr]) -> Res {
    let number = |k: usize, usual: (i64, i64)| -> Expr {
        a.get(k)
            .cloned()
            .unwrap_or_else(|| Expr::rat(usual.0, usual.1))
    };
    // a state that is not there yet is [] : zeros
    let state = |k: usize| -> Expr {
        match a[k].as_list() {
            Some([]) => zeros_like(&a[0]),
            _ => a[k].clone(),
        }
    };
    let root = |x: Expr| pow(x, Expr::rat(1, 2));
    let one = int(1);
    match name {
        "sgdmupdate" => {
            let (rate, momentum) = (number(3, (1, 100)), number(4, (9, 10)));
            let parts = [a[0].clone(), a[1].clone(), state(2)];
            let velocity = |p: &[Expr]| {
                sub(
                    mul(vec![momentum.clone(), p[2].clone()]),
                    mul(vec![rate.clone(), p[1].clone()]),
                )
            };
            let moved = zip(&|p| add(vec![p[0].clone(), velocity(p)]), &parts, name)?;
            Ok(Expr::list(vec![
                moved,
                zip(&|p| velocity(p), &parts, name)?,
            ]))
        }
        "adamupdate" => {
            let turn = whole(&a[4], "the number of the step", 1, 1_000_000_000)?;
            let (rate, b1, b2, eps) = (
                number(5, (1, 1000)),
                number(6, (9, 10)),
                number(7, (999, 1000)),
                number(8, (1, 100_000_000)),
            );
            let mean = |p: &[Expr]| {
                add(vec![
                    mul(vec![b1.clone(), p[2].clone()]),
                    mul(vec![sub(one.clone(), b1.clone()), p[1].clone()]),
                ])
            };
            let square = |p: &[Expr]| {
                add(vec![
                    mul(vec![b2.clone(), p[3].clone()]),
                    mul(vec![sub(one.clone(), b2.clone()), powi(p[1].clone(), 2)]),
                ])
            };
            let scale = div(
                mul(vec![
                    rate.clone(),
                    root(sub(one.clone(), powi(b2.clone(), turn))),
                ]),
                sub(one.clone(), powi(b1.clone(), turn)),
            );
            let parts = [a[0].clone(), a[1].clone(), state(2), state(3)];
            let moved = zip(
                &|p| {
                    sub(
                        p[0].clone(),
                        div(
                            mul(vec![scale.clone(), mean(p)]),
                            add(vec![root(square(p)), eps.clone()]),
                        ),
                    )
                },
                &parts,
                name,
            )?;
            Ok(Expr::list(vec![
                moved,
                zip(&|p| mean(p), &parts, name)?,
                zip(&|p| square(p), &parts, name)?,
            ]))
        }
        "rmspropupdate" => {
            let (rate, decay, eps) = (
                number(3, (1, 1000)),
                number(4, (9, 10)),
                number(5, (1, 100_000_000)),
            );
            let square = |p: &[Expr]| {
                add(vec![
                    mul(vec![decay.clone(), p[2].clone()]),
                    mul(vec![sub(one.clone(), decay.clone()), powi(p[1].clone(), 2)]),
                ])
            };
            let parts = [a[0].clone(), a[1].clone(), state(2)];
            let moved = zip(
                &|p| {
                    sub(
                        p[0].clone(),
                        div(
                            mul(vec![rate.clone(), p[1].clone()]),
                            add(vec![root(square(p)), eps.clone()]),
                        ),
                    )
                },
                &parts,
                name,
            )?;
            Ok(Expr::list(vec![moved, zip(&|p| square(p), &parts, name)?]))
        }
        _ => Err(format!("{name} is no step")),
    }
}

// ------------------------------------------------------------------ networks

/// A layer as it is written: the weights (a row for every unit), the biases, the first
/// name of the function, and the number that the function takes.
pub(crate) struct Written {
    w: Vec<Vec<Expr>>,
    b: Vec<Expr>,
    f: &'static str,
    extra: Option<Expr>,
}

/// A layer of numbers.
#[derive(Clone)]
pub(crate) struct Dense {
    pub(crate) w: Vec<Vec<Float>>,
    pub(crate) b: Vec<Float>,
    pub(crate) f: &'static str,
    pub(crate) extra: Option<Float>,
}

fn layer_of(e: &Expr) -> Option<Written> {
    let parts = e.as_list()?;
    if !(3..=4).contains(&parts.len()) {
        return None;
    }
    let f = unit_name(&text(&parts[2])?)?;
    let w = rows(&parts[0]);
    let b = labels_of(&parts[1]);
    (w.len() == b.len()
        && !w.is_empty()
        && w.iter().all(|r| r.len() == w[0].len() && !r.is_empty()))
    .then(|| Written {
        w,
        b,
        f,
        extra: parts.get(3).cloned(),
    })
}

pub(crate) fn is_net(e: &Expr) -> bool {
    e.as_list()
        .is_some_and(|l| !l.is_empty() && l.iter().all(|x| layer_of(x).is_some()))
}

pub(crate) fn net_of(e: &Expr, who: &str) -> Result<Vec<Written>, String> {
    let wrong = || {
        format!(
            "{who}: a network is a list of layers [weights, biases, \"function\"] with a row of weights for every unit (feedforwardnet makes one); the functions are {}",
            unit_names()
        )
    };
    let layers = e
        .as_list()
        .ok_or_else(wrong)?
        .iter()
        .map(layer_of)
        .collect::<Option<Vec<_>>>()
        .ok_or_else(wrong)?;
    if layers.is_empty() {
        return Err(wrong());
    }
    for k in 1..layers.len() {
        if layers[k].w[0].len() != layers[k - 1].w.len() {
            return Err(format!(
                "{who}: layer {} has {} units but layer {} takes {} values",
                k,
                layers[k - 1].w.len(),
                k + 1,
                layers[k].w[0].len()
            ));
        }
    }
    Ok(layers)
}

pub(crate) fn net_shown(net: &[Dense]) -> Expr {
    Expr::list(
        net.iter()
            .map(|l| {
                let mut parts = vec![
                    matrix(l.w.iter().map(|r| r.iter().map(shown).collect()).collect()),
                    Expr::list(l.b.iter().map(shown).collect()),
                    crate::texts::text(l.f),
                ];
                if let Some(x) = &l.extra {
                    parts.push(shown(x));
                }
                Expr::list(parts)
            })
            .collect(),
    )
}

pub(crate) fn dense(layers: &[Written], who: &str) -> Result<Vec<Dense>, String> {
    layers
        .iter()
        .map(|l| {
            Ok(Dense {
                w: l.w
                    .iter()
                    .map(|r| {
                        r.iter()
                            .map(|x| real(x, who))
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .collect::<Result<_, _>>()?,
                b: l.b.iter().map(|x| real(x, who)).collect::<Result<_, _>>()?,
                f: l.f,
                extra: match l.extra.as_ref().or(usual(l.f).as_ref()) {
                    Some(e) => Some(real(e, who)?),
                    None => None,
                },
            })
        })
        .collect()
}

fn sigmoid(z: &Float) -> Float {
    match *z >= 0 {
        true => (fl(1) + fl(-z).exp()).recip(),
        false => {
            let e = z.clone().exp();
            fl(&e / (fl(1) + &e))
        }
    }
}

fn softplus(z: &Float) -> Float {
    match *z > 0 {
        true => fl(z + (fl(1) + fl(-z).exp()).ln()),
        false => (fl(1) + z.clone().exp()).ln(),
    }
}

/// The function of a unit for a number.
pub(crate) fn value(f: &str, z: &Float, extra: Option<&Float>) -> Float {
    let one = fl(1);
    match f {
        "relu" => z.clone().max(&zero()),
        "leakyrelu" => match *z >= 0 {
            true => z.clone(),
            false => fl(z * extra.expect("a slope")),
        },
        "clippedrelu" => z.clone().max(&zero()).min(extra.expect("a ceiling")),
        "satlin" => z.clone().max(&zero()).min(&one),
        "satlins" => z.clone().max(&fl(-1)).min(&one),
        "elu" => match *z > 0 {
            true => z.clone(),
            false => fl(extra.expect("a scale") * (z.clone().exp() - 1u32)),
        },
        "gelu" => fl(z / 2u32) * (one + fl(z / fl(2).sqrt()).erf()),
        "swish" => fl(z * sigmoid(z)),
        "mish" => fl(z * softplus(z).tanh()),
        "softplus" => softplus(z),
        "softsign" => fl(z / (one + fl(z.abs_ref()))),
        "sigmoid" => sigmoid(z),
        "tanh" => z.clone().tanh(),
        "hardlim" => fl(i32::from(*z >= 0)),
        "hardlims" => fl(if *z >= 0 { 1 } else { -1 }),
        "radbas" => (-fl(z.square_ref())).exp(),
        "tribas" => (one - fl(z.abs_ref())).max(&zero()),
        "netinv" => z.clone().recip(),
        _ => z.clone(),
    }
}

/// The slope of the function of a unit, where it has one to learn by.
pub(crate) fn slope(f: &str, z: &Float, a: &Float, extra: Option<&Float>) -> Option<Float> {
    let one = fl(1);
    Some(match f {
        "relu" => fl(i32::from(*z > 0)),
        "leakyrelu" => match *z >= 0 {
            true => one,
            false => extra.expect("a slope").clone(),
        },
        "clippedrelu" => fl(i32::from(*z > 0 && z < extra.expect("a ceiling"))),
        "satlin" => fl(i32::from(*z > 0 && *z < 1)),
        "satlins" => fl(i32::from(*z > -1 && *z < 1)),
        "elu" => match *z > 0 {
            true => one,
            false => fl(a + extra.expect("a scale")),
        },
        "gelu" => {
            let bell =
                (-fl(z.square_ref()) / 2u32).exp() / (fl(rug::float::Constant::Pi) * 2u32).sqrt();
            (one + fl(z / fl(2).sqrt()).erf()) / 2u32 + fl(z * bell)
        }
        "swish" => {
            let s = sigmoid(z);
            fl(&s * (fl(1) + fl(z * (one - &s))))
        }
        "mish" => {
            let t = softplus(z).tanh();
            fl(&t + fl(z * (one - fl(t.square_ref()))) * sigmoid(z))
        }
        "softplus" => sigmoid(z),
        "softsign" => (one + fl(z.abs_ref())).square().recip(),
        "sigmoid" => fl(a * (one - a)),
        "tanh" => one - fl(a.square_ref()),
        "purelin" => one,
        "radbas" => fl(z * a) * -2,
        "tribas" => match fl(z.abs_ref()) < 1 {
            true => fl(-z.cmp0().map_or(0, |o| o as i32)),
            false => zero(),
        },
        "netinv" => -fl(z.square_ref()).recip(),
        _ => return None,
    })
}

/// What a layer of numbers gives for the values that come in: before and after its function.
fn through(l: &Dense, x: &[Float]) -> (Vec<Float>, Vec<Float>) {
    if l.f == "rbf" {
        // a unit answers to the distance from its row of weights
        let z: Vec<Float> =
            l.w.iter()
                .zip(&l.b)
                .map(|(row, b)| {
                    row.iter()
                        .zip(x)
                        .fold(zero(), |sum, (w, v)| sum + fl(v - w).square())
                        .sqrt()
                        * b
                })
                .collect();
        let a = z.iter().map(|d| (-fl(d.square_ref())).exp()).collect();
        return (z, a);
    }
    let z: Vec<Float> =
        l.w.iter()
            .zip(&l.b)
            .map(|(row, b)| {
                row.iter()
                    .zip(x)
                    .fold(b.clone(), |sum, (w, v)| sum + fl(w * v))
            })
            .collect();
    let a = match l.f {
        "softmax" => soft(&z),
        "compet" => {
            let mut best = 0;
            for k in 1..z.len() {
                if z[k] > z[best] {
                    best = k;
                }
            }
            (0..z.len()).map(|k| fl(i32::from(k == best))).collect()
        }
        f => z.iter().map(|v| value(f, v, l.extra.as_ref())).collect(),
    };
    (z, a)
}

/// The same with exact numbers and names.
fn through_exact(l: &Written, x: &[Expr], who: &str) -> Result<Vec<Expr>, String> {
    if l.f == "rbf" {
        return Err(format!(
            "{who}: a layer of distances works with numbers alone"
        ));
    }
    let z: Vec<Expr> =
        l.w.iter()
            .zip(&l.b)
            .map(|(row, b)| {
                simplify(&add(vec![
                    total(
                        row.iter()
                            .zip(x)
                            .map(|(w, v)| mul(vec![w.clone(), v.clone()])),
                    ),
                    b.clone(),
                ]))
            })
            .collect();
    match l.f {
        "softmax" => softmax(&z, who),
        "compet" => {
            let at = first_largest(&z, who)?;
            Ok((0..z.len()).map(|k| int(i64::from(k == at))).collect())
        }
        f => z.iter().map(|v| unit(f, v, l.extra.as_ref())).collect(),
    }
}

/// The observations of a command: rows; a flat list is one observation, or one number
/// each when a row has one value.
fn observations(
    e: &Expr,
    width: usize,
    who: &str,
    what: &str,
) -> Result<(Vec<Vec<Expr>>, bool), String> {
    let (data, single) = match (nested(e), e.as_list()) {
        (true, _) => (rows(e), false),
        (false, Some(items)) if width == 1 => {
            (items.iter().map(|x| vec![x.clone()]).collect(), false)
        }
        (false, Some(items)) => (vec![items.to_vec()], true),
        (false, None) => (vec![vec![e.clone()]], true),
    };
    match data.iter().all(|r| r.len() == width) {
        true => Ok((data, single)),
        false => Err(format!(
            "{who}: {what} of {width} value(s) in a row are needed"
        )),
    }
}

/// Whether a network and its data stay exact: names, or fractions through functions
/// that keep them.
fn exact(layers: &[Written], data: &[Vec<Expr>]) -> bool {
    let all = || {
        layers
            .iter()
            .flat_map(|l| l.w.iter().flatten().chain(l.b.iter()))
            .chain(data.iter().flatten())
    };
    if layers.iter().any(|l| l.f == "rbf") {
        return false;
    }
    if all().any(|x| numeric::real_value(x, 20).is_none()) {
        return true;
    }
    let plain = layers.iter().all(|l| {
        matches!(
            l.f,
            "relu"
                | "leakyrelu"
                | "clippedrelu"
                | "satlin"
                | "satlins"
                | "hardlim"
                | "hardlims"
                | "tribas"
                | "softsign"
                | "netinv"
                | "purelin"
                | "compet"
        )
    });
    plain && !all().any(|x| x.has_float())
}

/// What a network gives for observations.
fn outputs(layers: &[Written], data: &[Vec<Expr>], who: &str) -> Result<Vec<Vec<Expr>>, String> {
    if exact(layers, data) {
        return data
            .iter()
            .map(|x| {
                layers
                    .iter()
                    .try_fold(x.clone(), |values, l| through_exact(l, &values, who))
            })
            .collect();
    }
    let net = dense(layers, who)?;
    data.iter()
        .map(|x| {
            let x = x
                .iter()
                .map(|v| real(v, who))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(net
                .iter()
                .fold(x, |values, l| through(l, &values).1)
                .iter()
                .map(shown)
                .collect())
        })
        .collect()
}

/// Rows as they were given: one observation as a list (one number alone as itself),
/// rows of one value as a flat list.
fn formed(mut out: Vec<Vec<Expr>>, single: bool) -> Expr {
    match (single, out.first().map_or(0, Vec::len)) {
        (true, 1) => out.remove(0).remove(0),
        (true, _) => Expr::list(out.remove(0)),
        (false, 1) => Expr::list(out.into_iter().map(|mut r| r.remove(0)).collect()),
        (false, _) => matrix(out),
    }
}

/// The loss of a network of numbers on data, and its slopes by every weight and bias
/// (layer by layer: the rows of the weights, then the biases).
fn slopes(
    net: &[Dense],
    xs: &[Vec<Float>],
    ts: &[Vec<Float>],
    entropy: bool,
    l2: &Float,
    who: &str,
) -> Result<(Float, Vec<Float>), String> {
    let last = net.len() - 1;
    let outs = net[last].w.len();
    let per = match entropy {
        true => xs.len() as u32,
        false => (xs.len() * outs) as u32,
    };
    let mut loss = zero();
    let mut grads: Vec<(Vec<Vec<Float>>, Vec<Float>)> = net
        .iter()
        .map(|l| {
            (
                vec![vec![zero(); l.w[0].len()]; l.w.len()],
                vec![zero(); l.b.len()],
            )
        })
        .collect();
    for (x, t) in xs.iter().zip(ts) {
        let mut ins = vec![x.clone()];
        let mut befores = Vec::with_capacity(net.len());
        for l in net {
            let (z, a) = through(l, ins.last().expect("values"));
            befores.push(z);
            ins.push(a);
        }
        let y = &ins[net.len()];
        // the slope of the loss by what comes into the last function
        let mut delta: Vec<Float> = match entropy {
            true => {
                for (yk, tk) in y.iter().zip(t) {
                    if !tk.is_zero() {
                        if *yk <= 0 {
                            return Err(format!("{who}: the entropy needs chances above 0"));
                        }
                        loss -= fl(tk * yk.clone().ln()) / per;
                    }
                }
                y.iter().zip(t).map(|(yk, tk)| fl(yk - tk) / per).collect()
            }
            false => {
                let mut out = Vec::with_capacity(outs);
                for k in 0..outs {
                    let off = fl(&y[k] - &t[k]);
                    loss += fl(off.square_ref()) / per;
                    let s = slope(
                        net[last].f,
                        &befores[last][k],
                        &y[k],
                        net[last].extra.as_ref(),
                    )
                    .ok_or_else(|| {
                        format!(
                            "{who}: the function {} has no slope to learn by",
                            net[last].f
                        )
                    })?;
                    out.push(off * 2u32 / per * s);
                }
                out
            }
        };
        for k in (0..net.len()).rev() {
            for (i, d) in delta.iter().enumerate() {
                for (j, v) in ins[k].iter().enumerate() {
                    grads[k].0[i][j] += fl(d * v);
                }
                grads[k].1[i] += d;
            }
            if k > 0 {
                let mut next = Vec::with_capacity(net[k].w[0].len());
                for j in 0..net[k].w[0].len() {
                    let back = delta
                        .iter()
                        .enumerate()
                        .fold(zero(), |sum, (i, d)| sum + fl(d * &net[k].w[i][j]));
                    let s = slope(
                        net[k - 1].f,
                        &befores[k - 1][j],
                        &ins[k][j],
                        net[k - 1].extra.as_ref(),
                    )
                    .ok_or_else(|| {
                        format!(
                            "{who}: the function {} has no slope to learn by",
                            net[k - 1].f
                        )
                    })?;
                    next.push(back * s);
                }
                delta = next;
            }
        }
    }
    let mut flat = Vec::new();
    for (l, (gw, gb)) in net.iter().zip(grads) {
        for (row, grow) in l.w.iter().zip(gw) {
            for (w, g) in row.iter().zip(grow) {
                loss += fl(l2 * fl(w.square_ref())) / 2u32;
                flat.push(g + fl(l2 * w));
            }
        }
        flat.extend(gb);
    }
    Ok((loss, flat))
}

fn flat_of(net: &[Dense]) -> Vec<Float> {
    net.iter()
        .flat_map(|l| {
            l.w.iter()
                .flatten()
                .chain(l.b.iter())
                .cloned()
                .collect::<Vec<_>>()
        })
        .collect()
}

fn put(net: &mut [Dense], theta: &[Float]) {
    let mut at = theta.iter();
    for l in net.iter_mut() {
        for w in l.w.iter_mut().flatten() {
            *w = at.next().expect("a weight").clone();
        }
        for b in l.b.iter_mut() {
            *b = at.next().expect("a bias").clone();
        }
    }
}

fn dot(a: &[Float], b: &[Float]) -> Float {
    a.iter().zip(b).fold(zero(), |sum, (x, y)| sum + fl(x * y))
}

/// What a training found: the loss, the steps, and the largest slope that is left.
struct Trained {
    loss: Float,
    steps: usize,
    slope: Float,
}

/// The weights and biases that make the loss least: the steps of Broyden, Fletcher,
/// Goldfarb and Shanno, each as long as it makes the loss fall enough.
#[allow(clippy::too_many_arguments)]
fn train(
    net: &mut Vec<Dense>,
    xs: &[Vec<Float>],
    ts: &[Vec<Float>],
    entropy: bool,
    l2: &Float,
    most: usize,
    tolerance: &Float,
    who: &str,
) -> Result<Trained, String> {
    let mut theta = flat_of(net);
    let size = theta.len();
    let at = |theta: &[Float], net: &mut Vec<Dense>| -> Result<(Float, Vec<Float>), String> {
        put(net, theta);
        slopes(net, xs, ts, entropy, l2, who)
    };
    let (mut f, mut g) = at(&theta, net)?;
    let unit_matrix = || -> Vec<Vec<Float>> {
        (0..size)
            .map(|i| (0..size).map(|j| fl(i32::from(i == j))).collect())
            .collect()
    };
    let mut h = unit_matrix();
    let mut plain = true;
    let mut steps = 0;
    let largest = |g: &[Float]| g.iter().fold(zero(), |m, x| m.max(&fl(x.abs_ref())));
    while steps < most {
        if largest(&g) <= fl(tolerance * (fl(1) + fl(f.abs_ref()))) {
            break;
        }
        let mut d: Vec<Float> = h.iter().map(|row| -dot(row, &g)).collect();
        let mut down = dot(&g, &d);
        if down >= 0 {
            h = unit_matrix();
            plain = true;
            d = g.iter().map(|x| fl(-x)).collect();
            down = dot(&g, &d);
        }
        let mut t = match plain {
            true => dot(&g, &g).sqrt().recip().min(&fl(1)),
            false => fl(1),
        };
        let try_at = |t: &Float, net: &mut Vec<Dense>| -> Option<(Vec<Float>, Float, Vec<Float>)> {
            let trial: Vec<Float> = theta
                .iter()
                .zip(&d)
                .map(|(x, p)| fl(x + fl(t * p)))
                .collect();
            match at(&trial, net) {
                Ok((value, slope))
                    if value.is_finite() && value <= fl(&f + fl(t * &down) / 10000u32) =>
                {
                    Some((trial, value, slope))
                }
                _ => None,
            }
        };
        // shorter steps until the loss falls enough, or longer ones while it still falls steeply
        let mut found = try_at(&t, net);
        match &found {
            None => {
                for _ in 0..100 {
                    t /= 2u32;
                    found = try_at(&t, net);
                    if found.is_some() {
                        break;
                    }
                }
            }
            Some(_) => {
                for _ in 0..40 {
                    let steep = found
                        .as_ref()
                        .is_some_and(|(_, _, slope)| dot(slope, &d) < fl(&down * 9u32) / 10u32);
                    if !steep {
                        break;
                    }
                    let longer = fl(&t * 2u32);
                    match try_at(&longer, net) {
                        Some(next) => {
                            found = Some(next);
                            t = longer;
                        }
                        None => break,
                    }
                }
            }
        }
        let Some((trial, value, slope)) = found else {
            if plain {
                break;
            }
            // no step along this direction: start again along the slope
            h = unit_matrix();
            plain = true;
            continue;
        };
        let s: Vec<Float> = trial.iter().zip(&theta).map(|(a, b)| fl(a - b)).collect();
        let y: Vec<Float> = slope.iter().zip(&g).map(|(a, b)| fl(a - b)).collect();
        let sy = dot(&s, &y);
        if sy > 0 {
            if plain {
                // the size of the first guess of the curvature
                let scale = fl(&sy / dot(&y, &y));
                for (i, row) in h.iter_mut().enumerate() {
                    for (j, x) in row.iter_mut().enumerate() {
                        *x = match i == j {
                            true => scale.clone(),
                            false => zero(),
                        };
                    }
                }
                plain = false;
            }
            let hy: Vec<Float> = h.iter().map(|row| dot(row, &y)).collect();
            let factor = fl(&sy + dot(&y, &hy)) / fl(sy.square_ref());
            for i in 0..size {
                for j in 0..size {
                    let change = fl(&factor * fl(&s[i] * &s[j]))
                        - (fl(&hy[i] * &s[j]) + fl(&s[i] * &hy[j])) / &sy;
                    h[i][j] += change;
                }
            }
        }
        let still = largest(&s).is_zero();
        theta = trial;
        f = value;
        g = slope;
        steps += 1;
        if still {
            break;
        }
    }
    put(net, &theta);
    Ok(Trained {
        loss: f,
        steps,
        slope: largest(&g),
    })
}

/// Numbers between -1 and 1 that are the same on every machine.
pub(crate) struct Shuffle(pub(crate) u64);

impl Shuffle {
    pub(crate) fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
    }
}

/// The targets of a training as rows of numbers: for the entropy also the numbers of
/// the classes.
fn targets(
    e: &Expr,
    outs: usize,
    count: usize,
    entropy: bool,
    who: &str,
) -> Result<Vec<Vec<Float>>, String> {
    let items = e
        .as_list()
        .ok_or_else(|| format!("{who}: the targets are a list"))?;
    if entropy && !nested(e) && outs > 1 && items.len() == count {
        return items
            .iter()
            .map(|k| {
                let class = whole(k, "the class of an observation", 1, outs as i64)? as usize;
                Ok((1..=outs).map(|c| fl(i32::from(c == class))).collect())
            })
            .collect();
    }
    let (data, _) = observations(e, outs, who, "targets")?;
    if data.len() != count {
        return Err(format!(
            "{who}: {count} observations but {} targets",
            data.len()
        ));
    }
    data.iter()
        .map(|r| r.iter().map(|x| real(x, who)).collect())
        .collect()
}

/// The commands of networks.
fn network(name: &str, a: &[Expr]) -> Res {
    let n = a.len();
    match name {
        "feedforwardnet" | "patternnet" => {
            let sizes = labels_of(&a[0])
                .iter()
                .map(|e| count(e, "the number of units of a layer", 1, 10_000))
                .collect::<Result<Vec<_>, _>>()?;
            if sizes.len() < 2 {
                return Err(format!(
                    "{name}([values that come in, units of every layer ..., values that go out])"
                ));
            }
            let mut inner = "tanh";
            let mut seed = 0u64;
            for e in &a[1..] {
                match text(e) {
                    Some(w) => inner = unit_name(&w)
                        .filter(|f| !matches!(*f, "softmax" | "compet" | "rbf"))
                        .ok_or_else(|| {
                            format!(
                                "{name}: the function of the inner layers is one of {}; not {e}",
                                unit_names()
                            )
                        })?,
                    None => seed = whole(e, "the seed", 0, i64::MAX)? as u64,
                }
            }
            let mut shuffle = Shuffle(
                seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
                    .wrapping_add(0x2545_F491_4F6C_DD1D),
            );
            shuffle.next();
            let mut net = Vec::new();
            for k in 1..sizes.len() {
                let (ins, outs) = (sizes[k - 1], sizes[k]);
                let reach = (6.0 / (ins + outs) as f64).sqrt();
                let w = (0..outs)
                    .map(|_| {
                        (0..ins)
                            .map(|_| fl((shuffle.next() * reach * 1e6).round()) / 1_000_000u32)
                            .collect()
                    })
                    .collect();
                net.push(Dense {
                    w,
                    b: vec![zero(); outs],
                    f: match (k == sizes.len() - 1, name) {
                        (true, "patternnet") => "softmax",
                        (true, _) => "purelin",
                        (false, _) => inner,
                    },
                    extra: None,
                });
            }
            Ok(net_shown(&net))
        }
        "predict" | "classify" | "perform" => {
            let layers = net_of(&a[0], name)?;
            let (data, single) = observations(&a[1], layers[0].w[0].len(), name, "observations")?;
            let out = outputs(&layers, &data, name)?;
            match name {
                "predict" => Ok(formed(out, single)),
                "classify" => {
                    let classes: Vec<Expr> = match a.get(2) {
                        Some(e) => labels_of(e),
                        None => (1..=out.first().map_or(0, Vec::len) as i64)
                            .map(int)
                            .collect(),
                    };
                    let found = out
                        .iter()
                        .map(|row| match (row.len(), classes.len()) {
                            (width, k) if width == k => {
                                first_largest(row, name).map(|at| classes[at].clone())
                            }
                            // one chance: the second class from a half on
                            (1, 2) => Ok(match order(&row[0], &Expr::rat(1, 2))? {
                                Ordering::Less => classes[0].clone(),
                                _ => classes[1].clone(),
                            }),
                            (width, k) => Err(format!(
                                "{name}: {width} values go out but {k} classes are given"
                            )),
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(match single {
                        true => found[0].clone(),
                        false => Expr::list(found),
                    })
                }
                _ => {
                    let kind = match a.get(3) {
                        Some(e) => text(e).filter(|w| matches!(w.as_str(), "mse" | "sse" | "mae" | "crossentropy")).ok_or_else(|| {
                            format!("{name}: the loss is \"mse\", \"sse\", \"mae\" or \"crossentropy\", not {e}")
                        })?,
                        None if layers[layers.len() - 1].f == "softmax" => "crossentropy".to_string(),
                        None => "mse".to_string(),
                    };
                    let outs = out.first().map_or(0, Vec::len);
                    let wanted: Vec<Vec<Expr>> =
                        match targets(&a[2], outs, out.len(), kind == "crossentropy", name) {
                            Ok(t) if kind == "crossentropy" && !nested(&a[2]) && outs > 1 => {
                                t.iter().map(|r| r.iter().map(shown).collect()).collect()
                            }
                            Ok(_) => observations(&a[2], outs, name, "targets")?.0,
                            Err(e) => return Err(e),
                        };
                    loss(&kind, &[matrix(out), matrix(wanted)])
                }
            }
        }
        "trainnet" => {
            // the network may come first or after the data and the targets
            let (net_at, x_at, t_at) = match (is_net(&a[0]), n >= 3 && is_net(&a[2])) {
                (true, _) => (0, 1, 2),
                (false, true) => (2, 0, 1),
                _ => {
                    return Err(net_of(&a[0], name)
                        .err()
                        .unwrap_or_else(|| format!("{name}(network, observations, targets)")));
                }
            };
            if n < 3 {
                return Err(format!(
                    "{name}(network, observations, targets[, \"mse\" | \"crossentropy\"][, \"epochs\", n][, \"l2\", x][, \"tolerance\", x][, \"report\"])"
                ));
            }
            let layers = net_of(&a[net_at], name)?;
            let mut net = dense(&layers, name)?;
            let last = net[net.len() - 1].f;
            let (mut entropy, mut most, mut report) = (last == "softmax", 1000usize, false);
            let mut l2: Option<Float> = None;
            let mut tolerance = Float::with_val(
                bits(),
                Float::i_pow_u(10, numeric::digits().saturating_sub(12).max(6)),
            )
            .recip();
            let mut k = 3;
            while k < n {
                let w = text(&a[k])
                    .ok_or_else(|| format!("{name}: {} is not a choice of the training", a[k]))?;
                match w.as_str() {
                    "mse" => entropy = false,
                    "crossentropy" => entropy = true,
                    "report" => report = true,
                    "epochs" | "maxepochs" | "iterations" | "l2" | "l2regularization"
                    | "tolerance" => {
                        let value = a
                            .get(k + 1)
                            .ok_or_else(|| format!("{name}: a value must follow \"{w}\""))?;
                        match w.as_str() {
                            "epochs" | "maxepochs" | "iterations" => {
                                most = count(value, "the number of steps", 0, 10_000_000)?
                            }
                            "tolerance" => tolerance = real(value, name)?,
                            _ => l2 = Some(real(value, name)?),
                        }
                        k += 1;
                    }
                    _ => {
                        return Err(format!(
                            "{name}: {} is not a choice of the training: \"mse\", \"crossentropy\", \"epochs\", n, \"l2\", x, \"tolerance\", x, \"report\"",
                            a[k]
                        ));
                    }
                }
                k += 1;
            }
            if entropy != (last == "softmax") {
                return Err(format!(
                    "{name}: the entropy goes with a last layer of \"softmax\", and \"softmax\" with the entropy"
                ));
            }
            if net[..net.len() - 1]
                .iter()
                .any(|l| matches!(l.f, "softmax" | "compet" | "rbf"))
                || matches!(last, "compet" | "rbf")
            {
                return Err(format!(
                    "{name}: softmax is for the last layer; layers of \"compet\" and of distances are not trained"
                ));
            }
            let l2 = l2.unwrap_or_else(|| match entropy {
                true => fl(1) / 10000u32,
                false => zero(),
            });
            let (data, _) = observations(&a[x_at], net[0].w[0].len(), name, "observations")?;
            let xs: Vec<Vec<Float>> = data
                .iter()
                .map(|r| r.iter().map(|x| real(x, name)).collect())
                .collect::<Result<_, _>>()?;
            let ts = targets(
                &a[t_at],
                net[net.len() - 1].w.len(),
                xs.len(),
                entropy,
                name,
            )?;
            if xs.is_empty() {
                return Err(format!("{name}: no observations"));
            }
            let found = train(&mut net, &xs, &ts, entropy, &l2, most, &tolerance, name)?;
            Ok(match report {
                true => Expr::list(vec![
                    net_shown(&net),
                    shown(&found.loss),
                    int(found.steps as i64),
                    shown(&found.slope),
                ]),
                false => net_shown(&net),
            })
        }
        "perceptron" => {
            let data = rows(&a[0]);
            let width = even(&data, name)?;
            let wanted = match nested(&a[1]) {
                true => rows(&a[1]),
                false => labels_of(&a[1]).into_iter().map(|t| vec![t]).collect(),
            };
            let outs = even(&wanted, name)?;
            if wanted.len() != data.len() || width == 0 || outs == 0 {
                return Err(format!(
                    "{name}: {} observations but {} targets",
                    data.len(),
                    wanted.len()
                ));
            }
            let most = match a.get(2) {
                Some(e) => count(e, "the number of rounds", 1, 1_000_000)?,
                None => 100,
            };
            let mut w = vec![vec![int(0); width]; outs];
            let mut b = vec![int(0); outs];
            for _ in 0..most {
                let mut wrong = false;
                for (x, t) in data.iter().zip(&wanted) {
                    for k in 0..outs {
                        let sum = simplify(&add(vec![
                            total(
                                w[k].iter()
                                    .zip(x)
                                    .map(|(p, q)| mul(vec![p.clone(), q.clone()])),
                            ),
                            b[k].clone(),
                        ]));
                        let out = match side(&sum).ok_or_else(|| {
                            format!("{name}: the observations must be real numbers")
                        })? {
                            Ordering::Less => int(0),
                            _ => int(1),
                        };
                        let off = simplify(&sub(t[k].clone(), out));
                        if side(&off) != Some(Ordering::Equal) {
                            wrong = true;
                            for (weight, v) in w[k].iter_mut().zip(x) {
                                *weight = simplify(&add(vec![
                                    weight.clone(),
                                    mul(vec![off.clone(), v.clone()]),
                                ]));
                            }
                            b[k] = simplify(&add(vec![b[k].clone(), off]));
                        }
                    }
                }
                if !wrong {
                    break;
                }
            }
            Ok(Expr::list(vec![Expr::list(vec![
                matrix(w),
                Expr::list(b),
                crate::texts::text("hardlim"),
            ])]))
        }
        "newlind" => {
            let data = rows(&a[0]);
            let width = even(&data, name)?;
            let wanted = match nested(&a[1]) {
                true => rows(&a[1]),
                false => labels_of(&a[1]).into_iter().map(|t| vec![t]).collect(),
            };
            let outs = even(&wanted, name)?;
            if wanted.len() != data.len() || width == 0 || outs == 0 {
                return Err(format!(
                    "{name}: {} observations but {} targets",
                    data.len(),
                    wanted.len()
                ));
            }
            let with_one: Vec<Vec<Expr>> = data
                .iter()
                .map(|r| {
                    let mut row = r.clone();
                    row.push(int(1));
                    row
                })
                .collect();
            let mut w = Vec::new();
            let mut b = Vec::new();
            for k in 0..outs {
                let column = Expr::list(wanted.iter().map(|r| r[k].clone()).collect());
                let solved = crate::arrays::mldivide(&matrix(with_one.clone()), &column)?;
                let parts = labels_of(&solved);
                if parts.len() != width + 1 {
                    return Err(format!("{name}: the observations do not fix a line"));
                }
                w.push(parts[..width].to_vec());
                b.push(parts[width].clone());
            }
            Ok(Expr::list(vec![Expr::list(vec![
                matrix(w),
                Expr::list(b),
                crate::texts::text("purelin"),
            ])]))
        }
        "newrbe" => {
            let data = rows(&a[0]);
            let width = even(&data, name)?;
            let wanted = match nested(&a[1]) {
                true => rows(&a[1]),
                false => labels_of(&a[1]).into_iter().map(|t| vec![t]).collect(),
            };
            let outs = even(&wanted, name)?;
            if wanted.len() != data.len() || width == 0 || outs == 0 {
                return Err(format!(
                    "{name}: {} observations but {} targets",
                    data.len(),
                    wanted.len()
                ));
            }
            let spread = match a.get(2) {
                Some(e) => real(e, name)?,
                None => fl(1),
            };
            if spread <= 0 {
                return Err(format!("{name}: the spread is more than 0"));
            }
            // half the height at the distance of the spread
            let sharp = fl(2).ln().sqrt() / spread;
            let centres: Vec<Vec<Float>> = data
                .iter()
                .map(|r| r.iter().map(|x| real(x, name)).collect())
                .collect::<Result<_, _>>()?;
            let first = Dense {
                w: centres.clone(),
                b: vec![sharp; centres.len()],
                f: "rbf",
                extra: None,
            };
            let answers: Vec<Vec<Float>> = centres.iter().map(|x| through(&first, x).1).collect();
            let mut w = vec![Vec::new(); outs];
            for (k, row) in w.iter_mut().enumerate() {
                let column = wanted
                    .iter()
                    .map(|r| real(&r[k], name))
                    .collect::<Result<Vec<_>, _>>()?;
                *row = crate::mfun::linear(answers.clone(), column)
                    .ok_or_else(|| format!("{name}: two observations are the same place"))?;
            }
            let second = Dense {
                w,
                b: vec![zero(); outs],
                f: "purelin",
                extra: None,
            };
            Ok(net_shown(&[first, second]))
        }
        _ => Err(format!("{name} is no command of networks")),
    }
}

/// The layers that keep what was before them.
fn memory(name: &str, a: &[Expr]) -> Res {
    let numbers = |e: &Expr| -> Result<Vec<Vec<Float>>, String> {
        rows(e)
            .iter()
            .map(|r| r.iter().map(|x| real(x, name)).collect())
            .collect()
    };
    let list = |e: &Expr| -> Result<Vec<Float>, String> {
        labels_of(e).iter().map(|x| real(x, name)).collect()
    };
    let apply =
        |m: &[Vec<Float>], x: &[Float]| -> Vec<Float> { m.iter().map(|row| dot(row, x)).collect() };
    let show = |m: &[Vec<Float>]| matrix(m.iter().map(|r| r.iter().map(shown).collect()).collect());
    let shown_list = |v: &[Float]| Expr::list(v.iter().map(shown).collect());
    match name {
        "lstm" | "gru" => {
            let gates = match name {
                "lstm" => 4,
                _ => 3,
            };
            let xs = numbers(&a[0])?;
            let mut h = list(&a[1])?;
            let units = h.len();
            let (mut c, rest) = match name {
                "lstm" => (list(&a[2])?, &a[3..]),
                _ => (Vec::new(), &a[2..]),
            };
            let (w, r, b) = (numbers(&rest[0])?, numbers(&rest[1])?, list(&rest[2])?);
            let wide = xs.first().map_or(0, Vec::len);
            let fits = w.len() == gates * units
                && r.len() == gates * units
                && b.len() == gates * units
                && w.iter().all(|row| row.len() == wide)
                && r.iter().all(|row| row.len() == units)
                && xs.iter().all(|row| row.len() == wide)
                && (name == "gru" || c.len() == units)
                && units > 0;
            if !fits {
                return Err(format!(
                    "{name}: with {units} units and {wide} values at a time the weights are {} by {wide}, the weights of what was before {} by {units}, and there are {} biases",
                    gates * units,
                    gates * units,
                    gates * units
                ));
            }
            let mut all = Vec::with_capacity(xs.len());
            for x in &xs {
                let (wx, rh) = (apply(&w, x), apply(&r, &h));
                let part = |gate: usize, k: usize| {
                    fl(&wx[gate * units + k] + &rh[gate * units + k]) + &b[gate * units + k]
                };
                let mut next = Vec::with_capacity(units);
                for k in 0..units {
                    match name {
                        "lstm" => {
                            // the gates: in, forget, what is new, out
                            let (i, f, g, o) = (
                                sigmoid(&part(0, k)),
                                sigmoid(&part(1, k)),
                                part(2, k).tanh(),
                                sigmoid(&part(3, k)),
                            );
                            c[k] = fl(&f * &c[k]) + fl(&i * &g);
                            next.push(o * c[k].clone().tanh());
                        }
                        _ => {
                            // the gates: reset, update, what is new
                            let (reset, update) = (sigmoid(&part(0, k)), sigmoid(&part(1, k)));
                            let new = (fl(&wx[2 * units + k] + &b[2 * units + k])
                                + fl(&reset * &rh[2 * units + k]))
                            .tanh();
                            next.push(fl((fl(1) - &update) * &new) + fl(&update * &h[k]));
                        }
                    }
                }
                h = next;
                all.push(h.clone());
            }
            Ok(match name {
                "lstm" => Expr::list(vec![show(&all), shown_list(&h), shown_list(&c)]),
                _ => Expr::list(vec![show(&all), shown_list(&h)]),
            })
        }
        "batchnorm" | "layernorm" => {
            let flat = !nested(&a[0]);
            let x = match (flat, name) {
                // a flat list: one channel of a batch, or one observation of a layer
                (true, "batchnorm") => list(&a[0])?.into_iter().map(|v| vec![v]).collect(),
                _ => numbers(&a[0])?,
            };
            let width = x.first().map_or(0, Vec::len);
            if x.is_empty() || x.iter().any(|r| r.len() != width) || width == 0 {
                return Err(format!("{name}: rows of the same length are needed"));
            }
            let per = |e: &Expr| -> Result<Vec<Float>, String> {
                let v = list(e)?;
                match v.len() {
                    1 => Ok(vec![v[0].clone(); width]),
                    k if k == width => Ok(v),
                    k => Err(format!(
                        "{name}: {width} values in a row but {k} numbers for them"
                    )),
                }
            };
            let (offset, scale) = (per(&a[1])?, per(&a[2])?);
            let tiny = fl(1) / 100_000u32;
            let moments = |values: &[Float]| -> (Float, Float) {
                let count = values.len() as u32;
                let mean = values.iter().fold(zero(), |s, v| s + v) / count;
                let spread = values
                    .iter()
                    .fold(zero(), |s, v| s + fl(v - &mean).square())
                    / count;
                (mean, spread)
            };
            let mut out = x.clone();
            match name {
                "batchnorm" => {
                    let given = match a.len() {
                        5 => Some((per(&a[3])?, per(&a[4])?)),
                        3 => None,
                        _ => {
                            return Err(format!(
                                "{name}(x, offset, scale) or {name}(x, offset, scale, mean, variance)"
                            ));
                        }
                    };
                    for k in 0..width {
                        let column: Vec<Float> = x.iter().map(|r| r[k].clone()).collect();
                        let (mean, spread) = match &given {
                            Some((m, v)) => (m[k].clone(), v[k].clone()),
                            None => moments(&column),
                        };
                        let root = fl(&spread + &tiny).sqrt();
                        for (row, v) in out.iter_mut().zip(&column) {
                            row[k] = fl(&scale[k] * (fl(v - &mean) / &root)) + &offset[k];
                        }
                    }
                }
                _ => {
                    for (row, values) in out.iter_mut().zip(&x) {
                        let (mean, spread) = moments(values);
                        let root = fl(&spread + &tiny).sqrt();
                        for k in 0..width {
                            row[k] = fl(&scale[k] * (fl(&values[k] - &mean) / &root)) + &offset[k];
                        }
                    }
                }
            }
            Ok(match (flat, name) {
                (true, "batchnorm") => {
                    shown_list(&out.iter().map(|r| r[0].clone()).collect::<Vec<_>>())
                }
                (true, _) => shown_list(&out[0]),
                _ => show(&out),
            })
        }
        _ => Err(format!("{name} is no layer")),
    }
}

/// The commands of networks of neurons.
pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    let n = a.len();
    if let Some(r) = crate::convnet::call(name, a) {
        return Some(r);
    }
    Some(match name {
        "relu" | "leakyrelu" | "clippedrelu" | "elu" | "gelu" | "swish" | "mish" | "softplus"
        | "softsign" | "sigmoid" | "tansig" | "purelin" | "satlin" | "satlins" | "hardlim"
        | "hardlims" | "radbas" | "tribas" | "netinv" => {
            let f = unit_name(name).expect("a function of a unit");
            match (n, usual(f).is_some()) {
                (1, _) | (2, true) => map_result(&a[0], &|x| unit(f, x, a.get(1))),
                _ => Err(format!("{name} takes one value (or a list)")),
            }
        }
        "mse" | "sse" | "mae" | "l1loss" | "l2loss" | "crossentropy" | "huber" => loss(name, a),
        "fullyconnect" | "dlconv" | "maxpool" | "avgpool" | "embed" | "onehotencode"
        | "onehotdecode" | "ind2vec" | "vec2ind" | "compet" | "softmax" | "confusionmat"
        | "mapminmax" | "mapstd" | "divideblock" | "dlgradient" => layer(name, a),
        "sgdmupdate" | "adamupdate" | "rmspropupdate" => step(name, a),
        "feedforwardnet" | "patternnet" | "predict" | "classify" | "perform" | "trainnet"
        | "perceptron" | "newlind" | "newrbe" => network(name, a),
        "lstm" | "gru" | "batchnorm" | "layernorm" => memory(name, a),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn num(text: &str) -> Expr {
        shown(&Float::with_val(
            bits(),
            Float::parse(text).expect("a number"),
        ))
    }

    fn run(name: &str, a: Vec<Expr>) -> Expr {
        call(name, &a)
            .expect("a command of networks")
            .expect("an answer")
    }

    fn list(items: &[i64]) -> Expr {
        Expr::list(items.iter().map(|k| int(*k)).collect())
    }

    fn grid(m: &[&[i64]]) -> Expr {
        Expr::list(m.iter().map(|r| list(r)).collect())
    }

    #[test]
    fn units_are_exact_and_have_slopes() {
        let x = Expr::sym("x");
        assert_eq!(
            run(
                "relu",
                vec![Expr::list(vec![int(-2), Expr::rat(3, 2), x.clone()])]
            )
            .to_string(),
            "[0, 3/2, x/2 + abs(x)/2]"
        );
        assert_eq!(run("sigmoid", vec![int(0)]).to_string(), "1/2");
        assert_eq!(run("tribas", vec![Expr::rat(1, 4)]).to_string(), "3/4");
        assert_eq!(
            run("hardlim", vec![list(&[-1, 0, 2])]).to_string(),
            "[0, 1, 1]"
        );
        assert_eq!(
            run("leakyrelu", vec![int(-4), Expr::rat(1, 2)]).to_string(),
            "-2"
        );
        assert!(
            call("relu", &[func("sqrt", vec![int(-1)])])
                .expect("a command")
                .is_err()
        );
        // the formula of a name agrees with the numbers on both sides of 0
        for name in [
            "relu",
            "leakyrelu",
            "clippedrelu",
            "elu",
            "satlin",
            "satlins",
            "tribas",
        ] {
            let formula = run(name, vec![x.clone()]);
            for at in ["-7.5", "-0.75", "0.25", "0.75", "3.5", "9.25"] {
                let direct = real(&run(name, vec![num(at)]), name).expect("a number");
                let by_formula = real(
                    &simplify(&crate::canon::subst_sym(&formula, "x", &num(at))),
                    name,
                )
                .expect("a number");
                assert!(
                    fl(&direct - &by_formula).abs() < 1e-25,
                    "{name} at {at}: {direct} {by_formula}"
                );
            }
        }
        // the slopes that the training uses are those of the functions
        for name in [
            "relu",
            "leakyrelu",
            "clippedrelu",
            "elu",
            "gelu",
            "swish",
            "mish",
            "softplus",
            "softsign",
            "sigmoid",
            "tanh",
            "radbas",
            "tribas",
            "netinv",
            "satlins",
        ] {
            let extra = usual(name).map(|e| real(&e, name).expect("a number"));
            for at in [-1.75, -0.3, 0.4, 2.25] {
                let (z, h) = (fl(at), fl(1e-20));
                let a = value(name, &z, extra.as_ref());
                let around = (value(name, &fl(&z + &h), extra.as_ref())
                    - value(name, &fl(&z - &h), extra.as_ref()))
                    / fl(&h * 2u32);
                let slope = slope(name, &z, &a, extra.as_ref()).expect("a slope");
                assert!(
                    fl(&slope - &around).abs() < 1e-15,
                    "{name} at {at}: {slope} {around}"
                );
            }
        }
    }

    #[test]
    fn losses_and_layers() {
        assert_eq!(
            run("mse", vec![list(&[1, 2, 3]), list(&[1, 3, 5])]).to_string(),
            "5/3"
        );
        assert_eq!(
            run(
                "crossentropy",
                vec![
                    Expr::list(vec![Expr::rat(1, 2), Expr::rat(1, 4), Expr::rat(1, 4)]),
                    list(&[0, 1, 0])
                ]
            )
            .to_string(),
            "2ln(2)"
        );
        assert_eq!(
            run("huber", vec![list(&[0, 0]), list(&[1, 3])]).to_string(),
            "3/2"
        );
        assert_eq!(
            run("dlconv", vec![list(&[1, 2, 3, 4, 5]), list(&[1, 0, -1])]).to_string(),
            "[-2, -2, -2]"
        );
        let same = run(
            "dlconv",
            vec![
                list(&[1, 2, 3, 4, 5]),
                list(&[1, 1, 1]),
                crate::texts::text("padding"),
                crate::texts::text("same"),
            ],
        );
        assert_eq!(same.to_string(), "[3, 6, 9, 12, 9]");
        assert_eq!(
            run(
                "maxpool",
                vec![grid(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9]]), int(2)]
            )
            .to_string(),
            "[[5, 6], [8, 9]]"
        );
        assert_eq!(
            run("avgpool", vec![list(&[1, 3, 2, 5]), int(2)]).to_string(),
            "[2, 5/2, 7/2]"
        );
        assert_eq!(
            run("onehotencode", vec![list(&[3, 1, 2])]).to_string(),
            "[[0, 0, 1], [1, 0, 0], [0, 1, 0]]"
        );
        assert_eq!(
            run("vec2ind", vec![run("ind2vec", vec![list(&[2, 1, 3])])]).to_string(),
            "[2, 1, 3]"
        );
        assert_eq!(
            run("confusionmat", vec![list(&[1, 2, 2]), list(&[1, 2, 1])]).to_string(),
            "[[1, 0], [1, 1]]"
        );
        assert_eq!(
            run("mapminmax", vec![list(&[1, 2, 5])]).to_string(),
            "[-1, -1/2, 1]"
        );
        assert_eq!(
            run(
                "sgdmupdate",
                vec![list(&[1, 2]), list(&[1, -1]), Expr::list(Vec::new())]
            )
            .to_string(),
            "[[99/100, 201/100], [-1/100, 1/100]]"
        );
        assert_eq!(
            run(
                "dlgradient",
                vec![crate::canon::powi(Expr::sym("x"), 3), Expr::sym("x")]
            )
            .to_string(),
            "3x^2"
        );
    }

    #[test]
    fn a_network_learns() {
        let net = run("feedforwardnet", vec![list(&[2, 3, 1])]);
        assert_eq!(net, run("feedforwardnet", vec![list(&[2, 3, 1]), int(0)]));
        assert_ne!(net, run("feedforwardnet", vec![list(&[2, 3, 1]), int(1)]));
        let (x, t) = (
            grid(&[&[0, 0], &[0, 1], &[1, 0], &[1, 1]]),
            list(&[0, 1, 1, 0]),
        );
        let before = real(
            &run("perform", vec![net.clone(), x.clone(), t.clone()]),
            "test",
        )
        .expect("a loss");
        let taught = run("trainnet", vec![net, x.clone(), t.clone()]);
        let after = real(
            &run("perform", vec![taught.clone(), x.clone(), t.clone()]),
            "test",
        )
        .expect("a loss");
        assert!(after < 1e-30 && before > 0.1, "{before} {after}");
        let out = run("predict", vec![taught, x.clone()]);
        let values: Vec<Float> = out
            .as_list()
            .expect("a list")
            .iter()
            .map(|v| real(v, "test").expect("a number"))
            .collect();
        assert!(
            values
                .iter()
                .zip([0, 1, 1, 0])
                .all(|(v, t)| fl(v - t).abs() < 1e-12)
        );
        // classes: the entropy falls and the classes are found
        let classes = run("patternnet", vec![list(&[2, 3, 2])]);
        let labels = list(&[1, 2, 2, 1]);
        let taught = run("trainnet", vec![classes, x.clone(), labels.clone()]);
        assert_eq!(run("classify", vec![taught, x.clone()]), labels);
        // the rule of the perceptron and the line of least squares are exact
        let and = run("perceptron", vec![x.clone(), list(&[0, 0, 0, 1])]);
        assert_eq!(
            run("predict", vec![and, x.clone()]).to_string(),
            "[0, 0, 0, 1]"
        );
        let line = run("newlind", vec![grid(&[&[1], &[2], &[3]]), list(&[2, 4, 7])]);
        assert_eq!(line.to_string(), "[[[[5/2]], [-2/3], \"purelin\"]]");
        let through = run("newrbe", vec![grid(&[&[0], &[1], &[2]]), list(&[1, 3, 2])]);
        let back = run("predict", vec![through, grid(&[&[0], &[1], &[2]])]);
        let values: Vec<Float> = back
            .as_list()
            .expect("a list")
            .iter()
            .map(|v| real(v, "test").expect("a number"))
            .collect();
        assert!(
            values
                .iter()
                .zip([1, 3, 2])
                .all(|(v, t)| fl(v - t).abs() < 1e-25)
        );
        // a network of names
        let named = Expr::list(vec![Expr::list(vec![
            grid(&[&[1, 2]]),
            list(&[0]),
            crate::texts::text("tanh"),
        ])]);
        assert_eq!(
            run(
                "predict",
                vec![named, Expr::list(vec![Expr::sym("x"), Expr::sym("y")])]
            )
            .to_string(),
            "tanh(x + 2y)"
        );
    }

    #[test]
    fn slopes_of_the_loss_agree_with_differences() {
        let layers = net_of(
            &run(
                "patternnet",
                vec![list(&[2, 3, 2]), crate::texts::text("softplus"), int(3)],
            ),
            "test",
        )
        .expect("a network");
        let net = dense(&layers, "test").expect("numbers");
        let xs = vec![
            vec![fl(0.25), fl(-0.5)],
            vec![fl(1.5), fl(0.75)],
            vec![fl(-1.0), fl(0.5)],
        ];
        let ts = vec![vec![fl(1), fl(0)], vec![fl(0), fl(1)], vec![fl(1), fl(0)]];
        let l2 = fl(0.01);
        let (_, grads) = slopes(&net, &xs, &ts, true, &l2, "test").expect("slopes");
        let theta = flat_of(&net);
        let h = fl(1e-20);
        for k in 0..theta.len() {
            let at = |sign: i32| {
                let mut moved = theta.clone();
                moved[k] += fl(&h * sign);
                let mut copy = net.clone();
                put(&mut copy, &moved);
                slopes(&copy, &xs, &ts, true, &l2, "test")
                    .expect("slopes")
                    .0
            };
            let around = (at(1) - at(-1)) / fl(&h * 2u32);
            assert!(
                fl(&grads[k] - &around).abs() < 1e-15,
                "{k}: {} {around}",
                grads[k]
            );
        }
    }
}
