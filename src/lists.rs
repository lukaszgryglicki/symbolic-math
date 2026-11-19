//! Lists as data: building, picking, combining and counting; the commands that take functions;
//! the program-like forms (if, while, do, table).

use crate::canon::{add, div, mul, sub};
use crate::eval::Session;
use crate::expr::Expr;
use crate::logic::truth;
use crate::matrix;
use crate::numeric;
use crate::simplify::simplify;
use rug::{Float, Integer};
use std::cmp::Ordering;

type Res = Result<Expr, String>;

/// Most entries a command builds, most rounds a loop runs.
const LIMIT: usize = 1_000_000;

fn params_of(e: &Expr) -> Option<Vec<String>> {
    match e {
        Expr::Sym(s) => Some(vec![s.to_string()]),
        Expr::List(items) if !items.is_empty() => items
            .iter()
            .map(|i| i.sym_name().map(str::to_string))
            .collect(),
        _ => None,
    }
}

/// `x -> body` or `(x, y) -> body` as the parameters and the body.
pub fn lambda(e: &Expr) -> Option<(Vec<String>, &Expr)> {
    let a = e.fn_args("rule").filter(|a| a.len() == 2)?;
    params_of(&a[0]).map(|p| (p, &a[1]))
}

const CLASH: &str = "i and j are two names of the imaginary unit, and both are indices here: a loop inside a loop needs another name for its index";

/// Whether a loop inside the expression has the imaginary unit for its index.
fn inner_unit_index(e: &Expr) -> bool {
    let unit = |v: &Expr| *v == Expr::i();
    e.any(&|x| match x {
        Expr::Fn(name, a) if matches!(&**name, "sum" | "product") => a.get(1).is_some_and(unit),
        Expr::Fn(name, a) if matches!(&**name, "table" | "do") => a
            .iter()
            .skip(1)
            .any(|s| unit(s) || s.as_list().and_then(<[Expr]>::first).is_some_and(unit)),
        _ => false,
    })
}

/// `i` written as an index is a variable, not the imaginary unit.
fn index_i(e: &Expr) -> Expr {
    crate::eval::map_raw(e, &mut |x| (*x == Expr::i()).then(|| Expr::sym("i")))
}

/// `a, a + step, ...` up to `b`.
pub fn span(a: &Expr, b: &Expr, step: &Expr) -> Result<Vec<Expr>, String> {
    let too_many = || format!("a range of more than {LIMIT} entries");
    let count = match (a.as_rat(), b.as_rat(), step.as_rat()) {
        (Some(x), Some(y), Some(s)) => {
            if s.cmp0() == Ordering::Equal {
                return Err("the step of a range must not be zero".into());
            }
            let q = crate::num::Rat::from(y - x) / s;
            if q.cmp0() == Ordering::Less {
                0
            } else {
                (q.floor().into_numer_denom().0 + Integer::from(1))
                    .to_usize()
                    .filter(|n| *n <= LIMIT)
                    .ok_or_else(too_many)?
            }
        }
        _ => {
            let q = simplify(&div(sub(b.clone(), a.clone()), step.clone()));
            let v = numeric::real_value(&q, 20)
                .ok_or("the ends and the step of a range must be real numbers")?;
            // a whole number of steps may come out a hair short
            let steps = (v + 1e-12f64).floor();
            if steps.is_sign_negative() && !steps.is_zero() {
                0
            } else {
                (steps.to_integer().ok_or_else(too_many)? + Integer::from(1))
                    .to_usize()
                    .filter(|n| *n <= LIMIT)
                    .ok_or_else(too_many)?
            }
        }
    };
    let exact = a.as_rat().is_some() && step.as_rat().is_some();
    Ok((0..count)
        .map(|k| {
            let v = add(vec![
                a.clone(),
                mul(vec![Expr::int(k as i64), step.clone()]),
            ]);
            if exact { v } else { simplify(&v) }
        })
        .collect())
}

enum Key {
    Value(Float, Float),
    Other,
}

fn key(e: &Expr) -> Key {
    if e.is_inf() || e.is_neg_inf() {
        let inf = Float::with_val(53, rug::float::Special::Infinity);
        return Key::Value(if e.is_inf() { inf } else { -inf }, Float::new(53));
    }
    if matches!(e, Expr::List(_) | Expr::Bool(_) | Expr::Rel(..)) || !e.is_const() {
        return Key::Other;
    }
    match numeric::evalf_digits(e, numeric::digits()) {
        Ok(v) => {
            let (re, im) = v.into_real_imag();
            Key::Value(re, im)
        }
        Err(_) => Key::Other,
    }
}

/// Sorted: real numbers by value, complex numbers by their real and then imaginary parts, the
/// rest after them in the order of the canonical form; lists by their entries.
pub fn sorted(items: &[Expr]) -> Vec<Expr> {
    sorted_by(items, items)
}

/// `items` in the order of their `keys`.
pub fn sorted_by(items: &[Expr], keys: &[Expr]) -> Vec<Expr> {
    let mut order: Vec<usize> = (0..items.len()).collect();
    if keys.iter().all(|k| k.as_rat().is_some()) {
        order.sort_by(|a, b| keys[*a].as_rat().cmp(&keys[*b].as_rat()));
    } else {
        let computed: Vec<Key> = keys.iter().map(key).collect();
        order.sort_by(|a, b| match (&computed[*a], &computed[*b]) {
            (Key::Value(x, u), Key::Value(y, v)) => x.total_cmp(y).then_with(|| u.total_cmp(v)),
            (Key::Value(..), Key::Other) => Ordering::Less,
            (Key::Other, Key::Value(..)) => Ordering::Greater,
            (Key::Other, Key::Other) => compare_other(&keys[*a], &keys[*b]),
        });
    }
    order.into_iter().map(|k| items[k].clone()).collect()
}

