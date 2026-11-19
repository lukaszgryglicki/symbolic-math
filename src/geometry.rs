//! Geometry of points, vectors, lines and planes in any number of dimensions, angles and solid
//! angles, rotations, and the regular polytopes.

use crate::canon::{add, div, mul, neg, pow, powi, sqrt, sub};
use crate::expr::Expr;
use crate::funcs::func;
use crate::matrix::{self, Rows};
use crate::simplify::simplify;
use crate::{complex, system};

type Vector = Vec<Expr>;
type Res = Result<Expr, String>;

/// A point: a list of coordinates or a complex number.
pub fn point(e: &Expr) -> Result<Vector, String> {
    match e {
        Expr::List(items)
            if !items.is_empty() && items.iter().all(|i| !matches!(i, Expr::List(_))) =>
        {
            Ok(items.to_vec())
        }
        Expr::List(_) | Expr::Fn(..) if !e.is_const() || matches!(e, Expr::List(_)) => {
            Err(format!("{e} is not a point"))
        }
        _ => complex::rect(e)
            .map(|(re, im)| vec![re, im])
            .ok_or_else(|| format!("{e} is not a point")),
    }
}

fn same_size(a: &[Expr], b: &[Expr]) -> Result<(), String> {
    if a.len() == b.len() {
        Ok(())
    } else {
        Err(format!(
            "the points have {} and {} coordinates",
            a.len(),
            b.len()
        ))
    }
}

fn minus(a: &[Expr], b: &[Expr]) -> Vector {
    a.iter()
        .zip(b)
        .map(|(x, y)| sub(x.clone(), y.clone()))
        .collect()
}

fn plus(a: &[Expr], b: &[Expr]) -> Vector {
    a.iter()
        .zip(b)
        .map(|(x, y)| add(vec![x.clone(), y.clone()]))
        .collect()
}

fn scaled(a: &[Expr], s: &Expr) -> Vector {
    a.iter().map(|x| mul(vec![s.clone(), x.clone()])).collect()
}

fn dot(a: &[Expr], b: &[Expr]) -> Expr {
    add(a
        .iter()
        .zip(b)
        .map(|(x, y)| mul(vec![x.clone(), y.clone()]))
        .collect())
}

fn length(a: &[Expr]) -> Expr {
    simplify(&sqrt(dot(a, a)))
}

fn list(v: Vector) -> Expr {
    Expr::list(v.iter().map(simplify).collect())
}

/// The vector orthogonal to `n - 1` vectors of `n` coordinates.
pub fn cross(vectors: &[Vector]) -> Result<Vector, String> {
    let n = vectors.len() + 1;
    if vectors.iter().any(|v| v.len() != n) {
        return Err(format!(
            "the cross product of {} vector(s) needs vectors with {n} coordinates",
            n - 1
        ));
    }
    (0..n)
        .map(|skip| {
            let minor: Rows = vectors
                .iter()
                .map(|v| {
                    v.iter()
                        .enumerate()
                        .filter(|(k, _)| *k != skip)
                        .map(|(_, x)| x.clone())
                        .collect()
                })
                .collect();
            let d = if minor.is_empty() {
                Expr::one()
            } else {
                matrix::det(&minor)?
            };
            Ok(if (skip + n - 1).is_multiple_of(2) {
                d
            } else {
                neg(d)
            })
        })
        .collect()
}

/// A line or a plane: a point on it and its direction (line) or normal (plane).
enum Figure {
    Point(Vector),
    Line(Vector, Vector),
    Plane(Vector, Vector),
}

