//! Graphs. A graph is the list of its edges: `[a, b]` joins two vertices, `a -> b` goes one
//! way, a third entry `[a, b, w]` or `[a -> b, w]` is a weight, and `[v]` is a vertex alone.

use crate::canon::{add, mul, neg, pow, sub};
use crate::expr::Expr;
use crate::matrix;
use crate::numeric;
use rug::{Float, Integer, Rational};
use std::collections::VecDeque;
use std::time::Duration;

type Res = Result<Expr, String>;
/// The cost to reach each vertex and the edge by which it is reached.
type Reach = (Vec<Option<Float>>, Vec<Option<usize>>);

const USAGE: &str = "a graph is a list of edges: [[1, 2], [2, 3]], [1 -> 2, 2 -> 3], with weights [[1, 2, 5], [2 -> 3, 4]], a vertex alone as [4]";
const SLOW: &str = "the search took too long for this graph";

#[derive(Clone)]
struct Edge {
    a: usize,
    b: usize,
    weight: Expr,
    arrow: bool,
}

pub struct Graph {
    verts: Vec<Expr>,
    edges: Vec<Edge>,
    directed: bool,
    weighted: bool,
}

fn rule(e: &Expr) -> Option<(&Expr, &Expr)> {
    match e.fn_args("rule") {
        Some([a, b]) => Some((a, b)),
        _ => None,
    }
}

fn truth(b: bool) -> Expr {
    Expr::Bool(b)
}

fn int(n: usize) -> Expr {
    Expr::integer(Integer::from(n))
}

impl Graph {
    fn vertex(&mut self, v: &Expr) -> Result<usize, String> {
        if matches!(v, Expr::List(_) | Expr::Rel(..) | Expr::Bool(_)) || rule(v).is_some() {
            return Err(format!("{v} cannot be a vertex: {USAGE}"));
        }
        Ok(match self.verts.iter().position(|x| x == v) {
            Some(k) => k,
            None => {
                self.verts.push(v.clone());
                self.verts.len() - 1
            }
        })
    }

    fn join(
        &mut self,
        a: &Expr,
        b: &Expr,
        weight: Option<&Expr>,
        arrow: bool,
    ) -> Result<(), String> {
        let (a, b) = (self.vertex(a)?, self.vertex(b)?);
        self.weighted |= weight.is_some();
        self.edges.push(Edge {
            a,
            b,
            weight: weight.cloned().unwrap_or_else(Expr::one),
            arrow,
        });
        Ok(())
    }

    pub fn read(e: &Expr) -> Result<Graph, String> {
        let items = e.as_list().ok_or(USAGE)?;
        let mut g = Graph {
            verts: Vec::new(),
            edges: Vec::new(),
            directed: false,
            weighted: false,
        };
        for item in items {
            if let Some((a, b)) = rule(item) {
                g.join(a, b, None, true)?;
                continue;
            }
            match item.as_list() {
                Some([v]) => {
                    g.vertex(v)?;
                }
                Some([arrow, w]) if rule(arrow).is_some() => {
                    let (a, b) = rule(arrow).expect("checked");
                    g.join(a, b, Some(w), true)?;
                }
                Some([a, b]) => g.join(a, b, None, false)?,
                Some([a, b, w]) => g.join(a, b, Some(w), false)?,
                _ => return Err(USAGE.to_string()),
            }
        }
        g.directed = g.edges.iter().any(|e| e.arrow);
        g.sort_vertices();
        Ok(g)
    }