fn compare_other(a: &Expr, b: &Expr) -> Ordering {
    match (a.as_list(), b.as_list()) {
        (Some(x), Some(y)) => {
            for (p, q) in x.iter().zip(y) {
                let c = match (key(p), key(q)) {
                    (Key::Value(r, u), Key::Value(s, v)) => {
                        r.total_cmp(&s).then_with(|| u.total_cmp(&v))
                    }
                    (Key::Value(..), Key::Other) => Ordering::Less,
                    (Key::Other, Key::Value(..)) => Ordering::Greater,
                    (Key::Other, Key::Other) => compare_other(p, q),
                };
                if c != Ordering::Equal {
                    return c;
                }
            }
            x.len().cmp(&y.len())
        }
        _ => a.cmp(b),
    }
}

fn unique(items: &[Expr]) -> Vec<Expr> {
    let mut out: Vec<Expr> = Vec::new();
    for x in items {
        if !out.contains(x) {
            out.push(x.clone());
        }
    }
    out
}

/// Position `i` (from 1; negative from the end) as an offset into `len` entries.
fn offset(i: &Expr, len: usize) -> Result<usize, String> {
    let k = i
        .as_i64()
        .ok_or_else(|| format!("a position must be a whole number, {i} is none"))?;
    let at = if k > 0 {
        k - 1
    } else if k < 0 {
        len as i64 + k
    } else {
        -1
    };
    usize::try_from(at)
        .ok()
        .filter(|at| *at < len)
        .ok_or_else(|| format!("there is no position {k} in a list of {len} entries"))
}

fn part(e: &Expr, path: &[Expr]) -> Res {
    let Some((first, rest)) = path.split_first() else {
        return Ok(e.clone());
    };
    let items = e
        .as_list()
        .ok_or_else(|| format!("{e} is not a list: it has no parts"))?;
    match first {
        Expr::List(picks) => picks
            .iter()
            .map(|p| part(&items[offset(p, items.len())?], rest))
            .collect::<Result<Vec<_>, _>>()
            .map(Expr::list),
        Expr::Sym(s) if &**s == "all" => items
            .iter()
            .map(|x| part(x, rest))
            .collect::<Result<Vec<_>, _>>()
            .map(Expr::list),
        one => part(&items[offset(one, items.len())?], rest),
    }
}

/// The first `n` entries, the last `-n`, or the entries from `a` to `b` for `[a, b]`.
fn window(items: &[Expr], spec: &Expr) -> Result<(usize, usize), String> {
    let len = items.len();
    if let Some(pair) = spec.as_list().filter(|p| p.len() == 2) {
        let (a, b) = (offset(&pair[0], len)?, offset(&pair[1], len)?);
        return Ok((a, (b + 1).max(a)));
    }
    let n = spec
        .as_i64()
        .ok_or_else(|| format!("a count must be a whole number, {spec} is none"))?;
    let size = usize::try_from(n.unsigned_abs())
        .ok()
        .filter(|s| *s <= len)
        .ok_or_else(|| format!("the list has {len} entries only"))?;
    Ok(if n >= 0 { (0, size) } else { (len - size, len) })
}

fn total(items: &[Expr]) -> Res {
    if items.is_empty() {
        return Ok(Expr::zero());
    }
    matrix::add(items.to_vec())
}

/// Distinct arrangements of `k` of the entries, in the order of the positions.
fn arrangements(items: &[Expr], k: usize) -> Result<Vec<Expr>, String> {
    let distinct = unique(items);
    let mut left: Vec<usize> = distinct
        .iter()
        .map(|d| items.iter().filter(|x| *x == d).count())
        .collect();
    let mut out = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    fn go(
        distinct: &[Expr],
        left: &mut [usize],
        current: &mut Vec<usize>,
        k: usize,
        out: &mut Vec<Expr>,
    ) -> Result<(), String> {
        if current.len() == k {
            if out.len() >= LIMIT {
                return Err(format!("more than {LIMIT} arrangements"));
            }
            out.push(Expr::list(
                current.iter().map(|i| distinct[*i].clone()).collect(),
            ));
            return Ok(());
        }
        for i in 0..distinct.len() {
            if left[i] > 0 {
                left[i] -= 1;
                current.push(i);
                go(distinct, left, current, k, out)?;
                current.pop();
                left[i] += 1;
            }
        }
        Ok(())
    }
    go(&distinct, &mut left, &mut current, k, &mut out)?;
    Ok(out)
}

/// The sublists of `k` entries, in the order of the positions.
fn choices(items: &[Expr], k: usize, out: &mut Vec<Expr>) -> Result<(), String> {
    let n = items.len();
    if k > n {
        return Ok(());
    }
    let mut picks: Vec<usize> = (0..k).collect();
    loop {
        if out.len() >= LIMIT {
            return Err(format!("more than {LIMIT} sublists"));
        }
        out.push(Expr::list(
            picks.iter().map(|i| items[*i].clone()).collect(),
        ));
        let Some(at) = (0..k).rev().find(|j| picks[*j] != j + n - k) else {
            return Ok(());
        };
        picks[at] += 1;
        for j in at + 1..k {
            picks[j] = picks[j - 1] + 1;
        }
    }
}

