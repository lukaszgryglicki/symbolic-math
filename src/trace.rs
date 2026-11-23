//! The steps of a calculation, told while it runs: what `verbose` shows.
//!
//! A step is a line `step: ...`; what belongs to a step is indented under it. Level 1 tells the
//! methods and what they gave, level 2 everything that is known on the way. Nothing here changes
//! a result: with the level at 0 a step costs one comparison.

use std::cell::{Cell, RefCell};

/// The most lines of one statement, and the most characters of one line.
const LINES: usize = 4000;
const WIDTH: usize = 600;

type Sink = Box<dyn FnMut(&str)>;

thread_local! {
    static LEVEL: Cell<u8> = const { Cell::new(0) };
    static DEPTH: Cell<u32> = const { Cell::new(0) };
    static MUTED: Cell<u32> = const { Cell::new(0) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
    static HELD: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static SINK: RefCell<Option<Sink>> = const { RefCell::new(None) };
    /// The lines of the attempts that are under way, the innermost last.
    static TRIALS: RefCell<Vec<Vec<String>>> = const { RefCell::new(Vec::new()) };
}

pub fn level() -> u8 {
    LEVEL.get()
}

pub fn set_level(level: u8) {
    LEVEL.set(level.min(2));
}

/// Steps are told.
pub fn on() -> bool {
    LEVEL.get() > 0 && MUTED.get() == 0
}

/// Every detail is told.
pub fn deep() -> bool {
    LEVEL.get() > 1 && MUTED.get() == 0
}

/// Lines go to `sink` as they are told (the command line), not into the list of `take`.
pub fn set_sink(sink: Option<Sink>) {
    SINK.with(|s| *s.borrow_mut() = sink);
}

pub fn live() -> bool {
    SINK.with(|s| s.borrow().is_some())
}

/// A line of the output for the sink; false when there is none.
pub fn print(line: &str) -> bool {
    SINK.with(|s| match s.borrow_mut().as_mut() {
        Some(sink) => {
            sink(line);
            true
        }
        None => false,
    })
}

/// A line that stands: to the sink, or to the list of `take`.
fn out(line: String) {
    let count = COUNT.get();
    COUNT.set(count + 1);
    if count > LINES {
        return;
    }
    let line = if count == LINES {
        "step: (more steps are not shown)".to_string()
    } else {
        line
    };
    if !print(&line) {
        HELD.with(|h| h.borrow_mut().push(line));
    }
}

/// The names that methods use inside them, as a reader would write them: `#t` is `t`, `#L`
/// (the logarithm of `t` in a series) is `ln(t)`, `#u1` is `u1`. A `#` after a digit or a
/// letter stands in a number of another base (`2#101`) and stays.
fn plain(text: String) -> String {
    if !text.contains('#') {
        return text;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while at < chars.len() {
        let inner = chars[at] == '#'
            && (at == 0 || !chars[at - 1].is_alphanumeric())
            && chars.get(at + 1).is_some_and(|c| c.is_alphabetic());
        if !inner {
            out.push(chars[at]);
            at += 1;
            continue;
        }
        let end = (at + 1..chars.len())
            .find(|k| !chars[*k].is_alphanumeric())
            .unwrap_or(chars.len());
        let name: String = chars[at + 1..end].iter().collect();
        out.push_str(if name == "L" { "ln(t)" } else { &name });
        at = end;
    }
    out
}

fn tell(text: String) {
    let text = plain(text);
    let indent = "  ".repeat(DEPTH.get().min(12) as usize);
    let line = if text.chars().count() > WIDTH {
        let short: String = text.chars().take(WIDTH).collect();
        format!("step: {indent}{short} ...")
    } else {
        format!("step: {indent}{text}")
    };
    let waits = TRIALS.with(|t| match t.borrow_mut().last_mut() {
        Some(lines) => {
            if lines.len() <= LINES {
                lines.push(line.clone());
            }
            true
        }
        None => false,
    });
    if !waits {
        out(line);
    }
}

/// The lines of an attempt that ended: they join the attempt around it, or they stand.
fn settle(lines: Vec<String>) {
    let lines = TRIALS.with(|t| match t.borrow_mut().last_mut() {
        Some(outer) => {
            outer.extend(lines);
            Vec::new()
        }
        None => lines,
    });
    lines.into_iter().for_each(out);
}

/// Runs an attempt of a method: its steps are told only when it gives a result.
pub fn trial<T>(attempt: impl FnOnce() -> Option<T>) -> Option<T> {
    if LEVEL.get() == 0 {
        return attempt();
    }
    let depth = DEPTH.get();
    TRIALS.with(|t| t.borrow_mut().push(Vec::new()));
    let result = attempt();
    DEPTH.set(depth);
    let lines = TRIALS.with(|t| t.borrow_mut().pop()).unwrap_or_default();
    if result.is_some() {
        settle(lines);
    }
    result
}

/// Runs a method whose steps are told when it has ended, whatever it gives: inside it
/// `mark` and `forget` take back the steps of a way that led nowhere.
pub fn whole<T>(method: impl FnOnce() -> T) -> T {
    if LEVEL.get() == 0 {
        return method();
    }
    let depth = DEPTH.get();
    TRIALS.with(|t| t.borrow_mut().push(Vec::new()));
    let result = method();
    DEPTH.set(depth);
    settle(TRIALS.with(|t| t.borrow_mut().pop()).unwrap_or_default());
    result
}

/// Where the steps of the attempt under way stand now, for `forget`.
pub fn mark() -> (usize, u32) {
    (
        TRIALS.with(|t| t.borrow().last().map_or(0, Vec::len)),
        DEPTH.get(),
    )
}

/// Takes back the steps told since `mark`, inside `trial` or `whole`.
pub fn forget(mark: (usize, u32)) {
    TRIALS.with(|t| {
        if let Some(lines) = t.borrow_mut().last_mut() {
            lines.truncate(mark.0);
        }
    });
    DEPTH.set(mark.1);
}

/// A step of the method in use.
pub fn step(text: impl FnOnce() -> String) {
    if on() {
        tell(text());
    }
}

/// A detail: told at level 2 only.
pub fn detail(text: impl FnOnce() -> String) {
    if deep() {
        tell(text());
    }
}

/// What is told until the guard is dropped stands one level deeper.
pub struct Nest(bool);

impl Drop for Nest {
    fn drop(&mut self) {
        if self.0 {
            DEPTH.set(DEPTH.get().saturating_sub(1));
        }
    }
}

pub fn nest() -> Nest {
    let on = LEVEL.get() > 0;
    if on {
        DEPTH.set(DEPTH.get() + 1);
    }
    Nest(on)
}

/// Nothing is told until the guard is dropped: for the many small calculations inside a method.
pub struct Mute;

impl Drop for Mute {
    fn drop(&mut self) {
        MUTED.set(MUTED.get().saturating_sub(1));
    }
}

pub fn mute() -> Mute {
    MUTED.set(MUTED.get() + 1);
    Mute
}

/// The lines told since the last call, and a fresh count for the next statement.
pub fn take() -> Vec<String> {
    // (an error may have left attempts open: their steps are told as far as they went)
    while let Some(lines) = TRIALS.with(|t| t.borrow_mut().pop()) {
        settle(lines);
    }
    COUNT.set(0);
    DEPTH.set(0);
    MUTED.set(0);
    HELD.with(|h| std::mem::take(&mut *h.borrow_mut()))
}

/// `1 root`, `2 roots`, `no roots`: a count with what is counted.
pub fn count(n: usize, what: &str) -> String {
    match n {
        0 => format!("no {what}s"),
        1 => format!("1 {what}"),
        _ => format!("{n} {what}s"),
    }
}

/// A number in a few digits, as a step shows it.
pub fn number(z: &rug::Complex) -> String {
    crate::numeric::fmt_complex(z, 12)
}

/// A statement as it was read, before anything is worked out: products inside products and
/// sums inside sums are written as one, and the sign of a product goes into its number.
pub fn written(stmt: &crate::expr::Expr) -> String {
    use crate::expr::Expr;
    use std::rc::Rc;
    fn tidy(e: &Expr) -> Expr {
        match e {
            Expr::Mul(a) => {
                let mut factors: Vec<Expr> = Vec::new();
                for f in a.iter().map(tidy) {
                    match f {
                        Expr::Mul(inner) => factors.extend(inner.iter().cloned()),
                        other => factors.push(other),
                    }
                }
                // the numbers in front come together: -1 * 5 * x is -5 x
                while factors.len() > 1 && factors[0].is_number() && factors[1].is_number() {
                    let product = crate::canon::mul(vec![factors[0].clone(), factors[1].clone()]);
                    factors.splice(..2, [product]);
                }
                match factors.len() {
                    1 => factors.remove(0),
                    _ => Expr::Mul(factors.into()),
                }
            }
            Expr::Add(a) => {
                let mut terms: Vec<Expr> = Vec::new();
                for t in a.iter().map(tidy) {
                    match t {
                        Expr::Add(inner) => terms.extend(inner.iter().cloned()),
                        other => terms.push(other),
                    }
                }
                Expr::Add(terms.into())
            }
            Expr::Pow(b, x) => Expr::Pow(Rc::new(tidy(b)), Rc::new(tidy(x))),
            Expr::Fn(n, a) => Expr::Fn(n.clone(), a.iter().map(tidy).collect()),
            Expr::List(a) => Expr::List(a.iter().map(tidy).collect()),
            Expr::Rel(op, a, b) => Expr::Rel(*op, Rc::new(tidy(a)), Rc::new(tidy(b))),
            _ => e.clone(),
        }
    }
    tidy(stmt).to_string()
}
