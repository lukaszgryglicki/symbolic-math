//! Tables of named columns, tables with a time for every row, and values of a few kinds
//! (the tables, timetables and categorical arrays of MATLAB): each is written, shown and
//! kept as the call that makes it.

use crate::arrays::{order, tidy, truthy, whole, word};
use crate::canon::{add, div, mul, sub};
use crate::eval::Session;
use crate::expr::Expr;
use crate::radix::text_of;
use crate::texts::text;
use std::cmp::Ordering;

type Res = Result<Expr, String>;

const TABLE: &str = "table";
const TIMED: &str = "timetable";
const KINDS: &str = "categorical";
const TIME: &str = "Time";

#[derive(Clone)]
struct Table {
    timed: bool,
    names: Vec<String>,
    columns: Vec<Vec<Expr>>,
}

fn list_of(e: &Expr) -> Vec<Expr> {
    match e.as_list() {
        Some(items) => items.to_vec(),
        None => vec![e.clone()],
    }
}

fn texts(names: &[String]) -> Expr {
    Expr::list(names.iter().map(|n| text(n)).collect())
}

/// The order of two entries of a column: numbers by value, texts by their letters, what is
/// missing last.
fn compare(a: &Expr, b: &Expr) -> Ordering {
    match (a.is_undefined(), b.is_undefined()) {
        (true, true) => return Ordering::Equal,
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        _ => {}
    }
    match (text_of(a), text_of(b)) {
        (Some(x), Some(y)) => x.cmp(y),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => order(a, b).unwrap_or_else(|_| a.cmp(b)),
    }
}

fn same(a: &Expr, b: &Expr) -> bool {
    compare(a, b) == Ordering::Equal
}

fn same_row(a: &[Expr], b: &[Expr]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same(x, y))
}

fn compare_rows(a: &[Expr], b: &[Expr]) -> Ordering {
    a.iter()
        .zip(b)
        .map(|(x, y)| compare(x, y))
        .find(|o| *o != Ordering::Equal)
        .unwrap_or(Ordering::Equal)
}

fn is_number(e: &Expr) -> bool {
    e.is_const() && text_of(e).is_none() && !e.is_undefined() && e.as_list().is_none()
}

impl Table {
    fn height(&self) -> usize {
        self.columns.first().map_or(0, Vec::len)
    }

    fn expr(self) -> Expr {
        let kind = if self.timed { TIMED } else { TABLE };
        let args = self
            .names
            .iter()
            .zip(self.columns)
            .flat_map(|(n, c)| [text(n), Expr::list(c)])
            .collect();
        Expr::call(kind, args)
    }

    fn find(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    /// The column that a name or a number from 1 names.
    fn column(&self, e: &Expr, who: &str) -> Result<usize, String> {
        match text_of(e) {
            Some(name) => self.find(name).ok_or_else(|| {
                format!(
                    "{who}: the table has no variable \"{name}\" (it has {})",
                    self.names.join(", ")
                )
            }),
            None => whole(e, "the number of a variable", 1, self.names.len() as i64)
                .map(|k| k as usize - 1)
                .map_err(|e| format!("{who}: {e}")),
        }
    }

    fn columns_of(&self, e: &Expr, who: &str) -> Result<Vec<usize>, String> {
        list_of(e).iter().map(|x| self.column(x, who)).collect()
    }

    fn row(&self, k: usize) -> Vec<Expr> {
        self.columns.iter().map(|c| c[k].clone()).collect()
    }

    fn rows(&self) -> Vec<Vec<Expr>> {
        (0..self.height()).map(|k| self.row(k)).collect()
    }

    fn take(&self, rows: &[usize]) -> Table {
        Table {
            timed: self.timed,
            names: self.names.clone(),
            columns: self
                .columns
                .iter()
                .map(|c| rows.iter().map(|k| c[*k].clone()).collect())
                .collect(),
        }
    }

    fn only(&self, columns: &[usize]) -> Table {
        Table {
            timed: self.timed && columns.first() == Some(&0),
            names: columns.iter().map(|k| self.names[*k].clone()).collect(),
            columns: columns.iter().map(|k| self.columns[*k].clone()).collect(),
        }
    }

    fn with_rows(&self, rows: &[Vec<Expr>]) -> Table {
        Table {
            timed: self.timed,
            names: self.names.clone(),
            columns: (0..self.names.len())
                .map(|j| rows.iter().map(|r| r[j].clone()).collect())
                .collect(),
        }
    }

    /// The rows in the order of some columns, each rising or falling.
    fn sorted(&self, keys: &[(usize, bool)]) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.height()).collect();
        order.sort_by(|a, b| {
            keys.iter()
                .map(|(k, falling)| {
                    let o = compare(&self.columns[*k][*a], &self.columns[*k][*b]);
                    if *falling { o.reverse() } else { o }
                })
                .find(|o| *o != Ordering::Equal)
                .unwrap_or(Ordering::Equal)
        });
        order
    }

    fn checked(self, who: &str) -> Result<Table, String> {
        let height = self.height();
        if self.columns.iter().any(|c| c.len() != height) {
            return Err(format!("{who}: the variables of a table have one length"));
        }
        for (k, name) in self.names.iter().enumerate() {
            if name.is_empty() {
                return Err(format!("{who}: a variable needs a name"));
            }
            if self.names[..k].contains(name) {
                return Err(format!("{who}: the name \"{name}\" comes twice"));
            }
        }
        Ok(self)
    }
}

fn read(e: &Expr) -> Option<Table> {
    let (args, timed) = match (e.fn_args(TABLE), e.fn_args(TIMED)) {
        (Some(a), _) => (a, false),
        (None, Some(a)) => (a, true),
        _ => return None,
    };
    if !args.len().is_multiple_of(2) {
        return None;
    }
    let mut table = Table {
        timed,
        names: Vec::new(),
        columns: Vec::new(),
    };
    for pair in args.chunks(2) {
        table.names.push(text_of(&pair[0])?.to_string());
        table.columns.push(pair[1].as_list()?.to_vec());
    }
    let height = table.height();
    table
        .columns
        .iter()
        .all(|c| c.len() == height)
        .then_some(table)
}

pub(crate) fn is_table(e: &Expr) -> bool {
    read(e).is_some()
}

fn need(e: &Expr, who: &str) -> Result<Table, String> {
    read(e).ok_or_else(|| {
        format!("{who} needs a table made by table(\"name\", [values], ...), {e} is none")
    })
}

// ---- values of a few kinds ----

#[derive(Clone)]
struct Kinds {
    /// the place of every value among the kinds; none where it is of no kind
    values: Vec<Option<usize>>,
    kinds: Vec<Expr>,
    ordered: bool,
}

impl Kinds {
    fn expr(self) -> Expr {
        let values = self
            .values
            .iter()
            .map(|v| v.map_or_else(Expr::undefined, |k| self.kinds[k].clone()))
            .collect();
        let mut args = vec![Expr::list(values), Expr::list(self.kinds)];
        if self.ordered {
            args.push(text("ordinal"));
        }
        Expr::call(KINDS, args)
    }

    fn place(&self, kind: &Expr) -> Option<usize> {
        self.kinds.iter().position(|k| same(k, kind))
    }

    fn texts(&self) -> Vec<Expr> {
        self.values
            .iter()
            .map(|v| v.map_or_else(Expr::undefined, |k| self.kinds[k].clone()))
            .collect()
    }
}

fn read_kinds(e: &Expr) -> Option<Kinds> {
    let args = e.fn_args(KINDS)?;
    let (values, kinds) = (args.first()?.as_list()?, args.get(1)?.as_list()?);
    let ordered = match args.get(2) {
        None => false,
        Some(o) => word(o).as_deref() == Some("ordinal") && args.len() == 3,
    };
    if args.len() > 3 || (args.len() == 3 && !ordered) {
        return None;
    }
    let found = values
        .iter()
        .map(|v| kinds.iter().position(|k| same(k, v)))
        .collect();
    Some(Kinds {
        values: found,
        kinds: kinds.to_vec(),
        ordered,
    })
}

/// The kinds of some values: those that differ, in rising order.
fn kinds_of(values: &[Expr]) -> Vec<Expr> {
    let mut kinds: Vec<Expr> = Vec::new();
    for v in values.iter().filter(|v| !v.is_undefined()) {
        if !kinds.iter().any(|k| same(k, v)) {
            kinds.push(v.clone());
        }
    }
    kinds.sort_by(compare);
    kinds
}

