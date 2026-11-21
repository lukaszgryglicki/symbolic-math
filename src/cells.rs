//! Cell arrays: containers that hold anything in every cell. A cell array is written and
//! shown in braces, `{1, "text", [1, 2, 3]}`, and kept as the call `cellarray(...)`; rows
//! of cells are lists in it. The functions of lists take a cell array as the list of what
//! it holds; those that put a list in another order or choose from it give a cell again.

use crate::eval::Session;
use crate::expr::Expr;
use crate::radix::text_of;
use crate::texts::text;

type Res = Result<Expr, String>;

const CELL: &str = "cellarray";

pub(crate) fn is_cell(e: &Expr) -> bool {
    e.fn_args(CELL).is_some()
}

pub(crate) fn made(items: Vec<Expr>) -> Expr {
    Expr::call(CELL, items)
}

/// A cell array as the list of what it holds; the cells in it stay cells.
pub(crate) fn open(e: &Expr) -> Expr {
    match e.fn_args(CELL) {
        Some(items) => Expr::list(items.to_vec()),
        None => e.clone(),
    }
}

/// A cell array and the cells in it as lists.
fn open_all(e: &Expr) -> Expr {
    match (e.fn_args(CELL), e.as_list()) {
        (Some(items), _) | (_, Some(items)) => Expr::list(items.iter().map(open_all).collect()),
        _ => e.clone(),
    }
}

/// The functions that see a cell array as it is.
const AWARE: &[&str] = &[CELL, "class", "isequal", "struct", "setfield", "dictionary", "insert", "isa", "whos", "validateattributes", "is"];

/// The functions of lists whose answer is a cell array again when they were given one.
const KEEP: &[&str] = &[
    "sort", "unique", "reverse", "flip", "fliplr", "flipud", "horzcat", "vertcat", "join", "append", "prepend", "rest", "most", "take", "drop", "upper", "lower", "strrep", "strtrim",
    "regexprep", "union", "intersect", "setdiff", "circshift", "rotateleft", "rotateright", "strcat", "deleteat", "insertat", "replacepart",
];

