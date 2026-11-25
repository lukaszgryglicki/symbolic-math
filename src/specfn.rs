//! Exact values and derivatives of the special functions: Bessel and Airy functions, incomplete
//! gamma and beta functions, elliptic integrals and functions, hypergeometric functions,
//! polylogarithms, orthogonal polynomials.

use crate::canon::{add, div, mul, neg, pow, powi, sqrt, sub};
use crate::expr::Expr;
use crate::funcs::{func, is_neg_form};
use crate::numeric;
use crate::simplify::simplify;
use rug::Integer;

fn f1(name: &str, x: Expr) -> Expr {
    func(name, vec![x])
}

fn f2(name: &str, x: Expr, y: Expr) -> Expr {
    func(name, vec![x, y])
}

fn int(k: i64) -> Expr {
    Expr::int(k)
}

fn one() -> Expr {
    Expr::one()
}

fn factorial(n: i64) -> Expr {
    Expr::integer(Integer::from(Integer::factorial(n as u32)))
}

/// A constant with a positive real value.
fn positive(e: &Expr) -> bool {
    e.is_const() && numeric::sign(e).is_some_and(|s| s > 0)
}

/// `2 nu` for a half-integer order `nu`.
fn half_integer(e: &Expr) -> Option<i64> {
    let r = e.as_rat()?;
    (*r.denom() == 2).then(|| r.numer().to_i64()).flatten()
}

fn minus_one_pow(n: i64) -> Expr {
    int(if n % 2 == 0 { 1 } else { -1 })
}

/// Bessel functions of half-integer order by their elementary forms; `kind` is j, y, i or k.
fn half_order(kind: char, twice: i64, z: &Expr) -> Option<Expr> {
    if twice.unsigned_abs() > 21 {
        return None;
    }
    let inv = powi(z.clone(), -1);
    let front = |c: Expr| mul(vec![c, pow(z.clone(), Expr::rat(-1, 2))]);
    let two_over_pi = sqrt(div(int(2), Expr::pi()));
    match kind {
        'y' => {
            // Y_(n+1/2) = (-1)^(n+1) J_(-n-1/2)
            let n = (twice - 1).div_euclid(2);
            let j = half_order('j', -twice, z)?;
            Some(mul(vec![minus_one_pow(n + 1), j]))
        }
        'k' => {
            // K_(1/2) = sqrt(pi/(2z)) e^(-z), K_(nu+1) = K_(nu-1) + (2 nu/z) K_nu
            let twice = twice.abs();
            let (mut prev, mut cur) = (one(), one());
            let mut t = 1;
            while t < twice {
                let next = crate::expand::expand(&add(vec![
                    prev.clone(),
                    mul(vec![int(t), inv.clone(), cur.clone()]),
                ]));
                prev = cur;
                cur = next;
                t += 2;
            }
            Some(mul(vec![
                front(sqrt(div(Expr::pi(), int(2)))),
                pow(Expr::e(), neg(z.clone())),
                cur,
            ]))
        }
        _ => {
            let (s, c) = if kind == 'j' {
                (f1("sin", z.clone()), f1("cos", z.clone()))
            } else {
                (f1("sinh", z.clone()), f1("cosh", z.clone()))
            };
            // from the orders 1/2 (sin) and -1/2 (cos) by the recurrence in the order
            let (mut low, mut high) = (c, s);
            let mut t = 1;
            if twice > 0 {
                while t < twice {
                    // J_(nu+1) = (2 nu/z) J_nu - J_(nu-1); I_(nu+1) = I_(nu-1) - (2 nu/z) I_nu
                    let scaled = mul(vec![int(t), inv.clone(), high.clone()]);
                    let next = crate::expand::expand(&if kind == 'j' {
                        sub(scaled, low.clone())
                    } else {
                        sub(low.clone(), scaled)
                    });
                    low = high;
                    high = next;
                    t += 2;
                }
                Some(mul(vec![front(two_over_pi), high]))
            } else {
                t = -1;
                while t > twice {
                    // J_(nu-1) = (2 nu/z) J_nu - J_(nu+1); I_(nu-1) = I_(nu+1) + (2 nu/z) I_nu
                    let scaled = mul(vec![int(t), inv.clone(), low.clone()]);
                    let next = crate::expand::expand(&if kind == 'j' {
                        sub(scaled, high.clone())
                    } else {
                        add(vec![high.clone(), scaled])
                    });
                    high = low;
                    low = next;
                    t -= 2;
                }
                Some(mul(vec![front(two_over_pi), low]))
            }
        }
    }
}

