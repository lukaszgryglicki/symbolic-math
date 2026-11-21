//! Systems of control with several inputs and outputs: `ss(A, B, C, D)` of any sizes, or a
//! matrix whose entries are systems with one input and one output. They are joined, cut to
//! the states that they need, closed into loops, watched (Kalman) and steered (LQG).

use crate::arrays::{tidy, word};
use crate::canon::{add, div, mul};
use crate::control::{
    Sys, is_system, list_of, poly_add, poly_scaled, resolvent, state_space, tf_to_ss, trimmed,
};
use crate::decomp::{self, CMat};
use crate::eval::Session;
use crate::expr::Expr;
use crate::matrix::{self, Rows};
use crate::numeric;
use crate::pencils::{solved, times};
use rug::ops::Pow;
use rug::{Complex, Float};

type Res = Result<Expr, String>;

/// A matrix of expressions that may have no rows or no columns.
#[derive(Clone)]
struct Mat {
    r: usize,
    c: usize,
    v: Vec<Expr>,
}

impl Mat {
    fn zeros(r: usize, c: usize) -> Mat {
        Mat {
            r,
            c,
            v: vec![Expr::zero(); r * c],
        }
    }

    fn eye(n: usize) -> Mat {
        let mut m = Mat::zeros(n, n);
        for i in 0..n {
            m.v[i * n + i] = Expr::one();
        }
        m
    }

    fn of(rows: &Rows, c: usize) -> Mat {
        Mat {
            r: rows.len(),
            c,
            v: rows.iter().flatten().cloned().collect(),
        }
    }

    /// A matrix that is given: a number is one entry, a flat list a row (a column when
    /// `column` is said).
    fn read(e: &Expr, column: bool) -> Mat {
        match matrix::rows(e) {
            Some(rows) => Mat::of(&rows, rows[0].len()),
            None => {
                let items = list_of(e);
                match column {
                    true => Mat {
                        r: items.len(),
                        c: 1,
                        v: items,
                    },
                    false => Mat {
                        r: 1,
                        c: items.len(),
                        v: items,
                    },
                }
            }
        }
    }

    fn at(&self, i: usize, j: usize) -> &Expr {
        &self.v[i * self.c + j]
    }

    fn rows(&self) -> Rows {
        (0..self.r)
            .map(|i| self.v[i * self.c..(i + 1) * self.c].to_vec())
            .collect()
    }

    fn expr(&self) -> Expr {
        matrix::from_rows(self.rows())
    }

    fn map(&self, f: impl Fn(&Expr) -> Expr) -> Mat {
        Mat {
            r: self.r,
            c: self.c,
            v: self.v.iter().map(f).collect(),
        }
    }

    fn mul(&self, o: &Mat) -> Mat {
        debug_assert_eq!(self.c, o.r);
        let mut out = Mat::zeros(self.r, o.c);
        for i in 0..self.r {
            for j in 0..o.c {
                let terms: Vec<Expr> = (0..self.c)
                    .filter(|k| !self.at(i, *k).is_zero() && !o.at(*k, j).is_zero())
                    .map(|k| mul(vec![self.at(i, k).clone(), o.at(k, j).clone()]))
                    .collect();
                out.v[i * o.c + j] = tidy(add(terms));
            }
        }
        out
    }

    fn plus(&self, o: &Mat) -> Mat {
        debug_assert_eq!((self.r, self.c), (o.r, o.c));
        Mat {
            r: self.r,
            c: self.c,
            v: self
                .v
                .iter()
                .zip(&o.v)
                .map(|(x, y)| tidy(add(vec![x.clone(), y.clone()])))
                .collect(),
        }
    }

    fn minus(&self, o: &Mat) -> Mat {
        self.plus(&o.scaled(&Expr::int(-1)))
    }

    fn scaled(&self, k: &Expr) -> Mat {
        self.map(|x| tidy(mul(vec![k.clone(), x.clone()])))
    }

    fn t(&self) -> Mat {
        let mut out = Mat::zeros(self.c, self.r);
        for i in 0..self.r {
            for j in 0..self.c {
                out.v[j * self.r + i] = self.at(i, j).clone();
            }
        }
        out
    }

    fn inv(&self, what: &str) -> Result<Mat, String> {
        if self.r != self.c {
            return Err(format!("{what} is not square"));
        }
        if self.r == 0 {
            return Ok(self.clone());
        }
        if self.has_float() {
            // (decimal numbers: no entry is exactly 0 or 1)
            let numbers = self.numbers(what)?;
            let unit = decomp::c_unit(self.r, crate::mfun::bits());
            let found = solved(&numbers, &unit).ok_or_else(|| format!("{what} has no inverse"))?;
            return Ok(Mat::read(&decomp::from_complex(&found), false));
        }
        let rows = matrix::inverse(&self.rows()).map_err(|_| format!("{what} has no inverse"))?;
        Ok(Mat::of(&rows, self.c).map(|x| tidy(x.clone())))
    }

    fn beside(&self, o: &Mat) -> Mat {
        debug_assert_eq!(self.r, o.r);
        let mut out = Mat::zeros(self.r, self.c + o.c);
        for i in 0..self.r {
            for j in 0..self.c {
                out.v[i * out.c + j] = self.at(i, j).clone();
            }
            for j in 0..o.c {
                out.v[i * out.c + self.c + j] = o.at(i, j).clone();
            }
        }
        out
    }

    fn above(&self, o: &Mat) -> Mat {
        debug_assert_eq!(self.c, o.c);
        Mat {
            r: self.r + o.r,
            c: self.c,
            v: self.v.iter().chain(&o.v).cloned().collect(),
        }
    }

    /// [[a, b], [c, d]]
    fn blocks(a: &Mat, b: &Mat, c: &Mat, d: &Mat) -> Mat {
        a.beside(b).above(&c.beside(d))
    }

    fn pick(&self, rows: &[usize], cols: &[usize]) -> Mat {
        let mut out = Mat::zeros(rows.len(), cols.len());
        for (i, r) in rows.iter().enumerate() {
            for (j, c) in cols.iter().enumerate() {
                out.v[i * cols.len() + j] = self.at(*r, *c).clone();
            }
        }
        out
    }

    fn has_float(&self) -> bool {
        self.v.iter().any(Expr::has_float)
    }

    fn numbers(&self, who: &str) -> Result<CMat, String> {
        let prec = crate::mfun::bits();
        self.rows()
            .iter()
            .map(|r| {
                r.iter()
                    .map(|x| {
                        numeric::evalf(x, prec).map_err(|_| format!("{who}: {x} is no number"))
                    })
                    .collect()
            })
            .collect()
    }
}

fn span(from: usize, to: usize) -> Vec<usize> {
    (from..to).collect()
}

/// A system in the form of states: x' = A x + B u, y = C x + D u (x[k + 1] for a sampled
/// one).
#[derive(Clone)]
struct Ss {
    a: Mat,
    b: Mat,
    c: Mat,
    d: Mat,
    ts: Option<Expr>,
}

impl Ss {
    fn n(&self) -> usize {
        self.a.r
    }

    fn inputs(&self) -> usize {
        self.d.c
    }

    fn outputs(&self) -> usize {
        self.d.r
    }

    fn read(m: [Rows; 4], ts: Option<Expr>, who: &str) -> Result<Ss, String> {
        let [a, b, c, d] = m;
        let n = a.first().map_or(0, Vec::len);
        let (p, inputs) = (d.len(), d.first().map_or(0, Vec::len));
        let bad = || {
            format!(
                "{who}: ss(A, B, C, D): A is n by n, B has n rows, C has n columns, D a row for every output and a column for every input"
            )
        };
        if p == 0 || inputs == 0 || d.iter().any(|r| r.len() != inputs) {
            return Err(bad());
        }
        if n == 0 {
            return Ok(Ss {
                a: Mat::zeros(0, 0),
                b: Mat::zeros(0, inputs),
                c: Mat::zeros(p, 0),
                d: Mat::of(&d, inputs),
                ts,
            });
        }
        if a.len() != n
            || a.iter().any(|r| r.len() != n)
            || b.len() != n
            || b.iter().any(|r| r.len() != inputs)
            || c.len() != p
            || c.iter().any(|r| r.len() != n)
        {
            return Err(bad());
        }
        Ok(Ss {
            a: Mat::of(&a, n),
            b: Mat::of(&b, inputs),
            c: Mat::of(&c, n),
            d: Mat::of(&d, inputs),
            ts,
        })
    }

    fn gain(d: Mat, ts: Option<Expr>) -> Ss {
        Ss {
            a: Mat::zeros(0, 0),
            b: Mat::zeros(0, d.c),
            c: Mat::zeros(d.r, 0),
            d,
            ts,
        }
    }

    /// The object as it is written; a system without states has A = [[]].
    fn object(&self) -> Expr {
        let none = self.n() == 0;
        let a = match none {
            true => Expr::list(vec![Expr::list(Vec::new())]),
            false => self.a.expr(),
        };
        let c = match none {
            true => Expr::list(vec![Expr::list(Vec::new()); self.outputs()]),
            false => self.c.expr(),
        };
        let mut args = vec![a, self.b.expr(), c, self.d.expr()];
        args.extend(self.ts.clone());
        Expr::call("ss", args)
    }

    /// Inputs and outputs change places.
    fn turned(&self) -> Ss {
        Ss {
            a: self.a.t(),
            b: self.c.t(),
            c: self.b.t(),
            d: self.d.t(),
            ts: self.ts.clone(),
        }
    }

