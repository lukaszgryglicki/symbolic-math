//! Tensors as nested lists: shape, index permutations, products and contractions, and the
//! curvature tensors of a metric.

use crate::canon::{add, div, mul, neg, sub};
use crate::diff::diff;
use crate::expr::Expr;
use crate::matrix;
use crate::simplify::simplify;

/// A rectangular array: the sizes of its indices and its entries in row-major order.
#[derive(Clone, Debug)]
pub struct Tensor {
    pub dims: Vec<usize>,
    pub data: Vec<Expr>,
}

fn shape(e: &Expr) -> Vec<usize> {
    match e {
        Expr::List(items) => {
            let mut d = vec![items.len()];
            if let Some(first) = items.first() {
                let inner = shape(first);
                if !inner.is_empty() && items.iter().all(|i| shape(i) == inner) {
                    d.extend(inner);
                }
            }
            d
        }
        _ => Vec::new(),
    }
}

fn pour(e: &Expr, depth: usize, out: &mut Vec<Expr>) {
    match e {
        Expr::List(items) if depth > 0 => items.iter().for_each(|i| pour(i, depth - 1, out)),
        _ => out.push(e.clone()),
    }
}

impl Tensor {
    pub fn of(e: &Expr) -> Tensor {
        let dims = shape(e);
        let mut data = Vec::new();
        pour(e, dims.len(), &mut data);
        Tensor { dims, data }
    }

    pub fn expr(&self) -> Expr {
        fn build(dims: &[usize], data: &[Expr]) -> Expr {
            match dims.split_first() {
                None => data[0].clone(),
                Some((n, rest)) => {
                    let step = data.len() / n.max(&1);
                    Expr::list(
                        (0..*n)
                            .map(|k| build(rest, &data[k * step..(k + 1) * step]))
                            .collect(),
                    )
                }
            }
        }
        build(&self.dims, &self.data)
    }

    pub fn rank(&self) -> usize {
        self.dims.len()
    }

    fn position(&self, index: &[usize]) -> usize {
        index.iter().zip(&self.dims).fold(0, |p, (i, d)| p * d + i)
    }

    /// Every multi-index of the given sizes, in row-major order.
    fn indices(dims: &[usize]) -> Vec<Vec<usize>> {
        let mut out = vec![Vec::new()];
        for &d in dims {
            out = out
                .into_iter()
                .flat_map(|i| {
                    (0..d).map(move |k| {
                        let mut j = i.clone();
                        j.push(k);
                        j
                    })
                })
                .collect();
        }
        out
    }

    fn from_fn(dims: Vec<usize>, f: impl Fn(&[usize]) -> Expr) -> Tensor {
        let data = Tensor::indices(&dims).iter().map(|i| f(i)).collect();
        Tensor { dims, data }
    }

    fn map(&self, f: impl Fn(&Expr) -> Expr) -> Tensor {
        Tensor {
            dims: self.dims.clone(),
            data: self.data.iter().map(f).collect(),
        }
    }

    /// Index `k` of the result is index `perm[k]` of this tensor.
    pub fn transpose(&self, perm: &[usize]) -> Result<Tensor, String> {
        let r = self.rank();
        let mut seen = vec![false; r];
        if perm.len() != r
            || perm
                .iter()
                .any(|&p| p >= r || std::mem::replace(&mut seen[p], true))
        {
            return Err(format!(
                "the order of the indices must list 1 to {r} once each"
            ));
        }
        let dims: Vec<usize> = perm.iter().map(|&p| self.dims[p]).collect();
        Ok(Tensor::from_fn(dims, |j| {
            let mut i = vec![0; r];
            for (k, &p) in perm.iter().enumerate() {
                i[p] = j[k];
            }
            self.data[self.position(&i)].clone()
        }))
    }

    pub fn outer(&self, o: &Tensor) -> Tensor {
        let mut dims = self.dims.clone();
        dims.extend(&o.dims);
        let data = self
            .data
            .iter()
            .flat_map(|x| o.data.iter().map(move |y| mul(vec![x.clone(), y.clone()])))
            .collect();
        Tensor { dims, data }
    }