fn spherical(second: bool, n: i64, z: &Expr) -> Option<Expr> {
    if !(0..=12).contains(&n) {
        return None;
    }
    let inv = powi(z.clone(), -1);
    let (s, c) = (f1("sin", z.clone()), f1("cos", z.clone()));
    // j_0 = sin/z, j_(-1) = cos/z; y_0 = -cos/z, y_(-1) = sin/z
    let (mut prev, mut cur) = if second {
        (mul(vec![s, inv.clone()]), neg(mul(vec![c, inv.clone()])))
    } else {
        (mul(vec![c, inv.clone()]), mul(vec![s, inv.clone()]))
    };
    for k in 0..n {
        let next = crate::expand::expand(&sub(
            mul(vec![int(2 * k + 1), inv.clone(), cur.clone()]),
            prev.clone(),
        ));
        prev = cur;
        cur = next;
    }
    Some(cur)
}

fn bessel(name: &str, nu: &Expr, z: &Expr) -> Option<Expr> {
    let kind = match name {
        "besselj" => 'j',
        "bessely" => 'y',
        "besseli" => 'i',
        _ => 'k',
    };
    if z.is_zero() && matches!(kind, 'j' | 'i') {
        if nu.is_zero() {
            return Some(one());
        }
        if nu.as_int().is_some() || positive(nu) {
            return Some(Expr::zero());
        }
    }
    if let Some(n) = nu.as_i64() {
        if n < 0 {
            let flipped = f2(name, int(n.checked_neg()?), z.clone());
            return Some(match kind {
                'j' | 'y' => mul(vec![minus_one_pow(n), flipped]),
                _ => flipped,
            });
        }
        if matches!(kind, 'j' | 'i') && is_neg_form(z) && !z.is_number() {
            return Some(mul(vec![
                minus_one_pow(n),
                f2(name, nu.clone(), neg(z.clone())),
            ]));
        }
    }
    if kind == 'k'
        && nu
            .as_rat()
            .is_some_and(|r| r.cmp0() == std::cmp::Ordering::Less)
    {
        return Some(f2(name, neg(nu.clone()), z.clone()));
    }
    half_order(kind, half_integer(nu)?, z)
}

/// `(n - 1)! e^(-z) sum_{k < n} z^k/k!`.
fn gamma_sum(n: i64, z: &Expr) -> Expr {
    mul(vec![
        factorial(n - 1),
        pow(Expr::e(), neg(z.clone())),
        add((0..n)
            .map(|k| div(powi(z.clone(), k), factorial(k)))
            .collect()),
    ])
}

fn incomplete_gamma(name: &str, a: &Expr, z: &Expr) -> Option<Expr> {
    let upper = matches!(name, "uppergamma" | "gammaq");
    let regular = matches!(name, "gammaq" | "gammap");
    let whole = |v: Expr| {
        if regular {
            div(v, f1("gamma", a.clone()))
        } else {
            v
        }
    };
    let full = || {
        if regular {
            one()
        } else {
            f1("gamma", a.clone())
        }
    };
    // at 0 the integral is all of gamma(a) for a positive a, which a symbol is taken to be
    let not_negative = if a.is_const() {
        positive(a)
    } else {
        !crate::assume::is_negative(a)
    };
    if z.is_zero() && not_negative {
        return Some(if upper { full() } else { Expr::zero() });
    }
    if z.is_inf() {
        return Some(if upper { Expr::zero() } else { full() });
    }
    if let Some(n) = a.as_i64().filter(|n| (1..=24).contains(n)) {
        let tail = gamma_sum(n, z);
        return Some(whole(if upper {
            tail
        } else {
            sub(factorial(n - 1), tail)
        }));
    }
    if *a == Expr::half() {
        let root = sqrt(z.clone());
        let v = mul(vec![
            sqrt(Expr::pi()),
            f1(if upper { "erfc" } else { "erf" }, root),
        ]);
        return Some(whole(v));
    }
    None
}

/// `Li_(-n)(z)` as a rational function.
fn polylog_rational(n: i64, z: &Expr) -> Expr {
    let t = Expr::sym("#p");
    let mut f = div(t.clone(), sub(one(), t.clone()));
    for _ in 0..n {
        f = crate::ratsimp::together(&mul(vec![t.clone(), crate::diff::diff(&f, "#p")]));
    }
    crate::canon::subst(&f, &t, z)
}

