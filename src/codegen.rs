//! Expressions as code of other languages, with the parts they share named once, and as a
//! picture of text in several lines.

use crate::eval::Session;
use crate::expr::{Expr, Rel};
use crate::radix::text_of;
use crate::texts::text;
use std::collections::HashMap;

type Res = Result<Expr, String>;

#[derive(Clone, Copy, PartialEq)]
enum Lang {
    C,
    Fortran,
    Matlab,
}

const ADD: u8 = 1;
const MUL: u8 = 2;
const NEG: u8 = 3;
const POW: u8 = 4;
const ATOM: u8 = 5;

impl Lang {
    fn who(self) -> &'static str {
        match self {
            Lang::C => "ccode",
            Lang::Fortran => "fortran",
            Lang::Matlab => "matlabcode",
        }
    }

    fn whole(self, digits: &str) -> String {
        match self {
            Lang::C => format!("{digits}.0"),
            Lang::Fortran => format!("{digits}.0D0"),
            Lang::Matlab => digits.to_string(),
        }
    }

    /// A decimal number as the printer writes it.
    fn decimal(self, written: &str) -> String {
        match self {
            Lang::Fortran if written.contains(['e', 'E']) => written.replace(['e', 'E'], "D"),
            Lang::Fortran => format!("{written}D0"),
            _ => written.to_string(),
        }
    }

    fn times(self) -> &'static str {
        if self == Lang::Matlab { ".*" } else { "*" }
    }

    fn over(self) -> &'static str {
        if self == Lang::Matlab { "./" } else { "/" }
    }

    fn unit(self) -> String {
        match self {
            Lang::C => "I".into(),
            Lang::Fortran => "(0.0D0,1.0D0)".into(),
            Lang::Matlab => "1i".into(),
        }
    }

    fn constant(self, name: &str) -> Option<String> {
        Some(match (name, self) {
            ("pi", Lang::C) => "M_PI".into(),
            ("pi", Lang::Fortran) => "(4.0D0*atan(1.0D0))".into(),
            ("pi", Lang::Matlab) => "pi".into(),
            ("e", Lang::C) => "M_E".into(),
            ("e", Lang::Fortran) => "exp(1.0D0)".into(),
            ("e", Lang::Matlab) => "exp(1)".into(),
            ("inf", Lang::C) => "INFINITY".into(),
            ("inf", Lang::Fortran) => "huge(1.0D0)".into(),
            ("inf", Lang::Matlab) => "Inf".into(),
            ("undefined", Lang::C) => "NAN".into(),
            ("undefined", Lang::Matlab) => "NaN".into(),
            _ => return None,
        })
    }

    /// The name of a function of this program in the language; none where the language
    /// has no such function.
    fn function(self, name: &str) -> Option<&'static str> {
        let same = [
            "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "asinh", "acosh",
            "atanh", "exp", "sqrt", "floor", "erf", "erfc", "atan2",
        ];
        if let Some(known) = same.iter().find(|s| **s == name) {
            return Some(known);
        }
        Some(match (name, self) {
            ("ln" | "log", _) => "log",
            ("abs", Lang::C) => "fabs",
            ("abs", _) => "abs",
            ("gamma", Lang::C) => "tgamma",
            ("gamma", _) => "gamma",
            ("lgamma", Lang::C) => "lgamma",
            ("lgamma", Lang::Fortran) => "log_gamma",
            ("lgamma", Lang::Matlab) => "gammaln",
            ("ceil", Lang::Fortran) => "ceiling",
            ("ceil", _) => "ceil",
            ("round", Lang::Fortran) => "anint",
            ("round", _) => "round",
            ("max", Lang::C) => "fmax",
            ("max", _) => "max",
            ("min", Lang::C) => "fmin",
            ("min", _) => "min",
            ("mod", Lang::C) => "fmod",
            ("mod", _) => "mod",
            ("sign", Lang::Matlab) => "sign",
            ("heaviside", Lang::Matlab) => "heaviside",
            ("besselj", Lang::Matlab) => "besselj",
            ("bessely", Lang::Matlab) => "bessely",
            ("real", Lang::C) => "creal",
            ("imag", Lang::C) => "cimag",
            ("real", _) => "real",
            ("imag", Lang::Fortran) => "aimag",
            ("imag", Lang::Matlab) => "imag",
            ("conj", Lang::Fortran) => "conjg",
            ("conj", _) => "conj",
            _ => return None,
        })
    }
}