/// Every way to take one entry from each list.
fn products(lists: &[&[Expr]]) -> Result<Vec<Expr>, String> {
    let mut size = 1usize;
    for l in lists {
        size = size
            .checked_mul(l.len())
            .filter(|s| *s <= LIMIT)
            .ok_or_else(|| format!("more than {LIMIT} tuples"))?;
    }
    let mut out = Vec::with_capacity(size);
    let mut at = vec![0usize; lists.len()];
    if size == 0 {
        return Ok(out);
    }
    loop {
        out.push(Expr::list(
            at.iter().zip(lists).map(|(i, l)| l[*i].clone()).collect(),
        ));
        let mut k = lists.len();
        loop {
            if k == 0 {
                return Ok(out);
            }
            k -= 1;
            at[k] += 1;
            if at[k] < lists[k].len() {
                break;
            }
            at[k] = 0;
        }
    }
}

fn whole(e: &Expr, what: &str, lo: i64, hi: i64) -> Result<usize, String> {
    e.as_i64()
        .filter(|n| (lo..=hi).contains(n))
        .map(|n| n as usize)
        .ok_or_else(|| format!("{what} must be a whole number from {lo} to {hi}"))
}

/// Whether a whole number above 1 has no square factor; `None` when it is too large to tell.
fn square_free(n: &Integer) -> Option<bool> {
    let mut n = n.to_u64().filter(|n| *n < 1_000_000_000_000)?;
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            n /= d;
            if n % d == 0 {
                return Some(false);
            }
        }
        d += 1;
    }
    Some(true)
}

/// What is known about a part of a real constant that is no rational number.
enum Part {
    /// The square root of a whole number that is no square.
    Root(Integer),
    /// Another root of a rational number that is no such power.
    Algebraic,
    /// A number that is the root of no polynomial with whole coefficients.
    Transcendental,
    Unknown,
}

fn kind_of(x: &Expr) -> Part {
    let rational = |e: &Expr| e.as_rat().filter(|r| !r.is_zero()).is_some();
    match x {
        Expr::Sym(s) if matches!(&**s, "pi" | "e") => Part::Transcendental,
        Expr::Pow(b, e) => match (b.as_rat(), e.as_rat()) {
            (None, Some(_)) if b.is_sym("pi") && e.as_int().is_some() && rational(e) => {
                Part::Transcendental
            }
            (None, Some(_)) if b.is_sym("e") && rational(e) => Part::Transcendental,
            (Some(r), Some(q)) if !r.is_negative() && !q.is_integer() => {
                let Some(k) = q.denom().to_u32() else {
                    return Part::Unknown;
                };
                let power =
                    |n: &Integer| crate::num::int_root(n, k).is_some_and(|(_, exact)| exact);
                if power(r.numer()) && power(r.denom()) {
                    Part::Unknown
                } else if k == 2
                    && r.is_integer()
                    && q.numer().cmp_abs(&Integer::from(1)).is_eq()
                    && *q > 0
                {
                    Part::Root(r.numer().clone())
                } else {
                    Part::Algebraic
                }
            }
            _ => Part::Unknown,
        },
        Expr::Fn(name, a) if a.len() == 1 => match (&**name, a[0].as_rat()) {
            ("ln", Some(r)) if *r > 0 && *r != 1 => Part::Transcendental,
            ("sin" | "cos" | "tan" | "sinh" | "cosh" | "tanh" | "atan", Some(r))
                if !r.is_zero() =>
            {
                Part::Transcendental
            }
            _ => Part::Unknown,
        },
        _ => Part::Unknown,
    }
}

/// Whether a real constant is known to be no quotient of whole numbers: rational multiples of
/// square roots of different square-free numbers, of one other root, or of one number like pi,
/// next to a rational number.
fn irrational(x: &Expr) -> bool {
    let terms: &[Expr] = match x {
        Expr::Add(t) => t,
        other => std::slice::from_ref(other),
    };
    let (mut roots, mut algebraic, mut transcendental) = (Vec::new(), 0, 0);
    for t in terms {
        if t.as_rat().is_some() {
            continue;
        }
        let core = match t {
            Expr::Mul(f) => {
                let mut rest = f.iter().filter(|x| x.as_rat().is_none());
                match (rest.next(), rest.next()) {
                    (Some(one), None) => one,
                    _ => return false,
                }
            }
            other => other,
        };
        match kind_of(core) {
            Part::Root(n) => roots.push(n),
            Part::Algebraic => algebraic += 1,
            Part::Transcendental => transcendental += 1,
            Part::Unknown => return false,
        }
    }
    match (transcendental, algebraic, roots.len()) {
        (1, 0, _) | (0, 1, 0) | (0, 0, 1) => true,
        (0, 0, n) if n > 1 => roots.iter().all(|r| square_free(r) == Some(true)),
        _ => false,
    }
}