    /// The sum over equal values of the indices `a` and `b`.
    pub fn contract(&self, a: usize, b: usize) -> Result<Tensor, String> {
        let r = self.rank();
        if a >= r || b >= r || a == b || self.dims[a] != self.dims[b] {
            return Err("contraction needs two different indices of the same size".into());
        }
        let dims: Vec<usize> = (0..r)
            .filter(|k| *k != a && *k != b)
            .map(|k| self.dims[k])
            .collect();
        Ok(Tensor::from_fn(dims, |j| {
            add((0..self.dims[a])
                .map(|s| {
                    let mut i = Vec::with_capacity(r);
                    let mut rest = j.iter();
                    for k in 0..r {
                        i.push(if k == a || k == b {
                            s
                        } else {
                            *rest.next().expect("index")
                        });
                    }
                    self.data[self.position(&i)].clone()
                })
                .collect())
        }))
    }

    /// The last `count` indices of this tensor summed against the first ones of the other.
    pub fn inner(&self, o: &Tensor, count: usize) -> Result<Tensor, String> {
        let mut t = self.outer(o);
        let r = self.rank();
        if count > r || count > o.rank() {
            return Err("more indices to sum over than the tensors have".into());
        }
        for k in 0..count {
            // in the same order: the first of the summed indices of the left with the first of the right
            t = t.contract(r - count, r - k)?;
        }
        Ok(t)
    }

    fn permutations(n: usize) -> Vec<(Vec<usize>, i64)> {
        let mut out = vec![(Vec::new(), 1)];
        for k in 0..n {
            out = out
                .into_iter()
                .flat_map(|(p, sign)| {
                    (0..=k).map(move |at| {
                        let mut q = p.clone();
                        q.insert(at, k);
                        (q, if (k - at) % 2 == 0 { sign } else { -sign })
                    })
                })
                .collect();
        }
        out
    }

    /// The mean over all orders of the indices, with the signs of the permutations when `alternating`.
    pub fn symmetrize(&self, alternating: bool) -> Result<Tensor, String> {
        let r = self.rank();
        if r > 7 || self.dims.windows(2).any(|w| w[0] != w[1]) {
            return Err("all indices must have the same size (and there may be 7 at most)".into());
        }
        let perms = Tensor::permutations(r);
        let count = Expr::int(perms.len() as i64);
        let mut total: Vec<Vec<Expr>> = vec![Vec::new(); self.data.len()];
        for (p, sign) in &perms {
            let turned = self.transpose(p)?;
            for (slot, x) in total.iter_mut().zip(turned.data) {
                slot.push(if alternating && *sign < 0 { neg(x) } else { x });
            }
        }
        Ok(Tensor {
            dims: self.dims.clone(),
            data: total
                .into_iter()
                .map(|parts| simplify(&div(add(parts), count.clone())))
                .collect(),
        })
    }
}

/// The sign of a permutation of `0..n` given as a list; 0 when an entry repeats.
pub fn signature(p: &[usize]) -> i64 {
    let mut sign = 1;
    for i in 0..p.len() {
        for j in i + 1..p.len() {
            if p[i] == p[j] {
                return 0;
            }
            if p[i] > p[j] {
                sign = -sign;
            }
        }
    }
    sign
}

pub fn levi_civita(n: usize) -> Tensor {
    Tensor::from_fn(vec![n; n], |i| Expr::int(signature(i)))
}

/// The exterior product of two alternating tensors.
pub fn wedge(a: &Tensor, b: &Tensor) -> Result<Tensor, String> {
    let (p, q) = (a.rank() as u32, b.rank() as u32);
    let f = crate::ntheory::factorial;
    let weight = Expr::rational(crate::num::Rat::from((f(p + q), f(p) * f(q))));
    Ok(a.outer(b)
        .symmetrize(true)?
        .map(|x| simplify(&mul(vec![weight.clone(), x.clone()]))))
}