    /// The part of the states that the inputs reach.
    fn steerable(&self) -> Result<Ss, String> {
        let n = self.n();
        if n == 0 {
            return Ok(self.clone());
        }
        let mut reach = self.b.clone();
        let mut block = self.b.clone();
        for _ in 1..n {
            block = self.a.mul(&block);
            reach = reach.beside(&block);
        }
        // with decimal numbers: what rounding left is nothing
        let prec = crate::mfun::bits();
        let size = |e: &Expr| -> Option<Float> {
            numeric::evalf(e, prec)
                .ok()
                .map(|z| Float::with_val(prec, z.abs_ref()))
        };
        let loose = reach.has_float() || self.a.has_float();
        let top = reach
            .v
            .iter()
            .filter_map(size)
            .fold(Float::with_val(prec, 0), |m, x| m.max(&x));
        let least = top
            * Float::with_val(
                prec,
                Float::i_pow_u(10, numeric::digits().saturating_sub(6)),
            )
            .recip();
        let zero = move |e: &Expr| -> bool {
            match loose {
                true => size(e).is_none_or(|s| s <= least),
                false => crate::simplify::is_zero(e),
            }
        };
        let (reduced, rank, _) = matrix::eliminate_with(reach.t().rows(), true, &zero);
        if rank == n {
            return Ok(self.clone());
        }
        // a basis: the rows that are left, and a unit vector where none of them begins
        let pivots: Vec<usize> = (0..rank)
            .filter_map(|i| (0..n).find(|&j| !zero(&reduced[i][j])))
            .collect();
        let mut basis: Rows = reduced[..rank].to_vec();
        for j in (0..n).filter(|j| !pivots.contains(j)) {
            basis.push((0..n).map(|k| Expr::int(i64::from(k == j))).collect());
        }
        let t = Mat::of(&basis, n).t();
        let back = t.inv("the change of the states")?;
        let (keep, all_in, all_out) = (
            span(0, rank),
            span(0, self.inputs()),
            span(0, self.outputs()),
        );
        Ok(Ss {
            a: back.mul(&self.a).mul(&t).pick(&keep, &keep),
            b: back.mul(&self.b).pick(&keep, &all_in),
            c: self.c.mul(&t).pick(&all_out, &keep),
            d: self.d.clone(),
            ts: self.ts.clone(),
        })
    }

    /// With the states alone that inputs reach and outputs see.
    fn minimal(&self) -> Result<Ss, String> {
        Ok(self.steerable()?.turned().steerable()?.turned())
    }

    /// The system with the interconnection u = K y + r closed: from r to y.
    fn closed(&self, k: &Mat) -> Result<Ss, String> {
        let m = self.inputs();
        let w = Mat::eye(m)
            .minus(&k.mul(&self.d))
            .inv("the loop (it has no states of its own to break it: I - K*D)")?;
        let wk = w.mul(k);
        Ok(Ss {
            a: self.a.plus(&self.b.mul(&wk).mul(&self.c)),
            b: self.b.mul(&w),
            c: self.c.plus(&self.d.mul(&wk).mul(&self.c)),
            d: self.d.mul(&w),
            ts: self.ts.clone(),
        })
    }

    fn picked(&self, outputs: &[usize], inputs: &[usize]) -> Ss {
        let states = span(0, self.n());
        Ss {
            a: self.a.clone(),
            b: self.b.pick(&states, inputs),
            c: self.c.pick(outputs, &states),
            d: self.d.pick(outputs, inputs),
            ts: self.ts.clone(),
        }
    }

    /// Side by side: the inputs and the outputs of both, nothing between them.
    fn beside(&self, o: &Ss) -> Ss {
        let (n1, n2) = (self.n(), o.n());
        Ss {
            a: Mat::blocks(&self.a, &Mat::zeros(n1, n2), &Mat::zeros(n2, n1), &o.a),
            b: Mat::blocks(
                &self.b,
                &Mat::zeros(n1, o.inputs()),
                &Mat::zeros(n2, self.inputs()),
                &o.b,
            ),
            c: Mat::blocks(
                &self.c,
                &Mat::zeros(self.outputs(), n2),
                &Mat::zeros(o.outputs(), n1),
                &o.c,
            ),
            d: Mat::blocks(
                &self.d,
                &Mat::zeros(self.outputs(), o.inputs()),
                &Mat::zeros(o.outputs(), self.inputs()),
                &o.d,
            ),
            ts: self.ts.clone().or_else(|| o.ts.clone()),
        }
    }

    fn sum(&self, o: &Ss) -> Ss {
        Ss {
            a: Mat::blocks(
                &self.a,
                &Mat::zeros(self.n(), o.n()),
                &Mat::zeros(o.n(), self.n()),
                &o.a,
            ),
            b: self.b.above(&o.b),
            c: self.c.beside(&o.c),
            d: self.d.plus(&o.d),
            ts: self.ts.clone().or_else(|| o.ts.clone()),
        }
    }

    /// `self` after `o`: y = self(o(u)).
    fn after(&self, o: &Ss) -> Ss {
        Ss {
            a: Mat::blocks(
                &self.a,
                &self.b.mul(&o.c),
                &Mat::zeros(o.n(), self.n()),
                &o.a,
            ),
            b: self.b.mul(&o.d).above(&o.b),
            c: self.c.beside(&self.d.mul(&o.c)),
            d: self.d.mul(&o.d),
            ts: self.ts.clone().or_else(|| o.ts.clone()),
        }
    }

    fn inverse(&self) -> Result<Ss, String> {
        if self.inputs() != self.outputs() {
            return Err("the inverse of a system needs as many outputs as inputs".into());
        }
        let di = self
            .d
            .inv("the part D of the system that goes straight through (the inverse of the system needs it)")?;
        let bd = self.b.mul(&di);
        Ok(Ss {
            a: self.a.minus(&bd.mul(&self.c)),
            b: bd,
            c: di.mul(&self.c).scaled(&Expr::int(-1)),
            d: di,
            ts: self.ts.clone(),
        })
    }

    /// One input and one output in every entry of a matrix of outputs by inputs.
    fn spread(&self, outputs: usize, inputs: usize) -> Ss {
        let (rows, cols) = (vec![0; outputs], vec![0; inputs]);
        self.picked(&rows, &cols)
    }
}

/// An `ss(...)` with several inputs or outputs.
pub(crate) fn is_wide(e: &Expr) -> bool {
    state_space(e).is_some_and(|(m, _)| m[3].len() > 1 || m[3].first().is_some_and(|r| r.len() > 1))
}

/// A matrix whose entries are systems (ratios of polynomials in s or z count when one of
/// them is there).
fn holds_systems(e: &Expr) -> bool {
    matrix::rows(e).is_some_and(|rows| {
        rows.iter().flatten().all(|x| x.as_list().is_none())
            && rows
                .iter()
                .flatten()
                .any(|x| is_system(x) || x.has_sym("s") || x.has_sym("z"))
    })
}

fn same_time(a: &Option<Expr>, b: &Option<Expr>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => x == y || *x == Expr::int(-1) || *y == Expr::int(-1),
        _ => false,
    }
}