/// A yes-or-no question about a value: `None` when the value does not say.
fn predicate(name: &str, x: &Expr) -> Option<bool> {
    use crate::assume;
    if matches!(x, Expr::List(_)) {
        return Some(name == "islist");
    }
    let value = || {
        x.is_const()
            .then(|| numeric::evalf_digits(x, numeric::digits()).ok())
            .flatten()
    };
    let decided = |yes: bool, no: bool| {
        if yes {
            Some(true)
        } else if no {
            Some(false)
        } else {
            None
        }
    };
    match name {
        "islist" => Some(false),
        "isnumber" => Some(x.is_const() && !x.is_infinite() && value().is_some()),
        "isinteger" | "isrational" if x.as_num().is_none() && x.is_const() => {
            let v = value()?;
            if !v.imag().is_zero() || irrational(x) {
                return Some(false);
            }
            if name == "isrational" {
                return None;
            }
            // a whole number in another form is one only when the difference vanishes
            let near = v.real().clone().round().to_integer()?;
            let off = simplify(&sub(x.clone(), Expr::integer(near)));
            match numeric::is_zero_const(&off) {
                Some(true) => Some(true),
                Some(false) => Some(false),
                None => None,
            }
        }
        "isinteger" => match x.as_num() {
            Some(n) => Some(n.is_int()),
            None => decided(assume::is_integer(x), false),
        },
        "isrational" => match x.as_num() {
            Some(n) => Some(n.im.is_zero()),
            None => decided(assume::is_integer(x), false),
        },
        "iseven" | "isodd" => match x.as_int() {
            Some(n) => Some(n.is_even() == (name == "iseven")),
            None => decided(false, x.is_const()),
        },
        "isreal" => match value() {
            Some(v) => Some(v.imag().is_zero()),
            None => decided(assume::is_real(x), x.is_const()),
        },
        "ispositive" | "isnegative" | "isnonnegative" => {
            let s = if x.is_const() {
                match value() {
                    Some(v) if v.imag().is_zero() => v.real().cmp0().map(|o| o as i32),
                    _ => return Some(false),
                }
            } else {
                assume::sign(x)
            };
            match name {
                "ispositive" => s.map(|s| s > 0),
                "isnegative" => s.map(|s| s < 0),
                _ => s
                    .map(|s| s >= 0)
                    .or_else(|| assume::is_nonnegative(x).then_some(true)),
            }
        }
        _ => None,
    }
}

