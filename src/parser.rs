//! Text to expression tree. The tree is raw: nothing is simplified here.

use crate::expr::{Expr, Rel};
use crate::names;
use crate::num::{Int, Num, Rat, literal_digits, parse_decimal};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(String),
    Exact(Rat),
    Text(String),
    Ident(String),
    Op(&'static str),
    Sup(String),
    Root(u32),
    End,
}

pub struct Parsed {
    pub stmts: Vec<Expr>,
    /// Significant digits of the longest numeric literal.
    pub max_digits: u32,
    /// A literal was written with a decimal point or an exponent.
    pub decimal: bool,
}

const OPS2: &[(&str, &str)] = &[
    ("^^", "^^"),
    ("**", "^"),
    ("!=", "!="),
    ("<>", "!="),
    ("<=", "<="),
    (">=", ">="),
    ("==", "="),
    ("->", "->"),
    (":=", ":="),
    ("!!", "!!"),
    ("↑↑", "^^"),
    ("&&", "&&"),
    ("~=", "!="),
    (".*", ".*"),
    ("./", "./"),
    (".^", ".^"),
    (".\\", ".\\"),
];

fn op1(ch: char) -> Option<&'static str> {
    Some(match ch {
        '+' => "+",
        '-' | '−' | '–' | '—' | '‒' => "-",
        '*' | '×' | '·' | '⋅' | '∙' | '∗' | '✕' | '✖' => "*",
        '/' | '÷' | '∕' | '⁄' => "/",
        '^' | '↑' => "^",
        '!' => "!",
        '‼' => "!!",
        '\'' | '′' => "'",
        '″' => "''",
        '=' => "=",
        '<' => "<",
        '>' => ">",
        '≤' | '⩽' | '≦' => "<=",
        '≥' | '⩾' | '≧' => ">=",
        '≠' => "!=",
        '→' | '⟶' | '↦' => "->",
        '≔' => ":=",
        '∈' => "in",
        ',' => ",",
        ';' => ";",
        '(' => "(",
        ')' => ")",
        '[' => "[",
        ']' => "]",
        '{' => "{",
        '}' => "}",
        '|' => "|",
        '%' => "%",
        '°' => "°",
        '⌊' => "⌊",
        '⌋' => "⌋",
        '⌈' => "⌈",
        '⌉' => "⌉",
        '‖' => "‖",
        '±' => "±",
        '∓' => "∓",
        '∧' | '&' => "&&",
        '∨' => "||",
        '¬' | '~' => "¬",
        '\\' => "\\",
        ':' => ":",
        '@' => "@",
        _ => return None,
    })
}

/// The first argument of `base_conv(number, from, to)` as it is written, and where its comma
/// stands: the digits of a base are no expression.
fn written_argument(chars: &[char], after: usize) -> Option<(String, usize)> {
    let open = (after..chars.len()).find(|k| !chars[*k].is_whitespace())?;
    if chars[open] != '(' {
        return None;
    }
    let (mut depth, mut quoted, mut commas) = (0usize, false, Vec::new());
    let mut closed = false;
    for (k, c) in chars.iter().enumerate().skip(open + 1) {
        match c {
            '"' => quoted = !quoted,
            _ if quoted => {}
            '(' | '[' => depth += 1,
            ')' | ']' if depth == 0 => {
                closed = true;
                break;
            }
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => commas.push(k),
            _ => {}
        }
    }
    if !closed || !matches!(commas.len(), 2 | 3) {
        return None;
    }
    let text: String = chars[open + 1..commas[0]].iter().collect();
    let text = text.trim();
    let text = text
        .strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(text);
    Some((text.to_string(), commas[0]))
}

