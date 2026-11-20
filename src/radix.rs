//! Numbers in a base: their digits, the block of digits that repeats, the text with a point.

use crate::canon::sub;
use crate::complex;
use crate::expr::Expr;
use crate::funcs::func;
use crate::num::{Int, Rat, parse_decimal};
use crate::numeric;
use crate::simplify::simplify;
use rug::Float;
use rug::ops::Pow;

/// The most digits that are made of one number.
pub const MOST: usize = 10_000_000;
/// The digits after the point that are written of a number without a count.
const SHOWN: usize = 1000;
/// The digits after the point that are listed of a number without a count.
const LISTED: usize = 100_000;

/// The digits of a whole number that is not negative, the highest place first.
pub fn whole_digits(n: &Int, base: u32) -> Vec<u32> {
    if n.is_zero() {
        return vec![0];
    }
    if base <= 36 {
        return n
            .to_string_radix(base as i32)
            .bytes()
            .map(|c| {
                u32::from(if c.is_ascii_digit() {
                    c - b'0'
                } else {
                    c - b'a' + 10
                })
            })
            .collect();
    }
    // powers[k] = base^(2^k); the number is split at the largest one below it
    let mut powers = vec![Int::from(base)];
    while powers[powers.len() - 1] <= *n {
        let last = &powers[powers.len() - 1];
        powers.push(Int::from(last * last));
    }
    let mut out = Vec::new();
    split(n, powers.len() - 1, &powers, base, false, &mut out);
    out
}

/// The digits of `n < base^(2^level)`: all 2^level of them when `padded`, else from the first
/// one that is not zero.
fn split(n: &Int, level: usize, powers: &[Int], base: u32, padded: bool, out: &mut Vec<u32>) {
    if let Some(mut small) = n.to_u64() {
        let from = out.len();
        while small > 0 {
            out.push((small % u64::from(base)) as u32);
            small /= u64::from(base);
        }
        if padded {
            out.resize(from + (1usize << level), 0);
        }
        out[from..].reverse();
        return;
    }
    let (high, low) = n.clone().div_rem(powers[level - 1].clone());
    if high.is_zero() && !padded {
        split(&low, level - 1, powers, base, false, out);
    } else {
        split(&high, level - 1, powers, base, padded, out);
        split(&low, level - 1, powers, base, true, out);
    }
}

/// The digits of a number that is no negative fraction.
#[derive(Debug, PartialEq)]
pub struct Expansion {
    pub whole: Vec<u32>,
    /// The digits after the point that do not repeat.
    pub fraction: Vec<u32>,
    /// The block that repeats without end; empty when the number ends.
    pub repeat: Vec<u32>,
    /// The digits go on: the limit came before the end of the block that repeats.
    pub cut: bool,
}

/// The digits of `x >= 0`, with no more than `limit` of them after the point.
pub fn expand(x: &Rat, base: u32, limit: usize) -> Expansion {
    let whole = Int::from(x.floor_ref());
    let q = x.denom().clone();
    let mut r = Int::from(x.numer() - Int::from(&whole * &q));
    // the digits before the block: as many as it takes to clear what the base shares with q
    let (mut rest, mut lead) = (q.clone(), 0usize);
    loop {
        let shared = rest.clone().gcd_u(base);
        if shared == 1 {
            break;
        }
        rest /= shared;
        lead += 1;
    }
    let mut out = Expansion {
        whole: whole_digits(&whole, base),
        fraction: Vec::new(),
        repeat: Vec::new(),
        cut: false,
    };
    let digit = |r: &mut Int| {
        *r *= base;
        let (d, left) = std::mem::take(r).div_rem(q.clone());
        *r = left;
        d.to_u32().unwrap_or(0)
    };
    for _ in 0..lead {
        if out.fraction.len() == limit {
            out.cut = true;
            return out;
        }
        let d = digit(&mut r);
        out.fraction.push(d);
    }
    if r.is_zero() {
        return out;
    }
    let start = r.clone();
    loop {
        if out.fraction.len() + out.repeat.len() == limit {
            out.fraction.append(&mut out.repeat);
            out.cut = true;
            return out;
        }
        let d = digit(&mut r);
        out.repeat.push(d);
        if r == start {
            return out;
        }
    }
}

/// The number with these digits: the block `repeat` goes on without end.
pub fn value(whole: &[u32], fraction: &[u32], repeat: &[u32], base: u32) -> Rat {
    let number = |digits: &[u32]| {
        let mut n = Int::new();
        for d in digits {
            n = n * base + *d;
        }
        n
    };
    let shift = Int::from(base).pow(fraction.len() as u32);
    let mut out = Rat::from(number(whole)) + Rat::from((number(fraction), shift.clone()));
    if !repeat.is_empty() {
        let block = Int::from(base).pow(repeat.len() as u32) - 1u32;
        out += Rat::from((number(repeat), block * shift));
    }
    out
}

/// The largest base whose digits are characters: 0-9, a-z, A-Z, @ and _ (as the shell writes
/// them). Above it every digit is a number after `#`.
pub const LETTERED: u32 = 64;

/// A digit of a base up to 64 as it is written.
pub fn digit_char(d: u32) -> char {
    match d {
        0..=35 => char::from_digit(d, 36).unwrap_or('?'),
        36..=61 => char::from(b'A' + (d - 36) as u8),
        62 => '@',
        _ => '_',
    }
}

/// The digit that a character is in a base up to 64: capitals are the small letters up to
/// base 36, and digits of their own above.
pub fn digit_of(c: char, base: u32) -> Option<u32> {
    let d = match c {
        '0'..='9' => c as u32 - '0' as u32,
        'a'..='z' => c as u32 - 'a' as u32 + 10,
        'A'..='Z' if base <= 36 => c as u32 - 'A' as u32 + 10,
        'A'..='Z' => c as u32 - 'A' as u32 + 36,
        '@' => 62,
        '_' => 63,
        _ => return None,
    };
    (d < base && base <= LETTERED).then_some(d)
}