/// The Hodge dual of an alternating tensor for the Euclidean metric.
pub fn hodge(t: &Tensor) -> Result<Tensor, String> {
    let p = t.rank();
    let n = t.dims.first().copied().unwrap_or(0);
    if p == 0 || p > n || t.dims.iter().any(|d| *d != n) || n > 6 {
        return Err("the Hodge dual needs a tensor with up to 6 indices of one size".into());
    }
    let dual = t.inner(&levi_civita(n), p)?;
    let weight = Expr::integer(crate::ntheory::factorial(p as u32));
    Ok(dual.map(|x| simplify(&div(x.clone(), weight.clone()))))
}

/// The Kronecker product of two matrices.
pub fn kron(a: &matrix::Rows, b: &matrix::Rows) -> matrix::Rows {
    let mut out = Vec::new();
    for ra in a {
        for rb in b {
            out.push(
                ra.iter()
                    .flat_map(|x| rb.iter().map(move |y| mul(vec![x.clone(), y.clone()])))
                    .collect(),
            );
        }
    }
    out
}

/// A metric with its inverse and its coordinates.
pub struct Metric {
    g: matrix::Rows,
    inverse: matrix::Rows,
    x: Vec<String>,
}

impl Metric {
    pub fn new(g: &Expr, coordinates: &[String]) -> Result<Metric, String> {
        let rows = matrix::rows(g).ok_or("the metric must be a square matrix")?;
        let n = rows.len();
        if rows.iter().any(|r| r.len() != n) || coordinates.len() != n {
            return Err(format!(
                "the metric must be a square matrix with one coordinate for each row: {n} row(s), {} coordinate(s)",
                coordinates.len()
            ));
        }
        let inverse = matrix::inverse(&rows)?
            .into_iter()
            .map(|r| r.iter().map(simplify).collect())
            .collect();
        Ok(Metric {
            g: rows,
            inverse,
            x: coordinates.to_vec(),
        })
    }

    fn n(&self) -> usize {
        self.x.len()
    }

    /// Christoffel symbols of the second kind, `[i][j][k]` for the upper index `i`.
    pub fn christoffel(&self) -> Tensor {
        let n = self.n();
        let d = |a: usize, b: usize, c: usize| diff(&self.g[a][b], &self.x[c]);
        Tensor::from_fn(vec![n; 3], |ix| {
            let (i, j, k) = (ix[0], ix[1], ix[2]);
            simplify(&div(
                add((0..n)
                    .map(|l| {
                        mul(vec![
                            self.inverse[i][l].clone(),
                            add(vec![d(l, k, j), d(l, j, k), neg(d(j, k, l))]),
                        ])
                    })
                    .collect()),
                Expr::int(2),
            ))
        })
    }

    /// The Riemann tensor `R^i_jkl` as `[i][j][k][l]`.
    pub fn riemann(&self) -> Tensor {
        let n = self.n();
        let c = self.christoffel();
        let gamma = |i: usize, j: usize, k: usize| c.data[(i * n + j) * n + k].clone();
        Tensor::from_fn(vec![n; 4], |ix| {
            let (i, j, k, l) = (ix[0], ix[1], ix[2], ix[3]);
            let mut terms = vec![
                diff(&gamma(i, j, l), &self.x[k]),
                neg(diff(&gamma(i, j, k), &self.x[l])),
            ];
            for m in 0..n {
                terms.push(mul(vec![gamma(i, k, m), gamma(m, j, l)]));
                terms.push(neg(mul(vec![gamma(i, l, m), gamma(m, j, k)])));
            }
            simplify(&add(terms))
        })
    }

    pub fn ricci(&self) -> Tensor {
        let n = self.n();
        let r = self.riemann();
        Tensor::from_fn(vec![n; 2], |ix| {
            simplify(&add((0..n)
                .map(|i| r.data[((i * n + ix[0]) * n + i) * n + ix[1]].clone())
                .collect()))
        })
    }

    fn scalar_of(&self, ricci: &Tensor) -> Expr {
        let n = self.n();
        simplify(&add((0..n * n)
            .map(|p| {
                mul(vec![
                    self.inverse[p / n][p % n].clone(),
                    ricci.data[p].clone(),
                ])
            })
            .collect()))
    }

    pub fn scalar(&self) -> Expr {
        self.scalar_of(&self.ricci())
    }