fn wrap((code, level): (String, u8), least: u8) -> String {
    if level < least { format!("({code})") } else { code }
}

/// A name of this program as a name of the language: letters, digits and `_`.
fn plain_name(name: &str) -> String {
    let fits = name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    match fits {
        true => name.to_string(),
        false => name
            .chars()
            .map(|c| match c.is_ascii_alphanumeric() {
                true => c.to_string(),
                false => format!("_u{:x}_", c as u32),
            })
            .collect(),
    }
}

fn code(e: &Expr, lang: Lang) -> Result<(String, u8), String> {
    Ok(match e {
        Expr::Num(n) => {
            let part = |r: &crate::num::Rat| -> (String, u8) {
                let (negative, top, bottom) = (r.is_negative(), r.numer().clone().abs(), r.denom().clone());
                let body = match bottom == 1 {
                    true => lang.whole(&top.to_string()),
                    false => format!("{}/{}", lang.whole(&top.to_string()), lang.whole(&bottom.to_string())),
                };
                match (negative, bottom == 1) {
                    (true, _) => (format!("-{body}"), NEG.min(MUL)),
                    (false, true) => (body, ATOM),
                    (false, false) => (body, MUL),
                }
            };
            if n.is_real() {
                return Ok(part(&n.re));
            }
            // the imaginary part without its sign, then the sign
            let size = crate::num::Rat::from(n.im.clone().abs());
            let (imaginary, level) = match size == 1 {
                true => (lang.unit(), ATOM),
                false => (format!("{}{}{}", wrap(part(&size), POW), lang.times(), lang.unit()), MUL),
            };
            match (n.re.is_zero(), n.im.is_negative()) {
                (true, false) => (imaginary, level),
                (true, true) => (format!("-{imaginary}"), MUL),
                (false, true) => (format!("{} - {imaginary}", part(&n.re).0), ADD),
                (false, false) => (format!("{} + {imaginary}", part(&n.re).0), ADD),
            }
        }
        Expr::Float(f) => {
            let written = format!("{e}");
            match f.v.imag().is_zero() {
                true if written.starts_with('-') => (lang.decimal(&written), NEG.min(MUL)),
                true => (lang.decimal(&written), ATOM),
                false => return Err(format!("{}: a decimal number with an imaginary part is written as a + b*i", lang.who())),
            }
        }
        Expr::Sym(name) => match &**name {
            "i" => (lang.unit(), ATOM),
            known if crate::expr::is_const_name(known) => match lang.constant(known) {
                Some(code) => (code, ATOM),
                None => {
                    // a constant that the language has no name for: its digits
                    let value = crate::numeric::n(e, 17);
                    (lang.decimal(&format!("{value}")), ATOM)
                }
            },
            other => (plain_name(other), ATOM),
        },
        Expr::Add(terms) => {
            let mut out = String::new();
            for (k, term) in crate::printer::ordered_terms(terms).iter().enumerate() {
                let (code, level) = code(term, lang)?;
                match (k, code.strip_prefix('-')) {
                    (0, _) => out.push_str(&wrap((code, level), ADD)),
                    (_, Some(rest)) if level >= MUL => out.push_str(&format!(" - {rest}")),
                    _ => out.push_str(&format!(" + {}", wrap((code, level), MUL))),
                }
            }
            (out, ADD)
        }
        Expr::Mul(factors) => {
            let (mut negative, mut top, mut bottom) = (false, Vec::new(), Vec::new());
            for factor in factors.iter() {
                match factor {
                    Expr::Num(n) if n.is_real() => {
                        let r = &n.re;
                        negative ^= r.is_negative();
                        let (p, q) = (r.numer().clone().abs(), r.denom().clone());
                        if p != 1 {
                            top.push(lang.whole(&p.to_string()));
                        }
                        if q != 1 {
                            bottom.push(lang.whole(&q.to_string()));
                        }
                    }
                    Expr::Pow(base, power) if power.as_rat().is_some_and(|r| r.is_negative()) => {
                        let turned = crate::canon::pow((**base).clone(), crate::canon::neg((**power).clone()));
                        bottom.push(wrap(code(&turned, lang)?, POW));
                    }
                    other => top.push(wrap(code(other, lang)?, POW)),
                }
            }
            if top.is_empty() {
                top.push(lang.whole("1"));
            }
            let mut out = top.join(lang.times());
            match bottom.len() {
                0 => {}
                1 => out = format!("{out}{}{}", lang.over(), bottom[0]),
                _ => out = format!("{out}{}({})", lang.over(), bottom.join(lang.times())),
            }
            match negative {
                true => (format!("-{out}"), MUL),
                false => (out, MUL),
            }
        }
        Expr::Pow(base, power) => {
            if **base == Expr::e() {
                return Ok((format!("exp({})", code(power, lang)?.0), ATOM));
            }
            let half = power.as_rat().is_some_and(|r| *r.numer() == 1 && *r.denom() == 2);
            let whole = power.as_i64();
            match (half, whole) {
                (true, _) => (format!("sqrt({})", code(base, lang)?.0), ATOM),
                (_, Some(-1)) => (format!("{}{}{}", lang.whole("1"), lang.over(), wrap(code(base, lang)?, POW)), MUL),
                (_, Some(n)) if n < 0 => {
                    let turned = crate::canon::pow((**base).clone(), Expr::int(-n));
                    (format!("{}{}{}", lang.whole("1"), lang.over(), wrap(code(&turned, lang)?, POW)), MUL)
                }
                (_, Some(2)) if lang == Lang::C => {
                    let b = wrap(code(base, lang)?, ATOM);
                    (format!("{b}*{b}"), MUL)
                }
                (_, Some(n)) if lang != Lang::C => {
                    let operator = if lang == Lang::Matlab { ".^" } else { "**" };
                    (format!("{}{operator}{n}", wrap(code(base, lang)?, ATOM)), POW)
                }
                _ => match lang {
                    Lang::C => (format!("pow({}, {})", code(base, lang)?.0, code(power, lang)?.0), ATOM),
                    Lang::Fortran => (format!("{}**{}", wrap(code(base, lang)?, ATOM), wrap(code(power, lang)?, ATOM)), POW),
                    Lang::Matlab => (format!("{}.^{}", wrap(code(base, lang)?, ATOM), wrap(code(power, lang)?, ATOM)), POW),
                },
            }
        }
        Expr::Fn(name, args) => {
            let inside: Vec<String> = args
                .iter()
                .map(|a| code(a, lang).map(|c| c.0))
                .collect::<Result<_, _>>()?;
            match (&**name, lang, &inside[..]) {
                ("sign", Lang::C, [x]) => (format!("(({x} > 0.0) - ({x} < 0.0))"), ATOM),
                ("sign", Lang::Fortran, [x]) => (format!("sign(1.0D0, {x})"), ATOM),
                ("heaviside", Lang::C, [x]) => (format!("(({x}) >= 0.0 ? 1.0 : 0.0)"), ATOM),
                _ => match lang.function(name) {
                    Some(known) => (format!("{known}({})", inside.join(", ")), ATOM),
                    None if crate::names::function(name).is_some() => {
                        return Err(format!(
                            "{}: {} has no function for {name}",
                            lang.who(),
                            match lang {
                                Lang::C => "C",
                                Lang::Fortran => "Fortran",
                                Lang::Matlab => "MATLAB",
                            }
                        ));
                    }
                    // a function without a meaning of its own stays as it is called
                    None => (format!("{}({})", plain_name(name), inside.join(", ")), ATOM),
                },
            }
        }
        Expr::Rel(op, left, right) => {
            let sign = match (op, lang) {
                (Rel::Eq, Lang::Fortran) => ".eq.",
                (Rel::Ne, Lang::Fortran) => ".ne.",
                (Rel::Lt, Lang::Fortran) => ".lt.",
                (Rel::Le, Lang::Fortran) => ".le.",
                (Rel::Gt, Lang::Fortran) => ".gt.",
                (Rel::Ge, Lang::Fortran) => ".ge.",
                (Rel::Eq, _) => "==",
                (Rel::Ne, Lang::Matlab) => "~=",
                (Rel::Ne, _) => "!=",
                (Rel::Lt, _) => "<",
                (Rel::Le, _) => "<=",
                (Rel::Gt, _) => ">",
                (Rel::Ge, _) => ">=",
            };
            (format!("{} {sign} {}", wrap(code(left, lang)?, ADD), wrap(code(right, lang)?, ADD)), 0)
        }
        Expr::Bool(truth) => match (lang, truth) {
            (Lang::Fortran, true) => (".true.".into(), ATOM),
            (Lang::Fortran, false) => (".false.".into(), ATOM),
            (Lang::Matlab, true) => ("true".into(), ATOM),
            (Lang::Matlab, false) => ("false".into(), ATOM),
            (Lang::C, true) => ("1".into(), ATOM),
            (Lang::C, false) => ("0".into(), ATOM),
        },
        Expr::List(_) => return Err(format!("{}: a list inside an expression has no code", lang.who())),
    })
}