    /// Numbers by their size and names by the alphabet; other vertices stay as they came.
    fn sort_vertices(&mut self) {
        let n = self.verts.len();
        let mut order: Vec<usize> = (0..n).collect();
        let values: Option<Vec<Float>> = self
            .verts
            .iter()
            .map(|v| numeric::real_value(v, 30))
            .collect();
        if let Some(values) = values {
            order.sort_by(|a, b| {
                values[*a]
                    .partial_cmp(&values[*b])
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        } else if self.verts.iter().all(|v| v.sym_name().is_some()) {
            order.sort_by_key(|k| self.verts[*k].to_string());
        } else {
            return;
        }
        let mut place = vec![0; n];
        for (new, old) in order.iter().enumerate() {
            place[*old] = new;
        }
        self.verts = order.iter().map(|k| self.verts[*k].clone()).collect();
        for e in &mut self.edges {
            (e.a, e.b) = (place[e.a], place[e.b]);
        }
    }

    fn n(&self) -> usize {
        self.verts.len()
    }

    fn index(&self, v: &Expr) -> Result<usize, String> {
        self.verts
            .iter()
            .position(|x| x == v)
            .ok_or_else(|| format!("{v} is not a vertex of the graph"))
    }

    fn list(&self, vertices: &[usize]) -> Expr {
        Expr::list(vertices.iter().map(|k| self.verts[*k].clone()).collect())
    }

    fn edge_expr(&self, e: &Edge) -> Expr {
        let (a, b) = (self.verts[e.a].clone(), self.verts[e.b].clone());
        match (e.arrow, self.weighted) {
            (true, false) => Expr::call("rule", vec![a, b]),
            (true, true) => Expr::list(vec![Expr::call("rule", vec![a, b]), e.weight.clone()]),
            (false, false) => Expr::list(vec![a, b]),
            (false, true) => Expr::list(vec![a, b, e.weight.clone()]),
        }
    }

    /// The edges as a graph again, with the vertices that have none.
    fn graph_expr(&self, edges: &[Edge]) -> Expr {
        let mut items: Vec<Expr> = edges.iter().map(|e| self.edge_expr(e)).collect();
        for (k, v) in self.verts.iter().enumerate() {
            if !edges.iter().any(|e| e.a == k || e.b == k) {
                items.push(Expr::list(vec![v.clone()]));
            }
        }
        Expr::list(items)
    }

    /// For each vertex the vertices that its edges lead to, with the number of the edge.
    fn out(&self) -> Vec<Vec<(usize, usize)>> {
        let mut adj = vec![Vec::new(); self.n()];
        for (k, e) in self.edges.iter().enumerate() {
            adj[e.a].push((e.b, k));
            if !e.arrow && e.a != e.b {
                adj[e.b].push((e.a, k));
            }
        }
        adj
    }

    /// The same without the directions.
    fn both(&self) -> Vec<Vec<(usize, usize)>> {
        let mut adj = vec![Vec::new(); self.n()];
        for (k, e) in self.edges.iter().enumerate() {
            adj[e.a].push((e.b, k));
            if e.a != e.b {
                adj[e.b].push((e.a, k));
            }
        }
        adj
    }

    /// Whether two vertices are joined (in either direction).
    fn joined(&self) -> Vec<Vec<bool>> {
        let n = self.n();
        let mut m = vec![vec![false; n]; n];
        for e in &self.edges {
            if e.a != e.b {
                m[e.a][e.b] = true;
                m[e.b][e.a] = true;
            }
        }
        m
    }

    fn undirected(&self, what: &str) -> Result<(), String> {
        if self.directed {
            return Err(format!("{what} is for graphs without directions"));
        }
        Ok(())
    }

    fn costs(&self) -> Result<Vec<Float>, String> {
        self.edges
            .iter()
            .map(|e| {
                numeric::real_value(&e.weight, 40)
                    .ok_or_else(|| format!("the weight {} is not a real number", e.weight))
            })
            .collect()
    }

    /// The vertices that can be reached from `start`, in the order of a search by levels.
    fn reach(&self, adj: &[Vec<(usize, usize)>], start: usize) -> Vec<usize> {
        let mut seen = vec![false; self.n()];
        let mut order = vec![start];
        seen[start] = true;
        let mut at = 0;
        while at < order.len() {
            for &(w, _) in &adj[order[at]] {
                if !seen[w] {
                    seen[w] = true;
                    order.push(w);
                }
            }
            at += 1;
        }
        order
    }

    fn components(&self) -> Vec<Vec<usize>> {
        let adj = self.both();
        let mut seen = vec![false; self.n()];
        let mut out = Vec::new();
        for v in 0..self.n() {
            if !seen[v] {
                let mut part = self.reach(&adj, v);
                part.iter().for_each(|w| seen[*w] = true);
                part.sort_unstable();
                out.push(part);
            }
        }
        out
    }

    /// The parts in which every vertex reaches every other one along the directions.
    fn strong(&self) -> Vec<Vec<usize>> {
        let n = self.n();
        let adj = self.out();
        let mut back = vec![Vec::new(); n];
        for (v, list) in adj.iter().enumerate() {
            for &(w, _) in list {
                back[w].push((v, 0));
            }
        }
        // the order in which a search leaves the vertices
        let mut order = Vec::new();
        let mut seen = vec![false; n];
        for root in 0..n {
            if seen[root] {
                continue;
            }
            seen[root] = true;
            let mut stack = vec![(root, 0usize)];
            while let Some((v, k)) = stack.pop() {
                if k < adj[v].len() {
                    stack.push((v, k + 1));
                    let w = adj[v][k].0;
                    if !seen[w] {
                        seen[w] = true;
                        stack.push((w, 0));
                    }
                } else {
                    order.push(v);
                }
            }
        }
        let mut done = vec![false; n];
        let mut out = Vec::new();
        for &root in order.iter().rev() {
            if done[root] {
                continue;
            }
            let mut part = vec![root];
            done[root] = true;
            let mut at = 0;
            while at < part.len() {
                for &(w, _) in &back[part[at]] {
                    if !done[w] {
                        done[w] = true;
                        part.push(w);
                    }
                }
                at += 1;
            }
            part.sort_unstable();
            out.push(part);
        }
        out.sort();
        out
    }

    /// The least costs from `start` and the edge by which each vertex is reached. With weights
    /// below zero the edges are gone through again and again; a way around with a cost below
    /// zero is an error.
    fn paths(&self, start: usize) -> Result<Reach, String> {
        let n = self.n();
        let adj = self.out();
        let mut dist: Vec<Option<Float>> = vec![None; n];
        let mut by: Vec<Option<usize>> = vec![None; n];
        dist[start] = Some(Float::new(128));
        if !self.weighted {
            let mut queue = VecDeque::from([start]);
            while let Some(v) = queue.pop_front() {
                for &(w, k) in &adj[v] {
                    if dist[w].is_none() {
                        dist[w] = Some(Float::with_val(
                            128,
                            dist[v].as_ref().expect("reached") + 1u32,
                        ));
                        by[w] = Some(k);
                        queue.push_back(w);
                    }
                }
            }
            return Ok((dist, by));
        }
        let cost = self.costs()?;
        for round in 0..=n {
            let mut changed = false;
            for v in 0..n {
                let Some(dv) = dist[v].clone() else {
                    continue;
                };
                for &(w, k) in &adj[v] {
                    let through = Float::with_val(128, &dv + &cost[k]);
                    if dist[w].as_ref().is_none_or(|old| through < *old) {
                        dist[w] = Some(through);
                        by[w] = Some(k);
                        changed = true;
                    }
                }
            }
            if !changed {
                return Ok((dist, by));
            }
            if round == n {
                break;
            }
        }
        Err("the graph has a way around with a weight below zero: there is no shortest path".into())
    }

    /// The way from `start` to `end` by the edges of `by`.
    fn walk_back(&self, by: &[Option<usize>], start: usize, end: usize) -> Vec<usize> {
        let mut path = vec![end];
        let mut at = end;
        while at != start {
            let e = &self.edges[by[at].expect("reached")];
            at = if e.b == at && (e.arrow || e.a != at) {
                e.a
            } else {
                e.b
            };
            path.push(at);
        }
        path.reverse();
        path
    }

    /// The exact length of a way given by its edges.
    fn length(&self, by: &[Option<usize>], start: usize, end: usize) -> Expr {
        let mut total = Vec::new();
        let mut at = end;
        while at != start {
            let e = &self.edges[by[at].expect("reached")];
            total.push(e.weight.clone());
            at = if e.b == at && (e.arrow || e.a != at) {
                e.a
            } else {
                e.b
            };
        }
        crate::simplify::simplify(&add(total))
    }

    fn distances(&self) -> Result<Vec<Vec<Expr>>, String> {
        let n = self.n();
        let mut out = Vec::new();
        for s in 0..n {
            let (dist, by) = self.paths(s)?;
            out.push(
                (0..n)
                    .map(|t| match &dist[t] {
                        Some(_) => self.length(&by, s, t),
                        None => Expr::inf(),
                    })
                    .collect(),
            );
        }
        Ok(out)
    }

    /// The largest distance from each vertex, as a number for comparisons and as it is.
    fn eccentricities(&self) -> Result<Vec<(Option<Float>, Expr)>, String> {
        let n = self.n();
        let mut out = Vec::new();
        for s in 0..n {
            let (dist, by) = self.paths(s)?;
            let mut worst: (Option<Float>, Expr) = (Some(Float::new(128)), Expr::zero());
            for (t, cost) in dist.iter().enumerate() {
                match cost {
                    None => {
                        worst = (None, Expr::inf());
                        break;
                    }
                    Some(d) => {
                        if worst.0.as_ref().is_some_and(|w| d > w) {
                            worst = (Some(d.clone()), self.length(&by, s, t));
                        }
                    }
                }
            }
            out.push(worst);
        }
        Ok(out)
    }

    fn degrees(&self) -> Vec<usize> {
        let mut d = vec![0; self.n()];
        for e in &self.edges {
            d[e.a] += 1;
            d[e.b] += 1;
        }
        d
    }

    fn laplacian(&self) -> matrix::Rows {
        let n = self.n();
        let mut m = vec![vec![0i64; n]; n];
        for e in self.edges.iter().filter(|e| e.a != e.b) {
            m[e.a][e.a] += 1;
            m[e.a][e.b] -= 1;
            if !e.arrow {
                m[e.b][e.b] += 1;
                m[e.b][e.a] -= 1;
            }
        }
        m.into_iter()
            .map(|row| row.into_iter().map(Expr::int).collect())
            .collect()
    }

    /// Two sides with every edge between them, when there are such.
    fn sides(&self) -> Option<Vec<bool>> {
        let adj = self.both();
        let n = self.n();
        let mut side: Vec<Option<bool>> = vec![None; n];
        for root in 0..n {
            if side[root].is_some() {
                continue;
            }
            side[root] = Some(false);
            let mut queue = VecDeque::from([root]);
            while let Some(v) = queue.pop_front() {
                for &(w, _) in &adj[v] {
                    match side[w] {
                        None => {
                            side[w] = side[v].map(|s| !s);
                            queue.push_back(w);
                        }
                        Some(s) if Some(s) == side[v] => return None,
                        Some(_) => {}
                    }
                }
            }
        }
        if self.edges.iter().any(|e| e.a == e.b) {
            return None;
        }
        Some(side.into_iter().map(|s| s.unwrap_or(false)).collect())
    }

    /// A way around as vertices, the first one again at the end.
    fn cycle(&self) -> Option<Vec<usize>> {
        if let Some(e) = self.edges.iter().find(|e| e.a == e.b) {
            return Some(vec![e.a, e.a]);
        }
        let adj = self.out();
        let n = self.n();
        // 0 new, 1 on the way, 2 done
        let mut state = vec![0u8; n];
        let mut came: Vec<Option<(usize, usize)>> = vec![None; n];
        for root in 0..n {
            if state[root] != 0 {
                continue;
            }
            state[root] = 1;
            let mut stack = vec![(root, 0usize)];
            while let Some((v, k)) = stack.pop() {
                if k == adj[v].len() {
                    state[v] = 2;
                    continue;
                }
                stack.push((v, k + 1));
                let (w, edge) = adj[v][k];
                // (an edge without a direction is not gone back at once)
                if !self.edges[edge].arrow && came[v].is_some_and(|(_, e)| e == edge) {
                    continue;
                }
                if state[w] == 1 {
                    let mut path = vec![w, v];
                    let mut at = v;
                    while at != w {
                        at = came[at].expect("on the way").0;
                        path.push(at);
                    }
                    path.reverse();
                    return Some(path);
                }
                if state[w] == 0 {
                    state[w] = 1;
                    came[w] = Some((v, edge));
                    stack.push((w, 0));
                }
            }
        }
        None
    }

    /// The number of ways to split the vertices into `k` groups without an edge inside a
    /// group, for every `k`.
    fn partitions(&self) -> Result<Vec<Integer>, String> {
        let n = self.n();
        if n > 16 {
            return Err("the chromatic polynomial is found for graphs of up to 16 vertices".into());
        }
        if self.edges.iter().any(|e| e.a == e.b) {
            return Ok(vec![Integer::new(); n + 1]);
        }
        let joined = self.joined();
        let near: Vec<u32> = (0..n)
            .map(|v| (0..n).filter(|w| joined[v][*w]).fold(0, |m, w| m | 1 << w))
            .collect();
        let full = 1usize << n;
        let mut free = vec![true; full];
        for set in 1..full {
            let low = set.trailing_zeros() as usize;
            let rest = set & (set - 1);
            free[set] = free[rest] && near[low] & rest as u32 == 0;
        }
        // ways[k][set]: the splits of `set` into k groups
        let mut ways = vec![vec![0u128; full]; n + 1];
        ways[0][0] = 1;
        for k in 1..=n {
            if crate::budget::expired() {
                return Err(SLOW.into());
            }
            for set in 1..full {
                let low = 1usize << set.trailing_zeros();
                let others = set & !low;
                // the group of the lowest vertex: it with a part of the others
                let mut part = others;
                loop {
                    let group = part | low;
                    if free[group] {
                        ways[k][set] += ways[k - 1][set & !group];
                    }
                    if part == 0 {
                        break;
                    }
                    part = (part - 1) & others;
                }
            }
        }
        Ok((0..=n).map(|k| Integer::from(ways[k][full - 1])).collect())
    }

    /// Groups of vertices without an edge inside a group, as few as there can be.
    fn coloring(&self) -> Result<Vec<Vec<usize>>, String> {
        let n = self.n();
        if self.edges.iter().any(|e| e.a == e.b) {
            return Err("a graph with a loop has no coloring".into());
        }
        let joined = self.joined();
        let mut order: Vec<usize> = (0..n).collect();
        let degree: Vec<usize> = (0..n)
            .map(|v| joined[v].iter().filter(|x| **x).count())
            .collect();
        order.sort_by_key(|v| std::cmp::Reverse(degree[*v]));
        let mut best: Option<Vec<usize>> = None;
        let mut color = vec![usize::MAX; n];
        fn place(
            at: usize,
            used: usize,
            order: &[usize],
            joined: &[Vec<bool>],
            color: &mut Vec<usize>,
            best: &mut Option<Vec<usize>>,
            limit: &mut usize,
        ) -> bool {
            if crate::budget::expired() {
                return false;
            }
            if used >= *limit {
                return true;
            }
            if at == order.len() {
                *limit = used;
                *best = Some(color.clone());
                return true;
            }
            let v = order[at];
            for c in 0..=used.min(*limit - 1) {
                if c >= *limit {
                    break;
                }
                let clash = (0..color.len()).any(|w| joined[v][w] && color[w] == c);
                if !clash {
                    color[v] = c;
                    if !place(at + 1, used.max(c + 1), order, joined, color, best, limit) {
                        return false;
                    }
                    color[v] = usize::MAX;
                }
            }
            true
        }
        let mut limit = n + 1;
        if !place(0, 0, &order, &joined, &mut color, &mut best, &mut limit) {
            return Err(SLOW.into());
        }
        let color = best.unwrap_or_default();
        let count = color.iter().map(|c| c + 1).max().unwrap_or(0);
        let mut groups = vec![Vec::new(); count];
        for (v, c) in color.iter().enumerate() {
            groups[*c].push(v);
        }
        groups.sort();
        Ok(groups)
    }

    /// The largest set of vertices that are all joined to each other.
    fn clique(&self, joined: &[Vec<bool>]) -> Result<Vec<usize>, String> {
        fn grow(
            chosen: &mut Vec<usize>,
            mut open: Vec<usize>,
            mut shut: Vec<usize>,
            joined: &[Vec<bool>],
            best: &mut Vec<usize>,
        ) -> bool {
            if crate::budget::expired() {
                return false;
            }
            if open.is_empty() && shut.is_empty() {
                if chosen.len() > best.len() {
                    *best = chosen.clone();
                }
                return true;
            }
            if chosen.len() + open.len() <= best.len() {
                return true;
            }
            // without the neighbours of the vertex with the most of them
            let pivot = open
                .iter()
                .chain(&shut)
                .copied()
                .max_by_key(|u| open.iter().filter(|w| joined[*u][**w]).count())
                .expect("a vertex");
            let tries: Vec<usize> = open
                .iter()
                .copied()
                .filter(|v| !joined[pivot][*v])
                .collect();
            for v in tries {
                chosen.push(v);
                let next_open = open.iter().copied().filter(|w| joined[v][*w]).collect();
                let next_shut = shut.iter().copied().filter(|w| joined[v][*w]).collect();
                if !grow(chosen, next_open, next_shut, joined, best) {
                    return false;
                }
                chosen.pop();
                open.retain(|w| *w != v);
                shut.push(v);
            }
            true
        }
        let mut best = Vec::new();
        if !grow(
            &mut Vec::new(),
            (0..self.n()).collect(),
            Vec::new(),
            joined,
            &mut best,
        ) {
            return Err(SLOW.into());
        }
        best.sort_unstable();
        Ok(best)
    }

    fn not_joined(&self) -> Vec<Vec<bool>> {
        let joined = self.joined();
        let n = self.n();
        (0..n)
            .map(|v| (0..n).map(|w| v != w && !joined[v][w]).collect())
            .collect()
    }

    /// A way that takes every edge once, as vertices; it ends where it began when it can.
    fn euler(&self) -> Option<Vec<usize>> {
        let m = self.edges.len();
        if m == 0 {
            return None;
        }
        let n = self.n();
        let adj = self.out();
        let start = if self.directed {
            let (mut out, mut inn) = (vec![0i64; n], vec![0i64; n]);
            for e in &self.edges {
                out[e.a] += 1;
                inn[e.b] += 1;
                if !e.arrow {
                    return None;
                }
            }
            let plus: Vec<usize> = (0..n).filter(|v| out[*v] - inn[*v] == 1).collect();
            let minus: Vec<usize> = (0..n).filter(|v| inn[*v] - out[*v] == 1).collect();
            let even = (0..n).filter(|v| out[*v] == inn[*v]).count();
            match (plus.as_slice(), minus.as_slice()) {
                ([], []) if even == n => self.edges[0].a,
                ([s], [_]) if even == n - 2 => *s,
                _ => return None,
            }
        } else {
            let degree = self.degrees();
            let odd: Vec<usize> = (0..n).filter(|v| degree[*v] % 2 == 1).collect();
            match odd.len() {
                0 => self.edges[0].a,
                2 => odd[0],
                _ => return None,
            }
        };
        let mut used = vec![false; m];
        let mut next = vec![0usize; n];
        let mut stack = vec![start];
        let mut path = Vec::new();
        while let Some(&v) = stack.last() {
            while next[v] < adj[v].len() && used[adj[v][next[v]].1] {
                next[v] += 1;
            }
            if next[v] == adj[v].len() {
                path.push(v);
                stack.pop();
            } else {
                let (w, k) = adj[v][next[v]];
                used[k] = true;
                stack.push(w);
            }
        }
        path.reverse();
        (path.len() == m + 1).then_some(path)
    }

    /// A way through every vertex once; with `closed` an edge leads back to the start.
    fn hamilton(&self, closed: bool) -> Result<Option<Vec<usize>>, String> {
        let n = self.n();
        if n == 0 {
            return Ok(None);
        }
        let mut step = vec![vec![false; n]; n];
        for e in &self.edges {
            step[e.a][e.b] = true;
            if !e.arrow {
                step[e.b][e.a] = true;
            }
        }
        fn extend(
            path: &mut Vec<usize>,
            on: &mut Vec<bool>,
            step: &[Vec<bool>],
            closed: bool,
        ) -> Option<bool> {
            if crate::budget::expired() {
                return None;
            }
            let n = step.len();
            let last = *path.last().expect("a start");
            if path.len() == n {
                return Some(!closed || step[last][path[0]]);
            }
            for w in 0..n {
                if !on[w] && step[last][w] {
                    on[w] = true;
                    path.push(w);
                    if extend(path, on, step, closed)? {
                        return Some(true);
                    }
                    path.pop();
                    on[w] = false;
                }
            }
            Some(false)
        }
        let starts = if closed { 1 } else { n };
        for start in 0..starts {
            let mut path = vec![start];
            let mut on = vec![false; n];
            on[start] = true;
            match extend(&mut path, &mut on, &step, closed) {
                None => return Err(SLOW.into()),
                Some(true) => {
                    if closed && n > 1 {
                        path.push(start);
                    }
                    if closed && n < 3 && !self.directed {
                        return Ok(None);
                    }
                    return Ok(Some(path));
                }
                Some(false) => {}
            }
        }
        Ok(None)
    }

    /// The most that flows from `s` to `t` when the weights are the most that an edge takes.
    fn flow(&self, s: usize, t: usize) -> Result<Rational, String> {
        let n = self.n();
        let mut cap = vec![vec![Rational::new(); n]; n];
        for e in &self.edges {
            let c = e
                .weight
                .as_rat()
                .filter(|c| c.cmp0() != std::cmp::Ordering::Less)
                .ok_or_else(|| {
                    format!(
                        "the capacity {} is not a rational number from 0 on",
                        e.weight
                    )
                })?;
            cap[e.a][e.b] += c;
            if !e.arrow {
                cap[e.b][e.a] += c;
            }
        }
        let mut total = Rational::new();
        loop {
            // the shortest way with room on every edge
            let mut came: Vec<Option<usize>> = vec![None; n];
            came[s] = Some(s);
            let mut queue = VecDeque::from([s]);
            while let Some(v) = queue.pop_front() {
                for w in 0..n {
                    if came[w].is_none() && cap[v][w].cmp0() == std::cmp::Ordering::Greater {
                        came[w] = Some(v);
                        queue.push_back(w);
                    }
                }
            }
            if came[t].is_none() {
                return Ok(total);
            }
            let mut room: Option<Rational> = None;
            let mut at = t;
            while at != s {
                let from = came[at].expect("reached");
                if room.as_ref().is_none_or(|r| cap[from][at] < *r) {
                    room = Some(cap[from][at].clone());
                }
                at = from;
            }
            let room = room.expect("a way");
            let mut at = t;
            while at != s {
                let from = came[at].expect("reached");
                cap[from][at] -= &room;
                cap[at][from] += &room;
                at = from;
            }
            total += room;
        }
    }

    /// As many edges as possible without a common vertex (Edmonds' blossoms).
    fn matching(&self) -> Vec<(usize, usize)> {
        let n = self.n();
        let joined = self.joined();
        let adj: Vec<Vec<usize>> = (0..n)
            .map(|v| (0..n).filter(|w| joined[v][*w]).collect())
            .collect();
        let mut mate: Vec<Option<usize>> = vec![None; n];
        for root in 0..n {
            if mate[root].is_some() {
                continue;
            }
            let mut parent: Vec<Option<usize>> = vec![None; n];
            let mut base: Vec<usize> = (0..n).collect();
            let mut used = vec![false; n];
            used[root] = true;
            let mut queue = VecDeque::from([root]);
            let mut end = None;
            'search: while let Some(v) = queue.pop_front() {
                for &w in &adj[v] {
                    if base[v] == base[w] || mate[v] == Some(w) {
                        continue;
                    }
                    if w == root || mate[w].is_some_and(|m| parent[m].is_some()) {
                        // an odd way around: it shrinks to its lowest common vertex
                        let common = {
                            let mut seen = vec![false; n];
                            let mut a = v;
                            loop {
                                a = base[a];
                                seen[a] = true;
                                match mate[a] {
                                    Some(m) => a = parent[m].expect("a tree edge"),
                                    None => break,
                                }
                            }
                            let mut b = w;
                            loop {
                                b = base[b];
                                if seen[b] {
                                    break b;
                                }
                                b = parent[mate[b].expect("matched")].expect("a tree edge");
                            }
                        };
                        let mut inside = vec![false; n];
                        for (mut x, mut child) in [(v, w), (w, v)] {
                            while base[x] != common {
                                let m = mate[x].expect("matched");
                                inside[base[x]] = true;
                                inside[base[m]] = true;
                                parent[x] = Some(child);
                                child = m;
                                x = parent[m].expect("a tree edge");
                            }
                        }
                        for u in 0..n {
                            if inside[base[u]] {
                                base[u] = common;
                                if !used[u] {
                                    used[u] = true;
                                    queue.push_back(u);
                                }
                            }
                        }
                    } else if parent[w].is_none() {
                        parent[w] = Some(v);
                        match mate[w] {
                            None => {
                                end = Some(w);
                                break 'search;
                            }
                            Some(m) => {
                                used[m] = true;
                                queue.push_back(m);
                            }
                        }
                    }
                }
            }
            // the way found: its edges change sides
            let mut at = end;
            while let Some(v) = at {
                let before = parent[v].expect("a tree edge");
                let next = mate[before];
                mate[v] = Some(before);
                mate[before] = Some(v);
                at = next;
            }
        }
        (0..n)
            .filter_map(|v| mate[v].filter(|m| v < *m).map(|m| (v, m)))
            .collect()
    }

    /// A numbering of the vertices of `other` that carries the edges of this graph to its
    /// edges.
    fn same_shape(&self, other: &Graph) -> Result<bool, String> {
        let n = self.n();
        if n != other.n()
            || self.edges.len() != other.edges.len()
            || self.directed != other.directed
        {
            return Ok(false);
        }
        let count = |g: &Graph| {
            let mut m = vec![vec![0u32; n]; n];
            for e in &g.edges {
                m[e.a][e.b] += 1;
                if !e.arrow && e.a != e.b {
                    m[e.b][e.a] += 1;
                }
            }
            m
        };
        let (a, b) = (count(self), count(other));
        let signature = |m: &[Vec<u32>], v: usize| {
            let out: u32 = m[v].iter().sum();
            let inn: u32 = m.iter().map(|row| row[v]).sum();
            (out, inn, m[v][v])
        };
        let (sa, sb): (Vec<_>, Vec<_>) = (
            (0..n).map(|v| signature(&a, v)).collect(),
            (0..n).map(|v| signature(&b, v)).collect(),
        );
        let (mut xa, mut xb) = (sa.clone(), sb.clone());
        xa.sort_unstable();
        xb.sort_unstable();
        if xa != xb {
            return Ok(false);
        }
        fn assign(
            at: usize,
            image: &mut Vec<usize>,
            taken: &mut Vec<bool>,
            a: &[Vec<u32>],
            b: &[Vec<u32>],
            sa: &[(u32, u32, u32)],
            sb: &[(u32, u32, u32)],
        ) -> Option<bool> {
            if crate::budget::expired() {
                return None;
            }
            let n = a.len();
            if at == n {
                return Some(true);
            }
            for w in 0..n {
                if taken[w] || sa[at] != sb[w] {
                    continue;
                }
                let fits =
                    (0..at).all(|u| a[at][u] == b[w][image[u]] && a[u][at] == b[image[u]][w]);
                if fits {
                    taken[w] = true;
                    image.push(w);
                    if assign(at + 1, image, taken, a, b, sa, sb)? {
                        return Some(true);
                    }
                    image.pop();
                    taken[w] = false;
                }
            }
            Some(false)
        }
        assign(0, &mut Vec::new(), &mut vec![false; n], &a, &b, &sa, &sb)
            .ok_or_else(|| SLOW.to_string())
    }
}

/// The numbers `1 ..= n` joined by the pairs.
fn numbered(n: usize, pairs: &[(usize, usize)]) -> Expr {
    let mut items: Vec<Expr> = pairs
        .iter()
        .map(|(a, b)| Expr::list(vec![int(*a), int(*b)]))
        .collect();
    for v in 1..=n {
        if !pairs.iter().any(|(a, b)| *a == v || *b == v) {
            items.push(Expr::list(vec![int(v)]));
        }
    }
    Expr::list(items)
}

fn count_of(e: &Expr, what: &str, least: usize) -> Result<usize, String> {
    e.as_i64()
        .and_then(|k| usize::try_from(k).ok())
        .filter(|k| (least..=2000).contains(k))
        .ok_or_else(|| format!("{what} takes a whole number from {least} to 2000"))
}

/// The graphs that have a name.
fn named(name: &str, a: &[Expr]) -> Option<Res> {
    Some((|| -> Res {
        Ok(match (name, a) {
            ("completegraph", [n]) => {
                let n = count_of(n, name, 1)?;
                let pairs: Vec<_> = (1..=n)
                    .flat_map(|a| (a + 1..=n).map(move |b| (a, b)))
                    .collect();
                numbered(n, &pairs)
            }
            ("cyclegraph", [n]) => {
                let n = count_of(n, name, 3)?;
                let pairs: Vec<_> = (1..=n).map(|a| (a, a % n + 1)).collect();
                numbered(n, &pairs)
            }
            ("pathgraph", [n]) => {
                let n = count_of(n, name, 1)?;
                let pairs: Vec<_> = (1..n).map(|a| (a, a + 1)).collect();
                numbered(n, &pairs)
            }
            ("stargraph", [n]) => {
                let n = count_of(n, name, 1)?;
                let pairs: Vec<_> = (2..=n).map(|b| (1, b)).collect();
                numbered(n, &pairs)
            }
            ("wheelgraph", [n]) => {
                let n = count_of(n, name, 4)?;
                let mut pairs: Vec<_> = (2..=n).map(|b| (1, b)).collect();
                pairs.extend((2..=n).map(|a| (a, if a == n { 2 } else { a + 1 })));
                numbered(n, &pairs)
            }
            ("gridgraph", [rows, columns]) => {
                let (r, c) = (count_of(rows, name, 1)?, count_of(columns, name, 1)?);
                let at = |i: usize, j: usize| i * c + j + 1;
                let mut pairs = Vec::new();
                for i in 0..r {
                    for j in 0..c {
                        if j + 1 < c {
                            pairs.push((at(i, j), at(i, j + 1)));
                        }
                        if i + 1 < r {
                            pairs.push((at(i, j), at(i + 1, j)));
                        }
                    }
                }
                numbered(r * c, &pairs)
            }
            ("hypercubegraph", [d]) => {
                let d = count_of(d, name, 1)?;
                if d > 10 {
                    return Err("hypercubegraph takes a dimension up to 10".into());
                }
                let mut items = Vec::new();
                for v in 0..1usize << d {
                    for bit in 0..d {
                        let w = v | 1 << bit;
                        if w != v {
                            items.push(Expr::list(vec![int(v), int(w)]));
                        }
                    }
                }
                Expr::list(items)
            }
            ("petersengraph", []) => {
                let mut pairs: Vec<_> = (1..=5).map(|a| (a, a % 5 + 1)).collect();
                pairs.extend((1..=5).map(|a| (a, a + 5)));
                pairs.extend((1..=5).map(|a| (a + 5, (a + 1) % 5 + 6)));
                numbered(10, &pairs)
            }
            ("completebipartite", [m, n]) => {
                let (m, n) = (count_of(m, name, 1)?, count_of(n, name, 1)?);
                let pairs: Vec<_> = (1..=m)
                    .flat_map(|a| (m + 1..=m + n).map(move |b| (a, b)))
                    .collect();
                numbered(m + n, &pairs)
            }
            ("adjacencygraph", [m]) => {
                let rows = matrix::rows(m).filter(|r| r.iter().all(|row| row.len() == r.len()));
                let rows = rows.ok_or("adjacencygraph takes a square matrix")?;
                let n = rows.len();
                let even = (0..n).all(|i| (0..n).all(|j| rows[i][j] == rows[j][i]));
                let mut items = Vec::new();
                for (i, row) in rows.iter().enumerate() {
                    for (j, entry) in row.iter().enumerate() {
                        if entry.is_zero() || (even && j < i) {
                            continue;
                        }
                        let (a, b) = (int(i + 1), int(j + 1));
                        let edge = if even {
                            Expr::list(vec![a, b])
                        } else {
                            Expr::call("rule", vec![a, b])
                        };
                        // a whole number of edges, another entry is a weight
                        match entry.as_i64().filter(|k| (1..=64).contains(k)) {
                            Some(k) => items.extend(std::iter::repeat_n(edge, k as usize)),
                            None if even => {
                                items.push(Expr::list(vec![int(i + 1), int(j + 1), entry.clone()]))
                            }
                            None => items.push(Expr::list(vec![edge, entry.clone()])),
                        }
                    }
                }
                for (v, row) in rows.iter().enumerate() {
                    if row.iter().all(Expr::is_zero) && rows.iter().all(|r| r[v].is_zero()) {
                        items.push(Expr::list(vec![int(v + 1)]));
                    }
                }
                Expr::list(items)
            }
            _ => return Err(String::new()),
        })
    })())
    .filter(|r| !matches!(r, Err(e) if e.is_empty()))
}

/// The polynomial `sum ways[k] x (x - 1) ... (x - k + 1)`.
fn falling(ways: &[Integer], x: &Expr) -> Expr {
    let mut terms = Vec::new();
    let mut product = Expr::one();
    for (k, w) in ways.iter().enumerate() {
        if k > 0 {
            product = mul(vec![product, sub(x.clone(), int(k - 1))]);
        }
        terms.push(mul(vec![Expr::integer(w.clone()), product.clone()]));
    }
    crate::expand::expand(&add(terms))
}

const NAMES: &[&str] = &[
    "vertexlist",
    "edgelist",
    "vertexcount",
    "edgecount",
    "adjacencymatrix",
    "weightmatrix",
    "incidencematrix",
    "laplacianmatrix",
    "degrees",
    "indegrees",
    "outdegrees",
    "vertexdegree",
    "neighbors",
    "components",
    "strongcomponents",
    "isconnected",
    "shortestpath",
    "graphdistance",
    "distancematrix",
    "eccentricity",
    "diameter",
    "graphradius",
    "graphcenter",
    "girth",
    "spanningtree",
    "spanningtreecount",
    "isbipartite",
    "bipartition",
    "istree",
    "isacyclic",
    "findcycle",
    "topologicalsort",
    "chromaticpolynomial",
    "chromaticnumber",
    "graphcoloring",
    "iseulerian",
    "eulerianpath",
    "ishamiltonian",
    "hamiltoniancycle",
    "hamiltonianpath",
    "maxflow",
    "maxmatching",
    "maxclique",
    "cliquenumber",
    "maxindependentset",
    "independencenumber",
    "vertexcover",
    "complementgraph",
    "linegraph",
    "subgraph",
    "isisomorphic",
    "pagerank",
];

/// A command on graphs; `None` for other names.
pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    if let Some(r) = named(name, a) {
        return Some(r);
    }
    if !NAMES.contains(&name) {
        return None;
    }
    Some(crate::budget::with_limit(Duration::from_secs(20), || {
        command(name, a)
    }))
}