fn figure(e: &Expr) -> Result<Figure, String> {
    match e {
        Expr::Fn(n, a) if &**n == "line" && a.len() == 2 => {
            let (p, q) = (point(&a[0])?, point(&a[1])?);
            same_size(&p, &q)?;
            let d = minus(&q, &p);
            Ok(Figure::Line(p, d))
        }
        Expr::Fn(n, a) if &**n == "plane" && a.len() == 2 => {
            let (p, normal) = (point(&a[0])?, point(&a[1])?);
            same_size(&p, &normal)?;
            Ok(Figure::Plane(p, normal))
        }
        Expr::Fn(n, a) if &**n == "plane" && a.len() >= 3 => {
            let points: Vec<Vector> = a.iter().map(point).collect::<Result<_, _>>()?;
            let edges: Vec<Vector> = points[1..].iter().map(|q| minus(q, &points[0])).collect();
            let normal = cross(&edges)?;
            Ok(Figure::Plane(points[0].clone(), normal))
        }
        _ => Ok(Figure::Point(point(e)?)),
    }
}

/// The distance between two points, or of a point from a line or a plane, or between two lines.
pub fn distance(a: &Expr, b: &Expr) -> Res {
    let from_line = |p: &[Expr], at: &[Expr], d: &[Expr]| {
        let w = minus(p, at);
        let along = div(dot(&w, d), dot(d, d));
        length(&minus(&w, &scaled(d, &along)))
    };
    Ok(match (figure(a)?, figure(b)?) {
        (Figure::Point(p), Figure::Point(q)) => {
            same_size(&p, &q)?;
            length(&minus(&p, &q))
        }
        (Figure::Point(p), Figure::Line(at, d)) | (Figure::Line(at, d), Figure::Point(p)) => {
            same_size(&p, &at)?;
            from_line(&p, &at, &d)
        }
        (Figure::Point(p), Figure::Plane(at, n)) | (Figure::Plane(at, n), Figure::Point(p)) => {
            same_size(&p, &at)?;
            simplify(&div(func("abs", vec![dot(&minus(&p, &at), &n)]), length(&n)))
        }
        (Figure::Line(p, d), Figure::Line(q, e)) => {
            same_size(&p, &q)?;
            if p.len() != 3 {
                return Err("the distance of two lines is for lines of space".into());
            }
            let n = cross(&[d.clone(), e])?;
            if n.iter().all(|c| simplify(c).is_zero()) {
                from_line(&q, &p, &d)
            } else {
                simplify(&div(func("abs", vec![dot(&minus(&q, &p), &n)]), length(&n)))
            }
        }
        _ => return Err("distance(point, point), distance(point, line), distance(point, plane) or distance(line, line)".into()),
    })
}

fn angle_between(u: &[Expr], v: &[Expr]) -> Expr {
    simplify(&func(
        "acos",
        vec![simplify(&div(dot(u, v), mul(vec![length(u), length(v)])))],
    ))
}

/// The angle between two vectors (or lines, or planes, or a line and a plane), or at the middle
/// one of three points.
pub fn angle(args: &[Expr]) -> Res {
    match args {
        [a, b, c] => {
            let (a, b, c) = (point(a)?, point(b)?, point(c)?);
            same_size(&a, &b)?;
            same_size(&c, &b)?;
            Ok(angle_between(&minus(&a, &b), &minus(&c, &b)))
        }
        [a, b] => {
            Ok(match (figure(a)?, figure(b)?) {
                (Figure::Point(u), Figure::Point(v)) => {
                    same_size(&u, &v)?;
                    angle_between(&u, &v)
                }
                (Figure::Line(_, d), Figure::Line(_, e))
                | (Figure::Plane(_, d), Figure::Plane(_, e)) => {
                    same_size(&d, &e)?;
                    angle_between(&d, &e)
                }
                (Figure::Line(_, d), Figure::Plane(_, n))
                | (Figure::Plane(_, n), Figure::Line(_, d)) => {
                    same_size(&d, &n)?;
                    simplify(&sub(div(Expr::pi(), Expr::int(2)), angle_between(&d, &n)))
                }
                _ => return Err(
                    "angle(u, v) needs two vectors, two lines, two planes or a line and a plane"
                        .into(),
                ),
            })
        }
        _ => Err("angle(u, v) or angle(a, b, c) for the angle at b".into()),
    }
}