/// The lines that set a name to an expression, to the entries of a list or of a matrix.
fn lines(e: &Expr, lang: Lang, name: &str) -> Result<String, String> {
    let set = |place: String, value: &Expr| -> Result<String, String> {
        let (value, _) = code(value, lang)?;
        Ok(match lang {
            Lang::C => format!("{name}{place} = {value};"),
            Lang::Fortran => format!("      {name}{place} = {value}"),
            Lang::Matlab => unreachable!("MATLAB code is an expression"),
        })
    };
    let at = |indices: &[usize]| -> String {
        match lang {
            Lang::C => indices.iter().map(|k| format!("[{k}]")).collect(),
            _ => format!("({})", indices.iter().map(|k| (k + 1).to_string()).collect::<Vec<_>>().join(",")),
        }
    };
    let mut out = Vec::new();
    match (crate::matrix::rows(e), e.as_list()) {
        (Some(rows), _) if rows.iter().all(|r| r.iter().all(|x| x.as_list().is_none())) && e.as_list().is_some_and(|l| l.iter().all(|r| r.as_list().is_some())) => {
            for (i, row) in rows.iter().enumerate() {
                for (j, value) in row.iter().enumerate() {
                    out.push(set(at(&[i, j]), value)?);
                }
            }
        }
        (_, Some(items)) => {
            for (k, value) in items.iter().enumerate() {
                out.push(set(at(&[k]), value)?);
            }
        }
        _ => out.push(set(String::new(), e)?),
    }
    Ok(out.join("\n"))
}

