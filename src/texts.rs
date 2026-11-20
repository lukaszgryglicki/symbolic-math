//! Texts: taking them apart and putting them together, numbers as texts and back, formats.

use crate::arrays::{truthy, whole};
use crate::expr::Expr;
use crate::num::Rat;
use crate::numeric;
use rug::{Complex, Integer};
use std::cmp::Ordering;

type Res = Result<Expr, String>;

pub fn text(s: &str) -> Expr {
    Expr::call("string", vec![Expr::sym(s)])
}

fn string_of<'a>(e: &'a Expr, name: &str) -> Result<&'a str, String> {
    crate::radix::text_of(e).ok_or_else(|| format!("{name} needs a text in quotes, {e} is none"))
}

/// A function of a text, for a text or for every text of a list.
fn each(e: &Expr, name: &str, f: &dyn Fn(&str) -> Res) -> Res {
    match e {
        Expr::List(items) => Ok(Expr::list(
            items
                .iter()
                .map(|x| each(x, name, f))
                .collect::<Result<_, _>>()?,
        )),
        other => f(string_of(other, name)?),
    }
}

/// A function of an entry, for every entry of a list.
fn try_map(e: &Expr, f: &dyn Fn(&Expr) -> Res) -> Res {
    match e {
        Expr::List(items) => Ok(Expr::list(
            items
                .iter()
                .map(|x| try_map(x, f))
                .collect::<Result<_, _>>()?,
        )),
        other => f(other),
    }
}

fn value_of(e: &Expr, digits: u32) -> Result<Complex, String> {
    numeric::evalf_digits(e, digits).map_err(|err| err.to_string())
}

/// The digits that show a number as MATLAB's num2str does: four after the first of a number
/// below ten, more for larger ones.
fn shown_digits(v: &Complex) -> u32 {
    let size = v.real().clone().abs().max(&v.imag().clone().abs());
    if size.is_zero() || !size.is_finite() {
        return 5;
    }
    let lead = size.log10().floor().to_f64();
    ((lead as i64) + 5).clamp(5, 16) as u32
}

/// A number as a text; a whole number with all its digits, an expression as it prints.
pub fn num2str(x: &Expr, digits: Option<u32>) -> Result<String, String> {
    if let Some(s) = crate::radix::text_of(x) {
        return Ok(s.to_string());
    }
    if let Some(k) = x.as_int() {
        return Ok(k.to_string());
    }
    if let Expr::Bool(b) = x {
        return Ok(i32::from(*b).to_string());
    }
    if !x.is_const() || x.is_infinite() {
        return Ok(format!("{x}"));
    }
    let v = value_of(x, 60)?;
    let shown = digits.unwrap_or_else(|| shown_digits(&v));
    if v.imag().is_zero() {
        // (as %g of C writes it: with an exponent when the number is small or long)
        return sprintf(&format!("%.{shown}g"), std::slice::from_ref(x));
    }
    Ok(numeric::fmt_complex(&v, shown))
}

/// The digits of a whole number in a base, capitals above nine.
fn in_base(n: &Integer, base: i32, least: usize) -> String {
    let digits = n.to_string_radix(base).to_ascii_uppercase();
    format!("{}{digits}", "0".repeat(least.saturating_sub(digits.len())))
}

fn from_base(s: &str, base: i32, name: &str) -> Res {
    let clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    Integer::from_str_radix(&clean, base)
        .map(Expr::integer)
        .map_err(|_| format!("{name}: \"{s}\" is no number written in base {base}"))
}

/// The texts of whole numbers in a base, all as long as the longest.
fn to_base(e: &Expr, base: i32, least: usize, name: &str) -> Res {
    let digits = |x: &Expr| -> Result<Integer, String> {
        x.as_int()
            .filter(|k| k.cmp0() != Ordering::Less)
            .cloned()
            .ok_or_else(|| format!("{name} needs whole numbers that are not negative, {x} is none"))
    };
    match e {
        Expr::List(items) => {
            let all: Vec<Integer> = items.iter().map(digits).collect::<Result<_, _>>()?;
            let longest = all
                .iter()
                .map(|k| in_base(k, base, least).len())
                .max()
                .unwrap_or(least);
            Ok(Expr::list(
                all.iter()
                    .map(|k| text(&in_base(k, base, longest)))
                    .collect(),
            ))
        }
        other => Ok(text(&in_base(&digits(other)?, base, least))),
    }
}

