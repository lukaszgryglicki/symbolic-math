//! Assumptions about symbols (real, positive, integer, ...) and what follows from them.

use crate::expr::Expr;
use crate::numeric;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Facts {
    pub real: bool,
    pub positive: bool,
    pub negative: bool,
    pub nonnegative: bool,
    pub nonzero: bool,
    pub integer: bool,
}

impl Facts {
    pub fn real() -> Facts {
        Facts {
            real: true,
            ..Facts::default()
        }
    }
    pub fn positive() -> Facts {
        Facts {
            real: true,
            positive: true,
            nonnegative: true,
            nonzero: true,
            ..Facts::default()
        }
    }
    pub fn negative() -> Facts {
        Facts {
            real: true,
            negative: true,
            nonzero: true,
            ..Facts::default()
        }
    }
    pub fn nonnegative() -> Facts {
        Facts {
            real: true,
            nonnegative: true,
            ..Facts::default()
        }
    }
    pub fn integer() -> Facts {
        Facts {
            real: true,
            integer: true,
            ..Facts::default()
        }
    }
    pub fn merge(self, o: Facts) -> Facts {
        Facts {
            real: self.real || o.real,
            positive: self.positive || o.positive,
            negative: self.negative || o.negative,
            nonnegative: self.nonnegative || o.nonnegative,
            nonzero: self.nonzero || o.nonzero,
            integer: self.integer || o.integer,
        }
    }
    pub fn describe(self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.integer {
            v.push("integer");
        } else if self.real && !self.positive && !self.negative && !self.nonnegative {
            v.push("real");
        }
        if self.positive {
            v.push("positive");
        } else if self.nonnegative {
            v.push("nonnegative");
        }
        if self.negative {
            v.push("negative");
        }
        if self.nonzero && !self.positive && !self.negative {
            v.push("nonzero");
        }
        v
    }
}

/// The numbers a session works with.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Domain {
    #[default]
    Complex,
    /// Symbols are real, odd roots of negative numbers are real, only real solutions count.
    Real,
    /// `j` and `k` are quaternion units next to `i`; symbols are real.
    Quaternion,
}