fn matlab(e: &Expr) -> Result<String, String> {
    match e.as_list() {
        Some(items) if !items.is_empty() && items.iter().all(|r| r.as_list().is_some()) => {
            let rows: Vec<String> = items
                .iter()
                .map(|r| {
                    let cells: Vec<String> = r
                        .as_list()
                        .unwrap_or_default()
                        .iter()
                        .map(|x| code(x, Lang::Matlab).map(|c| c.0))
                        .collect::<Result<_, _>>()?;
                    Ok(cells.join(", "))
                })
                .collect::<Result<_, String>>()?;
            Ok(format!("[{}]", rows.join("; ")))
        }
        Some(items) => {
            let cells: Vec<String> = items
                .iter()
                .map(|x| code(x, Lang::Matlab).map(|c| c.0))
                .collect::<Result<_, _>>()?;
            Ok(format!("[{}]", cells.join(", ")))
        }
        None => Ok(code(e, Lang::Matlab)?.0),
    }
}

// ---- parts that an expression has more than once ----

fn count_parts(e: &Expr, counts: &mut HashMap<Expr, (usize, usize)>) -> usize {
    let inner: usize = match e {
        Expr::Add(items) | Expr::Mul(items) | Expr::List(items) => items.iter().map(|x| count_parts(x, counts)).sum(),
        Expr::Fn(name, args) if &**name != "string" => args.iter().map(|x| count_parts(x, counts)).sum(),
        Expr::Pow(base, power) => count_parts(base, counts) + count_parts(power, counts),
        Expr::Rel(_, left, right) => count_parts(left, counts) + count_parts(right, counts),
        _ => return 1,
    };
    let size = inner + 1;
    if !matches!(e, Expr::List(_) | Expr::Rel(..)) {
        counts.entry(e.clone()).or_insert((0, size)).0 += 1;
    }
    size
}