fn command(name: &str, a: &[Expr]) -> Res {
    let g = Graph::read(a.first().ok_or(USAGE)?)?;
    let n = g.n();
    let usage = |text: &str| Err(format!("{name}{text}"));
    Ok(match (name, &a[1..]) {
        ("vertexlist", []) => g.list(&(0..n).collect::<Vec<_>>()),
        ("edgelist", []) => Expr::list(g.edges.iter().map(|e| g.edge_expr(e)).collect()),
        ("vertexcount", []) => int(n),
        ("edgecount", []) => int(g.edges.len()),
        ("adjacencymatrix", []) | ("weightmatrix", []) => {
            let mut m = vec![vec![Vec::new(); n]; n];
            for e in &g.edges {
                let entry = if name == "weightmatrix" {
                    e.weight.clone()
                } else {
                    Expr::one()
                };
                m[e.a][e.b].push(entry.clone());
                if !e.arrow && e.a != e.b {
                    m[e.b][e.a].push(entry);
                }
            }
            matrix::from_rows(
                m.into_iter()
                    .map(|row| row.into_iter().map(add).collect())
                    .collect(),
            )
        }
        ("incidencematrix", []) => {
            let mut m = vec![vec![Expr::zero(); g.edges.len()]; n];
            for (k, e) in g.edges.iter().enumerate() {
                if e.a == e.b {
                    m[e.a][k] = Expr::int(if e.arrow { 0 } else { 2 });
                } else {
                    m[e.a][k] = Expr::int(if e.arrow { -1 } else { 1 });
                    m[e.b][k] = Expr::one();
                }
            }
            matrix::from_rows(m)
        }
        ("laplacianmatrix", []) => matrix::from_rows(g.laplacian()),
        ("degrees", []) => Expr::list(g.degrees().into_iter().map(int).collect()),
        ("indegrees", []) | ("outdegrees", []) => {
            let mut d = vec![0; n];
            for e in &g.edges {
                let (from, to) = (e.a, e.b);
                d[if name == "indegrees" { to } else { from }] += 1;
                if !e.arrow && from != to {
                    d[if name == "indegrees" { from } else { to }] += 1;
                }
            }
            Expr::list(d.into_iter().map(int).collect())
        }
        ("vertexdegree", [v]) => int(g.degrees()[g.index(v)?]),
        ("neighbors", [v]) => {
            let v = g.index(v)?;
            let mut near: Vec<usize> = g.out()[v].iter().map(|(w, _)| *w).collect();
            near.sort_unstable();
            near.dedup();
            g.list(&near)
        }
        ("components", []) => Expr::list(g.components().iter().map(|p| g.list(p)).collect()),
        ("strongcomponents", []) => Expr::list(g.strong().iter().map(|p| g.list(p)).collect()),
        ("isconnected", []) => truth(if g.directed {
            g.strong().len() <= 1
        } else {
            g.components().len() <= 1
        }),
        ("shortestpath", [s, t]) => {
            let (s, t) = (g.index(s)?, g.index(t)?);
            let (dist, by) = g.paths(s)?;
            match dist[t] {
                Some(_) => g.list(&g.walk_back(&by, s, t)),
                None => Expr::list(Vec::new()),
            }
        }
        ("graphdistance", [s, t]) => {
            let (s, t) = (g.index(s)?, g.index(t)?);
            let (dist, by) = g.paths(s)?;
            match dist[t] {
                Some(_) => g.length(&by, s, t),
                None => Expr::inf(),
            }
        }
        ("distancematrix", []) => matrix::from_rows(g.distances()?),
        ("eccentricity", [v]) => g.eccentricities()?[g.index(v)?].1.clone(),
        ("diameter", []) | ("graphradius", []) | ("graphcenter", []) => {
            let all = g.eccentricities()?;
            if n == 0 {
                return usage(": the graph has no vertex");
            }
            // (no number: a vertex that does not reach the others)
            let key = |k: &usize| match &all[*k].0 {
                Some(x) => (0, x.clone()),
                None => (1, Float::new(64)),
            };
            let cmp = |a: &usize, b: &usize| {
                key(a)
                    .partial_cmp(&key(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            };
            let least = (0..n).min_by(cmp).expect("a vertex");
            let most = (0..n).max_by(cmp).expect("a vertex");
            match name {
                "diameter" => all[most].1.clone(),
                "graphradius" => all[least].1.clone(),
                _ => {
                    let centre: Vec<usize> = (0..n)
                        .filter(|k| cmp(k, &least) == std::cmp::Ordering::Equal)
                        .collect();
                    g.list(&centre)
                }
            }
        }
        ("girth", []) => {
            g.undirected("girth")?;
            let mut best: Option<usize> = None;
            if g.edges.iter().any(|e| e.a == e.b) {
                best = Some(1);
            }
            let adj = g.out();
            for root in 0..n {
                if best == Some(1) {
                    break;
                }
                let mut dist = vec![usize::MAX; n];
                let mut by = vec![usize::MAX; n];
                dist[root] = 0;
                let mut queue = VecDeque::from([root]);
                while let Some(v) = queue.pop_front() {
                    for &(w, k) in &adj[v] {
                        if dist[w] == usize::MAX {
                            dist[w] = dist[v] + 1;
                            by[w] = k;
                            queue.push_back(w);
                        } else if by[v] != k {
                            let around = dist[v] + dist[w] + 1;
                            if best.is_none_or(|b| around < b) {
                                best = Some(around);
                            }
                        }
                    }
                }
            }
            best.map_or_else(Expr::inf, int)
        }
        ("spanningtree", []) => {
            g.undirected("spanningtree")?;
            let cost = g.costs()?;
            let mut order: Vec<usize> = (0..g.edges.len()).collect();
            order.sort_by(|a, b| {
                cost[*a]
                    .partial_cmp(&cost[*b])
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let mut group: Vec<usize> = (0..n).collect();
            fn find(group: &mut [usize], mut v: usize) -> usize {
                while group[v] != v {
                    group[v] = group[group[v]];
                    v = group[v];
                }
                v
            }
            let mut chosen = Vec::new();
            for k in order {
                let e = &g.edges[k];
                let (ra, rb) = (find(&mut group, e.a), find(&mut group, e.b));
                if ra != rb {
                    group[ra] = rb;
                    chosen.push(k);
                }
            }
            chosen.sort_unstable();
            let edges: Vec<Edge> = chosen.iter().map(|k| g.edges[*k].clone()).collect();
            g.graph_expr(&edges)
        }
        ("spanningtreecount", []) => {
            g.undirected("spanningtreecount")?;
            if n == 0 {
                return Ok(Expr::zero());
            }
            let l = g.laplacian();
            let minor: matrix::Rows = l[1..].iter().map(|row| row[1..].to_vec()).collect();
            if minor.is_empty() {
                Expr::one()
            } else {
                matrix::det(&minor)?
            }
        }
        ("isbipartite", []) => truth(g.sides().is_some()),
        ("bipartition", []) => match g.sides() {
            Some(side) => {
                let part = |s: bool| g.list(&(0..n).filter(|v| side[*v] == s).collect::<Vec<_>>());
                Expr::list(vec![part(false), part(true)])
            }
            None => return usage(": the graph has an odd way around"),
        },
        ("istree", []) => {
            truth(!g.directed && n > 0 && g.components().len() == 1 && g.edges.len() == n - 1)
        }
        ("isacyclic", []) => truth(g.cycle().is_none()),
        ("findcycle", []) => match g.cycle() {
            Some(c) => g.list(&c),
            None => Expr::list(Vec::new()),
        },
        ("topologicalsort", []) => {
            if g.edges.iter().any(|e| !e.arrow) {
                return usage(" is for graphs with directions: [1 -> 2, 2 -> 3]");
            }
            let adj = g.out();
            let mut waits = vec![0; n];
            g.edges.iter().for_each(|e| waits[e.b] += 1);
            let mut ready: Vec<usize> = (0..n).filter(|v| waits[*v] == 0).collect();
            let mut order = Vec::new();
            while let Some(&v) = ready.iter().min() {
                ready.retain(|w| *w != v);
                order.push(v);
                for &(w, _) in &adj[v] {
                    waits[w] -= 1;
                    if waits[w] == 0 {
                        ready.push(w);
                    }
                }
            }
            if order.len() < n {
                return usage(": the graph has a way around");
            }
            g.list(&order)
        }
        ("chromaticpolynomial", rest) if rest.len() <= 1 => {
            let x = rest.first().cloned().unwrap_or_else(|| Expr::sym("x"));
            falling(&g.partitions()?, &x)
        }
        ("chromaticnumber", []) => int(g.coloring()?.len()),
        ("graphcoloring", []) => Expr::list(g.coloring()?.iter().map(|c| g.list(c)).collect()),
        ("iseulerian", []) => truth(g.euler().is_some_and(|p| p.first() == p.last())),
        ("eulerianpath", []) => match g.euler() {
            Some(p) => g.list(&p),
            None => Expr::list(Vec::new()),
        },
        ("ishamiltonian", []) => truth(g.hamilton(true)?.is_some()),
        ("hamiltoniancycle", []) | ("hamiltonianpath", []) => {
            match g.hamilton(name == "hamiltoniancycle")? {
                Some(p) => g.list(&p),
                None => Expr::list(Vec::new()),
            }
        }
        ("maxflow", [s, t]) => {
            let (s, t) = (g.index(s)?, g.index(t)?);
            if s == t {
                return usage("(graph, s, t) takes two different vertices");
            }
            Expr::rational(g.flow(s, t)?)
        }
        ("maxmatching", []) => {
            g.undirected("maxmatching")?;
            Expr::list(
                g.matching()
                    .iter()
                    .map(|(a, b)| g.list(&[*a, *b]))
                    .collect(),
            )
        }
        ("maxclique", []) => g.list(&g.clique(&g.joined())?),
        ("cliquenumber", []) => int(g.clique(&g.joined())?.len()),
        ("maxindependentset", []) => g.list(&g.clique(&g.not_joined())?),
        ("independencenumber", []) => int(g.clique(&g.not_joined())?.len()),
        ("vertexcover", []) => {
            let free = g.clique(&g.not_joined())?;
            g.list(&(0..n).filter(|v| !free.contains(v)).collect::<Vec<_>>())
        }
        ("complementgraph", []) => {
            let mut step = vec![vec![false; n]; n];
            for e in &g.edges {
                step[e.a][e.b] = true;
                if !e.arrow {
                    step[e.b][e.a] = true;
                }
            }
            let mut edges = Vec::new();
            for (a, row) in step.iter().enumerate() {
                for (b, joined) in row.iter().enumerate() {
                    if a != b && !joined && (g.directed || a < b) {
                        edges.push(Edge {
                            a,
                            b,
                            weight: Expr::one(),
                            arrow: g.directed,
                        });
                    }
                }
            }
            let plain = Graph {
                verts: g.verts.clone(),
                edges: Vec::new(),
                directed: g.directed,
                weighted: false,
            };
            plain.graph_expr(&edges)
        }
        ("linegraph", []) => {
            g.undirected("linegraph")?;
            let m = g.edges.len();
            let mut pairs = Vec::new();
            for i in 0..m {
                for j in i + 1..m {
                    let (e, f) = (&g.edges[i], &g.edges[j]);
                    if e.a == f.a || e.a == f.b || e.b == f.a || e.b == f.b {
                        pairs.push((i + 1, j + 1));
                    }
                }
            }
            numbered(m, &pairs)
        }
        ("subgraph", [keep]) => {
            let keep: Vec<usize> = keep
                .as_list()
                .ok_or("subgraph(graph, [vertices])")?
                .iter()
                .map(|v| g.index(v))
                .collect::<Result<_, _>>()?;
            let edges: Vec<Edge> = g
                .edges
                .iter()
                .filter(|e| keep.contains(&e.a) && keep.contains(&e.b))
                .cloned()
                .collect();
            let mut items: Vec<Expr> = edges.iter().map(|e| g.edge_expr(e)).collect();
            for v in &keep {
                if !edges.iter().any(|e| e.a == *v || e.b == *v) {
                    items.push(Expr::list(vec![g.verts[*v].clone()]));
                }
            }
            Expr::list(items)
        }
        ("isisomorphic", [other]) => truth(g.same_shape(&Graph::read(other)?)?),
        ("pagerank", rest) if rest.len() <= 1 => {
            let damping = rest.first().cloned().unwrap_or_else(|| Expr::rat(85, 100));
            if n == 0 {
                return Ok(Expr::list(Vec::new()));
            }
            let adj = g.out();
            let share = pow(int(n), Expr::minus_one());
            // rank = (1 - d)/n + d * (what comes in), a vertex without a way out gives to all
            let mut rows = vec![vec![Expr::zero(); n]; n];
            for v in 0..n {
                let out = adj[v].len();
                for (w, row) in rows.iter_mut().enumerate() {
                    let part = if out == 0 {
                        share.clone()
                    } else {
                        let times = adj[v].iter().filter(|(x, _)| *x == w).count();
                        mul(vec![int(times), pow(int(out), Expr::minus_one())])
                    };
                    row[v] = neg(mul(vec![damping.clone(), part]));
                }
            }
            for (v, row) in rows.iter_mut().enumerate() {
                row[v] = add(vec![Expr::one(), row[v].clone()]);
            }
            let right = vec![mul(vec![sub(Expr::one(), damping), share]); n];
            matrix::linsolve(&rows, &right)?
        }
        _ => return usage(": the arguments do not fit (see --functions)"),
    })
}