/// A number with `places` digits after the point, halves away from zero.
fn fixed(x: &Rat, places: usize) -> String {
    let scale = Rat::from(Integer::from(Integer::u_pow_u(10, places as u32)));
    let scaled = Rat::from(x.clone().abs() * scale + Rat::from((1, 2)));
    let whole = scaled.floor().into_numer_denom().0.to_string();
    let whole = format!(
        "{}{whole}",
        "0".repeat((places + 1).saturating_sub(whole.len()))
    );
    let (head, tail) = whole.split_at(whole.len() - places);
    let negative = x.cmp0() == Ordering::Less && whole.chars().any(|c| c != '0');
    format!(
        "{}{head}{}{tail}",
        if negative { "-" } else { "" },
        if places > 0 { "." } else { "" }
    )
}

/// The first digit times a power of ten: `d.ddd` and the exponent.
fn scientific(x: &Rat, places: usize) -> (String, i64) {
    if x.cmp0() == Ordering::Equal {
        return (fixed(x, places), 0);
    }
    let size = x.clone().abs();
    let mut exp = (size.numer().to_string().len() as i64) - (size.denom().to_string().len() as i64);
    let ten = |k: i64| -> Rat {
        let p = Rat::from(Integer::from(Integer::u_pow_u(10, k.unsigned_abs() as u32)));
        if k >= 0 { p } else { p.recip() }
    };
    while &size / ten(exp) >= 10 {
        exp += 1;
    }
    while &size / ten(exp) < 1 {
        exp -= 1;
    }
    let mut body = fixed(&Rat::from(x / ten(exp)), places);
    if body.trim_start_matches('-').starts_with("10") {
        exp += 1;
        body = fixed(&Rat::from(x / ten(exp)), places);
    }
    (body, exp)
}

