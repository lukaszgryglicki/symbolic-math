//! Files of MATLAB's own format for values: MAT-files of level 5, as MATLAB writes them
//! since its version 5, and with their values packed by zlib as its version 7 does. A file
//! is read as a record of its variables and written from one.

use crate::canon::{add, mul};
use crate::expr::Expr;
use crate::radix::text_of;
use crate::texts::text;
use std::io::{Read, Write};

type Res = Result<Expr, String>;

const MATRIX: u32 = 14;
const PACKED: u32 = 15;

/// The elements of a file, one after another.
struct Bytes<'a> {
    all: &'a [u8],
    at: usize,
    swap: bool,
}

impl<'a> Bytes<'a> {
    fn word(&self, at: usize) -> Result<u32, String> {
        let four: [u8; 4] = self
            .all
            .get(at..at + 4)
            .and_then(|b| b.try_into().ok())
            .ok_or("the file ends in the middle of a value")?;
        Ok(if self.swap {
            u32::from_be_bytes(four)
        } else {
            u32::from_le_bytes(four)
        })
    }

    /// The kind and the bytes of the next element; `None` at the end.
    fn element(&mut self) -> Result<Option<(u32, &'a [u8])>, String> {
        if self.at + 8 > self.all.len() {
            return Ok(None);
        }
        let first = self.word(self.at)?;
        if first >> 16 != 0 {
            // a small element: the kind and the length in one word, the bytes in the next
            let (kind, length) = (first & 0xffff, (first >> 16) as usize);
            if length > 4 {
                return Err("a small element of the file holds more than four bytes".into());
            }
            let data = &self.all[self.at + 4..self.at + 4 + length];
            self.at += 8;
            return Ok(Some((kind, data)));
        }
        let length = self.word(self.at + 4)? as usize;
        let start = self.at + 8;
        let data = self
            .all
            .get(start..start + length)
            .ok_or("the file ends in the middle of a value")?;
        // (a packed element is not followed by filling)
        self.at = start
            + if first == PACKED {
                length
            } else {
                length.div_ceil(8) * 8
            };
        Ok(Some((first, data)))
    }

    fn needed(&mut self, what: &str) -> Result<(u32, &'a [u8]), String> {
        self.element()?
            .ok_or_else(|| format!("the file ends before {what}"))
    }
}

/// A double as a number of this program: whole numbers exact, others by their shortest
/// decimal form.
pub(crate) fn double(x: f64) -> Expr {
    if x.is_nan() {
        return Expr::undefined();
    }
    if x.is_infinite() {
        return if x > 0.0 {
            Expr::inf()
        } else {
            mul(vec![Expr::minus_one(), Expr::inf()])
        };
    }
    if x.fract() == 0.0 && x.abs() < 9_007_199_254_740_992.0 {
        return Expr::int(x as i64);
    }
    let digits = crate::numeric::digits();
    let prec = crate::numeric::bits_for(digits) + 32;
    match rug::Float::parse(format!("{x:e}")) {
        Ok(parsed) => crate::numeric::to_float_expr(
            rug::Complex::with_val(prec, rug::Float::with_val(prec, parsed)),
            digits,
        ),
        Err(_) => Expr::undefined(),
    }
}

/// The numbers of an element of numbers, by its kind.
fn numbers(kind: u32, data: &[u8], swap: bool) -> Result<Vec<Expr>, String> {
    fn each<const N: usize>(data: &[u8], swap: bool, f: impl Fn([u8; N]) -> Expr) -> Vec<Expr> {
        data.as_chunks::<N>()
            .0
            .iter()
            .map(|c| {
                let mut b: [u8; N] = *c;
                if swap {
                    b.reverse();
                }
                f(b)
            })
            .collect()
    }
    Ok(match kind {
        1 => each::<1>(data, swap, |b| Expr::int(i64::from(b[0] as i8))),
        2 | 16 => each::<1>(data, swap, |b| Expr::int(i64::from(b[0]))),
        3 => each::<2>(data, swap, |b| Expr::int(i64::from(i16::from_le_bytes(b)))),
        4 | 17 => each::<2>(data, swap, |b| Expr::int(i64::from(u16::from_le_bytes(b)))),
        5 => each::<4>(data, swap, |b| Expr::int(i64::from(i32::from_le_bytes(b)))),
        6 | 18 => each::<4>(data, swap, |b| Expr::int(i64::from(u32::from_le_bytes(b)))),
        7 => each::<4>(data, swap, |b| {
            // (a single by its own shortest decimal form)
            let x = f32::from_le_bytes(b);
            match format!("{x:e}").parse::<f64>() {
                Ok(v) if x.is_finite() => double(v),
                _ => double(f64::from(x)),
            }
        }),
        9 => each::<8>(data, swap, |b| double(f64::from_le_bytes(b))),
        12 => each::<8>(data, swap, |b| Expr::int(i64::from_le_bytes(b))),
        13 => each::<8>(data, swap, |b| {
            Expr::integer(rug::Integer::from(u64::from_le_bytes(b)))
        }),
        other => return Err(format!("numbers of the kind {other} are not known")),
    })
}

