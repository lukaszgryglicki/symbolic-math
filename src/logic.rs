//! Truth values: conditions, the logical connectives and piecewise expressions.

use crate::canon::sub;
use crate::expr::{Expr, Rel};
use crate::numeric;
use crate::simplify::simplify;

fn relation(op: Rel, l: &Expr, r: &Expr) -> Option<bool> {
    let equality = matches!(op, Rel::Eq | Rel::Ne);
    let as_equal = |same: bool| same == (op == Rel::Eq);
    if let (Expr::Bool(a), Expr::Bool(b)) = (l, r) {
        return equality.then(|| as_equal(a == b));
    }
    if let (Some(a), Some(b)) = (crate::radix::text_of(l), crate::radix::text_of(r)) {
        return Some(match op {
            Rel::Eq => a == b,
            Rel::Ne => a != b,
            Rel::Lt => a < b,
            Rel::Le => a <= b,
            Rel::Gt => a > b,
            Rel::Ge => a >= b,
        });
    }
    match (l.as_list(), r.as_list()) {
        (Some(a), Some(b)) => {
            if !equality {
                return None;
            }
            if a.len() != b.len() {
                return Some(as_equal(false));
            }
            let mut all = Some(true);
            for (x, y) in a.iter().zip(b) {
                match relation(Rel::Eq, x, y) {
                    Some(false) => return Some(as_equal(false)),
                    None => all = None,
                    Some(true) => {}
                }
            }
            return all.map(as_equal);
        }
        (Some(_), None) | (None, Some(_)) => {
            let other = if l.as_list().is_some() { r } else { l };
            return (equality && other.is_const()).then(|| as_equal(false));
        }
        (None, None) => {}
    }
    let d = simplify(&sub(l.clone(), r.clone()));
    if d.is_undefined() {
        return None;
    }
    if equality {
        let zero = if d.is_zero() {
            true
        } else if d.is_infinite() {
            false
        } else if d.is_const() {
            numeric::is_zero_const(&d)?
        } else {
            return None;
        };
        return Some(as_equal(zero));
    }
    let s = if d.is_inf() {
        1
    } else if d.is_neg_inf() {
        -1
    } else {
        numeric::sign(&d)?
    };
    Some(match op {
        Rel::Lt => s < 0,
        Rel::Le => s <= 0,
        Rel::Gt => s > 0,
        Rel::Ge => s >= 0,
        Rel::Eq | Rel::Ne => unreachable!(),
    })
}