fn polylog(s: &Expr, z: &Expr) -> Option<Expr> {
    if z.is_zero() {
        return Some(Expr::zero());
    }
    if let Some(n) = s.as_i64() {
        match n {
            1 => return Some(neg(f1("ln", sub(one(), z.clone())))),
            2 => return Some(f1("li2", z.clone())),
            0 => return Some(div(z.clone(), sub(one(), z.clone()))),
            n if (-12..0).contains(&n) => return Some(polylog_rational(-n, z)),
            _ => {}
        }
    }
    if z.is_one() && s.is_const() && numeric::sign(&sub(s.clone(), one())).is_some_and(|x| x > 0) {
        return Some(f1("zeta", s.clone()));
    }
    if z.is_minus_one() && s.is_const() && !s.is_one() {
        // -(1 - 2^(1-s)) zeta(s)
        return Some(neg(mul(vec![
            sub(one(), pow(int(2), sub(one(), s.clone()))),
            f1("zeta", s.clone()),
        ])));
    }
    None
}

fn hurwitz(s: &Expr, a: &Expr) -> Option<Expr> {
    if a.is_one() {
        return Some(f1("zeta", s.clone()));
    }
    if *a == Expr::half() {
        return Some(mul(vec![
            sub(pow(int(2), s.clone()), one()),
            f1("zeta", s.clone()),
        ]));
    }
    if s.is_zero() {
        return Some(sub(Expr::half(), a.clone()));
    }
    if let Some(n) = a.as_i64().filter(|n| (2..=200).contains(n)) {
        let head: Vec<Expr> = (1..n).map(|k| pow(int(k), neg(s.clone()))).collect();
        return Some(sub(f1("zeta", s.clone()), add(head)));
    }
    None
}

/// A hypergeometric series that ends: the polynomial, for the first upper parameter `-n`.
fn hyper_polynomial(upper: &[Expr], lower: &[Expr], z: &Expr) -> Option<Expr> {
    let n = upper
        .iter()
        .filter_map(Expr::as_i64)
        .filter(|n| (-60..=0).contains(n))
        .max()?;
    if lower
        .iter()
        .any(|b| b.as_i64().is_some_and(|m| m <= 0 && m >= n))
    {
        return None;
    }
    // (-n)_k/k! = (-1)^k binomial(n, k); the other parameters stay as rising products
    let degree = -n;
    let ending = upper.iter().position(|a| a.as_i64() == Some(n))?;
    let rising = |a: &Expr, k: i64| mul((0..k).map(|j| add(vec![a.clone(), int(j)])).collect());
    let mut binomial = Integer::from(1);
    let mut terms = vec![one()];
    for k in 1..=degree {
        binomial = binomial * (degree - k + 1) / k;
        let sign = if k % 2 == 0 { 1 } else { -1 };
        let top = mul(upper
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != ending)
            .map(|(_, a)| rising(a, k))
            .collect());
        let low = mul(lower.iter().map(|b| rising(b, k)).collect());
        terms.push(mul(vec![
            Expr::integer(binomial.clone() * sign),
            top,
            powi(low, -1),
            powi(z.clone(), k),
        ]));
    }
    Some(add(terms))
}

fn hypergeometric(upper: &[Expr], lower: &[Expr], z: &Expr) -> Option<Expr> {
    if z.is_zero() || upper.iter().any(Expr::is_zero) {
        return Some(one());
    }
    // a parameter above and below cancels
    for (i, a) in upper.iter().enumerate() {
        if let Some(j) = lower.iter().position(|b| b == a) {
            let mut up = upper.to_vec();
            let mut low = lower.to_vec();
            up.remove(i);
            low.remove(j);
            return Some(func(
                "hypergeometric",
                vec![Expr::list(up), Expr::list(low), z.clone()],
            ));
        }
    }
    if let Some(p) = hyper_polynomial(upper, lower, z) {
        return Some(p);
    }
    match (upper, lower) {
        ([], []) => Some(pow(Expr::e(), z.clone())),
        ([a], []) => Some(pow(sub(one(), z.clone()), neg(a.clone()))),
        ([a, b], [c]) if a.is_one() && b.is_one() && *c == int(2) => {
            Some(neg(div(f1("ln", sub(one(), z.clone())), z.clone())))
        }
        ([a], [b]) if a.is_one() && *b == int(2) => {
            Some(div(sub(pow(Expr::e(), z.clone()), one()), z.clone()))
        }
        _ => None,
    }
}