/// The orthogonal projection of a vector on a vector, or of a point on a line or a plane.
pub fn projection(a: &Expr, onto: &Expr) -> Res {
    let p = point(a)?;
    Ok(match figure(onto)? {
        Figure::Point(v) => {
            same_size(&p, &v)?;
            list(scaled(&v, &div(dot(&p, &v), dot(&v, &v))))
        }
        Figure::Line(at, d) => {
            same_size(&p, &at)?;
            let along = div(dot(&minus(&p, &at), &d), dot(&d, &d));
            list(plus(&at, &scaled(&d, &along)))
        }
        Figure::Plane(at, n) => {
            same_size(&p, &at)?;
            let off = div(dot(&minus(&p, &at), &n), dot(&n, &n));
            list(minus(&p, &scaled(&n, &off)))
        }
    })
}

/// Where two figures meet: a point, a line, or `undefined` when they do not meet in one.
pub fn intersection(a: &Expr, b: &Expr) -> Res {
    match (figure(a)?, figure(b)?) {
        (Figure::Line(p, d), Figure::Line(q, e)) => {
            same_size(&p, &q)?;
            // p + s d = q + t e
            let (s, t) = (Expr::sym("#s"), Expr::sym("#t"));
            let eqs: Vec<Expr> = (0..p.len())
                .map(|k| {
                    sub(
                        add(vec![p[k].clone(), mul(vec![s.clone(), d[k].clone()])]),
                        add(vec![q[k].clone(), mul(vec![t.clone(), e[k].clone()])]),
                    )
                })
                .collect();
            let solved = system::solve_system(&eqs, &["#s".to_string(), "#t".to_string()])?;
            let value = solved.solutions.first().and_then(|sol| {
                (sol.len() == 2
                    && sol
                        .iter()
                        .all(|(_, v)| !v.has_sym("#s") && !v.has_sym("#t")))
                .then(|| sol.iter().find(|(n, _)| n == "#s").map(|(_, v)| v.clone()))
                .flatten()
            });
            Ok(match value {
                Some(s) => list(plus(&p, &scaled(&d, &s))),
                None => Expr::undefined(),
            })
        }
        (Figure::Line(p, d), Figure::Plane(at, n)) | (Figure::Plane(at, n), Figure::Line(p, d)) => {
            same_size(&p, &at)?;
            let slope = simplify(&dot(&d, &n));
            if slope.is_zero() {
                return Ok(Expr::undefined());
            }
            let s = div(dot(&minus(&at, &p), &n), slope);
            Ok(list(plus(&p, &scaled(&d, &s))))
        }
        (Figure::Plane(p, m), Figure::Plane(q, n)) => {
            same_size(&p, &q)?;
            if p.len() != 3 {
                return Err("the line where two planes meet is for planes of space".into());
            }
            let d = cross(&[m.clone(), n.clone()])?;
            if d.iter().all(|c| simplify(c).is_zero()) {
                return Ok(Expr::undefined());
            }
            // the point of both planes nearest to the origin
            let rows: Rows = vec![m.clone(), n.clone(), d.clone()];
            let right = vec![dot(&m, &p), dot(&n, &q), Expr::zero()];
            let at = matrix::linsolve(&rows, &right)?;
            let at = at
                .as_list()
                .ok_or("the planes do not meet in a line")?
                .to_vec();
            Ok(Expr::call(
                "line",
                vec![list(at.clone()), list(plus(&at, &d))],
            ))
        }
        _ => Err("intersection of two lines, of a line and a plane, or of two planes".into()),
    }
}

/// An equation to be printed as it is, not solved.
fn shown(equation: Expr) -> Expr {
    Expr::call("hold", vec![equation])
}

fn coordinate_names(n: usize) -> Vec<Expr> {
    if n <= 3 {
        ["x", "y", "z"][..n].iter().map(|s| Expr::sym(s)).collect()
    } else {
        (1..=n).map(|k| Expr::sym(&format!("x{k}"))).collect()
    }
}