/// Values in the order of MATLAB (down the columns first) as a value of this program: one
/// value, a row as a list, a column as rows of one entry, a matrix as rows, more
/// dimensions as lists of those.
fn shaped(mut values: Vec<Expr>, dims: &[usize]) -> Expr {
    let total: usize = dims.iter().product();
    if total == 0 || values.len() != total {
        return Expr::list(Vec::new());
    }
    if total == 1 {
        return values.remove(0);
    }
    match dims {
        [1, _] => Expr::list(values),
        [rows, cols] => Expr::list(
            (0..*rows)
                .map(|i| Expr::list((0..*cols).map(|j| values[i + j * rows].clone()).collect()))
                .collect(),
        ),
        [first @ .., last] => {
            let page: usize = first.iter().product();
            Expr::list(
                (0..*last)
                    .map(|k| shaped(values[k * page..(k + 1) * page].to_vec(), first))
                    .collect(),
            )
        }
        [] => Expr::list(values),
    }
}

/// The name and the value of an array of the file.
#[allow(clippy::needless_range_loop)]
fn array(data: &[u8], swap: bool) -> Result<(String, Expr), String> {
    if data.is_empty() {
        return Ok((String::new(), Expr::list(Vec::new())));
    }
    let mut inner = Bytes {
        all: data,
        at: 0,
        swap,
    };
    let (_, flags) = inner.needed("the kind of an array")?;
    if flags.len() < 8 {
        return Err("an array of the file has no kind".into());
    }
    let word = Bytes {
        all: flags,
        at: 0,
        swap,
    }
    .word(0)?;
    let (class, complex, logical) = (word & 0xff, word & 0x0800 != 0, word & 0x0200 != 0);
    let (kind, sizes) = inner.needed("the sizes of an array")?;
    let dims: Vec<usize> = numbers(kind, sizes, swap)?
        .iter()
        .map(|d| d.as_i64().unwrap_or(0).max(0) as usize)
        .collect();
    let total: usize = dims.iter().product();
    let (_, name) = inner.needed("the name of an array")?;
    let name = String::from_utf8_lossy(name)
        .trim_end_matches('\0')
        .to_string();
    let value = match class {
        6..=15 => {
            let (kind, real) = inner.needed("the numbers of an array")?;
            let mut values = numbers(kind, real, swap)?;
            if complex {
                let (kind, imaginary) = inner.needed("the imaginary parts of an array")?;
                let parts = numbers(kind, imaginary, swap)?;
                values = values
                    .into_iter()
                    .zip(parts)
                    .map(|(re, im)| crate::arrays::tidy(add(vec![re, mul(vec![im, Expr::i()])])))
                    .collect();
            }
            if logical {
                values = values
                    .into_iter()
                    .map(|v| Expr::Bool(!v.is_zero()))
                    .collect();
            }
            shaped(values, &dims)
        }
        4 => {
            let (kind, data) = inner.needed("the signs of a text")?;
            let signs: Vec<char> = match kind {
                16 => String::from_utf8_lossy(data).chars().collect(),
                18 => numbers(kind, data, swap)?
                    .iter()
                    .filter_map(|c| c.as_i64().and_then(|c| char::from_u32(c as u32)))
                    .collect(),
                _ => {
                    let units: Vec<u16> = numbers(kind, data, swap)?
                        .iter()
                        .map(|c| c.as_i64().unwrap_or(0) as u16)
                        .collect();
                    char::decode_utf16(units)
                        .map(|c| c.unwrap_or('\u{fffd}'))
                        .collect()
                }
            };
            let rows = dims.first().copied().unwrap_or(0);
            match (rows, signs.len() == total) {
                (0, _) => text(""),
                (1, _) | (_, false) => text(&signs.into_iter().collect::<String>()),
                _ => {
                    let cols = total / rows;
                    Expr::list(
                        (0..rows)
                            .map(|i| {
                                text(&(0..cols).map(|j| signs[i + j * rows]).collect::<String>())
                            })
                            .collect(),
                    )
                }
            }
        }
        1 => {
            let mut values = Vec::with_capacity(total);
            for _ in 0..total {
                let (kind, data) = inner.needed("a cell")?;
                values.push(if kind == MATRIX {
                    array(data, swap)?.1
                } else {
                    Expr::list(Vec::new())
                });
            }
            match dims.as_slice() {
                [1, _] | [_, 1] => crate::cells::made(values),
                _ => match shaped(values, &dims) {
                    Expr::List(rows) => crate::cells::made(rows.to_vec()),
                    one => crate::cells::made(vec![one]),
                },
            }
        }
        2 | 3 => {
            if class == 3 {
                inner.needed("the name of a class")?;
            }
            let (kind, length) = inner.needed("the names of the fields")?;
            let length = numbers(kind, length, swap)?
                .first()
                .and_then(Expr::as_i64)
                .unwrap_or(0)
                .max(1) as usize;
            let (_, names) = inner.needed("the names of the fields")?;
            let fields: Vec<String> = names
                .chunks(length)
                .map(|c| {
                    String::from_utf8_lossy(c)
                        .trim_end_matches('\0')
                        .to_string()
                })
                .collect();
            let mut records = Vec::with_capacity(total);
            for _ in 0..total {
                let mut pairs = Vec::with_capacity(2 * fields.len());
                for field in &fields {
                    let (kind, data) = inner.needed("the value of a field")?;
                    pairs.push(text(field));
                    pairs.push(if kind == MATRIX {
                        array(data, swap)?.1
                    } else {
                        Expr::list(Vec::new())
                    });
                }
                records.push(Expr::call("struct", pairs));
            }
            shaped(records, &dims)
        }
        5 => {
            let (rows, cols) = (
                dims.first().copied().unwrap_or(0),
                dims.get(1).copied().unwrap_or(0),
            );
            if rows.saturating_mul(cols) > 4_000_000 {
                return Err(format!(
                    "the sparse matrix {name} of {rows} by {cols} is too large to be read as a matrix"
                ));
            }
            let whole = |inner: &mut Bytes, what: &str| -> Result<Vec<usize>, String> {
                let (kind, data) = inner.needed(what)?;
                Ok(numbers(kind, data, swap)?
                    .iter()
                    .map(|v| v.as_i64().unwrap_or(0).max(0) as usize)
                    .collect())
            };
            let (at_rows, starts) = (
                whole(&mut inner, "the rows of a sparse matrix")?,
                whole(&mut inner, "the columns of a sparse matrix")?,
            );
            let (kind, real) = inner.needed("the numbers of a sparse matrix")?;
            let mut values = numbers(kind, real, swap)?;
            if complex {
                let (kind, imaginary) = inner.needed("the imaginary parts of a sparse matrix")?;
                let parts = numbers(kind, imaginary, swap)?;
                values = values
                    .into_iter()
                    .zip(parts)
                    .map(|(re, im)| crate::arrays::tidy(add(vec![re, mul(vec![im, Expr::i()])])))
                    .collect();
            }
            let mut dense = vec![vec![Expr::zero(); cols]; rows];
            for j in 0..cols {
                for k in
                    starts.get(j).copied().unwrap_or(0)..starts.get(j + 1).copied().unwrap_or(0)
                {
                    if let (Some(i), Some(v)) =
                        (at_rows.get(k).filter(|i| **i < rows), values.get(k))
                    {
                        dense[*i][j] = if logical {
                            Expr::Bool(!v.is_zero())
                        } else {
                            v.clone()
                        };
                    }
                }
            }
            crate::matrix::from_rows(dense)
        }
        16 => text("(a function handle of MATLAB: not read)"),
        _ => text("(an object of MATLAB, as a string array, a table or a datetime: not read)"),
    };
    Ok((name, value))
}

