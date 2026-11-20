//! Patterns of texts as MATLAB writes them (regular expressions): what matches, where, the
//! tokens in brackets, the parts between, and texts with the matches replaced.

use crate::arrays::whole;
use crate::expr::Expr;
use crate::radix::text_of;
use crate::texts::text;
use std::cell::Cell;

type Res = Result<Expr, String>;
type Caps = Vec<Option<(usize, usize)>>;
type Then<'k> = &'k dyn Fn(usize, &mut Caps) -> bool;

/// The steps that a search may take before it is given up, how deep it may nest, and the
/// room that this depth is given (a search runs on a stack of its own).
const STEPS: u64 = 20_000_000;
const DEPTH: usize = 100_000;
const ROOM: usize = 512 << 20;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Class {
    Digit,
    Word,
    Space,
}

#[derive(Clone, PartialEq, Debug)]
enum Item {
    Range(char, char),
    Class(Class, bool),
}

#[derive(Clone, PartialEq, Debug)]
enum Node {
    Char(char),
    Any,
    Set(Vec<Item>, bool),
    Start,
    End,
    WordStart,
    WordEnd,
    Seq(Vec<Node>),
    Alt(Vec<Node>),
    Group(Box<Node>, usize),
    Repeat {
        inner: Box<Node>,
        least: usize,
        most: Option<usize>,
        lazy: bool,
    },
    Look {
        inner: Box<Node>,
        ahead: bool,
        wanted: bool,
    },
    Atomic(Box<Node>),
    Back(usize),
}

pub struct Pattern {
    node: Node,
    groups: usize,
    names: Vec<(String, usize)>,
    fold: bool,
}

struct Parser {
    chars: Vec<char>,
    at: usize,
    groups: usize,
    names: Vec<(String, usize)>,
    fold: bool,
}