fn make_kinds(a: &[Expr]) -> Res {
    let usage = "categorical([values][, [kinds][, [names of the kinds]]][, \"ordinal\"])";
    let mut rest = a;
    let mut ordered = false;
    loop {
        match rest {
            [head @ .., o] if word(o).as_deref() == Some("ordinal") && text_of(o).is_some() => {
                (rest, ordered) = (head, true);
            }
            [head @ .., o, flag] if word(o).as_deref() == Some("ordinal") => {
                (rest, ordered) = (head, truthy(flag)?);
            }
            _ => break,
        }
    }
    let values = match rest.first() {
        Some(v) => match read_kinds(v) {
            Some(known) if rest.len() == 1 => {
                return Ok(Kinds {
                    ordered: ordered || known.ordered,
                    ..known
                }
                .expr());
            }
            Some(known) => known.texts(),
            None => v.as_list().ok_or(usage)?.to_vec(),
        },
        None => return Err(usage.into()),
    };
    let kinds = match rest.get(1) {
        Some(k) => k.as_list().ok_or(usage)?.to_vec(),
        None => kinds_of(&values),
    };
    for (k, kind) in kinds.iter().enumerate() {
        if kinds[..k].iter().any(|other| same(other, kind)) {
            return Err(format!("categorical: the kind {kind} comes twice"));
        }
    }
    let found: Vec<Option<usize>> = values
        .iter()
        .map(|v| kinds.iter().position(|k| same(k, v)))
        .collect();
    let kinds = match rest.get(2) {
        Some(names) => {
            let names = names
                .as_list()
                .filter(|n| n.len() == kinds.len())
                .ok_or("categorical: a name for every kind")?;
            names.to_vec()
        }
        None => kinds,
    };
    if rest.len() > 3 {
        return Err(usage.into());
    }
    Ok(Kinds {
        values: found,
        kinds,
        ordered,
    }
    .expr())
}

// ---- making tables ----

/// The options at the end of the arguments: a name in quotes and its value.
fn options<'a>(a: &'a [Expr], known: &[&str]) -> (&'a [Expr], Vec<(String, &'a Expr)>) {
    let mut end = a.len();
    let mut found = Vec::new();
    while end >= 2 {
        match text_of(&a[end - 2]).map(str::to_lowercase) {
            Some(name) if known.contains(&name.as_str()) => {
                found.push((name, &a[end - 1]));
                end -= 2;
            }
            _ => break,
        }
    }
    found.reverse();
    (&a[..end], found)
}

fn column_of(e: &Expr, who: &str) -> Result<Vec<Expr>, String> {
    if let Some(kinds) = read_kinds(e) {
        return Ok(kinds.texts());
    }
    e.as_list()
        .map(<[Expr]>::to_vec)
        .ok_or_else(|| format!("{who}: a variable is a list of values, {e} is none"))
}

fn build(timed: bool, a: &[Expr]) -> Res {
    let who = if timed { TIMED } else { TABLE };
    let (plain, chosen) = options(a, &["variablenames", "rownames", "rowtimes"]);
    let mut table = Table {
        timed,
        names: Vec::new(),
        columns: Vec::new(),
    };
    let mut plain = plain;
    let mut times: Option<Vec<Expr>> = None;
    let mut row_names: Option<Vec<Expr>> = None;
    let mut given: Option<Vec<Expr>> = None;
    for (name, value) in &chosen {
        match name.as_str() {
            "variablenames" => given = Some(list_of(value)),
            "rownames" => row_names = Some(column_of(value, who)?),
            _ => times = Some(column_of(value, who)?),
        }
    }
    let paired = given.is_none()
        && !plain.is_empty()
        && plain.len().is_multiple_of(2)
        && plain.iter().step_by(2).all(|n| text_of(n).is_some());
    if timed && times.is_none() && !paired {
        let (first, rest) = plain
            .split_first()
            .ok_or("timetable(times, \"name\", [values], ...)")?;
        times = Some(column_of(first, who)?);
        plain = rest;
    }
    let paired = given.is_none()
        && !plain.is_empty()
        && plain.len().is_multiple_of(2)
        && plain.iter().step_by(2).all(|n| text_of(n).is_some());
    if paired {
        for pair in plain.chunks(2) {
            table
                .names
                .push(text_of(&pair[0]).unwrap_or_default().to_string());
            table.columns.push(column_of(&pair[1], who)?);
        }
    } else {
        for (k, column) in plain.iter().enumerate() {
            table.names.push(format!("Var{}", k + 1));
            table.columns.push(column_of(column, who)?);
        }
        if let Some(names) = given {
            if names.len() != table.names.len() {
                return Err(format!(
                    "{who}: a name for every one of the {} variables",
                    table.names.len()
                ));
            }
            table.names = names
                .iter()
                .map(|n| {
                    text_of(n)
                        .map(str::to_string)
                        .ok_or_else(|| format!("{who}: the names are texts"))
                })
                .collect::<Result<_, _>>()?;
        }
    }
    if let Some(names) = row_names {
        table.names.insert(0, "Row".into());
        table.columns.insert(0, names);
    }
    if timed {
        let times = times.ok_or("timetable(times, \"name\", [values], ...)")?;
        if table.find(TIME).is_none() {
            table.names.insert(0, TIME.into());
            table.columns.insert(0, times);
        }
    } else if let Some(times) = times {
        table.names.insert(0, TIME.into());
        table.columns.insert(0, times);
        table.timed = true;
    }
    Ok(table.checked(who)?.expr())
}

pub(crate) fn make(a: &[Expr]) -> Res {
    build(false, a)
}

/// Whether what is written after `table(` makes a table of columns rather than a list of
/// values of an expression: names in quotes before lists of values, lists only, or the
/// option that names the variables.
pub(crate) fn written_as_table(raw: &[Expr]) -> bool {
    let data = |e: &Expr| match e {
        Expr::List(items) => {
            !matches!(items.first(), Some(Expr::Sym(s)) if !crate::expr::is_const_name(s))
        }
        other => other.fn_args(KINDS).is_some(),
    };
    let named = raw
        .iter()
        .any(|e| text_of(e).is_some_and(|t| t.eq_ignore_ascii_case("variablenames")));
    let paired = raw.len().is_multiple_of(2)
        && raw
            .chunks(2)
            .all(|p| text_of(&p[0]).is_some() && data(&p[1]));
    named || paired || (!raw.is_empty() && raw.iter().all(data))
}