fn exponent(exp: i64) -> String {
    format!("e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs())
}

fn rational(e: &Expr) -> Result<Rat, String> {
    if let Some(r) = e.as_rat() {
        return Ok(r.clone());
    }
    if let Expr::Bool(b) = e {
        return Ok(Rat::from(i32::from(*b)));
    }
    let v = value_of(e, 60)?;
    if !v.imag().is_zero() {
        return Err(format!("a format for real numbers got {e}"));
    }
    v.real()
        .to_rational()
        .ok_or_else(|| format!("{e} is not a finite number"))
}

/// One value in the form of one `%` of a format.
fn formatted(spec: &str, kind: char, arg: &Expr) -> Result<String, String> {
    let (flags, rest): (String, String) = {
        let split = spec
            .find(|c: char| !matches!(c, '-' | '+' | ' ' | '0' | '#'))
            .unwrap_or(spec.len());
        (spec[..split].to_string(), spec[split..].to_string())
    };
    let (width, precision) = match rest.split_once('.') {
        Some((w, p)) => (w.parse::<usize>().unwrap_or(0), p.parse::<usize>().ok()),
        None => (rest.parse::<usize>().unwrap_or(0), None),
    };
    let mut body = match kind {
        's' => match crate::radix::text_of(arg) {
            Some(s) => s.to_string(),
            None => num2str(arg, None)?,
        },
        'c' => match (crate::radix::text_of(arg), arg.as_i64()) {
            (Some(s), _) => s.to_string(),
            (None, Some(code)) => char::from_u32(code as u32)
                .map(String::from)
                .ok_or_else(|| format!("{code} is no character"))?,
            _ => return Err(format!("%c needs a character or its number, not {arg}")),
        },
        'd' | 'i' | 'u' => match arg.as_int() {
            Some(k) => k.to_string(),
            None if matches!(arg, Expr::Bool(_)) => num2str(arg, None)?,
            None => {
                let (body, exp) = scientific(&rational(arg)?, precision.unwrap_or(6));
                format!("{body}{}", exponent(exp))
            }
        },
        'x' | 'X' | 'o' => {
            let k = arg
                .as_int()
                .filter(|k| k.cmp0() != Ordering::Less)
                .ok_or_else(|| {
                    format!("%{kind} needs a whole number that is not negative, not {arg}")
                })?;
            let s = k.to_string_radix(if kind == 'o' { 8 } else { 16 });
            if kind == 'X' {
                s.to_ascii_uppercase()
            } else {
                s
            }
        }
        'f' => fixed(&rational(arg)?, precision.unwrap_or(6)),
        'e' | 'E' => {
            let (body, exp) = scientific(&rational(arg)?, precision.unwrap_or(6));
            let s = format!("{body}{}", exponent(exp));
            if kind == 'E' {
                s.to_ascii_uppercase()
            } else {
                s
            }
        }
        'g' | 'G' => {
            let x = rational(arg)?;
            let digits = precision.unwrap_or(6).max(1);
            let (body, exp) = scientific(&x, digits - 1);
            let trim = |s: String| {
                if s.contains('.') {
                    s.trim_end_matches('0').trim_end_matches('.').to_string()
                } else {
                    s
                }
            };
            let s = if exp < -4 || exp >= digits as i64 {
                format!("{}{}", trim(body), exponent(exp))
            } else {
                trim(fixed(&x, (digits as i64 - 1 - exp).max(0) as usize))
            };
            if kind == 'G' {
                s.to_ascii_uppercase()
            } else {
                s
            }
        }
        other => return Err(format!("%{other} is no format")),
    };
    if flags.contains('+') && !body.starts_with('-') && !matches!(kind, 's' | 'c') {
        body.insert(0, '+');
    } else if flags.contains(' ') && !body.starts_with('-') && !matches!(kind, 's' | 'c') {
        body.insert(0, ' ');
    }
    let len = body.chars().count();
    if len < width {
        let pad = width - len;
        if flags.contains('-') {
            body.push_str(&" ".repeat(pad));
        } else if flags.contains('0') && !matches!(kind, 's' | 'c') {
            let at = usize::from(body.starts_with(['-', '+', ' ']));
            body.insert_str(at, &"0".repeat(pad));
        } else {
            body.insert_str(0, &" ".repeat(pad));
        }
    }
    Ok(body)
}

/// The text of a format with its values: `%d %i %u %f %e %g %s %c %x %o %%`, widths and
/// digits as in C, `\n` and `\t`; a format with fewer places than values is used again.
pub fn sprintf(format: &str, args: &[Expr]) -> Result<String, String> {
    let mut flat = Vec::new();
    fn spread(e: &Expr, out: &mut Vec<Expr>) {
        match e {
            Expr::List(items) => items.iter().for_each(|x| spread(x, out)),
            other => out.push(other.clone()),
        }
    }
    args.iter().for_each(|a| spread(a, &mut flat));
    let chars: Vec<char> = format.chars().collect();
    let mut out = String::new();
    let mut used = 0;
    loop {
        let before = used;
        let mut i = 0;
        let mut starved = false;
        while i < chars.len() {
            match chars[i] {
                '\\' if i + 1 < chars.len() => {
                    out.push(match chars[i + 1] {
                        'n' => '\n',
                        't' => '\t',
                        other => other,
                    });
                    i += 2;
                }
                '%' if chars.get(i + 1) == Some(&'%') => {
                    out.push('%');
                    i += 2;
                }
                '%' => {
                    let end = (i + 1..chars.len())
                        .find(|k| chars[*k].is_ascii_alphabetic())
                        .ok_or("the format ends in the middle of a %")?;
                    if used >= flat.len() {
                        starved = true;
                        break;
                    }
                    let spec: String = chars[i + 1..end].iter().collect();
                    out.push_str(&formatted(&spec, chars[end], &flat[used])?);
                    used += 1;
                    i = end + 1;
                }
                c => {
                    out.push(c);
                    i += 1;
                }
            }
        }
        if starved || used >= flat.len() || used == before {
            break;
        }
    }
    Ok(out)
}

/// A matrix as MATLAB writes one: `[1 2;3 4]`.
fn mat2str(e: &Expr, digits: u32) -> Result<String, String> {
    let one = |x: &Expr| num2str(x, Some(digits)).map(|s| s.replace(' ', ""));
    match crate::arrays::arr(e) {
        crate::arrays::Arr::Scalar(x) => one(&x),
        crate::arrays::Arr::Vector(v) => Ok(format!(
            "[{}]",
            v.iter().map(one).collect::<Result<Vec<_>, _>>()?.join(" ")
        )),
        crate::arrays::Arr::Matrix(m) => {
            let rows: Vec<String> = m
                .iter()
                .map(|r| {
                    Ok(r.iter()
                        .map(one)
                        .collect::<Result<Vec<_>, String>>()?
                        .join(" "))
                })
                .collect::<Result<_, String>>()?;
            Ok(format!("[{}]", rows.join(";")))
        }
    }
}

/// The first place at or after `from` where a part stands in a text.
fn found(hay: &[char], part: &[char], from: usize) -> Option<usize> {
    if part.is_empty() || part.len() > hay.len() {
        return None;
    }
    (from..=hay.len() - part.len()).find(|k| hay[*k..*k + part.len()] == *part)
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    let is_text = |k: usize| a.get(k).is_some_and(|x| crate::radix::text_of(x).is_some());
    let s = |k: usize| string_of(&a[k], name);
    let fold = |k: usize, lower: bool| -> Result<String, String> {
        Ok(if lower {
            s(k)?.to_lowercase()
        } else {
            s(k)?.to_string()
        })
    };
    Ok(Some(match (name, n) {
        ("num2str", 1 | 2) => {
            let digits = match a.get(1) {
                Some(d) => Some(whole(d, "the number of digits", 1, 100_000)? as u32),
                None => None,
            };
            try_map(&a[0], &|x| Ok(text(&num2str(x, digits)?)))?
        }
        ("int2str", 1) => try_map(&a[0], &|x| {
            let r = crate::funcs::func("round", vec![x.clone()]);
            Ok(text(&num2str(&r, None)?))
        })?,
        ("mat2str", 1 | 2) => {
            let digits = if n == 2 {
                whole(&a[1], "the number of digits", 1, 100_000)? as u32
            } else {
                15
            };
            text(&mat2str(&a[0], digits)?)
        }
        ("tostring", 1) => try_map(&a[0], &|x| {
            Ok(text(&match crate::radix::text_of(x) {
                Some(t) => t.to_string(),
                None => format!("{x}"),
            }))
        })?,
        ("dec2bin" | "dec2hex", 1 | 2) => {
            let least = if n == 2 {
                whole(&a[1], "the number of digits", 0, 1_000_000)? as usize
            } else {
                1
            };
            to_base(&a[0], if name == "dec2bin" { 2 } else { 16 }, least, name)?
        }
        ("dec2base", 2 | 3) => {
            let base = whole(&a[1], "the base", 2, 36)? as i32;
            let least = if n == 3 {
                whole(&a[2], "the number of digits", 0, 1_000_000)? as usize
            } else {
                1
            };
            to_base(&a[0], base, least, name)?
        }
        ("bin2dec" | "hex2dec", 1) => {
            let base = if name == "bin2dec" { 2 } else { 16 };
            each(&a[0], name, &|t| from_base(t, base, name))?
        }
        ("base2dec", 2) => {
            let base = whole(&a[1], "the base", 2, 36)? as i32;
            each(&a[0], name, &|t| from_base(t, base, name))?
        }
        ("char", 1..) => {
            let mut out = String::new();
            fn add(e: &Expr, out: &mut String) -> Result<(), String> {
                match e {
                    Expr::List(items) => items.iter().try_for_each(|x| add(x, out)),
                    other => {
                        if let Some(t) = crate::radix::text_of(other) {
                            out.push_str(t);
                            return Ok(());
                        }
                        let code = other
                            .as_i64()
                            .and_then(|c| u32::try_from(c).ok())
                            .and_then(char::from_u32)
                            .ok_or_else(|| format!("char: {other} is no number of a character"))?;
                        out.push(code);
                        Ok(())
                    }
                }
            }
            for x in a {
                add(x, &mut out)?;
            }
            text(&out)
        }
        ("double", 1) => match crate::radix::text_of(&a[0]) {
            Some(t) => {
                let codes: Vec<Expr> = t
                    .chars()
                    .map(|c| Expr::int(i64::from(u32::from(c))))
                    .collect();
                if codes.len() == 1 {
                    codes[0].clone()
                } else {
                    Expr::list(codes)
                }
            }
            None => numeric::n(&a[0], numeric::digits()),
        },
        ("strcat", 1..) => {
            let mut out = String::new();
            for x in a {
                out.push_str(&num2str(x, None)?);
            }
            text(&out)
        }
        ("join", 1..) if (0..n).all(is_text) => text(
            &a.iter()
                .filter_map(crate::radix::text_of)
                .collect::<String>(),
        ),
        ("strlength", 1) => each(&a[0], name, &|t| Ok(Expr::int(t.chars().count() as i64)))?,
        ("length", 1) if is_text(0) => Expr::int(s(0)?.chars().count() as i64),
        ("reverse" | "flip" | "fliplr", 1) if is_text(0) => {
            text(&s(0)?.chars().rev().collect::<String>())
        }
        ("upper", 1) => each(&a[0], name, &|t| Ok(text(&t.to_uppercase())))?,
        ("lower", 1) => each(&a[0], name, &|t| Ok(text(&t.to_lowercase())))?,
        ("strtrim", 1) => each(&a[0], name, &|t| Ok(text(t.trim())))?,
        ("erase", 2) => {
            let part = s(1)?;
            if part.is_empty() {
                a[0].clone()
            } else {
                each(&a[0], name, &|t| Ok(text(&t.replace(part, ""))))?
            }
        }
        ("extractbefore" | "extractafter", 2) => {
            let before = name == "extractbefore";
            each(&a[0], name, &|t| {
                let chars: Vec<char> = t.chars().collect();
                let cut = match a[1].as_i64() {
                    // (a place: the characters before it, or those after it)
                    Some(k) if (1..=chars.len() as i64 + i64::from(before)).contains(&k) => {
                        Some(if before {
                            (k as usize - 1, k as usize - 1)
                        } else {
                            (k as usize, k as usize)
                        })
                    }
                    Some(k) => {
                        return Err(format!(
                            "{name}: the place {k} is not in a text of {} characters",
                            chars.len()
                        ));
                    }
                    None => {
                        let part: Vec<char> = string_of(&a[1], name)?.chars().collect();
                        found(&chars, &part, 0).map(|at| (at, at + part.len()))
                    }
                };
                Ok(match cut {
                    Some((start, _)) if before => text(&chars[..start].iter().collect::<String>()),
                    Some((_, end)) => text(&chars[end..].iter().collect::<String>()),
                    None => Expr::undefined(),
                })
            })?
        }
        ("extractbetween", 3) => each(&a[0], name, &|t| {
            let chars: Vec<char> = t.chars().collect();
            if let (Some(from), Some(to)) = (a[1].as_i64(), a[2].as_i64()) {
                if from < 1 || to > chars.len() as i64 || from > to + 1 {
                    return Err(format!(
                        "{name}: the places {from} and {to} are not in a text of {} characters",
                        chars.len()
                    ));
                }
                return Ok(text(
                    &chars[from as usize - 1..to as usize]
                        .iter()
                        .collect::<String>(),
                ));
            }
            let (open, close): (Vec<char>, Vec<char>) = (
                string_of(&a[1], name)?.chars().collect(),
                string_of(&a[2], name)?.chars().collect(),
            );
            let mut out = Vec::new();
            let mut at = 0;
            while let Some(start) = found(&chars, &open, at) {
                let inside = start + open.len();
                let Some(end) = found(&chars, &close, inside) else {
                    break;
                };
                out.push(text(&chars[inside..end].iter().collect::<String>()));
                at = end + close.len().max(1);
            }
            Ok(if out.len() == 1 {
                out.remove(0)
            } else {
                Expr::list(out)
            })
        })?,
        ("insertafter" | "insertbefore", 3) => {
            let after = name == "insertafter";
            let new = s(2)?;
            each(&a[0], name, &|t| {
                let chars: Vec<char> = t.chars().collect();
                let mut places: Vec<usize> = match a[1].as_i64() {
                    Some(k)
                        if (i64::from(!after)..=chars.len() as i64 + i64::from(!after))
                            .contains(&k) =>
                    {
                        vec![if after { k as usize } else { k as usize - 1 }]
                    }
                    Some(k) => {
                        return Err(format!(
                            "{name}: the place {k} is not in a text of {} characters",
                            chars.len()
                        ));
                    }
                    None => {
                        let part: Vec<char> = string_of(&a[1], name)?.chars().collect();
                        let mut all = Vec::new();
                        let mut at = 0;
                        while let Some(start) = found(&chars, &part, at) {
                            all.push(if after { start + part.len() } else { start });
                            at = start + part.len().max(1);
                        }
                        all
                    }
                };
                places.sort_unstable();
                let mut out = String::new();
                for (k, c) in chars.iter().enumerate() {
                    if places.contains(&k) {
                        out.push_str(new);
                    }
                    out.push(*c);
                }
                if places.contains(&chars.len()) {
                    out.push_str(new);
                }
                Ok(text(&out))
            })?
        }
        ("pad", 2..=4) => {
            let width = whole(&a[1], "the length", 0, 1_000_000)? as usize;
            let (mut side, mut fill) = ("right".to_string(), ' ');
            for extra in &a[2..] {
                match string_of(extra, name)? {
                    w @ ("left" | "right" | "both") => side = w.to_string(),
                    one if one.chars().count() == 1 => fill = one.chars().next().unwrap_or(' '),
                    other => {
                        return Err(format!(
                            "pad(text, length, side, character): the side is \"left\", \"right\" or \"both\", the character is one character, not \"{other}\""
                        ));
                    }
                }
            }
            each(&a[0], name, &|t| {
                let missing = width.saturating_sub(t.chars().count());
                let (left, right) = match side.as_str() {
                    "left" => (missing, 0),
                    "both" => (missing / 2, missing - missing / 2),
                    _ => (0, missing),
                };
                let run = |k: usize| fill.to_string().repeat(k);
                Ok(text(&format!("{}{t}{}", run(left), run(right))))
            })?
        }
        ("isstrprop", 2) => {
            let kind = s(1)?;
            let test: fn(char) -> bool = match kind {
                "alpha" => char::is_alphabetic,
                "digit" => |c: char| c.is_ascii_digit(),
                "alphanum" => char::is_alphanumeric,
                "upper" => char::is_uppercase,
                "lower" => char::is_lowercase,
                "wspace" => char::is_whitespace,
                "punct" => |c: char| c.is_ascii_punctuation(),
                "xdigit" => |c: char| c.is_ascii_hexdigit(),
                other => {
                    return Err(format!(
                        "isstrprop(text, kind): the kinds are \"alpha\", \"digit\", \"alphanum\", \"upper\", \"lower\", \"wspace\", \"punct\" and \"xdigit\", not \"{other}\""
                    ));
                }
            };
            each(&a[0], name, &|t| {
                Ok(Expr::list(t.chars().map(|c| Expr::Bool(test(c))).collect()))
            })?
        }
        ("strrep", 3) => {
            let (old, new) = (s(1)?, s(2)?);
            if old.is_empty() {
                a[0].clone()
            } else {
                each(&a[0], name, &|t| Ok(text(&t.replace(old, new))))?
            }
        }
        ("strfind", 2) => {
            let (hay, pat): (Vec<char>, Vec<char>) =
                (s(0)?.chars().collect(), s(1)?.chars().collect());
            let mut out = Vec::new();
            if !pat.is_empty() && pat.len() <= hay.len() {
                for k in 0..=hay.len() - pat.len() {
                    if hay[k..k + pat.len()] == pat[..] {
                        out.push(Expr::int(k as i64 + 1));
                    }
                }
            }
            Expr::list(out)
        }
        ("strsplit", 1 | 2) => {
            let t = s(0)?;
            let parts: Vec<&str> = match a.get(1) {
                None => t.split_whitespace().collect(),
                Some(d) => {
                    let d = string_of(d, name)?;
                    if d.is_empty() {
                        return Err("strsplit: the text between the parts must not be empty".into());
                    }
                    t.split(d).collect()
                }
            };
            Expr::list(parts.into_iter().map(text).collect())
        }
        ("strjoin", 1 | 2) => {
            let items = a[0].as_list().ok_or("strjoin([texts], between)")?;
            let between = if n == 2 {
                s(1)?.to_string()
            } else {
                " ".to_string()
            };
            let parts: Vec<String> = items
                .iter()
                .map(|x| num2str(x, None))
                .collect::<Result<_, _>>()?;
            text(&parts.join(&between))
        }
        ("strcmp" | "strcmpi", 2) => {
            let lower = name == "strcmpi";
            Expr::Bool(is_text(0) && is_text(1) && fold(0, lower)? == fold(1, lower)?)
        }
        ("strncmp" | "strncmpi", 3) => {
            let lower = name == "strncmpi";
            let count = whole(&a[2], "the number of characters", 0, 1_000_000_000)? as usize;
            let head = |k: usize| -> Result<Option<String>, String> {
                let t: Vec<char> = fold(k, lower)?.chars().collect();
                Ok((t.len() >= count).then(|| t[..count].iter().collect()))
            };
            Expr::Bool(
                is_text(0) && is_text(1) && {
                    let (x, y) = (head(0)?, head(1)?);
                    x.is_some() && x == y
                },
            )
        }
        ("contains", 2) if is_text(0) && is_text(1) => Expr::Bool(s(0)?.contains(s(1)?)),
        ("count", 2) if is_text(0) && is_text(1) => {
            let pat = s(1)?;
            Expr::int(if pat.is_empty() {
                0
            } else {
                s(0)?.matches(pat).count() as i64
            })
        }
        ("startswith", 2) => Expr::Bool(s(0)?.starts_with(s(1)?)),
        ("endswith", 2) => Expr::Bool(s(0)?.ends_with(s(1)?)),
        ("blanks", 1) => {
            text(&" ".repeat(whole(&a[0], "the number of blanks", 0, 1_000_000)? as usize))
        }
        ("isletter", 1) => Expr::list(
            s(0)?
                .chars()
                .map(|c| Expr::Bool(c.is_alphabetic()))
                .collect(),
        ),
        ("isspace", 1) => Expr::list(
            s(0)?
                .chars()
                .map(|c| Expr::Bool(c.is_whitespace()))
                .collect(),
        ),
        ("ischar" | "isstring", 1) => Expr::Bool(is_text(0)),
        ("isnumeric", 1) => {
            let mut all = true;
            a[0].walk(&mut |x| {
                all &= match x {
                    Expr::List(_) => true,
                    Expr::Num(_) | Expr::Float(_) => true,
                    other => {
                        other.is_const()
                            && !matches!(other, Expr::Bool(_) | Expr::Rel(..))
                            && crate::radix::text_of(other).is_none()
                    }
                }
            });
            Expr::Bool(all && crate::radix::text_of(&a[0]).is_none())
        }
        ("islogical", 1) => {
            let mut leaves = Vec::new();
            fn spread(e: &Expr, out: &mut Vec<Expr>) {
                match e {
                    Expr::List(items) => items.iter().for_each(|x| spread(x, out)),
                    other => out.push(other.clone()),
                }
            }
            spread(&a[0], &mut leaves);
            Expr::Bool(!leaves.is_empty() && leaves.iter().all(|x| matches!(x, Expr::Bool(_))))
        }
        ("logical", 1) => try_map(&crate::arrays::threaded(&a[0]), &|x| {
            Ok(Expr::Bool(truthy(x)?))
        })?,
        ("sprintf", 1..) => text(&sprintf(s(0)?, &a[1..])?),
        ("rats", 1 | 2) => {
            let r = crate::elem::call("rat", &a[..1]).ok_or("rats needs a number")??;
            try_map(&r, &|x| Ok(text(&format!("{x}"))))?
        }
        _ => return Ok(None),
    }))
}

pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    inner(name, a).transpose()
}