/// Places counted from 1, as they are given.
fn places(e: &Expr, most: usize, what: &str) -> Result<Vec<usize>, String> {
    list_of(e)
        .iter()
        .flat_map(list_of)
        .map(|x| {
            x.as_i64()
                .filter(|k| (1..=most as i64).contains(k))
                .map(|k| k as usize - 1)
                .ok_or_else(|| format!("{what}: {x} is none of the numbers 1 to {most}"))
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq)]
enum Form {
    States,
    Grid,
}

impl Session {
    /// A system of any numbers of inputs and outputs in the form of states.
    fn wide_of(&mut self, e: &Expr, who: &str) -> Result<Ss, String> {
        if let Some((m, ts)) = state_space(e) {
            return Ss::read(m, ts, who);
        }
        if let Some(rows) = matrix::rows(e) {
            return self.of_grid(&rows, who);
        }
        let sys = self.sys_of(e, who)?;
        let m = tf_to_ss(&sys.num, &sys.den).map_err(|e| format!("{who}: {e}"))?;
        Ss::read(m, sys.ts, who)
    }

    /// The states of a matrix of systems: those of every entry, side by side.
    #[allow(clippy::needless_range_loop)]
    fn of_grid(&mut self, rows: &Rows, who: &str) -> Result<Ss, String> {
        let (p, m) = (rows.len(), rows[0].len());
        let mut parts = Vec::new();
        let mut ts: Option<Option<Expr>> = None;
        let mut total = 0;
        for (i, row) in rows.iter().enumerate() {
            for (j, e) in row.iter().enumerate() {
                let sys = self.sys_of(e, who)?;
                let own = tf_to_ss(&sys.num, &sys.den).map_err(|e| format!("{who}: {e}"))?;
                let size = own[0].first().map_or(0, Vec::len);
                if size > 0 || sys.ts.is_some() {
                    match &ts {
                        Some(known) if !same_time(known, &sys.ts) => {
                            return Err(format!(
                                "{who}: the entries of the matrix are not all sampled at one time"
                            ));
                        }
                        Some(Some(known)) if *known != Expr::int(-1) => {}
                        _ => ts = Some(sys.ts.clone()),
                    }
                }
                total += size;
                parts.push((i, j, own, size));
            }
        }
        let (mut a, mut b, mut c, mut d) = (
            Mat::zeros(total, total),
            Mat::zeros(total, m),
            Mat::zeros(p, total),
            Mat::zeros(p, m),
        );
        let mut at = 0;
        for (i, j, own, size) in parts {
            d.v[i * m + j] = own[3][0][0].clone();
            for x in 0..size {
                for y in 0..size {
                    a.v[(at + x) * total + at + y] = own[0][x][y].clone();
                }
                b.v[(at + x) * m + j] = own[1][x][0].clone();
                c.v[i * total + at + x] = own[2][0][x].clone();
            }
            at += size;
        }
        Ok(Ss {
            a,
            b,
            c,
            d,
            ts: ts.flatten(),
        })
    }

    /// The matrix of the systems from every input to every output; what a numerator and
    /// its denominator share cancels.
    fn grid_of(&mut self, s: &Ss, who: &str) -> Result<Vec<Vec<Sys>>, String> {
        let (n, m, p) = (s.n(), s.inputs(), s.outputs());
        let mut out = Vec::with_capacity(p);
        let (den, parts) = match n {
            0 => (vec![Expr::one()], Vec::new()),
            _ => resolvent(&s.a.rows())?,
        };
        // with decimal numbers nothing cancels in a ratio: every entry from the states of its own
        let loose = n > 0 && [&s.a, &s.b, &s.c, &s.d].iter().any(|x| x.has_float());
        for i in 0..p {
            let mut row = Vec::with_capacity(m);
            let ci: Rows = vec![(0..n).map(|k| s.c.at(i, k).clone()).collect()];
            for j in 0..m {
                if loose {
                    let own = s.picked(&[i], &[j]).minimal()?;
                    let k = own.n();
                    let (den, parts) = match k {
                        0 => (vec![Expr::one()], Vec::new()),
                        _ => resolvent(&own.a.rows())?,
                    };
                    let mut num = vec![Expr::zero()];
                    for part in &parts {
                        let through =
                            matrix::matmul(&matrix::matmul(&own.c.rows(), part)?, &own.b.rows())?;
                        num.push(tidy(through[0][0].clone()));
                    }
                    let num = poly_add(&num, &poly_scaled(&den, own.d.at(0, 0)));
                    let whole = Sys {
                        num: trimmed(crate::control::dusted(num)),
                        den: crate::control::dusted(den),
                        ts: s.ts.clone(),
                    };
                    row.push(self.sys_of_expr(&whole.expr(), s.ts.clone(), who)?);
                    continue;
                }
                let bj: Rows = (0..n).map(|k| vec![s.b.at(k, j).clone()]).collect();
                let mut num = vec![Expr::zero()];
                for part in &parts {
                    let through = matrix::matmul(&matrix::matmul(&ci, part)?, &bj)?;
                    num.push(tidy(through[0][0].clone()));
                }
                let num = poly_add(&num, &poly_scaled(&den, s.d.at(i, j)));
                let whole = Sys {
                    num: trimmed(num),
                    den: den.clone(),
                    ts: s.ts.clone(),
                };
                row.push(self.sys_of_expr(&whole.expr(), s.ts.clone(), who)?);
            }
            out.push(row);
        }
        Ok(out)
    }

    fn wide_shown(&mut self, s: &Ss, form: Form, who: &str) -> Res {
        match form {
            Form::States => Ok(s.object()),
            Form::Grid => {
                let grid = self.grid_of(s, who)?;
                Ok(match (s.outputs(), s.inputs()) {
                    (1, 1) => grid[0][0].object(),
                    _ => Expr::list(
                        grid.iter()
                            .map(|row| Expr::list(row.iter().map(Sys::object).collect()))
                            .collect(),
                    ),
                })
            }
        }
    }

    /// The eigenvalues of A.
    fn wide_poles(&mut self, s: &Ss) -> Result<Vec<Expr>, String> {
        match s.n() {
            0 => Ok(Vec::new()),
            _ => Ok(list_of(&self.call("eigenvalues", &[s.a.expr()])?)),
        }
    }

    fn wide_stable(&mut self, s: &Ss, who: &str) -> Result<bool, String> {
        let prec = crate::mfun::bits();
        for p in self.wide_poles(s)? {
            let z = numeric::evalf(&p, prec)
                .map_err(|_| format!("{who}: the pole {p} is no number"))?;
            let inside = match &s.ts {
                None => z.real().is_sign_negative() && !z.real().is_zero(),
                Some(_) => Float::with_val(prec, z.abs_ref()) < 1,
            };
            if !inside {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// The places where the matrix [[A - x I, B], [C, D]] loses rank: the zeros of the
    /// system (of its minimal form).
    fn wide_zeros(&mut self, s: &Ss, who: &str) -> Res {
        let s = s.minimal()?;
        let (n, m, p) = (s.n(), s.inputs(), s.outputs());
        let var = Expr::sym(if s.ts.is_some() { "z" } else { "s" });
        let shifted = s.a.minus(&Mat::eye(n).scaled(&var));
        let whole = Mat::blocks(&shifted, &s.b, &s.c, &s.d);
        let least = m.min(p);
        // every choice of as many inputs (or outputs) as the fewer of the two
        let mut choices: Vec<Vec<usize>> = vec![Vec::new()];
        for k in 0..m.max(p) {
            let mut more = Vec::new();
            for c in &choices {
                if c.len() < least {
                    let mut with = c.clone();
                    with.push(k);
                    more.push(with);
                }
            }
            choices.extend(more);
            if choices.len() > 5000 {
                return Err(format!("{who}: too many inputs and outputs for the zeros"));
            }
        }
        choices.retain(|c| c.len() == least);
        let mut shared: Option<Expr> = None;
        for choice in choices {
            let mut rows = span(0, n);
            let mut cols = span(0, n);
            match m > p {
                true => {
                    rows.extend(span(n, n + p));
                    cols.extend(choice.iter().map(|k| n + k));
                }
                false => {
                    rows.extend(choice.iter().map(|k| n + k));
                    cols.extend(span(n, n + m));
                }
            }
            let minor = matrix::det(&whole.pick(&rows, &cols).rows())?;
            let minor = self.call("expand", &[minor])?;
            if minor.is_zero() {
                continue;
            }
            shared = Some(match shared {
                None => minor,
                Some(so_far) => self.call("gcd", &[so_far, minor])?,
            });
        }
        match shared {
            None => Ok(Expr::list(Vec::new())),
            Some(poly) if !poly.has(&var) => Ok(Expr::list(Vec::new())),
            Some(poly) => {
                let coefficients = self.poly_of(&poly, &var, who)?;
                Ok(Expr::list(self.roots_of(&coefficients)?))
            }
        }
    }

    /// The matrix of the system at a place of the plane of s (of z), as numbers.
    fn wide_at(s: &Ss, numbers: &[CMat; 4], x: &Complex) -> Option<CMat> {
        let prec = x.prec().0;
        let [a, b, c, d] = numbers;
        if s.n() == 0 {
            return Some(d.clone());
        }
        let n = s.n();
        let shifted: CMat = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| match i == j {
                        true => Complex::with_val(prec, x - &a[i][j]),
                        false => Complex::with_val(prec, -&a[i][j]),
                    })
                    .collect()
            })
            .collect();
        let inner = solved(&shifted, b)?;
        let through = times(c, &inner);
        Some(
            through
                .iter()
                .zip(d)
                .map(|(r, q)| {
                    r.iter()
                        .zip(q)
                        .map(|(x, y)| Complex::with_val(prec, x + y))
                        .collect()
                })
                .collect(),
        )
    }

    fn wide_numbers(s: &Ss, who: &str) -> Result<[CMat; 4], String> {
        let prec = crate::mfun::bits();
        let fill = |m: &Mat| -> Result<CMat, String> {
            match (m.r, m.c) {
                (0, _) | (_, 0) => Ok(vec![vec![Complex::new(prec); m.c.max(1)]; m.r.max(1)]),
                _ => m.numbers(who),
            }
        };
        Ok([fill(&s.a)?, fill(&s.b)?, fill(&s.c)?, fill(&s.d)?])
    }

    /// The place of the plane for a frequency: j w, or e^(j w Ts) for a sampled system.
    fn wide_place(s: &Ss, w: &Float) -> Complex {
        let prec = w.prec();
        match &s.ts {
            None => Complex::with_val(prec, (0, w)),
            Some(ts) => {
                let step = match numeric::real_value(ts, numeric::digits() + 10) {
                    Some(t) if t > 0 => Float::with_val(prec, t),
                    _ => Float::with_val(prec, 1),
                };
                let angle = Float::with_val(prec, w * step);
                let (sin, cos) = angle.sin_cos(Float::new(prec));
                Complex::with_val(prec, (cos, sin))
            }
        }
    }

    /// The singular values of the matrix of the system at a frequency, the largest first.
    fn wide_gains(s: &Ss, numbers: &[CMat; 4], w: &Float) -> Result<Vec<Float>, String> {
        let at = Self::wide_at(s, numbers, &Self::wide_place(s, w))
            .ok_or("the system has a pole at this frequency")?;
        let (_, mut values, _) = decomp::svd(&at)?;
        values.sort_by(|x, y| y.partial_cmp(x).unwrap_or(std::cmp::Ordering::Equal));
        Ok(values)
    }

    /// Frequencies around the poles of a system: 61 of them in equal steps of the logarithm.
    fn wide_frequencies(&mut self, s: &Ss, who: &str) -> Result<Vec<Float>, String> {
        let prec = crate::mfun::bits();
        let mut sizes: Vec<Float> = Vec::new();
        for p in self.wide_poles(s)? {
            let z = numeric::evalf(&p, prec)
                .map_err(|_| format!("{who}: the pole {p} is no number"))?;
            let size = Float::with_val(prec, z.abs_ref());
            if !size.is_zero() {
                sizes.push(size);
            }
        }
        let one = Float::with_val(prec, 1);
        let low = sizes.iter().fold(one.clone(), |m, x| m.min(x));
        let high = sizes.iter().fold(one, |m, x| m.max(x));
        let (from, to) = (low.log10().floor() - 1u32, high.log10().ceil() + 1u32);
        let top = match &s.ts {
            None => None,
            Some(ts) => {
                let step = match numeric::real_value(ts, numeric::digits() + 10) {
                    Some(t) if t > 0 => Float::with_val(prec, t),
                    _ => Float::with_val(prec, 1),
                };
                Some(Float::with_val(prec, rug::float::Constant::Pi) / step)
            }
        };
        Ok((0..=60)
            .map(|k| {
                let exponent = Float::with_val(prec, &to - &from) * k / 60u32 + &from;
                let w = Float::with_val(prec, 10).pow(exponent);
                match &top {
                    Some(t) if w > *t => t.clone(),
                    _ => w,
                }
            })
            .collect())
    }

    /// The greatest gain over all frequencies and where it is.
    fn wide_peak(&mut self, s: &Ss, who: &str) -> Res {
        if !self.wide_stable(s, who)? {
            return Ok(Expr::list(vec![Expr::inf(), Expr::undefined()]));
        }
        let prec = crate::mfun::bits();
        let numbers = Self::wide_numbers(s, who)?;
        let gain = |w: &Float| -> Result<Float, String> {
            Ok(Self::wide_gains(s, &numbers, w)?.swap_remove(0))
        };
        let mut grid = self.wide_frequencies(s, who)?;
        // more places between them, and the frequencies of the poles
        let mut fine = Vec::new();
        for pair in grid.windows(2) {
            for k in 1..10u32 {
                let ratio = Float::with_val(prec, &pair[1] / &pair[0]);
                fine.push(Float::with_val(
                    prec,
                    &pair[0] * ratio.pow(Float::with_val(prec, k) / 10u32),
                ));
            }
        }
        grid.extend(fine);
        for p in self.wide_poles(s)? {
            if let Ok(z) = numeric::evalf(&p, prec) {
                let w = match &s.ts {
                    None => Float::with_val(prec, z.imag().abs_ref()),
                    Some(_) => {
                        let angle = Float::with_val(prec, z.clone().arg().real()).abs();
                        let step = Self::wide_place(s, &Float::with_val(prec, 1));
                        // (the angle of e^(j Ts) is Ts)
                        let ts = Float::with_val(prec, step.arg().real());
                        match ts.is_zero() {
                            true => angle,
                            false => angle / ts,
                        }
                    }
                };
                if !w.is_zero() {
                    grid.push(w);
                }
            }
        }
        grid.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        grid.dedup();
        let values: Vec<Float> = grid.iter().map(&gain).collect::<Result<_, _>>()?;
        // the ends: no frequency and the highest one
        let still = gain(&Float::with_val(prec, 0))?;
        let mut best = (still, Float::with_val(prec, 0));
        if s.ts.is_none() {
            let (_, mut direct, _) = decomp::svd(&numbers[3])?;
            direct.sort_by(|x, y| y.partial_cmp(x).unwrap_or(std::cmp::Ordering::Equal));
            if direct[0] > best.0 {
                best = (
                    direct.swap_remove(0),
                    Float::with_val(prec, rug::float::Special::Infinity),
                );
            }
        }
        let ratio = (Float::with_val(prec, 5).sqrt() - 1u32) / 2u32;
        for k in 0..grid.len() {
            let peak = (k == 0 || values[k] >= values[k - 1])
                && (k + 1 == grid.len() || values[k] >= values[k + 1]);
            if !peak {
                continue;
            }
            let (mut from, mut to) = (
                grid[k.saturating_sub(1)].clone(),
                grid[(k + 1).min(grid.len() - 1)].clone(),
            );
            let mut found = (values[k].clone(), grid[k].clone());
            let inner = |from: &Float, to: &Float, low: bool| -> Float {
                let wide = Float::with_val(prec, to - from);
                match low {
                    true => Float::with_val(prec, to - Float::with_val(prec, &ratio * &wide)),
                    false => Float::with_val(prec, from + Float::with_val(prec, &ratio * &wide)),
                }
            };
            let (mut x1, mut x2) = (inner(&from, &to, true), inner(&from, &to, false));
            let (mut f1, mut f2) = (gain(&x1)?, gain(&x2)?);
            let tiny = Float::with_val(prec, 1) >> (prec / 2 + 8);
            while Float::with_val(prec, &to - &from) > Float::with_val(prec, &tiny * &to) {
                if f1 > f2 {
                    to = x2;
                    x2 = x1;
                    f2 = f1;
                    x1 = inner(&from, &to, true);
                    f1 = gain(&x1)?;
                } else {
                    from = x1;
                    x1 = x2;
                    f1 = f2;
                    x2 = inner(&from, &to, false);
                    f2 = gain(&x2)?;
                }
            }
            let middle = Float::with_val(prec, &from + &to) / 2u32;
            let value = gain(&middle)?;
            if value > found.0 {
                found = (value, middle);
            }
            if found.0 > best.0 {
                best = found;
            }
        }
        let place = match best.1.is_infinite() {
            true => Expr::inf(),
            false => crate::mfun::value(&best.1),
        };
        Ok(Expr::list(vec![crate::mfun::value(&best.0), place]))
    }

    /// A value that takes part in a sum or a product of systems.
    fn wide_part(&mut self, e: &Expr, who: &str) -> Result<Ss, String> {
        if state_space(e).is_some() || is_system(e) {
            return self.wide_of(e, who);
        }
        if let Some(rows) = matrix::rows(e) {
            return match holds_systems(e) {
                true => self.of_grid(&rows, who),
                false => Ok(Ss::gain(Mat::of(&rows, rows[0].len()), None)),
            };
        }
        if e.as_list().is_some() || matches!(e, Expr::Rel(..)) {
            return Err(format!(
                "{who}: {e} is no system, no matrix, no number and no ratio of polynomials in s or z"
            ));
        }
        let sys = self.sys_of_expr(e, None, who).map_err(|_| {
            format!("{who}: {e} is no system, no matrix, no number and no ratio of polynomials in s or z")
        })?;
        let m = tf_to_ss(&sys.num, &sys.den).map_err(|e| format!("{who}: {e}"))?;
        Ss::read(m, sys.ts, who)
    }

    /// A sum or a product in which a system of several inputs or outputs takes part.
    pub(crate) fn wide_joined(&mut self, sum: bool, items: &[Expr]) -> Res {
        let who = if sum {
            "a sum with a system"
        } else {
            "a product with a system"
        };
        self.without_names(|session| {
            let mut all: Option<Ss> = None;
            for item in items {
                let next = session.wide_part(item, who)?;
                all = Some(match all {
                    None => next,
                    Some(so_far) => {
                        let (x, y) = Session::fitted(so_far, next, sum, who)?;
                        match sum {
                            true => x.sum(&y),
                            false => x.after(&y),
                        }
                    }
                });
            }
            Ok(all.map_or_else(Expr::zero, |s| s.object()))
        })
    }

    /// Two systems that are to be added or multiplied: one with one input and one output
    /// goes to every entry (of a sum) or along the diagonal (of a product).
    fn fitted(x: Ss, y: Ss, sum: bool, who: &str) -> Result<(Ss, Ss), String> {
        let timed = |s: &Ss| s.n() > 0 || s.ts.is_some();
        if timed(&x) && timed(&y) && !same_time(&x.ts, &y.ts) {
            return Err(format!(
                "{who}: a sampled system and one of running time, or two that are sampled at different times, do not go together"
            ));
        }
        let single = |s: &Ss| s.inputs() == 1 && s.outputs() == 1;
        let diagonal = |s: &Ss, size: usize| -> Ss {
            let mut all = s.clone();
            for _ in 1..size {
                all = all.beside(s);
            }
            all
        };
        let fits = match sum {
            true => (x.inputs(), x.outputs()) == (y.inputs(), y.outputs()),
            false => x.inputs() == y.outputs(),
        };
        if fits {
            return Ok((x, y));
        }
        match (single(&x), single(&y), sum) {
            (true, _, true) => Ok((x.spread(y.outputs(), y.inputs()), y)),
            (_, true, true) => {
                let wide = y.spread(x.outputs(), x.inputs());
                Ok((x, wide))
            }
            (true, _, false) => Ok((diagonal(&x, y.outputs()), y)),
            (_, true, false) => {
                let wide = diagonal(&y, x.inputs());
                Ok((x, wide))
            }
            _ => Err(format!(
                "{who}: the sizes do not fit: {} outputs by {} inputs and {} by {}",
                x.outputs(),
                x.inputs(),
                y.outputs(),
                y.inputs()
            )),
        }
    }

    /// A whole power of a system of several inputs and outputs.
    pub(crate) fn wide_power(&mut self, base: &Expr, exponent: &Expr) -> Res {
        let who = "a power of a system";
        let Some(k) = exponent.as_i64().filter(|k| k.abs() <= 64) else {
            return Err(format!(
                "{who} is a whole one, as G^2 or G^-1: not {exponent}"
            ));
        };
        self.without_names(|session| {
            let s = session.wide_of(base, who)?;
            if s.inputs() != s.outputs() {
                return Err(format!("{who} needs as many outputs as inputs"));
            }
            let one = match k < 0 {
                true => s.inverse()?,
                false => s.clone(),
            };
            let mut all = Ss::gain(Mat::eye(s.inputs()), s.ts.clone());
            for _ in 0..k.abs() {
                all = all.after(&one);
            }
            Ok(all.object())
        })
    }

    pub(crate) fn mimo(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        self.without_names(|session| session.mimo_of(name, a))
    }

    fn mimo_of(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        let n = a.len();
        let wide = a.iter().any(is_wide);
        // (a matrix of numbers beside a system is a matrix of gains)
        let pair = matches!(name, "feedback" | "series" | "parallel" | "lft")
            && n >= 2
            && (a[..2].iter().any(holds_systems)
                || (a[..2].iter().any(is_system)
                    && a[..2].iter().any(|e| matrix::rows(e).is_some())));
        let grid = a.first().is_some_and(holds_systems) || pair;
        let system = |e: &Expr| is_system(e) || holds_systems(e);
        let own = match name {
            // (a list with something at its end is not meant)
            "append" => {
                a.iter().any(system) && a.iter().all(|e| system(e) || e.as_list().is_none())
            }
            "connect" | "kalman" | "lqg" | "lqgreg" | "estim" | "lqry" | "augstate" | "lft"
            | "subsystem" | "iosize" => true,
            _ => false,
        };
        let sized =
            matches!(name, "size" | "order" | "ctrb" | "obsv") && n == 1 && is_system(&a[0]);
        if !(wide || grid || own || sized) {
            return None;
        }
        // the names of the operations
        if wide {
            let joined = match (name, n) {
                ("plus", 2..) => Some(self.wide_joined(true, a)),
                ("times" | "mtimes", 2..) => Some(self.wide_joined(false, a)),
                ("minus", 2) => Some(
                    self.wide_joined(false, &[Expr::int(-1), a[1].clone()])
                        .and_then(|less| self.wide_joined(true, &[a[0].clone(), less])),
                ),
                ("uminus", 1) => Some(self.wide_joined(false, &[Expr::int(-1), a[0].clone()])),
                ("uplus", 1) => Some(Ok(a[0].clone())),
                ("power" | "mpower", 2) => Some(self.wide_power(&a[0], &a[1])),
                ("mrdivide" | "rdivide", 2) => Some(
                    self.wide_power(&a[1], &Expr::int(-1))
                        .and_then(|under| self.wide_joined(false, &[a[0].clone(), under])),
                ),
                ("mldivide" | "ldivide", 2) => Some(
                    self.wide_power(&a[0], &Expr::int(-1))
                        .and_then(|under| self.wide_joined(false, &[under, a[1].clone()])),
                ),
                _ => None,
            };
            if joined.is_some() {
                return joined;
            }
        }
        // (the functions that go on entry by entry would call themselves for ever)
        if !matches!(name, "feedback" | "series" | "parallel" | "lft" | "append")
            && a.iter().skip(1).any(|e| is_system(e) || holds_systems(e))
        {
            return Some(Err(format!("{name}: only the first argument is a system")));
        }
        let who = name;
        let form = match a.first() {
            Some(e) if state_space(e).is_some() => Form::States,
            _ => Form::Grid,
        };
        let entries = |session: &mut Session, s: &Ss, rest: &[Expr], f: &str| -> Res {
            let grid = session.grid_of(s, f)?;
            let mut rows = Vec::new();
            for row in &grid {
                let mut out = Vec::new();
                for sys in row {
                    let mut args = vec![sys.object()];
                    args.extend(rest.iter().cloned());
                    out.push(session.call(f, &args)?);
                }
                rows.push(Expr::list(out));
            }
            Ok(Expr::list(rows))
        };
        Some((|| -> Res {
            Ok(match (name, n) {
                ("size" | "iosize", 1) => {
                    let s = self.wide_of(&a[0], who)?;
                    Expr::list(vec![Expr::int(s.outputs() as i64), Expr::int(s.inputs() as i64)])
                }
                ("order", 1) => {
                    let s = self.wide_of(&a[0], who)?;
                    let s = if form == Form::Grid { s.minimal()? } else { s };
                    Expr::int(s.n() as i64)
                }
                ("tf", 1) => {
                    let s = self.wide_of(&a[0], who)?;
                    self.wide_shown(&s, Form::Grid, who)?
                }
                ("zpk", 1) => {
                    let s = self.wide_of(&a[0], who)?;
                    entries(self, &s, &[], "zpk")?
                }
                ("ss", 1) => self.wide_of(&a[0], who)?.minimal()?.object(),
                ("ssdata", 1) => {
                    let s = self.wide_of(&a[0], who)?.minimal()?;
                    let Some(args) = s.object().fn_args("ss").map(<[Expr]>::to_vec) else {
                        return Err("ssdata(system)".into());
                    };
                    Expr::list(args[..4].to_vec())
                }
                ("minreal", 1) => {
                    let s = self.wide_of(&a[0], who)?.minimal()?;
                    self.wide_shown(&s, form, who)?
                }
                ("pole", 1) => {
                    let s = self.wide_of(&a[0], who)?.minimal()?;
                    Expr::list(self.wide_poles(&s)?)
                }
                ("zero" | "tzero", 1) => {
                    let s = self.wide_of(&a[0], who)?;
                    self.wide_zeros(&s, who)?
                }
                ("isstable", 1) => {
                    let s = self.wide_of(&a[0], who)?.minimal()?;
                    Expr::Bool(self.wide_stable(&s, who)?)
                }
                ("isproper", 1) => {
                    self.wide_of(&a[0], who)?;
                    Expr::Bool(true)
                }
                ("isct" | "isdt", 1) => {
                    let s = self.wide_of(&a[0], who)?;
                    Expr::Bool(s.ts.is_some() == (name == "isdt"))
                }
                ("damp", 1) => {
                    let s = self.wide_of(&a[0], who)?.minimal()?;
                    let (den, _) = match s.n() {
                        0 => (vec![Expr::one()], Vec::new()),
                        _ => resolvent(&s.a.rows())?,
                    };
                    let mut args = vec![Expr::list(vec![Expr::one()]), Expr::list(den)];
                    args.extend(s.ts.clone());
                    self.call("damp", &[Expr::call("tf", args)])?
                }
                ("dcgain", 1) => {
                    let s = self.wide_of(&a[0], who)?;
                    entries(self, &s, &[], "dcgain")?
                }
                ("evalfr", 2) => {
                    let s = self.wide_of(&a[0], who)?;
                    entries(self, &s, &a[1..], "evalfr")?
                }
                ("freqresp", 2) => {
                    let s = self.wide_of(&a[0], who)?;
                    let at: Vec<Expr> = list_of(&a[1]);
                    let mut all = Vec::new();
                    for w in &at {
                        all.push(entries(self, &s, std::slice::from_ref(w), "freqresp")?);
                    }
                    match a[1].as_list() {
                        Some(_) => Expr::list(all),
                        None => all.swap_remove(0),
                    }
                }
                ("step" | "impulse", 1 | 2) | ("stepinfo" | "tfdata" | "zpkdata" | "tf2sym" | "bandwidth", 1) | ("bode" | "nyquist" | "nichols", 1 | 2) => {
                    let s = self.wide_of(&a[0], who)?;
                    entries(self, &s, &a[1..], name)?
                }
                ("lsim", 2 | 3) => {
                    let s = self.wide_of(&a[0], who)?;
                    let (m, p) = (s.inputs(), s.outputs());
                    let inputs: Vec<Expr> = match (a[1].as_list(), m, &s.ts) {
                        (Some(items), _, None) if items.len() == m => items.to_vec(),
                        (Some(items), _, Some(_)) if items.len() == m && items.iter().all(|x| x.as_list().is_some()) => items.to_vec(),
                        (_, 1, _) => vec![a[1].clone()],
                        _ => {
                            return Err(format!(
                                "lsim(system, [u1, ..., u{m}][, times]): an input for each of the {m} inputs of the system (a list of samples each for a sampled system)"
                            ));
                        }
                    };
                    let grid = self.grid_of(&s, who)?;
                    let mut outputs = Vec::with_capacity(p);
                    for row in &grid {
                        let mut parts = Vec::with_capacity(m);
                        for (sys, input) in row.iter().zip(&inputs) {
                            let mut args = vec![sys.object(), input.clone()];
                            args.extend(a.get(2).cloned());
                            parts.push(self.call("lsim", &args)?);
                        }
                        outputs.push(tidy(matrix::add(parts)?));
                    }
                    Expr::list(outputs)
                }
                ("initial", 2 | 3) => {
                    let s = self.wide_of(&a[0], who)?;
                    if s.n() == 0 {
                        return Err("initial: the system has no states, it is a gain alone".into());
                    }
                    let mut outputs = Vec::new();
                    for i in 0..s.outputs() {
                        let one = s.picked(&[i], &[0]);
                        let mut args = vec![one.object()];
                        args.extend(a[1..].iter().cloned());
                        outputs.push(self.call("initial", &args)?);
                    }
                    Expr::list(outputs)
                }
                ("sigma", 1 | 2) => {
                    let s = self.wide_of(&a[0], who)?;
                    let prec = crate::mfun::bits();
                    let numbers = Self::wide_numbers(&s, who)?;
                    let ws: Vec<Float> = match a.get(1) {
                        Some(w) => list_of(w)
                            .iter()
                            .map(|x| crate::mfun::real(x, prec, who))
                            .collect::<Result<_, _>>()?,
                        None => self.wide_frequencies(&s, who)?,
                    };
                    let mut all = Vec::new();
                    for w in &ws {
                        let values = Self::wide_gains(&s, &numbers, w)?;
                        all.push(Expr::list(values.iter().map(crate::mfun::value).collect()));
                    }
                    match a.get(1) {
                        Some(w) if w.as_list().is_none() => all.swap_remove(0),
                        Some(_) => Expr::list(all),
                        None => Expr::list(vec![
                            Expr::list(all),
                            Expr::list(ws.iter().map(crate::mfun::value).collect()),
                        ]),
                    }
                }
                ("hinfnorm", 1) | ("norm", 2) if name == "hinfnorm" || a.get(1).is_some_and(|k| k.is_inf() || word(k).as_deref() == Some("inf")) => {
                    let s = self.wide_of(&a[0], who)?.minimal()?;
                    self.wide_peak(&s, who)?
                }
                ("h2norm", 1) | ("norm", 1 | 2) => {
                    if n == 2 && a[1] != Expr::int(2) {
                        return Err("norm(system, 2) or norm(system, inf)".into());
                    }
                    let s = self.wide_of(&a[0], who)?.minimal()?;
                    if !self.wide_stable(&s, who)? || (s.ts.is_none() && s.d.v.iter().any(|x| !x.is_zero())) {
                        return Ok(Expr::inf());
                    }
                    let mut square = match s.n() {
                        0 => Expr::zero(),
                        _ => {
                            let solver = if s.ts.is_some() { "dlyap" } else { "lyap" };
                            let reach = self.call(solver, &[s.a.expr(), s.b.mul(&s.b.t()).expr()])?;
                            let reach = Mat::read(&reach, false);
                            let seen = s.c.mul(&reach).mul(&s.c.t());
                            tidy(add((0..seen.r).map(|i| seen.at(i, i).clone()).collect()))
                        }
                    };
                    if s.ts.is_some() {
                        let direct = s.d.mul(&s.d.t());
                        square = tidy(add((0..direct.r)
                            .map(|i| direct.at(i, i).clone())
                            .chain(std::iter::once(square))
                            .collect()));
                    }
                    self.call("sqrt", &[square])?
                }
                ("covar", 2) => {
                    let s = self.wide_of(&a[0], who)?;
                    let noise = Mat::read(&a[1], false);
                    if noise.r != s.inputs() || noise.c != s.inputs() {
                        return Err(format!("covar(system, W): W has a row and a column for each of the {} inputs", s.inputs()));
                    }
                    if !self.wide_stable(&s, who)? {
                        return Err("covar: the system is not stable".into());
                    }
                    let direct = s.d.mul(&noise).mul(&s.d.t());
                    let states = match s.n() {
                        0 => Mat::zeros(0, 0),
                        _ => {
                            let solver = if s.ts.is_some() { "dlyap" } else { "lyap" };
                            Mat::read(&self.call(solver, &[s.a.expr(), s.b.mul(&noise).mul(&s.b.t()).expr()])?, false)
                        }
                    };
                    let shown = match s.n() {
                        0 => Expr::list(Vec::new()),
                        _ => states.expr(),
                    };
                    if s.ts.is_none() && direct.v.iter().any(|x| !x.is_zero()) {
                        // (noise that goes straight through has no bound in running time)
                        let all = Mat::zeros(s.outputs(), s.outputs()).map(|_| Expr::inf());
                        return Ok(Expr::list(vec![all.expr(), shown]));
                    }
                    let mut all = s.c.mul(&states).mul(&s.c.t());
                    if s.ts.is_some() {
                        all = all.plus(&direct);
                    }
                    Expr::list(vec![all.expr(), shown])
                }
                ("gram", 2) | ("balreal" | "hsvd", 1) | ("balred", 2 | 3) | ("ss2ss", 2) if form == Form::Grid => {
                    let s = self.wide_of(&a[0], who)?.minimal()?;
                    let mut args = vec![s.object()];
                    args.extend(a[1..].iter().cloned());
                    self.call(name, &args)?
                }
                ("ctrb" | "obsv", 1) => {
                    let s = self.wide_of(&a[0], who)?;
                    match name {
                        "ctrb" => self.call("ctrb", &[s.a.expr(), s.b.expr()])?,
                        _ => self.call("obsv", &[s.a.expr(), s.c.expr()])?,
                    }
                }
                ("series" | "parallel", 2) | ("feedback", 2 | 3) => {
                    let (x, y) = (self.wide_part(&a[0], who)?, self.wide_part(&a[1], who)?);
                    let form = match state_space(&a[0]).is_some() || state_space(&a[1]).is_some() {
                        true => Form::States,
                        false => Form::Grid,
                    };
                    let joined = match name {
                        "series" => {
                            let (second, first) = Self::fitted(y, x, false, who)?;
                            second.after(&first)
                        }
                        "parallel" => {
                            let (x, y) = Self::fitted(x, y, true, who)?;
                            x.sum(&y)
                        }
                        _ => {
                            let sign = match a.get(2) {
                                Some(s) if *s == Expr::one() => 1,
                                Some(s) if *s == Expr::int(-1) => -1,
                                Some(_) => return Err("feedback(G, K[, sign]): the sign is -1 (the usual, negative feedback) or +1".into()),
                                None => -1,
                            };
                            let (m, p) = (x.inputs(), x.outputs());
                            // (a gain of one entry stands for every loop)
                            let y = match y.inputs() == 1 && y.outputs() == 1 && (m, p) != (1, 1) && m == p {
                                true => {
                                    let mut all = y.clone();
                                    for _ in 1..m {
                                        all = all.beside(&y);
                                    }
                                    all
                                }
                                false => y,
                            };
                            if y.inputs() != p || y.outputs() != m {
                                return Err(format!("feedback(G, K): K takes the {p} outputs of G and gives its {m} inputs"));
                            }
                            let both = x.beside(&y);
                            // u_G = r + sign*y_K, u_K = y_G
                            let mut k = Mat::zeros(m + p, p + m);
                            for i in 0..m {
                                k.v[i * (p + m) + p + i] = Expr::int(sign);
                            }
                            for i in 0..p {
                                k.v[(m + i) * (p + m) + i] = Expr::one();
                            }
                            both.closed(&k)?.picked(&span(0, p), &span(0, m))
                        }
                    };
                    self.wide_shown(&joined, form, who)?
                }
                ("inv", 1) => {
                    let s = self.wide_of(&a[0], who)?.inverse()?;
                    self.wide_shown(&s, form, who)?
                }
                ("append", 1..) => {
                    let mut all: Option<Ss> = None;
                    for e in a {
                        let next = self.wide_part(e, who)?;
                        all = Some(match all {
                            None => next,
                            Some(so_far) => {
                                if (so_far.n() > 0 || so_far.ts.is_some()) && (next.n() > 0 || next.ts.is_some()) && !same_time(&so_far.ts, &next.ts) {
                                    return Err("append: a sampled system and one of running time, or two that are sampled at different times, do not go together".into());
                                }
                                so_far.beside(&next)
                            }
                        });
                    }
                    let all = all.ok_or("append(system, ...)")?;
                    let form = match a.iter().any(|e| state_space(e).is_some()) {
                        true => Form::States,
                        false => Form::Grid,
                    };
                    self.wide_shown(&all, form, who)?
                }
                ("connect", 4) => {
                    let s = self.wide_part(&a[0], who)?;
                    let (m, p) = (s.inputs(), s.outputs());
                    let mut k = Mat::zeros(m, p);
                    let rows = match matrix::rows(&a[1]) {
                        Some(rows) => rows,
                        None => vec![list_of(&a[1])],
                    };
                    let usage = || format!("connect(system, Q, inputs, outputs): a row [input, outputs that go into it (with a minus sign: taken off)] of Q for every input that is fed; the system has {m} inputs and {p} outputs");
                    for row in rows.iter().filter(|r| !r.is_empty()) {
                        let to = row[0].as_i64().filter(|k| (1..=m as i64).contains(k)).ok_or_else(usage)? as usize - 1;
                        for from in &row[1..] {
                            let from = from.as_i64().filter(|k| k.unsigned_abs() as usize <= p).ok_or_else(usage)?;
                            if from != 0 {
                                let at = to * p + from.unsigned_abs() as usize - 1;
                                k.v[at] = tidy(add(vec![k.v[at].clone(), Expr::int(from.signum())]));
                            }
                        }
                    }
                    let (inputs, outputs) = (places(&a[2], m, "connect: the inputs")?, places(&a[3], p, "connect: the outputs")?);
                    let joined = s.closed(&k)?.picked(&outputs, &inputs);
                    let form = match state_space(&a[0]).is_some() {
                        true => Form::States,
                        false => Form::Grid,
                    };
                    self.wide_shown(&joined, form, who)?
                }
                ("lft", 2 | 4) => {
                    let (x, y) = (self.wide_part(&a[0], who)?, self.wide_part(&a[1], who)?);
                    let (nu, ny) = match n {
                        4 => (
                            crate::arrays::whole(&a[2], "lft: the number nu of inputs", 0, x.inputs() as i64)? as usize,
                            crate::arrays::whole(&a[3], "lft: the number ny of outputs", 0, x.outputs() as i64)? as usize,
                        ),
                        _ => (y.outputs(), y.inputs()),
                    };
                    if nu > x.inputs() || ny > x.outputs() || y.outputs() != nu || y.inputs() != ny {
                        return Err("lft(P, K[, nu, ny]): K takes the last ny outputs of P and gives its last nu inputs".into());
                    }
                    let (m1, p1) = (x.inputs() - nu, x.outputs() - ny);
                    if m1 == 0 || p1 == 0 {
                        return Err("lft(P, K): P has inputs and outputs that K does not take".into());
                    }
                    let both = x.beside(&y);
                    let (m, p) = (both.inputs(), both.outputs());
                    let mut k = Mat::zeros(m, p);
                    // the last nu inputs of P are the outputs of K, the inputs of K the last ny outputs of P
                    for i in 0..nu {
                        k.v[(m1 + i) * p + p1 + ny + i] = Expr::one();
                    }
                    for i in 0..ny {
                        k.v[(m1 + nu + i) * p + p1 + i] = Expr::one();
                    }
                    let joined = both.closed(&k)?.picked(&span(0, p1), &span(0, m1));
                    let form = match state_space(&a[0]).is_some() || state_space(&a[1]).is_some() {
                        true => Form::States,
                        false => Form::Grid,
                    };
                    self.wide_shown(&joined, form, who)?
                }
                ("subsystem", 3) => {
                    let s = self.wide_of(&a[0], who)?;
                    let outputs = places(&a[1], s.outputs(), "subsystem: the outputs")?;
                    let inputs = places(&a[2], s.inputs(), "subsystem: the inputs")?;
                    if outputs.is_empty() || inputs.is_empty() {
                        return Err("subsystem(system, [outputs], [inputs])".into());
                    }
                    self.wide_shown(&s.picked(&outputs, &inputs), form, who)?
                }
                ("augstate", 1) => {
                    let s = self.wide_of(&a[0], who)?;
                    let n = s.n();
                    Ss {
                        c: s.c.above(&Mat::eye(n)),
                        d: s.d.above(&Mat::zeros(n, s.inputs())),
                        ..s
                    }
                    .object()
                }
                ("c2d", 2 | 3) => {
                    let s = self.wide_of(&a[0], who)?;
                    if s.ts.is_some() {
                        return Err("c2d: the system is sampled already".into());
                    }
                    let step = a[1].clone();
                    if numeric::real_value(&step, numeric::digits()).is_none_or(|t| t <= 0) {
                        return Err("c2d(system, Ts): the time Ts between samples is positive".into());
                    }
                    let (n, m) = (s.n(), s.inputs());
                    let method = a.get(2).and_then(word).unwrap_or_else(|| "zoh".into());
                    let sampled = match method.as_str() {
                        "zoh" => {
                            let whole = Mat::blocks(&s.a, &s.b, &Mat::zeros(m, n), &Mat::zeros(m, m)).scaled(&step);
                            // in closed form for one or two states of fractions (and for names); else by decimal numbers
                            let joined = whole.expr();
                            let entries = matrix::rows(&joined).unwrap_or_default();
                            let closed = entries.iter().flatten().any(|x| !x.is_const())
                                || (n <= 2 && entries.iter().flatten().all(|x| x.as_rat().is_some()));
                            let exact = match closed {
                                true => self
                                    .call("matrixexp", std::slice::from_ref(&joined))
                                    .ok()
                                    .filter(|g| matrix::rows(g).is_some() && !crate::control::has_i(g)),
                                false => None,
                            };
                            let grown = Mat::read(
                                &match exact {
                                    Some(g) => g,
                                    None => crate::decomp::from_complex(&crate::matfun::exp(&crate::decomp::to_complex(&entries)?)),
                                },
                                false,
                            );
                            Ss {
                                a: grown.pick(&span(0, n), &span(0, n)),
                                b: grown.pick(&span(0, n), &span(n, n + m)),
                                c: s.c.clone(),
                                d: s.d.clone(),
                                ts: Some(step),
                            }
                        }
                        "tustin" | "bilinear" => {
                            let half = tidy(div(step.clone(), Expr::int(2)));
                            let back = Mat::eye(n).minus(&s.a.scaled(&half)).inv("I - A*Ts/2")?;
                            let b = back.mul(&s.b).scaled(&step);
                            Ss {
                                a: back.mul(&Mat::eye(n).plus(&s.a.scaled(&half))),
                                d: s.d.plus(&s.c.mul(&b).scaled(&Expr::half())),
                                c: s.c.mul(&back),
                                b,
                                ts: Some(step),
                            }
                        }
                        _ => return Err("c2d of a system with several inputs or outputs: \"zoh\" (the usual) or \"tustin\"".into()),
                    };
                    self.wide_shown(&sampled, form, who)?
                }
                ("d2c", 1 | 2) => {
                    let s = self.wide_of(&a[0], who)?;
                    let Some(step) = s.ts.clone().filter(|t| numeric::real_value(t, numeric::digits()).is_some_and(|t| t > 0)) else {
                        return Err("d2c: a sampled system with the time Ts between its samples".into());
                    };
                    let (n, m) = (s.n(), s.inputs());
                    let method = a.get(1).and_then(word).unwrap_or_else(|| "zoh".into());
                    let running = match method.as_str() {
                        "zoh" => {
                            // (the logarithm of a matrix with the eigenvalue 1 several times: with numbers)
                            let whole = Mat::blocks(&s.a, &s.b, &Mat::zeros(m, n), &Mat::eye(m))
                                .map(|x| numeric::n(x, numeric::digits()));
                            let grown = Mat::read(&self.call("matrixlog", &[whole.expr()])?, false).scaled(&tidy(div(Expr::one(), step)));
                            Ss {
                                a: grown.pick(&span(0, n), &span(0, n)),
                                b: grown.pick(&span(0, n), &span(n, n + m)),
                                c: s.c.clone(),
                                d: s.d.clone(),
                                ts: None,
                            }
                        }
                        "tustin" | "bilinear" => {
                            let half = tidy(div(step.clone(), Expr::int(2)));
                            let own = s.a.plus(&Mat::eye(n)).inv("A + I")?.mul(&s.a.minus(&Mat::eye(n))).scaled(&tidy(div(Expr::int(2), step.clone())));
                            let less = Mat::eye(n).minus(&own.scaled(&half));
                            let b = less.mul(&s.b).scaled(&tidy(div(Expr::one(), step)));
                            Ss {
                                d: s.d.minus(&s.c.mul(&b).scaled(&half)),
                                c: s.c.mul(&less),
                                a: own,
                                b,
                                ts: None,
                            }
                        }
                        _ => return Err("d2c of a system with several inputs or outputs: \"zoh\" (the usual) or \"tustin\"".into()),
                    };
                    self.wide_shown(&running, form, who)?
                }
                ("lqry", 3 | 4) => {
                    let s = self.wide_of(&a[0], who)?;
                    let (q, r) = (Mat::read(&a[1], false), Mat::read(&a[2], false));
                    let (m, p) = (s.inputs(), s.outputs());
                    let cross = match a.get(3) {
                        Some(e) => Mat::read(e, true),
                        None => Mat::zeros(p, m),
                    };
                    if (q.r, q.c) != (p, p) || (r.r, r.c) != (m, m) || (cross.r, cross.c) != (p, m) || s.n() == 0 {
                        return Err(format!("lqry(system, Q, R[, N]): Q weighs the {p} outputs, R the {m} inputs, N both (outputs by inputs); the system has states"));
                    }
                    let own = s.c.t().mul(&q).mul(&s.c);
                    let both = s.c.t().mul(&q.mul(&s.d).plus(&cross));
                    let input = r
                        .plus(&s.d.t().mul(&q).mul(&s.d))
                        .plus(&s.d.t().mul(&cross))
                        .plus(&cross.t().mul(&s.d));
                    let solver = if s.ts.is_some() { "dlqr" } else { "lqr" };
                    self.call(solver, &[s.a.expr(), s.b.expr(), own.expr(), input.expr(), both.expr()])?
                }
                ("kalman", 3 | 4) => {
                    let s = self.wide_of(&a[0], who)?;
                    let (n, m, p) = (s.n(), s.inputs(), s.outputs());
                    let (qn, rn) = (Mat::read(&a[1], false), Mat::read(&a[2], false));
                    let noisy = qn.r;
                    let usage = || format!("kalman(system, Qn, Rn[, Nn]): the last inputs of the system are the noise w with the covariance Qn (at most {m} of them), Rn is the covariance of the noise of the {p} outputs, Nn the one between both (rows of w); the system has states");
                    if n == 0 || noisy == 0 || noisy > m || qn.c != noisy || (rn.r, rn.c) != (p, p) {
                        return Err(usage());
                    }
                    let cross = match a.get(3) {
                        Some(e) => Mat::read(e, true),
                        None => Mat::zeros(noisy, p),
                    };
                    if (cross.r, cross.c) != (noisy, p) {
                        return Err(usage());
                    }
                    let known = m - noisy;
                    let (states, outputs) = (span(0, n), span(0, p));
                    let (bu, g) = (s.b.pick(&states, &span(0, known)), s.b.pick(&states, &span(known, m)));
                    let (du, h) = (s.d.pick(&outputs, &span(0, known)), s.d.pick(&outputs, &span(known, m)));
                    // the noise that the outputs see, and how it goes with that of the states
                    let seen = rn
                        .plus(&h.mul(&cross))
                        .plus(&cross.t().mul(&h.t()))
                        .plus(&h.mul(&qn).mul(&h.t()));
                    let both = g.mul(&qn.mul(&h.t()).plus(&cross));
                    let own = g.mul(&qn).mul(&g.t());
                    let sampled = s.ts.is_some();
                    let solver = if sampled { "dlqr" } else { "lqr" };
                    let found = self.call(solver, &[s.a.t().expr(), s.c.t().expr(), own.expr(), seen.expr(), both.expr()])?;
                    let parts = list_of(&found);
                    let (l, cov) = (Mat::read(&parts[0], false).t(), Mat::read(&parts[1], false));
                    let watcher = match sampled {
                        false => Ss {
                            a: s.a.minus(&l.mul(&s.c)),
                            b: bu.minus(&l.mul(&du)).beside(&l),
                            c: s.c.above(&Mat::eye(n)),
                            d: Mat::blocks(&du, &Mat::zeros(p, p), &Mat::zeros(n, known), &Mat::zeros(n, p)),
                            ts: None,
                        },
                        true => {
                            // the estimate with the newest measurement in it
                            let inner = s.c.mul(&cov).mul(&s.c.t()).plus(&seen).inv("C*P*C' + R")?;
                            let news = cov.mul(&s.c.t()).mul(&inner);
                            let rest = Mat::eye(n).minus(&news.mul(&s.c));
                            let through = Mat::eye(p).minus(&s.c.mul(&news));
                            let after = rest.mul(&cov);
                            let watcher = Ss {
                                a: s.a.minus(&l.mul(&s.c)),
                                b: bu.minus(&l.mul(&du)).beside(&l),
                                c: s.c.mul(&rest).above(&rest),
                                d: Mat::blocks(&through.mul(&du), &s.c.mul(&news), &news.mul(&du).scaled(&Expr::int(-1)), &news),
                                ts: s.ts.clone(),
                            };
                            return Ok(Expr::list(vec![watcher.object(), l.expr(), cov.expr(), news.expr(), after.expr()]));
                        }
                    };
                    Expr::list(vec![watcher.object(), l.expr(), cov.expr()])
                }
                ("lqg", 3) => {
                    let s = self.wide_of(&a[0], who)?;
                    let (n, m, p) = (s.n(), s.inputs(), s.outputs());
                    let (cost, noise) = (Mat::read(&a[1], false), Mat::read(&a[2], false));
                    if n == 0 || (cost.r, cost.c) != (n + m, n + m) || (noise.r, noise.c) != (n + p, n + p) {
                        return Err(format!("lqg(system, QXU, QWV): QXU weighs the {n} states and the {m} inputs together, QWV is the covariance of the noise of the {n} states and of the {p} outputs together; the system has states"));
                    }
                    let (x, u, y) = (span(0, n), span(n, n + m), span(n, n + p));
                    let solver = if s.ts.is_some() { "dlqr" } else { "lqr" };
                    let steer = self.call(solver, &[s.a.expr(), s.b.expr(), cost.pick(&x, &x).expr(), cost.pick(&u, &u).expr(), cost.pick(&x, &u).expr()])?;
                    let k = Mat::read(&list_of(&steer)[0], false);
                    let watch = self.call(solver, &[s.a.t().expr(), s.c.t().expr(), noise.pick(&x, &x).expr(), noise.pick(&y, &y).expr(), noise.pick(&x, &y).expr()])?;
                    let l = Mat::read(&list_of(&watch)[0], false).t();
                    Ss {
                        a: s.a.minus(&s.b.mul(&k)).minus(&l.mul(&s.c)).plus(&l.mul(&s.d).mul(&k)),
                        b: l,
                        c: k.scaled(&Expr::int(-1)),
                        d: Mat::zeros(m, p),
                        ts: s.ts.clone(),
                    }
                    .object()
                }
                ("lqgreg", 2) => {
                    let watcher = self.wide_of(&a[0], who)?;
                    let k = Mat::read(&a[1], false);
                    let (controls, states) = (k.r, k.c);
                    if watcher.outputs() <= states || watcher.inputs() <= controls {
                        return Err("lqgreg(kest, K): kest is the watcher that kalman gives (inputs [u; y], outputs [y; x]), K the gain of the states".into());
                    }
                    let (p, seen) = (watcher.outputs() - states, watcher.inputs() - controls);
                    let all = span(0, watcher.n());
                    let (bu, by) = (watcher.b.pick(&all, &span(0, controls)), watcher.b.pick(&all, &span(controls, controls + seen)));
                    let cx = watcher.c.pick(&span(p, p + states), &all);
                    let dxu = watcher.d.pick(&span(p, p + states), &span(0, controls));
                    let dxy = watcher.d.pick(&span(p, p + states), &span(controls, controls + seen));
                    let loop_gain = Mat::eye(controls).plus(&k.mul(&dxu)).inv("I + K*D")?.mul(&k).scaled(&Expr::int(-1));
                    let (c, d) = (loop_gain.mul(&cx), loop_gain.mul(&dxy));
                    Ss {
                        a: watcher.a.plus(&bu.mul(&c)),
                        b: by.plus(&bu.mul(&d)),
                        c,
                        d,
                        ts: watcher.ts.clone(),
                    }
                    .object()
                }
                ("estim", 2 | 4) => {
                    let s = self.wide_of(&a[0], who)?;
                    let (n, m, p) = (s.n(), s.inputs(), s.outputs());
                    let (sensors, known) = match n {
                        _ if a.len() == 4 => (places(&a[2], p, "estim: the sensors")?, places(&a[3], m, "estim: the known inputs")?),
                        _ => (span(0, p), Vec::new()),
                    };
                    let l = Mat::read(&a[1], true);
                    if n == 0 || (l.r, l.c) != (n, sensors.len()) || sensors.is_empty() {
                        return Err(format!("estim(system, L[, sensors, known]): the gain L has a row for each of the {n} states and a column for every output that is measured"));
                    }
                    let states = span(0, n);
                    let (cs, bk) = (s.c.pick(&sensors, &states), s.b.pick(&states, &known));
                    let dk = s.d.pick(&sensors, &known);
                    Ss {
                        a: s.a.minus(&l.mul(&cs)),
                        b: bk.minus(&l.mul(&dk)).beside(&l),
                        c: cs.above(&Mat::eye(n)),
                        d: Mat::blocks(&dk, &Mat::zeros(sensors.len(), sensors.len()), &Mat::zeros(n, known.len()), &Mat::zeros(n, sensors.len())),
                        ts: s.ts.clone(),
                    }
                    .object()
                }
                ("reg", 3) => {
                    let s = self.wide_of(&a[0], who)?;
                    let (n, m, p) = (s.n(), s.inputs(), s.outputs());
                    let (k, l) = (Mat::read(&a[1], false), Mat::read(&a[2], true));
                    if n == 0 || (k.r, k.c) != (m, n) || (l.r, l.c) != (n, p) {
                        return Err(format!("reg(system, K, L): the gain K of the states ({m} by {n}) and the gain L of the watcher ({n} by {p})"));
                    }
                    Ss {
                        a: s.a.minus(&s.b.mul(&k)).minus(&l.mul(&s.c)).plus(&l.mul(&s.d).mul(&k)),
                        b: l,
                        c: k.scaled(&Expr::int(-1)),
                        d: Mat::zeros(m, p),
                        ts: s.ts.clone(),
                    }
                    .object()
                }
                _ => return Err(String::new()),
            })
        })())
        .filter(|r| !matches!(r, Err(e) if e.is_empty()))
    }
}

/// The negative of an expression, tidied (for the tests of this module).
#[cfg(test)]
fn said(input: &str) -> String {
    crate::eval::run_line(input).map_or_else(|e| format!("error: {e}"), |lines| lines.join("|"))
}

#[cfg(test)]
mod tests {
    use super::said;

    #[test]
    fn a_matrix_of_systems_and_its_states() {
        // two inputs, one output: the states of both entries, and back
        assert_eq!(
            said("ss([[1/(s + 1), 2/(s + 2)]])"),
            "ss([[-1, 0], [0, -2]], [[1, 0], [0, 1]], [[1, 2]], [[0, 0]])"
        );
        assert_eq!(
            said("tf(ss([[-1, 0], [0, -2]], [[1, 0], [0, 1]], [[1, 2]], [[0, 0]]))"),
            "[[tf([1], [1, 1]), tf([2], [1, 2])]]"
        );
        assert_eq!(
            said("size(ss([[-1, 0], [0, -2]], [[1, 0], [0, 1]], [[1, 2]], [[0, 0]]))"),
            "[1, 2]"
        );
        // a pole that two entries share is one state when one input feeds both
        assert_eq!(said("order([[1/(s + 1)], [3/(s + 1)]])"), "1");
        assert_eq!(
            said("pole([[1/(s + 1), 1/(s + 2)], [0, 1/(s + 1)]])"),
            "[-2, -1, -1]"
        );
    }

    #[test]
    fn zeros_of_several_inputs_and_outputs() {
        // det = (s - 1)/((s + 1)(s + 2)) up to a factor: the zero no entry shows
        assert_eq!(
            said("tzero([[1/(s + 1), 2/(s + 2)], [1/(s + 1), 1/(s + 1)]])"),
            "[0]"
        );
        assert_eq!(said("tzero([[1/(s + 1), 1/(s + 2)]])"), "[]");
    }

    #[test]
    fn loops_and_joins() {
        assert_eq!(
            said("feedback([[1/s, 0], [0, 1/s]], [[1, 0], [0, 2]])"),
            "[[tf([1], [1, 1]), tf([0], [1])], [tf([0], [1]), tf([1], [1, 2])]]"
        );
        assert_eq!(
            said("tf(append(tf([1], [1, 1]), 2))"),
            "[[tf([1], [1, 1]), tf([0], [1])], [tf([0], [1]), tf([2], [1])]]"
        );
        // the loop of the second block around the first, by the numbers of inputs and outputs
        assert_eq!(
            said("connect(append(tf([1], [1, 0]), 3), [[1, -2], [2, 1]], [1], [1])"),
            "tf([1], [1, 3])"
        );
        assert_eq!(
            said("tf(ss([[-1]], [[1, 2]], [[1], [1]], 0) * ss([[-3]], [[1]], [[1], [0]], 0))"),
            "[[tf([1], [1, 4, 3])], [tf([1], [1, 4, 3])]]"
        );
    }
}
