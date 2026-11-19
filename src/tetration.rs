//! Tetration `b^^h`, super-logarithm and super-root.
//!
//! Exact rules cover whole heights; every other value comes from the external `tet` program,
//! called as `tet-line` calls it.

use crate::canon::{add, pow};
use crate::expr::Expr;
use crate::funcs::func;
use crate::num::Num;
use crate::numeric::{self, Ctx, Env, NumErr, bits_for, cpow, digits_for, fmt_real};
use rug::{Complex, Float};
use std::cell::RefCell;
use std::collections::HashMap;
use std::process::{Command, Stdio};

pub const PROGRAM: &str = "tet";
const TOWER_LIMIT: i64 = 64;

pub fn is_tet_fn(name: &str) -> bool {
    matches!(name, "tet" | "slog" | "ssrt")
}

pub fn uses_tet(e: &Expr) -> bool {
    e.any(&|x| matches!(x, Expr::Fn(n, a) if is_tet_fn(n) && !(&**n == "tet" && whole_height(&a[1]).is_some())))
}

fn whole_height(h: &Expr) -> Option<i64> {
    h.as_i64().filter(|n| (-1..=TOWER_LIMIT).contains(n))
}

/// `b^^n` for a whole `n` as a number, when the tower stays a number.
fn exact_tower(b: &Num, n: i64) -> Option<Expr> {
    if n == -1 {
        return (!b.is_zero() && !b.is_one()).then(Expr::zero);
    }
    let base = Expr::num(b.clone());
    let mut t = Expr::one();
    for _ in 0..n {
        t = pow(base.clone(), t);
        t.as_num()?;
    }
    Some(t)
}

/// The tower `a^a^a^...` without end: it settles for the bases `e^(t e^(-t))` with `|t| < 1`,
/// and for the real ones from `e^(-e)` to `e^(1/e)`, at `e^t = -W(-ln a)/ln a`.
fn endless_tower(a: &Expr) -> Option<Expr> {
    use crate::canon::{div, neg};
    if a.is_one() {
        return Some(Expr::one());
    }
    if !a.is_const() {
        return None;
    }
    let ln = |x: Expr| func("ln", vec![x]);
    let e = Expr::e;
    let (top, bottom) = (pow(e(), pow(e(), Expr::minus_one())), pow(e(), neg(e())));
    if *a == top {
        return Some(e());
    }
    if *a == bottom {
        return Some(pow(e(), Expr::minus_one()));
    }
    // c^(1/c) for a number c from 1/e to e gives c back
    let fixed = |c: &Expr| {
        let v = numeric::real_value(c, 30)?;
        (v >= Float::with_val(64, -1).exp() && v <= Float::with_val(64, 1).exp()).then(|| c.clone())
    };
    if let Expr::Pow(c, x) = a
        && c.as_rat().is_some()
        && crate::simplify::is_zero(&crate::canon::sub(
            crate::canon::mul(vec![(**c).clone(), (**x).clone()]),
            Expr::one(),
        ))
        && let Some(c) = fixed(c)
    {
        return Some(c);
    }
    if *a == Expr::rat(1, 4) {
        return Some(Expr::half());
    }
    let value = numeric::evalf_digits(a, 30).ok()?;
    let closed = || {
        div(
            neg(func("lambertw", vec![neg(ln(a.clone()))])),
            ln(a.clone()),
        )
    };
    if value.imag().is_zero() {
        let x = value.real();
        let (hi, lo) = (
            numeric::real_value(&top, 30)?,
            numeric::real_value(&bottom, 30)?,
        );
        return if *x > hi {
            Some(Expr::inf())
        } else if *x >= lo {
            Some(closed())
        } else if !x.is_sign_negative() {
            // two values in turn, and no limit
            Some(Expr::undefined())
        } else {
            None
        };
    }
    let w = numeric::evalf_digits(&func("lambertw", vec![neg(ln(a.clone()))]), 30).ok()?;
    (Float::with_val(64, w.abs_ref()) < 1).then(closed)
}