/// The lines that show a table: the names, then a line for every row.
pub(crate) fn lines(e: &Expr) -> Option<Vec<String>> {
    let table = read(e)?;
    let cell = |x: &Expr| match text_of(x) {
        Some(t) => t.to_string(),
        None if x.is_undefined() => String::new(),
        None => format!("{x}"),
    };
    let mut cells: Vec<Vec<String>> = vec![table.names.clone()];
    cells.extend(table.rows().iter().map(|r| r.iter().map(cell).collect()));
    let widths: Vec<usize> = (0..table.names.len())
        .map(|j| {
            cells
                .iter()
                .map(|r| r[j].chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    Some(
        cells
            .iter()
            .map(|r| {
                r.iter()
                    .zip(&widths)
                    .map(|(c, w)| format!("{}{c}", " ".repeat(w - c.chars().count())))
                    .collect::<Vec<_>>()
                    .join("  ")
            })
            .collect(),
    )
}

/// The text of a table for a file: the names, then the rows, commas between the entries.
pub(crate) fn file_text(e: &Expr) -> Option<(String, usize, usize)> {
    let table = read(e)?;
    let cell = |x: &Expr| match text_of(x) {
        Some(t) if t.contains([',', '"', '\n']) => format!("\"{}\"", t.replace('"', "\"\"")),
        Some(t) => t.to_string(),
        None if x.is_undefined() => String::new(),
        None => format!("{x}"),
    };
    let mut out = table.names.join(",");
    out.push('\n');
    for row in table.rows() {
        out.push_str(&row.iter().map(cell).collect::<Vec<_>>().join(","));
        out.push('\n');
    }
    Some((out, table.height(), table.names.len()))
}

/// The keys by which two tables are joined: those asked for, else the names they share.
fn keys_of(
    left: &Table,
    right: &Table,
    chosen: &[(String, &Expr)],
    who: &str,
) -> Result<Vec<(usize, usize)>, String> {
    let names: Vec<String> = match chosen.iter().find(|(n, _)| n == "keys") {
        Some((_, k)) => list_of(k)
            .iter()
            .map(|n| {
                text_of(n)
                    .map(str::to_string)
                    .ok_or_else(|| format!("{who}: the keys are names in quotes"))
            })
            .collect::<Result<_, _>>()?,
        None => left
            .names
            .iter()
            .filter(|n| right.find(n).is_some())
            .cloned()
            .collect(),
    };
    if names.is_empty() {
        return Err(format!("{who}: the tables share no variable to join by"));
    }
    names
        .iter()
        .map(|n| match (left.find(n), right.find(n)) {
            (Some(l), Some(r)) => Ok((l, r)),
            _ => Err(format!("{who}: both tables need the variable \"{n}\"")),
        })
        .collect()
}

/// The rows of two tables side by side where their keys agree; with `left` / `right` the
/// rows without a partner stay, what is not there is missing.
fn joined(left: &Table, right: &Table, keys: &[(usize, usize)], keep: (bool, bool)) -> Table {
    let other_left: Vec<usize> = (0..left.names.len())
        .filter(|k| !keys.iter().any(|(l, _)| l == k))
        .collect();
    let other_right: Vec<usize> = (0..right.names.len())
        .filter(|k| !keys.iter().any(|(_, r)| r == k))
        .collect();
    let mut names: Vec<String> = keys.iter().map(|(l, _)| left.names[*l].clone()).collect();
    for k in &other_left {
        let name = &left.names[*k];
        names.push(match other_right.iter().any(|r| right.names[*r] == *name) {
            true => format!("{name}_left"),
            false => name.clone(),
        });
    }
    for k in &other_right {
        let name = &right.names[*k];
        names.push(match other_left.iter().any(|l| left.names[*l] == *name) {
            true => format!("{name}_right"),
            false => name.clone(),
        });
    }
    let key_left = |i: usize| -> Vec<Expr> {
        keys.iter()
            .map(|(l, _)| left.columns[*l][i].clone())
            .collect()
    };
    let key_right = |j: usize| -> Vec<Expr> {
        keys.iter()
            .map(|(_, r)| right.columns[*r][j].clone())
            .collect()
    };
    let mut rows: Vec<Vec<Expr>> = Vec::new();
    let mut met = vec![false; right.height()];
    for i in 0..left.height() {
        let key = key_left(i);
        let mut any = false;
        for (j, seen) in met.iter_mut().enumerate() {
            if same_row(&key, &key_right(j)) {
                (any, *seen) = (true, true);
                let mut row = key.clone();
                row.extend(other_left.iter().map(|k| left.columns[*k][i].clone()));
                row.extend(other_right.iter().map(|k| right.columns[*k][j].clone()));
                rows.push(row);
            }
        }
        if !any && keep.0 {
            let mut row = key;
            row.extend(other_left.iter().map(|k| left.columns[*k][i].clone()));
            row.extend(other_right.iter().map(|_| Expr::undefined()));
            rows.push(row);
        }
    }
    if keep.1 {
        for (j, _) in met.iter().enumerate().filter(|(_, seen)| !**seen) {
            let mut row = key_right(j);
            row.extend(other_left.iter().map(|_| Expr::undefined()));
            row.extend(other_right.iter().map(|k| right.columns[*k][j].clone()));
            rows.push(row);
        }
    }
    rows.sort_by(|a, b| compare_rows(&a[..keys.len()], &b[..keys.len()]));
    Table {
        timed: false,
        columns: (0..names.len())
            .map(|j| rows.iter().map(|r| r[j].clone()).collect())
            .collect(),
        names,
    }
}

/// The rows that a list of numbers from 1 or of truths chooses.
fn chosen_rows(e: &Expr, height: usize, who: &str) -> Result<Vec<usize>, String> {
    // (a condition written for a whole variable is a truth for every row)
    let items = list_of(&crate::arrays::threaded(e));
    let truths = items.len() == height
        && !items.is_empty()
        && items
            .iter()
            .all(|x| matches!(x, Expr::Bool(_) | Expr::Rel(..)));
    if truths {
        let mut rows = Vec::new();
        for (k, item) in items.iter().enumerate() {
            if truthy(item).map_err(|e| format!("{who}: {e}"))? {
                rows.push(k);
            }
        }
        return Ok(rows);
    }
    items
        .iter()
        .map(|x| {
            whole(x, "the number of a row", 1, height as i64)
                .map(|k| k as usize - 1)
                .map_err(|e| format!("{who}: {e}"))
        })
        .collect()
}

fn direction(e: &Expr, who: &str) -> Result<bool, String> {
    match word(e).as_deref() {
        Some("ascend") => Ok(false),
        Some("descend") => Ok(true),
        _ => Err(format!("{who}: the direction is \"ascend\" or \"descend\"")),
    }
}

impl Session {
    /// A method of a group as a number: the function of that name over the values.
    fn summed(&mut self, method: &str, values: &[Expr]) -> Res {
        let list = Expr::list(values.to_vec());
        match method {
            "numunique" => Ok(Expr::int(kinds_of(values).len() as i64)),
            "nummissing" => Ok(Expr::int(
                values.iter().filter(|v| v.is_undefined()).count() as i64,
            )),
            "nnz" => Ok(Expr::int(
                values
                    .iter()
                    .filter(|v| !v.is_zero() && !v.is_undefined())
                    .count() as i64,
            )),
            "count" => Ok(Expr::int(
                values.iter().filter(|v| !v.is_undefined()).count() as i64,
            )),
            "firstvalue" => Ok(values.first().cloned().unwrap_or_else(Expr::undefined)),
            "lastvalue" => Ok(values.last().cloned().unwrap_or_else(Expr::undefined)),
            "sum" | "mean" | "median" | "mode" | "min" | "max" | "std" | "var" | "range"
            | "prod" => {
                let present: Vec<Expr> = values
                    .iter()
                    .filter(|v| !v.is_undefined())
                    .cloned()
                    .collect();
                match present.is_empty() {
                    true => Ok(Expr::undefined()),
                    false => crate::datafun::stat(method, &present),
                }
            }
            _ => match crate::names::function(method) {
                Some(f) => self.call(f.name, &[list]),
                None => Err(format!(
                    "\"{method}\" is no method: sum mean median mode min max std var range prod count numunique nummissing firstvalue lastvalue, or the name of a function of a list"
                )),
            },
        }
    }

    /// The arguments of `table(...)` when they make a table of columns rather than the
    /// values of an expression: told from how they are written, or from what they hold.
    pub(crate) fn table_args(&mut self, raw: &[Expr]) -> Option<Result<Vec<Expr>, String>> {
        if written_as_table(raw) {
            return Some(self.eval_args(raw));
        }
        let iterator = |e: &Expr| {
            matches!(e, Expr::List(items)
                if matches!(items.first(), Some(Expr::Sym(s)) if !crate::expr::is_const_name(s)))
        };
        if raw.is_empty() || raw.iter().any(iterator) {
            return None;
        }
        // table(body, k, from, to[, step]) is a loop: its body is not worked out before it runs
        if (3..=5).contains(&raw.len())
            && matches!(&raw[1], Expr::Sym(s) if !crate::expr::is_const_name(s))
            && raw[2..].iter().all(|e| {
                self.eval(e)
                    .is_ok_and(|v| v.as_list().is_none() && text_of(&v).is_none())
            })
        {
            return None;
        }
        let paired =
            raw.len().is_multiple_of(2) && raw.iter().step_by(2).all(|n| text_of(n).is_some());
        if paired && raw.len() >= 4 {
            return Some(self.eval_args(raw));
        }
        // (what a trial of the arguments has to tell is told only when they are columns)
        let told = self.notes.len();
        let Ok(values) = self.eval_args(raw) else {
            self.notes.truncate(told);
            return None;
        };
        let column = |e: &Expr| e.as_list().is_some() || e.fn_args(KINDS).is_some();
        let long = |e: &Expr| match read_kinds(e) {
            Some(kinds) => kinds.values.len(),
            None => e.as_list().map_or(0, <[Expr]>::len),
        };
        let data = match paired {
            true => column(&values[1]),
            false => {
                values.iter().all(column) && values.iter().all(|v| long(v) == long(&values[0]))
            }
        };
        if !data {
            self.notes.truncate(told);
        }
        data.then_some(Ok(values))
    }

    /// The table of a file: names in the first line, then a line for every row.
    fn read_table(&mut self, file: &Expr) -> Res {
        let who = "readtable";
        let path = text_of(file).ok_or("readtable(\"file.csv\")")?;
        crate::sandbox::allowed(path).map_err(|e| format!("{who}: {e}"))?;
        let content = std::fs::read_to_string(path).map_err(|e| format!("{who}: {path}: {e}"))?;
        let split = |line: &str| -> Vec<String> {
            let mut cells = Vec::new();
            let (mut cell, mut quoted) = (String::new(), false);
            let mut signs = line.chars().peekable();
            while let Some(c) = signs.next() {
                match c {
                    '"' if quoted && signs.peek() == Some(&'"') => {
                        cell.push('"');
                        signs.next();
                    }
                    '"' => quoted = !quoted,
                    ',' | ';' | '\t' if !quoted => cells.push(std::mem::take(&mut cell)),
                    other => cell.push(other),
                }
            }
            cells.push(cell);
            cells.iter().map(|c| c.trim().to_string()).collect()
        };
        let rows: Vec<Vec<String>> = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(split)
            .collect();
        let Some(first) = rows.first() else {
            return Ok(Expr::call(TABLE, Vec::new()));
        };
        if rows.iter().any(|r| r.len() != first.len()) {
            return Err(format!(
                "{who}: {path}: the lines have different numbers of entries"
            ));
        }
        let mut value = |cell: &str| -> Expr {
            if cell.is_empty() {
                return Expr::undefined();
            }
            let plain = cell
                .chars()
                .all(|c| c.is_ascii_digit() || "+-.eE/".contains(c))
                && cell.chars().any(|c| c.is_ascii_digit());
            let parsed = match plain {
                true => crate::parser::parse(cell).ok(),
                false => None,
            };
            match parsed
                .as_ref()
                .and_then(|p| p.stmts.first().map(|s| (s, p.decimal)))
            {
                Some((stmt, decimal)) => match self.eval(stmt) {
                    Ok(v) if is_number(&v) => match decimal {
                        true => crate::numeric::n(&v, crate::numeric::digits()),
                        false => v,
                    },
                    _ => text(cell),
                },
                None => text(cell),
            }
        };
        let head: Vec<Expr> = first.iter().map(|c| value(c)).collect();
        let named = head.iter().all(|h| text_of(h).is_some());
        let names: Vec<String> = match named {
            true => first.clone(),
            false => (1..=first.len()).map(|k| format!("Var{k}")).collect(),
        };
        let body = &rows[usize::from(named)..];
        let columns = (0..names.len())
            .map(|j| body.iter().map(|r| value(&r[j])).collect())
            .collect();
        Ok(Table {
            timed: false,
            names,
            columns,
        }
        .checked(who)?
        .expr())
    }

    /// A table with a time for every row at other times: values carried, drawn between
    /// the rows, or gathered over the stretch from a time to the next.
    fn retime(&mut self, table: &Table, times: &[Expr], method: &str) -> Result<Table, String> {
        let who = "retime";
        let old = &table.columns[0];
        let mut out = Table {
            timed: true,
            names: table.names.clone(),
            columns: vec![times.to_vec()],
        };
        let at =
            |t: &Expr| -> Vec<usize> { (0..old.len()).filter(|k| same(&old[*k], t)).collect() };
        for column in &table.columns[1..] {
            let mut values = Vec::with_capacity(times.len());
            for (k, t) in times.iter().enumerate() {
                let here = at(t);
                let value = match method {
                    "fillwithmissing" => here
                        .first()
                        .map_or_else(Expr::undefined, |i| column[*i].clone()),
                    "previous" => (0..old.len())
                        .filter(|i| compare(&old[*i], t) != Ordering::Greater)
                        .max_by(|a, b| compare(&old[*a], &old[*b]))
                        .map_or_else(Expr::undefined, |i| column[i].clone()),
                    "next" => (0..old.len())
                        .filter(|i| compare(&old[*i], t) != Ordering::Less)
                        .min_by(|a, b| compare(&old[*a], &old[*b]))
                        .map_or_else(Expr::undefined, |i| column[i].clone()),
                    "nearest" | "linear" | "spline" | "pchip" | "makima" => {
                        let known: Vec<usize> =
                            (0..old.len()).filter(|i| is_number(&column[*i])).collect();
                        let inside = known
                            .iter()
                            .any(|i| compare(&old[*i], t) != Ordering::Greater)
                            && known.iter().any(|i| compare(&old[*i], t) != Ordering::Less);
                        match (known.len() >= 2, inside) {
                            (true, true) => self.call(
                                "interp1",
                                &[
                                    Expr::list(known.iter().map(|i| old[*i].clone()).collect()),
                                    Expr::list(known.iter().map(|i| column[*i].clone()).collect()),
                                    t.clone(),
                                    text(method),
                                ],
                            )?,
                            _ => here
                                .first()
                                .map_or_else(Expr::undefined, |i| column[*i].clone()),
                        }
                    }
                    gathered => {
                        // from this time to the next one (the last stretch has no end)
                        let within: Vec<Expr> = (0..old.len())
                            .filter(|i| {
                                compare(&old[*i], t) != Ordering::Less
                                    && times.get(k + 1).is_none_or(|next| {
                                        compare(&old[*i], next) == Ordering::Less
                                    })
                            })
                            .map(|i| column[i].clone())
                            .collect();
                        match within.is_empty() {
                            true if gathered == "count" => Expr::zero(),
                            true => Expr::undefined(),
                            false => self
                                .summed(gathered, &within)
                                .map_err(|e| format!("{who}: {e}"))?,
                        }
                    }
                };
                values.push(value);
            }
            out.columns.push(values);
        }
        Ok(out)
    }

    pub(crate) fn table_call(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        let n = a.len();
        let first_table = a.first().and_then(read);
        let first_kinds = a.first().and_then(read_kinds);
        // names that other values have too go on to them unless a table or kinds come first
        let shared = matches!(
            name,
            "size"
                | "numel"
                | "length"
                | "sortrows"
                | "unique"
                | "vertcat"
                | "horzcat"
                | "ismember"
                | "string"
                | "double"
                | "sort"
                | "rmmissing"
                | "ismissing"
                | "getfield"
                | "setfield"
                | "fieldnames"
                | "isfield"
                | "class"
                | "histcounts"
                | "join"
                | "isempty"
                | "mode"
        );
        if shared && first_table.is_none() && first_kinds.is_none() {
            return None;
        }
        Some((|| -> Res {
            match (name, n) {
                ("table", _) => return build(false, a),
                ("timetable", 1..) => return build(true, a),
                ("categorical", 1..) => return make_kinds(a),
                _ => {}
            }
            if let Some(kinds) = &first_kinds {
                let who = name;
                let found = match (name, n) {
                    ("class", 1) => text(KINDS),
                    ("size", 1) => {
                        Expr::list(vec![Expr::one(), Expr::int(kinds.values.len() as i64)])
                    }
                    ("numel" | "length", 1) => Expr::int(kinds.values.len() as i64),
                    ("isempty", 1) => Expr::Bool(kinds.values.is_empty()),
                    ("string" | "cellstr", 1) => Expr::list(kinds.texts()),
                    ("double", 1) => Expr::list(
                        kinds
                            .values
                            .iter()
                            .map(|v| v.map_or_else(Expr::undefined, |k| Expr::int(k as i64 + 1)))
                            .collect(),
                    ),
                    ("categories", 1) => Expr::list(kinds.kinds.clone()),
                    ("countcats" | "histcounts", 1) => Expr::list(
                        (0..kinds.kinds.len())
                            .map(|k| {
                                Expr::int(
                                    kinds.values.iter().filter(|v| **v == Some(k)).count() as i64
                                )
                            })
                            .collect(),
                    ),
                    ("mode", 1) => {
                        let counts: Vec<usize> = (0..kinds.kinds.len())
                            .map(|k| kinds.values.iter().filter(|v| **v == Some(k)).count())
                            .collect();
                        let most = counts.iter().max().copied().unwrap_or(0);
                        match counts.iter().position(|c| *c == most && most > 0) {
                            Some(k) => kinds.kinds[k].clone(),
                            None => Expr::undefined(),
                        }
                    }
                    ("isundefined" | "ismissing", 1) => Expr::list(
                        kinds
                            .values
                            .iter()
                            .map(|v| Expr::Bool(v.is_none()))
                            .collect(),
                    ),
                    ("isordinal", 1) => Expr::Bool(kinds.ordered),
                    ("iscategory", 2) => {
                        let asked: Vec<Expr> = list_of(&a[1])
                            .iter()
                            .map(|k| Expr::Bool(kinds.place(k).is_some()))
                            .collect();
                        match a[1].as_list() {
                            Some(_) => Expr::list(asked),
                            None => asked[0].clone(),
                        }
                    }
                    ("ismember", 2) => {
                        let wanted = list_of(&a[1]);
                        Expr::list(
                            kinds
                                .values
                                .iter()
                                .map(|v| {
                                    Expr::Bool(v.is_some_and(|k| {
                                        wanted.iter().any(|w| same(w, &kinds.kinds[k]))
                                    }))
                                })
                                .collect(),
                        )
                    }
                    ("unique", 1) => {
                        let mut used: Vec<usize> = kinds.values.iter().flatten().copied().collect();
                        used.sort_unstable();
                        used.dedup();
                        Kinds {
                            values: used.into_iter().map(Some).collect(),
                            ..kinds.clone()
                        }
                        .expr()
                    }
                    ("sort", 1 | 2) => {
                        let falling = match a.get(1) {
                            Some(d) => direction(d, who)?,
                            None => false,
                        };
                        let mut values = kinds.values.clone();
                        values.sort_by(|x, y| match (x, y) {
                            (Some(p), Some(q)) if falling => q.cmp(p),
                            (Some(p), Some(q)) => p.cmp(q),
                            (Some(_), None) => Ordering::Less,
                            (None, Some(_)) => Ordering::Greater,
                            (None, None) => Ordering::Equal,
                        });
                        Kinds {
                            values,
                            ..kinds.clone()
                        }
                        .expr()
                    }
                    ("takerows", 2) => {
                        let rows = chosen_rows(&a[1], kinds.values.len(), who)?;
                        Kinds {
                            values: rows.iter().map(|k| kinds.values[*k]).collect(),
                            ..kinds.clone()
                        }
                        .expr()
                    }
                    ("addcats", 2) => {
                        let mut out = kinds.clone();
                        for kind in list_of(&a[1]) {
                            if out.place(&kind).is_some() {
                                return Err(format!("addcats: the kind {kind} is there already"));
                            }
                            out.kinds.push(kind);
                        }
                        out.expr()
                    }
                    ("removecats", 1 | 2) => {
                        let gone: Vec<usize> = match a.get(1) {
                            Some(names) => list_of(names)
                                .iter()
                                .filter_map(|k| kinds.place(k))
                                .collect(),
                            None => (0..kinds.kinds.len())
                                .filter(|k| !kinds.values.contains(&Some(*k)))
                                .collect(),
                        };
                        let stays: Vec<usize> = (0..kinds.kinds.len())
                            .filter(|k| !gone.contains(k))
                            .collect();
                        Kinds {
                            values: kinds
                                .values
                                .iter()
                                .map(|v| v.and_then(|k| stays.iter().position(|s| *s == k)))
                                .collect(),
                            kinds: stays.iter().map(|k| kinds.kinds[*k].clone()).collect(),
                            ordered: kinds.ordered,
                        }
                        .expr()
                    }
                    ("renamecats", 2 | 3) => {
                        let mut out = kinds.clone();
                        let (old, new) = match n {
                            2 => ((0..kinds.kinds.len()).collect::<Vec<_>>(), list_of(&a[1])),
                            _ => (
                                list_of(&a[1])
                                    .iter()
                                    .map(|k| {
                                        kinds
                                            .place(k)
                                            .ok_or_else(|| format!("renamecats: no kind {k}"))
                                    })
                                    .collect::<Result<_, _>>()?,
                                list_of(&a[2]),
                            ),
                        };
                        if old.len() != new.len() {
                            return Err(
                                "renamecats: a new name for every kind that is renamed".into()
                            );
                        }
                        for (k, name) in old.iter().zip(new) {
                            out.kinds[*k] = name;
                        }
                        for (k, kind) in out.kinds.iter().enumerate() {
                            if out.kinds[..k].iter().any(|other| same(other, kind)) {
                                return Err(format!("renamecats: the kind {kind} comes twice"));
                            }
                        }
                        out.expr()
                    }
                    ("reordercats", 1 | 2) => {
                        let order: Vec<usize> = match a.get(1) {
                            Some(names) => {
                                let order: Vec<usize> = list_of(names)
                                    .iter()
                                    .map(|k| {
                                        kinds
                                            .place(k)
                                            .ok_or_else(|| format!("reordercats: no kind {k}"))
                                    })
                                    .collect::<Result<_, _>>()?;
                                let mut all = order.clone();
                                all.sort_unstable();
                                all.dedup();
                                if all.len() != kinds.kinds.len()
                                    || order.len() != kinds.kinds.len()
                                {
                                    return Err(
                                        "reordercats: every kind once in the new order".into()
                                    );
                                }
                                order
                            }
                            None => {
                                let mut order: Vec<usize> = (0..kinds.kinds.len()).collect();
                                order.sort_by(|x, y| compare(&kinds.kinds[*x], &kinds.kinds[*y]));
                                order
                            }
                        };
                        Kinds {
                            values: kinds
                                .values
                                .iter()
                                .map(|v| v.and_then(|k| order.iter().position(|o| *o == k)))
                                .collect(),
                            kinds: order.iter().map(|k| kinds.kinds[*k].clone()).collect(),
                            ordered: kinds.ordered,
                        }
                        .expr()
                    }
                    ("mergecats", 2 | 3) => {
                        let old: Vec<usize> = list_of(&a[1])
                            .iter()
                            .map(|k| {
                                kinds
                                    .place(k)
                                    .ok_or_else(|| format!("mergecats: no kind {k}"))
                            })
                            .collect::<Result<_, _>>()?;
                        let Some(first) = old.iter().min().copied() else {
                            return Err("mergecats(values, [kinds][, new name])".into());
                        };
                        let name = a
                            .get(2)
                            .cloned()
                            .unwrap_or_else(|| kinds.kinds[old[0]].clone());
                        let stays: Vec<usize> = (0..kinds.kinds.len())
                            .filter(|k| *k == first || !old.contains(k))
                            .collect();
                        let place = |k: usize| {
                            let k = if old.contains(&k) { first } else { k };
                            stays.iter().position(|s| *s == k)
                        };
                        Kinds {
                            values: kinds.values.iter().map(|v| v.and_then(place)).collect(),
                            kinds: stays
                                .iter()
                                .map(|k| {
                                    if *k == first {
                                        name.clone()
                                    } else {
                                        kinds.kinds[*k].clone()
                                    }
                                })
                                .collect(),
                            ordered: kinds.ordered,
                        }
                        .expr()
                    }
                    ("setcats", 2) => {
                        let new = list_of(&a[1]);
                        Kinds {
                            values: kinds
                                .values
                                .iter()
                                .map(|v| {
                                    v.and_then(|k| {
                                        new.iter().position(|c| same(c, &kinds.kinds[k]))
                                    })
                                })
                                .collect(),
                            kinds: new,
                            ordered: kinds.ordered,
                        }
                        .expr()
                    }
                    ("grp2idx", 1) => Expr::list(vec![
                        Expr::list(
                            kinds
                                .values
                                .iter()
                                .map(|v| {
                                    v.map_or_else(Expr::undefined, |k| Expr::int(k as i64 + 1))
                                })
                                .collect(),
                        ),
                        Expr::list(kinds.kinds.clone()),
                    ]),
                    ("summary", 1) => Table {
                        timed: false,
                        names: vec!["Category".into(), "Count".into()],
                        columns: vec![
                            kinds.kinds.clone(),
                            (0..kinds.kinds.len())
                                .map(|k| {
                                    Expr::int(
                                        kinds.values.iter().filter(|v| **v == Some(k)).count()
                                            as i64,
                                    )
                                })
                                .collect(),
                        ],
                    }
                    .expr(),
                    ("head" | "tail", 1 | 2) => {
                        let count = match a.get(1) {
                            Some(k) => whole(k, "the number of values", 0, i64::MAX)? as usize,
                            None => 8,
                        }
                        .min(kinds.values.len());
                        let values = match name {
                            "head" => kinds.values[..count].to_vec(),
                            _ => kinds.values[kinds.values.len() - count..].to_vec(),
                        };
                        Kinds {
                            values,
                            ..kinds.clone()
                        }
                        .expr()
                    }
                    ("iscategorical", 1) => Expr::Bool(true),
                    ("istable" | "istimetable", 1) => Expr::Bool(false),
                    _ => return Err(String::new()),
                };
                return Ok(found);
            }
            let who = name;
            Ok(match (name, n) {
                ("istable", 1) => Expr::Bool(first_table.as_ref().is_some_and(|t| !t.timed)),
                ("istimetable", 1) => Expr::Bool(first_table.as_ref().is_some_and(|t| t.timed)),
                ("iscategorical", 1) => Expr::Bool(false),
                ("grp2idx", 1) => {
                    let values = a[0].as_list().ok_or("grp2idx([values])")?;
                    let kinds = kinds_of(values);
                    Expr::list(vec![
                        Expr::list(
                            values
                                .iter()
                                .map(|v| {
                                    kinds
                                        .iter()
                                        .position(|k| same(k, v))
                                        .map_or_else(Expr::undefined, |k| Expr::int(k as i64 + 1))
                                })
                                .collect(),
                        ),
                        Expr::list(kinds),
                    ])
                }
                ("array2table" | "cell2table", 1 | 3) => {
                    let rows = crate::matrix::rows(&a[0]).ok_or_else(|| {
                        format!("{who} needs a matrix: a list of rows of one length")
                    })?;
                    let columns: Vec<Expr> = (0..rows.first().map_or(0, Vec::len))
                        .map(|j| Expr::list(rows.iter().map(|r| r[j].clone()).collect()))
                        .collect();
                    let mut args = columns;
                    args.extend(a[1..].iter().cloned());
                    build(false, &args)?
                }
                ("struct2table", 1) => {
                    let fields =
                        |s: &mut Session, e: &Expr| -> Result<Vec<(String, Expr)>, String> {
                            let names = s.call("fieldnames", std::slice::from_ref(e))?;
                            list_of(&names)
                                .iter()
                                .map(|f| {
                                    let value = s.call("getfield", &[e.clone(), f.clone()])?;
                                    Ok((text_of(f).unwrap_or_default().to_string(), value))
                                })
                                .collect()
                        };
                    match a[0].as_list() {
                        // a list of records: a row for each
                        Some(records) if !records.is_empty() => {
                            let mut table = Table {
                                timed: false,
                                names: Vec::new(),
                                columns: Vec::new(),
                            };
                            for (k, record) in records.iter().enumerate() {
                                let all = fields(self, record)?;
                                if k == 0 {
                                    table.names = all.iter().map(|(n, _)| n.clone()).collect();
                                    table.columns = vec![Vec::new(); all.len()];
                                }
                                if all.len() != table.names.len()
                                    || all.iter().zip(&table.names).any(|((n, _), m)| n != m)
                                {
                                    return Err(
                                        "struct2table: the records have the same fields".into()
                                    );
                                }
                                for (column, (_, value)) in table.columns.iter_mut().zip(all) {
                                    column.push(value);
                                }
                            }
                            table.checked(who)?.expr()
                        }
                        _ => {
                            let all = fields(self, &a[0])?;
                            Table {
                                timed: false,
                                names: all.iter().map(|(n, _)| n.clone()).collect(),
                                columns: all.iter().map(|(_, v)| list_of(v)).collect(),
                            }
                            .checked(who)?
                            .expr()
                        }
                    }
                }
                ("readtable", 1) => self.read_table(&a[0])?,
                ("head" | "tail", 1 | 2) if first_table.is_none() => {
                    let items = a[0]
                        .as_list()
                        .ok_or_else(|| format!("{who}(list or table[, how many])"))?;
                    let count = match a.get(1) {
                        Some(k) => whole(k, "the number of entries", 0, i64::MAX)? as usize,
                        None => 8,
                    }
                    .min(items.len());
                    Expr::list(match name {
                        "head" => items[..count].to_vec(),
                        _ => items[items.len() - count..].to_vec(),
                    })
                }
                ("height" | "width", 1) if first_table.is_none() => {
                    let size = crate::arrays::size(&a[0]);
                    Expr::int(match name {
                        "height" => size.first().copied().unwrap_or(1),
                        _ => size.get(1).copied().unwrap_or(1),
                    } as i64)
                }
                ("isregular", 1) if first_table.is_none() => {
                    let times = list_of(&a[0]);
                    let steps: Vec<Expr> = times
                        .windows(2)
                        .map(|w| tidy(sub(w[1].clone(), w[0].clone())))
                        .collect();
                    Expr::Bool(steps.windows(2).all(|w| crate::arrays::same(&w[0], &w[1])))
                }
                _ => {
                    let Some(table) = first_table else {
                        return Err(String::new());
                    };
                    self.on_table(name, table, a)?
                }
            })
        })())
        .filter(|r| !matches!(r, Err(e) if e.is_empty()))
    }

    /// What is done with a table that comes first among the arguments.
    fn on_table(&mut self, name: &str, table: Table, a: &[Expr]) -> Res {
        let n = a.len();
        let who = name;
        let height = table.height();
        let data_from = usize::from(table.timed);
        Ok(match (name, n) {
            ("class", 1) => text(if table.timed { TIMED } else { TABLE }),
            ("height", 1) => Expr::int(height as i64),
            ("width", 1) => Expr::int((table.names.len() - data_from) as i64),
            ("size", 1) => Expr::list(vec![
                Expr::int(height as i64),
                Expr::int((table.names.len() - data_from) as i64),
            ]),
            ("numel", 1) => Expr::int((height * (table.names.len() - data_from)) as i64),
            ("isempty", 1) => Expr::Bool(height == 0 || table.names.len() == data_from),
            ("varnames" | "fieldnames", 1) => texts(&table.names[data_from..]),
            ("isfield", 2) => {
                let asked: Vec<Expr> = list_of(&a[1])
                    .iter()
                    .map(|f| Expr::Bool(text_of(f).is_some_and(|f| table.find(f).is_some())))
                    .collect();
                match a[1].as_list() {
                    Some(_) => Expr::list(asked),
                    None => asked[0].clone(),
                }
            }
            ("getfield", 2) => Expr::list(table.columns[table.column(&a[1], who)?].clone()),
            ("setfield" | "addvars", 3) if text_of(&a[1]).is_some() => {
                let mut out = table.clone();
                let column = column_of(&a[2], who)?;
                let field = text_of(&a[1]).unwrap_or_default().to_string();
                match (out.find(&field), name) {
                    (Some(k), "setfield") => out.columns[k] = column,
                    _ => {
                        out.names.push(field);
                        out.columns.push(column);
                    }
                }
                out.checked(who)?.expr()
            }
            ("addvars", 2..) => {
                let (plain, chosen) = options(&a[1..], &["newvariablenames", "before", "after"]);
                let mut out = table.clone();
                let mut at = out.names.len();
                let mut names: Vec<String> = (0..plain.len())
                    .map(|k| format!("Var{}", out.names.len() - data_from + k + 1))
                    .collect();
                for (option, value) in &chosen {
                    match option.as_str() {
                        "newvariablenames" => {
                            names = list_of(value)
                                .iter()
                                .map(|v| {
                                    text_of(v)
                                        .map(str::to_string)
                                        .ok_or("addvars: the names are texts")
                                })
                                .collect::<Result<_, _>>()?;
                        }
                        "before" => at = table.column(value, who)?,
                        _ => at = table.column(value, who)? + 1,
                    }
                }
                if names.len() != plain.len() || plain.is_empty() {
                    return Err("addvars(table, \"name\", [values]) or addvars(table, [values], ..., \"NewVariableNames\", [names])".into());
                }
                for (k, (name, column)) in names.into_iter().zip(plain).enumerate() {
                    out.names.insert(at + k, name);
                    out.columns.insert(at + k, column_of(column, who)?);
                }
                out.checked(who)?.expr()
            }
            ("removevars", 2) => {
                let gone = table.columns_of(&a[1], who)?;
                let stay: Vec<usize> = (0..table.names.len())
                    .filter(|k| !gone.contains(k))
                    .collect();
                table.only(&stay).expr()
            }
            ("takevars", 2) => {
                let mut chosen = table.columns_of(&a[1], who)?;
                if table.timed && !chosen.contains(&0) {
                    chosen.insert(0, 0);
                }
                table.only(&chosen).checked(who)?.expr()
            }
            ("renamevars", 3) => {
                let (old, new) = (table.columns_of(&a[1], who)?, list_of(&a[2]));
                if old.len() != new.len() {
                    return Err("renamevars(table, [old names], [new names])".into());
                }
                let mut out = table.clone();
                for (k, name) in old.iter().zip(&new) {
                    out.names[*k] = text_of(name)
                        .ok_or("renamevars: the names are texts")?
                        .to_string();
                }
                out.checked(who)?.expr()
            }
            ("movevars", 4) => {
                let moved = table.columns_of(&a[1], who)?;
                let target = table.column(&a[3], who)?;
                let after = match word(&a[2]).as_deref() {
                    Some("before") => false,
                    Some("after") => true,
                    _ => {
                        return Err("movevars(table, [names], \"Before\" | \"After\", name)".into());
                    }
                };
                if moved.contains(&target) {
                    return Err("movevars: the variable to move by stays where it is".into());
                }
                let mut order: Vec<usize> = (0..table.names.len())
                    .filter(|k| !moved.contains(k))
                    .collect();
                let at = order.iter().position(|k| *k == target).unwrap_or(0) + usize::from(after);
                for (k, column) in moved.iter().enumerate() {
                    order.insert(at + k, *column);
                }
                Table {
                    timed: table.timed && order[0] == 0,
                    ..table.only(&order)
                }
                .expr()
            }
            ("takerows", 2) => table.take(&chosen_rows(&a[1], height, who)?).expr(),
            ("head" | "tail", 1 | 2) => {
                let count = match a.get(1) {
                    Some(k) => whole(k, "the number of rows", 0, i64::MAX)? as usize,
                    None => 8,
                }
                .min(height);
                let rows: Vec<usize> = match name {
                    "head" => (0..count).collect(),
                    _ => (height - count..height).collect(),
                };
                table.take(&rows).expr()
            }
            ("sortrows" | "topkrows", _) if (1..=4).contains(&n) => {
                let mut rest = &a[1..];
                let count = match name {
                    "topkrows" => {
                        let (k, more) = rest
                            .split_first()
                            .ok_or("topkrows(table, how many[, names][, direction])")?;
                        rest = more;
                        Some(whole(k, "the number of rows", 0, i64::MAX)? as usize)
                    }
                    _ => None,
                };
                let (columns, directions): (Vec<usize>, Vec<Expr>) = match rest {
                    [] => ((0..table.names.len()).collect(), Vec::new()),
                    [d] if word(d).is_some_and(|w| w == "ascend" || w == "descend")
                        && table.find(text_of(d).unwrap_or("")).is_none() =>
                    {
                        ((0..table.names.len()).collect(), vec![d.clone()])
                    }
                    [names] => (table.columns_of(names, who)?, Vec::new()),
                    [names, d] => (table.columns_of(names, who)?, list_of(d)),
                    _ => return Err(format!("{who}(table[, names][, \"ascend\" | \"descend\"])")),
                };
                // topkrows takes the largest unless told otherwise
                let usual = name == "topkrows";
                let keys: Vec<(usize, bool)> = columns
                    .iter()
                    .enumerate()
                    .map(|(k, c)| {
                        let falling = match directions
                            .get(k)
                            .or(directions.first().filter(|_| directions.len() == 1))
                        {
                            Some(d) => direction(d, who)?,
                            None => usual,
                        };
                        Ok((*c, falling))
                    })
                    .collect::<Result<_, String>>()?;
                let mut order = table.sorted(&keys);
                if let Some(k) = count {
                    order.truncate(k);
                }
                table.take(&order).expr()
            }
            ("unique", 1) => {
                let mut rows = table.rows();
                rows.sort_by(|x, y| compare_rows(x, y));
                rows.dedup_by(|x, y| same_row(x, y));
                table.with_rows(&rows).expr()
            }
            ("ismissing", 1) => Expr::list(
                table
                    .rows()
                    .iter()
                    .map(|r| {
                        Expr::list(
                            r[data_from..]
                                .iter()
                                .map(|x| Expr::Bool(x.is_undefined()))
                                .collect(),
                        )
                    })
                    .collect(),
            ),
            ("rmmissing", 1) => {
                let rows: Vec<usize> = (0..height)
                    .filter(|k| !table.columns.iter().any(|c| c[*k].is_undefined()))
                    .collect();
                table.take(&rows).expr()
            }
            ("table2array" | "table2cell", 1) => Expr::list(
                table
                    .rows()
                    .iter()
                    .map(|r| Expr::list(r[data_from..].to_vec()))
                    .collect(),
            ),
            ("table2struct", 1 | 3) => {
                let whole_columns = match a.get(1).and_then(word) {
                    Some(w) if w == "toscalar" => truthy(&a[2])?,
                    Some(_) => return Err("table2struct(table[, \"ToScalar\", true])".into()),
                    None => false,
                };
                let record = |values: Vec<Expr>| -> Expr {
                    Expr::call(
                        "struct",
                        table
                            .names
                            .iter()
                            .zip(values)
                            .flat_map(|(n, v)| [text(n), v])
                            .collect(),
                    )
                };
                match whole_columns {
                    true => record(
                        table
                            .columns
                            .iter()
                            .map(|c| Expr::list(c.clone()))
                            .collect(),
                    ),
                    false => Expr::list(table.rows().into_iter().map(record).collect()),
                }
            }
            ("timetable2table", 1) => Table {
                timed: false,
                ..table
            }
            .expr(),
            ("table2timetable", 1 | 3) => {
                let mut out = table.clone();
                match a.get(1).and_then(word).as_deref() {
                    Some("rowtimes") => match text_of(&a[2]) {
                        Some(_) => {
                            let k = table.column(&a[2], who)?;
                            let (name, column) = (out.names.remove(k), out.columns.remove(k));
                            out.names.insert(0, name);
                            out.columns.insert(0, column);
                        }
                        None => {
                            out.names.insert(0, TIME.into());
                            out.columns.insert(0, column_of(&a[2], who)?);
                        }
                    },
                    Some(_) => {
                        return Err("table2timetable(table[, \"RowTimes\", name or times])".into());
                    }
                    None => {}
                }
                out.timed = true;
                out.checked(who)?.expr()
            }
            ("vertcat", 2..) => {
                let mut out = table.clone();
                for more in &a[1..] {
                    let more = need(more, who)?;
                    if more.names != out.names {
                        return Err(
                            "vertcat: the tables have the same variables in the same order".into(),
                        );
                    }
                    for (column, added) in out.columns.iter_mut().zip(more.columns) {
                        column.extend(added);
                    }
                }
                out.expr()
            }
            ("horzcat", 2..) => {
                let mut out = table.clone();
                for more in &a[1..] {
                    let more = need(more, who)?;
                    out.names.extend(more.names);
                    out.columns.extend(more.columns);
                }
                out.checked(who)?.expr()
            }
            ("innerjoin" | "outerjoin" | "join", 2..) => {
                let (plain, chosen) = options(&a[1..], &["keys", "type"]);
                let [other] = plain else {
                    return Err(format!("{who}(left, right[, \"Keys\", names])"));
                };
                let right = need(other, who)?;
                let keys = keys_of(&table, &right, &chosen, who)?;
                let kind = chosen
                    .iter()
                    .find(|(n, _)| n == "type")
                    .map(|(_, v)| word(v).unwrap_or_default());
                let keep = match (name, kind.as_deref()) {
                    ("outerjoin", None | Some("full")) => (true, true),
                    ("outerjoin", Some("left")) => (true, false),
                    ("outerjoin", Some("right")) => (false, true),
                    ("outerjoin", _) => {
                        return Err("outerjoin: the type is \"full\", \"left\" or \"right\"".into());
                    }
                    _ => (false, false),
                };
                if name == "join" {
                    // every row of the left table finds one row of the right one
                    let key = |t: &Table, i: usize, right: bool| -> Vec<Expr> {
                        keys.iter()
                            .map(|(l, r)| t.columns[if right { *r } else { *l }][i].clone())
                            .collect()
                    };
                    let others: Vec<usize> = (0..right.names.len())
                        .filter(|k| !keys.iter().any(|(_, r)| r == k))
                        .collect();
                    let mut out = table.clone();
                    for k in &others {
                        let name = &right.names[*k];
                        out.names.push(match table.find(name) {
                            Some(_) => format!("{name}_right"),
                            None => name.clone(),
                        });
                        out.columns.push(Vec::with_capacity(height));
                    }
                    for i in 0..height {
                        let wanted = key(&table, i, false);
                        let partners: Vec<usize> = (0..right.height())
                            .filter(|j| same_row(&wanted, &key(&right, *j, true)))
                            .collect();
                        let [j] = partners[..] else {
                            return Err("join: every row of the left table needs one row of the right table with its key (innerjoin and outerjoin take what there is)".into());
                        };
                        for (slot, k) in others.iter().enumerate() {
                            out.columns[table.names.len() + slot]
                                .push(right.columns[*k][j].clone());
                        }
                    }
                    out.checked(who)?.expr()
                } else {
                    joined(&table, &right, &keys, keep).checked(who)?.expr()
                }
            }
            ("varfun", 2) => return Err("varfun(f, table): the function comes first".into()),
            ("summary", 1) => {
                let mut out = Table {
                    timed: false,
                    names: ["Variable", "Type", "Min", "Median", "Max", "NumMissing"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    columns: vec![Vec::new(); 6],
                };
                for (name, column) in table.names.iter().zip(&table.columns) {
                    let present: Vec<Expr> = column
                        .iter()
                        .filter(|v| !v.is_undefined())
                        .cloned()
                        .collect();
                    let numbers = !present.is_empty() && present.iter().all(is_number);
                    let kind = if numbers {
                        "number"
                    } else if present.iter().all(|v| text_of(v).is_some()) {
                        "text"
                    } else if present.iter().all(|v| matches!(v, Expr::Bool(_))) {
                        "truth"
                    } else {
                        "mixed"
                    };
                    out.columns[0].push(text(name));
                    out.columns[1].push(text(kind));
                    for (slot, method) in [(2, "min"), (3, "median"), (4, "max")] {
                        out.columns[slot].push(match numbers {
                            true => self.summed(method, &present)?,
                            false => Expr::undefined(),
                        });
                    }
                    out.columns[5].push(Expr::int((column.len() - present.len()) as i64));
                }
                out.expr()
            }
            ("groupcounts", 2) => {
                let keys = table.columns_of(&a[1], who)?;
                let groups = groups_of(&table, &keys);
                let mut out = table.only(&keys);
                out.timed = false;
                out.columns = (0..keys.len())
                    .map(|j| groups.iter().map(|(key, _)| key[j].clone()).collect())
                    .collect();
                out.names.push("GroupCount".into());
                out.columns.push(
                    groups
                        .iter()
                        .map(|(_, rows)| Expr::int(rows.len() as i64))
                        .collect(),
                );
                out.names.push("Percent".into());
                out.columns.push(
                    groups
                        .iter()
                        .map(|(_, rows)| {
                            tidy(div(
                                Expr::int(100 * rows.len() as i64),
                                Expr::int(height as i64),
                            ))
                        })
                        .collect(),
                );
                out.checked(who)?.expr()
            }
            ("groupsummary", 2..=4) => {
                let keys = table.columns_of(&a[1], who)?;
                let methods: Vec<String> = match a.get(2) {
                    Some(m) => list_of(m)
                        .iter()
                        .map(|m| {
                            word(m).ok_or("groupsummary: the methods are names such as \"mean\"")
                        })
                        .collect::<Result<_, _>>()?,
                    None => Vec::new(),
                };
                let data: Vec<usize> = match a.get(3) {
                    Some(names) => table.columns_of(names, who)?,
                    None => (data_from..table.names.len())
                        .filter(|k| {
                            !keys.contains(k)
                                && table.columns[*k]
                                    .iter()
                                    .all(|v| is_number(v) || v.is_undefined())
                        })
                        .collect(),
                };
                let groups = groups_of(&table, &keys);
                let mut out = table.only(&keys);
                out.timed = false;
                out.columns = (0..keys.len())
                    .map(|j| groups.iter().map(|(key, _)| key[j].clone()).collect())
                    .collect();
                out.names.push("GroupCount".into());
                out.columns.push(
                    groups
                        .iter()
                        .map(|(_, rows)| Expr::int(rows.len() as i64))
                        .collect(),
                );
                for method in &methods {
                    for k in &data {
                        let mut column = Vec::with_capacity(groups.len());
                        for (_, rows) in &groups {
                            let values: Vec<Expr> =
                                rows.iter().map(|r| table.columns[*k][*r].clone()).collect();
                            column.push(self.summed(method, &values)?);
                        }
                        out.names.push(format!("{method}_{}", table.names[*k]));
                        out.columns.push(column);
                    }
                }
                out.checked(who)?.expr()
            }
            ("stack", 2) => {
                let stacked = table.columns_of(&a[1], who)?;
                let kept: Vec<usize> = (0..table.names.len())
                    .filter(|k| !stacked.contains(k))
                    .collect();
                let joined_name = stacked
                    .iter()
                    .map(|k| table.names[*k].clone())
                    .collect::<Vec<_>>()
                    .join("_");
                let mut out = table.only(&kept);
                out.timed = false;
                for column in out.columns.iter_mut() {
                    column.clear();
                }
                let (mut marks, mut values) = (Vec::new(), Vec::new());
                for row in 0..height {
                    for k in &stacked {
                        for (column, from) in out.columns.iter_mut().zip(&kept) {
                            column.push(table.columns[*from][row].clone());
                        }
                        marks.push(text(&table.names[*k]));
                        values.push(table.columns[*k][row].clone());
                    }
                }
                out.names.push(format!("{joined_name}_Indicator"));
                out.columns.push(marks);
                out.names.push(joined_name);
                out.columns.push(values);
                out.checked(who)?.expr()
            }
            ("unstack", 3) => {
                let (value, mark) = (table.column(&a[1], who)?, table.column(&a[2], who)?);
                let kept: Vec<usize> = (0..table.names.len())
                    .filter(|k| *k != value && *k != mark)
                    .collect();
                let marks = kinds_of(&table.columns[mark]);
                let groups = groups_of(&table, &kept);
                let mut out = table.only(&kept);
                out.timed = false;
                out.columns = (0..kept.len())
                    .map(|j| groups.iter().map(|(key, _)| key[j].clone()).collect())
                    .collect();
                for kind in &marks {
                    let mut column = Vec::with_capacity(groups.len());
                    for (_, rows) in &groups {
                        let found: Vec<Expr> = rows
                            .iter()
                            .filter(|r| same(&table.columns[mark][**r], kind))
                            .map(|r| table.columns[value][*r].clone())
                            .collect();
                        column.push(match found.len() {
                            0 => Expr::undefined(),
                            1 => found[0].clone(),
                            _ => self.summed("sum", &found)?,
                        });
                    }
                    out.names.push(match text_of(kind) {
                        Some(t) => t.to_string(),
                        None => format!("x{kind}"),
                    });
                    out.columns.push(column);
                }
                out.checked(who)?.expr()
            }
            ("retime" | "lag" | "synchronize", _) if !table.timed => {
                return Err(format!(
                    "{who} needs a table with times, made by timetable([times], \"name\", [values], ...)"
                ));
            }
            ("isregular", 1) if table.timed => {
                let steps: Vec<Expr> = table.columns[0]
                    .windows(2)
                    .map(|w| tidy(sub(w[1].clone(), w[0].clone())))
                    .collect();
                Expr::Bool(steps.windows(2).all(|w| crate::arrays::same(&w[0], &w[1])))
            }
            ("lag", 1 | 2) if table.timed => {
                let by = match a.get(1) {
                    Some(k) => whole(k, "the number of rows", -(height as i64), height as i64)?,
                    None => 1,
                };
                let mut out = table.clone();
                for column in out.columns[1..].iter_mut() {
                    let old = column.clone();
                    for (k, slot) in column.iter_mut().enumerate() {
                        let from = k as i64 - by;
                        *slot = match (0..height as i64).contains(&from) {
                            true => old[from as usize].clone(),
                            false => Expr::undefined(),
                        };
                    }
                }
                out.expr()
            }
            ("retime", 2..=5) if table.timed => {
                let usage = "retime(timetable, [new times][, method]) or retime(timetable, \"regular\", method, \"TimeStep\", step); methods: fillwithmissing, previous, next, nearest, linear, spline, pchip, or gathered from a time to the next: mean, sum, min, max, median, count, firstvalue, lastvalue";
                let (plain, chosen) = options(&a[1..], &["timestep"]);
                let method = match plain.get(1) {
                    Some(m) => word(m).ok_or(usage)?,
                    None => "fillwithmissing".to_string(),
                };
                let times: Vec<Expr> = match (plain.first(), chosen.first()) {
                    (Some(r), Some((_, step))) if word(r).as_deref() == Some("regular") => {
                        let sorted = table.sorted(&[(0, false)]);
                        let (Some(first), Some(last)) = (sorted.first(), sorted.last()) else {
                            return Err(usage.into());
                        };
                        let (first, last) = (
                            table.columns[0][*first].clone(),
                            table.columns[0][*last].clone(),
                        );
                        if order(step, &Expr::zero()) != Ok(Ordering::Greater) {
                            return Err("retime: the step of time is above zero".into());
                        }
                        let mut times = vec![first.clone()];
                        while times.len() < 1_000_000 {
                            let next = tidy(add(vec![
                                first.clone(),
                                mul(vec![Expr::int(times.len() as i64), (*step).clone()]),
                            ]));
                            if compare(&next, &last) == Ordering::Greater {
                                break;
                            }
                            times.push(next);
                        }
                        times
                    }
                    (Some(t), None) if t.as_list().is_some() => list_of(t),
                    _ => return Err(usage.into()),
                };
                self.retime(&table, &times, &method)?.expr()
            }
            ("synchronize", 2..=4) if table.timed => {
                let other = need(&a[1], who)?;
                if !other.timed {
                    return Err("synchronize(timetable, timetable[, \"union\" | \"intersection\" | \"first\" | \"last\"][, method])".into());
                }
                let (mut how, mut method) = ("union".to_string(), "fillwithmissing".to_string());
                for option in &a[2..] {
                    match word(option).as_deref() {
                        Some(w @ ("union" | "intersection" | "first" | "last")) => {
                            how = w.to_string()
                        }
                        Some(w) => method = w.to_string(),
                        None => return Err("synchronize: the options are names in quotes".into()),
                    }
                }
                let mut times: Vec<Expr> = match how.as_str() {
                    "first" => table.columns[0].clone(),
                    "last" => other.columns[0].clone(),
                    "intersection" => table.columns[0]
                        .iter()
                        .filter(|t| other.columns[0].iter().any(|o| same(o, t)))
                        .cloned()
                        .collect(),
                    _ => table.columns[0]
                        .iter()
                        .chain(&other.columns[0])
                        .cloned()
                        .collect(),
                };
                times.sort_by(compare);
                times.dedup_by(|x, y| same(x, y));
                let mut out = self.retime(&table, &times, &method)?;
                let more = self.retime(&other, &times, &method)?;
                for (name, column) in more.names.into_iter().zip(more.columns).skip(1) {
                    out.names.push(match table.find(&name) {
                        Some(_) => format!("{name}_right"),
                        None => name,
                    });
                    out.columns.push(column);
                }
                out.checked(who)?.expr()
            }
            _ => return Err(String::new()),
        })
    }

    /// `varfun(f, table)` and `rowfun(f, table)`: a function over the variables or the rows.
    pub(crate) fn table_function(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        if !matches!((name, a.len()), ("varfun" | "rowfun", 2)) {
            return None;
        }
        let table = read(&a[1])?;
        Some((|| -> Res {
            let from = usize::from(table.timed);
            match name {
                "varfun" => {
                    let mut out = Table {
                        timed: false,
                        names: Vec::new(),
                        columns: Vec::new(),
                    };
                    for (field, column) in table.names.iter().zip(&table.columns).skip(from) {
                        let value = self.apply(&a[0], &[Expr::list(column.clone())])?;
                        out.names.push(field.clone());
                        out.columns.push(list_of(&value));
                    }
                    Ok(out.checked(name)?.expr())
                }
                _ => {
                    let mut values = Vec::with_capacity(table.height());
                    for row in table.rows() {
                        values.push(self.apply(&a[0], &row[from..])?);
                    }
                    Ok(Table {
                        timed: false,
                        names: vec!["Var1".into()],
                        columns: vec![values],
                    }
                    .expr())
                }
            }
        })())
    }
}

/// The rows of a table by the values of some columns: every group once, in rising order
/// of its key, with the rows that have it.
fn groups_of(table: &Table, keys: &[usize]) -> Vec<(Vec<Expr>, Vec<usize>)> {
    let mut groups: Vec<(Vec<Expr>, Vec<usize>)> = Vec::new();
    for row in 0..table.height() {
        let key: Vec<Expr> = keys
            .iter()
            .map(|k| table.columns[*k][row].clone())
            .collect();
        match groups.iter_mut().find(|(known, _)| same_row(known, &key)) {
            Some((_, rows)) => rows.push(row),
            None => groups.push((key, vec![row])),
        }
    }
    groups.sort_by(|a, b| compare_rows(&a.0, &b.0));
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numbers(v: &[i64]) -> Expr {
        Expr::list(v.iter().map(|k| Expr::int(*k)).collect())
    }

    #[test]
    fn a_table_is_read_from_what_it_shows() {
        let made = build(
            false,
            &[
                text("a"),
                numbers(&[3, 1, 2]),
                text("b"),
                numbers(&[4, 5, 6]),
            ],
        )
        .unwrap();
        let table = read(&made).expect("a table");
        assert_eq!(table.names, ["a", "b"]);
        assert_eq!(table.height(), 3);
        assert_eq!(table.sorted(&[(0, false)]), [1, 2, 0]);
        assert_eq!(table.clone().expr(), made);
        assert!(build(false, &[text("a"), numbers(&[1]), text("a"), numbers(&[2])]).is_err());
        assert!(
            build(
                false,
                &[text("a"), numbers(&[1, 2]), text("b"), numbers(&[2])]
            )
            .is_err()
        );
        let plain = build(false, &[numbers(&[1, 2]), numbers(&[3, 4])]).unwrap();
        assert_eq!(read(&plain).unwrap().names, ["Var1", "Var2"]);
    }

    #[test]
    fn what_is_written_is_told_apart() {
        let i = Expr::sym("k");
        assert!(written_as_table(&[text("a"), numbers(&[1, 2])]));
        assert!(written_as_table(&[numbers(&[1, 2]), numbers(&[3, 4])]));
        assert!(!written_as_table(&[
            i.clone(),
            Expr::list(vec![i.clone(), Expr::int(1), Expr::int(3)])
        ]));
        assert!(!written_as_table(&[text("a"), Expr::int(3)]));
        assert!(!written_as_table(&[
            text("a"),
            Expr::list(vec![i, Expr::int(1), Expr::int(3)])
        ]));
    }

    #[test]
    fn kinds_keep_their_order() {
        let made = make_kinds(&[
            Expr::list(vec![text("b"), text("a"), text("b"), text("z")]),
            Expr::list(vec![text("b"), text("a")]),
        ])
        .unwrap();
        let kinds = read_kinds(&made).expect("kinds");
        assert_eq!(kinds.values, [Some(0), Some(1), Some(0), None]);
        assert_eq!(kinds.clone().expr(), made);
        let found = make_kinds(&[Expr::list(vec![text("b"), text("a")])]).unwrap();
        assert_eq!(read_kinds(&found).unwrap().kinds, [text("a"), text("b")]);
    }
}