/// Digits as they are written: characters up to base 64, then numbers of equal width after
/// `#`, as `#001#234` in base 1000.
pub fn digits_text(digits: &[u32], base: u32) -> String {
    if base <= LETTERED {
        return digits.iter().map(|d| digit_char(*d)).collect();
    }
    let width = (base - 1).to_string().len();
    digits.iter().map(|d| format!("#{d:0width$}")).collect()
}

/// The text of an expansion: `12.3(45)`, with `...` when it is cut.
pub fn text(e: &Expansion, base: u32) -> String {
    let mut out = digits_text(&e.whole, base);
    if !e.fraction.is_empty() || !e.repeat.is_empty() {
        out.push('.');
        out.push_str(&digits_text(&e.fraction, base));
    }
    if !e.repeat.is_empty() {
        out.push('(');
        out.push_str(&digits_text(&e.repeat, base));
        out.push(')');
    }
    if e.cut {
        out.push_str("...");
    }
    out
}

/// A number written in a base at `chars[at..]`: digits, then a point with digits, then a block
/// that repeats in parentheses. The number and where its text ends, or nothing when no digit
/// stands there. `led`: the `#` of the first numbered digit is the one after the base.
pub fn scan(
    chars: &[char],
    at: usize,
    base: u32,
    led: bool,
) -> Result<Option<(Rat, usize)>, String> {
    // the digits from a place on, and where they end
    let run = |from: usize, led: bool| -> Result<(Vec<u32>, usize), String> {
        let mut digits = Vec::new();
        let mut k = from;
        if base <= LETTERED {
            while let Some(d) = chars.get(k).and_then(|c| digit_of(*c, base)) {
                digits.push(d);
                k += 1;
            }
            return Ok((digits, k));
        }
        loop {
            let mut j = k;
            if chars.get(j) == Some(&'#') {
                j += 1;
            } else if !(led && digits.is_empty()) {
                break;
            }
            let first = j;
            let mut n = 0u64;
            while let Some(d) = chars.get(j).and_then(|c| c.to_digit(10)) {
                n = (n * 10 + u64::from(d)).min(u64::from(u32::MAX) + 1);
                j += 1;
            }
            if j == first {
                break;
            }
            if n >= u64::from(base) {
                let written: String = chars[first..j].iter().collect();
                return Err(format!("{written} is no digit of base {base}"));
            }
            digits.push(n as u32);
            k = j;
        }
        Ok((digits, k))
    };
    let (whole, mut end) = run(at, led)?;
    let mut fraction = Vec::new();
    let mut repeat = Vec::new();
    if chars.get(end) == Some(&'.') {
        let after;
        (fraction, after) = run(end + 1, false)?;
        if chars.get(after) == Some(&'(') {
            let (block, close) = run(after + 1, false)?;
            if !block.is_empty() && chars.get(close) == Some(&')') {
                repeat = block;
                end = close + 1;
            }
        }
        // (a point without digits after it is no part of the number)
        if repeat.is_empty() && !fraction.is_empty() {
            end = after;
        }
        if base > LETTERED
            && fraction.is_empty()
            && repeat.is_empty()
            && !whole.is_empty()
            && chars.get(end + 1).is_some_and(char::is_ascii_digit)
        {
            return Err(format!(
                "every digit of base {base} stands after a #, also after the point: .#5"
            ));
        }
    }
    if whole.is_empty() && fraction.is_empty() && repeat.is_empty() {
        return Ok(None);
    }
    Ok(Some((value(&whole, &fraction, &repeat, base), end)))
}

/// A number written in a base as a text: the whole text must be the number, or a fraction of
/// two such numbers. In base 256 a text without `#` is read character by character.
pub fn read(text: &str, base: u32, notes: &mut Vec<String>) -> Result<Rat, String> {
    let fail = || format!("\"{text}\" is no number written in base {base}");
    let trimmed = text.trim();
    let (negative, body) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let one = |part: &str| -> Result<Rat, String> {
        let chars: Vec<char> = part.trim().chars().collect();
        match scan(&chars, 0, base, false) {
            Ok(Some((v, end))) if end == chars.len() => Ok(v),
            Err(why) if base > LETTERED => Err(why),
            _ => Err(fail()),
        }
    };
    let number = match body.split_once('/') {
        Some((p, q)) => one(p).and_then(|p| {
            let q = one(q)?;
            if q.cmp0() == std::cmp::Ordering::Equal {
                return Err(format!("\"{text}\" divides by zero"));
            }
            Ok(p / q)
        }),
        None => one(body),
    };
    match number {
        Ok(v) => Ok(if negative { -v } else { v }),
        Err(_) if base == 256 && !text.is_empty() && !text.contains('#') => {
            let digits: Vec<u32> = text.chars().map(u32::from).collect();
            if let Some(wide) = text.chars().find(|c| u32::from(*c) > 255) {
                return Err(format!(
                    "the character {wide} of \"{text}\" is no digit of base 256: its number is above 255"
                ));
            }
            notes.push(format!(
                "\"{text}\" is read character by character: {}",
                digits_text(&digits, base)
            ));
            Ok(value(&digits, &[], &[], base))
        }
        Err(why) => Err(why),
    }
}

/// A real number: a fraction, or an expression with a real value that is none.
enum Real {
    Exact(Rat),
    Other(Expr),
}