fn in_class(class: Class, c: char) -> bool {
    match class {
        Class::Digit => c.is_ascii_digit(),
        Class::Word => c.is_alphanumeric() || c == '_',
        Class::Space => matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c'),
    }
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek();
        self.at += usize::from(c.is_some());
        c
    }

    fn eat(&mut self, c: char) -> bool {
        let there = self.peek() == Some(c);
        self.at += usize::from(there);
        there
    }

    fn alternation(&mut self) -> Result<Node, String> {
        let mut branches = vec![self.sequence()?];
        while self.eat('|') {
            branches.push(self.sequence()?);
        }
        Ok(match branches.len() {
            1 => branches.remove(0),
            _ => Node::Alt(branches),
        })
    }

    fn sequence(&mut self) -> Result<Node, String> {
        let mut nodes = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            let atom = self.atom()?;
            nodes.push(self.repeated(atom)?);
        }
        Ok(match nodes.len() {
            1 => nodes.remove(0),
            _ => Node::Seq(nodes),
        })
    }

    /// The count in braces that stands here, as {3}, {2,} or {2,5}; braces that hold no
    /// count are signs like any other.
    fn count(&mut self) -> Option<(usize, Option<usize>)> {
        let from = self.at;
        let number = |p: &mut Parser| -> Option<usize> {
            let start = p.at;
            while p.peek().is_some_and(|c| c.is_ascii_digit()) {
                p.at += 1;
            }
            let digits: String = p.chars[start..p.at].iter().collect();
            digits.parse().ok().filter(|n| *n <= 100_000)
        };
        self.at += 1;
        let found = (|| {
            let least = number(self)?;
            if self.eat('}') {
                return Some((least, Some(least)));
            }
            if !self.eat(',') {
                return None;
            }
            if self.eat('}') {
                return Some((least, None));
            }
            let most = number(self)?;
            (self.eat('}') && most >= least).then_some((least, Some(most)))
        })();
        if found.is_none() {
            self.at = from;
        }
        found
    }

    fn repeated(&mut self, atom: Node) -> Result<Node, String> {
        let (least, most) = match self.peek() {
            Some('*') => (0, None),
            Some('+') => (1, None),
            Some('?') => (0, Some(1)),
            Some('{') => match self.count() {
                Some(range) => {
                    self.at -= 1;
                    range
                }
                None => return Ok(atom),
            },
            _ => return Ok(atom),
        };
        self.at += 1;
        let (lazy, keeps) = (self.eat('?'), self.eat('+'));
        let repeat = Node::Repeat {
            inner: Box::new(atom),
            least,
            most,
            lazy,
        };
        Ok(match keeps && !lazy {
            true => Node::Atomic(Box::new(repeat)),
            false => repeat,
        })
    }

    fn atom(&mut self) -> Result<Node, String> {
        Ok(match self.next() {
            Some('.') => Node::Any,
            Some('^') => Node::Start,
            Some('$') => Node::End,
            Some('[') => self.set()?,
            Some('(') => self.group()?,
            Some('\\') => self.escape()?,
            Some('{') => {
                self.at -= 1;
                if self.count().is_some() {
                    return Err("a pattern has a count in {} that repeats nothing".into());
                }
                self.at += 1;
                Node::Char('{')
            }
            Some(c @ ('*' | '+' | '?')) => {
                return Err(format!("a pattern has a `{c}` that repeats nothing"));
            }
            Some(c) => Node::Char(c),
            None => return Err("a pattern ends too soon".into()),
        })
    }

    fn group(&mut self) -> Result<Node, String> {
        let closed = |p: &mut Parser, node: Node| match p.eat(')') {
            true => Ok(node),
            false => Err("a pattern has a ( without its )".to_string()),
        };
        if !self.eat('?') {
            self.groups += 1;
            let index = self.groups;
            let inner = self.alternation()?;
            return closed(self, Node::Group(Box::new(inner), index));
        }
        match self.next() {
            Some(':') => {
                let inner = self.alternation()?;
                closed(self, inner)
            }
            Some('>') => {
                let inner = self.alternation()?;
                closed(self, Node::Atomic(Box::new(inner)))
            }
            Some(c @ ('=' | '!')) => {
                let inner = Box::new(self.alternation()?);
                closed(
                    self,
                    Node::Look {
                        inner,
                        ahead: true,
                        wanted: c == '=',
                    },
                )
            }
            Some('<') if matches!(self.peek(), Some('=' | '!')) => {
                let wanted = self.next() == Some('=');
                let inner = Box::new(self.alternation()?);
                closed(
                    self,
                    Node::Look {
                        inner,
                        ahead: false,
                        wanted,
                    },
                )
            }
            Some('<') => {
                let name = self.name()?;
                self.groups += 1;
                let index = self.groups;
                self.names.push((name, index));
                let inner = self.alternation()?;
                closed(self, Node::Group(Box::new(inner), index))
            }
            Some('#') => {
                while self.peek().is_some_and(|c| c != ')') {
                    self.at += 1;
                }
                closed(self, Node::Seq(Vec::new()))
            }
            Some(c @ ('i' | '-')) => {
                self.fold = c == 'i';
                if c == '-' && !self.eat('i') {
                    return Err("a pattern has (?- that is not (?-i)".into());
                }
                closed(self, Node::Seq(Vec::new()))
            }
            _ => Err(
                "a pattern has a (? that is none of (?: (?= (?! (?<= (?<! (?> (?<name> (?i) (?#"
                    .into(),
            ),
        }
    }

    /// The name before a `>`.
    fn name(&mut self) -> Result<String, String> {
        let from = self.at;
        while self.peek().is_some_and(|c| c.is_alphanumeric() || c == '_') {
            self.at += 1;
        }
        let name: String = self.chars[from..self.at].iter().collect();
        match !name.is_empty() && self.eat('>') {
            true => Ok(name),
            false => Err("a pattern has a name of a token that is not as in (?<name>...)".into()),
        }
    }

    /// The sign that a number of the base after `\x` or `\o` means, in braces or not.
    fn coded(&mut self, base: u32, most: usize) -> Result<char, String> {
        let braces = self.eat('{');
        let from = self.at;
        while self.peek().is_some_and(|c| c.is_digit(base)) && (braces || self.at - from < most) {
            self.at += 1;
        }
        let digits: String = self.chars[from..self.at].iter().collect();
        let sign = u32::from_str_radix(&digits, base)
            .ok()
            .and_then(char::from_u32);
        match (sign, !braces || self.eat('}')) {
            (Some(c), true) => Ok(c),
            _ => Err("a pattern has a \\x or \\o without the number of a sign".into()),
        }
    }

    /// The sign that a letter after a backslash means when it means one sign.
    fn sign(&mut self, c: char) -> Result<char, String> {
        Ok(match c {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            'f' => '\x0c',
            'v' => '\x0b',
            'a' => '\x07',
            'b' => '\x08',
            'e' => '\x1b',
            'x' => self.coded(16, 2)?,
            'o' => self.coded(8, 3)?,
            other => other,
        })
    }

    fn class(c: char) -> Option<(Class, bool)> {
        Some(match c {
            'd' => (Class::Digit, false),
            'D' => (Class::Digit, true),
            'w' => (Class::Word, false),
            'W' => (Class::Word, true),
            's' => (Class::Space, false),
            'S' => (Class::Space, true),
            _ => return None,
        })
    }

    fn escape(&mut self) -> Result<Node, String> {
        let c = self.next().ok_or("a pattern ends with a backslash")?;
        if let Some((class, negated)) = Self::class(c) {
            return Ok(Node::Set(vec![Item::Class(class, negated)], false));
        }
        Ok(match c {
            '<' => Node::WordStart,
            '>' => Node::WordEnd,
            '1'..='9' => {
                let mut index = c as usize - '0' as usize;
                while let Some(d) = self.peek().and_then(|d| d.to_digit(10)) {
                    if index * 10 + d as usize > self.groups {
                        break;
                    }
                    index = index * 10 + d as usize;
                    self.at += 1;
                }
                if index > self.groups {
                    return Err(format!(
                        "a pattern names the token {index} before it has one"
                    ));
                }
                Node::Back(index)
            }
            'k' => {
                if !self.eat('<') {
                    return Err("a pattern has a \\k that is not as in \\k<name>".into());
                }
                let name = self.name()?;
                let found = self.names.iter().find(|(n, _)| *n == name);
                Node::Back(
                    found
                        .ok_or_else(|| format!("a pattern has no token {name}"))?
                        .1,
                )
            }
            other => Node::Char(self.sign(other)?),
        })
    }

    fn set(&mut self) -> Result<Node, String> {
        let open = || "a pattern has a [ without its ]".to_string();
        let negated = self.eat('^');
        let mut items = Vec::new();
        let mut first = true;
        loop {
            let c = self.next().ok_or_else(open)?;
            if c == ']' && !first {
                break;
            }
            first = false;
            let lo = match c {
                '\\' => {
                    let e = self.next().ok_or_else(open)?;
                    if let Some((class, negated)) = Self::class(e) {
                        items.push(Item::Class(class, negated));
                        continue;
                    }
                    self.sign(e)?
                }
                other => other,
            };
            let ranged =
                self.peek() == Some('-') && self.chars.get(self.at + 1).is_some_and(|c| *c != ']');
            let hi = match ranged {
                true => {
                    self.at += 1;
                    match self.next().ok_or_else(open)? {
                        '\\' => {
                            let e = self.next().ok_or_else(open)?;
                            self.sign(e)?
                        }
                        other => other,
                    }
                }
                false => lo,
            };
            if hi < lo {
                return Err(format!(
                    "a pattern has the range {lo}-{hi} that runs backwards"
                ));
            }
            items.push(Item::Range(lo, hi));
        }
        Ok(Node::Set(items, negated))
    }
}

