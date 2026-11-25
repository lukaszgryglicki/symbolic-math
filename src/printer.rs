//! Expression to text. The output is ASCII and can be parsed again.

use crate::canon::{coeff_and_rest, neg};
use crate::expr::Expr;
use crate::funcs::is_neg_form;
use crate::num::{Int, Num, Rat};
use crate::numeric::fmt_real;
use std::fmt;

const P_ADD: u8 = 1;
const P_MUL: u8 = 2;
const P_POW: u8 = 4;
const P_TET: u8 = 5;
const P_ATOM: u8 = 6;

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&show(self, 0))
    }
}

pub fn to_string(e: &Expr) -> String {
    show(e, 0)
}

fn show(e: &Expr, min: u8) -> String {
    let (s, p) = render(e);
    if p < min { format!("({s})") } else { s }
}

fn rat_parts(r: &Rat) -> (bool, Int, Int) {
    (r.is_negative(), r.numer().clone().abs(), r.denom().clone())
}

fn num_str(n: &Num) -> (String, u8) {
    if n.is_real() {
        let (neg, p, q) = rat_parts(&n.re);
        let body = if q == 1 {
            p.to_string()
        } else {
            format!("{p}/{q}")
        };
        return (
            if neg { format!("-{body}") } else { body },
            if q == 1 && !neg {
                P_ATOM
            } else {
                P_MUL - u8::from(neg)
            },
        );
    }
    let (neg, p, q) = rat_parts(&n.im);
    let mut im = String::new();
    if p != 1 {
        im.push_str(&p.to_string());
    }
    im.push('i');
    if q != 1 {
        im.push_str(&format!("/{q}"));
    }
    if n.re.is_zero() {
        let simple = p == 1 && q == 1;
        return (
            if neg { format!("-{im}") } else { im },
            if neg {
                P_ADD
            } else if simple {
                P_ATOM
            } else {
                P_MUL
            },
        );
    }
    (
        format!(
            "{} {} {}",
            num_str(&Num::real(n.re.clone())).0,
            if neg { '-' } else { '+' },
            im
        ),
        P_ADD,
    )
}

fn starts_like_name(s: &str) -> bool {
    s.chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '(' || c == '_')
}

fn join_factors(lead: Option<String>, parts: &[String]) -> String {
    let mut out = String::new();
    for (k, part) in parts.iter().enumerate() {
        if k > 0 {
            out.push('*');
        }
        out.push_str(part);
    }
    match lead {
        None if out.is_empty() => "1".into(),
        None => out,
        Some(l) if out.is_empty() => l,
        Some(l) if juxtaposes(&out) => format!("{l}{out}"),
        Some(l) => format!("{l}*{out}"),
    }
}

/// A number may stand directly before this text: `5x`, `2sin(x)`, `3(x + 1)`, but `2*e5`.
fn juxtaposes(s: &str) -> bool {
    let mut it = s.chars();
    let (first, second) = (it.next(), it.next());
    let exponent_like = matches!(first, Some('e' | 'E'))
        && second.is_some_and(|c| c.is_ascii_digit() || c == '+' || c == '-');
    starts_like_name(s) && !exponent_like
}

fn factor_class(f: &Expr) -> (u8, String) {
    let class = match f {
        Expr::Pow(b, e) if b.is_number() && e.is_number() => 0,
        Expr::Sym(_) => 1,
        Expr::Pow(b, e) if matches!(**b, Expr::Sym(_)) && e.is_number() => 1,
        Expr::Pow(b, _) if matches!(**b, Expr::Add(_)) => 4,
        Expr::Pow(..) => 2,
        Expr::Fn(..) => 3,
        Expr::Add(_) => 4,
        _ => 5,
    };
    let name = match f {
        Expr::Pow(b, _) if class == 1 => show(b, 0),
        _ => show(f, 0),
    };
    (class, name)
}

