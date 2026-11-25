//! Orthogonal polynomials: explicit forms for whole degrees, hypergeometric values otherwise.

use crate::canon::{add, div, mul, neg, pow, powi, sub};
use crate::expand::expand;
use crate::expr::Expr;
use crate::funcs::func;
use crate::hyper::{hyp1f1, hyp2f1};
use crate::numeric::SafeExp;
use crate::special::{Out, gamma_any, pi, rgamma};
use rug::{Complex, Float};

/// Largest degree that is written out.
const MAX_DEGREE: i64 = 400;

fn tidy(e: Expr, symbolic: bool) -> Expr {
    if symbolic {
        expand(&e)
    } else {
        crate::simplify::simplify(&e)
    }
}

/// `p_n` from `p_0`, `p_1` and the step `p_(k+1) = f(k, p_k, p_(k-1))`.
fn recurrence(
    n: i64,
    p0: Expr,
    p1: Expr,
    symbolic: bool,
    step: &dyn Fn(i64, &Expr, &Expr) -> Expr,
) -> Expr {
    if n == 0 {
        return p0;
    }
    let (mut prev, mut cur) = (p0, tidy(p1, symbolic));
    for k in 1..n {
        let next = tidy(step(k, &cur, &prev), symbolic);
        prev = cur;
        cur = next;
    }
    cur
}

fn int(k: i64) -> Expr {
    Expr::int(k)
}

fn legendre_p(n: i64, x: &Expr, symbolic: bool) -> Expr {
    recurrence(n, Expr::one(), x.clone(), symbolic, &|k, cur, prev| {
        // (k + 1) P_(k+1) = (2k + 1) x P_k - k P_(k-1)
        div(
            sub(
                mul(vec![int(2 * k + 1), x.clone(), cur.clone()]),
                mul(vec![int(k), prev.clone()]),
            ),
            int(k + 1),
        )
    })
}

/// The polynomial part of `Q_n(x) = P_n(x) atanh(x) - W(x)`.
fn legendre_w(n: i64, x: &Expr, symbolic: bool) -> Expr {
    recurrence(n, Expr::zero(), Expr::one(), symbolic, &|k, cur, prev| {
        div(
            sub(
                mul(vec![int(2 * k + 1), x.clone(), cur.clone()]),
                mul(vec![int(k), prev.clone()]),
            ),
            int(k + 1),
        )
    })
}

fn chebyshev(second: bool, n: i64, x: &Expr, symbolic: bool) -> Expr {
    let first = if second {
        mul(vec![int(2), x.clone()])
    } else {
        x.clone()
    };
    recurrence(n, Expr::one(), first, symbolic, &|_, cur, prev| {
        sub(mul(vec![int(2), x.clone(), cur.clone()]), prev.clone())
    })
}

fn hermite(n: i64, x: &Expr, symbolic: bool) -> Expr {
    recurrence(
        n,
        Expr::one(),
        mul(vec![int(2), x.clone()]),
        symbolic,
        &|k, cur, prev| {
            sub(
                mul(vec![int(2), x.clone(), cur.clone()]),
                mul(vec![int(2 * k), prev.clone()]),
            )
        },
    )
}

fn laguerre(n: i64, a: &Expr, x: &Expr, symbolic: bool) -> Expr {
    let first = add(vec![Expr::one(), a.clone(), neg(x.clone())]);
    recurrence(n, Expr::one(), first, symbolic, &|k, cur, prev| {
        // (k + 1) L_(k+1) = (2k + 1 + a - x) L_k - (k + a) L_(k-1)
        div(
            sub(
                mul(vec![
                    add(vec![int(2 * k + 1), a.clone(), neg(x.clone())]),
                    cur.clone(),
                ]),
                mul(vec![add(vec![int(k), a.clone()]), prev.clone()]),
            ),
            int(k + 1),
        )
    })
}

fn gegenbauer(n: i64, l: &Expr, x: &Expr, symbolic: bool) -> Expr {
    let first = mul(vec![int(2), l.clone(), x.clone()]);
    recurrence(n, Expr::one(), first, symbolic, &|k, cur, prev| {
        // (k + 1) C_(k+1) = 2 (k + l) x C_k - (k + 2l - 1) C_(k-1)
        div(
            sub(
                mul(vec![
                    int(2),
                    add(vec![int(k), l.clone()]),
                    x.clone(),
                    cur.clone(),
                ]),
                mul(vec![
                    add(vec![int(k - 1), mul(vec![int(2), l.clone()])]),
                    prev.clone(),
                ]),
            ),
            int(k + 1),
        )
    })
}