pub fn compile(pattern: &str, fold: bool) -> Result<Pattern, String> {
    let mut parser = Parser {
        chars: pattern.chars().collect(),
        at: 0,
        groups: 0,
        names: Vec::new(),
        fold,
    };
    let node = parser.alternation()?;
    if parser.at < parser.chars.len() {
        return Err("a pattern has a ) without its (".into());
    }
    Ok(Pattern {
        node,
        groups: parser.groups,
        names: parser.names,
        fold: parser.fold,
    })
}

/// How a search goes: `.` may leave the ends of lines alone, `^` and `$` may stand at them.
#[derive(Clone, Copy, Default)]
struct Ways {
    lines: bool,
    dot_stays: bool,
}

struct Run<'a> {
    hay: &'a [char],
    fold: bool,
    ways: Ways,
    steps: Cell<u64>,
    depth: Cell<usize>,
    over: Cell<bool>,
}

fn folded(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

impl Run<'_> {
    fn same(&self, a: char, b: char) -> bool {
        a == b || (self.fold && folded(a) == folded(b))
    }

    fn within(&self, items: &[Item], c: char) -> bool {
        let one = |c: char| {
            items.iter().any(|item| match item {
                Item::Range(lo, hi) => (*lo..=*hi).contains(&c),
                Item::Class(class, negated) => in_class(*class, c) != *negated,
            })
        };
        one(c) || (self.fold && (one(folded(c)) || c.to_uppercase().any(one)))
    }

    /// Whether one sign at a place is what a node of one sign takes.
    fn takes(&self, node: &Node, at: usize) -> Option<bool> {
        let c = self.hay.get(at).copied();
        Some(match node {
            Node::Char(want) => c.is_some_and(|c| self.same(*want, c)),
            Node::Any => c.is_some_and(|c| !(self.ways.dot_stays && c == '\n')),
            Node::Set(items, negated) => c.is_some_and(|c| self.within(items, c) != *negated),
            _ => return None,
        })
    }

    fn word_at(&self, at: usize) -> bool {
        self.hay.get(at).is_some_and(|c| in_class(Class::Word, *c))
    }

    fn node(&self, node: &Node, at: usize, caps: &mut Caps, then: Then) -> bool {
        self.steps.set(self.steps.get() + 1);
        if self.over.get() || self.steps.get() > STEPS || self.depth.get() > DEPTH {
            self.over.set(true);
            return false;
        }
        if let Some(fits) = self.takes(node, at) {
            return fits && then(at + 1, caps);
        }
        self.depth.set(self.depth.get() + 1);
        let found = self.nested(node, at, caps, then);
        self.depth.set(self.depth.get() - 1);
        found
    }

    fn nested(&self, node: &Node, at: usize, caps: &mut Caps, then: Then) -> bool {
        let end = self.hay.len();
        match node {
            Node::Start => {
                (at == 0 || (self.ways.lines && self.hay[at - 1] == '\n')) && then(at, caps)
            }
            Node::End => (at == end || (self.ways.lines && self.hay[at] == '\n')) && then(at, caps),
            Node::WordStart => {
                self.word_at(at) && !(at > 0 && self.word_at(at - 1)) && then(at, caps)
            }
            Node::WordEnd => !self.word_at(at) && at > 0 && self.word_at(at - 1) && then(at, caps),
            Node::Seq(nodes) => self.sequence(nodes, at, caps, then),
            Node::Alt(branches) => branches.iter().any(|b| self.node(b, at, caps, then)),
            Node::Group(inner, index) => self.node(inner, at, caps, &|stop, caps| {
                let old = caps[*index];
                caps[*index] = Some((at, stop));
                then(stop, caps) || {
                    caps[*index] = old;
                    false
                }
            }),
            Node::Repeat {
                inner,
                least,
                most,
                lazy,
            } => match self.takes(inner, end) {
                Some(_) => self.run_of_signs(inner, (*least, *most, *lazy), at, caps, then),
                None => self.repeat(inner, (*least, *most, *lazy), 0, at, caps, then),
            },
            Node::Look {
                inner,
                ahead,
                wanted,
            } => {
                let mut tried = caps.clone();
                let found = match ahead {
                    true => self.node(inner, at, &mut tried, &|_, _| true),
                    false => (0..=at).rev().any(|from| {
                        let mut seen = caps.clone();
                        let fits = self.node(inner, from, &mut seen, &|stop, _| stop == at);
                        if fits {
                            tried = seen;
                        }
                        fits
                    }),
                };
                if found != *wanted {
                    return false;
                }
                let saved = match found {
                    true => std::mem::replace(caps, tried),
                    false => caps.clone(),
                };
                then(at, caps) || {
                    *caps = saved;
                    false
                }
            }
            Node::Atomic(inner) => {
                let mut tried = caps.clone();
                let stop = Cell::new(at);
                let fits = self.node(inner, at, &mut tried, &|pos, _| {
                    stop.set(pos);
                    true
                });
                if !fits {
                    return false;
                }
                let saved = std::mem::replace(caps, tried);
                then(stop.get(), caps) || {
                    *caps = saved;
                    false
                }
            }
            Node::Back(index) => match caps[*index] {
                Some((from, to)) => {
                    let n = to - from;
                    at + n <= end
                        && (0..n).all(|k| self.same(self.hay[from + k], self.hay[at + k]))
                        && then(at + n, caps)
                }
                None => false,
            },
            Node::Char(_) | Node::Any | Node::Set(..) => false,
        }
    }

    fn sequence(&self, nodes: &[Node], at: usize, caps: &mut Caps, then: Then) -> bool {
        match nodes {
            [] => then(at, caps),
            [first, rest @ ..] => self.node(first, at, caps, &|pos, caps| {
                self.sequence(rest, pos, caps, then)
            }),
        }
    }

    /// A node of one sign again and again: the places are walked, nothing nests.
    fn run_of_signs(
        &self,
        inner: &Node,
        (least, most, lazy): (usize, Option<usize>, bool),
        at: usize,
        caps: &mut Caps,
        then: Then,
    ) -> bool {
        let mut longest = 0;
        while most.is_none_or(|m| longest < m) && self.takes(inner, at + longest) == Some(true) {
            longest += 1;
        }
        if longest < least {
            return false;
        }
        match lazy {
            true => (least..=longest).any(|count| then(at + count, caps)),
            false => (least..=longest).rev().any(|count| then(at + count, caps)),
        }
    }

    fn repeat(
        &self,
        inner: &Node,
        (least, most, lazy): (usize, Option<usize>, bool),
        count: usize,
        at: usize,
        caps: &mut Caps,
        then: Then,
    ) -> bool {
        let again = |caps: &mut Caps| {
            most.is_none_or(|m| count < m)
                && self.node(inner, at, caps, &|pos, caps| {
                    // (a turn that takes nothing would go round for ever)
                    (pos > at || count < least)
                        && self.repeat(inner, (least, most, lazy), count + 1, pos, caps, then)
                })
        };
        match lazy {
            true => (count >= least && then(at, caps)) || again(caps),
            false => again(caps) || (count >= least && then(at, caps)),
        }
    }
}