/// The real and the imaginary part of a constant, with what is to tell about them.
fn parts(x: &Expr, notes: &mut Vec<String>) -> Result<[Real; 2], String> {
    if let Some(n) = x.as_num() {
        return Ok([Real::Exact(n.re.clone()), Real::Exact(n.im.clone())]);
    }
    if !x.is_const()
        || x.is_infinite()
        || matches!(x, Expr::List(_) | Expr::Rel(..) | Expr::Bool(_))
    {
        return Err(format!("{x} is no number"));
    }
    if x.has_fn("quat") {
        return Err(format!(
            "{x} is a quaternion: the digits of real and of complex numbers are written"
        ));
    }
    if x.has_float() {
        // a number with a point that came out of a calculation: the digits that it has
        let digits = numeric::digits();
        let v = numeric::evalf_digits(x, digits).map_err(|_| format!("{x} has no value"))?;
        let part = |f: &Float| {
            parse_decimal(&numeric::fmt_real(f, digits)).ok_or_else(|| format!("{x} has no value"))
        };
        notes.push(format!(
            "{x} is known with {digits} digits: its digits end there"
        ));
        return Ok([Real::Exact(part(v.real())?), Real::Exact(part(v.imag())?)]);
    }
    let (re, im) = complex::rect(x)
        .unwrap_or_else(|| (func("re", vec![x.clone()]), func("im", vec![x.clone()])));
    let real = |e: Expr| -> Result<Real, String> {
        let e = simplify(&e);
        if let Some(r) = e.as_rat() {
            return Ok(Real::Exact(r.clone()));
        }
        let rough = numeric::evalf(&e, 64).map_err(|_| format!("{x} has no value"))?;
        if !rough.real().is_finite() || !rough.imag().is_zero() {
            return Err(format!("{x} has no value"));
        }
        if rough.real().is_zero() && numeric::is_zero_const(&e) == Some(true) {
            return Ok(Real::Exact(Rat::new()));
        }
        Ok(Real::Other(e))
    };
    Ok([real(re)?, real(im)?])
}

/// `floor(|e| * base^places)` for a real constant that is no fraction: the same whole number
/// from two precisions, and far enough from the next one for both.
fn scaled(e: &Expr, base: u32, places: usize) -> Result<Int, String> {
    let no_value = || format!("{e} has no value");
    let rough = numeric::evalf(e, 64).map_err(|_| no_value())?;
    let before = u64::from(rough.real().get_exp().unwrap_or(0).max(0) as u32);
    let bits_a_digit = u64::from(32 - (base - 1).leading_zeros());
    let need = before + bits_a_digit * places as u64;
    if need > 1 << 30 {
        return Err("too many digits".to_string());
    }
    let power = Int::from(base).pow(places as u32);
    let at = |prec: u32| -> Result<(Int, Float), String> {
        let v = numeric::evalf(e, prec).map_err(|_| no_value())?;
        let y = Float::with_val(prec, v.real() * &power).abs();
        let floor = y.clone().floor();
        let whole = floor.to_integer().ok_or_else(no_value)?;
        Ok((whole, Float::with_val(64, &y - &floor)))
    };
    let mut guard = 64u32;
    let mut last = None;
    for _ in 0..4 {
        let prec = need as u32 + guard;
        let (low, _) = at(prec)?;
        let (high, gap) = at(prec + 64)?;
        let edge = Float::with_val(64, 1) >> (guard - 16);
        let clear = gap > edge && Float::with_val(64, 1 - &gap) > edge;
        if low == high && clear {
            return Ok(high);
        }
        // (next to a whole number: the nearer one of the two)
        last = Some(if gap < 0.5 { high } else { high + 1u32 });
        guard += 128;
    }
    // the number may be that fraction itself
    let near = Rat::from((last.expect("a try"), power));
    let size = func("abs", vec![e.clone()]);
    if numeric::is_zero_const(&sub(size, Expr::rational(near.clone()))) == Some(true) {
        return Ok(Int::from(near.floor_ref()) * Int::from(base).pow(places as u32) / near.denom());
    }
    Err(format!(
        "the digits of {e} cannot be told at the place {places}"
    ))
}

/// The first digits of a real number.
#[derive(Debug, PartialEq)]
pub struct Leading {
    /// The digits, the highest place first.
    pub digits: Vec<u32>,
    /// How many places there are before the point.
    pub whole: usize,
    /// The number has no digit after these.
    pub ended: bool,
}

impl Real {
    fn size(&self) -> Real {
        match self {
            Real::Exact(r) => Real::Exact(r.clone().abs()),
            Real::Other(e) => Real::Other(e.clone()),
        }
    }

    fn is_negative(&self) -> bool {
        match self {
            Real::Exact(r) => r.cmp0() == std::cmp::Ordering::Less,
            Real::Other(e) => numeric::evalf(e, 64).is_ok_and(|v| v.real().is_sign_negative()),
        }
    }

    /// The digits of the whole part of the size.
    fn whole(&self, base: u32) -> Result<Vec<u32>, String> {
        Ok(whole_digits(
            &match self {
                Real::Exact(r) => Int::from(r.clone().abs().floor_ref()),
                Real::Other(e) => scaled(e, base, 0)?,
            },
            base,
        ))
    }

    /// The first `count` digits of the size, with `places` places before the point (zeros in
    /// front when the number has fewer).
    fn leading(&self, base: u32, places: usize, count: usize) -> Result<Leading, String> {
        let own = self.whole(base)?;
        let places = places.max(own.len());
        let mut digits = vec![0; places - own.len()];
        digits.extend(&own);
        if count <= places {
            let ended = match self {
                Real::Exact(r) => count == places && r.is_integer(),
                Real::Other(_) => false,
            };
            digits.truncate(count);
            return Ok(Leading {
                digits,
                whole: places,
                ended,
            });
        }
        let after = count - places;
        let ended = match self {
            Real::Exact(r) => {
                let r = r.clone().abs();
                let q = r.denom().clone();
                let mut left = Int::from(r.numer() - Int::from(r.floor_ref()) * &q);
                for _ in 0..after {
                    if left.is_zero() {
                        break;
                    }
                    left *= base;
                    let (d, rest) = left.div_rem(q.clone());
                    left = rest;
                    digits.push(d.to_u32().unwrap_or(0));
                }
                left.is_zero()
            }
            Real::Other(e) => {
                let all = whole_digits(&scaled(e, base, after)?, base);
                let have = own.len() + after;
                // (a number below one has zeros after the point that the whole number lacks)
                let fraction = &all[all.len().saturating_sub(after)..];
                digits.resize(digits.len() + after - fraction.len().min(after), 0);
                digits.extend(fraction);
                debug_assert!(all.len() <= have);
                false
            }
        };
        Ok(Leading {
            digits,
            whole: places,
            ended,
        })
    }
}