/// `binomial(top, k)` for a whole `k` as a product.
fn binomial(top: &Expr, k: i64) -> Expr {
    mul((1..=k)
        .map(|j| div(add(vec![top.clone(), int(j - k)]), int(j)))
        .collect())
}

fn jacobi(n: i64, a: &Expr, b: &Expr, x: &Expr, symbolic: bool) -> Expr {
    let lower = div(sub(x.clone(), Expr::one()), int(2));
    let upper = div(add(vec![x.clone(), Expr::one()]), int(2));
    let (na, nb) = (add(vec![int(n), a.clone()]), add(vec![int(n), b.clone()]));
    // a power 0 is 1, also of a base that is 0 at x = 1 or x = -1
    let power = |base: &Expr, k: i64| {
        if k == 0 {
            Expr::one()
        } else {
            powi(base.clone(), k)
        }
    };
    let terms = (0..=n)
        .map(|s| {
            mul(vec![
                binomial(&na, n - s),
                binomial(&nb, s),
                power(&lower, s),
                power(&upper, n - s),
            ])
        })
        .collect();
    tidy(add(terms), symbolic)
}

fn factorial(n: i64) -> Expr {
    Expr::integer(rug::Integer::from(rug::Integer::factorial(n as u32)))
}

/// `P_n^m(x) = (-1)^m (1 - x^2)^(m/2) d^m/dx^m P_n(x)`.
fn associated_legendre(n: i64, m: i64, x: &Expr) -> Expr {
    if m < 0 {
        // P_n^(-m) = (-1)^m (n - m)!/(n + m)! P_n^m
        if m < -n {
            return Expr::zero();
        }
        let k = -m;
        let sign = if k % 2 == 0 { 1 } else { -1 };
        return mul(vec![
            int(sign),
            div(factorial(n - k), factorial(n + k)),
            associated_legendre(n, k, x),
        ]);
    }
    if m > n {
        return Expr::zero();
    }
    let t = Expr::sym("#x");
    let p = legendre_p(n, &t, true);
    let d = crate::diff::diff_n(&p, "#x", m as u32);
    let sign = if m % 2 == 0 { 1 } else { -1 };
    let poly = crate::canon::subst(&d, &t, x);
    mul(vec![
        int(sign),
        pow(sub(Expr::one(), powi(x.clone(), 2)), Expr::rat(m, 2)),
        poly,
    ])
}