impl Pattern {
    /// The first match that begins at or after a place: the ends of the whole and of every
    /// token.
    fn find(&self, hay: &[char], from: usize, ways: Ways) -> Result<Option<Caps>, String> {
        let run = Run {
            hay,
            fold: self.fold,
            ways,
            steps: Cell::new(0),
            depth: Cell::new(0),
            over: Cell::new(false),
        };
        for start in from..=hay.len() {
            let mut caps: Caps = vec![None; self.groups + 1];
            let stop = Cell::new(start);
            let fits = run.node(&self.node, start, &mut caps, &|pos, _| {
                stop.set(pos);
                true
            });
            if run.over.get() {
                return Err("the pattern takes too long on this text".into());
            }
            if fits {
                caps[0] = Some((start, stop.get()));
                return Ok(Some(caps));
            }
        }
        Ok(None)
    }

    /// All matches, one after the other (or the first only).
    fn find_all(
        &self,
        hay: &[char],
        ways: Ways,
        empty: bool,
        once: bool,
    ) -> Result<Vec<Caps>, String> {
        std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(ROOM)
                .spawn_scoped(scope, || self.all(hay, ways, empty, once))
                .map_err(|_| "there is no room for the search of a pattern".to_string())?
                .join()
                .map_err(|_| "the search of a pattern broke off".to_string())?
        })
    }

    fn all(&self, hay: &[char], ways: Ways, empty: bool, once: bool) -> Result<Vec<Caps>, String> {
        let mut out = Vec::new();
        let mut from = 0;
        while let Some(caps) = self.find(hay, from, ways)? {
            let (start, stop) = caps[0].unwrap_or((from, from));
            from = stop + usize::from(stop == start);
            // (a match that takes nothing counts only when it is asked for)
            if stop > start || empty {
                out.push(caps);
            }
            if (once && !out.is_empty()) || from > hay.len() {
                break;
            }
        }
        Ok(out)
    }

    /// What replaces a match: `$0` is the match, `$1` the first token, `$<name>` a token
    /// by its name.
    fn replacement(&self, with: &[char], caps: &Caps, hay: &[char]) -> String {
        let piece = |k: usize| -> String {
            match caps.get(k).copied().flatten() {
                Some((from, to)) => hay[from..to].iter().collect(),
                None => String::new(),
            }
        };
        let mut out = String::new();
        let mut at = 0;
        while at < with.len() {
            let c = with[at];
            at += 1;
            match c {
                '$' if with.get(at).is_some_and(char::is_ascii_digit) => {
                    let mut index = 0;
                    while let Some(d) = with.get(at).and_then(|d| d.to_digit(10)) {
                        if index > 0 && index * 10 + d as usize > self.groups {
                            break;
                        }
                        index = index * 10 + d as usize;
                        at += 1;
                    }
                    out.push_str(&piece(index));
                }
                '$' if with.get(at) == Some(&'<') => {
                    let stop = with[at..].iter().position(|c| *c == '>');
                    let name: Option<String> = stop.map(|n| with[at + 1..at + n].iter().collect());
                    let found = name
                        .as_ref()
                        .and_then(|name| self.names.iter().find(|(n, _)| n == name));
                    match (found, stop) {
                        (Some((_, index)), Some(n)) => {
                            out.push_str(&piece(*index));
                            at += n + 1;
                        }
                        _ => out.push('$'),
                    }
                }
                '\\' if at < with.len() => {
                    out.push(match with[at] {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        other => other,
                    });
                    at += 1;
                }
                other => out.push(other),
            }
        }
        out
    }
}