/// The first `count` digits of the size of a real constant in a base, the whole part first.
pub fn leading(
    x: &Expr,
    base: u32,
    count: usize,
    notes: &mut Vec<String>,
) -> Result<Leading, String> {
    let [re, im] = parts(x, notes)?;
    if !matches!(&im, Real::Exact(r) if r.cmp0() == std::cmp::Ordering::Equal) {
        return Err(format!("{x} is no real number"));
    }
    re.size().leading(base, 0, count)
}

/// The first `count` digits of the sizes of the real and of the imaginary part of a constant,
/// place by place: both start at the highest place that one of them has.
pub fn leading_pair(
    x: &Expr,
    base: u32,
    count: usize,
    notes: &mut Vec<String>,
) -> Result<[Leading; 2], String> {
    let [re, im] = parts(x, notes)?;
    let places = re.whole(base)?.len().max(im.whole(base)?.len());
    Ok([
        re.size().leading(base, places, count)?,
        im.size().leading(base, places, count)?,
    ])
}

/// A base: a whole number from 2 on (10 when none is named).
pub fn base_of(e: Option<&Expr>) -> Result<u32, String> {
    match e {
        None => Ok(10),
        Some(e) => plain(e.clone())
            .as_i64()
            .and_then(|b| u32::try_from(b).ok())
            .filter(|b| *b >= 2)
            .ok_or_else(|| format!("the base is a whole number from 2 to {}, not {e}", u32::MAX)),
    }
}

fn count_of(e: &Expr) -> Result<usize, String> {
    e.as_i64()
        .and_then(|n| usize::try_from(n).ok())
        .filter(|n| (1..=MOST).contains(n))
        .ok_or_else(|| format!("the number of digits is from 1 to {MOST}, not {e}"))
}

fn list(digits: &[u32]) -> Expr {
    Expr::list(digits.iter().map(|d| Expr::int(i64::from(*d))).collect())
}

/// The digits that a session shows of a number without an end: as many as its decimal digits
/// are worth in the base.
fn shown(base: u32) -> usize {
    let ten = Int::from(10).pow(numeric::digits());
    whole_digits(&ten, base).len()
}

/// The first `count` digits of a size, and all of its whole part when that has more.
fn first(v: &Real, base: u32, count: usize) -> Result<Leading, String> {
    let lead = v.leading(base, 0, count)?;
    if lead.whole > count {
        return v.leading(base, 0, lead.whole);
    }
    Ok(lead)
}

/// A real number written in a base, `count` digits of it or all of them.
fn written(
    v: &Real,
    base: u32,
    count: Option<usize>,
    notes: &mut Vec<String>,
) -> Result<String, String> {
    let sign = if v.is_negative() { "-" } else { "" };
    let v = v.size();
    let expansion = match (&v, count) {
        (Real::Exact(r), None) => {
            let e = expand(r, base, SHOWN);
            if e.cut {
                notes.push(format!(
                    "the digits repeat after more than {SHOWN} of them: period(x, b) tells after how many, tobase(x, b, n) writes the first n"
                ));
            }
            e
        }
        _ => {
            let lead = first(&v, base, count.unwrap_or_else(|| shown(base)))?;
            Expansion {
                whole: lead.digits[..lead.whole].to_vec(),
                fraction: lead.digits[lead.whole..].to_vec(),
                repeat: Vec::new(),
                cut: !lead.ended,
            }
        }
    };
    Ok(format!("{sign}{}", text(&expansion, base)))
}

/// The text of a complex number from the texts of its parts.
fn joined(
    parts: &[Real; 2],
    mut one: impl FnMut(&Real) -> Result<String, String>,
) -> Result<String, String> {
    let zero = |v: &Real| matches!(v, Real::Exact(r) if r.cmp0() == std::cmp::Ordering::Equal);
    let re = one(&parts[0])?;
    if zero(&parts[1]) {
        return Ok(re);
    }
    let im = one(&parts[1])?;
    if zero(&parts[0]) {
        return Ok(format!("{im}*i"));
    }
    Ok(match im.strip_prefix('-') {
        Some(size) => format!("{re} - {size}*i"),
        None => format!("{re} + {im}*i"),
    })
}

/// The commands of numbers in a base: the value and what is to tell about it.
pub fn call(name: &str, a: &[Expr]) -> Option<Result<(Expr, Vec<String>), String>> {
    let known = matches!(
        (name, a.len()),
        ("tobase", 1..=3)
            | ("base_conv", 2)
            | ("tofraction", 1 | 2)
            | ("digits", 1..=3)
            | ("period", 1 | 2)
            | ("digitcounts", 1..=3)
    ) || (name == "frombase"
        && a.len() == 2
        && (text_of(&a[0]).is_some()
            || a[0].as_list().is_some_and(|l| {
                l.iter()
                    .all(|d| d.as_int().is_some() || d.as_list().is_some())
            })));
    if !known {
        return None;
    }
    let mut notes = Vec::new();
    Some(command(name, a, &mut notes).map(|value| (value, notes)))
}

/// The text of a string.
pub fn text_of(e: &Expr) -> Option<&str> {
    match e.fn_args("string") {
        Some([inner]) => inner.sym_name(),
        _ => None,
    }
}

fn kept(value: &Expr, text: String) -> Expr {
    Expr::call("based", vec![value.clone(), Expr::sym(&text)])
}