fn pure_inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let list = |k: usize| -> Result<&[Expr], String> {
        a[k].as_list()
            .ok_or_else(|| format!("{name} needs a list, {} is none", a[k]))
    };
    let n = a.len();
    Ok(Some(match (name, n) {
        ("range", 1) => Expr::list(span(&Expr::one(), &a[0], &Expr::one())?),
        ("range", 2) => Expr::list(span(&a[0], &a[1], &Expr::one())?),
        ("range", 3) => Expr::list(span(&a[0], &a[1], &a[2])?),
        ("length", 1) => Expr::int(list(0)?.len() as i64),
        ("first", 1) => part(&a[0], &[Expr::one()])?,
        ("last", 1) => part(&a[0], &[Expr::minus_one()])?,
        ("rest", 1) => match list(0)? {
            [] => return Err("rest: the list is empty".into()),
            items => Expr::list(items[1..].to_vec()),
        },
        ("most", 1) => match list(0)? {
            [] => return Err("most: the list is empty".into()),
            items => Expr::list(items[..items.len() - 1].to_vec()),
        },
        ("part", 2..) => part(&a[0], &a[1..])?,
        ("take", 2) => {
            let items = list(0)?;
            let (from, to) = window(items, &a[1])?;
            Expr::list(items[from..to].to_vec())
        }
        ("drop", 2) => {
            let items = list(0)?;
            let (from, to) = window(items, &a[1])?;
            Expr::list([&items[..from], &items[to..]].concat())
        }
        ("reverse", 1) => Expr::list(list(0)?.iter().rev().cloned().collect()),
        ("sort", 1) => Expr::list(sorted(list(0)?)),
        ("rotateleft" | "rotateright", 1 | 2) => {
            let items = list(0)?;
            let by = match a.get(1) {
                Some(k) => k.as_i64().ok_or("the shift must be a whole number")?,
                None => 1,
            };
            let mut out = items.to_vec();
            if !out.is_empty() {
                let by = by.rem_euclid(out.len() as i64) as usize;
                if name == "rotateleft" {
                    out.rotate_left(by);
                } else {
                    out.rotate_right(by);
                }
            }
            Expr::list(out)
        }
        ("append", 2) => Expr::list([list(0)?, std::slice::from_ref(&a[1])].concat()),
        ("prepend", 2) => Expr::list([std::slice::from_ref(&a[1]), list(0)?].concat()),
        ("join", 1..) => {
            let mut out = Vec::new();
            for k in 0..n {
                out.extend_from_slice(list(k)?);
            }
            Expr::list(out)
        }
        ("insert", 3) => {
            let items = list(0)?;
            let at = offset(&a[2], items.len() + 1)?;
            let mut out = items.to_vec();
            out.insert(at, a[1].clone());
            Expr::list(out)
        }
        ("delete", 2) => {
            let items = list(0)?;
            let gone: Vec<usize> = match &a[1] {
                Expr::List(ps) => ps
                    .iter()
                    .map(|p| offset(p, items.len()))
                    .collect::<Result<_, _>>()?,
                p => vec![offset(p, items.len())?],
            };
            Expr::list(
                items
                    .iter()
                    .enumerate()
                    .filter(|(k, _)| !gone.contains(k))
                    .map(|(_, x)| x.clone())
                    .collect(),
            )
        }
        ("union", 1..) => {
            let mut all = Vec::new();
            for k in 0..n {
                all.extend_from_slice(list(k)?);
            }
            Expr::list(sorted(&unique(&all)))
        }
        ("intersection", 2..) => {
            let mut common = unique(list(0)?);
            for k in 1..n {
                let other = list(k)?;
                common.retain(|x| other.contains(x));
            }
            Expr::list(sorted(&common))
        }
        ("complement", 2..) => {
            let mut left = unique(list(0)?);
            for k in 1..n {
                let other = list(k)?;
                left.retain(|x| !other.contains(x));
            }
            Expr::list(sorted(&left))
        }
        ("unique", 1) => Expr::list(unique(list(0)?)),
        ("contains", 2) => Expr::Bool(list(0)?.contains(&a[1])),
        ("count", 2) => Expr::int(list(0)?.iter().filter(|x| **x == a[1]).count() as i64),
        ("position", 2) => Expr::list(
            list(0)?
                .iter()
                .enumerate()
                .filter(|(_, x)| **x == a[1])
                .map(|(k, _)| Expr::int(k as i64 + 1))
                .collect(),
        ),
        ("total", 1) | ("sum", 1) => total(list(0)?)?,
        ("product", 1) => {
            let items = list(0)?;
            if items.iter().any(|x| matches!(x, Expr::List(_))) {
                return Err("product of a list: the entries must be numbers or expressions".into());
            }
            mul(items.to_vec())
        }
        ("accumulate", 1) => {
            let mut out: Vec<Expr> = Vec::new();
            for x in list(0)? {
                let next = match out.last() {
                    Some(sum) => matrix::add(vec![sum.clone(), x.clone()])?,
                    None => x.clone(),
                };
                out.push(next);
            }
            Expr::list(out)
        }
        ("differences" | "ratios", 1 | 2) => {
            let times = match a.get(1) {
                Some(k) => whole(k, "the order", 0, 1000)?,
                None => 1,
            };
            let mut items = list(0)?.to_vec();
            for _ in 0..times {
                let mut next = Vec::new();
                for w in items.windows(2) {
                    next.push(if name == "differences" {
                        matrix::add(vec![
                            w[1].clone(),
                            matrix::mul(vec![Expr::minus_one(), w[0].clone()])?,
                        ])?
                    } else {
                        simplify(&div(w[1].clone(), w[0].clone()))
                    });
                }
                items = next;
            }
            Expr::list(items)
        }
        ("tally", 1) => {
            let items = list(0)?;
            Expr::list(
                unique(items)
                    .into_iter()
                    .map(|u| {
                        let count = items.iter().filter(|x| **x == u).count();
                        Expr::list(vec![u, Expr::int(count as i64)])
                    })
                    .collect(),
            )
        }
        ("zip", 1..) => {
            let lists: Vec<&[Expr]> = (0..n).map(list).collect::<Result<_, _>>()?;
            let len = lists.iter().map(|l| l.len()).min().unwrap_or(0);
            Expr::list(
                (0..len)
                    .map(|k| Expr::list(lists.iter().map(|l| l[k].clone()).collect()))
                    .collect(),
            )
        }
        ("permutations", 1) => {
            let items = list(0)?;
            Expr::list(arrangements(items, items.len())?)
        }
        ("permutations", 2) => match (a[0].as_int(), a[1].as_int()) {
            (Some(total), Some(k)) => {
                // the number of arrangements of k out of n things
                let (total, k) = (
                    total.to_u32().ok_or("permutations(n, k): n is too large")?,
                    k.to_u32().ok_or("permutations(n, k): k is too large")?,
                );
                if k > total {
                    Expr::zero()
                } else {
                    let mut count = Integer::from(1);
                    for j in 0..k {
                        count *= total - j;
                    }
                    Expr::integer(count)
                }
            }
            _ => {
                let items = list(0)?;
                let k = whole(&a[1], "the size", 0, items.len() as i64)?;
                Expr::list(arrangements(items, k)?)
            }
        },
        ("subsets", 1 | 2) => {
            let items = list(0)?;
            let mut out = Vec::new();
            match a.get(1) {
                Some(k) => choices(items, whole(k, "the size", 0, 1 << 20)?, &mut out)?,
                None => {
                    if items.len() > 19 {
                        return Err(format!("more than {LIMIT} sublists"));
                    }
                    for k in 0..=items.len() {
                        choices(items, k, &mut out)?;
                    }
                }
            }
            Expr::list(out)
        }
        ("tuples", 2) => {
            let items = list(0)?;
            let k = whole(&a[1], "the size", 0, 64)?;
            Expr::list(products(&vec![items; k])?)
        }
        ("cartesianproduct", 1..) => {
            let lists: Vec<&[Expr]> = (0..n).map(list).collect::<Result<_, _>>()?;
            Expr::list(products(&lists)?)
        }
        ("partition", 2) => {
            let items = list(0)?;
            let size = whole(&a[1], "the size", 1, LIMIT as i64)?;
            Expr::list(
                items
                    .chunks_exact(size)
                    .map(|c| Expr::list(c.to_vec()))
                    .collect(),
            )
        }
        (
            "islist" | "isnumber" | "isinteger" | "isrational" | "iseven" | "isodd" | "isreal"
            | "ispositive" | "isnegative" | "isnonnegative",
            1,
        ) => {
            let x = match &a[0] {
                Expr::List(_) => a[0].clone(),
                other => simplify(other),
            };
            match predicate(name, &x) {
                Some(b) => Expr::Bool(b),
                None => Expr::call(name, vec![x]),
            }
        }
        _ => return Ok(None),
    }))
}