/// The equation of a line of the plane, of a plane, of a circle or of a sphere; the points of a
/// line in more dimensions with the parameter `t`.
pub fn equation(e: &Expr) -> Res {
    if let Expr::Fn(n, a) = e
        && matches!(&**n, "circle" | "sphere")
        && a.len() == 2
    {
        let c = point(&a[0])?;
        let x = coordinate_names(c.len());
        return Ok(shown(Expr::eq(
            add(minus(&x, &c).into_iter().map(|d| powi(d, 2)).collect()),
            simplify(&powi(a[1].clone(), 2)),
        )));
    }
    match figure(e)? {
        Figure::Line(p, d) if p.len() == 2 => {
            let x = coordinate_names(2);
            let normal = vec![d[1].clone(), neg(d[0].clone())];
            Ok(shown(Expr::eq(simplify(&dot(&normal, &x)), simplify(&dot(&normal, &p)))))
        }
        Figure::Line(p, d) => {
            let x = coordinate_names(p.len());
            let t = Expr::sym("t");
            Ok(Expr::list(
                (0..p.len())
                    .map(|k| {
                        Expr::call(
                            "rule",
                            vec![
                                x[k].clone(),
                                simplify(&add(vec![p[k].clone(), mul(vec![t.clone(), d[k].clone()])])),
                            ],
                        )
                    })
                    .collect(),
            ))
        }
        Figure::Plane(p, n) => {
            let x = coordinate_names(p.len());
            Ok(shown(Expr::eq(simplify(&dot(&n, &x)), simplify(&dot(&n, &p)))))
        }
        Figure::Point(_) => Err("equation(line(p, q)), equation(plane(p, q, r)), equation(circle(c, r)) or equation(sphere(c, r))".into()),
    }
}

/// The center and the radius of the sphere through `n + 1` points of `n` coordinates.
pub fn circumsphere(points: &[Vector]) -> Res {
    let n = points.len() - 1;
    if n == 0 || points.iter().any(|p| p.len() != n) {
        return Err(format!(
            "the sphere through {} points needs points with {n} coordinates",
            n + 1
        ));
    }
    let rows: Rows = points[1..]
        .iter()
        .map(|p| scaled(&minus(p, &points[0]), &Expr::int(2)))
        .collect();
    let right: Vec<Expr> = points[1..]
        .iter()
        .map(|p| sub(dot(p, p), dot(&points[0], &points[0])))
        .collect();
    let center = matrix::linsolve(&rows, &right)?;
    let c = center
        .as_list()
        .ok_or("the points lie in one plane: no sphere goes through them")?
        .to_vec();
    Ok(Expr::list(vec![
        list(c.clone()),
        length(&minus(&c, &points[0])),
    ]))
}

/// The center and the radius of the circle inside a triangle.
pub fn incircle(a: &[Expr], b: &[Expr], c: &[Expr]) -> Res {
    same_size(a, b)?;
    same_size(a, c)?;
    let (la, lb, lc) = (
        length(&minus(b, c)),
        length(&minus(a, c)),
        length(&minus(a, b)),
    );
    let around = add(vec![la.clone(), lb.clone(), lc.clone()]);
    let center: Vector = (0..a.len())
        .map(|k| {
            div(
                add(vec![
                    mul(vec![la.clone(), a[k].clone()]),
                    mul(vec![lb.clone(), b[k].clone()]),
                    mul(vec![lc.clone(), c[k].clone()]),
                ]),
                around.clone(),
            )
        })
        .collect();
    // Heron: area = sqrt(s (s - a)(s - b)(s - c)), radius = area / s
    let s = div(around, Expr::int(2));
    let area = sqrt(mul(vec![
        s.clone(),
        sub(s.clone(), la),
        sub(s.clone(), lb),
        sub(s.clone(), lc),
    ]));
    Ok(Expr::list(vec![list(center), simplify(&div(area, s))]))
}