/// The variables of a file by their names, in the order of the file.
fn variables(path: &str) -> Result<Vec<(String, Expr)>, String> {
    crate::sandbox::allowed(path)?;
    let all = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    if all.len() < 128 {
        return Err(format!("{path} is no MAT-file: it has no heading"));
    }
    if all.starts_with(b"\x89HDF") || all[..116].starts_with(b"MATLAB 7.3") {
        return Err(format!(
            "{path} is a MAT-file of the version 7.3, which is a file of HDF5: save it with -v7 (the usual kind) to read it here"
        ));
    }
    let swap = match &all[126..128] {
        b"IM" => false,
        b"MI" => true,
        _ => {
            return Err(format!(
                "{path} is no MAT-file of level 5 (the kind that MATLAB writes since its version 5)"
            ));
        }
    };
    let mut file = Bytes {
        all: &all,
        at: 128,
        swap,
    };
    let mut found = Vec::new();
    while let Some((kind, data)) = file.element()? {
        match kind {
            MATRIX => found.push(array(data, swap)?),
            PACKED => {
                let mut plain = Vec::new();
                flate2::read::ZlibDecoder::new(data)
                    .take(1 << 30)
                    .read_to_end(&mut plain)
                    .map_err(|e| format!("{path}: a packed value does not unpack: {e}"))?;
                let mut inner = Bytes {
                    all: &plain,
                    at: 0,
                    swap,
                };
                if let Some((MATRIX, data)) = inner.element()? {
                    found.push(array(data, swap)?);
                }
            }
            _ => {}
        }
    }
    Ok(found
        .into_iter()
        .filter(|(name, _)| !name.is_empty() && !name.starts_with("__"))
        .collect())
}