fn replace(e: &Expr, part: &Expr, with: &Expr) -> Expr {
    if e == part {
        return with.clone();
    }
    match e {
        Expr::Add(items) => Expr::Add(items.iter().map(|x| replace(x, part, with)).collect()),
        Expr::Mul(items) => Expr::Mul(items.iter().map(|x| replace(x, part, with)).collect()),
        Expr::List(items) => Expr::list(items.iter().map(|x| replace(x, part, with)).collect()),
        Expr::Fn(name, args) => Expr::Fn(name.clone(), args.iter().map(|x| replace(x, part, with)).collect()),
        Expr::Pow(base, power) => Expr::Pow(replace(base, part, with).into(), replace(power, part, with).into()),
        Expr::Rel(op, left, right) => Expr::rel(*op, replace(left, part, with), replace(right, part, with)),
        other => other.clone(),
    }
}

/// The part that an expression has more than once and that saves most when it is named.
fn shared(e: &Expr) -> Option<Expr> {
    let mut counts = HashMap::new();
    count_parts(e, &mut counts);
    counts
        .into_iter()
        .filter(|(_, (count, size))| *count >= 2 && *size >= 2)
        .max_by(|(a, (ca, sa)), (b, (cb, sb))| {
            ((ca - 1) * sa)
                .cmp(&((cb - 1) * sb))
                .then(sa.cmp(sb))
                .then_with(|| b.cmp(a))
        })
        .map(|(part, _)| part)
}

// ---- a picture of text ----

/// Lines of one width, and the line that stands on the line of writing.
#[derive(Clone)]
struct Block {
    lines: Vec<String>,
    base: usize,
}

impl Block {
    fn word(s: &str) -> Block {
        Block {
            lines: vec![s.to_string()],
            base: 0,
        }
    }

    fn width(&self) -> usize {
        self.lines.first().map_or(0, |l| l.chars().count())
    }

    fn below(&self) -> usize {
        self.lines.len() - 1 - self.base
    }

    /// Blocks side by side, their lines of writing at one height.
    fn beside(parts: &[Block]) -> Block {
        let above = parts.iter().map(|p| p.base).max().unwrap_or(0);
        let below = parts.iter().map(Block::below).max().unwrap_or(0);
        let mut lines = vec![String::new(); above + below + 1];
        for part in parts {
            let top = above - part.base;
            for (k, line) in lines.iter_mut().enumerate() {
                match k.checked_sub(top).and_then(|i| part.lines.get(i)) {
                    Some(text) => line.push_str(text),
                    None => line.push_str(&" ".repeat(part.width())),
                }
            }
        }
        Block { lines, base: above }
    }

    fn padded(&self, width: usize) -> Block {
        let left = (width - self.width()) / 2;
        let right = width - self.width() - left;
        Block {
            lines: self
                .lines
                .iter()
                .map(|l| format!("{}{l}{}", " ".repeat(left), " ".repeat(right)))
                .collect(),
            base: self.base,
        }
    }

    /// One block over another with a bar between them.
    fn over(top: &Block, bottom: &Block) -> Block {
        let width = top.width().max(bottom.width());
        let mut lines = top.padded(width).lines;
        let base = lines.len();
        lines.push("-".repeat(width));
        lines.extend(bottom.padded(width).lines);
        Block { lines, base }
    }

    /// A block with a smaller one raised at its right.
    fn raised(&self, power: &Block) -> Block {
        let (wide, more) = (self.width(), power.width());
        let mut lines: Vec<String> = power
            .lines
            .iter()
            .map(|l| format!("{}{l}", " ".repeat(wide)))
            .collect();
        lines.extend(self.lines.iter().map(|l| format!("{l}{}", " ".repeat(more))));
        Block {
            lines,
            base: power.lines.len() + self.base,
        }
    }

