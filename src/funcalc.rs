//! Expressions in an unknown function and its derivatives, written `y`, `y'`, `y''`: the
//! derivative of a functional, and an equation of higher order as a field of first order.

use crate::canon::{add, div, mul, neg, subst};
use crate::diff::diff;
use crate::eval::{Session, map_raw};
use crate::expr::{Expr, Rel};
use crate::ode::marker;
use crate::simplify::simplify;

type Out = Result<(Expr, Vec<String>), String>;

/// The derivative of an order of the unknown function, as it is written.
fn primed(y: &str, order: u32) -> Expr {
    (0..order).fold(Expr::sym(y), |inner, _| {
        Expr::call("derivative'", vec![inner])
    })
}

/// The highest derivative that an expression holds.
fn highest(e: &Expr) -> u32 {
    let mut most = 0;
    e.walk(&mut |x| {
        if let Some(order) = x
            .sym_name()
            .and_then(|s| s.strip_prefix("#d"))
            .and_then(|k| k.parse::<u32>().ok())
        {
            most = most.max(order);
        }
    });
    most
}

/// The names stand for the derivatives again.
fn written(e: &Expr, y: &str, most: u32) -> Expr {
    (1..=most).fold(e.clone(), |e, order| {
        subst(&e, &Expr::sym(&marker(order)), &primed(y, order))
    })
}

impl Session {
    /// The unknown function and the variable that a command names, and its first argument
    /// with `y'`, `y''`, `diff(y, x, k)` as names of their own.
    fn with_names(&mut self, raw: &[Expr], usage: &str) -> Result<(Expr, String, String), String> {
        let first = raw.first().ok_or(usage)?;
        let name = |k: usize, otherwise: &str| match raw.get(k) {
            Some(Expr::Sym(s)) => Ok(s.to_string()),
            Some(_) => Err(usage.to_string()),
            None => Ok(otherwise.to_string()),
        };
        let y = name(
            1,
            &crate::eval::primed_name(first).unwrap_or_else(|| "y".to_string()),
        )?;
        let x = name(2, if y == "x" { "t" } else { "x" })?;
        let mut bad = None;
        let converted = map_raw(first, &mut |node| {
            let (mut order, mut base) = (0u32, node);
            while let Expr::Fn(n, a) = base
                && &**n == "derivative'"
            {
                order += 1;
                base = &a[0];
            }
            if order == 0
                && let Expr::Fn(n, a) = node
                && &**n == "diff"
                && (a.len() == 2 || a.len() == 3)
                && a[1].is_sym(&x)
                && (a[0].is_sym(&y) || matches!(&a[0], Expr::Fn(f, v) if **f == *y && v.len() == 1))
            {
                order = a
                    .get(2)
                    .map_or(Some(1), |k| k.as_i64().and_then(|k| u32::try_from(k).ok()))?;
                base = &a[0];
            }
            match base {
                Expr::Sym(s) if **s == *y && order > 0 => Some(Expr::sym(&marker(order))),
                Expr::Fn(n, a) if **n == *y && a.len() == 1 => {
                    if !a[0].is_sym(&x) {
                        bad = Some(format!("{usage}: {y} is a function of {x} alone"));
                    }
                    Some(match order {
                        0 => Expr::sym(&y),
                        _ => Expr::sym(&marker(order)),
                    })
                }
                _ => None,
            }
        });
        match bad {
            Some(why) => Err(why),
            None => Ok((self.eval(&converted)?, y, x)),
        }
    }

    /// The derivative of a functional `integral of f(x, y, y', ...) dx` by its function y:
    /// the sum of `(-1)^k d^k/dx^k` of the derivative of f by the k-th derivative of y. Where
    /// it is zero, the functional is stationary (the equation of Euler and Lagrange).
    pub(crate) fn functional_derivative(&mut self, raw: &[Expr], decimal: bool) -> Out {
        let usage =
            "functionalderivative(f, y, x), for example functionalderivative(y'^2/2 - y^2/2, y, x)";
        if raw.is_empty() || raw.len() > 3 {
            return Err(usage.into());
        }
        let (f, y, x) = self.with_names(raw, usage)?;
        if f.as_list().is_some() || matches!(f, Expr::Rel(..)) {
            return Err(usage.into());
        }
        let most = highest(&f);
        let name = |order: u32| match order {
            0 => y.clone(),
            _ => marker(order),
        };
        // the derivative along x of what holds y and its derivatives up to an order
        let along = |g: &Expr, up_to: u32| -> Expr {
            let mut terms = vec![diff(g, &x)];
            for order in 0..=up_to {
                let by = diff(g, &name(order));
                if !by.is_zero() {
                    terms.push(mul(vec![Expr::sym(&name(order + 1)), by]));
                }
            }
            add(terms)
        };
        let mut terms = Vec::new();
        for order in 0..=most {
            let mut part = diff(&f, &name(order));
            for turn in 0..order {
                part = simplify(&along(&part, most + turn));
            }
            terms.push(if order % 2 == 1 { neg(part) } else { part });
        }
        let value = written(&simplify(&add(terms)), &y, 2 * most);
        let shown = match decimal {
            true => crate::numeric::n(&value, crate::numeric::digits()),
            false => value.clone(),
        };
        Ok((value, vec![format!("{shown}")]))
    }

    /// An equation of order n in y as the derivatives of `[y, y', ..., y^(n-1)]`: the last
    /// one is the highest derivative solved for, the others are the next entry each.
    pub(crate) fn vector_field(&mut self, raw: &[Expr], decimal: bool) -> Out {
        let usage =
            "odetovectorfield(equation, y, x), for example odetovectorfield(y'' + y = 0, y, x)";
        if raw.is_empty() || raw.len() > 3 {
            return Err(usage.into());
        }
        let (value, y, _) = self.with_names(raw, usage)?;
        let f = match &value {
            Expr::Rel(Rel::Eq, l, r) => crate::canon::sub((**l).clone(), (**r).clone()),
            Expr::Rel(..) | Expr::List(_) => return Err(usage.into()),
            other => other.clone(),
        };
        let most = highest(&f);
        if most == 0 {
            return Err(format!("{usage}: the equation holds no derivative of {y}"));
        }
        let top = marker(most);
        let slope = simplify(&diff(&f, &top));
        if slope.is_zero() || slope.has(&Expr::sym(&top)) {
            return Err(format!(
                "odetovectorfield: the highest derivative of {y} must stand to the first power"
            ));
        }
        let rest = simplify(&subst(&f, &Expr::sym(&top), &Expr::zero()));
        let mut field: Vec<Expr> = (1..most).map(|order| primed(&y, order)).collect();
        field.push(written(&simplify(&div(neg(rest), slope)), &y, most));
        let value = Expr::list(field);
        let shown = match decimal {
            true => crate::numeric::n(&value, crate::numeric::digits()),
            false => value.clone(),
        };
        let state: Vec<String> = (0..most)
            .map(|order| format!("{}", primed(&y, order)))
            .collect();
        self.notes
            .push(format!("the derivatives of [{}]", state.join(", ")));
        Ok((value, vec![format!("{shown}")]))
    }
}
