use crate::num::{Flt, Int, Num, Rat};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Rel {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Rel {
    pub fn symbol(self) -> &'static str {
        match self {
            Rel::Eq => "=",
            Rel::Ne => "!=",
            Rel::Lt => "<",
            Rel::Le => "<=",
            Rel::Gt => ">",
            Rel::Ge => ">=",
        }
    }
    pub fn flip(self) -> Rel {
        match self {
            Rel::Lt => Rel::Gt,
            Rel::Le => Rel::Ge,
            Rel::Gt => Rel::Lt,
            Rel::Ge => Rel::Le,
            r => r,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Expr {
    Num(Rc<Num>),
    Float(Rc<Flt>),
    Sym(Rc<str>),
    Add(Rc<[Expr]>),
    Mul(Rc<[Expr]>),
    Pow(Rc<Expr>, Rc<Expr>),
    Fn(Rc<str>, Rc<[Expr]>),
    List(Rc<[Expr]>),
    Rel(Rel, Rc<Expr>, Rc<Expr>),
    Bool(bool),
}

pub const INF: &str = "inf";
pub const ZOO: &str = "ComplexInfinity";
pub const UNDEF: &str = "undefined";

/// Symbols that stand for fixed numbers, not for variables.
pub const NUMERIC_CONSTANTS: &[&str] = &[
    "pi",
    "e",
    "eulergamma",
    "catalan",
    "dottie",
    "khinchin",
    "soldner",
    "laplace",
    "erdosborwein",
    "cahen",
    "recfib",
    "niven",
    "mertens",
    "twinprime",
];

pub fn is_const_name(name: &str) -> bool {
    NUMERIC_CONSTANTS.contains(&name) || matches!(name, INF | ZOO | UNDEF)
}

impl Expr {
    pub fn num(n: Num) -> Expr {
        Expr::Num(Rc::new(n))
    }
    pub fn int(n: i64) -> Expr {
        Expr::num(Num::int(n))
    }
    pub fn integer(n: Int) -> Expr {
        Expr::num(Num::from_int(n))
    }
    pub fn rat(p: i64, q: i64) -> Expr {
        Expr::num(Num::frac(p, q))
    }
    pub fn rational(r: Rat) -> Expr {
        Expr::num(Num::real(r))
    }
    pub fn zero() -> Expr {
        Expr::int(0)
    }
    pub fn one() -> Expr {
        Expr::int(1)
    }
    pub fn minus_one() -> Expr {
        Expr::int(-1)
    }
    pub fn half() -> Expr {
        Expr::rat(1, 2)
    }
    pub fn i() -> Expr {
        Expr::num(Num::i())
    }
    pub fn float(f: Flt) -> Expr {
        Expr::Float(Rc::new(f))
    }
    pub fn sym(name: &str) -> Expr {
        Expr::Sym(Rc::from(name))
    }
    pub fn pi() -> Expr {
        Expr::sym("pi")
    }
    pub fn e() -> Expr {
        Expr::sym("e")
    }
    pub fn inf() -> Expr {
        Expr::sym(INF)
    }
    pub fn neg_inf() -> Expr {
        Expr::Mul(vec![Expr::minus_one(), Expr::inf()].into())
    }
    pub fn zoo() -> Expr {
        Expr::sym(ZOO)
    }
    pub fn undefined() -> Expr {
        Expr::sym(UNDEF)
    }
    /// Function application without evaluation.
    pub fn call(name: &str, args: Vec<Expr>) -> Expr {
        Expr::Fn(Rc::from(name), args.into())
    }
    pub fn list(items: Vec<Expr>) -> Expr {
        Expr::List(items.into())
    }
    pub fn rel(op: Rel, a: Expr, b: Expr) -> Expr {
        Expr::Rel(op, Rc::new(a), Rc::new(b))
    }
    pub fn eq(a: Expr, b: Expr) -> Expr {
        Expr::rel(Rel::Eq, a, b)
    }

    pub fn as_num(&self) -> Option<&Num> {
        match self {
            Expr::Num(n) => Some(n),
            _ => None,
        }
    }
    pub fn as_int(&self) -> Option<&Int> {
        self.as_num().and_then(Num::as_int)
    }
    pub fn as_i64(&self) -> Option<i64> {
        self.as_num().and_then(Num::as_i64)
    }
    pub fn as_rat(&self) -> Option<&Rat> {
        self.as_num().and_then(Num::as_rat)
    }
    pub fn is_zero(&self) -> bool {
        self.as_num().is_some_and(Num::is_zero)
    }
    pub fn is_one(&self) -> bool {
        self.as_num().is_some_and(Num::is_one)
    }
    pub fn is_minus_one(&self) -> bool {
        self.as_num().is_some_and(Num::is_minus_one)
    }
    pub fn is_number(&self) -> bool {
        matches!(self, Expr::Num(_) | Expr::Float(_))
    }
    pub fn is_sym(&self, name: &str) -> bool {
        matches!(self, Expr::Sym(s) if &**s == name)
    }
    pub fn sym_name(&self) -> Option<&str> {
        match self {
            Expr::Sym(s) => Some(s),
            _ => None,
        }
    }
    pub fn is_inf(&self) -> bool {
        self.is_sym(INF)
    }
    pub fn is_neg_inf(&self) -> bool {
        matches!(self, Expr::Mul(a) if a.len() == 2 && a[0].is_minus_one() && a[1].is_inf())
    }
    pub fn is_zoo(&self) -> bool {
        self.is_sym(ZOO)
    }
    pub fn is_undefined(&self) -> bool {
        self.is_sym(UNDEF)
    }
    /// inf, -inf, a directed infinity, complex infinity or undefined.
    pub fn is_infinite(&self) -> bool {
        match self {
            Expr::Sym(s) => matches!(&**s, INF | ZOO | UNDEF),
            Expr::Mul(a) => a.iter().any(Expr::is_infinite),
            _ => false,
        }
    }
    /// Arguments when this is the function `name`.
    pub fn fn_args(&self, name: &str) -> Option<&[Expr]> {
        match self {
            Expr::Fn(n, a) if &**n == name => Some(a),
            _ => None,
        }
    }
    pub fn fn_arg(&self, name: &str) -> Option<&Expr> {
        self.fn_args(name).filter(|a| a.len() == 1).map(|a| &a[0])
    }
    pub fn children(&self) -> &[Expr] {
        match self {
            Expr::Add(a) | Expr::Mul(a) | Expr::Fn(_, a) | Expr::List(a) => a,
            _ => &[],
        }
    }
    pub fn as_list(&self) -> Option<&[Expr]> {
        match self {
            Expr::List(a) => Some(a),
            _ => None,
        }
    }
    /// base and exponent of a power, `(self, 1)` otherwise.
    pub fn base_exp(&self) -> (Expr, Expr) {
        match self {
            Expr::Pow(b, e) => ((**b).clone(), (**e).clone()),
            _ => (self.clone(), Expr::one()),
        }
    }

    pub fn walk(&self, f: &mut dyn FnMut(&Expr)) {
        f(self);
        match self {
            Expr::Add(a) | Expr::Mul(a) | Expr::Fn(_, a) | Expr::List(a) => {
                a.iter().for_each(|x| x.walk(f))
            }
            Expr::Pow(a, b) | Expr::Rel(_, a, b) => {
                a.walk(f);
                b.walk(f);
            }
            _ => {}
        }
    }
    pub fn any(&self, f: &dyn Fn(&Expr) -> bool) -> bool {
        if f(self) {
            return true;
        }
        match self {
            Expr::Add(a) | Expr::Mul(a) | Expr::Fn(_, a) | Expr::List(a) => {
                a.iter().any(|x| x.any(f))
            }
            Expr::Pow(a, b) | Expr::Rel(_, a, b) => a.any(f) || b.any(f),
            _ => false,
        }
    }
    pub fn collect_syms(&self, out: &mut BTreeSet<Rc<str>>) {
        self.walk(&mut |e| {
            if let Expr::Sym(s) = e
                && !is_const_name(s)
            {
                out.insert(s.clone());
            }
        });
    }
    /// Free variables (named constants excluded), sorted.
    pub fn free_syms(&self) -> BTreeSet<Rc<str>> {
        let mut out = BTreeSet::new();
        self.collect_syms(&mut out);
        out
    }
    pub fn has_sym(&self, name: &str) -> bool {
        self.any(&|e| e.is_sym(name))
    }
    pub fn has(&self, sub: &Expr) -> bool {
        self.any(&|e| e == sub)
    }
    pub fn has_fn(&self, name: &str) -> bool {
        self.any(&|e| matches!(e, Expr::Fn(n, _) if &**n == name))
    }
    pub fn has_float(&self) -> bool {
        self.any(&|e| matches!(e, Expr::Float(_)))
    }
    /// No free variables.
    pub fn is_const(&self) -> bool {
        !self.any(&|e| matches!(e, Expr::Sym(s) if !is_const_name(s)))
    }
    pub fn free_of(&self, var: &str) -> bool {
        !self.has_sym(var)
    }
    pub fn size(&self) -> usize {
        let mut n = 0;
        self.walk(&mut |_| n += 1);
        n
    }

    fn rank(&self) -> u8 {
        match self {
            Expr::Num(_) => 0,
            Expr::Float(_) => 1,
            Expr::Sym(_) => 2,
            Expr::Pow(..) => 3,
            Expr::Fn(..) => 4,
            Expr::Mul(_) => 5,
            Expr::Add(_) => 6,
            Expr::List(_) => 7,
            Expr::Rel(..) => 8,
            Expr::Bool(_) => 9,
        }
    }
}

impl Ord for Expr {
    fn cmp(&self, o: &Self) -> Ordering {
        use Expr::*;
        match (self, o) {
            (Num(a), Num(b)) => a.cmp(b),
            (Float(a), Float(b)) => a.cmp(b),
            (Sym(a), Sym(b)) => a.cmp(b),
            (Add(a), Add(b)) | (Mul(a), Mul(b)) | (List(a), List(b)) => a.cmp(b),
            (Pow(a, x), Pow(b, y)) => a.cmp(b).then_with(|| x.cmp(y)),
            (Fn(n, a), Fn(m, b)) => n.cmp(m).then_with(|| a.cmp(b)),
            (Rel(p, a, x), Rel(q, b, y)) => p.cmp(q).then_with(|| a.cmp(b)).then_with(|| x.cmp(y)),
            (Bool(a), Bool(b)) => a.cmp(b),
            _ => self.rank().cmp(&o.rank()),
        }
    }
}

impl PartialOrd for Expr {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl From<i64> for Expr {
    fn from(n: i64) -> Expr {
        Expr::int(n)
    }
}
impl From<Int> for Expr {
    fn from(n: Int) -> Expr {
        Expr::integer(n)
    }
}
impl From<Rat> for Expr {
    fn from(r: Rat) -> Expr {
        Expr::rational(r)
    }
}
impl From<Num> for Expr {
    fn from(n: Num) -> Expr {
        Expr::num(n)
    }
}