    /// Brackets as tall as the block.
    fn bracketed(&self, round: bool) -> Block {
        let tall = self.lines.len();
        let side = |k: usize, left: bool| -> &'static str {
            match (round, tall, k) {
                (true, 1, _) => if left { "(" } else { ")" },
                (false, 1, _) => if left { "[" } else { "]" },
                (true, _, 0) => if left { "/" } else { "\\" },
                (true, _, k) if k + 1 == tall => if left { "\\" } else { "/" },
                _ => "|",
            }
        };
        Block {
            lines: self
                .lines
                .iter()
                .enumerate()
                .map(|(k, l)| format!("{}{l}{}", side(k, true), side(k, false)))
                .collect(),
            base: self.base,
        }
    }
}

fn block(e: &Expr) -> (Block, u8) {
    let wrapped = |e: &Expr, least: u8| -> Block {
        let (b, level) = block(e);
        if level < least { b.bracketed(true) } else { b }
    };
    match e {
        Expr::Num(n) if n.is_real() && *n.re.denom() != 1 => {
            let (top, bottom) = (n.re.numer().clone().abs(), n.re.denom().clone());
            let fraction = Block::over(&Block::word(&top.to_string()), &Block::word(&bottom.to_string()));
            match n.re.is_negative() {
                true => (Block::beside(&[Block::word("- "), fraction]), ADD),
                false => (fraction, ATOM),
            }
        }
        Expr::Add(terms) => {
            let mut parts = Vec::new();
            for (k, term) in crate::printer::ordered_terms(terms).iter().enumerate() {
                let negative = match term {
                    Expr::Mul(f) => f.first().and_then(Expr::as_rat).is_some_and(|r| r.is_negative()),
                    other => other.as_rat().is_some_and(|r| r.is_negative()),
                };
                let shown = if negative { crate::canon::neg(term.clone()) } else { term.clone() };
                parts.push(Block::word(match (k, negative) {
                    (0, true) => "- ",
                    (0, false) => "",
                    (_, true) => " - ",
                    (_, false) => " + ",
                }));
                parts.push(wrapped(&shown, MUL));
            }
            (Block::beside(&parts), ADD)
        }
        Expr::Mul(factors) => {
            let (mut negative, mut top, mut bottom) = (false, Vec::new(), Vec::new());
            for factor in factors.iter() {
                match factor {
                    Expr::Num(n) if n.is_real() => {
                        negative ^= n.re.is_negative();
                        let (p, q) = (n.re.numer().clone().abs(), n.re.denom().clone());
                        if p != 1 {
                            top.push(Expr::integer(p));
                        }
                        if q != 1 {
                            bottom.push(Expr::integer(q));
                        }
                    }
                    Expr::Pow(base, power) if power.as_rat().is_some_and(|r| r.is_negative()) => {
                        bottom.push(crate::canon::pow((**base).clone(), crate::canon::neg((**power).clone())));
                    }
                    other => top.push(other.clone()),
                }
            }
            let product = |factors: &[Expr]| -> Block {
                match factors {
                    [] => Block::word("1"),
                    [one] => wrapped(one, MUL),
                    many => {
                        let mut parts = Vec::new();
                        for (k, f) in many.iter().enumerate() {
                            if k > 0 {
                                parts.push(Block::word(" "));
                            }
                            parts.push(wrapped(f, POW));
                        }
                        Block::beside(&parts)
                    }
                }
            };
            let body = match bottom.is_empty() {
                true => product(&top),
                false => {
                    let over = |factors: &[Expr]| -> Block {
                        match factors {
                            [one] => block(one).0,
                            many => product(many),
                        }
                    };
                    Block::over(&over(&top), &over(&bottom))
                }
            };
            match negative {
                true => (Block::beside(&[Block::word("- "), body]), ADD),
                false => (body, MUL),
            }
        }
        Expr::Pow(base, power) => {
            if power.as_rat().is_some_and(|r| *r.numer() == 1 && *r.denom() == 2) {
                let inside = block(base).0;
                return (Block::beside(&[Block::word("sqrt"), inside.bracketed(true)]), ATOM);
            }
            if power.as_rat().is_some_and(|r| r.is_negative()) {
                let turned = crate::canon::pow((**base).clone(), crate::canon::neg((**power).clone()));
                return (Block::over(&Block::word("1"), &block(&turned).0), MUL);
            }
            (wrapped(base, ATOM).raised(&block(power).0), POW)
        }
        Expr::Fn(name, args) if &**name != "string" => {
            let mut parts = Vec::new();
            for (k, a) in args.iter().enumerate() {
                if k > 0 {
                    parts.push(Block::word(", "));
                }
                parts.push(block(a).0);
            }
            let inside = match parts.is_empty() {
                true => Block::word(""),
                false => Block::beside(&parts),
            };
            (Block::beside(&[Block::word(name), inside.bracketed(true)]), ATOM)
        }
        Expr::List(items) if !items.is_empty() && items.iter().all(|r| r.as_list().is_some_and(|l| l.len() == items[0].as_list().map_or(0, <[Expr]>::len))) => {
            // a matrix: the columns as wide as their widest entry, a free line between rows
            let cells: Vec<Vec<Block>> = items
                .iter()
                .map(|r| r.as_list().unwrap_or_default().iter().map(|x| block(x).0).collect())
                .collect();
            let columns = cells[0].len();
            let widths: Vec<usize> = (0..columns)
                .map(|j| cells.iter().map(|r| r[j].width()).max().unwrap_or(0))
                .collect();
            let mut lines = Vec::new();
            for (k, row) in cells.iter().enumerate() {
                let mut parts = Vec::new();
                for (j, cell) in row.iter().enumerate() {
                    if j > 0 {
                        parts.push(Block::word("  "));
                    }
                    parts.push(cell.padded(widths[j]));
                }
                if k > 0 {
                    lines.push(String::new());
                }
                lines.extend(Block::beside(&parts).lines);
            }
            let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
            let lines: Vec<String> = lines
                .into_iter()
                .map(|l| format!("{l}{}", " ".repeat(width - l.chars().count())))
                .collect();
            let base = lines.len() / 2;
            (Block { lines, base }.bracketed(false), ATOM)
        }
        Expr::List(items) => {
            let mut parts = Vec::new();
            for (k, x) in items.iter().enumerate() {
                if k > 0 {
                    parts.push(Block::word(", "));
                }
                parts.push(block(x).0);
            }
            let inside = match parts.is_empty() {
                true => Block::word(""),
                false => Block::beside(&parts),
            };
            (inside.bracketed(false), ATOM)
        }
        Expr::Rel(op, left, right) => {
            let sign = match op {
                Rel::Eq => " = ",
                Rel::Ne => " != ",
                Rel::Lt => " < ",
                Rel::Le => " <= ",
                Rel::Gt => " > ",
                Rel::Ge => " >= ",
            };
            (Block::beside(&[block(left).0, Block::word(sign), block(right).0]), 0)
        }
        other => {
            let written = format!("{other}");
            let level = if written.starts_with('-') { ADD } else { ATOM };
            (Block::word(&written), level)
        }
    }
}

