//! Derivatives.

use crate::canon::{add, div, mul, neg, pow, powi, sqrt, sub};
use crate::expr::Expr;
use crate::funcs::func;

fn f1(name: &str, u: &Expr) -> Expr {
    func(name, vec![u.clone()])
}

fn unevaluated(e: &Expr, var: &str) -> Expr {
    Expr::call("diff", vec![e.clone(), Expr::sym(var)])
}

/// Derivative of a one-argument function with respect to its argument `u`.
pub fn outer_derivative(name: &str, u: &Expr) -> Option<Expr> {
    let one = Expr::one;
    let sq = |e: &Expr| powi(e.clone(), 2);
    Some(match name {
        "sin" => f1("cos", u),
        "cos" => neg(f1("sin", u)),
        "tan" => powi(f1("cos", u), -2),
        "sinh" => f1("cosh", u),
        "cosh" => f1("sinh", u),
        "tanh" => powi(f1("cosh", u), -2),
        "asin" => powi(sqrt(sub(one(), sq(u))), -1),
        "acos" => neg(powi(sqrt(sub(one(), sq(u))), -1)),
        "atan" => powi(add(vec![one(), sq(u)]), -1),
        "asinh" => powi(sqrt(add(vec![sq(u), one()])), -1),
        "acosh" => powi(
            mul(vec![
                sqrt(sub(u.clone(), one())),
                sqrt(add(vec![u.clone(), one()])),
            ]),
            -1,
        ),
        "atanh" => powi(sub(one(), sq(u)), -1),
        "ln" => powi(u.clone(), -1),
        "abs" => f1("sign", u),
        "sign" | "floor" | "ceil" | "round" | "trunc" => Expr::zero(),
        "frac" => one(),
        "erf" => mul(vec![
            Expr::int(2),
            powi(sqrt(Expr::pi()), -1),
            pow(Expr::e(), neg(sq(u))),
        ]),
        "gamma" => mul(vec![f1("gamma", u), f1("digamma", u)]),
        "lgamma" => f1("digamma", u),
        "factorial" => {
            let v = add(vec![u.clone(), one()]);
            mul(vec![f1("gamma", &v), f1("digamma", &v)])
        }
        "digamma" => func("polygamma", vec![one(), u.clone()]),
        "ei" => div(pow(Expr::e(), u.clone()), u.clone()),
        "li" => powi(f1("ln", u), -1),
        "si" => div(f1("sin", u), u.clone()),
        "ci" => div(f1("cos", u), u.clone()),
        "shi" => div(f1("sinh", u), u.clone()),
        "chi" => div(f1("cosh", u), u.clone()),
        "li2" => neg(div(f1("ln", &sub(one(), u.clone())), u.clone())),
        "lambertw" => {
            let w = f1("lambertw", u);
            div(w.clone(), mul(vec![u.clone(), add(vec![one(), w])]))
        }
        "harmonic" => func("polygamma", vec![one(), add(vec![u.clone(), one()])]),
        _ => return crate::specfn::derivative(name, u),
    })
}