fn named_hypergeometric(name: &str, a: &[Expr]) -> Option<Expr> {
    let (upper, lower): (&[Expr], &[Expr]) = match (name, a.len()) {
        ("hyp0f1", 2) => (&[], &a[..1]),
        ("hyp1f1", 3) => (&a[..1], &a[1..2]),
        ("hyp2f1", 4) => (&a[..2], &a[2..3]),
        _ => return None,
    };
    let z = &a[a.len() - 1];
    let reduced = hypergeometric(upper, lower, z)?;
    // the general function goes back to its own name
    Some(match reduced.fn_args("hypergeometric") {
        Some([Expr::List(up), Expr::List(low), z]) => match (up.len(), low.len()) {
            (0, 1) => func("hyp0f1", vec![low[0].clone(), z.clone()]),
            (1, 1) => func("hyp1f1", vec![up[0].clone(), low[0].clone(), z.clone()]),
            (2, 1) => func(
                "hyp2f1",
                vec![up[0].clone(), up[1].clone(), low[0].clone(), z.clone()],
            ),
            _ => reduced,
        },
        _ => reduced,
    })
}

fn elliptic(name: &str, a: &[Expr]) -> Option<Expr> {
    let half_pi = div(Expr::pi(), int(2));
    Some(match (name, a) {
        ("elliptick", [m]) if m.is_zero() => half_pi,
        ("elliptick", [m]) if m.is_one() => Expr::inf(),
        ("elliptick", [m]) if *m == Expr::half() => div(
            powi(f1("gamma", Expr::rat(1, 4)), 2),
            mul(vec![int(4), sqrt(Expr::pi())]),
        ),
        ("elliptick", [m]) if m.is_minus_one() => div(
            powi(f1("gamma", Expr::rat(1, 4)), 2),
            mul(vec![int(4), sqrt(mul(vec![int(2), Expr::pi()]))]),
        ),
        ("elliptice", [m]) if m.is_zero() => half_pi,
        ("elliptice", [m]) if m.is_one() => one(),
        ("ellipticf" | "elliptice", [phi, _]) if phi.is_zero() => Expr::zero(),
        ("ellipticf" | "elliptice", [phi, m]) if m.is_zero() => phi.clone(),
        ("ellipticf", [phi, m]) if *phi == half_pi => f1("elliptick", m.clone()),
        ("elliptice", [phi, m]) if *phi == half_pi => f1("elliptice", m.clone()),
        ("elliptice", [phi, m]) if m.is_one() && phi.is_const() => {
            // sin(phi) between -pi/2 and pi/2
            let inside = numeric::real_value(phi, 20)
                .is_some_and(|v| v.clone().abs() <= std::f64::consts::FRAC_PI_2);
            if !inside {
                return None;
            }
            f1("sin", phi.clone())
        }
        ("ellipticpi", [n, m]) if n.is_zero() => f1("elliptick", m.clone()),
        ("ellipticpi", [n, m]) if m.is_zero() => div(half_pi, sqrt(sub(one(), n.clone()))),
        ("ellipticpi", [n, phi, m]) if n.is_zero() => f2("ellipticf", phi.clone(), m.clone()),
        ("ellipticpi", [n, phi, m]) if *phi == half_pi => f2("ellipticpi", n.clone(), m.clone()),
        ("ellipticpi", [_, phi, _]) if phi.is_zero() => Expr::zero(),
        ("ellipticnome", [m]) if m.is_zero() => Expr::zero(),
        ("ellipticnome", [m]) if m.is_one() => one(),
        ("ellipticnome", [m]) if *m == Expr::half() => pow(Expr::e(), neg(Expr::pi())),
        ("jacobisn", [u, m]) => match () {
            _ if u.is_zero() => Expr::zero(),
            _ if m.is_zero() => f1("sin", u.clone()),
            _ if m.is_one() => f1("tanh", u.clone()),
            _ if is_neg_form(u) && !u.is_number() => neg(f2(name, neg(u.clone()), m.clone())),
            _ => return None,
        },
        ("jacobicn" | "jacobidn", [u, m]) => match () {
            _ if u.is_zero() => one(),
            _ if m.is_zero() && name == "jacobicn" => f1("cos", u.clone()),
            _ if m.is_zero() => one(),
            _ if m.is_one() => powi(f1("cosh", u.clone()), -1),
            _ if is_neg_form(u) && !u.is_number() => f2(name, neg(u.clone()), m.clone()),
            _ => return None,
        },
        ("elliptictheta", [k, _, q]) if q.is_zero() => match k.as_i64()? {
            1 | 2 => Expr::zero(),
            3 | 4 => one(),
            _ => return None,
        },
        _ => return None,
    })
}

fn step(name: &str, x: &Expr) -> Option<Expr> {
    let sign = if x.is_const() {
        numeric::sign(x)
    } else {
        crate::assume::sign(x)
    };
    match (name, sign) {
        ("diracdelta", Some(0)) => None,
        ("diracdelta", Some(_)) => Some(Expr::zero()),
        ("diracdelta", None) if crate::assume::is_nonzero(x) => Some(Expr::zero()),
        // the delta is even
        ("diracdelta", None) => is_neg_form(x).then(|| f1("diracdelta", neg(x.clone()))),
        (_, Some(s)) if s > 0 => Some(one()),
        (_, Some(s)) if s < 0 => Some(Expr::zero()),
        ("heaviside", Some(_)) => Some(Expr::half()),
        ("unitstep", Some(_)) => Some(one()),
        _ => None,
    }
}