/// The tokens of a text whose numbers are written in base `radix`.
fn tokenize(src: &str, radix: u32) -> Result<Vec<(Tok, usize)>, String> {
    use crate::radix::{LETTERED, digit_of, scan};
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let digit = |k: usize| chars.get(k).is_some_and(char::is_ascii_digit);
    while i < chars.len() {
        let ch = chars[i];
        let start = i;
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        // (a number that names its base, as 64#Hello, is the same in every base)
        let names_base = ch.is_ascii_digit()
            && chars[i..]
                .iter()
                .find(|c| !c.is_ascii_digit())
                .is_some_and(|c| *c == '#');
        if radix != 10 && ch != '"' && !names_base {
            let found = if radix > LETTERED {
                match ch {
                    '#' => scan(&chars, i, radix, false)?,
                    _ if ch.is_ascii_digit() => {
                        return Err(format!(
                            "a number of base {radix} has a # before every digit, as in #001#234"
                        ));
                    }
                    _ => None,
                }
            } else {
                // a word of digits is a number; a word that starts with digits has one in front
                let word = |c: &char| c.is_ascii_alphanumeric() || *c == '_' || *c == '@';
                let end = (i..chars.len())
                    .find(|k| !word(&chars[*k]))
                    .unwrap_or(chars.len());
                let digits = chars[i..end]
                    .iter()
                    .take_while(|c| digit_of(**c, radix).is_some())
                    .count();
                let point = ch == '.'
                    && chars
                        .get(i + 1)
                        .is_some_and(|c| digit_of(*c, radix).is_some());
                if ch.is_ascii_digit() && digits == 0 {
                    return Err(format!("{ch} is no digit of base {radix}"));
                }
                if point || (digits > 0 && (digits == end - i || ch.is_ascii_digit())) {
                    scan(&chars, i, radix, false)?
                } else {
                    None
                }
            };
            if let Some((value, end)) = found {
                out.push((Tok::Exact(value), start));
                i = end;
                continue;
            }
        }
        if ch == '"' {
            let end = chars[i + 1..]
                .iter()
                .position(|c| *c == '"')
                .ok_or_else(|| format!("the text from position {} has no closing quote", i + 1))?;
            out.push((Tok::Text(chars[i + 1..i + 1 + end].iter().collect()), start));
            i += end + 2;
            continue;
        }
        if ch.is_ascii_digit() || (ch == '.' && digit(i + 1)) {
            let radix = match (ch, chars.get(i + 1)) {
                ('0', Some('x' | 'X')) if chars.get(i + 2).is_some_and(char::is_ascii_hexdigit) => {
                    16
                }
                ('0', Some('b' | 'B')) if matches!(chars.get(i + 2), Some('0' | '1')) => 2,
                ('0', Some('o' | 'O')) if matches!(chars.get(i + 2), Some('0'..='7')) => 8,
                _ => 10,
            };
            if radix != 10
                && let Some((value, end)) = scan(&chars, i + 2, radix, false)?
            {
                out.push((Tok::Exact(value), start));
                i = end;
                continue;
            }
            while digit(i) {
                i += 1;
            }
            // 16#ff.8, 2#0.(01), 1000#001#234: a number written in the base before the sign
            if chars.get(i) == Some(&'#') {
                let base: String = chars[start..i].iter().collect();
                let base = base
                    .parse::<u32>()
                    .ok()
                    .filter(|b| *b >= 2)
                    .ok_or_else(|| {
                        format!(
                            "the base before # is a whole number from 2 to {}, not {base}",
                            u32::MAX
                        )
                    })?;
                let (value, end) = scan(&chars, i + 1, base, true)?
                    .ok_or_else(|| format!("a digit of base {base} must follow {base}#"))?;
                out.push((Tok::Exact(value), start));
                i = end;
                continue;
            }
            if chars.get(i) == Some(&'.')
                && (digit(i + 1) || i > start)
                && !matches!(chars.get(i + 1), Some('.' | '*' | '/' | '^' | '\\'))
            {
                i += 1;
                while digit(i) {
                    i += 1;
                }
            }
            // 0.(3) and 0.1(6): the digits in parentheses repeat without end
            if chars[start..i].contains(&'.') && chars.get(i) == Some(&'(') {
                let mut j = i + 1;
                while digit(j) {
                    j += 1;
                }
                if j > i + 1 && chars.get(j) == Some(&')') {
                    let fixed: String = chars[start..i].iter().collect();
                    let cycle: String = chars[i + 1..j].iter().collect();
                    let places = fixed.len() - fixed.find('.').unwrap_or(0) - 1;
                    let base =
                        parse_decimal(&fixed).ok_or_else(|| format!("invalid number `{fixed}`"))?;
                    let nines = Int::from(Int::u_pow_u(10, cycle.len() as u32)) - 1u32;
                    let tail = Rat::from((
                        Int::from_str_radix(&cycle, 10).map_err(|e| e.to_string())?,
                        nines * Int::from(Int::u_pow_u(10, places as u32)),
                    ));
                    out.push((Tok::Exact(base + tail), start));
                    i = j + 1;
                    continue;
                }
            }
            if matches!(chars.get(i), Some('e' | 'E')) {
                let sign = usize::from(matches!(chars.get(i + 1), Some('+' | '-')));
                if digit(i + 1 + sign) {
                    i += 1 + sign;
                    while digit(i) {
                        i += 1;
                    }
                }
            }
            let text: String = chars[start..i].iter().collect();
            if let Some((p, q)) = chars.get(i).and_then(|c| names::vulgar_fraction(*c))
                && text.chars().all(|c| c.is_ascii_digit())
            {
                i += 1;
                let whole = Int::from_str_radix(&text, 10).map_err(|e| e.to_string())?;
                out.push((Tok::Exact(Rat::from(whole) + Rat::from((p, q))), start));
                continue;
            }
            out.push((Tok::Num(text), start));
            continue;
        }
        if let Some((p, q)) = names::vulgar_fraction(ch) {
            out.push((Tok::Exact(Rat::from((p, q))), start));
            i += 1;
            continue;
        }
        if let Some(first) = names::superscript(ch) {
            let mut text = String::from(first);
            i += 1;
            while let Some(c) = chars.get(i).and_then(|c| names::superscript(*c)) {
                text.push(c);
                i += 1;
            }
            out.push((Tok::Sup(text), start));
            continue;
        }
        if let Some(n) = match ch {
            '√' => Some(2),
            '∛' => Some(3),
            '∜' => Some(4),
            _ => None,
        } {
            out.push((Tok::Root(n), start));
            i += 1;
            continue;
        }
        if let Some(name) = names::symbol_char(ch) {
            out.push((Tok::Ident(name.to_string()), start));
            i += 1;
            continue;
        }
        let greek = names::greek(ch).or(if ch == '\u{2126}' {
            Some("Omega")
        } else {
            None
        });
        if greek.is_some() || ch.is_alphabetic() || ch == '_' {
            let mut name = String::new();
            match greek {
                Some(g) => name.push_str(g),
                None => name.push(ch),
            }
            i += 1;
            while let Some(&c) = chars.get(i) {
                if let Some(d) = names::subscript(c) {
                    name.push(d);
                } else if c.is_ascii_digit()
                    || (greek.is_none()
                        && (c == '_' || (c.is_alphabetic() && names::greek(c).is_none())))
                {
                    name.push(c);
                } else {
                    break;
                }
                i += 1;
            }
            let converts = names::function(&name).is_some_and(|f| f.name == "base_conv");
            if converts && let Some((text, comma)) = written_argument(&chars, i) {
                out.push((Tok::Ident(name), start));
                out.push((Tok::Op("("), i));
                out.push((Tok::Text(text), i + 1));
                i = comma;
                continue;
            }
            out.push((Tok::Ident(name), start));
            continue;
        }
        // the plain spellings of the two-valued signs
        if let Some(op) = match &chars[i..chars.len().min(i + 3)] {
            ['+', '/', '-'] => Some("±"),
            ['-', '/', '+'] => Some("∓"),
            _ => None,
        } {
            out.push((Tok::Op(op), start));
            i += 3;
            continue;
        }
        let two: String = chars[i..chars.len().min(i + 2)].iter().collect();
        if let Some((_, op)) = OPS2.iter().find(|(s, _)| *s == two) {
            out.push((Tok::Op(op), start));
            i += 2;
            continue;
        }
        match op1(ch) {
            Some(op) => out.push((Tok::Op(op), start)),
            None => {
                return Err(format!(
                    "unexpected character `{ch}` at position {}",
                    start + 1
                ));
            }
        }
        i += 1;
    }
    out.push((Tok::End, chars.len()));
    Ok(out)
}

thread_local! {
    static DEFS: RefCell<HashMap<&'static str, Expr>> = RefCell::new(HashMap::new());
}

struct Parser<'a> {
    toks: Vec<(Tok, usize)>,
    pos: usize,
    user_fn: &'a dyn Fn(&str) -> bool,
    max_digits: u32,
    decimal: bool,
    bars: u32,
    depth: u32,
    /// `j` is a quaternion unit, not another name of `i`.
    quaternions: bool,
    /// Inside a command with units: the names of units are symbols.
    units: bool,
    /// Two bars after an operand are `or`, not the bars of an absolute value.
    or_bars: bool,
}

type Res = Result<Expr, String>;

/// How a bare name is read as an argument.
#[derive(Clone, Copy, PartialEq)]
enum Lenient {
    No,
    /// Every bare word is a symbol: `assume(x, real)`.
    Words,
    /// The name of a function stands for the function: `map(sin, list)`.
    Functions,
    /// What is chosen for a picture: `gray`, `scale = 2`, `color = silver`.
    Choices,
}

/// More commands whose arguments may be the bare name of a function.
pub const FUNCTION_ARGS: &[&str] = &[
    "integrate",
    "fzero",
    "fminbnd",
    "fminsearch",
    "lsqcurvefit",
    "quad",
    "integral2",
    "integral3",
    "ode45",
    "fsolve",
    "feval",
    "bsxfun",
    "funm",
    "splitapply",
    "fmincon",
    "jackknife",
    "bootstrp",
    "structfun",
    "bvp4c",
    "bvpinit",
    "deval",
    "dde23",
    "pdepe",
    "patternsearch",
    "particleswarm",
    "simulannealbnd",
    "ga",
    "globalsearch",
    "fminimax",
    "fgoalattain",
    "varfun",
    "rowfun",
];

/// Commands that take a function as an argument.
pub const FUNCTION_FORMS: &[&str] = &[
    "map",
    "select",
    "reject",
    "fold",
    "foldlist",
    "nestlist",
    "nest",
    "fixedpoint",
    "apply",
    "sortby",
    "countif",
    "anytrue",
    "alltrue",
    "findfirst",
    "array",
];