pub fn diff(e: &Expr, var: &str) -> Expr {
    if !e.has_sym(var) {
        return match e {
            Expr::List(a) => Expr::list(a.iter().map(|_| Expr::zero()).collect()),
            _ => Expr::zero(),
        };
    }
    let d = |x: &Expr| diff(x, var);
    match e {
        Expr::Sym(_) => Expr::one(),
        Expr::Add(a) => add(a.iter().map(d).collect()),
        Expr::Mul(a) => add((0..a.len())
            .filter(|i| a[*i].has_sym(var))
            .map(|i| {
                mul(a
                    .iter()
                    .enumerate()
                    .map(|(j, f)| if i == j { d(f) } else { f.clone() })
                    .collect())
            })
            .collect()),
        Expr::Pow(b, x) => {
            let (b, x) = ((**b).clone(), (**x).clone());
            if !x.has_sym(var) {
                return mul(vec![x.clone(), pow(b.clone(), sub(x, Expr::one())), d(&b)]);
            }
            if !b.has_sym(var) {
                return mul(vec![e.clone(), f1("ln", &b), d(&x)]);
            }
            mul(vec![
                e.clone(),
                add(vec![
                    mul(vec![d(&x), f1("ln", &b)]),
                    div(mul(vec![x, d(&b)]), b),
                ]),
            ])
        }
        Expr::Fn(name, args) => match (&**name, args.len()) {
            (_, 1) => match outer_derivative(name, &args[0]) {
                Some(outer) => mul(vec![outer, d(&args[0])]),
                None => unevaluated(e, var),
            },
            ("tet", 2) => {
                let (b, h) = (&args[0], &args[1]);
                match h.as_i64() {
                    Some(n) if (1..=64).contains(&n) && !h.has_sym(var) => {
                        // b^^n = b^(b^^(n-1))
                        let lower = func("tet", vec![b.clone(), Expr::int(n - 1)]);
                        let inner = add(vec![
                            mul(vec![d(&lower), f1("ln", b)]),
                            div(mul(vec![lower, d(b)]), b.clone()),
                        ]);
                        mul(vec![e.clone(), inner])
                    }
                    _ => unevaluated(e, var),
                }
            }
            ("atan2", 2) => {
                let (y, x) = (&args[0], &args[1]);
                div(
                    sub(mul(vec![x.clone(), d(y)]), mul(vec![y.clone(), d(x)])),
                    add(vec![powi(x.clone(), 2), powi(y.clone(), 2)]),
                )
            }
            ("lambertw", 2) if !args[0].has_sym(var) => {
                let (w, u) = (e.clone(), &args[1]);
                mul(vec![
                    div(w.clone(), mul(vec![u.clone(), add(vec![Expr::one(), w])])),
                    d(u),
                ])
            }
            ("polygamma", 2) if !args[0].has_sym(var) => mul(vec![
                func(
                    "polygamma",
                    vec![add(vec![args[0].clone(), Expr::one()]), args[1].clone()],
                ),
                d(&args[1]),
            ]),
            ("piecewise", _) => crate::logic::map_values(args, &mut |v| Some(d(v)))
                .unwrap_or_else(|| unevaluated(e, var)),
            ("integrate", 2) if args[1].is_sym(var) => args[0].clone(),
            ("integrate", 4) => {
                let (f, v, lo, hi) = (&args[0], &args[1], &args[2], &args[3]);
                let at = |p: &Expr| crate::canon::subst(f, v, p);
                let mut terms = vec![mul(vec![at(hi), d(hi)]), neg(mul(vec![at(lo), d(lo)]))];
                if !v.is_sym(var) && f.has_sym(var) {
                    terms.push(Expr::call(
                        "integrate",
                        vec![d(f), v.clone(), lo.clone(), hi.clone()],
                    ));
                }
                add(terms)
            }
            _ => {
                // the chain rule over the arguments that hold the variable
                let mut terms = Vec::new();
                for (k, arg) in args.iter().enumerate() {
                    if !arg.has_sym(var) {
                        continue;
                    }
                    match crate::specfn::partial(name, args, k) {
                        Some(outer) => terms.push(mul(vec![outer, d(arg)])),
                        None => return unevaluated(e, var),
                    }
                }
                add(terms)
            }
        },
        Expr::List(a) => Expr::list(a.iter().map(d).collect()),
        Expr::Rel(op, a, b) => Expr::rel(*op, d(a), d(b)),
        _ => Expr::zero(),
    }
}

pub fn diff_n(e: &Expr, var: &str, n: u32) -> Expr {
    let mut out = e.clone();
    for _ in 0..n {
        out = crate::simplify::simplify(&diff(&out, var));
        if out.is_zero() {
            break;
        }
    }
    out
}