/// An odd function at an argument printed with a minus sign.
fn odd(name: &str, x: &Expr) -> Option<Expr> {
    (is_neg_form(x) && !x.is_number()).then(|| neg(f1(name, neg(x.clone()))))
}

/// The value of a special function where it has a simpler exact form.
pub fn value(name: &str, a: &[Expr]) -> Option<Expr> {
    match (name, a) {
        ("besselj" | "bessely" | "besseli" | "besselk", [nu, z]) => bessel(name, nu, z),
        ("sphericalbesselj" | "sphericalbessely", [n, z]) => {
            if z.is_zero() && name == "sphericalbesselj" {
                if n.is_zero() {
                    return Some(one());
                }
                if positive(n) {
                    return Some(Expr::zero());
                }
            }
            spherical(name == "sphericalbessely", n.as_i64()?, z)
        }
        ("airyai", [z]) if z.is_zero() => Some(div(
            pow(int(3), Expr::rat(-2, 3)),
            f1("gamma", Expr::rat(2, 3)),
        )),
        ("airybi", [z]) if z.is_zero() => Some(div(
            pow(int(3), Expr::rat(-1, 6)),
            f1("gamma", Expr::rat(2, 3)),
        )),
        ("airyaiprime", [z]) if z.is_zero() => Some(neg(div(
            pow(int(3), Expr::rat(-1, 3)),
            f1("gamma", Expr::rat(1, 3)),
        ))),
        ("airybiprime", [z]) if z.is_zero() => Some(div(
            pow(int(3), Expr::rat(1, 6)),
            f1("gamma", Expr::rat(1, 3)),
        )),
        ("airyai" | "airyaiprime", [z]) if z.is_inf() => Some(Expr::zero()),
        ("airybi" | "airybiprime", [z]) if z.is_inf() => Some(Expr::inf()),
        ("struveh" | "struvel", [nu, z]) if z.is_zero() && (nu.is_zero() || positive(nu)) => {
            Some(Expr::zero())
        }
        ("uppergamma", [a, z0, z1]) => Some(sub(
            f2("uppergamma", a.clone(), z0.clone()),
            f2("uppergamma", a.clone(), z1.clone()),
        )),
        ("uppergamma" | "lowergamma" | "gammaq" | "gammap", [a, z]) => incomplete_gamma(name, a, z),
        ("expint", [n, z]) => {
            if n.is_zero() {
                return Some(div(pow(Expr::e(), neg(z.clone())), z.clone()));
            }
            if z.is_zero() && n.is_const() {
                let above = numeric::sign(&sub(n.clone(), one()))?;
                return Some(if above > 0 {
                    powi(sub(n.clone(), one()), -1)
                } else {
                    Expr::inf()
                });
            }
            None
        }
        ("betainc" | "betaregularized", [z, a, b]) => {
            if z.is_zero() && positive(a) {
                return Some(Expr::zero());
            }
            if z.is_one() && positive(b) {
                return Some(if name == "betainc" {
                    f2("beta", a.clone(), b.clone())
                } else {
                    one()
                });
            }
            None
        }
        ("erfinv", [x]) => match () {
            _ if x.is_zero() => Some(Expr::zero()),
            _ if x.is_one() => Some(Expr::inf()),
            _ if x.is_minus_one() => Some(Expr::neg_inf()),
            _ => odd(name, x),
        },
        ("erfcinv", [x]) => match () {
            _ if x.is_one() => Some(Expr::zero()),
            _ if x.is_zero() => Some(Expr::inf()),
            _ if *x == int(2) => Some(Expr::neg_inf()),
            _ => None,
        },
        ("fresnels" | "fresnelc", [x]) => match () {
            _ if x.is_zero() => Some(Expr::zero()),
            _ if x.is_inf() => Some(Expr::half()),
            _ if x.is_neg_inf() => Some(Expr::rat(-1, 2)),
            _ => odd(name, x),
        },
        ("sinc", [x]) => match () {
            _ if x.is_zero() => Some(one()),
            _ if x.is_infinite() => Some(Expr::zero()),
            _ if x.is_const() => Some(div(f1("sin", x.clone()), x.clone())),
            _ if is_neg_form(x) => Some(f1(name, neg(x.clone()))),
            _ => None,
        },
        ("heaviside" | "unitstep" | "diracdelta", [x]) => step(name, x),
        ("li2", [x]) => {
            let pi2 = |p: i64, q: i64| mul(vec![Expr::rat(p, q), powi(Expr::pi(), 2)]);
            match () {
                _ if x.is_zero() => Some(Expr::zero()),
                _ if x.is_one() => Some(pi2(1, 6)),
                _ if x.is_minus_one() => Some(pi2(-1, 12)),
                _ if *x == Expr::half() => {
                    Some(sub(pi2(1, 12), div(powi(f1("ln", int(2)), 2), int(2))))
                }
                _ if *x == int(2) => Some(sub(
                    pi2(1, 4),
                    mul(vec![Expr::i(), Expr::pi(), f1("ln", int(2))]),
                )),
                _ => None,
            }
        }
        ("polylog", [s, z]) => polylog(s, z),
        // Lerch's transcendent where it is a function with a name of its own
        ("lerchphi", [z, s, a]) => {
            if z.is_zero() {
                Some(pow(a.clone(), neg(s.clone())))
            } else if z.is_one() {
                Some(Expr::call("zeta", vec![s.clone(), a.clone()]))
            } else if a.is_one() {
                Some(div(
                    Expr::call("polylog", vec![s.clone(), z.clone()]),
                    z.clone(),
                ))
            } else {
                None
            }
        }
        ("zeta", [s, a]) => hurwitz(s, a),
        ("dirichleteta", [s]) => Some(if s.is_one() {
            f1("ln", int(2))
        } else {
            mul(vec![
                sub(one(), pow(int(2), sub(one(), s.clone()))),
                f1("zeta", s.clone()),
            ])
        }),
        ("hyp0f1" | "hyp1f1" | "hyp2f1", _) => named_hypergeometric(name, a),
        ("hypergeometric", [Expr::List(up), Expr::List(low), z]) => {
            match hypergeometric(up, low, z) {
                Some(v) => Some(v),
                None => match (up.len(), low.len()) {
                    (0, 1) => Some(func("hyp0f1", vec![low[0].clone(), z.clone()])),
                    (1, 1) => Some(func(
                        "hyp1f1",
                        vec![up[0].clone(), low[0].clone(), z.clone()],
                    )),
                    (2, 1) => Some(func(
                        "hyp2f1",
                        vec![up[0].clone(), up[1].clone(), low[0].clone(), z.clone()],
                    )),
                    _ => None,
                },
            }
        }
        ("hyperu", [a, b, z]) => {
            if a.is_zero() {
                return Some(one());
            }
            if simplify(&sub(b.clone(), add(vec![a.clone(), one()]))).is_zero() {
                return Some(pow(z.clone(), neg(a.clone())));
            }
            None
        }
        (
            "elliptick" | "elliptice" | "ellipticf" | "ellipticpi" | "ellipticnome" | "jacobisn"
            | "jacobicn" | "jacobidn" | "elliptictheta",
            _,
        ) => elliptic(name, a),
        (
            "legendrep" | "legendreq" | "chebyshevt" | "chebyshevu" | "hermiteh" | "laguerrel"
            | "gegenbauerc" | "jacobip" | "sphericalharmonicy",
            _,
        ) => crate::orthopoly::symbolic(name, a),
        _ => None,
    }
}