/// The solid angle of the triangle `a b c` seen from the origin (Van Oosterom and Strackee).
pub fn solid_angle(a: &[Expr], b: &[Expr], c: &[Expr]) -> Res {
    if a.len() != 3 || b.len() != 3 || c.len() != 3 {
        return Err("the solid angle of a triangle needs points of space".into());
    }
    let triple = matrix::det(&vec![a.to_vec(), b.to_vec(), c.to_vec()])?;
    let (la, lb, lc) = (length(a), length(b), length(c));
    let low = add(vec![
        mul(vec![la.clone(), lb.clone(), lc.clone()]),
        mul(vec![dot(a, b), lc]),
        mul(vec![dot(a, c), lb]),
        mul(vec![dot(b, c), la]),
    ]);
    Ok(simplify(&mul(vec![
        Expr::int(2),
        func(
            "atan2",
            vec![simplify(&func("abs", vec![triple])), simplify(&low)],
        ),
    ])))
}

/// Volume of the ball of `n` dimensions and radius `r`, or the area of its surface.
pub fn ball(n: &Expr, r: &Expr, surface: bool) -> Expr {
    let half = div(n.clone(), Expr::int(2));
    let volume = div(
        mul(vec![
            pow(Expr::pi(), half.clone()),
            pow(r.clone(), n.clone()),
        ]),
        func("gamma", vec![add(vec![half, Expr::one()])]),
    );
    simplify(&if surface {
        div(mul(vec![n.clone(), volume]), r.clone())
    } else {
        volume
    })
}

/// The matrix of the rotation by `angle`: in the plane, about an axis of space, or in the plane
/// of two vectors of any dimension.
pub fn rotation_matrix(angle: &Expr, frame: &[Vector]) -> Result<Rows, String> {
    let (c, s) = (
        func("cos", vec![angle.clone()]),
        func("sin", vec![angle.clone()]),
    );
    let (u, v): (Vector, Vector) = match frame {
        [] => (vec![Expr::one(), Expr::zero()], vec![Expr::zero(), Expr::one()]),
        [axis] if axis.len() == 3 => {
            // Rodrigues: R = c I + s K + (1 - c) k k^T
            let size = length(axis);
            let k: Vector = axis.iter().map(|x| div(x.clone(), size.clone())).collect();
            let skew = [
                [Expr::zero(), neg(k[2].clone()), k[1].clone()],
                [k[2].clone(), Expr::zero(), neg(k[0].clone())],
                [neg(k[1].clone()), k[0].clone(), Expr::zero()],
            ];
            return Ok((0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            let unit = if i == j { c.clone() } else { Expr::zero() };
                            simplify(&add(vec![
                                unit,
                                mul(vec![s.clone(), skew[i][j].clone()]),
                                mul(vec![
                                    sub(Expr::one(), c.clone()),
                                    k[i].clone(),
                                    k[j].clone(),
                                ]),
                            ]))
                        })
                        .collect()
                })
                .collect());
        }
        [u, v] if u.len() == v.len() => {
            // an orthonormal pair in the plane of u and v
            let lu = length(u);
            let u: Vector = u.iter().map(|x| div(x.clone(), lu.clone())).collect();
            let w = minus(v, &scaled(&u, &dot(v, &u)));
            let lw = length(&w);
            if lw.is_zero() {
                return Err("the two vectors span no plane".into());
            }
            (u, w.iter().map(|x| div(x.clone(), lw.clone())).collect())
        }
        _ => return Err("rotationmatrix(angle), rotationmatrix(angle, [x, y, z]) about an axis, or rotationmatrix(angle, u, v) in the plane of u and v".into()),
    };
    let n = u.len();
    // R = I + s (v u^T - u v^T) + (c - 1)(u u^T + v v^T)
    Ok((0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    let unit = if i == j { Expr::one() } else { Expr::zero() };
                    simplify(&add(vec![
                        unit,
                        mul(vec![
                            s.clone(),
                            sub(
                                mul(vec![v[i].clone(), u[j].clone()]),
                                mul(vec![u[i].clone(), v[j].clone()]),
                            ),
                        ]),
                        mul(vec![
                            sub(c.clone(), Expr::one()),
                            add(vec![
                                mul(vec![u[i].clone(), u[j].clone()]),
                                mul(vec![v[i].clone(), v[j].clone()]),
                            ]),
                        ]),
                    ]))
                })
                .collect()
        })
        .collect())
}