/// Sign and unsigned text of a product.
fn mul_parts(args: &[Expr]) -> (bool, String) {
    let (mut negative, mut lead, mut den_lead) = (false, None, None);
    let mut top: Vec<String> = Vec::new();
    let mut bottom: Vec<String> = Vec::new();
    let rest = match &args[0] {
        Expr::Num(n) if n.is_real() || n.re.is_zero() => {
            let (neg, p, q) = rat_parts(if n.is_real() { &n.re } else { &n.im });
            negative = neg;
            if p != 1 {
                lead = Some(p.to_string());
            }
            if q != 1 {
                den_lead = Some(q.to_string());
            }
            if !n.is_real() {
                top.push("i".into());
            }
            &args[1..]
        }
        Expr::Float(f) if f.v.imag().is_zero() => {
            negative = f.v.real().is_sign_negative();
            lead = Some(fmt_real(&f.v.real().clone().abs(), f.digits));
            &args[1..]
        }
        _ => args,
    };
    let mut ordered: Vec<&Expr> = rest.iter().collect();
    ordered.sort_by_cached_key(|f| factor_class(f));
    for f in ordered {
        match f {
            Expr::Pow(b, e) if e.as_rat().is_some_and(|r| r.is_negative()) => {
                bottom.push(pow_str(b, &neg((**e).clone())).0);
            }
            _ => top.push(show(f, P_MUL + 1)),
        }
    }
    let numerator = join_factors(lead, &top);
    if bottom.is_empty() && den_lead.is_none() {
        return (negative, numerator);
    }
    let single = bottom.len() + usize::from(den_lead.is_some()) == 1;
    let denominator = join_factors(den_lead, &bottom);
    (
        negative,
        if single {
            format!("{numerator}/{denominator}")
        } else {
            format!("{numerator}/({denominator})")
        },
    )
}

fn atom_like(e: &Expr) -> bool {
    match e {
        Expr::Num(n) => (n.is_int() && !n.is_negative()) || (n.re.is_zero() && n.im == 1),
        Expr::Sym(_) => true,
        // (a quaternion and a quantity are written as a sum and as a product)
        Expr::Fn(n, a) if &**n == "quat" && a.len() == 4 => !crate::assume::quaternion_mode(),
        Expr::Fn(n, _) => !matches!(
            &**n,
            "tet" | "factorial" | "factorial2" | "derivative'" | "hold" | "quantity"
        ),
        Expr::Float(f) => f.v.imag().is_zero() && !f.v.real().is_sign_negative(),
        _ => false,
    }
}

fn pow_str(b: &Expr, e: &Expr) -> (String, u8) {
    if e.is_one() {
        return (show(b, P_MUL + 1), P_MUL + 1);
    }
    if e.as_rat().is_some_and(|r| *r == crate::num::rat(1, 2)) {
        return (format!("sqrt({})", show(b, 0)), P_ATOM);
    }
    if e.as_rat().is_some_and(|r| r.is_negative()) {
        return (format!("1/{}", pow_str(b, &neg(e.clone())).0), P_MUL);
    }
    let base = if atom_like(b) {
        show(b, 0)
    } else {
        format!("({})", show(b, 0))
    };
    let exponent = if atom_like(e) {
        show(e, 0)
    } else {
        format!("({})", show(e, 0))
    };
    (format!("{base}^{exponent}"), P_POW)
}

/// Where a term stands in a sum: its text decides between terms that are alike in the rest,
/// and is written only then (a tower of sums and powers would be written twice at each floor).
struct TermKey {
    group: u8,
    degree: i64,
    vars: Vec<(String, i64)>,
    rest: Expr,
    text: std::cell::OnceCell<String>,
}

impl TermKey {
    fn new(group: u8, degree: i64, vars: Vec<(String, i64)>, rest: Expr) -> Self {
        TermKey {
            group,
            degree,
            vars,
            rest,
            text: std::cell::OnceCell::new(),
        }
    }
    fn text(&self) -> &str {
        self.text.get_or_init(|| show(&self.rest, 0))
    }
}

impl Ord for TermKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.group
            .cmp(&other.group)
            .then_with(|| self.degree.cmp(&other.degree))
            .then_with(|| self.vars.cmp(&other.vars))
            .then_with(|| self.text().cmp(other.text()))
    }
}