/// The list commands whose arguments are plain values.
pub fn pure(name: &str, a: &[Expr]) -> Option<Res> {
    pure_inner(name, a).transpose()
}

impl Session {
    fn is_function(&self, e: &Expr) -> bool {
        if lambda(e).is_some() {
            return true;
        }
        match e.sym_name() {
            Some(n) => match self.vars.get(n) {
                Some(v) => lambda(v).is_some(),
                None => self.funcs.contains_key(n) || crate::names::function(n).is_some(),
            },
            None => false,
        }
    }

    pub(crate) fn with_bound<T>(
        &mut self,
        names: &[String],
        values: Vec<Expr>,
        f: impl FnOnce(&mut Session) -> T,
    ) -> T {
        let saved: Vec<Option<Expr>> = names.iter().map(|p| self.vars.get(p).cloned()).collect();
        for (p, v) in names.iter().zip(values) {
            self.vars.insert(p.clone(), v);
        }
        let out = f(self);
        for (p, old) in names.iter().zip(saved) {
            match old {
                Some(v) => self.vars.insert(p.clone(), v),
                None => self.vars.remove(p),
            };
        }
        out
    }

    /// The value of a function, `x -> body` or the name of one, at `args`.
    pub(crate) fn apply(&mut self, f: &Expr, args: &[Expr]) -> Res {
        if let Some((params, body)) = lambda(f) {
            let values: Vec<Expr> = if params.len() == args.len() {
                args.to_vec()
            } else if let [Expr::List(items)] = args
                && items.len() == params.len()
            {
                items.to_vec()
            } else {
                return Err(format!(
                    "{f} takes {} argument(s), {} given",
                    params.len(),
                    args.len()
                ));
            };
            let body = body.clone();
            return self.with_bound(&params, values, |s| s.eval(&body));
        }
        let Expr::Sym(name) = f else {
            return Err(format!(
                "{f} is not a function: write x -> expr or the name of a function"
            ));
        };
        if let Some(v) = self.vars.get(&**name).cloned() {
            return if lambda(&v).is_some() {
                self.apply(&v, args)
            } else {
                Err(format!("{name} is not a function"))
            };
        }
        if self.funcs.contains_key(&**name) {
            return self.apply_user(name, args);
        }
        let call = crate::parser::call_by_name(name, args.to_vec())?;
        self.eval(&call)
    }

    /// The variable and the values of `[i, a, b]`, `[i, a, b, step]`, `[i, n]`, `[i, list]`; a
    /// plain count has no variable.
    fn iterator(&mut self, spec: &Expr) -> Result<(Option<String>, Vec<Expr>), String> {
        let usage = "an iterator is [i, a, b], [i, a, b, step], [i, n], [i, list] or a count";
        let Some(items) = spec.as_list() else {
            let n = self.eval(spec)?;
            let n = n
                .as_i64()
                .filter(|n| (0..=LIMIT as i64).contains(n))
                .ok_or(usage)?;
            return Ok((None, vec![Expr::zero(); n as usize]));
        };
        let var = items
            .first()
            .and_then(Expr::sym_name)
            .ok_or(usage)?
            .to_string();
        let ends = self.eval_all(&items[1..])?;
        let values = match ends.as_slice() {
            [Expr::List(values)] => values.to_vec(),
            [n] => span(&Expr::one(), n, &Expr::one())?,
            [a, b] => span(a, b, &Expr::one())?,
            [a, b, step] => span(a, b, step)?,
            _ => return Err(usage.into()),
        };
        Ok((Some(var), values))
    }

    fn loop_rec(
        &mut self,
        collect: bool,
        body: &Expr,
        specs: &[Expr],
        count: &mut usize,
        last: &mut Expr,
    ) -> Res {
        let Some((spec, rest)) = specs.split_first() else {
            *count += 1;
            if *count > LIMIT {
                return Err(format!("more than {LIMIT} steps"));
            }
            let v = self.eval(body)?;
            if !collect {
                *last = v;
                return Ok(Expr::zero());
            }
            return Ok(v);
        };
        let (var, values) = self.iterator(spec)?;
        let saved = var.as_ref().and_then(|v| self.vars.remove(v));
        let mut out = Vec::new();
        let mut failure = None;
        for v in values {
            if let Some(name) = &var {
                self.vars.insert(name.clone(), v);
            }
            match self.loop_rec(collect, body, rest, count, last) {
                Ok(x) if collect => out.push(x),
                Ok(_) => {}
                Err(e) => {
                    failure = Some(e);
                    break;
                }
            }
        }
        if let Some(name) = &var {
            match saved {
                Some(v) => self.vars.insert(name.clone(), v),
                None => self.vars.remove(name),
            };
        }
        match failure {
            Some(e) => Err(e),
            None => Ok(Expr::list(out)),
        }
    }