/// The derivative of a function of several arguments with respect to argument `at`.
pub fn partial(name: &str, a: &[Expr], at: usize) -> Option<Expr> {
    let shifted = |k: usize, by: i64| add(vec![a[k].clone(), int(by)]);
    let with = |pairs: &[(usize, Expr)]| {
        let mut args = a.to_vec();
        for (k, v) in pairs {
            args[*k] = v.clone();
        }
        func(name, args)
    };
    let half = Expr::half;
    Some(match (name, a.len(), at) {
        ("besselj" | "bessely" | "hankel1" | "hankel2", 2, 1) => mul(vec![
            half(),
            sub(with(&[(0, shifted(0, -1))]), with(&[(0, shifted(0, 1))])),
        ]),
        ("besseli", 2, 1) => mul(vec![
            half(),
            add(vec![
                with(&[(0, shifted(0, -1))]),
                with(&[(0, shifted(0, 1))]),
            ]),
        ]),
        ("besselk", 2, 1) => mul(vec![
            Expr::rat(-1, 2),
            add(vec![
                with(&[(0, shifted(0, -1))]),
                with(&[(0, shifted(0, 1))]),
            ]),
        ]),
        ("sphericalbesselj" | "sphericalbessely", 2, 1) => sub(
            with(&[(0, shifted(0, -1))]),
            mul(vec![shifted(0, 1), powi(a[1].clone(), -1), with(&[])]),
        ),
        ("struveh" | "struvel", 2, 1) => sub(
            with(&[(0, shifted(0, -1))]),
            mul(vec![a[0].clone(), powi(a[1].clone(), -1), with(&[])]),
        ),
        ("uppergamma" | "lowergamma" | "gammaq" | "gammap", 2, 1) => {
            let density = mul(vec![
                pow(a[1].clone(), sub(a[0].clone(), one())),
                pow(Expr::e(), neg(a[1].clone())),
            ]);
            let scaled = if matches!(name, "gammaq" | "gammap") {
                div(density, f1("gamma", a[0].clone()))
            } else {
                density
            };
            if matches!(name, "uppergamma" | "gammaq") {
                neg(scaled)
            } else {
                scaled
            }
        }
        ("expint", 2, 1) => neg(with(&[(0, shifted(0, -1))])),
        ("betainc" | "betaregularized", 3, 0) => {
            let density = mul(vec![
                pow(a[0].clone(), sub(a[1].clone(), one())),
                pow(sub(one(), a[0].clone()), sub(a[2].clone(), one())),
            ]);
            if name == "betaregularized" {
                div(density, f2("beta", a[1].clone(), a[2].clone()))
            } else {
                density
            }
        }
        ("polylog", 2, 1) => div(with(&[(0, shifted(0, -1))]), a[1].clone()),
        // (phi(z, s - 1, a) - a*phi(z, s, a))/z, and -s*phi(z, s + 1, a)
        ("lerchphi", 3, 0) => div(
            sub(
                with(&[(1, shifted(1, -1))]),
                mul(vec![a[2].clone(), with(&[])]),
            ),
            a[0].clone(),
        ),
        ("lerchphi", 3, 2) => neg(mul(vec![a[1].clone(), with(&[(1, shifted(1, 1))])])),
        ("zeta", 2, 1) => neg(mul(vec![a[0].clone(), with(&[(0, shifted(0, 1))])])),
        ("ellipticf", 2, 0) => powi(
            sqrt(sub(
                one(),
                mul(vec![a[1].clone(), powi(f1("sin", a[0].clone()), 2)]),
            )),
            -1,
        ),
        ("elliptice", 2, 0) => sqrt(sub(
            one(),
            mul(vec![a[1].clone(), powi(f1("sin", a[0].clone()), 2)]),
        )),
        ("elliptice", 2, 1) => div(
            sub(with(&[]), f2("ellipticf", a[0].clone(), a[1].clone())),
            mul(vec![int(2), a[1].clone()]),
        ),
        ("ellipticf", 2, 1) => {
            let (phi, m) = (&a[0], &a[1]);
            let root = sqrt(sub(
                one(),
                mul(vec![m.clone(), powi(f1("sin", phi.clone()), 2)]),
            ));
            let one_m = sub(one(), m.clone());
            add(vec![
                div(
                    f2("elliptice", phi.clone(), m.clone()),
                    mul(vec![int(2), m.clone(), one_m.clone()]),
                ),
                neg(div(with(&[]), mul(vec![int(2), m.clone()]))),
                neg(div(
                    f1("sin", mul(vec![int(2), phi.clone()])),
                    mul(vec![int(4), one_m, root]),
                )),
            ])
        }
        ("ellipticpi", 3, 1) => {
            let s2 = powi(f1("sin", a[1].clone()), 2);
            powi(
                mul(vec![
                    sub(one(), mul(vec![a[0].clone(), s2.clone()])),
                    sqrt(sub(one(), mul(vec![a[2].clone(), s2]))),
                ]),
                -1,
            )
        }
        ("jacobisn", 2, 0) => mul(vec![
            f2("jacobicn", a[0].clone(), a[1].clone()),
            f2("jacobidn", a[0].clone(), a[1].clone()),
        ]),
        ("jacobicn", 2, 0) => neg(mul(vec![
            f2("jacobisn", a[0].clone(), a[1].clone()),
            f2("jacobidn", a[0].clone(), a[1].clone()),
        ])),
        ("jacobidn", 2, 0) => neg(mul(vec![
            a[1].clone(),
            f2("jacobisn", a[0].clone(), a[1].clone()),
            f2("jacobicn", a[0].clone(), a[1].clone()),
        ])),
        ("hyp0f1", 2, 1) => div(with(&[(0, shifted(0, 1))]), a[0].clone()),
        ("hyp1f1", 3, 2) => mul(vec![
            div(a[0].clone(), a[1].clone()),
            with(&[(0, shifted(0, 1)), (1, shifted(1, 1))]),
        ]),
        ("hyperu", 3, 2) => neg(mul(vec![
            a[0].clone(),
            with(&[(0, shifted(0, 1)), (1, shifted(1, 1))]),
        ])),
        ("hyp2f1", 4, 3) => mul(vec![
            div(mul(vec![a[0].clone(), a[1].clone()]), a[2].clone()),
            with(&[(0, shifted(0, 1)), (1, shifted(1, 1)), (2, shifted(2, 1))]),
        ]),
        ("hypergeometric", 3, 2) => {
            let (up, low) = (a[0].as_list()?, a[1].as_list()?);
            let bump = |items: &[Expr]| {
                Expr::list(items.iter().map(|x| add(vec![x.clone(), one()])).collect())
            };
            mul(vec![
                div(mul(up.to_vec()), mul(low.to_vec())),
                func(name, vec![bump(up), bump(low), a[2].clone()]),
            ])
        }
        ("legendrep" | "legendreq", 2, 1) => {
            let (n, x) = (&a[0], &a[1]);
            div(
                mul(vec![
                    n.clone(),
                    sub(
                        mul(vec![x.clone(), with(&[])]),
                        with(&[(0, shifted(0, -1))]),
                    ),
                ]),
                sub(powi(x.clone(), 2), one()),
            )
        }
        ("chebyshevt", 2, 1) => mul(vec![
            a[0].clone(),
            f2("chebyshevu", shifted(0, -1), a[1].clone()),
        ]),
        ("chebyshevu", 2, 1) => {
            let (n, x) = (&a[0], &a[1]);
            div(
                sub(
                    mul(vec![
                        add(vec![n.clone(), one()]),
                        f2("chebyshevt", shifted(0, 1), x.clone()),
                    ]),
                    mul(vec![x.clone(), with(&[])]),
                ),
                sub(powi(x.clone(), 2), one()),
            )
        }
        ("hermiteh", 2, 1) => mul(vec![int(2), a[0].clone(), with(&[(0, shifted(0, -1))])]),
        ("laguerrel", 2, 1) => neg(func(name, vec![shifted(0, -1), one(), a[1].clone()])),
        ("laguerrel", 3, 2) => neg(with(&[(0, shifted(0, -1)), (1, shifted(1, 1))])),
        ("gegenbauerc", 3, 2) => mul(vec![
            int(2),
            a[1].clone(),
            with(&[(0, shifted(0, -1)), (1, shifted(1, 1))]),
        ]),
        ("jacobip", 4, 3) => mul(vec![
            div(
                add(vec![a[0].clone(), a[1].clone(), a[2].clone(), one()]),
                int(2),
            ),
            with(&[(0, shifted(0, -1)), (1, shifted(1, 1)), (2, shifted(2, 1))]),
        ]),
        _ => return None,
    })
}