/// What a search is asked to give, in the order asked, and how it goes.
#[derive(Default)]
struct Asked {
    outputs: Vec<String>,
    once: bool,
    fold: bool,
    empty: bool,
    ways: Ways,
    nth: Option<usize>,
}

fn asked(options: &[Expr], name: &str, fold: bool) -> Result<Asked, String> {
    let mut out = Asked {
        fold,
        ..Asked::default()
    };
    for option in options {
        let Some(word) = text_of(option) else {
            if name == "regexprep" {
                out.nth = Some(whole(option, "the match to replace", 1, 1_000_000_000)? as usize);
                continue;
            }
            return Err(format!(
                "{name}: an option is a word in quotes, {option} is none"
            ));
        };
        match word.to_lowercase().as_str() {
            "once" => out.once = true,
            "all" => out.once = false,
            "ignorecase" => out.fold = true,
            "matchcase" => out.fold = false,
            "emptymatch" => out.empty = true,
            "noemptymatch" => out.empty = false,
            "lineanchors" => out.ways.lines = true,
            "stringanchors" => out.ways.lines = false,
            "dotexceptnewline" => out.ways.dot_stays = true,
            "dotall" => out.ways.dot_stays = false,
            kind @ ("match" | "tokens" | "names" | "start" | "end" | "split" | "tokenextents")
                if name != "regexprep" =>
            {
                out.outputs.push(kind.to_string());
            }
            _ if name == "regexprep" => {
                return Err(format!(
                    "regexprep: \"{word}\" is no option; they are \"once\", \"ignorecase\", \
                     \"emptymatch\", \"lineanchors\", \"dotexceptnewline\" and the number of \
                     the match to replace"
                ));
            }
            _ => {
                return Err(format!(
                    "{name}: \"{word}\" is no option; they are \"match\", \"tokens\", \"names\", \
                     \"start\", \"end\", \"split\", \"tokenextents\", \"once\", \"ignorecase\", \
                     \"emptymatch\", \"lineanchors\" and \"dotexceptnewline\""
                ));
            }
        }
    }
    Ok(out)
}