    /// `table(body, iterators...)` collects the values of the body; `do` returns the last one.
    fn loop_form(&mut self, collect: bool, body: &Expr, specs: &[Expr]) -> Res {
        let is_var = |e: &Expr| matches!(e, Expr::Sym(_)) || *e == Expr::i();
        let flat = (3..=4).contains(&specs.len())
            && is_var(&specs[0])
            && !specs.iter().any(|s| matches!(s, Expr::List(_)));
        let joined = [Expr::list(specs.to_vec())];
        let specs = if flat { &joined[..] } else { specs };
        let unit_index = specs.iter().any(|s| {
            s.as_list()
                .and_then(<[Expr]>::first)
                .is_some_and(|v| *v == Expr::i())
        });
        let units = specs
            .iter()
            .filter(|s| {
                s.as_list()
                    .and_then(<[Expr]>::first)
                    .is_some_and(|v| *v == Expr::i())
            })
            .count();
        if unit_index && (units > 1 || inner_unit_index(body)) {
            return Err(CLASH.into());
        }
        let (body, specs): (Expr, Vec<Expr>) = if unit_index {
            (index_i(body), specs.iter().map(index_i).collect())
        } else {
            (body.clone(), specs.to_vec())
        };
        let mut count = 0usize;
        let mut last = Expr::list(Vec::new());
        let out = self.loop_rec(collect, &body, &specs, &mut count, &mut last)?;
        Ok(if collect { out } else { last })
    }

    /// `sum` and `product` with an index that is written `i` or that has a value.
    fn index_call(&mut self, name: &str, raw: &[Expr]) -> Option<Res> {
        let (var, args): (String, Vec<Expr>) = if raw[1] == Expr::i() {
            if inner_unit_index(&raw[0]) {
                return Some(Err(CLASH.into()));
            }
            ("i".to_string(), raw.iter().map(index_i).collect())
        } else {
            let v = raw[1].sym_name().filter(|v| self.vars.contains_key(*v))?;
            (v.to_string(), raw.to_vec())
        };
        let saved = self.vars.remove(&var);
        let result = self.call(name, &args);
        if let Some(v) = saved {
            self.vars.insert(var, v);
        }
        Some(result)
    }