fn raw_pow(b: Expr, e: Expr) -> Expr {
    Expr::Pow(Rc::new(b), Rc::new(e))
}
fn raw_mul(v: Vec<Expr>) -> Expr {
    Expr::Mul(v.into())
}
fn raw_add(v: Vec<Expr>) -> Expr {
    Expr::Add(v.into())
}
fn raw_neg(e: Expr) -> Expr {
    raw_mul(vec![Expr::minus_one(), e])
}

impl Parser<'_> {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos].0
    }
    fn peek_at(&self, k: usize) -> &Tok {
        &self.toks[(self.pos + k).min(self.toks.len() - 1)].0
    }
    fn is_op(&self, op: &str) -> bool {
        matches!(self.peek(), Tok::Op(o) if *o == op)
    }
    fn op_at(&self, k: usize, op: &str) -> bool {
        matches!(self.peek_at(k), Tok::Op(o) if *o == op)
    }
    fn bump(&mut self) -> Tok {
        let t = self.toks[self.pos].0.clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }
    fn eat(&mut self, op: &str) -> bool {
        let hit = self.is_op(op);
        if hit {
            self.bump();
        }
        hit
    }
    fn error(&self, msg: &str) -> String {
        let found = match self.peek() {
            Tok::End => "end of input".to_string(),
            Tok::Num(s) => format!("`{s}`"),
            Tok::Exact(r) => format!("`{r}`"),
            Tok::Text(s) => format!("\"{s}\""),
            Tok::Ident(s) => format!("`{s}`"),
            Tok::Op(o) => format!("`{o}`"),
            Tok::Sup(s) => format!("superscript `{s}`"),
            Tok::Root(_) => "a root sign".to_string(),
        };
        format!(
            "{msg}, found {found} at position {}",
            self.toks[self.pos].1 + 1
        )
    }
    fn expect(&mut self, op: &str) -> Result<(), String> {
        if self.eat(op) {
            Ok(())
        } else {
            Err(self.error(&format!("expected `{op}`")))
        }
    }

    fn statements(&mut self) -> Result<Vec<Expr>, String> {
        let mut out = Vec::new();
        loop {
            while self.eat(";") {}
            if *self.peek() == Tok::End {
                break;
            }
            out.push(self.statement()?);
            if !self.is_op(";") && *self.peek() != Tok::End {
                return Err(self.error("expected an operator or the end of the expression"));
            }
        }
        Ok(out)
    }

    /// Items separated by commas: one item is itself, several are a list.
    fn statement(&mut self) -> Res {
        let mut items = vec![self.item()?];
        while self.eat(",") {
            items.push(self.item()?);
        }
        Ok(if items.len() == 1 {
            items.pop().unwrap()
        } else {
            Expr::list(items)
        })
    }

    fn item(&mut self) -> Res {
        if let Tok::Ident(name) = self.peek().clone() {
            if self.op_at(1, ":=") {
                self.pos += 2;
                let value = self.disjunction()?;
                return Ok(Expr::call(":=", vec![Expr::sym(&name), value]));
            }
            if self.op_at(1, "(") {
                let mut k = 2;
                let mut params = Vec::new();
                loop {
                    match (self.peek_at(k).clone(), self.peek_at(k + 1).clone()) {
                        (Tok::Ident(p), Tok::Op(",")) => {
                            params.push(Expr::sym(&p));
                            k += 2;
                        }
                        (Tok::Ident(p), Tok::Op(")")) => {
                            params.push(Expr::sym(&p));
                            k += 2;
                            break;
                        }
                        _ => {
                            params.clear();
                            break;
                        }
                    }
                }
                if !params.is_empty() && self.op_at(k, ":=") {
                    self.pos += k + 1;
                    let body = self.disjunction()?;
                    return Ok(Expr::call(":=", vec![Expr::call(&name, params), body]));
                }
            }
        }
        self.disjunction()
    }

    fn word(&self, word: &str) -> bool {
        matches!(self.peek(), Tok::Ident(s) if s == word)
    }

    /// `||` written right after an operand.
    fn double_bar(&self) -> bool {
        self.or_bars
            && self.bars == 0
            && self.is_op("|")
            && self.op_at(1, "|")
            && self.toks[self.pos].1 + 1 == self.toks[self.pos + 1].1
    }

    /// `a or b`, `a xor b`.
    fn disjunction(&mut self) -> Res {
        let mut e = self.conjunction()?;
        loop {
            let name = if self.double_bar() {
                self.bump();
                "or"
            } else if self.word("or") || self.is_op("||") {
                "or"
            } else if self.word("xor") {
                "xor"
            } else {
                return Ok(e);
            };
            self.bump();
            let right = self.conjunction()?;
            e = match e {
                Expr::Fn(n, items) if &*n == name => {
                    let mut all = items.to_vec();
                    all.push(right);
                    Expr::call(name, all)
                }
                left => Expr::call(name, vec![left, right]),
            };
        }
    }

    /// `a and b`; a chain `a < b < c` is one too.
    fn conjunction(&mut self) -> Res {
        let first = self.negation()?;
        if !(self.word("and") || self.is_op("&&")) {
            return Ok(first);
        }
        let flat = |e: Expr, out: &mut Vec<Expr>| match e {
            Expr::Fn(n, items) if &*n == "and" => out.extend(items.iter().cloned()),
            other => out.push(other),
        };
        let mut items = Vec::new();
        flat(first, &mut items);
        while self.word("and") || self.is_op("&&") {
            self.bump();
            flat(self.negation()?, &mut items);
        }
        Ok(Expr::call("and", items))
    }

    fn negation(&mut self) -> Res {
        if self.is_op("¬") || self.is_op("!") || (self.word("not") && !self.ends_argument(1)) {
            self.bump();
            return Ok(Expr::call("not", vec![self.negation()?]));
        }
        self.relation()
    }

    fn relop(&self) -> Option<Rel> {
        match self.peek() {
            Tok::Op("=") => Some(Rel::Eq),
            Tok::Op("!=") => Some(Rel::Ne),
            Tok::Op("<") => Some(Rel::Lt),
            Tok::Op("<=") => Some(Rel::Le),
            Tok::Op(">") => Some(Rel::Gt),
            Tok::Op(">=") => Some(Rel::Ge),
            _ => None,
        }
    }

    /// `a:b` and `a:step:b`: the numbers from a to b.
    fn span(&mut self) -> Res {
        let first = self.sum()?;
        if !self.eat(":") {
            return Ok(first);
        }
        let mut parts = vec![first, self.sum()?];
        if self.eat(":") {
            parts.push(self.sum()?);
        }
        Ok(Expr::call("colon", parts))
    }

    fn relation(&mut self) -> Res {
        let first = self.span()?;
        if self.eat("->") {
            let to = self.disjunction()?;
            return Ok(Expr::call("rule", vec![first, to]));
        }
        if self.is_op("in") || matches!(self.peek(), Tok::Ident(s) if s == "in") {
            self.bump();
            let domain = self.span()?;
            return Ok(Expr::call("element", vec![first, domain]));
        }
        let mut rels = Vec::new();
        let mut left = first.clone();
        while let Some(op) = self.relop() {
            self.bump();
            let right = self.span()?;
            rels.push(Expr::rel(op, left, right.clone()));
            left = right;
        }
        Ok(match rels.len() {
            0 => first,
            1 => rels.pop().unwrap(),
            _ => Expr::call("and", rels),
        })
    }

    /// The bracket at the current token closes right before a parenthesis.
    fn bracket_before_call(&self) -> bool {
        let mut depth = 0;
        for (k, (tok, _)) in self.toks[self.pos..].iter().enumerate() {
            match tok {
                Tok::Op("(" | "[") => depth += 1,
                Tok::Op(")" | "]") => {
                    depth -= 1;
                    if depth == 0 {
                        return self.op_at(k + 1, "(");
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// The token at `k` opens a parenthesis that holds two or more arguments.
    fn call_with_comma(&self, k: usize) -> bool {
        if !self.op_at(k, "(") {
            return false;
        }
        let mut depth = 0;
        for (tok, _) in &self.toks[(self.pos + k).min(self.toks.len() - 1)..] {
            match tok {
                Tok::Op("(" | "[") => depth += 1,
                Tok::Op(")" | "]") => {
                    depth -= 1;
                    if depth == 0 {
                        return false;
                    }
                }
                Tok::Op(",") if depth == 1 => return true,
                _ => {}
            }
        }
        false
    }

    fn ends_argument(&self, k: usize) -> bool {
        matches!(self.peek_at(k), Tok::Op(")" | "," | "]") | Tok::End)
    }

    fn sum(&mut self) -> Res {
        self.depth += 1;
        if self.depth > 2000 {
            return Err("the expression is nested too deeply".into());
        }
        let mut terms = vec![self.product()?];
        loop {
            let sign = if self.is_op("+") {
                1
            } else if self.is_op("-") {
                -1
            } else if self.is_op("±") || self.is_op("∓") {
                let lower_first = self.is_op("∓");
                self.bump();
                let t = self.product()?;
                let base = if terms.len() == 1 {
                    terms.pop().unwrap()
                } else {
                    raw_add(std::mem::take(&mut terms))
                };
                let mut pair = vec![
                    raw_add(vec![base.clone(), t.clone()]),
                    raw_add(vec![base, raw_neg(t)]),
                ];
                if lower_first {
                    pair.reverse();
                }
                terms.push(Expr::list(pair));
                continue;
            } else {
                break;
            };
            if self.ends_argument(1) {
                // one-sided limit point: `0+`, `0-`
                self.bump();
                let point = if terms.len() == 1 {
                    terms.pop().unwrap()
                } else {
                    raw_add(std::mem::take(&mut terms))
                };
                self.depth -= 1;
                return Ok(Expr::call("onesided", vec![point, Expr::int(sign)]));
            }
            self.bump();
            let t = self.product()?;
            terms.push(if sign < 0 { raw_neg(t) } else { t });
        }
        self.depth -= 1;
        Ok(if terms.len() == 1 {
            terms.pop().unwrap()
        } else {
            raw_add(terms)
        })
    }

    fn starts_operand(&self) -> bool {
        match self.peek() {
            // (among units `in` is the inch where no operand follows it)
            Tok::Ident(s) if s == "in" => {
                self.units
                    && matches!(
                        self.peek_at(1),
                        Tok::Op(")" | "," | "+" | "-" | "*" | "/" | "^" | "]") | Tok::End
                    )
            }
            Tok::Ident(s) => !matches!(s.as_str(), "and" | "or" | "xor"),
            Tok::Exact(_) | Tok::Root(_) => true,
            Tok::Op(o) => {
                matches!(*o, "(" | "[" | "⌊" | "⌈" | "@")
                    || (*o == "|" && self.bars == 0 && !self.double_bar())
                    || (*o == "‖" && self.bars == 0)
            }
            _ => false,
        }
    }

    fn product(&mut self) -> Res {
        let mut factors = vec![self.unary()?];
        loop {
            if self.eat("*") {
                factors.push(self.unary()?);
            } else if self.eat("/") {
                let d = self.unary()?;
                factors.push(raw_pow(d, Expr::minus_one()));
            } else if (self.is_op("%")
                && !self.ends_argument(1)
                && !matches!(self.peek_at(1), Tok::Op(o) if *o != "(" && *o != "-"))
                || (matches!(self.peek(), Tok::Ident(n) if n == "mod") && !self.call_with_comma(1))
            {
                self.bump();
                let m = self.unary()?;
                let left = if factors.len() == 1 {
                    factors.pop().unwrap()
                } else {
                    raw_mul(std::mem::take(&mut factors))
                };
                factors.push(Expr::call("mod", vec![left, m]));
            } else if let Some(name) = match self.peek() {
                Tok::Op(".*") => Some("times"),
                Tok::Op("./") => Some("rdivide"),
                Tok::Op(".\\") => Some("ldivide"),
                Tok::Op("\\") => Some("mldivide"),
                _ => None,
            } {
                self.bump();
                let right = self.unary()?;
                let left = if factors.len() == 1 {
                    factors.pop().unwrap()
                } else {
                    raw_mul(std::mem::take(&mut factors))
                };
                factors.push(Expr::call(name, vec![left, right]));
            } else if self.starts_operand() {
                factors.push(self.power()?);
            } else if matches!(self.peek(), Tok::Num(_)) {
                return Err(self.error("expected an operator between two numbers"));
            } else {
                break;
            }
        }
        Ok(if factors.len() == 1 {
            factors.pop().unwrap()
        } else {
            raw_mul(factors)
        })
    }

    fn unary(&mut self) -> Res {
        if self.is_op("-") || self.is_op("+") {
            if self.ends_argument(1) {
                let name = if self.is_op("-") { "-" } else { "+" };
                self.bump();
                return Ok(Expr::sym(name));
            }
            let negative = self.is_op("-");
            self.bump();
            let inner = self.unary()?;
            return Ok(if negative { raw_neg(inner) } else { inner });
        }
        if self.is_op("±") || self.is_op("∓") {
            let lower_first = self.is_op("∓");
            self.bump();
            let inner = self.unary()?;
            let mut pair = vec![inner.clone(), raw_neg(inner)];
            if lower_first {
                pair.reverse();
            }
            return Ok(Expr::list(pair));
        }
        self.power()
    }

    fn power(&mut self) -> Res {
        let base = self.tetration()?;
        if self.eat("^") {
            let exponent = self.unary()?;
            return Ok(raw_pow(base, exponent));
        }
        if self.eat(".^") {
            let exponent = self.unary()?;
            return Ok(Expr::call("power", vec![base, exponent]));
        }
        Ok(base)
    }

    fn tetration(&mut self) -> Res {
        let base = self.postfix()?;
        if self.eat("^^") {
            let negative = self.eat("-");
            if !negative {
                self.eat("+");
            }
            let height = self.tetration()?;
            return Ok(Expr::call(
                "tet",
                vec![base, if negative { raw_neg(height) } else { height }],
            ));
        }
        Ok(base)
    }

    /// `f'`; for a user function `f'(x)` is the derivative of the call `f(x)`.
    fn primed(&mut self, mut e: Expr, times: u32) -> Res {
        let mut depth = times;
        while let Expr::Fn(n, a) = &e
            && &**n == "derivative'"
        {
            depth += 1;
            e = a[0].clone();
        }
        if let Expr::Sym(name) = &e
            && (self.user_fn)(name)
            && self.is_op("(")
        {
            e = Expr::call(name, self.arguments(Lenient::No)?);
        }
        Ok((0..depth).fold(e, |inner, _| Expr::call("derivative'", vec![inner])))
    }

    fn postfix(&mut self) -> Res {
        let mut e = self.primary()?;
        loop {
            if self.eat("!!") {
                e = Expr::call("factorial2", vec![e]);
            } else if self.is_op("!") {
                self.bump();
                e = Expr::call("factorial", vec![e]);
            } else if self.eat("''") {
                e = self.primed(e, 2)?;
            } else if self.eat("'") {
                e = self.primed(e, 1)?;
            } else if self.eat("°") {
                e = raw_mul(vec![
                    e,
                    Expr::pi(),
                    raw_pow(Expr::int(180), Expr::minus_one()),
                ]);
            } else if let Tok::Sup(text) = self.peek().clone() {
                self.bump();
                let exponent = parse_with(&text, self.user_fn)?
                    .stmts
                    .pop()
                    .ok_or("empty superscript")?;
                e = raw_pow(e, exponent);
            } else if self.is_op("[")
                && self.op_at(1, "[")
                && !matches!(e, Expr::Num(_))
                && self.double_bracket_closes()
            {
                // v[[i]], m[[i, j]]: a part of a list
                self.pos += 2;
                let mut args = vec![e, self.item()?];
                while self.eat(",") {
                    args.push(self.item()?);
                }
                self.expect("]")?;
                self.expect("]")?;
                e = Expr::call("part", args);
            } else {
                return Ok(e);
            }
        }
    }

    /// The inner of two opening brackets at the current token closes right before another `]`.
    fn double_bracket_closes(&self) -> bool {
        let mut depth = 0;
        for (k, (tok, _)) in self.toks[self.pos + 1..].iter().enumerate() {
            match tok {
                Tok::Op("(" | "[" | "{") => depth += 1,
                Tok::Op(")" | "]" | "}") => {
                    depth -= 1;
                    if depth == 0 {
                        return self.op_at(k + 2, "]");
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn arguments(&mut self, lenient: Lenient) -> Result<Vec<Expr>, String> {
        self.expect("(")?;
        let outer = std::mem::take(&mut self.bars);
        let mut args = Vec::new();
        if !self.eat(")") {
            loop {
                if lenient == Lenient::Choices {
                    args.push(self.choice()?);
                } else if lenient != Lenient::No
                    && let Tok::Ident(word) = self.peek().clone()
                    && self.ends_argument(1)
                    && (lenient == Lenient::Words
                        || (self.user_fn)(&word)
                        || (names::function(&word).is_some()
                            && (args.is_empty() || names::constant(&word).is_none())))
                {
                    self.bump();
                    args.push(Expr::sym(&word));
                } else {
                    args.push(self.item()?);
                }
                if self.eat(")") {
                    break;
                }
                self.expect(",")?;
            }
        }
        self.bars = outer;
        Ok(args)
    }

    /// An argument of a command that draws: the words and the names of what is chosen for a
    /// picture are no functions and no constants there.
    fn choice(&mut self) -> Res {
        let Tok::Ident(word) = self.peek().clone() else {
            return self.item();
        };
        if self.ends_argument(1) && crate::picture::WORDS.contains(&word.as_str()) {
            self.bump();
            return Ok(Expr::sym(&word));
        }
        // rotate(yaw, pitch, roll) turns the body of a picture of space: any count of angles
        if word == "rotate" && self.op_at(1, "(") {
            self.bump();
            let args = self.arguments(Lenient::No)?;
            return Ok(Expr::call("rotate", args));
        }
        if !matches!(self.peek_at(1), Tok::Op("="))
            || !crate::picture::NAMED.contains(&word.as_str())
        {
            return self.item();
        }
        self.bump();
        self.bump();
        let value = match self.peek().clone() {
            Tok::Ident(name) if crate::picture::takes_name(&word) && self.ends_argument(1) => {
                self.bump();
                Expr::sym(&name)
            }
            Tok::Op(open @ ("[" | "{")) if word == "palette" => {
                self.colours(if open == "[" { "]" } else { "}" })?
            }
            _ => self.item()?,
        };
        Ok(Expr::rel(crate::expr::Rel::Eq, Expr::sym(&word), value))
    }

    /// The colours of a palette: names, or numbers that may be written (red, green, blue).
    fn colours(&mut self, close: &str) -> Res {
        self.bump();
        let outer = std::mem::take(&mut self.bars);
        let mut items = Vec::new();
        if !self.eat(close) {
            loop {
                let alone = self.op_at(1, ",") || self.op_at(1, close);
                items.push(match self.peek().clone() {
                    Tok::Ident(name) if alone => {
                        self.bump();
                        Expr::sym(&name)
                    }
                    Tok::Op("(") => {
                        self.bump();
                        let mut parts = vec![self.item()?];
                        while self.eat(",") {
                            parts.push(self.item()?);
                        }
                        self.expect(")")?;
                        if parts.len() == 1 {
                            parts.remove(0)
                        } else {
                            Expr::list(parts)
                        }
                    }
                    _ => self.item()?,
                });
                if self.eat(close) {
                    break;
                }
                self.expect(",")?;
            }
        }
        self.bars = outer;
        Ok(Expr::list(items))
    }

    fn enclosed(&mut self, close: &str, name: &str) -> Res {
        self.bump();
        let outer = std::mem::take(&mut self.bars);
        if close == "|" || close == "‖" {
            self.bars = 1;
        }
        let inner = self.statement()?;
        self.bars = outer;
        self.expect(close)?;
        Ok(Expr::call(name, vec![inner]))
    }

    fn list(&mut self, close: &str) -> Res {
        self.bump();
        let outer = std::mem::take(&mut self.bars);
        let mut rows: Vec<Vec<Expr>> = vec![Vec::new()];
        if !self.eat(close) {
            loop {
                rows.last_mut().unwrap().push(self.item()?);
                if self.eat(close) {
                    break;
                }
                if self.eat(";") {
                    rows.push(Vec::new());
                } else {
                    self.expect(",")?;
                }
            }
        }
        self.bars = outer;
        Ok(if rows.len() == 1 {
            Expr::list(rows.pop().unwrap())
        } else {
            Expr::list(rows.into_iter().map(Expr::list).collect())
        })
    }

    /// `d/dx f`, `d^2/dx^2 f`, `∂/∂x f`.
    fn leibniz(&mut self, name: &str) -> Option<Res> {
        let var_of = |t: &Tok| match t {
            Tok::Ident(s) if s.len() > 1 && s.starts_with('d') => Some(s[1..].to_string()),
            _ => None,
        };
        let (skip, var, order) =
            if name == "d" && self.op_at(1, "/") && var_of(self.peek_at(2)).is_some() {
                (3, var_of(self.peek_at(2))?, 1)
            } else if name == "d"
                && self.op_at(1, "^")
                && self.op_at(3, "/")
                && self.op_at(5, "^")
                && self.peek_at(2) == self.peek_at(6)
                && let Tok::Num(n) = self.peek_at(2).clone()
            {
                (7, var_of(self.peek_at(4))?, n.parse::<i64>().ok()?)
            } else if name == "diff"
                && self.op_at(1, "/")
                && matches!(self.peek_at(2), Tok::Ident(s) if s == "diff")
            {
                match self.peek_at(3).clone() {
                    Tok::Ident(v) => (4, v, 1),
                    _ => return None,
                }
            } else {
                return None;
            };
        self.pos += skip;
        Some(
            self.product()
                .map(|f| Expr::call("diff", vec![f, Expr::sym(&var), Expr::int(order)])),
        )
    }

    fn constant(&mut self, c: &'static names::Const) -> Res {
        if c.def.is_empty() {
            return Ok(match c.name {
                "i" => Expr::i(),
                "true" => Expr::Bool(true),
                "false" => Expr::Bool(false),
                name => Expr::sym(name),
            });
        }
        if let Some(hit) = DEFS.with(|d| d.borrow().get(c.name).cloned()) {
            return Ok(hit);
        }
        let def = parse_with(c.def, &|_| false)?
            .stmts
            .pop()
            .ok_or("empty definition")?;
        DEFS.with(|d| d.borrow_mut().insert(c.name, def.clone()));
        Ok(def)
    }

    fn primary(&mut self) -> Res {
        match self.peek().clone() {
            Tok::Num(text) => {
                self.bump();
                let value =
                    parse_decimal(&text).ok_or_else(|| format!("invalid number `{text}`"))?;
                self.max_digits = self.max_digits.max(literal_digits(&text));
                self.decimal |= text.contains(['.', 'e', 'E']);
                Ok(Expr::rational(value))
            }
            Tok::Exact(r) => {
                self.bump();
                Ok(Expr::rational(r))
            }
            Tok::Text(text) => {
                self.bump();
                Ok(Expr::call("string", vec![Expr::sym(&text)]))
            }
            Tok::Root(n) => {
                self.bump();
                let operand = self.postfix()?;
                Ok(raw_pow(operand, Expr::num(Num::frac(1, i64::from(n)))))
            }
            Tok::Ident(name) => {
                if self.units
                    && crate::units::is_unit(&name)
                    && !matches!(self.peek_at(1), Tok::Op("("))
                {
                    self.bump();
                    return Ok(Expr::sym(&name));
                }
                if let Some(r) = self.leibniz(&name) {
                    return r;
                }
                self.bump();
                // sin²(x): the power of the function's value
                if let Tok::Sup(text) = self.peek().clone()
                    && matches!(self.peek_at(1), Tok::Op("("))
                    && name.chars().count() > 1
                    && let Some(f) = names::function(&name)
                {
                    let exponent = parse_with(&text, self.user_fn)?
                        .stmts
                        .pop()
                        .ok_or("empty superscript")?;
                    if exponent.as_i64().is_none_or(|k| k < 1) {
                        return Err(format!(
                            "`{name}` with this superscript is unclear: write the power or the inverse function out"
                        ));
                    }
                    self.bump();
                    let args = self.arguments(Lenient::No)?;
                    return Ok(raw_pow(build_call(f, args)?, exponent));
                }
                // f[t](x): the function applied t times
                if self.is_op("[")
                    && ((self.user_fn)(&name) || names::function(&name).is_some())
                    && self.bracket_before_call()
                {
                    self.bump();
                    let steps = self.item()?;
                    self.expect("]")?;
                    let mut args = self.arguments(Lenient::No)?;
                    if args.len() != 1 {
                        return Err(format!(
                            "{name}[t](x) takes one argument: the function applied t times"
                        ));
                    }
                    let dummy = Expr::sym("#it");
                    let body = if (self.user_fn)(&name) {
                        Expr::call(&name, vec![dummy.clone()])
                    } else {
                        build_call(
                            names::function(&name).expect("checked"),
                            vec![dummy.clone()],
                        )?
                    };
                    return Ok(Expr::call(
                        "iterate",
                        vec![body, dummy, steps, args.remove(0)],
                    ));
                }
                if self.is_op("(") {
                    if (self.user_fn)(&name) {
                        let args = self.arguments(Lenient::No)?;
                        return Ok(Expr::call(&name, args));
                    }
                    if let Some(f) = names::function(&name) {
                        // inside qsolve j is the quaternion unit of every domain
                        let before = (self.quaternions, self.units);
                        self.quaternions |= f.name == "qsolve";
                        self.units |= crate::units::takes_units(f.name);
                        let args = self.arguments(match f.name {
                            "assume" | "forget" | "domain" | "polytope" | "clear" => Lenient::Words,
                            name if FUNCTION_FORMS.contains(&name)
                                || FUNCTION_ARGS.contains(&name) =>
                            {
                                Lenient::Functions
                            }
                            name if crate::picture::takes_choices(name) => Lenient::Choices,
                            _ => Lenient::No,
                        });
                        (self.quaternions, self.units) = before;
                        let args = args?;
                        if f.name == "domain"
                            && let Some(word) = args.first().and_then(Expr::sym_name)
                        {
                            self.quaternions = matches!(word, "quaternion" | "quaternions" | "H");
                        }
                        return build_call(f, args);
                    }
                    let variable_like = name.chars().count() == 1
                        || names::is_greek_name(&name)
                        || name.chars().any(|c| c.is_ascii_digit() || c == '_');
                    if names::constant(&name).is_none() && !variable_like {
                        return Err(format!(
                            "unknown function `{name}` (write `{name}*(...)` for a product; --functions lists the functions)"
                        ));
                    }
                }
                if self.quaternions && name == "j" {
                    return Ok(Expr::sym("j"));
                }
                if let Some(c) = names::constant(&name) {
                    return self.constant(c);
                }
                if name.chars().count() > 1
                    && !names::is_greek_name(&name)
                    && !crate::units::is_unit(&name)
                    && names::function(&name).is_some()
                {
                    return Err(format!("expected `(` after the function name `{name}`"));
                }
                Ok(Expr::sym(if name == "ans" { "ans" } else { &name }))
            }
            Tok::Op("(") => {
                self.bump();
                let outer = std::mem::take(&mut self.bars);
                let inner = self.statement()?;
                self.bars = outer;
                self.expect(")")?;
                Ok(inner)
            }
            Tok::Op("@") => {
                // @(x) expr is x -> expr, @name is the function of that name
                self.bump();
                if let Tok::Ident(name) = self.peek().clone() {
                    self.bump();
                    return Ok(Expr::sym(&name));
                }
                self.expect("(")?;
                let mut params = Vec::new();
                loop {
                    match self.bump() {
                        Tok::Ident(p) => params.push(Expr::sym(&p)),
                        _ => {
                            return Err(
                                "@(x) expr: the names of the arguments stand in the parentheses"
                                    .into(),
                            );
                        }
                    }
                    if self.eat(")") {
                        break;
                    }
                    self.expect(",")?;
                }
                let body = self.disjunction()?;
                let head = if params.len() == 1 {
                    params.pop().unwrap()
                } else {
                    Expr::list(params)
                };
                Ok(Expr::call("rule", vec![head, body]))
            }
            Tok::Op("[") => self.list("]"),
            Tok::Op("{") => self.list("}"),
            Tok::Op("|") => self.enclosed("|", "abs"),
            Tok::Op("‖") => self.enclosed("‖", "norm"),
            Tok::Op("⌊") => self.enclosed("⌋", "floor"),
            Tok::Op("⌈") => self.enclosed("⌉", "ceil"),
            Tok::Op("%") => {
                self.bump();
                Ok(Expr::sym("ans"))
            }
            _ => Err(self.error("expected a number, a name or `(`")),
        }
    }
}

fn build_call(f: &names::Func, mut a: Vec<Expr>) -> Res {
    let n = a.len();
    if n < usize::from(f.min) || (f.max != names::ANY && n > usize::from(f.max)) {
        let want = if f.min == f.max {
            format!("{}", f.min)
        } else if f.max == names::ANY {
            format!("at least {}", f.min)
        } else {
            format!("{} to {}", f.min, f.max)
        };
        return Err(format!("{} takes {want} argument(s), {n} given", f.name));
    }
    let call = |name: &str, args: Vec<Expr>| Expr::call(name, args);
    let times_i = |x: Expr| raw_mul(vec![Expr::i(), x]);
    let mut next = || a.remove(0);
    Ok(match (f.name, n) {
        ("sqr", _) => raw_pow(next(), Expr::int(2)),
        ("pow", _) => {
            let base = next();
            raw_pow(base, next())
        }
        ("exp2", _) => raw_pow(Expr::int(2), next()),
        ("expf", _) => {
            let z = next();
            raw_pow(next(), z)
        }
        ("abs2", _) => raw_pow(call("abs", vec![next()]), Expr::int(2)),
        ("l1_norm", _) => {
            let z = next();
            raw_add(vec![
                call("abs", vec![call("re", vec![z.clone()])]),
                call("abs", vec![call("im", vec![z])]),
            ])
        }
        ("to_polar", _) => {
            let z = next();
            raw_add(vec![
                call("abs", vec![z.clone()]),
                times_i(call("arg", vec![z])),
            ])
        }
        ("cis", _) => raw_pow(Expr::e(), times_i(next())),
        ("scale", _) => raw_mul(vec![next(), next()]),
        ("unscale" | "fdiv", _) => {
            let z = next();
            raw_mul(vec![z, raw_pow(next(), Expr::minus_one())])
        }
        ("complex", _) | ("rect", 2) => {
            let x = next();
            raw_add(vec![x, times_i(next())])
        }
        ("rect", _) => call("rectform", vec![next()]),
        ("polar", 2) => {
            let r = next();
            raw_mul(vec![r, raw_pow(Expr::e(), times_i(next()))])
        }
        ("polar", _) => call("polarform", vec![next()]),
        ("hypot", _) => {
            let (x, y) = (next(), next());
            raw_pow(
                raw_add(vec![raw_pow(x, Expr::int(2)), raw_pow(y, Expr::int(2))]),
                Expr::half(),
            )
        }
        ("mod", 1) => call("abs", vec![next()]),
        ("D", _) => call("diff", a),
        ("gamma", 2 | 3) => call("uppergamma", a),
        ("beta", 3) => call("betainc", a),
        ("lambertw", 2) => call("lambertw", a),
        ("select", 3 | 4) => call("lfilter", a),
        ("inv", 3) => crate::mstats::rewrite("finv", a)?,
        (name, _) if crate::mstats::is_name(name) => crate::mstats::rewrite(name, a)?,
        ("pdf" | "cdf" | "quantile" | "random", 2..)
            if crate::mstats::by_title(f.name, &a).is_some() =>
        {
            crate::mstats::by_title(f.name, &a).expect("checked")?
        }
        (name, _) if matlab_form(name, n) => matlab_call(name, a),
        (name, _) => call(name, a),
    })
}

/// The functions of MATLAB that are other functions in another dress.
fn matlab_form(name: &str, n: usize) -> bool {
    matches!(
        (name, n),
        (
            "expm1"
                | "log1p"
                | "sind"
                | "cosd"
                | "tand"
                | "cotd"
                | "secd"
                | "cscd"
                | "asind"
                | "acosd"
                | "atand"
                | "acotd"
                | "asecd"
                | "acscd"
                | "atan2d"
                | "sinpi"
                | "cospi"
                | "deg2rad"
                | "rad2deg"
                | "pow2"
                | "erfcx"
                | "dawson"
                | "betaln"
                | "ssinint"
                | "whittakerm"
                | "whittakerw"
                | "wrightomega"
                | "rectangularpulse"
                | "triangularpulse"
                | "jacobicd"
                | "jacobisd"
                | "jacobind"
                | "jacobidc"
                | "jacobinc"
                | "jacobisc"
                | "jacobins"
                | "jacobids"
                | "jacobics"
                | "jacobiam"
                | "jacobizeta"
                | "ellipticck"
                | "ellipticce"
                | "ellipticcpi"
                | "ellipke"
                | "ellipj"
                | "gammaincinv"
                | "betaincinv",
            _
        ) | ("expint" | "kroneckerdelta" | "angle" | "airy", 1)
            | ("digamma" | "besselh", 2)
    )
}

fn matlab_call(name: &str, mut a: Vec<Expr>) -> Expr {
    let n = a.len();
    let call = |name: &str, args: Vec<Expr>| Expr::call(name, args);
    let over = |x: Expr, y: Expr| raw_mul(vec![x, raw_pow(y, Expr::minus_one())]);
    let less = |x: Expr, y: Expr| raw_add(vec![x, raw_neg(y)]);
    let step = |x: Expr| call("heaviside", vec![x]);
    let mut next = || a.remove(0);
    match (name, n) {
        ("expm1", _) => less(raw_pow(Expr::e(), next()), Expr::one()),
        ("log1p", _) => call("ln", vec![raw_add(vec![Expr::one(), next()])]),
        ("sind" | "cosd" | "tand" | "cotd" | "secd" | "cscd", _) => call(
            &name[..3],
            vec![over(raw_mul(vec![next(), Expr::pi()]), Expr::int(180))],
        ),
        ("asind" | "acosd" | "atand" | "acotd" | "asecd" | "acscd", _) => over(
            raw_mul(vec![Expr::int(180), call(&name[..4], vec![next()])]),
            Expr::pi(),
        ),
        ("atan2d", _) => over(raw_mul(vec![Expr::int(180), call("atan2", a)]), Expr::pi()),
        ("sinpi" | "cospi", _) => call(&name[..3], vec![raw_mul(vec![Expr::pi(), next()])]),
        ("deg2rad", _) => over(raw_mul(vec![next(), Expr::pi()]), Expr::int(180)),
        ("rad2deg", _) => over(raw_mul(vec![Expr::int(180), next()]), Expr::pi()),
        ("pow2", 1) => raw_pow(Expr::int(2), next()),
        ("pow2", _) => {
            let scale = next();
            raw_mul(vec![scale, raw_pow(Expr::int(2), next())])
        }
        ("erfcx", _) => {
            let x = next();
            raw_mul(vec![
                raw_pow(Expr::e(), raw_pow(x.clone(), Expr::int(2))),
                call("erfc", vec![x]),
            ])
        }
        ("dawson", _) => {
            let x = next();
            raw_mul(vec![
                Expr::half(),
                raw_pow(Expr::pi(), Expr::half()),
                raw_pow(Expr::e(), raw_neg(raw_pow(x.clone(), Expr::int(2)))),
                call("erfi", vec![x]),
            ])
        }
        ("betaln", _) => call("ln", vec![call("beta", a)]),
        ("ssinint", _) => less(call("si", vec![next()]), over(Expr::pi(), Expr::int(2))),
        ("expint", _) => call("expint", vec![Expr::one(), next()]),
        ("kroneckerdelta", _) => call("kroneckerdelta", vec![next(), Expr::zero()]),
        ("angle", _) => call("arg", a),
        ("airy", _) => call("airyai", a),
        ("digamma", _) => call("polygamma", a),
        ("besselh", _) => call("hankel1", a),
        ("whittakerm" | "whittakerw", _) => {
            let (k, m, z) = (next(), next(), next());
            let first = raw_add(vec![m.clone(), raw_neg(k), Expr::half()]);
            let second = raw_add(vec![Expr::one(), raw_mul(vec![Expr::int(2), m.clone()])]);
            let kummer = if name == "whittakerm" {
                "hyp1f1"
            } else {
                "hyperu"
            };
            raw_mul(vec![
                raw_pow(Expr::e(), raw_mul(vec![Expr::rat(-1, 2), z.clone()])),
                raw_pow(z.clone(), raw_add(vec![m, Expr::half()])),
                call(kummer, vec![first, second, z]),
            ])
        }
        ("wrightomega", _) => {
            let x = next();
            let branch = call(
                "ceil",
                vec![over(
                    less(call("im", vec![x.clone()]), Expr::pi()),
                    raw_mul(vec![Expr::int(2), Expr::pi()]),
                )],
            );
            call("lambertw", vec![branch, raw_pow(Expr::e(), x)])
        }
        ("rectangularpulse", 3) => {
            let (lo, hi, x) = (next(), next(), next());
            less(step(less(x.clone(), lo)), step(less(x, hi)))
        }
        ("rectangularpulse", _) => {
            let x = next();
            less(
                step(raw_add(vec![x.clone(), Expr::half()])),
                step(less(x, Expr::half())),
            )
        }
        ("triangularpulse", _) => {
            let (lo, mid, hi, x) = match n {
                4 => (next(), next(), next(), next()),
                3 => {
                    let (lo, hi) = (next(), next());
                    let mid = over(raw_add(vec![lo.clone(), hi.clone()]), Expr::int(2));
                    (lo, mid, hi, next())
                }
                _ => (Expr::minus_one(), Expr::zero(), Expr::one(), next()),
            };
            let rise = raw_mul(vec![
                over(less(x.clone(), lo.clone()), less(mid.clone(), lo.clone())),
                less(
                    step(less(x.clone(), lo)),
                    step(less(x.clone(), mid.clone())),
                ),
            ]);
            let fall = raw_mul(vec![
                over(less(hi.clone(), x.clone()), less(hi.clone(), mid.clone())),
                less(step(less(x.clone(), mid)), step(less(x, hi))),
            ]);
            raw_add(vec![rise, fall])
        }
        ("jacobiam" | "jacobizeta", _) => {
            let (u, m) = (next(), next());
            let args = || vec![u.clone(), m.clone()];
            let quarter = call("elliptick", vec![m.clone()]);
            let angle = call(
                "atan2",
                vec![call("jacobisn", args()), call("jacobicn", args())],
            );
            let turns = call(
                "floor",
                vec![over(
                    raw_add(vec![
                        u.clone(),
                        raw_mul(vec![Expr::int(2), quarter.clone()]),
                    ]),
                    raw_mul(vec![Expr::int(4), quarter.clone()]),
                )],
            );
            if name == "jacobiam" {
                raw_add(vec![angle, raw_mul(vec![Expr::int(2), Expr::pi(), turns])])
            } else {
                let near = less(
                    u.clone(),
                    raw_mul(vec![Expr::int(4), quarter.clone(), turns]),
                );
                less(
                    call("elliptice", vec![angle, m.clone()]),
                    raw_mul(vec![
                        over(call("elliptice", vec![m.clone()]), quarter),
                        near,
                    ]),
                )
            }
        }
        (
            "jacobicd" | "jacobisd" | "jacobind" | "jacobidc" | "jacobinc" | "jacobisc"
            | "jacobins" | "jacobids" | "jacobics",
            _,
        ) => {
            let part = |letter: u8| match letter {
                b's' => call("jacobisn", a.clone()),
                b'c' => call("jacobicn", a.clone()),
                b'd' => call("jacobidn", a.clone()),
                _ => Expr::one(),
            };
            let letters = name.as_bytes();
            over(part(letters[6]), part(letters[7]))
        }
        ("ellipticck" | "ellipticce", _) => {
            let m = next();
            call(
                if name == "ellipticck" {
                    "elliptick"
                } else {
                    "elliptice"
                },
                vec![less(Expr::one(), m)],
            )
        }
        ("ellipticcpi", _) => {
            let (nu, m) = (next(), next());
            call("ellipticpi", vec![nu, less(Expr::one(), m)])
        }
        ("ellipke", _) => {
            let m = next();
            Expr::list(vec![
                call("elliptick", vec![m.clone()]),
                call("elliptice", vec![m]),
            ])
        }
        ("ellipj", _) => Expr::list(vec![
            call("jacobisn", a.clone()),
            call("jacobicn", a.clone()),
            call("jacobidn", a),
        ]),
        ("gammaincinv", _) => {
            let (p, shape) = (next(), next());
            call(
                "quantile",
                vec![call("gammadist", vec![shape, Expr::one()]), p],
            )
        }
        (_, _) => {
            let (p, first, second) = (next(), next(), next());
            call("quantile", vec![call("betadist", vec![first, second]), p])
        }
    }
}

/// The call of a built-in function by any of its names, as the parser builds it.
pub fn call_by_name(name: &str, args: Vec<Expr>) -> Res {
    match names::function(name) {
        Some(f) => build_call(f, args),
        None => Err(format!("`{name}` is not a function")),
    }
}

pub fn parse_with(src: &str, user_fn: &dyn Fn(&str) -> bool) -> Result<Parsed, String> {
    parse_radix(src, 10, user_fn)
}

/// The statements of a text whose numbers are written in a base: there a word made of digits
/// of the base is a number.
pub fn parse_radix(
    src: &str,
    radix: u32,
    user_fn: &dyn Fn(&str) -> bool,
) -> Result<Parsed, String> {
    let toks = tokenize(src, radix)?;
    let read = |or_bars: bool| {
        let mut p = Parser {
            toks: toks.clone(),
            pos: 0,
            user_fn,
            max_digits: 0,
            decimal: false,
            bars: 0,
            depth: 0,
            quaternions: crate::assume::quaternion_mode(),
            units: false,
            or_bars,
        };
        let stmts = p.statements()?;
        Ok(Parsed {
            stmts,
            max_digits: p.max_digits,
            decimal: p.decimal,
        })
    };
    // a || b is `or`; where that reading fails the bars are those of absolute values
    let doubled = toks
        .windows(2)
        .any(|w| w[0].0 == Tok::Op("|") && w[1].0 == Tok::Op("|") && w[0].1 + 1 == w[1].1);
    match read(doubled) {
        Err(_) if doubled => read(false),
        other => other,
    }
}

pub fn parse(src: &str) -> Result<Parsed, String> {
    parse_with(src, &|_| false)
}