impl PartialOrd for TermKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for TermKey {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl Eq for TermKey {}

fn term_key(t: &Expr) -> TermKey {
    let (c, rest) = coeff_and_rest(t);
    if t.any(&|x| matches!(x, Expr::Sym(s) if crate::assume::is_integer_sym(s))) {
        return TermKey::new(5, 0, Vec::new(), rest);
    }
    if t.is_const() {
        let group = if t.as_num().is_some_and(Num::is_real) {
            2
        } else if c.is_real() {
            3
        } else {
            4
        };
        return TermKey::new(group, 0, Vec::new(), rest);
    }
    let factors: Vec<Expr> = match &rest {
        Expr::Mul(a) => a.to_vec(),
        other => vec![other.clone()],
    };
    let (mut degree, mut vars, mut polynomial) = (0i64, Vec::new(), true);
    for f in &factors {
        let (b, e) = f.base_exp();
        match (&b, e.as_rat()) {
            (Expr::Sym(s), Some(r)) => {
                if !b.is_const() {
                    // (an exponent beyond every degree that a sum is ordered by counts as 2^60)
                    let scaled = ((r.to_f64() * 1e6) as i64).clamp(-(1 << 60), 1 << 60);
                    degree = degree.saturating_add(scaled);
                    vars.push((s.to_string(), -scaled));
                }
            }
            _ if f.is_const() => {}
            _ => polynomial = false,
        }
    }
    vars.sort();
    TermKey::new(u8::from(!polynomial), -degree, vars, rest)
}

/// The terms of a sum in the order they are written in.
pub(crate) fn ordered_terms(terms: &[Expr]) -> Vec<Expr> {
    let mut ordered: Vec<&Expr> = terms.iter().collect();
    ordered.sort_by_cached_key(|t| term_key(t));
    if ordered.len() == 2
        && is_neg_form(ordered[0])
        && !is_neg_form(ordered[1])
        && term_key(ordered[1]).group < 4
    {
        ordered.swap(0, 1);
    }
    ordered.into_iter().cloned().collect()
}

fn add_str(terms: &[Expr]) -> String {
    let mut split: Vec<Expr> = Vec::with_capacity(terms.len() + 1);
    for t in terms {
        match t {
            Expr::Num(n) if !n.re.is_zero() && !n.im.is_zero() => {
                split.push(Expr::rational(n.re.clone()));
                split.push(Expr::num(Num::new(Rat::new(), n.im.clone())));
            }
            // a product with such a coefficient: its real and imaginary multiples
            Expr::Mul(a) if matches!(&a[0], Expr::Num(n) if !n.re.is_zero() && !n.im.is_zero()) => {
                let n = a[0].as_num().unwrap();
                for c in [Num::real(n.re.clone()), Num::new(Rat::new(), n.im.clone())] {
                    let factors = std::iter::once(Expr::num(c)).chain(a[1..].iter().cloned());
                    split.push(crate::canon::mul(factors.collect()));
                }
            }
            _ => split.push(t.clone()),
        }
    }
    let mut ordered: Vec<&Expr> = split.iter().collect();
    ordered.sort_by_cached_key(|t| term_key(t));
    if ordered.len() == 2
        && is_neg_form(ordered[0])
        && !is_neg_form(ordered[1])
        && term_key(ordered[1]).group < 4
    {
        ordered.swap(0, 1);
    }
    let mut out = String::new();
    for (k, t) in ordered.iter().enumerate() {
        let (negative, body) = signed(t);
        if k == 0 {
            if negative {
                out.push('-');
            }
        } else {
            out.push_str(if negative { " - " } else { " + " });
        }
        out.push_str(&body);
    }
    out
}

/// Sign and the text without it, as a term of a sum.
fn signed(t: &Expr) -> (bool, String) {
    match t {
        Expr::Mul(a) => mul_parts(a),
        Expr::Num(n) if n.is_real() || n.re.is_zero() => {
            let negative = n.is_neg_form();
            (
                negative,
                num_str(&if negative { n.neg() } else { (**n).clone() }).0,
            )
        }
        Expr::Float(f) if f.v.imag().is_zero() => (
            f.v.real().is_sign_negative(),
            fmt_real(&f.v.real().clone().abs(), f.digits),
        ),
        Expr::Float(f) if f.v.real().is_zero() => (
            f.v.imag().is_sign_negative(),
            format!("{}i", fmt_real(&f.v.imag().clone().abs(), f.digits)),
        ),
        _ => (false, show(t, P_ADD + 1)),
    }
}

fn held(e: &Expr) -> (String, u8) {
    match e {
        Expr::Mul(a) => {
            let mut out = String::new();
            for (k, f) in a.iter().enumerate() {
                let part = match f {
                    Expr::Pow(b, x)
                        if !x.is_number()
                            || x.as_rat().is_some_and(|r| *r == crate::num::rat(1, 2)) =>
                    {
                        pow_str(b, x).0
                    }
                    Expr::Pow(b, x) => format!("{}^{}", held_operand(b), held_operand(x)),
                    Expr::Add(_) | Expr::Mul(_) => format!("({})", held(f).0),
                    _ => show(f, P_MUL + 1),
                };
                if k > 0 && !(k == 1 && a[0].is_number() && starts_like_name(&part)) {
                    out.push('*');
                }
                out.push_str(&part);
            }
            (out, P_MUL)
        }
        Expr::Pow(b, x) if !x.is_number() => pow_str(b, x),
        Expr::Pow(b, x) => (format!("{}^{}", held_operand(b), held_operand(x)), P_POW),
        Expr::Add(a) => (add_str(a), P_ADD),
        _ => render(e),
    }
}

fn held_operand(e: &Expr) -> String {
    if atom_like(e) {
        show(e, 0)
    } else {
        format!("({})", held(e).0)
    }
}

fn render(e: &Expr) -> (String, u8) {
    match e {
        Expr::Num(n) => num_str(n),
        Expr::Float(f) => {
            let s = crate::numeric::fmt_complex(&f.v, f.digits);
            let p = if (!f.v.imag().is_zero() && !f.v.real().is_zero()) || s.starts_with('-') {
                P_ADD
            } else if !f.v.imag().is_zero() {
                P_MUL
            } else {
                P_ATOM
            };
            (s, p)
        }
        Expr::Sym(s) => (s.to_string(), P_ATOM),
        Expr::Add(a) => (add_str(a), P_ADD),
        Expr::Mul(a) if matches!(&a[0], Expr::Num(n) if !n.re.is_zero() && !n.im.is_zero()) => {
            (add_str(std::slice::from_ref(e)), P_ADD)
        }
        Expr::Mul(a) => {
            let (negative, body) = mul_parts(a);
            if negative {
                (format!("-{body}"), P_ADD)
            } else {
                (body, P_MUL)
            }
        }
        Expr::Pow(b, x) => pow_str(b, x),
        Expr::Fn(name, args) => match (&**name, args.len()) {
            (":=", 2) => (format!("{} := {}", show(&args[0], 0), show(&args[1], 0)), 0),
            ("cellarray", _) => {
                let inner: Vec<String> = args.iter().map(|a| show(a, 0)).collect();
                (format!("{{{}}}", inner.join(", ")), P_ATOM)
            }
            ("factorial", 1) | ("factorial2", 1) => {
                let bang = if &**name == "factorial" { "!" } else { "!!" };
                (
                    format!(
                        "{}{bang}",
                        if atom_like(&args[0]) {
                            show(&args[0], 0)
                        } else {
                            format!("({})", args[0])
                        }
                    ),
                    P_TET,
                )
            }
            ("tet", 2) => {
                let side = |x: &Expr| {
                    if atom_like(x) {
                        show(x, 0)
                    } else {
                        format!("({x})")
                    }
                };
                (format!("{}^^{}", side(&args[0]), side(&args[1])), P_TET)
            }
            ("derivative'", 1) => {
                let inner = match &args[0] {
                    Expr::Fn(n, _) if &**n == "derivative'" => show(&args[0], 0),
                    other => show(other, P_ATOM),
                };
                (format!("{inner}'"), P_TET)
            }
            ("rule", 2) => (
                format!("{} -> {}", show(&args[0], P_ADD), show(&args[1], P_ADD)),
                0,
            ),
            ("hold", 1) => held(&args[0]),
            ("string", 1) if args[0].sym_name().is_some() => (
                format!("\"{}\"", args[0].sym_name().unwrap_or_default()),
                P_ATOM,
            ),
            // a number as it was written in a base
            ("based", 2) if args[1].sym_name().is_some() => {
                let text = args[1].sym_name().unwrap_or_default();
                let sum = text.starts_with('-') || text.contains(' ');
                (text.to_string(), if sum { P_ADD } else { P_ATOM })
            }
            ("and" | "or" | "xor", 2..) => {
                let parts: Vec<String> = args
                    .iter()
                    .map(|x| match x {
                        Expr::Fn(n, _) if matches!(&**n, "and" | "or" | "xor") && **n != **name => {
                            format!("({})", show(x, 0))
                        }
                        _ => show(x, 0),
                    })
                    .collect();
                (parts.join(&format!(" {name} ")), 0)
            }
            ("not", 1) => (
                match &args[0] {
                    Expr::Sym(_) | Expr::Bool(_) => format!("not {}", show(&args[0], 0)),
                    other => format!("not ({})", show(other, 0)),
                },
                0,
            ),
            ("iterate", 4)
                if args[1].is_sym("#it")
                    && matches!(&args[0], Expr::Fn(_, inner) if inner.len() == 1 && inner[0].is_sym("#it")) =>
            {
                let Expr::Fn(inner, _) = &args[0] else {
                    unreachable!()
                };
                (
                    format!("{inner}[{}]({})", show(&args[2], 0), show(&args[3], 0)),
                    P_ATOM,
                )
            }
            ("quat", 4) if crate::assume::quaternion_mode() => {
                let mut out = String::new();
                for (c, unit) in args.iter().zip(["", "i", "j", "k"]) {
                    if c.is_zero() {
                        continue;
                    }
                    if unit.is_empty() {
                        out.push_str(&show(c, P_ADD));
                        continue;
                    }
                    let negative = crate::funcs::is_neg_form(c);
                    let size = if negative {
                        crate::canon::neg(c.clone())
                    } else {
                        c.clone()
                    };
                    let body = if size.is_number() {
                        signed(&crate::canon::mul(vec![size, Expr::sym(unit)])).1
                    } else {
                        let (top, low) = crate::canon::numer_denom(&size);
                        let mut text = if top.is_one() {
                            unit.to_string()
                        } else {
                            format!("{}*{unit}", show(&top, P_MUL))
                        };
                        if !low.is_one() {
                            text.push('/');
                            text.push_str(&show(&low, P_POW));
                        }
                        text
                    };
                    if !out.is_empty() {
                        out.push_str(if negative { " - " } else { " + " });
                    } else if negative {
                        out.push('-');
                    }
                    out.push_str(&body);
                }
                (out, P_ADD)
            }
            ("quantity", 2) => (
                format!(
                    "{} {}",
                    crate::units::shown(&args[0]),
                    crate::units::unit_text(&args[1])
                ),
                P_MUL,
            ),
            ("ordered", _) => {
                let mut out = String::new();
                for (k, t) in args.iter().enumerate() {
                    let (negative, body) = signed(t);
                    if k > 0 {
                        out.push_str(if negative { " - " } else { " + " });
                    } else if negative {
                        out.push('-');
                    }
                    out.push_str(&body);
                }
                (out, P_ADD)
            }
            _ => {
                let inner: Vec<String> = args.iter().map(|a| show(a, 0)).collect();
                (format!("{name}({})", inner.join(", ")), P_ATOM)
            }
        },
        Expr::List(items) => {
            let inner: Vec<String> = items.iter().map(|a| show(a, 0)).collect();
            (format!("[{}]", inner.join(", ")), P_ATOM)
        }
        Expr::Rel(op, a, b) => (
            format!("{} {} {}", show(a, P_ADD), op.symbol(), show(b, P_ADD)),
            0,
        ),
        Expr::Bool(b) => (b.to_string(), P_ATOM),
    }
}