/// Derivatives of the special functions of one argument.
pub fn derivative(name: &str, u: &Expr) -> Option<Expr> {
    let sq = |e: &Expr| powi(e.clone(), 2);
    let half_pi_square = || mul(vec![Expr::pi(), sq(u), Expr::half()]);
    Some(match name {
        "airyai" => f1("airyaiprime", u.clone()),
        "airybi" => f1("airybiprime", u.clone()),
        "airyaiprime" => mul(vec![u.clone(), f1("airyai", u.clone())]),
        "airybiprime" => mul(vec![u.clone(), f1("airybi", u.clone())]),
        "erfinv" => mul(vec![
            sqrt(Expr::pi()),
            Expr::half(),
            pow(Expr::e(), sq(&f1("erfinv", u.clone()))),
        ]),
        "erfcinv" => neg(mul(vec![
            sqrt(Expr::pi()),
            Expr::half(),
            pow(Expr::e(), sq(&f1("erfcinv", u.clone()))),
        ])),
        "fresnels" => f1("sin", half_pi_square()),
        "fresnelc" => f1("cos", half_pi_square()),
        "sinc" => div(
            sub(
                mul(vec![u.clone(), f1("cos", u.clone())]),
                f1("sin", u.clone()),
            ),
            sq(u),
        ),
        "heaviside" | "unitstep" => f1("diracdelta", u.clone()),
        "elliptick" => {
            let one_m = sub(one(), u.clone());
            div(
                sub(
                    f1("elliptice", u.clone()),
                    mul(vec![one_m.clone(), f1("elliptick", u.clone())]),
                ),
                mul(vec![int(2), u.clone(), one_m]),
            )
        }
        "elliptice" => div(
            sub(f1("elliptice", u.clone()), f1("elliptick", u.clone())),
            mul(vec![int(2), u.clone()]),
        ),
        "dirichleteta" => return None,
        _ => return None,
    })
}