/// The explicit form of an orthogonal polynomial of whole degree.
pub fn symbolic(name: &str, a: &[Expr]) -> Option<Expr> {
    let degree = |e: &Expr| e.as_i64().filter(|n| (0..=MAX_DEGREE).contains(n));
    let x = a.last()?;
    let sym = !x.is_const();
    // the values at the ends that hold for every degree
    if a.len() == 2 && a[0].as_i64().is_none() {
        let n = &a[0];
        match name {
            "legendrep" | "chebyshevt" if x.is_one() => return Some(Expr::one()),
            "chebyshevu" if x.is_one() => return Some(add(vec![n.clone(), Expr::one()])),
            "laguerrel" if x.is_zero() => return Some(Expr::one()),
            "chebyshevt" if x.is_zero() => {
                return Some(func(
                    "cos",
                    vec![mul(vec![Expr::rat(1, 2), Expr::pi(), n.clone()])],
                ));
            }
            _ => {}
        }
    }
    Some(match (name, a.len()) {
        ("legendrep", 2) => {
            // P_(-n-1) = P_n
            let n = a[0].as_i64().map(|n| if n < 0 { -(n + 1) } else { n })?;
            legendre_p(degree(&int(n))?, x, sym)
        }
        ("legendrep", 3) => {
            let (n, m) = (degree(&a[0])?, a[1].as_i64()?);
            let value = associated_legendre(n, m, x);
            if sym {
                value
            } else {
                crate::simplify::simplify(&value)
            }
        }
        ("legendreq", 2) => {
            let n = degree(&a[0])?;
            if !sym && (x.is_one() || x.is_minus_one()) {
                return Some(if x.is_one() || n % 2 == 1 {
                    Expr::inf()
                } else {
                    Expr::neg_inf()
                });
            }
            let p = legendre_p(n, x, sym);
            let w = legendre_w(n, x, sym);
            let out = sub(mul(vec![p, func("atanh", vec![x.clone()])]), w);
            if sym {
                out
            } else {
                crate::simplify::simplify(&out)
            }
        }
        ("chebyshevt", 2) => chebyshev(false, degree(&int(a[0].as_i64()?.checked_abs()?))?, x, sym),
        ("chebyshevu", 2) => {
            let n = a[0].as_i64()?;
            match n {
                -1 => Expr::zero(),
                n if n < -1 => neg(chebyshev(true, degree(&int(-(n + 2)))?, x, sym)),
                n => chebyshev(true, degree(&int(n))?, x, sym),
            }
        }
        ("hermiteh", 2) => hermite(degree(&a[0])?, x, sym),
        ("laguerrel", 2) => laguerre(degree(&a[0])?, &Expr::zero(), x, sym),
        ("laguerrel", 3) => {
            let n = degree(&a[0])?;
            laguerre(n, &a[1], x, sym || !a[1].is_const())
        }
        ("gegenbauerc", 3) => {
            let n = degree(&a[0])?;
            gegenbauer(n, &a[1], x, sym || !a[1].is_const())
        }
        ("jacobip", 4) => {
            let n = degree(&a[0])?.min(60);
            if a[0].as_i64()? > 60 {
                return None;
            }
            jacobi(
                n,
                &a[1],
                &a[2],
                x,
                sym || !a[1].is_const() || !a[2].is_const(),
            )
        }
        ("sphericalharmonicy", 4) => {
            let (l, m) = (degree(&a[0])?, a[1].as_i64()?);
            if m.unsigned_abs() > l as u64 {
                return Some(Expr::zero());
            }
            if l > 60 {
                return None;
            }
            // sqrt((2l + 1)/(4 pi) (l - m)!/(l + m)!) P_l^m(cos theta) e^(i m phi)
            let norm = crate::canon::sqrt(div(
                mul(vec![int(2 * l + 1), factorial(l - m)]),
                mul(vec![int(4), Expr::pi(), factorial(l + m)]),
            ));
            let c = func("cos", vec![a[2].clone()]);
            // (1 - cos^2)^(m/2) is sin^m for theta between 0 and pi
            let t = Expr::sym("#x");
            let p = legendre_p(l, &t, true);
            let k = m.abs();
            let d = crate::diff::diff_n(&p, "#x", k as u32);
            let mut legendre = mul(vec![
                int(if k % 2 == 0 { 1 } else { -1 }),
                powi(func("sin", vec![a[2].clone()]), k),
                crate::canon::subst(&d, &t, &c),
            ]);
            if m < 0 {
                legendre = mul(vec![
                    int(if k % 2 == 0 { 1 } else { -1 }),
                    div(factorial(l - k), factorial(l + k)),
                    legendre,
                ]);
            }
            let phase = pow(Expr::e(), mul(vec![Expr::i(), int(m), a[3].clone()]));
            crate::simplify::simplify(&mul(vec![norm, legendre, phase]))
        }
        _ => return None,
    })
}