/// The mirror image of a vector in the hyperplane through the origin with the given normal.
pub fn reflect(v: &[Expr], normal: &[Expr]) -> Res {
    same_size(v, normal)?;
    let twice = div(mul(vec![Expr::int(2), dot(v, normal)]), dot(normal, normal));
    Ok(list(minus(v, &scaled(normal, &twice))))
}

/// Polar, spherical and hyperspherical coordinates `[r, angles...]` of a point, or back.
pub fn spherical(p: &[Expr], to: bool) -> Res {
    let n = p.len();
    if n < 2 {
        return Err("two or more coordinates are needed".into());
    }
    if to {
        let mut out = vec![length(p)];
        for k in 0..n - 2 {
            // the angle from the axis k: cos = x_k / |(x_k, ..., x_n)|; 0 when nothing is left
            let rest = length(&p[k..]);
            out.push(if rest.is_zero() {
                Expr::zero()
            } else {
                simplify(&func("acos", vec![div(p[k].clone(), rest)]))
            });
        }
        let last = simplify(&func("atan2", vec![p[n - 1].clone(), p[n - 2].clone()]));
        out.push(if last.is_undefined() {
            Expr::zero()
        } else {
            last
        });
        return Ok(Expr::list(out));
    }
    let mut out = Vec::new();
    let mut prefix = p[0].clone();
    for a in &p[1..] {
        out.push(simplify(&mul(vec![
            prefix.clone(),
            func("cos", vec![a.clone()]),
        ])));
        prefix = mul(vec![prefix, func("sin", vec![a.clone()])]);
    }
    out.push(simplify(&prefix));
    Ok(Expr::list(out))
}

fn rules(items: Vec<(&str, Expr)>) -> Expr {
    Expr::list(
        items
            .into_iter()
            .map(|(name, value)| Expr::call("rule", vec![Expr::sym(name), simplify(&value)]))
            .collect(),
    )
}

fn root(n: i64) -> Expr {
    sqrt(Expr::int(n))
}

/// The regular polygon of `n` sides of length `a`.
pub fn regular_polygon(n: &Expr, a: &Expr) -> Expr {
    let turn = div(Expr::pi(), n.clone());
    let tan = func("tan", vec![turn.clone()]);
    rules(vec![
        ("sides", n.clone()),
        (
            "diagonals",
            div(
                mul(vec![n.clone(), sub(n.clone(), Expr::int(3))]),
                Expr::int(2),
            ),
        ),
        (
            "area",
            div(
                mul(vec![n.clone(), powi(a.clone(), 2)]),
                mul(vec![Expr::int(4), tan.clone()]),
            ),
        ),
        ("perimeter", mul(vec![n.clone(), a.clone()])),
        (
            "circumradius",
            div(a.clone(), mul(vec![Expr::int(2), func("sin", vec![turn])])),
        ),
        ("inradius", div(a.clone(), mul(vec![Expr::int(2), tan]))),
        (
            "angle",
            div(
                mul(vec![sub(n.clone(), Expr::int(2)), Expr::pi()]),
                n.clone(),
            ),
        ),
    ])
}