pub fn simplify(name: &str, a: &Expr, b: &Expr) -> Option<Expr> {
    match name {
        "tet" => {
            if b.is_zero() {
                return Some(Expr::one());
            }
            if b.is_one() {
                return Some(a.clone());
            }
            if b.is_inf() {
                return endless_tower(a);
            }
            if let (Some(base), Some(n)) = (a.as_num(), whole_height(b))
                && let Some(t) = exact_tower(base, n)
            {
                return Some(t);
            }
            if let Some(args) = b.fn_args("slog")
                && args[0] == *a
            {
                return Some(args[1].clone());
            }
            // a^^(slog(a, x) + n): n more exponentials (or logarithms) of x
            if let Expr::Add(terms) = b
                && let [first, second] = &terms[..]
                && let Some((n, inner)) = match (first.as_i64(), second.as_i64()) {
                    (Some(n), None) => Some((n, second)),
                    (None, Some(n)) => Some((n, first)),
                    _ => None,
                }
                && n.abs() <= 16
                && let Some(args) = inner.fn_args("slog")
                && args[0] == *a
            {
                let mut v = args[1].clone();
                for _ in 0..n.abs() {
                    v = if n > 0 {
                        pow(a.clone(), v)
                    } else {
                        crate::canon::div(func("ln", vec![v]), func("ln", vec![a.clone()]))
                    };
                }
                return Some(v);
            }
            if let Some(args) = a.fn_args("ssrt")
                && args[1] == *b
            {
                return Some(args[0].clone());
            }
            None
        }
        "slog" => {
            if a.is_zero() || a.is_one() {
                return Some(Expr::undefined());
            }
            if b.is_one() {
                return Some(Expr::zero());
            }
            if b == a {
                return Some(Expr::one());
            }
            if b.is_zero() {
                return Some(Expr::minus_one());
            }
            if let Some(args) = b.fn_args("tet")
                && args[0] == *a
            {
                return Some(args[1].clone());
            }
            if let (Some(base), Some(value)) = (a.as_num(), b.as_num()) {
                let mut t = Num::one();
                for n in 0..=TOWER_LIMIT {
                    if t == *value {
                        return Some(Expr::int(n));
                    }
                    if t.bits() > value.bits() + 64 {
                        break;
                    }
                    match pow(Expr::num(base.clone()), Expr::num(t)).as_num() {
                        Some(next) => t = next.clone(),
                        None => break,
                    }
                }
            }
            if let Expr::Pow(base, w) = b
                && **base == *a
            {
                let inner = func("slog", vec![a.clone(), (**w).clone()]);
                if !inner.has_fn("slog") {
                    return Some(add(vec![Expr::one(), inner]));
                }
            }
            None
        }
        _ => {
            if b.is_one() {
                return Some(a.clone());
            }
            if b.is_zero() {
                return Some(Expr::undefined());
            }
            // the base of an endless tower with the value a: a^(1/a), when the tower of that
            // base settles at a
            if b.is_inf() && a.is_const() {
                let base = pow(a.clone(), pow(a.clone(), Expr::minus_one()));
                let back = endless_tower(&base)?;
                let same = numeric::is_zero_const(&crate::canon::sub(back, a.clone()));
                return Some(if same == Some(true) {
                    base
                } else {
                    Expr::undefined()
                });
            }
            if let Some(args) = a.fn_args("tet")
                && args[1] == *b
                && args[0].as_rat().is_some_and(|r| *r >= 1)
            {
                return Some(args[0].clone());
            }
            if let (Some(value), Some(n)) = (a.as_num(), whole_height(b).filter(|n| *n >= 1)) {
                if value.is_one() {
                    return Some(Expr::one());
                }
                if value.is_int() && value.is_positive() {
                    for base in 2..=64 {
                        let t = exact_tower(&Num::int(base), n)?;
                        let t = t.as_num()?;
                        if t == value {
                            return Some(Expr::int(base));
                        }
                        if t.re > value.re {
                            break;
                        }
                    }
                }
            }
            None
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Mode {
    Tet,
    Slog,
    Sroot,
}

type Key = (Mode, Expr, Expr, String);

thread_local! {
    static CACHE: RefCell<HashMap<Key, (u32, Complex)>> = RefCell::new(HashMap::new());
}

fn env_key(a: &Expr, b: &Expr, env: Option<&Env>) -> String {
    let Some(env) = env else { return String::new() };
    let mut names = a.free_syms();
    names.extend(b.free_syms());
    names
        .iter()
        .filter_map(|n| {
            env.get(n)
                .map(|v| format!("{n}={};", v.to_string_radix(32, None)))
        })
        .collect()
}

fn tet_arg(x: &Float, digits: u32) -> String {
    fmt_real(x, digits)
}

fn parse_output(out: &[u8], prec: u32) -> Result<Complex, String> {
    let text = std::str::from_utf8(out).map_err(|_| "tet output is not UTF-8".to_string())?;
    let mut parts = text.split_whitespace();
    let mut number = || -> Result<Float, String> {
        let part = parts.next().ok_or("tet did not return two numbers")?;
        let parsed =
            Float::parse(part).map_err(|_| format!("tet returned `{part}`, not a number"))?;
        let value = Float::with_val(prec, parsed);
        if value.is_finite() {
            Ok(value)
        } else {
            Err("tet returned a value that is not finite".into())
        }
    };
    let value = Complex::with_val(prec, (number()?, number()?));
    if parts.next().is_some() {
        return Err("tet returned unexpected extra output".into());
    }
    Ok(value)
}

#[cfg(not(all(target_os = "freebsd", target_feature = "crt-static")))]
fn command() -> Command {
    Command::new(PROGRAM)
}

/// In a static FreeBSD executable the standard library cannot find `environ` (it asks dlsym, which
/// has nothing to search there: rust-lang/rust#158939) and crashes when it starts a process with the
/// inherited environment. The environment is read here and passed on, and the program is found
/// on PATH here, so that the library needs neither.
#[cfg(all(target_os = "freebsd", target_feature = "crt-static"))]
fn command() -> Command {
    use std::ffi::{CStr, OsString, c_char};
    use std::os::unix::ffi::OsStringExt;
    unsafe extern "C" {
        static environ: *const *const c_char;
    }
    let mut inherited = Vec::new();
    // SAFETY: `environ` is the null-terminated array of the C runtime, and nothing here changes it.
    unsafe {
        let mut entry = environ;
        while !entry.is_null() && !(*entry).is_null() {
            let bytes = CStr::from_ptr(*entry).to_bytes();
            if let Some(at) = bytes.iter().skip(1).position(|b| *b == b'=') {
                let (key, value) = (bytes[..=at].to_vec(), bytes[at + 2..].to_vec());
                inherited.push((OsString::from_vec(key), OsString::from_vec(value)));
            }
            entry = entry.add(1);
        }
    }
    let found = std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|dir| dir.join(PROGRAM))
            .find(|file| file.is_file())
    });
    // a path that does not exist gives the same "not found" as a name that is not on PATH
    let mut command = Command::new(
        found.unwrap_or_else(|| std::path::PathBuf::from(format!("./{PROGRAM}-is-not-on-PATH"))),
    );
    command.env_clear().envs(inherited);
    command
}