fn piece(hay: &[char], ends: Option<(usize, usize)>) -> Expr {
    match ends {
        Some((from, to)) => text(&hay[from..to].iter().collect::<String>()),
        None => text(""),
    }
}

/// What a search of one text gives.
fn searched(hay: &str, pattern: &Pattern, asked: &Asked) -> Res {
    let hay: Vec<char> = hay.chars().collect();
    let found = pattern.find_all(&hay, asked.ways, asked.empty, asked.once)?;
    let tokens = |caps: &Caps| -> Expr {
        match pattern.groups {
            0 => Expr::list(vec![piece(&hay, caps[0])]),
            n => Expr::list((1..=n).map(|k| piece(&hay, caps[k])).collect()),
        }
    };
    let extents = |caps: &Caps| -> Expr {
        let ends = |k: usize| match caps[k] {
            Some((from, to)) => Expr::list(vec![Expr::int(from as i64 + 1), Expr::int(to as i64)]),
            None => Expr::list(vec![Expr::int(0), Expr::int(0)]),
        };
        match pattern.groups {
            0 => Expr::list(vec![ends(0)]),
            n => Expr::list((1..=n).map(ends).collect()),
        }
    };
    let names = |caps: &Caps| -> Expr {
        Expr::list(
            pattern
                .names
                .iter()
                .map(|(name, k)| Expr::list(vec![text(name), piece(&hay, caps[*k])]))
                .collect(),
        )
    };
    let one = |kind: &str| -> Expr {
        let each = |f: &dyn Fn(&Caps) -> Expr| -> Expr {
            match (asked.once, found.first()) {
                (true, Some(caps)) => f(caps),
                (true, None) => match kind {
                    "match" => text(""),
                    _ => Expr::list(Vec::new()),
                },
                (false, _) => Expr::list(found.iter().map(f).collect()),
            }
        };
        let whole = |caps: &Caps| caps[0].unwrap_or((0, 0));
        match kind {
            "match" => each(&|caps| piece(&hay, caps[0])),
            "tokens" => each(&tokens),
            "tokenextents" => each(&extents),
            "names" => each(&names),
            "end" => each(&|caps| Expr::int(whole(caps).1 as i64)),
            "split" => {
                let mut parts = Vec::new();
                let mut from = 0;
                for caps in &found {
                    let (start, stop) = whole(caps);
                    parts.push(piece(&hay, Some((from, start))));
                    from = stop;
                }
                parts.push(piece(&hay, Some((from, hay.len()))));
                Expr::list(parts)
            }
            _ => each(&|caps| Expr::int(whole(caps).0 as i64 + 1)),
        }
    };
    Ok(match asked.outputs.as_slice() {
        [] => one("start"),
        [kind] => one(kind),
        kinds => Expr::list(kinds.iter().map(|kind| one(kind)).collect()),
    })
}

/// A text with the matches of a pattern replaced.
fn replaced(hay: &str, pattern: &Pattern, with: &str, asked: &Asked) -> Result<String, String> {
    let hay: Vec<char> = hay.chars().collect();
    let with: Vec<char> = with.chars().collect();
    let found = pattern.find_all(&hay, asked.ways, asked.empty, asked.once)?;
    let mut out = String::new();
    let mut from = 0;
    for (k, caps) in found.iter().enumerate() {
        if asked.nth.is_some_and(|nth| nth != k + 1) {
            continue;
        }
        let (start, stop) = caps[0].unwrap_or((from, from));
        out.extend(&hay[from..start]);
        out.push_str(&pattern.replacement(&with, caps, &hay));
        from = stop;
    }
    out.extend(&hay[from..]);
    Ok(out)
}

/// A text as a pattern that matches just that text, or one with `*` and `?` for any signs.
fn translated(kind: &str, s: &str) -> Result<String, String> {
    let mut out = String::new();
    for c in s.chars() {
        match (kind, c) {
            ("wildcard", '*') => out.push_str(".*"),
            ("wildcard", '?') => out.push('.'),
            ("escape" | "wildcard", c) => {
                if "\\^$.|?*+()[]{}".contains(c) {
                    out.push('\\');
                }
                out.push(c);
            }
            _ => {
                return Err(
                    "regexptranslate(kind, text): the kind is \"escape\" or \"wildcard\"".into(),
                );
            }
        }
    }
    Ok(out)
}