/// The lines of an expression written as on paper: fractions with a bar, powers raised.
pub(crate) fn pretty(e: &Expr) -> Vec<String> {
    block(e)
        .0
        .lines
        .iter()
        .map(|l| l.trim_end().to_string())
        .collect()
}

impl Session {
    pub(crate) fn code_call(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        Some(match (name, a.len()) {
            ("ccode" | "fortran", 1 | 2) => (|| {
                let lang = if name == "ccode" { Lang::C } else { Lang::Fortran };
                let target = match a.get(1) {
                    Some(t) => text_of(t)
                        .or_else(|| t.sym_name())
                        .map(plain_name)
                        .ok_or_else(|| format!("{name}(expression[, \"name\"]): the name that is set"))?,
                    None => "t0".to_string(),
                };
                Ok(text(&lines(&a[0], lang, &target)?))
            })(),
            ("matlabcode", 1) => matlab(&a[0]).map(|code| text(&code)),
            ("matlabfunction", 1 | 3) => (|| {
                let usage = "matlabfunction(expression[, \"Vars\", [x, y]]): the function of its variables";
                let names: Vec<Expr> = match a.get(1) {
                    Some(option) if crate::arrays::word(option).as_deref() == Some("vars") => {
                        let given = match a[2].as_list() {
                            Some(items) => items.to_vec(),
                            None => vec![a[2].clone()],
                        };
                        match given.iter().all(|v| v.sym_name().is_some()) {
                            true => given,
                            false => return Err(usage.into()),
                        }
                    }
                    Some(_) => return Err(usage.into()),
                    None => a[0].free_syms().iter().map(|s| Expr::sym(s)).collect(),
                };
                if names.is_empty() {
                    return Err("matlabfunction: the expression has no variable; \"Vars\", [x] names one".into());
                }
                Ok(Expr::call("rule", vec![Expr::list(names), a[0].clone()]))
            })(),
            ("subexpr", 1 | 2) | ("cse", 1) => (|| {
                let given = match a.get(1) {
                    Some(n) => n
                        .sym_name()
                        .or_else(|| text_of(n))
                        .map(str::to_string)
                        .ok_or("subexpr(expression[, name]): the name of the shared part")?,
                    None => "sigma".to_string(),
                };
                if name == "subexpr" {
                    return Ok(match shared(&a[0]) {
                        Some(part) => {
                            let mark = Expr::sym(&given);
                            Expr::list(vec![replace(&a[0], &part, &mark), Expr::rel(Rel::Eq, mark, part)])
                        }
                        None => Expr::list(vec![a[0].clone(), Expr::list(Vec::new())]),
                    });
                }
                // every shared part in turn, the largest saving first
                let mut rest = a[0].clone();
                let mut named: Vec<(Expr, Expr)> = Vec::new();
                while named.len() < 1000 {
                    let Some(part) = shared(&rest) else { break };
                    let mark = Expr::sym(&format!("sigma{}", named.len() + 1));
                    rest = replace(&rest, &part, &mark);
                    for (_, value) in named.iter_mut() {
                        *value = replace(value, &part, &mark);
                    }
                    named.push((mark, part));
                }
                // a part may use the parts named after it: those come first
                named.reverse();
                Ok(Expr::list(vec![
                    Expr::list(named.into_iter().map(|(mark, value)| Expr::rel(Rel::Eq, mark, value)).collect()),
                    rest,
                ]))
            })(),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str) -> Expr {
        let parsed = crate::parser::parse(text).expect("an expression");
        Session::new().eval(&parsed.stmts[0]).expect("a value")
    }

    #[test]
    fn code_of_three_languages() {
        let e = parsed("x^2/3 + sqrt(y) - exp(-x)");
        let c = lines(&e, Lang::C, "t0").unwrap();
        assert!(c.starts_with("t0 = ") && c.ends_with(';'), "{c}");
        assert!(c.contains("(x*x)/3.0") && c.contains("sqrt(y)") && c.contains("exp(-x)"), "{c}");
        let f = lines(&e, Lang::Fortran, "t0").unwrap();
        assert!(f.contains("x**2/3.0D0"), "{f}");
        let m = matlab(&e).unwrap();
        assert!(m.contains("x.^2./3"), "{m}");
        assert!(code(&parsed("besselj(0, x)"), Lang::C).is_err());
    }

    #[test]
    fn a_shared_part_is_found() {
        let e = parsed("sin(x + y)^2 + cos(x + y) + 1/(x + y)");
        let part = shared(&e).expect("a part");
        assert_eq!(part, parsed("x + y"));
        assert!(shared(&parsed("x + y")).is_none());
    }

    #[test]
    fn fractions_stand_on_bars() {
        let lines = pretty(&parsed("(x + 1)/(y^2 + 3)"));
        assert_eq!(lines.len(), 4);
        assert!(lines.iter().any(|l| l.chars().all(|c| c == '-') && l.len() >= 5));
    }
}
