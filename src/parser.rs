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
        '∧' => "&&",
        '∨' => "||",
        '¬' => "¬",
        _ => return None,
    })
}

fn tokenize(src: &str) -> Result<Vec<(Tok, usize)>, String> {
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
        if ch.is_ascii_digit() || (ch == '.' && digit(i + 1)) {
            let radix = match (ch, chars.get(i + 1)) {
                ('0', Some('x' | 'X')) if chars.get(i + 2).is_some_and(char::is_ascii_hexdigit) => {
                    16
                }
                ('0', Some('b' | 'B')) if matches!(chars.get(i + 2), Some('0' | '1')) => 2,
                _ => 10,
            };
            if radix != 10 {
                i += 2;
                let from = i;
                while chars.get(i).is_some_and(|c| c.is_digit(radix)) {
                    i += 1;
                }
                let text: String = chars[from..i].iter().collect();
                let n = Int::from_str_radix(&text, radix as i32).map_err(|e| e.to_string())?;
                out.push((Tok::Exact(Rat::from(n)), start));
                continue;
            }
            while digit(i) {
                i += 1;
            }
            if chars.get(i) == Some(&'.')
                && (digit(i + 1) || i > start)
                && chars.get(i + 1) != Some(&'.')
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
}

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

    /// `a or b`, `a xor b`.
    fn disjunction(&mut self) -> Res {
        let mut e = self.conjunction()?;
        loop {
            let name = if self.word("or") || self.is_op("||") {
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
        if self.is_op("¬") || (self.word("not") && !self.ends_argument(1)) {
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

    fn relation(&mut self) -> Res {
        let first = self.sum()?;
        if self.eat("->") {
            let to = self.disjunction()?;
            return Ok(Expr::call("rule", vec![first, to]));
        }
        if self.is_op("in") || matches!(self.peek(), Tok::Ident(s) if s == "in") {
            self.bump();
            let domain = self.sum()?;
            return Ok(Expr::call("element", vec![first, domain]));
        }
        let mut rels = Vec::new();
        let mut left = first.clone();
        while let Some(op) = self.relop() {
            self.bump();
            let right = self.sum()?;
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
                matches!(*o, "(" | "[" | "⌊" | "⌈")
                    || (*o == "|" && self.bars == 0)
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
                if lenient != Lenient::No
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
                            "assume" | "forget" | "domain" | "polytope" => Lenient::Words,
                            name if FUNCTION_FORMS.contains(&name) => Lenient::Functions,
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
        (name, _) => call(name, a),
    })
}

/// The call of a built-in function by any of its names, as the parser builds it.
pub fn call_by_name(name: &str, args: Vec<Expr>) -> Res {
    match names::function(name) {
        Some(f) => build_call(f, args),
        None => Err(format!("`{name}` is not a function")),
    }
}

pub fn parse_with(src: &str, user_fn: &dyn Fn(&str) -> bool) -> Result<Parsed, String> {
    let mut p = Parser {
        toks: tokenize(src)?,
        pos: 0,
        user_fn,
        max_digits: 0,
        decimal: false,
        bars: 0,
        depth: 0,
        quaternions: crate::assume::quaternion_mode(),
        units: false,
    };
    let stmts = p.statements()?;
    Ok(Parsed {
        stmts,
        max_digits: p.max_digits,
        decimal: p.decimal,
    })
}

pub fn parse(src: &str) -> Result<Parsed, String> {
    parse_with(src, &|_| false)
}