/// The truth of a condition, when it can be decided.
pub fn truth(e: &Expr) -> Option<bool> {
    match e {
        Expr::Bool(b) => Some(*b),
        Expr::Rel(op, l, r) => relation(*op, l, r),
        Expr::Fn(name, a) => match (&**name, a.len()) {
            ("and", _) => {
                let mut all = true;
                for x in a.iter() {
                    match truth(x) {
                        Some(false) => return Some(false),
                        None => all = false,
                        Some(true) => {}
                    }
                }
                all.then_some(true)
            }
            ("or", _) => {
                let mut none = true;
                for x in a.iter() {
                    match truth(x) {
                        Some(true) => return Some(true),
                        None => none = false,
                        Some(false) => {}
                    }
                }
                none.then_some(false)
            }
            ("not", 1) => truth(&a[0]).map(|b| !b),
            ("xor", _) => a.iter().try_fold(false, |acc, x| truth(x).map(|t| acc ^ t)),
            ("implies", 2) => match (truth(&a[0]), truth(&a[1])) {
                (Some(false), _) | (_, Some(true)) => Some(true),
                (Some(true), Some(false)) => Some(false),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

/// A connective with its decided operands resolved.
pub fn connective(name: &str, a: &[Expr]) -> Option<Expr> {
    let whole = Expr::call(name, a.to_vec());
    if let Some(b) = truth(&whole) {
        return Some(Expr::Bool(b));
    }
    match name {
        "and" | "or" => {
            // the operands that are decided say nothing more
            let kept: Vec<Expr> = a.iter().filter(|x| truth(x).is_none()).cloned().collect();
            match kept.len() {
                n if n == a.len() => None,
                1 => Some(kept[0].clone()),
                _ => Some(Expr::call(name, kept)),
            }
        }
        "not" => match &a[0] {
            Expr::Rel(op, l, r) => {
                let flipped = match op {
                    Rel::Eq => Rel::Ne,
                    Rel::Ne => Rel::Eq,
                    Rel::Lt => Rel::Ge,
                    Rel::Le => Rel::Gt,
                    Rel::Gt => Rel::Le,
                    Rel::Ge => Rel::Lt,
                };
                Some(Expr::rel(flipped, (**l).clone(), (**r).clone()))
            }
            Expr::Fn(n, inner) if &**n == "not" && inner.len() == 1 => Some(inner[0].clone()),
            _ => None,
        },
        _ => None,
    }
}

/// `piecewise(c1, v1, c2, v2, ..., otherwise)`: the first value whose condition holds. The form
/// `piecewise([[v1, c1], [v2, c2]], otherwise)` is read too.
pub fn piecewise(args: &[Expr]) -> Option<Expr> {
    let listed: Option<Vec<Expr>> = match args.first() {
        Some(Expr::List(rows))
            if args.len() <= 2
                && !rows.is_empty()
                && rows
                    .iter()
                    .all(|r| r.as_list().is_some_and(|p| p.len() == 2)) =>
        {
            let mut flat = Vec::new();
            for r in rows.iter() {
                let pair = r.as_list().expect("checked");
                flat.push(pair[1].clone());
                flat.push(pair[0].clone());
            }
            flat.extend(args.get(1).cloned());
            Some(flat)
        }
        _ => None,
    };
    let rewritten = listed.is_some();
    let a: &[Expr] = listed.as_deref().unwrap_or(args);
    let mut kept: Vec<Expr> = Vec::new();
    let mut otherwise = (a.len() % 2 == 1).then(|| a[a.len() - 1].clone());
    let mut changed = rewritten;
    for pair in a.as_chunks::<2>().0 {
        match truth(&pair[0]) {
            Some(true) => {
                otherwise = Some(pair[1].clone());
                changed = true;
                break;
            }
            Some(false) => changed = true,
            None => kept.extend(pair.iter().cloned()),
        }
    }
    if kept.is_empty() {
        return Some(otherwise.unwrap_or_else(Expr::undefined));
    }
    if !changed {
        return None;
    }
    kept.extend(otherwise);
    Some(Expr::call("piecewise", kept))
}

/// The conditions and values of a piecewise expression, and what it is otherwise.
pub fn pieces(args: &[Expr]) -> (Vec<(&Expr, &Expr)>, Option<&Expr>) {
    let pairs = args
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| (&p[0], &p[1]))
        .collect();
    (pairs, (args.len() % 2 == 1).then(|| &args[args.len() - 1]))
}

/// The same pieces with other values.
pub fn map_values(args: &[Expr], f: &mut dyn FnMut(&Expr) -> Option<Expr>) -> Option<Expr> {
    let mut out = Vec::with_capacity(args.len());
    for (k, x) in args.iter().enumerate() {
        let is_value = k % 2 == 1 || k + 1 == args.len();
        out.push(if is_value { f(x)? } else { x.clone() });
    }
    Some(crate::funcs::func("piecewise", out))
}

/// The expressions `l - r` of the relations of a condition: where they change sign, it may change.
pub fn boundaries(cond: &Expr, out: &mut Vec<Expr>) {
    match cond {
        Expr::Rel(_, l, r) => out.push(sub((**l).clone(), (**r).clone())),
        Expr::Fn(n, a) if matches!(&**n, "and" | "or" | "not" | "xor" | "implies") => {
            a.iter().for_each(|x| boundaries(x, out))
        }
        _ => {}
    }
}