/// `matread("file.mat")`: the record of the variables of a file; with a name the value of
/// one. `matinfo("file.mat")`: the names with the kinds and sizes of their values.
pub(crate) fn call(name: &str, a: &[Expr]) -> Option<Res> {
    Some(match (name, a.len()) {
        ("matread", 1 | 2) => (|| -> Res {
            let path = text_of(&a[0]).ok_or("matread(\"file.mat\"[, \"name\"]): the variables of a MAT-file as a record, or the value of one")?;
            let found = variables(path).map_err(|e| format!("matread: {e}"))?;
            match a.get(1) {
                None => Ok(Expr::call(
                    "struct",
                    found
                        .into_iter()
                        .flat_map(|(name, value)| [text(&name), value])
                        .collect(),
                )),
                Some(wanted) => {
                    let wanted = text_of(wanted).ok_or(
                        "matread(\"file.mat\", \"name\"): the name of a variable in quotes",
                    )?;
                    let names: Vec<String> = found.iter().map(|(n, _)| n.clone()).collect();
                    found
                        .into_iter()
                        .find(|(n, _)| n == wanted)
                        .map(|(_, v)| v)
                        .ok_or_else(|| {
                            format!(
                                "matread: {path} has no variable {wanted}; it has {}",
                                names.join(", ")
                            )
                        })
                }
            }
        })(),
        ("matinfo", 1) => {
            (|| -> Res {
                let path = text_of(&a[0]).ok_or("matinfo(\"file.mat\"): the names, sizes and kinds of the variables of a MAT-file")?;
                let found = variables(path).map_err(|e| format!("matinfo: {e}"))?;
                Ok(Expr::list(
                    found
                        .iter()
                        .map(|(name, value)| {
                            let kind = crate::extras::call("class", std::slice::from_ref(value))
                                .and_then(Result::ok)
                                .unwrap_or_else(|| text("?"));
                            let kind = if crate::cells::is_cell(value) {
                                text("cell")
                            } else {
                                kind
                            };
                            let size = crate::cells::open(value);
                            let (rows, cols) = match size.as_list() {
                                None => (1, 1),
                                Some(items) if text_of(value).is_some() => (1, items.len()),
                                Some(items) => match items.first().and_then(Expr::as_list) {
                                    Some(first)
                                        if items.iter().all(|r| {
                                            r.as_list().is_some_and(|r| r.len() == first.len())
                                        }) =>
                                    {
                                        (items.len(), first.len())
                                    }
                                    _ => (usize::from(!items.is_empty()), items.len()),
                                },
                            };
                            let (rows, cols) = match text_of(value) {
                                Some(t) => (1, t.chars().count()),
                                None => (rows, cols),
                            };
                            Expr::list(vec![
                                text(name),
                                Expr::list(vec![Expr::int(rows as i64), Expr::int(cols as i64)]),
                                kind,
                            ])
                        })
                        .collect(),
                ))
            })()
        }
        _ => return None,
    })
}