    fn functional(&mut self, name: &str, raw: &[Expr]) -> Res {
        let usage = || {
            match name {
            "map" => "map(f, list): f is x -> expr or the name of a function",
            "select" | "reject" | "countif" | "anytrue" | "alltrue" | "findfirst" => {
                "a test and a list are needed, as in select(x -> x > 2, list) or select(list, isprime)"
            }
            "fold" | "foldlist" => "fold(f, start, list) with f = (a, b) -> expr",
            "nestlist" => "nestlist(f, start, n)",
            "nest" => "nest(f, start, n)",
            "fixedpoint" => "fixedpoint(f, start) or fixedpoint(f, start, most)",
            "apply" => "apply(f, list)",
            "sortby" => "sortby(f, list)",
            _ => "array(f, n) or array(f, [m, n])",
        }
        .to_string()
        };
        // the function may come first or last: map(f, list) and select(list, f) are both read
        let pos = raw
            .iter()
            .position(|e| self.is_function(e))
            .ok_or_else(usage)?;
        let f = raw[pos].clone();
        let mut rest = Vec::new();
        for (k, e) in raw.iter().enumerate() {
            if k != pos {
                rest.push(self.eval(e)?);
            }
        }
        let items = |e: &Expr| -> Result<Vec<Expr>, String> {
            e.as_list()
                .map(<[Expr]>::to_vec)
                .ok_or_else(|| format!("{name} needs a list, {e} is none"))
        };
        let one = std::slice::from_ref::<Expr>;
        match (name, rest.len()) {
            ("map", 1) => items(&rest[0])?
                .iter()
                .map(|x| self.apply(&f, one(x)))
                .collect::<Result<Vec<_>, _>>()
                .map(Expr::list),
            ("map", _) if rest.len() > 1 => {
                let lists: Vec<Vec<Expr>> = rest.iter().map(&items).collect::<Result<_, _>>()?;
                if lists.iter().any(|l| l.len() != lists[0].len()) {
                    return Err("map: the lists must be equally long".into());
                }
                (0..lists[0].len())
                    .map(|k| {
                        let args: Vec<Expr> = lists.iter().map(|l| l[k].clone()).collect();
                        self.apply(&f, &args)
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map(Expr::list)
            }
            ("select" | "reject" | "countif" | "anytrue" | "alltrue" | "findfirst", 1) => {
                let mut kept = Vec::new();
                let all = items(&rest[0])?;
                for x in &all {
                    let t = truth(&self.apply(&f, one(x))?);
                    let hit = if name == "reject" {
                        t == Some(false)
                    } else {
                        t == Some(true)
                    };
                    if hit {
                        if name == "findfirst" {
                            return Ok(x.clone());
                        }
                        kept.push(x.clone());
                    }
                }
                Ok(match name {
                    "countif" => Expr::int(kept.len() as i64),
                    "anytrue" => Expr::Bool(!kept.is_empty()),
                    "alltrue" => Expr::Bool(kept.len() == all.len()),
                    "findfirst" => Expr::list(Vec::new()),
                    _ => Expr::list(kept),
                })
            }
            ("fold" | "foldlist", 1 | 2) => {
                let all = items(&rest[rest.len() - 1])?;
                let (mut acc, from) = match rest.len() {
                    2 => (rest[0].clone(), 0),
                    _ => (
                        all.first()
                            .cloned()
                            .ok_or("fold: the list is empty and there is no start")?,
                        1,
                    ),
                };
                let mut steps = vec![acc.clone()];
                for x in &all[from..] {
                    acc = self.apply(&f, &[acc, x.clone()])?;
                    steps.push(acc.clone());
                }
                Ok(if name == "fold" {
                    acc
                } else {
                    Expr::list(steps)
                })
            }
            ("nestlist" | "nest", 2) => {
                let n = whole(&rest[1], "nestlist(f, start, n): n", 0, LIMIT as i64)?;
                let mut out = vec![rest[0].clone()];
                for _ in 0..n {
                    let next = self.apply(&f, one(out.last().expect("not empty")))?;
                    out.push(next);
                }
                Ok(if name == "nest" {
                    out.pop().expect("not empty")
                } else {
                    Expr::list(out)
                })
            }
            ("fixedpoint", 1 | 2) => {
                let most = match rest.get(1) {
                    Some(m) => whole(m, "fixedpoint(f, start, most): most", 1, LIMIT as i64)?,
                    None => 10_000,
                };
                // values that never repeat exactly are followed as approximate numbers
                let digits = numeric::digits();
                let mut approximate = self.decimal;
                let mut x = rest[0].clone();
                for step in 0..most {
                    let mut next = self.apply(&f, one(&x))?;
                    let closed = next.as_num().is_some() || matches!(next, Expr::Float(_));
                    if next.is_const()
                        && !next.is_infinite()
                        && !matches!(next, Expr::List(_) | Expr::Bool(_))
                        && (!closed || approximate || step >= 50)
                        && next != x
                    {
                        if !approximate && !self.decimal {
                            self.notes.push(
                                "the values do not repeat exactly: they are followed as approximate numbers"
                                    .to_string(),
                            );
                        }
                        approximate = true;
                        next = numeric::n(&next, digits + 8);
                    }
                    let same = next == x
                        || (matches!(next, Expr::Float(_)) && next.to_string() == x.to_string());
                    x = next;
                    if same {
                        break;
                    }
                    if step + 1 == most {
                        self.notes.push(format!(
                            "no fixed point after {most} steps: the last value is shown"
                        ));
                    }
                }
                Ok(match &x {
                    Expr::Float(f) if approximate => numeric::to_float_expr(f.v.clone(), digits),
                    _ => x,
                })
            }
            ("apply", 1) => {
                let args = items(&rest[0])?;
                self.apply(&f, &args)
            }
            ("sortby", 1) => {
                let all = items(&rest[0])?;
                let keys: Vec<Expr> = all
                    .iter()
                    .map(|x| self.apply(&f, one(x)))
                    .collect::<Result<_, _>>()?;
                Ok(Expr::list(sorted_by(&all, &keys)))
            }
            ("array", 1) => {
                let sizes: Vec<usize> = match &rest[0] {
                    Expr::List(dims) => dims
                        .iter()
                        .map(|d| whole(d, "a size", 0, LIMIT as i64))
                        .collect::<Result<_, _>>()?,
                    d => vec![whole(d, "a size", 0, LIMIT as i64)?],
                };
                if sizes.iter().product::<usize>() > LIMIT {
                    return Err(format!("an array of more than {LIMIT} entries"));
                }
                let mut at = Vec::new();
                self.array(&f, &sizes, &mut at)
            }
            _ => Err(usage()),
        }
    }

    fn array(&mut self, f: &Expr, sizes: &[usize], at: &mut Vec<Expr>) -> Res {
        let Some((size, rest)) = sizes.split_first() else {
            return self.apply(f, at);
        };
        let mut out = Vec::with_capacity(*size);
        for k in 1..=*size {
            at.push(Expr::int(k as i64));
            let value = self.array(f, rest, at);
            at.pop();
            out.push(value?);
        }
        Ok(Expr::list(out))
    }

    /// Forms that evaluate their arguments themselves.
    pub(crate) fn form(&mut self, name: &str, raw: &[Expr]) -> Option<Res> {
        let n = raw.len();
        Some(match name {
            "if" if (2..=3).contains(&n) => (|| {
                let c = self.eval(&raw[0])?;
                match truth(&c) {
                    Some(true) => self.eval(&raw[1]),
                    Some(false) => match raw.get(2) {
                        Some(e) => self.eval(e),
                        None => Ok(Expr::list(Vec::new())),
                    },
                    None => {
                        let mut args = vec![c, self.eval(&raw[1])?];
                        if let Some(e) = raw.get(2) {
                            args.push(self.eval(e)?);
                        }
                        Ok(crate::funcs::func("piecewise", args))
                    }
                }
            })(),
            "while" if n == 2 => (|| {
                let mut last = Expr::list(Vec::new());
                for _ in 0..LIMIT {
                    let c = self.eval(&raw[0])?;
                    match truth(&c) {
                        Some(true) => last = self.eval(&raw[1])?,
                        Some(false) => return Ok(last),
                        None => {
                            return Err(format!(
                                "while: the condition {c} is neither true nor false"
                            ));
                        }
                    }
                }
                Err(format!("while: more than {LIMIT} rounds"))
            })(),
            "block" if n >= 1 => (|| {
                let mut last = Expr::list(Vec::new());
                for step in raw {
                    last = self.eval(step)?;
                }
                Ok(last)
            })(),
            "table" | "do" if n >= 2 => self.loop_form(name == "table", &raw[0], &raw[1..]),
            "for" if (3..=5).contains(&n) => {
                let spec = Expr::list(raw[..n - 1].to_vec());
                self.loop_form(false, &raw[n - 1], &[spec])
            }
            "sum" | "product" if n == 4 => return self.index_call(name, raw),
            _ if crate::parser::FUNCTION_FORMS.contains(&name) => self.functional(name, raw),
            _ => return None,
        })
    }
}