fn run_tet(mode: Mode, digits: u32, a: &Complex, b: &Complex) -> Result<Complex, NumErr> {
    let flags: &[&str] = match mode {
        Mode::Tet => &["-q"],
        Mode::Slog => &["-q", "--slog"],
        Mode::Sroot => &["-q", "--sroot"],
    };
    let d = digits + 8;
    let output = command()
        .args(flags)
        .arg(digits.to_string())
        .args([
            tet_arg(a.real(), d),
            tet_arg(a.imag(), d),
            tet_arg(b.real(), d),
            tet_arg(b.imag(), d),
        ])
        .env("SILENT", "1")
        .stdin(Stdio::null())
        .output()
        .map_err(|err| {
            NumErr::External(format!(
                "could not run tet: {err}; ensure the tet executable is on PATH"
            ))
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason: String = stderr
            .trim()
            .trim_start_matches("error:")
            .trim()
            .chars()
            .take(300)
            .collect();
        return Err(NumErr::External(if reason.is_empty() {
            format!("tet exited with {}", output.status)
        } else {
            format!("tet: {reason}")
        }));
    }
    parse_output(&output.stdout, bits_for(digits) + 8).map_err(NumErr::External)
}

fn tower(base: &Complex, n: i64, prec: u32) -> Option<Complex> {
    if n == -1 {
        return Some(Complex::new(prec));
    }
    let mut t = Complex::with_val(prec, 1);
    for _ in 0..n {
        t = cpow(base, &t)?;
    }
    Some(t)
}

fn value(
    mode: Mode,
    a: &Expr,
    b: &Expr,
    digits: u32,
    env: Option<&Env>,
) -> Result<Complex, NumErr> {
    let key = (mode, a.clone(), b.clone(), env_key(a, b, env));
    if let Some((_, v)) =
        CACHE.with(|c| c.borrow().get(&key).filter(|(d, _)| *d >= digits).cloned())
    {
        return Ok(v);
    }
    let x = numeric::evalf_digits_env(a, digits + 6, env)?;
    let y = numeric::evalf_digits_env(b, digits + 6, env)?;
    let v = run_tet(mode, digits, &x, &y)?;
    CACHE.with(|c| c.borrow_mut().insert(key, (digits, v.clone())));
    Ok(v)
}

fn mode_of(name: &str, args: &[Expr]) -> Option<Mode> {
    match (name, args.len()) {
        ("tet", 2) => Some(Mode::Tet),
        ("slog", 2) => Some(Mode::Slog),
        ("ssrt", 2) => Some(Mode::Sroot),
        _ => None,
    }
}

/// Numeric value of a tetration function inside an evaluation at `cx.prec` bits.
pub fn numeric(name: &str, args: &[Expr], cx: &mut Ctx) -> Option<Result<Complex, NumErr>> {
    let mode = mode_of(name, args)?;
    if mode == Mode::Tet
        && let Some(n) = whole_height(&args[1])
    {
        let sub = Ctx {
            prec: cx.prec,
            env: cx.env,
            max_exp: i64::MIN,
        };
        let base = match cx.env {
            Some(env) => numeric::evalf_env(&args[0], sub.prec, env),
            None => numeric::evalf(&args[0], sub.prec),
        };
        return Some(base.and_then(|b| {
            tower(&b, n, cx.prec)
                .ok_or_else(|| NumErr::Undefined(format!("{}^^{}", args[0], args[1])))
        }));
    }
    let digits = digits_for(cx.prec) + 2;
    Some(value(mode, &args[0], &args[1], digits, cx.env).map(|v| Complex::with_val(cx.prec, v)))
}

/// A tetration function of exact numbers needs one run of `tet` at the digits asked for.
pub fn direct(e: &Expr, digits: u32, env: Option<&Env>) -> Option<Result<Complex, NumErr>> {
    let Expr::Fn(name, args) = e else { return None };
    let mode = mode_of(name, args)?;
    if !args.iter().all(|a| a.as_num().is_some())
        || (mode == Mode::Tet && whole_height(&args[1]).is_some())
    {
        return None;
    }
    Some(value(mode, &args[0], &args[1], digits + 4, env))
}