/// A function of a text, for a text or for every text of a list.
fn each(e: &Expr, name: &str, f: &dyn Fn(&str) -> Res) -> Res {
    match e.as_list() {
        Some(items) => Ok(Expr::list(
            items
                .iter()
                .map(|x| each(x, name, f))
                .collect::<Result<_, _>>()?,
        )),
        None => f(text_of(e).ok_or_else(|| format!("{name} needs a text in quotes, {e} is none"))?),
    }
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let s = |k: usize| {
        text_of(&a[k]).ok_or_else(|| format!("{name} needs a text in quotes, {} is none", a[k]))
    };
    Ok(Some(match (name, a.len()) {
        ("regexp" | "regexpi", 2..) => {
            let asked = asked(&a[2..], name, name == "regexpi")?;
            let pattern = compile(s(1)?, asked.fold)?;
            each(&a[0], name, &|hay| searched(hay, &pattern, &asked))?
        }
        ("regexprep", 3..) => {
            let asked = asked(&a[3..], name, false)?;
            let texts = |e: &Expr| -> Result<Vec<String>, String> {
                let one = |x: &Expr| {
                    text_of(x)
                        .map(str::to_string)
                        .ok_or_else(|| format!("regexprep needs texts in quotes, {x} is none"))
                };
                match e.as_list() {
                    Some(items) => items.iter().map(one).collect(),
                    None => Ok(vec![one(e)?]),
                }
            };
            let (patterns, with) = (texts(&a[1])?, texts(&a[2])?);
            if with.len() != 1 && with.len() != patterns.len() {
                return Err("regexprep needs one replacement, or one for every pattern".into());
            }
            let patterns: Vec<Pattern> = patterns
                .iter()
                .map(|p| compile(p, asked.fold))
                .collect::<Result<_, _>>()?;
            each(&a[0], name, &|hay| {
                let mut out = hay.to_string();
                for (k, pattern) in patterns.iter().enumerate() {
                    out = replaced(&out, pattern, &with[k.min(with.len() - 1)], &asked)?;
                }
                Ok(text(&out))
            })?
        }
        ("regexptranslate", 2) => {
            let kind = s(0)?.to_lowercase();
            each(&a[1], name, &|t| Ok(text(&translated(&kind, t)?)))?
        }
        ("strjust", 1 | 2) => {
            let side = match a.get(1) {
                Some(_) => s(1)?.to_lowercase(),
                None => "right".to_string(),
            };
            if !matches!(side.as_str(), "left" | "right" | "center") {
                return Err(
                    "strjust(text, side): the side is \"left\", \"right\" or \"center\"".into(),
                );
            }
            each(&a[0], name, &|t| {
                let width = t.chars().count();
                let body = t.trim_matches(' ');
                let room = width - body.chars().count();
                let before = match side.as_str() {
                    "left" => 0,
                    "right" => room,
                    _ => room / 2,
                };
                Ok(text(&format!(
                    "{}{body}{}",
                    " ".repeat(before),
                    " ".repeat(room - before)
                )))
            })?
        }
        ("validatestring", 2) => {
            let word = s(0)?;
            let options = a[1]
                .as_list()
                .ok_or("validatestring(text, [words]) needs the list of words")?;
            let words: Vec<&str> = options
                .iter()
                .map(|x| text_of(x).ok_or("validatestring(text, [words]) needs words in quotes"))
                .collect::<Result<_, _>>()?;
            let lower = word.to_lowercase();
            let begun = |w: &str| !lower.is_empty() && w.to_lowercase().starts_with(&lower);
            let fits: Vec<&str> = words.iter().copied().filter(|w| begun(w)).collect();
            let exact = words.iter().copied().find(|w| w.to_lowercase() == lower);
            match (exact, fits.as_slice()) {
                (Some(w), _) | (None, &[w]) => text(w),
                (None, []) => {
                    return Err(format!("validatestring: \"{word}\" is none of {}", a[1]));
                }
                // (several words that begin alike: the shortest, when the others begin with it)
                (None, [first, rest @ ..]) => {
                    let shortest = rest.iter().fold(*first, |s, w| match w.len() < s.len() {
                        true => w,
                        false => s,
                    });
                    let within = shortest.to_lowercase();
                    if !fits.iter().all(|w| w.to_lowercase().starts_with(&within)) {
                        return Err(format!(
                            "validatestring: \"{word}\" begins more than one of {}",
                            a[1]
                        ));
                    }
                    text(shortest)
                }
            }
        }
        _ => return Ok(None),
    }))
}

pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    inner(name, a).transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(pattern: &str, hay: &str) -> Vec<String> {
        let hay: Vec<char> = hay.chars().collect();
        let compiled = compile(pattern, false).expect("a pattern");
        compiled
            .find_all(&hay, Ways::default(), false, false)
            .expect("a search")
            .iter()
            .map(|caps| {
                let (from, to) = caps[0].expect("a match");
                hay[from..to].iter().collect()
            })
            .collect()
    }

    #[test]
    fn patterns_match_as_written() {
        for (pattern, hay, wanted) in [
            (r"\d+", "a12b345c", vec!["12", "345"]),
            (r"[a-c]+", "abcdabc", vec!["abc", "abc"]),
            (r"[^a-c]+", "abcdefabc", vec!["def"]),
            (r"a.c", "abc a\nc axc", vec!["abc", "a\nc", "axc"]),
            (r"colou?r", "color colour", vec!["color", "colour"]),
            (r"a{2,3}", "a aa aaa aaaa", vec!["aa", "aaa", "aaa"]),
            (r"a{2}", "aaaaa", vec!["aa", "aa"]),
            (r"a{2,}", "a aa aaaaa", vec!["aa", "aaaaa"]),
            (r"<.+>", "<a><b>", vec!["<a><b>"]),
            (r"<.+?>", "<a><b>", vec!["<a>", "<b>"]),
            (r"(ab)+", "ababab ab", vec!["ababab", "ab"]),
            (r"cat|dog|bird", "a dog, a cat", vec!["dog", "cat"]),
            (r"^\w+", "hello world", vec!["hello"]),
            (r"\w+$", "hello world", vec!["world"]),
            (r"\<w\w*", "a word went away", vec!["word", "went"]),
            (
                r"\w*d\>",
                "a word and a deed done",
                vec!["word", "and", "deed"],
            ),
            (r"(\w)\1", "aabbcd", vec!["aa", "bb"]),
            (r"(?<x>\w)\k<x>", "aabbcd", vec!["aa", "bb"]),
            (r"\w+(?=,)", "one, two, three", vec!["one", "two"]),
            (r"\w+(?!,)\>", "one, two, three", vec!["three"]),
            (r"(?<=\$)\d+", "cost $15 or 20", vec!["15"]),
            (r"(?<!\$)\<\d+", "cost $15 or 20", vec!["20"]),
            (r"a++a", "aaa", vec![]),
            (r"(?>a+)b", "aaab", vec!["aaab"]),
            (r"(?i)hello", "Hello HELLO", vec!["Hello", "HELLO"]),
            (r"a{,3}", "a{,3}", vec!["a{,3}"]),
            (r"a*?", "aaa", vec![]),
            (r"b*", "abbc", vec!["bb"]),
            (r"[\d.]+", "pi is 3.14", vec!["3.14"]),
            (r"[]a]+", "a]b", vec!["a]"]),
            (r"\x41\o102", "ABC", vec!["AB"]),
            (r"(a|ab)(c|bcd)(d*)", "abcd", vec!["abcd"]),
            (r"(?:a|b)*c", "ababc", vec!["ababc"]),
            (r"x*", "abc", vec![]),
            (r"\s+", "a b\t\nc", vec![" ", "\t\n"]),
            (r"\S+", "a bc", vec!["a", "bc"]),
            (r"(?#the word)\w+", "hi", vec!["hi"]),
            (r"ż+", "żżółw", vec!["żż"]),
        ] {
            assert_eq!(matches(pattern, hay), wanted, "{pattern} on {hay}");
        }
    }

    #[test]
    fn patterns_that_are_none_say_why() {
        for pattern in [
            "(a", "a)", "[a", "*a", "a**", "(?x)", r"\2(a)", "a{2}{3}", r"\k<n>", "[b-a]", "\\",
        ] {
            assert!(compile(pattern, false).is_err(), "{pattern}");
        }
    }

    #[test]
    fn long_texts_and_hard_patterns_end() {
        let hay: Vec<char> = "ab".repeat(200_000).chars().collect();
        let all = compile(".*", false).expect("a pattern");
        let found = all
            .find_all(&hay, Ways::default(), false, true)
            .expect("a search");
        assert_eq!(found[0][0], Some((0, 400_000)));
        // a pattern that nests once for every sign says so instead of overflowing
        let nests = compile("(ab)*", false).expect("a pattern");
        assert!(nests.find_all(&hay, Ways::default(), false, true).is_err());
        let short: Vec<char> = "ab".repeat(20_000).chars().collect();
        let found = nests
            .find_all(&short, Ways::default(), false, true)
            .expect("a search");
        assert_eq!(found[0][0], Some((0, 40_000)));
        // a pattern that tries every split gives up after its steps
        let hard = compile("(a*)*b", false).expect("a pattern");
        let many: Vec<char> = "a".repeat(40).chars().collect();
        assert!(hard.find_all(&many, Ways::default(), false, true).is_err());
    }

    #[test]
    fn replacements_take_tokens() {
        let pattern = compile(r"(?<first>\w+) (\w+)", false).expect("a pattern");
        let asked = Asked::default();
        assert_eq!(
            replaced("hello world", &pattern, "$2 $1", &asked).as_deref(),
            Ok("world hello")
        );
        assert_eq!(
            replaced("hello world", &pattern, r"[$0] $<first>\n\$", &asked).as_deref(),
            Ok("[hello world] hello\n$")
        );
        assert_eq!(translated("escape", "a.b*c").as_deref(), Ok(r"a\.b\*c"));
        assert_eq!(translated("wildcard", "*.txt").as_deref(), Ok(r".*\.txt"));
    }
}