    pub fn einstein(&self) -> Tensor {
        let n = self.n();
        let ricci = self.ricci();
        let half = div(self.scalar_of(&ricci), Expr::int(2));
        Tensor::from_fn(vec![n; 2], |ix| {
            simplify(&sub(
                ricci.data[ix[0] * n + ix[1]].clone(),
                mul(vec![half.clone(), self.g[ix[0]][ix[1]].clone()]),
            ))
        })
    }

    fn tensor_of(m: &matrix::Rows) -> Tensor {
        Tensor::of(&matrix::from_rows(m.clone()))
    }

    /// The index `k` of the tensor raised (with the inverse metric) or lowered.
    pub fn shift(&self, t: &Tensor, k: usize, up: bool) -> Result<Tensor, String> {
        let r = t.rank();
        if k >= r || t.dims[k] != self.n() {
            return Err("there is no such index of the size of the metric".into());
        }
        let m = Metric::tensor_of(if up { &self.inverse } else { &self.g });
        // the new index comes first: move it back to place k
        let moved = m.outer(t).contract(1, 2 + k)?;
        let mut order: Vec<usize> = (1..r).collect();
        order.insert(k, 0);
        Ok(moved.transpose(&order)?.map(simplify))
    }

    /// The Kretschmann scalar `R_abcd R^abcd`.
    pub fn kretschmann(&self) -> Result<Expr, String> {
        let up = self.riemann();
        let down = self.shift(&up, 0, false)?;
        let mut raised = up;
        for k in 1..4 {
            raised = self.shift(&raised, k, true)?;
        }
        Ok(simplify(&add(down
            .data
            .iter()
            .zip(&raised.data)
            .map(|(a, b)| mul(vec![a.clone(), b.clone()]))
            .collect())))
    }

    /// The right sides of the geodesic equations: the second derivative of every coordinate in
    /// terms of the first derivatives `v`.
    pub fn geodesics(&self, v: &[Expr]) -> Vec<Expr> {
        let n = self.n();
        let c = self.christoffel();
        (0..n)
            .map(|i| {
                simplify(&neg(add((0..n * n)
                    .map(|p| {
                        mul(vec![
                            c.data[i * n * n + p].clone(),
                            v[p / n].clone(),
                            v[p % n].clone(),
                        ])
                    })
                    .collect())))
            })
            .collect()
    }

    /// The covariant derivative of a vector field with an upper index: `[i][j]` for `D_j V^i`.
    pub fn covariant(&self, v: &[Expr]) -> Result<Tensor, String> {
        let n = self.n();
        if v.len() != n {
            return Err(format!("the field needs {n} components"));
        }
        let c = self.christoffel();
        Ok(Tensor::from_fn(vec![n; 2], |ix| {
            let (i, j) = (ix[0], ix[1]);
            let mut terms = vec![diff(&v[i], &self.x[j])];
            for (k, vk) in v.iter().enumerate() {
                terms.push(mul(vec![c.data[(i * n + j) * n + k].clone(), vk.clone()]));
            }
            simplify(&add(terms))
        }))
    }

    /// The divergence of a vector field with an upper index.
    pub fn divergence(&self, v: &[Expr]) -> Result<Expr, String> {
        let n = self.n();
        if v.len() != n {
            return Err(format!("the field needs {n} components"));
        }
        let det = matrix::det(&self.g)?;
        // (1/sqrt g) d_i (sqrt g v^i) = d_i v^i + v^i d_i g / (2 g)
        Ok(simplify(&add((0..n)
            .map(|i| {
                add(vec![
                    diff(&v[i], &self.x[i]),
                    div(
                        mul(vec![v[i].clone(), diff(&det, &self.x[i])]),
                        mul(vec![Expr::int(2), det.clone()]),
                    ),
                ])
            })
            .collect())))
    }

    /// The Laplace-Beltrami operator on a function.
    pub fn laplacian(&self, f: &Expr) -> Result<Expr, String> {
        let n = self.n();
        let gradient: Vec<Expr> = (0..n)
            .map(|i| {
                add((0..n)
                    .map(|j| mul(vec![self.inverse[i][j].clone(), diff(f, &self.x[j])]))
                    .collect())
            })
            .collect();
        self.divergence(&gradient)
    }
}