/// An element of a file: its kind, its length and its bytes, filled up to eight.
fn element(kind: u32, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + data.len() + 8);
    out.extend(kind.to_le_bytes());
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(data);
    out.resize(8 + data.len().div_ceil(8) * 8, 0);
    out
}

/// The heading of every array: its class with what is true of it, its sizes, its name.
fn heading(class: u32, complex: bool, logical: bool, dims: &[usize], name: &str) -> Vec<u8> {
    let flags = class | if complex { 0x0800 } else { 0 } | if logical { 0x0200 } else { 0 };
    let mut out = element(6, &[flags.to_le_bytes(), 0u32.to_le_bytes()].concat());
    out.extend(element(
        5,
        &dims
            .iter()
            .flat_map(|d| (*d as i32).to_le_bytes())
            .collect::<Vec<u8>>(),
    ));
    out.extend(element(1, name.as_bytes()));
    out
}

/// A number as the two doubles of its parts; `None` for what is no number.
fn parts(e: &Expr) -> Option<(f64, f64)> {
    if let Expr::Bool(b) = e {
        return Some((f64::from(u8::from(*b)), 0.0));
    }
    if matches!(e, Expr::List(_) | Expr::Fn(..)) && !e.is_const() {
        return None;
    }
    if e.is_sym(crate::expr::INF) {
        return Some((f64::INFINITY, 0.0));
    }
    if e.is_sym(crate::expr::UNDEF) {
        return Some((f64::NAN, 0.0));
    }
    let z = crate::numeric::evalf_digits(e, 20).ok()?;
    Some((z.real().to_f64(), z.imag().to_f64()))
}