impl Session {
    pub(crate) fn cells(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        let n = a.len();
        let size = |k: usize| crate::arrays::whole(&a[k], "the size", 0, 100_000).map(|s| s as usize);
        let empty = || Expr::list(Vec::new());
        Some(match (name, n) {
            (CELL, _) => Ok(made(a.to_vec())),
            ("iscell", 1) if is_cell(&a[0]) => Ok(Expr::Bool(true)),
            ("class", 1) if is_cell(&a[0]) => Ok(text("cell")),
            ("iscellstr", 1) if is_cell(&a[0]) => Ok(Expr::Bool(a[0].fn_args(CELL).is_some_and(|items| items.iter().all(|x| text_of(x).is_some())))),
            ("cell", 1) => size(0).map(|count| made((0..count).map(|_| empty()).collect())),
            ("cell", 2) => (|| -> Res {
                let (rows, columns) = (size(0)?, size(1)?);
                Ok(made((0..rows).map(|_| Expr::list((0..columns).map(|_| empty()).collect())).collect()))
            })(),
            ("num2cell", 1) => Ok(match a[0].as_list() {
                Some(items) => made(items.to_vec()),
                None if is_cell(&a[0]) => a[0].clone(),
                None => made(vec![a[0].clone()]),
            }),
            ("cell2mat", 1) if is_cell(&a[0]) => self.call("cell2mat", &[open_all(&a[0])]),
            ("struct2cell", 1) => crate::records::call_plain("struct2cell", a).map(|values| match values.as_list() {
                Some(items) => made(items.to_vec()),
                None => values,
            }),
            ("celldisp", 1 | 2) => (|| -> Res {
                let items = a[0].fn_args(CELL).ok_or("celldisp({a, b, ...}[, \"name\"]): what every cell of a cell array holds")?;
                let named = match a.get(1) {
                    Some(t) => text_of(t).ok_or("celldisp: the name in quotes")?.to_string(),
                    None => "ans".to_string(),
                };
                let mut lines = Vec::new();
                fn walk(prefix: &str, items: &[Expr], lines: &mut Vec<Expr>) {
                    for (k, item) in items.iter().enumerate() {
                        let here = format!("{prefix}{{{}}}", k + 1);
                        match item.fn_args(CELL) {
                            Some(inner) => walk(&here, inner, lines),
                            None => lines.push(text(&format!("{here} = {item}"))),
                        }
                    }
                }
                walk(&named, items, &mut lines);
                Ok(Expr::list(lines))
            })(),
            ("mat2cell", 2 | 3) => (|| -> Res {
                let usage = "mat2cell(matrix, [rows of every block], [columns of every block]): the blocks of a matrix as a cell array of rows of blocks";
                let rows = crate::matrix::rows(&a[0]).ok_or(usage)?;
                let width = rows.first().map_or(0, Vec::len);
                let sizes = |e: &Expr, whole: usize| -> Result<Vec<usize>, String> {
                    let items = e.as_list().map(<[Expr]>::to_vec).unwrap_or_else(|| vec![e.clone()]);
                    let all: Vec<usize> = items.iter().map(|x| crate::arrays::whole(x, "the size of a block", 0, 1_000_000).map(|s| s as usize)).collect::<Result<_, _>>()?;
                    match all.iter().sum::<usize>() == whole {
                        true => Ok(all),
                        false => Err(format!("mat2cell: the sizes of the blocks add up to {}, the matrix has {whole}", all.iter().sum::<usize>())),
                    }
                };
                let down = sizes(&a[1], rows.len())?;
                let across = match a.get(2) {
                    Some(e) => sizes(e, width)?,
                    None => vec![width],
                };
                let mut out = Vec::new();
                let mut top = 0;
                for high in &down {
                    let mut line = Vec::new();
                    let mut left = 0;
                    for wide in &across {
                        let block: Vec<Vec<Expr>> = rows[top..top + high].iter().map(|r| r[left..left + wide].to_vec()).collect();
                        line.push(crate::matrix::from_rows(block));
                        left += wide;
                    }
                    out.push(if across.len() == 1 { line.remove(0) } else { Expr::list(line) });
                    top += high;
                }
                Ok(made(out))
            })(),
            _ if !AWARE.contains(&name) && a.iter().any(is_cell) => (|| -> Res {
                let plain: Vec<Expr> = a.iter().map(open).collect();
                let out = self.call(name, &plain)?;
                Ok(match (KEEP.contains(&name) && is_cell(&a[0]), out.as_list()) {
                    (true, Some(items)) => made(items.to_vec()),
                    _ => out,
                })
            })(),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    fn said(input: &str) -> String {
        crate::eval::run_line(input).map_or_else(|e| format!("error: {e}"), |lines| lines.join("|"))
    }

    #[test]
    fn cells_hold_anything_and_stay_cells() {
        assert_eq!(said("c := {1, \"two\", [3, 4], {5, x^2}}"), "{1, \"two\", [3, 4], {5, x^2}}");
        assert_eq!(said("[class({1, 2}), iscell({1, 2}), iscell([1, 2]), iscellstr({\"a\", \"b\"}), iscellstr({\"a\", 1})]"), "[\"cell\", true, false, true, false]");
        assert_eq!(said("[length({1, \"a\", [1, 2]}), numel({}), isempty({})]"), "[3, 0, true]");
        assert_eq!(said("part({1, \"two\", [3, 4]}, 3)"), "[3, 4]");
        assert_eq!(said("cell(3)"), "{[], [], []}");
        assert_eq!(said("cell(2, 2)"), "{[[], []], [[], []]}");
        assert_eq!(said("num2cell([1, 2, 3])"), "{1, 2, 3}");
        assert_eq!(said("cell2mat({[1, 2], [3]})"), "[1, 2, 3]");
        assert_eq!(said("cell2mat({[[1, 2], [3, 4]], [[5], [6]]})"), "[[1, 2, 5], [3, 4, 6]]");
        assert_eq!(said("mat2cell([[1, 2, 3], [4, 5, 6], [7, 8, 9]], [1, 2], [2, 1])"), "{[[[1, 2]], [[3]]], [[[4, 5], [7, 8]], [[6], [9]]]}");
        assert_eq!(said("celldisp({1, {\"a\", [2, 3]}}, \"c\")"), "[\"c{1} = 1\", \"c{2}{1} = \"a\"\", \"c{2}{2} = [2, 3]\"]");
        // functions of lists take a cell; those that order or choose give a cell again
        assert_eq!(said("sort({3, 1, 2})"), "{1, 2, 3}");
        assert_eq!(said("reverse({1, \"a\"})"), "{\"a\", 1}");
        assert_eq!(said("horzcat({1, 2}, {\"a\"})"), "{1, 2, \"a\"}");
        assert_eq!(said("upper({\"ab\", \"c\"})"), "{\"AB\", \"C\"}");
        assert_eq!(said("strjoin({\"a\", \"b\"}, \"-\")"), "\"a-b\"");
        assert_eq!(said("sum({1, 2, 3})"), "6");
        // cellfun: numbers give a list, other things a cell; UniformOutput false always a cell
        assert_eq!(said("cellfun(length, {[1, 2], [3], []})"), "[2, 1, 0]");
        assert_eq!(said("cellfun(x -> 2*x, {1, 2}, \"UniformOutput\", false)"), "{2, 4}");
        assert_eq!(said("cellfun(reverse, {[1, 2], [3, 4]})"), "{[2, 1], [4, 3]}");
        assert_eq!(said("cellfun(\"isempty\", {[], 1, \"\"})"), "[true, false, true]");
        assert_eq!(said("struct2cell(struct(\"a\", 1, \"b\", \"x\"))"), "{1, \"x\"}");
        assert_eq!(said("cell2struct({1, \"x\"}, [\"a\", \"b\"])"), "struct(\"a\", 1, \"b\", \"x\")");
        assert_eq!(said("isequal({1, [2, 3]}, {1, [2, 3]})"), "true");
    }
}