/// The value of a number that is kept as it was written in a base.
pub fn plain(e: Expr) -> Expr {
    match e.fn_args("based") {
        Some([value, _]) => value.clone(),
        _ => e,
    }
}

/// A command for each number of a list, or for the one number.
fn each(x: &Expr, one: &mut dyn FnMut(&Expr) -> Result<Expr, String>) -> Result<Expr, String> {
    match x {
        Expr::List(items) => Ok(Expr::list(
            items
                .iter()
                .map(|item| each(item, one))
                .collect::<Result<_, _>>()?,
        )),
        _ => one(&plain(x.clone())),
    }
}

/// The text that the digits of a whole number are in base 256, when all of them print.
fn characters(v: &[Real; 2]) -> Option<String> {
    let [Real::Exact(re), Real::Exact(im)] = v else {
        return None;
    };
    if !re.is_integer() || re.cmp0() != std::cmp::Ordering::Greater || !im.is_zero() {
        return None;
    }
    whole_digits(re.numer(), 256)
        .into_iter()
        .map(|d| char::from_u32(d).filter(|c| (' '..='~').contains(c) && *c != '"'))
        .collect()
}

fn command(name: &str, a: &[Expr], notes: &mut Vec<String>) -> Result<Expr, String> {
    match name {
        "tobase" | "base_conv" => {
            let base = base_of(a.get(1))?;
            let count = a.get(2).map(count_of).transpose()?;
            each(&a[0], &mut |x| {
                let both = parts(x, notes).map_err(|why| match (name, x.sym_name()) {
                    // (a word may be meant as the digits of a base)
                    ("base_conv", Some(word)) => format!(
                        "{why}: base_conv(x, to) writes a value in the base to; base_conv({word}, from, to) reads {word} as it is written in the base from"
                    ),
                    _ => why,
                })?;
                let text = joined(&both, |v| written(v, base, count, notes))?;
                if base == 256
                    && let Some(shown) = characters(&both)
                {
                    notes.push(format!(
                        "as a text, one character for each digit: \"{shown}\""
                    ));
                }
                Ok(kept(x, text))
            })
        }
        "tofraction" => {
            let base = base_of(a.get(1))?;
            each(&a[0], &mut |x| {
                let both = parts(x, notes)?;
                let text = joined(&both, |v| match v {
                    Real::Exact(r) => {
                        let sign = if v.is_negative() { "-" } else { "" };
                        let part =
                            |n: &Int| digits_text(&whole_digits(&n.clone().abs(), base), base);
                        Ok(if r.is_integer() {
                            format!("{sign}{}", part(r.numer()))
                        } else {
                            format!("{sign}{}/{}", part(r.numer()), part(r.denom()))
                        })
                    }
                    Real::Other(e) => Err(format!("{e} is no fraction of whole numbers")),
                })?;
                Ok(kept(x, text))
            })
        }
        "digits" => {
            let base = base_of(a.get(1))?;
            let [re, im] = parts(&a[0], notes)?;
            if !matches!(&im, Real::Exact(r) if r.cmp0() == std::cmp::Ordering::Equal) {
                return Err(format!(
                    "{} is no real number: digits takes re and im one by one",
                    a[0]
                ));
            }
            match (a.get(2), re.size()) {
                (None, Real::Exact(r)) => {
                    let e = expand(&r, base, LISTED);
                    if e.cut {
                        return Err(format!(
                            "the digits repeat after more than {LISTED} of them: digits(x, b, n) lists the first n"
                        ));
                    }
                    Ok(Expr::list(vec![
                        list(&e.whole),
                        list(&e.fraction),
                        list(&e.repeat),
                    ]))
                }
                (None, Real::Other(e)) => Err(format!(
                    "the digits of {e} do not end: digits(x, b, n) lists the first n"
                )),
                (Some(n), v) => {
                    let lead = first(&v, base, count_of(n)?)?;
                    let (whole, fraction) = lead.digits.split_at(lead.whole);
                    Ok(Expr::list(vec![list(whole), list(fraction)]))
                }
            }
        }
        "frombase" => {
            let base = base_of(a.get(1))?;
            if let Some(text) = text_of(&a[0]) {
                return Ok(Expr::rational(read(text, base, notes)?));
            }
            let usage = "frombase([digits], b) or frombase([[whole digits], [digits after the point], [digits that repeat]], b)";
            let all = a[0].as_list().ok_or(usage)?;
            let groups: Vec<&[Expr]> = if all.iter().any(|g| g.as_list().is_some()) {
                all.iter()
                    .map(|g| g.as_list().ok_or(usage))
                    .collect::<Result<_, _>>()?
            } else {
                vec![all]
            };
            let mut read = Vec::new();
            for group in groups {
                let mut digits = Vec::new();
                for d in group {
                    let d = d
                        .as_i64()
                        .and_then(|d| u32::try_from(d).ok())
                        .filter(|d| *d < base)
                        .ok_or_else(|| format!("{d} is no digit of base {base}"))?;
                    digits.push(d);
                }
                read.push(digits);
            }
            let empty = Vec::new();
            match read.as_slice() {
                [whole] => Ok(Expr::rational(value(whole, &empty, &empty, base))),
                [whole, fraction] => Ok(Expr::rational(value(whole, fraction, &empty, base))),
                [whole, fraction, repeat] => {
                    Ok(Expr::rational(value(whole, fraction, repeat, base)))
                }
                _ => Err(usage.to_string()),
            }
        }
        "period" => {
            let base = base_of(a.get(1))?;
            let [re, im] = parts(&a[0], notes)?;
            let (Real::Exact(r), Real::Exact(zero)) = (&re, &im) else {
                return Err(format!(
                    "{} is no fraction of whole numbers: its digits do not repeat",
                    a[0]
                ));
            };
            if zero.cmp0() != std::cmp::Ordering::Equal {
                return Err(format!("{} is no real number", a[0]));
            }
            let mut rest = r.denom().clone();
            loop {
                let shared = rest.clone().gcd_u(base);
                if shared == 1 {
                    break;
                }
                rest /= shared;
            }
            if rest == 1 {
                return Ok(Expr::zero());
            }
            crate::ntheory::multiplicative_order(&Int::from(base), &rest)
                .map(Expr::integer)
                .ok_or_else(|| {
                    format!("the period needs the factors of {rest}, which were not found")
                })
        }
        _ => {
            let base = base_of(a.get(1))?;
            if base > 1_000_000 {
                return Err(
                    "digitcounts lists a count for each digit: its base is at most 1000000"
                        .to_string(),
                );
            }
            let [re, im] = parts(&a[0], notes)?;
            if !matches!(&im, Real::Exact(r) if r.cmp0() == std::cmp::Ordering::Equal) {
                return Err(format!("{} is no real number", a[0]));
            }
            let digits = match (a.get(2), re.size()) {
                (None, Real::Exact(r)) => {
                    let e = expand(&r, base, LISTED);
                    if e.cut || !e.repeat.is_empty() {
                        return Err(
                            "the digits do not end: digitcounts(x, b, n) counts the first n".into(),
                        );
                    }
                    [e.whole, e.fraction].concat()
                }
                (None, Real::Other(e)) => {
                    return Err(format!(
                        "the digits of {e} do not end: digitcounts(x, b, n) counts the first n"
                    ));
                }
                (Some(n), v) => v.leading(base, 0, count_of(n)?)?.digits,
            };
            let mut counts = vec![0i64; base as usize];
            for d in digits {
                counts[d as usize] += 1;
            }
            Ok(Expr::list(counts.into_iter().map(Expr::int).collect()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::num::rat;

    /// The digits by plain long division, as a check of the quick ways.
    fn plain(n: &Int, base: u32) -> Vec<u32> {
        let mut n = n.clone();
        let mut out = Vec::new();
        while !n.is_zero() {
            out.push(n.mod_u(base));
            n /= base;
        }
        if out.is_empty() {
            out.push(0);
        }
        out.reverse();
        out
    }

    #[test]
    fn digits_of_whole_numbers() {
        let mut n = Int::from(1);
        for k in 0..400u32 {
            for base in [
                2,
                3,
                7,
                10,
                16,
                36,
                37,
                60,
                255,
                256,
                1000,
                65537,
                4_000_000_007,
            ] {
                assert_eq!(
                    whole_digits(&n, base),
                    plain(&n, base),
                    "{n} in base {base}"
                );
            }
            n = n * 7919u32 + k * k + 1;
        }
        assert_eq!(whole_digits(&Int::new(), 60), [0]);
        assert_eq!(whole_digits(&Int::from(59), 60), [59]);
        assert_eq!(whole_digits(&Int::from(3600), 60), [1, 0, 0]);
        // powers of the base, where the zeros of the halves count
        for base in [37u32, 100, 1 << 20] {
            for k in [1u32, 2, 7, 8, 9, 31, 32, 33, 64, 100] {
                let n = Int::from(base).pow(k);
                let mut want = vec![0; k as usize + 1];
                want[0] = 1;
                assert_eq!(whole_digits(&n, base), want);
                let below = Int::from(&n - 1u32);
                assert_eq!(whole_digits(&below, base), vec![base - 1; k as usize]);
            }
        }
    }

    #[test]
    fn fractions_and_their_blocks() {
        let e = |p, q, base| expand(&rat(p, q), base, 10_000);
        let digits = |e: &Expansion| (e.whole.clone(), e.fraction.clone(), e.repeat.clone());
        assert_eq!(digits(&e(1, 3, 10)), (vec![0], vec![], vec![3]));
        assert_eq!(
            digits(&e(1, 7, 10)),
            (vec![0], vec![], vec![1, 4, 2, 8, 5, 7])
        );
        assert_eq!(digits(&e(1, 6, 10)), (vec![0], vec![1], vec![6]));
        assert_eq!(digits(&e(1, 4, 10)), (vec![0], vec![2, 5], vec![]));
        assert_eq!(digits(&e(679, 55, 10)), (vec![1, 2], vec![3], vec![4, 5]));
        assert_eq!(digits(&e(1, 3, 2)), (vec![0], vec![], vec![0, 1]));
        assert_eq!(digits(&e(1, 10, 2)), (vec![0], vec![0], vec![0, 0, 1, 1]));
        assert_eq!(digits(&e(1, 3, 3)), (vec![0], vec![1], vec![]));
        assert_eq!(digits(&e(255, 1, 16)), (vec![15, 15], vec![], vec![]));
        assert_eq!(digits(&e(1, 2, 60)), (vec![0], vec![30], vec![]));
        assert_eq!(digits(&e(0, 1, 5)), (vec![0], vec![], vec![]));
        assert_eq!(digits(&e(1, 12, 6)), (vec![0], vec![0, 3], vec![]));
        assert_eq!(digits(&e(1, 8, 4)), (vec![0], vec![0, 2], vec![]));
        // every fraction comes back from its digits, in every base
        for base in [2u32, 3, 6, 10, 12, 16, 36, 60, 1000] {
            for q in 1..=60i64 {
                for p in [0, 1, 2, q - 1, q + 1, 7 * q + 3, 1000 * q + 1] {
                    let x = rat(p, q);
                    let e = expand(&x, base, 10_000);
                    assert!(!e.cut);
                    assert_eq!(
                        value(&e.whole, &e.fraction, &e.repeat, base),
                        x,
                        "{x} in {base}"
                    );
                    // the block is as short as it can be and starts as early as it can
                    if let Some(last) = e.repeat.last() {
                        assert!(e.fraction.last() != Some(last), "{x} in base {base}: {e:?}");
                        let n = e.repeat.len();
                        for part in (1..n).filter(|d| n.is_multiple_of(*d)) {
                            let same = e.repeat.chunks(part).all(|c| c == &e.repeat[..part]);
                            assert!(!same, "{x} in base {base}: {e:?}");
                        }
                    } else {
                        assert!(e.fraction.last() != Some(&0), "{x} in base {base}: {e:?}");
                    }
                }
            }
        }
        // a limit cuts the digits
        let cut = expand(&rat(1, 7), 10, 4);
        assert_eq!(digits(&cut), (vec![0], vec![1, 4, 2, 8], vec![]));
        assert!(cut.cut);
        let cut = expand(&rat(1, 6), 10, 0);
        assert!(cut.cut && cut.fraction.is_empty() && cut.repeat.is_empty());
        assert!(!expand(&rat(1, 7), 10, 6).cut);
        assert!(!expand(&rat(1, 4), 10, 2).cut);
        assert!(expand(&rat(1, 4), 10, 1).cut);
    }

    #[test]
    fn numbers_as_text() {
        let show = |p, q, base| text(&expand(&rat(p, q), base, 100), base);
        assert_eq!(show(1, 3, 10), "0.(3)");
        assert_eq!(show(1, 7, 10), "0.(142857)");
        assert_eq!(show(679, 55, 10), "12.3(45)");
        assert_eq!(show(1, 3, 2), "0.(01)");
        assert_eq!(show(255, 1, 16), "ff");
        assert_eq!(show(511, 2, 16), "ff.8");
        assert_eq!(show(35, 1, 36), "z");
        assert_eq!(show(1, 97, 10).len(), 2 + 96 + 2);
        assert_eq!(text(&expand(&rat(1, 7), 10, 3), 10), "0.142...");
        // the digits above z, and digits that are numbers
        assert_eq!(show(36 * 64 + 61, 1, 64), "AZ");
        assert_eq!(show(62 * 64 + 63, 1, 64), "@_");
        assert_eq!(show(10 * 37 + 36, 1, 37), "aA");
        assert_eq!(show(1000, 1, 1000), "#001#000");
        assert_eq!(show(999, 1, 1000), "#999");
        assert_eq!(show(1234, 1, 1000), "#001#234");
        assert_eq!(show(2469, 2, 1000), "#001#234.#500");
        assert_eq!(show(1, 3, 1000), "#000.(#333)");
        assert_eq!(show(255, 1, 65), "#03#60");
        assert_eq!(show(1, 2, 256), "#000.#128");
        assert_eq!(show(7, 1, 4_294_967_295), "#0000000007");
        assert_eq!(text(&expand(&rat(1, 7), 1000, 1), 1000), "#000.#142...");
        assert_eq!(text(&expand(&rat(1, 7), 1000, 2), 1000), "#000.(#142#857)");
        let mut notes = Vec::new();
        for (written, base, p, q) in [
            ("AZ", 64, 36 * 64 + 61, 1),
            ("@_", 64, 62 * 64 + 63, 1),
            ("az", 64, 10 * 64 + 35, 1),
            ("Az", 36, 10 * 36 + 35, 1),
            ("#001#234", 1000, 1234, 1),
            ("#1#234", 1000, 1234, 1),
            ("#001#234.#500", 1000, 2469, 2),
            ("#000.(#333)", 1000, 1, 3),
            ("-#999", 1000, -999, 1),
            ("#255#255", 256, 65535, 1),
            ("1/11", 2, 1, 3),
            ("-ff/a", 16, -51, 2),
            ("#001#000 / #003", 1000, 1000, 3),
            ("0.(3)/0.5", 10, 2, 3),
        ] {
            assert_eq!(
                read(written, base, &mut notes),
                Ok(rat(p, q)),
                "{written} in base {base}"
            );
        }
        assert!(notes.is_empty());
        // a text in base 256 is its characters
        assert_eq!(
            read("Hello", 256, &mut notes),
            Ok(Rat::from(Int::from(310_939_249_775u64)))
        );
        assert_eq!(
            notes,
            ["\"Hello\" is read character by character: #072#101#108#108#111"]
        );
        assert_eq!(read("-1", 256, &mut notes), Ok(rat(45 * 256 + 49, 1)));
        assert_eq!(read("#045#049", 256, &mut notes), Ok(rat(45 * 256 + 49, 1)));
        assert!(read("ż", 256, &mut notes).is_err());
        for (written, base, why) in [
            ("#1000", 1000, "1000 is no digit of base 1000"),
            (
                "#001.5",
                1000,
                "every digit of base 1000 stands after a #, also after the point: .#5",
            ),
            ("12", 1000, "\"12\" is no number written in base 1000"),
            ("1/0", 10, "\"1/0\" divides by zero"),
            ("1/2/3", 10, "\"1/2/3\" is no number written in base 10"),
            ("A", 10, "\"A\" is no number written in base 10"),
            ("_", 63, "\"_\" is no number written in base 63"),
        ] {
            assert_eq!(
                read(written, base, &mut notes),
                Err(why.to_string()),
                "{written}"
            );
        }
        for (written, base, p, q) in [
            ("0.(3)", 10, 1, 3),
            ("12.3(45)", 10, 679, 55),
            ("0.(01)", 2, 1, 3),
            ("ff.8", 16, 511, 2),
            ("FF.8", 16, 511, 2),
            ("-0.1", 2, -1, 2),
            ("+z", 36, 35, 1),
            (".5", 10, 1, 2),
            (" 101 ", 2, 5, 1),
            ("0.0(0011)", 2, 1, 10),
        ] {
            assert_eq!(
                read(written, base, &mut notes),
                Ok(rat(p, q)),
                "{written} in base {base}"
            );
        }
        for (written, base) in [
            ("12", 2),
            ("0.(3", 10),
            ("", 10),
            ("1.2.3", 10),
            ("g", 16),
            ("-", 10),
            ("0.()", 10),
            ("1.", 10),
        ] {
            assert!(
                read(written, base, &mut notes).is_err(),
                "{written} in base {base}"
            );
        }
        // where the text of a number ends
        let chars: Vec<char> = "101.1(01)x".chars().collect();
        assert_eq!(scan(&chars, 0, 2, false), Ok(Some((rat(17, 3), 9))));
        let chars: Vec<char> = "ffg".chars().collect();
        assert_eq!(scan(&chars, 0, 16, false), Ok(Some((rat(255, 1), 2))));
        let chars: Vec<char> = "12.x".chars().collect();
        assert_eq!(scan(&chars, 0, 10, false), Ok(Some((rat(12, 1), 2))));
        let chars: Vec<char> = "1.5(2".chars().collect();
        assert_eq!(scan(&chars, 0, 10, false), Ok(Some((rat(3, 2), 3))));
        assert_eq!(scan(&chars, 5, 10, false), Ok(None));
        // the first digit of a number after its base has the # of the base
        let chars: Vec<char> = "001#234.#5(#6)+".chars().collect();
        assert_eq!(scan(&chars, 0, 1000, false), Ok(None));
        let v = rat(1234, 1) + rat(5, 1000) + rat(6, 999_000);
        assert_eq!(scan(&chars, 0, 1000, true), Ok(Some((v, 14))));
    }

    fn number(text: &str) -> Expr {
        let parsed = crate::parser::parse(text).expect("an expression");
        crate::Session::new()
            .eval(&parsed.stmts[0])
            .expect("a value")
    }

    fn first(x: &str, base: u32, count: usize) -> Leading {
        leading(&number(x), base, count, &mut Vec::new()).expect("digits")
    }

    #[test]
    fn digits_of_numbers_without_end() {
        let pi = first("pi", 10, 40);
        assert_eq!(
            pi.digits.iter().map(|d| digit_char(*d)).collect::<String>(),
            "3141592653589793238462643383279502884197"
        );
        assert_eq!((pi.whole, pi.ended), (1, false));
        let hex = first("pi", 16, 20);
        assert_eq!(
            hex.digits
                .iter()
                .map(|d| digit_char(*d))
                .collect::<String>(),
            "3243f6a8885a308d3131"
        );
        // the digits of a root of a whole number against the whole root of a power
        for (n, base, count) in [
            (2u32, 10u32, 300usize),
            (3, 7, 200),
            (5, 60, 150),
            (7, 1000, 60),
        ] {
            let lead = first(&format!("sqrt({n})"), base, count);
            let power = Int::from(base).pow(2 * (count as u32 - 1));
            let root = (Int::from(n) * power).sqrt();
            assert_eq!(
                lead.digits,
                whole_digits(&root, base),
                "sqrt({n}) in base {base}"
            );
            assert_eq!(lead.whole, 1);
        }
        // numbers below one and large ones
        assert_eq!(first("pi/1000", 10, 8).digits, [0, 0, 0, 3, 1, 4, 1, 5]);
        assert_eq!(first("pi/1000", 10, 8).whole, 1);
        assert_eq!(first("1000pi", 10, 8).digits, [3, 1, 4, 1, 5, 9, 2, 6]);
        assert_eq!(first("1000pi", 10, 8).whole, 4);
        assert_eq!(first("1000pi", 10, 2).digits, [3, 1]);
        assert_eq!(first("-pi", 10, 3).digits, [3, 1, 4]);
        assert_eq!(
            first("e^(pi*sqrt(163))", 10, 30).digits[15..],
            [7, 4, 3, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9]
        );
        assert_eq!(first("e^(pi*sqrt(163))", 10, 32).digits[30..], [2, 5]);
        // fractions end or go on
        let quarter = first("1/4", 10, 100);
        assert_eq!(
            (quarter.digits.as_slice(), quarter.ended),
            ([0, 2, 5].as_slice(), true)
        );
        let third = first("1/3", 10, 5);
        assert_eq!(
            (third.digits.as_slice(), third.ended),
            ([0, 3, 3, 3, 3].as_slice(), false)
        );
        let whole = first("1200", 10, 100);
        assert_eq!(
            (whole.digits.as_slice(), whole.whole, whole.ended),
            ([1, 2, 0, 0].as_slice(), 4, true)
        );
        assert!(!first("1200", 10, 3).ended);
        assert!(first("1/4", 10, 3).ended);
        assert!(!first("1/4", 10, 2).ended);
        assert_eq!(first("1/4", 10, 1).digits, [0]);
        // a value that is a fraction in disguise
        assert_eq!(first("sin(pi/6)", 10, 9).digits, [0, 5]);
    }

    #[test]
    fn two_parts_place_by_place() {
        let e = number("1000e + i*pi");
        let [re, im] = leading_pair(&e, 10, 8, &mut Vec::new()).expect("digits");
        assert_eq!(re.digits, [2, 7, 1, 8, 2, 8, 1, 8]);
        assert_eq!(im.digits, [0, 0, 0, 3, 1, 4, 1, 5]);
        assert_eq!((re.whole, im.whole), (4, 4));
        let e = number("1/4 - 100i/3");
        let [re, im] = leading_pair(&e, 10, 6, &mut Vec::new()).expect("digits");
        assert_eq!(
            (re.digits.as_slice(), re.ended),
            ([0, 0, 2, 5].as_slice(), true)
        );
        assert_eq!(
            (im.digits.as_slice(), im.ended),
            ([3, 3, 3, 3, 3, 3].as_slice(), false)
        );
        assert_eq!((re.whole, im.whole), (2, 2));
        let e = number("x + 1");
        assert!(leading_pair(&e, 10, 6, &mut Vec::new()).is_err());
        let e = number("pi + i");
        assert!(
            leading(&e, 10, 6, &mut Vec::new())
                .unwrap_err()
                .contains("no real number")
        );
    }
}