/// An array of the file for a value, with a name (none inside cells and records).
fn written(name: &str, value: &Expr) -> Result<Vec<u8>, String> {
    let numeric = |values: &[&Expr], dims: &[usize]| -> Option<Vec<u8>> {
        let all: Vec<(f64, f64)> = values.iter().map(|v| parts(v)).collect::<Option<_>>()?;
        let logical = !values.is_empty() && values.iter().all(|v| matches!(v, Expr::Bool(_)));
        let complex = all.iter().any(|(_, im)| *im != 0.0);
        let mut out = heading(if logical { 9 } else { 6 }, complex, logical, dims, name);
        if logical {
            out.extend(element(
                2,
                &all.iter().map(|(re, _)| *re as u8).collect::<Vec<u8>>(),
            ));
        } else {
            out.extend(element(
                9,
                &all.iter()
                    .flat_map(|(re, _)| re.to_le_bytes())
                    .collect::<Vec<u8>>(),
            ));
            if complex {
                out.extend(element(
                    9,
                    &all.iter()
                        .flat_map(|(_, im)| im.to_le_bytes())
                        .collect::<Vec<u8>>(),
                ));
            }
        }
        Some(out)
    };
    // (plain letters as MATLAB writes them, two bytes each; others in UTF-8, which MATLAB
    // and SciPy both read)
    let letters = |t: &str| -> Vec<u8> {
        let count = t.chars().count();
        let mut out = heading(4, false, false, &[usize::from(count > 0), count], name);
        match t.is_ascii() {
            true => out.extend(element(
                4,
                &t.bytes()
                    .flat_map(|b| u16::from(b).to_le_bytes())
                    .collect::<Vec<u8>>(),
            )),
            false => out.extend(element(16, t.as_bytes())),
        }
        out
    };
    let cell = |items: &[Expr]| -> Result<Vec<u8>, String> {
        let mut out = heading(
            1,
            false,
            false,
            &[usize::from(!items.is_empty()), items.len()],
            name,
        );
        for item in items {
            out.extend(written("", item)?);
        }
        Ok(out)
    };
    let fields_of = |e: &Expr| -> Option<Vec<(String, Expr)>> {
        let args = e.fn_args("struct")?;
        (args.len() % 2 == 0).then(|| {
            args.chunks(2)
                .filter_map(|c| Some((text_of(&c[0])?.to_string(), c[1].clone())))
                .collect()
        })
    };
    let records = |all: &[Vec<(String, Expr)>]| -> Result<Vec<u8>, String> {
        let names: Vec<&String> = all
            .first()
            .map(|r| r.iter().map(|(k, _)| k).collect())
            .unwrap_or_default();
        let width = names.iter().map(|n| n.len()).max().unwrap_or(0) + 1;
        if width > 64 {
            return Err("the name of a field of a MAT-file has 63 bytes at most".into());
        }
        let mut out = heading(2, false, false, &[1, all.len()], name);
        // (the length of the names is a small element)
        out.extend([5u8, 0, 4, 0]);
        out.extend((width as i32).to_le_bytes());
        let mut table = Vec::new();
        for n in &names {
            let mut padded = n.as_bytes().to_vec();
            padded.resize(width, 0);
            table.extend(padded);
        }
        out.extend(element(1, &table));
        for record in all {
            for (_, value) in record {
                out.extend(written("", value)?);
            }
        }
        Ok(out)
    };
    let body = if let Some(t) = text_of(value) {
        letters(t)
    } else if let Some(items) = value.fn_args("cellarray") {
        cell(items)?
    } else if let Some(fields) = fields_of(value) {
        records(&[fields])?
    } else if let Some(items) = value.as_list() {
        let rows: Option<Vec<&[Expr]>> = items.iter().map(Expr::as_list).collect();
        let together: Option<Vec<Vec<(String, Expr)>>> = items.iter().map(&fields_of).collect();
        match (rows, together) {
            _ if items.is_empty() => numeric(&[], &[0, 0]).unwrap_or_default(),
            (Some(rows), _)
                if !rows[0].is_empty()
                    && rows.iter().all(|r| r.len() == rows[0].len())
                    && rows.iter().all(|r| r.iter().all(|x| parts(x).is_some())) =>
            {
                // a matrix: down the columns first
                let (high, wide) = (rows.len(), rows[0].len());
                let ordered: Vec<&Expr> = (0..wide)
                    .flat_map(|j| rows.iter().map(move |r| &r[j]))
                    .collect();
                numeric(&ordered, &[high, wide]).unwrap_or_default()
            }
            (_, Some(all))
                if all
                    .iter()
                    .all(|r| r.iter().map(|(k, _)| k).eq(all[0].iter().map(|(k, _)| k))) =>
            {
                records(&all)?
            }
            _ => match numeric(&items.iter().collect::<Vec<_>>(), &[1, items.len()]) {
                Some(row) => row,
                None => cell(items)?,
            },
        }
    } else {
        match numeric(&[value], &[1, 1]) {
            Some(one) => one,
            // (a formula goes as its text)
            None => letters(&format!("{value}")),
        }
    };
    Ok(element(MATRIX, &body))
}