/// Values for degrees that are not whole numbers, by hypergeometric functions.
pub fn numeric(name: &str, a: &[Complex], p: u32) -> Out {
    let wp = p + 32;
    let c = |v: &Complex| Complex::with_val(wp, v);
    let half_one_minus = |z: &Complex| Complex::with_val(wp, 1u32 - c(z)) / 2u32;
    let cut = |v: Complex| Some(Some(Complex::with_val(p, v)));
    match (name, a.len()) {
        ("legendrep", 2) => {
            let (nu, z) = (&a[0], &a[1]);
            hyp2f1(
                &Complex::with_val(wp, -nu),
                &Complex::with_val(wp, nu + 1u32),
                &Complex::with_val(wp, 1),
                &half_one_minus(z),
                p,
            )
        }
        ("chebyshevt", 2) => {
            let (nu, z) = (&a[0], &a[1]);
            hyp2f1(
                &Complex::with_val(wp, -nu),
                &c(nu),
                &Complex::with_val(wp, 0.5),
                &half_one_minus(z),
                p,
            )
        }
        ("chebyshevu", 2) => {
            let (nu, z) = (&a[0], &a[1]);
            let Some(f) = hyp2f1(
                &Complex::with_val(wp, -nu),
                &Complex::with_val(wp, nu + 2u32),
                &Complex::with_val(wp, 1.5),
                &half_one_minus(z),
                wp,
            )?
            else {
                return Some(None);
            };
            cut(f * Complex::with_val(wp, nu + 1u32))
        }
        ("hermiteh", 2) => {
            // 2^nu sqrt(pi) (M(-nu/2, 1/2, z^2)/gamma((1 - nu)/2) - 2z M((1 - nu)/2, 3/2, z^2)/gamma(-nu/2))
            let (nu, z) = (&a[0], &a[1]);
            crate::hyper::adaptive(p, &mut |wp| {
                let w = wp + 16;
                let z2 = Complex::with_val(w, z.square_ref());
                let minus_half = Complex::with_val(w, -nu) / 2u32;
                let other = Complex::with_val(w, 1u32 - Complex::with_val(w, nu)) / 2u32;
                let m1 = hyp1f1(&minus_half, &Complex::with_val(w, 0.5), &z2, w)??;
                let m2 = hyp1f1(&other, &Complex::with_val(w, 1.5), &z2, w)??;
                let t1 = m1 * rgamma(&other, w)?;
                let t2 = m2 * rgamma(&minus_half, w)? * z * 2u32;
                let diff = Complex::with_val(w, &t1 - &t2);
                let lost = crate::special::expo(&t1).max(crate::special::expo(&t2))
                    - crate::special::expo(&diff);
                let scale =
                    (Complex::with_val(w, nu) * Float::with_val(w, 2).ln()).cexp() * pi(w).sqrt();
                Some(Some((diff * scale, lost.max(0))))
            })
        }
        ("laguerrel", 2 | 3) => {
            let (nu, z) = (&a[0], &a[a.len() - 1]);
            let alpha = if a.len() == 3 {
                c(&a[1])
            } else {
                Complex::new(wp)
            };
            let top = Complex::with_val(wp, nu + &alpha) + 1u32;
            let Some(g) = gamma_any(&top, wp)? else {
                return Some(None);
            };
            let a1 = Complex::with_val(wp, &alpha + 1u32);
            let Some(m) = hyp1f1(&Complex::with_val(wp, -nu), &a1, z, wp)? else {
                return Some(None);
            };
            cut(m * g * rgamma(&a1, wp)? * rgamma(&Complex::with_val(wp, nu + 1u32), wp)?)
        }
        ("gegenbauerc", 3) => {
            let (nu, l, z) = (&a[0], &a[1], &a[2]);
            let l2 = Complex::with_val(wp, l * 2u32);
            let top = Complex::with_val(wp, nu + &l2);
            let Some(g) = gamma_any(&top, wp)? else {
                return Some(None);
            };
            let Some(f) = hyp2f1(
                &Complex::with_val(wp, -nu),
                &top,
                &(c(l) + Float::with_val(wp, 0.5)),
                &half_one_minus(z),
                wp,
            )?
            else {
                return Some(None);
            };
            cut(f * g * rgamma(&l2, wp)? * rgamma(&Complex::with_val(wp, nu + 1u32), wp)?)
        }
        ("jacobip", 4) => {
            let (nu, al, be, z) = (&a[0], &a[1], &a[2], &a[3]);
            let top = Complex::with_val(wp, nu + al) + 1u32;
            let Some(g) = gamma_any(&top, wp)? else {
                return Some(None);
            };
            let a1 = Complex::with_val(wp, al + 1u32);
            let second = Complex::with_val(wp, nu + al) + be + 1u32;
            let Some(f) = hyp2f1(
                &Complex::with_val(wp, -nu),
                &second,
                &a1,
                &half_one_minus(z),
                wp,
            )?
            else {
                return Some(None);
            };
            cut(f * g * rgamma(&a1, wp)? * rgamma(&Complex::with_val(wp, nu + 1u32), wp)?)
        }
        _ => None,
    }
}