/// A regular polytope with edge `a`: the five solids, the six regular polytopes of four
/// dimensions, and the simplex, the cube and the orthoplex of `n` dimensions.
pub fn polytope(name: &str, n: Option<i64>, a: &Expr) -> Res {
    let int = Expr::int;
    let frac = |p: i64, q: i64| Expr::rat(p, q);
    let a_pow = |k: i64| powi(a.clone(), k);
    let times = |parts: Vec<Expr>| mul(parts);
    let s5 = root(5);
    let acos = |x: Expr| func("acos", vec![x]);
    let solid = |v: i64,
                 e: i64,
                 f: i64,
                 volume: Expr,
                 surface: Expr,
                 big: Expr,
                 small: Expr,
                 dihedral: Expr| {
        rules(vec![
            ("vertices", int(v)),
            ("edges", int(e)),
            ("faces", int(f)),
            ("volume", times(vec![volume, a_pow(3)])),
            ("surface", times(vec![surface, a_pow(2)])),
            ("circumradius", times(vec![big, a.clone()])),
            ("inradius", times(vec![small, a.clone()])),
            ("dihedral", dihedral),
        ])
    };
    let four =
        |v: i64, e: i64, f: i64, c: i64, volume: Expr, surface: Expr, big: Expr, dihedral: Expr| {
            rules(vec![
                ("vertices", int(v)),
                ("edges", int(e)),
                ("faces", int(f)),
                ("cells", int(c)),
                ("volume", times(vec![volume.clone(), a_pow(4)])),
                ("surface", times(vec![surface.clone(), a_pow(3)])),
                ("circumradius", times(vec![big, a.clone()])),
                (
                    "inradius",
                    times(vec![int(4), volume, pow(surface, int(-1)), a.clone()]),
                ),
                ("dihedral", dihedral),
            ])
        };
    let half_pi = div(Expr::pi(), int(2));
    let general = |n: i64| -> Result<i64, String> {
        if (1..=1000).contains(&n) {
            Ok(n)
        } else {
            Err("the dimension must be a whole number from 1 to 1000".to_string())
        }
    };
    let fact = |k: i64| Expr::integer(crate::ntheory::factorial(k as u32));
    let simplex_volume = |k: i64| {
        // edge 1: sqrt(k + 1) / (k! 2^(k/2))
        div(
            sqrt(int(k + 1)),
            times(vec![fact(k), pow(int(2), frac(k, 2))]),
        )
    };
    Ok(match (name, n) {
        ("tetrahedron", None) => solid(
            4,
            6,
            4,
            div(root(2), int(12)),
            root(3),
            div(root(6), int(4)),
            div(root(6), int(12)),
            acos(frac(1, 3)),
        ),
        ("cube" | "hexahedron", None) => solid(
            8,
            12,
            6,
            int(1),
            int(6),
            div(root(3), int(2)),
            frac(1, 2),
            half_pi,
        ),
        ("octahedron", None) => solid(
            6,
            12,
            8,
            div(root(2), int(3)),
            times(vec![int(2), root(3)]),
            div(root(2), int(2)),
            div(root(6), int(6)),
            acos(frac(-1, 3)),
        ),
        ("dodecahedron", None) => solid(
            20,
            30,
            12,
            div(add(vec![int(15), times(vec![int(7), s5.clone()])]), int(4)),
            times(vec![
                int(3),
                sqrt(add(vec![int(25), times(vec![int(10), s5.clone()])])),
            ]),
            div(times(vec![root(3), add(vec![int(1), s5.clone()])]), int(4)),
            div(
                sqrt(add(vec![int(250), times(vec![int(110), s5.clone()])])),
                int(20),
            ),
            acos(neg(div(s5.clone(), int(5)))),
        ),
        ("icosahedron", None) => solid(
            12,
            30,
            20,
            div(times(vec![int(5), add(vec![int(3), s5.clone()])]), int(12)),
            times(vec![int(5), root(3)]),
            div(
                sqrt(add(vec![int(10), times(vec![int(2), s5.clone()])])),
                int(4),
            ),
            div(times(vec![root(3), add(vec![int(3), s5.clone()])]), int(12)),
            acos(neg(div(s5.clone(), int(3)))),
        ),
        ("cell5" | "pentachoron", None) => four(
            5,
            10,
            10,
            5,
            div(s5.clone(), int(96)),
            div(times(vec![int(5), root(2)]), int(12)),
            div(root(10), int(5)),
            acos(frac(1, 4)),
        ),
        ("cell8" | "tesseract", None) => four(16, 32, 24, 8, int(1), int(8), int(1), half_pi),
        ("cell16" | "hexadecachoron", None) => four(
            8,
            24,
            32,
            16,
            frac(1, 6),
            div(times(vec![int(4), root(2)]), int(3)),
            div(root(2), int(2)),
            div(times(vec![int(2), Expr::pi()]), int(3)),
        ),
        ("cell24" | "icositetrachoron", None) => four(
            24,
            96,
            96,
            24,
            int(2),
            times(vec![int(8), root(2)]),
            int(1),
            div(times(vec![int(2), Expr::pi()]), int(3)),
        ),
        ("cell120" | "hecatonicosachoron", None) => four(
            600,
            1200,
            720,
            120,
            div(
                times(vec![
                    int(15),
                    add(vec![int(105), times(vec![int(47), s5.clone()])]),
                ]),
                int(4),
            ),
            times(vec![
                int(30),
                add(vec![int(15), times(vec![int(7), s5.clone()])]),
            ]),
            div(times(vec![root(2), add(vec![int(3), s5.clone()])]), int(2)),
            div(times(vec![int(4), Expr::pi()]), int(5)),
        ),
        ("cell600" | "hexacosichoron", None) => four(
            120,
            720,
            1200,
            600,
            div(times(vec![int(25), add(vec![int(2), s5.clone()])]), int(4)),
            times(vec![int(50), root(2)]),
            div(add(vec![int(1), s5.clone()]), int(2)),
            acos(neg(div(add(vec![int(1), times(vec![int(3), s5])]), int(8)))),
        ),
        ("simplex", Some(n)) => {
            let n = general(n)?;
            rules(vec![
                ("vertices", int(n + 1)),
                ("edges", int(n * (n + 1) / 2)),
                ("facets", int(n + 1)),
                ("volume", times(vec![simplex_volume(n), a_pow(n)])),
                (
                    "surface",
                    times(vec![int(n + 1), simplex_volume(n - 1), a_pow(n - 1)]),
                ),
                (
                    "circumradius",
                    times(vec![sqrt(frac(n, 2 * (n + 1))), a.clone()]),
                ),
                (
                    "inradius",
                    times(vec![pow(sqrt(int(2 * n * (n + 1))), int(-1)), a.clone()]),
                ),
                ("dihedral", acos(frac(1, n))),
            ])
        }
        ("cube" | "hypercube", Some(n)) => {
            let n = general(n)?;
            rules(vec![
                ("vertices", pow(int(2), int(n))),
                ("edges", times(vec![int(n), pow(int(2), int(n - 1))])),
                ("facets", int(2 * n)),
                ("volume", a_pow(n)),
                ("surface", times(vec![int(2 * n), a_pow(n - 1)])),
                (
                    "circumradius",
                    times(vec![div(sqrt(int(n)), int(2)), a.clone()]),
                ),
                ("inradius", div(a.clone(), int(2))),
                ("dihedral", half_pi),
            ])
        }
        ("orthoplex" | "crosspolytope", Some(n)) => {
            let n = general(n)?;
            rules(vec![
                ("vertices", int(2 * n)),
                ("edges", int(2 * n * (n - 1))),
                ("facets", pow(int(2), int(n))),
                (
                    "volume",
                    times(vec![div(pow(int(2), frac(n, 2)), fact(n)), a_pow(n)]),
                ),
                (
                    "surface",
                    times(vec![
                        pow(int(2), int(n)),
                        simplex_volume(n - 1),
                        a_pow(n - 1),
                    ]),
                ),
                ("circumradius", times(vec![div(root(2), int(2)), a.clone()])),
                (
                    "inradius",
                    times(vec![pow(sqrt(int(2 * n)), int(-1)), a.clone()]),
                ),
                ("dihedral", acos(frac(2 - n, n))),
            ])
        }
        _ => {
            return Err(format!(
                "polytope({name}): the names are tetrahedron, cube, octahedron, dodecahedron, icosahedron; cell5, cell8 (tesseract), cell16, cell24, cell120, cell600; and simplex, cube, orthoplex with the dimension, as in polytope(simplex, 5)"
            ));
        }
    })
}