thread_local! {
    static FACTS: RefCell<HashMap<Rc<str>, Facts>> = RefCell::new(HashMap::new());
    static DOMAIN: Cell<Domain> = const { Cell::new(Domain::Complex) };
    static ALL_REAL: Cell<bool> = const { Cell::new(false) };
    static COMPLEX: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Runs `f` with the symbols `names` free of what the domain says about every symbol.
pub fn with_complex<T>(names: &[String], f: impl FnOnce() -> T) -> T {
    let before = COMPLEX.with(|c| c.borrow().len());
    COMPLEX.with(|c| c.borrow_mut().extend(names.iter().cloned()));
    let out = f();
    COMPLEX.with(|c| c.borrow_mut().truncate(before));
    out
}

/// What was assumed about the symbol itself, without the rule of the domain.
pub fn declared(name: &str) -> Facts {
    FACTS.with(|m| m.borrow().get(name).copied().unwrap_or_default())
}

/// Runs `f` with every symbol taken as real.
pub fn with_real_symbols<T>(f: impl FnOnce() -> T) -> T {
    let before = ALL_REAL.replace(true);
    let out = f();
    ALL_REAL.set(before);
    out
}

pub fn set_domain(d: Domain) {
    DOMAIN.set(d);
}
pub fn domain() -> Domain {
    DOMAIN.get()
}
pub fn real_mode() -> bool {
    domain() == Domain::Real
}
pub fn quaternion_mode() -> bool {
    domain() == Domain::Quaternion
}

pub fn set(name: &str, f: Facts) {
    FACTS.with(|m| {
        let mut m = m.borrow_mut();
        let old = m.get(name).copied().unwrap_or_default();
        m.insert(Rc::from(name), old.merge(f));
    });
}
pub fn clear(name: &str) {
    FACTS.with(|m| m.borrow_mut().remove(name));
}
pub fn clear_all() {
    FACTS.with(|m| m.borrow_mut().clear());
}
pub fn get(name: &str) -> Facts {
    let facts = declared(name);
    if (domain() != Domain::Complex || ALL_REAL.get())
        && !name.starts_with('#')
        && !COMPLEX.with(|c| c.borrow().iter().any(|n| n == name))
    {
        facts.merge(Facts::real())
    } else {
        facts
    }
}
pub fn all() -> Vec<(Rc<str>, Facts)> {
    let mut v: Vec<_> = FACTS.with(|m| m.borrow().iter().map(|(k, f)| (k.clone(), *f)).collect());
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}
pub fn is_integer_sym(name: &str) -> bool {
    get(name).integer
}
pub fn is_real_sym(name: &str) -> bool {
    get(name).real
}
pub fn is_positive_sym(name: &str) -> bool {
    get(name).positive
}

const REAL_ON_REALS: &[&str] = &[
    "sin", "cos", "tan", "sinh", "cosh", "tanh", "atan", "asinh", "erf", "floor", "ceil", "round",
    "trunc", "frac", "sign", "si", "shi", "min", "max", "atan2", "gamma", "erfc", "erfi", "zeta",
];
const ALWAYS_REAL: &[&str] = &["abs", "re", "im", "arg"];

pub fn is_real(e: &Expr) -> bool {
    match e {
        Expr::Num(n) => n.is_real(),
        Expr::Float(f) => f.v.imag().is_zero(),
        Expr::Sym(s) => {
            matches!(&**s, "inf")
                || (crate::expr::is_const_name(s) && !e.is_infinite())
                || get(s).real
        }
        Expr::Add(a) | Expr::Mul(a) => a.iter().all(is_real),
        Expr::Pow(b, x) => {
            (is_real(b) && is_integer(x))
                || (is_positive(b) && is_real(x))
                || (is_nonnegative(b) && is_positive(x))
                || (b.is_sym("e") && is_real(x))
                || (real_mode() && is_real(b) && x.as_rat().is_some_and(|r| r.denom().is_odd()))
        }
        Expr::Fn(n, a) => {
            ALWAYS_REAL.contains(&&**n)
                || (REAL_ON_REALS.contains(&&**n) && a.iter().all(is_real))
                || (&**n == "ln" && a.len() == 1 && is_positive(&a[0]))
        }
        _ => false,
    }
}

pub fn is_integer(e: &Expr) -> bool {
    match e {
        Expr::Num(n) => n.is_int(),
        Expr::Sym(s) => get(s).integer,
        Expr::Add(a) | Expr::Mul(a) => a.iter().all(is_integer),
        Expr::Pow(b, x) => is_integer(b) && x.as_int().is_some_and(|k| !k.is_negative()),
        Expr::Fn(n, a) => {
            matches!(&**n, "floor" | "ceil" | "round" | "trunc") && a.len() == 1 && is_real(&a[0])
                || (matches!(&**n, "factorial" | "binomial")
                    && a.iter().all(is_nonnegative_integer))
        }
        _ => false,
    }
}

fn is_nonnegative_integer(e: &Expr) -> bool {
    is_integer(e) && is_nonnegative(e)
}

pub fn is_positive(e: &Expr) -> bool {
    match e {
        Expr::Num(n) => n.is_positive(),
        Expr::Sym(s) => {
            get(s).positive || e.is_inf() || (e.is_const() && numeric::sign(e) == Some(1))
        }
        Expr::Add(a) => a.iter().all(is_nonnegative) && a.iter().any(is_positive),
        Expr::Mul(a) => {
            a.iter().all(|f| is_positive(f) || is_negative(f))
                && a.iter().filter(|f| is_negative(f)).count() % 2 == 0
        }
        Expr::Pow(b, x) => {
            (is_positive(b) && is_real(x))
                || (b.is_sym("e") && is_real(x))
                || (is_real(b) && is_nonzero(b) && x.as_int().is_some_and(|k| k.is_even()))
        }
        Expr::Fn(n, a) => match (&**n, a.len()) {
            ("abs", 1) => is_nonzero(&a[0]),
            ("cosh", 1) => is_real(&a[0]),
            ("factorial", 1) => is_nonnegative_integer(&a[0]),
            _ => e.is_const() && numeric::sign(e) == Some(1),
        },
        _ => false,
    }
}

pub fn is_negative(e: &Expr) -> bool {
    match e {
        Expr::Num(n) => n.is_negative(),
        Expr::Sym(s) => get(s).negative,
        Expr::Add(a) => a.iter().all(|t| is_negative(t) || t.is_zero()),
        Expr::Mul(a) => {
            a.iter().all(|f| is_positive(f) || is_negative(f))
                && a.iter().filter(|f| is_negative(f)).count() % 2 == 1
        }
        _ => e.is_const() && !e.is_infinite() && numeric::sign(e) == Some(-1),
    }
}

pub fn is_nonnegative(e: &Expr) -> bool {
    if is_positive(e) || e.is_zero() {
        return true;
    }
    match e {
        Expr::Sym(s) => get(s).nonnegative,
        Expr::Add(a) | Expr::Mul(a) => a.iter().all(is_nonnegative),
        Expr::Pow(b, x) => {
            (is_real(b) && x.as_int().is_some_and(|k| k.is_even() && k.is_positive()))
                || (is_nonnegative(b) && is_positive(x))
        }
        Expr::Fn(n, a) => matches!((&**n, a.len()), ("abs", 1)),
        _ => false,
    }
}

pub fn is_nonzero(e: &Expr) -> bool {
    match e {
        Expr::Num(n) => !n.is_zero(),
        Expr::Sym(s) => get(s).nonzero || (e.is_const() && !e.is_undefined()),
        Expr::Mul(a) => a.iter().all(is_nonzero),
        Expr::Pow(b, x) => (is_nonzero(b) && is_real(x)) || b.is_sym("e"),
        _ => {
            is_positive(e)
                || is_negative(e)
                || (e.is_const() && numeric::is_zero_const(e) == Some(false))
        }
    }
}

pub fn sign(e: &Expr) -> Option<i32> {
    if is_positive(e) {
        Some(1)
    } else if is_negative(e) {
        Some(-1)
    } else if e.is_zero() {
        Some(0)
    } else {
        None
    }
}