/// `matwrite("file.mat", record)` or `matwrite("file.mat", "name", value, ...)`, with
/// "compress" as the last word to pack the values as MATLAB's version 7 does.
pub(crate) fn write(a: &[Expr]) -> Result<Vec<String>, String> {
    let usage = "matwrite(\"file.mat\", record) or matwrite(\"file.mat\", \"name\", value, ...): the values as the variables of a MAT-file; \"compress\" as the last word packs them";
    let path = a.first().and_then(text_of).ok_or(usage)?;
    let mut rest = &a[1..];
    let packed = rest.len() % 2 == 1
        && rest
            .last()
            .and_then(text_of)
            .is_some_and(|w| matches!(w, "compress" | "-v7" | "v7"));
    if packed {
        rest = &rest[..rest.len() - 1];
    }
    let pairs: Vec<(String, Expr)> = match rest {
        [record] => {
            let args = record
                .fn_args("struct")
                .filter(|f| f.len() % 2 == 0)
                .ok_or(usage)?;
            args.chunks(2)
                .map(|c| Ok((text_of(&c[0]).ok_or(usage)?.to_string(), c[1].clone())))
                .collect::<Result<_, String>>()?
        }
        _ if !rest.is_empty() && rest.len().is_multiple_of(2) => rest
            .chunks(2)
            .map(|c| Ok((text_of(&c[0]).ok_or(usage)?.to_string(), c[1].clone())))
            .collect::<Result<_, String>>()?,
        _ => return Err(usage.into()),
    };
    let mut out = Vec::new();
    let mut title =
        b"MATLAB 5.0 MAT-file, Platform: symbolic-math, Created by: symbolic-math".to_vec();
    title.resize(116, b' ');
    out.extend(title);
    out.extend([0u8; 8]);
    out.extend(0x0100u16.to_le_bytes());
    out.extend(b"IM");
    for (name, value) in &pairs {
        let mut signs = name.chars();
        if !signs.next().is_some_and(|c| c.is_ascii_alphabetic())
            || !signs.all(|c| c.is_ascii_alphanumeric() || c == '_')
            || name.len() > 63
        {
            return Err(format!(
                "matwrite: the name of a variable of MATLAB begins with a letter and holds letters, digits and _ (63 at most), {name} does not"
            ));
        }
        let one = written(name, value)?;
        if packed {
            let mut encoder =
                flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            encoder
                .write_all(&one)
                .map_err(|e| format!("matwrite: {e}"))?;
            let squeezed = encoder.finish().map_err(|e| format!("matwrite: {e}"))?;
            out.extend(PACKED.to_le_bytes());
            out.extend((squeezed.len() as u32).to_le_bytes());
            out.extend(squeezed);
        } else {
            out.extend(one);
        }
    }
    crate::sandbox::allowed(path)
        .and_then(|()| crate::sandbox::spend(out.len()))
        .map_err(|e| format!("matwrite: {e}"))?;
    std::fs::write(path, &out).map_err(|e| format!("matwrite: {path}: {e}"))?;
    Ok(vec![format!(
        "saved {path}: a MAT-file of level 5 with {} variable(s){}, {} bytes",
        pairs.len(),
        if packed { ", packed" } else { "" },
        out.len()
    )])
}

#[cfg(test)]
mod tests {
    fn said(input: &str) -> String {
        crate::eval::run_line(input).map_or_else(|e| format!("error: {e}"), |lines| lines.join("|"))
    }

    #[test]
    fn what_is_written_is_read_again() {
        let dir = std::env::temp_dir().join(format!("symbolic-math-mat-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory");
        for (k, packed) in ["", ", \"compress\""].iter().enumerate() {
            let file = dir.join(format!("values{k}.mat"));
            let file = file.to_str().expect("a name");
            let saved = said(&format!(
                "matwrite(\"{file}\", \"n\", 5, \"row\", [1, 2.5, -3], \"m\", [[1, 2], [3, 4]], \"z\", 1 + 2*i, \"t\", \"zażółć\", \"b\", [true, false], \"c\", {{1, \"two\", [3, 4]}}, \"s\", struct(\"a\", 1, \"b\", \"x\"){packed})"
            ));
            assert!(
                saved.starts_with("saved") && saved.contains("8 variable(s)"),
                "{saved}"
            );
            assert_eq!(
                said(&format!("matread(\"{file}\")")),
                "struct(\"n\", 5, \"row\", [1, 2.5, -3], \"m\", [[1, 2], [3, 4]], \"z\", 1 + 2i, \"t\", \"zażółć\", \"b\", [true, false], \"c\", {1, \"two\", [3, 4]}, \"s\", struct(\"a\", 1, \"b\", \"x\"))"
            );
            assert_eq!(
                said(&format!("matread(\"{file}\", \"m\")")),
                "[[1, 2], [3, 4]]"
            );
            assert_eq!(
                said(&format!("matinfo(\"{file}\")")),
                "[[\"n\", [1, 1], \"double\"], [\"row\", [1, 3], \"double\"], [\"m\", [2, 2], \"double\"], [\"z\", [1, 1], \"double\"], [\"t\", [1, 6], \"char\"], [\"b\", [1, 2], \"logical\"], [\"c\", [1, 3], \"cell\"], [\"s\", [1, 1], \"struct\"]]"
            );
            assert!(said(&format!("matread(\"{file}\", \"q\")")).contains("has no variable q"));
        }
        std::fs::remove_dir_all(&dir).expect("removed");
    }
}
